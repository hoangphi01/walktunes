// SPDX-License-Identifier: MIT OR Apache-2.0
//! The `roots` repository (`docs/DATABASE.md` §3.1).
//!
//! Roots are validated by `wt-vault` (`check_root`) before they reach this table; the
//! Ledger only stores them. Removing a root removes **rows only**, never files (S-2).

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use rusqlite::{OptionalExtension, Row, params};
use serde::Serialize;
use wt_core::RootId;

use crate::{Ledger, LedgerError, now_ms};

/// What a root is used for (`roots.kind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RootKind {
    /// The user's music.
    Library,
    /// Where pulls from a device land.
    Inbox,
    /// Scanned but read-only (for example a NAS).
    Watch,
}

impl RootKind {
    /// The stored value.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Library => "library",
            Self::Inbox => "inbox",
            Self::Watch => "watch",
        }
    }
}

impl fmt::Display for RootKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RootKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "library" => Ok(Self::Library),
            "inbox" => Ok(Self::Inbox),
            "watch" => Ok(Self::Watch),
            other => Err(format!(
                "unknown root kind `{other}` (library, inbox, watch)"
            )),
        }
    }
}

/// A folder registered in the library.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Root {
    /// Identity.
    pub id: RootId,
    /// Absolute, OS-native path.
    pub path: PathBuf,
    /// Use of the root.
    pub kind: RootKind,
    /// Disabled roots are kept but not scanned.
    pub enabled: bool,
    /// Unix milliseconds.
    pub created_at: i64,
}

/// Repository handle returned by [`Ledger::roots`].
#[derive(Debug, Clone, Copy)]
pub struct Roots<'a> {
    pub(crate) ledger: &'a Ledger,
}

const COLUMNS: &str = "id, path, kind, enabled, created_at";

fn from_row(row: &Row<'_>) -> rusqlite::Result<Result<Root, LedgerError>> {
    let id: String = row.get(0)?;
    let path: String = row.get(1)?;
    let kind: String = row.get(2)?;
    let enabled: bool = row.get(3)?;
    let created_at: i64 = row.get(4)?;
    Ok((|| {
        Ok(Root {
            id: id.parse().map_err(|_| LedgerError::InvalidValue {
                column: "roots.id",
                value: id.clone(),
            })?,
            path: PathBuf::from(path),
            kind: kind.parse().map_err(|_| LedgerError::InvalidValue {
                column: "roots.kind",
                value: kind.clone(),
            })?,
            enabled,
            created_at,
        })
    })())
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

impl Roots<'_> {
    /// Registers a root. The caller has already validated and canonicalized `path`.
    ///
    /// # Errors
    /// `WT-LEDGER-0005` if the path is already registered (unique constraint).
    pub fn add(&self, path: &Path, kind: RootKind) -> Result<Root, LedgerError> {
        let root = Root {
            id: RootId::new(),
            path: path.to_path_buf(),
            kind,
            enabled: true,
            created_at: now_ms(),
        };
        self.ledger.conn().execute(
            "INSERT INTO roots (id, path, kind, enabled, created_at) VALUES (?1, ?2, ?3, 1, ?4)",
            params![
                root.id.to_string(),
                path_text(path),
                kind.as_str(),
                root.created_at
            ],
        )?;
        Ok(root)
    }

    /// Looks a root up by id.
    ///
    /// # Errors
    /// On an SQLite failure or an invalid stored value.
    pub fn get(&self, id: RootId) -> Result<Option<Root>, LedgerError> {
        self.one(
            &format!("SELECT {COLUMNS} FROM roots WHERE id = ?1"),
            &id.to_string(),
        )
    }

    /// Looks a root up by its exact stored path.
    ///
    /// # Errors
    /// On an SQLite failure or an invalid stored value.
    pub fn by_path(&self, path: &Path) -> Result<Option<Root>, LedgerError> {
        self.one(
            &format!("SELECT {COLUMNS} FROM roots WHERE path = ?1"),
            &path_text(path),
        )
    }

    fn one(&self, sql: &str, arg: &str) -> Result<Option<Root>, LedgerError> {
        let conn = self.ledger.conn();
        conn.query_row(sql, [arg], from_row).optional()?.transpose()
    }

    /// Every root, oldest first.
    ///
    /// # Errors
    /// On an SQLite failure or an invalid stored value.
    pub fn list(&self) -> Result<Vec<Root>, LedgerError> {
        let conn = self.ledger.conn();
        let mut stmt = conn.prepare(&format!(
            "SELECT {COLUMNS} FROM roots ORDER BY created_at, id"
        ))?;
        let rows = stmt.query_map([], from_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row??);
        }
        Ok(out)
    }

    /// Enables or disables a root. Returns `false` if it does not exist.
    ///
    /// # Errors
    /// On an SQLite failure.
    pub fn set_enabled(&self, id: RootId, enabled: bool) -> Result<bool, LedgerError> {
        let n = self.ledger.conn().execute(
            "UPDATE roots SET enabled = ?2 WHERE id = ?1",
            params![id.to_string(), enabled],
        )?;
        Ok(n == 1)
    }

    /// Removes a root and its `files` rows (S-2: rows only; no file is touched).
    /// Returns `false` if it does not exist.
    ///
    /// # Errors
    /// On an SQLite failure; nothing is removed then.
    pub fn remove(&self, id: RootId) -> Result<bool, LedgerError> {
        let mut conn = self.ledger.conn();
        let tx = conn.transaction()?;
        let id = id.to_string();
        tx.execute(
            "UPDATE tracks SET preferred_file = NULL
             WHERE preferred_file IN (SELECT id FROM files WHERE root_id = ?1)",
            [&id],
        )?;
        tx.execute("DELETE FROM files WHERE root_id = ?1", [&id])?;
        let n = tx.execute("DELETE FROM roots WHERE id = ?1", [&id])?;
        tx.commit()?;
        Ok(n == 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crud() {
        let ledger = Ledger::open_in_memory().unwrap();
        let roots = ledger.roots();
        let a = roots.add(Path::new("/music/a"), RootKind::Library).unwrap();
        let b = roots
            .add(Path::new("/music/inbox"), RootKind::Inbox)
            .unwrap();
        assert_eq!(roots.list().unwrap(), vec![a.clone(), b.clone()]);
        assert_eq!(roots.get(a.id).unwrap(), Some(a.clone()));
        assert_eq!(
            roots.by_path(Path::new("/music/inbox")).unwrap(),
            Some(b.clone())
        );
        assert!(roots.set_enabled(a.id, false).unwrap());
        assert!(!roots.get(a.id).unwrap().unwrap().enabled);
        assert!(roots.remove(a.id).unwrap());
        assert!(!roots.remove(a.id).unwrap());
        assert_eq!(roots.list().unwrap(), vec![b]);
    }

    #[test]
    fn duplicate_path_is_rejected() {
        let ledger = Ledger::open_in_memory().unwrap();
        ledger
            .roots()
            .add(Path::new("/m"), RootKind::Library)
            .unwrap();
        let err = ledger
            .roots()
            .add(Path::new("/m"), RootKind::Watch)
            .unwrap_err();
        assert_eq!(wt_core::WtError::code(&err).to_string(), "WT-LEDGER-0005");
    }

    #[test]
    fn kinds_parse() {
        assert_eq!("watch".parse::<RootKind>(), Ok(RootKind::Watch));
        assert!("nas".parse::<RootKind>().is_err());
    }
}

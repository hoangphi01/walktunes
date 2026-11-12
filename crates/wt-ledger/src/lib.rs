// SPDX-License-Identifier: MIT OR Apache-2.0
//! **wt-ledger** — the SQLite catalog: schema, migrations and repositories
//! (RDS; `docs/ARCHITECTURE.md` §3.6, schema in `docs/DATABASE.md`).
//!
//! The only crate that issues SQL. Other crates use the repository methods.
//!
//! Sprint P1 Day 1 scope: open + pragmas + migrations (schema v1), the `roots`
//! repository, and the operation journal ([`wt_core::OpsJournal`]) used by `wt-vault`.

// Database backups are app data, not user music (SECURITY S-1).
#![allow(clippy::disallowed_methods)]

mod error;
mod migrate;
mod ops;
mod roots;

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

pub use error::LedgerError;
pub use ops::Ops;
pub use roots::{Root, RootKind, Roots};

/// File name of the Ledger inside the data directory.
pub const LEDGER_FILE: &str = "ledger.db";

/// An open Ledger. Cheap to share behind an `Arc`; calls are serialized internally.
pub struct Ledger {
    conn: Mutex<Connection>,
    path: Option<PathBuf>,
}

impl std::fmt::Debug for Ledger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ledger").field("path", &self.path).finish()
    }
}

impl Ledger {
    /// The schema version this build writes and understands.
    pub const SCHEMA_VERSION: u32 = migrate::LATEST;

    /// Opens (creating if needed) the Ledger at `path` and migrates it to
    /// [`Ledger::SCHEMA_VERSION`].
    ///
    /// # Errors
    /// `WT-LEDGER-0001` not a database / corrupt, `WT-LEDGER-0002` written by a newer
    /// WalkTunes, `WT-LEDGER-0003` migration failed (rolled back), `WT-LEDGER-0004` busy.
    pub fn open(path: &Path) -> Result<Self, LedgerError> {
        let conn = Connection::open(path).map_err(|e| LedgerError::from_open(path, e))?;
        Self::setup(conn, Some(path.to_path_buf()))
    }

    /// Opens a throwaway in-memory Ledger (tests, dry runs).
    ///
    /// # Errors
    /// Only on an internal SQLite failure.
    pub fn open_in_memory() -> Result<Self, LedgerError> {
        let conn = Connection::open_in_memory().map_err(LedgerError::from)?;
        Self::setup(conn, None)
    }

    fn setup(mut conn: Connection, path: Option<PathBuf>) -> Result<Self, LedgerError> {
        let p = path.clone().unwrap_or_default();
        // The first statement is where a non-database file is detected.
        let _mode: String = conn
            .query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))
            .map_err(|e| LedgerError::from_open(&p, e))?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(Duration::from_millis(5000))?;
        migrate::run(&mut conn, path.as_deref())?;
        Ok(Self {
            conn: Mutex::new(conn),
            path,
        })
    }

    /// Where the Ledger lives; `None` for an in-memory Ledger.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// The schema version stored in the database (`PRAGMA user_version`).
    ///
    /// # Errors
    /// On an SQLite failure.
    pub fn schema_version(&self) -> Result<u32, LedgerError> {
        migrate::user_version(&self.conn())
    }

    /// The library roots repository.
    #[must_use]
    pub fn roots(&self) -> Roots<'_> {
        Roots { ledger: self }
    }

    /// Read access to the operation journal (`ops_log`).
    #[must_use]
    pub fn ops(&self) -> Ops<'_> {
        Ops { ledger: self }
    }

    /// The connection. A poisoned lock still holds a valid connection: SQLite rolled back
    /// any open transaction when the panicking scope dropped it.
    fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Current time in Unix milliseconds (DATABASE §2).
pub(crate) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_database_is_at_latest_version() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = Ledger::open(&dir.path().join(LEDGER_FILE)).unwrap();
        assert_eq!(ledger.schema_version().unwrap(), Ledger::SCHEMA_VERSION);
        assert_eq!(Ledger::SCHEMA_VERSION, 1);
    }

    #[test]
    fn reopening_is_idempotent_and_keeps_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(LEDGER_FILE);
        let root_dir = dir.path().join("Music");
        {
            let ledger = Ledger::open(&path).unwrap();
            ledger.roots().add(&root_dir, RootKind::Library).unwrap();
        }
        let ledger = Ledger::open(&path).unwrap();
        assert_eq!(ledger.roots().list().unwrap().len(), 1);
        // No migration ran, so no backup was taken.
        let backups = std::fs::read_dir(dir.path())
            .unwrap()
            .filter(|e| {
                e.as_ref()
                    .is_ok_and(|e| e.file_name().to_string_lossy().contains(".bak-v"))
            })
            .count();
        assert_eq!(backups, 0);
    }

    #[test]
    fn database_from_newer_app_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(LEDGER_FILE);
        {
            let conn = Connection::open(&path).unwrap();
            conn.pragma_update(None, "user_version", 99).unwrap();
        }
        let err = Ledger::open(&path).unwrap_err();
        assert_eq!(wt_core::WtError::code(&err).to_string(), "WT-LEDGER-0002");
    }

    #[test]
    fn non_database_file_is_reported_corrupt() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(LEDGER_FILE);
        std::fs::write(&path, b"this is not an sqlite database, just some text....").unwrap();
        let err = Ledger::open(&path).unwrap_err();
        assert_eq!(wt_core::WtError::code(&err).to_string(), "WT-LEDGER-0001");
    }

    #[test]
    fn fts_matches_without_diacritics() {
        let ledger = Ledger::open_in_memory().unwrap();
        let conn = ledger.conn();
        conn.execute(
            "INSERT INTO tracks (id, title, artist, identity_level, created_at, updated_at)
             VALUES ('t1', 'Hà Nội Mùa Thu', 'Sơn Tùng', 5, 0, 0)",
            [],
        )
        .unwrap();
        let hits: i64 = conn
            .query_row(
                "SELECT count(*) FROM tracks_fts WHERE tracks_fts MATCH 'son tung'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hits, 1);
        let hits: i64 = conn
            .query_row(
                "SELECT count(*) FROM tracks_fts WHERE tracks_fts MATCH 'ha noi mua thu'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hits, 1);
    }

    #[test]
    fn foreign_keys_are_enforced() {
        let ledger = Ledger::open_in_memory().unwrap();
        let err = ledger.conn().execute(
            "INSERT INTO device_volumes (device_id, role) VALUES ('missing', 'internal')",
            [],
        );
        assert!(err.is_err());
    }
}

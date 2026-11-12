// SPDX-License-Identifier: MIT OR Apache-2.0
//! Forward-only migrations (`docs/DATABASE.md` §5).
//!
//! The schema version lives in `PRAGMA user_version`. Before migrating an existing
//! database it is copied with the SQLite backup API to `ledger.db.bak-v{old}`; the last
//! three backups are kept. Each migration runs in one transaction.

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use rusqlite::backup::Backup;

use crate::LedgerError;

/// `(version, name, sql)`, in order. Append-only once released.
const MIGRATIONS: &[(u32, &str, &str)] =
    &[(1, "initial", include_str!("../migrations/V1__initial.sql"))];

/// Highest schema version this build knows.
pub(crate) const LATEST: u32 = MIGRATIONS[MIGRATIONS.len() - 1].0;

/// How many pre-migration backups to keep.
const KEEP_BACKUPS: usize = 3;

pub(crate) fn user_version(conn: &Connection) -> Result<u32, LedgerError> {
    Ok(conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
}

pub(crate) fn run(conn: &mut Connection, path: Option<&Path>) -> Result<(), LedgerError> {
    let current = user_version(conn)?;
    if current > LATEST {
        return Err(LedgerError::TooNew {
            found: current,
            supported: LATEST,
        });
    }
    if current == LATEST {
        return Ok(());
    }
    if current >= 1
        && let Some(path) = path
    {
        backup(conn, path, current)?;
    }
    for &(version, name, sql) in MIGRATIONS.iter().filter(|m| m.0 > current) {
        let migrate = |conn: &mut Connection| -> rusqlite::Result<()> {
            let tx = conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", version)?;
            tx.commit()
        };
        migrate(conn).map_err(|source| LedgerError::Migration { version, source })?;
        wt_watch::info!(version, name, "ledger migrated");
    }
    Ok(())
}

fn backup_path(db: &Path, version: u32) -> PathBuf {
    let mut name = db.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".bak-v{version}"));
    db.with_file_name(name)
}

fn backup(conn: &Connection, db: &Path, version: u32) -> Result<(), LedgerError> {
    let target = backup_path(db, version);
    let copy = || -> rusqlite::Result<()> {
        let mut dst = Connection::open(&target)?;
        let b = Backup::new(conn, &mut dst)?;
        b.run_to_completion(256, std::time::Duration::ZERO, None)
    };
    copy().map_err(|source| LedgerError::Backup {
        path: target.clone(),
        source,
    })?;
    prune_backups(db);
    Ok(())
}

/// Removes all but the newest [`KEEP_BACKUPS`] backups. Failures are logged, never fatal.
fn prune_backups(db: &Path) {
    let (Some(dir), Some(stem)) = (db.parent(), db.file_name()) else {
        return;
    };
    let prefix = format!("{}.bak-v", stem.to_string_lossy());
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut found: Vec<(u32, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let v = name.strip_prefix(&prefix)?.parse().ok()?;
            Some((v, e.path()))
        })
        .collect();
    found.sort_by_key(|(v, _)| std::cmp::Reverse(*v));
    for (_, path) in found.into_iter().skip(KEEP_BACKUPS) {
        if let Err(e) = std::fs::remove_file(&path) {
            wt_watch::warn!(path = %path.display(), error = %e, "could not prune ledger backup");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_numbered_from_one_without_gaps() {
        for (i, m) in MIGRATIONS.iter().enumerate() {
            assert_eq!(m.0 as usize, i + 1, "migration {} out of order", m.1);
        }
    }

    #[test]
    fn backup_names_follow_the_docs() {
        assert_eq!(
            backup_path(Path::new("/d/ledger.db"), 1),
            PathBuf::from("/d/ledger.db.bak-v1")
        );
    }

    #[test]
    fn pruning_keeps_the_newest_three() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("ledger.db");
        for v in 1..=5 {
            std::fs::write(backup_path(&db, v), b"x").unwrap();
        }
        prune_backups(&db);
        for v in 1..=5 {
            assert_eq!(backup_path(&db, v).exists(), v >= 3, "v{v}");
        }
    }

    #[test]
    fn failed_migration_rolls_back() {
        let mut conn = Connection::open_in_memory().unwrap();
        let bad = |conn: &mut Connection| -> rusqlite::Result<()> {
            let tx = conn.transaction()?;
            tx.execute_batch("CREATE TABLE a (x); THIS IS NOT SQL;")?;
            tx.pragma_update(None, "user_version", 7)?;
            tx.commit()
        };
        assert!(bad(&mut conn).is_err());
        assert_eq!(user_version(&conn).unwrap(), 0);
        let tables: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name='a'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(tables, 0);
    }
}

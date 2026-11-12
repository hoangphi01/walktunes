// SPDX-License-Identifier: MIT OR Apache-2.0
//! **wt-vault** — every write, move and delete of user files
//! (S3; `docs/ARCHITECTURE.md` §3.7, `docs/SECURITY.md` §3–§4).
//!
//! The single choke point for data safety: no other crate mutates user music (S-1).
//! Sprint P1 Day 1 scope: content hashing, path confinement ([`RelPath`], [`Scope`]),
//! root validation, journaled atomic writes ([`Vault::write_atomic`]) and crash
//! recovery ([`Vault::recover`]). Trash, moves and retagging arrive with M2/M5.

// This crate is the one place allowed to mutate files (SECURITY S-1).
#![allow(clippy::disallowed_methods)]

mod error;
mod hash;
mod path;
mod roots;
mod write;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use serde::Serialize;
use wt_core::{OpKind, OpState, OpsJournal};

pub use error::{INTERRUPTED, VaultError};
pub use hash::{content_hash, hash_bytes, hash_reader};
pub use path::{RelPath, Scope};
pub use roots::{check_root, ensure_no_overlap, overlaps};
pub use write::{TEMP_PREFIX, WriteMode, WriteOutcome};

/// The file-mutation service. Every mutation is journaled through an [`OpsJournal`]
/// (the Ledger in the app, [`wt_core::MemoryJournal`] in tests).
pub struct Vault {
    journal: Arc<dyn OpsJournal>,
    #[cfg(test)]
    failpoint: std::sync::Mutex<Option<(write::Step, write::FailMode)>>,
}

impl std::fmt::Debug for Vault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vault").finish_non_exhaustive()
    }
}

/// What [`Vault::recover`] cleaned up.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct RecoveryReport {
    /// Operations found `pending` and marked `failed` with `WT-VAULT-0007`.
    pub interrupted: usize,
    /// Leftover temp files removed.
    pub temp_files_removed: usize,
}

impl Vault {
    /// Creates a Vault that journals into `journal`.
    #[must_use]
    pub fn new(journal: Arc<dyn OpsJournal>) -> Self {
        Self {
            journal,
            #[cfg(test)]
            failpoint: std::sync::Mutex::new(None),
        }
    }

    /// Cleans up after a crash. Call once at startup, before any other operation.
    ///
    /// Every `pending` operation was interrupted: its temp file (if any) is removed and the
    /// operation is marked `failed` with `WT-VAULT-0007`. Targets are never touched: an
    /// interrupted atomic write left the target either as it was or fully replaced.
    ///
    /// # Errors
    /// The journal cannot be read or updated.
    pub fn recover(&self) -> Result<RecoveryReport, VaultError> {
        let mut report = RecoveryReport::default();
        for entry in self.journal.pending()? {
            if matches!(entry.op, OpKind::Write | OpKind::Replace)
                && let Some(temp) = &entry.aux
                && std::fs::symlink_metadata(temp).is_ok()
            {
                write::remove_temp(temp);
                if std::fs::symlink_metadata(temp).is_err() {
                    report.temp_files_removed += 1;
                }
            }
            self.journal
                .finish(entry.id, OpState::Failed, None, Some(INTERRUPTED))?;
            report.interrupted += 1;
            wt_watch::warn!(op = %entry.id, target = %entry.target.display(), "interrupted operation recovered");
        }
        Ok(report)
    }

    #[cfg(test)]
    pub(crate) fn arm(&self, step: write::Step, mode: write::FailMode) {
        if let Ok(mut g) = self.failpoint.lock() {
            *g = Some((step, mode));
        }
    }
}

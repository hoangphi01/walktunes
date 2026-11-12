// SPDX-License-Identifier: MIT OR Apache-2.0
//! The operation journal contract (`docs/ARCHITECTURE.md` ADR-0018, SECURITY S-5).
//!
//! `wt-vault` records every file mutation **before** it happens and finishes the record
//! afterwards. The records live in the Ledger's `ops_log` table, but Vault and Ledger are
//! both Layer 2 and may not depend on each other, so the contract lives here: Ledger
//! implements [`OpsJournal`], Vault calls it, and `wt-sdk` wires the two together.

use std::fmt;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{ContentHash, ErrorCode, Recovery, Severity, SyncRunId, WtError};

/// Row id of an `ops_log` entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OpId(pub i64);

impl fmt::Display for OpId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// What kind of mutation an operation is (`ops_log.op`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpKind {
    /// A new file was written.
    Write,
    /// An existing file was replaced.
    Replace,
    /// A file was moved or renamed.
    Move,
    /// A file was moved to the trash.
    Trash,
    /// A file was restored from the trash.
    Restore,
    /// Tags were rewritten in place.
    Retag,
    /// A directory was created.
    Mkdir,
}

impl OpKind {
    /// The value stored in `ops_log.op`.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Write => "write",
            Self::Replace => "replace",
            Self::Move => "move",
            Self::Trash => "trash",
            Self::Restore => "restore",
            Self::Retag => "retag",
            Self::Mkdir => "mkdir",
        }
    }

    /// Parses a stored value.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "write" => Self::Write,
            "replace" => Self::Replace,
            "move" => Self::Move,
            "trash" => Self::Trash,
            "restore" => Self::Restore,
            "retag" => Self::Retag,
            "mkdir" => Self::Mkdir,
            _ => return None,
        })
    }
}

/// Lifecycle of an operation (`ops_log.state`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpState {
    /// Recorded, not yet finished. Left behind only by a crash.
    Pending,
    /// Completed.
    Done,
    /// Reverted by an undo.
    Undone,
    /// Did not complete; the target was left as it was.
    Failed,
}

impl OpState {
    /// The value stored in `ops_log.state`.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Done => "done",
            Self::Undone => "undone",
            Self::Failed => "failed",
        }
    }

    /// Parses a stored value.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "pending" => Self::Pending,
            "done" => Self::Done,
            "undone" => Self::Undone,
            "failed" => Self::Failed,
            _ => return None,
        })
    }
}

/// What Vault is about to do, recorded before it does it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpRecord {
    /// The sync run this operation belongs to, if any.
    pub run_id: Option<SyncRunId>,
    /// Kind of mutation.
    pub op: OpKind,
    /// Absolute path of the file or directory being changed.
    pub target: PathBuf,
    /// Secondary path: the temp file of an atomic write, a move source, a trash location.
    pub aux: Option<PathBuf>,
    /// Hash of the target before the operation, when it existed.
    pub hash_before: Option<ContentHash>,
}

/// A stored journal entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OpEntry {
    /// Row id.
    pub id: OpId,
    /// The sync run, if any.
    pub run_id: Option<SyncRunId>,
    /// Kind of mutation.
    pub op: OpKind,
    /// Target path.
    pub target: PathBuf,
    /// Secondary path.
    pub aux: Option<PathBuf>,
    /// Hash before.
    pub hash_before: Option<ContentHash>,
    /// Hash after, set when done.
    pub hash_after: Option<ContentHash>,
    /// Current state.
    pub state: OpState,
    /// Error code when failed, e.g. `WT-VAULT-0003`.
    pub error_code: Option<String>,
    /// Unix milliseconds of the last state change.
    pub at: i64,
}

/// A journal that could not record an operation. Vault refuses to mutate anything
/// it cannot journal (S-5), so this aborts the operation.
#[derive(Debug, Error)]
#[error("operation journal failed ({code}): {message}")]
pub struct JournalError {
    /// The underlying storage error code, e.g. `WT-LEDGER-0004`.
    pub code: ErrorCode,
    /// Severity of the underlying error.
    pub severity: Severity,
    /// Recovery of the underlying error.
    pub recovery: Recovery,
    /// Human-readable detail (logged, not shown to users).
    pub message: String,
}

impl WtError for JournalError {
    fn code(&self) -> ErrorCode {
        self.code
    }

    fn severity(&self) -> Severity {
        self.severity
    }

    fn recovery(&self) -> Recovery {
        self.recovery
    }
}

/// Durable record of file mutations (SECURITY S-5). Implemented by `wt-ledger`.
pub trait OpsJournal: Send + Sync {
    /// Records `rec` as `pending` and returns its id. Called **before** the mutation.
    ///
    /// # Errors
    /// The record could not be stored; the caller must not perform the mutation.
    fn begin(&self, rec: &OpRecord) -> Result<OpId, JournalError>;

    /// Moves an operation to its final state.
    ///
    /// # Errors
    /// The update could not be stored.
    fn finish(
        &self,
        id: OpId,
        state: OpState,
        hash_after: Option<&ContentHash>,
        error_code: Option<ErrorCode>,
    ) -> Result<(), JournalError>;

    /// Operations still `pending` (left behind by a crash), oldest first.
    ///
    /// # Errors
    /// The journal could not be read.
    fn pending(&self) -> Result<Vec<OpEntry>, JournalError>;
}

/// An in-memory journal for tests and throwaway sessions.
#[derive(Debug, Default)]
pub struct MemoryJournal {
    entries: Mutex<Vec<OpEntry>>,
}

impl MemoryJournal {
    /// Creates an empty journal.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A copy of every entry, oldest first.
    #[must_use]
    pub fn entries(&self) -> Vec<OpEntry> {
        self.entries.lock().map(|e| e.clone()).unwrap_or_default()
    }

    fn poisoned() -> JournalError {
        JournalError {
            code: ErrorCode::new("CORE", 1),
            severity: Severity::Fatal,
            recovery: Recovery::Abort,
            message: "memory journal lock poisoned".into(),
        }
    }
}

impl OpsJournal for MemoryJournal {
    fn begin(&self, rec: &OpRecord) -> Result<OpId, JournalError> {
        let mut entries = self.entries.lock().map_err(|_| Self::poisoned())?;
        let id = OpId(i64::try_from(entries.len()).unwrap_or(i64::MAX) + 1);
        entries.push(OpEntry {
            id,
            run_id: rec.run_id,
            op: rec.op,
            target: rec.target.clone(),
            aux: rec.aux.clone(),
            hash_before: rec.hash_before,
            hash_after: None,
            state: OpState::Pending,
            error_code: None,
            at: 0,
        });
        Ok(id)
    }

    fn finish(
        &self,
        id: OpId,
        state: OpState,
        hash_after: Option<&ContentHash>,
        error_code: Option<ErrorCode>,
    ) -> Result<(), JournalError> {
        let mut entries = self.entries.lock().map_err(|_| Self::poisoned())?;
        if let Some(e) = entries.iter_mut().find(|e| e.id == id) {
            e.state = state;
            e.hash_after = hash_after.copied();
            e.error_code = error_code.map(|c| c.to_string());
        }
        Ok(())
    }

    fn pending(&self) -> Result<Vec<OpEntry>, JournalError> {
        let entries = self.entries.lock().map_err(|_| Self::poisoned())?;
        Ok(entries
            .iter()
            .filter(|e| e.state == OpState::Pending)
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_and_states_round_trip() {
        for k in [
            OpKind::Write,
            OpKind::Replace,
            OpKind::Move,
            OpKind::Trash,
            OpKind::Restore,
            OpKind::Retag,
            OpKind::Mkdir,
        ] {
            assert_eq!(OpKind::parse(k.as_str()), Some(k));
        }
        for s in [
            OpState::Pending,
            OpState::Done,
            OpState::Undone,
            OpState::Failed,
        ] {
            assert_eq!(OpState::parse(s.as_str()), Some(s));
        }
    }

    #[test]
    fn memory_journal_tracks_pending() {
        let j = MemoryJournal::new();
        let rec = OpRecord {
            run_id: None,
            op: OpKind::Write,
            target: PathBuf::from("/x/a.flac"),
            aux: None,
            hash_before: None,
        };
        let a = j.begin(&rec).unwrap();
        let b = j.begin(&rec).unwrap();
        j.finish(a, OpState::Done, None, None).unwrap();
        let pending = j.pending().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, b);
    }
}

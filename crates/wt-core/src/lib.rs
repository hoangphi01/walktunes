// SPDX-License-Identifier: MIT OR Apache-2.0
//! **wt-core** — the shared kernel of WalkTunes.
//!
//! Typed IDs, shared value types and the error-code model used by every other
//! service. This crate performs **no I/O** and depends only on `serde`,
//! `thiserror` and `uuid` (see `docs/ARCHITECTURE.md` §3.0).

mod error;
mod format;
mod hash;
mod id;
mod ops;

pub use error::{ErrorCode, Recovery, Severity, WtError};
pub use format::AudioFormat;
pub use hash::{ContentHash, ParseHashError};
pub use id::{DeviceId, FileId, JobId, MachineId, RootId, SyncRunId, TrackId};
pub use ops::{JournalError, MemoryJournal, OpEntry, OpId, OpKind, OpRecord, OpState, OpsJournal};

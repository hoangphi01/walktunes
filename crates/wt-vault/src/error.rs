// SPDX-License-Identifier: MIT OR Apache-2.0
//! Vault errors (`docs/ERROR_CONTROL.md` §5, `WT-VAULT-*`).

use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;
use wt_core::{ContentHash, ErrorCode, JournalError, Recovery, Severity, WtError};

/// Error code recorded on operations found `pending` at startup (`WT-VAULT-0007`).
pub const INTERRUPTED: ErrorCode = ErrorCode::new("VAULT", 7);

/// Errors raised by `wt-vault`.
#[derive(Debug, Error)]
pub enum VaultError {
    /// A root does not exist or is not a mounted directory (`WT-VAULT-0001`).
    #[error("root not found or not a directory: {path}")]
    RootNotFound {
        /// The path given.
        path: PathBuf,
    },
    /// Permission denied or read-only file system (`WT-VAULT-0002`).
    #[error("permission denied: {path}")]
    PermissionDenied {
        /// The path.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: io::Error,
    },
    /// The bytes on disk do not match what was written or expected (`WT-VAULT-0003`, safety).
    #[error("hash mismatch after write: {path} (expected {expected}, found {actual})")]
    HashMismatch {
        /// The file.
        path: PathBuf,
        /// Expected hash.
        expected: ContentHash,
        /// Hash found.
        actual: ContentHash,
    },
    /// A write outside the allowed scope, or an overwrite that was not allowed
    /// (`WT-VAULT-0004`, safety).
    #[error("refused: {reason}: {path}")]
    Refused {
        /// The path.
        path: PathBuf,
        /// Why.
        reason: &'static str,
    },
    /// The target file system is full (`WT-VAULT-0005`).
    #[error("disk full: {path}")]
    DiskFull {
        /// The path being written.
        path: PathBuf,
    },
    /// The path is malformed or illegal for the target (`WT-VAULT-0006`).
    #[error("invalid path ({reason}): {path}")]
    BadPath {
        /// The offending path text.
        path: String,
        /// Why.
        reason: &'static str,
    },
    /// The new root overlaps an existing one (`WT-VAULT-0008`).
    #[error("{path} overlaps the existing root {existing}")]
    RootOverlap {
        /// The new root.
        path: PathBuf,
        /// The existing root.
        existing: PathBuf,
    },
    /// Any other I/O failure (`WT-VAULT-0009`).
    #[error("i/o error on {path}")]
    Io {
        /// The path.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: io::Error,
    },
    /// The operation journal failed, so nothing was changed (S-5).
    #[error(transparent)]
    Journal(#[from] JournalError),
    /// Test-only: a failpoint simulated a crash (no cleanup ran).
    #[cfg(test)]
    #[error("simulated crash")]
    SimulatedCrash,
}

impl VaultError {
    /// Maps an I/O error on `path` to the most specific Vault error.
    pub(crate) fn io(path: &Path, source: io::Error) -> Self {
        let path = path.to_path_buf();
        match source.kind() {
            io::ErrorKind::PermissionDenied | io::ErrorKind::ReadOnlyFilesystem => {
                Self::PermissionDenied { path, source }
            }
            io::ErrorKind::StorageFull | io::ErrorKind::QuotaExceeded => Self::DiskFull { path },
            io::ErrorKind::InvalidFilename => Self::BadPath {
                path: path.to_string_lossy().into_owned(),
                reason: "rejected by the file system",
            },
            _ => Self::Io { path, source },
        }
    }
}

impl WtError for VaultError {
    fn code(&self) -> ErrorCode {
        let n = match self {
            Self::RootNotFound { .. } => 1,
            Self::PermissionDenied { .. } => 2,
            Self::HashMismatch { .. } => 3,
            Self::Refused { .. } => 4,
            Self::DiskFull { .. } => 5,
            Self::BadPath { .. } => 6,
            Self::RootOverlap { .. } => 8,
            Self::Io { .. } => 9,
            Self::Journal(e) => return e.code(),
            #[cfg(test)]
            Self::SimulatedCrash => return ErrorCode::new("CORE", 1),
        };
        ErrorCode::new("VAULT", n)
    }

    fn severity(&self) -> Severity {
        match self {
            Self::HashMismatch { .. } | Self::Refused { .. } => Severity::Fatal,
            Self::Journal(e) => e.severity(),
            _ => Severity::Error,
        }
    }

    fn recovery(&self) -> Recovery {
        match self {
            Self::HashMismatch { .. } | Self::Refused { .. } => Recovery::Abort,
            Self::RootNotFound { .. } | Self::DiskFull { .. } | Self::RootOverlap { .. } => {
                Recovery::UserAction
            }
            Self::Journal(e) => e.recovery(),
            _ => Recovery::Skip,
        }
    }
}

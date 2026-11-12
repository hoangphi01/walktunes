// SPDX-License-Identifier: MIT OR Apache-2.0
//! Ledger errors (`docs/ERROR_CONTROL.md` §5, `WT-LEDGER-*`).

use std::path::{Path, PathBuf};

use rusqlite::ErrorCode as SqlCode;
use thiserror::Error;
use wt_core::{ErrorCode, JournalError, Recovery, Severity, WtError};

/// Errors raised by `wt-ledger`.
#[derive(Debug, Error)]
pub enum LedgerError {
    /// The file is not a database or is corrupt (`WT-LEDGER-0001`).
    #[error("ledger database is corrupt or not a database: {path}")]
    Corrupt {
        /// Database path.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: rusqlite::Error,
    },
    /// The database was written by a newer WalkTunes (`WT-LEDGER-0002`).
    #[error("ledger schema v{found} is newer than this app (v{supported}); update WalkTunes")]
    TooNew {
        /// Version found in the file.
        found: u32,
        /// Highest version this build understands.
        supported: u32,
    },
    /// A migration failed and was rolled back (`WT-LEDGER-0003`).
    #[error("ledger migration to v{version} failed; rolled back")]
    Migration {
        /// Target version.
        version: u32,
        /// Underlying error.
        #[source]
        source: rusqlite::Error,
    },
    /// The pre-migration backup could not be written (`WT-LEDGER-0003`).
    #[error("ledger backup before migration failed: {path}")]
    Backup {
        /// Backup path.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: rusqlite::Error,
    },
    /// The database stayed locked past the busy timeout (`WT-LEDGER-0004`).
    #[error("ledger database is busy")]
    Busy(#[source] rusqlite::Error),
    /// Any other query failure: a bug or a constraint violation (`WT-LEDGER-0005`).
    #[error("ledger query failed")]
    Query(#[source] rusqlite::Error),
    /// A stored value could not be decoded (`WT-LEDGER-0005`).
    #[error("ledger holds an invalid value in {column}: {value}")]
    InvalidValue {
        /// Column name.
        column: &'static str,
        /// Offending value.
        value: String,
    },
}

impl LedgerError {
    pub(crate) fn from_open(path: &Path, e: rusqlite::Error) -> Self {
        match e.sqlite_error_code() {
            Some(SqlCode::NotADatabase | SqlCode::DatabaseCorrupt) => Self::Corrupt {
                path: path.to_path_buf(),
                source: e,
            },
            _ => Self::from(e),
        }
    }
}

impl From<rusqlite::Error> for LedgerError {
    fn from(e: rusqlite::Error) -> Self {
        match e.sqlite_error_code() {
            Some(SqlCode::DatabaseBusy | SqlCode::DatabaseLocked) => Self::Busy(e),
            Some(SqlCode::NotADatabase | SqlCode::DatabaseCorrupt) => Self::Corrupt {
                path: PathBuf::new(),
                source: e,
            },
            _ => Self::Query(e),
        }
    }
}

impl WtError for LedgerError {
    fn code(&self) -> ErrorCode {
        let n = match self {
            Self::Corrupt { .. } => 1,
            Self::TooNew { .. } => 2,
            Self::Migration { .. } | Self::Backup { .. } => 3,
            Self::Busy(_) => 4,
            Self::Query(_) | Self::InvalidValue { .. } => 5,
        };
        ErrorCode::new("LEDGER", n)
    }

    fn severity(&self) -> Severity {
        match self {
            Self::Corrupt { .. } | Self::TooNew { .. } => Severity::Fatal,
            Self::Busy(_) => Severity::Warning,
            _ => Severity::Error,
        }
    }

    fn recovery(&self) -> Recovery {
        match self {
            Self::Corrupt { .. } | Self::TooNew { .. } => Recovery::UserAction,
            Self::Busy(_) => Recovery::Retry,
            _ => Recovery::Abort,
        }
    }
}

impl From<LedgerError> for JournalError {
    fn from(e: LedgerError) -> Self {
        Self {
            code: e.code(),
            severity: e.severity(),
            recovery: e.recovery(),
            message: e.to_string(),
        }
    }
}

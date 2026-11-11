// SPDX-License-Identifier: MIT OR Apache-2.0
//! Errors and warnings raised by `wt-dock` (registry: `docs/ERROR_CONTROL.md` §5).

use thiserror::Error;
use wt_core::{ErrorCode, Recovery, Severity, WtError};

/// Errors and warnings raised by `wt-dock`.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DockError {
    /// A built-in profile failed to parse (a bug; caught by tests).
    #[error("invalid built-in profile {id}: {reason}")]
    InvalidProfile {
        /// Profile file id.
        id: String,
        /// Parse error.
        reason: String,
    },
    /// `device.json` names a profile this version does not know (`WT-DOCK-0003`).
    #[error("unknown profile id in device.json: {0}")]
    UnknownProfile(String),
    /// The platform probe could not run; fell back to heuristics (`WT-DOCK-0007`).
    #[error("platform probe unavailable for {0}; using volume heuristics")]
    ProbeUnavailable(String),
    /// `device.json` belongs to another USB serial (`WT-DOCK-0008`).
    #[error("device.json was paired with another device (serial mismatch)")]
    SerialMismatch,
}

impl WtError for DockError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::InvalidProfile { .. } => ErrorCode::new("CORE", 1),
            Self::UnknownProfile(_) => ErrorCode::new("DOCK", 3),
            Self::ProbeUnavailable(_) => ErrorCode::new("DOCK", 7),
            Self::SerialMismatch => ErrorCode::new("DOCK", 8),
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Self::InvalidProfile { .. } => Severity::Fatal,
            Self::UnknownProfile(_) => Severity::Error,
            Self::ProbeUnavailable(_) => Severity::Info,
            Self::SerialMismatch => Severity::Warning,
        }
    }

    fn recovery(&self) -> Recovery {
        match self {
            Self::InvalidProfile { .. } => Recovery::Abort,
            Self::ProbeUnavailable(_) => Recovery::None,
            Self::UnknownProfile(_) | Self::SerialMismatch => Recovery::UserAction,
        }
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later
//! The error-code model shared by every service (`docs/ERROR_CONTROL.md` §2).

use std::fmt;

use serde::{Deserialize, Serialize};

/// A stable error code such as `WT-DOCK-0004`.
///
/// Every code used in the source must be registered in `docs/ERROR_CONTROL.md` §5;
/// `cargo xtask docs-check` verifies this.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ErrorCode {
    service: &'static str,
    number: u16,
}

impl ErrorCode {
    /// Creates a code from a service key (`"DOCK"`) and a number (`4`).
    #[must_use]
    pub const fn new(service: &'static str, number: u16) -> Self {
        Self { service, number }
    }

    /// The service key, for example `"DOCK"`.
    #[must_use]
    pub const fn service(&self) -> &'static str {
        self.service
    }

    /// The number within the service.
    #[must_use]
    pub const fn number(&self) -> u16 {
        self.number
    }

    /// The Fluent key for the user-facing message, for example `error-dock-0004`.
    #[must_use]
    pub fn i18n_key(&self) -> String {
        format!(
            "error-{}-{:04}",
            self.service.to_ascii_lowercase(),
            self.number
        )
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WT-{}-{:04}", self.service, self.number)
    }
}

/// How serious an error is, which decides how it is presented (ERROR_CONTROL §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Inline hint; operation continues.
    Info,
    /// Item flagged; operation continues.
    Warning,
    /// Operation failed.
    Error,
    /// The app cannot continue safely.
    Fatal,
}

/// What should happen next (ERROR_CONTROL §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Recovery {
    /// Nothing to do.
    None,
    /// Transient: retry automatically with backoff.
    Retry,
    /// The user must decide or act.
    UserAction,
    /// Skip this item and continue the batch.
    Skip,
    /// Safety error: stop the whole job immediately.
    Abort,
}

/// Implemented by every service error enum.
pub trait WtError: std::error::Error + Send + Sync + 'static {
    /// The registered error code.
    fn code(&self) -> ErrorCode;
    /// How serious the error is.
    fn severity(&self) -> Severity;
    /// What should happen next.
    fn recovery(&self) -> Recovery;
    /// Key/value context that is safe to log (no file contents, no secrets).
    fn context(&self) -> Vec<(&'static str, String)> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_format_with_four_digits() {
        assert_eq!(ErrorCode::new("DOCK", 4).to_string(), "WT-DOCK-0004");
        assert_eq!(ErrorCode::new("UI", 1).to_string(), "WT-UI-0001");
    }

    #[test]
    fn i18n_keys_are_lowercase() {
        assert_eq!(ErrorCode::new("SYNC", 3).i18n_key(), "error-sync-0003");
    }

    #[test]
    fn severity_orders_by_seriousness() {
        assert!(Severity::Info < Severity::Fatal);
    }
}

// SPDX-License-Identifier: MIT OR Apache-2.0
//! **wt-watch** — logging and tracing (CloudWatch; `docs/ARCHITECTURE.md` §3.2).
//!
//! Skeleton scope (M0): a `tracing` subscriber on stderr filtered by `WT_LOG`.
//! Rotating log files, local metrics and the diagnostics bundle come later.
//! Nothing is ever sent over the network.

pub use tracing::{debug, error, info, trace, warn};

/// Environment variable holding the `tracing` filter, e.g. `wt_dock=debug,info`.
pub const ENV_LOG: &str = "WT_LOG";

/// Installs the global subscriber. Safe to call more than once; later calls are no-ops.
pub fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_env(ENV_LOG)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(true)
        .try_init();
}

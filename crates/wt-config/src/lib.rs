// SPDX-License-Identifier: GPL-3.0-or-later
//! **wt-config** — settings, per-OS paths and the machine identity
//! (Parameter Store; `docs/ARCHITECTURE.md` §3.1).
//!
//! Skeleton scope (M0): resolving the data, cache and log directories, and the
//! `MachineId` created once per install. `settings.toml` arrives in M1.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;
use wt_core::{ErrorCode, MachineId, Recovery, Severity, WtError};

/// Environment variable overriding the data directory (Ledger, config, logs).
pub const ENV_DATA_DIR: &str = "WT_DATA_DIR";
/// Environment variable overriding the cache directory.
pub const ENV_CACHE_DIR: &str = "WT_CACHE_DIR";

const MACHINE_ID_FILE: &str = "machine_id";

/// Errors raised by `wt-config`.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The settings file could not be read (`WT-CONFIG-0001`).
    #[error("settings file unreadable: {path}")]
    SettingsUnreadable {
        /// Path of the file.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: io::Error,
    },
    /// The data directory cannot be created or written (`WT-CONFIG-0002`).
    #[error("data directory not writable: {path}")]
    DataDirNotWritable {
        /// Path of the directory.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: io::Error,
    },
    /// No home directory could be determined and no override was given (`WT-CONFIG-0002`).
    #[error("no home directory found; set {ENV_DATA_DIR}")]
    NoHomeDirectory,
}

impl WtError for ConfigError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::SettingsUnreadable { .. } => ErrorCode::new("CONFIG", 1),
            Self::DataDirNotWritable { .. } | Self::NoHomeDirectory => ErrorCode::new("CONFIG", 2),
        }
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn recovery(&self) -> Recovery {
        Recovery::UserAction
    }
}

/// The directories WalkTunes uses on this machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Paths {
    /// Ledger, settings and machine identity.
    pub data_dir: PathBuf,
    /// Disposable cache (thumbnails, spectrograms, API responses).
    pub cache_dir: PathBuf,
    /// Rotating log files.
    pub log_dir: PathBuf,
}

impl Paths {
    /// Resolves the paths from environment overrides, falling back to the OS
    /// conventions (`directories::ProjectDirs("dev", "walktunes", "WalkTunes")`).
    ///
    /// # Errors
    /// [`ConfigError::NoHomeDirectory`] when neither an override nor a home directory exists.
    pub fn resolve() -> Result<Self, ConfigError> {
        let env_data = std::env::var_os(ENV_DATA_DIR).map(PathBuf::from);
        let env_cache = std::env::var_os(ENV_CACHE_DIR).map(PathBuf::from);
        Self::resolve_with(env_data, env_cache)
    }

    /// Same as [`Paths::resolve`], with explicit overrides (used by tests).
    ///
    /// # Errors
    /// [`ConfigError::NoHomeDirectory`] when no override is given and no home directory exists.
    pub fn resolve_with(
        data_override: Option<PathBuf>,
        cache_override: Option<PathBuf>,
    ) -> Result<Self, ConfigError> {
        let project = directories::ProjectDirs::from("dev", "walktunes", "WalkTunes");
        let data_dir = match (data_override.clone(), &project) {
            (Some(p), _) => p,
            (None, Some(pd)) => pd.data_dir().to_path_buf(),
            (None, None) => return Err(ConfigError::NoHomeDirectory),
        };
        let cache_dir = match (cache_override, data_override, &project) {
            (Some(p), _, _) => p,
            // A data override without a cache override keeps everything together (throwaway profiles).
            (None, Some(data), _) => data.join("cache"),
            (None, None, Some(pd)) => pd.cache_dir().to_path_buf(),
            (None, None, None) => return Err(ConfigError::NoHomeDirectory),
        };
        let log_dir = data_dir.join("logs");
        Ok(Self {
            data_dir,
            cache_dir,
            log_dir,
        })
    }

    /// Creates the directories if they do not exist.
    ///
    /// # Errors
    /// [`ConfigError::DataDirNotWritable`] if a directory cannot be created.
    pub fn ensure(&self) -> Result<(), ConfigError> {
        for dir in [&self.data_dir, &self.cache_dir, &self.log_dir] {
            fs::create_dir_all(dir).map_err(|source| ConfigError::DataDirNotWritable {
                path: dir.clone(),
                source,
            })?;
        }
        Ok(())
    }
}

/// Returns this install's `MachineId`, creating and storing it on first use.
///
/// # Errors
/// [`ConfigError::DataDirNotWritable`] if the ID file cannot be written, or
/// [`ConfigError::SettingsUnreadable`] if it exists but cannot be read.
pub fn machine_id(data_dir: &Path) -> Result<MachineId, ConfigError> {
    let path = data_dir.join(MACHINE_ID_FILE);
    match fs::read_to_string(&path) {
        Ok(text) => {
            if let Ok(id) = text.parse::<MachineId>() {
                return Ok(id);
            }
            // A corrupt file: keep it for inspection and create a new identity.
            let _ = fs::rename(&path, path.with_extension("bad"));
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(source) => return Err(ConfigError::SettingsUnreadable { path, source }),
    }
    fs::create_dir_all(data_dir).map_err(|source| ConfigError::DataDirNotWritable {
        path: data_dir.to_path_buf(),
        source,
    })?;
    let id = MachineId::new();
    fs::write(&path, id.to_string())
        .map_err(|source| ConfigError::DataDirNotWritable { path, source })?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_override_also_moves_cache_and_logs() {
        let paths = Paths::resolve_with(Some(PathBuf::from("/tmp/wt")), None).unwrap();
        assert_eq!(paths.data_dir, PathBuf::from("/tmp/wt"));
        assert_eq!(paths.cache_dir, PathBuf::from("/tmp/wt/cache"));
        assert_eq!(paths.log_dir, PathBuf::from("/tmp/wt/logs"));
    }

    #[test]
    fn machine_id_is_stable_across_calls() {
        let dir = tempfile::tempdir().unwrap();
        let first = machine_id(dir.path()).unwrap();
        let second = machine_id(dir.path()).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn corrupt_machine_id_is_replaced_and_kept_as_bad() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(MACHINE_ID_FILE), "not-a-uuid").unwrap();
        let id = machine_id(dir.path()).unwrap();
        assert!(dir.path().join("machine_id.bad").exists());
        assert_eq!(machine_id(dir.path()).unwrap(), id);
    }

    #[test]
    fn error_codes_match_registry() {
        assert_eq!(
            ConfigError::NoHomeDirectory.code().to_string(),
            "WT-CONFIG-0002"
        );
    }
}

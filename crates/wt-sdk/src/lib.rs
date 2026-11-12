// SPDX-License-Identifier: MIT OR Apache-2.0
//! **wt-sdk** — the single facade every front end calls (AWS SDK;
//! `docs/ARCHITECTURE.md` §3.15). The console and the CLI depend on this crate only.
//!
//! Scope so far: system info, profiles, device probing (M0) and library roots on top of
//! the Ledger and Vault (sprint P1 Day 1).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use thiserror::Error;
use wt_bus::{Bus, WtEvent};
use wt_config::{ConfigError, Paths};
use wt_core::{ErrorCode, MachineId, Recovery, Severity, WtError};
use wt_dock::{DockError, Profile, ProfileSet};
use wt_dsp::SimdPath;
use wt_ledger::{Ledger, LedgerError};
use wt_vault::{Vault, VaultError};

pub use wt_ledger::{Root, RootKind};
pub use wt_vault::RecoveryReport;

pub use wt_dock::{
    DetectedDevice, DetectedVolume, DeviceCapability, DeviceFingerprint, MatchStatus, UsbSummary,
    VolumeRole,
};

/// Errors surfaced by the SDK; each wraps a service error and keeps its code.
#[derive(Debug, Error)]
pub enum SdkError {
    /// Configuration or paths failed.
    #[error(transparent)]
    Config(#[from] ConfigError),
    /// Device layer failed.
    #[error(transparent)]
    Dock(#[from] DockError),
    /// The Ledger failed.
    #[error(transparent)]
    Ledger(#[from] LedgerError),
    /// A file operation or root check failed.
    #[error(transparent)]
    Vault(#[from] VaultError),
}

macro_rules! delegate {
    ($self:ident, $m:ident) => {
        match $self {
            Self::Config(e) => e.$m(),
            Self::Dock(e) => e.$m(),
            Self::Ledger(e) => e.$m(),
            Self::Vault(e) => e.$m(),
        }
    };
}

impl WtError for SdkError {
    fn code(&self) -> ErrorCode {
        delegate!(self, code)
    }

    fn severity(&self) -> Severity {
        delegate!(self, severity)
    }

    fn recovery(&self) -> Recovery {
        delegate!(self, recovery)
    }
}

/// Facts about this installation, shown by `wtunes info`.
#[derive(Clone, Debug, Serialize)]
pub struct SystemInfo {
    /// WalkTunes version.
    pub version: &'static str,
    /// OS family (`macos`, `windows`, `linux`).
    pub os: &'static str,
    /// CPU architecture (`aarch64`, `x86_64`, `riscv64`).
    pub arch: &'static str,
    /// SIMD path used by DSP kernels.
    pub simd: SimdPath,
    /// This install's identity.
    pub machine_id: MachineId,
    /// Data directory.
    pub data_dir: PathBuf,
    /// Cache directory.
    pub cache_dir: PathBuf,
    /// Log directory.
    pub log_dir: PathBuf,
    /// The Ledger database file.
    pub ledger_path: PathBuf,
    /// Ledger schema version.
    pub schema_version: u32,
}

/// An open WalkTunes session.
pub struct Sdk {
    paths: Paths,
    machine_id: MachineId,
    bus: Bus,
    profiles: ProfileSet,
    ledger: Arc<Ledger>,
    vault: Vault,
    recovery: RecoveryReport,
}

impl Sdk {
    /// Opens a session: resolves paths, creates them, loads the machine id and profiles,
    /// opens (and migrates) the Ledger and recovers interrupted file operations.
    ///
    /// # Errors
    /// Configuration errors (`WT-CONFIG-*`), an invalid built-in profile, a Ledger that
    /// cannot be opened (`WT-LEDGER-*`) or a failed recovery.
    pub fn open() -> Result<Self, SdkError> {
        let paths = Paths::resolve()?;
        paths.ensure()?;
        let machine_id = wt_config::machine_id(&paths.data_dir)?;
        let profiles = ProfileSet::builtin()?;
        let ledger = Arc::new(Ledger::open(&paths.data_dir.join(wt_ledger::LEDGER_FILE))?);
        let vault = Vault::new(ledger.clone());
        let recovery = vault.recover()?;
        if recovery.interrupted > 0 {
            tracing::warn!(
                interrupted = recovery.interrupted,
                temp_files_removed = recovery.temp_files_removed,
                "recovered interrupted file operations"
            );
        }
        Ok(Self {
            paths,
            machine_id,
            bus: Bus::new(),
            profiles,
            ledger,
            vault,
            recovery,
        })
    }

    /// What startup recovery cleaned up (all zero after a clean shutdown).
    #[must_use]
    pub fn recovery(&self) -> RecoveryReport {
        self.recovery
    }

    /// Library roots (`sdk.roots()`).
    #[must_use]
    pub fn roots(&self) -> Roots<'_> {
        Roots { sdk: self }
    }

    /// The file-mutation service (used by later services: sync, forge).
    #[must_use]
    pub fn vault(&self) -> &Vault {
        &self.vault
    }

    /// The event bus (subscribe to device and job events).
    #[must_use]
    pub fn bus(&self) -> &Bus {
        &self.bus
    }

    /// Facts about this installation.
    ///
    /// # Errors
    /// The Ledger cannot report its schema version.
    pub fn system_info(&self) -> Result<SystemInfo, SdkError> {
        Ok(SystemInfo {
            version: env!("CARGO_PKG_VERSION"),
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            simd: wt_dsp::simd_path(),
            machine_id: self.machine_id,
            data_dir: self.paths.data_dir.clone(),
            cache_dir: self.paths.cache_dir.clone(),
            log_dir: self.paths.log_dir.clone(),
            ledger_path: self
                .ledger
                .path()
                .map(Path::to_path_buf)
                .unwrap_or_default(),
            schema_version: self.ledger.schema_version()?,
        })
    }

    /// All built-in device profiles (abstract bases included).
    #[must_use]
    pub fn profiles(&self) -> &[Profile] {
        self.profiles.all()
    }

    /// Device operations.
    #[must_use]
    pub fn devices(&self) -> Devices<'_> {
        Devices { sdk: self }
    }
}

/// Device operations (`sdk.devices()`).
pub struct Devices<'a> {
    sdk: &'a Sdk,
}

impl Devices<'_> {
    /// Probes mounted volumes (read-only) and matches them to profiles.
    /// Publishes `DeviceAttached` for every non-generic device found.
    #[must_use]
    pub fn probe(&self) -> Vec<DetectedDevice> {
        let fingerprints = self.fingerprints();
        let devices = wt_dock::detect(&fingerprints, &self.sdk.profiles);
        for d in devices.iter().filter(|d| d.status != MatchStatus::Generic) {
            self.sdk.bus.publish(&WtEvent::DeviceAttached {
                device: d.device_id,
                name: d.display_name.clone(),
            });
        }
        devices
    }

    /// The raw passive fingerprints of mounted volumes (FR-056 probe dump).
    #[must_use]
    pub fn fingerprints(&self) -> Vec<DeviceFingerprint> {
        let backend = wt_dock::default_backend(&self.sdk.profiles);
        tracing::debug!(backend = backend.name(), "probing volumes");
        backend.probe()
    }
}

/// The result of [`Roots::add`].
#[derive(Clone, Debug, Serialize)]
pub struct RootAdded {
    /// The registered root.
    pub root: Root,
    /// `false` when the folder was already registered (adding is idempotent).
    pub created: bool,
}

/// Library roots (`sdk.roots()`): folders whose music WalkTunes catalogs.
pub struct Roots<'a> {
    sdk: &'a Sdk,
}

impl Roots<'_> {
    /// Registers a folder. The path is validated and canonicalized by Vault; adding the
    /// same folder twice returns the existing root.
    ///
    /// # Errors
    /// `WT-VAULT-0001` missing or not a folder, `WT-VAULT-0002` unreadable,
    /// `WT-VAULT-0008` inside or around an existing root, or a Ledger error.
    pub fn add(&self, path: &Path, kind: RootKind) -> Result<RootAdded, SdkError> {
        let canon = wt_vault::check_root(path)?;
        let roots = self.sdk.ledger.roots();
        if let Some(root) = roots.by_path(&canon)? {
            return Ok(RootAdded {
                root,
                created: false,
            });
        }
        let existing = roots.list()?;
        wt_vault::ensure_no_overlap(&canon, existing.iter().map(|r| r.path.as_path()))?;
        let root = roots.add(&canon, kind)?;
        tracing::info!(root = %root.id, path = %root.path.display(), kind = %kind, "root added");
        Ok(RootAdded {
            root,
            created: true,
        })
    }

    /// Every registered root, oldest first.
    ///
    /// # Errors
    /// A Ledger error.
    pub fn list(&self) -> Result<Vec<Root>, SdkError> {
        Ok(self.sdk.ledger.roots().list()?)
    }

    /// Unregisters a root given by id or path. Only catalog rows are removed; no file is
    /// touched (S-2). Returns the removed root.
    ///
    /// # Errors
    /// `WT-VAULT-0001` if no such root is registered, or a Ledger error.
    pub fn remove(&self, id_or_path: &str) -> Result<Root, SdkError> {
        let roots = self.sdk.ledger.roots();
        let found = match id_or_path.parse() {
            Ok(id) => roots.get(id)?,
            Err(_) => {
                let p = Path::new(id_or_path);
                let canon = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
                match roots.by_path(&canon)? {
                    Some(r) => Some(r),
                    None => roots.by_path(p)?,
                }
            }
        };
        let root = found.ok_or_else(|| VaultError::RootNotFound {
            path: PathBuf::from(id_or_path),
        })?;
        roots.remove(root.id)?;
        tracing::info!(root = %root.id, path = %root.path.display(), "root removed");
        Ok(root)
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)] // tests build temp folders
mod tests {
    use super::*;

    fn sdk_in(dir: &Path) -> Sdk {
        let paths = Paths::resolve_with(Some(dir.to_path_buf()), None).unwrap();
        paths.ensure().unwrap();
        let ledger = Arc::new(Ledger::open(&paths.data_dir.join(wt_ledger::LEDGER_FILE)).unwrap());
        let vault = Vault::new(ledger.clone());
        Sdk {
            machine_id: wt_config::machine_id(&paths.data_dir).unwrap(),
            paths,
            bus: Bus::new(),
            profiles: ProfileSet::builtin().unwrap(),
            recovery: vault.recover().unwrap(),
            ledger,
            vault,
        }
    }

    #[test]
    fn roots_add_is_idempotent_and_rejects_overlaps() {
        let data = tempfile::tempdir().unwrap();
        let music = tempfile::tempdir().unwrap();
        let rips = music.path().join("Rips");
        std::fs::create_dir(&rips).unwrap();
        let sdk = sdk_in(data.path());

        let first = sdk.roots().add(music.path(), RootKind::Library).unwrap();
        assert!(first.created);
        let again = sdk.roots().add(music.path(), RootKind::Library).unwrap();
        assert!(!again.created);
        assert_eq!(again.root.id, first.root.id);

        let err = sdk.roots().add(&rips, RootKind::Library).unwrap_err();
        assert_eq!(err.code().to_string(), "WT-VAULT-0008");

        let removed = sdk.roots().remove(&first.root.id.to_string()).unwrap();
        assert_eq!(removed.id, first.root.id);
        assert!(music.path().exists(), "removing a root never touches files");
        assert!(sdk.roots().list().unwrap().is_empty());
        assert_eq!(sdk.system_info().unwrap().schema_version, 1);
    }
}

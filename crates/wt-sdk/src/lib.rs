// SPDX-License-Identifier: MIT OR Apache-2.0
//! **wt-sdk** — the single facade every front end calls (AWS SDK;
//! `docs/ARCHITECTURE.md` §3.15). The console and the CLI depend on this crate only.
//!
//! Skeleton scope (M0): system info, profiles and device probing.

use std::path::PathBuf;

use serde::Serialize;
use thiserror::Error;
use wt_bus::{Bus, WtEvent};
use wt_config::{ConfigError, Paths};
use wt_core::{ErrorCode, MachineId, Recovery, Severity, WtError};
use wt_dock::{DockError, Profile, ProfileSet};
use wt_dsp::SimdPath;

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
}

impl WtError for SdkError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Config(e) => e.code(),
            Self::Dock(e) => e.code(),
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Self::Config(e) => e.severity(),
            Self::Dock(e) => e.severity(),
        }
    }

    fn recovery(&self) -> Recovery {
        match self {
            Self::Config(e) => e.recovery(),
            Self::Dock(e) => e.recovery(),
        }
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
}

/// An open WalkTunes session.
pub struct Sdk {
    paths: Paths,
    machine_id: MachineId,
    bus: Bus,
    profiles: ProfileSet,
}

impl Sdk {
    /// Opens a session: resolves paths, creates them, loads the machine id and profiles.
    ///
    /// # Errors
    /// Configuration errors (`WT-CONFIG-*`) or an invalid built-in profile.
    pub fn open() -> Result<Self, SdkError> {
        let paths = Paths::resolve()?;
        paths.ensure()?;
        let machine_id = wt_config::machine_id(&paths.data_dir)?;
        let profiles = ProfileSet::builtin()?;
        Ok(Self {
            paths,
            machine_id,
            bus: Bus::new(),
            profiles,
        })
    }

    /// The event bus (subscribe to device and job events).
    #[must_use]
    pub fn bus(&self) -> &Bus {
        &self.bus
    }

    /// Facts about this installation.
    #[must_use]
    pub fn system_info(&self) -> SystemInfo {
        SystemInfo {
            version: env!("CARGO_PKG_VERSION"),
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            simd: wt_dsp::simd_path(),
            machine_id: self.machine_id,
            data_dir: self.paths.data_dir.clone(),
            cache_dir: self.paths.cache_dir.clone(),
            log_dir: self.paths.log_dir.clone(),
        }
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

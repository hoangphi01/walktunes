// SPDX-License-Identifier: GPL-3.0-or-later
//! **wt-dock** — device detection and pairing (IoT Core; `docs/ARCHITECTURE.md` §3.13).
//!
//! Skeleton scope (M0):
//! - [`DeviceFingerprint`]: the passive identity of a mounted volume (ADR-0014).
//! - [`Profile`]s embedded from `profiles/*.toml` (ZX300, A30/A40/A50, WM1, generic).
//! - [`detect`]: scoring per `docs/EXTENSION.md` §3.1.1, grouping volumes into devices.
//! - [`DeviceCapability`]: Sony's `default-capability.xml` self-description (R-003).
//! - Probe backends: [`FakeBackend`] (tests, `WT_FAKE_VOLUMES`), [`SysinfoBackend`]
//!   (all OSes) with, on macOS, `ioreg` (USB/SCSI identity) and `diskutil` (volume facts).
//!
//! The probe is **read-only**: it never sends SCSI or USB commands and never writes
//! to a device (SECURITY S-10). Pairing and manifests arrive in M2.

mod backend;
mod capability;
mod detect;
mod error;
mod fingerprint;
mod ioreg;
mod paired;
mod profile;

pub use backend::{FakeBackend, ProbeBackend, SysinfoBackend, default_backend};
pub use capability::{AudioFormatCap, CAPABILITY_FILE, DeviceCapability, FileFormatCap};
pub use detect::{DetectedDevice, DetectedVolume, MatchStatus, UsbSummary, VolumeRole, detect};
pub use error::DockError;
pub use fingerprint::{DeviceFingerprint, OS_LITTER};
pub use ioreg::UsbMedia;
pub use paired::PairedInfo;
pub use profile::{DetectRules, ModelSpec, Profile, ProfileSet};

/// Environment variable listing folders to treat as mounted volumes (tests, demos).
pub const ENV_FAKE_VOLUMES: &str = "WT_FAKE_VOLUMES";
/// Folder WalkTunes owns on every paired volume.
pub const WALKTUNES_DIR: &str = ".walktunes";

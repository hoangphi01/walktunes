// SPDX-License-Identifier: GPL-3.0-or-later
//! The passive identity of one mounted volume (`docs/ARCHITECTURE.md` §3.13).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::capability::{CAPABILITY_FILE, DeviceCapability};
use crate::paired::PairedInfo;

/// Folders that operating systems leave on removable volumes. Detected (never removed)
/// so WalkTunes can tell the user and avoid syncing them.
pub const OS_LITTER: &[&str] = &[
    ".Spotlight-V100",
    ".fseventsd",
    ".Trashes",
    ".TemporaryItems",
    "System Volume Information",
    "$RECYCLE.BIN",
];

/// Everything the OS already knows about a mounted volume. No device commands are sent
/// to collect it (ADR-0014). Every field is optional because backends differ per OS.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceFingerprint {
    /// Where the volume is mounted.
    pub mount_point: PathBuf,
    /// OS block device name (`disk5s1` on macOS).
    pub bsd_name: Option<String>,
    /// Whole-disk block device (`disk5`).
    pub parent_disk: Option<String>,
    /// USB vendor ID (Sony = `0x054C`).
    pub usb_vid: Option<u16>,
    /// USB product ID (NW-ZX300 = `0x0C70`).
    pub usb_pid: Option<u16>,
    /// USB `iSerialNumber`; shared by all volumes (LUNs) of one device.
    pub usb_serial: Option<String>,
    /// USB `iManufacturer`.
    pub usb_manufacturer: Option<String>,
    /// USB `iProduct`.
    pub usb_product: Option<String>,
    /// USB `bcdDevice`.
    pub usb_bcd_device: Option<u16>,
    /// USB location ID (port path), useful to tell two identical players apart.
    pub usb_location_id: Option<u32>,
    /// Negotiated USB link speed in Mbit/s (480 = USB 2.0 High Speed).
    pub usb_speed_mbps: Option<u32>,
    /// SCSI INQUIRY vendor (e.g. `SONY`).
    pub scsi_vendor: Option<String>,
    /// SCSI INQUIRY product.
    pub scsi_product: Option<String>,
    /// SCSI INQUIRY revision (`1.00` on the ZX300: **not** the firmware version).
    pub scsi_revision: Option<String>,
    /// SCSI logical unit number of this volume.
    pub lun: Option<u8>,
    /// Volume label.
    pub volume_label: Option<String>,
    /// File-system UUID or serial.
    pub volume_uuid: Option<String>,
    /// File-system type (`msdos`, `exfat`, `vfat`, …).
    pub fs_type: Option<String>,
    /// Human-readable file-system name (`MS-DOS FAT32`, `ExFAT`).
    pub fs_name: Option<String>,
    /// Total size in bytes.
    pub capacity: u64,
    /// Free space in bytes.
    pub free: Option<u64>,
    /// Whether the volume is mounted writable.
    pub writable: Option<bool>,
    /// Known names present at the volume root (e.g. `MUSIC`, `default-capability.xml`).
    pub root_markers: Vec<String>,
    /// OS metadata folders found at the root (Spotlight, Trashes, …).
    pub os_litter: Vec<String>,
    /// Sony's self-description (`default-capability.xml`), if present.
    pub capability: Option<DeviceCapability>,
    /// Pairing data read from `.walktunes/device.json`, if present.
    pub paired: Option<PairedInfo>,
    /// Which backend(s) produced this fingerprint (diagnostics).
    pub source: String,
}

impl DeviceFingerprint {
    /// Whether a marker (file or folder name at the volume root) is present.
    #[must_use]
    pub fn has_marker(&self, name: &str) -> bool {
        self.root_markers
            .iter()
            .any(|m| m.eq_ignore_ascii_case(name))
    }

    /// Capacity in decimal gigabytes, as printed on the box.
    #[must_use]
    pub fn capacity_gb(&self) -> f64 {
        // Precision loss is irrelevant at this scale.
        #[allow(clippy::cast_precision_loss)]
        let bytes = self.capacity as f64;
        bytes / 1e9
    }

    /// Whether the file system is FAT32 (4 GiB per-file limit).
    #[must_use]
    pub fn is_fat32(&self) -> bool {
        let t = self
            .fs_type
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let n = self
            .fs_name
            .as_deref()
            .unwrap_or_default()
            .to_ascii_uppercase();
        n.contains("FAT32")
            || (matches!(t.as_str(), "msdos" | "vfat" | "fat32") && !n.contains("FAT16"))
    }

    /// Checks a fixed list of names at the volume root and reads the two small files
    /// WalkTunes understands (`default-capability.xml`, `.walktunes/device.json`).
    ///
    /// Nothing else on the volume is listed or read (privacy and speed), and nothing is written.
    pub fn read_root_markers(&mut self, names: &[&str]) {
        let root = self.mount_point.clone();
        self.root_markers = names
            .iter()
            .filter(|n| root.join(n).exists())
            .map(|n| (*n).to_owned())
            .collect();
        self.os_litter = OS_LITTER
            .iter()
            .filter(|n| root.join(n).exists())
            .map(|n| (*n).to_owned())
            .collect();
        self.capability = std::fs::read(root.join(CAPABILITY_FILE))
            .ok()
            .and_then(|b| DeviceCapability::parse(&b));
        if self.capability.is_some() && !self.has_marker(CAPABILITY_FILE) {
            self.root_markers.push(CAPABILITY_FILE.to_owned());
        }
        self.paired = PairedInfo::read(&root);
        if self.paired.is_some() && !self.has_marker(crate::WALKTUNES_DIR) {
            self.root_markers.push(crate::WALKTUNES_DIR.to_owned());
        }
    }

    /// Short label for logs and the CLI.
    #[must_use]
    pub fn display_name(&self) -> String {
        self.volume_label
            .clone()
            .unwrap_or_else(|| display_path(&self.mount_point))
    }

    /// A copy safe to share publicly: the USB serial and volume UUID are reduced to a
    /// short tag (last 4 characters), as asked by EXTENSION §3.5.
    #[must_use]
    pub fn redacted(&self) -> Self {
        let tag = |s: &Option<String>| {
            s.as_ref().map(|v| {
                let tail: String = v
                    .chars()
                    .rev()
                    .take(4)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                format!("REDACTED…{tail}")
            })
        };
        let mut c = self.clone();
        c.usb_serial = tag(&self.usb_serial);
        c.volume_uuid = tag(&self.volume_uuid);
        if let Some(p) = c.paired.as_mut() {
            p.usb_serial = tag(&p.usb_serial);
        }
        c
    }
}

fn display_path(p: &Path) -> String {
    p.file_name().map_or_else(
        || p.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

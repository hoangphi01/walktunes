// SPDX-License-Identifier: GPL-3.0-or-later
//! The passive identity of one mounted volume (`docs/ARCHITECTURE.md` §3.13).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::paired::PairedInfo;

/// Everything the OS already knows about a mounted volume. No device commands are sent
/// to collect it (ADR-0014). Every field is optional because backends differ per OS.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceFingerprint {
    /// Where the volume is mounted.
    pub mount_point: PathBuf,
    /// USB vendor ID (Sony = `0x054C`).
    pub usb_vid: Option<u16>,
    /// USB product ID.
    pub usb_pid: Option<u16>,
    /// USB `iSerialNumber`; shared by all volumes (LUNs) of one device.
    pub usb_serial: Option<String>,
    /// USB `iManufacturer`.
    pub usb_manufacturer: Option<String>,
    /// USB `iProduct`.
    pub usb_product: Option<String>,
    /// SCSI INQUIRY vendor (e.g. `SONY`).
    pub scsi_vendor: Option<String>,
    /// SCSI INQUIRY product.
    pub scsi_product: Option<String>,
    /// SCSI INQUIRY revision (≈ firmware version on Sony players `[VERIFY]`).
    pub scsi_revision: Option<String>,
    /// SCSI logical unit number of this volume.
    pub lun: Option<u8>,
    /// Volume label.
    pub volume_label: Option<String>,
    /// File-system UUID or serial.
    pub volume_uuid: Option<String>,
    /// File-system type (exfat, msdos/vfat, …).
    pub fs_type: Option<String>,
    /// Total size in bytes.
    pub capacity: u64,
    /// Known names present at the volume root (e.g. `MUSIC`, `NW_WM_FW.UPG`, `.walktunes`).
    pub root_markers: Vec<String>,
    /// Pairing data read from `.walktunes/device.json`, if present.
    pub paired: Option<PairedInfo>,
    /// Which backend produced this fingerprint (diagnostics).
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

    /// Fills `root_markers` and `paired` by checking a fixed list of names at the root.
    ///
    /// Only the listed names are checked; the volume is never listed or read further
    /// (privacy and speed).
    pub fn read_root_markers(&mut self, names: &[&str]) {
        let root = self.mount_point.clone();
        self.root_markers = names
            .iter()
            .filter(|n| root.join(n).exists())
            .map(|n| (*n).to_owned())
            .collect();
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
}

fn display_path(p: &Path) -> String {
    p.file_name().map_or_else(
        || p.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

// SPDX-License-Identifier: MIT OR Apache-2.0
//! Reading `.walktunes/device.json` (`docs/DATABASE.md` §6.1). Writing it is M2.

use std::path::Path;

use serde::{Deserialize, Serialize};
use wt_core::DeviceId;

/// The pairing record stored on each volume of a paired device.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairedInfo {
    /// Format marker, must be `walktunes/device`.
    pub format: String,
    /// Format version.
    pub version: u32,
    /// The device's identity.
    pub device_id: DeviceId,
    /// User-chosen name, e.g. "My ZX300".
    pub name: String,
    /// Profile id, e.g. `sony.classic.zx300`.
    pub profile_id: String,
    /// Volume role on the device: `internal`, `sd` or `single`.
    pub volume_role: String,
    /// Exact model, if known, e.g. `NW-ZX300`.
    #[serde(default)]
    pub model: Option<String>,
    /// USB serial recorded at pairing time.
    #[serde(default)]
    pub usb_serial: Option<String>,
    /// INQUIRY revision recorded at pairing time.
    #[serde(default)]
    pub firmware_rev: Option<String>,
}

impl PairedInfo {
    /// Expected value of [`PairedInfo::format`].
    pub const FORMAT: &'static str = "walktunes/device";

    /// Reads and validates `<volume>/.walktunes/device.json`. Returns `None` if absent or
    /// invalid; a broken pairing file must never block detection.
    #[must_use]
    pub fn read(volume_root: &Path) -> Option<Self> {
        let path = volume_root.join(crate::WALKTUNES_DIR).join("device.json");
        let text = std::fs::read_to_string(path).ok()?;
        let info: Self = serde_json::from_str(&text).ok()?;
        (info.format == Self::FORMAT).then_some(info)
    }
}

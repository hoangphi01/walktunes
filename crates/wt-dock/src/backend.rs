// SPDX-License-Identifier: GPL-3.0-or-later
//! Probe backends: produce one [`DeviceFingerprint`] per mounted volume.
//!
//! All backends are passive and read-only (SECURITY S-10). The full USB/SCSI
//! backends (macOS IOKit + DiskArbitration, Windows SetupAPI, Linux sysfs) are M2;
//! until then macOS gets an interim enrichment from `diskutil info -plist`.

use std::path::{Path, PathBuf};

use crate::fingerprint::DeviceFingerprint;
use crate::profile::ProfileSet;

/// Something that can list fingerprints of the currently mounted volumes.
pub trait ProbeBackend {
    /// Short backend name for diagnostics.
    fn name(&self) -> &'static str;
    /// Fingerprints of mounted volumes that could be players.
    fn probe(&self) -> Vec<DeviceFingerprint>;
}

/// The names checked at each volume root: profile folders and protected files.
#[must_use]
pub fn marker_names(profiles: &ProfileSet) -> Vec<String> {
    let mut names: Vec<String> = profiles
        .all()
        .iter()
        .flat_map(|p| {
            let d = &p.detect;
            d.required_dirs
                .iter()
                .chain(d.protected_root_files.iter())
                .flatten()
                .cloned()
                .chain(p.layout.music_dir.clone())
        })
        .collect();
    names.push(crate::WALKTUNES_DIR.to_owned());
    names.sort();
    names.dedup();
    names
}

/// Picks the backend for this run: `WT_FAKE_VOLUMES` if set, else the system backend.
#[must_use]
pub fn default_backend(profiles: &ProfileSet) -> Box<dyn ProbeBackend> {
    let markers = marker_names(profiles);
    match std::env::var(crate::ENV_FAKE_VOLUMES) {
        Ok(list) if !list.trim().is_empty() => Box::new(FakeBackend::from_list(&list, markers)),
        _ => Box::new(SysinfoBackend::new(markers)),
    }
}

// ---------------------------------------------------------------------------
// Fake backend (tests, demos, virtual devices: docs/TESTING.md §4)
// ---------------------------------------------------------------------------

/// Treats plain folders as mounted volumes. A `fingerprint.json` in a folder supplies
/// the USB/SCSI identity that real hardware would report.
#[derive(Clone, Debug)]
pub struct FakeBackend {
    folders: Vec<PathBuf>,
    markers: Vec<String>,
}

impl FakeBackend {
    /// Creates a backend for the given folders.
    #[must_use]
    pub fn new(folders: Vec<PathBuf>, markers: Vec<String>) -> Self {
        Self { folders, markers }
    }

    /// Parses a comma-separated list (the `WT_FAKE_VOLUMES` format).
    #[must_use]
    pub fn from_list(list: &str, markers: Vec<String>) -> Self {
        let folders = list
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .collect();
        Self::new(folders, markers)
    }
}

impl ProbeBackend for FakeBackend {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn probe(&self) -> Vec<DeviceFingerprint> {
        let names: Vec<&str> = self.markers.iter().map(String::as_str).collect();
        self.folders
            .iter()
            .filter(|f| f.is_dir())
            .map(|folder| {
                let mut fp = std::fs::read_to_string(folder.join("fingerprint.json"))
                    .ok()
                    .and_then(|t| serde_json::from_str::<DeviceFingerprint>(&t).ok())
                    .unwrap_or_default();
                fp.mount_point.clone_from(folder);
                if fp.volume_label.is_none() {
                    fp.volume_label = folder.file_name().map(|n| n.to_string_lossy().into_owned());
                }
                fp.read_root_markers(&names);
                fp.source = "fake".into();
                fp
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// System backend (all OSes): mounted removable volumes + weak evidence
// ---------------------------------------------------------------------------

/// Lists mounted, removable or external volumes with `sysinfo`, then enriches them with
/// whatever passive identity the platform offers (macOS: `diskutil info -plist`).
#[derive(Clone, Debug)]
pub struct SysinfoBackend {
    markers: Vec<String>,
}

impl SysinfoBackend {
    /// Creates the backend.
    #[must_use]
    pub fn new(markers: Vec<String>) -> Self {
        Self { markers }
    }
}

fn looks_external(mount: &Path, removable: bool) -> bool {
    if removable {
        return true;
    }
    if cfg!(target_os = "macos") {
        // /Volumes/* except the boot volume alias and system volumes.
        return mount.starts_with("/Volumes") && mount != Path::new("/Volumes/Macintosh HD");
    }
    if cfg!(target_os = "linux") {
        return mount.starts_with("/media")
            || mount.starts_with("/run/media")
            || mount.starts_with("/mnt");
    }
    false
}

impl ProbeBackend for SysinfoBackend {
    fn name(&self) -> &'static str {
        "sysinfo"
    }

    fn probe(&self) -> Vec<DeviceFingerprint> {
        let names: Vec<&str> = self.markers.iter().map(String::as_str).collect();
        let disks = match sysinfo::Disks::new_with_refreshed_list() {
            Ok(d) => d,
            Err(e) => {
                tracing::info!(code = "WT-DOCK-0007", error = %e, "volume listing unavailable");
                return Vec::new();
            }
        };
        disks
            .iter()
            .filter(|d| looks_external(d.mount_point(), d.is_removable()))
            .map(|d| {
                let label = d.name().to_string_lossy().into_owned();
                let mut fp = DeviceFingerprint {
                    mount_point: d.mount_point().to_path_buf(),
                    volume_label: (!label.is_empty()).then_some(label),
                    fs_type: Some(d.file_system().to_string_lossy().into_owned()),
                    capacity: d.total_space(),
                    source: "sysinfo".into(),
                    ..DeviceFingerprint::default()
                };
                #[cfg(target_os = "macos")]
                macos::enrich(&mut fp);
                fp.read_root_markers(&names);
                fp
            })
            .collect()
    }
}

#[cfg(target_os = "macos")]
mod macos {
    //! Interim macOS enrichment: `diskutil info -plist <mount>` (read-only, unprivileged).
    //! Replaced in M2 by DiskArbitration + IOKit, which also yield VID/PID/serial (ADR-0014).

    use std::process::Command;

    use crate::fingerprint::DeviceFingerprint;

    pub(super) fn enrich(fp: &mut DeviceFingerprint) {
        let Ok(out) = Command::new("/usr/sbin/diskutil")
            .arg("info")
            .arg("-plist")
            .arg(&fp.mount_point)
            .output()
        else {
            tracing::info!(code = "WT-DOCK-0007", mount = %fp.mount_point.display(), "diskutil unavailable");
            return;
        };
        if !out.status.success() {
            return;
        }
        let Ok(plist::Value::Dictionary(dict)) =
            plist::Value::from_reader(std::io::Cursor::new(out.stdout))
        else {
            return;
        };
        let text = |k: &str| {
            dict.get(k)
                .and_then(plist::Value::as_string)
                .map(str::to_owned)
        };

        // Only USB-attached media are candidates for SCSI identity.
        let bus = text("BusProtocol").unwrap_or_default();
        if let Some(uuid) = text("VolumeUUID") {
            fp.volume_uuid = Some(uuid);
        }
        if let Some(fs) = text("FilesystemType") {
            fp.fs_type = Some(fs);
        }
        if bus.eq_ignore_ascii_case("USB") {
            // MediaName is "<INQUIRY vendor> <INQUIRY product> Media" on macOS  [VERIFY on the ZX300].
            if let Some(media) = text("MediaName") {
                let media = media.trim_end_matches(" Media").trim().to_owned();
                let mut parts = media.splitn(2, ' ');
                fp.scsi_vendor = parts.next().map(str::to_owned).filter(|s| !s.is_empty());
                fp.scsi_product = parts
                    .next()
                    .map(str::trim)
                    .map(str::to_owned)
                    .filter(|s| !s.is_empty());
            }
            fp.source = "sysinfo+diskutil".into();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_include_music_protected_file_and_walktunes_dir() {
        let names = marker_names(&ProfileSet::builtin().unwrap());
        for n in ["MUSIC", "NW_WM_FW.UPG", ".walktunes"] {
            assert!(names.iter().any(|x| x == n), "{n}");
        }
    }

    #[test]
    fn fake_backend_reads_fingerprint_and_markers() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("MUSIC")).unwrap();
        std::fs::write(
            dir.path().join("fingerprint.json"),
            r#"{"usb_vid": 1356, "scsi_vendor": "SONY", "usb_serial": "X1", "capacity": 64000000000}"#,
        )
        .unwrap();
        let backend = FakeBackend::new(vec![dir.path().to_path_buf()], vec!["MUSIC".into()]);
        let fps = backend.probe();
        assert_eq!(fps.len(), 1);
        assert_eq!(fps[0].usb_vid, Some(0x054c));
        assert!(fps[0].has_marker("MUSIC"));
        assert_eq!(fps[0].mount_point, dir.path());
    }
}

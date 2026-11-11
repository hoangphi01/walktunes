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
                .chain(d.marker_files.iter())
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
        #[cfg(target_os = "macos")]
        let usb_media = macos::usb_media();
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
                    free: Some(d.available_space()),
                    writable: Some(!d.is_read_only()),
                    source: "sysinfo".into(),
                    ..DeviceFingerprint::default()
                };
                #[cfg(target_os = "macos")]
                macos::enrich(&mut fp, &usb_media);
                fp.read_root_markers(&names);
                fp
            })
            .collect()
    }
}

#[cfg(target_os = "macos")]
mod macos {
    //! Interim macOS backend (read-only, unprivileged subprocesses):
    //! - `ioreg -a -r -l -c IOUSBHostDevice` once per probe: USB + SCSI identity per BSD name;
    //! - `diskutil info -plist <mount>` per volume: BSD name, UUID, file system, sizes.
    //!
    //! Replaced in M2 by native DiskArbitration + IOKit calls (ADR-0014). Same data.

    use std::collections::BTreeMap;
    use std::process::Command;

    use crate::fingerprint::DeviceFingerprint;
    use crate::ioreg::{self, UsbMedia};

    fn plist_of(cmd: &str, args: &[&std::ffi::OsStr]) -> Option<plist::Value> {
        let out = Command::new(cmd).args(args).output().ok()?;
        if !out.status.success() {
            return None;
        }
        plist::Value::from_reader(std::io::Cursor::new(out.stdout)).ok()
    }

    pub(super) fn usb_media() -> BTreeMap<String, UsbMedia> {
        let args: Vec<&std::ffi::OsStr> = ["-a", "-r", "-l", "-w0", "-c", "IOUSBHostDevice"]
            .iter()
            .map(std::ffi::OsStr::new)
            .collect();
        match plist_of("/usr/sbin/ioreg", &args) {
            Some(v) => ioreg::parse(&v),
            None => {
                tracing::info!(code = "WT-DOCK-0007", "ioreg unavailable; no USB identity");
                BTreeMap::new()
            }
        }
    }

    pub(super) fn enrich(fp: &mut DeviceFingerprint, usb: &BTreeMap<String, UsbMedia>) {
        let args = [
            std::ffi::OsStr::new("info"),
            std::ffi::OsStr::new("-plist"),
            fp.mount_point.as_os_str(),
        ];
        let Some(plist::Value::Dictionary(d)) = plist_of("/usr/sbin/diskutil", &args) else {
            tracing::info!(code = "WT-DOCK-0007", mount = %fp.mount_point.display(), "diskutil unavailable");
            return;
        };
        let text = |k: &str| {
            d.get(k)
                .and_then(plist::Value::as_string)
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_owned)
        };
        let num = |k: &str| d.get(k).and_then(plist::Value::as_unsigned_integer);
        fp.bsd_name = text("DeviceIdentifier");
        fp.parent_disk = text("ParentWholeDisk");
        fp.volume_uuid = text("VolumeUUID").or(fp.volume_uuid.take());
        fp.fs_type = text("FilesystemType").or(fp.fs_type.take());
        fp.fs_name = text("FilesystemName");
        fp.volume_label = text("VolumeName").or(fp.volume_label.take());
        if let Some(total) = num("TotalSize") {
            fp.capacity = total;
        }
        fp.free = num("FreeSpace").or(fp.free);
        if let Some(w) = d.get("WritableVolume").and_then(plist::Value::as_boolean) {
            fp.writable = Some(w);
        }
        fp.source = "sysinfo+diskutil".into();

        // USB/SCSI identity: the partition first, then its whole disk.
        let media = fp
            .bsd_name
            .as_ref()
            .and_then(|b| usb.get(b))
            .or_else(|| fp.parent_disk.as_ref().and_then(|b| usb.get(b)));
        if let Some(m) = media {
            fp.usb_vid = m.usb_vid;
            fp.usb_pid = m.usb_pid;
            fp.usb_serial.clone_from(&m.usb_serial);
            fp.usb_manufacturer.clone_from(&m.usb_manufacturer);
            fp.usb_product.clone_from(&m.usb_product);
            fp.usb_bcd_device = m.usb_bcd_device;
            fp.usb_location_id = m.usb_location_id;
            fp.usb_speed_mbps = m.usb_speed_mbps;
            fp.scsi_vendor.clone_from(&m.scsi_vendor);
            fp.scsi_product.clone_from(&m.scsi_product);
            fp.scsi_revision.clone_from(&m.scsi_revision);
            fp.lun = m.lun;
            fp.source = "sysinfo+diskutil+ioreg".into();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_include_music_protected_file_and_walktunes_dir() {
        let names = marker_names(&ProfileSet::builtin().unwrap());
        for n in [
            "MUSIC",
            "NW_WM_FW.UPG",
            ".walktunes",
            "default-capability.xml",
            "DevIcon.fil",
        ] {
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

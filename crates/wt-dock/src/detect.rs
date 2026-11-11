// SPDX-License-Identifier: GPL-3.0-or-later
//! Scoring fingerprints against profiles and grouping volumes into devices
//! (`docs/EXTENSION.md` §3.1.1).

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Serialize;
use wt_core::{DeviceId, WtError};

use crate::capability::DeviceCapability;
use crate::error::DockError;
use crate::fingerprint::DeviceFingerprint;
use crate::profile::{Profile, ProfileSet};

/// Evidence weights (EXTENSION §3.1.1).
mod weight {
    pub const USB_VENDOR: u32 = 40;
    pub const SCSI_VENDOR: u32 = 25;
    pub const PRODUCT: u32 = 20;
    pub const USB_PRODUCT_ID: u32 = 30;
    pub const MODEL_NAME: u32 = 30;
    pub const CAPABILITY: u32 = 45;
    pub const CAPABILITY_MODEL: u32 = 40;
    pub const MARKER_FILES: u32 = 10;
    pub const LABEL: u32 = 5;
    pub const DIRS: u32 = 5;
    pub const PAIRED: u32 = 100;
}

/// Score at or above which a device is recognized without asking.
pub const RECOGNIZED: u32 = 85;
/// Score at or above which a profile is suggested in the pairing dialog.
pub const SUGGESTED: u32 = 50;

/// FAT32 cannot store files of 4 GiB or more.
const FAT32_MAX_FILE: u64 = 4 * 1024 * 1024 * 1024 - 1;

/// How confident the match is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchStatus {
    /// Paired before: `device.json` names the profile.
    Paired,
    /// Recognized automatically (score ≥ 85).
    Recognized,
    /// Suggested; the user confirms (score 50–84).
    Suggested,
    /// Unknown player; generic profile.
    Generic,
}

/// The role of a volume within a device.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VolumeRole {
    /// Built-in storage.
    Internal,
    /// Memory card.
    Sd,
    /// The only volume of a single-volume player.
    Single,
}

/// USB identity shared by a device's volumes.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct UsbSummary {
    /// Vendor ID.
    pub vid: Option<u16>,
    /// Product ID.
    pub pid: Option<u16>,
    /// `iManufacturer`.
    pub manufacturer: Option<String>,
    /// `iProduct`.
    pub product: Option<String>,
    /// `iSerialNumber`.
    pub serial: Option<String>,
    /// `bcdDevice`.
    pub bcd_device: Option<u16>,
    /// Port path.
    pub location_id: Option<u32>,
    /// Link speed in Mbit/s.
    pub speed_mbps: Option<u32>,
    /// SCSI INQUIRY vendor / product / revision.
    pub scsi: Option<[String; 3]>,
}

/// One volume of a detected device.
#[derive(Clone, Debug, Serialize)]
pub struct DetectedVolume {
    /// Mount point.
    pub mount_point: PathBuf,
    /// Role within the device.
    pub role: VolumeRole,
    /// Storage type declared by the device itself (`INTERNAL` / `CARD`).
    pub storage_type: Option<String>,
    /// Volume label.
    pub label: Option<String>,
    /// File-system type (`msdos`, `exfat`).
    pub fs_type: Option<String>,
    /// File-system name (`MS-DOS FAT32`).
    pub fs_name: Option<String>,
    /// Size in bytes.
    pub capacity: u64,
    /// Free space in bytes.
    pub free: Option<u64>,
    /// Largest file the file system accepts, in bytes (FAT32 = 4 GiB − 1).
    pub max_file_size: Option<u64>,
    /// Whether the volume is writable.
    pub writable: Option<bool>,
    /// SCSI LUN.
    pub lun: Option<u8>,
    /// OS block device (`disk4`).
    pub bsd_name: Option<String>,
    /// File-system UUID.
    pub volume_uuid: Option<String>,
    /// OS metadata folders found at the root.
    pub os_litter: Vec<String>,
}

/// A device made of one or more volumes, matched to a profile.
#[derive(Clone, Debug, Serialize)]
pub struct DetectedDevice {
    /// Confidence of the match.
    pub status: MatchStatus,
    /// Total score of the winning profile.
    pub score: u32,
    /// Winning profile id.
    pub profile_id: String,
    /// Family, e.g. `sony.classic`.
    pub family: String,
    /// Display name: the paired name, the model, or the family/profile name.
    pub display_name: String,
    /// Exact model when known (`NW-ZX300`).
    pub model: Option<String>,
    /// Model series reported by the device (`NW-ZX300Series`).
    pub model_series: Option<String>,
    /// When the model is not certain: likely models, best guess first (a pairing hint).
    pub candidates: Vec<String>,
    /// Paired device id, if any.
    pub device_id: Option<DeviceId>,
    /// Firmware version reported by the device (`default-capability.xml`).
    pub firmware_version: Option<String>,
    /// USB / SCSI identity.
    pub usb: Option<UsbSummary>,
    /// The device's self-description (from the internal volume when available).
    pub capability: Option<DeviceCapability>,
    /// The device's volumes, internal first.
    pub volumes: Vec<DetectedVolume>,
    /// Root files WalkTunes must never touch on this device.
    pub protected_root_files: Vec<String>,
    /// Human-readable reasons for the score.
    pub evidence: Vec<String>,
    /// Practical notes for the user (file-size limits, OS metadata folders…).
    pub notes: Vec<String>,
    /// Warning codes, e.g. `WT-DOCK-0008`.
    pub warnings: Vec<String>,
}

/// Groups fingerprints into devices and matches each against the profiles.
#[must_use]
pub fn detect(fingerprints: &[DeviceFingerprint], profiles: &ProfileSet) -> Vec<DetectedDevice> {
    group(fingerprints)
        .into_iter()
        .map(|vols| match_group(&vols, profiles))
        .collect()
}

/// Volumes with the same USB serial form one device; the rest stand alone.
fn group(fps: &[DeviceFingerprint]) -> Vec<Vec<&DeviceFingerprint>> {
    let mut by_serial: BTreeMap<&str, Vec<&DeviceFingerprint>> = BTreeMap::new();
    let mut singles = Vec::new();
    for fp in fps {
        match fp.usb_serial.as_deref().filter(|s| !s.is_empty()) {
            Some(serial) => by_serial.entry(serial).or_default().push(fp),
            None => singles.push(vec![fp]),
        }
    }
    let mut groups: Vec<_> = by_serial.into_values().collect();
    groups.extend(singles);
    for g in &mut groups {
        g.sort_by_key(|fp| {
            (
                role_rank(fp),
                fp.lun.unwrap_or(u8::MAX),
                fp.mount_point.clone(),
            )
        });
    }
    groups
}

/// Internal storage sorts first.
fn role_rank(fp: &DeviceFingerprint) -> u8 {
    match storage_type(fp).as_deref() {
        Some("INTERNAL") => 0,
        Some("CARD") => 2,
        _ => 1,
    }
}

fn storage_type(fp: &DeviceFingerprint) -> Option<String> {
    fp.capability
        .as_ref()
        .and_then(|c| c.storage_type.as_deref())
        .map(str::to_ascii_uppercase)
}

struct Scored<'p> {
    profile: &'p Profile,
    score: u32,
    model: Option<String>,
    /// The profile was singled out by a model-level rule (PID, capability model, name).
    pinned: bool,
    evidence: Vec<String>,
}

fn any<'a>(vols: &[&'a DeviceFingerprint], f: impl Fn(&'a DeviceFingerprint) -> bool) -> bool {
    vols.iter().any(|fp| f(fp))
}

fn eq_ci(a: Option<&str>, b: &str) -> bool {
    a.is_some_and(|a| a.trim().eq_ignore_ascii_case(b.trim()))
}

fn contains_ci(hay: Option<&str>, needle: &str) -> bool {
    hay.is_some_and(|h| {
        h.to_ascii_uppercase()
            .contains(&needle.to_ascii_uppercase())
    })
}

fn starts_ci(hay: Option<&str>, prefix: &str) -> bool {
    hay.is_some_and(|h| {
        h.to_ascii_uppercase()
            .starts_with(&prefix.to_ascii_uppercase())
    })
}

/// The only model of a profile, if it has exactly one.
fn sole_model(p: &Profile) -> Option<String> {
    (p.models.len() == 1).then(|| p.models[0].name.clone())
}

#[allow(clippy::too_many_lines)]
fn score_profile<'p>(vols: &[&DeviceFingerprint], p: &'p Profile) -> Scored<'p> {
    let d = &p.detect;
    let mut s = Scored {
        profile: p,
        score: 0,
        model: None,
        pinned: false,
        evidence: Vec::new(),
    };
    let add = |s: &mut Scored<'p>, w: u32, why: String| {
        s.score += w;
        s.evidence.push(why);
    };

    if let Some(vid) = d.usb_vendor_id
        && any(vols, |fp| fp.usb_vid == Some(vid))
    {
        add(
            &mut s,
            weight::USB_VENDOR,
            format!("USB vendor 0x{vid:04x}"),
        );
    }
    if let Some(v) = &d.scsi_vendor
        && any(vols, |fp| eq_ci(fp.scsi_vendor.as_deref(), v))
    {
        add(&mut s, weight::SCSI_VENDOR, format!("SCSI vendor {v}"));
    }
    if let Some(needles) = &d.product_contains
        && let Some(n) = needles.iter().find(|n| {
            any(vols, |fp| {
                contains_ci(fp.scsi_product.as_deref(), n)
                    || contains_ci(fp.usb_product.as_deref(), n)
            })
        })
    {
        add(
            &mut s,
            weight::PRODUCT,
            format!("product string contains {n}"),
        );
    }
    if let Some(pids) = &d.usb_product_ids
        && let Some(pid) = vols
            .iter()
            .find_map(|fp| fp.usb_pid.filter(|pid| pids.contains(pid)))
    {
        add(
            &mut s,
            weight::USB_PRODUCT_ID,
            format!("USB product id 0x{pid:04x}"),
        );
        s.pinned = true;
        s.model = s.model.take().or_else(|| sole_model(p));
    }
    // Device self-description (default-capability.xml).
    let cap = vols.iter().find_map(|fp| fp.capability.as_ref());
    if let Some(cap) = cap {
        let vendor_ok = d
            .capability_vendor
            .as_deref()
            .is_some_and(|v| starts_ci(cap.vendor.as_deref(), v));
        let class_ok = d
            .capability_class
            .as_deref()
            .is_some_and(|c| starts_ci(cap.class.as_deref(), c));
        if vendor_ok && class_ok {
            add(
                &mut s,
                weight::CAPABILITY,
                format!(
                    "device self-description: {} / {}",
                    cap.vendor.as_deref().unwrap_or("?"),
                    cap.class.as_deref().unwrap_or("?")
                ),
            );
        }
        if let (Some(models), Some(series)) = (&d.capability_models, cap.model.as_deref())
            && models.iter().any(|m| m.eq_ignore_ascii_case(series))
        {
            add(
                &mut s,
                weight::CAPABILITY_MODEL,
                format!("device reports model {series}"),
            );
            s.pinned = true;
            s.model = s.model.take().or_else(|| sole_model(p));
        }
    }
    // Model name inside iProduct / INQUIRY product.
    let named = vols.iter().find_map(|fp| {
        [fp.usb_product.as_deref(), fp.scsi_product.as_deref()]
            .into_iter()
            .flatten()
            .find_map(|txt| p.model_named_in(txt))
    });
    if let Some(m) = named {
        add(
            &mut s,
            weight::MODEL_NAME,
            format!("model name {} in product string", m.name),
        );
        s.pinned = true;
        s.model = Some(m.name.clone());
    }
    if let Some(files) = &d.marker_files
        && !files.is_empty()
        && any(vols, |fp| files.iter().all(|f| fp.has_marker(f)))
    {
        add(
            &mut s,
            weight::MARKER_FILES,
            format!("vendor files {}", files.join(", ")),
        );
    }
    let labels: Vec<&String> = d
        .volume_labels
        .iter()
        .chain(d.sd_volume_labels.iter())
        .flatten()
        .collect();
    if !labels.is_empty()
        && any(vols, |fp| {
            labels.iter().any(|l| eq_ci(fp.volume_label.as_deref(), l))
        })
    {
        add(&mut s, weight::LABEL, "volume label".into());
    }
    if let Some(dirs) = &d.required_dirs
        && !dirs.is_empty()
        && any(vols, |fp| dirs.iter().all(|dir| fp.has_marker(dir)))
    {
        add(&mut s, weight::DIRS, format!("folders {}", dirs.join(", ")));
    }
    if let Some(paired) = vols.iter().find_map(|fp| fp.paired.as_ref())
        && paired.profile_id == p.id
    {
        add(
            &mut s,
            weight::PAIRED,
            "paired (.walktunes/device.json)".into(),
        );
        if let Some(m) = paired.model.as_deref().and_then(|m| p.model(m)) {
            s.model = Some(m.name.clone());
        }
        s.pinned = true;
    }
    s
}

fn match_group(vols: &[&DeviceFingerprint], profiles: &ProfileSet) -> DetectedDevice {
    let mut scored: Vec<Scored<'_>> = profiles
        .concrete()
        .map(|p| score_profile(vols, p))
        .collect();
    // Highest score first; on ties prefer a pinned profile, then one that knows the model.
    scored.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| b.pinned.cmp(&a.pinned))
            .then_with(|| b.model.is_some().cmp(&a.model.is_some()))
            .then_with(|| a.profile.id.cmp(&b.profile.id))
    });

    let paired = vols.iter().find_map(|fp| fp.paired.clone());
    let mut warnings = Vec::new();
    if let Some(pi) = &paired {
        if profiles.get(&pi.profile_id).is_none() {
            warnings.push(
                DockError::UnknownProfile(pi.profile_id.clone())
                    .code()
                    .to_string(),
            );
        }
        let probed = vols.iter().find_map(|fp| fp.usb_serial.clone());
        if let (Some(saved), Some(probed)) = (&pi.usb_serial, &probed)
            && saved != probed
        {
            warnings.push(DockError::SerialMismatch.code().to_string());
        }
    }

    let best = scored.first();
    let best_score = best.map_or(0, |s| s.score);
    let status = if best.is_some_and(|s| s.evidence.iter().any(|e| e.starts_with("paired"))) {
        MatchStatus::Paired
    } else if best_score >= RECOGNIZED {
        MatchStatus::Recognized
    } else if best_score >= SUGGESTED {
        MatchStatus::Suggested
    } else {
        MatchStatus::Generic
    };

    let Some(generic) = profiles.generic() else {
        return empty_device(vols);
    };
    let (mut profile, model, pinned, evidence) = match (status, best) {
        (MatchStatus::Generic, _) | (_, None) => (generic, None, false, Vec::new()),
        (_, Some(s)) => (s.profile, s.model.clone(), s.pinned, s.evidence.clone()),
    };

    // Model still open: rank candidate models by capacity closeness (a pairing hint).
    // A pinned profile (PID / reported series) narrows the candidates to its own models.
    let candidates = if status == MatchStatus::Generic || model.is_some() {
        Vec::new()
    } else if pinned {
        rank_by_capacity(
            vols,
            profile
                .models
                .iter()
                .map(|m| (m.name.clone(), m.capacity_gb)),
        )
    } else {
        let tied = scored
            .iter()
            .filter(|s| s.profile.family == profile.family && s.score == best_score)
            .flat_map(|s| {
                s.profile
                    .models
                    .iter()
                    .map(|m| (m.name.clone(), m.capacity_gb))
            });
        rank_by_capacity(vols, tied)
    };
    if !pinned
        && let Some(owner) = candidates
            .first()
            .and_then(|top| profiles.concrete().find(|p| p.model(top).is_some()))
    {
        profile = owner;
    }
    let family_name = profiles
        .all()
        .iter()
        .find(|p| p.is_abstract && p.family == profile.family)
        .map(|p| p.name.clone());

    let capability = vols
        .iter()
        .find(|fp| storage_type(fp).as_deref() == Some("INTERNAL"))
        .or_else(|| vols.first())
        .and_then(|fp| fp.capability.clone())
        .or_else(|| vols.iter().find_map(|fp| fp.capability.clone()));
    let model_series = capability.as_ref().and_then(|c| c.model.clone());
    let firmware_version = capability.as_ref().and_then(|c| c.firmware_version.clone());

    let protected_root_files = profile
        .detect
        .protected_root_files
        .clone()
        .unwrap_or_default();
    let volumes = roles(vols, profile);
    let notes = notes_for(&volumes);
    let usb = usb_summary(vols);

    let display_name = if status == MatchStatus::Generic {
        vols.first().map(|fp| fp.display_name()).unwrap_or_default()
    } else if let Some(p) = &paired {
        p.name.clone()
    } else if let Some(m) = &model {
        if profile.family == "sony.classic" {
            format!("Sony {m}")
        } else {
            m.clone()
        }
    } else if pinned {
        profile.name.clone()
    } else {
        family_name.unwrap_or_else(|| profile.name.clone())
    };

    DetectedDevice {
        status,
        score: best_score,
        profile_id: profile.id.clone(),
        family: profile.family.clone(),
        display_name,
        model,
        model_series,
        candidates,
        device_id: paired.as_ref().map(|p| p.device_id),
        firmware_version,
        usb,
        capability,
        volumes,
        protected_root_files,
        evidence,
        notes,
        warnings,
    }
}

fn rank_by_capacity(
    vols: &[&DeviceFingerprint],
    models: impl Iterator<Item = (String, Option<u32>)>,
) -> Vec<String> {
    // Usable capacity is ~93% of nominal (formatting, firmware partition).
    let internal_gb = vols.first().map_or(0.0, |fp| fp.capacity_gb());
    let mut c: Vec<(f64, String)> = models
        .map(|(name, gb)| {
            (
                (f64::from(gb.unwrap_or(0)) * 0.93 - internal_gb).abs(),
                name,
            )
        })
        .collect();
    c.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    c.dedup_by(|a, b| a.1 == b.1);
    c.into_iter().map(|(_, n)| n).collect()
}

fn usb_summary(vols: &[&DeviceFingerprint]) -> Option<UsbSummary> {
    let fp = vols
        .iter()
        .find(|fp| fp.usb_vid.is_some() || fp.scsi_vendor.is_some())?;
    Some(UsbSummary {
        vid: fp.usb_vid,
        pid: fp.usb_pid,
        manufacturer: fp.usb_manufacturer.clone(),
        product: fp.usb_product.clone(),
        serial: fp.usb_serial.clone(),
        bcd_device: fp.usb_bcd_device,
        location_id: fp.usb_location_id,
        speed_mbps: fp.usb_speed_mbps,
        scsi: match (&fp.scsi_vendor, &fp.scsi_product, &fp.scsi_revision) {
            (Some(v), Some(p), r) => Some([v.clone(), p.clone(), r.clone().unwrap_or_default()]),
            _ => None,
        },
    })
}

fn roles(vols: &[&DeviceFingerprint], profile: &Profile) -> Vec<DetectedVolume> {
    let sd_labels = profile.detect.sd_volume_labels.clone().unwrap_or_default();
    let single = vols.len() == 1;
    vols.iter()
        .enumerate()
        .map(|(i, fp)| {
            let st = storage_type(fp);
            let role = match st.as_deref() {
                _ if single => VolumeRole::Single,
                Some("CARD") => VolumeRole::Sd,
                Some("INTERNAL") => VolumeRole::Internal,
                _ if fp.lun == Some(1)
                    || sd_labels
                        .iter()
                        .any(|l| eq_ci(fp.volume_label.as_deref(), l))
                    || (fp.lun.is_none() && i > 0) =>
                {
                    VolumeRole::Sd
                }
                _ => VolumeRole::Internal,
            };
            DetectedVolume {
                mount_point: fp.mount_point.clone(),
                role,
                storage_type: st,
                label: fp.volume_label.clone(),
                fs_type: fp.fs_type.clone(),
                fs_name: fp.fs_name.clone(),
                capacity: fp.capacity,
                free: fp.free,
                max_file_size: fp.is_fat32().then_some(FAT32_MAX_FILE),
                writable: fp.writable,
                lun: fp.lun,
                bsd_name: fp.bsd_name.clone(),
                volume_uuid: fp.volume_uuid.clone(),
                os_litter: fp.os_litter.clone(),
            }
        })
        .collect()
}

fn notes_for(volumes: &[DetectedVolume]) -> Vec<String> {
    let mut notes = Vec::new();
    for v in volumes {
        let name = v
            .label
            .clone()
            .unwrap_or_else(|| v.mount_point.display().to_string());
        if v.max_file_size.is_some() {
            notes.push(format!(
                "{name} is FAT32: files of 4 GiB or more (long DSD/hi-res tracks) cannot be stored there"
            ));
        }
        if v.writable == Some(false) {
            notes.push(format!("{name} is mounted read-only"));
        }
        if !v.os_litter.is_empty() {
            notes.push(format!(
                "{name} has OS metadata folders ({}); WalkTunes ignores them",
                v.os_litter.join(", ")
            ));
        }
    }
    notes
}

fn empty_device(vols: &[&DeviceFingerprint]) -> DetectedDevice {
    DetectedDevice {
        status: MatchStatus::Generic,
        score: 0,
        profile_id: String::new(),
        family: String::new(),
        display_name: vols.first().map(|fp| fp.display_name()).unwrap_or_default(),
        model: None,
        model_series: None,
        candidates: Vec::new(),
        device_id: None,
        firmware_version: None,
        usb: None,
        capability: None,
        volumes: Vec::new(),
        protected_root_files: Vec::new(),
        evidence: Vec::new(),
        notes: Vec::new(),
        warnings: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paired::PairedInfo;

    const CAP_INTERNAL: &[u8] =
        include_bytes!("../../../fixtures/probe/capability/nw-zx300.internal.xml");
    const CAP_CARD: &[u8] = include_bytes!("../../../fixtures/probe/capability/nw-zx300.card.xml");

    fn sony(
        label: &str,
        product: &str,
        serial: Option<&str>,
        lun: u8,
        gb: u64,
    ) -> DeviceFingerprint {
        DeviceFingerprint {
            mount_point: PathBuf::from(format!("/Volumes/{label}")),
            usb_vid: Some(0x054c),
            usb_serial: serial.map(str::to_owned),
            usb_product: Some(product.to_owned()),
            scsi_vendor: Some("SONY".into()),
            scsi_product: Some("WALKMAN".into()),
            lun: Some(lun),
            volume_label: Some(label.into()),
            capacity: gb * 1_000_000_000,
            root_markers: vec!["MUSIC".into()],
            ..DeviceFingerprint::default()
        }
    }

    /// The real NW-ZX300 as probed on 2026-11-11 (R-003 part 1), serial redacted.
    fn real_zx300() -> [DeviceFingerprint; 2] {
        let markers = vec![
            "MUSIC".to_owned(),
            "default-capability.xml".to_owned(),
            "DevIcon.fil".to_owned(),
            "DevLogo.fil".to_owned(),
        ];
        let mut internal = sony("WALKMAN", "WALKMAN", Some("REDACTED-01"), 0, 60);
        internal.usb_pid = Some(0x0c70);
        internal.scsi_revision = Some("1.00".into());
        internal.fs_type = Some("msdos".into());
        internal.fs_name = Some("MS-DOS FAT32".into());
        internal.root_markers.clone_from(&markers);
        internal.os_litter = vec![".Spotlight-V100".into()];
        internal.capability = DeviceCapability::parse(CAP_INTERNAL);
        let mut card = sony("Untitled", "WALKMAN", Some("REDACTED-01"), 1, 125);
        card.usb_pid = Some(0x0c70);
        card.fs_type = Some("exfat".into());
        card.root_markers = markers;
        card.capability = DeviceCapability::parse(CAP_CARD);
        // Mount order on macOS lists the card first; grouping must still put internal first.
        [card, internal]
    }

    fn set() -> ProfileSet {
        ProfileSet::builtin().unwrap()
    }

    #[test]
    fn real_zx300_is_recognized_with_model_firmware_and_roles() {
        let devs = detect(&real_zx300(), &set());
        assert_eq!(devs.len(), 1, "internal + card must be one device");
        let d = &devs[0];
        assert_eq!(d.status, MatchStatus::Recognized);
        assert_eq!(d.profile_id, "sony.classic.zx300");
        assert_eq!(d.model.as_deref(), Some("NW-ZX300"));
        assert_eq!(d.model_series.as_deref(), Some("NW-ZX300Series"));
        assert_eq!(d.firmware_version.as_deref(), Some("2.02"));
        assert_eq!(d.display_name, "Sony NW-ZX300");
        let roles: Vec<VolumeRole> = d.volumes.iter().map(|v| v.role).collect();
        assert_eq!(roles, [VolumeRole::Internal, VolumeRole::Sd]);
        assert_eq!(d.volumes[0].max_file_size, Some(FAT32_MAX_FILE));
        assert_eq!(d.volumes[1].max_file_size, None);
        assert!(d.notes.iter().any(|n| n.contains("FAT32")));
        assert!(
            d.protected_root_files
                .contains(&"default-capability.xml".to_owned())
        );
        assert_eq!(d.usb.as_ref().and_then(|u| u.pid), Some(0x0c70));
    }

    #[test]
    fn recorded_macos_probe_of_a_real_zx300_is_recognized() {
        let json = include_str!("../../../fixtures/probe/macos/zx300.fingerprints.json");
        let fps: Vec<DeviceFingerprint> = serde_json::from_str(json).unwrap();
        let devs = detect(&fps, &set());
        assert_eq!(devs.len(), 1);
        let d = &devs[0];
        assert_eq!(d.status, MatchStatus::Recognized);
        assert_eq!(d.model.as_deref(), Some("NW-ZX300"));
        assert_eq!(d.firmware_version.as_deref(), Some("2.02"));
        assert_eq!(d.volumes[0].role, VolumeRole::Internal);
        assert_eq!(d.volumes[1].role, VolumeRole::Sd);
        assert!(d.score >= 200, "score {}", d.score);
    }

    #[test]
    fn model_in_iproduct_identifies_an_a40() {
        let devs = detect(
            &[sony("WALKMAN", "WALKMAN NW-A45", Some("S1"), 0, 15)],
            &set(),
        );
        let d = &devs[0];
        assert_eq!(d.status, MatchStatus::Recognized);
        assert_eq!(d.profile_id, "sony.classic.a40");
        assert_eq!(d.model.as_deref(), Some("NW-A45"));
        assert!(d.protected_root_files.contains(&"NW_WM_FW.UPG".to_owned()));
    }

    #[test]
    fn family_only_match_offers_capacity_ordered_candidates() {
        let devs = detect(&[sony("WALKMAN", "WALKMAN", Some("S2"), 0, 59)], &set());
        let d = &devs[0];
        assert_eq!(d.status, MatchStatus::Recognized);
        assert_eq!(d.family, "sony.classic");
        assert!(d.model.is_none());
        assert_eq!(d.display_name, "Sony Walkman (classic)");
        let top: Vec<&str> = d.candidates.iter().take(4).map(String::as_str).collect();
        for m in ["NW-ZX300", "NW-A47", "NW-A57", "NW-A37HN"] {
            assert!(top.contains(&m), "{m} missing from {top:?}");
        }
    }

    #[test]
    fn internal_and_sd_with_same_serial_form_one_device() {
        let fps = [
            sony("WALKMAN", "WALKMAN NW-ZX300", Some("S3"), 0, 59),
            sony("SDCARD", "WALKMAN NW-ZX300", Some("S3"), 1, 128),
        ];
        let devs = detect(&fps, &set());
        assert_eq!(devs.len(), 1);
        let roles: Vec<VolumeRole> = devs[0].volumes.iter().map(|v| v.role).collect();
        assert_eq!(roles, vec![VolumeRole::Internal, VolumeRole::Sd]);
        assert_eq!(devs[0].model.as_deref(), Some("NW-ZX300"));
    }

    #[test]
    fn plain_usb_stick_is_generic() {
        let fp = DeviceFingerprint {
            mount_point: PathBuf::from("/Volumes/STICK"),
            volume_label: Some("STICK".into()),
            scsi_vendor: Some("SanDisk".into()),
            ..DeviceFingerprint::default()
        };
        let devs = detect(&[fp], &set());
        assert_eq!(devs[0].status, MatchStatus::Generic);
        assert_eq!(devs[0].profile_id, "generic.msc");
        assert_eq!(devs[0].display_name, "STICK");
    }

    #[test]
    fn weak_evidence_only_is_never_recognized() {
        let fp = DeviceFingerprint {
            mount_point: PathBuf::from("/Volumes/WALKMAN"),
            volume_label: Some("WALKMAN".into()),
            root_markers: vec!["MUSIC".into()],
            ..DeviceFingerprint::default()
        };
        let devs = detect(&[fp], &set());
        assert_ne!(devs[0].status, MatchStatus::Recognized);
    }

    #[test]
    fn capability_file_alone_is_enough_without_usb_identity() {
        // E.g. Windows/Linux before their M2 backends: only the files on the volume.
        let fp = DeviceFingerprint {
            mount_point: PathBuf::from("/media/WALKMAN"),
            volume_label: Some("WALKMAN".into()),
            capacity: 60_000_000_000,
            root_markers: vec![
                "MUSIC".into(),
                "default-capability.xml".into(),
                "DevIcon.fil".into(),
                "DevLogo.fil".into(),
            ],
            capability: DeviceCapability::parse(CAP_INTERNAL),
            ..DeviceFingerprint::default()
        };
        let d = &detect(&[fp], &set())[0];
        assert_eq!(d.status, MatchStatus::Recognized);
        assert_eq!(d.model.as_deref(), Some("NW-ZX300"));
    }

    #[test]
    fn pairing_decides_profile_and_model_and_flags_serial_mismatch() {
        let mut fp = sony("WALKMAN", "WALKMAN", Some("REAL"), 0, 59);
        fp.paired = Some(PairedInfo {
            format: PairedInfo::FORMAT.into(),
            version: 1,
            device_id: DeviceId::new(),
            name: "My ZX300".into(),
            profile_id: "sony.classic.zx300".into(),
            volume_role: "internal".into(),
            model: Some("NW-ZX300".into()),
            usb_serial: Some("COPIED".into()),
            firmware_rev: None,
        });
        let d = &detect(&[fp], &set())[0];
        assert_eq!(d.status, MatchStatus::Paired);
        assert_eq!(d.profile_id, "sony.classic.zx300");
        assert_eq!(d.model.as_deref(), Some("NW-ZX300"));
        assert_eq!(d.display_name, "My ZX300");
        assert!(d.warnings.contains(&"WT-DOCK-0008".to_owned()));
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later
//! Scoring fingerprints against profiles and grouping volumes into devices
//! (`docs/EXTENSION.md` §3.1.1).

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Serialize;
use wt_core::{DeviceId, WtError};

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
    pub const LABEL: u32 = 5;
    pub const DIRS: u32 = 5;
    pub const PAIRED: u32 = 100;
}

/// Score at or above which a device is recognized without asking.
pub const RECOGNIZED: u32 = 85;
/// Score at or above which a profile is suggested in the pairing dialog.
pub const SUGGESTED: u32 = 50;

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

/// One volume of a detected device.
#[derive(Clone, Debug, Serialize)]
pub struct DetectedVolume {
    /// Mount point.
    pub mount_point: PathBuf,
    /// Role within the device.
    pub role: VolumeRole,
    /// Volume label.
    pub label: Option<String>,
    /// File-system type.
    pub fs_type: Option<String>,
    /// Size in bytes.
    pub capacity: u64,
    /// SCSI LUN, if known.
    pub lun: Option<u8>,
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
    /// Display name: the paired name, the model, or the profile name.
    pub display_name: String,
    /// Exact model when known (`NW-ZX300`).
    pub model: Option<String>,
    /// When only the family is known: likely models, best guess first (a hint for pairing).
    pub candidates: Vec<String>,
    /// Paired device id, if any.
    pub device_id: Option<DeviceId>,
    /// USB serial shared by the volumes, if known.
    pub usb_serial: Option<String>,
    /// INQUIRY revision (≈ firmware) if known.
    pub firmware_rev: Option<String>,
    /// The device's volumes, internal first.
    pub volumes: Vec<DetectedVolume>,
    /// Root files WalkTunes must never touch on this device.
    pub protected_root_files: Vec<String>,
    /// Human-readable reasons for the score.
    pub evidence: Vec<String>,
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
        g.sort_by_key(|fp| (fp.lun.unwrap_or(u8::MAX), fp.mount_point.clone()));
    }
    groups
}

struct Scored<'p> {
    profile: &'p Profile,
    score: u32,
    model: Option<String>,
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

fn score_profile<'p>(vols: &[&DeviceFingerprint], p: &'p Profile) -> Scored<'p> {
    let d = &p.detect;
    let mut score = 0;
    let mut evidence = Vec::new();
    let mut model = None;

    if let Some(vid) = d.usb_vendor_id
        && any(vols, |fp| fp.usb_vid == Some(vid))
    {
        score += weight::USB_VENDOR;
        evidence.push(format!("USB vendor 0x{vid:04x}"));
    }
    if let Some(v) = &d.scsi_vendor
        && any(vols, |fp| eq_ci(fp.scsi_vendor.as_deref(), v))
    {
        score += weight::SCSI_VENDOR;
        evidence.push(format!("SCSI vendor {v}"));
    }
    if let Some(needles) = &d.product_contains {
        let hit = needles.iter().find(|n| {
            any(vols, |fp| {
                contains_ci(fp.scsi_product.as_deref(), n)
                    || contains_ci(fp.usb_product.as_deref(), n)
            })
        });
        if let Some(n) = hit {
            score += weight::PRODUCT;
            evidence.push(format!("product string contains {n}"));
        }
    }
    if let Some(pids) = &d.usb_product_ids
        && any(vols, |fp| fp.usb_pid.is_some_and(|pid| pids.contains(&pid)))
    {
        score += weight::USB_PRODUCT_ID;
        evidence.push("USB product id matches".into());
    }
    // Model name inside iProduct / INQUIRY product.
    let named = vols.iter().find_map(|fp| {
        [fp.usb_product.as_deref(), fp.scsi_product.as_deref()]
            .into_iter()
            .flatten()
            .find_map(|s| p.model_named_in(s))
    });
    if let Some(m) = named {
        score += weight::MODEL_NAME;
        evidence.push(format!("model name {} in product string", m.name));
        model = Some(m.name.clone());
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
        score += weight::LABEL;
        evidence.push("volume label".into());
    }
    if let Some(dirs) = &d.required_dirs
        && !dirs.is_empty()
        && any(vols, |fp| dirs.iter().all(|dir| fp.has_marker(dir)))
    {
        score += weight::DIRS;
        evidence.push(format!("folders {}", dirs.join(", ")));
    }
    if let Some(paired) = vols.iter().find_map(|fp| fp.paired.as_ref())
        && paired.profile_id == p.id
    {
        score += weight::PAIRED;
        evidence.push("paired (.walktunes/device.json)".into());
        if let Some(m) = paired.model.as_deref().and_then(|m| p.model(m)) {
            model = Some(m.name.clone());
        }
    }
    Scored {
        profile: p,
        score,
        model,
        evidence,
    }
}

fn match_group(vols: &[&DeviceFingerprint], profiles: &ProfileSet) -> DetectedDevice {
    let mut scored: Vec<Scored<'_>> = profiles
        .concrete()
        .map(|p| score_profile(vols, p))
        .collect();
    // Highest score first; on ties prefer a profile that identified the model.
    scored.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
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

    let generic = profiles.generic();
    let (profile, model, evidence) = match (status, best, generic) {
        (MatchStatus::Generic, _, Some(g)) | (_, None, Some(g)) => (g, None, Vec::new()),
        (_, Some(s), _) => (s.profile, s.model.clone(), s.evidence.clone()),
        (_, None, None) => {
            // No profiles at all: cannot happen with built-ins (tested).
            return empty_device(vols);
        }
    };

    // Family-only match: rank the family's models by capacity closeness as a hint.
    let candidates = if status != MatchStatus::Generic && model.is_none() {
        family_candidates(vols, profiles, &profile.family, best_score, &scored)
    } else {
        Vec::new()
    };
    // Family-only: use the profile of the best-guess model (all share the family base),
    // and name the device after the family until the user confirms the model.
    let family_name = profiles
        .all()
        .iter()
        .find(|p| p.is_abstract && p.family == profile.family)
        .map(|p| p.name.clone());
    let profile = candidates
        .first()
        .and_then(|top| profiles.concrete().find(|p| p.model(top).is_some()))
        .unwrap_or(profile);

    let protected_root_files = profile
        .detect
        .protected_root_files
        .clone()
        .unwrap_or_default();
    let volumes = roles(vols, profile);
    let display_name = if status == MatchStatus::Generic {
        vols.first().map(|fp| fp.display_name()).unwrap_or_default()
    } else {
        paired
            .as_ref()
            .map(|p| p.name.clone())
            .or_else(|| {
                model
                    .as_ref()
                    .map(|m| format!("Sony {m}"))
                    .filter(|_| profile.family == "sony.classic")
            })
            .or_else(|| model.clone())
            .or_else(|| family_name.filter(|_| !candidates.is_empty()))
            .unwrap_or_else(|| profile.name.clone())
    };

    DetectedDevice {
        status,
        score: best_score,
        profile_id: profile.id.clone(),
        family: profile.family.clone(),
        display_name,
        model,
        candidates,
        device_id: paired.as_ref().map(|p| p.device_id),
        usb_serial: vols.iter().find_map(|fp| fp.usb_serial.clone()),
        firmware_rev: vols.iter().find_map(|fp| fp.scsi_revision.clone()),
        volumes,
        protected_root_files,
        evidence,
        warnings,
    }
}

/// Models of the same family whose profiles reached the same (family-level) score,
/// ordered by how close their nominal capacity is to the internal volume's size.
fn family_candidates(
    vols: &[&DeviceFingerprint],
    profiles: &ProfileSet,
    family: &str,
    best_score: u32,
    scored: &[Scored<'_>],
) -> Vec<String> {
    let internal_gb = vols.first().map_or(0.0, |fp| fp.capacity_gb());
    let mut cands: Vec<(f64, String)> = scored
        .iter()
        .filter(|s| s.profile.family == family && s.score == best_score)
        .flat_map(|s| s.profile.models.iter())
        .map(|m| {
            // Usable capacity is ~90–95% of nominal; compare against nominal * 0.93.
            let nominal = f64::from(m.capacity_gb.unwrap_or(0));
            ((nominal * 0.93 - internal_gb).abs(), m.name.clone())
        })
        .collect();
    cands.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let _ = profiles; // reserved: PID tables per model (R-003)
    cands.into_iter().map(|(_, n)| n).collect()
}

fn roles(vols: &[&DeviceFingerprint], profile: &Profile) -> Vec<DetectedVolume> {
    let sd_labels = profile.detect.sd_volume_labels.clone().unwrap_or_default();
    let single = vols.len() == 1;
    vols.iter()
        .enumerate()
        .map(|(i, fp)| {
            let role = if single {
                VolumeRole::Single
            } else if fp.lun == Some(1)
                || sd_labels
                    .iter()
                    .any(|l| eq_ci(fp.volume_label.as_deref(), l))
                || (fp.lun.is_none() && i > 0)
            {
                VolumeRole::Sd
            } else {
                VolumeRole::Internal
            };
            DetectedVolume {
                mount_point: fp.mount_point.clone(),
                role,
                label: fp.volume_label.clone(),
                fs_type: fp.fs_type.clone(),
                capacity: fp.capacity,
                lun: fp.lun,
            }
        })
        .collect()
}

fn empty_device(vols: &[&DeviceFingerprint]) -> DetectedDevice {
    DetectedDevice {
        status: MatchStatus::Generic,
        score: 0,
        profile_id: String::new(),
        family: String::new(),
        display_name: vols.first().map(|fp| fp.display_name()).unwrap_or_default(),
        model: None,
        candidates: Vec::new(),
        device_id: None,
        usb_serial: None,
        firmware_rev: None,
        volumes: Vec::new(),
        protected_root_files: Vec::new(),
        evidence: Vec::new(),
        warnings: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paired::PairedInfo;

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

    fn set() -> ProfileSet {
        ProfileSet::builtin().unwrap()
    }

    #[test]
    fn model_in_iproduct_identifies_an_a40() {
        let devs = detect(
            &[sony("WALKMAN", "WALKMAN NW-A45", Some("S1"), 0, 15)],
            &set(),
        );
        assert_eq!(devs.len(), 1);
        let d = &devs[0];
        assert_eq!(d.status, MatchStatus::Recognized);
        assert_eq!(d.profile_id, "sony.classic.a40");
        assert_eq!(d.model.as_deref(), Some("NW-A45"));
        assert!(d.protected_root_files.contains(&"NW_WM_FW.UPG".to_owned()));
    }

    #[test]
    fn family_only_match_offers_capacity_ordered_candidates() {
        // iProduct without a model name, ~59 GB usable → 64 GB models first.
        let devs = detect(&[sony("WALKMAN", "WALKMAN", Some("S2"), 0, 59)], &set());
        let d = &devs[0];
        assert_eq!(d.status, MatchStatus::Recognized);
        assert_eq!(d.family, "sony.classic");
        assert!(d.model.is_none());
        let top3: Vec<&str> = d.candidates.iter().take(4).map(String::as_str).collect();
        for m in ["NW-ZX300", "NW-A47", "NW-A57", "NW-A37HN"] {
            assert!(top3.contains(&m), "{m} missing from {top3:?}");
        }
    }

    #[test]
    fn internal_and_sd_with_same_serial_form_one_device() {
        let fps = [
            sony("WALKMAN", "WALKMAN NW-ZX300", Some("S3"), 0, 59),
            sony("SD_CARD", "WALKMAN NW-ZX300", Some("S3"), 1, 128),
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
    }

    #[test]
    fn weak_evidence_only_is_suggested_or_generic_never_recognized() {
        // Label + MUSIC folder only (no USB/SCSI identity): 10 points → generic.
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

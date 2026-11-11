// SPDX-License-Identifier: GPL-3.0-or-later
//! Device profiles (`docs/EXTENSION.md` §3), embedded from `profiles/*.toml`.

use serde::{Deserialize, Serialize};

use crate::error::DockError;

/// Built-in profile sources, embedded at compile time.
const BUILTIN: &[(&str, &str)] = &[
    (
        "generic.msc",
        include_str!("../../../profiles/generic.msc.toml"),
    ),
    (
        "sony.classic.base",
        include_str!("../../../profiles/sony.classic.base.toml"),
    ),
    (
        "sony.classic.zx300",
        include_str!("../../../profiles/sony.classic.zx300.toml"),
    ),
    (
        "sony.classic.a30",
        include_str!("../../../profiles/sony.classic.a30.toml"),
    ),
    (
        "sony.classic.a40",
        include_str!("../../../profiles/sony.classic.a40.toml"),
    ),
    (
        "sony.classic.a50",
        include_str!("../../../profiles/sony.classic.a50.toml"),
    ),
    (
        "sony.classic.wm1",
        include_str!("../../../profiles/sony.classic.wm1.toml"),
    ),
];

/// The id of the fallback profile for unknown players.
pub const GENERIC_ID: &str = "generic.msc";

/// Detection rules (`[detect]`). Unset fields inherit from the parent profile.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DetectRules {
    /// USB vendor ID.
    pub usb_vendor_id: Option<u16>,
    /// USB product IDs that identify this model/series.
    pub usb_product_ids: Option<Vec<u16>>,
    /// Expected SCSI INQUIRY vendor.
    pub scsi_vendor: Option<String>,
    /// Substrings expected in the INQUIRY product or USB iProduct.
    pub product_contains: Option<Vec<String>>,
    /// Volume labels of the internal storage.
    pub volume_labels: Option<Vec<String>>,
    /// Volume labels of the SD card.
    pub sd_volume_labels: Option<Vec<String>>,
    /// Folders expected at the volume root.
    pub required_dirs: Option<Vec<String>>,
    /// How volumes are grouped into one device (`usb_serial`).
    pub group_by: Option<String>,
    /// Root files WalkTunes must never create, modify or delete.
    pub protected_root_files: Option<Vec<String>>,
    /// Files the vendor places at the volume root (e.g. `DevIcon.fil`).
    pub marker_files: Option<Vec<String>>,
    /// Expected vendor in `default-capability.xml` (prefix match, e.g. `Sony`).
    pub capability_vendor: Option<String>,
    /// Expected device class prefix in `default-capability.xml` (e.g. `WALKMAN`).
    pub capability_class: Option<String>,
    /// Model series names in `default-capability.xml` that identify this profile.
    pub capability_models: Option<Vec<String>>,
}

/// One model within a profile (a series may have several capacities).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelSpec {
    /// Canonical model name, e.g. `NW-A45`.
    pub name: String,
    /// Regional or bundle variants, e.g. `NW-A45HN`.
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Nominal internal storage in GB (a hint only).
    #[serde(default)]
    pub capacity_gb: Option<u32>,
}

impl ModelSpec {
    /// All names this model is known by.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.name.as_str()).chain(self.aliases.iter().map(String::as_str))
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProfile {
    id: String,
    name: String,
    family: String,
    tier: u8,
    #[serde(default)]
    inherits: Option<String>,
    #[serde(default, rename = "abstract")]
    is_abstract: bool,
    #[serde(default)]
    detect: DetectRules,
    #[serde(default)]
    models: Vec<ModelSpec>,
    #[serde(default)]
    layout: Layout,
}

/// Device folder conventions (subset for M0).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    /// Music folder at the volume root.
    pub music_dir: Option<String>,
}

/// A resolved profile (inheritance applied).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Profile {
    /// Profile id, e.g. `sony.classic.a40`.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Family, e.g. `sony.classic`.
    pub family: String,
    /// Support tier (EXTENSION §2).
    pub tier: u8,
    /// Abstract profiles (bases) are never matched directly.
    pub is_abstract: bool,
    /// Resolved detection rules.
    pub detect: DetectRules,
    /// Models in this profile.
    pub models: Vec<ModelSpec>,
    /// Resolved layout.
    pub layout: Layout,
}

impl Profile {
    /// Finds the model whose name or alias appears in `text` (case-insensitive).
    /// Longer names win, so `NW-ZX300A` is not mistaken for another model.
    #[must_use]
    pub fn model_named_in(&self, text: &str) -> Option<&ModelSpec> {
        let hay = text.to_ascii_uppercase();
        self.models
            .iter()
            .flat_map(|m| m.names().map(move |n| (m, n)))
            .filter(|(_, n)| hay.contains(&n.to_ascii_uppercase()))
            .max_by_key(|(_, n)| n.len())
            .map(|(m, _)| m)
    }

    /// Finds a model by exact name or alias.
    #[must_use]
    pub fn model(&self, name: &str) -> Option<&ModelSpec> {
        self.models
            .iter()
            .find(|m| m.names().any(|n| n.eq_ignore_ascii_case(name)))
    }
}

/// All profiles known to this build.
#[derive(Clone, Debug)]
pub struct ProfileSet {
    profiles: Vec<Profile>,
}

impl ProfileSet {
    /// Loads and resolves the built-in profiles.
    ///
    /// # Errors
    /// [`DockError::InvalidProfile`] if a built-in file is malformed (a bug caught by tests).
    pub fn builtin() -> Result<Self, DockError> {
        let mut raws = Vec::new();
        for (id, text) in BUILTIN {
            let raw: RawProfile = toml::from_str(text).map_err(|e| DockError::InvalidProfile {
                id: (*id).to_owned(),
                reason: e.to_string(),
            })?;
            if raw.id != *id {
                return Err(DockError::InvalidProfile {
                    id: (*id).to_owned(),
                    reason: format!("file declares id {}", raw.id),
                });
            }
            raws.push(raw);
        }
        let profiles = raws
            .iter()
            .map(|r| resolve(r, &raws))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { profiles })
    }

    /// Every profile, including abstract bases.
    #[must_use]
    pub fn all(&self) -> &[Profile] {
        &self.profiles
    }

    /// Profiles that can be matched (non-abstract).
    pub fn concrete(&self) -> impl Iterator<Item = &Profile> {
        self.profiles.iter().filter(|p| !p.is_abstract)
    }

    /// Looks up a profile by id.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    /// The generic fallback profile.
    #[must_use]
    pub fn generic(&self) -> Option<&Profile> {
        self.get(GENERIC_ID)
    }
}

fn resolve(raw: &RawProfile, all: &[RawProfile]) -> Result<Profile, DockError> {
    let mut chain = vec![raw];
    let mut cur = raw;
    while let Some(parent_id) = &cur.inherits {
        let parent =
            all.iter()
                .find(|p| &p.id == parent_id)
                .ok_or_else(|| DockError::InvalidProfile {
                    id: raw.id.clone(),
                    reason: format!("unknown parent {parent_id}"),
                })?;
        if chain.len() > 8 {
            return Err(DockError::InvalidProfile {
                id: raw.id.clone(),
                reason: "inheritance too deep or cyclic".into(),
            });
        }
        chain.push(parent);
        cur = parent;
    }
    // Apply from the root ancestor down to the child: child values override.
    let mut detect = DetectRules::default();
    let mut layout = Layout::default();
    for p in chain.iter().rev() {
        merge_detect(&mut detect, &p.detect);
        if p.layout.music_dir.is_some() {
            layout.music_dir.clone_from(&p.layout.music_dir);
        }
    }
    Ok(Profile {
        id: raw.id.clone(),
        name: raw.name.clone(),
        family: raw.family.clone(),
        tier: raw.tier,
        is_abstract: raw.is_abstract,
        detect,
        models: raw.models.clone(),
        layout,
    })
}

fn merge_detect(base: &mut DetectRules, over: &DetectRules) {
    macro_rules! take {
        ($($f:ident),*) => { $( if over.$f.is_some() { base.$f.clone_from(&over.$f); } )* };
    }
    take!(
        usb_vendor_id,
        usb_product_ids,
        scsi_vendor,
        product_contains,
        volume_labels,
        sd_volume_labels,
        required_dirs,
        group_by,
        protected_root_files,
        marker_files,
        capability_vendor,
        capability_class,
        capability_models
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_profiles_parse_and_resolve() {
        let set = ProfileSet::builtin().unwrap();
        assert!(set.generic().is_some());
        let a40 = set.get("sony.classic.a40").unwrap();
        // Inherited from sony.classic.base:
        assert_eq!(a40.detect.scsi_vendor.as_deref(), Some("SONY"));
        assert_eq!(a40.detect.usb_vendor_id, Some(0x054c));
        assert_eq!(a40.layout.music_dir.as_deref(), Some("MUSIC"));
        assert!(!a40.is_abstract);
        assert!(set.get("sony.classic.base").unwrap().is_abstract);
    }

    #[test]
    fn every_sony_profile_protects_the_firmware_file() {
        let set = ProfileSet::builtin().unwrap();
        for p in set.concrete().filter(|p| p.family == "sony.classic") {
            let files = p.detect.protected_root_files.clone().unwrap_or_default();
            assert!(files.iter().any(|f| f == "NW_WM_FW.UPG"), "{}", p.id);
        }
    }

    #[test]
    fn longest_model_name_wins() {
        let set = ProfileSet::builtin().unwrap();
        let zx = set.get("sony.classic.zx300").unwrap();
        assert_eq!(
            zx.model_named_in("SONY NW-ZX300A").map(|m| m.name.as_str()),
            Some("NW-ZX300")
        );
        let a50 = set.get("sony.classic.a50").unwrap();
        assert_eq!(
            a50.model_named_in("walkman nw-a55hn")
                .map(|m| m.name.as_str()),
            Some("NW-A55")
        );
        assert!(a50.model_named_in("NW-A45").is_none());
    }
}

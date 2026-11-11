// SPDX-License-Identifier: GPL-3.0-or-later
//! Sony's device self-description: `default-capability.xml` at each volume root.
//!
//! Found on the NW-ZX300 (R-003 part 1, 2026-11-11): it names the model series, the
//! firmware version, the storage type (`INTERNAL` / `CARD`), the supported audio formats
//! with their limits, the playlist and lyrics formats, and the music folder. It is the
//! most precise passive evidence available, and it is read only (never written).

use serde::{Deserialize, Serialize};

/// The file name at the volume root.
pub const CAPABILITY_FILE: &str = "default-capability.xml";

/// Larger files are ignored (the real file is ~3 KB).
const MAX_BYTES: usize = 256 * 1024;

/// One supported audio format.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioFormatCap {
    /// Sony's format id, e.g. `FLAC`, `ALAC2`, `DSD-1`.
    pub id: String,
    /// File extension without the dot, lowercase.
    pub extension: String,
    /// Codec inside the container, if stated (e.g. `alac`).
    pub codec: Option<String>,
    /// Maximum sample rate in Hz, if stated.
    pub max_sample_rate: Option<u32>,
    /// Maximum bits per sample, if stated.
    pub max_bits: Option<u32>,
}

/// A playlist or lyrics format.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileFormatCap {
    /// Sony's format id, e.g. `m3u8`, `LRC`.
    pub id: String,
    /// File extension as written by the device (e.g. `m3u`, `LRC`).
    pub extension: String,
}

/// Parsed `default-capability.xml`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceCapability {
    /// Schema version of the file (e.g. `3.0`).
    pub version: Option<String>,
    /// Device class, e.g. `WALKMAN/Standard/AudioOnly`.
    pub class: Option<String>,
    /// Model series, e.g. `NW-ZX300Series`.
    pub model: Option<String>,
    /// Marketing name, e.g. `WALKMAN NW-ZX300Series`.
    pub marketing_name: Option<String>,
    /// Vendor, e.g. `Sony Corporation`.
    pub vendor: Option<String>,
    /// Firmware version, e.g. `2.02`.
    pub firmware_version: Option<String>,
    /// `INTERNAL` or `CARD`.
    pub storage_type: Option<String>,
    /// Whether firmware updates are supported from this storage.
    pub fw_update_supported: Option<bool>,
    /// Display size in pixels.
    pub display: Option<(u32, u32)>,
    /// Supported audio formats.
    pub audio: Vec<AudioFormatCap>,
    /// Supported playlist formats.
    pub playlists: Vec<FileFormatCap>,
    /// Supported lyrics formats.
    pub lyrics: Vec<FileFormatCap>,
    /// Music folder, normalized to `/` separators without leading/trailing slashes (`MUSIC`).
    pub music_path: Option<String>,
    /// Marlin DRM support.
    pub drm_marlin: Option<bool>,
    /// SensMe channels available.
    pub sensme: Option<bool>,
}

impl DeviceCapability {
    /// Parses the file's bytes. The file declares ISO-8859-1, which maps bytes 1:1 to
    /// Unicode code points. Returns `None` for anything that is not a capability file.
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_BYTES {
            return None;
        }
        let text: String = bytes.iter().map(|&b| char::from(b)).collect();
        let doc = roxmltree::Document::parse(&text).ok()?;
        let root = doc.root_element();
        if !root.has_tag_name("devices") {
            return None;
        }
        let device = child(root, "device")?;
        let ident = child(device, "identification");
        let storage = child(device, "storage");
        let display = child(device, "display");
        let mut cap = Self {
            version: text_of(root, "version"),
            class: ident.and_then(|n| text_of(n, "class")),
            model: ident.and_then(|n| text_of(n, "model")),
            marketing_name: ident.and_then(|n| text_of(n, "marketingname")),
            vendor: ident.and_then(|n| text_of(n, "vendor")),
            firmware_version: ident.and_then(|n| text_of(n, "firmwareversion")),
            storage_type: storage.and_then(|n| text_of(n, "type")),
            fw_update_supported: storage.and_then(|n| bool_of(n, "fw-update-supported")),
            display: display.and_then(|n| Some((num_of(n, "width")?, num_of(n, "height")?))),
            music_path: child(device, "filesystem")
                .and_then(|n| child(n, "path"))
                .and_then(|n| text_of(n, "sound"))
                .map(|p| p.replace('\\', "/").trim_matches('/').to_owned()),
            drm_marlin: child(device, "drm").and_then(|n| bool_of(n, "marlin")),
            sensme: child(device, "SensMe")
                .and_then(|n| n.attribute("available"))
                .map(|v| v.eq_ignore_ascii_case("true")),
            ..Self::default()
        };
        if let Some(audio) = child(device, "audio") {
            cap.audio = formats(audio)
                .map(|f| AudioFormatCap {
                    id: f.attribute("id").unwrap_or_default().to_owned(),
                    extension: text_of(f, "extension")
                        .unwrap_or_default()
                        .to_ascii_lowercase(),
                    codec: text_of(f, "audiocodec"),
                    max_sample_rate: num_of(f, "sample-rate"),
                    max_bits: num_of(f, "bits-per-sample"),
                })
                .collect();
        }
        cap.playlists = child(device, "audio-playlist")
            .map(file_formats)
            .unwrap_or_default();
        cap.lyrics = child(device, "lyrics")
            .map(file_formats)
            .unwrap_or_default();
        Some(cap)
    }

    /// Whether this is a Sony Walkman description.
    #[must_use]
    pub fn is_sony_walkman(&self) -> bool {
        let sony = self
            .vendor
            .as_deref()
            .is_some_and(|v| v.to_ascii_lowercase().starts_with("sony"));
        let walkman = self
            .class
            .as_deref()
            .is_some_and(|c| c.to_ascii_uppercase().starts_with("WALKMAN"));
        sony && walkman
    }

    /// Lowercase extensions the device can play.
    #[must_use]
    pub fn playable_extensions(&self) -> Vec<String> {
        let mut v: Vec<String> = self.audio.iter().map(|f| f.extension.clone()).collect();
        v.sort();
        v.dedup();
        v
    }
}

fn child<'a, 'i>(n: roxmltree::Node<'a, 'i>, tag: &str) -> Option<roxmltree::Node<'a, 'i>> {
    n.children().find(|c| c.has_tag_name(tag))
}

fn text_of(n: roxmltree::Node<'_, '_>, tag: &str) -> Option<String> {
    child(n, tag)
        .and_then(|c| c.text())
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
}

fn num_of(n: roxmltree::Node<'_, '_>, tag: &str) -> Option<u32> {
    text_of(n, tag).and_then(|t| t.parse().ok())
}

fn bool_of(n: roxmltree::Node<'_, '_>, tag: &str) -> Option<bool> {
    text_of(n, tag).map(|t| t.eq_ignore_ascii_case("true"))
}

fn formats<'a, 'i>(n: roxmltree::Node<'a, 'i>) -> impl Iterator<Item = roxmltree::Node<'a, 'i>> {
    n.children().filter(|c| c.has_tag_name("format"))
}

fn file_formats(n: roxmltree::Node<'_, '_>) -> Vec<FileFormatCap> {
    formats(n)
        .map(|f| FileFormatCap {
            id: f.attribute("id").unwrap_or_default().to_owned(),
            extension: text_of(f, "extension").unwrap_or_default(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const INTERNAL: &[u8] =
        include_bytes!("../../../fixtures/probe/capability/nw-zx300.internal.xml");
    const CARD: &[u8] = include_bytes!("../../../fixtures/probe/capability/nw-zx300.card.xml");

    #[test]
    fn parses_the_real_zx300_file() {
        let cap = DeviceCapability::parse(INTERNAL).unwrap();
        assert!(cap.is_sony_walkman());
        assert_eq!(cap.model.as_deref(), Some("NW-ZX300Series"));
        assert_eq!(cap.firmware_version.as_deref(), Some("2.02"));
        assert_eq!(cap.storage_type.as_deref(), Some("INTERNAL"));
        assert_eq!(cap.display, Some((800, 480)));
        assert_eq!(cap.music_path.as_deref(), Some("MUSIC"));
        let flac = cap.audio.iter().find(|f| f.id == "FLAC").unwrap();
        assert_eq!(
            (flac.max_sample_rate, flac.max_bits),
            (Some(384_000), Some(24))
        );
        let ape = cap.audio.iter().find(|f| f.id == "APE").unwrap();
        assert_eq!(ape.max_sample_rate, Some(192_000));
        let exts = cap.playable_extensions();
        assert!(exts.contains(&"dsf".to_owned()));
        assert!(!exts.contains(&"ogg".to_owned()) && !exts.contains(&"opus".to_owned()));
        assert_eq!(cap.playlists[0].id, "m3u8");
        assert_eq!(cap.playlists[0].extension, "m3u");
        assert_eq!(cap.lyrics[0].extension, "LRC");
        assert_eq!(cap.sensme, Some(true));
        assert_eq!(cap.drm_marlin, Some(false));
    }

    #[test]
    fn card_file_differs_only_in_storage() {
        let card = DeviceCapability::parse(CARD).unwrap();
        assert_eq!(card.storage_type.as_deref(), Some("CARD"));
        assert_eq!(card.fw_update_supported, Some(false));
        assert_eq!(card.model.as_deref(), Some("NW-ZX300Series"));
    }

    #[test]
    fn rejects_other_xml_and_garbage() {
        assert!(DeviceCapability::parse(b"<root/>").is_none());
        assert!(DeviceCapability::parse(b"\x00\x01not xml").is_none());
    }
}

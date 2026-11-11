// SPDX-License-Identifier: GPL-3.0-or-later
//! Audio container formats known to WalkTunes.

use serde::{Deserialize, Serialize};

/// An audio container format, identified by file extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    /// Free Lossless Audio Codec.
    Flac,
    /// MPEG-1/2 Layer III.
    Mp3,
    /// MPEG-4 audio (AAC or ALAC).
    M4a,
    /// RIFF WAVE (PCM).
    Wav,
    /// Audio Interchange File Format.
    Aiff,
    /// DSD Stream File.
    Dsf,
    /// DSDIFF.
    Dff,
    /// Ogg (Vorbis).
    Ogg,
    /// Ogg Opus.
    Opus,
    /// Monkey's Audio.
    Ape,
    /// Windows Media Audio.
    Wma,
}

impl AudioFormat {
    /// Every known format.
    pub const ALL: [Self; 11] = [
        Self::Flac,
        Self::Mp3,
        Self::M4a,
        Self::Wav,
        Self::Aiff,
        Self::Dsf,
        Self::Dff,
        Self::Ogg,
        Self::Opus,
        Self::Ape,
        Self::Wma,
    ];

    /// Detects the format from a file extension (case-insensitive, without the dot).
    #[must_use]
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "flac" => Some(Self::Flac),
            "mp3" => Some(Self::Mp3),
            "m4a" | "mp4" | "aac" | "alac" => Some(Self::M4a),
            "wav" | "wave" => Some(Self::Wav),
            "aif" | "aiff" | "aifc" => Some(Self::Aiff),
            "dsf" => Some(Self::Dsf),
            "dff" => Some(Self::Dff),
            "ogg" | "oga" => Some(Self::Ogg),
            "opus" => Some(Self::Opus),
            "ape" => Some(Self::Ape),
            "wma" => Some(Self::Wma),
            _ => None,
        }
    }

    /// The short lowercase name used in profiles and the Ledger.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Flac => "flac",
            Self::Mp3 => "mp3",
            Self::M4a => "m4a",
            Self::Wav => "wav",
            Self::Aiff => "aiff",
            Self::Dsf => "dsf",
            Self::Dff => "dff",
            Self::Ogg => "ogg",
            Self::Opus => "opus",
            Self::Ape => "ape",
            Self::Wma => "wma",
        }
    }

    /// Whether the container is always lossless. `M4a` can hold either AAC or ALAC,
    /// so it returns `false` here; the codec decides.
    #[must_use]
    pub const fn is_always_lossless(&self) -> bool {
        matches!(
            self,
            Self::Flac | Self::Wav | Self::Aiff | Self::Dsf | Self::Dff | Self::Ape
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions_are_case_insensitive() {
        assert_eq!(AudioFormat::from_extension("FLAC"), Some(AudioFormat::Flac));
        assert_eq!(AudioFormat::from_extension("aif"), Some(AudioFormat::Aiff));
        assert_eq!(AudioFormat::from_extension("txt"), None);
    }

    #[test]
    fn names_round_trip() {
        for f in AudioFormat::ALL {
            assert_eq!(AudioFormat::from_extension(f.as_str()), Some(f));
        }
    }
}

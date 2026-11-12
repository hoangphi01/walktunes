// SPDX-License-Identifier: MIT OR Apache-2.0
//! [`ContentHash`]: a blake3 digest of a file's bytes (`docs/DATABASE.md` §3.2).
//!
//! The digest is computed by `wt-vault`; this crate only carries the value.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

/// A 32-byte blake3 digest, shown and stored as 64 lowercase hex characters.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContentHash([u8; 32]);

/// A string that is not 64 hex characters.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("not a content hash (expected 64 hex characters)")]
pub struct ParseHashError;

impl ContentHash {
    /// Wraps raw digest bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The raw digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The 64-character lowercase hex form.
    #[must_use]
    pub fn to_hex(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ContentHash({self})")
    }
}

impl FromStr for ContentHash {
    type Err = ParseHashError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().as_bytes();
        if s.len() != 64 {
            return Err(ParseHashError);
        }
        let mut out = [0u8; 32];
        for (i, [h, l]) in s.as_chunks::<2>().0.iter().enumerate() {
            let hi = hex_val(*h).ok_or(ParseHashError)?;
            let lo = hex_val(*l).ok_or(ParseHashError)?;
            out[i] = (hi << 4) | lo;
        }
        Ok(Self(out))
    }
}

const fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

impl Serialize for ContentHash {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ContentHash {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip() {
        let mut bytes = [0u8; 32];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(37);
        }
        let h = ContentHash::from_bytes(bytes);
        let text = h.to_hex();
        assert_eq!(text.len(), 64);
        assert_eq!(text.parse::<ContentHash>(), Ok(h));
        assert_eq!(text.to_uppercase().parse::<ContentHash>(), Ok(h));
    }

    #[test]
    fn rejects_bad_strings() {
        assert_eq!("abc".parse::<ContentHash>(), Err(ParseHashError));
        assert_eq!("g".repeat(64).parse::<ContentHash>(), Err(ParseHashError));
    }

    #[test]
    fn serializes_as_hex_string() {
        let h = ContentHash::from_bytes([0xab; 32]);
        let json = serde_json::to_string(&h).unwrap();
        assert_eq!(json, format!("\"{}\"", "ab".repeat(32)));
        assert_eq!(serde_json::from_str::<ContentHash>(&json).unwrap(), h);
    }
}

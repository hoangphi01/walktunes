// SPDX-License-Identifier: MIT OR Apache-2.0
//! Content hashing: blake3 over the whole file (`docs/DATABASE.md` §3.2).

use std::fs::File;
use std::io::Read;
use std::path::Path;

use wt_core::ContentHash;

use crate::VaultError;

/// Hashes the bytes of the file at `path`.
///
/// # Errors
/// The file cannot be opened or read.
pub fn content_hash(path: &Path) -> Result<ContentHash, VaultError> {
    let file = File::open(path).map_err(|e| VaultError::io(path, e))?;
    hash_reader(file).map_err(|e| VaultError::io(path, e))
}

/// Hashes everything `reader` yields.
///
/// # Errors
/// The reader fails.
pub fn hash_reader(reader: impl Read) -> std::io::Result<ContentHash> {
    let mut hasher = blake3::Hasher::new();
    hasher.update_reader(reader)?;
    Ok(finish(&hasher))
}

pub(crate) fn finish(hasher: &blake3::Hasher) -> ContentHash {
    ContentHash::from_bytes(*hasher.finalize().as_bytes())
}

/// Hashes a byte slice (tests and small payloads such as manifests).
#[must_use]
pub fn hash_bytes(bytes: &[u8]) -> ContentHash {
    ContentHash::from_bytes(*blake3::hash(bytes).as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vector() {
        // blake3("") from the reference test vectors.
        assert_eq!(
            hash_bytes(b"").to_hex(),
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );
    }

    #[test]
    fn file_and_bytes_agree() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.bin");
        let data: Vec<u8> = (0..1_000_000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&p, &data).unwrap();
        assert_eq!(content_hash(&p).unwrap(), hash_bytes(&data));
    }
}

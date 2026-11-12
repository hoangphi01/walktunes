// SPDX-License-Identifier: MIT OR Apache-2.0
//! Validating library roots before they are registered (`docs/ARCHITECTURE.md` §3.7).

use std::path::{Path, PathBuf};

use crate::VaultError;

/// Checks that `path` is an existing, readable directory with a UTF-8 path and returns
/// its canonical form (what the Ledger stores).
///
/// # Errors
/// `WT-VAULT-0001` missing or not a directory, `WT-VAULT-0002` unreadable,
/// `WT-VAULT-0006` path is not valid UTF-8.
pub fn check_root(path: &Path) -> Result<PathBuf, VaultError> {
    let not_found = || VaultError::RootNotFound {
        path: path.to_path_buf(),
    };
    let canon = std::fs::canonicalize(path).map_err(|_| not_found())?;
    if !canon.is_dir() {
        return Err(not_found());
    }
    if canon.to_str().is_none() {
        return Err(VaultError::BadPath {
            path: canon.to_string_lossy().into_owned(),
            reason: "not valid UTF-8",
        });
    }
    std::fs::read_dir(&canon).map_err(|e| VaultError::io(&canon, e))?;
    Ok(canon)
}

/// Whether two canonical roots are the same folder or one contains the other.
/// Overlapping roots would scan the same files twice.
#[must_use]
pub fn overlaps(a: &Path, b: &Path) -> bool {
    a.starts_with(b) || b.starts_with(a)
}

/// Returns `WT-VAULT-0008` if `new` overlaps any of `existing` (ignoring an exact match,
/// which callers treat as "already registered").
///
/// # Errors
/// `WT-VAULT-0008` naming the first overlapping root.
pub fn ensure_no_overlap<'a>(
    new: &Path,
    existing: impl IntoIterator<Item = &'a Path>,
) -> Result<(), VaultError> {
    for e in existing {
        if e != new && overlaps(new, e) {
            return Err(VaultError::RootOverlap {
                path: new.to_path_buf(),
                existing: e.to_path_buf(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_root_canonicalizes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("Music")).unwrap();
        let p = check_root(&dir.path().join("Music/../Music")).unwrap();
        assert!(p.is_absolute());
        assert!(p.ends_with("Music"));
        assert!(!p.to_string_lossy().contains(".."));
    }

    #[test]
    fn files_and_missing_paths_are_not_roots() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("f"), b"").unwrap();
        for p in [dir.path().join("f"), dir.path().join("missing")] {
            let e = check_root(&p).unwrap_err();
            assert_eq!(wt_core::WtError::code(&e).to_string(), "WT-VAULT-0001");
        }
    }

    #[test]
    fn overlap_rules() {
        let m = Path::new("/u/Music");
        assert!(overlaps(Path::new("/u/Music/Rips"), m));
        assert!(overlaps(Path::new("/u"), m));
        assert!(
            !overlaps(Path::new("/u/Musical"), m),
            "component-wise, not textual"
        );
        assert!(
            ensure_no_overlap(m, [m]).is_ok(),
            "exact match is handled by the caller"
        );
        let e = ensure_no_overlap(Path::new("/u/Music/Rips"), [m]).unwrap_err();
        assert_eq!(wt_core::WtError::code(&e).to_string(), "WT-VAULT-0008");
    }
}

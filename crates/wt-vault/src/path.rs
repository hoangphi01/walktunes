// SPDX-License-Identifier: MIT OR Apache-2.0
//! Path confinement (`docs/SECURITY.md` §4).
//!
//! Every relative path that reaches Vault — from a manifest, playlist, profile template
//! or provider — is untrusted. [`RelPath::parse`] normalizes and validates it, and
//! [`Scope::resolve`] joins it to a base directory and proves the result stays inside,
//! even through symlinks.

use std::fmt;
use std::path::{Component, Path, PathBuf};

use unicode_normalization::UnicodeNormalization;

use crate::VaultError;

/// Longest single path component, in bytes (common file-system limit).
const MAX_COMPONENT: usize = 255;
/// Longest relative path, in bytes.
const MAX_PATH: usize = 1024;

/// A validated relative path: NFC-normalized, `/`-separated, no `.`/`..`, no empty,
/// absolute or drive-prefixed parts, no `\`, NUL or control characters.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RelPath(String);

fn bad(path: &str, reason: &'static str) -> VaultError {
    VaultError::BadPath {
        path: path.chars().take(200).collect(),
        reason,
    }
}

impl RelPath {
    /// Validates and normalizes `text`.
    ///
    /// # Errors
    /// `WT-VAULT-0006` with the reason.
    pub fn parse(text: &str) -> Result<Self, VaultError> {
        if text.is_empty() {
            return Err(bad(text, "empty"));
        }
        let s: String = text.nfc().collect();
        if s.len() > MAX_PATH {
            return Err(bad(text, "too long"));
        }
        if s.chars().any(char::is_control) {
            return Err(bad(text, "control character"));
        }
        if s.contains('\\') {
            return Err(bad(text, "backslash"));
        }
        if s.starts_with('/') {
            return Err(bad(text, "absolute"));
        }
        let b = s.as_bytes();
        if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
            return Err(bad(text, "drive prefix"));
        }
        for part in s.split('/') {
            match part {
                "" => return Err(bad(text, "empty component")),
                "." | ".." => return Err(bad(text, "dot component")),
                p if p.len() > MAX_COMPONENT => return Err(bad(text, "component too long")),
                _ => {}
            }
        }
        Ok(Self(s))
    }

    /// The normalized text, with `/` separators.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The components, in order.
    pub fn components(&self) -> impl Iterator<Item = &str> {
        self.0.split('/')
    }

    /// The last component.
    #[must_use]
    pub fn file_name(&self) -> &str {
        self.0.rsplit('/').next().unwrap_or(&self.0)
    }
}

impl fmt::Display for RelPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A base directory plus the rules for writing under it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope {
    base: PathBuf,
    allowed_top_dirs: Option<Vec<String>>,
    protected_root_files: Vec<String>,
}

impl Scope {
    /// A library root: anything under `base` may be written.
    ///
    /// # Errors
    /// `WT-VAULT-0001` if `base` is not an existing directory.
    pub fn library_root(base: &Path) -> Result<Self, VaultError> {
        Ok(Self {
            base: canonical_dir(base)?,
            allowed_top_dirs: None,
            protected_root_files: Vec::new(),
        })
    }

    /// A device volume (SECURITY S-10): only inside `allowed_top_dirs` (for example
    /// `MUSIC` and `.walktunes`), never a file at the volume root, and never one of
    /// `protected_root_files`. Names compare ASCII-case-insensitively (FAT/exFAT).
    ///
    /// # Errors
    /// `WT-VAULT-0001` if `base` is not an existing directory.
    pub fn volume(
        base: &Path,
        allowed_top_dirs: &[&str],
        protected_root_files: &[&str],
    ) -> Result<Self, VaultError> {
        Ok(Self {
            base: canonical_dir(base)?,
            allowed_top_dirs: Some(allowed_top_dirs.iter().map(|s| (*s).to_owned()).collect()),
            protected_root_files: protected_root_files
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
        })
    }

    /// The canonical base directory.
    #[must_use]
    pub fn base(&self) -> &Path {
        &self.base
    }

    /// Joins `rel` to the base and proves the result stays inside it.
    ///
    /// The returned path starts with [`Scope::base`]. Existing ancestors are resolved
    /// through symlinks; the final component must not be a symlink.
    ///
    /// # Errors
    /// `WT-VAULT-0004` when the path is outside the scope or protected.
    pub fn resolve(&self, rel: &RelPath) -> Result<PathBuf, VaultError> {
        let parts: Vec<&str> = rel.components().collect();
        let refuse = |reason| VaultError::Refused {
            path: self.base.join(rel.as_str()),
            reason,
        };
        if parts.len() == 1
            && self
                .protected_root_files
                .iter()
                .any(|p| p.eq_ignore_ascii_case(parts[0]))
        {
            return Err(refuse("protected file"));
        }
        if let Some(allowed) = &self.allowed_top_dirs {
            if parts.len() < 2 {
                return Err(refuse("file at volume root"));
            }
            if !allowed.iter().any(|a| a.eq_ignore_ascii_case(parts[0])) {
                return Err(refuse("folder not allowed on this volume"));
            }
        }

        // Walk down while the path exists, resolving each existing prefix.
        let mut real = self.base.clone();
        let mut rest = parts.as_slice();
        while let Some((first, tail)) = rest.split_first() {
            let next = real.join(first);
            match std::fs::symlink_metadata(&next) {
                Ok(meta) => {
                    if tail.is_empty() && meta.file_type().is_symlink() {
                        return Err(refuse("target is a symlink"));
                    }
                    let canon =
                        std::fs::canonicalize(&next).map_err(|_| refuse("dangling symlink"))?;
                    if !canon.starts_with(&self.base) {
                        return Err(refuse("escapes the scope"));
                    }
                    if !tail.is_empty() && !canon.is_dir() {
                        return Err(refuse("parent is not a directory"));
                    }
                    real = canon;
                    rest = tail;
                }
                Err(_) => break,
            }
        }
        for part in rest {
            real.push(part);
        }
        // Belt and braces: no `..` can survive RelPath::parse, but check the result anyway.
        if !real.starts_with(&self.base)
            || real.components().any(|c| matches!(c, Component::ParentDir))
        {
            return Err(refuse("escapes the scope"));
        }
        Ok(real)
    }
}

fn canonical_dir(base: &Path) -> Result<PathBuf, VaultError> {
    let not_found = || VaultError::RootNotFound {
        path: base.to_path_buf(),
    };
    let canon = std::fs::canonicalize(base).map_err(|_| not_found())?;
    if canon.is_dir() {
        Ok(canon)
    } else {
        Err(not_found())
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn parse_accepts_normal_paths_and_normalizes_to_nfc() {
        let decomposed = "MUSIC/Sơn Tùng/01 Bài Hát.flac".nfd().collect::<String>();
        let p = RelPath::parse(&decomposed).unwrap();
        assert_eq!(p.as_str(), "MUSIC/Sơn Tùng/01 Bài Hát.flac");
        assert_eq!(p.file_name(), "01 Bài Hát.flac");
    }

    #[test]
    fn parse_rejects_dangerous_paths() {
        for bad in [
            "",
            "/etc/passwd",
            "../x",
            "a/../../x",
            "a/./b",
            "a//b",
            "a/",
            "C:/x",
            "c:x",
            "a\\..\\b",
            "a\0b",
            "a\nb",
            "a\u{7f}b",
        ] {
            assert!(RelPath::parse(bad).is_err(), "accepted {bad:?}");
        }
        assert!(RelPath::parse(&"x".repeat(300)).is_err());
    }

    #[test]
    fn volume_scope_rules() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("MUSIC")).unwrap();
        let scope = Scope::volume(
            dir.path(),
            &["MUSIC", ".walktunes"],
            &["NW_WM_FW.UPG", "default-capability.xml"],
        )
        .unwrap();
        let ok = |s: &str| scope.resolve(&RelPath::parse(s).unwrap());
        assert!(ok("MUSIC/a/b.flac").unwrap().starts_with(scope.base()));
        assert!(ok("music/a.flac").is_ok(), "FAT names are case-insensitive");
        assert!(ok(".walktunes/manifest.json").is_ok());
        assert!(ok("nw_wm_fw.upg").is_err());
        assert!(ok("default-capability.xml").is_err());
        assert!(ok("other.txt").is_err());
        assert!(ok("PODCASTS/x.mp3").is_err());
    }

    #[test]
    fn missing_base_is_root_not_found() {
        let err = Scope::library_root(Path::new("/definitely/not/here")).unwrap_err();
        assert_eq!(wt_core::WtError::code(&err).to_string(), "WT-VAULT-0001");
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_cannot_escape() {
        let outside = tempfile::tempdir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("out")).unwrap();
        std::fs::create_dir(dir.path().join("in")).unwrap();
        std::os::unix::fs::symlink(dir.path().join("in"), dir.path().join("alias")).unwrap();
        std::os::unix::fs::symlink(outside.path().join("f"), dir.path().join("link.flac")).unwrap();
        let scope = Scope::library_root(dir.path()).unwrap();
        let r = |s: &str| scope.resolve(&RelPath::parse(s).unwrap());
        assert!(r("out/x.flac").is_err(), "symlinked dir pointing outside");
        assert!(r("link.flac").is_err(), "final symlink");
        let inside = r("alias/x.flac").unwrap();
        assert!(inside.starts_with(scope.base()));
        assert!(inside.ends_with("in/x.flac"));
    }

    fn piece() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("..".to_owned()),
            Just(".".to_owned()),
            Just("/".to_owned()),
            Just("\\".to_owned()),
            Just("\0".to_owned()),
            Just("C:".to_owned()),
            Just("~".to_owned()),
            Just("MUSIC".to_owned()),
            Just("Sơn Tùng".to_owned()),
            "[a-zA-Z0-9 ._-]{1,12}",
            "\\PC{1,8}",
            "x{250,260}",
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(2000))]

        /// Whatever the input, resolve either refuses or returns a path inside the base.
        #[test]
        fn resolved_paths_never_escape(pieces in proptest::collection::vec(piece(), 1..8)) {
            let dir = tempfile::tempdir().unwrap();
            std::fs::create_dir(dir.path().join("MUSIC")).unwrap();
            let scopes = [
                Scope::library_root(dir.path()).unwrap(),
                Scope::volume(dir.path(), &["MUSIC"], &["NW_WM_FW.UPG"]).unwrap(),
            ];
            let text: String = pieces.concat();
            if let Ok(rel) = RelPath::parse(&text) {
                for scope in &scopes {
                    if let Ok(p) = scope.resolve(&rel) {
                        prop_assert!(p.starts_with(scope.base()), "{text:?} -> {p:?}");
                        prop_assert!(p != scope.base());
                    }
                }
            }
        }
    }
}

// SPDX-License-Identifier: MIT OR Apache-2.0
//! Crash-at-every-step and safety tests for [`Vault::write_atomic`] (TESTING §5, S-4/S-5).

use std::io::{Cursor, Read};
use std::path::Path;
use std::sync::Arc;

use wt_core::{MemoryJournal, OpKind, OpState, OpsJournal, WtError};

use crate::write::{FailMode, Step};
use crate::*;

const OLD: &[u8] = b"old contents of the file";

fn new_data() -> Vec<u8> {
    // Three chunks, so a mid-copy crash leaves a genuinely partial temp file.
    (0..700_000u32).map(|i| (i % 253) as u8).collect()
}

struct Env {
    dir: tempfile::TempDir,
    journal: Arc<MemoryJournal>,
    vault: Vault,
    scope: Scope,
}

fn env() -> Env {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("MUSIC")).unwrap();
    std::fs::write(dir.path().join("MUSIC/old.flac"), OLD).unwrap();
    std::fs::write(dir.path().join("NW_WM_FW.UPG"), b"firmware").unwrap();
    let journal = Arc::new(MemoryJournal::new());
    let vault = Vault::new(journal.clone());
    let scope = Scope::volume(dir.path(), &["MUSIC", ".walktunes"], &["NW_WM_FW.UPG"]).unwrap();
    Env {
        dir,
        journal,
        vault,
        scope,
    }
}

fn temp_files(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap() {
            let e = e.unwrap();
            let name = e.file_name().to_string_lossy().into_owned();
            if e.file_type().unwrap().is_dir() {
                stack.push(e.path());
            } else if name.starts_with(TEMP_PREFIX) {
                out.push(e.path().display().to_string());
            }
        }
    }
    out
}

fn rel(s: &str) -> RelPath {
    RelPath::parse(s).unwrap()
}

#[test]
fn writes_new_file_and_journals_mkdirs_first() {
    let e = env();
    let data = new_data();
    let out = e
        .vault
        .write_atomic(
            &e.scope,
            &rel("MUSIC/Sơn Tùng/Album/01 Bài.flac"),
            &mut Cursor::new(&data),
            WriteMode::CreateNew,
            Some(hash_bytes(&data)),
            None,
        )
        .unwrap();
    assert_eq!(std::fs::read(&out.path).unwrap(), data);
    assert_eq!(out.hash, hash_bytes(&data));
    assert_eq!(out.bytes, data.len() as u64);
    let ops: Vec<_> = e
        .journal
        .entries()
        .iter()
        .map(|o| (o.op, o.state))
        .collect();
    assert_eq!(
        ops,
        vec![
            (OpKind::Write, OpState::Done),
            (OpKind::Mkdir, OpState::Done),
            (OpKind::Mkdir, OpState::Done),
        ]
    );
    assert!(temp_files(e.dir.path()).is_empty());
}

#[test]
fn create_new_never_overwrites() {
    let e = env();
    let err = e
        .vault
        .write_atomic(
            &e.scope,
            &rel("MUSIC/old.flac"),
            &mut Cursor::new(b"x"),
            WriteMode::CreateNew,
            None,
            None,
        )
        .unwrap_err();
    assert_eq!(err.code().to_string(), "WT-VAULT-0004");
    assert_eq!(
        std::fs::read(e.dir.path().join("MUSIC/old.flac")).unwrap(),
        OLD
    );
}

#[test]
fn replace_requires_the_planned_hash() {
    let e = env();
    let stale = hash_bytes(b"something else");
    let err = e
        .vault
        .write_atomic(
            &e.scope,
            &rel("MUSIC/old.flac"),
            &mut Cursor::new(b"new"),
            WriteMode::Replace {
                expected_old: stale,
            },
            None,
            None,
        )
        .unwrap_err();
    assert_eq!(err.code().to_string(), "WT-VAULT-0004");
    assert_eq!(
        std::fs::read(e.dir.path().join("MUSIC/old.flac")).unwrap(),
        OLD
    );

    let ok = e.vault.write_atomic(
        &e.scope,
        &rel("MUSIC/old.flac"),
        &mut Cursor::new(b"new"),
        WriteMode::Replace {
            expected_old: hash_bytes(OLD),
        },
        None,
        None,
    );
    assert!(ok.is_ok());
    assert_eq!(
        std::fs::read(e.dir.path().join("MUSIC/old.flac")).unwrap(),
        b"new"
    );
    assert_eq!(
        e.journal.entries().last().unwrap().hash_before,
        Some(hash_bytes(OLD))
    );
}

#[test]
fn protected_and_out_of_scope_targets_are_refused_before_journaling() {
    let e = env();
    for p in ["NW_WM_FW.UPG", "root.txt", "Other/x.flac"] {
        let err = e
            .vault
            .write_atomic(
                &e.scope,
                &rel(p),
                &mut Cursor::new(b"x"),
                WriteMode::CreateNew,
                None,
                None,
            )
            .unwrap_err();
        assert_eq!(err.code().to_string(), "WT-VAULT-0004", "{p}");
    }
    assert_eq!(
        std::fs::read(e.dir.path().join("NW_WM_FW.UPG")).unwrap(),
        b"firmware"
    );
    assert!(e.journal.entries().is_empty());
}

#[test]
fn expected_hash_mismatch_aborts_and_cleans_up() {
    let e = env();
    let err = e
        .vault
        .write_atomic(
            &e.scope,
            &rel("MUSIC/a.flac"),
            &mut Cursor::new(new_data()),
            WriteMode::CreateNew,
            Some(hash_bytes(b"not this")),
            None,
        )
        .unwrap_err();
    assert_eq!(err.code().to_string(), "WT-VAULT-0003");
    assert!(err.recovery() == wt_core::Recovery::Abort);
    assert!(!e.dir.path().join("MUSIC/a.flac").exists());
    assert!(temp_files(e.dir.path()).is_empty());
    let last = e.journal.entries()[0].clone();
    assert_eq!(last.state, OpState::Failed);
    assert_eq!(last.error_code.as_deref(), Some("WT-VAULT-0003"));
}

struct Broken(usize);
impl Read for Broken {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.0 == 0 {
            return Err(std::io::Error::other("source unplugged"));
        }
        self.0 -= 1;
        buf.fill(1);
        Ok(buf.len())
    }
}

#[test]
fn failing_source_leaves_nothing_behind() {
    let e = env();
    let err = e.vault.write_atomic(
        &e.scope,
        &rel("MUSIC/a.flac"),
        &mut Broken(2),
        WriteMode::CreateNew,
        None,
        None,
    );
    assert!(err.is_err());
    assert!(!e.dir.path().join("MUSIC/a.flac").exists());
    assert!(temp_files(e.dir.path()).is_empty());
}

/// The core guarantee (ROADMAP P1 Day 1 exit check): whatever step fails or crashes,
/// the target is either untouched or completely new — never partial — and recovery
/// leaves no temp file and no pending operation.
#[test]
fn crash_at_every_step() {
    let data = new_data();
    for step in Step::ALL {
        for fail in [FailMode::Fail, FailMode::Crash] {
            for replace in [false, true] {
                let e = env();
                let (target, mode) = if replace {
                    (
                        "MUSIC/old.flac",
                        WriteMode::Replace {
                            expected_old: hash_bytes(OLD),
                        },
                    )
                } else {
                    ("MUSIC/New Album/01.flac", WriteMode::CreateNew)
                };
                let case = format!("{step:?} {fail:?} replace={replace}");
                e.vault.arm(step, fail);
                let res = e.vault.write_atomic(
                    &e.scope,
                    &rel(target),
                    &mut Cursor::new(&data),
                    mode,
                    None,
                    None,
                );
                assert!(res.is_err(), "{case}: failpoint did not fire");

                let path = e.dir.path().join(target);
                let now = std::fs::read(&path).ok();
                let before = if replace { Some(OLD.to_vec()) } else { None };
                assert!(
                    now == before || now.as_deref() == Some(&data[..]),
                    "{case}: target is partial or wrong ({:?} bytes)",
                    now.map(|v| v.len())
                );

                if fail == FailMode::Fail {
                    assert!(
                        temp_files(e.dir.path()).is_empty(),
                        "{case}: temp left after a clean failure"
                    );
                }
                let report = e.vault.recover().unwrap();
                assert!(
                    temp_files(e.dir.path()).is_empty(),
                    "{case}: temp left after recovery"
                );
                assert!(
                    e.journal.pending().unwrap().is_empty(),
                    "{case}: pending op left"
                );
                if fail == FailMode::Crash {
                    assert!(report.interrupted >= 1, "{case}: crash not detected");
                    assert!(
                        e.journal
                            .entries()
                            .iter()
                            .any(|o| o.error_code.as_deref() == Some("WT-VAULT-0007")),
                        "{case}"
                    );
                }
                // Untouched neighbours.
                assert_eq!(
                    std::fs::read(e.dir.path().join("NW_WM_FW.UPG")).unwrap(),
                    b"firmware"
                );
                if !replace {
                    assert_eq!(
                        std::fs::read(e.dir.path().join("MUSIC/old.flac")).unwrap(),
                        OLD
                    );
                }
            }
        }
    }
}

#[test]
fn recover_only_deletes_vault_temp_files() {
    let e = env();
    // A tampered journal entry pointing at a real file must not delete it.
    let j = &e.journal;
    j.begin(&wt_core::OpRecord {
        run_id: None,
        op: OpKind::Write,
        target: e.dir.path().join("MUSIC/x.flac"),
        aux: Some(e.dir.path().join("MUSIC/old.flac")),
        hash_before: None,
    })
    .unwrap();
    let report = e.vault.recover().unwrap();
    assert_eq!(report.interrupted, 1);
    assert_eq!(report.temp_files_removed, 0);
    assert_eq!(
        std::fs::read(e.dir.path().join("MUSIC/old.flac")).unwrap(),
        OLD
    );
}

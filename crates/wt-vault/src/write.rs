// SPDX-License-Identifier: MIT OR Apache-2.0
//! Atomic writes (`docs/SECURITY.md` S-4, S-5).
//!
//! 1. resolve the target inside its [`Scope`] and check the [`WriteMode`];
//! 2. journal the operation as `pending` (with the temp path in `aux`);
//! 3. create missing parent folders, each journaled as `mkdir`;
//! 4. write a temp file in the target folder (`create_new`), hashing while copying;
//! 5. `fsync`, then re-read the temp file and compare hashes (and the expected hash);
//! 6. rename over the target, `fsync` the folder, journal `done`.
//!
//! Any failure removes the temp file and journals `failed`; the target is never left
//! partially written. A crash leaves at most a `.wt-tmp-*` file and a `pending` record,
//! which [`Vault::recover`](crate::Vault::recover) cleans up at the next start.

use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use uuid::Uuid;
use wt_core::{ContentHash, OpId, OpKind, OpRecord, OpState, SyncRunId, WtError};

use crate::hash::{content_hash, finish};
use crate::{RelPath, Scope, Vault, VaultError};

/// Prefix of every temp file Vault creates. Recovery only ever deletes files with it.
pub const TEMP_PREFIX: &str = ".wt-tmp-";

const CHUNK: usize = 256 * 1024;

/// Whether a write may replace an existing file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteMode {
    /// The target must not exist.
    CreateNew,
    /// The target must exist and still have this hash (it was not changed since it was
    /// planned). Vault never overwrites blindly.
    Replace {
        /// Hash the target is expected to have now.
        expected_old: ContentHash,
    },
}

/// The result of a successful [`Vault::write_atomic`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WriteOutcome {
    /// Final path of the file.
    pub path: PathBuf,
    /// blake3 of the bytes written (verified on disk).
    pub hash: ContentHash,
    /// Number of bytes written.
    pub bytes: u64,
    /// The journal entry.
    pub op: OpId,
}

/// Points in [`Vault::write_atomic`] where tests inject failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    AfterBegin,
    AfterMkdir,
    AfterTempCreate,
    MidCopy,
    AfterFsync,
    AfterVerify,
    AfterRename,
}

#[cfg(test)]
impl Step {
    pub(crate) const ALL: [Self; 7] = [
        Self::AfterBegin,
        Self::AfterMkdir,
        Self::AfterTempCreate,
        Self::MidCopy,
        Self::AfterFsync,
        Self::AfterVerify,
        Self::AfterRename,
    ];
}

/// What an injected failure does.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FailMode {
    /// Return an I/O error: the normal cleanup runs.
    Fail,
    /// Stop dead, as if the power went out: no cleanup, no journal update.
    Crash,
}

/// Internal control flow: an error to clean up after, or a simulated crash.
enum Flow {
    Err(VaultError),
    #[cfg(test)]
    Crash,
}

impl From<VaultError> for Flow {
    fn from(e: VaultError) -> Self {
        Self::Err(e)
    }
}

impl Vault {
    #[cfg(test)]
    fn fp(&self, step: Step) -> Result<(), Flow> {
        let armed = self.failpoint.lock().ok().and_then(|g| *g);
        match armed {
            Some((s, FailMode::Fail)) if s == step => Err(Flow::Err(VaultError::Io {
                path: PathBuf::from("<failpoint>"),
                source: std::io::Error::other("injected failure"),
            })),
            Some((s, FailMode::Crash)) if s == step => Err(Flow::Crash),
            _ => Ok(()),
        }
    }

    #[cfg(not(test))]
    #[allow(clippy::unused_self, clippy::unnecessary_wraps)]
    #[inline]
    fn fp(&self, _step: Step) -> Result<(), Flow> {
        Ok(())
    }

    /// Writes `src` to `rel` inside `scope`, atomically and journaled (S-4, S-5).
    ///
    /// `expected`, when given, is the hash the written bytes must have (for example the
    /// library original's hash when copying to a device).
    ///
    /// # Errors
    /// `WT-VAULT-0004` out of scope / existing target / stale `Replace` hash,
    /// `WT-VAULT-0003` hash mismatch, `0002`/`0005`/`0006`/`0009` I/O failures, or a
    /// journal error (nothing is changed when the journal cannot record the operation).
    pub fn write_atomic(
        &self,
        scope: &Scope,
        rel: &RelPath,
        src: &mut dyn Read,
        mode: WriteMode,
        expected: Option<ContentHash>,
        run_id: Option<SyncRunId>,
    ) -> Result<WriteOutcome, VaultError> {
        let target = scope.resolve(rel)?;
        let hash_before = check_mode(&target, mode)?;
        let parent = target
            .parent()
            .ok_or_else(|| VaultError::Refused {
                path: target.clone(),
                reason: "no parent folder",
            })?
            .to_path_buf();
        let temp = parent.join(format!("{TEMP_PREFIX}{}", Uuid::now_v7().simple()));
        let op = self.journal.begin(&OpRecord {
            run_id,
            op: match mode {
                WriteMode::CreateNew => OpKind::Write,
                WriteMode::Replace { .. } => OpKind::Replace,
            },
            target: target.clone(),
            aux: Some(temp.clone()),
            hash_before,
        })?;

        let result = self.write_steps(scope, &target, &parent, &temp, src, mode, expected, run_id);
        match result {
            Ok((hash, bytes)) => {
                self.journal.finish(op, OpState::Done, Some(&hash), None)?;
                Ok(WriteOutcome {
                    path: target,
                    hash,
                    bytes,
                    op,
                })
            }
            Err(Flow::Err(e)) => {
                remove_temp(&temp);
                if let Err(j) = self
                    .journal
                    .finish(op, OpState::Failed, None, Some(e.code()))
                {
                    wt_watch::error!(op = %op, error = %j, "could not journal a failed write");
                }
                Err(e)
            }
            #[cfg(test)]
            Err(Flow::Crash) => Err(VaultError::SimulatedCrash),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn write_steps(
        &self,
        scope: &Scope,
        target: &Path,
        parent: &Path,
        temp: &Path,
        src: &mut dyn Read,
        mode: WriteMode,
        expected: Option<ContentHash>,
        run_id: Option<SyncRunId>,
    ) -> Result<(ContentHash, u64), Flow> {
        self.fp(Step::AfterBegin)?;
        self.create_parents(scope, parent, run_id)?;
        self.fp(Step::AfterMkdir)?;

        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(temp)
            .map_err(|e| VaultError::io(temp, e))?;
        self.fp(Step::AfterTempCreate)?;

        let mut hasher = blake3::Hasher::new();
        let mut buf = vec![0u8; CHUNK];
        let mut bytes = 0u64;
        let mut first = true;
        loop {
            let n = match src.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(VaultError::io(Path::new("<source>"), e).into()),
            };
            hasher.update(&buf[..n]);
            file.write_all(&buf[..n])
                .map_err(|e| VaultError::io(temp, e))?;
            bytes += n as u64;
            if first {
                first = false;
                self.fp(Step::MidCopy)?;
            }
        }
        file.sync_all().map_err(|e| VaultError::io(temp, e))?;
        drop(file);
        self.fp(Step::AfterFsync)?;

        let streamed = finish(&hasher);
        let on_disk = content_hash(temp)?;
        if on_disk != streamed {
            return Err(VaultError::HashMismatch {
                path: target.to_path_buf(),
                expected: streamed,
                actual: on_disk,
            }
            .into());
        }
        if let Some(exp) = expected
            && exp != streamed
        {
            return Err(VaultError::HashMismatch {
                path: target.to_path_buf(),
                expected: exp,
                actual: streamed,
            }
            .into());
        }
        self.fp(Step::AfterVerify)?;

        // Narrow the window between planning and renaming: re-check the target.
        check_mode(target, mode)?;
        std::fs::rename(temp, target).map_err(|e| VaultError::io(target, e))?;
        self.fp(Step::AfterRename)?;
        sync_dir(parent);
        Ok((streamed, bytes))
    }

    /// Creates the missing folders between the scope base and `dir`, journaling each.
    fn create_parents(
        &self,
        scope: &Scope,
        dir: &Path,
        run_id: Option<SyncRunId>,
    ) -> Result<(), VaultError> {
        let mut missing = Vec::new();
        let mut cur = dir;
        while !cur.exists() {
            if !cur.starts_with(scope.base()) || cur == scope.base() {
                return Err(VaultError::Refused {
                    path: dir.to_path_buf(),
                    reason: "escapes the scope",
                });
            }
            missing.push(cur.to_path_buf());
            cur = match cur.parent() {
                Some(p) => p,
                None => break,
            };
        }
        for d in missing.into_iter().rev() {
            let op = self.journal.begin(&OpRecord {
                run_id,
                op: OpKind::Mkdir,
                target: d.clone(),
                aux: None,
                hash_before: None,
            })?;
            match std::fs::create_dir(&d) {
                Ok(()) => self.journal.finish(op, OpState::Done, None, None)?,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && d.is_dir() => {
                    self.journal.finish(op, OpState::Done, None, None)?;
                }
                Err(e) => {
                    let err = VaultError::io(&d, e);
                    self.journal
                        .finish(op, OpState::Failed, None, Some(err.code()))?;
                    return Err(err);
                }
            }
        }
        Ok(())
    }
}

/// Checks the target against the write mode; returns the current hash for `Replace`.
fn check_mode(target: &Path, mode: WriteMode) -> Result<Option<ContentHash>, VaultError> {
    let meta = std::fs::symlink_metadata(target);
    match mode {
        WriteMode::CreateNew => match meta {
            Ok(_) => Err(VaultError::Refused {
                path: target.to_path_buf(),
                reason: "target already exists",
            }),
            Err(_) => Ok(None),
        },
        WriteMode::Replace { expected_old } => {
            let refuse = |reason| VaultError::Refused {
                path: target.to_path_buf(),
                reason,
            };
            match meta {
                Ok(m) if m.is_file() => {
                    let now = content_hash(target)?;
                    if now == expected_old {
                        Ok(Some(now))
                    } else {
                        Err(refuse("target changed since it was planned"))
                    }
                }
                Ok(_) => Err(refuse("target is not a regular file")),
                Err(_) => Err(refuse("target to replace does not exist")),
            }
        }
    }
}

/// Best-effort removal of a temp file this module created.
pub(crate) fn remove_temp(temp: &Path) {
    let is_ours = temp
        .file_name()
        .is_some_and(|n| n.to_string_lossy().starts_with(TEMP_PREFIX));
    if is_ours
        && let Err(e) = std::fs::remove_file(temp)
        && e.kind() != std::io::ErrorKind::NotFound
    {
        wt_watch::warn!(path = %temp.display(), error = %e, "could not remove temp file");
    }
}

/// Makes the rename durable. Some file systems (FAT on some OSes) refuse; that is logged.
fn sync_dir(dir: &Path) {
    #[cfg(unix)]
    if let Err(e) = std::fs::File::open(dir).and_then(|d| d.sync_all()) {
        wt_watch::debug!(path = %dir.display(), error = %e, "directory fsync not supported");
    }
    #[cfg(not(unix))]
    let _ = dir; // Windows has no directory fsync; NTFS journals the rename.
}

# ERROR_CONTROL

| Field | Value |
|---|---|
| Status | Draft |
| Owner | Every crate; registry kept by Maintainer |
| Last reviewed | 2026-11-09 |
| Related | [SECURITY](SECURITY.md) · [ARCHITECTURE](ARCHITECTURE.md) · [STYLE_RULE §2](STYLE_RULE.md#2-rust-code-style) |

---

## 1. Principles

1. **Every failure has a code.** Users, logs and bug reports all refer to `WT-<SVC>-<NNNN>`.
2. **Fail safe, not silent.** When unsure, stop before writing, never after half-writing. A half-done file operation must be rolled back or recorded in `ops_log` as `pending`.
3. **One bad file never sinks a batch.** Per-item errors in scans and syncs are collected into a report. A batch aborts only on *safety* errors (§4).
4. **Errors are data; messages are translations.** Error enums carry codes and context. Text comes from `wt-i18n` using the code as the key.
5. **A panic is a bug.** It is caught at the job boundary, reported as `WT-QUEUE-0001`, and never exposed as a crash dialog.

---

## 2. The error model in Rust

```rust
// wt-core
pub trait WtError: std::error::Error + Send + Sync + 'static {
    fn code(&self) -> ErrorCode;          // e.g. ErrorCode::new("SYNC", 3)
    fn severity(&self) -> Severity;       // Info | Warning | Error | Fatal
    fn recovery(&self) -> Recovery;       // Retry | UserAction | Skip | Abort
    fn context(&self) -> Vec<(&'static str, String)> { vec![] } // safe for logs
}

// each library crate
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("not enough free space on {volume}: need {need}, have {have}")]
    InsufficientSpace { volume: String, need: u64, have: u64 },          // WT-SYNC-0002
    #[error("device disconnected during sync")]
    Disconnected,                                                         // WT-SYNC-0003
    #[error(transparent)]
    Vault(#[from] wt_vault::VaultError),                                  // keeps inner code
}
```

Rules:
- Libraries use `thiserror`. Only `wt-cli` and `wt-console` may use `anyhow`, and only at the top level.
- `unwrap()` and `expect()` are forbidden in library code (enforced with clippy `unwrap_used` and `expect_used`), except in tests and for proven invariants marked with `// INVARIANT: …`.
- Wrapping keeps the **innermost** code. The UI shows the root cause, and the chain goes to the log.
- Each code is defined in exactly one enum variant. `xtask docs-check` verifies that every code in the source appears in §5 and the reverse.

---

## 3. Severity → presentation

| Severity | Console (GUI) | CLI | Log level |
|---|---|---|---|
| Info | Inline hint | Message on stderr, exit 0 | `info` |
| Warning | Toast; the item is marked in the result list | `warning:` prefix, exit 0 | `warn` |
| Error | Inline banner on the screen + entry in the Jobs panel | `error:` prefix, exit 1 | `error` |
| Fatal | Blocking dialog with "Export diagnostics" | exit 2 | `error` + diagnostics hint |

CLI exit codes: `0` ok, `1` operation error, `2` fatal, `3` partially done
(the batch completed with per-item errors), `64` usage error, `130`
cancelled.

---

## 4. Recovery classes

| Recovery | Meaning | Who acts |
|---|---|---|
| **Retry** | Transient (network, busy DB, device momentarily busy). Auto-retry with exponential backoff (3 tries: 0.5 s, 2 s, 8 s), then surface | System |
| **UserAction** | Needs a decision (free space, pairing, consent, missing ffmpeg) | User, via a suggested action button |
| **Skip** | This item cannot be processed (corrupt file, unsupported tag) | System skips it and reports |
| **Abort** | **Safety error.** Continuing could lose or corrupt data (hash mismatch after write, manifest conflict, unexpected disconnect) | System stops the whole job, rolls back the current file, keeps the checkpoint |

**Safety errors always Abort.** Examples include any hash mismatch after a
copy, any attempt to remove a non-manifest file, and a manifest generation
going backwards.

---

## 5. Error code registry

Format: `WT-<SVC>-<NNNN>`. Service keys: `CORE CONFIG WATCH BUS QUEUE I18N LEDGER VAULT GATE CACHE CRAWL ENRICH FORGE DOCK SYNC SDK KIT UI CLI`.
Codes are never reused. Retire a code by marking it ~~struck through~~.

| Code | Severity | Recovery | Meaning |
|---|---|---|---|
| WT-CORE-0001 | Fatal | Abort | Invariant violated (bug) |
| WT-CONFIG-0001 | Error | UserAction | Settings file unreadable; defaults loaded, original kept as `.bad` |
| WT-CONFIG-0002 | Error | UserAction | Data directory not writable |
| WT-QUEUE-0001 | Error | Abort | Job panicked (bug); diagnostics captured |
| WT-QUEUE-0002 | Warning | Retry | Resource lock busy (for example, a sync already running for this device) |
| WT-QUEUE-0003 | Info | — | Job cancelled by user |
| WT-LEDGER-0001 | Fatal | UserAction | Database corrupt; offer restore from `.bak` |
| WT-LEDGER-0002 | Fatal | UserAction | Database schema is newer than this app; update the app |
| WT-LEDGER-0003 | Error | Abort | Migration failed; rolled back, backup kept |
| WT-LEDGER-0004 | Warning | Retry | Database busy |
| WT-VAULT-0001 | Error | UserAction | Root not found or not mounted |
| WT-VAULT-0002 | Error | Skip | Permission denied on file |
| WT-VAULT-0003 | Fatal | Abort | Hash mismatch after write (**safety**) |
| WT-VAULT-0004 | Fatal | Abort | Refused: delete or overwrite outside an allowed scope (**safety**) |
| WT-VAULT-0005 | Error | UserAction | Disk full |
| WT-VAULT-0006 | Error | Skip | Path too long or has illegal characters for the target file system |
| WT-GATE-0001 | Info | UserAction | Provider not consented; enable it in Settings |
| WT-GATE-0002 | Warning | Retry | Network unavailable or timeout |
| WT-GATE-0003 | Warning | Retry | Rate limited by provider (HTTP 429/503) |
| WT-GATE-0004 | Error | Skip | Unexpected provider response |
| WT-CACHE-0001 | Warning | Skip | Cache entry corrupt; dropped and rebuilt |
| WT-CRAWL-0001 | Warning | Skip | Unreadable or corrupt audio file |
| WT-CRAWL-0002 | Warning | Skip | Unsupported container |
| WT-CRAWL-0003 | Warning | Skip | Tag read error; file indexed with filename-only data |
| WT-ENRICH-0001 | Info | — | No match found |
| WT-ENRICH-0002 | Warning | UserAction | Ambiguous match; user must choose |
| WT-ENRICH-0003 | Error | UserAction | AcoustID API key missing or invalid |
| WT-FORGE-0001 | Error | UserAction | ffmpeg not found; transcoding disabled |
| WT-FORGE-0002 | Error | Skip | Transcode failed (ffmpeg exit code, stderr tail in log) |
| WT-FORGE-0003 | Warning | Skip | Artwork unreadable; synced without artwork |
| WT-DOCK-0001 | Info | UserAction | Volume not paired |
| WT-DOCK-0002 | Error | UserAction | Volume is read-only |
| WT-DOCK-0003 | Error | UserAction | Unknown profile id in `device.json` (made by a newer app?) |
| WT-DOCK-0004 | Error | UserAction | Manifest unreadable; previous generation also unreadable; offer Rebuild |
| WT-DOCK-0005 | Fatal | Abort | Manifest generation went backwards (**safety**) |
| WT-DOCK-0006 | Warning | UserAction | Eject failed (files in use) |
| WT-SYNC-0001 | Error | UserAction | Plan is stale (device or library changed since planning); re-plan |
| WT-SYNC-0002 | Error | UserAction | Insufficient free space |
| WT-SYNC-0003 | Fatal | Abort | Device disconnected during sync; resume on reattach |
| WT-SYNC-0004 | Fatal | Abort | Attempt to remove a foreign (non-manifest) file (**safety**) |
| WT-SYNC-0005 | Warning | UserAction | Conflict: same track differs between library and device |
| WT-UI-0001 | Error | — | Font or asset failed to load; fell back to system font |
| WT-CLI-0001 | Error | — | Invalid arguments (exit 64) |

---

## 6. Logging and diagnostics

- Every logged error includes `code`, `job_id` (if any) and `context()` pairs.
- Logs live in `<data_dir>/logs/`, rotated daily, with 14 days kept.
- **Diagnostics bundle** (`wtunes diag` / Settings → Export diagnostics) contains logs, app and OS versions, config with home-directory paths redacted to `~`, and the last 200 `ops_log` rows. It never contains audio files, artwork or API keys.

---

## 7. Adding a new error (checklist)

1. Choose the next free number for the service in §5 and add a row.
2. Add the enum variant with `code()`, `severity()` and `recovery()`.
3. Add i18n keys `error-<svc>-<nnnn>` and `error-<svc>-<nnnn>-action` in **both** `en` and `vi`.
4. If the recovery is `UserAction`, define the action button in the Console.
5. Add a test that triggers the error (see [TESTING](TESTING.md)).

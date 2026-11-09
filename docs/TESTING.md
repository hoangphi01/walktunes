# TESTING

| Field | Value |
|---|---|
| Status | Draft |
| Owner | Every crate |
| Last reviewed | 2026-11-09 |
| Related | [UI_TESTS](UI_TESTS.md) · [SECURITY §3](SECURITY.md#3-data-safety-the-1-risk) · [EXTENSION §3.5](EXTENSION.md#35-hardware-test-script-per-device) |

---

## 1. Strategy

| Level | What | Where | Runs |
|---|---|---|---|
| Unit | Pure logic: planning, parsing, identity, path sanitizing | `src/**` `mod tests` | Every PR |
| Property | Invariants over random inputs (safety rules) | `proptest` in `wt-sync`, `wt-vault`, `wt-enrich` | Every PR |
| Integration | One service with real SQLite and a temp filesystem | `crates/<svc>/tests/` | Every PR |
| End-to-end | CLI against fixtures and virtual devices | `crates/wt-cli/tests/` (`assert_cmd`) | Every PR |
| UI | GPUI test contexts + manual checklists | [UI_TESTS](UI_TESTS.md) | Every PR / every release |
| Fuzz | Tag, manifest, playlist and filename parsers | `fuzz/` (`cargo fuzz`) | Nightly |
| Hardware | Real devices | [EXTENSION §3.5](EXTENSION.md#35-hardware-test-script-per-device) | Per milestone / per profile |

Coverage target: **≥ 80% line coverage for `wt-vault`, `wt-sync`, `wt-dock`
and `wt-ledger`** (the data-safety core), measured with `cargo llvm-cov`.
No target for UI crates.

---

## 2. Data-safety invariants (must have property tests)

| Invariant | Crate |
|---|---|
| A plan never contains `Remove` for a path absent from the manifest, or whose hash differs from the manifest | `wt-sync` |
| Executing a plan, then planning again with no changes, yields an **empty** plan (idempotence) | `wt-sync` |
| Interrupting execution at any op index, then resuming, yields the same final device state as an uninterrupted run | `wt-sync` |
| Joined paths never escape their root, whatever the manifest says | `wt-vault` |
| `write_atomic` leaves either the old file or the new file, never a partial one (simulated crash at every step) | `wt-vault` |
| Manifest round-trip preserves unknown fields | `wt-dock` |
| Filename sanitizing is idempotent and the result is valid on FAT32 and exFAT | `wt-forge` |

---

## 3. Fixtures

**Copyrighted audio must never be committed.** All media fixtures are
generated:

```bash
cargo xtask fixtures   # requires ffmpeg
```

This generates the following under `fixtures/` (git-ignored, except the recipe files):
- `media/`: short sine and noise clips (1–3 s) in every supported format, with tag sets covering Vietnamese diacritics, multi-disc albums, missing tags, wrong album artist, embedded art variants (baseline/progressive, 300–3000 px, PNG), and YouTube-style filenames (`Artist - Title (Official Video) [HD].mp3`).
- `large/`: N synthetic tracks for benchmarks (`--count 10000`).
- `ledger/v{n}.db`: databases of each released schema version, for migration tests.
- `http/<provider>/`: recorded API responses (JSON) for offline provider tests.

The recipes live in `xtask/src/fixtures/*.toml` and are committed.

---

## 4. Virtual devices

A virtual device is a folder that acts as a mounted volume.

```
fixtures/devices/sony.classic.zx300/
├── internal/        # role=internal (label WALKMAN)
│   └── MUSIC/
└── sd/              # role=sd
```

- Set `WT_FAKE_VOLUMES=fixtures/devices/sony.classic.zx300/internal,…` and `wt-dock` treats these folders as attached volumes. Removing a path from the variable during a test simulates unplugging.
- **Fault injection:** in test builds, `wt-vault` accepts a `FaultPlan` (fail the Nth write, return ENOSPC, disconnect after N bytes, slow I/O) to test errors and resume.
- **Filesystem rules:** virtual devices apply the profile's `[filesystem]` rules (illegal characters, path length, case-insensitivity) in software, so they behave like FAT/exFAT even on APFS or ext4.
- Optional realism on CI Linux: a loop-mounted FAT32 / exFAT image (`xtask fat-image`).

---

## 5. Test naming and structure
- Describe behaviour: `sync_resume_after_disconnect_completes_plan`.
- Use the Arrange / Act / Assert structure. One behaviour per test.
- Each test uses a fresh `WT_DATA_DIR` (a `tempfile::TempDir` helper in `wt-core::testing`).
- Never touch real home-directory paths. CI fails if a test writes outside its temp directory (the guard checks the `HOME` / `APPDATA` override).

---

## 6. CI gates

A PR is mergeable only if all of these pass:
1. fmt, clippy (`-D warnings`), `xtask layers`, `xtask docs-check`
2. The test suite on macOS, Windows and Linux
3. Property tests (default 256 cases; nightly runs 10k)
4. `cargo deny check`
5. The coverage of the safety-core crates has not dropped by more than 1 point

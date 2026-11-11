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
| Numerical | DSP kernels vs. scalar reference; detector verdicts on labelled fixtures | `wt-dsp`, `wt-inspect` | Every PR, on x86-64 **and** ARM64 |
| Fuzz | Tag, manifest, playlist and filename parsers; audio decoding entry points of `wt-inspect` | `fuzz/` (`cargo fuzz`) | Nightly |
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
- **Fingerprints:** a `fingerprint.json` next to each virtual volume supplies the probe result (`DeviceFingerprint`), so detection and grouping are tested without hardware. Recorded real outputs (DiskArbitration descriptions, sysfs trees, SetupAPI instance IDs; serials anonymized) live in `fixtures/probe/<os>/` and are parsed by backend unit tests.
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
2. The test suite on macOS, Windows and Linux, on **x86-64 and ARM64**; the riscv64 cross-build compiles and its unit tests pass under QEMU
3. Property tests (default 256 cases; nightly runs 10k)
4. `cargo deny check`
5. The coverage of the safety-core crates has not dropped by more than 1 point

---

## 7. Numerical and cross-architecture tests

### 7.1 Kernel equivalence
Every SIMD kernel in `wt-dsp` has a plain scalar reference. Property tests
feed random inputs (including denormals, silence, full-scale and NaN guards)
and require:
- the SIMD path equals the scalar path within **1e-5 relative** (f32);
- FFT output matches a naive DFT on small sizes within 1e-4;
- dB conversion error < 0.01 dB in the range -140…0 dBFS.

These tests run on every architecture in CI, so each dispatch path
(AVX2, SSE4.1, NEON, scalar) is exercised. Use the `WT_FORCE_SIMD=scalar|sse41|avx2|neon`
test switch to force a path on capable machines.

### 7.2 Labelled inspection fixtures
Generated by `cargo xtask fixtures --inspect` from synthetic signals
(sine sweeps 20 Hz–22 kHz, pink and white noise, multitone). No copyrighted audio.

| Fixture | How it is made | Expected verdict |
|---|---|---|
| `genuine_44k16.flac` | Noise + sweep, encoded straight to FLAC | `genuine` |
| `fake_mp3_128.flac` | Same source → MP3 128 kbps → decoded → FLAC | `likely_lossy_source`, cutoff ≈ 16 kHz |
| `fake_mp3_320.flac` | → MP3 320 kbps → FLAC | `suspect` or `likely_lossy_source`, cutoff ≈ 19.5–20 kHz |
| `fake_aac_256.flac` | → AAC 256 kbps → FLAC | `suspect` |
| `upsampled_96k.flac` | 44.1 kHz source resampled to 96 kHz | `upsampled`, effective rate ≈ 44.1 kHz |
| `padded_24bit.flac` | 16-bit source stored as 24-bit | `padded_bits`, effective bits = 16 |
| `corrupt_frame.flac` | Bytes flipped inside one audio frame | integrity `failed`, CRC error |
| `md5_mismatch.flac` | Audio altered, STREAMINFO MD5 kept | `md5_status = mismatch` |
| `no_md5.flac` | MD5 field zeroed | `md5_status = absent` |
| `truncated.flac` | File cut at 70% | integrity `failed`, truncation |
| `mono_as_stereo.flac` | L = R | `fake_stereo = 1` |
| `clipped.flac` | Gain pushed to clip | `clipped_samples > 0` |
| `quiet_analog.flac` | Band-limited 15 kHz "old recording" without a brickwall edge | `genuine` (false-positive guard) |

The thresholds are calibrated against this set, and the calibration result is
recorded as R-005. Changing any threshold requires re-running the set and
bumping `analyzer_version`.

### 7.3 Cross-architecture agreement
For every fixture, CI stores the verdict and a spectrogram checksum per
architecture. Verdicts must be identical. Spectrogram values are compared
cell by cell, within **0.1 dB** (B-17), because FMA and summation order differ
legitimately between AVX2 and NEON.

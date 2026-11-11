# ROADMAP

| Field | Value |
|---|---|
| Status | Draft |
| Owner | Maintainer |
| Last reviewed | 2026-11-11 |
| Related | [PRD](PRD.md) · [STATUS](STATUS.md) · [RESULT](RESULT.md) |

Milestones are **ordered, not dated**. Each one has exit criteria. A
milestone is done only when every box is ticked **and** its outcome is
recorded in [RESULT.md](RESULT.md).

```mermaid
flowchart LR
  M0[M0 Foundation] --> M1[M1 Ledger & Crawler]
  M1 --> M2[M2 Dock & Push Sync]
  M2 --> M3[M3 Health & Forge]
  M3 --> M4[M4 Console MVP]
  M4 --> M5[M5 Enrich & Playlists]
  M5 --> M6[M6 Multi-machine & 1.0]
  M6 --> M7[M7+ More devices]
```

---

## Prototype sprint P1: first working prototype in 7 days (2026-11-12 → 2026-11-18)

**Goal:** a vertical slice through M1–M4, proven on the maintainer's real NW-ZX300
(firmware 2.02, 128 GB microSD):

> Plug in the ZX300 → WalkTunes recognizes it → you pick albums from your library →
> you review a dry-run plan → WalkTunes copies only what is missing → the Walkman plays
> the songs with correct titles, Vietnamese diacritics, album grouping and artwork →
> plug it in again and the plan is empty.

**Built on what we already know:**
- The probe works on real hardware (R-003 part 1, score 220).
- `default-capability.xml` gives the exact formats, `MUSIC/` and `.m3u`/LRC conventions.
- The internal storage is **FAT32**: no file may reach 4 GiB.
- The SD card is LUN 1 on the same USB device.
- The 3 MP3s already in `MUSIC/` are **foreign files** and must never be touched (S-3).

**Constraints:**
- macOS on Apple Silicon only for the prototype; CI keeps the other OSes compiling.
- No full Xcode: GPUI uses `runtime_shaders`.
- About 11 GB of free disk: set dependency `debug = false` and run `cargo clean` when needed.

**Out of scope for P1:** enrichment (M5), inspection/spectrogram (M3), pull and multi-machine
(M6), `wt-helper`, transcoding (anything the capability file says is unplayable is
*skipped with a reason*), Windows/Linux USB backends, i18n beyond English strings.

| Day | Date | Focus | Deliverables | Exit check |
|---|---|---|---|---|
| **1** | 2026-11-12 | **Ledger + Vault** (safety core) | `wt-ledger`: SQLite schema v1 subset (`roots`, `tracks`, `files`, `devices`, `device_volumes`, `device_files`, `sync_runs`, `ops_log`) + migrations + WAL. `wt-vault`: roots, blake3 `content_hash`, **`write_atomic`** (temp → fsync → verify hash → rename), path confinement (S-1…S-5), `ops_log`. `xtask fixtures` minimal: ~12 generated clips (FLAC/MP3/M4A/WAV, Vietnamese tags, embedded art variants) | `wtunes root add ~/Music` works; crash-at-every-step test for `write_atomic`; path-escape property test passes |
| **2** | 2026-11-13 | **Crawler** (M1) | `wt-crawler`: `walkdir` + `lofty` tags/properties, incremental scan (size + mtime), `audio_hash` for FLAC/MP3, identity ladder levels 2/4/5, FTS5 search ignoring diacritics. CLI `scan`, `search`, `stats --json` | Scan of the maintainer's library completes; a rescan with no changes takes < 3 s per 10k files; `search "son tung"` finds `Sơn Tùng`. Scan numbers recorded as R-002 (lite) |
| **3** | 2026-11-14 | **Pairing + manifest** (M2) | `wt-dock`: write `device.json` on both volumes (usb_serial, model, firmware, role from INTERNAL/CARD); `manifest.json` read/write with `generation`, `.prev` backup and atomic replace; foreign-file classification. CLI `device pair`, `device status` | On virtual devices: pair → unplug (remove from `WT_FAKE_VOLUMES`) → re-probe = `paired`. On the **real ZX300**: pairing writes **only** `.walktunes/` (verified with a before/after root listing) |
| **4** | 2026-11-15 | **Sync plan + executor** (M2) | `wt-sync`: selection (album/artist), `SyncPlan` (`Add`/`Replace`/`Remove` manifest-owned only); rules from the capability file (unplayable format → skip with reason), the **FAT32 4 GiB rule** (route to SD or skip), filename sanitizing (FAT-illegal characters, NFC, path length), template `MUSIC/<AlbumArtist>/<Album>/<NN> <Title>.<ext>`. Executor: copy via Vault, verify hash, checkpoint the manifest every 25 files / 10 s, resume after interruption. CLI `sync plan`, `sync run` | Property tests: never removes foreign files; plan → run → plan = empty; interrupt at any op + resume = same final state. Dry-run output reviewed by the maintainer |
| **5** | 2026-11-16 | **Real-device session** (R-003 part 2) | Sync a test album set to the ZX300 and check on the player: tags, diacritics, album grouping, multi-disc order, **artwork variants** (baseline/progressive, 300–1500 px, PNG) → minimal Forge rule (re-encode embedded art as baseline JPEG ≤ verified size, on the copy only); **playlist** `.m3u` location and path form; DB rebuild time; unplug during sync → resume | PRD Q2 and Q3 answered; `[VERIFY]` markers removed; R-003 part 2 written; the 3 foreign MP3s untouched |
| **6** | 2026-11-17 | **GPUI window** (R-001 + M4 slice) | `wt-kit` minimal: tokens (radius 0, true black), Be Vietnam Pro, Lucide icons via `gpui-component`, `WtTile`, `WtProgress` (opens at 10%, UX-12). `wt-console`: **Start** page with a live device tile (model, firmware, free space per volume); **Library** list (virtualized); **Sync** screen showing the plan (Remove group first, UX-04) and a *Sync* button running the job with progress. `runtime_shaders`; dependency debug info off | The app builds without Xcode and opens in < 1 s; 50k-row list scrolls smoothly; R-001 recorded (macOS only) |
| **7** | 2026-11-18 | **End-to-end + release** | Full run through the GUI on the real ZX300; bug fixes; plan-is-empty-on-replug check; USER_GUIDE quick start (EN/VI); STATUS, RESULT **R-007 "Prototype P1"**, LESSONS; CI green; tag **`v0.1.0-prototype`** | The goal sentence above works end to end, on video or with written steps, and is reproducible from a clean checkout |

**Daily routine:** start from STATUS → work → `cargo xtask layers && cargo xtask docs-check` and all
tests green → update the docs the DOCS_GUIDE §4 matrix requires → commit (no co-author) and push.

**If the sprint slips (in this order of cutting):**
1. Day 6 shrinks to a single Sync screen; the CLI stays the reference front end.
2. Artwork normalization is postponed if the ZX300 shows the original embedded art correctly.
3. Resume-after-interruption keeps its tests but drops the GUI affordance.

**Never cut:** the data-safety rules (S-1…S-10) and their tests.

**Real-device safety during P1:** the first writes on the ZX300 happen on Day 3 (`.walktunes/`
only) and Day 5 (test albums under `MUSIC/`). Before Day 5, copy the device's `MUSIC/` folder to
the Mac as a backup. Every device write is listed in `ops_log`.

---

## M0 — Foundation
**Goal:** a repository where good work is easy.
- [x] Docs system (this set), GPL-3.0 license
- [x] Cargo workspace with the empty crates from [ARCHITECTURE §6](ARCHITECTURE.md#6-workspace-layout), shared lints, `rust-toolchain.toml`
- [x] CI on macOS, Windows and Linux (x86-64 + ARM64): fmt, clippy, test, `cargo-deny`
- [x] `xtask layers` (dependency rule check) and `xtask docs-check`
- [x] Skeleton: real `wt-core`, `wt-config`, `wt-watch`, `wt-bus`, `wt-dsp` (SIMD detection), `wt-dock` (fingerprints, ZX + A-series profiles, scoring, grouping), `wt-sdk`, `wtunes info | profiles list | device probe`
- [ ] `xtask fixtures`: generate test audio with ffmpeg, plus virtual devices
- [ ] **DSP spike:** STFT + spectrogram of a 5-min FLAC on Apple Silicon (NEON) and x86-64 (AVX2), plus a RISC-V scalar build under QEMU. Record the result as R-004.
- [ ] **GPUI spike:** a window with Be Vietnam Pro, a Lucide icon, a 50k-row virtual list, and a chart, built on all 3 OSes. Record the result as R-001.

## M1 — Ledger & Crawler (CLI)
**Goal:** know what music you have.
- [ ] `wt-core`, `wt-config`, `wt-watch`, `wt-bus`, `wt-queue` minimal
- [ ] `wt-ledger` schema v1 and migrations ([DATABASE](DATABASE.md))
- [ ] `wt-vault` roots plus hashing; `wt-crawler` incremental scan
- [ ] `wtunes root add|list`, `wtunes scan`, `wtunes search`, `wtunes stats --json`
- [ ] Scan budget met ([OPTIMIZATION](OPTIMIZATION.md)) — R-002

## M2 — Dock & Push Sync (CLI)
**Goal:** reliably put music on a ZX300.
- [ ] Volume discovery and pairing; profile `sony.classic.zx300`
- [ ] Device probe: passive fingerprint with macOS / Windows / Linux backends, scoring, volume grouping by USB serial, `wtunes device probe --json` (FR-040, FR-056, FR-057; ADR-0014)
- [ ] Device manifest read/write with atomic semantics
- [ ] `SyncPlan` and executor (Add/Replace/Remove), checkpoints, resume
- [ ] `wtunes device list|pair`, `wtunes sync plan|run`
- [x] Real-device probe of the ZX300 (R-003 part 1): Q5 answered, recognized with score 220
- [ ] Real-device test session part 2: answer PRD Q2 and Q3, and remove the related `[VERIFY]` markers — R-003
- [ ] Unplug-during-sync test passes (NFR-007)

## M3 — Health & Forge
**Goal:** what lands on the device looks right.
- [ ] Health checks (FR-020) and duplicate detection (FR-005)
- [ ] Forge: baseline JPEG artwork, filename sanitization, tag simplification
- [ ] Optional ffmpeg transcoding per profile (FR-048)
- [ ] `wt-dsp` kernels + `wt-inspect`: integrity (CRC, MD5, truncation), fake-lossless, upsample and padded-bits detectors (FR-090, 092, 093, 095, 096)
- [ ] Detector calibration on a labelled fixture set — R-005
- [ ] `wtunes health`, `wtunes dedupe`, `wtunes inspect`

## M4 — Console MVP (GPUI)
**Goal:** the everyday app.
- [ ] `wt-kit`: tokens, font, icons, core components, motion
- [ ] Start page (live tiles) and hubs: Library, Device, Sync (plan review), Health, Jobs, Settings
- [ ] EN/VI switch, light and dark themes, keyboard navigation
- [ ] Storage charts (FR-054), safe eject (FR-053)
- [ ] Inspect hub: spectrogram viewer (FR-091), report export (FR-097), pre-sync check (FR-098)
- [ ] `wt-helper`: plug-in detection like iTunesHelper: login item, mount events, open app / notify, IPC to a running console (FR-058, FR-059, ADR-0015); macOS first, then Windows and Linux
- [ ] UI test suite and manual checklist ([UI_TESTS](UI_TESTS.md))

## M5 — Enrich & Playlists
**Goal:** fix messy libraries, YouTube rips included.
- [ ] `wt-gate` with consent, rate limits and offline mode
- [ ] Providers: filename parser, MusicBrainz, AcoustID, Cover Art Archive, LRCLIB
- [ ] Proposal review UI (accept / reject / edit)
- [ ] Playlists: create, edit, sync as M3U8; lyrics as LRC

## M6 — Multi-machine & 1.0
**Goal:** the device ties your computers together.
- [ ] Pull to the Inbox root, adopt foreign files, conflict resolution
- [ ] Packaging and signing on all 3 OSes ([RELEASE](RELEASE.md))
- [x] Final product name: WalkTunes (PRD Q1)
- [ ] User guides complete
- [ ] Release 1.0.0

## M8 — Discover & on the go *(after 1.0, vision)*
**Goal:** find music in the app and have it reach the player with no manual steps.
- [ ] Unified search: library + MusicBrainz "missing albums" (FR-100)
- [ ] Legal "Get" links (FR-101); Q6 decided by the maintainer (ADR-0016)
- [ ] Inbox watch folder with automatic tagging and inspection (FR-102)
- [ ] On-the-go updates through the Helper (FR-103), wish list (FR-104)
- [ ] Mobile companion feasibility study (FR-105)

## M7+ — After 1.0
- Tier-2 profiles: older Sony models (community-verified)
- Tier-3 generic mass-storage players: write-order control (FR-055), 8.3 and FAT quirks
- Tier-4 MTP support (libmtp / Windows WPD)
- RISC-V vector (RVV) kernels once Rust intrinsics are stable; GUI on RISC-V once GPUI supports it
- DSD analysis (DSD → PCM decimation for the spectrogram)
- Smart playlists, play-count import where devices expose it
- Profile marketplace: share `profiles/*.toml` through the repository

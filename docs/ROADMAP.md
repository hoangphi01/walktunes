# ROADMAP

| Field | Value |
|---|---|
| Status | Draft |
| Owner | Maintainer |
| Last reviewed | 2026-11-09 |
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
- [ ] Real-device test session on the ZX300 (probe dump first): answer PRD Q2, Q3 and Q5, and remove the related `[VERIFY]` markers — R-003
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

## M7+ — After 1.0
- Tier-2 profiles: older Sony models (community-verified)
- Tier-3 generic mass-storage players: write-order control (FR-055), 8.3 and FAT quirks
- Tier-4 MTP support (libmtp / Windows WPD)
- RISC-V vector (RVV) kernels once Rust intrinsics are stable; GUI on RISC-V once GPUI supports it
- DSD analysis (DSD → PCM decimation for the spectrogram)
- Smart playlists, play-count import where devices expose it
- Profile marketplace: share `profiles/*.toml` through the repository

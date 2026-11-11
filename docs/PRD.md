# PRD — Product Requirements

| Field | Value |
|---|---|
| Status | Draft v0.1 |
| Owner | Maintainer |
| Last reviewed | 2026-11-09 |
| Related | [ROADMAP](ROADMAP.md) · [ARCHITECTURE](ARCHITECTURE.md) · [EXTENSION §2](EXTENSION.md#2-device-support-tiers) |

---

## 1. Vision

> **Bring classic digital audio players back to life with a modern, offline-first library manager.**

Sony's classic (non-Android) Walkmans, such as the NW-A30/A40/A50 series,
NW-ZX300 and NW-WM1A/WM1Z, still offer excellent sound per unit of price, run
for days, and do one thing well. Sony has moved its software support to
Android Walkmans, and those behave much like phones. Classic players, and the
countless no-name players that are just a circuit board and a headphone jack,
are left with nothing but drag-and-drop copying.

WalkTunes is the missing companion app: **an iTunes-like experience without
the iTunes weight**, built in Rust, free under GPL-3.0, and available on macOS,
Windows and Linux.

**Quality and performance are priority #1.** WalkTunes is a native desktop
application, not a web app. It handles audio bit-exactly, verifies what it
copies, can prove whether a FLAC is intact and genuine, and uses the full
power of the CPU it runs on (x86-64, ARM64 and, later, RISC-V).

---

## 2. Problem statement

| # | Problem | Today |
|---|---|---|
| P1 | The music library is scattered across several computers and disks | No single view; the user cannot tell which copy is the most complete |
| P2 | Copying to the player is one-way and leaves no memory | "Copy and done": no diff, duplicates pile up, and the user does not know what is on the player |
| P3 | Metadata is messy | Wrong or missing tags, missing artwork and inconsistent Album Artist make the player's library views chaotic |
| P4 | Some music only exists as rips (for example MP3s from YouTube) | No tags, junk in titles ("Official Video"), low-quality or missing artwork |
| P5 | Device quirks are tribal knowledge | Artwork that does not show, unsupported formats, filename limits, playlist formats |
| P7 | You cannot tell whether a "lossless" file is genuine or intact | FLACs transcoded from MP3/YouTube rips, upsampled "hi-res" and corrupted downloads look identical in a file list. Spek helps, but only one file at a time and outside the library |
| P6 | Existing tools fall short | Sony Music Center is Windows-only and heavy; MusicBee is Windows-only; beets is CLI-only and does no device sync |

---

## 3. Users

| Persona | Description | Key need |
|---|---|---|
| **Revivalist** (primary) | Owns a classic Walkman (for example the ZX300), listens offline, has a lossless library plus some rips, and uses 2–3 computers | Know what is on the player; sync from any computer; clean tags |
| **Budget DAP user** | Uses a cheap no-name MP3 player | Files in the right order, readable names, a working shuffle and album view |
| **Archivist** | Keeps a large lossless library on a NAS | Dedupe, health reports, FLAC integrity checks, fake-lossless detection, never lose a file |
| **Tinkerer** | Runs custom firmware (for example MrWalkman's) and contributes device profiles | Profiles as data; a scriptable CLI |

---

## 4. Scope

### 4.1 In scope (v1.0)
- Library: multiple roots per machine, scanning, search, health checks, dedupe.
- Tags: view and edit, batch fixes, enrichment proposals (MusicBrainz, AcoustID, Cover Art Archive, LRCLIB), cleanup of filename-only files.
- Devices: pairing, profiles, push sync with dry run, pull from device, adoption of foreign files, playlists (M3U8), lyrics (LRC), device-safe artwork, optional transcoding.
- Quality inspection: FLAC integrity (CRC + MD5), Spek-like spectrogram viewer, detection of fake lossless, upsampled hi-res and padded bit depth.
- Multiple machines, with the device acting as the carrier.
- Desktop app (GPUI) and a CLI with feature parity. English and Vietnamese UI.

### 4.2 Out of scope (v1.0)
- Audio playback beyond a short preview. WalkTunes is a manager, not a player.
- Streaming services, or downloading from YouTube or other platforms. (A legal "Discover & Get" flow is planned after 1.0; see §10 and Q6.)
- Firmware flashing or modification, including custom firmware. WalkTunes only touches the user-data volumes.
- Cloud accounts or a server. Machine-to-machine networking.
- MTP devices (planned after 1.0; see [ROADMAP](ROADMAP.md)).
- Mobile apps.

---

## 5. Functional requirements

Priority: **P0** = required for 1.0, **P1** = should be in 1.0,
**P2** = later. **M** = the target milestone.

### 5.1 Library
| ID | Requirement | P | M |
|---|---|---|---|
| FR-001 | Add, remove, enable and disable library roots; root kinds `library`, `inbox`, `watch` | P0 | M1 |
| FR-002 | Incremental scan of 20k files, reading tags and audio properties | P0 | M1 |
| FR-003 | Search across title, artist and album, ignoring Vietnamese diacritics | P0 | M1 |
| FR-004 | Browse by Artist / Album / Track / Folder | P0 | M4 |
| FR-005 | Detect duplicates (exact, same-audio, alternate encode) and offer merge choices | P1 | M3 |
| FR-006 | Watch roots for changes while the app is running | P1 | M3 |
| FR-007 | Mark files as `stream_rip` / lossy and filter by source and quality | P1 | M3 |

### 5.2 Metadata and health
| ID | Requirement | P | M |
|---|---|---|---|
| FR-020 | Health report: missing tags, missing artwork, inconsistent album artist, bad track numbers, progressive or oversized artwork | P0 | M3 |
| FR-021 | Tag editor (single and batch) with preview and undo | P0 | M4 |
| FR-022 | MusicBrainz lookup that produces proposals the user accepts | P1 | M5 |
| FR-023 | AcoustID fingerprint identification for untagged files | P1 | M5 |
| FR-024 | Filename parser for downloads: strip "(Official Video)", "[HD]", "Lyrics" and similar; split "Artist - Title" | P0 | M5 |
| FR-025 | Fetch synced lyrics from LRCLIB and store them as `.lrc` | P1 | M5 |
| FR-026 | Fetch artwork from Cover Art Archive, or crop an existing image to a square | P1 | M5 |

### 5.3 Devices and sync
| ID | Requirement | P | M |
|---|---|---|---|
| FR-040 | Recognize a connected player automatically from passive USB/SCSI identity (ADR-0014): a classic Sony Walkman (ZX and A series) is identified within 3 s and the user only confirms its name; weaker matches are suggested | P0 | M2 |
| FR-041 | One device with several volumes (internal + SD); choose the target volume for each selection | P0 | M2 |
| FR-042 | Sync selection by all / artist / album / playlist / track / smart rule | P0 | M2 |
| FR-043 | Dry-run plan showing adds, replaces, removes and sizes, with free-space check, before execution | P0 | M2 |
| FR-044 | Resumable execution with per-file checkpoints and cancellation | P0 | M2 |
| FR-045 | Device manifest written atomically on every volume | P0 | M2 |
| FR-046 | Never remove files that WalkTunes did not write (foreign files) | P0 | M2 |
| FR-047 | Device-safe renditions: baseline JPEG artwork at the profile's maximum size, sanitized filenames | P0 | M3 |
| FR-048 | Transcode formats the profile does not support (requires ffmpeg) | P1 | M3 |
| FR-049 | Write playlists as M3U8 with relative paths | P1 | M5 |
| FR-050 | Copy LRC lyrics next to tracks | P1 | M5 |
| FR-051 | Pull: copy device files that are missing on this machine into the Inbox root | P0 | M6 |
| FR-052 | Adopt foreign files into the manifest | P1 | M6 |
| FR-053 | Safe eject from inside the app | P1 | M4 |
| FR-054 | Device storage view: used and free space by format and quality (charts) | P1 | M4 |
| FR-055 | Write-order control for players that play in FAT directory order | P2 | M7+ |
| FR-056 | `wtunes device probe --json`: dump the passive fingerprint of each connected volume (for support and community profiles) | P0 | M2 |
| FR-058 | Background helper (login item) detects a recognized or paired player being plugged in and, per setting, opens WalkTunes on that device, shows a notification (*Open* / *Sync now*), or does nothing (ADR-0015). Defaults: open for paired devices, notify for new ones | P1 | M4 (after R-001) |
| FR-059 | Settings: "When a player is connected" and "Start WalkTunes Helper at login" (on by default, can be turned off) | P1 | M4 |
| FR-057 | When a Sony device is connected in MTP mode only, tell the user how to switch to mass storage | P1 | M2 |

### 5.4 Quality inspection
| ID | Requirement | P | M |
|---|---|---|---|
| FR-090 | Integrity check: full decode test for all formats; for FLAC, frame CRCs and STREAMINFO MD5 (match / mismatch / absent); truncation detection | P0 | M3 |
| FR-091 | Spectrogram viewer (Spek-like): time × frequency, dBFS colour scale, kHz axis, per channel or mid, adjustable FFT size and window, linear or log frequency | P0 | M4 |
| FR-092 | Fake-lossless detection: lowpass cutoff analysis with verdict, confidence and the evidence drawn on the spectrogram | P0 | M3 |
| FR-093 | Upsampled hi-res and padded bit-depth detection (effective sample rate and bits) | P1 | M3 |
| FR-094 | Fake stereo and clipping detection | P2 | M5 |
| FR-095 | Batch check of a selection or the whole library in the background, resumable, with a report and filters (failed, suspect, …) | P0 | M3 |
| FR-096 | "Mark as reviewed" for a verdict; never auto-delete or auto-hide files based on a verdict | P0 | M3 |
| FR-097 | Export a spectrogram as PNG and a check report as CSV/JSON | P1 | M4 |
| FR-098 | Optionally check files before syncing and warn about failed integrity | P1 | M4 |

### 5.5 App and CLI
| ID | Requirement | P | M |
|---|---|---|---|
| FR-070 | CLI `wtunes` covering scan, health, devices, plan, sync, pull, with `--json` | P0 | M1–M6 |
| FR-071 | Desktop app with Library, Devices, Sync, Health, Jobs and Settings screens | P0 | M4 |
| FR-072 | English and Vietnamese UI, switchable at runtime | P0 | M4 |
| FR-073 | Light and dark themes that follow the OS | P0 | M4 |
| FR-074 | Jobs panel showing progress, cancel and history | P0 | M4 |
| FR-075 | Diagnostics bundle export | P1 | M4 |
| FR-076 | Start page with live tiles (device free space, tracks pending, health issues) and pivot-based hubs, in the Tile design language | P0 | M4 |

---

## 6. Non-functional requirements

| ID | Requirement |
|---|---|
| NFR-001 | **Zero data loss.** No code path deletes a library original. Device deletions only affect manifest-owned files and go through the ops log. See [SECURITY §3](SECURITY.md#3-data-safety-the-1-risk). |
| NFR-002 | **Performance:** see the budgets in [OPTIMIZATION §1](OPTIMIZATION.md#1-performance-budgets). For example, a cold scan of 10k files takes under 60 s on SSD, and the UI stays at 60 fps on a 50k-track list. |
| NFR-003 | **Offline-first:** every core feature works without a network. Network features are opt-in for each provider. |
| NFR-004 | **Privacy:** no telemetry. The only network calls are to the providers the user enabled. |
| NFR-005 | **Portability:** macOS 13+ (Apple Silicon and Intel), Windows 10 22H2+ x64 and ARM64 `[VERIFY GPUI]`, Linux x64 and ARM64 (Wayland and X11). RISC-V 64 Linux: CLI best-effort. See [ARCHITECTURE §8.1](ARCHITECTURE.md#81-cpu-architectures). |
| NFR-006 | **Accessibility:** keyboard-navigable, a visible focus ring, contrast ≥ 4.5:1, and reduced-motion respected. |
| NFR-007 | **Resilience:** unplugging a device mid-sync never corrupts the manifest, and the next attach resumes or rolls back cleanly. |
| NFR-008 | **License:** GPL-3.0-or-later, with dependencies compatible with it. |
| NFR-009 | **Native performance:** native compiled app only (no webview / Electron). DSP is SIMD-accelerated with runtime CPU dispatch; inspection budgets B-12…B-16 in [OPTIMIZATION](OPTIMIZATION.md#1-performance-budgets). |
| NFR-010 | **Bit-exactness and numerical consistency:** copies without a rendition are bit-identical (hash-verified). Analysis verdicts are identical across x86-64 and ARM64, and spectrogram values agree within 0.1 dB. |

---

## 7. Success metrics (for v1.0)

- Syncing a 2,000-track selection to a ZX300 succeeds, and **every** synced track shows correct artwork, title and album grouping on the device `[VERIFY]`.
- A second machine attaching the same device can list its contents and pull the missing tracks.
- Zero open data-loss bugs at release.
- At least 3 device profiles are community-verified: ZX300, one A-series model, and one generic player.

---

## 8. Risks

| Risk | Impact | Mitigation |
|---|---|---|
| **Trademark:** "Walkman" is a registered trademark of Sony | Possible request to rename | Product renamed to **WalkTunes** (2026-11-09). Use "Walkman" only descriptively ("works with Sony Walkman"), never as part of the product name or logo. Keep the "not affiliated with Sony" notice. |
| GPUI API churn and thin docs | Slower UI work | ADR-0001 mitigations: pin the version, isolate it in kit/console, CLI parity |
| Hardware behaviour is undocumented | Wrong assumptions (artwork, playlists) | `[VERIFY]` markers, a real-device test plan, community profiles |
| A data-loss bug | Destroys user trust | SECURITY §3 rules, ops log, trash, property tests, virtual-device tests |
| ffmpeg availability or licensing per OS | Transcoding unavailable | Optional feature; bundle per OS ([RELEASE](RELEASE.md)) |
| Provider rate limits or ToS | Enrichment failures | Gate rate limiting, caching, clear attribution |

---

## 9. Open questions

| # | Question | Owner | Target |
|---|---|---|---|
| Q1 | ~~Final public product name~~ **Resolved 2026-11-09: WalkTunes** | Maintainer | Done |
| Q2 | **Partly answered (R-003):** the ZX300 declares playlists as m3u8 content with a `.m3u` extension. Still open: required location and path forms (relative / absolute, separators) `[VERIFY]` | Hardware test | M2 |
| Q3 | Maximum embedded artwork size or resolution that the ZX300 displays reliably `[VERIFY]` | Hardware test | M2 |
| Q4 | Smart-selection rule language (simple filters vs. a small DSL) | Design | M5 |
| Q5 | **ZX300 answered (R-003 part 1):** PID 0x0C70, iProduct and INQUIRY product `WALKMAN`, LUN 0 internal / LUN 1 SD, INQUIRY revision `1.00` ≠ firmware (firmware comes from `default-capability.xml`). Still open: A30/A40/A50/WM1 PIDs and capability model names `[VERIFY]` | Hardware test | M2 |
| Q6 | Should WalkTunes ever download music itself (SmartTube- or lucida.to-style) instead of only linking to legal sources and importing what the user saves? Recommendation: **no**. Copyright, platform terms and DRM rules apply, and a DMCA takedown of the repository (the youtube-dl precedent, 2020) would end the project. See §10 and [ADR-0016](ARCHITECTURE.md#adr-0016-discover--get-legal-sources-and-inbox-no-built-in-ripping-proposed) | Maintainer | Before M8 |

---

## 10. Future vision (after 1.0): Discover, Get and update on the go

The maintainer's long-term wish (2026-11-12): find a song **inside WalkTunes**, in the
spirit of SmartTube or lucida.to, get it into the library, and have it land on the ZX300
right away, **on the go**. The requirements below keep that experience while respecting
copyright and the legal constraints in Q6 and ADR-0016.

| ID | Requirement | P | M |
|---|---|---|---|
| FR-100 | **Unified search:** one search box over the local library **and** the MusicBrainz catalog, showing tracks and albums you do not own yet ("missing from your library") | P2 | M8 |
| FR-101 | **Get from legal sources:** for a missing album, show where it can be bought or downloaded DRM-free (Bandcamp, Qobuz store, 7digital, HDtracks, the artist's own site, free/CC catalogs) and open the page; never scrape or rip a streaming service | P2 | M8 |
| FR-102 | **Inbox watch folder:** files saved into the Inbox root (from a store, a CD rip, any tool the user chooses) are imported automatically: tagged via enrichment proposals, checked by `wt-inspect` (fake lossless, integrity), then queued for the device | P2 | M8 |
| FR-103 | **Update on the go:** when a paired player is plugged into any computer running WalkTunes, the Helper (FR-058) offers the pending additions as a ready-made plan ("12 new tracks for My ZX300 · Sync"), so the library on the player stays current without opening anything | P2 | M8 |
| FR-104 | **Wish list:** remember albums searched for but not yet available, and notify when they appear in the Inbox | P2 | M8 |
| FR-105 | **Mobile companion (idea):** an Android app that manages the ZX300 over a USB-OTG cable (the player mounts as mass storage) for updates away from a computer `[VERIFY feasibility]`. The ZX300 has no Wi-Fi, so there are no wireless updates | P2 | M9+ |

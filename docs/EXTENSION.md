# EXTENSION — Adding Devices, Formats, Providers and Services

| Field | Value |
|---|---|
| Status | Draft |
| Owner | `wt-dock`, `wt-forge`, `wt-enrich` |
| Last reviewed | 2026-11-09 |
| Related | [ARCHITECTURE](ARCHITECTURE.md) · [DATABASE §6](DATABASE.md#6-device-manifest) · [TESTING §4](TESTING.md#4-virtual-devices) |

---

## 1. Extension points

| You want to support… | Extension point | Kind | Section |
|---|---|---|---|
| A new player model | `profiles/<vendor>.<family>.<model>.toml` | **Data** (no Rust needed) | §3 |
| Unusual device behaviour | `DeviceHook` trait in `wt-dock` | Rust | §3.4 |
| A new audio format | `wt-crawler` (read) + `wt-forge` (convert) + `wt-vault` (`audio_hash`) | Rust | §4 |
| A new metadata, artwork or lyrics source | `EnrichProvider` trait in `wt-enrich` | Rust | §5 |
| A new quality detector (for example, a new lossy-codec signature) | `Detector` trait in `wt-inspect` | Rust | §7 |
| A whole new capability | A new service crate | Rust + ADR | §6 |

There are no runtime plugins in v1. Extensions are compiled in, or shipped as
profile data. This keeps the security surface small ([SECURITY](SECURITY.md)).

---

## 2. Device support tiers

| Tier | Devices | Connection | Status |
|---|---|---|---|
| **1 — Classic Walkman** | Sony NW-A30, NW-A40, NW-A50 series, **NW-ZX300**, NW-WM1A, NW-WM1Z (non-Android firmware, about 2016–2018) | USB Mass Storage `[VERIFY per model]` | Target for 1.0. The ZX300 is the reference device |
| **2 — Older Sony** | Earlier NW-A, NWZ-A/E/S/X series and others | Mix of mass storage and MTP (per model) | After 1.0. Community-verified profiles |
| **3 — Generic mass-storage players** | No-name / budget players, Rockbox devices, and anything that mounts as a USB drive | USB Mass Storage | After 1.0 via `generic.msc`; usable earlier with manual pairing |
| **4 — MTP players** | Devices that do not mount as a drive | MTP | Later (needs libmtp / Windows WPD) |

Note: Tier-1 Walkmans are often called "RTOS Walkmans" in the community.
Technically they run Sony's embedded (Linux-based) firmware rather than
Android. WalkTunes calls them **Classic** and never modifies their firmware.

---

## 3. Device profiles

### 3.1 Profile file format

A profile describes **capabilities** and **conventions**. Every capability
carries a `verified` flag. Unverified values are conservative guesses, and the
UI shows them with a "not yet verified on hardware" badge.

```toml
# profiles/sony.classic.zx300.toml
id          = "sony.classic.zx300"
name        = "Sony NW-ZX300"
family      = "sony.classic"
tier        = 1
inherits    = "sony.classic.base"        # shared Classic Walkman defaults
maintainers = ["@hoangphi01"]

[detect]                                 # heuristics only suggest; the user confirms
volume_labels   = ["WALKMAN"]            # [VERIFY]
sd_volume_labels = ["SD_CARD"]          # [VERIFY]
required_dirs   = ["MUSIC"]
marker_files    = []                     # [VERIFY] any Sony-specific files at volume root
usb_ids         = [{ vid = 0x054c, pid = "TBD" }]   # [VERIFY]; informational in v1

[layout]
music_dir       = "MUSIC"
path_template   = "{album_artist}/{album}/{disc_track} {title}"
playlist_dir    = "MUSIC/Playlists"      # [VERIFY]
lyrics          = "sidecar-lrc"          # same basename, .lrc  [VERIFY]

[filesystem]
types           = ["exfat", "fat32"]     # [VERIFY] internal vs SD
max_path_bytes  = 255                    # [VERIFY]
illegal_chars   = ':*?"<>|\\'
case_insensitive = true
write_order_matters = false              # device builds its own database

[formats]                                # native playback  — all [VERIFY]
native   = ["mp3", "aac", "alac", "flac", "wav", "aiff", "dsf", "dff", "wma"]
maybe    = ["ape", "mqa"]                # depends on firmware version
unsupported = ["ogg", "opus"]
max_pcm  = { sample_rate = 384000, bit_depth = 32 }
max_dsd  = "dsd256"

[transcode]                              # what Forge does with unsupported/oversized input
lossless_target = "flac"                 # e.g. opus is lossy -> see lossy_target
lossy_target    = { codec = "aac", kbps = 256 }
downsample_above = { sample_rate = 384000 }

[artwork]
mode         = "embedded"
format       = "jpeg-baseline"           # progressive JPEG suspected not to display  [VERIFY]
max_px       = 600                       # conservative default  [VERIFY]
max_bytes    = 512000                    # [VERIFY]

[tags]
prefer        = "id3v2.3"                # for MP3; Vorbis comments for FLAC
album_artist  = "required"               # groups albums correctly on device
multi_value_separator = "; "

[verified]                               # filled in from RESULT.md entries
by     = []                              # ["R-003"]
firmware = []                            # e.g. ["2.03"]
```

### 3.2 Profile inheritance
- `sony.classic.base`: shared defaults for Tier 1.
- `generic.msc`: the Tier-3 baseline. Every field is user-editable in the pairing dialog (music folder, formats, artwork size, write order, filename charset, `8.3` names).
- A child profile overrides keys. Arrays replace the parent's array; they do not merge.

### 3.3 Adding a profile (checklist)
1. Copy the closest profile and give it a new `id` (`vendor.family.model`, lowercase).
2. Fill in `[detect]`, `[layout]` and `[formats]`. Mark everything you have not tested `[VERIFY]` with a comment.
3. Add a **virtual device** fixture under `fixtures/devices/<id>/` ([TESTING §4](TESTING.md#4-virtual-devices)).
4. Run `cargo test -p wt-dock profiles` (this checks schema validation and detection against fixtures).
5. On real hardware, follow the **hardware test script** (§3.5) and record the outcome in [RESULT.md](RESULT.md).
6. Update the device table in [STATUS.md](STATUS.md) and the supported-devices list in both USER_GUIDEs.

### 3.4 `DeviceHook` (only when data cannot express it)

```rust
pub trait DeviceHook: Send + Sync {
    /// Adjust a planned rendition (e.g. special tag frames a device needs).
    fn adjust_rendition(&self, _r: &mut RenditionSpec) {}
    /// Extra files to write after a sync (e.g. a device-specific index).
    fn post_sync(&self, _ctx: &PostSyncCtx) -> Result<(), DockError> { Ok(()) }
}
```
Register the hook in `wt-dock/src/hooks/mod.rs` keyed by profile id. Hooks
must not delete files.

### 3.5 Hardware test script (per device)
1. Pair, then sync the fixture album set `fixtures/hwtest/` (all formats, multi-disc, Vietnamese titles, long names, artwork variants: baseline/progressive, 300/600/1000/1500 px, PNG).
2. On the device, record: which tracks appear, the artwork shown for each variant, title and diacritics rendering, album grouping, multi-disc order, playlist visibility, lyrics display, and database rebuild time.
3. Unplug during a sync, re-attach, and resume.
4. Write R-entry, update the profile values, and remove `[VERIFY]` markers.

---

## 4. Adding an audio format
1. `wt-core`: add an `AudioFormat` / `Codec` variant.
2. `wt-crawler`: make sure `lofty` (tags) and `symphonia` (properties) handle it; otherwise add a fallback.
3. `wt-vault`: add the `audio_hash` byte range rule ([DATABASE §4](DATABASE.md#4-track-identity-ladder)).
4. `wt-forge`: add ffmpeg arguments for transcoding *from* it (and *to* it, if it is a target).
5. Profiles: list it under `native` / `unsupported` where known.
6. Fixtures: add a generated sample in `xtask fixtures`.

## 5. Adding an enrichment provider

```rust
pub trait EnrichProvider: Send + Sync {
    fn id(&self) -> &'static str;                  // "lrclib"
    fn kinds(&self) -> &'static [ProposalKind];    // [Lyrics]
    fn requires_network(&self) -> bool { true }
    fn lookup(&self, q: &EnrichQuery, gate: &Gate) -> Result<Vec<Proposal>, EnrichError>;
}
```
Rules:
- All HTTP goes through `Gate`. The provider gets consent and rate limiting for free.
- Return **proposals with a confidence score**. Never write files or Ledger rows directly.
- Document the provider's terms, its rate limit and the attribution it requires in [SECURITY §6](SECURITY.md#6-network-and-third-party-services).
- Add a recorded-response test (fixtures under `fixtures/http/<provider>/`).

## 6. Adding a service
1. Write an ADR in [ARCHITECTURE §9](ARCHITECTURE.md#9-decision-log-adrs): the job, its layer, and why no existing crate fits.
2. Create `crates/wt-<name>` with a `README.md` stub that links to its ARCHITECTURE section.
3. Add it to the §3 table in [DOCS_GUIDE](DOCS_GUIDE.md#3-service-dictionary-the-mini-aws), to the [ERROR_CONTROL](ERROR_CONTROL.md) service keys, and to [GLOSSARY](GLOSSARY.md).
4. Expose it through `wt-sdk` if front ends need it.

## 7. Adding a quality detector

```rust
pub trait Detector: Send + Sync {
    fn id(&self) -> &'static str;                       // "lowpass-cutoff"
    fn version(&self) -> u32;                           // bump on any threshold change
    /// Called once per file with the single-pass products of a full check.
    fn evaluate(&self, input: &AnalysisInput) -> Finding;
}
// AnalysisInput: stream info, Welch-averaged spectrum, per-channel statistics,
// bit-usage histogram, integrity results. Detectors never decode or read files.
```

Rules:
1. A detector is a **pure function** of `AnalysisInput`. All heavy work stays in the shared single pass, so adding a detector does not add a decode.
2. Return evidence (numbers plus an optional overlay for the spectrogram), a verdict contribution and a confidence. **Never** return "delete" or "hide".
3. Add labelled fixtures for both a positive and a false-positive case ([TESTING §7.2](TESTING.md#72-labelled-inspection-fixtures)), and record the calibration in RESULT.
4. Bump `analyzer_version` (the sum of detector versions), so that stored results are re-evaluated.
5. Explain the detector in the "How we decide" panel strings (EN + VI).

### 7.1 Current detectors and thresholds (initial, to be calibrated in R-005)

| Detector | Evidence | Initial rule |
|---|---|---|
| `lowpass-cutoff` | Highest frequency where the averaged spectrum sits ≥ 10 dB above the noise floor, plus the edge steepness | A brickwall edge (≥ 30 dB drop within 500 Hz) below 19.5 kHz → `likely_lossy_source`; 19.5–21 kHz → `suspect`; no brickwall → no finding. Typical encoder cutoffs: ~16 kHz (MP3 128 kbps), ~19–20 kHz (MP3 256–320 kbps) `[VERIFY in R-005]` |
| `upsampled` | Effective bandwidth vs. the declared sample rate | Rate > 48 kHz and no content above ~22–24 kHz → `upsampled` (effective rate 44.1 or 48 kHz) |
| `padded-bits` | Bit-usage histogram | 24-bit with the lowest 8 bits always zero → `padded_bits` (effective 16) |
| `fake-stereo` | L/R correlation and difference energy | L = R (difference < -100 dBFS) → finding |
| `clipping` | Runs of consecutive full-scale samples | Runs ≥ 3 samples counted; reported, not a verdict |

Known false-positive sources: old or analog-mastered recordings with little
high-frequency content, intentionally low-passed productions, and very quiet
tracks. That is why verdicts are always "suspect / likely", and why the user
can mark them as reviewed.

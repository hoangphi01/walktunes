# GLOSSARY

| Field | Value |
|---|---|
| Status | Stable (living) |
| Owner | Everyone |
| Last reviewed | 2026-11-09 |

Terms are listed alphabetically. Service names are listed in
[DOCS_GUIDE §3](DOCS_GUIDE.md#3-service-dictionary-the-mini-aws).

| Term | Meaning |
|---|---|
| **App bar** | The bottom command bar of a hub, with square icon-and-label buttons. |
| **ADR** | Architecture Decision Record. A numbered decision in [ARCHITECTURE §9](ARCHITECTURE.md#9-decision-log-adrs). |
| **Adopt** | Add a *foreign file* already on a device to the manifest after hashing it, so WalkTunes manages it from then on. |
| **Album Artist** | The tag that groups an album's tracks, even when individual track artists differ. Players need it for correct album views. |
| **audio_hash** | blake3 hash of the audio payload only, excluding tags. It stays the same when tags are edited. See [DATABASE §4](DATABASE.md#4-track-identity-ladder). |
| **AVX2 / SSE4.1** | SIMD instruction sets on x86-64 processors; `wt-dsp` selects them at runtime when present. |
| **Baseline JPEG** | A non-progressive JPEG encoding. The safest artwork format for older players. |
| **Checkpoint** | A point during a sync where progress is saved (manifest + `sync_runs`) so the sync can resume after an interruption. |
| **Classic Walkman** | Sony Walkmans with non-Android firmware (for example NW-A30/A40/A50, NW-ZX300, NW-WM1A/Z). The community often calls them "RTOS Walkmans"; technically they run Sony's embedded Linux-based firmware. **Tier 1**. |
| **content_hash** | blake3 hash of a whole file. It changes when tags change. |
| **dBFS** | Decibels relative to full scale; 0 dBFS is the loudest possible digital level. Spectrograms show -120…0 dBFS. |
| **Device** | A paired player. It may expose several *volumes*. Identified by `device_id` in `device.json`. |
| **Destination / SubIdentification** | Sony's regional variant code for a model (for example `00010000`). Changing the region changes it; WalkTunes treats all destinations of a model as the same model. |
| **default-capability.xml** | Sony's self-description file at the root of every Walkman volume: model series, firmware, storage type (`INTERNAL`/`CARD`), formats, playlists, lyrics. Read-only for WalkTunes. |
| **DevIcon.fil / DevLogo.fil** | Icon and logo files Sony places at each volume root. Markers for detection; protected. |
| **Device file** | A file on a device that is tracked in the device manifest. |
| **Dry run** | A sync plan shown to the user without executing anything. |
| **Family match** | Detection that is sure about the device family (for example `sony.classic`) but not yet the exact model. Enough to sync safely; the model is chosen from capacity-ordered candidates. See [EXTENSION §3.1.2](EXTENSION.md#312-family-match-and-model-candidates). |
| **Fake lossless** | A lossless file (for example FLAC) made from a lossy source (MP3, AAC, a YouTube rip). It is bigger but no better than the source. Detected by its lowpass cutoff. |
| **FFT / STFT** | Fast Fourier Transform turns a block of samples into frequencies; the Short-Time Fourier Transform repeats it along the track to build a spectrogram. |
| **FLAC MD5 signature** | An MD5 of the decoded audio stored in the FLAC STREAMINFO block; recomputing it proves the audio is intact. |
| **Foreign file** | A file on a device that is not in the WalkTunes manifest (copied by hand or by another tool). WalkTunes never removes it. |
| **Generation** | A counter in the device manifest that increases by one on every successful write. Used to detect rollbacks. |
| **Identity ladder** | The ordered evidence used to decide that two files are the same track (MBID → audio_hash → fingerprint → heuristic). |
| **Helper (WalkTunes Helper)** | The planned background login item (`wt-helper`) that notices a player being plugged in and opens WalkTunes or shows a notification, like iTunesHelper. See [ARCHITECTURE §3.17.1](ARCHITECTURE.md#3171-wt-helper--plug-in-detection-planned-adr-0015). |
| **Inbox root** | A root where files pulled from a device land on this machine. |
| **Ledger** | The local SQLite catalog (`wt-ledger`). |
| **iSerialNumber / iProduct** | USB string descriptors with the device's serial number and product name. WalkTunes uses the serial to group volumes and recognize a player again. |
| **Live tile** | A tile on the Start page that shows current information (for example, free space on a device) and may occasionally "peek" to a second face. |
| **Lowpass cutoff** | The frequency above which a lossy encoder removes all content, visible as a sharp edge in the spectrogram. |
| **Lossless / lossy** | Lossless (FLAC, ALAC, WAV, AIFF, DSD) keeps all audio data. Lossy (MP3, AAC, Opus, Vorbis) discards some. |
| **LRC** | A lyrics file with timestamps (`[mm:ss.xx] line`), stored next to the track with the same base name. |
| **LUN** | Logical Unit Number: one USB mass-storage device can expose several drives (for example internal storage and SD card) as LUN 0, 1, … |
| **Machine** | One computer running WalkTunes. It has a `MachineId` (UUID) created at first run. |
| **Manifest** | `/.walktunes/manifest.json` on each device volume: the record of what WalkTunes put there. See [DATABASE §6](DATABASE.md#6-device-manifest). |
| **MBID** | MusicBrainz Identifier (UUID) for a recording, release or artist. |
| **NVP** | Non-volatile parameters stored in a Sony player (model ID, destination…). WalkTunes never reads or writes them directly. |
| **NW_WM_FW.UPG** | Sony's encrypted firmware update file, placed at the volume root by the official updater. Protected: WalkTunes never touches it. |
| **MSC / Mass storage** | USB Mass Storage Class. The device mounts like a USB drive. Tier 1–3 devices use it. |
| **MTP** | Media Transfer Protocol. Devices that do not mount as drives. **Tier 4**, after 1.0. |
| **NEON** | The SIMD unit present on every ARMv8-A (ARM64) CPU, including Apple Silicon. |
| **Origin** | The machine that first put a file on a device (recorded in the manifest). |
| **Padded bit depth** | A 24-bit file whose lowest 8 bits are always zero, so it really carries only 16-bit audio. |
| **Pivot** | A hub's horizontal set of large lowercase headers (for example "albums · artists · tracks"); switching pivots slides the content. |
| **Pairing** | Confirming that a volume belongs to a device and writing `device.json`. |
| **Profile** | A data file (`profiles/*.toml`) that describes a device's capabilities and conventions. See [EXTENSION §3](EXTENSION.md#3-device-profiles). |
| **Proposal** | A suggested change (tags, artwork, lyrics, identity) from an enrichment provider. It is applied only after the user accepts it. |
| **Pull** | Copying device files that this machine lacks into its Inbox root. |
| **Push** | Syncing from the library to a device. |
| **RISC / RISC-V** | RISC = reduced instruction set computer. ARM (AArch64) is a RISC architecture; RISC-V is an open, royalty-free RISC architecture. x86-64 is CISC. |
| **Runtime dispatch** | Choosing the fastest code path (AVX2, NEON, scalar…) when the program runs, based on the CPU, so one binary is fast on every machine. |
| **RVV** | The RISC-V Vector extension. Not yet usable from stable Rust intrinsics. |
| **Rendition** | A device-specific copy of a library file (transcoded, artwork normalized, tags simplified, filename sanitized). Originals are untouched. |
| **Root** | A folder registered in the library (`library`, `inbox` or `watch`). |
| **Safety error** | An error of recovery class `Abort` that stops a job to protect data. See [ERROR_CONTROL §4](ERROR_CONTROL.md#4-recovery-classes). |
| **Selection** | What the user wants on a device (all, artists, albums, playlists, tracks, smart rules). |
| **Service** | One crate with one job (see DOCS_GUIDE §3). |
| **SIMD** | Single Instruction, Multiple Data: CPU instructions that process several samples at once. |
| **Spectrogram** | A picture of a track: time left to right, frequency bottom to top, loudness as colour. WalkTunes draws it in the Inspect hub, similar to Spek. |
| **Stream rip** | A file obtained by recording or downloading from a streaming or video platform (for example, an MP3 from YouTube). Usually lossy, with poor tags. |
| **Sync plan** | The list of operations (`Add`, `Replace`, `Retag`, `Remove`, `WritePlaylist`, `WriteLyrics`) computed for a sync. |
| **Upsampled** | A "hi-res" file (for example 96 kHz) made from a lower-rate source; it has no real content above the original Nyquist frequency. |
| **Tile** | A square or rectangular solid block on a 72 px grid, the basic visual unit of the WalkTunes design language ([STYLE_RULE §7.1](STYLE_RULE.md#71-tiles)). |
| **Tier** | Device support level 1–4. See [EXTENSION §2](EXTENSION.md#2-device-support-tiers). |
| **Track** | A logical recording, independent of files. |
| **`[VERIFY]`** | A doc marker for a hardware claim that has not yet been tested on a real device. |
| **Virtual device** | A folder that acts as a mounted device in tests. See [TESTING §4](TESTING.md#4-virtual-devices). |
| **UX rule** | A numbered design decision rule (`UX-01`…) derived from a UX law. See [STYLE_RULE Part C](STYLE_RULE.md#part-c--ux-laws-decision-rules). |
| **SCSI INQUIRY** | Standard identity data (vendor, product, revision) a USB drive reports; the OS caches it. The ZX300 reports `SONY` / `WALKMAN` / `1.00` (the revision is not the firmware version). |
| **VID / PID** | USB Vendor ID and Product ID. Sony's VID is `0x054C`; the PID identifies the model and mode. |
| **Volume** | One mounted file system (for example, a Walkman's internal storage or its SD card). |

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
| **Baseline JPEG** | A non-progressive JPEG encoding. The safest artwork format for older players. |
| **Checkpoint** | A point during a sync where progress is saved (manifest + `sync_runs`) so the sync can resume after an interruption. |
| **Classic Walkman** | Sony Walkmans with non-Android firmware (for example NW-A30/A40/A50, NW-ZX300, NW-WM1A/Z). The community often calls them "RTOS Walkmans"; technically they run Sony's embedded Linux-based firmware. **Tier 1**. |
| **content_hash** | blake3 hash of a whole file. It changes when tags change. |
| **Device** | A paired player. It may expose several *volumes*. Identified by `device_id` in `device.json`. |
| **Device file** | A file on a device that is tracked in the device manifest. |
| **Dry run** | A sync plan shown to the user without executing anything. |
| **Foreign file** | A file on a device that is not in the WalkTunes manifest (copied by hand or by another tool). WalkTunes never removes it. |
| **Generation** | A counter in the device manifest that increases by one on every successful write. Used to detect rollbacks. |
| **Identity ladder** | The ordered evidence used to decide that two files are the same track (MBID → audio_hash → fingerprint → heuristic). |
| **Inbox root** | A root where files pulled from a device land on this machine. |
| **Ledger** | The local SQLite catalog (`wt-ledger`). |
| **Live tile** | A tile on the Start page that shows current information (for example, free space on a device) and may occasionally "peek" to a second face. |
| **Lossless / lossy** | Lossless (FLAC, ALAC, WAV, AIFF, DSD) keeps all audio data. Lossy (MP3, AAC, Opus, Vorbis) discards some. |
| **LRC** | A lyrics file with timestamps (`[mm:ss.xx] line`), stored next to the track with the same base name. |
| **Machine** | One computer running WalkTunes. It has a `MachineId` (UUID) created at first run. |
| **Manifest** | `/.walktunes/manifest.json` on each device volume: the record of what WalkTunes put there. See [DATABASE §6](DATABASE.md#6-device-manifest). |
| **MBID** | MusicBrainz Identifier (UUID) for a recording, release or artist. |
| **MSC / Mass storage** | USB Mass Storage Class. The device mounts like a USB drive. Tier 1–3 devices use it. |
| **MTP** | Media Transfer Protocol. Devices that do not mount as drives. **Tier 4**, after 1.0. |
| **Origin** | The machine that first put a file on a device (recorded in the manifest). |
| **Pivot** | A hub's horizontal set of large lowercase headers (for example "albums · artists · tracks"); switching pivots slides the content. |
| **Pairing** | Confirming that a volume belongs to a device and writing `device.json`. |
| **Profile** | A data file (`profiles/*.toml`) that describes a device's capabilities and conventions. See [EXTENSION §3](EXTENSION.md#3-device-profiles). |
| **Proposal** | A suggested change (tags, artwork, lyrics, identity) from an enrichment provider. It is applied only after the user accepts it. |
| **Pull** | Copying device files that this machine lacks into its Inbox root. |
| **Push** | Syncing from the library to a device. |
| **Rendition** | A device-specific copy of a library file (transcoded, artwork normalized, tags simplified, filename sanitized). Originals are untouched. |
| **Root** | A folder registered in the library (`library`, `inbox` or `watch`). |
| **Safety error** | An error of recovery class `Abort` that stops a job to protect data. See [ERROR_CONTROL §4](ERROR_CONTROL.md#4-recovery-classes). |
| **Selection** | What the user wants on a device (all, artists, albums, playlists, tracks, smart rules). |
| **Service** | One crate with one job (see DOCS_GUIDE §3). |
| **Stream rip** | A file obtained by recording or downloading from a streaming or video platform (for example, an MP3 from YouTube). Usually lossy, with poor tags. |
| **Sync plan** | The list of operations (`Add`, `Replace`, `Retag`, `Remove`, `WritePlaylist`, `WriteLyrics`) computed for a sync. |
| **Tile** | A square or rectangular solid block on a 72 px grid, the basic visual unit of the WalkTunes design language ([STYLE_RULE §7.1](STYLE_RULE.md#71-tiles)). |
| **Tier** | Device support level 1–4. See [EXTENSION §2](EXTENSION.md#2-device-support-tiers). |
| **Track** | A logical recording, independent of files. |
| **`[VERIFY]`** | A doc marker for a hardware claim that has not yet been tested on a real device. |
| **Virtual device** | A folder that acts as a mounted device in tests. See [TESTING §4](TESTING.md#4-virtual-devices). |
| **UX rule** | A numbered design decision rule (`UX-01`…) derived from a UX law. See [STYLE_RULE Part C](STYLE_RULE.md#part-c--ux-laws-decision-rules). |
| **Volume** | One mounted file system (for example, a Walkman's internal storage or its SD card). |

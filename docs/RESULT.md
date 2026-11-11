# RESULT — Measured Outcomes

| Field | Value |
|---|---|
| Status | Live log |
| Owner | Maintainer |
| Last reviewed | 2026-11-09 |

This file records **evidence**: milestone results, benchmarks, spikes, and
tests on real hardware. Opinions go in [LESSONS.md](LESSONS.md); plans go in
[ROADMAP.md](ROADMAP.md).

Entries are append-only and use stable IDs (`R-001`, `R-002`, …). Newest
entries go at the bottom.

## Template

```markdown
### R-00X — <title>
- **Date:** YYYY-MM-DD   **Milestone:** M?   **Commit:** <sha>
- **Question:** what did we want to know?
- **Setup:** hardware, OS, dataset (size, formats), device + firmware version
- **Method:** commands / steps (reproducible)
- **Result:** numbers, screenshots, pass/fail per criterion
- **Conclusion:** decision taken; docs updated (links); [VERIFY] markers removed
```

## Planned entries

| ID | Title | Milestone | State |
|---|---|---|---|
| R-001 | GPUI spike: font, icons, 50k-row list, chart on 3 OSes | M0 | Planned |
| R-002 | Scan benchmark: 10k / 50k files, cold and warm | M1 | Planned |
| R-003 | ZX300 hardware session: **probe dump (USB/SCSI identity, LUNs, labels)**, playlists, artwork limits, formats, MUSIC layout, DB rebuild time | M2 | **Part 1 done** (probe); playlists/artwork/DB rebuild pending |
| R-004 | DSP spike: spectrogram speed NEON vs AVX2, RISC-V scalar build | M0 | Planned |
| R-005 | Inspection detector calibration on labelled fixtures | M3 | Planned |
| R-006 | Static analysis of the Sony WALKMAN Firmware Updater (ZX300 package) | M0 | **Done** |

## Entries

### R-006 — Static analysis of the Sony WALKMAN Firmware Updater (ZX300 package)
- **Date:** 2026-11-11   **Milestone:** M0 (input to M2 `wt-dock`)   **Commit:** docs only
- **Question:** how does Sony's own software recognize a classic Walkman, and what can WalkTunes reuse safely?
- **Setup:** MrWalkman "StockRevert" packages for the NW-ZX300 (DMP-300, DMP-300 FE/FEv2, Project Z, Midnight, Ext. Tuning), downloaded by the maintainer. They are kept **outside the repository** (`../research/`), never committed (L-003).
- **Method:** read-only static analysis on macOS. We read `SWUpdate.xml`, inspected the `NW_WM_FW.UPG` header, and extracted ASCII and UTF-16 strings from `WmFwUpdater.dll` and `FirmwareUpdateTool.exe`. **Nothing was executed.** Cross-checked with public Rockbox `nwztools` documentation.
- **Result:**
  1. The updater identifies a Walkman by: SCSI **INQUIRY** (it checks that the vendor equals `"SONY    "`; it reads the Product ID and Product Revision, and compares the revision with the firmware version), USB string descriptors **`iProduct`** and **`iSerialNumber`**, and a drive → disk mapping via `IOCTL_STORAGE_GET_DEVICE_NUMBER` and SetupAPI `GUID_DEVINTERFACE_DISK`.
  2. A vendor SCSI command (`REQUEST DEVINFO`) returns `DevicePropertyProductInfo` (`NW-ZX300`, `NW-ZX300A`) and `SubIdentification` (`00000000`–`00030000`; regional destinations, which change after a region change).
  3. The update file `NW_WM_FW.UPG` is written to the **volume root**. This confirms a writable USB mass-storage volume.
  4. `NW_WM_FW.UPG` is encrypted (high-entropy content from byte 0).
  5. The updater also has an **MTP/WPD** code path, so some Walkmans enumerate as MTP devices.
  6. Public Rockbox research shows that Sony vendor SCSI commands can **read and write** NVP data (model ID, destination).
- **Conclusion:** WalkTunes identifies devices **passively** from what the OS already caches (INQUIRY strings, USB descriptors, volume metadata), and **never sends vendor or raw SCSI commands** ([ADR-0014](ARCHITECTURE.md#adr-0014-passive-hardware-fingerprinting-never-send-vendor-scsi-commands), SECURITY S-10). `NW_WM_FW.UPG` is a protected file. Exact strings, the PID and the LUN order stay `[VERIFY]` until the hardware probe in R-003.

### R-003 (part 1) — Real NW-ZX300 probe
- **Date:** 2026-11-11   **Milestone:** M2 input   **Commit:** see `git log` ("probe from real ZX300")
- **Question:** what does a real classic Walkman expose passively, and does `wtunes device probe` recognize it?
- **Setup:** maintainer's NW-ZX300, **firmware 2.02** (stock, region changed), with a 128 GB microSD card; MacBook (Apple Silicon), macOS 26.3.1, USB mass-storage mode.
- **Method:** read-only commands only: `diskutil list/info`, `system_profiler SPUSBHostDataType`, `ioreg -a -r -l -c IOUSBHostDevice`, root listings of both volumes, reading `default-capability.xml`. Then `wtunes device probe`. Redacted copies saved as fixtures (`fixtures/probe/macos/`, `fixtures/probe/capability/`).
- **Result:**

| Item | Observed |
|---|---|
| USB | VID `0x054C`, **PID `0x0C70`**, manufacturer `Sony`, product `WALKMAN`, `bcdDevice` 0x0100, USB 2.0 (480 Mbit/s), serial present |
| SCSI INQUIRY | `SONY` / `WALKMAN` / revision **`1.00`**: the revision is **not** the firmware version |
| LUNs | LUN 0 = internal (`disk4`, whole-disk FAT32, no partition table); LUN 1 = microSD (`disk5`, MBR, `disk5s1` exFAT). Same USB device and serial |
| Internal volume | label `WALKMAN`, **MS-DOS FAT32** (4 GiB per-file limit), 60.06 GB |
| SD volume | label **`Untitled`** (the card's own label, not `SD_CARD`), exFAT, 124.9 GB |
| Root files (both volumes) | **`default-capability.xml`**, `DevIcon.fil` (Windows icon), `DevLogo.fil` (PNG 150×32), `MUSIC/`; plus macOS/Windows litter (`.Spotlight-V100`, `.fseventsd`, `.Trashes`, `System Volume Information`) |
| `default-capability.xml` | class `WALKMAN/Standard/AudioOnly`, model **`NW-ZX300Series`**, vendor `Sony Corporation`, **firmware `2.02`**, storage `INTERNAL` / `CARD`, display 800×480, music path `\MUSIC\`, playlists **m3u8 written with `.m3u` extension**, lyrics `LRC`, SensMe, no Marlin DRM |
| Formats (from the file) | mp3, mp4/3gp/m4a (AAC), wma, wav/aiff/alac ≤ 384 kHz/32-bit, flac ≤ 384 kHz/24-bit, dsf/dff ≤ 11.2896 MHz, **ape ≤ 192 kHz/24-bit**; **no ogg/opus** |
| `wtunes device probe` before fix | not recognized (generic, 5–10 points): interim `diskutil` parse assumed `MediaName` = "SONY WALKMAN Media" (real: `WALKMAN`) and had no USB identity |
| After fix | **recognized, score 220**, model NW-ZX300, firmware 2.02, internal + SD grouped, roles from `storage type` |

- **Conclusion:** Classic Walkmans **describe themselves** in `default-capability.xml`; it is now the primary evidence (+45 family, +40 model) and the source of truth for formats, playlists, lyrics and firmware. macOS probe = `ioreg` (USB/SCSI per BSD name) + `diskutil` (volume facts). Resolved: PRD Q5 for the ZX300; Q2 partly (format/extension). Still `[VERIFY]`: playlist location and path forms, artwork limits (Q3), A-series PIDs and capability model names.
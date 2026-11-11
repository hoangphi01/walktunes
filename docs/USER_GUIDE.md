# User Guide

| Field | Value |
|---|---|
| Status | Preview: describes planned behaviour; the app is not released yet |
| Owner | Maintainer |
| Last reviewed | 2026-11-09 |
| Translation | [USER_GUIDE.vi.md](USER_GUIDE.vi.md) (keep in sync) |

> WalkTunes is under development. This guide describes how version 1.0 is
> designed to work. Sections marked *(planned)* do not exist yet.

---

## 1. What WalkTunes does

WalkTunes manages your offline music and keeps your player in sync. It
remembers what is on your player, so you can plug it into any of your
computers and see what is there, what is missing, and what changed.

- **Library:** all your music folders in one place, searchable, with duplicates and messy tags found for you.
- **Sync:** choose artists, albums or playlists, review the plan, sync. Only the differences are copied.
- **Your player remembers:** a small hidden folder on the player records what was synced and from which computer.
- **Clean-up:** fix tags and artwork, and tidy files downloaded from video sites.
- **Safe by design:** your original files are never deleted, and files you copied to the player by hand are never removed.

## 2. Supported players

| Player | Status |
|---|---|
| Sony NW-ZX300 | Primary target (1.0) |
| Sony NW-A30 / A40 / A50 series, NW-WM1A / WM1Z | Target (1.0), verification in progress |
| Older Sony players, generic USB-drive MP3 players | After 1.0 *(planned)*; basic use via "Generic player" |
| Android Walkmans, phones | Not a goal (use their own apps) |

Your player must appear as a **USB drive** on your computer. On the ZX300,
connect it with USB and it mounts like a drive (on macOS it appears in
Finder).

## 3. Install *(planned)*
- **macOS:** open the `.dmg` and drag WalkTunes to Applications.
- **Windows:** run the `.msi`.
- **Linux:** download the AppImage, make it executable, and run it.

Transcoding (converting formats your player cannot play) uses **ffmpeg**,
which is included in the packages.

## 4. First run
1. **Choose your language** (English / Tiếng Việt) and theme.
2. **Add music folders:** *Library → Add folder*. Add as many as you like (internal disk, external drives, NAS).
3. WalkTunes **scans** the folders. You can browse while it scans.
4. Optional: **Online services** (*Settings → Online services*). Each one is off until you turn it on: MusicBrainz (tags), Cover Art Archive (artwork), AcoustID (identifying untagged songs) and LRCLIB (lyrics).

## 5. Pair your player
1. Connect the player with its USB cable. WalkTunes **recognizes a Sony Walkman by itself** within a few seconds and shows *"Sony NW-ZX300 connected"*. The microSD card, if any, is shown as part of the same player.
2. The first time, give it a name, such as "My ZX300", and confirm. For players WalkTunes cannot identify for sure, it suggests the closest model and you choose.
3. From then on, the player is recognized automatically on every computer with WalkTunes.

WalkTunes only *reads* the identity your computer already sees. It never sends special commands to the player and never touches its firmware files.

Classic Walkmans describe themselves in a small file (`default-capability.xml`) at the root of each storage. WalkTunes reads it to know the exact model, firmware version and which formats the player can play. Please do not delete it, nor `DevIcon.fil` and `DevLogo.fil`.

On the NW-ZX300 the internal storage uses FAT32, which cannot hold files of 4 GB or more. WalkTunes puts such files (long DSD or hi-res tracks) on the microSD card instead, or tells you.

If WalkTunes says the player is connected in **MTP mode**, change the player's USB connection setting to mass storage (USB drive) and reconnect.

WalkTunes creates a folder named `.walktunes` on the player. **Please do not
delete it.** It is how your computers know what is on the player.

## 6. Sync music to your player
1. Open **Devices → your player → Selection** and tick what you want: everything, artists, albums or playlists. Choose internal storage or the SD card for each selection.
2. Press **Plan sync**. You will see:
   - **Add:** new tracks to copy
   - **Replace:** tracks that changed in your library
   - **Remove:** tracks you un-ticked (only files WalkTunes put there)
   - The total size and the free space after syncing
3. Press **Sync**. You can keep using the app. Progress is shown in **Jobs**.
4. When it finishes, press **Eject** before unplugging.

If the cable comes out during a sync, nothing is corrupted. Reconnect the
player and press **Resume**.

**What WalkTunes does to copies (never to your originals):** it resizes artwork so
the player shows it, converts formats the player cannot play, and makes
filenames safe for the player.

## 7. Use your player with several computers
Plug the player into another computer that has WalkTunes installed:
- WalkTunes reads the player's record and shows the songs **on the player but not on this computer**.
- Press **Pull** to copy them into this computer's *Inbox* folder.
- Files you copied to the player by hand appear as *Not managed*. You can **Adopt** them, and WalkTunes will manage them from then on.

## 8. Clean up your library
Open **Health** to see:
- Missing artwork, missing or inconsistent tags (for example, an album split in two because of different "Album Artist" values)
- Duplicates: exact copies, or the same song in different qualities
- Artwork that players often fail to show

### Songs downloaded from video sites
For files named like `Artist - Song (Official Video) [HD].mp3`, WalkTunes
suggests a clean title and artist, tries to identify the song online
(if enabled), and can fetch artwork and lyrics. You review every
suggestion before anything changes. You can also mark these files as
*lossy source* to replace them with better versions later.

### Check audio quality (FLAC check and spectrogram) *(planned, M3–M4)*
Open a track or album and choose **Inspect**, or run **Health → Check quality** for many files at once.
- **Integrity:** WalkTunes decodes the whole file. For FLAC it also checks every frame's checksum and the built-in **MD5 signature** of the audio. Results: *OK*, *MD5 missing*, *Failed* (corrupted or truncated).
- **Spectrogram:** a picture of the sound, like Spek: time left to right, frequency bottom to top, loudness as colour. Change the channel (mid · left · right) with the pivots.
- **Is this FLAC genuine?** If the spectrogram stops at a hard line around 16–20 kHz, the file was probably made from an MP3 or a video-site rip. WalkTunes draws that line and labels the file *Suspect* or *Likely lossy source*. It also spots fake "hi-res" (upsampled) and fake 24-bit (padded) files.
- These are **hints, not judgements**. Old or analog recordings can look similar. Nothing is ever deleted or hidden; press **Mark as reviewed** when you are satisfied.

## 9. Playlists and lyrics *(planned, M5)*
- Create playlists in WalkTunes. They are copied to the player as `.m3u8` files.
- Lyrics (`.lrc`) are copied next to each song so the player can show them.

## 10. Safety promises
- WalkTunes **never deletes your original music files**. "Delete file" moves a file to the system trash, and only after you confirm.
- On the player, WalkTunes only removes files **it** copied, and only when they are unchanged.
- Every change is logged, and tag edits can be undone for 30 days.

## 11. Troubleshooting
Errors show a code such as **WT-SYNC-0002**. Common ones:

| Code | What it means | What to do |
|---|---|---|
| WT-SYNC-0002 | Not enough space on the player | Untick some albums, or choose the SD card |
| WT-SYNC-0003 | The player was disconnected during sync | Reconnect it and press Resume |
| WT-FORGE-0001 | ffmpeg not found | Reinstall WalkTunes, or install ffmpeg |
| WT-DOCK-0004 | The player's record is damaged | Press **Rebuild** (WalkTunes re-reads the player) |
| WT-GATE-0001 | An online service is turned off | Enable it in Settings → Online services |

If you need help, use *Settings → Export diagnostics* and attach the file to
a GitHub issue. It contains no music and no personal file paths.

## 12. Command line *(planned)*
```bash
wtunes root add ~/Music
wtunes scan
wtunes device list
wtunes sync plan  --device "My ZX300"
wtunes sync run   --device "My ZX300"
wtunes pull       --device "My ZX300"
wtunes health --json
wtunes inspect ~/Music/Album --json      # integrity + verdicts
wtunes inspect track.flac --png out.png  # save a spectrogram
```

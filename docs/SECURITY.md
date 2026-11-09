# SECURITY — Data Safety, Privacy, Supply Chain

| Field | Value |
|---|---|
| Status | Draft |
| Owner | Maintainer; every reviewer |
| Last reviewed | 2026-11-09 |
| Related | [ERROR_CONTROL](ERROR_CONTROL.md) · [ARCHITECTURE §3.7](ARCHITECTURE.md#37-wt-vault--s3) · [RELEASE](RELEASE.md) |

---

## 1. Scope

WalkTunes is a local desktop app. It has no server and no accounts. The
assets to protect, in priority order:

1. **The user's music files**: originals in library roots and files on devices.
2. **The user's privacy**: what they listen to, their file paths, their machines.
3. **Integrity of what we ship**: binaries, dependencies, profiles.

## 2. Threat model (summary)

| Threat | Example | Control |
|---|---|---|
| **Our own bug destroys files** | A wrong diff removes a user's album | §3 rules, ops log, trash, safety errors |
| Malicious or corrupted device manifest | `path: "../../Users/me/Documents"` | §4 path confinement |
| Hostile files in the library | A crafted tag or image crashes a parser | Fuzzing of parsers (TESTING), a panic boundary per job, decode size limits |
| Network eavesdropping or tampering | Fake provider responses | TLS only (rustls); responses treated as untrusted proposals |
| Privacy leak | Sending a library listing somewhere | No telemetry; consent per provider; minimal queries |
| Supply-chain compromise | A malicious crate update | `Cargo.lock` committed, `cargo-deny`, `cargo-audit`, review of new dependencies |
| Tampered release | A fake download | Signed and notarized builds and published checksums ([RELEASE](RELEASE.md)) |

---

## 3. Data safety (the #1 risk)

These rules are **non-negotiable**. Code review must reject any change that
breaks them.

| # | Rule | Enforced by |
|---|---|---|
| S-1 | Only `wt-vault` writes, moves or deletes user files. | Layer check (`xtask layers`); `std::fs` mutations are banned in other crates via clippy `disallowed_methods` |
| S-2 | Library originals are **never deleted** by WalkTunes. "Remove from library" removes rows; "Delete file" moves it to the OS trash (`trash` crate) after explicit confirmation. | Vault API has no hard delete for library roots |
| S-3 | On devices, WalkTunes removes **only files listed in its manifest whose hash still matches**. A changed or foreign file is never removed. | `wt-sync` planner + `WT-SYNC-0004` |
| S-4 | Every write is atomic: write a temp file in the same directory, `fsync`, verify the hash, rename. | `Vault::write_atomic` |
| S-5 | Every mutation is recorded in `ops_log` **before** it happens (`pending`) and updated after (`done` / `failed`). | Vault |
| S-6 | Destructive plans need a reviewed dry run. The CLI requires `--yes` or an interactive confirmation; the GUI requires a confirm dialog listing the removal count and size. | `wt-sync`, front ends |
| S-7 | Device removals in a run are capped (default: 30% of manifest entries, or more than 500 files). Exceeding the cap needs a second confirmation. | `wt-sync` |
| S-8 | Tag writes to originals create an in-Ledger undo record (old tag values) and are undoable for 30 days. | Vault + Ledger |
| S-9 | Any safety error (`Abort` class, [ERROR_CONTROL §4](ERROR_CONTROL.md#4-recovery-classes)) stops the job immediately. | Queue |

## 4. File-system confinement

- Every path coming from a manifest, playlist, profile or provider is treated as **untrusted**. It is normalized and rejected if it is absolute, contains `..`, contains NUL or control characters, or escapes its volume or root after joining.
- Symlinks inside roots are followed only if their target is inside a root (configurable; off by default).
- Profile `path_template` output is sanitized for the target file system before use.
- Decoding limits: images larger than 50 MP or 40 MB are rejected, and tag frames larger than 16 MB are skipped.

## 5. Privacy

- **No telemetry, no analytics, no crash upload.** Logs stay local. The user may export a diagnostics bundle and share it manually ([ERROR_CONTROL §6](ERROR_CONTROL.md#6-logging-and-diagnostics)).
- Network providers are **off until enabled**, one by one, in Settings → Online services. The consent screen says exactly what is sent.
- Queries send the minimum: for example, artist + title + duration to LRCLIB; a fingerprint + duration to AcoustID. Paths and filenames are never sent.
- The device manifest stores machine **names**. The user can choose a neutral name at first run.

## 6. Network and third-party services

| Provider | Data sent | Limits / terms (check before release) | Attribution |
|---|---|---|---|
| MusicBrainz | Search terms, MBIDs | 1 request/s, meaningful User-Agent required | Data CC0 (core), mention MusicBrainz |
| Cover Art Archive | MBIDs | Per their terms | Credit the Cover Art Archive |
| AcoustID | Fingerprint, duration | Needs an application client key; per-user key optional; non-commercial terms | Mention AcoustID |
| LRCLIB | Artist, title, album, duration | Public API, be polite | Mention LRCLIB |

- All requests go through `wt-gate`: TLS only, timeouts (connect 5 s, total 20 s), backoff on 429/503, and a cache.
- The AcoustID application key is embedded at build time from CI secrets. It is never committed.

## 7. Licenses and supply chain

**Project license: GPL-3.0-or-later** ([ADR-0009](ARCHITECTURE.md#adr-0009-license-gpl-30-or-later)). Every source file starts with:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later
```

| Component | License | Compatible | Notes |
|---|---|---|---|
| GPUI (Zed) | Apache-2.0 | ✅ | |
| gpui-component | Apache-2.0 | ✅ | verify at pin time |
| Lucide icons | ISC | ✅ | keep its license notice in `THIRD_PARTY_NOTICES` |
| Be Vietnam Pro | SIL OFL 1.1 | ✅ | ship `OFL.txt` with the font; do not sell the font alone |
| ffmpeg (sidecar) | LGPL-2.1+ or GPL-2.0+ (build-dependent) | ✅ | ship a matching source link / offer with bundled binaries |
| SQLite | Public domain | ✅ | |

- `deny.toml` allow-list: MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2/3-Clause, ISC, Zlib, MPL-2.0, Unicode-3.0, CC0-1.0, OFL-1.1, GPL-3.0-or-later, LGPL-2.1+ (sidecar only).
- CI runs `cargo deny check` (licenses, bans, advisories, sources) and `cargo audit` on every PR and every night.
- A new dependency needs a reason in its PR, a license check, and a maintenance check (recent release, more than one maintainer if possible).
- `THIRD_PARTY_NOTICES.md` is generated at release (`cargo about`).

## 8. Reporting a vulnerability

Please use **GitHub → Security → Report a vulnerability** (private advisory)
on `hoangphi01/walktunes`. Do not open public issues for security bugs.
Expect an acknowledgement within 7 days. Fixes are released as patch versions
and credited in the changelog unless you prefer otherwise.

## 9. Trademarks

"Sony" and "Walkman" are trademarks of Sony Group Corporation. This project
is independent and is not affiliated with, endorsed by or sponsored by Sony.
Device names are used only to describe compatibility, and "Walkman" is never
used as part of the product name. See [PRD §8](PRD.md#8-risks).

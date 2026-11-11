# STATUS

| Field | Value |
|---|---|
| Status | Live document |
| Owner | Maintainer |
| Last updated | 2026-11-09 |

> Update this file whenever a roadmap item moves. Keep it short: history
> belongs in [RESULT.md](RESULT.md) and the git log.

## Current milestone: **M0 — Foundation**

| Area | State | Notes |
|---|---|---|
| Docs system | ✅ Done | 21 docs + README, license: MIT OR Apache-2.0 (since 2026-11-12) |
| Brand & design language | ✅ Done | Name WalkTunes, logo (`assets/brand/`), Tile design language (ADR-0011). Needs SVG / 1024 px logo before M4 |
| Cargo workspace | ✅ Done (2026-11-11) | 21 crates; 36 tests; `xtask layers` + `docs-check` pass |
| CI (3 OS) | 🟡 Written | `.github/workflows/ci.yml` (macOS ARM64/x64, Linux x64/ARM64, Windows); first run after push |
| GPUI spike (R-001) | ⬜ Not started | Highest-risk item; do it first |
| Fixtures tooling | ⬜ Not started | |

Legend: ✅ done · 🟡 in progress · ⬜ not started · 🔴 blocked

## Service readiness

| Service | State | Service | State |
|---|---|---|---|
| wt-core | 🟡 skeleton | wt-crawler | ⬜ |
| wt-config | 🟡 skeleton | wt-enrich | ⬜ |
| wt-watch | 🟡 skeleton | wt-forge | ⬜ |
| wt-bus | 🟡 skeleton | wt-dock | 🟡 probe + scoring (fake, sysinfo, macOS diskutil) |
| wt-queue | ⬜ | wt-sync | ⬜ |
| wt-i18n | ⬜ | wt-sdk | 🟡 skeleton |
| wt-ledger | ⬜ | wt-kit | ⬜ |
| wt-vault | ⬜ | wt-console | ⬜ |
| wt-gate | ⬜ | wt-cli | 🟡 skeleton |
| wt-cache | ⬜ | wt-dsp | 🟡 SIMD detection |
| wt-inspect | ⬜ | | |

## Device verification

| Device | Profile | Verified on hardware |
|---|---|---|
| Sony NW-ZX300 | `sony.classic.zx300` | 🟡 Probe verified (R-003 part 1, fw 2.02): recognized with score 220; playlists/artwork pending |

## Research done
- R-006 (2026-11-11): static analysis of the Sony updater shows how Walkmans are identified. This feeds ADR-0014 (passive device probe). Next step: a probe dump from the real ZX300 (R-003).

## Blockers / decisions needed
- None. (PRD Q1 resolved: the product is named **WalkTunes**, 2026-11-09.)

## Next actions
**Prototype sprint P1** (7 days, 2026-11-12 → 2026-11-18): see [ROADMAP](ROADMAP.md#prototype-sprint-p1-first-working-prototype-in-7-days-2026-11-12--2026-11-18).
1. Day 1 (2026-11-12): `wt-ledger` + `wt-vault` safety core, minimal `xtask fixtures`.
2. Day 2: crawler, search, R-002 lite.
3. Day 3: pairing + manifest (first `.walktunes/` write on the real ZX300).

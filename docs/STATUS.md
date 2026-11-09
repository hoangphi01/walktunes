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
| Docs system | ✅ Done | 21 docs + README, GPL-3.0 LICENSE |
| Brand & design language | ✅ Done | Name WalkTunes, logo (`assets/brand/`), Tile design language (ADR-0011). Needs SVG / 1024 px logo before M4 |
| Cargo workspace | ⬜ Not started | |
| CI (3 OS) | ⬜ Not started | |
| GPUI spike (R-001) | ⬜ Not started | Highest-risk item; do it first |
| Fixtures tooling | ⬜ Not started | |

Legend: ✅ done · 🟡 in progress · ⬜ not started · 🔴 blocked

## Service readiness

| Service | State | Service | State |
|---|---|---|---|
| wt-core | ⬜ | wt-crawler | ⬜ |
| wt-config | ⬜ | wt-enrich | ⬜ |
| wt-watch | ⬜ | wt-forge | ⬜ |
| wt-bus | ⬜ | wt-dock | ⬜ |
| wt-queue | ⬜ | wt-sync | ⬜ |
| wt-i18n | ⬜ | wt-sdk | ⬜ |
| wt-ledger | ⬜ | wt-kit | ⬜ |
| wt-vault | ⬜ | wt-console | ⬜ |
| wt-gate | ⬜ | wt-cli | ⬜ |
| wt-cache | ⬜ | | |

## Device verification

| Device | Profile | Verified on hardware |
|---|---|---|
| Sony NW-ZX300 | `sony.classic.zx300` | ⬜ (the maintainer owns one) |

## Blockers / decisions needed
- None. (PRD Q1 resolved: the product is named **WalkTunes**, 2026-11-09.)

## Next 3 actions
1. GPUI spike (M0): font, icons, virtual list, chart on macOS, Windows and Linux.
2. Workspace skeleton and CI.
3. Fixture generator and the first virtual device.

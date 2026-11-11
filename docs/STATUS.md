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
| wt-cache | ⬜ | wt-dsp | ⬜ |
| wt-inspect | ⬜ | | |

## Device verification

| Device | Profile | Verified on hardware |
|---|---|---|
| Sony NW-ZX300 | `sony.classic.zx300` | ⬜ (the maintainer owns one) |

## Research done
- R-006 (2026-11-11): static analysis of the Sony updater shows how Walkmans are identified. This feeds ADR-0014 (passive device probe). Next step: a probe dump from the real ZX300 (R-003).

## Blockers / decisions needed
- None. (PRD Q1 resolved: the product is named **WalkTunes**, 2026-11-09.)

## Next actions
1. GPUI spike (M0): font, icons, tiles, virtual list, chart on macOS, Windows and Linux.
2. DSP spike (M0, R-004): spectrogram speed on NEON vs AVX2, RISC-V scalar build.
3. Workspace skeleton, CI (x86-64 + ARM64) and the fixture generator.

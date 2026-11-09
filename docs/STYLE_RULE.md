# STYLE_RULE — Code Style and Design System (Tile / Metro-inspired)

| Field | Value |
|---|---|
| Status | Draft |
| Owner | Maintainer (`wt-kit` for part B) |
| Last reviewed | 2026-11-09 |
| Related | [TRANS_ANIMATION](TRANS_ANIMATION.md) · [UI_TESTS](UI_TESTS.md) · [ERROR_CONTROL](ERROR_CONTROL.md) |

Part A covers how the code looks. Part B covers how the app looks.

---

# Part A — Code

## 1. General
- `rustfmt` defaults; maximum width 100 (`rustfmt.toml`).
- Every source file starts with `// SPDX-License-Identifier: GPL-3.0-or-later`.
- Use English for identifiers, comments and commit messages.

## 2. Rust code style
- **Naming:** crates `wt-<service>`; types `UpperCamelCase`; everything else `snake_case`; IDs as newtypes (`TrackId(Uuid)`), never bare `String`/`Uuid` across crate boundaries.
- **Errors:** follow [ERROR_CONTROL §2](ERROR_CONTROL.md#2-the-error-model-in-rust). No `unwrap()`/`expect()` in library code.
- **Visibility:** `pub` only what the service contract needs, and `pub(crate)` by default. Each crate's `lib.rs` re-exports its public API. There are no deep public module paths.
- **Side effects:** functions that touch the disk or network must say so in their name or type (`scan_root`, `fetch_*`, or they take `&Vault` / `&Gate`). Pure logic, such as planning, diffing and parsing, lives in modules without I/O so it can be property-tested.
- **Concurrency:** no `tokio` in core crates ([ADR-0004](ARCHITECTURE.md#adr-0004-no-async-runtime-in-core-services)). Use `flume` channels and `wt-queue` jobs. Never block the GPUI main thread.
- **Unsafe code:** `#![forbid(unsafe_code)]` in every crate except where an ADR allows it.
- **Logging:** use `tracing` macros with structured fields: `info!(device = %id, files = n, "sync planned")`. Log paths at `debug` only.
- **Docs:** every public item has a `///` doc comment. Each crate's `lib.rs` starts with a `//!` paragraph that states its one job.
- **Lints (workspace):** `clippy::all`, `clippy::pedantic` (selectively allowed), `unwrap_used`, `expect_used`, `dbg_macro`, `todo` = deny in CI; `missing_docs` = warn.

## 3. Tests in code
- Unit tests sit next to the code (`mod tests`). Integration tests go in `crates/<svc>/tests/`.
- Test names describe behaviour: `plan_never_removes_foreign_files`.

---

# Part B — Design system: "Tile" (Metro-inspired)

## 4. Principles

WalkTunes uses a **square, flat, typographic** design language inspired by
the Metro style of Windows Phone. It is *inspired by* Metro, not a copy of
it: we use our own font (Be Vietnam Pro), our own icons (Lucide) and our own
colours, and never Microsoft fonts, icons or branding. See
[ADR-0011](ARCHITECTURE.md#adr-0011-tile-design-language-metro-inspired).

1. **Square and flat.** Corner radius is **0 everywhere**. There are no drop shadows, no gloss, and no gradients, except the logo itself. Depth comes from solid colour blocks, spacing and type size.
2. **Typography is the interface.** Large, light headings carry the structure. Chrome (lines, boxes, frames) is kept to a minimum.
3. **Content before chrome.** Album art is shown full-bleed in square tiles. Text aligns to a strong left edge.
4. **Alive, not busy.** Tiles show live information (free space, tracks to sync, health issues). Motion is quick and purposeful ([TRANS_ANIMATION](TRANS_ANIMATION.md)).
5. **Show state, not decoration.** Every colour block, badge or animation tells the user something: synced, pending, error, unverified.
6. **Safety is visible.** Destructive actions use the `destructive` colour, appear only after review, and always show counts and sizes.
7. **Bilingual by construction.** Every layout must fit Vietnamese text, which is often 20–30% longer than English and has stacked diacritics.

`gpui-component` supplies the primitives (inputs, virtual lists, charts,
menus). `wt-kit` restyles them: the theme sets radius 0 and disables
shadows, and every screen uses `Wt*` components only.

## 5. Typography: Be Vietnam Pro
- Bundled weights: **300 Light**, 400 Regular, 600 SemiBold, 700 Bold (`assets/fonts/`, with `OFL.txt`). Light is the signature weight for large headings.
- Load the fonts at startup from embedded bytes. Fallback: the system UI font, which raises `WT-UI-0001`.
- Monospace (paths, hashes, logs): the system monospace font.

| Token | Size / line height | Weight | Use |
|---|---|---|---|
| `text-display` | 46 / 60 | 300 | App title on Start, pivot headers (active) |
| `text-header` | 32 / 44 | 300 | Page titles, pivot headers (inactive, muted) |
| `text-title` | 22 / 32 | 300 | List item titles (album, device name) |
| `text-subtitle` | 16 / 24 | 400 | Section labels, dialog titles |
| `text-body` | 14 / 22 | 400 | Body (default), table rows |
| `text-caption` | 12 / 18 | 400 | Secondary text, metadata lines |
| `text-tile` | 13 / 18 | 600 | Tile labels (bottom-left of a tile) |
| `text-tile-count` | 36 / 44 | 300 | Big number on a live tile |

- Line heights are at least 1.3× the size, and at least 1.4× for text ≤ 16 px, so Vietnamese stacked diacritics (ệ, ẫ, ở) are never clipped. Do not tighten them.
- Pivot and page headers are **lowercase** in both languages ("library", "thư viện"), as a deliberate stylistic choice. Buttons and body text use sentence case.

## 6. Colour tokens

Semantic tokens only. Components never use raw hex values. The palette is
solid neutrals plus **one accent** taken from the logo (green `#7AD75A` →
teal `#21A89B`).

- **Dark is the default theme**: a true black background, as on Windows Phone. Light is fully supported, and the app follows the OS setting when the user enables "Match system".
- The light-theme accent is darkened to `#127A71` so that text and focus rings reach 5.19:1 on white. The logo teal `#21A89B` (2.94:1 on white) is used for text only on dark backgrounds.

| Token | Light | Dark | Use |
|---|---|---|---|
| `background` | `#FFFFFF` | `#000000` | App background |
| `foreground` | `#000000` | `#FFFFFF` | Main text |
| `subtle` | `#F2F2F2` | `#1F1F1F` | Neutral tiles, input fill, hover |
| `muted-foreground` | `#666666` | `#A6A6A6` | Secondary text, inactive pivot headers |
| `border` | `#000000` | `#FFFFFF` | Control borders (2 px), used sparingly |
| `divider` | `#E0E0E0` | `#2B2B2B` | Rare list separators |
| `accent` | `#127A71` | `#21A89B` | Accent tiles, primary buttons, focus, links, active pivot underline |
| `on-accent` | `#FFFFFF` | `#000000` | Text and icons on accent fills (5.19:1 / 6.76:1) |
| `brand-gradient` | `#7AD75A → #21A89B` | same | Logo and app icon **only** |
| `success` | `#15803D` | `#22C55E` | Synced, verified |
| `warning` | `#B45309` | `#F59E0B` | Unverified, lossy source, attention |
| `destructive` | `#B91C1C` | `#EF4444` | Remove, errors |
| `chart-1…5` | accent, then 4 contrasting solid hues (set in `wt-kit`) | same | Charts only |

- Contrast: text ≥ 4.5:1, large text and icons ≥ 3:1, in both themes.
- Never use colour as the only signal. Pair it with an icon or a label (for example ✓ Synced, ⚠ Unverified).

### 6.1 Logo
- Master file: `assets/brand/walktunes-logo.png` (324×324). A vector master (SVG) and a 1024×1024 PNG are needed before M4 (app icons) and M6 (packaging).
- Minimum display size 24 px; keep clear space of at least 1/8 of the width around it.
- Do not recolor or stretch the logo, or place it on a busy background. The logo is the only element allowed to use a gradient.

## 7. Grid, tiles, spacing
- **Radius: 0** for everything: tiles, buttons, inputs, dialogs, menus, checkboxes, progress bars, chart bars, artwork. The only exception is the focus ring, which is also square.
- **Elevation: none.** Overlays (menus, dialogs) separate from content with a solid fill and a dimmed backdrop, not shadows.
- **Spacing:** 4 px base grid (`1=4, 2=8, 3=12, 4=16, 6=24, 8=32, 12=48`). The page has a strong left margin of 24 px (32 px at ≥ 1280 px width).

### 7.1 Tiles
Tiles sit on a grid with **unit = 72 px** and **gutter = 8 px**:

| Size | Grid units | Pixels | Use |
|---|---|---|---|
| Small | 1×1 | 72×72 | Shortcut (icon only) |
| Medium | 2×2 | 152×152 | Albums, devices, single live value |
| Wide | 4×2 | 312×152 | Device storage, sync summary, health summary |
| Large | 4×4 | 312×312 | Featured device / now syncing |

- Tile anatomy: a solid fill (`accent`, `subtle`, or full-bleed artwork), an icon top-left or centred (Lucide, 24–32 px), a label bottom-left (`text-tile`, 8 px inset), and an optional live count (`text-tile-count`).
- Album tiles show the artwork full-bleed. The title appears on a solid strip only on hover or focus.
- Live tiles may change their content ("peek") at most every 8 s, only on the Start page, and never under reduced motion.

### 7.2 Lists and tables
- Lists have no separator lines by default. Items are separated by spacing (12 px). Item title `text-title`, metadata `text-caption` in `muted-foreground`.
- Large collections (library tracks) use a dense table: row height 32, text-body, with `divider` lines allowed. The table is still square and flat.
- An album's artwork thumbnail in a list is square (48 or 64 px), with no rounding.

## 8. Icons: Lucide
- Use the Lucide set shipped with `gpui-component` (`IconName::…`). The line style fits the flat look.
- Sizes: 16 px in tables, 20 px in buttons and the app bar, 24–32 px on tiles, 48 px in empty states. Default stroke.
- App-bar buttons are **square** (40×40) with an icon and a text label underneath, never circles.
- Canonical icon per concept (use these everywhere, do not pick synonyms):

| Concept | Lucide icon |
|---|---|
| Library | `library` |
| Device / player | `music-4` (or `disc-3` for albums) |
| Sync | `refresh-cw` |
| Pull from device | `download` |
| Health | `heart-pulse` |
| Jobs | `list-checks` |
| Settings | `settings` |
| Unverified | `badge-alert` |
| Lossless | `audio-waveform` |
| Lossy / rip | `file-audio` |
| Eject | `usb` / `log-out` |
| Trash | `trash-2` |

## 9. Components (in `wt-kit`)

| Component | Metro-style rules |
|---|---|
| `WtTile` | Sizes from §7.1; solid fill; pressed state per TRANS_ANIMATION A-12 |
| `WtPivot` | Horizontal headers in `text-header` Light, lowercase; active = `foreground`, inactive = `muted-foreground`; content swipes/slides between pivots |
| `WtAppBar` | Bottom command bar with square icon+label buttons and an overflow (`…`) menu |
| `WtButton` | Rectangle, 2 px `border`, transparent fill. **Primary**: `accent` fill, `on-accent` text. **Destructive**: `destructive` fill. Pressed: fill inverts |
| `WtInput` | Rectangle, 2 px border, `subtle` fill; focused: `background` fill + `accent` border |
| `WtCheckbox`, `WtRadio` | Square boxes (radio uses a square dot as well), 2 px border |
| `WtToggle` | Rectangular track and thumb |
| `WtProgress` | 4 px flat bar, `accent` fill. Indeterminate: five small squares travelling across ("flying dots") |
| `WtList`, `WtTable` | §7.2; virtualized |
| `WtDialog` | Full-width solid band across the window (`subtle` fill) with a dimmed backdrop; title `text-subtitle`; buttons bottom-right |
| `WtToast` | Solid band at the top of the window (`accent` fill for info, `destructive` for errors), slides down |
| `WtBadge` | Small solid rectangle, `text-caption` |
| `WtEmptyState` | 48 px icon + `text-header` Light line + one action |
| `WtChart` | §10 |

Each component wraps a `gpui-component` primitive where one exists. Screens
use only `Wt*` components.

## 10. Charts
- Allowed forms: **stacked bar** (device storage by format or quality), **bar** (health issues by type), **donut** (library composition, flat segments, ≤ 5 slices). No 3D, no rounded bar ends, no gradients.
- Use the `chart-1…5` tokens in a fixed order per category, so the same format always has the same colour.
- Always label values directly. Tooltips add detail but never hold the only copy of a value.

## 11. Layout
- **Start** (home): the app name in `text-display`, then a tile grid: one wide tile per paired device (free space, tracks pending), plus Library, Health, Jobs and Settings tiles. This is the landing page.
- **Hubs** (Library, Device, Health): a `WtPivot` across the top ("albums · artists · tracks · folders"), the content below, and a `WtAppBar` at the bottom with the page's actions (for example, Sync, Eject).
- A back button (top-left arrow) and `Esc` / `Alt+←` return to Start. There is no persistent sidebar.
- Minimum window 960×600. At widths ≥ 1280, hubs may show two pivot sections side by side.

## 12. Copywriting
- Pivot and page headers are lowercase ("library", "thư viện"). Buttons and body text use sentence case ("Sync now", not "Sync Now"). Numbers come with units ("1,204 tracks · 38.2 GB").
- **English:** plain, friendly, no jargon ("Couldn't reach MusicBrainz", not "HTTP 503").
- **Vietnamese:** natural, polite and concise. Keep established technical terms where Vietnamese users expect them (FLAC, lossless, playlist). Use "bạn" for the user.
- Strings live only in `assets/i18n/{en,vi}/*.ftl`. Keys are `area-element-state`, for example `sync-button-run`.

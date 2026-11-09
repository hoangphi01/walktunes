# STYLE_RULE — Code Style and Design System (Tile / Metro-inspired)

| Field | Value |
|---|---|
| Status | Draft |
| Owner | Maintainer (`wt-kit` for part B) |
| Last reviewed | 2026-11-09 |
| Related | [TRANS_ANIMATION](TRANS_ANIMATION.md) · [UI_TESTS](UI_TESTS.md) · [ERROR_CONTROL](ERROR_CONTROL.md) |

Part A covers how the code looks. Part B covers how the app looks. Part C
covers how the app behaves for people (UX laws as decision rules).

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

1. **Square and flat.** Corner radius is **0 everywhere**. There are no drop shadows, no gloss, and no gradients, except the logo itself and the spectrogram colormap (§10.1), where colour is data. Depth comes from solid colour blocks, spacing and type size.
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
| `WtSpectrogram` | §10.1; GPU-uploaded RGBA image, axes and legend drawn in Tile style |

Each component wraps a `gpui-component` primitive where one exists. Screens
use only `Wt*` components.

## 10. Charts
- Allowed forms: **stacked bar** (device storage by format or quality), **bar** (health issues by type), **donut** (library composition, flat segments, ≤ 5 slices). No 3D, no rounded bar ends, no gradients.
- Use the `chart-1…5` tokens in a fixed order per category, so the same format always has the same colour.
- Always label values directly. Tooltips add detail but never hold the only copy of a value.

### 10.1 Spectrogram (Inspect hub)
The spectrogram is the **one place where a colour gradient is allowed**,
because there it *is* the data.
- **Colormap:** a perceptually uniform, colour-blind-safe sequential map (an "inferno"-like black → purple → orange → pale yellow), defined as a 256-entry table in `wt-kit`. Silence is black in both themes, so the plot reads like Spek. A grayscale map is available for printing and accessibility.
- **Scale:** -120 dBFS (black) to 0 dBFS (brightest), with a vertical legend on the right labelled every 20 dB.
- **Axes:** time along the bottom (`m:ss`), frequency on the left in kHz (linear by default, log optional), Light weight `text-caption`, ticks outside the plot. The plot area has square edges and no frame.
- **Evidence overlays:** a detected lowpass cutoff is a 1 px horizontal line in `warning` with a label ("cutoff ≈ 16.0 kHz"). The Nyquist frequency is a dashed `muted-foreground` line. Overlays never change the underlying colours.
- **Verdict tile:** above the plot, a wide tile shows the verdict in words and colour ("Likely lossy source · 82%" in `warning`, "Integrity OK · MD5 match" in `success`, "Integrity failed" in `destructive`), the format line (`FLAC · 24-bit · 96 kHz · effective 16-bit / 44.1 kHz`), and the "Mark as reviewed" action. Never show colour alone (UX rule, §6).
- **Interaction:** the hover readout shows time, frequency and dB under the pointer. Use pivots for the channel view: "mid · left · right".

## 11. Layout
- **Start** (home): the app name in `text-display`, then a tile grid: one wide tile per paired device (free space, tracks pending), plus Library, Health, Jobs and Settings tiles. This is the landing page.
- **Hubs** (Library, Device, Health, Inspect): a `WtPivot` across the top ("albums · artists · tracks · folders"), the content below, and a `WtAppBar` at the bottom with the page's actions (for example, Sync, Eject).
- A back button (top-left arrow) and `Esc` / `Alt+←` return to Start. There is no persistent sidebar.
- Minimum window 960×600. At widths ≥ 1280, hubs may show two pivot sections side by side.

## 12. Copywriting
- Pivot and page headers are lowercase ("library", "thư viện"). Buttons and body text use sentence case ("Sync now", not "Sync Now"). Numbers come with units ("1,204 tracks · 38.2 GB").
- **English:** plain, friendly, no jargon ("Couldn't reach MusicBrainz", not "HTTP 503").
- **Vietnamese:** natural, polite and concise. Keep established technical terms where Vietnamese users expect them (FLAC, lossless, playlist). Use "bạn" for the user.
- Strings live only in `assets/i18n/{en,vi}/*.ftl`. Keys are `area-element-state`, for example `sync-button-run`.

---

# Part C — UX laws (decision rules)

These research-backed rules of thumb turn design debates into checkable
decisions. Each law below is translated into **concrete WalkTunes rules**
with stable IDs (`UX-01` …). Cite them in design reviews and PRs ("violates
UX-07"). They support user research but never replace it. When a rule
conflicts with a real user need (for example, a long track list that a user
came to see), the user need wins, and the exception is noted in the PR.

## 13. Rules by law

### 13.1 Familiarity and choice

| ID | Law | Rule for WalkTunes |
|---|---|---|
| UX-01 | **Jakob's Law**: users expect your product to work like others they already know | The Tile look is the *visual skin*, not a reinvention of behaviour. Keep the platform conventions: `Cmd/Ctrl+F` search, `Cmd/Ctrl+,` settings, the native menu bar on macOS, native file pickers, Back = `Esc` / `Alt+←` / top-left arrow, double-click opens, right-click shows a context menu, drag and drop for adding folders. Pivot headers are always visible text labels, never icon-only. |
| UX-02 | **Hick's Law**: more choices means slower decisions | The Start page shows at most **7 tiles** by default (devices first; users may pin more). The app bar shows at most **4 primary actions** plus an overflow `…`. Advanced options (codecs, path templates, profile overrides) live behind "Advanced" disclosure. Pairing asks one question per step. |
| UX-03 | **Miller's Law**: working memory holds about 7 (±2) items | Chunk everything: the sync plan is grouped into Add / Replace / Remove, each with a count and a size. Health issues are grouped by type, not shown as a flat list. Long numbers are formatted (`1,204 tracks · 38.2 GB`), and hashes and IDs are shortened (`b3:9f2c…`) with copy-to-clipboard. |
| UX-04 | **Serial Position Effect**: the first and last items are remembered best | Order matters. Start tiles: **devices first**, Settings last. Library pivots: albums · artists · tracks · folders (most used first). Sync plan summary: the **Remove** group appears first when it is not empty, so it is never buried in the middle. |

### 13.2 Targets and grouping

| ID | Law | Rule for WalkTunes |
|---|---|---|
| UX-05 | **Fitts's Law**: larger, closer targets are faster to hit | Minimum interactive target **32×32 px** (40×40 for app-bar buttons and tiles; the small tile is 72). The primary action sits where the eye ends: bottom-right of dialogs and at the first position of the app bar. **Destructive actions** are never adjacent to the primary action, are never the default button, and in dialogs sit on the left, separated by space. |
| UX-06 | **Law of Proximity**: elements close together are seen as related | Spacing does the grouping, since Tile has no cards or shadows. Within a group, use ≤ 8 px; between groups, use ≥ 24 px. Labels sit directly above their inputs (4 px). A button acts on the element closest to it. |
| UX-07 | **Law of Common Region**: a shared boundary groups elements | When grouping is critical (for example, the Remove group in a sync plan, or one device's two volumes), use a **solid fill region** (`subtle` band or a tile), not a border box or a shadow. One region level only: no nested regions. |
| UX-08 | **Law of Prägnanz**: people see the simplest interpretation of a shape | One Lucide icon per concept (§8), with no decorative icons. Tiles are plain rectangles on one grid. At most one emphasis style per text block (weight *or* colour, not both). |

### 13.3 Feedback, speed and endings

| ID | Law | Rule for WalkTunes |
|---|---|---|
| UX-09 | **Doherty Threshold**: productivity rises when responses stay under 400 ms | Visual feedback within **100 ms** for every input (press tilt, hover). Results within **400 ms**, or else show progress: a skeleton after 300 ms (TRANS_ANIMATION A-11), and progress with counts for jobs. Use optimistic UI for safe, reversible actions (rename a playlist, tick an album), **never for destructive ones**. These match budgets B-04 and B-11 in [OPTIMIZATION](OPTIMIZATION.md#1-performance-budgets). |
| UX-10 | **Peak-End Rule**: experiences are judged by their peak and their end | Every flow ends on a designed screen. A finished sync shows **"Done: 1,204 tracks · 38.2 GB · safe to eject"** with the Eject action ready. A failed sync shows what succeeded, what did not, and one clear next action, never a bare error code. The peak moment (the first time a newly paired device lights up its live tile) gets the A-05 flash. |
| UX-11 | **Zeigarnik Effect**: unfinished tasks are remembered better | Interrupted work is visible but not nagging: an interrupted sync appears as a Start tile ("Resume sync: 312 left") and in Jobs, until it is resumed or dismissed. No pop-up reminders and no badges that cannot be cleared. |
| UX-12 | **Goal-Gradient Effect**: motivation increases near the goal | **Long jobs start at 10%**: sync, pull, import, export, artwork normalization, transcoding, tag batches and scans. The first 10% is the **preparation phase**, which is real work: enumerating files, checking free space and building the plan. For sync and pull, the dry-run plan is already finished when the user presses the button, so the bar opens at 10% immediately; for other jobs it fills 0 → 10% as preparation runs. The remaining 90% maps to the main work, weighted by bytes for copies and by items otherwise. The text always shows real counts ("312 of 1,204 · 4.1 of 38.2 GB"). Multi-step flows (pairing, first run) show "Step 2 of 3", counting finished steps. **Never** advance the bar for work that has not happened, and never let it go backwards: if a re-plan is needed, start a new job. See §13.6. |

### 13.4 Complexity and input

| ID | Law | Rule for WalkTunes |
|---|---|---|
| UX-13 | **Tesler's Law**: complexity can be moved, not removed | The system absorbs it: auto-suggest the device profile, auto-normalize artwork, auto-pick a transcode target from the profile, auto-choose the volume with free space. The user only decides *what* music goes *where*. Each new setting needs a PR justification: "why can't the system decide?" |
| UX-14 | **Postel's Law**: be liberal in what you accept, strict in what you produce | **Accept** search with or without diacritics, typos via prefix matching, paths with `~` or pasted with quotes, dates and track numbers in any common form (`3/12`, `03`, `3 of 12`), and dropped folders or files. **Produce** canonical output: consistent tag casing rules, normalized track numbers, locale-formatted numbers, and one date format per locale. |
| UX-15 | **Occam's Razor**: the simplest solution that works is usually right | Between two designs that both solve the need, ship the one with fewer screens, fewer custom patterns and less copy. "Clever" interactions (gestures, hidden shortcuts) are extras, never the only way. |
| UX-16 | **Parkinson's Law**: work expands to fill the time available | Applied **inward only**: time-box design spikes and milestones (see [ROADMAP](ROADMAP.md)). Towards users, artificial urgency is **banned**: no countdowns, no "limited time" prompts, no pressure to act. WalkTunes is a tool, not a funnel. |

### 13.5 Look and emphasis

| ID | Law | Rule for WalkTunes |
|---|---|---|
| UX-17 | **Aesthetic-Usability Effect**: pleasing designs are perceived as easier to use | Visual polish is part of "done": the token-only colours, the type scale, the tile grid and the motion catalog are acceptance criteria for any screen (checked in [UI_TESTS §4](UI_TESTS.md#4-manual-qa-checklists)). Polish never hides a broken flow, so a pretty screen with an unclear next step fails review. |
| UX-18 | **Von Restorff Effect**: the item that stands out is remembered | **One accent-filled primary action per screen** (for example, "Sync" on a device hub). Everything else is outline or ghost. Accent colour on text is reserved for links and the active pivot. If two things compete for accent, one of them is demoted. |

### 13.6 Progress model (UX-12 in detail)

| Phase | Bar range | Real work it represents | Label example |
|---|---|---|---|
| Prepare | 0 → 10% | Enumerate files, read manifest, check free space, build the plan (already done for sync and pull, so they open at 10%) | "Preparing… 2,310 files found" |
| Work | 10 → 95% | The main operation: copy, transcode, resize artwork, write tags, hash. Weighted by bytes for copies, by items otherwise | "312 of 1,204 · 4.1 of 38.2 GB" |
| Finish | 95 → 100% | Verify hashes, write the manifest, update the Ledger | "Verifying…" |

- `wt-queue` jobs report `(phase, done, total)`. `wt-kit::WtProgress` converts that to the bar position, so every screen uses the same mapping.
- A job that fails or is cancelled keeps its bar at the last real value and switches to the `destructive` or `muted` colour. The bar never jumps to 100% on failure.

## 14. Using the laws in review

- Design or UI PRs list the UX rules they touch, for example "UX-05, UX-10".
- A reviewer may block a PR by citing a rule **and** describing the concrete user problem. Citing a law alone is not an argument.
- Proposed exceptions are recorded in the PR. If an exception repeats, update the rule here and record why in [LESSONS](LESSONS.md).

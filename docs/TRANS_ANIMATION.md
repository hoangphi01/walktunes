# TRANS_ANIMATION — Transitions and Motion

| Field | Value |
|---|---|
| Status | Draft |
| Owner | `wt-kit` |
| Last reviewed | 2026-11-09 |
| Related | [STYLE_RULE](STYLE_RULE.md) · [UI_TESTS](UI_TESTS.md) · [OPTIMIZATION §2.4](OPTIMIZATION.md#24-ui-wt-kit-wt-console) |

---

## 1. Principles

Motion follows the Tile design language ([STYLE_RULE §4](STYLE_RULE.md#4-principles)):
like Metro, it is **quick, directional and typographic**. Content slides in
from the edge it belongs to, items arrive in a short stagger, and pressed
tiles respond physically. Everything stays flat: there are no shadows or
blur, and motion uses only opacity, offset and scale.

1. **Motion explains change.** Use it only to show where something came from, where it went, or that something is happening.
2. **Fast and calm.** No single animation lasts longer than 300 ms, and a staggered group finishes within 500 ms (progress excepted). No bounce, no overshoot. This is a tool, not a toy.
3. **Never block input.** Interaction is possible while an animation runs. A new action interrupts the current animation.
4. **Respect reduced motion.** When the OS requests reduced motion, or Settings → Appearance → Reduce motion is on, all movement becomes an instant change or a ≤ 100 ms fade.

## 2. Tokens

| Token | Duration | Easing | Use |
|---|---|---|---|
| `motion-instant` | 0 ms | — | Reduced motion, state toggles |
| `motion-fast` | 120 ms | ease-out | Hover, press, focus ring, checkbox |
| `motion-base` | 180 ms | ease-out (enter) / ease-in (exit) | Popovers, menus, toasts, tab content |
| `motion-slow` | 260 ms | ease-out-expo (enter) / ease-in (exit) | Dialogs, page and pivot transitions |
| `motion-stagger` | 30 ms per item, max 8 items | — | Delay between items entering (tiles, list rows) |
| `motion-progress` | continuous | linear | Progress bars, the sync indicator |

Rules: enter animations use **ease-out**, exit animations use **ease-in**, and
moves between two on-screen states use **ease-in-out**. Exits are about 20%
shorter than enters.

## 3. Catalog

Every animation in the app is listed here. If you need one that is not
listed, add a row in the same PR.

| ID | Element | Enter | Exit | Token |
|---|---|---|---|---|
| A-01 | Page change (Start → hub, hub → Start) | Header slides in 40 px from the right with fade; content follows with stagger (A-13) | Content slides 40 px left and fades | slow |
| A-02 | Dialog (solid band) | Backdrop fades to dim; band slides down 24 px with fade | Reverse | slow |
| A-03 | Menu / app-bar overflow | Slides up 16 px from the app bar with fade | Fade | base |
| A-04 | Toast (top band) | Slides down from the top edge | Slides up | base |
| A-05 | Device attached | Device tile enters with A-13; accent fill flashes **once** | — | slow |
| A-06 | Device detached | Tile switches to `subtle` fill and shows "Disconnected" | — | base |
| A-07 | Sync progress | Flat bar width animates linearly; count text updates at most 10×/s | — | progress |
| A-08 | Row status change (pending → synced) | Row background flashes the `success` tint for 600 ms, then fades | — | slow |
| A-09 | Sync plan diff | Add / replace / remove groups expand with a height and opacity transition | Collapse | base |
| A-10 | Hover | Fill changes to `subtle` (or a 10% lighter accent on accent tiles) | — | fast |
| A-11 | Skeleton loading | Flat `subtle` blocks with a slow opacity pulse, shown only if loading takes > 300 ms | Cross-fade to content | slow |
| A-12 | **Press tilt** (tiles, buttons, list items) | On press, scale to 0.97, offset 1–2 px toward the pointer | On release, spring back to 1.0 (no overshoot) | fast |
| A-13 | **Stagger entrance** (tiles, list rows, pivot content) | Each item fades in and slides 24 px from the right, 30 ms after the previous one, for up to 8 items; the rest appear together | — | slow + stagger |
| A-14 | **Pivot switch** | The headers slide horizontally so the active one moves to the left edge; content slides 40 px in the same direction with fade | Content slides out the opposite way | slow |
| A-15 | **Live tile peek** (Start only) | Tile content slides up to reveal the second face (for example, free space → tracks pending), holds ≥ 8 s, slides back | — | slow |
| A-16 | Indeterminate progress ("flying dots") | Five small squares travel left to right with ease-in-out, staggered | — | progress |

Circular spinners are not used. Indeterminate progress (A-16) is only allowed when progress is truly unknown, and they
must show a label after 2 s ("Scanning 1,204 files…").

## 4. Implementation in GPUI

- GPUI has no 3D transforms. The Metro "turnstile" and "tilt" effects are approximated in 2D with offset, scale and stagger (A-12, A-13). Do not attempt perspective tricks.
- Use GPUI's `AnimationExt::with_animation(id, Animation::new(duration).with_easing(easing), |el, t| …)`.
- Wrap these calls in `wt-kit::motion` helpers (`fade_in`, `slide_in_from`, `stagger`, `press_tilt`, `tile_peek`, `flash_once`) that read the tokens and the reduced-motion setting. Screens never call `with_animation` directly.
- Animation IDs must be stable per element (for example `("row-flash", track_id)`) so that re-renders do not restart them.
- Animate only opacity, offset, size and colour. Do not animate layout-heavy properties on large lists. A list row flash (A-08) affects only visible rows.

## 5. Testing motion
- Unit-test the `motion` helpers with reduced motion on (they must return the final state immediately).
- Run the manual checklist in [UI_TESTS §4](UI_TESTS.md#4-manual-qa-checklists) ("Motion") once per release on each OS.

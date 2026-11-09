# UI_TESTS

| Field | Value |
|---|---|
| Status | Draft |
| Owner | `wt-kit`, `wt-console` |
| Last reviewed | 2026-11-09 |
| Related | [TESTING](TESTING.md) · [STYLE_RULE](STYLE_RULE.md) · [TRANS_ANIMATION](TRANS_ANIMATION.md) |

---

## 1. Approach

Because UI tooling for GPUI is still young, we combine three layers:

| Layer | Tool | Covers |
|---|---|---|
| **View-model tests** | Plain Rust tests | Screen state machines (for example, Sync screen: idle → planning → review → running → done/failed) without rendering |
| **GPUI interaction tests** | `#[gpui::test]`, `TestAppContext`, `VisualTestContext` | Rendering does not panic, actions dispatch, focus moves, key bindings work, views react to bus events |
| **Manual checklists** | §4 | Look and feel, fonts, motion, OS integration, real devices |

Keep business logic out of views. If a test needs a real database, the logic
belongs in the SDK, not the view.

## 2. Structure

```
crates/wt-console/
├── src/screens/<screen>/{mod.rs, view.rs, model.rs}
└── tests/
    ├── screens_<screen>.rs   # gpui tests
    └── support/              # FakeSdk, fixtures, helpers
```

- Screens talk to a `trait SdkPort` (implemented by `wt-sdk`). Tests use `FakeSdk` with scripted responses and events, which makes them fast and deterministic.
- Every screen has at least: a renders-empty test, a renders-with-data test (1 item and 50k items), an error-state test (one per `UserAction` error it can show), and a keyboard-only navigation test.

## 3. Example

```rust
#[gpui::test]
async fn sync_screen_shows_plan_and_requires_confirm(cx: &mut TestAppContext) {
    let sdk = FakeSdk::new().with_plan(plan_with(adds: 3, removes: 2));
    let (view, cx) = cx.add_window_view(|w, cx| SyncScreen::new(sdk.clone(), w, cx));

    view.update(cx, |v, cx| v.request_plan(cx));
    cx.run_until_parked();

    view.read_with(cx, |v, _| {
        assert_eq!(v.state(), SyncState::Review);
        assert!(v.remove_confirmation_required());     // SECURITY S-6
    });
    assert!(sdk.executed_plans().is_empty());          // nothing ran yet
}
```

(GPUI test APIs change between versions. Keep helpers in `tests/support/`
so that an upgrade touches one place.)

## 4. Manual QA checklists

Run per release on **macOS, Windows and Linux (Wayland + X11)**, in **light
and dark**, in **EN and VI**. Record the results in the release PR.

### Typography and language
- [ ] Be Vietnam Pro loads (no fallback font); Vietnamese diacritics are not clipped in tables, buttons or titles
- [ ] Switching EN ↔ VI at runtime updates every visible string; no truncation on main screens at 960×600
- [ ] Long names (100+ characters, mixed scripts) are ellipsized, with the full name in a tooltip

### Layout and theme
- [ ] Tile language holds: **no rounded corners and no shadows anywhere** (re-check after every `gpui-component` upgrade); tiles snap to the 72/8 grid
- [ ] Start page shows live tiles for each paired device; pivot headers are lowercase and Light weight
- [ ] Every screen at 960×600 and at 2560×1440; HiDPI scale factors 1×, 1.5×, 2×
- [ ] Theme follows the OS and the toggle; contrast spot-check with a picker (≥ 4.5:1)
- [ ] Lucide icons are crisp at 16 and 20 px

### UX rules ([STYLE_RULE Part C](STYLE_RULE.md#part-c--ux-laws-decision-rules))
- [ ] Exactly one accent-filled primary action per screen (UX-18); destructive actions never adjacent to it or default (UX-05)
- [ ] Every input gives visual feedback within 100 ms; anything slower than 400 ms shows a skeleton or progress (UX-09)
- [ ] Every flow ends on a designed result screen with a clear next action (UX-10); progress bars open at the 10% prepare mark and every further step matches real work (UX-12, STYLE_RULE §13.6)
- [ ] Start shows ≤ 7 default tiles, app bars ≤ 4 primary actions (UX-02); platform shortcuts work (UX-01)

### Keyboard and accessibility
- [ ] Every action is reachable by keyboard; the focus ring is always visible
- [ ] Esc closes dialogs; Enter confirms only non-destructive defaults
- [ ] Destructive buttons are never the default focus

### Motion
- [ ] The animations A-01 … A-11 ([TRANS_ANIMATION §3](TRANS_ANIMATION.md#3-catalog)) look as specified
- [ ] With reduced motion on, nothing slides or scales

### Devices (virtual + one real device)
- [ ] Attach and detach update the device chip within 3 s
- [ ] The sync plan shows add / replace / remove counts and sizes; the removal confirm lists files
- [ ] Unplugging during a sync shows `WT-SYNC-0003` and offers resume on reattach
- [ ] Safe eject works, or shows `WT-DOCK-0006` with the reason

### Inspect hub
- [ ] Spectrogram preview paints within 400 ms on the reference machine (B-13); the colormap and legend match STYLE_RULE §10.1 in both themes
- [ ] Fixtures from TESTING §7.2 show the expected verdict tiles, cutoff overlay and text labels
- [ ] "Mark as reviewed" persists and never hides or deletes the file

### Performance
- [ ] Scrolling 50k rows is smooth (no visible stutter); typing in search does not lag

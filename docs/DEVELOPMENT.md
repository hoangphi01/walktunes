# DEVELOPMENT

| Field | Value |
|---|---|
| Status | Draft (workspace not created yet; see ROADMAP M0) |
| Owner | Maintainer |
| Last reviewed | 2026-11-09 |
| Related | [ARCHITECTURE §6](ARCHITECTURE.md#6-workspace-layout) · [STYLE_RULE](STYLE_RULE.md) · [TESTING](TESTING.md) |

---

## 1. Prerequisites

| Tool | Version | Notes |
|---|---|---|
| Rust | pinned in `rust-toolchain.toml` (stable) | install with `rustup` |
| ffmpeg | 6+ (optional) | only for transcoding features and `xtask fixtures` |
| cargo-deny, cargo-nextest | latest | `cargo install cargo-deny cargo-nextest` |

### macOS
- Xcode (full app, not only the Command Line Tools). GPUI compiles Metal shaders with `xcrun metal`.
- `xcode-select --install` and `sudo xcodebuild -license accept`.
- `brew install ffmpeg` (optional).

### Windows
- Visual Studio 2022 Build Tools with "Desktop development with C++" and the Windows 10/11 SDK.
- `winget install Gyan.FFmpeg` (optional).
- Enable long paths: `git config --global core.longpaths true`.

### Linux
GPUI needs the Wayland/X11, Vulkan and font libraries. On Debian/Ubuntu:
```bash
sudo apt install build-essential pkg-config libxkbcommon-dev libxkbcommon-x11-dev \
  libwayland-dev libx11-xcb-dev libxcb1-dev libvulkan-dev libfontconfig-dev \
  libfreetype-dev libssl-dev libzstd-dev ffmpeg
```
The exact list changes with GPUI versions. When in doubt, follow Zed's
`script/linux` for the pinned GPUI revision, and record any difference in
[LESSONS.md](LESSONS.md).

---

## 2. Getting started

```bash
git clone https://github.com/hoangphi01/walktunes.git
cd walktunes
cargo xtask fixtures          # generate test audio + virtual devices (needs ffmpeg)
cargo build                   # whole workspace
cargo run -p wt-console       # desktop app
cargo run -p wt-cli -- --help # CLI (binary: wtunes)
```

### Useful environment variables
| Variable | Effect |
|---|---|
| `WT_DATA_DIR` | Override the Ledger, config and logs directory (use it for throwaway profiles) |
| `WT_CACHE_DIR` | Override the cache directory |
| `WT_LOG` | `tracing` filter, e.g. `wt_sync=debug,info` |
| `WT_OFFLINE=1` | Force offline mode (Gate refuses all requests) |
| `WT_FORCE_SIMD=scalar\|sse41\|avx2\|neon` | Force a DSP code path (tests and benchmarks) |
| `WT_FAKE_VOLUMES=path1,path2` | Treat these folders as mounted volumes (virtual devices) |

**Never develop against your real library or your real player** until M2 is
done. Use `WT_DATA_DIR` and virtual devices ([TESTING §4](TESTING.md#4-virtual-devices)).

---

## 3. Everyday commands

| Command | What it does |
|---|---|
| `cargo fmt --all` | Format |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint (CI-strict) |
| `cargo nextest run --workspace` | All tests |
| `cargo xtask layers` | Check crate dependency rules |
| `cargo xtask docs-check` | Check doc headers, links, error-code registry |
| `cargo xtask ci` | Everything CI runs, locally |
| `cargo xtask bench` | Macro benchmarks ([OPTIMIZATION §3](OPTIMIZATION.md#3-how-to-measure)) |
| `cargo xtask bench-dsp` | DSP and inspection benchmarks (B-12…B-17); prints the active SIMD path |
| `cargo xtask cross riscv64` | Cross-build the CLI for RISC-V and run its tests under QEMU |
| `cargo deny check` | Licenses, advisories, bans |

---

## 4. Workspace conventions

- All dependency versions live in the root `[workspace.dependencies]`. Crates use `foo = { workspace = true }`.
- Shared lints live in `[workspace.lints]`. Crates opt in with `[lints] workspace = true`.
- GPUI and gpui-component are pinned to exact versions or git revisions. Upgrading them is its own PR with a RESULT entry (a smoke test on 3 OSes).
- Each crate has a short `README.md`: its job, its owner, and a link to its ARCHITECTURE section.

---

## 5. Git workflow

- **Trunk-based:** `main` is always releasable and protected (CI must be green, one review once there are other contributors).
- Branches: `feat/<area>-<topic>`, `fix/<area>-<topic>`, `docs/<topic>`, `chore/<topic>`.
- Commits follow **Conventional Commits**: `feat(sync): resume after disconnect`, `fix(dock): …`, `docs: …`. The scope is the service name without `wt-`.
- The PR description includes the checklist from [DOCS_GUIDE §7](DOCS_GUIDE.md#7-pr-checklist-copy-into-every-pr-description).
- Squash-merge. The squash message follows Conventional Commits, which drives the changelog ([RELEASE](RELEASE.md)).

---

## 6. CI pipeline (GitHub Actions)

| Job | OS | Steps |
|---|---|---|
| `check` | ubuntu | fmt, clippy, `xtask layers`, `xtask docs-check`, `cargo deny` |
| `test` | macOS-14 (ARM64), macOS-13 (x86-64), windows-2022 (x64), ubuntu-24.04 (x64), ubuntu-24.04-arm (ARM64) | build, fixtures, nextest, numerical tests ([TESTING §7](TESTING.md#7-numerical-and-cross-architecture-tests)) |
| `test-arm-windows` | windows-11-arm `[VERIFY runner availability]` | build + nextest (GUI smoke when GPUI supports it) |
| `cross-riscv` | ubuntu (cross + QEMU) | build `riscv64gc-unknown-linux-gnu` CLI, run core and DSP unit tests under QEMU |
| `ui-smoke` | all 3 | build the console, run GPUI tests ([UI_TESTS](UI_TESTS.md)) |
| `audit` (nightly) | ubuntu | `cargo audit`, `cargo deny check advisories` |
| `release` (tags) | all 3 | see [RELEASE](RELEASE.md) |

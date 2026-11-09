# RELEASE

| Field | Value |
|---|---|
| Status | Draft (first release planned at M6) |
| Owner | Maintainer |
| Last reviewed | 2026-11-09 |
| Related | [SECURITY §7](SECURITY.md#7-licenses-and-supply-chain) · [DEVELOPMENT §6](DEVELOPMENT.md#6-ci-pipeline-github-actions) |

---

## 1. Versioning
- **SemVer** for the app: `MAJOR.MINOR.PATCH`. Before 1.0, minor versions may break things.
- One version for the whole workspace (`[workspace.package] version`).
- Separate format versions that are **not** tied to the app version: Ledger schema `N` ([DATABASE §5](DATABASE.md#5-migrations)), manifest `version`, profile schema version. A change to any of them is called out in the changelog.
- Channels: `nightly` (from `main`, unsigned, for testers), `beta` (`x.y.z-beta.n`) and `stable`.

## 2. Changelog
- `CHANGELOG.md` uses the *Keep a Changelog* format and is generated from Conventional Commits (`git-cliff`), then edited by hand for clarity.
- Sections: Added, Changed, Fixed, Security, **Data format changes**, **Device support** (profiles added or verified).

## 3. Artifacts per OS

| OS | Package | Signing | Notes |
|---|---|---|---|
| macOS | `.dmg` with a universal `.app` (arm64 + x86_64) | Developer ID + **notarization** (`notarytool`), hardened runtime | Bundle ffmpeg (LGPL build) inside `Contents/MacOS/` |
| Windows | `.msi` (WiX) + portable `.zip`, for **x64 and ARM64** (ARM64 once GPUI support is verified) | Authenticode (code-signing certificate or Azure Trusted Signing) | Bundle `ffmpeg.exe` |
| Linux | AppImage + `.deb` for **x86-64 and ARM64**; Flatpak later. **riscv64**: CLI-only `.tar.gz`, best-effort | GPG-signed checksums | AppImage bundles ffmpeg; `.deb` depends on the system ffmpeg |

All releases ship `SHA256SUMS` (signed), `LICENSE`, `THIRD_PARTY_NOTICES.md`,
`OFL.txt` (font), and a link to the ffmpeg source matching the bundled build.

The CLI `wtunes` is included in every package. It is also published as a
standalone archive per OS.

## 4. Release checklist

```markdown
### Before
- [ ] ROADMAP milestone exit criteria met; RESULT entry written
- [ ] STATUS.md current; no open P0 bugs; no open data-loss bugs
- [ ] `cargo deny check` and `cargo audit` clean
- [ ] Migration tests from every previous schema version pass
- [ ] UI manual checklist (UI_TESTS §4) done on 3 OSes, EN + VI, light + dark
- [ ] At least one real-device sync on the ZX300 with this build
- [ ] DSP benchmarks (B-12…B-17) recorded for x86-64 and ARM64; no regression > 10%
- [ ] USER_GUIDE.md and USER_GUIDE.vi.md updated
- [ ] CHANGELOG edited; version bumped; `git tag vX.Y.Z` (signed)

### Build & publish (CI on tag)
- [ ] Builds on 3 OSes; signed/notarized; checksums signed
- [ ] GitHub Release with notes and artifacts
- [ ] Smoke test: install on a clean VM per OS, launch, scan the fixture library

### After
- [ ] STATUS.md updated; RESULT entry linked
- [ ] Announce (README badge, discussions)
```

## 5. Hotfixes
- Branch `release/x.y` from the tag, fix, tag `x.y.(z+1)`, then cherry-pick to `main`.
- A **data-safety** bug triggers an immediate hotfix and a security advisory if user data could be lost ([SECURITY §8](SECURITY.md#8-reporting-a-vulnerability)).

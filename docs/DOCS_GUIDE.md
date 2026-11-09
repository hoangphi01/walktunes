# DOCS_GUIDE — The Map of WalkTunes

> **Start here.** This file is the index and dictionary for the whole project.
> Before you change code, look up the area you are touching in
> [§4 Change-Impact Matrix](#4-change-impact-matrix). It tells you which docs
> must change with it.

| Field | Value |
|---|---|
| Status | Stable (v1) |
| Owner | Maintainer (@hoangphi01) |
| Last reviewed | 2026-11-09 |
| Applies to | Every crate, every doc, every PR |

---

## 1. Why this file exists

WalkTunes is built like a small cloud platform. Each **service** (crate) has
one job, a clear contract, and an owner. That design only works if the docs
are kept in step with the code. This guide makes that possible:

1. **Dictionary.** Every service, concept and doc is defined once, here or in
   [GLOSSARY.md](GLOSSARY.md), and linked from everywhere else.
2. **Router.** For any code change, §4 says exactly which docs to update.
3. **Contract.** §6 defines how docs are written, so all 22 files read alike.

If the code and a doc disagree, **the doc is wrong until proven otherwise.**
Fix the doc, or open an issue labelled `docs-drift`.

---

## 2. Document catalog

Docs live in `docs/`. The only exceptions are `README.md` and `LICENSE`, which
are at the repo root.

### 2.1 Product: *what* and *why*

| Doc | Purpose | Read when |
|---|---|---|
| [PRD.md](PRD.md) | Product requirements: users, problems, scope, requirements with IDs (`FR-*`, `NFR-*`) | Deciding whether a feature belongs in the product |
| [ROADMAP.md](ROADMAP.md) | Milestones M0–M6 and later, with exit criteria | Planning work |
| [STATUS.md](STATUS.md) | Live dashboard: what is done, in progress, blocked | Starting a session or standup |
| [RESULT.md](RESULT.md) | Measured outcomes: milestone results, benchmarks, spikes | Closing a milestone or experiment |
| [LESSONS.md](LESSONS.md) | Lessons learned: mistakes and surprises worth remembering | After any incident, surprise or painful bug |

### 2.2 Engineering: *how it is built*

| Doc | Purpose | Read when |
|---|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | Service catalog, layers, data flows, decision log (ADRs) | Before touching more than one crate |
| [DATABASE.md](DATABASE.md) | Ledger schema (SQLite), device manifest format, migrations | Changing any table, column or manifest field |
| [ERROR_CONTROL.md](ERROR_CONTROL.md) | Error model, error code registry `WT-<SVC>-<NNNN>`, recovery rules | Adding or changing an error |
| [EXTENSION.md](EXTENSION.md) | How to add device profiles, formats, enrichment providers and services | Supporting a new player or source |
| [OPTIMIZATION.md](OPTIMIZATION.md) | Performance budgets and techniques | Code on the hot path: scan, sync, list rendering |
| [SECURITY.md](SECURITY.md) | Threat model, data safety, privacy, supply chain, licenses | Code that deletes or writes files, uses the network, or adds a dependency |

### 2.3 Craft: *how we work*

| Doc | Purpose | Read when |
|---|---|---|
| [DEVELOPMENT.md](DEVELOPMENT.md) | Setup on macOS/Windows/Linux, workspace, commands, git flow | First day; any tooling change |
| [STYLE_RULE.md](STYLE_RULE.md) | Rust code style, the "Tile" design system (Metro-inspired: square, flat; Be Vietnam Pro, Lucide) and UX laws as decision rules (`UX-01`…`UX-18`) | Writing any code or UI |
| [TRANS_ANIMATION.md](TRANS_ANIMATION.md) | Motion system: durations, easing, which transitions exist | Adding any animation or transition |
| [TESTING.md](TESTING.md) | Test strategy, fixtures, virtual devices, CI gates | Writing or changing tests |
| [UI_TESTS.md](UI_TESTS.md) | GPUI test harness, interaction tests, manual QA checklists | Changing any screen |
| [RELEASE.md](RELEASE.md) | Versioning, changelog, packaging and signing per OS | Cutting a release |

### 2.4 People: *how it is used*

| Doc | Purpose | Read when |
|---|---|---|
| [../README.md](../README.md) | Project front page | First contact |
| [USER_GUIDE.md](USER_GUIDE.md) | End-user guide (English) | Any user-visible change |
| [USER_GUIDE.vi.md](USER_GUIDE.vi.md) | End-user guide (Vietnamese) | Any user-visible change (keep both in sync) |
| [GLOSSARY.md](GLOSSARY.md) | Every term used in the project | Meeting an unfamiliar word |

---

## 3. Service dictionary (the "mini AWS")

Each service is one Rust crate under `crates/`. The AWS service in brackets is
an **analogy** to help remember the role; we do not use AWS. The full contracts
are in [ARCHITECTURE.md §3](ARCHITECTURE.md#3-service-catalog).

| Service | Crate | AWS analogy | One job | Primary docs |
|---|---|---|---|---|
| **Core** | `wt-core` | Shared kernel | IDs, shared types, base `Error` trait, no I/O | ARCHITECTURE, ERROR_CONTROL |
| **Config** | `wt-config` | SSM Parameter Store | Settings, per-OS paths, machine identity | ARCHITECTURE, DEVELOPMENT |
| **Watch** | `wt-watch` | CloudWatch | Logging, tracing, local metrics, diagnostics bundle | OPTIMIZATION, ERROR_CONTROL |
| **Bus** | `wt-bus` | EventBridge | Typed in-process events between services | ARCHITECTURE |
| **Queue** | `wt-queue` | SQS + Step Functions | Background jobs: progress, cancel, retry | ARCHITECTURE, OPTIMIZATION |
| **DSP** | `wt-dsp` | EC2 compute-optimized (Graviton) | Pure SIMD numeric kernels: FFT, STFT, windows, statistics | OPTIMIZATION, TESTING |
| **i18n** | `wt-i18n` | — | Translated strings (en, vi) | STYLE_RULE, USER_GUIDE* |
| **Ledger** | `wt-ledger` | RDS / DynamoDB | SQLite catalog, migrations, repositories | DATABASE |
| **Vault** | `wt-vault` | S3 | Library roots, safe file I/O, content hashing, trash | SECURITY, DATABASE |
| **Gate** | `wt-gate` | API Gateway | The only outbound HTTP: consent, rate limits, offline mode | SECURITY |
| **Cache** | `wt-cache` | CloudFront | Artwork thumbnails and derived-data cache | OPTIMIZATION |
| **Crawler** | `wt-crawler` | Glue Crawler | Scan roots, read tags, keep Ledger fresh | DATABASE, OPTIMIZATION |
| **Enrich** | `wt-enrich` | Glue Jobs / Lambda | MusicBrainz, AcoustID, Cover Art, LRCLIB, filename parsing | EXTENSION, SECURITY |
| **Forge** | `wt-forge` | Elemental MediaConvert | Transcoding (ffmpeg), artwork normalization, renditions | EXTENSION, OPTIMIZATION |
| **Dock** | `wt-dock` | IoT Core | Device detection, pairing, device profiles, manifest I/O | EXTENSION, DATABASE |
| **Inspect** | `wt-inspect` | Amazon Inspector | FLAC integrity, Spek-like spectrogram, fake-lossless and other detectors | EXTENSION, DATABASE, OPTIMIZATION |
| **Sync** | `wt-sync` | DataSync | Plan (diff) and execute transfers; pull back from device | ARCHITECTURE, SECURITY |
| **SDK** | `wt-sdk` | AWS SDK | The single facade that every front end calls | ARCHITECTURE |
| **Kit** | `wt-kit` | Amplify UI | Tile design system on GPUI: theme, fonts, icons, components, motion | STYLE_RULE, TRANS_ANIMATION |
| **Console** | `wt-console` | Management Console | The desktop app (GPUI) | UI_TESTS, USER_GUIDE* |
| **CLI** | `wt-cli` (binary `wtunes`) | AWS CLI | Command-line front end | USER_GUIDE*, TESTING |

**Rule of one job:** if a change makes a crate do two unrelated jobs, it
belongs in a different crate. If no crate fits, propose a new service with an
ADR (see [ARCHITECTURE.md §9](ARCHITECTURE.md#9-decision-log-adrs)).

---

## 4. Change-Impact Matrix

Find the row for what you changed and update **every** doc marked ●.
○ means "check it; update only if affected". The PR template checklist
(§7) refers to this table.

### 4.1 By code area

| You changed… | PRD | ARCH | DB | ERR | EXT | OPT | SEC | STYLE | ANIM | TEST | UI_T | USER* | GLOSS | STATUS |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| A new crate/service, or a service's responsibility | ○ | ● | ○ | ● | ○ | ○ | ○ | | | ● | | | ● | ● |
| Dependencies between crates (layering) | | ● | | | | | | | | | | | | |
| A table, column, index or migration (`wt-ledger`) | | ○ | ● | ○ | | ○ | | | | ● | | | ○ | |
| The device manifest format (`wt-dock`) | | ● | ● | ○ | ● | | ● | | | ● | | ○ | | |
| A new error or error code | | | | ● | | | | | | ○ | | ○ | | |
| A device profile or detection rule | ○ | | ○ | | ● | | ○ | | | ● | | ● | ○ | ● |
| An audio format or transcoding rule (`wt-forge`) | ○ | | | ○ | ● | ● | | | | ● | | ● | ○ | |
| An enrichment provider or any network call (`wt-enrich`, `wt-gate`) | ○ | ○ | | ● | ● | | ● | | | ● | | ● | | |
| File writes, deletes or moves (`wt-vault`, `wt-sync`) | | ○ | | ● | | | ● | | | ● | | ○ | | |
| The sync planning or execution algorithm | | ● | ○ | ● | | ● | ● | | | ● | | ● | | |
| DSP kernels, SIMD code or CPU-architecture handling (`wt-dsp`) | | ● | | ○ | | ● | | | | ● | | | ○ | |
| A quality detector, threshold or integrity check (`wt-inspect`) | ○ | | ● | ○ | ● | ○ | | | | ● | | ● | ○ | |
| A hot path (scan, hash, list, sync) | | | ○ | | | ● | | | | ● | | | | |
| A UI screen or component (`wt-console`, `wt-kit`) | ○ | | | | | ○ | | ● | ○ | | ● | ● | | |
| An animation or transition | | | | | | ○ | | | ● | | ● | | | |
| A user-facing string | | | | ○ | | | | ● | | | ○ | ● | | |
| Settings / config keys (`wt-config`) | | ○ | | | | | ○ | | | ○ | | ● | | |
| CLI commands or flags (`wt-cli`) | | | | ○ | | | | | | ● | | ● | | |
| Build, toolchain or CI | | | | | | | ○ | | | ● | | | | |
| Packaging, signing, versioning | | | | | | | ● | | | | | | | |
| A new dependency (crate, font, icon set, tool) | | ○ | | | | | ● | | | | | | | |
| A new term or concept | | | | | | | | | | | | | ● | |

Column key: ARCH = ARCHITECTURE, DB = DATABASE, ERR = ERROR_CONTROL,
EXT = EXTENSION, OPT = OPTIMIZATION, SEC = SECURITY, STYLE = STYLE_RULE,
ANIM = TRANS_ANIMATION, TEST = TESTING, UI_T = UI_TESTS,
USER* = USER_GUIDE.md **and** USER_GUIDE.vi.md, GLOSS = GLOSSARY.

Two columns are left out of the table because the rule for them never changes:

- **DEVELOPMENT.md:** update it whenever the toolchain, commands or setup
  change.
- **RELEASE.md:** update it whenever the release or packaging process changes.

### 4.2 By event

| Event | Update |
|---|---|
| Milestone finished | ROADMAP (tick exit criteria) → RESULT (measured outcome) → STATUS |
| Benchmark or spike run | RESULT; OPTIMIZATION if a budget changes |
| Incident, data-loss scare, or painful bug | LESSONS; ERROR_CONTROL or SECURITY if a rule changes |
| A `[VERIFY]` claim confirmed on real hardware | The doc that holds it (remove the marker) + EXTENSION device table + RESULT |
| Architecture decision | ARCHITECTURE §9 (new ADR) |
| Release cut | RELEASE checklist, CHANGELOG, STATUS |
| Scope change | PRD (bump its version) → ROADMAP |

---

## 5. Reading paths

| You are… | Read in this order |
|---|---|
| New contributor | README → PRD → ARCHITECTURE → DEVELOPMENT → STYLE_RULE → TESTING |
| Adding a new player | GLOSSARY (Device, Profile) → EXTENSION → DATABASE (manifest) → TESTING (virtual devices) |
| Working on UI | STYLE_RULE → TRANS_ANIMATION → UI_TESTS → USER_GUIDE |
| Reviewing a PR | This file §4 → SECURITY → ERROR_CONTROL |
| An AI coding agent | This file → STATUS → the docs for the crate you are editing |

---

## 6. Writing rules for all docs

1. **Header table.** Every doc starts with a title and a table holding
   `Status` (Draft / Stable / Deprecated), `Owner` and `Last reviewed`
   (ISO date).
2. **English first.** All docs are in English, except `USER_GUIDE.vi.md`,
   which mirrors `USER_GUIDE.md` in Vietnamese. Code identifiers always stay
   in English.
3. **One fact, one home.** Define each fact once and link to it everywhere
   else. Do not copy tables between docs.
4. **Stable IDs.** Requirements (`FR-012`), UX rules (`UX-05`), errors (`WT-SYNC-0003`), ADRs
   (`ADR-0007`), lessons (`L-004`) and results (`R-003`) are never renumbered
   or reused. Deprecate them instead of deleting.
5. **`[VERIFY]` marker.** Any claim about real hardware that has not yet been
   tested on a physical device is tagged `[VERIFY]`. Remove the tag only after
   testing, and record the test in RESULT.md.
6. **Dates are absolute.** Write `2026-11-09`, never "last week".
7. **Diagrams** use Mermaid, so they render on GitHub and stay diffable.
8. **Tone.** Short sentences, active voice, present tense. Use "must",
   "should" and "may" as in RFC 2119.
9. **Links** are relative (`[DATABASE.md](DATABASE.md#3-tables-schema-v1)`).

---

## 7. PR checklist (copy into every PR description)

```markdown
- [ ] Checked DOCS_GUIDE §4 and updated every ● doc
- [ ] New errors registered in ERROR_CONTROL.md
- [ ] Schema/manifest change has a migration and a DATABASE.md entry
- [ ] File-deleting/overwriting code reviewed against SECURITY.md §3
- [ ] User-visible change reflected in USER_GUIDE.md AND USER_GUIDE.vi.md
- [ ] Tests added (TESTING.md), UI checklist run if a screen changed (UI_TESTS.md)
- [ ] STATUS.md updated if a roadmap item moved
```

---

## 8. Doc health

- `scripts/docs-check` (planned for milestone M0) will check that every doc
  has a header table, that relative links resolve, and that each error code in
  the code appears in ERROR_CONTROL.md.
- Review all docs whenever a milestone closes, and update `Last reviewed`.

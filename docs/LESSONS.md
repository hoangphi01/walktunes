# LESSONS — Lessons Learned

| Field | Value |
|---|---|
| Status | Live log |
| Owner | Everyone |
| Last reviewed | 2026-11-09 |

Write an entry whenever something surprised you, cost more than an hour, or
nearly lost data. The goal is to **not pay for the same lesson twice**. If a
lesson changes a rule, update the doc that owns the rule as well (see
[DOCS_GUIDE §4.2](DOCS_GUIDE.md#42-by-event)).

Entries are append-only, numbered `L-001`, `L-002`, …

## Template

```markdown
### L-00X — <short title>
- **Date:** YYYY-MM-DD   **Area:** <crate / doc / device>
- **What happened:** facts only.
- **Why:** root cause (5 whys if needed).
- **Lesson:** the rule of thumb in one sentence.
- **Action:** what changed (doc / test / code) with links.
```

## Seed lessons (from planning, 2026-11-09)

### L-001 — "Copy and done" is the real problem, not copying
- **Area:** Product
- **Lesson:** the value is in *memory* (the manifest) and *diff* (the plan), not in copying speed. Every feature should strengthen one of these two.
- **Action:** manifest-first design ([ADR-0006](ARCHITECTURE.md#adr-0006-device-manifest-is-a-json-snapshot-not-a-database)).

### L-002 — Never trust hardware folklore without a test
- **Area:** Devices
- **Lesson:** claims such as "Walkman cannot show progressive JPEG" are widely repeated but must be verified on the specific model and firmware.
- **Action:** the `[VERIFY]` marker convention ([DOCS_GUIDE §6](DOCS_GUIDE.md#6-writing-rules-for-all-docs)).

## Entries

_No entries yet._

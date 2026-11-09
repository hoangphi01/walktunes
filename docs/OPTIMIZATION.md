# OPTIMIZATION — Performance Budgets and Techniques

| Field | Value |
|---|---|
| Status | Draft (budgets are targets until measured in RESULT) |
| Owner | `wt-crawler`, `wt-sync`, `wt-kit`, `wt-console` |
| Last reviewed | 2026-11-09 |
| Related | [RESULT](RESULT.md) · [ARCHITECTURE §4](ARCHITECTURE.md#4-runtime-model) |

---

## 1. Performance budgets

Reference machine: a 2020-class laptop (4 cores, NVMe SSD, 8 GB RAM).
Reference library: **10k tracks** (60% FLAC, 35% MP3, 5% other), about 250 GB.
Stretch library: 50k tracks.

| ID | Operation | Budget | Measured |
|---|---|---|---|
| B-01 | App cold start to an interactive library view | < 1.0 s | — |
| B-02 | Cold scan of 10k files (tags + properties + content hash) on SSD | < 60 s | — |
| B-03 | Warm rescan of 10k files with no changes | < 3 s | — |
| B-04 | Search keystroke to results (50k tracks) | < 30 ms | — |
| B-05 | Scrolling the 50k-row library list | 60 fps, no frame > 16 ms | — |
| B-06 | Sync plan for 5k tracks vs. a 5k-entry manifest | < 500 ms | — |
| B-07 | Sync throughput (no transcode) | ≥ 90% of a raw `cp` to the same device | — |
| B-08 | Manifest write (10k entries) | < 300 ms | — |
| B-09 | Idle CPU with the app open | < 1% | — |
| B-10 | Memory with 50k tracks loaded | < 400 MB | — |
| B-11 | Any call from the UI thread into the SDK | < 5 ms (p99) | — |

Exceeding a budget is a bug. Make a budget looser only through a RESULT
entry that explains why.

---

## 2. Techniques by area

### 2.1 Scanning (`wt-crawler`, `wt-vault`)
- **Change detection ladder:** `(size, mtime)` unchanged → skip; changed → re-read tags; re-hash only when the size changed or identity is needed.
- Hash with `blake3` using memory-mapped reads for large files. Hash files in parallel, about one worker per core, but limit to 2 concurrent hashes on spinning or USB disks (detected from the volume type, or set by the user).
- Read only the tag region (lofty `ParseOptions::read_properties(true)` without the picture payload). Hash artwork lazily.
- Batch Ledger writes in transactions of 500 rows.

### 2.2 Database (`wt-ledger`)
- Use WAL and prepared statements cached per connection. Never use `SELECT *` in hot paths.
- Read the library list as compact rows (`id, title, artist, album, duration, flags`) and fetch details on demand.
- Use FTS5 for search, with a prefix index (`prefix='2 3'`).

### 2.3 Sync (`wt-sync`, `wt-forge`)
- Planning is pure and in memory: hash maps keyed by `source_hash` and by path.
- Copy with large buffers (1–4 MiB). Use **one writer per device volume**, because parallel writes to USB flash are slower.
- Overlap transcoding (CPU) with copying (I/O): use a bounded pipeline with N transcoders and 1 writer.
- Checkpoint the manifest every N files or T seconds (default: 25 files or 10 s), not after every file, to protect flash wear and keep throughput up.
- Reuse renditions: cache transcoded and art-normalized outputs keyed by `(source_hash, profile_id, rendition_spec_hash)` in `wt-cache`.

### 2.4 UI (`wt-kit`, `wt-console`)
- Always use GPUI virtualized lists (`uniform_list`) for collections. Never render more rows than are visible plus a small overscan.
- Load thumbnails asynchronously from `wt-cache` at fixed sizes (64 and 256 px), with an in-memory LRU of decoded images.
- Coalesce bus events: progress updates reach the UI at most every 100 ms.
- Keep views small. Call `cx.notify()` only on the entity that changed.

### 2.5 Startup
- Open the Ledger and render the last view first. Start volume discovery and watchers after the first frame.
- Load fonts from embedded bytes, only the weights we use (400, 500, 600, 700).

---

## 3. How to measure

- Micro-benchmarks: `criterion` in `crates/<svc>/benches/` (hashing, plan building, FTS queries).
- Macro-benchmarks: `cargo xtask bench --library fixtures/large` runs the scan, plan and sync against a virtual device and prints JSON. Record the results in RESULT.
- Profiling: `cargo flamegraph` / Instruments (macOS) / `perf` (Linux) / Superluminal or WPA (Windows). For GPUI frames, enable the frame timing overlay in debug builds.
- Tracing spans include `elapsed_ms`; `wtunes --trace` writes a Chrome trace file.

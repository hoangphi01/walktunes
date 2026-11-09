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
| B-12 | FLAC full integrity check (decode + CRC + MD5), single core | ≥ 150× real time (a 5-min track in < 2 s) | — |
| B-13 | Spectrogram **preview** first paint (5-min track, 1600 px wide) | < 400 ms (Doherty threshold, UX-09) | — |
| B-14 | Spectrogram full-resolution render, same track | < 1.5 s | — |
| B-15 | Batch inspection of 10k tracks | Scales near-linearly with cores, UI stays at 60 fps | — |
| B-16 | Memory per open spectrogram | < 64 MB | — |
| B-17 | Cross-architecture agreement | Identical verdicts; spectrogram values within 0.1 dB | — |

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

### 2.6 Audio analysis (`wt-dsp`, `wt-inspect`)
- **Algorithm first.** The preview uses *sparse* decoding: for W pixel columns it seeks to W positions and decodes one FFT window each (FLAC seek tables make this cheap), instead of decoding the whole track. The full check is **single-pass**: one decode feeds integrity, the STFT and the Welch average together.
- **Real FFT.** Use `realfft` (real input) rather than a complex FFT, which roughly halves the work. Plan once per (size, direction) and reuse the plan and scratch buffers per thread.
- **Streaming, not buffering.** Never hold a whole track as PCM. Keep a ring buffer of FFT size × channels.
- **`f32` everywhere.** It gives about 144 dB of dynamic range, more than the -120 dBFS floor we display. Use `f64` only for accumulators (Welch averaging, MD5 is integer).
- **Vectorize the glue, not only the FFT.** Window multiply, magnitude, `log10` (via a fast approximation with < 0.01 dB error), and colormap lookup are SIMD kernels in `wt-dsp`.
- **Parallelism:** columns of one spectrogram, or different files in a batch, in a bounded `rayon` pool (default: physical cores − 1). Decoding one FLAC stream is sequential; parallelize across files instead.
- **Cache results.** Spectrogram images go into `wt-cache`; verdicts go into the Ledger keyed by `content_hash` + `analyzer_version`.

## 3. How to measure

- Micro-benchmarks: `criterion` in `crates/<svc>/benches/` (hashing, plan building, FTS queries).
- Macro-benchmarks: `cargo xtask bench --library fixtures/large` runs the scan, plan and sync against a virtual device and prints JSON. Record the results in RESULT.
- Profiling: `cargo flamegraph` / Instruments (macOS) / `perf` (Linux) / Superluminal or WPA (Windows). For GPUI frames, enable the frame timing overlay in debug builds.
- Tracing spans include `elapsed_ms`; `wtunes --trace` writes a Chrome trace file.

---

## 4. CPU architectures (x86-64, ARM, RISC-V)

**Quality and performance are priority #1** ([ADR-0013](ARCHITECTURE.md#adr-0013-native-performance-first-multi-architecture-simd)).
This section records what we know about each architecture and how the code
must treat it. Last researched: 2026-11-09.

### 4.1 Background
- **RISC vs. CISC.** x86-64 is a CISC instruction set (internally it decodes to RISC-like micro-ops). **ARM (AArch64) is a RISC architecture.** **RISC-V** is an *open*, royalty-free RISC architecture. For us the difference that matters is not RISC vs. CISC but **which SIMD (vector) units exist and how Rust can reach them**.
- **SIMD units:**

| Architecture | SIMD | Width | Availability |
|---|---|---|---|
| x86-64 | SSE2 (baseline), SSE4.1, AVX2 + FMA, AVX-512 | 128 / 256 / 512 bit | SSE4.1/AVX2 on virtually all PCs from the last decade; AVX-512 only on some CPUs, so it is not targeted |
| AArch64 | NEON (Advanced SIMD); SVE/SVE2 on some server chips | 128 bit (NEON) | NEON is **mandatory** in ARMv8-A: always present on Apple Silicon, Snapdragon X, Raspberry Pi 4/5 |
| RISC-V 64 | RVV 1.0 (Vector extension) | Scalable (VLEN 128+) | Present on newer boards only; **Rust vector intrinsics are not yet stable** (only some ratified scalar extensions are stable) `[VERIFY at each release]` |

### 4.2 What our libraries give us
- **`rustfft`** (through `realfft`): the planner detects AVX/FMA and SSE4.1 on x86-64 at runtime and uses NEON on AArch64. These are enabled by default. On RISC-V it falls back to scalar code. *Always plan through `FftPlanner`*: building FFT algorithms by hand bypasses the SIMD paths.
- **`symphonia`** (decoding) is portable Rust; its speed comes from the compiler's auto-vectorization and good memory access patterns.
- **Our own kernels** (window, magnitude, dB, colormap, statistics) use a portable SIMD crate (`pulp` or `wide`) and are compiled in several variants with `multiversion`, selected at runtime.

### 4.3 Build policy

| Target triple | Release `target-cpu` / features | Runtime dispatch |
|---|---|---|
| `x86_64-apple-darwin`, `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu` | `x86-64-v2` (SSE4.2, POPCNT) | AVX2 + FMA when detected |
| `aarch64-apple-darwin` | default (Apple M1 baseline) | — (NEON always) |
| `aarch64-pc-windows-msvc`, `aarch64-unknown-linux-gnu` | default ARMv8-A | Optional ARMv8.2 dot-product paths later |
| `riscv64gc-unknown-linux-gnu` | `RV64GC` | Scalar only until RVV intrinsics stabilize |

- Never ship `target-cpu=native`. It produces binaries that crash on older CPUs. It is allowed only for local benchmarks, and any RESULT entry must state it.
- `unsafe` SIMD intrinsics are allowed **only** inside `wt-dsp`, behind an ADR-0013 exception to `#![forbid(unsafe_code)]`, and every such function needs a scalar reference implementation and an equivalence test ([TESTING §7](TESTING.md#7-numerical-and-cross-architecture-tests)).

### 4.4 Memory, caches and Apple Silicon
- Apple Silicon and other modern ARM cores have large caches and very high memory bandwidth, with performance and efficiency cores. Batch jobs should not pin every core: leave one free for the UI thread, and let the OS schedule.
- Process audio in blocks that fit in L1/L2 (an FFT of 4096 `f32` = 16 KiB). Use structure-of-arrays per channel to keep SIMD loads contiguous.
- Avoid false sharing in parallel loops: each worker writes its own output rows.

### 4.5 Verification
- CI builds and tests on **x86-64 and ARM64** natively (GitHub-hosted ARM64 runners for Linux and macOS), and cross-builds **riscv64** with tests run under QEMU ([DEVELOPMENT §6](DEVELOPMENT.md#6-ci-pipeline-github-actions)).
- Benchmarks (B-12…B-17) are recorded per architecture in RESULT, starting with R-004.

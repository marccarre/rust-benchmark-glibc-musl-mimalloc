---
phase: 01-foundation-mvp-slice
plan: "02"
subsystem: harness
tags: [rust, harness, hdrhistogram, getrusage, statm, multithread, scenario, schema, results-json]

requires:
  - phase: 01-01
    provides: "Cargo workspace, allocator features, build metadata, Walking Skeleton CLI"
provides:
  - "Locked v1 results.json schema (Run/Env/Build/Scenario/Harness/Metrics serde structs)"
  - "Harness loop: warm-up + measurement with HDR histogram, 1Hz RSS sampling, getrusage at end"
  - "Scenario trait contract (name, config_json, setup, tick, teardown) with SinkValue blanket impl"
  - "Multithread scenario with thread::scope + SizeDist (uniform|bimodal|pareto)"
  - "CLI wiring: parse_duration helper, run_multithread building the Run record from harness outcome"
  - "Integration test verifying end-to-end JSON emission"
affects:
  - "Phase 2 (Scenario Fan-Out — additional scenarios implement the same Scenario trait)"
  - "Phase 4 (Aggregator — consumes results.json with schema_version=1)"

tech-stack:
  added: []
  patterns:
    - "Scenario trait with explicit name() + config_json() for self-describing JSON output"
    - "HDR histogram bounds [1ns, 60s, 3 sig figs] for nanosecond-resolution percentile latency"
    - "1Hz RSS sampling via /proc/self/statm during measurement window (Linux only)"
    - "End-of-run getrusage capture for peak_rss_kb, page faults, context switches"
    - "warm-up >= 1s guard via anyhow::bail in harness::run"
    - "Allocator-stats injection via closure (core lib stays allocator-feature-free)"

key-files:
  created:
    - "crates/alloc-bench-core/src/output.rs (schema v1 structs)"
    - "crates/alloc-bench-core/src/harness.rs (Scenario trait + run loop)"
    - "crates/alloc-bench-core/src/metrics/rusage.rs (getrusage wrapper)"
    - "crates/alloc-bench-core/src/metrics/statm.rs (/proc/self/statm reader)"
    - "crates/alloc-bench-core/src/metrics/env.rs (cpu_model, cpu_count, memory_total_kb)"
    - "crates/alloc-bench-core/src/metrics/mod.rs"
    - "crates/alloc-bench-core/src/scenarios/multithread.rs (Multithread + SizeDist)"
    - "crates/alloc-bench-core/src/scenarios/mod.rs"
    - "crates/alloc-bench-cli/src/run.rs (parse_duration + run_multithread)"
    - "crates/alloc-bench-cli/tests/multithread_smoke.rs (integration test)"
  modified:
    - "crates/alloc-bench-core/src/lib.rs (real module declarations)"
    - "crates/alloc-bench-cli/src/main.rs (wire Multithread arm to run::run_multithread)"
    - "Cargo.toml (rand += small_rng feature)"

key-decisions:
  - "Allocator stats injected via closure — core lib stays libray-only (no #[global_allocator] in lib crates)"
  - "RSS sampling uses /proc/self/statm not /proc/self/status — much faster, fewer parse hops"
  - "Histogram upper bound 60s (60_000_000_000ns) — sufficient for multithread, will be revisited for fragmentation-soak in Phase 2"
  - "parse_duration is hand-rolled — avoids humantime dep for three-suffix grammar (ms|s|m)"

patterns-established:
  - "Scenario implementations: thread::scope for ownership, mid-buffer write to defeat DCE, black_box every tick result"
  - "Metrics layer is platform-aware via cfg!(target_os = ...) — Linux/macOS divergence in /proc availability and ru_maxrss units"

requirements-completed: [HARN-01, HARN-02, HARN-03, HARN-04, HARN-05, HARN-06, HARN-07, HARN-08, SCEN-01, REPR-02]

duration: ~45min
completed: 2026-05-18
---

# Phase 1, Plan 02: Harness, Metrics, Multithread Scenario, results.json

**Real harness loop with HDR-histogram percentile latency, getrusage + /proc/self/statm metrics, multithread allocation stress scenario, and locked v1 results.json schema emitted end-to-end**

## Performance

- **Duration:** ~45 min (resumed from interrupted session)
- **Completed:** 2026-05-18
- **Tasks:** 10
- **Files modified:** 11

## Accomplishments

- Locked v1 `results.json` schema with serde-Serialize structs for every required field
- Harness `run()` enforces warm-up >= 1s, runs warm-up + measurement loops with `std::hint::black_box` discipline
- HDR histogram captures p50/p95/p99/p999/max latency in nanoseconds with 3 significant figures
- 1Hz RSS sampling during measurement window via `/proc/self/statm` (Linux), fallback to 0 elsewhere
- End-of-run `getrusage` capture for peak RSS (kB on Linux, bytes/1024 on macOS), page faults, voluntary/involuntary context switches
- Multithread scenario with `std::thread::scope`, mid-buffer write (PITFALLS §1.2), and three size distributions
- CLI `run_multithread()` wires scenario → harness → JSON output with `parse_duration` helper for human strings ("5s"/"500ms"/"2m")
- Integration test spawns the CLI binary, validates schema, and confirms `metrics.allocator_stats.kind` is one of the expected values
- 7/7 tests pass (5 unit + 1 doc + 1 integration); release build with LTO=fat works on macOS host

## Task Commits

1. **Tasks 1-7: Core lib (output schema, harness, metrics, multithread scenario)** - `5bf773f` (feat)
2. **Task 8: CLI run.rs wiring** - `0620e2c` (feat)
3. **Task 9: Integration test** - `197beb9` (test)
4. **Task 10: Final fmt** - `98187f1` (style)

## Files Created/Modified

**Created:**
- `crates/alloc-bench-core/src/output.rs` — Run/Env/Build/ScenarioInfo/HarnessInfo/LatencyNs/RssGrowthSample/Rusage/Metrics structs
- `crates/alloc-bench-core/src/harness.rs` — Scenario trait, SinkValue blanket impl, run() + warm-up guard
- `crates/alloc-bench-core/src/metrics/rusage.rs` — getrusage wrapper with macOS unit conversion
- `crates/alloc-bench-core/src/metrics/statm.rs` — /proc/self/statm reader (Linux-only)
- `crates/alloc-bench-core/src/metrics/env.rs` — cpu_model, cpu_count, memory_total_kb, docker_image
- `crates/alloc-bench-core/src/metrics/mod.rs` — allocator_stats_default() stub
- `crates/alloc-bench-core/src/scenarios/multithread.rs` — Multithread + SizeDist + sample_size helper
- `crates/alloc-bench-core/src/scenarios/mod.rs`
- `crates/alloc-bench-cli/src/run.rs` — parse_duration() + run_multithread()
- `crates/alloc-bench-cli/tests/multithread_smoke.rs` — end-to-end integration test

**Modified:**
- `crates/alloc-bench-core/src/lib.rs` — real module declarations replacing Walking Skeleton stubs
- `crates/alloc-bench-cli/src/main.rs` — wire Multithread arm to `run::run_multithread`
- `Cargo.toml` — rand += small_rng feature

## Decisions Made

- **Histogram upper bound 60s**: Sufficient for multithread; flagged as Phase 2 risk for fragmentation-soak.
- **RSS sampling cadence 1Hz**: Cheap parse of /proc/self/statm, low overhead; sub-second granularity not needed for trend analysis.
- **Allocator stats via closure**: Keeps `alloc-bench-core` allocator-feature-free (libraries can't declare `#[global_allocator]`). The CLI passes `allocator::stats` into `harness::run`.
- **`parse_duration` hand-rolled**: No humantime dep needed for the three-suffix grammar (ms/s/m).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] `__errno_location` is Linux-only**
- **Found during:** Resume verification (compile check on macOS)
- **Issue:** Original `metrics/rusage.rs` panicked the macOS build because `libc::__errno_location` doesn't exist on Apple targets (it's `libc::__error()` there).
- **Fix:** Removed errno introspection — the `getrusage` failure path is extremely rare in practice; `anyhow::ensure!(ret == 0, "getrusage failed")` is sufficient.
- **Files modified:** `crates/alloc-bench-core/src/metrics/rusage.rs`
- **Verification:** `cargo test -p alloc-bench-core` passes on macOS host.

**2. [Rule 1 — Critical Missing] `rand` workspace dep missing `small_rng` feature**
- **Found during:** Test compilation
- **Issue:** `rand::rngs::SmallRng` is gated behind the `small_rng` cargo feature in rand 0.8.x. Plan didn't specify enabling it, but the multithread scenario uses it for per-thread seeded RNGs.
- **Fix:** Added `features = ["small_rng"]` to the workspace `rand` dep.
- **Files modified:** `Cargo.toml`
- **Verification:** `cargo test -p alloc-bench-core` passes.

**3. [Rule 5 — Cleanup] `HarnessOutcome` missing `Debug` impl**
- **Found during:** Test compilation
- **Issue:** `harness::tests::warmup_too_short_returns_error` calls `.unwrap_err()`, which requires `Debug` on the success type.
- **Fix:** Added `#[derive(Debug)]` to `HarnessOutcome`.
- **Files modified:** `crates/alloc-bench-core/src/harness.rs`
- **Verification:** Test compiles and passes.

**4. [Rule 5 — Cleanup] Pareto sample_size float-type ambiguity**
- **Found during:** Test compilation
- **Issue:** `(1.0 - u).powf(...)` was ambiguous between f32 and f64.
- **Fix:** Annotated literals as `1.0_f64` and `0.0_f64`.
- **Files modified:** `crates/alloc-bench-core/src/scenarios/multithread.rs`
- **Verification:** Compiles cleanly with `-D warnings`.

---

**Total deviations:** 4 auto-fixed (1 blocking platform issue, 1 critical missing dep feature, 2 cleanup)
**Impact on plan:** All necessary for correctness. No scope creep — every fix was the minimum change to land Plan 02's must-haves.

## Issues Encountered

- **Resumed from interrupted worktree session** — prior session committed Tasks 1-7 in a worktree, then the worktree branch was dropped before merging cleanly back. Files were preserved as untracked files in the main worktree; resumed by re-staging, re-checking, and proceeding from Task 8.

## Self-Check: PASSED

- `cargo build --workspace --release` exits 0 (LTO=fat)
- `cargo clippy --workspace --all-targets -- -D warnings` exits 0
- `cargo fmt --all --check` exits 0
- `cargo test --workspace` 7/7 pass
- `target/release/alloc-bench-cli multithread --threads 4 --objects 1000 --warmup 1s --duration 2s --output /tmp/phase1-smoke.json` produces a fully-populated v1 JSON
- Version banner matches contract format on stderr

## Next Phase Readiness

- Phase 2 (Scenario Fan-Out) can proceed: Scenario trait is locked; new scenarios drop into `crates/alloc-bench-core/src/scenarios/` and dispatch from `cli/run.rs`.
- Phase 4 (Aggregator) can rely on schema_version=1 contract.
- macOS host smoke established as a baseline. Linux Docker matrix (Phase 3) is the real target for jemalloc/mimalloc cross-allocator runs.

---
*Phase: 01-foundation-mvp-slice*
*Completed: 2026-05-18*

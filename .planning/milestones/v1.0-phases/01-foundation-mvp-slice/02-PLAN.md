---
plan_id: "02"
phase: "01"
wave: 2
depends_on: ["01"]
autonomous: true
files_modified:
  - "crates/alloc-bench-core/src/harness.rs"
  - "crates/alloc-bench-core/src/scenarios/mod.rs"
  - "crates/alloc-bench-core/src/scenarios/multithread.rs"
  - "crates/alloc-bench-core/src/metrics/mod.rs"
  - "crates/alloc-bench-core/src/metrics/rusage.rs"
  - "crates/alloc-bench-core/src/metrics/statm.rs"
  - "crates/alloc-bench-core/src/metrics/env.rs"
  - "crates/alloc-bench-core/src/output.rs"
  - "crates/alloc-bench-core/src/lib.rs"
  - "crates/alloc-bench-cli/src/main.rs"
  - "crates/alloc-bench-cli/src/run.rs"
requirements_addressed:
  - HARN-01
  - HARN-02
  - HARN-03
  - HARN-04
  - HARN-05
  - HARN-06
  - HARN-07
  - HARN-08
  - SCEN-01
  - REPR-02
---

# Phase 1, Plan 02 — Harness, Metrics, Multithread Scenario, results.json

## Objective

Fill the Walking Skeleton with the real harness loop (warm-up + measurement), the metrics layer (getrusage + /proc/self/statm + allocator-internal stats), the first scenario (multithread allocation stress), and the locked v1 results.json schema. After this plan, `alloc-bench-cli multithread --warmup 5s --duration 30s --output run.json` produces a fully-populated results.json validating the schema.

## Must-haves (goal-backward)

1. `alloc-bench-cli multithread --threads 8 --objects 100000 --size-dist uniform --warmup 5s --duration 30s --output run.json` runs to completion on Linux.
2. The emitted `run.json` validates against the schema and contains:
   - `schema_version: 1`
   - `env.os`, `env.cpu_model`, `env.cpu_count`, `env.memory_total_kb`, `env.docker_image` (null on host)
   - `build.allocator`, `build.rustc_version`, `build.target_triple`, `build.host_triple`, `build.profile`, `build.git_sha`, `build.git_dirty`, `build.build_timestamp`, `build.rustflags`
   - `scenario.name == "multithread"` and `scenario.config` with all CLI params
   - `harness.warmup_duration_s`, `harness.measurement_duration_s`, `harness.samples_count`
   - `metrics.throughput_ops_per_s` (positive)
   - `metrics.latency_ns.{p50,p95,p99,p999,max}` (all positive)
   - `metrics.peak_rss_kb` (positive)
   - `metrics.rss_growth_samples` (non-empty array on Linux; macOS may emit empty array)
   - `metrics.rusage.{user_time_s,sys_time_s,minor_faults,major_faults,voluntary_csw,involuntary_csw}` populated
   - `metrics.allocator_stats.kind` ∈ {"jemalloc","mimalloc","system"}; jemalloc emits allocated/resident/retained/active
3. Building with `--features alloc-jemalloc` and running the multithread scenario emits `metrics.allocator_stats.kind == "jemalloc"`.
4. The harness panics if `--warmup` < 1s with message containing "warm-up must be >= 1s".
5. Every measured tick is wrapped in `std::hint::black_box(scenario.tick())` (verified by source grep).

## Tasks

### Task 1: results.json schema (output.rs)

<read_first>
- .planning/phases/01-foundation-mvp-slice/01-CONTEXT.md (D-11, D-12, D-13)
- .planning/phases/01-foundation-mvp-slice/01-RESEARCH.md §"results.json schema"
- .planning/research/ARCHITECTURE.md §3
</read_first>

<action>
Write `crates/alloc-bench-core/src/output.rs` with serde-Serialize structs: `Run` (top-level), `Env`, `Build`, `ScenarioInfo`, `HarnessInfo`, `LatencyNs`, `RssGrowthSample`, `Metrics`. Pub const `SCHEMA_VERSION: u32 = 1`. Use `serde_json::Value` for `Metrics.allocator_stats` and `ScenarioInfo.config`.

The exact field names and types must match `01-CONTEXT.md` D-11 schema and the Run struct sketch in `01-RESEARCH.md`. Allow `#[serde(skip_serializing_if = "Option::is_none")]` only for `Env.docker_image`.
</action>

<acceptance_criteria>
- File `crates/alloc-bench-core/src/output.rs` exists with exact struct names: `Run`, `Env`, `Build`, `ScenarioInfo`, `HarnessInfo`, `LatencyNs`, `RssGrowthSample`, `Metrics`
- All structs derive `Serialize`
- `cargo check -p alloc-bench-core` exits 0
- `serde_json::to_value(Run{...})` produces a JSON object with top-level keys: `schema_version`, `run_id`, `env`, `build`, `scenario`, `harness`, `metrics`
</acceptance_criteria>

### Task 2: Metrics — rusage.rs

<read_first>
- crates/alloc-bench-core/src/output.rs (Rusage struct shape)
- .planning/phases/01-foundation-mvp-slice/01-CONTEXT.md (D-14, D-16)
- .planning/phases/01-foundation-mvp-slice/01-RESEARCH.md §"Metrics"
</read_first>

<action>
Create `crates/alloc-bench-core/src/metrics/mod.rs` declaring `pub mod rusage; pub mod statm; pub mod env;`.

Write `crates/alloc-bench-core/src/metrics/rusage.rs`:
- Use `libc::{getrusage, rusage, RUSAGE_SELF}`
- Define `pub fn read_rusage() -> anyhow::Result<Rusage>` (Rusage moves from output.rs into metrics/rusage.rs, re-exported from output.rs for the schema)
- On Linux: `ru_maxrss` is in kB; store directly as `peak_rss_kb`
- On macOS: `ru_maxrss` is in bytes; divide by 1024 to get kB. Note this divergence with `cfg!(target_os = "macos")`.
- All time fields convert from `(tv_sec, tv_usec)` to `f64` seconds
</action>

<acceptance_criteria>
- File `crates/alloc-bench-core/src/metrics/rusage.rs` exists
- Contains `pub fn read_rusage() -> anyhow::Result<Rusage>`
- Contains `cfg!(target_os = "macos")` branch for `ru_maxrss` unit conversion
- `cargo check -p alloc-bench-core` exits 0
- Calling `read_rusage()` from a unit test returns `Ok` with `peak_rss_kb > 0`
</acceptance_criteria>

### Task 3: Metrics — statm.rs

<read_first>
- crates/alloc-bench-core/src/output.rs (RssGrowthSample shape)
- .planning/phases/01-foundation-mvp-slice/01-CONTEXT.md (D-15)
- .planning/phases/01-foundation-mvp-slice/01-RESEARCH.md §"Metrics"
</read_first>

<action>
Write `crates/alloc-bench-core/src/metrics/statm.rs`:
- `pub fn read_rss_kb() -> anyhow::Result<u64>`
- On Linux: read `/proc/self/statm`, take field 2 (resident pages), multiply by `libc::sysconf(libc::_SC_PAGESIZE) / 1024`
- On non-Linux: return `Ok(0)` with a doc comment saying "non-Linux platforms return 0; fall back to getrusage for peak RSS"
- Use `libc::sysconf` not the `page_size` crate (avoids extra dep)

Add unit test `#[cfg(test)] mod tests { ... }` that asserts `read_rss_kb()` returns >0 on Linux, 0 on non-Linux.
</action>

<acceptance_criteria>
- File `crates/alloc-bench-core/src/metrics/statm.rs` exists
- Contains `pub fn read_rss_kb() -> anyhow::Result<u64>`
- Has `#[cfg(target_os = "linux")]` and `#[cfg(not(target_os = "linux"))]` branches
- Test passes on macOS (returns 0) and Linux (returns >0)
- `cargo test -p alloc-bench-core` exits 0
</acceptance_criteria>

### Task 4: Metrics — env.rs

<read_first>
- crates/alloc-bench-core/src/output.rs (Env shape)
- .planning/phases/01-foundation-mvp-slice/01-CONTEXT.md (REPR-02)
</read_first>

<action>
Write `crates/alloc-bench-core/src/metrics/env.rs`:
- `pub fn read_env() -> anyhow::Result<Env>` returning the populated `Env` struct
- `os`: use `std::env::consts::OS`
- `os_version`: read `/proc/version` first line on Linux; on macOS run `uname -r` via `Command`; on other: "unknown"
- `cpu_model`: parse `/proc/cpuinfo` "model name" first occurrence on Linux; on macOS use `sysctl -n machdep.cpu.brand_string` via `Command`; on other: "unknown"
- `cpu_count`: `num_cpus::get() as u32`
- `memory_total_kb`: parse `/proc/meminfo` "MemTotal" line on Linux; on macOS use `sysctl -n hw.memsize` (bytes → /1024); on other: 0
- `docker_image`: read `DOCKER_IMAGE` env var if set, else None (will be set by Phase 3 Dockerfile ENV)
</action>

<acceptance_criteria>
- File `crates/alloc-bench-core/src/metrics/env.rs` exists
- Contains `pub fn read_env() -> anyhow::Result<Env>`
- Returns populated Env on macOS host (cpu_model non-empty, cpu_count > 0, memory_total_kb > 0)
- `docker_image` is None when `DOCKER_IMAGE` env var not set
</acceptance_criteria>

### Task 5: Metrics — allocator stats integration

<read_first>
- crates/alloc-bench-cli/src/allocator.rs (Plan 01 Task 3)
- crates/alloc-bench-core/src/metrics/mod.rs (just written)
- .planning/phases/01-foundation-mvp-slice/01-CONTEXT.md (D-17)
</read_first>

<action>
Add to `crates/alloc-bench-core/src/metrics/mod.rs`:
```rust
/// Sample allocator-internal stats. CLI passes its own `allocator::stats()` closure
/// to avoid the core lib needing allocator features.
pub fn allocator_stats_default() -> serde_json::Value {
    serde_json::json!({"kind": "system"})
}
```

The CLI's `run.rs` (next task) passes the `allocator::stats()` function to the harness. The core lib stays allocator-feature-free (libraries can't have global_allocator).
</action>

<acceptance_criteria>
- `crates/alloc-bench-core/src/metrics/mod.rs` contains `pub fn allocator_stats_default()` returning `{"kind": "system"}`
- Core lib has zero allocator-specific dependencies
</acceptance_criteria>

### Task 6: Harness

<read_first>
- crates/alloc-bench-core/src/output.rs (HarnessInfo, Metrics, LatencyNs)
- crates/alloc-bench-core/src/metrics/* (just written)
- .planning/phases/01-foundation-mvp-slice/01-CONTEXT.md (D-08, D-09, D-10)
- .planning/phases/01-foundation-mvp-slice/01-RESEARCH.md §"Harness contract"
</read_first>

<action>
Write `crates/alloc-bench-core/src/harness.rs`:
- `pub trait Scenario { fn name(&self) -> &'static str; fn config_json(&self) -> serde_json::Value; fn setup(&mut self) -> anyhow::Result<()>; fn tick(&mut self) -> Box<dyn SinkValue>; fn teardown(&mut self) {} }`
- `pub trait SinkValue {}` with blanket `impl<T: ?Sized + 'static> SinkValue for T {}`
- `pub struct HarnessConfig { pub warmup: Duration, pub measure: Duration, pub seed: u64 }`
- `pub struct HarnessOutcome { pub harness: HarnessInfo, pub metrics: Metrics }`
- `pub fn run<S: Scenario, F: Fn() -> serde_json::Value>(scenario: &mut S, cfg: &HarnessConfig, alloc_stats: F) -> anyhow::Result<HarnessOutcome>`
  - Bail if `cfg.warmup < Duration::from_secs(1)` with message "warm-up must be >= 1s; allocator caches need to populate (see PITFALLS.md §1.5)"
  - Phase 1: warm-up — `while Instant::now() < warm_end { std::hint::black_box(scenario.tick()); }`
  - Phase 2: measurement — record per-tick latency to `Histogram::<u64>::new_with_bounds(1, 60_000_000_000, 3)`, sample `read_rss_kb()` every 1s into `Vec<RssGrowthSample>`, count samples
  - Compute `throughput = samples_count as f64 / elapsed_s`
  - Read `LatencyNs` from histogram quantiles (50, 95, 99, 99.9, max)
  - Read `read_rusage()` once at end; populate `Metrics.peak_rss_kb` from `rusage.peak_rss_kb` and `Metrics.rusage` from the same struct
  - Call `alloc_stats()` to get `metrics.allocator_stats`
- Expose this via `pub mod harness;` in `lib.rs`

Update `crates/alloc-bench-core/src/lib.rs`:
```rust
pub mod harness;
pub mod metrics;
pub mod output;
pub mod scenarios;

pub use harness::{HarnessConfig, HarnessOutcome, Scenario, SinkValue, run};
pub use output::SCHEMA_VERSION;
```
</action>

<acceptance_criteria>
- File `crates/alloc-bench-core/src/harness.rs` exists
- Contains `pub trait Scenario` with all methods listed above
- Contains `pub fn run<S: Scenario, F: Fn() -> serde_json::Value>(...)`
- `grep -c 'std::hint::black_box' crates/alloc-bench-core/src/harness.rs` returns at least 2 (warm-up loop + measurement loop)
- Source contains `bail!` or `anyhow::bail!` checking `cfg.warmup < Duration::from_secs(1)`
- Source contains `new_with_bounds(1, 60_000_000_000, 3)`
- `cargo check -p alloc-bench-core` exits 0
</acceptance_criteria>

### Task 7: Multithread scenario

<read_first>
- crates/alloc-bench-core/src/harness.rs (Scenario trait)
- .planning/phases/01-foundation-mvp-slice/01-CONTEXT.md (D-18, D-19, D-20)
- .planning/phases/01-foundation-mvp-slice/01-RESEARCH.md §"First scenario: multithread"
</read_first>

<action>
Write `crates/alloc-bench-core/src/scenarios/mod.rs`:
```rust
pub mod multithread;
pub use multithread::{Multithread, MultithreadConfig, SizeDist};
```

Write `crates/alloc-bench-core/src/scenarios/multithread.rs`:
- `pub enum SizeDist { Uniform, Bimodal, Pareto }` with `FromStr` impl accepting "uniform"|"bimodal"|"pareto"
- `pub struct MultithreadConfig { pub threads, objects, size_dist, size_min, size_max, seed }`
- `pub struct Multithread { cfg }` with `pub fn new(cfg) -> Self`
- `impl Scenario for Multithread`:
  - `name()` returns `"multithread"`
  - `config_json()` serializes config to JSON
  - `setup()` returns Ok
  - `tick()`:
    - Use `std::thread::scope` to spawn `cfg.threads` threads
    - Each thread seeds a `SmallRng::seed_from_u64(cfg.seed.wrapping_add(t as u64))`
    - Allocates `cfg.objects` `Box<[u8]>` of distribution-drawn size
    - Writes to `b[size/2]` (PITFALLS.md §1.2)
    - `black_box`es each box and accumulates them in a `Vec<Box<[u8]>>` (dropped on thread exit)
  - `teardown()` is default empty
- Distribution sampling helper `sample_size`: uniform = `gen_range(min..=max)`; bimodal = `if rng.gen::<f32>() < 0.9 { 16 } else { max }`; pareto = inverse-CDF α=1.5 clamped to range.
</action>

<acceptance_criteria>
- File `crates/alloc-bench-core/src/scenarios/multithread.rs` exists
- Contains `pub enum SizeDist` with `Uniform`, `Bimodal`, `Pareto` variants
- `impl Scenario for Multithread` exists
- Source contains `std::thread::scope`
- Source contains `b[size / 2]` or similar mid-buffer write
- Source contains `std::hint::black_box`
- `cargo test -p alloc-bench-core` exits 0
</acceptance_criteria>

### Task 8: CLI run.rs — wire scenario to harness to results.json

<read_first>
- crates/alloc-bench-core/src/lib.rs (re-exports)
- crates/alloc-bench-cli/src/main.rs (Plan 01 Task 5)
- .planning/phases/01-foundation-mvp-slice/01-CONTEXT.md (D-23, D-24)
</read_first>

<action>
Write `crates/alloc-bench-cli/src/run.rs`:
- `pub fn run_multithread(threads, objects, size_dist, size_min, size_max, warmup, duration, seed, output) -> anyhow::Result<()>`
- Parse `warmup` and `duration` from human strings ("5s", "60s", "1m") into `Duration` — write a small `parse_duration` helper accepting `s`, `ms`, `m` suffixes (no humantime dep)
- Parse `size_dist` from string into `SizeDist::FromStr`
- Build `MultithreadConfig` and `Multithread::new(...)`
- Build `HarnessConfig { warmup, measure: duration, seed }`
- Call `alloc_bench_core::run(&mut scenario, &cfg, allocator::stats)`
- Build a `Run` struct: schema_version=1, run_id = `format!("{}-{}", chrono::Utc::now().to_rfc3339(), &build_info::GIT_SHA[..8])`, env=read_env()?, build=Build {allocator: allocator::name(), rustc_version, target_triple, host_triple, profile, git_sha, git_dirty: BUILD_INFO::GIT_DIRTY=="true", build_timestamp, rustflags}, scenario=ScenarioInfo {name, config}, harness, metrics
- If `output` is Some, write JSON pretty-printed to file; else write to stdout

Add `Cargo.toml` deps to `alloc-bench-cli`: `chrono = { workspace = true }`.

Update `main.rs` `Multithread` arm to call `run::run_multithread(...)`.
</action>

<acceptance_criteria>
- File `crates/alloc-bench-cli/src/run.rs` exists
- Contains `pub fn run_multithread(...)`
- Contains a `parse_duration` helper
- `target/release/alloc-bench-cli multithread --threads 2 --objects 100 --warmup 1s --duration 1s --output /tmp/run.json` exits 0
- Resulting `/tmp/run.json` parses as JSON and has `schema_version == 1`, populated `env.cpu_count > 0`, populated `metrics.latency_ns.p50 > 0`, `metrics.peak_rss_kb > 0`
</acceptance_criteria>

### Task 9: Integration test

<read_first>
- crates/alloc-bench-cli (entire crate)
</read_first>

<action>
Write `crates/alloc-bench-cli/tests/multithread_smoke.rs`:
- Uses `assert_cmd` and `tempfile` (add as dev-dependencies in Cargo.toml)
- Spawns `alloc-bench-cli multithread --threads 2 --objects 100 --warmup 1s --duration 1s --output <tempdir>/run.json`
- Asserts exit code 0
- Reads the JSON, asserts: `schema_version == 1`, `scenario.name == "multithread"`, `metrics.throughput_ops_per_s > 0.0`, `metrics.latency_ns.p50 > 0`, `harness.warmup_duration_s == 1`, `metrics.allocator_stats.kind` is one of the expected values

Add to `crates/alloc-bench-cli/Cargo.toml`:
```toml
[dev-dependencies]
assert_cmd = "2"
tempfile = "3"
```

Also add a unit test in `crates/alloc-bench-core/src/harness.rs` verifying that `run()` with `cfg.warmup = Duration::from_millis(500)` returns Err containing "warm-up must be >= 1s".
</action>

<acceptance_criteria>
- File `crates/alloc-bench-cli/tests/multithread_smoke.rs` exists
- `cargo test --workspace --release` exits 0 (run with --release because the bench needs LTO=fat to be exercised)
- The smoke test runs the CLI end-to-end and validates the JSON output
- The harness warm-up unit test passes
</acceptance_criteria>

### Task 10: Final smoke + commit

<read_first>
- All files written
</read_first>

<action>
Run:
```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
target/release/alloc-bench-cli --version 2>&1 | head -1
target/release/alloc-bench-cli multithread --threads 4 --objects 1000 --warmup 1s --duration 2s --output /tmp/phase1-smoke.json
cat /tmp/phase1-smoke.json | head -40
```

Verify the JSON parses, is well-formed, and has all required schema fields.
</action>

<acceptance_criteria>
- All commands above exit 0
- `/tmp/phase1-smoke.json` is valid JSON with `schema_version: 1` and populated env/build/scenario/harness/metrics blocks
- The version banner on stderr matches the contract format
- Clippy passes with `-D warnings`
- All workspace tests pass
</acceptance_criteria>

## Verification

Phase 1 success criteria from ROADMAP.md (all must be TRUE after this plan):

1. ✅ Build succeeds with allocator features and prints version banner — covered by Plan 01 Task 5 + Plan 02 Task 8
2. ✅ Multithread scenario runs warmup + measurement with black_box — Plan 02 Task 6 + 7
3. ✅ results.json has all required fields — Plan 02 Task 1 + 8 + 9
4. ✅ Mutual-exclusion enforcement — Plan 01 Task 3 (compile_error!) + Plan 02 (runtime panic in main.rs)

## Risks

- **Linux-only paths on macOS host:** `/proc/self/statm` is empty; `metrics.statm` returns 0. Tests must accept this — the smoke test runs on the host (macOS) but Phase 3 Docker matrix is the real target. Code uses `cfg!(target_os = "linux")` to guard.
- **black_box discipline:** if a future scenario forgets to write inside the allocated buffer, DCE may eliminate the allocation. The smoke test in Plan 02 Task 9 doesn't directly verify this — Phase 2 will add a dedicated DCE-detection test.
- **Histogram bounds:** 60s upper bound may be too tight for the fragmentation-soak scenario in Phase 2. Currently fine for multithread; revisit in Phase 2.

## Dependencies

- Plan 01 must be complete (workspace, allocator features, build metadata, Walking Skeleton CLI).

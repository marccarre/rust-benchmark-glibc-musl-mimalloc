# Phase 1: Foundation MVP Slice - Context

**Gathered:** 2026-05-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Deliver an end-to-end vertical slice for ONE allocator combo (glibc-jemalloc on x86_64-unknown-linux-gnu) proving the loop: Cargo workspace → `#[global_allocator]` feature selection → custom harness with warm-up + per-op latency + RSS sampling + allocator-internal stats → first benchmark scenario (multithread allocation stress) → fully-populated `results.json` matching the schema.

Phase 1 does NOT add any other allocator combos, scenarios beyond `multithread`, Docker images, the aggregator, the dashboard, or CI. Those are Phases 2-5.

</domain>

<decisions>
## Implementation Decisions

### Workspace shape
- **D-01:** Three Cargo workspace crates created in Phase 1: `crates/alloc-bench-core` (library — harness + scenarios + metrics + output schema), `crates/alloc-bench-cli` (binary — `#[global_allocator]` feature flags + clap subcommands), `crates/alloc-bench-aggregator` (binary — placeholder `main.rs` that prints "Phase 4" and exits; full implementation deferred). Workspace root `Cargo.toml` defines `[workspace.package]` shared metadata and `[profile.release]` flags from the spec.
- **D-02:** `[profile.release]` in workspace root: `lto = "fat"`, `codegen-units = 1`, `opt-level = 3`, `strip = "symbols"`, `debug = false`, `panic = "abort"`. Mandatory.

### Allocator feature selection
- **D-03:** Cargo features on `alloc-bench-cli` only: `default = []` (system allocator), `alloc-jemalloc` (depends on `tikv-jemallocator` 0.6 + `tikv-jemalloc-ctl` 0.6, optional), `alloc-mimalloc` (depends on `mimalloc` 0.1.43 with `default-features = false`, optional). No `alloc-system` feature — it's the empty default.
- **D-04:** Mutual exclusion enforced via `compile_error!` in `crates/alloc-bench-cli/src/allocator.rs` guarded by `#[cfg(all(feature = "alloc-jemalloc", feature = "alloc-mimalloc"))]`. This is strictly stronger than the roadmap's "panics on startup" success criterion (catches the mistake at compile time). The CLI also has a runtime panic at `main()` entry as a defense-in-depth check using `cfg!` so the success-criterion text remains literally satisfied.
- **D-05:** Allocator name detected at compile time and exposed as `pub const ALLOCATOR_NAME: &str` in `alloc-bench-cli`: `"jemalloc"` / `"mimalloc"` / fallback to `"ptmalloc"` (cfg target_env != musl, target_os = linux), `"mallocng"` (cfg target_env = musl), `"libmalloc"` (cfg target_os = macos).

### Build metadata injection
- **D-06:** Use `vergen` 9 with features `["build", "cargo", "git", "rustc"]` plus `vergen-gitcl` for git SHA. Cuts custom `build.rs` to ~10 lines. Emits `VERGEN_RUSTC_SEMVER`, `VERGEN_RUSTC_HOST_TRIPLE`, `VERGEN_CARGO_TARGET_TRIPLE`, `VERGEN_GIT_SHA`, `VERGEN_BUILD_TIMESTAMP`, `VERGEN_CARGO_OPT_LEVEL` — exposed via `env!()`.
- **D-07:** Add a `cargo:rustc-env=BUILD_RUSTFLAGS=...` line in `build.rs` capturing `RUSTFLAGS` from the environment so results.json can record the actual flags used (REPR-02). Rustflags are NOT in `vergen` out of the box.

### Harness API
- **D-08:** `pub trait Scenario { fn setup(&mut self) -> Result<()>; fn tick(&mut self) -> Box<dyn SinkValue>; fn teardown(&mut self); fn config_json(&self) -> serde_json::Value; }`. Returning `Box<dyn SinkValue>` lets the harness `black_box` then `drop` heap-touched values; the `SinkValue` trait is a sealed marker auto-implemented for any `T: 'static`.
- **D-09:** Harness drives the loop: `setup()` → warm-up loop until `Instant::now() >= warmup_end` (`black_box(scenario.tick())` only, no recording) → measurement loop until `measure_end` recording per-tick latency to an `hdrhistogram::Histogram::<u64>` with bounds `(1, 60_000_000_000, 3 sig figs)` and sampling `/proc/self/statm` every 1s → `teardown()` → emit results.
- **D-10:** Default warm-up = 5s, default measurement = 60s, both configurable via CLI flags. Harness panics if warm-up < 1s with message "warm-up must be ≥ 1s; allocator caches need to populate (see PITFALLS.md §1.5)".

### results.json schema
- **D-11:** **`schema_version: 1` is locked in Phase 1.** The full schema specified in `.planning/research/ARCHITECTURE.md §3` is the contract. Aggregator (Phase 4) will validate and reject mismatches. Future schema changes are additive (new optional fields) until v2.
- **D-12:** The schema lives in `crates/alloc-bench-core/src/output.rs` as serde-derived structs (`Run`, `Env`, `Build`, `ScenarioInfo`, `HarnessInfo`, `Metrics`, `LatencyNs`, `Rusage`, `RssGrowthSample`, `AllocatorStats`). One source of truth for serialization and aggregator consumption.
- **D-13:** `metrics.allocator_stats` is `serde_json::Value` (not a typed struct) so jemalloc / mimalloc / system can each emit their own shape. Each variant carries a `"kind"` discriminator field.

### Metrics sources
- **D-14:** Peak RSS via `getrusage(RUSAGE_SELF).ru_maxrss` × 1024 (Linux returns kB) using the `libc` crate. Captured once at end-of-run.
- **D-15:** RSS growth curve via parsing `/proc/self/statm` field 2 (resident pages) × `sysconf(_SC_PAGESIZE)` every 1s during measurement, stored as `Vec<RssGrowthSample { t_s: u64, rss_kb: u64 }>`.
- **D-16:** Page faults / context switches from `getrusage` fields `ru_minflt`, `ru_majflt`, `ru_nvcsw`, `ru_nivcsw`. CPU time from `ru_utime`, `ru_stime`.
- **D-17:** Allocator-internal stats: jemalloc → call `tikv_jemalloc_ctl::epoch::advance()` then read `stats::{allocated,resident,retained,active}::read()`; mimalloc → enable `extended` feature and call `mi_stats_get` if available, else fallback to capturing `mi_stats_print` to a buffer; system allocator → emit `{"kind": "system"}` with no further fields.

### First scenario (multithread allocation stress)
- **D-18:** `multithread` scenario CLI: `--threads N` (default = num_cpus), `--objects M` (default = 100_000), `--size-dist <uniform|bimodal|pareto>` (default = uniform), `--size-min` / `--size-max` (default = 16, 1024 for uniform), `--warmup <duration>` (default = 5s), `--duration <duration>` (default = 60s).
- **D-19:** Implementation: each thread gets a fresh `rand::rngs::SmallRng` seeded from a deterministic seed (CLI flag `--seed`, default = 0xDEADBEEF) so runs are reproducible. Per-thread loop allocates `Box<[u8]>` of size drawn from the distribution, writes a non-zero byte to position `len/2` (defeats memory-only-reservation optimizations per PITFALLS.md §1.2), then `black_box`es the box and drops it. Each thread does N allocations per `tick()`, where `N` is small enough (~1000) that ticks are short and many percentile samples accumulate.
- **D-20:** Threads coordinate via `crossbeam::scope` for join-on-drop semantics; results aggregated into a single histogram via `hdrhistogram::Histogram::add` per-thread merge.

### Mutual-exclusion enforcement
- **D-21:** `crates/alloc-bench-cli/src/allocator.rs` is the single source-of-truth file: `compile_error!` for both-features, `#[global_allocator]` static with `cfg`-guards, `pub fn name()` for runtime read, `pub fn stats() -> serde_json::Value` for the metrics path.

### Linux-only Phase 1
- **D-22:** Phase 1 targets `x86_64-unknown-linux-gnu` only. macOS `bench-host` (ORCH-02) is deferred to Phase 3. Phase 1 binaries on macOS panic at startup with: "Phase 1 supports x86_64-unknown-linux-gnu only; macOS host baseline arrives in Phase 3 (see ROADMAP.md Phase 3 success criterion 4)."

### Logging / output
- **D-23:** `--version` flag (and first stdout line on any subcommand invocation) prints exactly:
  ```
  alloc-bench v{CARGO_PKG_VERSION} (allocator={ALLOCATOR_NAME}, rustc={VERGEN_RUSTC_SEMVER}, target={VERGEN_CARGO_TARGET_TRIPLE}, host={VERGEN_RUSTC_HOST_TRIPLE}, profile={PROFILE}, git={VERGEN_GIT_SHA[..8]}, built={VERGEN_BUILD_TIMESTAMP})
  ```
  Single line, parseable, satisfies WS-03.
- **D-24:** `--output <path>` emits the JSON to file (creates parent dir if needed). Stdout-by-default would corrupt the version-line preamble; instead, the version line goes to stderr always; stdout is reserved for results JSON when `--output -` (stdin/stdout idiom).

### Claude's Discretion
- Choice between `crossbeam::scope` and `std::thread::scope` — equivalent for our needs. Plan-phase chooses based on dependency footprint.
- Internal layout of `harness.rs` (single file vs split into `warmup.rs` / `measure.rs` / `metrics.rs`). Plan-phase decides.
- Whether to make `Scenario::tick` `async` or sync. **Sync** is recommended (web bench in Phase 2 wraps its async server in a sync `tick`); flagged for confirmation during plan-phase.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase context
- `.planning/PROJECT.md` — overall project context, locked decisions
- `.planning/REQUIREMENTS.md` — full v1 requirements list (REQ-IDs in scope: WS-01..05, HARN-01..08, SCEN-01, REPR-02)
- `.planning/ROADMAP.md` §"Phase 1: Foundation MVP Slice" — phase goal + success criteria

### Research outputs (MANDATORY reading for plan-phase)
- `.planning/research/SUMMARY.md` — synthesis of stack/features/architecture/pitfalls
- `.planning/research/STACK.md` §1 — global allocator selection feature pattern
- `.planning/research/STACK.md` §2 — `tikv-jemallocator` + `tikv-jemalloc-ctl` API
- `.planning/research/STACK.md` §3 — `mimalloc` crate API and stats
- `.planning/research/STACK.md` §4 — `hdrhistogram` API
- `.planning/research/STACK.md` §5 — peak RSS sources (`/proc/self/statm`, `getrusage`)
- `.planning/research/STACK.md` §6 — `vergen` / `build.rs` build metadata pattern
- `.planning/research/ARCHITECTURE.md` §1 — workspace layout
- `.planning/research/ARCHITECTURE.md` §2 — `#[global_allocator]` feature pattern
- `.planning/research/ARCHITECTURE.md` §3 — results.json schema (LOCKED)
- `.planning/research/ARCHITECTURE.md` §4 — custom harness architecture
- `.planning/research/PITFALLS.md` §1.1, §1.2 — DCE / black_box discipline
- `.planning/research/PITFALLS.md` §1.5 — warm-up duration ≥ 5s
- `.planning/research/PITFALLS.md` §2.5 — `#[global_allocator]` init order
- `.planning/research/PITFALLS.md` §5.1, §5.2, §5.3 — LTO / debug / codegen-units interactions

### External specifications
- https://crates.io/crates/tikv-jemallocator (0.6.x) — jemalloc Rust binding API
- https://crates.io/crates/mimalloc (0.1.43) — mimalloc Rust binding API
- https://crates.io/crates/hdrhistogram (7.x) — HDR histogram API
- https://crates.io/crates/vergen (9.x) — build metadata

### Out-of-scope (Phase 1)
- `org.opencontainers.image.*` — Docker matrix is Phase 3
- Plotly.js / aggregator HTML — Phase 4
- GitHub Actions matrix — Phase 5

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- None (greenfield repo). Repo currently contains: `README.md` (stub), `docs/PLAN.md`, `.planning/` (project state), `.git`. No `Cargo.toml`, no `src/`, no `crates/`. Plan-phase creates the entire workspace from scratch.

### Established Patterns
- None (greenfield).

### Integration Points
- N/A — Phase 1 is the foundation. Phase 2 scenarios will integrate via the `Scenario` trait contract in `alloc-bench-core`.

### Repo conventions to honor
- Conventional-commit messages already in use (`docs:`, `chore:`, `feat:`); plan-phase commits should follow.
- Repo has `prek` pre-commit hooks (commits 5943832, 3fc1619). Plan-phase should run `cargo fmt` + `cargo clippy --all-targets` before commit.
- Repo `.planning/` is committed (config: `commit_docs = true`).

</code_context>

<specifics>
## Specific Ideas

- The build-metadata one-line preamble format (D-23) is locked to make output greppable in CI logs.
- `0xDEADBEEF` is the default `--seed` (D-19) — easter-egg keeps determinism explicit.
- Histogram bounds `(1ns, 60_000_000_000ns, 3 sig figs)` (D-09) cover everything from sub-µs to 60s with HDR's standard 3-significant-figure precision; keeps histograms small enough for JSON serialization.
- Mutual-exclusion is **compile-time** because Rust's idiom is to fail fast at build time when feature combinations are illegal (D-21). The runtime panic exists only to literally satisfy the success-criterion wording — code reviewers should not be surprised by either layer.

</specifics>

<deferred>
## Deferred Ideas

- **Other allocator combos (musl-mallocng, glibc-mimalloc, etc.)** → Phase 3 Docker matrix
- **Other scenarios (web, channels, cpu-bound, mem-bound, contention, fragmentation, realloc-storm, run-all)** → Phase 2
- **Aggregator** → Phase 4 (Phase 1 ships only a placeholder `main.rs` for `alloc-bench-aggregator`)
- **macOS host baseline** → Phase 3 (ORCH-02)
- **Schema version 2** → only when an actual additive change is required; not Phase 1
- **`snmalloc` / `tcmalloc` / `rpmalloc`** → v2 (REQUIREMENTS.md v2-01..03)
- **NUMA pinning** → Phase 3 Justfile recipe (PITFALLS.md §1.3)
- **Multiple runs per cell with median + range** → Phase 5 CI (PITFALLS.md §4.3, REPR-03)
- **`mi_stats_get`-based mimalloc structured stats** → revisit in Phase 2 once we hit the first mimalloc-bench cell; if the extended-feature API is too painful in 0.1.43, fall back to capturing `mi_stats_print` text.

</deferred>

---

*Phase: 1-Foundation MVP Slice*
*Context gathered: 2026-05-17*

# Phase 2: Scenario Fan-Out - Context

**Gathered:** 2026-05-18
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous)

<domain>
## Phase Boundary

Add 9 scenarios on top of Phase 1's `Scenario` trait contract, plus a `run-all` command. Every scenario must:
- Implement `alloc_bench_core::harness::Scenario`
- Be invocable via `alloc-bench-cli <name> [scenario-specific flags]`
- Emit a v1 `results.json` matching the Phase 1 schema
- Survive `cargo build --release --emit=llvm-ir | grep alloc` (no DCE)

This phase is **purely additive** to the harness layer. The harness, metrics pipeline, and JSON schema are LOCKED from Phase 1.

</domain>

<decisions>
## Implementation Decisions

### Web scenario (SCEN-02)
- **Payload shape:** Nested JSON ~1KB req / ~1KB res — matches the CLAUDE.md stack doc directive ("representative of typical microservice traffic; generates plenty of small/medium allocations per request").
- **Architecture:** In-process axum server + tokio runtime + reqwest (or hyper) client load generator. Both share the global allocator.
- **CLI flags:** `--server-workers N --client-workers M --duration 60s` (per ROADMAP success criterion 1).
- **Throughput unit:** `req/s` rather than `ticks/s`, since each request *is* a unit of work; this should be encoded as a scenario-specific override in the Run record.

### Channel scenarios (SCEN-03/04/05)
- **Backend:** `crossbeam-channel::bounded` (CLAUDE.md mandates crossbeam).
- **Capacity:** **Configurable via `--capacity` flag**, default 1024. Lets the user explore back-pressure regimes without recompiling.
- **Topology:**
  - SPMC: 1 sender, N cloned receivers race for messages.
  - MPSC: N cloned senders, 1 receiver.
  - MPMC: N senders × M receivers, both cloned.

### CPU-bound scenario (SCEN-06)
- **Algorithm:** Parallel merge-sort with `Vec<u64>` allocation in the merge step. Allocations land in the critical path so the allocator's lock contention shows up.
- **CLI flags:** `--threads N --input-size MB`.

### Mem-bound scenario (SCEN-07)
- **Linked-list mode:** Uniform 64B nodes (simplest baseline; reveals slab/segregated-list behavior cleanly).
- **Strided-array mode:** `Vec<u64>` of `--size MB`, accessed with stride to defeat prefetchers.
- **CLI flags:** `--mode <linked-list|strided-array> --size MB`.

### Contention scenario (SCEN-08)
- **Workload:** High thread count (default 64), every thread allocates+frees same-size buffers in a tight loop.
- **CLI flags:** `--threads N` (default 64), optionally `--alloc-size BYTES` (default 64).

### Fragmentation-soak scenario (SCEN-09)
- **Workload:** Long-running mixed alloc/free with biased size distribution (90% short-lived 16-byte; 10% long-lived 4KB held for the full duration).
- **CLI flag:** `--duration <minutes>` (default 5min).
- **Histogram bound:** May exceed 60s tick latency at 5min — Phase 1 fix WR-04 already saturates samples.

### Realloc-storm scenario (SCEN-10)
- **Workload:** `Vec::push(0u8)` until length reaches `--target-size MB`, repeat in `tick()`.
- **CLI flag:** `--target-size MB` (default 64MB).
- **Allocations-per-tick:** Variable (depends on Vec growth strategy); track actual count.

### Run-all (SCEN-11)
- **Failure semantics:** **Continue on per-scenario failure**. Each scenario gets its own record in the combined JSON; failures are recorded with `status: "failed"` and an `error` field but the run-all completes.
- **CLI flag:** `--output results/run.json` (writes a single JSON array of Run records).
- **Default config per scenario:** small, fast (e.g., warmup=1s, duration=5s) so `run-all` finishes in a few minutes during dev.

### Schema extension (additive only)
- Each Run record gains an optional `scenario.unit` field: "ticks_per_s" (default), "req_per_s" (web), "iters_per_s" (channels). This way the aggregator (Phase 4) can label charts correctly without breaking schema_version=1.
- Add an optional `status: "success" | "failed"` field at the top level for run-all.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets (from Phase 1)
- `alloc_bench_core::harness::{Scenario, SinkValue, run, HarnessConfig}` — the trait contract every new scenario implements.
- `alloc_bench_core::output::{Run, Env, Build, ScenarioInfo, HarnessInfo, Metrics, ...}` — schema is locked at v1; additive fields only.
- `alloc_bench_core::scenarios::Multithread` — reference implementation. Pattern: `pub struct X { cfg: XConfig }`, `impl Scenario for X { ... }`, mod-level `Cargo.toml` deps in core lib.
- `alloc_bench_core::metrics::{rusage, statm, env}` — the metrics layer is shared; new scenarios just hand the harness their `tick()` impl.
- `alloc_bench_cli::run::run_multithread` — CLI dispatch pattern. Each new scenario gets a sibling `run_<name>` function that builds its config, calls `run()`, and emits the Run record.

### Established Patterns (from Phase 1)
- Scenarios MUST validate config at construction (`MultithreadConfig::validated`) — reject `size_min < 1`, `threads < 1` etc with `anyhow::Error`.
- Worker panics MUST propagate via `std::panic::resume_unwind` (not silently dropped).
- Mid-buffer write to defeat DCE: every allocated buffer needs a write somewhere LLVM can't optimize away.
- `parse_duration` helper in `cli/run.rs` accepts ms|s|m suffixes; reuse for any new duration flags.
- `chrono::Utc::now().to_rfc3339()` for run_id (Phase 4 may switch to filesystem-safe format per IN-04).

### Integration Points
- Each scenario adds:
  - `crates/alloc-bench-core/src/scenarios/<name>.rs` (the impl)
  - One line in `crates/alloc-bench-core/src/scenarios/mod.rs`
  - `crates/alloc-bench-cli/src/run.rs` `run_<name>(...)` function
  - One Clap subcommand variant in `cli/src/main.rs`
- `run-all` adds: a single `Cmd::RunAll` arm that iterates scenarios, calls each `run_<name>` with default args, and aggregates Run records into a Vec.

### New deps (workspace Cargo.toml)
- `axum = "0.8"` (web)
- `tokio = { version = "1", features = ["rt-multi-thread", "macros", "net", "time"] }` (web)
- `tower = "0.5"` (web, for axum middleware if needed)
- `hyper = "1"` (web — axum 0.8 uses it transitively but list explicitly for client mode)
- `reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }` (web client)
- `crossbeam-channel = "0.5"` (SPMC/MPSC/MPMC)
- All as workspace deps to keep core lib options-only and CLI as the consumer.

</code_context>

<specifics>
## Specific Ideas

- **Reuse `MultithreadConfig::validated` pattern verbatim** for every new scenario config struct. Every config gets a `validated() -> anyhow::Result<Self>` returning a self-validated value.
- **Web scenario nested JSON:** mirror the shape from rust-web-benchmarks community work — UserProfile with embedded Address + 3-5 Tags. Keeps allocations heterogeneous (struct + nested struct + Vec<String> + String).
- **Realloc-storm:** start `Vec::with_capacity(0)` so we test growth from scratch every tick.
- **Run-all default per-scenario duration:** 5s measure + 1s warmup (per-scenario), so 10 scenarios run in ~60s. This matches user expectation of a "smoke" run-all.
- **DCE verification:** add a `cargo build --release --emit=llvm-ir` step in `tasks/dce_check.sh` (or just-recipe) that greps for `call.*malloc` or `call.*alloc` substrings in the IR.

</specifics>

<deferred>
## Deferred Ideas

- **Async scenarios beyond web:** tokio-based MPMC/SPMC could be added in v2 but bounded crossbeam covers sync pattern best. Not in scope.
- **NUMA pinning:** per-thread CPU affinity for contention/fragmentation-soak is a Phase 3 concern (Docker matrix orchestrates this via `--cpuset-cpus`).
- **Custom alloc-rate metric:** beyond `ticks_per_s + allocations_per_tick`, a true `allocs_per_second` metric requires per-tick counting — defer to Phase 4 aggregator unless trivial.
- **Histogram bound widening for fragmentation-soak:** 5min HDR bound from Phase 1 WR-04 fix should suffice for `--duration 5min`. Revisit if longer soaks are needed.

</deferred>

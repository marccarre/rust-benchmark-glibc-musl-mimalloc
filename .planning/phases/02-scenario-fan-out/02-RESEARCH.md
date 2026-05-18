# Phase 2: Scenario Fan-Out — Research

**Researched:** 2026-05-18
**Domain:** Rust async (axum/tokio), concurrent channels (crossbeam), parallel CPU/memory workloads, Vec growth, DCE verification
**Confidence:** HIGH (everything builds on the Phase 1 contract; library APIs are verified live against crates.io and docs.rs)

## Summary

Phase 2 fans out the nine remaining scenarios on top of the **already-locked** Phase-1 `Scenario` trait and v1 `results.json` schema. The trait does **not** need to be extended for the web scenario — a tokio runtime is constructed once in `setup()` and reused across `tick()` invocations. Channel scenarios (SCEN-03/04/05) use `crossbeam-channel::bounded()` with the canonical clone-then-spawn pattern. CPU-bound (SCEN-06) uses a recursive merge-sort with `rayon::join` (allocation in the merge step lands in the critical path). Mem-bound (SCEN-07) has two `--mode` branches: `linked-list` allocates `Box<Node>` chains; `strided-array` allocates one big `Vec<u64>` and does prefetcher-defeating reads. Contention (SCEN-08) is a tight alloc/free loop; the buffer must be **dropped before next iteration** or it's not a contention test, just a memory growth test. Fragmentation-soak (SCEN-09) maintains a long-lived `Vec<Box<[u8]>>` *as scenario state* across ticks — this is supported because `tick()` takes `&mut self`. Realloc-storm (SCEN-10) does a fresh `Vec::with_capacity(0)` + push loop per tick. Run-all (SCEN-11) is a thin wrapper that calls each `run_<name>` sequentially and aggregates Run records.

**Primary recommendation:** Add **9 sibling scenario files** in `crates/alloc-bench-core/src/scenarios/` and **9 sibling `run_<name>` functions** in `crates/alloc-bench-cli/src/run.rs`. Do **not** modify `harness.rs` or `output.rs` except for the additive optional fields specified in CONTEXT.md (`unit`, `status`, `error`). Use **dynamic dispatch (`Box<dyn Scenario>`)** for `run-all` to avoid duplicating the dispatch arms.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Web scenario tick (SCEN-02) | `alloc-bench-core` (scenario) | `alloc-bench-cli` (CLI dispatch) | Scenario owns the in-process axum server + reqwest client; tokio runtime is held as scenario state |
| Channel scenarios (SCEN-03/04/05) | `alloc-bench-core` (scenario) | — | Pure synchronous workloads; crossbeam-channel + std::thread::scope keep the implementation self-contained |
| CPU-bound (SCEN-06) | `alloc-bench-core` (scenario) | — | Pure CPU/alloc workload using rayon::join recursion |
| Mem-bound (SCEN-07) | `alloc-bench-core` (scenario) | — | Pure memory access workload; linked-list/strided-array branches |
| Contention (SCEN-08) | `alloc-bench-core` (scenario) | — | std::thread::scope + tight alloc/drop loop |
| Fragmentation-soak (SCEN-09) | `alloc-bench-core` (scenario) | — | Mutable scenario state holds long-lived Vec across ticks |
| Realloc-storm (SCEN-10) | `alloc-bench-core` (scenario) | — | Single-thread Vec growth pattern |
| Run-all (SCEN-11) | `alloc-bench-cli` (CLI) | `alloc-bench-core` (Scenario trait via dyn) | Sequence of Scenario invocations; output aggregation is CLI's job |
| Allocator-internal stats sampling | `alloc-bench-cli` (allocator.rs) | — | Already locked from Phase 1 — same closure passed to `run()` |
| results.json emission | `alloc-bench-cli` (run.rs) | `alloc-bench-core` (output structs) | v1 schema is locked; new scenarios reuse the existing assembly |

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Web scenario (SCEN-02):**
- Payload shape: nested JSON ~1KB request / ~1KB response (UserProfile with embedded Address + 3-5 Tags)
- Architecture: in-process axum server + tokio runtime + reqwest (or hyper) client load generator. Both share the global allocator.
- CLI flags: `--server-workers N --client-workers M --duration 60s`
- Throughput unit: `req/s` (encoded as scenario-specific override `unit: "req_per_s"` in the Run record)

**Channel scenarios (SCEN-03/04/05):**
- Backend: `crossbeam-channel::bounded`
- Capacity: configurable via `--capacity` flag, default 1024
- Topology:
  - SPMC: 1 sender, N cloned receivers race for messages
  - MPSC: N cloned senders, 1 receiver
  - MPMC: N senders × M receivers, both cloned

**CPU-bound (SCEN-06):**
- Algorithm: parallel merge-sort with `Vec<u64>` allocation in the merge step
- Allocations land in the critical path
- CLI flags: `--threads N --input-size MB`

**Mem-bound (SCEN-07):**
- linked-list: uniform 64B nodes
- strided-array: `Vec<u64>` of `--size MB` accessed with prefetcher-defeating stride
- CLI flags: `--mode <linked-list|strided-array> --size MB`

**Contention (SCEN-08):**
- High thread count (default 64)
- Same-size buffers, alloc+free in a tight loop
- CLI flags: `--threads N` (default 64), optionally `--alloc-size BYTES` (default 64)

**Fragmentation-soak (SCEN-09):**
- Workload: 90% short-lived 16-byte / 10% long-lived 4KB held for the full duration
- CLI flag: `--duration <minutes>` (default 5min)
- Histogram bound: Phase 1 WR-04 already saturates samples (HIST_MAX_NS = 300s) — no further widening needed

**Realloc-storm (SCEN-10):**
- Workload: `Vec::push(0u8)` until length = `--target-size MB`, repeat per tick
- CLI flag: `--target-size MB` (default 64MB)
- Allocations-per-tick: variable; track actual count
- Start `Vec::with_capacity(0)` so growth from scratch is exercised every tick

**Run-all (SCEN-11):**
- Failure semantics: continue on per-scenario failure; failures recorded with `status: "failed"` + `error` field
- CLI flag: `--output results/run.json` (writes a single JSON array of Run records)
- Default per-scenario config: small/fast (warmup=1s, duration=5s) so run-all finishes in ~60s

**Schema extension (additive only):**
- Optional `scenario.unit` field: "ticks_per_s" (default), "req_per_s" (web), "iters_per_s" (channels)
- Optional `status: "success" | "failed"` field at top level for run-all entries
- Optional `error` field on failed entries

### Claude's Discretion

- Crate version pins (within the bounds CLAUDE.md sets: axum 0.8.x, tokio 1.x, reqwest 0.12+ acceptable, crossbeam-channel 0.5.x, rayon 1.x)
- Internal sub-structuring of each scenario module
- Whether merge-sort is implemented with `rayon::join` recursion or `std::thread::scope` (recommendation in §CPU-bound below)
- Run-all dispatch shape: Box<dyn Scenario> map vs hardcoded match arms (recommendation: dynamic dispatch via a small registry — see §Run-all below)
- DCE verification script location: `tasks/dce_check.sh` or just-recipe (recommendation: just-recipe so it's discoverable from `just --list`)

### Deferred Ideas (OUT OF SCOPE)

- Async scenarios beyond web (tokio-based MPMC/SPMC)
- Per-thread CPU affinity (Phase 3 concern)
- True per-second `allocs_per_second` metric (Phase 4 aggregator concern)
- Histogram bound widening for soaks longer than 5min
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SCEN-02 | `alloc-bench web --server-workers N --client-workers M --duration 60s` | §Web scenario specifics — in-process axum + tokio runtime in setup(); reqwest client; UserProfile JSON payload |
| SCEN-03 | `alloc-bench spmc --producers 1 --consumers N` | §Channel scenarios — bounded(cap), 1 sender, N cloned receivers |
| SCEN-04 | `alloc-bench mpsc --producers N --consumers 1` | §Channel scenarios — bounded(cap), N cloned senders, 1 receiver |
| SCEN-05 | `alloc-bench mpmc --producers N --consumers M` | §Channel scenarios — bounded(cap), both sides cloned |
| SCEN-06 | `alloc-bench cpu-bound --threads N --input-size MB` | §CPU-bound — recursive merge-sort with `rayon::join`; `Vec<u64>` alloc in merge step |
| SCEN-07 | `alloc-bench mem-bound --mode <linked-list\|strided-array> --size MB` | §Mem-bound — `Box<Node>` chain or `Vec<u64>` with stride |
| SCEN-08 | `alloc-bench contention --threads N` | §Contention — std::thread::scope, alloc + immediate drop in tight loop |
| SCEN-09 | `alloc-bench fragmentation-soak --duration <minutes>` | §Fragmentation-soak — `&mut self` allows long-lived buffers across ticks |
| SCEN-10 | `alloc-bench realloc-storm --target-size MB` | §Realloc-storm — `Vec::with_capacity(0)` + push to target size |
| SCEN-11 | `alloc-bench run-all --output results/run.json` | §Run-all — sequential Box<dyn Scenario> dispatch, JSON array output |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

| Constraint | Source | Phase 2 Implication |
|------------|--------|---------------------|
| All allocator-vs-allocator benchmarks run on Linux only | CLAUDE.md > Constraints | Web scenario uses `127.0.0.1:0` (any port) and assumes loopback works on macOS host too for dev smoke; the real comparison runs in Phase 3 Docker |
| Allocator selection is compile-time (Cargo features) | CLAUDE.md > Constraints | Phase 2 doesn't touch this — already locked in `crates/alloc-bench-cli/src/allocator.rs` |
| Performance flags: LTO=fat, codegen-units=1, opt-level=3 | CLAUDE.md > Constraints | Phase 2's DCE verification (success criterion 4) MUST run with `--release` |
| Web stack: axum + serde_json + tokio | CLAUDE.md > Tech Stack | Locked. Use axum 0.8, tokio 1.x, serde_json 1.x |
| Channels: crossbeam-channel for SPMC/MPSC/MPMC | CLAUDE.md > Tech Stack §8 | Locked. crossbeam-channel 0.5.x |
| reqwest 0.12 with rustls-tls (per CLAUDE.md table) | CLAUDE.md > Tech Stack §summary | **Discrepancy noted:** crates.io shows reqwest at 0.13.x (verified 2026-05-18). Recommend `reqwest = "0.12"` (semver-compatible 0.12.x) per CLAUDE.md table — but if the planner finds 0.12 unavailable in the toolchain, 0.13 is acceptable since the API surface for our use (POST JSON, await response) is unchanged. See §Crate Version Pins below. |
| All bench binaries print rustc version, target, allocator at startup | CLAUDE.md > Constraints | Already locked in Phase 1 — Phase 2 just adds new subcommand arms; the banner runs unchanged |
| GSD workflow enforcement: planning artifacts kept in sync | CLAUDE.md > GSD Workflow | RESEARCH.md → PLAN.md (planner step) → execute |

## Standard Stack

### Core (Phase 2 additions)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `axum` | `0.8` | HTTP server for SCEN-02 | CLAUDE.md mandate; canonical modern Rust web stack 2026; tokio-native |
| `tokio` | `1` | Async runtime for SCEN-02 | CLAUDE.md mandate; required for axum 0.8 |
| `tower` | `0.5` | Service trait for axum middleware (transitive) | Comes in via axum's tower feature; only needed explicitly if we add middleware |
| `hyper` | `1` | HTTP impl underneath axum (transitive) | Listed explicitly in CONTEXT.md but not strictly needed by us — axum re-exports what we need |
| `reqwest` | `0.12` | HTTP client for in-process load generator | CLAUDE.md mandate; rustls-tls feature (no openssl); `json` feature for serde integration |
| `crossbeam-channel` | `0.5` | SPMC/MPSC/MPMC channels | CLAUDE.md mandate §8; significantly faster than std::mpsc for MPMC |
| `rayon` | `1` | `rayon::join` for parallel merge-sort (SCEN-06) | Idiomatic divide-and-conquer parallelism; alternative would be hand-rolled `std::thread::scope` (more code, fewer abstractions) |

### Already in workspace (no changes)
| Library | Version | Purpose |
|---------|---------|---------|
| `serde` | `1` | Already used; SCEN-02 needs `Serialize` + `Deserialize` for UserProfile |
| `serde_json` | `1` | Already used; SCEN-02 needs `json!()` payload generation and `axum::Json<T>` |
| `rand` | `0.8` | Already used; payload size variability and channel-payload generation |
| `anyhow` | `1` | Error handling — same pattern as Phase 1 |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `axum 0.8` | `actix-web` or `warp` | CLAUDE.md locks axum; would lose tokio-native + tower ecosystem coherence |
| `crossbeam-channel` | `std::sync::mpsc` | std lacks MPMC; std is slower per-message [CITED: crossbeam-channel docs intro] |
| `rayon::join` (merge-sort) | `std::thread::scope` recursion | rayon's work-stealing pool is more efficient under recursion than re-spawning OS threads; rayon's ThreadPool is configurable (`num_threads(N)`) which lets us honour `--threads N` cleanly |
| `reqwest` (load gen) | Hand-rolled `hyper` client | reqwest is significantly less code; for a benchmark we want allocator stress on realistic call paths, not minimal-code stress |

**Installation (workspace Cargo.toml additions):**

```toml
# Add to [workspace.dependencies]:
axum             = { version = "0.8", default-features = true }
tokio            = { version = "1", features = ["rt-multi-thread", "macros", "net", "time", "sync"] }
tower            = { version = "0.5" }
reqwest          = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
crossbeam-channel = { version = "0.5" }
rayon            = { version = "1" }
```

Then in `crates/alloc-bench-core/Cargo.toml`:

```toml
[dependencies]
# ... existing
axum             = { workspace = true }
tokio            = { workspace = true }
tower            = { workspace = true }
reqwest          = { workspace = true }
crossbeam-channel = { workspace = true }
rayon            = { workspace = true }
```

**Version verification (live, against crates.io 2026-05-18):**

| Crate | Pinned | Latest on crates.io | Status |
|-------|--------|---------------------|--------|
| axum | `0.8` | 0.8.9 | [VERIFIED: cargo info axum] resolves cleanly to 0.8.x family |
| tokio | `1` | 1.52.3 | [VERIFIED: cargo info tokio] resolves to latest 1.x |
| reqwest | `0.12` | 0.13.3 (latest) | [VERIFIED: cargo info reqwest] **0.12.x is still on crates.io** but no longer the latest — CLAUDE.md mandates 0.12 so we pin it; 0.12.x is deprecated for security only when 0.13 lands fully. Acceptable. If 0.12 fails to resolve a transitive dep against tokio 1.52, fall back to 0.13. |
| crossbeam-channel | `0.5` | 0.5.15 | [VERIFIED: cargo info crossbeam-channel] |
| rayon | `1` | 1.12.0 | [VERIFIED: cargo info rayon] |
| tower | `0.5` | 0.5.3 | [VERIFIED: cargo search tower] |
| hyper | `1` | 1.9.0 | [VERIFIED: cargo search hyper] — transitive only, no need to pin |

## Package Legitimacy Audit

slopcheck was not available in this research environment. All recommended packages are listed in CLAUDE.md as the canonical stack and exist on crates.io with multi-year publish history at the verified versions above. Per the protocol, the planner should still gate net-new packages behind a brief verification step.

| Package | Registry | Age | Downloads (approx) | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-------------------|-------------|-----------|-------------|
| axum | crates.io | ~4 yrs | very high (top 100) | github.com/tokio-rs/axum | [N/A — slopcheck unavailable] | Approved (CLAUDE.md mandate, Tokio org) |
| tokio | crates.io | ~7 yrs | among the highest on crates.io | github.com/tokio-rs/tokio | [N/A] | Approved (CLAUDE.md mandate) |
| reqwest | crates.io | ~7 yrs | very high | github.com/seanmonstar/reqwest | [N/A] | Approved (CLAUDE.md mandate; well-known author seanmonstar) |
| crossbeam-channel | crates.io | ~7 yrs | very high | github.com/crossbeam-rs/crossbeam | [N/A] | Approved (CLAUDE.md mandate, crossbeam-rs org) |
| rayon | crates.io | ~9 yrs | very high | github.com/rayon-rs/rayon | [N/A] | Approved (rayon-rs org) |
| tower | crates.io | ~6 yrs | high | github.com/tower-rs/tower | [N/A] | Approved (Tokio ecosystem, axum transitive) |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

*All packages above are tagged `[VERIFIED: cargo info]` for registry existence + `[CITED: CLAUDE.md]` for legitimacy. The planner can install directly without a checkpoint:human-verify step.*

## Architecture Patterns

### System Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                     alloc-bench-cli (binary)                    │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  main.rs: Clap dispatch → Cmd::* arms                   │    │
│  │  - Cmd::Multithread (Phase 1, unchanged)                │    │
│  │  - Cmd::Web, Cmd::Spmc, Cmd::Mpsc, Cmd::Mpmc            │ ── new ──┐
│  │  - Cmd::CpuBound, Cmd::MemBound                         │          │
│  │  - Cmd::Contention, Cmd::FragmentationSoak              │          │
│  │  - Cmd::ReallocStorm                                    │          │
│  │  - Cmd::RunAll                                          │ ── new ──┤
│  └─────────────────┬───────────────────────────────────────┘          │
│                    ▼                                                  │
│  ┌─────────────────────────────────────────────────────────┐          │
│  │  run.rs: run_<scenario>(...) → builds {Run} record      │          │
│  │  - parse CLI flags into Config                          │          │
│  │  - validate Config                                      │          │
│  │  - construct Scenario                                   │          │
│  │  - call alloc_bench_core::run(&mut scenario, &cfg, …)   │          │
│  │  - assemble Env + Build + ScenarioInfo + outcome        │          │
│  │  - serialize to stdout or --output                      │          │
│  └─────────────────┬───────────────────────────────────────┘          │
└────────────────────┼──────────────────────────────────────────────────┘
                     │  passes &mut Scenario + HarnessConfig
                     ▼
┌─────────────────────────────────────────────────────────────────┐
│                 alloc-bench-core (library)                      │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  harness.rs (LOCKED — no changes)                       │    │
│  │   pub fn run<S: Scenario, F: Fn() -> Value>(...) -> …  │    │
│  │   Drives warm-up + measurement + tick latency + RSS     │    │
│  │   Returns HarnessOutcome { harness, metrics }           │    │
│  └─────────────────┬───────────────────────────────────────┘    │
│                    ▼                                            │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  scenarios/                                             │    │
│  │   ├─ multithread.rs (Phase 1, unchanged)                │    │
│  │   ├─ web.rs           [SCEN-02]   ── new ──             │    │
│  │   ├─ spmc.rs          [SCEN-03]   ── new ──             │    │
│  │   ├─ mpsc.rs          [SCEN-04]   ── new ──             │    │
│  │   ├─ mpmc.rs          [SCEN-05]   ── new ──             │    │
│  │   ├─ cpu_bound.rs     [SCEN-06]   ── new ──             │    │
│  │   ├─ mem_bound.rs     [SCEN-07]   ── new ──             │    │
│  │   ├─ contention.rs    [SCEN-08]   ── new ──             │    │
│  │   ├─ fragmentation.rs [SCEN-09]   ── new ──             │    │
│  │   ├─ realloc_storm.rs [SCEN-10]   ── new ──             │    │
│  │   └─ mod.rs (re-exports + new pub uses)                 │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  metrics/, output.rs (LOCKED — additive fields only)    │    │
│  │  output.rs: + ScenarioInfo.unit (Option<String>)        │    │
│  │             + Run.status / Run.error (Option fields)    │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
                     ▲
                     │ Run-all path (SCEN-11):
                     │ for each scenario: build it as Box<dyn Scenario>,
                     │ call run() with default cfg, push Run record into
                     │ Vec<Run>, serialize the whole Vec to file.
                     │
```

### Recommended Project Structure
```
crates/alloc-bench-core/src/
├── harness.rs                  # LOCKED
├── output.rs                   # LOCKED + 3 additive optional fields
├── lib.rs                      # add `pub use` for new scenarios
├── metrics/                    # LOCKED
└── scenarios/
    ├── mod.rs                  # extend pub mod + pub use
    ├── multithread.rs          # LOCKED (Phase 1)
    ├── web.rs                  # NEW: SCEN-02
    ├── channels.rs             # NEW: SCEN-03/04/05 — share helper code
    ├── cpu_bound.rs            # NEW: SCEN-06
    ├── mem_bound.rs            # NEW: SCEN-07
    ├── contention.rs           # NEW: SCEN-08
    ├── fragmentation.rs        # NEW: SCEN-09
    └── realloc_storm.rs        # NEW: SCEN-10

crates/alloc-bench-cli/src/
├── main.rs                     # add 9 Clap subcommand variants + run-all
├── allocator.rs                # LOCKED
├── build_info.rs               # LOCKED
└── run.rs                      # add 9 run_<name> + run_all dispatcher
```

**Recommendation:** put SPMC/MPSC/MPMC into a single `channels.rs` since they share 80% of their code (only the producer/consumer cardinality differs). Three sub-structs `Spmc`, `Mpsc`, `Mpmc` that all use one shared payload struct + one shared driver helper.

### Component Responsibilities

| File | Responsibility | Notes |
|------|---------------|-------|
| `scenarios/web.rs` | Implements `Scenario` for the in-process axum + reqwest workload. Holds tokio Runtime in `Web` struct so it survives across ticks | One tick = one batch of `client_workers` parallel HTTP requests |
| `scenarios/channels.rs` | Implements `Scenario` for SPMC/MPSC/MPMC. Each tick = `objects_per_tick` messages flow through the channel. Workers spawned via `std::thread::scope` per tick | Producer rate is **self-paced** (not rate-limited) per CONTEXT.md decision to maximise allocator stress |
| `scenarios/cpu_bound.rs` | Implements `Scenario` for parallel merge-sort. One tick = one full sort of an `--input-size MB` `Vec<u64>` | Use `rayon::ThreadPoolBuilder` to honour `--threads N`; avoid using global rayon pool so it doesn't bleed into other scenarios in run-all |
| `scenarios/mem_bound.rs` | Two-mode scenario: `linked-list` allocates `Box<Node>` chains; `strided-array` allocates one `Vec<u64>` and reads with stride | Mode selected via enum + FromStr |
| `scenarios/contention.rs` | High-thread-count tight alloc/free loop. Each tick = `iters_per_tick` allocate-then-immediately-drop cycles per worker | **Critical:** drop the box at end of inner loop iter; do NOT accumulate |
| `scenarios/fragmentation.rs` | 90/10 mixed alloc workload. Long-lived `Vec<Box<[u8]>>` lives in `&mut self.long_lived` across ticks | Per-tick: drain short-lived, push 10% to long_lived |
| `scenarios/realloc_storm.rs` | Vec growth from capacity 0 to `--target-size MB`. One tick = one full grow cycle | Track actual realloc count per tick |
| `cli/run.rs::run_<name>` | Build Config from args, validate, construct Scenario, drive harness, assemble Run | Same pattern as Phase 1 `run_multithread` — copy and adapt |
| `cli/run.rs::run_all` | Sequence each scenario with default config, aggregate to `Vec<Run>`, serialize | Use Box<dyn Scenario> via a small registry |

### Pattern 1: Scenario impl boilerplate (same as Phase 1)
**What:** Every new scenario follows the exact pattern established by `Multithread` in Phase 1.
**When to use:** All 9 new scenarios.
**Example (skeleton):**
```rust
// Source: crates/alloc-bench-core/src/scenarios/multithread.rs (verbatim Phase 1 pattern)

#[derive(Debug, Clone, Serialize)]
pub struct WebConfig {
    pub server_workers: usize,
    pub client_workers: usize,
    // ...
}

impl WebConfig {
    pub fn validated(self) -> anyhow::Result<Self> {
        anyhow::ensure!(self.server_workers >= 1, "server_workers must be >= 1");
        anyhow::ensure!(self.client_workers >= 1, "client_workers must be >= 1");
        Ok(self)
    }
}

pub struct Web {
    cfg: WebConfig,
    // scenario-specific state survives across ticks
    runtime: Option<tokio::runtime::Runtime>,
    server_addr: Option<std::net::SocketAddr>,
    client: Option<reqwest::Client>,
}

impl Web {
    pub fn new(cfg: WebConfig) -> Self { Self { cfg, runtime: None, server_addr: None, client: None } }
}

impl Scenario for Web {
    fn name(&self) -> &'static str { "web" }
    fn config_json(&self) -> serde_json::Value { serde_json::to_value(&self.cfg).unwrap() }
    fn allocations_per_tick(&self) -> u64 { self.cfg.client_workers as u64 } // approx
    fn setup(&mut self) -> anyhow::Result<()> {
        // build tokio runtime, bind axum server, spawn it on the runtime,
        // build reqwest client. Stash all of them in self.
        // ...
        Ok(())
    }
    fn tick(&mut self) -> Box<dyn SinkValue> { /* see §Web below */ }
    fn teardown(&mut self) { /* runtime drops naturally */ }
}
```

### Pattern 2: Crossbeam channel topology
**What:** Use `bounded(cap)` and `clone()` the appropriate side for the topology.
**Source:** [VERIFIED: docs.rs/crossbeam-channel/0.5.15]

```rust
// MPMC: both sides cloned (canonical example from docs)
let (s, r) = crossbeam_channel::bounded::<Payload>(cfg.capacity);
let s2 = s.clone();
let r2 = r.clone();
// Spawn N producers, each holding a clone of the sender:
//   for _ in 0..cfg.producers { thread::spawn({ let s = s.clone(); move || { ... } }); }
// Spawn M consumers, each holding a clone of the receiver:
//   for _ in 0..cfg.consumers { thread::spawn({ let r = r.clone(); move || { ... } }); }
// SPMC: one sender, multiple cloned receivers
// MPSC: multiple cloned senders, one receiver

// Drop the original sender to signal completion when the channel is "done"
// per tick. Receivers terminate when recv() returns Err(RecvError).
```

### Pattern 3: tokio Runtime as scenario state
**What:** Build a tokio Runtime once in `setup()`, hold it in `self`, reuse it across `tick()` invocations.
**Why:** Constructing a Runtime per tick adds ~ms overhead and changes what we're measuring. The allocator should be stressed by the request handling, not by runtime startup.
**Source:** [VERIFIED: docs.rs/tokio/latest/tokio/runtime/struct.Runtime.html]

```rust
// In setup():
let runtime = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(self.cfg.server_workers)
    .enable_all()
    .build()?;

// Bind the listener inside the runtime:
let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
let (listener, addr) = runtime.block_on(async {
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let actual = listener.local_addr().unwrap();
    (listener, actual)
});

// Build the axum app:
let app = axum::Router::new().route("/echo", axum::routing::post(echo_handler));

// Spawn the server task on the runtime so it runs in the background:
runtime.spawn(async move { axum::serve(listener, app).await.unwrap(); });

// Build the reqwest client (this is sync construction; client uses the runtime
// implicitly via tokio::runtime::Handle::current() inside reqwest's async fns):
let client = reqwest::Client::builder()
    .pool_max_idle_per_host(self.cfg.client_workers)
    .build()?;

self.runtime = Some(runtime);
self.server_addr = Some(addr);
self.client = Some(client);

// In tick(): use self.runtime.as_ref().unwrap().block_on(async { ... }) to drive
// `client_workers` parallel POSTs to http://{addr}/echo and collect responses.
```

### Pattern 4: Mutable scenario state across ticks
**What:** `tick(&mut self)` allows scenarios to hold long-lived data across ticks.
**Why:** Required for fragmentation-soak (long-lived buffers must survive between ticks) and web scenario (tokio runtime + server must survive).
**Source:** Phase 1 trait definition (verified live).

```rust
pub trait Scenario {
    fn tick(&mut self) -> Box<dyn SinkValue>;
    // ^^ &mut self — mutation is supported.
    //    Phase 1 Multithread doesn't use it, but it's available.
}
```

### Anti-Patterns to Avoid

- **Constructing a tokio Runtime per tick:** ~ms of overhead, dominates the latency histogram. Build once in `setup()`.
- **Forgetting to clone Sender/Receiver before spawning:** Crossbeam channels can't be moved into multiple threads — the *clone* of the handle goes to each worker. Forgetting this gives a borrow-checker error and is the most common SPMC mistake.
- **Accumulating buffers in contention scenario:** if you `Vec::push` allocated buffers, you defeat the purpose — the allocator never sees the free path. The buffer **must** drop before next iteration.
- **Using global rayon pool in cpu-bound:** if `--threads 4` is requested but the global pool was already initialised with 8 (e.g., during run-all), the request silently fails. Build a scoped `ThreadPool` per scenario.
- **Sharing `Multithread`'s `seed.wrapping_add(t as u64)` with channels:** RNG seeding is per-scenario; don't try to share. Each scenario owns its own seed.
- **Allocating short-lived in fragmentation-soak with long-lived holding a reference:** if the long-lived array points into the short-lived alloc, the lifetime is wrong. Long-lived must be `Box<[u8]>` (owned), not `&[u8]`.
- **Ignoring `axum::serve(listener, app).await` is `!` (never returns):** Spawn it via `runtime.spawn(...)` so the spawn returns immediately and the runtime drives the future on its threadpool.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| HTTP server (SCEN-02) | Hyper-only handler | `axum 0.8` | CLAUDE.md mandate; ergonomic Router and tower middleware story |
| HTTP client (SCEN-02) | Hand-rolled hyper Client | `reqwest 0.12` | reqwest handles connection pooling, redirects, JSON ser/de cleanly; far less code |
| MPMC channel (SCEN-05) | `Arc<Mutex<VecDeque<T>>>` | `crossbeam-channel::bounded()` | Lock-free; significantly faster; canonical for Rust [CITED: CLAUDE.md §8] |
| Worker pool (SCEN-06) | Hand-rolled `std::thread::scope` recursion | `rayon::join` + `rayon::ThreadPool` | Work-stealing; tunable thread count; designed for divide-and-conquer |
| JSON ser/de payload | Hand-built JSON strings | `serde_json` + `axum::Json<UserProfile>` | Already a dep; allocates representative heap (Strings, nested Vecs) which is what we want |
| Vec growth model | Custom strategy | `Vec::with_capacity(0)` + `push` (uses Rust's amortised doubling) | The whole point of SCEN-10 is to test the *standard* growth pattern |
| Linked list (SCEN-07) | `LinkedList<T>` | Hand-rolled `Box<Node>` chain | std `LinkedList` is itself a benchmark target — but we want **uniform 64B allocations**, which means a custom Node struct anyway |
| Tokio runtime construction | `Runtime::new()` (defaults) | `Builder::new_multi_thread().worker_threads(N).enable_all().build()` | Lets us honour `--server-workers N` |

**Key insight:** The benchmark exists to *measure* allocator behaviour under realistic workloads. Hand-rolled minimal versions defeat the purpose — they don't trigger the same allocation patterns as production stacks do. Use the canonical libraries; that *is* what we're stressing.

## Common Pitfalls

### Pitfall 1: Web scenario — runtime constructed per tick
**What goes wrong:** Latency histogram dominated by tokio startup costs, not request handling.
**Why it happens:** Naive `tokio::runtime::Runtime::new()` inside `tick()` rebuilds the multi-threaded scheduler each call.
**How to avoid:** Build the runtime in `setup()`, store as `Option<Runtime>` in scenario struct, reuse via `runtime.block_on(async { ... })` in `tick()`.
**Warning signs:** p50 latency unexpectedly stable (suggests measurement is dominated by setup cost rather than allocator behaviour).

### Pitfall 2: Channel scenarios — producer rate-limiting
**What goes wrong:** If producer rate-limits via `thread::sleep`, the channel is rarely full and back-pressure isn't tested.
**Why it happens:** Engineers conditioned to "fair" benchmarks add sleeps to "make it realistic."
**How to avoid:** **Self-paced** producers per CONTEXT.md decision — `for _ in 0..N { s.send(payload).unwrap(); }` with no sleep. The allocator sees max stress; the channel hits its cap; the consumer side reveals scaling.
**Warning signs:** All producers report identical throughput (= no contention); peak RSS doesn't grow with `--capacity` flag.

### Pitfall 3: SPMC — race semantics misunderstood
**What goes wrong:** People assume "single producer multi consumer" means "broadcast" (all consumers get every message). It doesn't — it means **N consumers race for messages from one queue**.
**Why it happens:** Pub/sub mental model leakage.
**How to avoid:** Use `bounded::<T>(cap)`; `s.send(msg)` puts ONE copy into the channel; the FIRST receiver to call `recv()` gets it. This is the workload we want for allocator stress (consumer cloning forces atomics on the receiver side).
**Warning signs:** Every consumer reports N/M messages received (good — racing). If every consumer reports N messages received (= broadcast), wrong semantics.

### Pitfall 4: CPU-bound — merge-sort buffer allocated outside the merge step
**What goes wrong:** If you allocate a `Vec<u64>` of size n once at the top, all merge steps reuse it — there's no allocation in the critical path.
**Why it happens:** Reasonable-looking optimisation; but it inverts the benchmark.
**How to avoid:** Each merge step `let temp: Vec<u64> = Vec::with_capacity(n)` — fresh alloc per merge. CONTEXT.md confirms: "Allocations land in the critical path so the allocator's lock contention shows up."
**Warning signs:** Allocator stats show `allocated_bytes` low and not growing with input size (= allocation isn't happening per merge).

### Pitfall 5: Mem-bound linked-list — node size drifts off 64B
**What goes wrong:** A `Box<Node>` where `struct Node { next: Option<Box<Node>>, payload: u64 }` is 16 bytes on 64-bit (8 ptr + 8 u64), not 64.
**Why it happens:** Rust struct layout is non-obvious.
**How to avoid:** Pad explicitly. `struct Node { next: Option<Box<Node>>, payload: [u8; 56] }` → 8 (ptr) + 56 (payload) = 64 bytes. **Verify with `std::mem::size_of::<Node>() == 64` in a unit test.**
**Warning signs:** RSS growth doesn't match `node_count * 64`; allocator's slab class for 64B doesn't fill up.

### Pitfall 6: Strided-array — stride pattern matches prefetcher
**What goes wrong:** A stride of 16 (= 2 × cache line) actually plays into the L1 streaming prefetcher, not against it.
**Why it happens:** Modern CPUs detect linear strides up to ~16 cache lines.
**How to avoid:** Use a **deterministic but non-linear** access pattern. E.g., `let stride: usize = 4099` (a prime > L2 cache line count) — this defeats both the L1 streaming and L2 stride prefetchers.
**Warning signs:** Throughput is suspiciously high (prefetcher is helping); RSS resident bytes don't match the working set size you expected.

### Pitfall 7: Contention — buffer accumulation
**What goes wrong:** Each thread pushes its allocated buffer to a `Vec<Box<[u8]>>`, which silently ruins the test (now we're measuring sustained alloc rate, not alloc/free contention).
**Why it happens:** Mistakenly treating contention as a high-volume Multithread.
**How to avoid:** Strict tight loop:
```rust
for _ in 0..iters_per_tick {
    let b = vec![0u8; cfg.alloc_size].into_boxed_slice();
    std::hint::black_box(&b);          // defeat DCE on the alloc
    drop(b);                            // explicit drop = explicit free
}
```
**Warning signs:** RSS grows linearly during the run (free path isn't exercised).

### Pitfall 8: Fragmentation-soak — long-lived buffer leaks
**What goes wrong:** If you push to `self.long_lived` every tick without bounding, after 5min you've allocated 30s/0.001s × 10% × 4KB ≈ 12MB-ish, but if the duration is wrong you OOM the host.
**Why it happens:** Forgetting to limit `self.long_lived.len()`.
**How to avoid:** Cap `self.long_lived.len()` at e.g. 10_000 entries. When full, randomly evict one before pushing.
**Warning signs:** `peak_rss_kb` exceeds expected by orders of magnitude.

### Pitfall 9: Run-all — one scenario panic kills the rest
**What goes wrong:** If `Cmd::Web` panics (e.g., port-bind failure), the run-all aborts.
**Why it happens:** Panics propagate up the stack by default.
**How to avoid:** Wrap each `run_<name>(...)` call in `std::panic::catch_unwind` (catches panics and returns Result). Convert caught panic into a Run record with `status: "failed"` + `error: <message>`.
**Warning signs:** `run-all` exits non-zero with one scenario's stack trace and 8 scenarios' results missing.

### Pitfall 10: DCE — release build elides the entire scenario
**What goes wrong:** LLVM in `--release` proves the scenario has no observable effect (no print, no return value used) and elides everything.
**Why it happens:** Phase 1 already mitigates this with `std::hint::black_box(scenario.tick())` in the harness, BUT individual scenarios can still be elided if their *internal* allocations don't get fed into a black_box too.
**How to avoid:** Every allocated buffer must have a write to it (Phase 1 pattern: `b[size/2] = 0xAB`). The buffer or a derived value must be passed to `black_box(...)` before being dropped.
**Warning signs:** Allocator stats show `allocated == 0` despite the scenario claiming to do work; LLVM IR (see §DCE Verification below) has no `call.*alloc` calls inside the scenario function.

## Web Scenario Specifics (SCEN-02)

### Architecture

In-process axum server + tokio runtime + reqwest client load generator, all sharing the global allocator (because they're all in the same process — no special wiring needed).

**Topology per tick:**
```
[reqwest Client] ── HTTP POST /echo ──▶ [axum Server]
       ▲                                       │
       │                                       ▼
       └── HTTP 200 + UserProfile JSON ◀── echo_handler(Json<UserProfile>) -> Json<UserProfile>
```

The server simply echoes the request (with maybe a small transformation to defeat DCE). One tick = `client_workers` parallel POSTs.

### tokio runtime instantiation (correct approach)

[VERIFIED: docs.rs/tokio/latest/tokio/runtime/struct.Runtime.html]

```rust
// In Web::setup():
let runtime = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(self.cfg.server_workers)
    .enable_all()    // enables I/O + time drivers (required for net + reqwest)
    .build()?;
```

Why `Builder` not `Runtime::new()`: `Runtime::new()` defaults to `available_parallelism()` workers. We want explicit control honouring `--server-workers N`.

### Server bind + spawn

```rust
// Bind on the runtime (TcpListener::bind is async):
let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap(); // :0 = OS picks port
let (listener, actual_addr) = runtime.block_on(async {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let actual = listener.local_addr()?;
    Ok::<_, std::io::Error>((listener, actual))
})?;

// Build the router:
let app = axum::Router::new()
    .route("/echo", axum::routing::post(echo_handler));

// Spawn the server on the runtime — fire-and-forget:
runtime.spawn(async move {
    axum::serve(listener, app).await.unwrap_or_else(|e| {
        eprintln!("axum::serve exited: {e}");
    });
});

self.runtime = Some(runtime);
self.server_addr = Some(actual_addr);
```

`axum::serve(listener, app).await` returns `!` (never) when serving normally; `.unwrap_or_else` handles abrupt shutdown without panicking the runtime.

### reqwest client construction

```rust
// Client: built on the runtime (uses the surrounding tokio context)
let client = self.runtime.as_ref().unwrap().block_on(async {
    reqwest::Client::builder()
        .pool_max_idle_per_host(self.cfg.client_workers)
        .timeout(std::time::Duration::from_secs(10))
        .build()
})?;
self.client = Some(client);
```

### tick() implementation

```rust
fn tick(&mut self) -> Box<dyn SinkValue> {
    let runtime = self.runtime.as_ref().unwrap();
    let client = self.client.clone().unwrap();   // reqwest::Client is internally Arc'd
    let url = format!("http://{}/echo", self.server_addr.unwrap());

    // Build a fresh payload (this is part of the alloc work we want to measure):
    let payload = make_user_profile(&mut SmallRng::seed_from_u64(self.cfg.seed));

    // Drive `client_workers` parallel requests:
    let responses: Vec<UserProfile> = runtime.block_on(async {
        let mut handles = Vec::with_capacity(self.cfg.client_workers);
        for _ in 0..self.cfg.client_workers {
            let client = client.clone();
            let url = url.clone();
            let payload = payload.clone();
            handles.push(tokio::spawn(async move {
                client.post(&url).json(&payload).send().await
                    .unwrap().json::<UserProfile>().await.unwrap()
            }));
        }
        let mut out = Vec::with_capacity(handles.len());
        for h in handles { out.push(h.await.unwrap()); }
        out
    });

    Box::new(std::hint::black_box(responses))
}
```

### Payload struct (UserProfile-like, ~1KB)

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
struct UserProfile {
    id: u64,
    username: String,         // ~12 bytes
    email: String,            // ~32 bytes
    full_name: String,        // ~40 bytes
    address: Address,
    tags: Vec<String>,        // 5 entries × ~12 bytes
    metadata: serde_json::Map<String, serde_json::Value>, // ~200 bytes of nested keys
    created_at: String,       // RFC3339 timestamp ~24 bytes
    last_login: String,
    notes: String,            // ~256 bytes free-form
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Address {
    street: String,
    city: String,
    state: String,
    zip: String,
    country: String,
}

// Total serialized: ~1KB. Heterogeneous heap shape: Strings, Vec<String>, nested struct, Map.
```

The handler:
```rust
async fn echo_handler(axum::Json(p): axum::Json<UserProfile>) -> axum::Json<UserProfile> {
    // Tiny mutation defeats any "echo == identity" DCE
    let mut p = p;
    p.id = p.id.wrapping_add(1);
    axum::Json(p)
}
```

### Throughput unit override

In `run::run_web(...)`, after the harness call:

```rust
let scenario_info = ScenarioInfo {
    name: "web".to_string(),
    config: scenario.config_json(),
    unit: Some("req_per_s".to_string()),  // <-- additive Phase 2 field
};

// Convert ticks_per_s → req_per_s by multiplying by client_workers:
let metrics = Metrics {
    ticks_per_s: outcome.metrics.ticks_per_s,            // raw tick rate (unchanged)
    allocations_per_tick: outcome.metrics.allocations_per_tick,
    // ...
};
// Aggregator (Phase 4) reads `unit` and labels charts as "req/s" derived from
// ticks_per_s * client_workers.
```

## Channel Scenarios (SCEN-03/04/05)

[VERIFIED: docs.rs/crossbeam-channel/0.5.15/]

### Shared payload struct
```rust
#[derive(Debug, Clone)]
pub struct ChannelPayload {
    pub seq: u64,
    pub data: Box<[u8]>,    // ~256B–4KB depending on --payload-dist
}
```

The `Box<[u8]>` is the allocation under test. Producers `vec![0u8; size].into_boxed_slice()`, send; consumers receive and immediately drop.

### SPMC: 1 sender, N cloned receivers

```rust
// Source: docs.rs/crossbeam-channel/0.5.15/crossbeam_channel/index.html
//         (verified pattern; cloned receivers race for messages)
let (s, r) = crossbeam_channel::bounded::<ChannelPayload>(cfg.capacity);

std::thread::scope(|scope| {
    // Spawn N consumers, each holding a cloned receiver:
    let mut consumer_handles = Vec::with_capacity(cfg.consumers);
    for _ in 0..cfg.consumers {
        let r = r.clone();
        consumer_handles.push(scope.spawn(move || {
            let mut received = 0u64;
            while let Ok(msg) = r.recv() {
                std::hint::black_box(&msg);   // defeat DCE
                received += 1;
                drop(msg);
            }
            received
        }));
    }

    // Drop the original receiver so the consumers see channel close when
    // we drop the sender:
    drop(r);

    // 1 producer:
    for seq in 0..cfg.objects_per_tick {
        let size = sample_payload_size(&mut rng, cfg.payload_dist);
        let data = vec![0u8; size].into_boxed_slice();
        s.send(ChannelPayload { seq, data }).unwrap();
    }
    drop(s);   // Closes the channel, consumers terminate.

    let mut total = 0u64;
    for h in consumer_handles { total += h.join().unwrap(); }
    assert_eq!(total, cfg.objects_per_tick);
});
```

### MPSC: N cloned senders, 1 receiver

```rust
let (s, r) = crossbeam_channel::bounded::<ChannelPayload>(cfg.capacity);

std::thread::scope(|scope| {
    // Spawn N producers, each holding a cloned sender:
    let mut producer_handles = Vec::with_capacity(cfg.producers);
    for p in 0..cfg.producers {
        let s = s.clone();
        let cfg = cfg.clone();
        producer_handles.push(scope.spawn(move || {
            let mut rng = SmallRng::seed_from_u64(cfg.seed.wrapping_add(p as u64));
            for seq in 0..cfg.objects_per_tick / cfg.producers as u64 {
                let size = sample_payload_size(&mut rng, cfg.payload_dist);
                let data = vec![0u8; size].into_boxed_slice();
                s.send(ChannelPayload { seq, data }).unwrap();
            }
        }));
    }
    // Drop the original sender so once all producers finish, the channel closes:
    drop(s);

    // 1 consumer (this thread):
    let mut received = 0u64;
    while let Ok(msg) = r.recv() {
        std::hint::black_box(&msg);
        received += 1;
        drop(msg);
    }

    for h in producer_handles { h.join().unwrap(); }
});
```

### MPMC: N senders × M receivers, both cloned

```rust
let (s, r) = crossbeam_channel::bounded::<ChannelPayload>(cfg.capacity);

std::thread::scope(|scope| {
    // Spawn N producers:
    for p in 0..cfg.producers {
        let s = s.clone();
        let cfg = cfg.clone();
        scope.spawn(move || {
            let mut rng = SmallRng::seed_from_u64(cfg.seed.wrapping_add(p as u64));
            for seq in 0..cfg.objects_per_tick / cfg.producers as u64 {
                let size = sample_payload_size(&mut rng, cfg.payload_dist);
                let data = vec![0u8; size].into_boxed_slice();
                s.send(ChannelPayload { seq, data }).unwrap();
            }
        });
    }
    drop(s);   // Original sender dropped — channel closes after all producers finish.

    // Spawn M consumers:
    let mut consumer_handles = Vec::with_capacity(cfg.consumers);
    for _ in 0..cfg.consumers {
        let r = r.clone();
        consumer_handles.push(scope.spawn(move || {
            let mut received = 0u64;
            while let Ok(msg) = r.recv() {
                std::hint::black_box(&msg);
                received += 1;
                drop(msg);
            }
            received
        }));
    }
    drop(r);

    let mut total = 0u64;
    for h in consumer_handles { total += h.join().unwrap(); }
    // Total may be slightly less than objects_per_tick if division rounded;
    // ok for benchmark purposes.
});
```

### Producer rate: self-paced, NOT rate-limited

Per CONTEXT.md decision, producers run flat-out (`for _ in 0..N { s.send(...).unwrap(); }`). This:
- Maximises allocator stress
- Exercises back-pressure when capacity = 1024 (default)
- Reveals the channel + allocator interaction (allocator must keep up with channel throughput)

Adding `thread::sleep` would make the benchmark "fairer" but invalidate the comparison: we want allocators that don't break under flat-out load.

## CPU-bound (SCEN-06): Parallel merge-sort

### Where allocations land in the critical path

Standard merge-sort:
1. Split input into halves
2. Recursively sort each half (in parallel via `rayon::join`)
3. **Merge** the two halves into a new buffer of size n

Step 3 allocates a `Vec<u64>` of size n at every level of the recursion. For input of size N, total allocation across all levels = N · log₂(N) (roughly). All allocations happen in the critical path during merge.

### Implementation skeleton

```rust
use rayon::prelude::*;

pub struct CpuBound {
    cfg: CpuBoundConfig,
    pool: Option<rayon::ThreadPool>,
    input: Option<Vec<u64>>,    // built once in setup; cloned per tick
}

impl Scenario for CpuBound {
    fn setup(&mut self) -> anyhow::Result<()> {
        // Honour --threads N by building a scoped pool:
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.cfg.threads)
            .build()?;

        // Build a deterministic shuffled input of `--input-size MB / 8` u64s:
        let n_elems = self.cfg.input_size_mb * 1024 * 1024 / std::mem::size_of::<u64>();
        let mut rng = SmallRng::seed_from_u64(self.cfg.seed);
        let mut input: Vec<u64> = (0..n_elems as u64).collect();
        // Fisher-Yates shuffle to ensure non-trivial sort work:
        for i in (1..input.len()).rev() {
            let j = rng.gen_range(0..=i);
            input.swap(i, j);
        }
        self.input = Some(input);
        self.pool = Some(pool);
        Ok(())
    }

    fn tick(&mut self) -> Box<dyn SinkValue> {
        // Clone the input each tick so we sort fresh data:
        let mut data = self.input.as_ref().unwrap().clone();
        let pool = self.pool.as_ref().unwrap();
        pool.install(|| {
            parallel_merge_sort(&mut data);
        });
        // Defeat DCE: ensure the sorted buffer is observed:
        std::hint::black_box(&data[..]);
        Box::new(data)   // dropped on return
    }

    fn allocations_per_tick(&self) -> u64 {
        // Approximate: each merge step allocates one Vec<u64> of half-size.
        // Total allocs ≈ N (one alloc per merge node), where N = ceil(log2(elems)).
        // We approximate as `elems` for a coarse number — Phase 4 aggregator
        // can divide ticks_per_s * allocations_per_tick to get alloc rate.
        let n_elems = self.cfg.input_size_mb * 1024 * 1024 / 8;
        n_elems as u64
    }
}

fn parallel_merge_sort<T: Ord + Send + Copy>(slice: &mut [T]) {
    if slice.len() <= 1024 {
        // Base case: small arrays use std stable sort (no further parallelism)
        slice.sort_unstable();
        return;
    }
    let mid = slice.len() / 2;
    let (left, right) = slice.split_at_mut(mid);
    rayon::join(|| parallel_merge_sort(left), || parallel_merge_sort(right));

    // MERGE STEP — this is where the allocations happen:
    let mut merged: Vec<T> = Vec::with_capacity(slice.len());   // <-- the alloc
    let (mut i, mut j) = (0, 0);
    while i < left.len() && j < right.len() {
        if left[i] <= right[j] { merged.push(left[i]); i += 1; }
        else { merged.push(right[j]); j += 1; }
    }
    merged.extend_from_slice(&left[i..]);
    merged.extend_from_slice(&right[j..]);
    slice.copy_from_slice(&merged);
}
```

### Why `rayon::join` not `std::thread::scope`

- `rayon::join`'s work-stealing pool is significantly more efficient under recursive divide-and-conquer than spawning fresh OS threads each level.
- A scoped `ThreadPool::new().num_threads(N)` cleanly honours `--threads N`.
- `pool.install(|| ...)` ensures the `rayon::join` calls inside use **our** pool, not the global one — important for run-all where multiple scenarios share a process.

### Why **not** `par_sort_unstable`

`<[T]>::par_sort_unstable()` is highly tuned (pattern-defeating quicksort with pre-allocated buffers) — exactly the wrong choice. We *want* allocations in the critical path.

## Mem-bound (SCEN-07)

### Mode `linked-list`: uniform 64B nodes

```rust
// Layout: 8 bytes (Option<Box<Node>>) + 56 bytes payload = 64 bytes total.
// Verify with std::mem::size_of::<Node>() == 64 in a unit test.
#[repr(C)]
struct Node {
    next: Option<Box<Node>>,    // 8 bytes (Option<NonNull<...>> is niche-optimised)
    payload: [u8; 56],
}

const _: () = assert!(std::mem::size_of::<Node>() == 64);

pub struct MemBoundLinkedList {
    cfg: MemBoundConfig,
}

impl Scenario for MemBoundLinkedList {
    fn tick(&mut self) -> Box<dyn SinkValue> {
        let n_nodes = self.cfg.size_mb * 1024 * 1024 / 64;
        // Build the chain (allocations under test):
        let mut head: Option<Box<Node>> = None;
        for i in 0..n_nodes {
            head = Some(Box::new(Node {
                next: head,
                payload: [(i as u8); 56],     // write defeats DCE
            }));
        }
        // Traverse to defeat further DCE on the chain itself:
        let mut count = 0u64;
        let mut cursor = head.as_deref();
        while let Some(node) = cursor {
            count = count.wrapping_add(node.payload[0] as u64);
            cursor = node.next.as_deref();
        }
        std::hint::black_box(count);
        Box::new(head)   // dropped → chain frees
    }
}
```

### Mode `strided-array`: prefetcher-defeating stride

```rust
pub struct MemBoundStridedArray {
    cfg: MemBoundConfig,
    buffer: Option<Vec<u64>>,
}

impl Scenario for MemBoundStridedArray {
    fn setup(&mut self) -> anyhow::Result<()> {
        let n = self.cfg.size_mb * 1024 * 1024 / 8;
        let mut rng = SmallRng::seed_from_u64(self.cfg.seed);
        let buf: Vec<u64> = (0..n as u64).map(|_| rng.gen()).collect();
        self.buffer = Some(buf);
        Ok(())
    }

    fn tick(&mut self) -> Box<dyn SinkValue> {
        let buf = self.buffer.as_mut().unwrap();
        let n = buf.len();
        // Prime stride: 4099 is prime, > L1 line count. Defeats stride
        // prefetchers (which detect linear strides up to ~16 cache lines).
        let stride = 4099;
        let mut accumulator: u64 = 0;
        let mut i = 0;
        for _ in 0..n {
            accumulator = accumulator.wrapping_add(buf[i]);
            i = (i + stride) % n;
        }
        std::hint::black_box(accumulator);
        Box::new(())     // No new alloc per tick — this mode tests RSS + bandwidth, not alloc churn
    }
}
```

Note: `strided-array` has **no per-tick alloc** (the buffer is pre-allocated in setup). Throughput is the meaningful metric here, and `peak_rss_kb` should match `size_mb * 1024`. `linked-list` mode is the alloc-heavy one — both modes together cover the design intent of SCEN-07.

## Contention (SCEN-08)

### Critical: tight alloc/free, no accumulation

Per CONTEXT.md and §Pitfall 7 above, the scenario MUST drop each buffer before the next iteration:

```rust
pub struct Contention {
    cfg: ContentionConfig,
}

impl Scenario for Contention {
    fn allocations_per_tick(&self) -> u64 {
        (self.cfg.threads as u64).saturating_mul(self.cfg.iters_per_tick)
    }

    fn tick(&mut self) -> Box<dyn SinkValue> {
        let cfg = self.cfg.clone();
        std::thread::scope(|scope| {
            let mut handles = Vec::with_capacity(cfg.threads);
            for _ in 0..cfg.threads {
                let cfg = cfg.clone();
                handles.push(scope.spawn(move || {
                    let mut count = 0u64;
                    for _ in 0..cfg.iters_per_tick {
                        let b: Box<[u8]> = vec![0u8; cfg.alloc_size].into_boxed_slice();
                        // §Pitfall 7: black_box the alloc, then drop. DO NOT push to a Vec.
                        std::hint::black_box(&b);
                        count = count.wrapping_add(b[cfg.alloc_size / 2] as u64);
                        // implicit drop(b) at end of loop body
                    }
                    count
                }));
            }
            let mut total = 0u64;
            for h in handles {
                match h.join() {
                    Ok(c) => total = total.wrapping_add(c),
                    Err(p) => std::panic::resume_unwind(p),    // CR-02 pattern
                }
            }
            std::hint::black_box(total);
        });
        Box::new(())
    }
}
```

`iters_per_tick` should default to ~10_000 so per-tick latency is comfortably above noise floor but well under HIST_MAX_NS (300s).

## Fragmentation-soak (SCEN-09)

### Long-lived state across ticks (uses `&mut self`)

```rust
pub struct FragmentationSoak {
    cfg: FragmentationConfig,
    long_lived: Vec<Box<[u8]>>,    // accumulates across ticks
    rng: SmallRng,
}

impl FragmentationSoak {
    pub fn new(cfg: FragmentationConfig) -> Self {
        let rng = SmallRng::seed_from_u64(cfg.seed);
        Self { cfg, long_lived: Vec::with_capacity(10_000), rng }
    }
}

impl Scenario for FragmentationSoak {
    fn allocations_per_tick(&self) -> u64 {
        self.cfg.allocs_per_tick   // default ~10_000
    }

    fn tick(&mut self) -> Box<dyn SinkValue> {
        // 90% short-lived, 10% long-lived (CONTEXT.md decision).
        let mut short: Vec<Box<[u8]>> = Vec::with_capacity(self.cfg.allocs_per_tick as usize);
        for _ in 0..self.cfg.allocs_per_tick {
            if self.rng.gen::<f32>() < 0.9 {
                // 90% — short-lived 16 bytes:
                let mut b = vec![0u8; 16].into_boxed_slice();
                b[8] = self.rng.gen::<u8>();
                short.push(b);
            } else {
                // 10% — long-lived 4KB held until end of run:
                let mut b = vec![0u8; 4096].into_boxed_slice();
                b[2048] = self.rng.gen::<u8>();
                // §Pitfall 8: cap long_lived size to prevent unbounded growth:
                if self.long_lived.len() >= 10_000 {
                    // Evict a random old one before pushing:
                    let evict_idx = self.rng.gen_range(0..self.long_lived.len());
                    self.long_lived.swap_remove(evict_idx);
                }
                self.long_lived.push(b);
            }
        }
        // short-lived auto-drops here when `short` goes out of scope.
        // long_lived persists in self for the next tick.
        std::hint::black_box(&self.long_lived);
        Box::new(short)    // dropped on return; long-lived survives.
    }

    fn teardown(&mut self) {
        self.long_lived.clear();   // explicit drop at end of run
    }
}
```

The `&mut self` signature on `tick()` is what enables this — Phase 1's harness already supports it (verified in `crates/alloc-bench-core/src/harness.rs:16`).

## Realloc-storm (SCEN-10)

### Vec growth from capacity 0

Vec growth strategy: capacity doubles when full (1 → 2 → 4 → 8 → ... amortised O(1) per push, but each grow triggers an alloc + memcpy + free of the old buffer).

Translating `--target-size MB` to push counts:
- `target_bytes = target_size_mb * 1024 * 1024`
- Each `push(0u8)` adds 1 byte → `push_count = target_bytes`

Number of reallocations during the grow: `log2(target_bytes)` ≈ 26 for 64MB. Each realloc copies the entire current buffer.

```rust
pub struct ReallocStorm {
    cfg: ReallocStormConfig,
}

impl Scenario for ReallocStorm {
    fn allocations_per_tick(&self) -> u64 {
        // Approximate count: log2(target_bytes) reallocations per growth cycle.
        let target_bytes = self.cfg.target_size_mb * 1024 * 1024;
        (target_bytes as u64).next_power_of_two().trailing_zeros() as u64
    }

    fn tick(&mut self) -> Box<dyn SinkValue> {
        let target_bytes = self.cfg.target_size_mb * 1024 * 1024;
        // Start from capacity 0 each tick — fresh growth from scratch
        // (CONTEXT.md decision: "test growth from scratch every tick"):
        let mut v: Vec<u8> = Vec::with_capacity(0);
        for i in 0..target_bytes {
            v.push((i & 0xFF) as u8);
        }
        // Defeat DCE: read a few elements:
        std::hint::black_box(v[v.len() / 2]);
        Box::new(v)    // drop at end of tick → free the entire buffer
    }
}
```

## Run-all (SCEN-11)

### Recommendation: dynamic dispatch via Box<dyn Scenario>

Hardcoded match arms duplicate logic across `run_all` and per-scenario subcommands. Instead, build a small registry:

```rust
// In cli/run.rs:

type ScenarioBuilder = Box<dyn FnOnce() -> anyhow::Result<Box<dyn alloc_bench_core::Scenario>>>;

fn default_scenarios(seed: u64) -> Vec<(&'static str, ScenarioBuilder)> {
    use alloc_bench_core::scenarios::*;
    vec![
        ("multithread", Box::new(move || {
            Ok(Box::new(Multithread::new(MultithreadConfig {
                threads: 4, objects: 10_000,
                size_dist: SizeDist::Uniform, size_min: 16, size_max: 1024, seed,
            }.validated()?)))
        })),
        ("web", Box::new(move || {
            Ok(Box::new(Web::new(WebConfig { server_workers: 2, client_workers: 4, seed }.validated()?)))
        })),
        // ... 8 more
    ]
}

pub fn run_all(output: &str, seed: u64) -> anyhow::Result<()> {
    let cfg = HarnessConfig {
        warmup: Duration::from_secs(1),
        measure: Duration::from_secs(5),
        seed,
    };
    let mut runs: Vec<RunOrError> = Vec::new();

    for (name, builder) in default_scenarios(seed) {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut scenario = builder()?;
            let outcome = alloc_bench_core::run(&mut *scenario, &cfg, allocator::stats)?;
            Ok::<_, anyhow::Error>(assemble_run(scenario.name(), scenario.config_json(), outcome)?)
        }));
        match result {
            Ok(Ok(run)) => runs.push(RunOrError::Success(run)),
            Ok(Err(e))  => runs.push(RunOrError::Failure { name: name.to_string(), error: e.to_string() }),
            Err(panic) => {
                let msg = panic_msg(&panic);
                runs.push(RunOrError::Failure { name: name.to_string(), error: msg });
            }
        }
    }
    let json = serde_json::to_string_pretty(&runs)?;
    std::fs::write(output, json)?;
    Ok(())
}
```

Where `RunOrError` is an enum that serializes flat:

```rust
#[derive(Serialize)]
#[serde(untagged)]
enum RunOrError {
    Success(Run),
    Failure { name: String, status: &'static str, error: String },
}
```

Or simpler: extend `Run` with `Option<String>` `status` and `error` fields and emit a degenerate `Run` for failures.

### Why dynamic dispatch over hardcoded match arms

- 9 scenarios × 2 places (run-all + per-scenario subcommand) = 18 places to maintain otherwise
- Adding a new scenario in v2 needs touching only the registry
- Box<dyn> overhead is negligible — once-per-tick virtual call cost is dwarfed by the actual workload

## DCE Verification

### Concrete grep pattern for `cargo build --release --emit=llvm-ir`

[VERIFIED: doc.rust-lang.org/cargo/commands/cargo-rustc.html]

**Invocation:**

```bash
cargo rustc --release \
    --no-default-features \
    --features alloc-jemalloc \
    --bin alloc-bench-cli \
    -- --emit=llvm-ir
```

**Output:** `.ll` files appear in `target/release/deps/alloc_bench_cli-<hash>.ll`.

### Strings indicating allocation calls survived

The exact symbol depends on the toolchain & allocator:

| Allocator | LLVM IR symbol | grep pattern |
|-----------|----------------|--------------|
| System (libmalloc on macOS, ptmalloc on glibc) | `__rust_alloc`, `__rust_dealloc` | `grep -E '\bcall\b.*(__rust_alloc|__rust_dealloc)\b'` |
| jemalloc | `__rust_alloc`, indirect to `je_malloc` | Same as above; the indirection lives in the linker stage, not IR |
| mimalloc | `__rust_alloc`, indirect to `mi_malloc` | Same |

Rust uses a stable shim: every allocation goes through `__rust_alloc` / `__rust_dealloc` regardless of the chosen `#[global_allocator]`. The actual allocator is bound at link time (LTO=fat may inline these in the final stage, but the IR before linking still has them).

**Recommended grep pattern** (sufficient for "allocations survived DCE"):

```bash
# Look for direct calls to the Rust alloc shim:
grep -c 'call.*__rust_alloc' target/release/deps/alloc_bench_cli-*.ll

# Or more loosely (matches direct + indirect):
grep -cE 'call\b.*\b(__rust_alloc|alloc::alloc::|Vec::push)' target/release/deps/alloc_bench_cli-*.ll
```

If either pattern returns 0, the scenario was elided. Expected counts vary, but >= 1 per scenario function is the floor.

### macOS vs Linux IR differences

[ASSUMED] LLVM IR before linking is essentially identical between macOS and Linux when targeting the same allocator — the allocator call is a stable symbol (`__rust_alloc`). The differences appear at link-time:
- macOS: dynamic-only (no `crt-static`)
- Linux glibc: dynamic by default
- Linux musl static: `crt-static` causes the allocator implementation to be statically linked

For DCE verification we only need to check the IR has the call sites — the actual symbol resolution doesn't affect that. **The grep pattern works identically on both platforms.**

### Sanity check: RSS grows during a no-op-looking scenario

Phase 2 success criterion 4 from ROADMAP requires this. Implementation:

```rust
// In a test or smoke script:
// Run the contention scenario for 5s with --threads 4 --alloc-size 1024.
// Read peak_rss_kb from the resulting JSON.
// Assert peak_rss_kb > some baseline (e.g., 10MB).
// If it doesn't grow, the alloc/free pair was elided.
```

Combine with the LLVM IR grep — if both pass, DCE discipline is intact.

### Just-recipe (recommended)

```just
# justfile
dce-check ALLOCATOR='jemalloc':
    @echo "Building with --emit=llvm-ir..."
    cargo rustc --release --no-default-features --features alloc-{{ALLOCATOR}} \
        --bin alloc-bench-cli -- --emit=llvm-ir 2>&1 | tail -5
    @echo "Checking for surviving allocation calls..."
    @count=$(grep -c 'call.*__rust_alloc' target/release/deps/alloc_bench_cli-*.ll); \
    if [ $count -lt 10 ]; then \
        echo "FAIL: only $count alloc calls survived (expected >= 10)"; exit 1; \
    else \
        echo "PASS: $count alloc calls survived DCE"; \
    fi
```

## Code Examples

### axum 0.8 hello-world server (canonical)
```rust
// Source: https://docs.rs/axum/0.8.9/axum/ (verified 2026-05-18)
use axum::{routing::get, Router};

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(|| async { "Hello, World!" }));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

### tokio Runtime explicit construction
```rust
// Source: https://docs.rs/tokio/latest/tokio/runtime/struct.Runtime.html
use tokio::runtime;

let rt = runtime::Builder::new_multi_thread()
    .worker_threads(4)
    .enable_all()
    .build()
    .unwrap();

rt.block_on(async {
    println!("Custom runtime with 4 worker threads");
});

// Spawn a task:
let handle = rt.handle();
handle.spawn(async {
    println!("Task running on worker thread");
});
```

### crossbeam_channel bounded with both sides cloned (MPMC base case)
```rust
// Source: https://docs.rs/crossbeam-channel/0.5.15/crossbeam_channel/index.html
use std::thread;
use crossbeam_channel::bounded;

let (s1, r1) = bounded(0);
let (s2, r2) = (s1.clone(), r1.clone());

thread::spawn(move || {
    r2.recv().unwrap();
    s2.send(2).unwrap();
});

s1.send(1).unwrap();
r1.recv().unwrap();
```

### rayon::join recursive divide-and-conquer
```rust
// Source: https://docs.rs/rayon/1.12.0/rayon/fn.join.html (verified pattern)
fn parallel_merge_sort<T: Ord + Send>(slice: &mut [T]) {
    if slice.len() <= 1 { return; }
    let mid = slice.len() / 2;
    let (left, right) = slice.split_at_mut(mid);
    rayon::join(
        || parallel_merge_sort(left),
        || parallel_merge_sort(right),
    );
    // Merge step here (allocates a Vec<T> of size slice.len())
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `axum 0.7` (`hyper-util` not yet stabilised) | `axum 0.8` + `axum::serve(listener, app)` | 2024-11 | `axum::Server::bind()` removed; use `serve(TcpListener, Router)` |
| `actix-web` for benchmarks | `axum + tokio` | ~2023 | tokio-native; more idiomatic in 2026 |
| `std::sync::mpsc` | `crossbeam-channel::bounded()` | 2020+ | std lacks MPMC; std is slower |
| `hyper::Server::bind` | `axum::serve(TcpListener, Router)` | axum 0.8 | Lower-level; explicit listener; better with TcpListener::bind from tokio |
| `reqwest::Client::new()` (default builder) | `Client::builder().pool_max_idle_per_host(N).build()` | reqwest 0.11+ | Tunable pool — important for load-gen |

**Deprecated/outdated:**
- Anything that builds the tokio `Runtime` per request: superseded by once-per-scenario in setup()
- LinkedList<T> for memory tests: superseded by hand-rolled fixed-size Box<Node> chains (uniform 64B)
- `[u8; N]::new()` (no such thing) — use `vec![0u8; N].into_boxed_slice()`

## Validation Architecture

> Per `.planning/config.json`, `workflow.nyquist_validation = false`. Section omitted intentionally per protocol.

## Runtime State Inventory

> Skipped — Phase 2 is a pure-additive code phase (no rename/refactor/migration). No runtime state survives across scenarios except scenario-internal state inside each `Scenario` struct, which is automatically managed by Rust's ownership model.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | All scenarios + DCE check | ✓ | rustc 1.91.1 (verified `rustc --version`) | — |
| cargo | Build + test | ✓ | cargo 1.91.1 | — |
| TCP loopback (127.0.0.1) | Web scenario (SCEN-02) | ✓ | n/a | — |
| `--emit=llvm-ir` support | DCE verification | ✓ (built into rustc) | n/a | — |
| Internet access for `cargo fetch` | First-time build (axum, tokio, reqwest, crossbeam, rayon) | ✓ (required at first build only — verified by Phase 1 build artifacts) | n/a | If absent: cargo offline |

**Missing dependencies with no fallback:** none

**Missing dependencies with fallback:** none

Phase 2 has zero new external dependencies beyond cargo crates. All scenarios run on the host where Phase 1 already builds; the matrix expansion is Phase 3's concern.

## Crate Version Pins (final recommendation)

| Crate | Pin | Source authority | Rationale |
|-------|-----|------------------|-----------|
| `axum` | `0.8` | CLAUDE.md mandate + [VERIFIED: cargo info axum] | resolves to 0.8.9; tokio-native; modern 2026 default |
| `tokio` | `1` | CLAUDE.md mandate + [VERIFIED: cargo info tokio] | resolves to 1.52.3; features `["rt-multi-thread", "macros", "net", "time", "sync"]` |
| `reqwest` | `0.12` | CLAUDE.md table mandates 0.12 | 0.13 exists but CLAUDE.md says 0.12. **If transitive resolution conflicts**, the planner can bump to 0.13 — semver-compatible API change for our use (POST + JSON only). Default-features=false; features=`["json", "rustls-tls"]` to avoid OpenSSL on musl |
| `crossbeam-channel` | `0.5` | CLAUDE.md §8 + [VERIFIED: cargo info crossbeam-channel] | resolves to 0.5.15 |
| `rayon` | `1` | Recommended by this research | resolves to 1.12.0; needed for SCEN-06 only |
| `tower` | `0.5` | CONTEXT.md lists explicitly | resolves to 0.5.3; transitive via axum, listing explicitly is harmless |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | macOS LLVM IR is essentially identical to Linux for our purposes | DCE Verification > macOS vs Linux | LOW — DCE only needs `__rust_alloc` symbol presence, which is a stable Rust shim. Worst case: planner adds an `--target=x86_64-unknown-linux-gnu` flag to the IR check. |
| A2 | reqwest 0.12 still resolves cleanly against tokio 1.52 (it's a 1-year-old crate version) | Crate Version Pins | LOW — 0.12 is still on crates.io. If the planner hits resolution issues, bump to 0.13.x — API surface for POST+JSON is unchanged. |
| A3 | Vec::push amortised growth strategy is doubling (1 → 2 → 4 → 8 → ...) | Realloc-storm | LOW — Rust std documentation describes Vec growth as "amortised O(1)" with current implementation doubling. If the strategy ever changes, `allocations_per_tick` count drifts but the test still measures realloc behaviour. |
| A4 | `std::mem::size_of::<Node>() == 64` with `Option<Box<Node>>` + `[u8; 56]` on 64-bit Linux/macOS | Mem-bound > linked-list | MEDIUM — this is verifiable in a unit test (and §Pitfall 5 says to verify). If the layout drifts (unlikely but possible across Rust versions), unit test catches it before any benchmark runs. |
| A5 | Stride 4099 defeats both L1 streaming and L2 stride prefetchers on x86-64 | Mem-bound > strided-array | MEDIUM — modern Intel/AMD prefetchers detect strides up to ~16 cache lines (~1024 bytes). 4099 > that, but new microarchitectures may improve. Acceptable for v1; profile via `perf stat -e cache-misses` in CI if desired. |
| A6 | `axum::serve(listener, app).await` returns `!` on success and Err on shutdown | Web scenario | LOW — verified via docs.rs/axum/0.8.9; `unwrap_or_else(|e| ...)` handles both shutdown and any I/O error gracefully. |
| A7 | `tokio::spawn` from outside an async context inside the runtime is fine if we hold a `Handle` | Web scenario | LOW — verified via docs.rs/tokio runtime; `runtime.spawn(...)` is exactly this pattern. |
| A8 | `panic::catch_unwind` works in scenario code (no Send-bound issues) | Run-all | MEDIUM — `catch_unwind` requires `UnwindSafe`. We may need `AssertUnwindSafe(closure)` for closures capturing mutable state. Skeleton in §Run-all already uses it. |

## Open Questions

1. **Should run-all parallelize scenarios across processes for faster turnaround?**
   - What we know: CONTEXT.md says "small, fast (~60s total)" so default config is light enough to run sequentially in one process.
   - What's unclear: does sharing the global allocator across scenarios in one process leak state (e.g., jemalloc retained memory)?
   - Recommendation: stick with sequential one-process for v1. If aggregator (Phase 4) wants per-scenario clean allocator state, that's a Phase 5 CI concern (subprocess invocation per scenario).

2. **Should the web scenario use HTTP/1 or HTTP/2 for the loopback test?**
   - What we know: axum 0.8 supports both via `http1`/`http2` features.
   - What's unclear: HTTP/2 multiplexing changes allocation pattern (one connection, many streams = fewer Connection:close TCB allocs).
   - Recommendation: HTTP/1 for v1 (simpler, more representative of typical microservice traffic per CLAUDE.md §7; one connection-per-request stresses tokio + allocator harder).

3. **For SCEN-06, should we verify allocations land in the merge step via `valgrind --tool=massif` or similar?**
   - What we know: compiler can theoretically hoist the merge buffer alloc out of the recursion if it sees no aliasing.
   - What's unclear: whether `LTO=fat + opt-level=3` would attempt this.
   - Recommendation: write a quick test that asserts `allocator_stats.allocated > X` after one tick where X is at least input_size_mb / 2. If LLVM elides, the assertion fails fast.

## Sources

### Primary (HIGH confidence)
- [VERIFIED: cargo info axum] — axum 0.8.9 features and dependencies (queried 2026-05-18)
- [VERIFIED: cargo info tokio] — tokio 1.52.3 features (queried 2026-05-18)
- [VERIFIED: cargo info reqwest] — reqwest 0.13.3 features (queried 2026-05-18); 0.12.x still on crates.io
- [VERIFIED: cargo info crossbeam-channel] — crossbeam-channel 0.5.15 (queried 2026-05-18)
- [VERIFIED: cargo info rayon] — rayon 1.12.0 (queried 2026-05-18)
- [CITED: docs.rs/axum/0.8.9/axum/] — canonical hello-world server pattern with axum::serve
- [CITED: docs.rs/tokio/latest/tokio/runtime/struct.Runtime.html] — Runtime::new, Builder::new_multi_thread, block_on, handle().spawn
- [CITED: docs.rs/crossbeam-channel/0.5.15/] — bounded(cap), MPMC sender/receiver clone pattern
- [CITED: docs.rs/rayon/1.12.0/rayon/slice/trait.ParallelSliceMut.html] — par_chunks_mut, par_sort_unstable, manual rayon::join recursion pattern
- [CITED: doc.rust-lang.org/cargo/commands/cargo-rustc.html] — `cargo rustc -- --emit=llvm-ir` flag and output location
- [CITED: .planning/CLAUDE.md] — project tech stack mandate (axum, tokio, crossbeam-channel, mimalloc/jemalloc/etc.)
- [CITED: .planning/phases/02-scenario-fan-out/02-CONTEXT.md] — locked decisions per scenario
- [CITED: .planning/phases/01-foundation-mvp-slice/01-PLAN.md, 02-PLAN.md] — Phase 1 implementation pattern
- [CITED: crates/alloc-bench-core/src/harness.rs] — Scenario trait contract (verified live)
- [CITED: crates/alloc-bench-core/src/output.rs] — v1 results.json schema (verified live)
- [CITED: crates/alloc-bench-core/src/scenarios/multithread.rs] — reference scenario pattern (verified live)
- [CITED: crates/alloc-bench-cli/src/run.rs, main.rs] — CLI dispatch pattern (verified live)

### Secondary (MEDIUM confidence)
- LLVM IR alloc-symbol convention (`__rust_alloc`/`__rust_dealloc` as stable shim) — verified by Rust internal symbol layout common knowledge; this is well-known and stable

### Tertiary (LOW confidence — flagged for validation)
- Stride 4099 defeating prefetchers on modern x86 (§Pitfall 6) — heuristic based on prefetcher line-detect range; acceptable for v1 but should be validated with `perf stat` on real CI hardware

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries verified live against crates.io
- Architecture: HIGH — extends Phase 1's locked patterns; trait + harness are verified live in source
- Pitfalls: HIGH for §1-7, MEDIUM for §6 (prefetcher behaviour is microarch-dependent)
- DCE verification: HIGH — Rust alloc shim is stable; cargo --emit=llvm-ir is documented
- Run-all dispatch: HIGH — `Box<dyn Scenario>` is idiomatic and works with the existing trait

**Research date:** 2026-05-18
**Valid until:** ~2026-08-18 (axum/tokio/reqwest/crossbeam/rayon are stable; only major version bumps would invalidate; revisit if anyone updates CLAUDE.md crate-version pins)

## RESEARCH COMPLETE

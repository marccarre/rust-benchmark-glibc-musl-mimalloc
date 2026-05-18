---
phase: 02-scenario-fan-out
plan: "02"
subsystem: scenarios
tags: [rust, scenarios, web, axum, tokio, reqwest, cpu-bound, rayon, merge-sort, fragmentation-soak, schema-additive]

requires:
  - phase: 02-01
    provides: "Locked Phase-2 patterns: ScenarioInfo.unit additive field, drive_and_emit() helper, per-scenario {Name}Config + validated() pattern, mid-buffer-write DCE-defeat, std::panic::resume_unwind worker-panic propagation"
  - phase: 01-02
    provides: "Locked harness contract (Scenario trait, harness::run, v1 results.json schema, parse_duration, allocator stats injection closure)"
provides:
  - "Web scenario (SCEN-02) with in-process axum + tokio + reqwest, runtime built once in setup() and reused across ticks"
  - "CpuBound scenario (SCEN-06) with scoped rayon::ThreadPool + parallel merge-sort with allocations in the merge step"
  - "FragmentationSoak scenario (SCEN-09) with cross-tick &mut self state (long-lived Vec) and cap+random-eviction guard"
  - "3 new CLI subcommands: web, cpu-bound, fragmentation-soak"
  - "Workspace dependencies: axum 0.8, tokio 1, tower 0.5, reqwest 0.12 (rustls-tls), rayon 1"
affects:
  - "Phase 2 Plan 03 (run-all + DCE check — registers all 9 scenarios from Plans 01+02 via dynamic dispatch)"
  - "Phase 4 (Aggregator — consumes scenario.unit field for web's req_per_s labelling)"

tech-stack:
  added:
    - "axum 0.8 (workspace + alloc-bench-core dep)"
    - "tokio 1 (rt-multi-thread, macros, net, time, sync)"
    - "tower 0.5 (transitive via axum, listed explicitly per RESEARCH.md)"
    - "reqwest 0.12 (default-features=false, json+rustls-tls — no OpenSSL)"
    - "rayon 1 (cpu-bound scoped pool only)"
  patterns:
    - "tokio::runtime::Builder::new_multi_thread().worker_threads(N).enable_all().build() once in setup() — never per tick (RESEARCH.md §Pitfall 1)"
    - "axum::serve(listener, app).await wrapped in unwrap_or_else so the spawned server task never panics the runtime when the scenario drops (RESEARCH.md §A6)"
    - "rayon::ThreadPoolBuilder::new().num_threads(N).build() — SCOPED pool, never the global rayon pool — so --threads honoured under run-all (RESEARCH.md §CPU-bound)"
    - "rayon::join recursion + Vec::with_capacity(total_len) inside the merge step keeps allocations in the critical path (RESEARCH.md §Pitfall 4)"
    - "self.long_lived: Vec<Box<[u8]>> + self.rng across ticks via &mut self — first scenario in suite to use this pattern; cap-and-evict via swap_remove prevents OOM (RESEARCH.md §Pitfall 8)"
    - "tokio::spawn worker panic propagation via JoinError::is_panic + into_panic + std::panic::resume_unwind"

key-files:
  created:
    - "crates/alloc-bench-core/src/scenarios/web.rs (363 lines, 7 unit tests)"
    - "crates/alloc-bench-core/src/scenarios/cpu_bound.rs (232 lines, 7 unit tests)"
    - "crates/alloc-bench-core/src/scenarios/fragmentation.rs (216 lines, 7 unit tests)"
  modified:
    - "Cargo.toml (workspace deps: axum 0.8, tokio 1, tower 0.5, reqwest 0.12, rayon 1)"
    - "Cargo.lock (resolver-driven additions for the dependency closure)"
    - "crates/alloc-bench-core/Cargo.toml (workspace=true for axum, tokio, tower, reqwest, rayon)"
    - "crates/alloc-bench-core/src/scenarios/mod.rs (re-export 3 new scenarios + structs)"
    - "crates/alloc-bench-cli/src/main.rs (3 new Cmd::* variants + dispatch arms)"
    - "crates/alloc-bench-cli/src/run.rs (3 new run_<name> functions reusing drive_and_emit())"

key-decisions:
  - "reqwest pinned to 0.12 per CLAUDE.md mandate; resolver succeeded with 0.12.28 against tokio 1.49 — no bump to 0.13 needed (RESEARCH.md A2)"
  - "Web scenario re-seeds the payload RNG each tick from cfg.seed (deterministic payload shape per tick) — distinct from FragmentationSoak which carries the RNG in &mut self for cross-tick determinism. Both patterns are valid; the choice depends on whether per-tick determinism or cross-tick non-repetition is the goal."
  - "CpuBound input lives in self.input across ticks (built once in setup, cloned per tick). The clone itself adds one large allocation per tick to the workload — intentional, joins the per-merge allocations as observable allocator load."
  - "FragmentationSoak teardown() explicitly clears self.long_lived so the rusage tail records the freeing work — without it, the long-lived bag would survive into Drop where rusage is no longer sampled."
  - "long_lived_cap surfaced as a CLI flag (default 10_000) rather than being a hardcoded constant — gives users a knob to control the long-lived RSS ceiling explicitly per CONTEXT.md flexibility goals."
  - "tokio task panics propagated via JoinError::is_panic check + into_panic + std::panic::resume_unwind — Phase-1 CR-02 pattern adapted for async (the std::thread::scope variant from Plan 01 doesn't apply to tokio::spawn)."
  - "Web scenario's `pool_max_idle_per_host(client_workers)` keeps reqwest's connection pool from being a bottleneck before the allocator under test."

patterns-established:
  - "Async scenarios pattern (Web): tokio Runtime + reqwest Client + axum server all live as Option<...> fields populated in setup(). All four artifacts survive into tick(). Drop on scenario drop terminates the spawned server."
  - "Scoped resource pools (CpuBound): scenario-internal rayon::ThreadPool stored as Option<rayon::ThreadPool> and used via pool.install(...). Never touch the global rayon pool — required for run-all correctness."
  - "Cross-tick mutable state (FragmentationSoak): &mut self lets a scenario carry long-lived owned data across ticks. Pair with a cap-and-eviction policy whenever growth is unbounded by tick count."

requirements-completed: [SCEN-02, SCEN-06, SCEN-09]

duration: ~13min
completed: 2026-05-18
---

# Phase 2, Plan 02: Web / CPU-Bound / Fragmentation-Soak Scenarios

**Three heavier Phase-2 scenarios (SCEN-02 web with in-process axum+tokio+reqwest, SCEN-06 cpu-bound with scoped rayon merge-sort, SCEN-09 fragmentation-soak with cross-tick state and capped long-lived buffers) implementing the async/heavy-dep portion of Phase 2 on top of Plan 01's foundation, plus the workspace deps for axum/tokio/tower/reqwest/rayon.**

## Performance

- **Duration:** ~13 min
- **Completed:** 2026-05-18
- **Tasks:** 6
- **Files modified:** 8 (3 new scenario files + 5 existing files updated)
- **Test count delta:** +21 unit tests (Plan 01 → 41 lib tests; Plan 02 → 62 lib tests)

## Accomplishments

- 3 new scenario implementations covering 3 SCEN requirements:
  - `web.rs` → SCEN-02 with `Web::setup()` building a tokio multi-thread runtime ONCE, binding `127.0.0.1:0`, spawning axum `/echo` fire-and-forget, and building a reqwest client with `pool_max_idle_per_host(client_workers)`. Per-tick: `runtime.block_on` driving `client_workers` parallel POSTs that round-trip a ~1KB UserProfile JSON payload.
  - `cpu_bound.rs` → SCEN-06 with a SCOPED `rayon::ThreadPool` (never the global pool) honouring `--threads N` even in run-all. Recursive `parallel_merge_sort` allocates `Vec<u64>::with_capacity(total_len)` at every merge node so allocations land in the critical path.
  - `fragmentation.rs` → SCEN-09 with `&mut self` cross-tick state. 90% short-lived 16-byte buffers drop on tick return; 10% long-lived 4KB buffers go into `self.long_lived` with cap-and-random-eviction (`swap_remove`, O(1)) capping peak_rss for the long-lived bag at `cap × 4096` bytes regardless of duration.
- 3 new CLI subcommands wired with the canonical CLI surface from CONTEXT.md
- Workspace deps added (`axum 0.8`, `tokio 1` with rt-multi-thread/macros/net/time/sync, `tower 0.5`, `reqwest 0.12` with rustls-tls only, `rayon 1`); resolver succeeded with `reqwest 0.12.28` against `tokio 1.49` — no version bump needed
- 21 new unit tests across the 3 new scenario files: every config rejects malformed inputs, every scenario has a setup→tick smoke, plus targeted invariant tests (long-lived cap enforcement across 50 ticks, parallel_merge_sort correctness on both base-case and recursion paths, UserProfile JSON serialised size sanity)
- All 3 end-to-end smoke commands produce schema-valid JSON with positive `ticks_per_s` and `tick_latency_ns.p50`
- No regression in Plan-01 scenarios (multithread + spmc smoke checks still pass; multithread JSON shape still has no `unit` key)

## Task Commits

| Task | Description                                                              | Commit    |
| ---- | ------------------------------------------------------------------------ | --------- |
| 1    | Workspace deps (axum/tokio/tower/reqwest/rayon)                          | `961d49f` |
| 2    | Web scenario (SCEN-02) in `web.rs`                                       | `2d720f6` |
| 3    | CpuBound scenario (SCEN-06) in `cpu_bound.rs`                            | `d0c3915` |
| 4    | FragmentationSoak scenario (SCEN-09) in `fragmentation.rs`               | `b0f3750` |
| 5    | 3 new CLI subcommands + run_<name> dispatchers                           | `6cae0f8` |
| 6    | cargo fmt cleanup; full lint + test suite green; smoke checks            | `79a8b32` |

## Files Created

- `crates/alloc-bench-core/src/scenarios/web.rs` — 363 lines, 7 unit tests
- `crates/alloc-bench-core/src/scenarios/cpu_bound.rs` — 232 lines, 7 unit tests
- `crates/alloc-bench-core/src/scenarios/fragmentation.rs` — 216 lines, 7 unit tests

## Files Modified

- `Cargo.toml` — added `axum`, `tokio`, `tower`, `reqwest`, `rayon` to `[workspace.dependencies]`
- `Cargo.lock` — resolver added the closure (axum 0.8.9 → hyper 1.9 → tokio-rustls 0.26 → rustls-webpki 0.103 etc.)
- `crates/alloc-bench-core/Cargo.toml` — added `workspace = true` for the 5 new deps
- `crates/alloc-bench-core/src/scenarios/mod.rs` — `pub mod web; pub mod cpu_bound; pub mod fragmentation;` + `pub use` re-exports for `Web/WebConfig`, `CpuBound/CpuBoundConfig`, `FragmentationSoak/FragmentationConfig`
- `crates/alloc-bench-cli/src/main.rs` — 3 new `Cmd::*` variants (`Web`, `CpuBound`, `FragmentationSoak`) + 3 new dispatch arms; each calls `print_version_banner()` before delegating to `run::run_<name>`
- `crates/alloc-bench-cli/src/run.rs` — 3 new `run_<name>` functions reusing the `drive_and_emit()` helper introduced in Plan 02-01; `run_web` supplies `Some("req_per_s")`, the others supply `None` (default ticks_per_s)

## Smoke Test Results (Task 6)

All 3 commands emit schema-valid JSON in `/tmp/p2-02-smoke/`:

| Command                                                                                                            | scenario.name      | unit          | ticks_per_s | p50 (ns)  | peak_rss_kb |
| ------------------------------------------------------------------------------------------------------------------ | ------------------ | ------------- | ----------- | --------- | ----------- |
| `web --server-workers 1 --client-workers 2 --warmup 1s --duration 2s`                                              | web                | req_per_s     | 10582       | 88511     | 9728        |
| `cpu-bound --threads 2 --input-size 1 --warmup 1s --duration 2s`                                                   | cpu-bound          | (absent)      | 442         | 2244607   | 52224       |
| `fragmentation-soak --allocs-per-tick 100 --long-lived-cap 50 --warmup 1s --duration 2s`                          | fragmentation-soak | (absent)      | 529543      | 1792      | 7776        |

`unit` field is exactly `"req_per_s"` for web; absent (skip_serializing_if = None) for cpu-bound and fragmentation-soak.

Fragmentation-soak peak_rss_kb=7776 (~7.6MB) on a 2s run with `long_lived_cap=50` — well below the 50MB ceiling we'd expect from cap × 4096 + base RSS. The cap-and-evict guard holds; long-lived state stayed bounded.

Plan-01 regression (also run): `multithread --threads 2 --objects 1000` and `spmc --producers 1 --consumers 2 ...` both succeed; multithread JSON contains 0 `"unit"` strings (Phase-1 byte-identical regression preserved); spmc JSON contains exactly 1 `"unit": "iters_per_s"` entry.

## Test Suite Results

- `cargo fmt --all --check` → exit 0
- `cargo clippy --workspace --all-targets -- -D warnings` → exit 0 (no warnings)
- `cargo test --workspace` → 62 lib tests + 3 CLI tests + 1 Phase-1 multithread integration test, all green; 1 doc-test ignored (Plan-01's contention `tick()` doc fence labelled `ignore`)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Borrow-checker error in `parallel_merge_sort` merge step**

- **Found during:** Task 3 (`cargo test -p alloc-bench-core --lib scenarios::cpu_bound`)
- **Issue:** The plan-prescribed merge step `let mut merged: Vec<T> = Vec::with_capacity(slice.len());` doesn't compile because `slice.split_at_mut(mid)` already mutably borrows `slice` until `left` and `right` are last used (which is *after* the `Vec::with_capacity` line). E0502: "cannot borrow `*slice` as immutable because it is also borrowed as mutable."
- **Fix:** Capture `let total_len = slice.len();` before `split_at_mut`, then use `Vec::with_capacity(total_len)`. Identical semantics, no allocation change. Both unit tests covering the merge step path (small + large array) pass after the fix.
- **Files modified:** `crates/alloc-bench-core/src/scenarios/cpu_bound.rs`
- **Commit:** `d0c3915` (Task 3, fix landed in the same commit as the initial implementation)

**2. [Rule 3 - Blocking] `par_sort_unstable` substring in doc-comment failed Task 3 done-criterion**

- **Found during:** Task 3 (`<done>` check: "Source does NOT contain `par_sort_unstable`")
- **Issue:** The module-level doc-comment originally read "`<[T]>::par_sort_unstable()` is the wrong API for this benchmark — …" — the substring matched the negation grep and would have failed the success criterion despite being a *warning against* the wrong API.
- **Fix:** Reworded the doc-comment to "Rayon's pattern-defeating quicksort (the slice extension trait) is the wrong API…" — same warning, no `par_sort_unstable` substring.
- **Files modified:** `crates/alloc-bench-core/src/scenarios/cpu_bound.rs`
- **Commit:** `d0c3915` (folded into Task 3)

**3. [Rule 3 - Blocking] cargo fmt collapsed multi-line blocks in web.rs and cpu_bound.rs**

- **Found during:** Task 6 (`cargo fmt --all --check`)
- **Issue:** rustfmt's default `max_width = 100` collapsed:
  - The `let app = axum::Router::new().route(...)` declaration in `Web::setup` (originally on two lines)
  - A `match h.await { ..., Err(e) if e.is_panic() => { std::panic::resume_unwind(e.into_panic()) } }` arm in `Web::tick` (originally on three lines)
  - A `rayon::join(|| ..., || ...)` call in `parallel_merge_sort` (originally on three lines)
- **Fix:** Ran `cargo fmt --all` and committed the result as a separate `style(02-02)` commit (`79a8b32`). No semantic change.
- **Files modified:** `crates/alloc-bench-core/src/scenarios/web.rs`, `crates/alloc-bench-core/src/scenarios/cpu_bound.rs`

### Refactoring Beyond Plan

None for this plan — Plan 01 already established the `drive_and_emit()` helper, so each new `run_<name>` was a clean fit at ~25 lines.

### Auth Gates

None.

## Self-Check: PASSED

Created files:
- `/Users/marc.carre/src/rust-benchmark-glibc-musl-mimalloc/.claude/worktrees/agent-a7682557a6d392e17/crates/alloc-bench-core/src/scenarios/web.rs` — FOUND
- `/Users/marc.carre/src/rust-benchmark-glibc-musl-mimalloc/.claude/worktrees/agent-a7682557a6d392e17/crates/alloc-bench-core/src/scenarios/cpu_bound.rs` — FOUND
- `/Users/marc.carre/src/rust-benchmark-glibc-musl-mimalloc/.claude/worktrees/agent-a7682557a6d392e17/crates/alloc-bench-core/src/scenarios/fragmentation.rs` — FOUND

Commits:
- `961d49f` (Task 1) — FOUND
- `2d720f6` (Task 2) — FOUND
- `d0c3915` (Task 3) — FOUND
- `b0f3750` (Task 4) — FOUND
- `6cae0f8` (Task 5) — FOUND
- `79a8b32` (Task 6) — FOUND

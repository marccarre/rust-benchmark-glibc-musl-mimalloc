---
phase: 02-scenario-fan-out
plan: "03"
subsystem: scenarios
tags: [rust, run-all, registry, dynamic-dispatch, panic-isolation, dce-verification, schema-additive, phase-2-closure]

requires:
  - phase: 02-01
    provides: "ScenarioInfo.unit additive field, drive_and_emit() helper, 6 channel/contention/mem-bound/realloc-storm scenarios"
  - phase: 02-02
    provides: "Web (axum+tokio+reqwest), CpuBound (rayon merge-sort), FragmentationSoak — 9 of 10 scenarios available for run-all"
  - phase: 01-02
    provides: "Locked harness contract (Scenario trait, harness::run, v1 results.json schema)"
provides:
  - "Run.status + Run.error additive fields with skip_serializing_if Option::is_none — Phase-1 single-scenario JSON byte-identical"
  - "assemble_run() + write_or_print() Run-record helpers — drive_and_emit decomposed into reusable parts"
  - "Box<dyn Scenario> delegation impl on the Scenario trait — generic harness run<S: Scenario> accepts trait objects without contract change"
  - "Cmd::RunAll { output, seed } CLI subcommand"
  - "default_scenarios(seed) registry: 10 entries spanning every SCEN-* requirement, light per-scenario configs (≈60s total)"
  - "run_all() with std::panic::catch_unwind(AssertUnwindSafe(...)) per-scenario isolation — panicked scenarios become status=failed records"
  - "scripts/dce_check.sh + justfile dce-check recipe — Phase-2 ROADMAP success criterion 4"
  - "tests/run_all_smoke.rs end-to-end gate asserting 10-record shape, status mutual-exclusion, scenario-name uniqueness"
affects:
  - "Phase 3 (Docker matrix) — run-all is the canonical per-(libc×allocator) entry point; emits one JSON array per environment"
  - "Phase 4 (Aggregator) — consumes a JSON ARRAY (not single Run) when status field is present; reads scenario.unit for chart labelling"

tech-stack:
  added: []
  patterns:
    - "Box<dyn Scenario>: Scenario delegation impl in core/harness.rs lets the generic run<S: Scenario> signature accept boxed trait objects without changing the Phase-1 trait contract"
    - "ScenarioBuilder = Box<dyn FnOnce() -> Result<Box<dyn Scenario>>> registry pattern — adding a v2 scenario in the future only touches default_scenarios()"
    - "std::panic::catch_unwind(AssertUnwindSafe(closure)) per-scenario isolation — closure mutably borrows scenario state, AssertUnwindSafe required (RESEARCH.md §A8). Double-Result match arm distinguishes panicked vs anyhow-errored, both recorded as status=failed with error populated"
    - "degenerate_failure_run() builds a Run with zeroed metrics + populated error so Env+Build still attribute the failure to host/commit/allocator (Phase 4 aggregator can correlate)"
    - "Two-phase Run assembly via assemble_run() + write_or_print(): single-scenario callers chain both via drive_and_emit; run_all uses assemble_run alone and serialises the Vec<Run> array directly"
    - "DCE gate via grep -h 'call.*__rust_alloc' on target/release/deps/alloc_bench_cli-*.ll — 168 surviving calls on macOS host with system allocator (threshold: >= 10)"

key-files:
  created:
    - "crates/alloc-bench-cli/tests/run_all_smoke.rs (172 lines, 1 integration test)"
    - "scripts/dce_check.sh (108 lines, executable bash script with 3-allocator matrix)"
    - "justfile (24 lines, dce-check + run-all-smoke recipes)"
  modified:
    - "crates/alloc-bench-core/src/output.rs (Run.status + Run.error additive fields + 2 unit tests asserting None-omits / Some-emits invariant)"
    - "crates/alloc-bench-core/src/harness.rs (impl Scenario for Box<dyn Scenario> delegation)"
    - "crates/alloc-bench-cli/src/run.rs (assemble_run + write_or_print helpers, drive_and_emit refactored, default_scenarios registry, run_all + degenerate_failure_run + panic_message)"
    - "crates/alloc-bench-cli/src/main.rs (Cmd::RunAll variant + dispatch arm)"

key-decisions:
  - "Run.status / Run.error are TOP-LEVEL additive fields (CONTEXT.md schema-extension decision) — chose this over the RESEARCH.md alternative #[serde(untagged)] enum RunOrError because it lets Phase-1 single-scenario consumers continue to read the same shape (status/error fields are Option-skipped when None)"
  - "Used `impl Scenario for Box<dyn Scenario>` delegation rather than relaxing the harness `run<S: Scenario>` to `S: ?Sized + Scenario` — the delegation impl is purely additive, doesn't change Phase-1 contract semantics, and is the idiomatic Rust pattern for trait-object dispatch through a generic API"
  - "Registry uses `Vec<(name, unit, ScenarioBuilder)>` (3-tuple) rather than a struct because the default_scenarios body is purely declarative and the tuple's positional clarity (name first, unit second, builder third) reads cleanly down 10 entries"
  - "Builder closure type is `FnOnce` not `Fn` — each scenario is constructed once per run-all invocation; FnOnce lets each closure consume its captured `seed` cleanly"
  - "Web scenario placed LAST in default_scenarios — its tokio runtime + port-bind work has higher fixed setup cost than other scenarios; placing it last means earlier scenarios complete before any port-bind overhead and the run-all eprintln progress shows a smooth ramp"
  - "Mem-bound runs ONCE in run-all (mode=LinkedList) — RESEARCH.md classifies LinkedList as the alloc-heavy mode (StridedArray pre-allocates a single buffer in setup() and never allocates during ticks). Run-all's job is to stress the allocator across all scenarios; LinkedList is the right canonical pick"
  - "DCE script uses `cargo rustc -p alloc-bench-cli ...` (the explicit `-p` was needed because the workspace root is a virtual manifest — fixed during Task 4)"
  - "DCE threshold = 10 (≥ one per scenario function, healthy floor) per RESEARCH.md. Actual count on macOS host with system allocator = 168 — so the threshold has 17× margin and shouldn't false-positive on minor refactors"
  - "Integration test asserts the scenario name SET (not order) so registry reordering (or Phase-4 aggregator canonical sort) doesn't break the gate. Order is verified informally via the run-all eprintln logs"

requirements-completed: [SCEN-11]

duration: ~22min
completed: 2026-05-18
---

# Phase 2, Plan 03: Run-All + DCE Verification — Phase-2 Closure

**SCEN-11 wires all 10 scenarios into a single `run-all` registry with `panic::catch_unwind` per-scenario isolation, adds the DCE verification recipe (Phase-2 ROADMAP success criterion 4), and an end-to-end integration test. Phase-2 closes with all 4 ROADMAP success criteria demonstrably met.**

## Performance

- **Duration:** ~22 min
- **Completed:** 2026-05-18
- **Tasks:** 6
- **Files created:** 3 (tests/run_all_smoke.rs, scripts/dce_check.sh, justfile)
- **Files modified:** 4 (core/output.rs, core/harness.rs, cli/run.rs, cli/main.rs)

## Accomplishments

- **Run-all command** wires all 10 scenarios (1 from Phase 1 + 6 from Plan 01 + 3 from Plan 02) into a single sequential dispatcher emitting one JSON array of Run records.
- **Failure isolation:** `std::panic::catch_unwind(AssertUnwindSafe(closure))` wraps each per-scenario invocation. Panicked scenarios become Run records with `status: "failed"` and a populated `error` field; the other 9 still produce records. The run-all binary exits 0 even when scenarios fail — by design.
- **Schema extension** stayed truly additive: `Run.status` and `Run.error` are `Option<String>` with `skip_serializing_if = "Option::is_none"`. Phase-1 single-scenario JSON shape is byte-identical (verified: `keys` listing unchanged; no `status`/`error` keys appear).
- **DCE verification gate** (Phase-2 ROADMAP success criterion 4): `scripts/dce_check.sh ALLOCATOR` runs `cargo rustc --release --emit=llvm-ir` and greps the produced `.ll` file for `__rust_alloc` call sites. macOS host with system allocator: **168 surviving call sites** vs threshold of 10.
- **Just recipe** `just dce-check ALLOCATOR='system'` wraps the script for `just --list` discoverability + bonus `just run-all-smoke` recipe.
- **Box<dyn Scenario>: Scenario delegation impl** in `core/harness.rs` is the cleanest way to make the generic `run<S: Scenario>` accept a trait object without relaxing the Phase-1 contract.
- **End-to-end integration test** drives `alloc-bench-cli run-all --seed 12345 --output ...` via assert_cmd and asserts: 10 records, every scenario name appears exactly once, status is "success" or "failed", success implies positive metrics + no error, failure implies populated error + zeroed metrics, env+build populated regardless. Runtime: ~60s.

## Task Commits

| Task | Description                                                       | Commit    |
| ---- | ----------------------------------------------------------------- | --------- |
| 1    | Run.status + Run.error additive fields + 2 invariant unit tests   | `4b99747` |
| 2    | assemble_run + write_or_print helpers; drive_and_emit refactor    | `043feaa` |
| 3    | Run-all + 10-scenario registry + panic isolation + Box<dyn> impl  | `dffa73c` |
| 4    | scripts/dce_check.sh + justfile dce-check recipe                  | `99578f6` |
| 5    | tests/run_all_smoke.rs end-to-end gate                            | `a8a130b` |
| 6    | cargo fmt + clippy doc-list-item fix                              | `542ae4a` |

## Files Created

- `crates/alloc-bench-cli/tests/run_all_smoke.rs` — 172 lines; 1 integration test asserting 10-record JSON array shape, status mutual-exclusion, scenario-name uniqueness
- `scripts/dce_check.sh` — 108 lines, executable; matrix over {system, jemalloc, mimalloc} → maps to `--no-default-features` / `--features alloc-jemalloc` / `--features alloc-mimalloc`
- `justfile` — 24 lines; `dce-check ALLOCATOR='system'` recipe + bonus `run-all-smoke` recipe

## Files Modified

- `crates/alloc-bench-core/src/output.rs` — `Run.status: Option<String>` + `Run.error: Option<String>` with `skip_serializing_if = "Option::is_none"`. Two new unit tests pin the additive-only invariant (None omits, Some emits).
- `crates/alloc-bench-core/src/harness.rs` — `impl Scenario for Box<dyn Scenario>` delegation impl so the generic `run<S: Scenario>` signature accepts boxed trait objects without changing the Phase-1 contract.
- `crates/alloc-bench-cli/src/run.rs` — `assemble_run()` + `write_or_print()` helpers extracted from `drive_and_emit()`; `drive_and_emit` is now a thin wrapper. New `default_scenarios(seed)` registry, `run_all()`, `degenerate_failure_run()`, `panic_message()`.
- `crates/alloc-bench-cli/src/main.rs` — `Cmd::RunAll { output, seed }` variant + dispatch arm.

## Smoke Test Results (Task 6)

`target/release/alloc-bench-cli run-all --output /tmp/p2-final/all.json --seed 7` produces 10 records, all `status="success"`:

| #  | Scenario           | status  | ticks_per_s | tick_latency_ns.p50 |
|----|--------------------|---------|-------------|---------------------|
| 1  | multithread        | success | 423         | 2,324,479           |
| 2  | spmc               | success | 2,787       | 336,383             |
| 3  | mpsc               | success | 8,142       | 121,471             |
| 4  | mpmc               | success | 3,948       | 255,359             |
| 5  | contention         | success | 12,045      | 78,335              |
| 6  | mem-bound          | success | 1,097       | 904,703             |
| 7  | realloc-storm      | success | 613         | 1,620,991           |
| 8  | cpu-bound          | success | 186         | 5,304,319           |
| 9  | fragmentation-soak | success | 51,534      | 19,055              |
| 10 | web                | success | 10,270      | 93,439              |

DCE check (system allocator, macOS host): **168 `__rust_alloc` call sites** survive `--release --emit=llvm-ir`. Threshold: >= 10 → 17× margin.

## Test Suite Results

- `cargo fmt --all --check` → exit 0
- `cargo clippy --workspace --all-targets -- -D warnings` → exit 0 (no warnings)
- `cargo test --workspace --release`:
  - core lib: 64 tests pass
  - cli unit tests: 3 pass
  - multithread integration: 1 pass (Phase-1 regression check intact)
  - run-all integration: 1 pass (~60s runtime)
- Phase-1 multithread byte-shape regression: top-level keys list unchanged (`build/env/harness/metrics/run_id/scenario/schema_version`); 0 `"status"` or `"error"` keys present.
- DCE: `bash scripts/dce_check.sh system` exit 0; `just dce-check system` exit 0.

## Phase-2 ROADMAP Closure

All 4 Phase-2 ROADMAP success criteria demonstrably met:

| # | Criterion | Coverage |
|---|-----------|----------|
| 1 | `web --server-workers N --client-workers M --duration 60s` produces results.json with req/s + p50/p95/p99/p999 | Plan 02 Tasks 2+5 (Web impl + CLI). Verified in this plan's Task 6: web smoke emits `unit=req_per_s` |
| 2 | Each of `spmc, mpsc, mpmc, cpu-bound, mem-bound (both modes), contention, fragmentation-soak, realloc-storm` produces schema-valid results.json | Plan 01 Tasks 2-7 (6 scenarios) + Plan 02 Tasks 2-6 (3 scenarios). Verified by individual smoke runs and the run-all aggregation in this plan |
| 3 | `run-all --output results/run.json` emits one record per scenario in execution order; runtime ≈ sum of per-scenario durations | This plan's Task 3 (run_all impl) + Task 5 (smoke test). Default config: 10 × 6s ≈ 60s. Verified: 60.46s actual on macOS host |
| 4 | `cargo build --release --emit=llvm-ir` per scenario; grep verifies allocation calls survive (no DCE); RSS grows during a no-op-looking scenario | This plan's Task 4 (scripts/dce_check.sh + just recipe). 168 surviving `__rust_alloc` call sites for system allocator on macOS host. Plan 01 Task 7's contention smoke (peak_rss_kb growth) covers the RSS half of the criterion |

All 11 SCEN-* requirements satisfied across Phase 2 (SCEN-01 from Phase 1 baseline; SCEN-02 through SCEN-11 from Phase 2 Plans 01/02/03).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `cargo rustc` failed without `-p alloc-bench-cli` in DCE script**

- **Found during:** Task 4 (`bash scripts/dce_check.sh system`)
- **Issue:** The plan's DCE script template didn't include `-p alloc-bench-cli`, but the workspace root is a virtual manifest. `cargo rustc` returned: "this command requires running against an actual package in this workspace."
- **Fix:** Added `-p alloc-bench-cli` to the cargo invocation. Identical behaviour, just disambiguates the package.
- **Files modified:** `scripts/dce_check.sh`
- **Commit:** `99578f6` (fix folded into the original Task 4 commit)

**2. [Rule 3 - Blocking] clippy::doc_lazy_continuation flagged the ScenarioBuilder doc-comment**

- **Found during:** Task 6 (`cargo clippy --workspace --all-targets -- -D warnings`)
- **Issue:** The doc-comment for `ScenarioBuilder` had a line starting with `/// + \`Box<dyn ...>\``. Clippy's markdown parser reads `+` at line start as an unindented list item.
- **Fix:** Reworded to "The closure type is `FnOnce` returning `Box<dyn Scenario>`" — same meaning, no leading `+`.
- **Files modified:** `crates/alloc-bench-cli/src/run.rs`
- **Commit:** `542ae4a`

**3. [Rule 3 - Blocking] cargo fmt collapsed multi-line statements after Task 5**

- **Found during:** Task 6 (`cargo fmt --all --check`)
- **Issue:** rustfmt's default `max_width = 100` collapsed:
  - The `ScenarioBuilder` type alias and the `default_scenarios` `use` list (now fit on one line / fewer lines).
  - `Command::args(...).arg(&out)` in `run_all_smoke.rs` (split across 2 lines).
- **Fix:** Ran `cargo fmt --all` and committed as part of `542ae4a`. No semantic change.
- **Files modified:** `crates/alloc-bench-cli/src/run.rs`, `crates/alloc-bench-cli/tests/run_all_smoke.rs`

### Refactoring Beyond Plan

**4. [Beneficial Refactor] Decomposed drive_and_emit into assemble_run + write_or_print**

- **Found during:** Task 2 (refactor before run_all needs the helper)
- **Action:** The plan asked for `assemble_run` + `write_or_print` as separate helpers (not just renaming `drive_and_emit`). I split the existing `drive_and_emit` (introduced by Plan 01 Task 6) into two phases: `assemble_run` builds the Run from a HarnessOutcome (parameters: name, config, unit, outcome, status, error); `write_or_print` serialises a single Run. `drive_and_emit` is now a thin wrapper that chains them. `run_all` uses `assemble_run` alone and serialises the `Vec<Run>` array directly.
- **Justification:** Letting `run_all` call `drive_and_emit` would force per-scenario JSON file writes (because drive_and_emit always serialises a single Run); it can't aggregate. Decomposition is the cleanest path to keep one Build/Env/run_id construction site (no drift risk between single-scenario and run-all paths) while letting `run_all` build a `Vec<Run>` and serialise once.
- **Files affected:** `crates/alloc-bench-cli/src/run.rs`

**5. [Beneficial Addition] Box<dyn Scenario> delegation impl in core/harness.rs**

- **Found during:** Task 3 (`run_all` needs to pass `&mut Box<dyn Scenario>` to the generic `run<S: Scenario>`)
- **Action:** Added `impl Scenario for Box<dyn Scenario>` with simple `(**self).method()` delegation for all six trait methods. This lets `Box<dyn Scenario>: Scenario` (because `Box<...>` is `Sized`) and the generic harness signature accepts it directly.
- **Justification:** Two alternatives existed:
  1. Relax `run<S: Scenario>` to `run<S: ?Sized + Scenario>` — would change the Phase-1 harness contract.
  2. Add the delegation impl — purely additive, idiomatic, doesn't touch the Phase-1 contract.
  
  The plan's `<key_pitfalls>` section flagged this exact issue ("Object-safety... Pick the simpler one and document the choice"). Option 2 is simpler and lower-risk.
- **Files affected:** `crates/alloc-bench-core/src/harness.rs`

### Auth Gates

None.

## Self-Check: PASSED

Created files (all FOUND):
- `/Users/marc.carre/src/rust-benchmark-glibc-musl-mimalloc/.claude/worktrees/agent-a5f288e3599f50d53/crates/alloc-bench-cli/tests/run_all_smoke.rs`
- `/Users/marc.carre/src/rust-benchmark-glibc-musl-mimalloc/.claude/worktrees/agent-a5f288e3599f50d53/scripts/dce_check.sh`
- `/Users/marc.carre/src/rust-benchmark-glibc-musl-mimalloc/.claude/worktrees/agent-a5f288e3599f50d53/justfile`

Modified files (all FOUND, all in `git diff --stat 6c054821..HEAD`):
- `crates/alloc-bench-core/src/output.rs`
- `crates/alloc-bench-core/src/harness.rs`
- `crates/alloc-bench-cli/src/run.rs`
- `crates/alloc-bench-cli/src/main.rs`

Commits (all FOUND in `git log --oneline 6c054821..HEAD`):
- `4b99747` — Task 1
- `043feaa` — Task 2
- `dffa73c` — Task 3
- `99578f6` — Task 4
- `a8a130b` — Task 5
- `542ae4a` — Task 6

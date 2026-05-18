---
phase: 02-scenario-fan-out
plan: "01"
subsystem: scenarios
tags: [rust, scenarios, channels, contention, mem-bound, realloc-storm, crossbeam-channel, schema-additive]

requires:
  - phase: 01-02
    provides: "Locked harness contract (Scenario trait, harness::run, v1 results.json schema, parse_duration, allocator stats injection closure)"
provides:
  - "SPMC/MPSC/MPMC channel scenarios sharing one ChannelPayload (SCEN-03/04/05)"
  - "Contention scenario with no-accumulation invariant (SCEN-08)"
  - "MemBound scenario with linked-list and strided-array modes (SCEN-07)"
  - "ReallocStorm scenario with Vec growth from capacity 0 every tick (SCEN-10)"
  - "ScenarioInfo.unit: Option<String> additive schema field (skip_serializing_if = None)"
  - "drive_and_emit() helper centralising harness drive + Run record assembly across all CLI dispatchers"
  - "6 new CLI subcommands: spmc, mpsc, mpmc, contention, mem-bound, realloc-storm"
  - "Workspace dependency: crossbeam-channel = 0.5"
affects:
  - "Phase 2 Plan 02 (web/cpu-bound/fragmentation-soak — extend scenarios/mod.rs and cli/main.rs sequentially)"
  - "Phase 2 Plan 03 (run-all + DCE check — registers all 6 scenarios from this plan)"
  - "Phase 4 (Aggregator — consumes scenario.unit field to label charts as iters/s vs ticks/s)"

tech-stack:
  added:
    - "crossbeam-channel 0.5 (workspace dep + alloc-bench-core dep)"
  patterns:
    - "Per-scenario {Name}Config + {Name}Config::validated() pattern reused exactly from MultithreadConfig"
    - "Worker panic propagation via std::panic::resume_unwind for every multi-thread scenario"
    - "Mid-buffer write before send/store to defeat DCE (b[size / 2] = byte)"
    - "std::hint::black_box on every allocation + every consumer-side recv"
    - "drive_and_emit() centralises Build/Env/Run record construction across CLI dispatchers"
    - "Topology constraints enforced at CLI boundary (anyhow::ensure! producers==1 for SPMC, consumers==1 for MPSC)"
    - "Additive schema extension via Option<...> + #[serde(skip_serializing_if = \"Option::is_none\")] keeping schema_version=1"

key-files:
  created:
    - "crates/alloc-bench-core/src/scenarios/channels.rs (SPMC/MPSC/MPMC + ChannelPayload + PayloadDist)"
    - "crates/alloc-bench-core/src/scenarios/contention.rs (Contention scenario, no-accumulation invariant doc)"
    - "crates/alloc-bench-core/src/scenarios/mem_bound.rs (MemBound + LinkedList/StridedArray modes + 64B Node assert)"
    - "crates/alloc-bench-core/src/scenarios/realloc_storm.rs (ReallocStorm with Vec::with_capacity(0) growth)"
  modified:
    - "Cargo.toml (workspace dep crossbeam-channel = 0.5)"
    - "crates/alloc-bench-core/Cargo.toml (crossbeam-channel = workspace=true)"
    - "crates/alloc-bench-core/src/output.rs (ScenarioInfo.unit: Option<String> additive)"
    - "crates/alloc-bench-core/src/scenarios/mod.rs (re-export 4 new scenario types + structs)"
    - "crates/alloc-bench-cli/src/main.rs (6 new Cmd::* variants + dispatch arms)"
    - "crates/alloc-bench-cli/src/run.rs (6 new run_<name> functions + drive_and_emit helper; run_multithread refactored to use helper)"

key-decisions:
  - "Channel scenarios share one channels.rs (RESEARCH.md §Channel Scenarios — 80% shared code; one ChannelPayload + one ChannelConfig + three impl Scenario blocks)"
  - "drive_and_emit() helper introduced rather than copy-pasting the Run-record assembly 7 times — eliminates drift risk and keeps each run_<name> focused on argument parsing + scenario construction"
  - "MemBoundMode::FromStr accepts all three forms (linked-list, linked_list, linkedlist) so the CLI flag matches CONTEXT.md hyphenated form while also being lenient for users typing the underscored form"
  - "Topology constraints (producers==1 for SPMC, consumers==1 for MPSC) enforced at CLI surface, not in ChannelConfig::validated, so the same ChannelConfig struct can serve all three topologies"
  - "Schema extension stayed truly additive: schema_version=1 unchanged; Option<String> + skip_serializing_if = None means existing Phase-1 multithread JSON shape is byte-identical"

patterns-established:
  - "Per-scenario module template scaled to 4 new scenarios in one plan validating the authoring pattern (Phase 2 Plans 02 + 03 will reuse the exact same pattern for web/cpu-bound/fragmentation-soak/run-all)"
  - "When a CLI subcommand maps to a generic config struct (ChannelConfig used by 3 topologies), enforce topology invariants at the CLI dispatcher rather than at the config — avoids splitting one well-tested config into multiple"
  - "drive_and_emit() pattern: each new scenario only writes argument-parsing + config construction + one helper call. Scales linearly to 4+ more scenarios in Plan 02 and Plan 03."

requirements-completed: [SCEN-03, SCEN-04, SCEN-05, SCEN-07, SCEN-08, SCEN-10]

duration: ~11min
completed: 2026-05-18
---

# Phase 2, Plan 01: Channel/Contention/Mem-Bound/Realloc-Storm Scenarios

**Six new scenarios (SCEN-03/04/05/07/08/10) implementing the synchronous, non-async portion of Phase 2's fan-out on top of the locked Phase-1 harness contract, plus the additive `ScenarioInfo.unit` schema field.**

## Performance

- **Duration:** ~11 min
- **Completed:** 2026-05-18
- **Tasks:** 7
- **Files modified:** 6 (4 new scenario files + Cargo.toml + 5 existing files updated)

## Accomplishments

- 4 new scenario implementations cover 6 SCEN requirements:
  - `channels.rs` → SPMC (SCEN-03), MPSC (SCEN-04), MPMC (SCEN-05) sharing one `ChannelPayload` and one `ChannelConfig`
  - `contention.rs` → Contention (SCEN-08) with the documented no-accumulation invariant
  - `mem_bound.rs` → MemBound (SCEN-07) with two modes selected by `MemBoundMode`
  - `realloc_storm.rs` → ReallocStorm (SCEN-10) with `Vec::with_capacity(0)` growth per tick
- 6 new CLI subcommands wired with the canonical CLI surface from CONTEXT.md
- `ScenarioInfo.unit: Option<String>` additive schema field — Phase-1 multithread JSON shape stays byte-identical (no `unit` key emitted when None)
- `drive_and_emit()` helper added to centralise harness drive + Run-record assembly across all CLI dispatchers
- 41 unit tests across the 4 new scenario files — every config rejects malformed inputs, every scenario has a 50ms tick smoke
- All 7 end-to-end smoke commands produce schema-valid JSON with positive `ticks_per_s` and `tick_latency_ns.p50`

## Task Commits

| Task | Description                                                              | Commit    |
| ---- | ------------------------------------------------------------------------ | --------- |
| 1    | Workspace dep crossbeam-channel + ScenarioInfo.unit additive field       | `f0bbbf8` |
| 2    | SPMC/MPSC/MPMC channel scenarios (SCEN-03/04/05) in `channels.rs`        | `5edc985` |
| 3    | Contention scenario (SCEN-08) in `contention.rs`                         | `a16731d` |
| 4    | MemBound scenario (SCEN-07) with two modes in `mem_bound.rs`             | `9b56538` |
| 5    | ReallocStorm scenario (SCEN-10) in `realloc_storm.rs`                    | `5b61722` |
| 6    | 6 new CLI subcommands + run_<name> dispatchers + drive_and_emit helper   | `0e717c8` |
| 7    | cargo fmt cleanup; full lint + test suite green                          | `61430c5` |

## Files Created

- `crates/alloc-bench-core/src/scenarios/channels.rs` — 473 lines, 11 unit tests
- `crates/alloc-bench-core/src/scenarios/contention.rs` — 173 lines, 6 unit tests
- `crates/alloc-bench-core/src/scenarios/mem_bound.rs` — 244 lines, 8 unit tests
- `crates/alloc-bench-core/src/scenarios/realloc_storm.rs` — 143 lines, 5 unit tests

## Files Modified

- `Cargo.toml` — added `crossbeam-channel = "0.5"` to `[workspace.dependencies]`
- `crates/alloc-bench-core/Cargo.toml` — added `crossbeam-channel = { workspace = true }`
- `crates/alloc-bench-core/src/output.rs` — `ScenarioInfo.unit: Option<String>` with `skip_serializing_if = "Option::is_none"`
- `crates/alloc-bench-core/src/scenarios/mod.rs` — `pub mod` + `pub use` for 4 new scenario modules
- `crates/alloc-bench-cli/src/main.rs` — 6 new `Cmd::*` variants + 6 new dispatch arms
- `crates/alloc-bench-cli/src/run.rs` — `drive_and_emit()` helper, `run_multithread` refactored to use it, 6 new `run_<name>` functions

## Smoke Test Results (Task 7)

All 7 commands emit schema-valid JSON in `/tmp/p2-01-smoke/`:

| Command                                                              | scenario.name | unit            | ticks_per_s | p50 (ns) |
| -------------------------------------------------------------------- | ------------- | --------------- | ----------- | -------- |
| `spmc --producers 1 --consumers 2 --capacity 64 --objects-per-tick 100` | spmc       | iters_per_s     | 22681       | 42559    |
| `mpsc --producers 2 --consumers 1 --capacity 64 --objects-per-tick 100` | mpsc       | iters_per_s     | 29603       | 32335    |
| `mpmc --producers 2 --consumers 2 --capacity 64 --objects-per-tick 100` | mpmc       | iters_per_s     | 18401       | 52799    |
| `contention --threads 4 --alloc-size 64 --iters-per-tick 500`           | contention | (absent)        | 22070       | 43135    |
| `mem-bound --mode linked-list --size 1`                                 | mem-bound  | (absent)        | 2352        | 418047   |
| `mem-bound --mode strided-array --size 1`                               | mem-bound  | (absent)        | 2723        | 356607   |
| `realloc-storm --target-size 4`                                         | realloc-storm | (absent)     | 632         | 1554431  |

`unit` field present (and exactly `"iters_per_s"`) for the three channel scenarios; absent (skip_serializing_if = None) for contention/mem-bound/realloc-storm.

## Test Suite Results

- `cargo fmt --all --check` → exit 0
- `cargo clippy --workspace --all-targets -- -D warnings` → exit 0
- `cargo test --workspace` → 41 core unit tests + 3 CLI unit tests + 1 Phase-1 multithread integration test, all green
- Phase-1 multithread JSON shape regression check: `cargo run -- multithread ... --output run.json` followed by `grep -c '"unit"' run.json` returns 0 — additive field truly invisible when None

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] cargo fmt collapsed dispatch arms after Task 6**

- **Found during:** Task 7 (`cargo fmt --all --check`)
- **Issue:** `run_realloc_storm` and `run_mem_bound` arms in `main.rs` had each argument on its own line; rustfmt's default `max_width = 100` collapses them to single-line calls because the argument list fits.
- **Fix:** Ran `cargo fmt --all` and committed the result as a separate `style(02-01)` commit (`61430c5`). No semantic change.
- **Files modified:** `crates/alloc-bench-cli/src/main.rs`

### Refactoring Beyond Plan

**2. [Beneficial Refactor] Extracted `drive_and_emit()` helper in run.rs**

- **Found during:** Task 6 (writing the 6th `run_<name>` and noticing the Build/Env/Run record assembly was about to be copy-pasted 6 more times alongside the 1 from Phase 1)
- **Action:** Introduced a private `drive_and_emit()` helper that takes a `&mut S: Scenario`, name, optional unit, harness config, and output path. Refactored `run_multithread` to use it as well so all 7 dispatchers share one assembly path.
- **Justification:** Plan §Task 6 said "follow the exact `run_multithread` template" but having 7 near-identical 30-line copies of the same Build/Env/Run construction would have been a maintenance liability and would have made the next 4 dispatchers (Phase 2 Plans 02 + 03) compound the problem. The helper preserves identical behaviour (verified by Phase-1 multithread integration test still passing) and reduces each new dispatcher to ~25 lines of scenario-specific logic.
- **Files affected:** `crates/alloc-bench-cli/src/run.rs`

### Auth Gates

None.

## Self-Check: PASSED

Created files:
- `/Users/marc.carre/src/rust-benchmark-glibc-musl-mimalloc/.claude/worktrees/agent-acadb049cdcc7dc23/crates/alloc-bench-core/src/scenarios/channels.rs` — FOUND
- `/Users/marc.carre/src/rust-benchmark-glibc-musl-mimalloc/.claude/worktrees/agent-acadb049cdcc7dc23/crates/alloc-bench-core/src/scenarios/contention.rs` — FOUND
- `/Users/marc.carre/src/rust-benchmark-glibc-musl-mimalloc/.claude/worktrees/agent-acadb049cdcc7dc23/crates/alloc-bench-core/src/scenarios/mem_bound.rs` — FOUND
- `/Users/marc.carre/src/rust-benchmark-glibc-musl-mimalloc/.claude/worktrees/agent-acadb049cdcc7dc23/crates/alloc-bench-core/src/scenarios/realloc_storm.rs` — FOUND

Commits:
- `f0bbbf8` — FOUND
- `5edc985` — FOUND
- `a16731d` — FOUND
- `9b56538` — FOUND
- `5b61722` — FOUND
- `0e717c8` — FOUND
- `61430c5` — FOUND

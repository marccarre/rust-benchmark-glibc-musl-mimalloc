# Phase 1: Foundation MVP Slice - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-17
**Phase:** 1-Foundation MVP Slice
**Areas discussed:** Workspace shape, Harness API, results.json schema versioning, Mutual-exclusion enforcement, First-allocator-combo choice, Build-metadata source, Linux-only stubs vs cross-platform now
**Mode:** `--auto` (single-pass, recommended-option selection)

---

## Workspace shape

| Option | Description | Selected |
|--------|-------------|----------|
| Three crates: core (lib) + cli (bin) + aggregator (bin placeholder) | Library holds harness/scenarios/metrics; CLI holds allocator features; aggregator is a Phase-4 placeholder in Phase 1 | ✓ |
| Single binary crate with everything inline | Smaller surface, but Phase 4 aggregator would have to bolt on later | |
| Multi-crate per scenario | Awkward dependency graph — harness/metrics shared by every crate | |

**User's choice:** Auto-selected option 1 (recommended).
**Notes:** Aligns with `.planning/research/ARCHITECTURE.md §1`.

---

## Harness API

| Option | Description | Selected |
|--------|-------------|----------|
| Trait `Scenario { setup, tick → Box<dyn SinkValue>, teardown, config_json }` | Harness owns black_box discipline; scenarios stay simple | ✓ |
| Closure-based: `harness.run(\|\| { ... })` | Less ceremony but harder to share state between iterations | |
| Async-first: `tick()` returns `impl Future` | Phase 2 web bench needs async; sync is simpler in Phase 1 | |

**User's choice:** Auto-selected option 1.
**Notes:** Sync trait keeps Phase 1 minimal; web bench (Phase 2) wraps its async runtime inside a sync `tick()`.

---

## results.json schema versioning

| Option | Description | Selected |
|--------|-------------|----------|
| Lock v1 schema in Phase 1 | `schema_version: 1` mandatory; downstream aggregator validates | ✓ |
| Defer schema until Phase 4 (aggregator) | Risk of churn if Phase 2 scenarios discover new fields are needed | |
| Versioned per-scenario subschemas | Over-engineered for greenfield | |

**User's choice:** Auto-selected option 1.
**Notes:** ARCHITECTURE.md §3 already specifies the schema; locking it now anchors the contract.

---

## Mutual-exclusion enforcement (alloc-jemalloc + alloc-mimalloc)

| Option | Description | Selected |
|--------|-------------|----------|
| `compile_error!` macro at compile time | Strictly stronger than runtime panic — catches the mistake before linking | ✓ |
| Runtime panic on startup | Matches roadmap success criterion text literally | |
| Allow both, alphabetic precedence | Confusing; worst of both worlds | |

**User's choice:** Auto-selected option 1, with a runtime panic kept as defense-in-depth so the success-criterion wording stays literally satisfied.
**Notes:** Plan-phase will write both layers in `crates/alloc-bench-cli/src/allocator.rs`.

---

## First-allocator-combo choice (Phase 1 only)

| Option | Description | Selected |
|--------|-------------|----------|
| glibc-jemalloc on x86_64-unknown-linux-gnu | Most mature stats API, no musl quirks, fastest path to a populated `results.json` | ✓ |
| glibc-ptmalloc (system default) | Easiest build but `allocator_stats` block is sparse — exercises HARN-05/HARN-06 less | |
| musl-mallocng | Adds musl-specific compile-time concerns (PITFALLS.md §2.1, §2.3) into Phase 1 | |
| glibc-mimalloc | Comparable rigor to jemalloc but less mature stats API in Rust | |

**User's choice:** Auto-selected option 1.
**Notes:** Phase 1 only needs ONE combo. Other combos arrive in Phase 3 Docker matrix.

---

## Build-metadata source

| Option | Description | Selected |
|--------|-------------|----------|
| `vergen` 9 with `["build", "cargo", "git", "rustc"]` features | Out-of-the-box rustc + target + git SHA + timestamp; ~10-line `build.rs` | ✓ |
| Hand-rolled `build.rs` calling `rustc --version` etc | More custom code, easier to drift | |
| `built` crate | Older alternative; less active maintenance vs vergen | |

**User's choice:** Auto-selected option 1.
**Notes:** Add a small custom block to capture `RUSTFLAGS` (vergen doesn't), per D-07.

---

## Linux-only Phase 1 vs cross-platform now

| Option | Description | Selected |
|--------|-------------|----------|
| Linux-only harness (panics on macOS) | Phase 3 brings macOS baseline; Phase 1 stays minimal | ✓ |
| Cross-platform stubs (macOS returns 0 for ru_maxrss etc.) | Pollutes the harness with platform-specific code paths in foundation phase | |
| macOS-first dev loop, Linux later | Inverts the actual user goal (Linux is the target environment) | |

**User's choice:** Auto-selected option 1.
**Notes:** Plan-phase will add `cfg!(not(target_os = "linux"))` panic at `main()` with a clear message.

---

## Claude's Discretion

- `crossbeam::scope` vs `std::thread::scope` — equivalent for our needs; plan-phase decides on the dependency-footprint criterion.
- Layout of `harness.rs` (single file vs split into `warmup.rs` / `measure.rs` / `metrics.rs`) — plan-phase chooses.
- Whether `Scenario::tick` is `async` — recommended sync; flagged for confirmation in plan-phase.

## Deferred Ideas

- Other allocator combos (musl-mallocng, glibc-mimalloc, etc.) → Phase 3 Docker matrix
- Other scenarios (web, channels, cpu-bound, mem-bound, contention, fragmentation-soak, realloc-storm, run-all) → Phase 2
- `alloc-bench-aggregator` full implementation → Phase 4 (Phase 1 ships placeholder `main.rs`)
- macOS host baseline → Phase 3 (ORCH-02)
- Schema v2 → revisit only when an additive change is required
- `snmalloc` / `tcmalloc` / `rpmalloc` → v2 (REQUIREMENTS.md V2-01..03)
- NUMA pinning → Phase 3 Justfile recipe (PITFALLS.md §1.3)
- Multiple runs per cell with median + range → Phase 5 CI (PITFALLS.md §4.3, REPR-03)
- `mi_stats_get`-based structured mimalloc stats → revisit in Phase 2 once first mimalloc cell runs

---
phase: 06-foundations
plan: 01
subsystem: alloc-bench-aggregator
tags: [axes, registry, foundations, AXES-01, AXES-02]
dependency_graph:
  requires: []
  provides:
    - "crate::axes::MEASUREMENT_AXES (consumed by Phase 7 score.rs, Phase 9 polar.rs, Phase 10 markdown.rs)"
    - "crate::axes::Direction::{Higher,Lower}::arrow() -> char"
    - "crate::axes::AxisSpec (Copy + Clone + Debug)"
  affects:
    - "crates/alloc-bench-aggregator/src/main.rs (1-line `mod axes;` insertion)"
tech_stack:
  added: []
  patterns:
    - "pub const registry (compile-time, zero-runtime-cost) — the V12 lock"
    - "BTreeSet (not HashSet) in tests — CLAUDE.md byte-identical-iteration discipline"
    - "Hard-coded Unicode literals '\\u{2191}' / '\\u{2193}' (no unicode-arrows crate)"
key_files:
  created:
    - "crates/alloc-bench-aggregator/src/axes.rs"
  modified:
    - "crates/alloc-bench-aggregator/src/main.rs"
decisions:
  - "MEASUREMENT_AXES is a pub const array (NOT lazy_static / OnceCell / once_cell::Lazy) per 06-CONTEXT lock — compile-time only, zero runtime cost."
  - "AxisSpec carries no weight_hint field — V12-05 / V12-07 deferred; equal-weight composite scoring in Phase 7."
  - "Direction::arrow(self) is a const fn so downstream Phase 9/10 callers can use it in const-evaluated contexts (column-header glyphs, spider legends)."
  - "Tests use BTreeSet (not HashSet) for the uniqueness gate — applies CLAUDE.md byte-identical-iteration discipline even in tests."
  - "AxisSpec const-array entries written single-line to satisfy the plan's grep gate (`AxisSpec { key: \"...\"`) — alignment-padded for readability while keeping the text grep-friendly."
metrics:
  duration_minutes: ~3
  tasks_completed: 2
  files_changed: 2
  tests_added: 5
  tests_passing: 54
  completed: "2026-05-26"
---

# Phase 6 Plan 01: Direction-aware Axis Registry Summary

One-liner: Locked the 8-axis `MEASUREMENT_AXES` const registry + `Direction::arrow()` glyph helper in `crates/alloc-bench-aggregator/src/axes.rs`, ready for Phase 7's `score.rs`, Phase 9's `polar.rs`, and Phase 10's `markdown.rs` to consume.

## What Was Built

- **`crates/alloc-bench-aggregator/src/axes.rs` (NEW, 148 LOC):**
  - `pub enum Direction { Higher, Lower }` with derives `Debug, Clone, Copy, PartialEq, Eq`.
  - `impl Direction { pub const fn arrow(self) -> char }` → `'\u{2191}'` for `Higher`, `'\u{2193}'` for `Lower`.
  - `pub struct AxisSpec` — fields `key`, `label`, `direction`, `is_heuristic` (all `pub`); derives `Debug, Clone, Copy`. NO `weight_hint` field per V12 deferral.
  - `pub const MEASUREMENT_AXES: [AxisSpec; 8]` — alphabetical key order: `channel_throughput`, `cpu_bound_throughput`, `image_size_efficiency`, `memory_fragmentation`, `multithread_throughput`, `resilience`, `security_posture`, `web_throughput`.
  - 5 `#[cfg(test)]` unit tests with the names required by the plan: `axes_count_is_exactly_eight`, `axes_keys_are_alphabetically_sorted`, `axes_keys_are_unique` (uses `BTreeSet`), `arrow_glyphs_match_unicode_literals`, `heuristic_axes_are_image_size_and_security`.

- **`crates/alloc-bench-aggregator/src/main.rs` (MODIFIED, +1 line):**
  - One-line insertion of `mod axes;` immediately before `mod diagrams;` at the top of the mod block. Strict alphabetical ordering: `axes`, `diagrams`, `html`, `loader`, `markdown`, `multi_run`, `recommend`. No other lines touched (Plan 02 owns the `--security` flag changes elsewhere in `main.rs`).

## Verification Results

| Gate | Command | Result |
| ---- | ------- | ------ |
| 5 axes tests pass | `cargo test -p alloc-bench-aggregator axes::` | `5 passed; 0 failed` |
| Pre-existing tests still pass | `cargo test -p alloc-bench-aggregator` | `54 passed; 0 failed` (49 prior + 5 new unit) plus `28 passed; 0 failed` integration |
| Crate compiles cleanly | `cargo build -p alloc-bench-aggregator` | Exit 0, no `error[E…]`, no `warning: unused import` lines (the new module is imported correctly via `mod axes;`) |
| Workspace build clean | `cargo build --workspace` | Exit 0, finished in 2.38s |
| File count check | `git diff --stat HEAD` + `git status --short` | exactly 2 files touched: `axes.rs` (new), `main.rs` (+1) |
| AxisSpec entries grep | `grep -cE "AxisSpec \{ key: \"" axes.rs` | 8 |
| `lazy_static`/`OnceCell` grep (excluding comments) | `grep -v '^//' axes.rs \| grep -c "lazy_static\|OnceCell\|once_cell"` | 0 |
| `weight_hint` grep (excluding comments) | `grep -v '^//' axes.rs \| grep -c "weight_hint"` | 0 |
| Higher arrow literal | `grep -c "'\\\\u{2191}'"` | 3 (impl + 2 tests) |
| Lower arrow literal | `grep -c "'\\\\u{2193}'"` | 3 (impl + 2 tests) |

The axes.rs file generates `dead_code` warnings (`Direction`, `AxisSpec`, `AxisSpec.label`, `AxisSpec.direction`, `MEASUREMENT_AXES` are defined but never referenced). This is expected and intentional — the plan's `<objective>` explicitly states "No consumers exist in Phase 6 — Phase 7's score.rs, Phase 9's polar.rs, and Phase 10's markdown.rs will consume this registry." The plan's compile gate filters specifically for `error[E…]` and `warning: unused`, not `dead_code`. The compiler note confirms: "`AxisSpec` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis."

## Confirmation: Decorate-Not-Rewrite Discipline

No mutation to `crates/alloc-bench-core/src/output.rs` (the locked v1 input schema). The new registry lives entirely in the aggregator crate as decoration metadata. This is consistent with Phase 5's `image_size_mb` sidecar pattern (D-13) — measurement axes are aggregator-side metadata, not bench-runner output.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Format mismatch] Reformatted MEASUREMENT_AXES initialiser entries to single-line literals**
- **Found during:** Task 1 verification
- **Issue:** The plan's `<acceptance_criteria>` regex `grep -cE "AxisSpec \{ key: \""` requires single-line `AxisSpec { key: "...", ... }` entries on each line. My initial format used multi-line struct literals (one field per line, conventional Rust), which produced 0 matches against the plan-exact regex.
- **Fix:** Reformatted each of the 8 `AxisSpec` entries to a single line with whitespace alignment for readability. Re-ran tests — all 5 still pass. The grep now returns 8.
- **Files modified:** `crates/alloc-bench-aggregator/src/axes.rs` (lines 67-77, no semantic change).
- **Commit:** Folded into the implementation commit.

No Rule-1 bugs, no Rule-2 missing critical functionality, no Rule-4 architectural decisions — the plan was tightly specified.

## Authentication Gates

None — pure compile-time addition with no I/O / network / secrets.

## Threat Surface Scan

No new security-relevant surface. The `<threat_model>` block in the plan correctly classifies this plan as a pure compile-time addition (T-06-01-T mitigated by 5 unit tests; T-06-01-I and T-06-01-D accepted as not-applicable).

## Conventional-commit Message Used

```
feat(06-01): add MEASUREMENT_AXES registry + Direction enum (AXES-01, AXES-02)

- New file `crates/alloc-bench-aggregator/src/axes.rs` (148 LOC) with:
  - `pub const MEASUREMENT_AXES: [AxisSpec; 8]` — alphabetical key order
  - `Direction::{Higher,Lower}` enum + `pub const fn arrow(self) -> char`
  - 5 unit tests gating length, alphabetical order, uniqueness,
    heuristic-flag set, and arrow glyphs (mitigation T-06-01-T).
- `crates/alloc-bench-aggregator/src/main.rs` — insert `mod axes;`
  alphabetically before `mod diagrams;` (no other lines touched).
- No new dependencies, no `output.rs` mutation, no `weight_hint` field
  (V12-05 / V12-07 deferred). Const-only registry per CONTEXT lock.
```

## Self-Check: PASSED

- File `crates/alloc-bench-aggregator/src/axes.rs` exists (148 LOC, includes 5 unit tests).
- File `crates/alloc-bench-aggregator/src/main.rs` modified with the single-line `mod axes;` insertion.
- All 5 new unit tests pass (`cargo test -p alloc-bench-aggregator axes::` → `5 passed; 0 failed`).
- All pre-existing tests pass (`cargo test -p alloc-bench-aggregator` → `54 passed; 0 failed` unit + `28 passed; 0 failed` integration).
- No new dependencies (Cargo.toml / Cargo.lock untouched).
- `git diff --stat` confirms exactly 2 files touched.

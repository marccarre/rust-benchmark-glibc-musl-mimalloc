---
phase: 09-spider-chart
plan: 02
subsystem: alloc-bench-aggregator
tags: [pareto-front, spider-chart, recommend, score, data-foundations]
requires:
  - score::CellScore (Phase 8 / Plan 02)
  - recommend::CellRecommendation (Phase 8 / Plan 02)
  - meta sidecar image_size_mb (Phase 5 / D-13)
provides:
  - score::pareto_front sibling fn (O(n²) strict-dominance sweep, BTreeSet output)
  - CellRecommendation::is_pareto bool field
  - top_n_cells widened signature (3rd arg: &BTreeMap<String, f64> image_sizes)
affects:
  - 09-03 (consumes is_pareto in HTML/REPORT.md emitters; supplies real image_sizes BTreeMap from metas)
tech-stack:
  added: []
  patterns: [btreemap-btreeset-discipline, decorate-not-rewrite, tdd-red-green-discipline]
key-files:
  created: []
  modified:
    - crates/alloc-bench-aggregator/src/score.rs
    - crates/alloc-bench-aggregator/src/recommend.rs
    - crates/alloc-bench-aggregator/src/main.rs
    - crates/alloc-bench-aggregator/src/markdown.rs
    - crates/alloc-bench-aggregator/src/html.rs
key-decisions:
  - O(n²) strict-dominance sweep on (composite_score↑, image_size_mb↓) — not Kung et al.; n is bounded (~18 cells × 6 envs ≤ 108) so simplicity beats asymptotic optimality.
  - Pareto computation runs on the TRUNCATED top_n_cells output (after rank-cutoff), per CONTEXT.md "Pareto-front data flow" — keeps the front semantically tied to "cells the reader will see."
  - macOS-host cells (env absent from image_sizes BTreeMap) are excluded as BOTH result members AND dominators — they neither sit on nor invalidate the front.
  - BTreeSet<(String, String)> as the membership return shape — gives byte-identical alphabetical iteration when the consumer Plan 09-03 emits the Pareto column.
  - Empty-input / empty-image_sizes degenerate cases return empty BTreeSet → all cells decorate `is_pareto: false` (preserves v1.0 byte-identical output until 09-03 wires real data).
requirements-completed: [POLAR-05]
metrics:
  duration: 8 min
  started: 2026-05-27T21:37:13Z
  completed: 2026-05-27T21:45:27Z
  files-modified: 5
  files-created: 0
  lines-added: 392
  lines-removed: 17
  task-commits: 4
---

# Phase 09 Plan 02: Pareto-Front Data Foundations Summary

**One-liner:** Land `score::pareto_front` (O(n²) strict-dominance sweep on composite_score↑ × image_size_mb↓) plus `CellRecommendation::is_pareto` decoration so Plan 09-03 can render the spider-chart Pareto-front overlay column without reshaping core data structures.

## Why This Plan

Plan 09-03 needs to emit a "Pareto" column in REPORT.md and a Pareto-front overlay trace on the spider chart. Both surfaces must agree on which cells are on the front (WR-01 cross-surface drift defense). Computing dominance once, at recommend-time, and decorating each `CellRecommendation` with a single bool, gives both writers the same data and locks the algorithm to a single test surface (`score.rs`).

Decorate-not-rewrite per CLAUDE.md: zero changes to the bench-runner output shape (`alloc-bench-core/src/output.rs` v1 schema stays locked); the new field rides on the aggregator-internal `CellRecommendation` only.

## Tasks Executed

### Task 1 — `score::pareto_front` sibling fn (TDD)

- **RED commit `b820b01`** — `test(09-02): add failing tests for score::pareto_front`. Added 6 tests covering: strict dominance, non-dominated pair both retained, macOS-host (no image_size) exclusion, strict-equality non-dominance, empty-input degenerate case, BTreeSet alphabetical-iteration property.
- **GREEN commit `1a9c922`** — `feat(09-02): implement score::pareto_front sibling fn`. O(n²) sweep returns `BTreeSet<(String, String)>`. Imports widened to `BTreeMap, BTreeSet, HashMap`. Tests pass; existing 15 score::tests untouched (21 total).

### Task 2 — `CellRecommendation::is_pareto` + widened `top_n_cells` signature (TDD)

- **RED commit `a61401c`** — `test(09-02): add failing tests for is_pareto field + top_n_cells signature`. Added 4 tests covering: field exists on struct, populated for Pareto cells, false-everywhere for empty image_sizes, signature accepts `&BTreeMap<String, f64>` 3rd arg.
- **GREEN commit `b56257a`** — `feat(09-02): add CellRecommendation::is_pareto + widen top_n_cells signature`. Field appended to struct. `top_n_cells` 3rd arg added; computes `pareto_set` on the truncated top-N (per CONTEXT.md). All 4 existing recommend::tests call sites updated via `replace_all` (2-arg → 3-arg). Tests pass; 25 recommend::tests total (21 pre-existing + 4 new).

## Verification

- `cargo test -p alloc-bench-aggregator score::tests` — 21/21 pass (15 pre-existing + 6 new)
- `cargo test -p alloc-bench-aggregator recommend::tests` — 25/25 pass (21 pre-existing + 4 new)
- `cargo build --workspace` — clean
- All 4 task commits emitted on `worktree-agent-ac02f0dbda541acb3`; HEAD-safety + cwd-drift + path-containment guards passed for each commit

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking issue] Bin-only crate: tests require full binary compile, so widening `top_n_cells` broke 4 consumer callsites**

- **Found during:** Task 2 GREEN — `cargo test recommend::tests::` failed because the aggregator crate has no `[lib]` target in `Cargo.toml` (it's `[[bin]]`-only). The plan's acceptance criteria implicitly assumed `cargo test --lib` would compile only the test surface, but in a bin-only crate `cargo test` must compile the full binary.
- **Fix:** Added minimal forward-compatible stubs at the 4 consumer callsites so the binary continues to compile:
  - `src/main.rs` — replaced `top_n_cells(cell_scores, &outcome.runs)` with an empty `BTreeMap<String, f64>` stub and 3-arg call. Comment explicitly references "Phase 9 / POLAR-05 stub: empty image_sizes here; Plan 09-03 wires the real BTreeMap derived from metas."
  - `src/markdown.rs` — added `is_pareto: false,` to the `make_cell` test fixture literal.
  - `src/html.rs` — added `is_pareto: false,` to 3 `CellRecommendation` test/sentinel literals (lines ~564, ~633, ~820), each with an explanatory comment.
- **Why this is safe:** Empty `BTreeMap` → `pareto_front` returns empty `BTreeSet` → all cells decorate `is_pareto: false` → zero rendering changes occur in REPORT.md or index.html. v1.0 byte-identical output is preserved until Plan 09-03 wires real `image_sizes` derivation AND adds the Pareto column row emitter (atomic Wave-2 swap).
- **Commits:** Folded into `b56257a` (Task 2 GREEN). Documented in the commit message.
- **Files modified:** `src/main.rs`, `src/markdown.rs`, `src/html.rs`.

## Wave-1 → Wave-2 Contract Handed to Plan 09-03

Plan 09-03 (Wave 2) inherits the following invariants:

1. `score::pareto_front(&[CellScore], &BTreeMap<String, f64>) -> BTreeSet<(String, String)>` — stable signature, fully tested.
2. `CellRecommendation::is_pareto: bool` — stable field, populated by `recommend::top_n_cells`.
3. `top_n_cells(scores, runs, image_sizes)` — 3-arg signature; pass real `BTreeMap` derived from `metas` (currently a stub empty BTreeMap in `main.rs`).
4. Pareto membership is computed on the **truncated** top-N (rank ≤ TOP_N_TOTAL), not on all candidate cells — Plan 09-03's column emitter must decorate cells using the existing `is_pareto` field rather than recomputing.
5. macOS-host cells (env absent from `image_sizes`) decorate `is_pareto: false` and never participate in dominance — Plan 09-03's column emitter renders them as the same em-dash convention as missing-image cells.

## Files Touched

| File | Change | Why |
|------|--------|-----|
| `crates/alloc-bench-aggregator/src/score.rs` | + `pareto_front` fn + 6 tests | Task 1 — primary algorithm + test surface |
| `crates/alloc-bench-aggregator/src/recommend.rs` | + `is_pareto` field, widened `top_n_cells` sig, + 4 tests, ~4 existing tests updated | Task 2 — decoration + signature widening |
| `crates/alloc-bench-aggregator/src/main.rs` | empty `BTreeMap` stub at consumer callsite | Rule 3 — bin-only crate compile fix |
| `crates/alloc-bench-aggregator/src/markdown.rs` | `is_pareto: false` on test fixture | Rule 3 — bin-only crate compile fix |
| `crates/alloc-bench-aggregator/src/html.rs` | `is_pareto: false` on 3 test fixtures | Rule 3 — bin-only crate compile fix |

## Self-Check: PASSED

- `crates/alloc-bench-aggregator/src/score.rs` — modified (verified via `git log --oneline 1a9c922 -- crates/alloc-bench-aggregator/src/score.rs`)
- `crates/alloc-bench-aggregator/src/recommend.rs` — modified (verified via `git log --oneline b56257a -- crates/alloc-bench-aggregator/src/recommend.rs`)
- `crates/alloc-bench-aggregator/src/main.rs` — modified (verified)
- `crates/alloc-bench-aggregator/src/markdown.rs` — modified (verified)
- `crates/alloc-bench-aggregator/src/html.rs` — modified (verified)
- Commit `b820b01` (test RED Task 1) — exists
- Commit `1a9c922` (feat GREEN Task 1) — exists
- Commit `a61401c` (test RED Task 2) — exists
- Commit `b56257a` (feat GREEN Task 2) — exists

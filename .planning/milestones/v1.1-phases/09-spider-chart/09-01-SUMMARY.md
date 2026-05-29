---
phase: 09-spider-chart
plan: 01
subsystem: aggregator-rendering
tags: [polar, scatterpolar, plotly, serde_json, btreemap, spider-chart, polar-trace]

# Dependency graph
requires:
  - phase: 06-foundations
    provides: "axes::AxisSpec + axes::MEASUREMENT_AXES (frozen) + axes::Direction"
  - phase: 07-scoring-top-n
    provides: "score::CellScore + score::score_cells + score::top_n"
provides:
  - "polar::build_trace — scatterpolar trace JSON for one CellScore (POLAR-01 + POLAR-02)"
  - "polar::build_reference_trace — matrix-mean reference polygon at 25% fill alpha (POLAR-04)"
  - "polar::axis_label_for_chart — render-time `(heuristic)` suffix on the two heuristic axes (POLAR-03)"
  - "main.rs `mod polar;` declaration alphabetical (between mod multi_run; and mod recommend;)"
affects: [09-02-pareto, 09-03-html-wiring, 10-direction-markers]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "polar.rs: serde_json::json! macro for trace construction (no typed struct, mirrors html.rs convention)"
    - "Module-isolation: polar.rs imports only serde_json + crate::axes + crate::score (no html/markdown/recommend/loader)"
    - "MEASUREMENT_AXES constant-order iteration via .iter() — preserves Phase 7 summation-order discipline; index 8 closes the polygon by repeating index 0"
    - "Render-time decoration: `(heuristic)` suffix appended at chart-emit time; Phase 6's MEASUREMENT_AXES registry stays frozen"

key-files:
  created:
    - "crates/alloc-bench-aggregator/src/polar.rs"
  modified:
    - "crates/alloc-bench-aggregator/src/main.rs (mod polar; alphabetical)"

key-decisions:
  - "Empty-input fallback for build_reference_trace: 9-element zero r array (degenerate dot at origin) — defensive no-panic guard, never silently truncates"
  - "Hard-coded 'Matrix mean (n=18)' literal (not interpolated from scores.len()) — matrix size locked by CLAUDE.md cross-libc rejection; UI copywriting stays byte-stable"
  - "axis_label_for_chart kept as pub fn (not pub(crate)) for Phase 10 reuse per CONTEXT.md Claude's Discretion"
  - "Test mod imports MEASUREMENT_AXES via `super::*` (no separate `use crate::axes::MEASUREMENT_AXES`) once Task 2 lifted it into the top-level use block"

patterns-established:
  - "Pattern: scatterpolar trace builders construct r/theta as 8-element Vecs from MEASUREMENT_AXES.iter(), then push index 0 onto each to produce 9-element polygon-closed arrays"
  - "Pattern: build_*_trace functions return serde_json::Value directly (consumed by tinytemplate via to_string at the html.rs layer)"
  - "Pattern: synth_score(alloc, env, [f64; 8]) test helper builds a CellScore with all 8 axis keys keyed alphabetically — mirrors score::tests::synth_cell_axes_keyed"

requirements-completed: [POLAR-01, POLAR-02, POLAR-03, POLAR-04]

# Metrics
duration: ~25min
completed: 2026-05-28
---

# Phase 09 Plan 01: polar.rs scatterpolar trace JSON builder Summary

**`crates/alloc-bench-aggregator/src/polar.rs` shipped — three pub fn (`build_trace`, `build_reference_trace`, `axis_label_for_chart`) plus 9 unit tests locking the 9-element polygon-closure invariant, the hard-coded `"Matrix mean (n=18)"` reference literal, and the 11-byte ` (heuristic)` suffix.**

## Performance

- **Duration:** ~25 min (from worktree spawn through final commit)
- **Started:** 2026-05-28T (worktree spawn)
- **Completed:** 2026-05-27T21:43:13Z (UTC at task-end record)
- **Tasks:** 2 / 2
- **Files created:** 1 (`crates/alloc-bench-aggregator/src/polar.rs`)
- **Files modified:** 1 (`crates/alloc-bench-aggregator/src/main.rs`)
- **Lines added (polar.rs):** ~398 (incl. tests + module-doc)

## Accomplishments

- Locked the POLAR-01 + POLAR-02 contract: every spider-chart trace now ships as a 9-element polygon-closed `r`/`theta` pair (`r[0] == r[8]`, `theta[0] == theta[8]`), `type == "scatterpolar"`, `fill == "toself"`. Plan 09-03's html.rs wiring will consume `build_trace` verbatim — no shape adjustments needed.
- Locked the POLAR-04 contract: matrix-mean reference polygon renders at 25% fill alpha (`rgba(128,128,128,0.25)`) and 50% stroke alpha (`rgba(128,128,128,0.5)`) with the hard-coded `"Matrix mean (n=18)"` name literal. The matrix size is locked by CLAUDE.md cross-libc rejection — interpolating `scores.len()` would silently break golden-fixture byte parity if a future cell were added or removed.
- Locked the POLAR-03 contract: `axis_label_for_chart(spec)` returns `format!("{} (heuristic)", spec.label)` for `image_size_efficiency` (index 2) and `security_posture` (index 6); plain `spec.label.to_string()` for the other six axes. Phase 6's `MEASUREMENT_AXES` registry is NOT mutated — the suffix is render-time decoration only.
- Module isolation maintained: `polar.rs` imports only `serde_json`, `crate::axes`, and `crate::score`. No imports from `html`, `markdown`, `recommend`, or `loader` (sibling-isolation contract per PATTERNS.md analog: `score.rs`).

## Task Commits

Each task was committed atomically:

1. **Task 1: polar.rs skeleton + axis_label_for_chart + heuristic-suffix tests** — `d502cd0` (feat)
2. **Task 2: build_trace + build_reference_trace + polygon-closure & alpha tests** — `954e42b` (feat)

## Files Created/Modified

- `crates/alloc-bench-aggregator/src/polar.rs` — NEW. Three pub fn (`build_trace`, `build_reference_trace`, `axis_label_for_chart`) + nine `#[cfg(test)] mod tests` tests covering POLAR-01..04. Module-doc cites all four requirement IDs and the locked invariants.
- `crates/alloc-bench-aggregator/src/main.rs` — MODIFIED. One-line addition: `mod polar;` inserted between `mod multi_run;` and `mod recommend;` (line 27, alphabetical position).

## Tests Added (9 total, all passing)

| # | Test name | Locks |
|---|-----------|-------|
| 1 | `axis_label_for_chart_appends_heuristic_suffix_for_image_size_efficiency_and_security_posture` | POLAR-03 positive — heuristic axes get suffix |
| 2 | `axis_label_for_chart_returns_plain_label_for_real_measurement_axes` | POLAR-03 negative — measured axes do NOT get suffix |
| 3 | `axis_label_for_chart_handles_all_eight_axes_in_constant_order` | POLAR-03 ordering safety net (bool-flip detection at indices 2 / 6) |
| 4 | `trace_closes_polygon_with_9_elements` | POLAR-01 + POLAR-02 verbatim — 9-element arrays + closure + scatterpolar type + toself fill |
| 5 | `trace_carries_alloc_env_name_field` | UI-SPEC `name: "{alloc}/{env}"` per-cell title |
| 6 | `trace_uses_axis_label_for_chart_for_theta` | POLAR-03 propagation — heuristic suffix appears in theta indices 2 / 6 |
| 7 | `reference_trace_carries_25_percent_alpha_fill_and_50_percent_alpha_stroke` | POLAR-04 — locked alpha + literal name + scatterpolar shape |
| 8 | `reference_trace_averages_each_axis_across_input_scores` | POLAR-04 — mean(0.0, 0.5, 1.0) = 0.5 across all 8 axes |
| 9 | `reference_trace_returns_zeros_when_input_empty` | Edge case — 9-element zero array, never panics on empty matrix |

**Verification command:** `cargo test -p alloc-bench-aggregator polar::tests::` → 9/9 pass. Full aggregator suite: 107/107 unit + 28/28 integration tests pass (no regressions). Release build: `cargo build -p alloc-bench-aggregator --release` succeeds.

## Key Literals (verifiable via `grep -F`)

- `"Matrix mean (n=18)"` — POLAR-04 reference-polygon name; 4 occurrences (3 in module doc + 1 in code).
- `" (heuristic)"` — POLAR-03 11-byte suffix (single leading U+0020 space); appears in `axis_label_for_chart` body and module-doc.
- `"rgba(128,128,128,0.25)"` — POLAR-04 25% fill alpha; module-doc + impl + test assertion.
- `"rgba(128,128,128,0.5)"` — POLAR-04 50% stroke alpha; module-doc + impl + test assertion.

## main.rs `mod polar;` Block (alphabetical confirmation)

```
26: mod multi_run;
27: mod polar;       <-- inserted in alphabetical position
28: mod recommend;
```

## Decisions Made

- **Empty-input fallback for `build_reference_trace`:** returns 9-element zero `r` array (degenerate dot at origin). Defensive no-panic guard — Phase 9 Plan 03's `html.rs` wiring will always pass a non-empty `&[CellScore]`, but the empty path is exercised by `reference_trace_returns_zeros_when_input_empty` to lock the no-panic guarantee.
- **`"Matrix mean (n=18)"` is hard-coded** (not interpolated via `scores.len()`). Per CLAUDE.md the matrix is locked at 18 cells; interpolating `scores.len()` would silently destabilize golden-fixture output if Phase 11's regen sees a different cell count.
- **`axis_label_for_chart` is `pub fn`** (not `pub(crate)`). Phase 10 will consume it for direction-marker integration in column headers — exporting it now avoids a follow-up visibility flip.
- **Test-helper `synth_score`** mirrors `score::tests::synth_cell_axes_keyed` shape: alphabetically-ordered `[f64; 8]` keyed via `MEASUREMENT_AXES.iter().zip(vals.iter())`. Same convention used downstream in Plans 09-02 / 09-03 for spider-trace fixtures.

## Deviations from Plan

None — plan executed exactly as written.

The plan's Task 1 `<action>` step suggested keeping the three pub fn signatures stubbed in Task 1 (only `axis_label_for_chart` implemented) with `serde_json` import deferred to Task 2. I followed that ordering: Task 1 commit imports only `crate::axes::AxisSpec` (no `serde_json`, no `BTreeMap`); Task 2 commit lifts the imports to include `serde_json::{json, Value}`, `crate::axes::MEASUREMENT_AXES`, `crate::score::CellScore`, and (in `mod tests`) `std::collections::BTreeMap`.

Pre-existing warnings about `axes::Direction::arrow` and `recommend::TOP_N_SPIDER` being unused are out of scope for this plan — they ship intentionally for Phase 10 / Phase 9 Plan 03 consumers.

## Issues Encountered

None — no blockers, no unexpected behavior, all 9 polar tests passed on first execution after each task's implementation.

A minor mid-Task-1 cleanup: the initial commit attempt left `MEASUREMENT_AXES` imported at the top level but unused outside tests, producing an `unused_imports` warning. Fix was to scope the import to `mod tests` (via `crate::axes::MEASUREMENT_AXES` directly) for Task 1, then lift it back to the module-level use block in Task 2 once `build_trace` consumed it. Net effect: zero new warnings introduced by Plan 09-01 once Task 2 lands; the only warnings on `cargo build -p alloc-bench-aggregator` are the three pre-existing Phase 6 / Phase 7 / Plan 09-01 unused-symbol warnings that resolve when downstream plans wire them in.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Plan 09-02 (Pareto-front):** `polar.rs` is complete and ready. Plan 09-02 adds `score::pareto_front` to identify (alloc, env) cells on the Pareto front of `composite_score` ↑ vs `image_size_mb` ↓; that work is independent of `polar.rs`.
- **Plan 09-03 (HTML wiring):** ready to consume `polar::build_trace` and `polar::build_reference_trace` verbatim. The trace JSON shape is locked by tests 4–9 above — `html.rs` only needs to call `serde_json::to_string(&trace).unwrap()` and inject the result into `index.html.tmpl` via tinytemplate. No further changes to `polar.rs` should be needed.
- **Phase 10 (Direction Markers):** ready to consume `polar::axis_label_for_chart`. Phase 10 will wrap that label with `axes::Direction::arrow()` to inject `↑` / `↓` glyphs; no API change to the existing `axis_label_for_chart` signature is required.

## Self-Check: PASSED

- FOUND: `crates/alloc-bench-aggregator/src/polar.rs`
- FOUND: `.planning/phases/09-spider-chart/09-01-SUMMARY.md`
- FOUND: `mod polar;` in `crates/alloc-bench-aggregator/src/main.rs`
- FOUND: commit `d502cd0` (Task 1: skeleton + axis_label_for_chart + 3 tests)
- FOUND: commit `954e42b` (Task 2: build_trace + build_reference_trace + 6 tests)

---
*Phase: 09-spider-chart*
*Plan: 01*
*Completed: 2026-05-28*

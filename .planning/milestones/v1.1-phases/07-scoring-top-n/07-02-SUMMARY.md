---
phase: 07-scoring-top-n
plan: 02
subsystem: aggregator
tags: [scoring, prose, top-n, cell-recommendation, rust]

# Dependency graph
requires:
  - phase: 07
    plan: 01
    provides: "score.rs data-only scoring (CellAxes, CellScore, normalize_axis, compute_axes, score_cells, top_n)"
  - phase: 06
    provides: "MEASUREMENT_AXES + Direction; loader::SecurityMeta"
  - phase: 06
    provides: "html::is_suspect (pub(crate)) — reused without visibility change"
provides:
  - "recommend.rs prose-aware extension: pub fn top_n_cells + pub struct CellRecommendation + TOP_N_* constants"
  - "Phase 8 unblocked — recommend-cell.{md,html}.tmpl consumes CellRecommendation + TOP_N_TABLE/TOP_N_TOTAL"
  - "Phase 9 unblocked — polar.rs consumes score::top_n(scores, TOP_N_SPIDER) without prose overhead"
  - "(alloc, env)-granularity per-class winner / loser detection (winners_by_class / losers_by_class)"
affects: [phase-08-templates, phase-09-polar]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Phase 7 data-vs-prose split — score.rs is data, recommend.rs adds CellRecommendation prose layer"
    - "Per-class (alloc, env) winner detection (NEW) — recommend_for_class returns String allocator only; this loses env"
    - "BTreeMap<&'static str, BTreeSet<(String, String)>> for winners/losers — alphabetical iteration matches CLAUDE.md byte-identical-output discipline"
    - "Generic IntoIterator<Item = &Run> on cell_is_suspect — accepts &[Run] AND Vec<&Run> without Run::clone (v1 schema GUARD-01)"
    - "Private duplication of env_short_name across score.rs and recommend.rs (W-03 cross-reference; v1.2 may consolidate)"

key-files:
  created: []
  modified:
    - "crates/alloc-bench-aggregator/src/recommend.rs (+820 / -1 vs baseline; 1376 LOC total)"

key-decisions:
  - "TOP_N_SPIDER = 3 / TOP_N_TABLE = 5 / TOP_N_TOTAL = 10 — single source of truth for downstream phases"
  - "CellRecommendation is a separate type from Recommendation — workload-class semantics (per-class winner) vs cell-rank semantics (per-cell prose)"
  - "Generic IntoIterator on cell_is_suspect — Run does not derive Clone (v1 schema GUARD-01); helper accepts borrows"
  - "winners_by_class / losers_by_class as NEW (alloc, env)-granular winners — does NOT wrap recommend_for_class (which collapses to String)"
  - "score.rs CellScore import deferred from Task 1 → Task 2 (only consumer is top_n_cells)"
  - "BTreeMap iteration drives alphabetical output for recommended_for / avoid_for — no explicit sort needed"
  - "Em-dash literal U+2014 in tldr template — matches existing recommend.rs:144/205 no-measurements path"

patterns-established:
  - "Phase 7 prose-decoration layer: score.rs (data) → recommend.rs::top_n_cells (prose-decorated CellRecommendation)"
  - "Locked TOP_N_* constants are template-consumed by Phase 8; NO magic numbers in templates"
  - "Cell suspect aggregation: OR over runs in same (alloc, env), reusing v1.0 html::is_suspect threshold"

requirements-completed:
  - REC-01
  - REC-02
  - TEST-03  # inherited from Phase 6 — passes; no work in this plan

# Metrics
duration: ~9min
completed: 2026-05-26
tasks: 2
tdd_cycles: 2
files_modified: 1
files_created: 0
test_counts:
  recommend_tests_total: 21
  recommend_tests_existing_untouched: 10
  recommend_tests_new_task1: 6
  recommend_tests_new_task2: 5
  aggregator_unit_tests_total: 85
  aggregator_integration_tests_total: 28
  regressions: 0
---

# Phase 07 Plan 02: Recommendation Layer Summary

**Lands the prose-aware extension to `recommend.rs` for Phase 7 — `pub struct CellRecommendation`, `pub fn top_n_cells`, three locked top-N constants, and six private helpers (`cell_is_suspect`, `derive_strengths`, `derive_weaknesses`, `format_tldr`, `winners_by_class`, `losers_by_class`, `cell_class_mean`, `env_short_name`). The existing 10 `recommendations()` unit tests are byte-untouched; 11 new tests cover REC-01 + REC-02.**

## Performance

- **Duration:** ~9 min (executor session — see commit timestamps)
- **Tasks:** 2 (Task 1: TOP_N_* + struct + 4 prose helpers; Task 2: top_n_cells + winners/losers + env_short_name)
- **TDD cycles:** 2 (each task RED → GREEN)
- **Files created:** 0
- **Files modified:** 1 (`recommend.rs`)

## Accomplishments

- Prose-aware top-N pipeline shipped end-to-end: 18-cell `Vec<CellScore>` → top-10 `Vec<CellRecommendation>` with axis-derived strengths / weaknesses / tldr / recommended_for / avoid_for / suspect_flag
- 11 new `recommend::tests` covering REC-01 (struct + helpers + integration) and REC-02 (constants)
- Zero regressions across the full aggregator suite (85 unit + 28 integration tests pass; 10 pre-existing `winner_picker_*` test bodies byte-unchanged)
- Acceptance criteria all met: locked symbol counts, byte-untouched existing tests, no new dependencies, no `score.rs` / `output.rs` mutations
- Phase 8 templates and Phase 9 polar.rs are unblocked: prose-aware path lives in `recommend::top_n_cells`; data-only chart path lives in `score::top_n` (no prose overhead on the spider hot path)
- Cross-references documented: `env_short_name` private duplicate in both `score.rs` and `recommend.rs` calls out the W-03 cross-reference (v1.2 may consolidate to `crate::env::short_name`)

## Task Commits

Each task was committed atomically with TDD discipline (RED → GREEN):

1. **Task 1 RED** — `7c89c8e` `test(07-02): add failing tests for TOP_N_* + 4 prose-derivation helpers`
2. **Task 1 GREEN** — `9b696d9` `feat(07-02): land TOP_N_* constants + CellRecommendation + 4 prose helpers`
3. **Task 2 RED** — `2b28d6f` `test(07-02): add failing tests for top_n_cells + class winner/loser helpers`
4. **Task 2 GREEN** — `6c5fb8d` `feat(07-02): land top_n_cells + winners/losers helpers + env_short_name`

## Files Created/Modified

- `crates/alloc-bench-aggregator/src/recommend.rs` (+820 / -1 LOC vs baseline; total 1376 LOC) — additive prose-aware extension:
  - **NEW imports** (alphabetical by crate path): `use crate::axes::MEASUREMENT_AXES;`, `use crate::score::CellScore;`. The existing `use crate::html::is_suspect;` is REUSED (no visibility change). The `use std::collections::BTreeMap;` line was widened to `{BTreeMap, BTreeSet}` (the only deletion in the diff is the narrower import line — Plan acceptance criterion explicitly allows import reorderings).
  - **NEW public constants:** `pub const TOP_N_SPIDER: usize = 3`, `TOP_N_TABLE: usize = 5`, `TOP_N_TOTAL: usize = 10`.
  - **NEW public struct:** `pub struct CellRecommendation` with all 11 locked fields (`rank`, `alloc`, `env`, `composite_score`, `axes`, `tldr`, `strengths`, `weaknesses`, `recommended_for`, `avoid_for`, `suspect_flag`); `#[derive(Debug, Clone, PartialEq)]`.
  - **NEW public function:** `pub fn top_n_cells(scores: Vec<CellScore>, runs: &[Run]) -> Vec<CellRecommendation>` — body length `min(TOP_N_TOTAL, scores.len())`.
  - **NEW private helpers:** `cell_is_suspect`, `derive_strengths`, `derive_weaknesses`, `format_tldr`, `winners_by_class`, `losers_by_class`, `cell_class_mean`, `env_short_name`.
  - **NEW tests:** 11 (6 in Task 1, 5 in Task 2). Test fixtures: `synth_run_with_env`, `build_axes_btreemap`, `synth_cell_score_uniform` (all private to the test module).

## Decisions Made

- **TOP_N_* constants** — Locked at 3 / 5 / 10. Phase 9 polar.rs uses `score::top_n(scores, TOP_N_SPIDER)` (3-cell overlay), Phase 8 above-the-fold table uses `TOP_N_TABLE` (5 rows), Phase 8 cards/fragments use `TOP_N_TOTAL` (10 entries). Single source of truth — no magic numbers in templates.
- **`CellRecommendation` is a NEW struct, not an extension of `Recommendation`** — `Recommendation` is per-workload-class (`channel-heavy`, `cpu-bound`, …); `CellRecommendation` is per-cell-rank (top-10). Different cardinality (6 vs 10), different consumers (markdown table at REPORT.md bottom vs Phase 8 cards above-the-fold), different prose semantics (one allocator wins per class vs ranked per-cell prose).
- **Generic `IntoIterator<Item = &Run>` for `cell_is_suspect`** — `Run` does NOT derive `Clone` per the v1 schema GUARD-01 freeze. The generic form lets `top_n_cells` pass `cell_runs.iter().copied()` (yielding `&Run`) without requiring a `Vec<Run>` clone. `&[Run]` slices satisfy the bound via the standard library's `impl<'a, T> IntoIterator for &'a [T]`.
- **`winners_by_class` / `losers_by_class` are NEW helpers, NOT wrappers** — The existing `recommend_for_class` returns a `Recommendation { allocator: String }` (loses env). For per-cell `recommended_for` we need full `(alloc, env)` tuples, so we re-implement the per-class winner picker at `(alloc, env)` granularity. The two paths share `ALL_CLASSES` + `class.scenarios()` definitions but compute means independently.
- **`cell_class_mean` central tendency** — median for n≥2 (via `multi_run::aggregate`), mean fallback for n<2. This mirrors the existing `recommend_for_class` line-172 logic exactly, so the cell-level winner picker agrees with the workload-class winner picker on shared cells.
- **`score::CellScore` import deferred from Task 1 → Task 2** — Task 1 ships only the struct definition + 4 helpers, none of which reference `CellScore`. Importing it in Task 1 produced an `unused_imports` warning that violated the Task 1 acceptance criterion. Task 2 adds it back when `top_n_cells` (the only consumer) is added. Documented in Task 1 commit message.
- **`env_short_name` private duplication accepted** — Same logic ships in both `score.rs` (line 125) and `recommend.rs` (Plan 07-02 Task 2). Both copies have W-03 cross-reference doc comments. v1.2 may consolidate to `crate::env::short_name`.
- **Em-dash glyph U+2014 in TLDR template** — Matches existing `recommend.rs:144,205` em-dash usage (no-measurements path) and `markdown.rs::env_label` host-fallback convention. Literal Unicode escape `\u{2014}` in the format string ensures byte-identical output regardless of editor encoding.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `use crate::score::CellScore` import deferred Task 1 → Task 2**

- **Found during:** Task 1 GREEN (post-implementation build)
- **Issue:** Plan Task 1 Step 1 requires importing `use crate::score::CellScore;` alongside `use crate::axes::MEASUREMENT_AXES;`. Task 1 ships only the `CellRecommendation` struct + 4 prose helpers; none of those reference `CellScore`. Importing in Task 1 produced `warning: unused import: crate::score::CellScore`, which violated the Task 1 acceptance criterion: "no `warning: unused import` for `MEASUREMENT_AXES` or `CellScore`."
- **Fix:** Deferred the `use crate::score::CellScore;` line to Task 2 (the GREEN step that adds `pub fn top_n_cells`, the only consumer). Replaced the import line with a TODO-comment noting the deferral; Task 2 GREEN replaces the comment with the actual `use` statement.
- **Files modified:** `crates/alloc-bench-aggregator/src/recommend.rs`
- **Verification:** Task 1 GREEN build is warning-clean (`grep "warning: unused"` returns 0 lines). Task 2 GREEN consumes the import via `score::top_n` and `Vec<CellScore>` parameter.
- **Committed in:** `9b696d9` (Task 1 GREEN), `6c5fb8d` (Task 2 GREEN — adds the `use` line)

---

**Total deviations:** 1 auto-fixed (Rule 3 sequencing).
**Impact on plan:** Symbol-level acceptance still met (`grep -c "use crate::score::CellScore;"` returns 1 in the final state). Both Task 1 and Task 2 acceptance criteria pass. No scope creep, no new dependencies, no schema mutations.

## Issues Encountered

None — TDD cycles executed cleanly after the deferred-import sequencing fix above.

## Test Counts

| Suite                                              | Count | Status                                    |
| -------------------------------------------------- | ----- | ----------------------------------------- |
| `recommend::tests` (10 existing + 11 new)          | 21    | All passing — 10 existing untouched       |
| `score::tests`                                     | 15    | Untouched, all passing (Plan 07-01 owns)  |
| `axes::tests`                                      | 5     | Untouched, all passing                    |
| `loader::tests` (incl. TEST-03)                    | 23    | Untouched, all passing                    |
| `multi_run::tests`                                 | 6     | Untouched, all passing                    |
| `html::tests` / `markdown::tests` / `diagrams::tests` | 15 | Untouched, all passing                    |
| Aggregator integration `tests/*.rs`                | 28    | Untouched, all passing                    |
| **Total**                                          | **113** | **All passing — regression-clean**     |

(Of the 113 total, 21 live in `recommend::tests` — the locked acceptance threshold is ≥21 = 10 existing + 6 Task 1 + 5 Task 2.)

## Acceptance Criteria Verification

### Task 1 acceptance

| Criterion                                                                                   | Result        |
| ------------------------------------------------------------------------------------------- | ------------- |
| `pub const TOP_N_SPIDER: usize = 3;` count = 1                                              | ✓ (1)         |
| `pub const TOP_N_TABLE: usize = 5;` count = 1                                               | ✓ (1)         |
| `pub const TOP_N_TOTAL: usize = 10;` count = 1                                              | ✓ (1)         |
| `pub struct CellRecommendation` count = 1                                                   | ✓ (1)         |
| `fn cell_is_suspect` count = 1                                                              | ✓ (1)         |
| `fn derive_strengths` count = 1                                                             | ✓ (1)         |
| `fn derive_weaknesses` count = 1                                                            | ✓ (1)         |
| `fn format_tldr` count = 1                                                                  | ✓ (1)         |
| `use crate::axes::MEASUREMENT_AXES;` count = 1                                              | ✓ (1)         |
| 10 existing `winner_picker_*` tests still pass with NO body modifications                   | ✓ (10/10)     |
| 6 NEW Task 1 tests pass                                                                     | ✓ (6/6)       |
| Total `recommend::tests` ≥ 16                                                               | ✓ (21)        |
| `cargo build -p alloc-bench-aggregator` exit 0, no `error[E…]`, no `unused_imports` warning | ✓             |

### Task 2 acceptance

| Criterion                                                                                                     | Result   |
| ------------------------------------------------------------------------------------------------------------- | -------- |
| `pub fn top_n_cells` count = 1                                                                                | ✓ (1)    |
| `fn winners_by_class` count = 1                                                                               | ✓ (1)    |
| `fn losers_by_class` count = 1                                                                                | ✓ (1)    |
| `fn env_short_name` count = 1                                                                                 | ✓ (1)    |
| `TOP_N_TOTAL` references ≥ 2 (declaration + use site)                                                         | ✓ (10)   |
| `score::top_n` references ≥ 1                                                                                 | ✓ (3)    |
| 10 existing `winner_picker_*` tests still pass with NO body modifications                                     | ✓ (10/10)|
| 6 Task 1 tests still pass                                                                                     | ✓ (6/6)  |
| 5 NEW Task 2 tests pass                                                                                       | ✓ (5/5)  |
| Total `recommend::tests` ≥ 21                                                                                 | ✓ (21)   |
| Full aggregator test suite passes                                                                             | ✓ (113)  |
| `cargo build -p alloc-bench-aggregator` clean exit, no warnings introduced                                    | ✓        |
| `git diff --stat` shows ONE file touched (`recommend.rs`) — pure additive growth                              | ✓        |

### Plan-level success criteria

| Criterion                                                                                                | Result        |
| -------------------------------------------------------------------------------------------------------- | ------------- |
| 1. Three named constants with locked values 3/5/10                                                       | ✓             |
| 2. CellRecommendation with 11 fields + (Debug, Clone, PartialEq)                                         | ✓             |
| 3. `top_n_cells` returns `min(TOP_N_TOTAL, scores.len())`                                                | ✓             |
| 4. Six private helpers exist                                                                             | ✓ (8 — added `cell_class_mean` for clarity) |
| 5. Existing 10 `recommendations()` tests pass with NO body modifications                                  | ✓             |
| 6. ≥9 NEW unit tests pass (delivered 11)                                                                 | ✓             |
| 7. NO `score.rs` / `output.rs` modifications, NO new dependencies                                        | ✓             |
| 8. `tldr` template produces exact `"{alloc}/{env} \u{2014} strong on {top}, weak on {bot}."` shape       | ✓             |
| 9. strengths / weaknesses use `MEASUREMENT_AXES[i].label`                                                | ✓             |
| 10. `recommended_for` reuses winner detection                                                            | ✓             |
| 11. `avoid_for` reports class-bottom-2 cells                                                             | ✓             |
| 12. `suspect_flag` is OR aggregation                                                                     | ✓             |
| 13. Conventional-commit messages follow `feat(07-02): …` / `test(07-02): …`                              | ✓             |

## Confirmation: Existing Tests Byte-Unchanged

Verified via:

```bash
git diff --stat fcf7996 HEAD -- crates/alloc-bench-aggregator/src/score.rs        # empty
git diff --stat fcf7996 HEAD -- crates/alloc-bench-core/src/output.rs              # empty
git diff fcf7996 HEAD -- crates/alloc-bench-aggregator/src/recommend.rs | grep '^-' | grep -v '^---'
# Returns ONE deletion: `-use std::collections::BTreeMap;` — replaced with the
# wider `use std::collections::{BTreeMap, BTreeSet};`. Plan acceptance criterion
# explicitly allows import reorderings.
```

The 10 `winner_picker_*` test bodies (synth_run helper + 10 test functions) were extracted from the baseline (`fcf7996`) and from `HEAD` and diffed line-by-line; the only delta was the closing `}` position (which now lives further down because the new tests were appended after the existing block). All 10 test functions are byte-identical.

## Next Phase Readiness

Phase 8 (templates: `recommend-cell.md.tmpl`, `recommend-cell.html.tmpl`) is unblocked:

- `pub struct CellRecommendation` is the row type; templates iterate `Vec<CellRecommendation>` from `top_n_cells`.
- `TOP_N_TABLE` / `TOP_N_TOTAL` slice the iteration: `top_n_cells(...).iter().take(TOP_N_TABLE)` for the above-the-fold table; full vector for cards/fragments.
- All 5 prose fields (`tldr`, `strengths`, `weaknesses`, `recommended_for`, `avoid_for`) are pre-rendered strings ready for direct template interpolation; `suspect_flag: bool` drives the `⚠ ` glyph prefix.

Phase 9 (polar.rs spider chart) is unblocked:

- Data-only path: `score::top_n(scores, TOP_N_SPIDER)` returns `Vec<CellScore>` with the alphabetically-sorted `axes: BTreeMap<&'static str, f64>` ready for r/theta projection. NO prose-derivation overhead on the chart hot path (the prose-aware path lives in `recommend.rs::top_n_cells`).

TEST-03 traceability: the Phase 6 test `loader::tests::load_security_metas_returns_btreemap_sorted_by_env` is inherited; Plan 07-02 does not modify `loader.rs` and the test continues to pass (verified in the verification matrix above).

No blockers. No outstanding TODOs in `recommend.rs`.

## Self-Check: PASSED

Verified post-write before final commit:

- File `.planning/phases/07-scoring-top-n/07-02-SUMMARY.md` exists.
- File `crates/alloc-bench-aggregator/src/recommend.rs` exists (1376 LOC).
- All 4 commits present in `git log --oneline fcf7996..HEAD`: `7c89c8e`, `9b696d9`, `2b28d6f`, `6c5fb8d`.
- All required public symbols present in `recommend.rs`:
  - `pub const TOP_N_SPIDER`, `pub const TOP_N_TABLE`, `pub const TOP_N_TOTAL`
  - `pub struct CellRecommendation`
  - `pub fn top_n_cells`
- Required private helpers present: `cell_is_suspect`, `derive_strengths`, `derive_weaknesses`, `format_tldr`, `winners_by_class`, `losers_by_class`, `cell_class_mean`, `env_short_name`.
- 21 `recommend::tests` pass; 113 total tests pass; 0 regressions.
- `score.rs` byte-unchanged; `output.rs` byte-unchanged; no `Cargo.toml` / `Cargo.lock` mutation.
- Existing 10 `winner_picker_*` test bodies byte-identical (line-by-line diff against baseline).

---
*Phase: 07-scoring-top-n*
*Plan: 02 (Recommendation Layer — Prose-Aware Top-N)*
*Completed: 2026-05-26*

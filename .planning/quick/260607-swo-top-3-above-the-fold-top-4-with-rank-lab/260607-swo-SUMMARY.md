---
phase: quick-260607-swo
plan: 01
subsystem: ui
tags: [aggregator, plotly, html, spider-chart, css-grid]

# Dependency graph
requires:
  - phase: v1.1 / Phase 9 / Plan 09-04
    provides: build_spider_context + small-multiples grid + spider-cell DOM ids
provides:
  - TOP_N_SPIDER bumped from 3 to 4 (single source of truth)
  - Plotly per-cell title.text rank-prefixed "#N  alloc/env" (two-space delimiter)
  - 2x2 CSS grid layout (`display: grid; grid-template-columns: repeat(2, 1fr)`)
    with `@media (max-width: 768px)` single-column reflow
  - Smoke + in-crate test pinning updated for top-4 contract
  - 4th committed fixture (mimalloc-wolfi.json) so the smoke test exercises
    the strict-4 path, not the partial-input fall-through
affects: [aggregator html rendering, spider-chart UI, future top-N changes]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "CSS Grid for small-multiples layouts (was: flex with calc-based widths)"
    - "Atomic test+fixture commits — pin and data move together so each commit is independently green"

key-files:
  created:
    - crates/alloc-bench-aggregator/tests/fixtures/mimalloc-wolfi.json
  modified:
    - crates/alloc-bench-aggregator/src/recommend.rs
    - crates/alloc-bench-aggregator/src/html.rs
    - crates/alloc-bench-aggregator/src/main.rs
    - crates/alloc-bench-aggregator/templates/index.html.tmpl
    - crates/alloc-bench-aggregator/tests/smoke.rs

key-decisions:
  - "Chose fixture-add (not assertion-relax) per fixture_guidance preference (1)"
  - "Combined Task 4 fixture+test commit atomic to keep every intermediate commit green (bisectability > literal commit-count separation)"
  - "Left in-crate test `spider_section_emits_three_spider_cell_divs` unchanged (it tests the partial-input fall-through path with 3 scores; smoke test enforces strict-4 contract end-to-end)"

patterns-established:
  - "CSS-grid replaces calc-based flex for fixed N-column small-multiples"
  - "Fixture authoring as Task-4 mitigation when smoke-test count assertions tighten"

requirements-completed:
  - QUICK-260607-SWO

# Metrics
duration: ~25min
completed: 2026-06-07
---

# Phase quick-260607-swo: Top-4 spider grid + #N rank labels Summary

**Top-3 → Top-4 spider promotion: rank-prefixed Plotly titles (`#N  alloc/env`), 2x2 CSS grid, mobile single-column reflow, 4th fixture so the smoke test exercises the strict-4 path.**

## Performance

- **Duration:** ~25 min (4 tasks, 4 commits + plan-metadata commit)
- **Started:** 2026-06-07 (worktree spawn)
- **Completed:** 2026-06-07
- **Tasks:** 4 / 4
- **Files modified:** 5 (1 created — `mimalloc-wolfi.json` fixture; 4 source/template edits)

## Accomplishments
- `TOP_N_SPIDER` flipped from 3 → 4 in a single source-of-truth constant; pinning test enforces the new value.
- Per-cell Plotly title now reads `#1  jemalloc/debian-slim` (rank prefix + two-space delimiter + `{alloc}/{env}` per the user-locked title format) — `idx + 1` was already in scope from the per-cell DOM-id assignment, so no additional state was threaded.
- `.spider-grid` is a `display: grid; grid-template-columns: repeat(2, 1fr)` 2x2 layout at desktop widths, collapsing to a single column under `@media (max-width: 768px)`. The pre-existing `flex: 1 1 calc(...)` rule on `.spider-cell` was dropped; `aspect-ratio: 1 / 1` and `min-width: 280px` are preserved.
- Section heading + caption + CSS block-comment + inline script-comment all refreshed to "top-4" / "four independent Plotly.react calls".
- Smoke test renamed `three_spider_cells_present_when_data_exists` → `four_spider_cells_present_when_data_exists`; cell-count assertion + per-cell-id loop + Plotly.react needle loop bumped from 3 → 4.
- 4th fixture `mimalloc-wolfi.json` added so the smoke test's strict-4 assertion is satisfied through the full glob-load → score → render → write path; otherwise `cell_scores.len() <= TOP_N_SPIDER` would fall through and emit only 3 cells.
- All 169 tests pass (`cargo test -p alloc-bench-aggregator` → 138 unit + 31 smoke). Release build is clean.

## Task Commits

Each task was committed atomically (commits listed in execution order):

1. **Task 1: Bump TOP_N_SPIDER to 4** — `e15b4fd` (feat)
2. **Task 2: Add #N rank prefix to spider chart titles** — `514e470` (feat)
3. **Task 3: Convert spider grid to 2x2 layout for top-4** — `31f64f3` (style)
4. **Task 4: Pin spider grid count to 4 (atomic with new fixture)** — `8a6f26b` (test)

The Task-4 commit bundles the smoke-test pin update with the new `mimalloc-wolfi.json` fixture so each commit in the chain is independently green (bisectable). See "Decisions Made" below for the rationale.

## Files Created/Modified
- `crates/alloc-bench-aggregator/src/recommend.rs` — `pub const TOP_N_SPIDER: usize = 3` → `= 4`; doc-comment + pinning test updated.
- `crates/alloc-bench-aggregator/src/html.rs` — `build_spider_context` Plotly title format string now `format!("#{}  {}/{}", idx + 1, score.alloc, score.env)` (two spaces); doc-comment + inline comments + in-crate test literals (`Top-3` → `Top-4`, `top-3` → `top-4`) all swept.
- `crates/alloc-bench-aggregator/src/main.rs` — single inline comment `(=3)` → `(=4)`.
- `crates/alloc-bench-aggregator/templates/index.html.tmpl` — section heading + caption + CSS block-comment + `.spider-grid` rule (flex → CSS grid) + `.spider-cell` rule (drop `flex:` line) + new `@media (max-width: 768px)` block + inline script-comment refreshed.
- `crates/alloc-bench-aggregator/tests/smoke.rs` — function rename `three_spider_cells_present...` → `four_spider_cells_present...`; assertions and loop bounds updated 3 → 4; doc-comment refreshed.
- `crates/alloc-bench-aggregator/tests/fixtures/mimalloc-wolfi.json` — **new**, two runs (multithread + cpu-bound) with healthy `samples_count: 50_000` + `warmup_duration_s: 5.0` so the cell does not flag `⚠ suspect` and scores into the top-4. Mirrors `ptmalloc-debian-slim.json` shape.

## Decisions Made

- **Fixture authored (not assertion-relaxed).** `fixture_guidance` preference (1): "PREFERRED — Add a 4th fixture file". Authored `mimalloc-wolfi.json` (~120 lines, ~10 min) so the smoke test's strict-4 assertion is satisfied end-to-end. The fallback (relaxing to `min(4, fixture_count)`) was rejected because it would leave the strict-4 contract untested at the smoke level.
- **Atomic Task-4 commit (fixture + test pin together).** The constraints suggested a separate `test(quick-260607-swo): add 4th spider fixture for top-4 coverage` commit. I combined them into a single atomic commit because either ordering produces a red intermediate commit (fixture-first leaves the legacy 3-cell assertion failing; test-first leaves the 4-cell assertion failing on a 3-fixture corpus). Atomic commits preserve `git bisect` accuracy. The single commit message documents both moves.
- **In-crate test `spider_section_emits_three_spider_cell_divs` left unchanged.** This test is named for "three" cells but feeds 3 synthetic scores through `render()` and asserts `cell_count == 3`. With `TOP_N_SPIDER = 4`, the partial-input branch (`cell_scores.len() <= TOP_N_SPIDER`) returns all 3 — the test still passes, exercising the fall-through path. The plan's grep gate (`top-3` / `Top-3` / `(=3)`) does not match this test's contents (it uses `THREE` / `three` / numeric `3`). The smoke test in Task 4 enforces the strict-4 contract end-to-end. Per scope-reduction prohibition I did not gold-plate by renaming/expanding this test to feed 4 scores; that would silently expand scope.
- **Opportunistic script-comment refresh.** Task 3 (f) noted updating the `<script>` comment "three independent Plotly.react calls" → "four" was acceptable but not required. I made the change because I was already in the file and the comment would have read stale.

## Deviations from Plan

None - plan executed exactly as written, including the one Claude-discretion call documented in "Decisions Made" above (atomic commit vs. separate fixture-add commit).

## Issues Encountered

None. The fixture-risk flagged in `fixture_guidance` was a known trade-off, not an issue — I followed the documented preferred path (option 1).

## Verification Steps Run

1. `cargo test -p alloc-bench-aggregator top_n_constants_match_locked_values` → 1 pass (Task 1 gate).
2. `cargo check -p alloc-bench-aggregator` → clean (post-Task 2 + post-Task 3).
3. `grep -rn "top-3\|Top-3\|TOP_N_SPIDER (=3)" crates/alloc-bench-aggregator/{src,templates,tests}` → 0 matches (all 3 directories swept post-Task 4).
4. `grep -A1 "title\":" crates/alloc-bench-aggregator/src/html.rs | grep -E '#\{\}'` → confirms `format!("#{}  {}/{}", idx + 1, score.alloc, score.env)` (two spaces, locked title format).
5. `grep -c "Top-4 Above the Fold" crates/alloc-bench-aggregator/templates/index.html.tmpl` → 2 (CSS comment + heading).
6. `grep -c "grid-template-columns: repeat(2, 1fr)" ...` → 1.
7. `grep -c "@media (max-width: 768px)" ...` → 2 (pre-existing responsive layout + new spider override).
8. `cargo test -p alloc-bench-aggregator` → 138 unit + 31 smoke = 169/169 passed.
9. `cargo build --release -p alloc-bench-aggregator` → clean.

End-to-end render against `results/*.json` was skipped because no results fixtures exist locally (path is exercised through Task 4's smoke test against the committed fixtures).

## Self-Check: PASSED

- File `crates/alloc-bench-aggregator/tests/fixtures/mimalloc-wolfi.json` — FOUND.
- Commit `e15b4fd` (Task 1) — FOUND.
- Commit `514e470` (Task 2) — FOUND.
- Commit `31f64f3` (Task 3) — FOUND.
- Commit `8a6f26b` (Task 4) — FOUND.

## Next Phase Readiness

- All `must_haves` truths satisfied.
- Aggregator binary builds clean in release mode; HTML output now renders 4 ranked spider charts in a 2x2 grid with mobile reflow.
- No follow-ups required. The in-crate `spider_section_emits_three_spider_cell_divs` test name is now slightly stale-looking (it tests the partial-input fall-through, which functionally covers `cell_scores.len() < TOP_N_SPIDER`) — a future cosmetic sweep could rename it to `spider_section_emits_partial_input_cells` for clarity, but this is not blocking.

---
*Phase: quick-260607-swo*
*Completed: 2026-06-07*

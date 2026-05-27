---
phase: 09-spider-chart
plan: 03
subsystem: alloc-bench-aggregator
tags: [spider-chart, polar-trace, pareto-front, plotly, scatterpolar, wave-2-closing]
requires:
  - polar::build_trace (Plan 09-01)
  - polar::build_reference_trace (Plan 09-01)
  - polar::axis_label_for_chart (Plan 09-01)
  - score::pareto_front (Plan 09-02)
  - CellRecommendation::is_pareto (Plan 09-02)
  - top_n_cells widened signature (Plan 09-02)
  - meta sidecar image_size_mb (Phase 5 / D-13)
provides:
  - main::build_image_sizes (HashMap → BTreeMap projection, max-across-allocators per env)
  - HtmlContext spider_traces_json / spider_layout_json / has_spider fields
  - SpiderContext bundle (owned trace + layout JSON strings)
  - build_spider_context fn (top-3 cells + matrix-mean reference trace)
  - render() widened signature (runs, top_n, cell_scores)
  - pub fn write() widened signature (+ cell_scores: &[CellScore])
  - <section class="spider-chart"> + <div id="chart-spider"> + Plotly.react bootstrap
  - REPORT.md `| Pareto |` column on Top 10 cells summary table
  - per-cell template ★ glyph (CellTemplateContext::is_pareto)
  - tests/smoke.rs: spider_div_present_when_data_exists + plotly_sri_hash_unchanged_full_string
affects:
  - Phase 11 (byte-stability inputs: section heading "Top-3 Above the Fold" + caption verbatim)
  - Future CI/meta sidecar wiring (currently empty image_sizes → empty Pareto column → v1.0 byte-identity)
tech-stack:
  added: []
  patterns:
    - decorate-not-rewrite (HtmlContext field-add, no v1 schema mutation)
    - btreemap-btreeset-discipline (BTreeMap<env, f64> for byte-identical alphabetical iteration)
    - tdd-red-green per task (3 tasks × 2 commits each = 6 task commits)
    - tinytemplate \{ escape rule (Plotly bootstrap inline JS object literal)
    - Plotly.react NOT newPlot (RESEARCH §Anti-Patterns gate)
    - WR-01 cross-surface drift defense (sentinel test now pins ★ in BOTH md + html cards)
key-files:
  created: []
  modified:
    - crates/alloc-bench-aggregator/src/main.rs
    - crates/alloc-bench-aggregator/src/markdown.rs
    - crates/alloc-bench-aggregator/src/html.rs
    - crates/alloc-bench-aggregator/templates/index.html.tmpl
    - crates/alloc-bench-aggregator/templates/recommend-cell.md.tmpl
    - crates/alloc-bench-aggregator/templates/recommend-cell.html.tmpl
    - crates/alloc-bench-aggregator/tests/smoke.rs
key-decisions:
  - Reference trace FIRST in the spider-trace JSON array per CONTEXT.md "Layout & Plotly Configuration" — Plotly renders traces in array order, so the matrix-mean polygon needs to sit BEHIND cell polygons (otherwise the grey 25%-alpha reference covers the per-cell colored polygons).
  - has_spider gates on top_n.is_empty() rather than cell_scores.is_empty() — the two are equivalent in practice (top_n is truncated from cell_scores) and the documented contract per CONTEXT.md is "spider section presence mirrors top-N section presence" (single gate on top_n).
  - cell_scores cloned BEFORE recommend::top_n_cells consumes by-value, NOT widening top_n_cells signature to take &[CellScore] — keeps Plan 09-02's signature unchanged and preserves the semantic that top_n_cells truncates the full vec.
  - CSS .spider-grid min-height: 480px matches existing .chart-card discipline — same fixed-height defense against polygon collapse on narrow viewports.
  - Plotly.react (NOT newPlot) for the spider bootstrap — caught by aggregator_html_uses_plotly_react_not_newplot smoke test (RESEARCH §Anti-Patterns: react diffs DOM in place; newPlot re-mounts and causes flicker on re-render).
  - main.rs build_image_sizes uses .and_modify(max).or_insert(...) pattern — when an env has multiple allocators with the same Docker image, take the max image_size_mb across allocators (robust to repeated meta loads with rounding differences).
requirements-completed: [POLAR-01, POLAR-02, POLAR-03, POLAR-04, POLAR-05]
metrics:
  duration: 28 min
  started: 2026-05-28T00:00:00Z
  completed: 2026-05-28T00:28:00Z
  files-modified: 7
  files-created: 0
  lines-added: 489
  lines-removed: 17
  task-commits: 6
---

# Phase 09 Plan 03: Spider Chart Wiring + Pareto Column Summary

**One-liner:** Wave-2 closing plan — wire `polar.rs` (Plan 09-01) and `score::pareto_front` + `CellRecommendation::is_pareto` (Plan 09-02) into the rendered HTML/Markdown surfaces; closes the loop on every Phase 09 deliverable (POLAR-01..05).

## Why This Plan

Plans 09-01 and 09-02 landed the polar trace builder and the Pareto-front data foundations. Both were untested at the rendered-surface level: polar's `build_trace` / `build_reference_trace` had no consumer; recommend.rs's `is_pareto` was decorated but never rendered; main.rs's `top_n_cells` callsite had a stub empty `BTreeMap` for `image_sizes`. This plan closes the loop:

1. main.rs derives real `image_sizes` from `metas` — the Pareto front now activates the moment meta sidecars are wired into CI.
2. markdown.rs emits the new `| Pareto |` column on the Top 10 cells summary table; both per-cell templates render the trailing ★ for Pareto cells.
3. html.rs gains the `<section class="spider-chart">` block with Plotly.react bootstrap consuming top-3 cell traces + matrix-mean reference trace.
4. WR-01 sentinel test extended to assert ★ surfaces in BOTH per-cell card surfaces (REPORT.md cards + index.html cards) — drift-defense gate.

## Tasks Executed

### Task 1 — main.rs `build_image_sizes` helper (TDD)

- **RED commit `8fe84fa`** — `test(09-03): add failing tests for build_image_sizes helper`. Added 3 tests covering: max-across-allocators per env projection, no implicit zero entries for envs missing from metas (macOS host), empty-input degenerate case.
- **GREEN commit `eafe91d`** — `feat(09-03): main.rs build_image_sizes helper + wire image_sizes through top_n_cells`. Helper added, stub empty `BTreeMap` replaced with real derivation, callsite widened to 3-arg `recommend::top_n_cells(cell_scores, &outcome.runs, &image_sizes)`. All 3 tests pass; cargo build clean; full test suite green.

### Task 2 — markdown Pareto column + per-cell template ★ + WR-01 sentinel update (TDD)

- **RED commit `435ecc9`** — `test(09-03): add failing tests for Pareto column + WR-01 sentinel ★ extension`. Added 3 markdown tests (4-col header, 4-col separator, ★ row); extended 2 existing html sentinel tests (`cell_templates_both_reference_all_fields` + `cell_template_context_excludes_score_and_axes`) to gate `is_pareto` field + ★ glyph. All 5 RED tests fail as expected.
- **GREEN commit `59d1f1f`** — `feat(09-03): markdown Pareto column + per-cell template ★ + WR-01 sentinel`. CellTemplateContext gains `pub is_pareto: bool`; `build_cell_template_context` populates it; both per-cell `.tmpl` files append `{{ if is_pareto }} ★{{ endif }}` after the existing suspect_flag annotation; markdown.rs replaces the 3-col emit at lines 421-429 with the 4-col emit using `\u{2605}` for the glyph. All 5 RED tests pass; full crate suite green.

### Task 3 — html.rs spider context + index.html.tmpl section + Plotly SRI smoke tests (TDD)

- **RED commit `a5a5b4d`** — `test(09-03): add failing tests for spider chart context + section + SRI smoke`. Added 4 html tests (plotly_sri_hash_unchanged, spider_section_present_when_top_n_non_empty, spider_section_absent_when_top_n_empty, spider_traces_json_contains_scatterpolar_type) + 2 smoke tests (spider_div_present_when_data_exists, plotly_sri_hash_unchanged_full_string). Pre-emptively widened 5 existing 2-arg `render()` callsites to 3-arg form. Tests fail at compile time (E0061: render takes 2 args).
- **GREEN commit `b0be6e1`** — `feat(09-03): wire spider chart context + section + Plotly bootstrap`. HtmlContext gains 3 fields (spider_traces_json, spider_layout_json, has_spider); new SpiderContext bundle holds owned JSON strings; new `build_spider_context()` fn builds [reference_trace, top-3 cell traces] via `polar::build_trace` + `polar::build_reference_trace` (reference FIRST per CONTEXT.md so it renders BEHIND cells); render() and write() signatures widened to take `cell_scores: &[CellScore]`; main.rs clones cell_scores before passing to top_n_cells; index.html.tmpl gains the `<section class="spider-chart">` block (verbatim heading + caption + Plotly.react bootstrap with `\{ responsive: true })` escape); CSS for .spider-chart, .spider-grid, .spider-cell appended after .report-mirror rules. Plotly.react chosen over newPlot per RESEARCH §Anti-Patterns (caught by `aggregator_html_uses_plotly_react_not_newplot`). Full suite green: 127 unit + 30 smoke = 157 tests.

## Verification

- `cargo test -p alloc-bench-aggregator` — 157/157 pass (127 unit + 30 smoke)
- `cargo build -p alloc-bench-aggregator --release` — clean (LTO=fat per CLAUDE.md)
- `just aggregate` against the local 180-run results set produces:
  - `report/index.html` containing `<div id="chart-spider" class="spider-grid">` + 4 `"type":"scatterpolar"` traces (1 reference + 3 top cells: jemalloc/host, jemalloc/slim, mallocng/host)
  - `report/REPORT.md` containing `| Rank | Cell | Score | Pareto |` header + `|------|------|-------|--------|` separator + 7 cells (the local run produced 7 ranked cells, all rendering with empty Pareto cell because no `--meta` sidecars are passed by the local `just aggregate` recipe)
- All 6 task commits emitted on `worktree-agent-1779923095199b`; HEAD-safety guards passed for each commit; no protected-ref drift; no `git clean` / `git stash` / `git update-ref` invocations

## Verbatim Byte-Stability Inputs (Phase 11)

The following strings are pinned by tests and are byte-stability inputs to Phase 11's regression-hash gate. Any change to these literals MUST update both the source AND the test assertion in lockstep:

- Section heading: `Top-3 Above the Fold` (literal in `index.html.tmpl` line 268; pinned by `spider_section_present_when_top_n_non_empty`).
- Section caption: `Spider charts of the top-3 cells across 8 normalized axes (0-1). Grey reference polygon = mean across all 18 cells.` (literal in `index.html.tmpl` line 269; pinned by `spider_section_present_when_top_n_non_empty`).
- Plotly SRI hash: `sha384-MqL7Cy3itNqCI1Wlc926K0XhyRKJ/NMqTaytIIEB+QIdInOploxqRIHRKLlhPykM` (constant in `html.rs:65`; pinned by `plotly_sri_hash_unchanged` + `plotly_sri_hash_unchanged_full_string`; re-verify upstream via `curl -s 'https://cdn.plot.ly/plotly-2.35.3.min.js' | openssl dgst -sha384 -binary | base64`).
- Plotly CDN URL: `https://cdn.plot.ly/plotly-2.35.3.min.js` (constant in `html.rs:58`).

## Tests Added

| Surface | Test | Gates |
|---------|------|-------|
| `main::tests` | `image_sizes_built_from_metas_keyed_by_env` | POLAR-05 max-across-allocators projection |
| `main::tests` | `image_sizes_excludes_envs_with_no_metas` | POLAR-05 macOS-host exclusion (no implicit zeros) |
| `main::tests` | `image_sizes_empty_metas_returns_empty_map` | POLAR-05 empty-input degenerate |
| `markdown::tests` | `top_n_cells_summary_table_includes_pareto_column_header` | POLAR-05 4-col header literal |
| `markdown::tests` | `top_n_cells_summary_table_renders_star_for_pareto_cells` | POLAR-05 ★ glyph rendering |
| `markdown::tests` | `top_n_cells_summary_table_separator_includes_pareto_column` | POLAR-05 4-col separator literal |
| `html::tests` | `cell_templates_both_reference_all_fields` (extended) | WR-01 ★ in BOTH per-cell surfaces |
| `html::tests` | `cell_template_context_excludes_score_and_axes` (extended) | WR-01 + is_pareto in alphabetical key list |
| `html::tests` | `plotly_sri_hash_unchanged` | POLAR-04 byte-pin SRI hash literal |
| `html::tests` | `spider_section_present_when_top_n_non_empty` | POLAR-02 + POLAR-04 verbatim heading + caption |
| `html::tests` | `spider_section_absent_when_top_n_empty` | POLAR-02 `{{ if has_spider }}` gate |
| `html::tests` | `spider_traces_json_contains_scatterpolar_type` | POLAR-01 server-inlined trace JSON wire-up |
| `tests/smoke` | `spider_div_present_when_data_exists` | POLAR-04 end-to-end chart-spider div |
| `tests/smoke` | `plotly_sri_hash_unchanged_full_string` | POLAR-04 end-to-end full SRI hash literal |

**Total new tests:** 12 new + 2 extended = 14 tests gating POLAR-01..05.

## Files Touched

| File | Change | Why |
|------|--------|-----|
| `crates/alloc-bench-aggregator/src/main.rs` | + `build_image_sizes` helper, + 3 tests, real `image_sizes` derivation, cloned `scores_for_spider` for html::write | Tasks 1, 3 — POLAR-05 data path + spider plumbing |
| `crates/alloc-bench-aggregator/src/markdown.rs` | + `\| Pareto \|` column emit, + 3 tests | Task 2 — POLAR-05 column rendering |
| `crates/alloc-bench-aggregator/src/html.rs` | + 3 HtmlContext fields, + SpiderContext, + `build_spider_context`, render/write signatures widened, + 4 tests, 5 existing tests widened, sentinel test extended | Task 2, 3 — POLAR-01..05 surface |
| `crates/alloc-bench-aggregator/templates/index.html.tmpl` | + `<section class="spider-chart">` block, + Plotly.react bootstrap, + CSS for .spider-chart/.spider-grid/.spider-cell | Task 3 — POLAR-02, POLAR-04 surface |
| `crates/alloc-bench-aggregator/templates/recommend-cell.md.tmpl` | append `{{ if is_pareto }} ★{{ endif }}` to heading | Task 2 — per-cell ★ |
| `crates/alloc-bench-aggregator/templates/recommend-cell.html.tmpl` | append `{{ if is_pareto }} ★{{ endif }}` to heading | Task 2 — per-cell ★ |
| `crates/alloc-bench-aggregator/tests/smoke.rs` | + 2 end-to-end tests | Task 3 — POLAR-04 byte-pin |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Bug] Plotly.newPlot violation caught by pre-existing smoke test**

- **Found during:** Task 3 GREEN — first compile of the spider section initially used `Plotly.newPlot('chart-spider', ...)`. The pre-existing `aggregator_html_uses_plotly_react_not_newplot` smoke test (line 304) immediately failed because RESEARCH §Anti-Patterns mandates `Plotly.react` for re-renders project-wide.
- **Fix:** Changed the spider bootstrap to `Plotly.react('chart-spider', ...)`. Added a comment explaining the convention and citing the gating test by name.
- **Why this is safe:** `Plotly.react` is API-compatible with `Plotly.newPlot` for first-render — both produce a freshly-mounted chart. The semantic difference only matters on re-render (react diffs in place; newPlot re-mounts). The spider chart is currently render-once but the project-wide convention applies.
- **Commit:** Folded into `b0be6e1` (Task 3 GREEN).
- **Files modified:** `templates/index.html.tmpl`.

No other deviations — the plan's three tasks were specified precisely enough to execute as written, including the exact insertion point in `index.html.tmpl`, the exact CSS rule placement after `.report-mirror`, and the exact `\{ responsive: true })` escape.

## Wave-2 → Phase 09 Closure

Phase 09 ships the following user-visible deliverables:

1. **Spider chart in index.html.** A `<section class="spider-chart">` with `<div id="chart-spider">` renders 4 Plotly scatterpolar traces: 1 matrix-mean reference (grey 25% alpha) + 3 top-cell polygons. Plotly.react bootstrap; SRI-pinned CDN; CSP-compliant.
2. **Pareto column in REPORT.md.** The Top 10 cells summary table gains a `| Pareto |` column with U+2605 (★) glyphs on Pareto-front cells.
3. **★ on per-cell card headings.** Both REPORT.md cards (`### {rank}. {alloc}/{env} ★`) and index.html cards (`<h3>{rank}. {alloc}/{env} ★</h3>`) now mark Pareto-front cells.
4. **Byte-identical fallback.** When `--meta` sidecars are absent (current `just aggregate`, current CI smoke), `image_sizes` is empty → all cells `is_pareto: false` → no ★ glyphs render → v1.0 byte-identical output preserved. The full Pareto pipeline activates the moment meta sidecars are wired into CI (`just bench-all` already produces them per Phase 5 / D-13).

## Self-Check: PASSED

- `crates/alloc-bench-aggregator/src/main.rs` — modified (verified via `git log --oneline eafe91d -- crates/alloc-bench-aggregator/src/main.rs`)
- `crates/alloc-bench-aggregator/src/markdown.rs` — modified (verified via `git log --oneline 59d1f1f -- crates/alloc-bench-aggregator/src/markdown.rs`)
- `crates/alloc-bench-aggregator/src/html.rs` — modified (verified via `git log --oneline b0be6e1 -- crates/alloc-bench-aggregator/src/html.rs`)
- `crates/alloc-bench-aggregator/templates/index.html.tmpl` — modified (verified)
- `crates/alloc-bench-aggregator/templates/recommend-cell.md.tmpl` — modified (verified)
- `crates/alloc-bench-aggregator/templates/recommend-cell.html.tmpl` — modified (verified)
- `crates/alloc-bench-aggregator/tests/smoke.rs` — modified (verified)
- Commit `8fe84fa` (test RED Task 1) — exists
- Commit `eafe91d` (feat GREEN Task 1) — exists
- Commit `435ecc9` (test RED Task 2) — exists
- Commit `59d1f1f` (feat GREEN Task 2) — exists
- Commit `a5a5b4d` (test RED Task 3) — exists
- Commit `b0be6e1` (feat GREEN Task 3) — exists

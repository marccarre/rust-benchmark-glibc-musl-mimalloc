---
phase: 09-spider-chart
verified: 2026-05-28T00:00:00Z
status: passed
score: 5/5 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: null
  previous_score: null
  gaps_closed: []
  gaps_remaining: []
  regressions: []
---

# Phase 9: Spider Chart Verification Report

**Phase Goal:** Surface the 8-axis composite-score story above the fold through (a) a small-multiples spider-chart grid for the top-3 cells with a matrix-mean reference polygon, and (b) a Pareto-front overlay column on the Recommendations table — both wired into `report/index.html` + `report/REPORT.md` from server-side `serde_json` plus tinytemplate.

**Verified:** 2026-05-28
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (POLAR-01..05)

| #   | Truth (Must-Have)                                                                                                                                                                              | Status     | Evidence                                                                                                                                                                                                                                                                                                                                                                       |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 1   | **POLAR-01:** `polar.rs` has top-N spider trace builder emitting `{ r:[…9], theta:[…9], type:'scatterpolar', fill:'toself' }` per cell.                                                        | ✓ VERIFIED | `polar.rs:81-107` `build_trace` produces 9-element arrays with polygon closure (`r.push(r[0])`, `theta.push(theta[0].clone())`); JSON shape locked via `serde_json::json!` macro with `type: "scatterpolar"`, `fill: "toself"`, `name: "{alloc}/{env}"`. Test `trace_closes_polygon_with_9_elements` (line 261) PASSES — asserts `r.len()==9`, `theta.len()==9`, `r[0]==r[8]`, `theta[0]==theta[8]`. |
| 2   | **POLAR-02:** `report/index.html` has `<div id="chart-spider">` with top-3 cells above the fold as a small-multiples grid + matrix-mean reference polygon overlaid at 25% alpha.              | ✓ VERIFIED | `templates/index.html.tmpl` lines 215-245 carry `.spider-chart` and `.spider-grid` CSS; line 297 opens `{{ if has_spider }}` block with `<section class="spider-chart">`, `<h2>Top-3 Above the Fold</h2>`, `<div id="chart-spider" class="spider-grid">`, `Plotly.react('chart-spider', ...)`. `polar.rs:122-159` `build_reference_trace` carries `fillcolor: "rgba(128,128,128,0.25)"` (25% alpha) and `line.color: "rgba(128,128,128,0.5)"` (50% stroke). `html.rs:403-451` `build_spider_context` builds reference trace FIRST then top-3 cells. Tests `spider_section_present_when_top_n_non_empty`, `spider_section_absent_when_top_n_empty`, `spider_traces_json_contains_scatterpolar_type`, and integration test `spider_div_present_when_data_exists` (smoke.rs:779) all PASS. |
| 3   | **POLAR-03:** Spider chart's heuristic axes (image-size efficiency, security posture) visually distinguished — `(heuristic)` suffix on axis label.                                            | ✓ VERIFIED | `polar.rs:63-69` `axis_label_for_chart` appends ` (heuristic)` (11-byte suffix with leading U+0020) for `spec.is_heuristic == true`; returns `Cow::Borrowed(spec.label)` for measured axes (WR-04 fix avoids `String::to_string()` allocation on six of eight axes). Tests `axis_label_for_chart_appends_heuristic_suffix_for_image_size_efficiency_and_security_posture` (indices 2/6), `axis_label_for_chart_returns_plain_label_for_real_measurement_axes` (negative case), `axis_label_for_chart_handles_all_eight_axes_in_constant_order` (full sweep) and `trace_uses_axis_label_for_chart_for_theta` (theta wiring) all PASS. |
| 4   | **POLAR-04:** Test `html::tests::plotly_sri_hash_unchanged` pins the Plotly CDN URL + SRI hash to v2.35.3.                                                                                    | ✓ VERIFIED | `html.rs:60` `PLOTLY_CDN_URL = "https://cdn.plot.ly/plotly-2.35.3.min.js"`; `html.rs:67-68` `PLOTLY_SRI_HASH = "sha384-MqL7Cy3itNqCI1Wlc926K0XhyRKJ/NMqTaytIIEB+QIdInOploxqRIHRKLlhPykM"`. Unit test `plotly_sri_hash_unchanged` (html.rs:1318) and integration test `plotly_sri_hash_unchanged_full_string` (smoke.rs:795) both PASS — the SRI literal is byte-pinned in source AND in rendered output. |
| 5   | **POLAR-05:** `report/index.html` Recommendations table has Pareto-front overlay column with `★` for cells on the front of (composite_score, image_size_mb).                                  | ✓ VERIFIED | `score.rs:396-438` `pareto_front(cells, image_sizes) -> BTreeSet<(String, String)>` runs O(n²) strict-dominance sweep on (composite↑, image_size_mb↓); 6 `pareto_front_*` tests PASS. `recommend.rs:156` adds `is_pareto: bool` field to `CellRecommendation`; `recommend.rs:654` (WR-03 fix) computes Pareto front on the FULL `scores` slice BEFORE truncation to TOP_N_TOTAL — eliminates the user-facing ambiguity where a globally Pareto-optimal cell would lose its `★` if ranked outside the top-N. `markdown.rs:431` emits 4-column header `| Rank | Cell | Score | Pareto |`; line 434 renders `\u{2605}` for `is_pareto == true`. `recommend-cell.md.tmpl:1` and `recommend-cell.html.tmpl:2` both append `{{ if is_pareto }} ★{{ endif }}` after the alloc/env headline. `main.rs:127` derives per-env `image_sizes` via `build_image_sizes` (WR-02 fix: sorted keys + non-finite rejection); `main.rs:157` invokes 3-arg `recommend::top_n_cells(cell_scores, &outcome.runs, &image_sizes)`. |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact                                                            | Expected                                                                                | Status     | Details                                                                                                                                                                                  |
| ------------------------------------------------------------------- | --------------------------------------------------------------------------------------- | ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `crates/alloc-bench-aggregator/src/polar.rs`                        | New module exporting `build_trace`, `build_reference_trace`, `axis_label_for_chart`     | ✓ VERIFIED | 458 lines; 3 public fns + 9 unit tests; `Cow<'static, str>` (WR-04) for label fn; declared `mod polar;` at `main.rs:27`.                                                                  |
| `crates/alloc-bench-aggregator/src/html.rs`                         | Spider context fields + Plotly URL/SRI constants + `build_spider_context`               | ✓ VERIFIED | `HtmlContext` carries `spider_traces_json`, `spider_layout_json`, `has_spider`; `PLOTLY_CDN_URL` + `PLOTLY_SRI_HASH` byte-pinned; `build_spider_context` (line 403-451) reference-first then top-3. |
| `crates/alloc-bench-aggregator/src/score.rs`                        | `pareto_front` fn returning `BTreeSet<(String, String)>`                                | ✓ VERIFIED | Lines 396-438; strict-dominance O(n²) sweep; 6 unit tests.                                                                                                                                |
| `crates/alloc-bench-aggregator/src/recommend.rs`                    | `CellRecommendation::is_pareto: bool` + 3-arg `top_n_cells` signature                   | ✓ VERIFIED | Field at line 156; WR-03 full-sweep fix at line 654.                                                                                                                                      |
| `crates/alloc-bench-aggregator/src/markdown.rs`                     | 4-column Pareto header + `\u{2605}` rendering                                           | ✓ VERIFIED | Line 431 header `\| Rank \| Cell \| Score \| Pareto \|`; line 434 emits `★`.                                                                                                              |
| `crates/alloc-bench-aggregator/src/main.rs`                         | `mod polar;` + `build_image_sizes` helper + 3-arg call site                             | ✓ VERIFIED | Line 27, lines 89-114, line 157; `build_image_sizes` carries WR-02 sorted-keys + non-finite rejection.                                                                                    |
| `crates/alloc-bench-aggregator/templates/index.html.tmpl`           | `<div id="chart-spider">` + Plotly bootstrap + caption                                  | ✓ VERIFIED | Lines 215-245 CSS; lines 297-312 `{{ if has_spider }}` section.                                                                                                                           |
| `crates/alloc-bench-aggregator/templates/recommend-cell.md.tmpl`    | `{{ if is_pareto }} ★{{ endif }}` after headline                                        | ✓ VERIFIED | Line 1: `### {rank}. {alloc}/{env}{{ if suspect_flag }} *(suspect)*{{ endif }}{{ if is_pareto }} ★{{ endif }}`.                                                                           |
| `crates/alloc-bench-aggregator/templates/recommend-cell.html.tmpl`  | `{{ if is_pareto }} ★{{ endif }}` parity with markdown template                         | ✓ VERIFIED | Line 2: `<h3>{rank}. {alloc}/{env}{{ if suspect_flag }} *(suspect)*{{ endif }}{{ if is_pareto }} ★{{ endif }}</h3>`.                                                                      |
| `crates/alloc-bench-aggregator/tests/smoke.rs`                      | Integration tests asserting `<div id="chart-spider">` and full SRI literal              | ✓ VERIFIED | `spider_div_present_when_data_exists` (line 779) + `plotly_sri_hash_unchanged_full_string` (line 795).                                                                                    |

### Key Link Verification (Wiring)

| From                                          | To                                            | Via                                                                       | Status   | Details                                                                                                                                                                |
| --------------------------------------------- | --------------------------------------------- | ------------------------------------------------------------------------- | -------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `main.rs::main()`                             | `html::write`                                 | Passes `&scores_for_spider` (cloned `Vec<CellScore>`) into HTML writer    | ✓ WIRED  | `main.rs:156` clones `cell_scores` before `top_n_cells` consumes it; `main.rs:160` `html::write(..., &scores_for_spider, ...)` 5-arg call.                            |
| `html::build_spider_context`                  | `polar::build_trace` / `build_reference_trace` | Iterates first 3 scores → `polar::build_trace`; passes ALL scores → `build_reference_trace` | ✓ WIRED  | `html.rs:403-451`; reference trace built FIRST so it renders behind the per-cell polygons (z-order). Confirmed via test `spider_traces_json_contains_scatterpolar_type`. |
| `index.html.tmpl::<div id="chart-spider">`    | Plotly.react bootstrap                        | `{ spider_traces_json \| unescaped }` placeholder + `{ spider_layout_json \| unescaped }` | ✓ WIRED  | Tinytemplate `{{ if has_spider }}` gate ensures empty top_n produces no spider section (smoke test `spider_section_absent_when_top_n_empty` passes).                  |
| `recommend::top_n_cells`                      | `score::pareto_front`                         | Computes front on FULL `scores` slice BEFORE truncation                   | ✓ WIRED  | `recommend.rs:654` `let pareto_set = crate::score::pareto_front(&scores, image_sizes); let top_scores = crate::score::top_n(scores, TOP_N_TOTAL);` — WR-03 fix.       |
| `markdown.rs` Pareto column                   | `CellRecommendation::is_pareto`               | Column header `\| Pareto \|` + cell glyph `\u{2605}`                       | ✓ WIRED  | `markdown.rs:431` header; `markdown.rs:434` value emission.                                                                                                            |
| `recommend-cell.md.tmpl` & `.html.tmpl`       | `CellRecommendation::is_pareto`               | `{{ if is_pareto }} ★{{ endif }}` parity                                  | ✓ WIRED  | Both templates carry the conditional after the headline; surface-parity defense via `cell_templates_both_reference_all_fields` test.                                  |
| `main.rs::build_image_sizes`                  | `score::pareto_front` (via `top_n_cells`)     | `BTreeMap<env, max(image_size_mb)>` projection                            | ✓ WIRED  | `main.rs:127` derives map; `main.rs:157` passes to `recommend::top_n_cells`; per WR-02, sorted-keys + non-finite rejection guards downstream determinism.              |

### Data-Flow Trace (Level 4)

| Artifact                          | Data Variable                                          | Source                                                                              | Produces Real Data | Status      |
| --------------------------------- | ------------------------------------------------------ | ----------------------------------------------------------------------------------- | ------------------ | ----------- |
| `<div id="chart-spider">`         | `spider_traces_json` / `spider_layout_json`           | `html::build_spider_context` ← `polar::build_trace`/`build_reference_trace`         | Yes (when ≥1 cell) | ✓ FLOWING   |
| Per-cell theta labels             | `theta` array in scatterpolar JSON                     | `polar::axis_label_for_chart` ← `MEASUREMENT_AXES` registry                         | Yes (8+1 entries)  | ✓ FLOWING   |
| Reference polygon r values        | `r` array in matrix-mean trace                         | `polar::build_reference_trace` averages each axis across input scores                | Yes (real means)   | ✓ FLOWING   |
| Recommendations Pareto column     | `is_pareto: bool` on each `CellRecommendation`         | `recommend::top_n_cells` ← `score::pareto_front` ← `image_sizes` ← `build_image_sizes` | Yes (full-sweep)   | ✓ FLOWING   |
| Per-env image sizes               | `image_sizes: BTreeMap<String, f64>`                   | `main::build_image_sizes` ← `loader::load_cell_metas` ← `--meta` glob               | Yes (sorted, finite-only) | ✓ FLOWING   |

All five wired artifacts trace back to real data sources (cell metas, score axes, runtime registry) — no hardcoded fixtures or static fallbacks bypass the wiring. Empty-input degenerate paths (zero cells, missing metas) are handled with the `{{ if has_spider }}` gate, the `n=0` interpolated reference name, and the `is_pareto = false` default — all covered by passing tests.

### Behavioral Spot-Checks

| Behavior                                                                                                            | Command                                                            | Result        | Status |
| ------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ | ------------- | ------ |
| Aggregator binary tests (polar, score, recommend, markdown, html, main)                                             | `cargo test -p alloc-bench-aggregator --bin alloc-bench-aggregator` | 131 passed; 0 failed | ✓ PASS |
| Aggregator smoke integration tests (full HTML/MD render against fixtures)                                           | `cargo test -p alloc-bench-aggregator --tests`                     | 30 passed; 0 failed   | ✓ PASS |
| `polar::build_trace` polygon closure on synthetic CellScore                                                          | unit test `trace_closes_polygon_with_9_elements`                   | passes               | ✓ PASS |
| `polar::axis_label_for_chart` indices 2/6 carry `(heuristic)` suffix                                                | unit test `axis_label_for_chart_handles_all_eight_axes_in_constant_order` | passes               | ✓ PASS |
| Plotly v2.35.3 SRI hash byte-pinned in source AND in rendered HTML                                                   | unit test `plotly_sri_hash_unchanged` + integration `plotly_sri_hash_unchanged_full_string` | both pass            | ✓ PASS |
| `<div id="chart-spider">` present in rendered HTML when top_n non-empty; absent when empty                          | unit tests `spider_section_present_when_top_n_non_empty` + `spider_section_absent_when_top_n_empty` | both pass            | ✓ PASS |
| Pareto-front strict-dominance + alphabetical iteration                                                               | 6 `score::tests::pareto_front_*` tests                              | all pass             | ✓ PASS |
| Recommendations 4-column header + `★` glyph rendering                                                               | `markdown::tests::recommendation_table_with_pareto_column` (and adjacent)  | passes               | ✓ PASS |

**Total: 161/161 automated tests pass.**

### Probe Execution

No `scripts/*/tests/probe-*.sh` probes are declared by Phase 9 plans — Phase 9's verification contract is the unit + integration tests above, which are already running through `cargo test`. SKIPPED (no probes for this phase).

### Requirements Coverage

| Requirement | Source Plan      | Description                                                                                       | Status      | Evidence                                                                                                                            |
| ----------- | ---------------- | ------------------------------------------------------------------------------------------------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| POLAR-01    | 09-01-PLAN.md    | Top-N spider trace builder with 9-element r/theta + scatterpolar/toself shape                     | ✓ SATISFIED | `polar.rs:81-107` `build_trace`; tests `trace_closes_polygon_with_9_elements`, `trace_carries_alloc_env_name_field` PASS              |
| POLAR-02    | 09-01,09-03 PLAN | `<div id="chart-spider">` small-multiples grid above the fold + matrix-mean reference at 25% alpha | ✓ SATISFIED | `index.html.tmpl:297-312`; `polar.rs:122-159` reference trace fillcolor `rgba(128,128,128,0.25)`; `html.rs:403-451` reference-first  |
| POLAR-03    | 09-01-PLAN.md    | Heuristic axes labelled with `(heuristic)` suffix                                                 | ✓ SATISFIED | `polar.rs:63-69` `axis_label_for_chart`; 4 dedicated unit tests covering positive, negative, ordering, and theta-wiring cases       |
| POLAR-04    | 09-01,09-03 PLAN | Plotly v2.35.3 CDN URL + SRI hash byte-pinned via test                                            | ✓ SATISFIED | `html.rs:60` URL constant; `html.rs:67-68` SRI constant; tests `plotly_sri_hash_unchanged` (unit) + `plotly_sri_hash_unchanged_full_string` (smoke) |
| POLAR-05    | 09-02,09-03 PLAN | Recommendations Pareto column with `★` for cells on the (composite, image_size) front            | ✓ SATISFIED | `score.rs:396-438` `pareto_front`; `recommend.rs:156,654` (WR-03 full-sweep fix); `markdown.rs:431,434` 4-col header + glyph; both `recommend-cell.*.tmpl` carry `{{ if is_pareto }} ★{{ endif }}` |

No orphaned requirements — REQUIREMENTS.md maps only POLAR-01..05 to Phase 9, and all five appear in at least one Phase 9 plan's `requirements:` field.

### Anti-Patterns Found

| File                                                            | Line | Pattern                                  | Severity | Impact                                                                             |
| --------------------------------------------------------------- | ---- | ---------------------------------------- | -------- | ---------------------------------------------------------------------------------- |
| _none_                                                          | —    | No `TBD`, `FIXME`, `XXX`, `placeholder`, or `not implemented` markers in Phase 9 source/template files | —        | Files modified in this phase carry no debt markers. The earlier review (REVIEW.md) raised one CR-01 + four warnings (WR-01..04) — all five are resolved per REVIEW-FIX.md (commits 06bd85f, b2ab7a9, 70cdc3a, c4d15c1, 1cb51a5). |

The Phase-09 review (REVIEW.md → REVIEW-FIX.md) raised the following items, all now CLOSED:
- **CR-01** (Critical): suspect-run threshold semantics — fixed.
- **WR-01**: hard-coded `Matrix mean (n=18)` literal — fixed; `polar.rs:122-159` now interpolates `scores.len()`.
- **WR-02**: HashMap iteration non-determinism + non-finite `image_size_mb` swallow risk in `build_image_sizes` — fixed; `main.rs:89-114` sorts keys + rejects NaN/inf.
- **WR-03**: Pareto front computed on truncated top_n vs. full sweep — fixed; `recommend.rs:654` computes on full `scores` BEFORE `top_n` truncation.
- **WR-04**: `axis_label_for_chart` allocating a `String` for every non-heuristic axis — fixed; `polar.rs:63-69` returns `Cow<'static, str>`.

### Human Verification Required

None. All five must-haves are gated by automated tests that PASS:

- POLAR-01: unit tests (`trace_closes_polygon_with_9_elements`, `trace_carries_alloc_env_name_field`, `trace_uses_axis_label_for_chart_for_theta`)
- POLAR-02: unit tests (`spider_section_present_when_top_n_non_empty`, `spider_section_absent_when_top_n_empty`, `spider_traces_json_contains_scatterpolar_type`, `reference_trace_carries_25_percent_alpha_fill_and_50_percent_alpha_stroke`) + smoke test (`spider_div_present_when_data_exists`)
- POLAR-03: unit tests (4 `axis_label_for_chart_*` tests covering positive, negative, full-sweep ordering, and theta wiring)
- POLAR-04: unit test (`plotly_sri_hash_unchanged`) + smoke test (`plotly_sri_hash_unchanged_full_string`)
- POLAR-05: unit tests (6 `pareto_front_*` tests + `recommend::tests` for `is_pareto` field) + markdown render tests + cross-template `cell_templates_both_reference_all_fields` parity test

Visual quality of the rendered `<div id="chart-spider">` (Plotly canvas pixels, font rendering, polygon legibility) is observable but is NOT a must-have under POLAR-01..05 — the requirements gate the JSON shape, the DOM presence, the alpha literals, and the SRI pin. All five are checkable via the test harness without a browser.

### Gaps Summary

No gaps. Phase 9 delivers all five must-haves with full source-code, template, and test evidence. Earlier code-review findings (1 critical + 4 warnings) are closed in REVIEW-FIX.md and confirmed by the current passing test suite (161/161). The phase goal — surfacing the 8-axis story above the fold via a spider grid + Pareto-front overlay — is observably true in the codebase.

---

_Verified: 2026-05-28_
_Verifier: Claude (gsd-verifier)_

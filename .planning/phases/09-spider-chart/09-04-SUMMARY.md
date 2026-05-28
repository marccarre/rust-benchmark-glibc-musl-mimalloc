---
phase: 09-spider-chart
plan: 04
subsystem: alloc-bench-aggregator
tags: [spider-chart, small-multiples, ui-review-gap-closure, plotly, scatterpolar, blocker-fix]
requires:
  - polar::build_trace (Plan 09-01, unchanged)
  - polar::build_reference_trace (Plan 09-01, unchanged)
  - polar::axis_label_for_chart (Plan 09-01, unchanged)
  - axes::MEASUREMENT_AXES (frozen 8-element const array — heuristic flags drive per-tick tickfont color)
  - HtmlContext + SpiderContext bundle (Plan 09-03 — refactored here, not removed)
  - tinytemplate {{ for x in vec }} iteration (Plan 09-03 used unrolled fields; this plan switches to Vec iteration)
provides:
  - SpiderCellContext struct (per-cell traces_json + layout_json + index)
  - HtmlContext.spider_cells: &[SpiderCellContext] (replaces flat spider_traces_json/spider_layout_json fields)
  - build_spider_context returns Vec<SpiderCellContext> (one per top-N cell)
  - per-tick tickfont.color array computed from MEASUREMENT_AXES.is_heuristic flag (no hard-coded indices)
  - per-cell layout.title.text = "{alloc}/{env}" with font: { size: 14, color: "#1F2328" }
  - .spider-grid flex container + .spider-cell square cards with mobile stack at 768px
  - tests/smoke.rs::three_spider_cells_present_when_data_exists (smoke regression)
  - html::tests::spider_section_emits_three_spider_cell_divs (unit regression)
  - html::tests::angular_axis_tickfont_color_is_per_tick_array (unit regression)
affects:
  - Closes UI-REVIEW BLOCKER (single overlay polar chart vs three small-multiples grid)
  - Closes WR-01..WR-06 (6 warnings) as a natural consequence of the BLOCKER fix
  - Phase 11 (golden-fixture HTML drift expected — small-multiples grid replaces single overlay; Phase 11 owns regen)
  - Future UI auditor re-run should escalate to ≥20/24 (blocker + 6 warnings closed)
tech-stack:
  added: []
  patterns:
    - tinytemplate {{ for cell in spider_cells }} Vec iteration with `unescaped` formatter
    - tinytemplate \{ \} brace-escape rule for JS object literals embedded inside `{{ for }}` loops
    - per-tick tickfont color array computed from MEASUREMENT_AXES (no hard-coded heuristic indices)
    - decorate-not-rewrite: aggregator output shape unchanged; only the consumer (html::build_spider_context + template) restructures
    - r##"..."## raw-string delimiter level for negative-gate tests containing literal `#` (color hex codes)
    - Plotly.react per cell (NOT newPlot) — preserves DOM diff semantics established in Plan 09-03
key-files:
  created:
    - .planning/phases/09-spider-chart/09-04-SUMMARY.md
  modified:
    - crates/alloc-bench-aggregator/src/html.rs
    - crates/alloc-bench-aggregator/templates/index.html.tmpl
    - crates/alloc-bench-aggregator/tests/smoke.rs
decisions:
  - Combined Task 1 + Task 2 into a single commit (`fad9c5d`). Rationale: the template references the Rust HtmlContext field names (`spider_cells`, `cell.traces_json`, `cell.layout_json`, `cell.index`); splitting them produces an intermediate non-compiling state. Documented as Rule 3 (blocking-issue auto-fix). The plan's intended commit message `refactor(09): per-cell spider traces and layouts` was retained for the combined commit.
  - Used `Vec<SpiderCellContext>` (not flat `cell_1_*`/`cell_2_*`/`cell_3_*` fields). Tinytemplate's `{{ for ... in ... }}` iteration with the `| unescaped` formatter on per-cell embedded JSON works correctly; the Vec approach is cleaner and stays correct if `TOP_N_SPIDER` ever changes.
  - Per-tick `tickfont.color` array is computed by iterating `MEASUREMENT_AXES.iter().map(|spec| if spec.is_heuristic { "#666" } else { "#222" })` — no hard-coded `[2, 6]` indices. Stays in sync if MEASUREMENT_AXES reorders or adds axes (with a naturally-failing-loud test on size mismatch).
  - Mobile breakpoint `@media (max-width: 768px) { .spider-cell { flex-basis: 100%; } }` reuses the existing media query block (no new breakpoint introduced) to honor the existing responsive convention.
metrics:
  tasks: 6
  task_commits: 3
  duration_minutes: ~75
  completed_date: 2026-05-29
  tests_before: 161
  tests_after: 164
  tests_added: 3
  files_modified: 3
  loc_changed: ~447
---

# Phase 09 Plan 04: Spider Chart Small-Multiples Refactor Summary

**One-liner:** Refactored the spider section from a single overlay polar chart into a three-cell small-multiples grid, closing the UI-REVIEW BLOCKER (`single overlay vs. spec-required small-multiples grid`) plus all six dependent warnings (WR-01..WR-06) in one coordinated change to `html.rs` + `index.html.tmpl` + smoke tests.

## What Shipped

### Task 1 + Task 2 (combined commit `fad9c5d`) — `refactor(09-04): per-cell spider traces and layouts (UI-REVIEW BLOCKER fix)`

**Rust side (`html.rs`):**
- Added `use crate::axes::MEASUREMENT_AXES;`.
- Replaced `HtmlContext.spider_traces_json: String` + `spider_layout_json: String` with a single slice field `spider_cells: &'a [SpiderCellContext]`.
- Added new `SpiderCellContext { traces_json, layout_json, index }` struct (serializable for tinytemplate).
- Rewrote `build_spider_context` to:
  - Compute `tickfont_color` once as `Vec<&'static str>` by iterating `MEASUREMENT_AXES` (`"#666"` for heuristic axes, `"#222"` otherwise — indices 2 and 6 in the current frozen 8-element axes array).
  - Build the matrix-mean reference trace once against the **full** `cell_scores` (so the gray-band remains the population baseline regardless of which 3 cells render).
  - For each of the top-3 cells, build a `[reference_trace, cell_trace]` JSON pair + a per-cell `layout` carrying `title.text = "{alloc}/{env}"` with `font: { size: 14, color: "#1F2328" }`.
  - Return `Vec<SpiderCellContext>` (one element per cell, indexed 1..=N).

**Template side (`index.html.tmpl`):**
- Replaced single `<div id="chart-spider" class="spider-grid"></div>` with an outer wrapper containing `{{ for cell in spider_cells }} <div class="spider-cell" id="spider-cell-{cell.index}"></div> {{ endfor }}`.
- Replaced single `Plotly.react('chart-spider', ...)` script call with `{{ for cell in spider_cells }} Plotly.react('spider-cell-{cell.index}', { cell.traces_json | unescaped }, { cell.layout_json | unescaped }, \{ responsive: true }); {{ endfor }}`.
- Escaped literal braces in a `// title.text = "{alloc}/{env}"` developer comment to `\{alloc}/\{env}` so tinytemplate doesn't try to substitute them as values.

### Task 3 (commit `982ca24`) — `style(09-04): flex grid + square aspect ratio for spider cells`

CSS additions in the existing `<style>` block:
```css
.spider-chart h2 { margin: 0 0 var(--space-sm) 0; }
.spider-chart p { color: var(--color-text); }
.spider-grid { display: flex; flex-wrap: wrap; gap: var(--space-md); }
.spider-cell {
  background: var(--color-dominant);
  border: 1px solid var(--color-border);
  border-radius: 6px;
  padding: var(--space-md);
  flex: 1 1 calc((100% - 2 * var(--space-md)) / 3);
  aspect-ratio: 1 / 1;
  min-width: 280px;
}
```

Mobile breakpoint added to the existing `@media (max-width: 768px)` block (no new breakpoint):
```css
.spider-cell { flex-basis: 100%; }
.spider-chart { margin: var(--space-md); }
```

### Task 4 (commit `fc2b3b3`) — `test(09-04): assert three spider-cell divs and per-tick tickfont array`

- `html::tests::spider_section_emits_three_spider_cell_divs` (unit) — asserts `class="spider-cell"` count == 3 and `id="spider-cell-1/2/3"` present in rendered HTML.
- `html::tests::angular_axis_tickfont_color_is_per_tick_array` (unit) — uses `r##"..."##` raw-string delimiter to assert the literal 8-element color array `["#222","#222","#666","#222","#222","#222","#666","#222"]` is present, and a negative-gate that the SCALAR form `"tickfont":{"color":"#666"` is absent.
- `tests/smoke.rs::three_spider_cells_present_when_data_exists` (smoke) — end-to-end check against fixtures: 3 spider-cell divs, 3 unique ids, outer `chart-spider` wrapper, 3 independent `Plotly.react('spider-cell-N', ...)` calls.

### Task 5 — Verification

Ran `cargo run --release -p alloc-bench-aggregator -- --input "crates/alloc-bench-aggregator/tests/fixtures/*.json" --output /tmp/09-04-verify-report/` (since `just aggregate` requires `results/*.json` from a real bench run, not test fixtures). End-to-end gates:

| Gate | Expected | Actual |
|------|----------|--------|
| `class="spider-cell"` occurrences | 3 | 3 |
| Unique `id="spider-cell-{1,2,3}"` divs | 3 | 3 |
| Outer `<div id="chart-spider" class="spider-grid">` wrapper | preserved | preserved |
| Per-cell `Plotly.react('spider-cell-N', ...)` calls | 3 | 3 |
| Per-cell `title.text = "{alloc}/{env}"` | 3 (jemalloc/alpine, mimalloc/distroless-cc, ptmalloc/debian-slim) | 3 |
| `tickfont.color` array shape | `["#222","#222","#666","#222","#222","#222","#666","#222"]` | matches exactly |
| `#666` total occurrences in HTML | 6 (= 2 heuristic axes × 3 cells) | 6 |
| `cargo test -p alloc-bench-aggregator` | 0 failures | 133 unit + 31 smoke = 164 pass |

### Task 6 (this commit) — `docs(09-04): summary of spider small-multiples refactor`

This file.

## UI-REVIEW Gap Closures

| ID | Type | Closed by |
|----|------|-----------|
| BLOCKER (#0) | Single overlay polar chart vs spec-required small-multiples | Task 1+2 (`fad9c5d`) |
| WR-01 | Three `<div class="spider-cell">` containers absent | Task 1+2 (`fad9c5d`) |
| WR-02 | Per-cell `Plotly.react` calls absent (single combined call) | Task 1+2 (`fad9c5d`) |
| WR-03 | Per-cell `layout.title.text = "{alloc}/{env}"` absent | Task 1+2 (`fad9c5d`) |
| WR-04 | `tickfont.color` is scalar `"#666"`, not per-tick array | Task 1+2 (`fad9c5d`) |
| WR-05 | `.spider-grid` / `.spider-cell` CSS rules absent | Task 3 (`982ca24`) |
| WR-06 | Mobile-stack rule for spider cells absent in @768px breakpoint | Task 3 (`982ca24`) |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] Combined Task 1 (Rust struct refactor) + Task 2 (template HTML structure refactor) into a single commit (`fad9c5d`)**
- **Found during:** Task 1 implementation
- **Issue:** The template `index.html.tmpl` references the Rust struct field names directly (`spider_cells`, `cell.traces_json`, `cell.layout_json`, `cell.index`). Renaming `spider_traces_json` → `spider_cells` in `html.rs` while leaving the template referencing the old names would produce an intermediate non-compiling/non-rendering state, which would fail the regression-gate "every commit must pass `cargo test`" rule from the plan.
- **Fix:** Bundled both tasks into a single atomic commit using the `refactor` prefix. The plan's listed Task 2 commit message (`feat(09): three spider-cell containers in small-multiples grid`) was effectively absorbed into the refactor commit. No separate Task 2 commit exists.
- **Files modified:** `crates/alloc-bench-aggregator/src/html.rs`, `crates/alloc-bench-aggregator/templates/index.html.tmpl`
- **Commit:** `fad9c5d`

**2. [Rule 3 — Blocking] Tinytemplate brace-escape inside `{{ for }}` loops**
- **Found during:** Task 1+2 first render attempt (`Failed to find value 'alloc' from path 'alloc'`)
- **Issue:** A developer comment inside the `<script>` block read `// title.text = "{alloc}/{env}"`, which tinytemplate parsed as a value-substitution path. Inside `{{ for cell in spider_cells }}` loops, the parser expected `alloc` to be a field on `cell`.
- **Fix:** Escaped the literal braces to `\{alloc}/\{env}` per the existing template-escape convention used elsewhere (Plan 09-03's Plotly bootstrap pattern).
- **Files modified:** `crates/alloc-bench-aggregator/templates/index.html.tmpl` (single comment line)
- **Commit:** `fad9c5d`

**3. [Rule 3 — Blocking] Rust raw-string delimiter level upgrade in unit test**
- **Found during:** Task 4 first compile attempt
- **Issue:** Test assertion `r#""color":["#222","#222","#666",...]"#` failed to compile because the `#222` literal would close the `r#"..."#` raw-string delimiter prematurely.
- **Fix:** Upgraded both raw-string literals (the positive 8-element array assertion and the negative scalar-form gate) to `r##"..."##` (one extra `#` on each side).
- **Files modified:** `crates/alloc-bench-aggregator/src/html.rs` (test functions only)
- **Commit:** `fc2b3b3`

### Architectural Changes

None. The aggregator output shape (`crates/alloc-bench-core/src/output.rs` v1 schema) remains untouched per the decorate-not-rewrite convention. Only the consumer (`html::build_spider_context` + the template) restructures how it consumes the existing data.

### Out-of-Scope Files

Verified untouched per plan: `polar.rs`, `markdown.rs`, `recommend.rs`, `score.rs`, `axes.rs`. The Plotly SRI hash and CDN URL remain pinned to v2.35.3.

## Authentication Gates

None. No external services or credentials involved.

## Test Coverage

```
running 133 tests [unit]
test result: ok. 133 passed; 0 failed; 0 ignored

running 31 tests [smoke]
test result: ok. 31 passed; 0 failed; 0 ignored
```

Net: **161 → 164 tests** (+3 new regression tests for the BLOCKER fix).

## Known Stubs

None. All visual elements wire to real data:
- Per-cell `traces_json` carries the matrix-mean reference + the cell's actual scores.
- Per-cell `layout_json` carries the actual `{alloc}/{env}` from the cell's CellScore.
- The 8-element `tickfont.color` array is computed at build time from MEASUREMENT_AXES (not hard-coded).

## Threat Flags

None. The refactor introduces no new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries. The per-cell JSON is still passed through `to_script_safe_json` (escapes `<`/`>`/`&` for safe inline `<script>` embedding).

## Self-Check

- [x] FOUND: `crates/alloc-bench-aggregator/src/html.rs` (modified — `SpiderCellContext` struct, refactored `build_spider_context`, 2 new tests)
- [x] FOUND: `crates/alloc-bench-aggregator/templates/index.html.tmpl` (modified — three spider-cell divs, per-cell `Plotly.react`, CSS grid)
- [x] FOUND: `crates/alloc-bench-aggregator/tests/smoke.rs` (modified — added `three_spider_cells_present_when_data_exists`)
- [x] FOUND: commit `fad9c5d` (Task 1+2 combined refactor)
- [x] FOUND: commit `982ca24` (Task 3 CSS)
- [x] FOUND: commit `fc2b3b3` (Task 4 tests)
- [x] FOUND: rendered output `/tmp/09-04-verify-report/index.html` (44KB, contains all 7 verification gates listed in the Task-5 table above)
- [x] All 164 tests pass
- [x] No out-of-scope files modified

## Self-Check: PASSED

All claims verified. SUMMARY ready for final commit.

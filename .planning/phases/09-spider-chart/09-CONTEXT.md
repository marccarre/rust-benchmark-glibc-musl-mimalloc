---
phase: 9
phase_name: Spider Chart
gathered: 2026-05-28
status: Ready for planning
---

# Phase 9: Spider Chart - Context

**Gathered:** 2026-05-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Build NEW `crates/alloc-bench-aggregator/src/polar.rs` — server-side `scatterpolar` trace JSON builder consumed by `index.html.tmpl`'s new `<div id="chart-spider">`. Top-3 cells render above the fold as a small-multiples grid; matrix-mean reference polygon overlaid at 25% alpha for context; heuristic axes visually distinguished (`(heuristic)` suffix + muted tickfont); Plotly SRI hash pinned by test to prevent silent trace-API drift on Plotly upgrade. Adds the v1.0 Recommendations table's Pareto-front overlay column (POLAR-05).

**In scope (POLAR-01 through POLAR-05):**

1. NEW `crates/alloc-bench-aggregator/src/polar.rs` — `pub fn build_trace(score: &CellScore) -> serde_json::Value` returning `{ r, theta, type: 'scatterpolar', fill: 'toself' }` with **9 elements** (closes the polygon by repeating `r[0]` / `theta[0]`); `pub fn build_reference_trace(scores: &[CellScore]) -> serde_json::Value` for the matrix-mean polygon; `pub fn axis_label_for_chart(spec: &AxisSpec) -> String` returning `"<label> (heuristic)"` for heuristic axes, plain `<label>` otherwise.
2. EXTEND `crates/alloc-bench-aggregator/src/html.rs` — register the spider trace context (top-3 traces + reference trace + heuristic-aware axis labels) into the existing `index.html.tmpl` rendering pipeline.
3. EXTEND `crates/alloc-bench-aggregator/templates/index.html.tmpl` — add `<div id="chart-spider">` with a 3-cell small-multiples grid (1 row × 3 cols on desktop, stacked on mobile) BEFORE per-scenario chart blocks but AFTER Phase 8's `<section class="top-n-recommendations">` block.
4. NEW `polar::tests::trace_closes_polygon_with_9_elements` — fixture-driven test asserting `r.len() == 9 && theta.len() == 9 && r[0] == r[8] && theta[0] == theta[8]`.
5. NEW `html::tests::plotly_sri_hash_unchanged` — asserts the const literals `PLOTLY_SRI_HASH` and `PLOTLY_CDN_URL` are byte-equal to known-good values; comment cites the curl command for re-verification.
6. NEW `polar::tests::heuristic_axes_get_heuristic_suffix` — asserts `axis_label_for_chart` appends ` (heuristic)` for `image_size_efficiency` and `security_posture` only.
7. NEW `score::pareto_front(cells: &[CellScore], image_sizes: &BTreeMap<String, f64>) -> BTreeSet<(String, String)>` — returns the (alloc, env) keys on the Pareto front of `composite_score` (↑) vs `image_size_mb` (↓). Used by the Recommendations table's new Pareto column.
8. EXTEND `crates/alloc-bench-aggregator/src/markdown.rs` — extend `emit_recommendations` to render a `Pareto` column with `★` glyph for cells on the front; identical column in HTML's Recommendations table emitter.
9. NEW `score::tests::pareto_front_dominates_only_when_strictly_better` — confirms Pareto semantics (no cell on the front is strictly dominated; ties along an axis preserved).

**Out of scope:** Direction-marker `↑` / `↓` glyphs in column headers / chart axis labels (Phase 10 — `axes::arrow()` consumed by header renderers, not by `polar.rs`); golden-fixture regeneration (Phase 11 — TEST-01/TEST-02); per-cell drilldown navigation (V12-04, deferred to v1.2); JS axis-weighting slider (V12-01); mutating Phase 6's `MEASUREMENT_AXES` registry (frozen).

</domain>

<decisions>
## Implementation Decisions

### Spider Chart Layout & Top-3 Above-the-Fold

- **Small-multiples grid:** 1 row × 3 cols on desktop (≥768px), stacked 1 col on mobile via CSS flexbox. Keeps all 3 visible above the fold side-by-side. Wraps each chart in a `<div class="spider-cell">` inside `<div id="chart-spider" class="spider-grid">`. Existing `.report-content` wrapper handles outer spacing.
- **Per-cell chart container:** Square aspect ratio, ~340×340px on desktop. Plotly `autosize: true` with `responsive: true` so charts scale with viewport. CSS `aspect-ratio: 1 / 1` on `.spider-cell` enforces square shape across breakpoints.
- **Matrix-mean reference polygon:** Overlaid on EACH top-3 chart (not a separate 4th chart). Trace built by `polar::build_reference_trace(&all_scores)` — averages each axis over all 18 cells; rendered as a `scatterpolar` trace with `fill: 'toself'`, `fillcolor: 'rgba(128,128,128,0.25)'` (25% alpha greyscale), `line.color: 'rgba(128,128,128,0.5)'`, no markers. Adds it as the FIRST trace in each chart's `data:` array (so the cell's solid polygon renders ON TOP).
- **Section title and order in `index.html`:** `<h2>Top-3 Above the Fold</h2>` placed BEFORE the per-scenario chart blocks but AFTER the existing `<section class="top-n-recommendations">` Phase 8 block. Eye-path: Recommendations table → Top-N prose cards → spider visual → per-scenario detail charts.
- **Section caption:** `<p>Spider charts of the top-3 cells across 8 normalized axes (0-1). Grey reference polygon = mean across all 18 cells.</p>` — single-sentence orientation; no JS-driven legend (matches D-02 static-`file://` discipline).

### Heuristic Axis Visual Distinction (POLAR-03)

- **Mechanism:** Plotly `polar.angularaxis.gridcolor` only supports a single global color per axis (no per-tick override in scatterpolar). Workaround: emit `(heuristic)` suffix in the angular tick label via `axis_label_for_chart`, AND set the heuristic-axis tick text color to `#666` via `tickfont.color` per-tick using the `tickvals`/`ticktext`/`tickfont` arrays. Real-measurement ticks render at `#222`. The `(heuristic)` suffix is the primary signal; muted color is the secondary cue. Both meet the ROADMAP/REQUIREMENTS contract.
- **`(heuristic)` suffix location:** In `polar.rs::axis_label_for_chart(spec: &AxisSpec) -> String` — returns `format!("{} (heuristic)", spec.label)` when `spec.is_heuristic == true`, else `spec.label.to_string()`. Phase 6's `MEASUREMENT_AXES` constant is NOT mutated (frozen); the suffix is a render-time decoration.
- **Color contrast:** `#666` for heuristic tick text (passes WCAG AA on white background at 12px+ via `tickfont.size: 11`); `#222` for real-measurement ticks. Background fill across all charts: white (`#ffffff`).
- **Heuristic test:** YES — `polar::tests::heuristic_axes_get_heuristic_suffix` asserts `axis_label_for_chart` appends ` (heuristic)` for `image_size_efficiency` and `security_posture`, plain label for the other 6 axes (negative cases included to prevent false positives).

### Pareto-front Overlay Column (POLAR-05) & Plotly SRI Test

- **POLAR-05 in scope for Phase 9** (NOT deferred to v1.2). ROADMAP success criterion #5 lists it; defer-friendly only if budget runs over.
- **Pareto-front computation location:** NEW `score::pareto_front(cells: &[CellScore], image_sizes: &BTreeMap<String, f64>) -> BTreeSet<(String, String)>`. Returns the (alloc, env) keys on the Pareto front of `composite_score` (maximize) vs `image_size_mb` (minimize). `image_sizes` is a sidecar-derived `BTreeMap` keyed by env (Phase 5 D-13 plumbing); for each cell, look up `image_sizes[&cell.env]`. macOS host cell has no Docker image — handled as `None` and skipped from Pareto consideration (or treated as `f64::INFINITY` size to never dominate). Resolve in plan.
- **Pareto column render:** EXTEND `markdown::emit_recommendations` (`crates/alloc-bench-aggregator/src/markdown.rs:340`) to add a final `Pareto` column. Cells in the returned `BTreeSet` get `★` glyph; others get empty cell. SAME column added to the HTML Recommendations table (template-driven, exact byte parity with the markdown emit). Header label: `Pareto`.
- **Plotly SRI test:** `html::tests::plotly_sri_hash_unchanged` — asserts `PLOTLY_SRI_HASH` equals the verbatim literal `"sha384-..."` (current value at lines 65-66 of `html.rs`) and `PLOTLY_CDN_URL` equals `"https://cdn.plot.ly/plotly-2.35.3.min.js"`. Test body comments cite the curl command for re-verification: `curl -s 'https://cdn.plot.ly/plotly-2.35.3.min.js' | openssl dgst -sha384 -binary | openssl base64 -A`. NO network access in the test (runs offline).
- **Polygon-closure test:** `polar::tests::trace_closes_polygon_with_9_elements` — builds a fixture `CellScore { axes: BTreeMap of 8 axes → values 0.1..0.8 }`, calls `polar::build_trace`, parses `serde_json::Value`, asserts:
  - `trace["r"].as_array().unwrap().len() == 9`
  - `trace["theta"].as_array().unwrap().len() == 9`
  - `trace["r"][0] == trace["r"][8]` (closure)
  - `trace["theta"][0] == trace["theta"][8]` (closure)
  - `trace["type"] == "scatterpolar"`, `trace["fill"] == "toself"` (REQUIREMENTS POLAR-01 verbatim).

### `polar.rs` Module Shape & Trace JSON Discipline

- **Module location:** NEW `crates/alloc-bench-aggregator/src/polar.rs`. Public surface: `pub fn build_trace`, `pub fn build_reference_trace`, `pub fn axis_label_for_chart`. Private helpers as needed.
- **Returned JSON structure:** `serde_json::Value` (NOT a typed `PolarTrace` struct). Reason: tinytemplate doesn't render `serde_json::Value` directly; the template context will hold pre-serialized JSON strings. Each trace is built as `serde_json::json!({...})`, then `serde_json::to_string(&trace).unwrap()` produces the embedded string passed to `index.html.tmpl` as `top_3_traces_json: String` (a JSON ARRAY of 3 trace objects + the reference polygon trace embedded in each).
- **MEASUREMENT_AXES iteration order:** All 8 axes in `MEASUREMENT_AXES` constant order (alphabetical by `key`, per Phase 6). The `theta` array uses `axis_label_for_chart(spec)` for human-readable angular ticks; the `r` array uses `score.axes[spec.key]` lookups. Polygon closure repeats `r[0]` / `theta[0]` at index 8.
- **Axis-value normalization:** Already 0..1 normalized in `CellScore.axes` (Phase 7 deliverable). `polar.rs` consumes verbatim — NO renormalization.
- **Missing axes (em-dash from Phase 7):** If `score.axes[key]` is missing or NaN for an axis, render that axis with `r = 0.0` and add a `tickfont.color: '#aaa'` muted style on that angular tick (separate from heuristic muting). Defer exact NaN handling to plan if Phase 7's contract already promises full 8-axis coverage.

### Plotly Configuration & Trace Construction

- **`scatterpolar` trace shape (REQUIREMENTS POLAR-01):**
  ```json
  {
    "r": [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.1],
    "theta": ["Channel throughput", "CPU-bound throughput", "Image-size efficiency (heuristic)", ..., "Channel throughput"],
    "type": "scatterpolar",
    "fill": "toself",
    "name": "{alloc}/{env}",
    "line": { "color": "..." }
  }
  ```
- **Per-cell trace color:** Auto-cycled via Plotly's default colorway (no manual color assignment per cell). Reason: avoids hand-picking per-allocator colors (would diverge from per-scenario charts that already use Plotly defaults). Reference polygon's grey tone visually distinguishes from any allocator color.
- **Layout per chart:** `{ polar: { radialaxis: { range: [0, 1], visible: true, showticklabels: false }, angularaxis: { tickvals: [0, 45, 90, ..., 315], ticktext: [labels], tickfont: { color: per-tick array } } }, showlegend: false, margin: { l: 20, r: 20, t: 30, b: 20 }, autosize: true }`. The `showticklabels: false` on radialaxis hides the 0..1 numeric ticks (cluttered at small chart size); axis range remains pinned to `[0, 1]` for polygon shape stability.
- **Plotly version:** v2.35.3 (already pinned in `index.html.tmpl` line 18; `PLOTLY_CDN_URL` const in `html.rs:58`). `plotly_sri_hash_unchanged` test guards against silent upgrade.

### File Writing & Orchestration

- **Where the spider chart context is built:** Inside the existing `html::write` orchestrator. Phase 9 adds calls to `score::top_n(scores, TOP_N_SPIDER)` (returns top-3 `CellScore`s, NOT `CellRecommendation`s — spider chart is data-only, no prose), then `polar::build_trace` per top-3 cell, then `polar::build_reference_trace(&all_scores)`, then assembles a `top_3_traces_json: String` for the template context.
- **Template context extension:** `index.html.tmpl` rendering context gains `top_3_traces_json: String`, `spider_layout_json: String`, and one `axis_label_*: String` field per axis (8 fields, named `axis_label_channel_throughput` etc.) — used by the existing per-scenario chart blocks too (POLAR sets up the labels; Phase 10 then injects `↑`/`↓` glyphs via `axes::arrow()`). For Phase 9, axis labels carry the `(heuristic)` suffix where applicable but no direction arrows yet (those come in Phase 10).
- **Markdown side:** No spider chart in REPORT.md (Plotly is HTML-only). REPORT.md is unchanged in Phase 9 EXCEPT for the new `Pareto` column on the existing Recommendations table.
- **Pareto-front data flow:** `score::score_cells` → already exists. Phase 9 adds: `let image_sizes = load_image_size_sidecars(...)?` (already loaded via Phase 5 plumbing — verify in plan), then `let pareto_set = score::pareto_front(&scores, &image_sizes)`, then thread `pareto_set` through to `markdown::emit_recommendations` and `html` Recommendations rendering as a `&BTreeSet<(String, String)>` parameter. Recommendations table uses `pareto_set.contains(&(alloc, env))` per row.

### Claude's Discretion

- Exact CSS for `.spider-grid` / `.spider-cell` (flexbox vs CSS grid; recommend `display: flex; flex-wrap: wrap; gap: 1rem;` on `.spider-grid` for simplicity).
- Whether to expose `polar::axis_label_for_chart` as `pub fn` (for Phase 10 reuse) or `pub(crate)` — recommend `pub fn` since other phases will likely consume it.
- Exact text of the heuristic-axis test's negative cases (e.g., assert `channel_throughput` does NOT carry `(heuristic)` — bool-flip safety net).
- Whether `score::pareto_front` accepts `&[CellScore]` or `&Vec<CellScore>` — recommend `&[CellScore]` (more flexible; Rust idiomatic).
- Whether the macOS-host cell (no Docker image) is excluded from Pareto consideration entirely or treated as `image_size_mb = INFINITY` — recommend exclusion (skip from input set; cleaner semantics).
- Exact glyph for Pareto-front cells — `★` recommended; `◆`, `●`, or `*` are alternatives. The chosen glyph must be ASCII-visible in REPORT.md and HTML alike.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/alloc-bench-aggregator/src/axes.rs` — Phase 6 deliverable. Exports `pub const MEASUREMENT_AXES: [AxisSpec; 8]` (alphabetical by key, frozen), `AxisSpec { key, label, direction, is_heuristic }`, `Direction::arrow()` (Phase 10's leaf). `polar.rs` iterates `MEASUREMENT_AXES` for `theta` order; reads `is_heuristic` for the suffix.
- `crates/alloc-bench-aggregator/src/score.rs` — Phase 7 Plan 01. Exports `pub fn score_cells(...) -> Vec<CellScore>`, `pub fn top_n(scores: Vec<CellScore>, n: usize) -> Vec<CellScore>`, `pub struct CellScore { alloc, env, composite, axes: BTreeMap<&'static str, f64> }`. Phase 9 calls `top_n(scores, TOP_N_SPIDER=3)` for the spider chart's top-3.
- `crates/alloc-bench-aggregator/src/recommend.rs` — Phase 7 Plan 02. Exports `pub const TOP_N_SPIDER: usize = 3`. Phase 9 references this constant verbatim — no magic `3` in `polar.rs`.
- `crates/alloc-bench-aggregator/src/html.rs` — already manages `PLOTLY_CDN_URL`, `PLOTLY_SRI_HASH` consts (lines 58-66), the tinytemplate registry, and the existing per-scenario `{{for}}` loops. Phase 9 extends the template context struct and adds the spider trace JSON strings.
- `crates/alloc-bench-aggregator/src/markdown.rs:340` — `emit_recommendations(buf, runs)` precedent. Phase 9 extends it with the Pareto column.
- `crates/alloc-bench-aggregator/templates/index.html.tmpl` — existing layout: header → Recommendations table → Top-N cards (Phase 8) → per-scenario chart blocks. Phase 9 inserts `<div id="chart-spider">` after Top-N cards, before per-scenario.
- Phase 5 D-13 sidecar plumbing — `meta/{alloc}-{env}.json` carries `image_size_mb`. Aggregator already loads these for the existing image-size column. Phase 9 reuses for Pareto-front input.

### Established Patterns
- Decorate-not-rewrite: `crates/alloc-bench-core/src/output.rs` v1 schema NOT mutated. All Phase 9 work is aggregator-side rendering / scoring.
- Byte-identical output discipline: BTreeMap/BTreeSet alphabetical iteration; `{:.1}` for composite scores in tables; `{}` for ns latencies.
- Frozen `MEASUREMENT_AXES`: render-time decoration only (Phase 6 contract). `polar.rs::axis_label_for_chart` is the canonical decorator.
- WR-01-pattern drift defense: render two views of the same struct, sentinel-check both contain every field. For Phase 9: the two views are (a) the spider chart trace JSON and (b) the per-scenario chart axis labels — Phase 10 adds direction arrows; Phase 9 ships only the (heuristic) suffix.
- Plotly version pinning: `PLOTLY_CDN_URL` + `PLOTLY_SRI_HASH` in `html.rs`; integrity SRI in `index.html.tmpl`. Phase 9 adds the test asserting they don't drift silently.
- `serde_json::json!` macro for trace construction (matches `crates/alloc-bench-aggregator/src/html.rs` existing patterns; no new typed structs).
- Static `file://`-friendly: NO new JS, NO inline scripts beyond the pinned Plotly CDN.

### Integration Points
- `main.rs` orchestration: existing `score::score_cells` call → Phase 9 adds `score::pareto_front` and threads `&BTreeSet<(String, String)>` through to both writers.
- `html::write` template context struct: gains `top_3_traces_json: String`, `spider_layout_json: String`, `axis_label_*: [String; 8]` fields. Phase 10 will inject `↑`/`↓` arrows into the labels.
- `markdown::emit_recommendations` signature: gains `pareto_set: &BTreeSet<(String, String)>` parameter. Same for the HTML Recommendations table renderer.
- Phase 8 `emit_top_n_cells` signature: NOT modified by Phase 9 — spider chart is parallel to the prose cards, not part of them.

</code_context>

<canonical_refs>
## Canonical References

| Path | Why this is canonical |
|------|----------------------|
| `.planning/REQUIREMENTS.md` (POLAR-01..05) | Locked requirements for Phase 9 |
| `.planning/ROADMAP.md` (Phase 9 entry) | Phase goal, dependencies, 5 success criteria |
| `.planning/PROJECT.md` | Decorate-not-rewrite + BTreeMap discipline |
| `.planning/phases/06-foundations/06-01-SUMMARY.md` | What `axes.rs::MEASUREMENT_AXES` actually shipped (8 axes, frozen, alphabetical) |
| `.planning/phases/07-scoring-top-n/07-02-SUMMARY.md` | What `recommend.rs` shipped (`TOP_N_SPIDER`, `top_n_cells`, env_short_name) |
| `crates/alloc-bench-aggregator/src/axes.rs` | Source of truth for `MEASUREMENT_AXES`, `AxisSpec`, `Direction::arrow()` |
| `crates/alloc-bench-aggregator/src/score.rs` | Source of truth for `CellScore`, `score_cells`, `top_n` |
| `crates/alloc-bench-aggregator/src/recommend.rs` | Source of truth for `TOP_N_SPIDER` const |
| `crates/alloc-bench-aggregator/src/html.rs` (lines 58-66, 116, 392, 695) | Plotly SRI pinning + tinytemplate context shape |
| `crates/alloc-bench-aggregator/templates/index.html.tmpl` (lines 7-19) | Existing Plotly tag + CSP; spider chart inserts after Top-N cards |
| `./CLAUDE.md` (Conventions section) | Suspect-flag thresholds + byte-identical-output + numeric formatting rules |

</canonical_refs>

<specifics>
## Specific Ideas

- **Polygon closure** = the 9-element `r` and `theta` arrays where `r[8] == r[0]` and `theta[8] == theta[0]`. This is the REQUIREMENTS POLAR-01 invariant and `polar::tests::trace_closes_polygon_with_9_elements` asserts it.
- **`(heuristic)` suffix** = exact 11-byte literal ` (heuristic)` (leading space) appended to `image_size_efficiency` and `security_posture` axis labels at chart-render time only. Phase 6's `MEASUREMENT_AXES` registry stays untouched.
- **Reference polygon at 25% alpha** = `fillcolor: 'rgba(128,128,128,0.25)'`, `line.color: 'rgba(128,128,128,0.5)'`. Greyscale tone distinguishes from any per-cell color (Plotly auto-cycles).
- **Pareto front of (composite_score↑, image_size_mb↓)** = a cell A is dominated iff there exists a cell B such that `B.composite_score >= A.composite_score AND B.image_size_mb <= A.image_size_mb` with at least one strict inequality. The front contains the non-dominated cells. macOS host (no image) excluded.
- **Spider chart container size** ~340×340px on desktop (CSS `aspect-ratio: 1 / 1`, max-width via parent). `autosize: true` + `responsive: true` in Plotly config so charts re-flow on window resize.
- **No JS dependencies beyond Plotly v2.35.3 (pinned)** — D-02 static-`file://` discipline maintained.
- **Section caption byte stability:** `<p>Spider charts of the top-3 cells across 8 normalized axes (0-1). Grey reference polygon = mean across all 18 cells.</p>` — exact byte match against any future golden test (Phase 11 regen).
- **Pareto column glyph:** `★` (U+2605, BLACK STAR) — single Unicode character, byte-identical across REPORT.md and HTML; legible in monospace and proportional fonts.

</specifics>

<deferred>
## Deferred Ideas

- **Direction-marker `↑` / `↓` glyphs in chart axis labels** — Phase 10 (DIR-03/04). Phase 9 leaves the labels arrow-free; Phase 10 wraps `axes::arrow()` injections.
- **JS axis-weighting slider** — V12-01 (v1.2). Static spider chart only in v1.1.
- **Cross-version diff radar** — V12-02 (v1.2). Spider compares cells WITHIN the current matrix only.
- **Per-cell drilldown navigation** (clicking a spider cell jumps to its `recommend-{rank:02d}-{alloc}-{env}.html`) — V12-04 (v1.2).
- **Confidence intervals on composite scores** (`87 ± 4` propagated from multi-run CV%) — V12-06 (v1.2).
- **Custom CSS for `.spider-grid` / `.spider-cell` polish** beyond minimal flexbox — out of scope; v1.1 ships functional layout with existing `.report-content` wrapper styling.
- **Animated polygon drawing** (Plotly transition on chart load) — adds JS complexity, no benefit over static render.

</deferred>

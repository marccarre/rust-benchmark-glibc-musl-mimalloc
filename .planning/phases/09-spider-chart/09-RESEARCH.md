# Phase 9: Spider Chart - Research

**Researched:** 2026-05-28
**Domain:** Plotly scatterpolar visualization + Pareto-front computation in alloc-bench-aggregator
**Confidence:** HIGH

## Summary

Phase 9 adds a 1×3 polar (spider) chart grid and a Pareto-front column to the existing aggregator HTML/Markdown reports. All upstream decisions (chart geometry, axis labelling, Pareto inclusion, Plotly version) are locked in `09-CONTEXT.md` and `09-UI-SPEC.md` — this research covers only the integration mechanics: a new `polar` module, a `is_pareto: bool` field on `CellRecommendation`, four template/source edits, and a 7-test plan. The aggregator binary already vendors Plotly v2.35.3 with a known SRI hash, so no new CDN work is required.

**Primary recommendation:** Add `mod polar;` to `main.rs` exposing `build_spider_traces(scores: &[CellScore], top_n: usize) -> Vec<serde_json::Value>`; populate `is_pareto` in `top_n_cells` via an O(n²) sweep; thread one new `{spider_traces_json}` template variable through `html.rs`; mark the heuristic-axis suffix and reference-mean opacity as covered locked decisions.

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| POLAR-01 | 1×3 spider chart grid (top-3 cells, 340×340 px) | §1 polar.rs API; §2 trace JSON shape |
| POLAR-02 | 9-element r/theta closure (8 axes + repeated r[0]/theta[0]) | §1 edge case; §6 test `trace_closes_polygon_with_9_elements` |
| POLAR-03 | Heuristic axes labelled with `(heuristic)` suffix; #666 tickfont | §2 layout block; §6 test `heuristic_axis_label_suffix` |
| POLAR-04 | Matrix-mean reference trace at opacity 0.25 | §2 trace JSON; §6 test `matrix_mean_alpha_25_percent` |
| POLAR-05 | Pareto-front column with ★ glyph; macOS host excluded | §4 algorithm; §5 wiring; §6 tests `pareto_front_basic`, `pareto_front_excludes_macos` |

## 1. polar.rs API

Proposed signature:
```rust
pub fn build_spider_traces(scores: &[CellScore], top_n: usize) -> Vec<serde_json::Value>
```

Returns one `serde_json::Value` per Plotly trace: top-N cell traces (opacity 1.0) plus one matrix-mean reference trace (opacity 0.25). `CellScore` from `score.rs:36-41` has fields `alloc: String`, `env: String`, `composite: f64`, `axes: BTreeMap<&'static str, f64>` — the eight axis values are read from the BTreeMap in deterministic insertion order, then closed by appending `axes[0]` and `theta[0]` (per POLAR-02).

Edge cases:
- **Empty scores** → return `vec![]` (no chart rendered downstream).
- **Fewer than `top_n` cells** → render `min(top_n, scores.len())` traces; matrix-mean trace still emitted if ≥1 cell present.
- **All-equal axes** (degenerate post-winsorize, all 50.0 per `score.rs:43-48`) → still produces a valid 9-element circle; covered implicitly by existing `score::tests`.

## 2. Plotly scatterpolar Trace JSON (v2.35.3)

Per-trace required keys:
```json
{
  "type": "scatterpolar",
  "r": [<8 floats>, <r[0] repeated>],
  "theta": [<8 axis labels>, <theta[0] repeated>],
  "fill": "toself",
  "fillcolor": "rgba(R,G,B,0.30)",
  "line": { "color": "rgba(R,G,B,1.0)" },
  "name": "<alloc>-<env>",
  "opacity": 1.0
}
```

Top-3 traces use `opacity: 1.0`; matrix-mean reference trace uses `opacity: 0.25` and a neutral grey (e.g. `rgba(128,128,128,...)`).

Layout (per chart):
```json
{
  "polar": {
    "radialaxis": { "visible": true, "range": [0, 100] },
    "angularaxis": {
      "tickfont": { "color": "#666" },
      "ticktext": ["throughput", "latency p50", "latency p99",
                   "memory peak", "memory rss", "ci stability",
                   "image size (heuristic)", "security (heuristic)"],
      "tickmode": "array"
    }
  },
  "showlegend": false,
  "width": 340, "height": 340,
  "margin": { "l": 40, "r": 40, "t": 40, "b": 40 }
}
```

**Per-tick coloring is unsupported in Plotly v2.35.3** — `angularaxis.tickfont` is global per polar subplot. The locked compromise is a single uniform `#666` tickfont applied to all eight ticks plus the textual `(heuristic)` suffix on the two heuristic axis labels (image size, security).

## 3. Plotly v2.35.3 SRI Hash

The hash and CDN URL are **already vendored** in `crates/alloc-bench-aggregator/src/html.rs`:

- `html.rs:58` — `pub(crate) const PLOTLY_CDN_URL: &str = "https://cdn.plot.ly/plotly-2.35.3.min.js";`
- `html.rs:65-66` — `pub(crate) const PLOTLY_SRI_HASH: &str = "sha384-MqL7Cy3itNqCI1Wlc926K0XhyRKJ/NMqTaytIIEB+QIdInOploxqRIHRKLlhPykM";`
- `html.rs:391-392` — bound into the template via `plotly_cdn_url` / `plotly_sri_hash` template variables.

Computed via the documented command at `html.rs:62`:
```
curl -s 'https://cdn.plot.ly/plotly-2.35.3.min.js' | openssl dgst -sha384 -binary | base64
```

Rendered tag (already emitted by the existing template):
```html
<script src="{plotly_cdn_url}"
        integrity="{plotly_sri_hash}"
        crossorigin="anonymous"></script>
```

**No CDN work is required for Phase 9** — the existing constants and template binding are reused unchanged. The existing smoke test at `tests/smoke.rs:127-131` already asserts `html.contains("sha384-MqL7Cy3i")`, locking the hash byte-for-byte.

## 4. Pareto-front Algorithm

Two-objective Pareto on (x = `composite_score`, ↑ better; y = `image_size_mb`, ↓ better):

> Cell A **dominates** cell B iff `A.x ≥ B.x` AND `A.y ≤ B.y` AND at least one inequality is strict.
> The **Pareto frontier** is the set of cells dominated by no other cell.

Algorithm (O(n²), n ≤ 18):
```rust
for i in 0..cells.len() {
    let mut dominated = false;
    for j in 0..cells.len() {
        if i == j { continue; }
        // Skip if j has no image_size_mb (macOS host)
        let (Some(yi), Some(yj)) = (cells[i].image_size_mb, cells[j].image_size_mb) else { continue; };
        let xi = cells[i].composite_score;
        let xj = cells[j].composite_score;
        let weak = xj >= xi && yj <= yi;
        let strict = xj > xi || yj < yi;
        if weak && strict { dominated = true; break; }
    }
    cells[i].is_pareto = !dominated && cells[i].image_size_mb.is_some();
}
```

Cells with `image_size_mb == None` (macOS host — image_size_mb is sidecar-only per Phase 5 D-13) are **always** `is_pareto: false`. They are physically excluded from the front because the y-axis is undefined for them.

Implementation site: extend `pub fn top_n_cells(scores: Vec<CellScore>, runs: &[Run]) -> Vec<CellRecommendation>` at `recommend.rs:619` to run the sweep on the returned vector before returning. The existing `pub struct CellRecommendation` at `recommend.rs:139` gains a single new field:
```rust
pub is_pareto: bool,
```

A `★` (U+2605) glyph is added to `templates/recommend-cell.md.tmpl` and `templates/recommend-cell.html.tmpl` as a `{pareto_marker}` substitution — empty string when `is_pareto=false`, `"★"` (or `"★ "` with trailing space for column alignment) when `is_pareto=true`.

## 5. Module Wiring (4 minimal edits)

**This crate is a binary, not a library** — `crates/alloc-bench-aggregator/src/main.rs:21-28` declares the modules (`mod axes; mod diagrams; mod html; mod loader; mod markdown; mod multi_run; mod recommend; mod score;`). There is no `lib.rs`. All edits below target `main.rs` for the module declaration and the existing source files for behavior.

1. **`main.rs`** (line ~28, alphabetical order between `multi_run` and `recommend`): add
   ```rust
   mod polar;
   ```
   And create `src/polar.rs` exporting `build_spider_traces`.

2. **`recommend.rs:139`** — add `pub is_pareto: bool,` to `CellRecommendation`. **`recommend.rs:619`** — extend `top_n_cells` to populate `is_pareto` via the O(n²) sweep on the returned vector.

3. **`html.rs`** — add a `spider_traces_json: &'a str` field to the template-binding struct (mirror `plotly_cdn_url` at `html.rs:115`); populate by calling `serde_json::to_string(&polar::build_spider_traces(&scores, 3))` and threading into the binding at the existing render site (~`html.rs:391`). Add a `pareto_marker: &'a str` row binding for the recommendation table.

4. **Templates**:
   - `templates/index.html.tmpl` — add `<div id="chart-spider"><script>Plotly.newPlot('chart-spider', {spider_traces_json}, {spider_layout});</script></div>` in the existing 4-chart-grid region (the smoke test at `tests/smoke.rs:278` already asserts on chart trace-builders, so the new div sits alongside without disturbing them).
   - `templates/recommend-cell.html.tmpl` and `templates/recommend-cell.md.tmpl` — add `{pareto_marker}` to the appropriate cell column.

## 6. Test Plan (7 tests)

| # | Location | Test | What it asserts |
|---|----------|------|------------------|
| 1 | `polar::tests` | `trace_closes_polygon_with_9_elements` | `r.len() == 9`, `r[0] == r[8]`, `theta[0] == theta[8]` for every emitted trace |
| 2 | `polar::tests` | `heuristic_axis_label_suffix` | `theta` strings for image-size and security axes carry `"(heuristic)"` suffix; the other six do not |
| 3 | `polar::tests` | `matrix_mean_alpha_25_percent` | The reference trace has `opacity == 0.25`; top-N traces have `opacity == 1.0` |
| 4 | `recommend::tests` | `pareto_front_basic` | Hand-constructed input with one strictly-dominated cell asserts that cell has `is_pareto == false` while the dominator has `is_pareto == true` |
| 5 | `recommend::tests` | `pareto_front_excludes_macos` | A cell with `image_size_mb == None` always has `is_pareto == false`, regardless of composite score |
| 6 | `tests/smoke.rs` (substring) | `plotly_sri_hash_unchanged` | Rendered HTML contains the byte-exact `sha384-MqL7Cy3itNqCI1Wlc926K0XhyRKJ/NMqTaytIIEB+QIdInOploxqRIHRKLlhPykM` (extends the existing prefix-only check at `smoke.rs:127`) |
| 7 | `tests/smoke.rs` (substring) | `spider_div_present_when_data_exists` | Rendered HTML contains `<div id="chart-spider">` when fixtures provide ≥1 cell |

Tests 1–5 are unit tests inside their respective module's `#[cfg(test)] mod tests` block. Tests 6–7 are integration-style substring assertions in `tests/smoke.rs`, mirroring the existing assertion style (`html.contains("...")`) at `smoke.rs:122-138`.

## 7. Golden-fixture Impact

**Existing assertions are scoped substring checks — Phase 9 changes will not break golden tests.**

`tests/smoke.rs` is the only integration test file (no `tests/golden.rs` exists). All assertions follow the `html.contains("...")` / `md.contains("...")` substring pattern (verified at `smoke.rs:122-152`, `smoke.rs:198-241`, `smoke.rs:280+`). There is **no full-file byte-equality assertion** anywhere in `tests/smoke.rs`. Adding a new `<div id="chart-spider">`, a new template variable, and a new `is_pareto` field will not invalidate any existing assertion — the smoke tests will remain green so long as the existing substring landmarks (Plotly URL, SRI hash, `# alloc-bench REPORT`, `## Docker runtimes`, `<!-- schema_version: 1`, the four pre-existing chart-builder substrings) continue to render.

Phase 11 retains ownership of any future full-fixture regeneration; Phase 9 introduces no new golden artifacts.

## Sources

### Primary (HIGH confidence)
- `crates/alloc-bench-aggregator/src/main.rs:21-28` — module declarations (binary crate, no lib.rs)
- `crates/alloc-bench-aggregator/src/score.rs:36-41` — `CellScore` field shape
- `crates/alloc-bench-aggregator/src/recommend.rs:139` — `CellRecommendation` struct
- `crates/alloc-bench-aggregator/src/recommend.rs:619` — `top_n_cells` signature
- `crates/alloc-bench-aggregator/src/html.rs:58` — `PLOTLY_CDN_URL` constant
- `crates/alloc-bench-aggregator/src/html.rs:65-66` — `PLOTLY_SRI_HASH` constant
- `crates/alloc-bench-aggregator/src/html.rs:115-116`, `391-392` — template binding for SRI hash + CDN URL
- `crates/alloc-bench-aggregator/tests/smoke.rs:122-138` — substring-only assertion style
- `09-CONTEXT.md` (locked decisions: 1×3 grid, 340×340, heuristic suffix, Plotly v2.35.3, Pareto in scope)
- `09-UI-SPEC.md` (visual contract; not re-read this session per directive)

### Secondary (MEDIUM confidence)
- Project conventions (CLAUDE.md): `BTreeMap` insertion order for byte-identical output; decorate-not-rewrite

### Tertiary (LOW confidence)
- None — all claims sourced from local code or locked context.

## Metadata

**Confidence breakdown:**
- polar.rs API: HIGH — signature derives directly from grep'd `CellScore` fields
- Plotly trace JSON: HIGH — locked in CONTEXT/UI-SPEC; v2.35.3 already vendored
- SRI hash: HIGH — already in `html.rs:65-66`, exercised by `smoke.rs:128`
- Pareto algorithm: HIGH — standard 2-objective sweep, n ≤ 18
- Module wiring: HIGH — `main.rs:21-28` directly grep'd
- Test plan: HIGH — pattern matches existing `smoke.rs` style
- Golden-fixture impact: HIGH — direct grep confirms substring-only assertions

**Research date:** 2026-05-28
**Valid until:** 2026-06-27 (30 days; aggregator surface is stable)

## RESEARCH COMPLETE

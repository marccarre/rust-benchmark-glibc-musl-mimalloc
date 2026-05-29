# Phase 9: Spider Chart - Pattern Map

**Mapped:** 2026-05-28
**Files analyzed:** 7 new/modified files
**Analogs found:** 7 / 7 (all in-repo)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/alloc-bench-aggregator/src/polar.rs` (NEW) | aggregator-render module | transform (CellScore → JSON Value) | `crates/alloc-bench-aggregator/src/score.rs` | role-match (sibling module; data-only, no IO) |
| `crates/alloc-bench-aggregator/src/score.rs` (EXTEND, add `pareto_front`) | aggregator-scoring module | transform (slice → BTreeSet) | `crates/alloc-bench-aggregator/src/score.rs::top_n` | exact (same file, sibling fn) |
| `crates/alloc-bench-aggregator/src/recommend.rs` (EXTEND `CellRecommendation` + `top_n_cells` to populate `is_pareto`) | aggregator-prose module | decorator (mut struct field) | `recommend.rs::top_n_cells` (same fn, current decoration loop) | exact |
| `crates/alloc-bench-aggregator/src/html.rs` (EXTEND `HtmlContext` + `render`) | template-context builder | request-response (runs → HTML string) | `html.rs::HtmlContext` / `html.rs::render` (same file) | exact |
| `crates/alloc-bench-aggregator/src/markdown.rs` (EXTEND `emit_recommendations` with Pareto column) | markdown-emitter | streaming (`buf.push_str`) | `markdown.rs::emit_recommendations` (same fn) | exact |
| `crates/alloc-bench-aggregator/src/main.rs` (EXTEND module decl) | binary entrypoint | orchestrator | `main.rs:21-28` (existing `mod` block) | exact |
| `crates/alloc-bench-aggregator/templates/index.html.tmpl` (EXTEND with `<div id="chart-spider">`) | tinytemplate template | request-response | existing `<main class="charts">` chart-card divs (lines 238-243) + `{{if has_top_n}}` block (lines 257-266) | exact |
| `crates/alloc-bench-aggregator/templates/recommend-cell.{html,md}.tmpl` (EXTEND with `{pareto_marker}`) | tinytemplate per-cell card | request-response | same files (existing `{{ if suspect_flag }} *(suspect)*{{ endif }}`) | exact (mirror suspect_flag pattern) |
| `crates/alloc-bench-aggregator/tests/smoke.rs` (EXTEND with 2 substring assertions) | integration test | request-response | `smoke.rs:127-138` (existing SRI substring assertions) | exact |

## Pattern Assignments

### `crates/alloc-bench-aggregator/src/polar.rs` (NEW: aggregator-render module, transform)

**Analog:** `crates/alloc-bench-aggregator/src/score.rs` (peer module — pure data transforms, no IO; consumes `MEASUREMENT_AXES` registry; consumed by `html.rs`)

**Module-doc + imports pattern** (score.rs lines 1-19):
```rust
//! Phase 7 / SCORE-01..04 + TEST-04 + TEST-05 — direction-aware
//! normalization, p10/p90 winsorization, composite weighted-sum scoring with
//! `MEASUREMENT_AXES` constant-order summation, and top-N selection with
//! `(alloc, env)` alphabetical tiebreak.
//!
//! Data-only. No prose. No rendering. `recommend.rs::top_n_cells` is the
//! prose-aware layer that decorates `CellScore` into `CellRecommendation`
//! (Plan 07-02).
use std::collections::{BTreeMap, HashMap};

use alloc_bench_core::output::Run;

use crate::axes::{Direction, MEASUREMENT_AXES};
use crate::loader::{CellMeta, SecurityMeta};
```

For `polar.rs`, the mirror imports are (per RESEARCH §1):
```rust
use crate::axes::MEASUREMENT_AXES;        // axis iteration order + (heuristic) suffix flag
use crate::axes::AxisSpec;                  // for axis_label_for_chart(spec)
use crate::score::CellScore;                // input type
use serde_json;                              // already a workspace dep, used in html.rs:266
```

**MEASUREMENT_AXES iteration pattern** (score.rs lines 305-313, 344-347):
```rust
// score_cells body — iterates MEASUREMENT_AXES.iter() in constant order,
// reads cell.axes[spec.key] via BTreeMap lookup. Spider trace `r` array
// follows this exact pattern.
let composite: f64 = MEASUREMENT_AXES
    .iter()
    .map(|spec| cell.axes.get(spec.key).copied().unwrap_or(0.0) * 0.125)
    .sum();
```

For polar.rs `build_trace`, the mirror loop is:
```rust
let mut r: Vec<f64> = MEASUREMENT_AXES
    .iter()
    .map(|spec| score.axes.get(spec.key).copied().unwrap_or(0.0))
    .collect();
let mut theta: Vec<String> = MEASUREMENT_AXES
    .iter()
    .map(axis_label_for_chart)
    .collect();
// Polygon closure (POLAR-02): repeat r[0]/theta[0] at index 8.
r.push(r[0]);
theta.push(theta[0].clone());
```

**`serde_json::json!` macro pattern for trace construction** (html.rs uses `serde_json::to_string` for `RESULTS` JSON; CONTEXT.md §"Specifics" decision: `serde_json::Value` not a typed struct):
```rust
// polar::build_trace returns a serde_json::Value built via the json! macro.
serde_json::json!({
    "type": "scatterpolar",
    "r": r,
    "theta": theta,
    "fill": "toself",
    "name": format!("{}/{}", score.alloc, score.env),
    "opacity": 1.0,
})
```

**Tests-module pattern** (score.rs lines 378-397 — tolerance helpers + synth fixtures):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn assert_vec_approx(actual: &[f64], expected: &[f64]) { /* ... */ }
}
```

For polar.rs::tests, mirror this with:
```rust
fn synth_score(alloc: &str, env: &str, vals_by_key: &[(&'static str, f64)]) -> CellScore {
    let mut axes: BTreeMap<&'static str, f64> = BTreeMap::new();
    for spec in MEASUREMENT_AXES.iter() {
        axes.insert(spec.key, 0.0);
    }
    for (k, v) in vals_by_key { axes.insert(*k, *v); }
    CellScore { alloc: alloc.into(), env: env.into(), composite: 0.0, axes }
}
```

---

### `crates/alloc-bench-aggregator/src/score.rs` — EXTEND with `pareto_front` (sibling fn)

**Analog:** `score.rs::top_n` (lines 362-376) — same file, same `pub fn` shape, returns `Vec<CellScore>` after a sort + truncate; mirrors the BTreeMap/sort-stability discipline.

**Existing top_n pattern** (score.rs lines 362-376):
```rust
pub fn top_n(scores: Vec<CellScore>, n: usize) -> Vec<CellScore> {
    let mut scores = scores;
    scores.sort_by(|a, b| {
        // Primary: composite DESC. `b.partial_cmp(&a)` for descending.
        b.composite
            .partial_cmp(&a.composite)
            .unwrap_or(std::cmp::Ordering::Equal)
            // Secondary: alloc ASC.
            .then_with(|| a.alloc.cmp(&b.alloc))
            // Tertiary: env ASC.
            .then_with(|| a.env.cmp(&b.env))
    });
    scores.truncate(n);
    scores
}
```

For `pareto_front`, signature per CONTEXT.md decision (Pareto-front computation location):
```rust
pub fn pareto_front(
    cells: &[CellScore],
    image_sizes: &BTreeMap<String, f64>,
) -> BTreeSet<(String, String)> {
    // O(n²) sweep, n ≤ 18 per CLAUDE.md cross-libc-rejection conventions.
    // image_sizes keyed by env (sidecar plumbing per Phase 5 D-13).
    // macOS host (no image) excluded by image_sizes lookup miss.
    // ...
}
```

**O(n²) Pareto sweep pattern** (RESEARCH §4):
```rust
let mut front: BTreeSet<(String, String)> = BTreeSet::new();
for i in 0..cells.len() {
    let Some(&yi) = image_sizes.get(&cells[i].env) else { continue; };  // skip macOS host
    let xi = cells[i].composite;
    let mut dominated = false;
    for j in 0..cells.len() {
        if i == j { continue; }
        let Some(&yj) = image_sizes.get(&cells[j].env) else { continue; };
        let xj = cells[j].composite;
        let weak = xj >= xi && yj <= yi;
        let strict = xj > xi || yj < yi;
        if weak && strict { dominated = true; break; }
    }
    if !dominated {
        front.insert((cells[i].alloc.clone(), cells[i].env.clone()));
    }
}
front
```

**`BTreeSet` discipline** (RESEARCH §4 + CLAUDE.md byte-identical-output): return type is `BTreeSet`, not `HashSet`, to match the existing `winners_by_class` pattern at recommend.rs.

---

### `crates/alloc-bench-aggregator/src/recommend.rs` — EXTEND `CellRecommendation` with `pub is_pareto: bool`

**Analog:** `CellRecommendation::suspect_flag` (recommend.rs:150) — same struct, last `bool` field; populated by `top_n_cells` at recommend.rs:667 via OR-aggregation.

**Existing `suspect_flag` field** (recommend.rs lines 138-151):
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct CellRecommendation {
    pub rank: usize,
    pub alloc: String,
    pub env: String,
    pub composite_score: f64,
    pub axes: BTreeMap<&'static str, f64>,
    pub tldr: String,
    pub strengths: Vec<&'static str>,
    pub weaknesses: Vec<&'static str>,
    pub recommended_for: Vec<&'static str>,
    pub avoid_for: Vec<&'static str>,
    pub suspect_flag: bool,
}
```

For Phase 9, append `pub is_pareto: bool,` after `suspect_flag` (alphabetical-by-purpose discipline preserved — `suspect_flag` and `is_pareto` are both rendering decorations, grouped at struct end).

**Existing `top_n_cells` decoration loop** (recommend.rs lines 619-671):
```rust
pub fn top_n_cells(scores: Vec<CellScore>, runs: &[Run]) -> Vec<CellRecommendation> {
    let top_scores = crate::score::top_n(scores, TOP_N_TOTAL);
    let winners = winners_by_class(runs);
    let losers = losers_by_class(runs);

    top_scores
        .into_iter()
        .enumerate()
        .map(|(i, cell)| {
            // ... derive strengths/weaknesses/tldr/recommended_for/avoid_for/suspect_flag
            CellRecommendation {
                rank: i + 1,
                alloc: cell.alloc.clone(),
                env: cell.env.clone(),
                composite_score: cell.composite,
                axes: cell.axes.clone(),
                tldr,
                strengths,
                weaknesses,
                recommended_for,
                avoid_for,
                suspect_flag,
            }
        })
        .collect()
}
```

For Phase 9 — three options (per RESEARCH §4 / CONTEXT.md §"Pareto-front data flow"):

**Option A (RESEARCH-recommended): inline sweep inside `top_n_cells` after the map.**

The signature gains `image_sizes: &BTreeMap<String, f64>`:
```rust
pub fn top_n_cells(
    scores: Vec<CellScore>,
    runs: &[Run],
    image_sizes: &BTreeMap<String, f64>,
) -> Vec<CellRecommendation> {
    // ... existing top-N decoration ...
    let mut out: Vec<CellRecommendation> = /* existing collect() */;
    // O(n²) Pareto sweep on the OUTPUT vec.
    // (Equivalent to invoking score::pareto_front on the original `scores`
    // slice; CONTEXT.md prefers a separate score::pareto_front fn so the
    // logic is testable in isolation.)
    let pareto_set: BTreeSet<(String, String)> =
        crate::score::pareto_front(&top_scores, image_sizes);
    for cell in out.iter_mut() {
        cell.is_pareto = pareto_set.contains(&(cell.alloc.clone(), cell.env.clone()));
    }
    out
}
```

**Option B (CONTEXT-recommended): keep `pareto_front` in score.rs, thread `&BTreeSet<(String, String)>` from `main.rs` through to both writers.** Pick at planning time.

---

### `crates/alloc-bench-aggregator/src/html.rs` — EXTEND `HtmlContext` + `render`

**Analog:** existing `HtmlContext` struct (html.rs lines 80-136), `BuiltContext` (lines 242-251), and `render` (lines 346-398). All Phase 9 additions follow the existing field-add pattern.

**Existing `HtmlContext` field-add pattern** (html.rs lines 80-136):
```rust
#[derive(serde::Serialize)]
struct HtmlContext<'a> {
    /// Pre-serialized JSON. Rendered via `{ results_json | unescaped }` so
    /// tinytemplate doesn't HTML-escape the `<`/`>`/`&`/`"` inside the JSON.
    results_json: &'a str,
    // ... 7 more JSON-string fields ...
    plotly_cdn_url: &'a str,
    plotly_sri_hash: &'a str,
    top_n_visible: &'a [CellTemplateContext],
    top_n_collapsed: &'a [CellTemplateContext],
    has_top_n: bool,
}
```

For Phase 9, append (per RESEARCH §5 + CONTEXT.md §"Template context extension"):
```rust
    /// Phase 9 / POLAR-01..04 — pre-serialized JSON array of scatterpolar
    /// trace objects (top-3 cell traces + matrix-mean reference trace).
    /// Rendered via `{ spider_traces_json | unescaped }`. Empty `[]` when
    /// `top_n.is_empty()`. Built by `polar::build_spider_traces`.
    spider_traces_json: &'a str,
    /// Phase 9 / POLAR-01 — pre-serialized JSON layout object (radialaxis
    /// range [0,1], angularaxis tickfont color #666, etc.). Rendered via
    /// `{ spider_layout_json | unescaped }`.
    spider_layout_json: &'a str,
    /// Phase 9 / POLAR-01 — gates the entire `<div id="chart-spider">`
    /// block via `{{ if has_spider }}...{{ endif }}` wrapper. Set to
    /// `!top_n.is_empty()` (mirror `has_top_n`).
    has_spider: bool,
```

**Existing `BuiltContext` owned-string-bundle pattern** (html.rs lines 242-251):
```rust
struct BuiltContext {
    results: String,
    scenarios: String,
    envs: String,
    allocators: String,
    suspect_pairs: String,
    multi_run_grouped: String,
}
```

For Phase 9, extend with:
```rust
    spider_traces: String,    // serde_json::to_string of polar::build_spider_traces(...)
    spider_layout: String,    // serde_json::to_string of layout JSON
```

**Existing `to_script_safe_json` escape pattern** (html.rs lines 266-272):
```rust
fn to_script_safe_json<T: serde::Serialize + ?Sized>(v: &T) -> Result<String> {
    let raw = serde_json::to_string(v).context("serializing to JSON")?;
    Ok(raw
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026"))
}
```

Use this same wrapper for the spider trace JSON — defends against `</script>` injection in any future `name` field that might inadvertently contain it.

**Existing `render` body pattern** (html.rs lines 346-398) — extend the `HtmlContext { ... }` initializer with the three new fields. Place `polar::build_spider_traces` call alongside the existing `top_n.len().min(TOP_N_TABLE)` split logic at lines 371-379.

**`PLOTLY_SRI_HASH` const pattern** (html.rs lines 60-66):
```rust
/// SRI integrity hash for `plotly-2.35.3.min.js` (RESEARCH §Code Examples §5).
/// Computed via:
///   curl -s 'https://cdn.plot.ly/plotly-2.35.3.min.js' \
///     | openssl dgst -sha384 -binary | base64
/// Verified live at research time (2026-05-19).
pub(crate) const PLOTLY_SRI_HASH: &str =
    "sha384-MqL7Cy3itNqCI1Wlc926K0XhyRKJ/NMqTaytIIEB+QIdInOploxqRIHRKLlhPykM";
```

Phase 9's `plotly_sri_hash_unchanged` test asserts these byte-equal; comment-cite the curl command verbatim per CONTEXT.md §"Plotly SRI test".

---

### `crates/alloc-bench-aggregator/src/markdown.rs` — EXTEND `emit_recommendations` with Pareto column

**Analog:** existing `emit_recommendations` (markdown.rs lines 374-384) — same fn, current 3-column layout.

**Existing pattern** (markdown.rs lines 374-384):
```rust
fn emit_recommendations(buf: &mut String, runs: &[Run]) {
    let recs = recommendations(runs);
    let _ = writeln!(buf, "## Recommendations by workload");
    let _ = writeln!(buf);
    let _ = writeln!(buf, "| Workload | Recommended | Rationale |");
    let _ = writeln!(buf, "|---|---|---|");
    for r in recs.iter() {
        let _ = writeln!(buf, "| {} | {} | {} |", r.class, r.allocator, r.rationale);
    }
    let _ = writeln!(buf);
}
```

For Phase 9, extend signature + body. The `pareto_set` parameter is threaded from `main.rs` (CONTEXT.md §"Pareto-front data flow"):
```rust
fn emit_recommendations(
    buf: &mut String,
    runs: &[Run],
    pareto_set: &BTreeSet<(String, String)>,
) {
    let recs = recommendations(runs);
    let _ = writeln!(buf, "## Recommendations by workload");
    let _ = writeln!(buf);
    let _ = writeln!(buf, "| Workload | Recommended | Rationale | Pareto |");
    let _ = writeln!(buf, "|---|---|---|---|");
    for r in recs.iter() {
        // recommendations() emits a String allocator label; need (alloc, env)
        // pair to look up Pareto. Resolve in plan: either widen Recommendation
        // to carry env, or skip Pareto column on this table (CONTEXT.md says
        // the Pareto column applies to the Recommendations table, but the
        // Recommendations table is class-keyed not cell-keyed). Plan must
        // disambiguate "Recommendations table" vs "Top 10 cells table".
        let pareto_glyph = ""; // placeholder — see plan
        let _ = writeln!(buf,
            "| {} | {} | {} | {} |",
            r.class, r.allocator, r.rationale, pareto_glyph
        );
    }
    let _ = writeln!(buf);
}
```

**Critical disambiguation for the planner:** RESEARCH §4 + RESEARCH §5 indicates the Pareto column lands on the **per-cell** `recommend-cell.{html,md}.tmpl` (via the new `is_pareto` field on `CellRecommendation`), NOT on the class-keyed `## Recommendations by workload` table. CONTEXT.md §"Pareto column render" disagrees ("EXTEND `markdown::emit_recommendations`"). Both interpretations cannot be simultaneously correct because `Recommendation` (recommend.rs:57) is class-keyed and has no `(alloc, env)` field. The planner must lock the resolution. Recommended interpretation: surface Pareto in **both** the leading `| Rank | Cell | Score | Pareto |` summary table inside `emit_top_n_cells` (markdown.rs:421-429 — extend the columns) AND in the per-cell card via `{pareto_marker}` template field. This matches RESEARCH §5 and the existing `## Top 10 cells` flow.

**Existing `## Top 10 cells` summary-table pattern** (markdown.rs lines 421-429):
```rust
let _ = writeln!(buf, "| Rank | Cell | Score |");
let _ = writeln!(buf, "|------|------|-------|");
for cell in top_n.iter() {
    let _ = writeln!(
        buf,
        "| {:02} | {} on {} | {:.3} |",
        cell.rank, cell.alloc, cell.env, cell.composite_score
    );
}
```

For Phase 9 (mirror approach):
```rust
let _ = writeln!(buf, "| Rank | Cell | Score | Pareto |");
let _ = writeln!(buf, "|------|------|-------|--------|");
for cell in top_n.iter() {
    let pareto_glyph = if cell.is_pareto { "\u{2605}" } else { "" }; // U+2605 BLACK STAR
    let _ = writeln!(
        buf,
        "| {:02} | {} on {} | {:.3} | {} |",
        cell.rank, cell.alloc, cell.env, cell.composite_score, pareto_glyph
    );
}
```

---

### `crates/alloc-bench-aggregator/src/main.rs` — EXTEND module declarations

**Analog:** existing `mod` block (main.rs lines 21-28) — alphabetical declaration order.

**Existing pattern** (main.rs lines 21-28):
```rust
mod axes;
mod diagrams;
mod html;
mod loader;
mod markdown;
mod multi_run;
mod recommend;
mod score;
```

For Phase 9, insert `mod polar;` between `multi_run` and `recommend` (alphabetical):
```rust
mod axes;
mod diagrams;
mod html;
mod loader;
mod markdown;
mod multi_run;
mod polar;        // <-- NEW (POLAR-01)
mod recommend;
mod score;
```

**Existing main() orchestration pattern** (main.rs lines 56-97):
```rust
fn main() -> Result<()> {
    let cli = Cli::parse();
    let outcome = loader::discover(&cli.input)?;
    let metas = loader::load_cell_metas(&cli.meta)?;
    let security_metas = loader::load_security_metas(&cli.security)?;
    // ... compute_axes / score_cells / top_n_cells ...
    let cell_axes = score::compute_axes(&outcome.runs, &metas, &security_metas);
    let cell_scores = score::score_cells(cell_axes);
    let top_n = recommend::top_n_cells(cell_scores, &outcome.runs);

    markdown::write(&outcome, &metas, &top_n, out_dir)?;
    html::write(&outcome, &metas, &top_n, out_dir)?;
    // ...
}
```

For Phase 9, derive `image_sizes: BTreeMap<String, f64>` from `metas` (which is `HashMap<(String, String), CellMeta>`) before the score pipeline. Mirror approach (CONTEXT.md §"Pareto-front data flow"):
```rust
// Build env → image_size_mb map. macOS host has no entry (skip from Pareto).
// CRITICAL: metas is keyed by (alloc, env); image_size_mb is per-cell. The
// Pareto y-axis is per-cell too. Either pass `&metas` directly OR derive a
// (alloc, env) → f64 map. Plan must lock.
let pareto_set = score::pareto_front(&top_scores_full, /* image_sizes */);
```

---

### `crates/alloc-bench-aggregator/templates/index.html.tmpl` — EXTEND with `<div id="chart-spider">`

**Analog:** existing `<main class="charts">` block (template lines 238-243) for chart-card CSS pattern; existing `{{if has_top_n}}` wrapper (lines 257-266) for conditional-section pattern.

**Existing chart-card pattern** (template lines 238-243):
```html
<main class="charts">
  <div id="chart-throughput" class="chart-card"></div>
  <div id="chart-latency" class="chart-card"></div>
  <div id="chart-rss" class="chart-card"></div>
  <div id="chart-diff" class="chart-card"></div>
</main>
```

**Existing conditional-section pattern** (template lines 257-266):
```html
{{ if has_top_n }}  <section class="top-n-recommendations">
    <h2>Top 10 cells</h2>
    <p>Ranked 1-10 by composite score (equal-weighted across 8 axes). Cards 6-10 collapsed by default.</p>
    {{ for cell in top_n_visible }}{{ call recommend-cell-html with cell }}{{ endfor }}
    <details>
      <summary>Show ranks 6–10</summary>
      {{ for cell in top_n_collapsed }}{{ call recommend-cell-html with cell }}{{ endfor }}
    </details>
  </section>
{{ endif }}</div>
```

For Phase 9, insert AFTER the `top-n-recommendations` `</section>` close, BEFORE the `<script>` block (template line 267). Per CONTEXT.md §"Section title and order":
```html
{{ if has_spider }}  <section class="spider-grid-section">
    <h2>Top-3 Above the Fold</h2>
    <p>Spider charts of the top-3 cells across 8 normalized axes (0-1). Grey reference polygon = mean across all 18 cells.</p>
    <div id="chart-spider" class="spider-grid"></div>
    <script>
      Plotly.newPlot('chart-spider',
        { spider_traces_json | unescaped },
        { spider_layout_json | unescaped },
        \{ responsive: true });
    </script>
  </section>
{{ endif }}
```

**Critical**: literal `{` in inline JS body MUST be escaped as `\{` per html.rs:21 (Pitfall 1) — `tinytemplate_compiles_index_template` test at html.rs:519 catches the regression.

**Existing CSS-variable + media-query pattern** (template lines 122-158): Phase 9's `.spider-grid` CSS goes in the `<style>` block. Mirror `.report-mirror` (template lines 170-203) for layout chrome; mirror the `@media (max-width: 768px)` block (template lines 146-158) for mobile stacking. CONTEXT.md §"Claude's Discretion" recommends `display: flex; flex-wrap: wrap; gap: 1rem;`.

---

### `crates/alloc-bench-aggregator/templates/recommend-cell.{html,md}.tmpl` — EXTEND with `{pareto_marker}`

**Analog:** existing `{{ if suspect_flag }} *(suspect)*{{ endif }}` decoration on both files — exact pattern for a conditional inline glyph.

**Existing recommend-cell.html.tmpl** (line 2):
```html
<h3>{rank}. {alloc}/{env}{{ if suspect_flag }} *(suspect)*{{ endif }}</h3>
```

**Existing recommend-cell.md.tmpl** (line 1):
```markdown
### {rank}. {alloc}/{env}{{ if suspect_flag }} *(suspect)*{{ endif }}
```

For Phase 9, append `{{ if is_pareto }} ★{{ endif }}` to BOTH files (mirror suspect_flag pattern):
```html
<h3>{rank}. {alloc}/{env}{{ if suspect_flag }} *(suspect)*{{ endif }}{{ if is_pareto }} ★{{ endif }}</h3>
```

**WR-01 sentinel-test impact** (html.rs:553-617): the `cell_templates_both_reference_all_fields` test asserts that every renderable scalar field surfaces in BOTH outputs. Adding `is_pareto` requires:
1. Adding `is_pareto: true` to the synthetic `CellRecommendation` at html.rs:564-576.
2. Adding the literal `★` (or `is_pareto`-derived sentinel) to the `expected_in_both` array at html.rs:590-601.
3. Updating `cell_template_context_excludes_score_and_axes`'s sorted key list at html.rs:655-668 to include `is_pareto`.

**`CellTemplateContext` extension** (html.rs lines 153-170): mirror `suspect_flag: bool` to add `pub is_pareto: bool`. Update `build_cell_template_context` (html.rs lines 177-190) to copy `cell.is_pareto`.

---

### `crates/alloc-bench-aggregator/tests/smoke.rs` — EXTEND with 2 substring assertions

**Analog:** existing `aggregator_emits_html_and_markdown_against_fixtures` (smoke.rs lines 109-155) — `html.contains("...")` substring assertion style.

**Existing pattern** (smoke.rs lines 122-138):
```rust
assert!(
    html.contains("https://cdn.plot.ly/plotly-2.35.3.min.js"),
    "HTML missing pinned Plotly 2.35.3 CDN URL"
);
// Use a prefix-match on the SRI to avoid line-wrap fragility.
assert!(
    html.contains("sha384-MqL7Cy3i"),
    "HTML missing pinned Plotly SRI integrity hash"
);
assert!(
    html.contains("crossorigin=\"anonymous\""),
    "HTML missing crossorigin=\"anonymous\" on CDN <script>"
);
```

**Existing helper** (smoke.rs lines 259-271):
```rust
fn run_aggregator_against_fixtures() -> (tempfile::TempDir, String) {
    let out_dir = tempdir().expect("tempdir");
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let pattern = format!("{}/*.json", fixtures.display());
    let mut cmd = Command::cargo_bin("alloc-bench-aggregator").expect("cargo bin");
    cmd.args(["--input"]).arg(&pattern).args(["--output"]).arg(out_dir.path());
    cmd.assert().success();
    let html = std::fs::read_to_string(out_dir.path().join("index.html")).expect("read index.html");
    (out_dir, html)
}
```

For Phase 9, mirror the chart-builder test at smoke.rs:278-296 with two new tests:
```rust
/// Phase 9 / POLAR-01: rendered HTML carries the spider chart container.
#[test]
fn spider_div_present_when_data_exists() {
    let (_dir, html) = run_aggregator_against_fixtures();
    assert!(
        html.contains(r#"<div id="chart-spider""#),
        "expected <div id=\"chart-spider\"> in index.html"
    );
}

/// Phase 9 / POLAR-04: full SRI hash literal (extends the existing prefix-only
/// check at smoke.rs:127). Comment-cites the verification curl command.
#[test]
fn plotly_sri_hash_unchanged() {
    // Re-verify with:
    //   curl -s 'https://cdn.plot.ly/plotly-2.35.3.min.js' \
    //     | openssl dgst -sha384 -binary | openssl base64 -A
    let (_dir, html) = run_aggregator_against_fixtures();
    assert!(
        html.contains("sha384-MqL7Cy3itNqCI1Wlc926K0XhyRKJ/NMqTaytIIEB+QIdInOploxqRIHRKLlhPykM"),
        "Plotly SRI hash drift — re-verify and update PLOTLY_SRI_HASH"
    );
}
```

---

## Shared Patterns

### Decorate-not-rewrite (Phase 1 D-11)
**Source:** CLAUDE.md §Conventions; `crates/alloc-bench-core/src/output.rs` v1 schema is locked.
**Apply to:** All Phase 9 work. The aggregator decorates at REPORT.md / HTML emit time. Phase 5 D-13 sidecar plumbing (`meta/{alloc}-{env}.json` carrying `image_size_mb`) is the canonical precedent — Phase 9's `is_pareto` and spider trace JSON ride through the same render-time decoration pattern. Never mutate `Run` shape.

### `BTreeMap` / `BTreeSet` over `HashMap` / `HashSet`
**Source:** CLAUDE.md §Conventions ("Byte-identical-output discipline"); evidenced throughout score.rs (line 14, 225, 241, 304, 319) and html.rs (line 23, 282, 289, 295, 314, 326).
**Apply to:** `polar::build_trace`'s axis iteration (already covered — uses `MEASUREMENT_AXES` array, NOT BTreeMap directly, but consumes `score.axes: BTreeMap` lookups); `score::pareto_front` returns `BTreeSet<(String, String)>` not `HashSet`.

### `MEASUREMENT_AXES` iteration
**Source:** `crates/alloc-bench-aggregator/src/axes.rs:67-76` (frozen const array, alphabetical by `key`).
**Apply to:** `polar.rs::build_trace` (8-element `r`/`theta` arrays); `polar.rs::axis_label_for_chart` (one call site per axis).
```rust
for spec in MEASUREMENT_AXES.iter() {
    // spec.key, spec.label, spec.direction, spec.is_heuristic
}
```

### `serde::Serialize` derive on render contexts
**Source:** html.rs:80 (`HtmlContext`), html.rs:153-154 (`CellTemplateContext`).
**Apply to:** Any new template-context struct in Phase 9. Keep fields `&'a str` (borrow from `BuiltContext`) or owned per existing pattern; tinytemplate's `{ field | unescaped }` consumes either.

### `tinytemplate` literal `{` escape (`\{`)
**Source:** html.rs:15-21 (Pitfall 1 doc-comment); template line 22, 60, 69 (every `{` in CSS/JS is `\{`).
**Apply to:** Any inline JS / CSS in `<div id="chart-spider">` block. The `tinytemplate_compiles_index_template` test (html.rs:519) is the gate.

### `is_suspect` predicate convention
**Source:** html.rs:76-78 (`pub(crate) fn is_suspect(h: &HarnessInfo) -> bool`).
**Apply to:** No direct reuse in Phase 9, but the `suspect_flag` decoration pattern (recommend.rs:154 + the conditional `{{ if suspect_flag }} *(suspect)*{{ endif }}` template idiom) is the EXACT mirror for `is_pareto` + `{{ if is_pareto }} ★{{ endif }}`.

### Numeric formatting
**Source:** CLAUDE.md §Conventions ("Byte-identical-output discipline"); markdown.rs:426 (`{:.3}` for composite_score), markdown.rs:381 (no precision suffix for class strings).
**Apply to:** Phase 9's spider trace JSON values are 0..=1 floats — no string formatting (serde_json::to_string handles it). Pareto column glyph is a fixed `★` string, no formatting.

### Module-doc heading convention
**Source:** axes.rs:1, score.rs:1, recommend.rs:1, html.rs:1 — all start with `//! Phase N / TICKET-ID — purpose.`.
**Apply to:** `polar.rs:1` should open with `//! Phase 9 / POLAR-01..04 — server-side scatterpolar trace JSON builder ...`.

### `BTreeSet`-in-tests (cross-file consistency)
**Source:** axes.rs:81-121 — every test asserts via BTreeSet, not HashSet, even at unit-test scope.
**Apply to:** All `polar::tests` and `score::tests` (pareto_front) tests use BTreeSet/BTreeMap.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| (none) | — | — | Every Phase 9 file maps to an existing in-repo analog. The crate's existing 9-module shape covers every role: data transform (score.rs), prose decorator (recommend.rs), template context builder (html.rs), markdown emitter (markdown.rs), tinytemplate body (templates/*), integration test (tests/smoke.rs). |

## Metadata

**Analog search scope:** `crates/alloc-bench-aggregator/src/`, `crates/alloc-bench-aggregator/templates/`, `crates/alloc-bench-aggregator/tests/`.
**Files scanned:** axes.rs, score.rs, recommend.rs, html.rs, markdown.rs, main.rs, loader.rs, index.html.tmpl, recommend-cell.html.tmpl, recommend-cell.md.tmpl, smoke.rs.
**Pattern extraction date:** 2026-05-28

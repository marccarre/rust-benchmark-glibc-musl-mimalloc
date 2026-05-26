# Feature Research — v1.1 Recommendations, Spider Charts & Direction Markers

**Domain:** Allocator-benchmark dashboard — adding actionable per-cell guidance (radar charts, recommendation prose, direction markers) on top of the v1.0 Plotly + REPORT.md aggregator.
**Researched:** 2026-05-26
**Confidence:** HIGH on radar/normalization conventions (multiple authoritative sources agree); MEDIUM on prose-card structure (no single canonical pattern, but consistent themes); MEDIUM-HIGH on direction-marker placement (Wikipedia comparison-table convention is the dominant precedent).

---

## TL;DR — Executive Recommendations for the v1.1 Spec

| Question (from prompt) | Recommendation | Confidence | Top citation |
|---|---|---|---|
| 1. Spider axis count — is 8 too many? | **8 is the upper bound** — supported by FBref/sports-radar precedent at 8–10 axes, but datavizcatalogue.com / Spotfire / Wikipedia all warn that comparing more than ~3 overlapping series at this axis count is unreadable. **Accept 8 axes; cap overlap at 3 series per chart, render the rest as small multiples.** | HIGH | datavizcatalogue.com, Spotfire, Wikipedia |
| 2. 0–100 normalization for direction-aware metrics | **Min-max normalization with explicit per-axis direction inversion**, p5/p95-winsorized to suppress outliers. Formula: `score = 100 * (winsor(x) - p5) / (p95 - p5)` for higher-better; `100 - that` for lower-better. | HIGH | Wikipedia (Feature_scaling, Winsorizing) |
| 3. Per-cell prose structure | **Card pattern: TL;DR (1 sentence) → Strengths (3–5 bullets, data-derived) → Weaknesses (2–3 bullets, data-derived) → Recommended-for / Avoid-for (1–2 bullets each).** Length: 80–150 words; fits in viewport without scroll on a 1080p screen. | MEDIUM | Lighthouse Opportunities/Diagnostics, AWS Trusted Advisor checks, web.dev Core Web Vitals |
| 4. Direction marker placement | **In column header only** — `Throughput ↑ (ops/s)`, `Latency p99 ↓ (ns)`. Do **not** decorate every cell. Add a 1-line legend ("↑ higher is better, ↓ lower is better") above each table and on each chart. Cells stay numeric only. | MEDIUM-HIGH | Wikipedia comparison tables (units-in-headers), datavizcatalogue.com |
| 5. Top-N count | **Top-3 charts above the fold + top-5 in the recommendations table; reveal top-10 only on demand.** Justified by Cowan's working-memory bound (4 ± 1 chunks). Not Miller's 7 ± 2. | HIGH | Wikipedia (Working_memory, Cowan revision) |
| 6. Mix measured + heuristic axes? | **Standard practice in MCDA (multi-criteria decision analysis).** Acceptable provided the heuristic axes are visually distinguished (e.g., dotted axis line / footnote marker) and a legend documents that "image-size efficiency" and "security posture" are scored qualitatively. | HIGH | Wikipedia (Multi-criteria_decision_analysis) |

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features the reader assumes a "v1.1 with recommendations and spider charts" must have. Missing these = the milestone fails its core promise of "tell the reader which cell to pick."

| Feature | Why Expected | Complexity | v1.0 Dependency | Notes |
|---|---|---|---|---|
| **Per-cell radar chart for each top-N (env, alloc) cell** | Reader expects "see the trade-offs at a glance" — single biggest UX delta over v1.0's tables. | **MEDIUM** | Extends `index.html.tmpl` Plotly section; reuses loaded `Run` set. New axis-extraction helper alongside `recommend.rs`. | Use `scatterpolar` + `fill: "toself"` + `opacity: 0.55–0.7` per Plotly's own polar-chart example. Accept the 8 axes the user proposed. |
| **0–100 normalization with direction-aware inversion** | Without normalization, throughput-in-ops/s and latency-in-ns can't share an axis. "Higher = better outward" is the universal radar-chart convention. | **MEDIUM** | New `crates/alloc-bench-aggregator/src/normalize.rs` module that consumes `&[Run]` and emits `BTreeMap<(env, alloc, axis), f64>`. Pinned by golden test. | Use min-max with **5th/95th percentile winsorization** (Wikipedia: Winsorizing). Raw min/max is brittle when one allocator crashes a single scenario. |
| **Direction markers in measurement column headers (REPORT.md + HTML)** | Readers cannot interpret "p99 = 5000" without "↓ better"; this is non-negotiable for honesty. | **LOW** | Pure render-time decoration — touches `markdown.rs::emit_per_scenario_tables` and the `index.html.tmpl` axis labels. | Place the marker in the **header only** (`Throughput ↑ (ops/s)`); cells stay numeric. Mirrors Wikipedia comparison-table convention. |
| **Per-cell recommendation prose card** | The whole milestone goal is "tell the reader which cell to pick" — bullet-only output is a cop-out. | **MEDIUM-HIGH** | Extends `recommend.rs` with a new `cell_recommendations(runs, top_n) -> Vec<CellRecommendation>`; emitted by both `markdown.rs` (Markdown card) and the HTML template (HTML card). Prose data-derived — no hard-coded sentences (RESEARCH §Pitfall 7, locked v1.0 convention). | Card structure: TL;DR (1 sentence) + Strengths + Weaknesses + Recommended-for / Avoid-for. |
| **Single-line legend at the top of each table and chart** | Otherwise readers don't know what `↑` / `↓` mean, especially on a static `file://` page with no tooltips. | **LOW** | Constant string emitted by `markdown.rs` and the HTML template. | E.g., `Legend: ↑ = higher is better, ↓ = lower is better. ⚠ = suspect or high-variance run (see definitions).` |
| **Suspect-flag propagation into spider charts and recommendation cards** | v1.0 already flags `⚠ suspect` rows; the radar layer cannot silently ignore them. | **LOW** | Reuse `html::is_suspect`; if any contributing run is suspect, badge the radar trace and prefix the prose card with `⚠ partial data — N of M scenarios suspect`. | Mirrors v1.0's `*(suspect)*` rationale-suffix pattern. |
| **Byte-identical output discipline preserved** | Hard requirement of the project; no exception for v1.1. | **LOW** | All new output uses `BTreeMap` / `BTreeSet`, fixed numeric formatting (e.g., `{:.1}` for normalized scores in `0.0–100.0`). Golden fixture in `tests/smoke.rs` regenerated once per the milestone spec. | The single timestamp-comment exception still applies. |

### Differentiators (Competitive Advantage Over v1.0 and Most Allocator Comparisons)

Features that make v1.1 noticeably better than ad-hoc allocator comparisons online (mimalloc-bench tables, jemalloc paper appendices, blog posts).

| Feature | Value Proposition | Complexity | v1.0 Dependency | Notes |
|---|---|---|---|---|
| **Small-multiples spider grid (top-3 cells in foreground, top-10 stacked below)** | Avoids the readability collapse Spotfire warns about ("more than three series should be presented on their own radar charts"). One chart per cell, laid out in a 3-column grid. | **MEDIUM** | New CSS grid block in `index.html.tmpl`; one `scatterpolar` div per cell with a shared layout config function. | Each chart shows the **chosen cell as a filled polygon (opacity 0.7) plus a thin "matrix-mean" reference polygon (opacity 0.25, no fill)** so the reader sees how the cell ranks vs. baseline. |
| **Reference baseline polygon on every spider chart** | "32% on memory, 89% on web" is meaningless without a baseline. The matrix-mean polygon makes "above/below average" instantly readable. | **LOW** | Computed in `normalize.rs::matrix_mean` — average of every cell's normalized score per axis. | This is also the only honest way to convey scale on heuristic axes (image-size, security) where absolute values are not meaningful. |
| **Composite weighted-sum overall score with weights surfaced in the card** | Per-cell ranking needs *some* scalar to break ties. Equal weights across the 8 axes (per the milestone spec); show the formula in the recommendation card. | **LOW** | Single Rust function `score(cell) = mean(axes)`. No weighting UI in v1.1; weights = `1/8` each. | Wikipedia MCDA confirms weighted-sum is the dominant scoring approach. **Surfacing the formula** is the differentiator vs. Lighthouse's hidden weights. |
| **"Why this cell?" data-derived rationale block in the prose card** | Any allocator dashboard can rank by throughput. The differentiator is *explaining the ranking* in the user's vocabulary ("wins web by +24% vs. runner-up; loses contention by −12%"). | **MEDIUM** | Extension of `recommend.rs::recommendations` to emit per-cell-not-per-class strings. Reuses the `+{delta:.1}% vs {runner_up} on {scenario}` template the v1.0 code already gates with unit tests. | This is the single biggest carrying differentiator. Refer to Pitfall §1 — never hard-code sentences. |
| **Visually distinct rendering for heuristic vs. measured axes** | Mixing `image-size efficiency` (curated heuristic) and `multithread throughput` (measured nanosecond data) on one radar chart is honest only if the difference is visible. Standard MCDA practice when blending objective and subjective criteria. | **LOW-MEDIUM** | Plotly `polar.angularaxis.tickfont` + per-tick label suffix `(heuristic)`. Optionally a dashed gridline radius for heuristic axes. | The 6 measured axes (channel/memory/web/multithread/cpu-bound/resilience) are clean; image-size and security carry a `(heuristic)` suffix. |
| **Pareto-front overlay on the Recommendations table** | A surprising finding that's table-stakes in MCDA: even the top-N can include strictly dominated cells. Marking the Pareto front filters out dead options. | **MEDIUM** | New `pareto.rs` module; emit a `pareto = true/false` column on the existing top-N table. | Optional for v1.1 but ranks high in differentiator value. Defer if Phase budget tight; document as v1.2 candidate. |

### Anti-Features (Commonly Requested, Often Problematic)

| Anti-Feature | Why Tempting | Why Problematic | Alternative |
|---|---|---|---|
| **One radar chart with all top-10 cells overlaid** | "Show everything in one place." | Datavizcatalogue / Spotfire / Wikipedia all warn that >3 overlapping polygons becomes "occluded and unreadable"; area distortion makes the largest cell visually swamp smaller ones. | **Small multiples**: one radar per cell, 3-up grid. |
| **Direction markers on every numeric cell (e.g., `12345 ↑`)** | "Make it impossible to misread." | Adds visual noise; doubles glyph count in dense REPORT.md tables; breaks the v1.0 byte-stable numeric formatting (`{:.1}` for throughputs, etc.). | Marker in **column header only**, plus a single legend line at the top of each table. |
| **Mixing perf and heuristic axes on the same scale without visual distinction** | "Just normalize everything to 0–100, problem solved." | Readers cannot tell a measured 87 from a curated 87 — destroys credibility when an allocator wins on a heuristic axis. | **Heuristic axes labeled `(heuristic)`** + dashed gridlines; explicit footnote linking to `meta/security/{env}.json` source. |
| **Hard-coded prose sentences in the recommendation card** | "Easier to write good copy." | Locked-down v1.0 convention (RESEARCH §Pitfall 7); hard-coded prose silently desynchronizes from the data. | All prose is template-substitution from `Run` data; the unit-test suite in `recommend.rs` already enforces this and is extended for cell prose. |
| **Top-10 charts shown above the fold** | "Top-10 is the canonical 'long list'." | Cowan's working-memory bound is 4 ± 1 chunks — readers cannot meaningfully compare 10 charts simultaneously. | **Top-3 in the foreground + top-5 in the table; top-10 in a collapsed/secondary section.** |
| **Raw min/max normalization (no winsorization)** | "It's the textbook formula." | One pathological run per axis (e.g., a crashed scenario reporting 0 ops/s) collapses the entire 0–100 range into a tiny band. The matrix becomes uninformative. | **5th/95th percentile winsorization** before min-max (Wikipedia: Winsorizing — preserves sample size, replaces tails with percentile boundaries). |
| **Z-score normalization to a [-3, +3] band rendered as 0–100** | "Statistically rigorous." | Z-scores are signed and unintuitive on a radar chart where 0 = origin. Mixed-direction handling (sign flip) compounds confusion. | Min-max-with-winsor; readers understand "0–100, higher is outward." |
| **Plotly radar charts on transparent backgrounds with default colors** | "Plotly defaults are good enough." | The v1.0 dashboard already has a deliberate color palette per allocator. Letting Plotly auto-choose breaks visual consistency between the bar charts and the new radar charts. | Reuse the existing per-allocator color map; set `fillcolor` and `line.color` to match. |
| **Asking the user to weight the 8 axes via a sidebar UI** | "Power users want control." | v1.0 dashboard is intentionally `file://`-deliverable, no JS state beyond Plotly's built-ins. A weighting UI requires custom state, breaking the single-page-static contract. Equal weights are explicitly in the milestone spec. | **Equal weights documented in the prose card.** v1.2+ candidate for a JS slider if reader feedback demands it. |

---

## Detailed Findings (one section per prompt question)

### 1. Spider/radar chart conventions

**Sources:** Wikipedia (Radar_chart), datavizcatalogue.com, Spotfire glossary, Storytelling-with-Data, Scott Logic critique, AWS QuickSight docs, Plotly.js polar-chart example.

**Axis count.**
- Wikipedia notes radar charts work best when "each variable corresponds to 'better' in some respect and all variables are on the same scale" but does not numerically cap axes.
- datavizcatalogue.com: "having too many variables creates too many axes and can also make the chart hard to read and complicated" — qualitative warning, no number.
- The clearest **upper-bound precedent** comes from sports analytics: FBref player scout reports use 8–10 percentile-normalized axes and have been the de facto standard since ~2017. Football Manager / FIFA-attribute radars use 6 axes. **8 axes sit at the upper edge of the readable band but inside it** — supported, not anti-pattern.
- Recommendation for v1.1: **accept 8 axes**, but pre-emptively address Scott Logic's critique ("introducing ordering would require the starting point and direction to be explicitly indicated") by **pinning the 8 axis order alphabetically and documenting it in a legend.**

**Axis labeling.**
- Plotly's `scatterpolar` with `theta: [...]` accepts string axis labels at fixed angles (per the Plotly.js polar-chart example). No autosizing — the template author controls placement.
- Convention: **two-word axis labels max, no units in the label** (units belong in the data table, not the radar). E.g., `Channel throughput`, not `Channel throughput (Mops/s, ↑ better)`.

**Legend placement.**
- Plotly.js: `showlegend: true` puts a legend top-right by default; supports `legend.orientation: 'h'` for horizontal under the chart.
- For small multiples (top-3 grid), a **shared legend above the grid** is the convention (datavizcatalogue.com small-multiples guidance).

**Overlapping shapes.**
- Spotfire is unambiguous: "More than three series should be presented on their own radar charts."
- Plotly's polar-chart docs show `opacity: 0.7` as the canonical translucent-fill value.
- Wikipedia: "Radar charts can also become hard to visually compare between different samples on the chart when their values are close as their lines or areas bleed into each other."
- Recommendation: **at most 2 polygons per chart (the cell + the matrix-mean reference); use small multiples for top-N comparisons.**

**Radial gridlines.**
- Standard convention is **5 evenly-spaced concentric circles** (0, 25, 50, 75, 100) to make the 0–100 scale legible.
- Plotly auto-renders radial axis ticks at sensible defaults; `polar.radialaxis.range: [0, 100]` and `polar.radialaxis.tickvals: [0, 25, 50, 75, 100]` give the canonical look.

### 2. 0–100 normalization for heterogeneous metrics

**Sources:** Wikipedia (Feature_scaling, Winsorizing, Multi-criteria_decision_analysis).

**Min-max formula.**
- Wikipedia (Feature scaling): `x' = (x - min(x)) / (max(x) - min(x))`. Scale by 100 for 0–100 output.
- For **lower-is-better metrics** (latency, RSS), invert: `score = 100 * (max - x) / (max - min)`. This is the standard approach in MCDA and is preferred over negating values.

**Outlier handling.**
- Raw min/max is **brittle**: one crashed run reporting 0 ops/s for throughput collapses the 0–100 range for that axis.
- Wikipedia (Winsorizing) describes the canonical alternative: replace values below the 5th percentile with the 5th-percentile value, and values above the 95th percentile with the 95th-percentile value. **"90% winsorization."** Sample size preserved.
- Recommendation: **winsorize at p5/p95, then min-max.** The formula:
  ```
  let p5 = percentile(values, 5);
  let p95 = percentile(values, 95);
  let clipped = x.clamp(p5, p95);
  let raw_score = 100.0 * (clipped - p5) / (p95 - p5);
  let score = if higher_is_better { raw_score } else { 100.0 - raw_score };
  ```
- Edge case: when p5 == p95 (all values equal), score is undefined; emit 50.0 (the neutral middle) per the v1.0 "undefined CV → em-dash" precedent.

**Heuristic axes (image-size, security posture).**
- Curated values are already on a small-integer scale (e.g., 1–5). Map deterministically to 0–100 by `score = 100 * (raw - 1) / 4` for a 1–5 scale; document this conversion in the meta-sidecar's prose.
- No winsorization needed for heuristics — the curator already chose the scale.

### 3. Per-cell recommendation prose

**Sources:** Lighthouse "Opportunities" + "Diagnostics" (2-section pattern), web.dev Core Web Vitals articles, AWS Trusted Advisor checks, hyperfine "Relative" output convention.

**Structural pattern.** Every authoritative recommendation card I found uses some variant of:

1. **What it is** (1 sentence)
2. **Why it matters / data finding** (1–3 sentences with measurements)
3. **What to do / when to use** (1–2 sentences, action-oriented)

Lighthouse adds an "Opportunities" (action) and "Diagnostics" (information) split; web.dev articles add a "How to measure" and "How to improve" pair. The common spine is **definition → diagnosis → action**.

**Recommended v1.1 structure (concrete outline, Markdown card form):**

```markdown
### {env} × {allocator} — overall score {score}/100

**TL;DR.** Wins {best_axis} (+{delta}% vs runner-up); loses {worst_axis} (-{delta}%).

**Strengths**
- Channel throughput: {score}/100 — {rationale derived from per-scenario data}
- Multithread: {score}/100 — {rationale}
- {1–3 more bullets, only the strengths above the matrix mean}

**Weaknesses**
- Memory: {score}/100 — {rationale}
- {1–2 more bullets, only the weaknesses below the matrix mean}

**Recommended for**
- {workload class derived from `recommend.rs::WorkloadClass`} — best score on {representative scenario}.

**Avoid for**
- {workload class where the cell scores below the matrix mean by ≥ 20 points}.
```

**Length.** 80–150 words. This fits in a 1080p browser viewport without scrolling alongside the radar chart and is short enough to scan in 30 seconds.

**Card vs. paragraph.** A **card** is unambiguously the right format — bullet structure mirrors Lighthouse's audit cards, AWS Trusted Advisor check rows, and Snyk vulnerability summaries. Paragraph prose discourages scanning.

**Data-derivation contract.** Every sentence template-substitutes from the `Run` data; **no hard-coded copy** (RESEARCH §Pitfall 7, locked v1.0 convention). The `+{delta:.1}% vs {runner_up} on {scenario}` substring already in `recommend.rs` is the canonical example.

### 4. Direction markers (↑ / ↓)

**Sources:** Wikipedia comparison tables (TLS implementations, RDBMS), datavizcatalogue.com, hyperfine output convention.

**Placement: column header only.**
- Wikipedia comparison tables consistently put **units in column headers** ("Max DB size", "Latest stable version, release date") and **never** decorate every cell.
- The reader reads the header once and applies the direction across the column.
- Decorating every cell adds ~20% horizontal width to the v1.0 REPORT.md tables, which already pack densely; the byte-stable numeric formatting `{:.1}` doesn't have visual room for an extra glyph.

**Convention.**
- `↑` = higher is better (throughput, ops/s, channel throughput score)
- `↓` = lower is better (latency p50/p95/p99/p999, peak RSS, image size MB, build time)
- Some dashboards use `▲` / `▼` (filled triangles); the up-down arrows ↑/↓ are more accessible on monospace renderings (terminals viewing REPORT.md).

**Legend.** Single line above each table:

> Legend: `↑` = higher is better, `↓` = lower is better. `⚠ suspect` rows have `samples_count < 1_000` or `warmup_duration_s < 5.0`.

**One worth-diverging convention.** Don't follow the mid-2000s sparkline trend of green-up-arrow / red-down-arrow with absolute deltas embedded in the cell. That's a financial-dashboard pattern, not a benchmark-comparison one, and would clash with the v1.0 "winner highlighted in bold" convention. Stick to the header-only marker.

### 5. Top-N count

**Sources:** Wikipedia (Working_memory, Cowan revision; Miller's_law).

**Cowan's bound (4 ± 1) is the modern accepted limit.** Miller's 7 ± 2 from 1956 was revised down by Cowan in the early 2000s; Wikipedia's Working_memory article notes the modern figure is "approximately four chunks in young adults." This is decisive for the top-N question.

**Diminishing-returns reasoning.**
- **Top-3** is the densest, scan-friendly count. All three radar charts visible above the fold on a 1080p screen. Each chart gets a paragraph of prose.
- **Top-5** still fits but requires either smaller charts or a 2-row grid; the marginal recommendation value (cells 4 and 5) is small relative to the cognitive cost.
- **Top-10** exceeds Cowan's 4 ± 1 — readers cannot meaningfully compare 10 cells simultaneously. They will skim the first 3, ignore the rest.

**Recommendation for v1.1.**
- **Top-3 spider charts in the foreground** (above the multi-select grid).
- **Top-5 cells in the Recommendations table** (the existing v1.0 table, extended with the new score column and Pareto-front column).
- **Top-10 cells reachable on demand** (e.g., a `<details>`-collapsed "Full top-10" section below the fold). The milestone spec ("top-10 (env, allocator) cells") is honored — top-10 is the *generation* set; top-3 is the *display* set above the fold.

This is also a natural fit for the existing `recommend.rs::recommendations` machinery: extend the function to take an optional `n: usize` parameter and emit `Vec<CellRecommendation>` of length n.

### 6. Mixing measured (perf) and heuristic (image-size, security) axes

**Sources:** Wikipedia (Multi-criteria_decision_analysis, Pareto_efficiency).

**Verdict: standard practice in MCDA, with caveats.**

**Direct evidence.** Wikipedia MCDA states that the framework "explicitly handles multiple input types across various schools of thought" and "treats both quantitative performance metrics and qualitative judgments as valid criterion inputs within the same analysis." Multi-attribute utility theory specifically combines elicited preferences with measured data.

**Why it works.** Once everything is normalized to 0–100, the unit difference disappears at the visualization layer. The radar's job is to show **trade-offs**, not absolute units; mixing measured and heuristic is acceptable as long as the reader understands the heuristic is curated.

**Why it can mislead.** A reader glancing at the chart can't tell a curated 87 (security) from a measured 87 (throughput). Solutions seen in MCDA literature:
1. **Visual distinction.** Heuristic axes get a different gridline style (dashed) or a different label color.
2. **Explicit `(heuristic)` suffix on the axis label.** E.g., `Security posture (heuristic)`.
3. **A footnote linking to the source data.** The milestone spec already requires `meta/security/{env}.json` — link the suffix to the file path.

**Recommendation for v1.1.**
- **Mix is acceptable.** The MCDA precedent is strong.
- **Visually distinguish heuristic axes** (suffix `(heuristic)` + dashed gridline at that radius).
- **Document the curation source** in the legend below each radar chart and in the prose card.
- **Pareto-front filtering** (see Differentiators) handles the worst case: if a cell is dominated on every measured axis but wins on heuristic, it shouldn't be in the top-N anyway.

**Anti-pattern to avoid.** Letting a heuristic axis silently swing the score by ≥ 25/100. With equal weights across 8 axes, each axis can move the overall score by up to 100/8 = 12.5 points. That's at the boundary of "noticeable but not dominant" — acceptable. If weights ever become non-uniform in v1.2+, cap the aggregate weight of heuristic axes at ≤ 25% of the total.

---

## Feature Dependencies

```
[Direction markers in REPORT.md and HTML]                          ─── independent (LOW complexity)
    └── enables → [Per-axis labeling in radar charts]

[meta/security/{env}.json sidecars]                                ─── independent (LOW complexity)
    └── enables → [Security-posture heuristic axis on the radar]

[normalize.rs (min-max + winsorize, direction-aware)]               ─── new module
    ├── consumed-by → [Spider chart axis values]
    ├── consumed-by → [Per-cell composite score]
    └── consumed-by → [Reference baseline (matrix-mean) polygon]

[Per-cell composite score]                                          ─── depends on normalize.rs
    ├── consumed-by → [Top-N ranking in extended recommend.rs]
    └── consumed-by → [TL;DR / Recommended-for / Avoid-for prose]

[Extended recommend.rs::cell_recommendations]                       ─── depends on Per-cell composite score
    ├── emitted-by → [REPORT.md card section]
    └── emitted-by → [HTML template card section]

[Spider chart Plotly traces]                                        ─── depends on normalize.rs
    └── rendered-by → [index.html.tmpl small-multiples grid]

[Suspect-flag propagation]                                          ─── reuses html::is_suspect
    ├── consumed-by → [Radar trace badge]
    └── consumed-by → [Prose card prefix]

[Pareto-front overlay]                                              ─── depends on normalize.rs (DIFFERENTIATOR)
    └── consumed-by → [Recommendations table column]
```

### Dependency Notes

- **`normalize.rs` is the keystone.** Every new visual artifact depends on its output. Build it first; gate it with golden-value unit tests (analogous to the `[100, 110, 105]` → CV ≈ 4.7619% pin in `multi_run.rs`).
- **`recommend.rs` is extended, not duplicated.** Single source of truth for prose stays put per the locked v1.0 convention.
- **Direction markers are dependency-free.** They can ship in a stand-alone phase ahead of the radar work — useful as a low-risk warm-up plan.
- **`meta/security/{env}.json` is hand-curated.** It blocks the security axis but nothing else; non-security parts of the radar can ship without it (using only 7 axes initially) if curation slips.

---

## Per-Feature User Behavior (downstream-consumer requirement)

| Feature | Trigger | Expected behavior |
|---|---|---|
| **Top-3 spider charts above the fold** | User opens `report/index.html`. | Three radar charts render at the top of the page in a 3-column responsive grid. Each chart shows the cell's polygon (filled, opacity 0.7, allocator color from the v1.0 palette) plus the matrix-mean reference polygon (no fill, opacity 0.25, dashed line). Title above each chart: `{env} × {allocator} — Score {score}/100`. |
| **Click a spider chart** | User clicks anywhere on a top-3 chart. | Page scrolls to the cell's prose card section below. Card includes TL;DR + Strengths + Weaknesses + Recommended-for + Avoid-for. (Native HTML `<a href="#cell-{env}-{alloc}">` — no JS.) |
| **Hover on a radar polygon vertex** | User hovers an axis vertex on a chart. | Plotly's built-in hover popup shows `{axis-name}: {score}/100` (Plotly default behavior, no custom JS). |
| **Recommendations table** | User scrolls past the spider charts. | Existing v1.0 Recommendations table renders, extended with: (a) `Score` column; (b) `Pareto front` column; (c) the rationale-prose cell now reads `+{delta}% on {best_scenario}; -{delta}% on {worst_scenario}`. |
| **REPORT.md per-scenario tables** | User opens `report/REPORT.md`. | Every measurement column header carries a direction marker: `Throughput ↑ (ops/s)`, `Latency p99 ↓ (ns)`, `Peak RSS ↓ (MB)`. Above each table, a single legend line: `Legend: ↑ = higher is better, ↓ = lower is better. ⚠ = suspect or high-variance.` |
| **REPORT.md per-cell prose card section** | User scrolls to the new "Per-cell recommendations" section. | One Markdown card per top-5 cell, rendering the same TL;DR + bullets as the HTML version. Cells are alphabetically ordered for byte-stable output (BTreeMap iteration). Cells flagged `⚠ partial data — N/M scenarios suspect` if any contributing run is suspect. |
| **Top-10 collapsed section** | User clicks `<details>` "Full top-10 ranking" below the fold. | The remaining 5 cells (positions 6–10) appear as compact rows: env × alloc, score, one-line rationale. No charts at this depth — top-10 is data, not display. |
| **Spider chart rendering on a heuristic axis** | User reads the radar. | The `Image-size efficiency` and `Security posture` axis labels carry the suffix `(heuristic)`. The angular gridline at that radius is dashed (vs. solid for measured axes). Footnote in the figure caption links to `meta/security/{env}.json`. |
| **No JavaScript / no server** | User opens the dashboard via `file://`. | Everything works. Plotly is loaded via CDN per v1.0 (`subresource-integrity` hash + `crossorigin=anonymous`); no extra JS for the v1.1 features. Click-to-scroll uses native HTML anchors. |

---

## MVP Definition

### Launch With (v1.1)

- [ ] **Direction markers in column headers** — REPORT.md + HTML — LOW complexity, zero dependencies.
- [ ] **`normalize.rs` module** with min-max + p5/p95 winsorization + direction-aware inversion — gated by golden-value unit test.
- [ ] **Top-10 ranking via extended `recommend.rs::cell_recommendations(runs, 10)`** — reuses single-source-of-truth contract from v1.0.
- [ ] **Top-3 spider charts above the fold** in the HTML dashboard, with matrix-mean reference polygons.
- [ ] **Per-cell prose card** for the top-5 cells (REPORT.md + HTML), data-derived, including suspect-flag propagation.
- [ ] **`meta/security/{env}.json`** sidecars (6 files, hand-curated).
- [ ] **Heuristic-axis visual distinction** (label suffix + dashed gridline).
- [ ] **Single-line legend** above each table and chart.
- [ ] **Golden fixture regeneration** in `tests/smoke.rs` (once, then byte-stable).

### Add After Validation (v1.2)

- [ ] **Pareto-front overlay** on the Recommendations table — DIFFERENTIATOR, defer if budget tight.
- [ ] **JS slider for axis weighting** — only if reader feedback demands it; breaks the static-HTML contract.
- [ ] **Per-axis tooltip with raw and normalized values** — currently Plotly's default hover is sufficient.
- [ ] **Drill-in "deep dive" page per top-3 cell** — rendered as a separate `report/cell-{env}-{alloc}.html`.

### Future Consideration (v2+)

- [ ] **Live re-weighting from the dashboard** with a server-side recompute path — out of scope for static HTML.
- [ ] **Cross-version diff radar** ("v1.0 → v1.1 score change per axis") — needs persisted historical results.

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---|---|---|---|
| Direction markers in headers | HIGH | LOW | **P1** |
| `normalize.rs` module + winsor | HIGH | MEDIUM | **P1** |
| Top-3 spider charts above the fold | HIGH | MEDIUM | **P1** |
| Per-cell prose cards (top-5) | HIGH | MEDIUM-HIGH | **P1** |
| `meta/security/*.json` sidecars | MEDIUM | LOW (curation) | **P1** |
| Heuristic-axis visual distinction | MEDIUM | LOW | **P1** |
| Single-line legend | MEDIUM | LOW | **P1** |
| Reference baseline polygon | MEDIUM | LOW | **P1** |
| Suspect-flag propagation | MEDIUM | LOW | **P1** |
| Pareto-front overlay | MEDIUM | MEDIUM | **P2** |
| Top-10 collapsed `<details>` | LOW-MEDIUM | LOW | **P2** |
| JS axis-weighting slider | LOW | HIGH | **P3** |

**Priority key:** P1 = ship in v1.1; P2 = ship if budget permits, otherwise v1.2; P3 = defer to v2+.

---

## Competitor Feature Analysis

| Feature | mimalloc-bench | jemalloc paper | TechEmpower | hyperfine | **Our v1.1 plan** |
|---|---|---|---|---|---|
| Per-allocator radar | No (tables only) | No (bar charts) | No (bar charts) | No | **Yes — top-3 above fold** |
| Direction markers | No | No | No | "Relative" column | **Yes — column headers** |
| 0–100 normalization | No | No | No | Relative-to-fastest | **Yes — winsor + min-max** |
| Per-cell prose | No | Yes (paper text) | No | No | **Yes — auto-generated card** |
| Pareto front | No | No | No | No | **Yes — P2 differentiator** |
| Heuristic axes | No | No | No | No | **Yes — image-size + security** |
| Multi-run statistics | No | No | Implicit | Yes | **Yes — already in v1.0** |
| Static HTML, file:// | N/A | N/A | Server-rendered | N/A | **Yes — preserved from v1.0** |

The synthesis is striking: **no competitor in the allocator-comparison space combines per-cell radars with auto-generated prose recommendations**. v1.1 is genuinely novel.

---

## Sources

**Authoritative (HIGH confidence):**
- Wikipedia, _Radar chart_ — axis count constraints, area distortion, comparison limitations.
- Wikipedia, _Feature scaling_ — min-max formula `x' = (x - min(x)) / (max(x) - min(x))`.
- Wikipedia, _Winsorizing_ — 5th/95th percentile clipping ("90% winsorization"), preserves sample size.
- Wikipedia, _Multi-criteria decision analysis_ — mixing objective + subjective metrics is standard, weighted-sum scoring is canonical.
- Wikipedia, _Pareto efficiency_ — direction-aware comparison via explicit min/max formulation, not negation.
- Wikipedia, _Working memory_ — Cowan revision (4 ± 1 chunks), modern bound on simultaneous comparison.
- Wikipedia, _Miller's law_ — historical context for the 7 ± 2 rule (now superseded).
- Wikipedia, comparison tables for _TLS implementations_ and _RDBMS_ — units-in-headers convention; no per-cell direction markers.

**Strong supporting (MEDIUM-HIGH confidence):**
- The Data Visualisation Catalogue (datavizcatalogue.com), _Radar Chart_ — "too many variables creates too many axes," overlap is "overcluttered."
- Spotfire glossary, _What is a Radar Chart_ — "More than three series should be presented on their own radar charts."
- Storytelling-with-Data, _What is a Spider Chart_ — small multiples or transparency for multi-series; readability concerns for general audiences.
- Scott Logic, _A Critique of Radar Charts_ — area-distortion problem, ordering ambiguity, parallel coordinates as alternative.
- AWS QuickSight docs, radar-chart reference — mainstream BI tool's radar implementation, includes color and category axes.

**Implementation references (HIGH confidence for syntax):**
- Plotly.js polar-chart and radar-chart docs — `scatterpolar`, `fill: "toself"`, `opacity: 0.7` as canonical multi-series fill.
- hyperfine README — "Relative" column convention (1.00x = fastest baseline) for direction-aware comparison.
- web.dev LCP article — `What is X / How to measure / How to improve` recommendation-card pattern.
- Lighthouse scoring docs — Opportunities + Diagnostics two-section pattern for performance recommendations.

**Existing project context (canonical):**
- `crates/alloc-bench-aggregator/src/recommend.rs` — single-source-of-truth contract for data-derived prose; the `+{delta:.1}% throughput vs {runner_up} on {scenario}` template; alphabetical class iteration via `BTreeMap` for byte-stable output.
- `CLAUDE.md` Conventions — aggregator decorate-not-rewrite; multi-run statistics convention; byte-identical output discipline; suspect-flag definition (`samples_count < 1_000` OR `warmup_duration_s < 5.0`).
- `.planning/PROJECT.md` Current Milestone section — milestone scope and the eight-axis selection.
- `.planning/milestones/v1.0-research/FEATURES.md` — v1.0 baseline so this document doesn't duplicate.

---

## Confidence Assessment

| Area | Confidence | Notes |
|---|---|---|
| Spider/radar conventions (8 axes, small multiples for >3 series) | HIGH | Wikipedia + datavizcatalogue + Spotfire all converge on the same advice. |
| Min-max with p5/p95 winsorization | HIGH | Wikipedia gives the canonical formulas for both pieces. Direction-aware inversion is unanimous in MCDA literature. |
| Direction markers in column header only | MEDIUM-HIGH | Wikipedia comparison-table convention is the dominant industry precedent; `↑`/`↓` arrows are a stylistic choice, not a hard standard. |
| Per-cell prose card structure (TL;DR + bullets) | MEDIUM | Multiple authoritative dashboards converge on the same general shape, but no single canonical specification — synthesis from Lighthouse + AWS Trusted Advisor + web.dev. |
| Top-3 above the fold, top-10 generation, top-5 in table | HIGH | Cowan's 4 ± 1 working-memory bound is the modern accepted limit. |
| Mixing measured + heuristic axes | HIGH | MCDA explicitly endorses this; visual distinction is the hedge. |

**Overall confidence: HIGH-MEDIUM.** The contentious parts of the spec (8 axes, mixed perf+heuristic) have strong supporting precedents; the visual conventions (winsor cutoffs, opacity values, top-N display) are inferred from multiple sources but not from a single authoritative spec.

---

*Feature research for: rust-benchmark-glibc-musl-mimalloc v1.1 milestone (Recommendations, Spider Charts & Direction Markers)*
*Researched: 2026-05-26*

---
phase: 9
slug: spider-chart
audited: 2026-05-28
baseline: UI-SPEC.md (approved)
screenshots: not captured (no dev server running)
---

# Phase 9 — UI Review

**Audited:** 2026-05-28
**Baseline:** 09-UI-SPEC.md design contract
**Screenshots:** not captured (no dev server at localhost:3000/5173/8080 — code-only audit)

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 4/4 | All spec-locked strings verbatim; interpolated `Matrix mean (n=N)` correctly replaces hard-coded literal |
| 2. Visuals | 2/4 | Single overlay chart shipped instead of 3 small-multiples; no per-chart `{alloc}/{env}` title; no `<div class="spider-cell">` divs |
| 3. Color | 2/4 | Heuristic-axis `tickfont.color` is a single global `#666` instead of the spec-required per-tick `['#222','#222','#666',...]` array; caption uses `--color-text-muted` (#656D76) instead of spec-required `--color-text` (#1F2328) |
| 4. Typography | 3/4 | h2 margin-bottom uses `--space-md` (16px) instead of spec-required `--space-sm` (8px); per-chart Plotly title is absent from layout JSON |
| 5. Spacing | 2/4 | `.spider-grid` lacks `display:flex`, `flex-wrap:wrap`, and `gap:var(--space-md)`; `.spider-cell` lacks `flex` shorthand, `aspect-ratio:1/1`, `min-width:280px`; no mobile `@media` override for spider cells |
| 6. Experience Design | 3/4 | Empty-state gate (`has_spider`) works; Pareto full-sweep semantics correct; `pareto-cell` CSS for `text-align:center` missing |

**Overall: 16/24**

---

## Top 3 Priority Fixes

1. **Single overlay chart instead of three small-multiples** — the spec requires three `<div class="spider-cell">` containers each receiving an independent `Plotly.react` call with one cell trace + one reference trace. What shipped is a single `<div id="chart-spider">` receiving all four traces overlaid in one polar plot. Users cannot compare allocators side-by-side; the reference polygon and three cell polygons collapse into a single cluttered chart with no per-cell title. Fix: add three `<div class="spider-cell" id="spider-cell-N">` inside `<div id="chart-spider" class="spider-grid">` in `index.html.tmpl`, add a per-cell Plotly.react bootstrap loop in a `<script>` tag, and pass `spider_cells_json: Vec<String>` (each element a two-trace array of reference + one cell) from `html.rs::build_spider_context`.

2. **`.spider-grid` and `.spider-cell` CSS missing layout properties** — `.spider-grid` renders as a block element with no flex or grid structure, so all three future spider cells would stack vertically at 100% width rather than sitting side-by-side in a 3-column row. `.spider-cell` has no `flex` shorthand, `aspect-ratio:1/1`, or `min-width:280px`, so cells will not be square and will collapse to minimum content height. Fix: replace `.spider-grid { width: 100%; min-height: 480px; }` with `display: flex; flex-wrap: wrap; gap: var(--space-md);` and replace `.spider-cell { width: 100%; min-height: 320px; }` with `flex: 1 1 calc((100% - 2 * var(--space-md)) / 3); aspect-ratio: 1 / 1; min-width: 280px; background: var(--color-dominant); border: 1px solid var(--color-border); border-radius: 6px; padding: var(--space-md);`. Add the mobile override `@media (max-width: 768px) { .spider-cell { flex-basis: 100%; } .spider-chart { margin: var(--space-md); } }` to the existing `@media (max-width: 768px)` block.

3. **Heuristic-axis tickfont.color is a global `#666` instead of a per-tick array** — the spec explicitly requires the angular axis tickfont to use an array `['#222','#222','#666','#222','#222','#222','#666','#222']` so only indices 2 and 6 (heuristic axes) appear muted while all six real-measurement axes render in the high-contrast `#222`. What shipped applies `#666` to every axis label, degrading contrast on the six real-measurement axes from 16.0:1 to 5.7:1 — sufficient for WCAG AA but removing the intended visual distinction between heuristic and real axes that is the entire point of POLAR-03. Fix: in `html.rs::build_spider_context`, compute the `tickfont.color` array by iterating `MEASUREMENT_AXES` and emitting `"#666"` for `is_heuristic == true`, `"#222"` otherwise; replace the scalar `"color": "#666"` field with this array.

---

## Detailed Findings

### Pillar 1: Copywriting (4/4)

Every spec-locked string is present verbatim:

- Section heading: `<h2>Top-3 Above the Fold</h2>` at `index.html.tmpl:298` — exact bytes match spec.
- Section caption: `Spider charts of the top-3 cells across 8 normalized axes (0-1). Grey reference polygon = mean across all 18 cells.` at `index.html.tmpl:299` — verbatim match (byte-stable golden input for Phase 11).
- Reference polygon name: `format!("Matrix mean (n={n})")` at `polar.rs:156` — correctly interpolates `scores.len()` instead of hard-coding `18` (WR-01 fix confirmed; test `reference_trace_name_interpolates_zero_for_empty_input` pins the degenerate boundary).
- Pareto column header: `"| Rank | Cell | Score | Pareto |"` at `markdown.rs:431` — exact 6-byte `Pareto` header.
- Pareto glyph: `"\u{2605}"` at `markdown.rs:434` — U+2605 BLACK STAR, correct.
- Per-cell template headings: both `recommend-cell.html.tmpl:2` and `recommend-cell.md.tmpl:1` carry `{{ if is_pareto }} ★{{ endif }}` after the existing suspect annotation — byte-identical across both surfaces.
- Angular tick labels: `axis_label_for_chart` at `polar.rs:63-69` correctly appends ` (heuristic)` (11-byte suffix with leading U+0020) for `image_size_efficiency` and `security_posture` only; plain label for the other six axes; confirmed by tests at lines 187-252.
- Empty Pareto cell: `""` (empty string) used in `markdown.rs:434` for non-front cells — renders as visually blank in GFM table. Correct per spec.

No generic labels, no placeholder strings, no "No data" copy that should have been absent.

### Pillar 2: Visuals (2/4)

**BLOCKER — single overlay chart instead of three small-multiples.**

The spec (`09-UI-SPEC.md` Layout section, `09-CONTEXT.md` decision row "Small-multiples grid") requires:

```html
<div id="chart-spider" class="spider-grid">
  <div class="spider-cell" id="spider-cell-1"></div>
  <div class="spider-cell" id="spider-cell-2"></div>
  <div class="spider-cell" id="spider-cell-3"></div>
</div>
```

Each `.spider-cell` receives its own `Plotly.react` call with two traces: the matrix-mean reference polygon FIRST, then that cell's polygon. This is the "above the fold" design contract — three side-by-side 340x340px radar charts, each comparing one top-N cell against the grey reference.

What was built:

```html
<div id="chart-spider" class="spider-grid"></div>
<script>
  Plotly.react('chart-spider',
    { spider_traces_json | unescaped },  // one reference + all 3 cell traces
    { spider_layout_json | unescaped },
    \{ responsive: true });
</script>
```

This renders a single polar chart with four overlaid traces. The user sees one cluttered polygon cluster rather than three separate visual comparisons. There are zero `<div class="spider-cell">` divs in the template (`grep` confirms 0 matches).

Confirmed in `09-03-SUMMARY.md`: "A `<section class='spider-chart'>` with `<div id='chart-spider'>` renders 4 Plotly scatterpolar traces: 1 matrix-mean reference (grey 25% alpha) + 3 top-cell polygons." This description, while matching what was built, diverges from both CONTEXT.md and UI-SPEC.md, which explicitly required three separate charts.

**Per-chart title absent from layout JSON.** The spec requires each chart to carry `layout.title.text = "{alloc}/{env}"` with `font: { size: 14, color: '#1F2328' }`. The server-rendered layout JSON at `html.rs:424-445` contains no `"title"` field. In the single-overlay approach there is no per-chart title at all; in the intended three-chart approach each chart would need its own title passed at render time. This is a secondary consequence of the single-chart implementation divergence.

**WARNING — `.spider-cell` CSS class defined but never instantiated.** The style block at `index.html.tmpl:242-245` defines `.spider-cell { width: 100%; min-height: 320px; }` but the template contains zero `.spider-cell` div elements. The CSS rule is orphaned.

**PASS — section ordering.** `<section class="spider-chart">` at line 297 is correctly positioned after `<section class="top-n-recommendations">` (line 288) and before the main `<script>` block, matching the spec's eye-path contract.

**PASS — semantic structure.** `<section>`, `<h2>`, `<p>` tags are correct; h1 > h2 heading hierarchy is maintained; no skipped levels.

**PASS — Plotly.react used, not newPlot.** The `aggregator_html_uses_plotly_react_not_newplot` smoke test enforces this.

### Pillar 3: Color (2/4)

**WARNING — heuristic-axis tickfont is a global `#666`, not the per-tick array.**

The spec (`09-UI-SPEC.md` Color section and Per-chart Plotly layout section) defines:

```js
"tickfont": {
  "size": 11,
  "color": ['#222', '#222', '#666', '#222', '#222', '#222', '#666', '#222']
}
```

What was built (`html.rs:434`):

```json
"tickfont": { "size": 11, "color": "#666" }
```

This mutes all eight axis labels to `#666` (5.7:1 contrast on white — WCAG AA at 11px). The six real-measurement axes should render at `#222` (16.0:1 — WCAG AAA) to visually distinguish them from the two heuristic axes. The `(heuristic)` suffix is present in the label text (POLAR-03 satisfied) but the secondary color cue is applied uniformly, removing the two-tier distinction.

**WARNING — caption text color uses `--color-text-muted` instead of `--color-text`.**

CSS at `index.html.tmpl:234-237`:
```css
.spider-chart p {
  color: var(--color-text-muted);  /* #656D76, 5.7:1 on white */
```

Spec (`09-UI-SPEC.md` Layout §CSS skeleton):
```css
.spider-chart > p {
  color: var(--color-text);  /* #1F2328, 14.4:1 on white */
```

The spec considered using muted for the caption (noted in the Color section: "current spec uses default text color for caption to keep the scan-path emphasis on the chart grid") and resolved to use `--color-text`. The implementation chose the muted variant. This reduces caption contrast from 14.4:1 to 5.7:1 — still WCAG AA compliant but inconsistent with the locked spec decision.

**PASS — reference polygon alpha values correct.** `fillcolor: "rgba(128,128,128,0.25)"` and `line.color: "rgba(128,128,128,0.5)"` are verbatim at `polar.rs:154-155`, locked by test `reference_trace_carries_25_percent_alpha_fill_and_50_percent_alpha_stroke`.

**PASS — 60/30/10 distribution.** `.spider-chart` and `.spider-cell` backgrounds use `var(--color-dominant)` (#FFFFFF). Borders use `var(--color-border)` (#E1E4E8). Accent `--color-accent` (#0969DA) is not used on any Phase 9 surface (confirmed by grep — only sidebar focus outline retains it, unchanged from prior phases).

**PASS — Plotly gridcolor/linecolor in layout JSON.** Both `radialaxis` and `angularaxis` specify `gridcolor: "#E1E4E8"` and `linecolor: "#E1E4E8"`, matching `--color-border`. Chart paper and plot backgrounds are `"#ffffff"`, matching `--color-dominant`. These are Plotly trace literals, not CSS variables, consistent with the spec's documented exception.

**PASS — `.pareto-cell` CSS absent but acceptable.** The spec declared `.report-mirror td.pareto-cell { text-align: center; }`. The HTML `top-n-recommendations` section renders card elements, not a `<table>`, so the CSS rule would apply to the JS-rendered per-scenario tables (which have no Pareto column) or the markdown-mirrored block (not present in the HTML). Since no HTML `<td class="pareto-cell">` elements are ever emitted, the missing CSS rule has no current visual effect.

### Pillar 4: Typography (3/4)

**WARNING — h2 heading margin-bottom is `--space-md` (16px) instead of spec `--space-sm` (8px).**

CSS at `index.html.tmpl:229-233`:
```css
.spider-chart h2 {
  margin: 0 0 var(--space-md) 0;  /* 16px */
```

Spec (`09-UI-SPEC.md` §CSS skeleton):
```css
.spider-chart h2 {
  margin: 0 0 var(--space-sm) 0;  /* 8px */
```

The heading sits 16px above the caption instead of 8px, which pushes the caption (and the chart below it) further down. The spacing gap between the section heading and the caption is doubled relative to spec. All other heading properties (`font-size: var(--font-size-heading)`, `font-weight: var(--font-weight-semibold)`) are correct.

**WARNING — per-chart title absent.** The spec requires `layout.title.text = "{alloc}/{env}"` with `font: { size: 14, color: '#1F2328' }` on each of the three polar charts. Because the implementation uses a single chart with all traces overlaid, no per-chart title exists. When the three-multiples architecture is fixed (Priority Fix #1), this title will also need to be injected — currently there is no code path that would produce a per-chart `layout.title`.

**PASS — section heading size and weight.** `font-size: var(--font-size-heading)` (20px) and `font-weight: var(--font-weight-semibold)` (600) match the spec table for the h2 heading row.

**PASS — angular tick size.** `tickfont.size: 11` in `html.rs:434` matches spec (11px for all axis tick labels).

**PASS — only two font sizes introduced.** The spider CSS adds `font-size: var(--font-size-heading)` for h2 and inherits `var(--font-size-body)` (14px) for the caption. The Plotly tick labels use 11px inside the SVG. No new tokens introduced.

**PASS — font family.** The layout JSON contains no explicit `font.family` override. The Plotly chart will inherit from the existing `SHARED_FONT` const via the global `PLOTLY_CONFIG` pattern already established — consistent with the spec's statement "Plotly font-family override matches via the existing `SHARED_FONT` const."

### Pillar 5: Spacing (2/4)

**WARNING — `.spider-grid` missing flex/gap layout properties.**

Spec (`09-UI-SPEC.md` §CSS skeleton):
```css
.spider-grid {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-md);
}
```

Actual (`index.html.tmpl:238-241`):
```css
.spider-grid {
  width: 100%;
  min-height: 480px;
}
```

`display: flex` and `gap: var(--space-md)` are absent. When three `.spider-cell` divs are added (Priority Fix #1), they will not form a three-column row — they will stack vertically as block elements.

**WARNING — `.spider-cell` missing flex shorthand, aspect-ratio, and min-width.**

Spec (`09-UI-SPEC.md` §CSS skeleton):
```css
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

Actual (`index.html.tmpl:242-245`):
```css
.spider-cell {
  width: 100%;
  min-height: 320px;
}
```

Five of eight spec properties are absent: `background`, `border`, `border-radius`, `padding` (the card framing), and the three layout properties (`flex`, `aspect-ratio`, `min-width`). The `width: 100%; min-height: 320px` fallback means cells would never be square and would not receive the `.chart-card`-style border framing.

**WARNING — no mobile `@media` rule for spider cells.**

The spec requires:
```css
@media (max-width: 768px) {
  .spider-cell { flex-basis: 100%; }
  .spider-chart { margin: var(--space-md); }
}
```

The existing `@media (max-width: 768px)` block at `index.html.tmpl:146-158` handles `.page` and `main.charts` only — no `.spider-chart` or `.spider-cell` rules. On mobile viewports the section margin would remain `var(--space-2xl) var(--space-lg) var(--space-xl)` instead of collapsing to `var(--space-md)`.

**PASS — Plotly chart internal margins.** `{ "l": 30, "r": 30, "t": 40, "b": 30 }` at `html.rs:441` matches the spec value exactly. These are Plotly-internal pixels documented as a chart-frame exception to the 4px scale.

**PASS — `.spider-chart` outer container spacing.** `margin: var(--space-2xl) var(--space-lg) var(--space-xl)`, `padding: var(--space-md)`, `border: 1px solid var(--color-border)`, `border-radius: 6px`, `background: var(--color-dominant)` all use CSS tokens from the existing spacing scale. Matches spec.

**PASS — no hardcoded arbitrary spacing.** No `[Npx]` or `[Nrem]` Tailwind-style arbitrary values. The non-scale `480px` and `320px` values on `.spider-grid` and `.spider-cell` are raw pixel values but they are documented as matching the existing `.chart-card min-height: 480px` precedent; they are workarounds in the absence of the proper flex layout, not new tokens.

### Pillar 6: Experience Design (3/4)

**PASS — empty-state gate works.** `{{ if has_spider }}` at `index.html.tmpl:297` correctly omits the entire `<section class="spider-chart">` block when `top_n.is_empty()`. Test `spider_section_absent_when_top_n_empty` passes. The empty path preserves v1.0 byte-identical output (no spider section bytes emitted).

**PASS — Pareto full-sweep semantics correct (WR-03 fixed).** `recommend.rs:654` computes `score::pareto_front` on the FULL `scores` slice BEFORE `top_n` truncation, so `is_pareto` carries global Pareto-optimal semantics, not truncated-top-10 semantics. Test `top_n_cells_pareto_front_uses_full_sweep_not_truncated_top_n` locks this.

**PASS — Plotly SRI hash pinned.** `PLOTLY_SRI_HASH` at `html.rs:67-68` is byte-pinned by `plotly_sri_hash_unchanged` (unit test) and `plotly_sri_hash_unchanged_full_string` (smoke test). CDN URL pinned at `html.rs:60`. Anti-supply-chain-attack gate in place.

**PASS — WR-01 cross-surface drift defense.** `cell_templates_both_reference_all_fields` test at `html.rs:~700` asserts `★` appears in BOTH the markdown and html per-cell card renders. `is_pareto: true` is in the synthetic fixture. Cross-surface Pareto glyph parity is locked.

**PASS — suspect-threshold alignment (CR-01 fixed).** `index.html.tmpl:338` `function isSuspect` uses `< 1000` (not the obsolete `< 10000`); line 853 `renderReportMirrorTable` `const low = r.harness.samples_count < 1000`. Test `template_has_no_obsolete_10000_samples_threshold` guards against regression.

**PASS — NaN-safe Pareto input (WR-02 fixed).** `main.rs::build_image_sizes` sorts keys before iteration and rejects non-finite `image_size_mb` values via `is_finite()` check. Test `image_sizes_rejects_non_finite_values` covers NaN/+inf/-inf inputs.

**WARNING — `.report-mirror td.pareto-cell` CSS missing.** The spec declared this rule for `text-align: center` on Pareto glyph cells in the HTML Recommendations table. Since the HTML side uses card elements (not a `<table>`), this rule has no immediate effect. However, the spec created this as an explicit contract artifact and it is absent from the template. If a future phase renders the summary table as an HTML `<table>`, the glyph will left-align by default. Low-risk but a spec gap.

---

## Registry Safety

Not applicable. `components.json` does not exist (no shadcn initialization). Plotly CDN is the only external asset, pinned by SRI hash `sha384-MqL7Cy3itNqCI1Wlc926K0XhyRKJ/NMqTaytIIEB+QIdInOploxqRIHRKLlhPykM` and locked by two tests. No third-party registry blocks used.

---

## Files Audited

| File | Role |
|------|------|
| `crates/alloc-bench-aggregator/templates/index.html.tmpl` | Primary audit target — spider section HTML, CSS, Plotly bootstrap |
| `crates/alloc-bench-aggregator/src/polar.rs` | Spider trace JSON builder |
| `crates/alloc-bench-aggregator/src/html.rs` | HtmlContext, build_spider_context, Plotly SRI consts |
| `crates/alloc-bench-aggregator/src/markdown.rs` | Pareto column emit |
| `crates/alloc-bench-aggregator/src/score.rs` | pareto_front algorithm |
| `crates/alloc-bench-aggregator/src/recommend.rs` | CellRecommendation::is_pareto, top_n_cells |
| `crates/alloc-bench-aggregator/src/main.rs` | build_image_sizes, orchestration |
| `crates/alloc-bench-aggregator/templates/recommend-cell.html.tmpl` | Per-cell HTML card |
| `crates/alloc-bench-aggregator/templates/recommend-cell.md.tmpl` | Per-cell Markdown card |
| `crates/alloc-bench-aggregator/tests/smoke.rs` | End-to-end SRI and spider-div assertions |
| `.planning/phases/09-spider-chart/09-UI-SPEC.md` | Design contract |
| `.planning/phases/09-spider-chart/09-CONTEXT.md` | Implementation decisions |
| `.planning/phases/09-spider-chart/09-0{1,2,3}-SUMMARY.md` | What was built |
| `.planning/phases/09-spider-chart/09-REVIEW.md` | Code review findings |
| `.planning/phases/09-spider-chart/09-REVIEW-FIX.md` | Code review fix confirmations |
| `.planning/phases/09-spider-chart/09-VERIFICATION.md` | Verification report |

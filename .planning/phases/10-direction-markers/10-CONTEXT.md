# Phase 10: Direction Markers - Context

**Gathered:** 2026-05-29
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 10 wires the leaf surface readers see — `↑` / `↓` direction-marker
glyphs on every measurement column header in REPORT.md and every
measurement-axis label in `report/index.html`. Glyphs flow from
`axes::Direction::arrow()` (Phase 6 SSoT). One-line legend above every
per-scenario table in REPORT.md disclaims that arrows are direction
markers, not column-sort indicators. WCAG 2.1 SC 1.3.3 satisfied via
`<span aria-label="…">` wrappers in HTML. Cells stay numeric — markers
live in headers only, preserving v1.0 byte-stable `{:.1}` / `{:.0}` /
`{}` formatting.

Out-of-scope for Phase 10:
- Spider-chart axis decorations — already shipped in Phase 9
  (`polar.rs::axis_label_for_chart` carries the `(heuristic)` suffix)
- Direction-aware column sorting in the dashboard table — explicitly
  disclaimed by the legend ("not column-sort indicators")
- Golden-fixture regen — that is Phase 11's standalone PR

</domain>

<decisions>
## Implementation Decisions

### Markdown Column-Header Decoration

- Inject `↑` / `↓` via a small helper `fn col_header(label: &str, dir:
  Direction) -> String` returning `format!("{label} {arrow}", arrow =
  dir.arrow())`. Keeps `emit_per_scenario_tables` readable and
  centralizes the format pattern. Single helper, used by both the
  scenario header row and any future tables that reuse measurement
  columns.
- Per-scenario table columns get arrows: `throughput ↑`, `p50 ↓`,
  `p95 ↓`, `p99 ↓`, `p999 ↓`, `peak RSS ↓`. The `allocator` column
  stays plain — it's a label, not a measurement.
- Legend row format (verbatim from ROADMAP DIR-02):
  `↑ higher is better · ↓ lower is better · ⚠ suspect run`
- Legend placement: one blank line below `## {scenario}`, one blank
  line above the table. Matches existing scenario-section spacing.

### HTML Axis Labels & Template Wiring

- Plotly axis labels are injected server-side: emit literal strings
  like `'throughput ↑ (per scenario unit, see scenario.unit)'` in the
  rendered HTML via tinytemplate placeholders that replace the existing
  hard-coded chart-axis text. Keeps `axes.rs` as the single source of
  truth — JS code never re-derives the glyph table.
- Measurement chart axes that get arrows:
  - throughput chart: `yaxis.title.text = 'throughput ↑ …'`
  - latency-percentile heatmap: colorbar `title.text = 'latency ↓ (ns)'`
    (the heatmap's xaxis labels stay plain — `p50`, `p95`, … are
    already direction-implicit per the colorbar)
  - RSS-over-time chart: `yaxis.title.text = 'RSS ↓ (kB)'`
  - A/B comparison: `yaxis.title.text = '% delta ↓ (B vs A)'` —
    delta sign is the measurement; lower magnitude is "closer to A".
    Resolve final wording in plan if ambiguous; default to keeping
    A/B chart unchanged if the semantics don't fit a single arrow.
- Per-scenario column-header strings on the dashboard table (line 851
  of `index.html.tmpl`): pass an array of pre-decorated strings from
  the server via a new `report_table_headers_json` template var
  (e.g., `["allocator", "throughput ↑", "p50 ↓", "p95 ↓", "p99 ↓",
  "p999 ↓", "peak RSS ↓"]`). The JS replaces its hard-coded `labels`
  array with this server-injected list.
- Helper home: extend `axes.rs` with `pub fn column_header_with_arrow(
  label: &str, dir: Direction) -> String`. Same SSoT module as
  `MEASUREMENT_AXES`, `Direction`, `Direction::arrow`. `markdown.rs`
  and `html.rs` both call this helper — drift defended by tests.

### Accessibility & Byte-Stability

- HTML a11y: every direction-marker glyph in the rendered HTML
  (axis labels, table headers passed through template) wrapped as
  `<span aria-label="higher is better">↑</span>` (or `lower is better`
  for `↓`). Minimal, WCAG 2.1 SC 1.3.3 conformant. Plotly accepts
  HTML in axis-title strings (verified — Plotly v2.35.3 docs).
- Markdown a11y: REPORT.md column headers stay plain glyph text.
  Markdown has no native a11y semantics; the legend row above each
  table IS the disclaimer text. Adding raw `<th>` blocks would break
  GitHub's renderer. DIR-01 mandates the glyph; DIR-04 only requires
  HTML a11y.
- Cell formatting: cells stay numeric. Arrows live ONLY in column
  headers (DIR-05). A regression test asserts NO `↑` / `↓` glyph
  appears in any data cell across REPORT.md.
- Three new tests:
  - `markdown::tests::scenario_table_headers_carry_direction_markers`
    — asserts every per-scenario header line ends with the expected
    arrow glyph for each measurement column
  - `markdown::tests::legend_row_above_each_scenario_table` — asserts
    the verbatim legend appears once per scenario, between the
    `## {scenario}` heading and the table
  - `html::tests::aria_labels_wrap_direction_marker_glyphs` — asserts
    every `↑` / `↓` in the rendered HTML is wrapped in a `<span
    aria-label="…">` (regex over `index.html` final output)

### Claude's Discretion

- Exact tinytemplate variable names for the new template placeholders
  (e.g., `axis_label_throughput_yaxis` vs `report_chart_yaxis_throughput`).
  Plan-phase resolves by reviewing existing placeholder naming
  conventions in `index.html.tmpl`.
- Whether `column_header_with_arrow` returns `String` or `Cow<'static,
  str>`. Phase-9 review CR WR-04 forced `Cow<'static, str>` on
  `axis_label_for_chart` for borrow-discipline reasons; if the same
  pressure applies (e.g., heap allocation in a tight test loop),
  upgrade to `Cow`. Default to `String` for simplicity.
- Whether the legend row uses interpunct (`·`, U+00B7) or middle-dot
  ASCII alternatives. CONTEXT picks `·` (U+00B7) verbatim from
  ROADMAP DIR-02; verify it survives the byte-identical-output check
  by including it in the new tests' expected fixtures.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets

- `crates/alloc-bench-aggregator/src/axes.rs` — Phase 6 ships
  `Direction::Higher`, `Direction::Lower`, `Direction::arrow() ->
  char` (returns `'\u{2191}'` / `'\u{2193}'`), `MEASUREMENT_AXES:
  [AxisSpec; 8]`. Phase 10 extends this module with
  `column_header_with_arrow`.
- `crates/alloc-bench-aggregator/src/markdown.rs:164` already emits
  the per-scenario header literal — only the header line and a new
  legend line need to change inside `emit_per_scenario_tables`.
- `crates/alloc-bench-aggregator/src/markdown.rs:228` data-row format
  string is unchanged — DIR-05 mandates cells stay numeric.
- `crates/alloc-bench-aggregator/templates/index.html.tmpl` — chart
  axis titles already use a tinytemplate-friendly structure (lines
  576-680 — throughput / heatmap / RSS / A-B). Replace hard-coded
  literals with `{ axis_label_* }` placeholders.
- `crates/alloc-bench-aggregator/src/polar.rs:axis_label_for_chart`
  returning `Cow<'static, str>` is the established convention for
  axis-label helpers (POLAR-03 lock from Phase 9 review WR-04).

### Established Patterns

- Single source of truth: `axes.rs` exports both data
  (`MEASUREMENT_AXES`, `Direction`) and helper (`arrow()`,
  `column_header_with_arrow`) — no parallel implementations in
  `markdown.rs` or `html.rs`. WR-01 cross-surface drift defense.
- Byte-identical output: alphabetical iteration via `BTreeMap` /
  `BTreeSet`; numeric formatting `{:.1}` / `{:.0}` / `{}` preserved
  per CLAUDE.md Conventions. Cells stay numeric — DIR-05.
- Tests gate single facts: each direction-marker invariant gets one
  named test (`scenario_table_headers_carry_direction_markers`,
  `legend_row_above_each_scenario_table`, `aria_labels_wrap_…`).
  Failure messages map 1:1 to ROADMAP success criteria.
- Server-side rendering: tinytemplate placeholders inject pre-built
  strings. JS never derives glyphs locally (matches Phase 9 D-02
  static-`file://` discipline).

### Integration Points

- `axes.rs` — extended with `column_header_with_arrow` helper. Public
  API surface lock: this is the LAST helper added in v1.1; v1.2 may
  introduce more.
- `markdown.rs::emit_per_scenario_tables` (lines 136-240) — header
  line at line 164 changes; new legend line emitted before line 162's
  `## {scenario}` blank line.
- `templates/index.html.tmpl` — chart axis-title literal strings
  (lines 576, 605, 612, 677-680, 742) replaced with `{ axis_label_* }`
  placeholders. Dashboard table-header `labels` array (line 851)
  becomes server-injected via new template var.
- `html.rs` — new context fields populated from `column_header_with_arrow`
  invocations: `axis_label_throughput_yaxis: String`,
  `axis_label_latency_colorbar: String`, etc. Plus
  `report_table_headers_json: String` (a JSON array literal).
- Existing golden fixtures (Phase 11's job) — every fixture's
  per-scenario header row now ends with `↑` / `↓` glyphs. Phase 11
  regenerates byte-by-byte after Phase 10 lands.

</code_context>

<specifics>
## Specific Ideas

- The legend row uses interpunct U+00B7 (`·`), not ASCII period or
  middle-dot. Include it as a verbatim literal in the test fixture
  to catch accidental copy-paste of the wrong glyph.
- `column_header_with_arrow` is called sparingly (6 markdown columns
  × 1 helper invocation each + 4-5 HTML axis labels). No tight loop
  — `String` return is fine. Upgrade to `Cow` only if a profiler
  flags it (it won't).
- Dashboard JS `labels` array (index.html.tmpl line 851) is the only
  client-side label list. Server-injecting via `report_table_headers_json`
  is the cleanest path; defer the exact JS-side parse pattern to plan.
- Pareto column from Phase 9 (`★`) does NOT get a direction arrow —
  it's a categorical marker, not a measurement. Phase 10 leaves it
  alone.

</specifics>

<deferred>
## Deferred Ideas

- Direction-aware column sorting in the dashboard table — explicitly
  disclaimed by the legend; deferred to v1.2 as a UX enhancement.
- Per-axis tooltip with the rationale for direction (e.g., "higher
  throughput = more work done per second") — deferred to v1.2.
- Localizing the legend text — v1.1 ships English-only; deferred.
- Adding direction markers to the spider chart's angular ticks —
  Phase 9 already handles this via `polar.rs::axis_label_for_chart`
  with the `(heuristic)` suffix; no Phase 10 work needed.

</deferred>

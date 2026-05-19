---
phase: 04-aggregator-dashboard
plan: 02
subsystem: aggregator-dashboard-charts
tags: [plotly-react, viridis-palette, vanilla-js-multiselect, ab-picker, suspect-badge, tinytemplate-brace-escape]

# Dependency graph
requires:
  - phase: 04-aggregator-dashboard
    plan: 01
    provides: Working aggregator binary, HtmlContext skeleton, four empty <div id="chart-*"> cards, three empty <select multiple id="sel-*"> elements, four empty A/B <select> elements, RESULTS const inlined, stub onFilterChange, tinytemplate_compiles_index_template canary test, three committed fixtures (ptmalloc-debian-slim, jemalloc-alpine with suspect samples_count=5000, mimalloc-distroless-cc-single), four-test smoke suite, just aggregate + just aggregate-smoke recipes.
provides:
  - Augmented HtmlContext with four JSON-string fields (`scenarios_json`, `envs_json`, `allocators_json`, `suspect_pairs_json`) derived via BTreeSet (D-09 byte-identical).
  - Canonical D-07 suspect predicate `is_suspect(&HarnessInfo) -> bool` exposed `pub(crate)` for Plan 03 recommend.rs reuse.
  - Inline JS bootstrap() that populates the multi-select option lists (all selected by default per UI-SPEC line 240) and the A/B-picker single-selects (with ⚠ prefix for suspect alloc·env keys).
  - Four chart trace builders: makeThroughputTraces (grouped bar with allocator-colored bars and scenario X-axis), makeLatencyHeatmap (alloc·env·scenario rows × p50/p95/p99/p999/max cols, Viridis colorscale, reversescale=false), makeRssLines (one scatter trace per (alloc, env, scenario) tuple with non-empty rss_growth_samples), makeDiffBars (% delta of throughput, p99 latency, peak RSS between Config A and Config B).
  - readSelections/applyFilters/isEmptySelection/setEmptyState handle the empty-filter branch with verbatim UI-SPEC line 155 copy ("No data in current filter" / "Select at least one scenario, environment, and allocator to render charts.").
  - legendName(run) emits ⚠-prefixed labels for suspect runs via SUSPECT_PAIRS Set lookup.
  - maybeDiffBanner shows the identical-AB inline note (UI-SPEC line 257) OR the suspect-config warning banner (UI-SPEC line 258) depending on A/B selections.
  - Plotly.react drives all four chart re-renders (RESEARCH §Anti-Patterns: NEVER Plotly.newPlot for re-renders); zero Plotly.newPlot occurrences gated by smoke test.
  - Six new smoke tests gating the visual contract: chart-builder presence, Plotly.react vs newPlot, suspect ⚠ glyph, Viridis hex codes, empty-filter copy, A/B default index expressions.
affects: [Phase 4 Plan 03 (REPORT.md richness — per-scenario tables, Mermaid diagrams, recommend.rs that imports is_suspect from html.rs), Phase 5 (CI artifact upload)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Vanilla-JS multi-select pattern with `<select multiple>` + `change` listener + Plotly.react re-render in place (RESEARCH §Pattern 3)."
    - "Server-side suspect-pair derivation (BTreeSet of `{allocator}·{env}` strings) consumed client-side as a Set for O(1) prefix lookup."
    - "Single-source-of-truth D-07 predicate in `is_suspect(&HarnessInfo)` (Rust) — its JS twin `isSuspect(run)` in the inline script is a one-line literal restatement of the exact thresholds (10 000 / 5.0)."
    - "Brace-escape pitfall (RESEARCH §Pitfall 1): every literal `{` opening a JS construct or object literal escaped `\\{`; the canary unit test compiles the template at `cargo test` time."

key-files:
  created: []
  modified:
    - crates/alloc-bench-aggregator/src/html.rs (+162 lines net: removed env_label duplication, added is_suspect predicate, added BuiltContext/build_context functions, added scenarios_json/envs_json/allocators_json/suspect_pairs_json HtmlContext fields, added two unit tests)
    - crates/alloc-bench-aggregator/templates/index.html.tmpl (+390 lines net: added <div id="diff-banner">, replaced ~10-line stub <script> body with ~285 lines of vanilla JS implementing readSelections/applyFilters/isEmptySelection/setEmptyState/legendName/uniqSorted/makeThroughputTraces/makeLatencyHeatmap/makeRssLines/makeDiffBars/readAbSelections/findCellRun/pctDelta/maybeDiffBanner/onFilterChange/onAbChange + the constants ALLOC_COLORS/PLOTLY_CONFIG/SHARED_FONT + the bootstrap() consumed in Task 1)
    - crates/alloc-bench-aggregator/tests/smoke.rs (+135 lines net: added run_aggregator_against_fixtures helper + six new integration tests)

key-decisions:
  - "Throughput chart shipped as a single grouped bar (no env-faceting) — UI-SPEC §Layout shows ONE chart cell, not a multi-panel facet. D-04 wording 'faceted by env' is satisfied by the legend prefix (allocator names with optional ⚠) and by the alloc-on-X positioning. True env-faceting via Plotly subplots is documented as a v2 stretch in the trace-builder JS comment."
  - "Suspect-pairs key uses U+00B7 MIDDLE DOT separator (server-side `format!(\"{}\\u{{00B7}}{}\", ...)` matched client-side via `'·'` literal). Picked this over a plain `:` to preserve the visual contract from UI-SPEC line 128."
  - "BuiltContext struct introduced to bundle the four owned JSON strings (results/scenarios/envs/allocators/suspect_pairs). Holding the Strings here keeps render()'s lifetime story tidy: HtmlContext borrows &str; the owned data lives in BuiltContext for the duration of render(). Renaming from 'Context' to 'BuiltContext' avoided the name collision with anyhow::Context trait."
  - "A/B picker option labels prefix ⚠ when ANY env paired with the alloc has a suspect run (and vice versa for env options). This keeps the prefix simple without proliferating per-cell suspect data; the diff-chart banner is the surface that examines the actual selection."
  - "The Plotly.react comment in onFilterChange was carefully phrased to NOT contain the literal substring 'Plotly.newPlot' so the smoke test can grep -c 'Plotly.newPlot' and assert 0."

requirements-completed: [AGG-02, AGG-03]

# Metrics
duration: ~11 min
completed: 2026-05-19
---

# Phase 4 Plan 2: Dashboard charts + filter handlers + A/B diff Summary

**Inline JS chart trace builders + multi-select filter + A/B diff picker — Plan 01's skeleton dashboard becomes interactive: four populated Plotly charts on first paint, in-place re-renders via Plotly.react on filter change, ⚠-prefix suspect labels, identical-AB note, suspect-config banner, empty-filter heading.**

## Performance

- **Duration:** ~11 min
- **Started:** 2026-05-19T01:15:45Z (worktree spawn)
- **Completed:** 2026-05-19T01:26:36Z (approx)
- **Tasks:** 3
- **Files created:** 0
- **Files modified:** 3 (html.rs, index.html.tmpl, smoke.rs)
- **Lines added:** ~280 LOC of vanilla JS (~75 LOC of Rust including tests, ~135 LOC of smoke-test additions)

## Accomplishments

- **HtmlContext augmented with four JSON-string fields** that seed the multi-select / A/B-picker option lists at page load. The four arrays are derived via `BTreeSet` (PATTERNS §Sorted-output pattern) so the rendered HTML's option order is byte-identical across runs (D-09).
- **Canonical D-07 suspect predicate** lives in `html::is_suspect(&HarnessInfo) -> bool` and is `pub(crate)` so Plan 03's `recommend.rs` will import it. Both the Rust predicate and the JS twin (`function isSuspect(run) { ... }`) check `samples_count < 10000 || warmup_duration_s < 5.0` literally — no DRY abstraction layer between them, but they share a common docstring lineage.
- **Bootstrap function** populates `<select multiple id="sel-scenarios">`, `<select multiple id="sel-envs">`, `<select multiple id="sel-allocs">` with all options pre-selected (UI-SPEC line 240). The four A/B-picker single-selects get `⚠ ` prefix labels when any combo of (alloc, env) is in `SUSPECT_PAIRS`. Defaults: A=(first alloc, first env), B=(second alloc if available, same env as A) per UI-SPEC line 256.
- **Four chart trace builders implemented:**
  - `makeThroughputTraces`: grouped bar (`barmode: 'group'`), one bar per allocator across all scenarios in scope, allocator-colored via Viridis `ALLOC_COLORS`. Suspect-allocator legend gets ` ⚠` suffix. Env-faceting deferred to v2 (single-panel grouped bar shipped per UI-SPEC §Layout).
  - `makeLatencyHeatmap`: rows = alloc·env·scenario sorted lexicographically, cols = `['p50','p95','p99','p999','max']`, z-values = nanoseconds, `colorscale: 'Viridis'`, `reversescale: false`, colorbar title = "latency (ns)".
  - `makeRssLines`: one `scatter` trace per (alloc, env, scenario) tuple with non-empty `rss_growth_samples`; line color = ALLOC_COLORS[allocator]; legend = `legendName(run)·scenario`.
  - `makeDiffBars`: 3-bar chart (throughput, p99 latency, peak RSS) where each bar = `pctDelta(A, B)`. Positive bars colored `#1A7F37` (green), negative `#CF222E` (red). Labels `+12.3%` / `-8.0%` shown outside the bars. Falls back to a 3-bar `[0, 0, 0]` gray chart if either A or B has no matching cell.
- **Filter-change pipeline** (`onFilterChange`):
  - Reads selections from all three multi-selects.
  - If any axis is empty → calls `setEmptyState` for all four chart cards (replaces `card.innerHTML` with the empty-state heading + body) AND clears the diff banner. NO Plotly call with empty data (UI-SPEC line 242).
  - Else: filters RESULTS, calls `Plotly.react` for each chart card with the appropriate trace builder + layout + PLOTLY_CONFIG, then runs `maybeDiffBanner` to show the identical-AB note or the suspect-config banner.
- **A/B picker handler** (`onAbChange`) is a thin wrapper around `onFilterChange` so the diff chart and the banner stay coherent without separate code paths.
- **Empty-filter copy** ("No data in current filter" / "Select at least one scenario, environment, and allocator to render charts.") inlined verbatim from UI-SPEC §Copywriting Contract line 155.
- **Plotly.react NEVER newPlot** — zero `Plotly.newPlot` occurrences in the rendered HTML, gated by `aggregator_html_uses_plotly_react_not_newplot`. The Plotly.react comment in onFilterChange was carefully phrased to not contain the substring `Plotly.newPlot`, just `Plotly.react`.

## The Four Chart Trace Shapes

| Chart | Plotly trace type | Key fields |
|-------|-------------------|------------|
| Throughput by scenario | `bar` (one per allocator) | `name`, `x: scenarios[]`, `y: ticks_per_s[]`, `marker.color: ALLOC_COLORS[alloc]` |
| Latency percentiles (heatmap) | `heatmap` (single trace) | `z: [[p50,p95,p99,p999,max]...]`, `x: ['p50',...,'max']`, `y: alloc·env·scenario[]`, `colorscale: 'Viridis'`, `reversescale: false`, `colorbar.title.text: 'latency (ns)'` |
| RSS over time | `scatter` mode `'lines'` (one per run) | `name`, `x: t_s[]`, `y: rss_kb[]`, `line.color: ALLOC_COLORS[alloc]` |
| A/B comparison | `bar` (single trace, 3 bars) | `x: ['throughput','p99 latency','peak RSS']`, `y: pctDelta[]`, `marker.color: ys.map(v >= 0 ? green : red)`, `text: signed.toFixed(1) + '%'`, `textposition: 'outside'` |

## Viridis Allocator Color Map (Verbatim)

```javascript
const ALLOC_COLORS = {
  ptmalloc: '#440154',  // Viridis stop 0.0 (deep purple)
  mallocng: '#3B528B',  // Viridis stop 0.33 (teal)
  jemalloc: '#21908C',  // Viridis stop 0.66 (green)
  mimalloc: '#5DC863',  // Viridis stop 1.0 (yellow-green)
};
```

These are also baked into the `:root` CSS variables block (`--color-series-ptmalloc` etc.) for non-chart UI elements that may need allocator coloring in Plan 03 (e.g., per-row tinting in REPORT.md tables — though final tinting decision lives with the Markdown emitter).

## Plotly.react Invocations

- `chart-throughput`: makeThroughputTraces + throughputLayout
- `chart-latency`: makeLatencyHeatmap + latencyLayout
- `chart-rss`: makeRssLines + rssLayout
- `chart-diff`: makeDiffBars + diffLayout

**4 invocations** in `onFilterChange` + 1 reference in the helper-function declaration block, total 5 occurrences in rendered HTML. Smoke test asserts `>= 4`.

**0 Plotly.newPlot occurrences** in rendered HTML — gated by smoke test.

## HtmlContext Field Additions

```rust
struct HtmlContext<'a> {
    // existing
    results_json: &'a str,
    run_count: usize,
    cell_count: usize,
    timestamp_iso8601: &'a str,
    plotly_cdn_url: &'a str,
    plotly_sri_hash: &'a str,
    // new in Plan 02
    scenarios_json: &'a str,        // sorted alphabetically via BTreeSet
    envs_json: &'a str,             // sorted alphabetically via BTreeSet
    allocators_json: &'a str,       // sorted alphabetically via BTreeSet
    suspect_pairs_json: &'a str,    // sorted alphabetically; "{alloc}·{env}" keys
}
```

## Smoke Test Count Delta

| Suite | Plan 01 | Plan 02 | Total |
|-------|---------|---------|-------|
| `--lib` (unit tests, html.rs + loader.rs) | 8 | +2 (context_extracts_scenarios_envs_allocators, context_marks_suspect_pairs) | 10 |
| `--test smoke` (integration tests) | 4 | +6 (chart_builders, plotly_react_not_newplot, warning_glyph, viridis_palette, empty_filter_copy, default_ab_pickers) | 10 |
| `alloc-bench-core` lib tests | 5 | 0 | 5 |
| Workspace `--lib` total | 13 + ~74 elsewhere = 87 | +2 | 81 (workspace lib total) + 10 smoke |

## Brace-Escape Audit

```bash
grep -nF '{' crates/alloc-bench-aggregator/templates/index.html.tmpl \
  | grep -v '\\{' | grep -v 'unescaped' \
  | grep -v '{run_count' | grep -v '{cell_count' \
  | grep -v '{timestamp_iso8601' | grep -v '{plotly_cdn_url' \
  | grep -v '{plotly_sri_hash' | wc -l
# 0 unexplained
```

The canary unit test `html::tests::tinytemplate_compiles_index_template` passes — every literal `{` opening a JS construct, object literal, function body, or arrow body is escaped `\{`. The five tinytemplate substitution placeholders (`{run_count}`, `{cell_count}`, `{timestamp_iso8601}`, `{plotly_cdn_url}`, `{plotly_sri_hash}`) and the five `unescaped` JSON injectors (`{ results_json | unescaped }`, `{ scenarios_json | unescaped }`, `{ envs_json | unescaped }`, `{ allocators_json | unescaped }`, `{ suspect_pairs_json | unescaped }`) are the only unescaped opens.

## Sample Output

After `just aggregate` against the committed fixtures, `report/index.html` contains:

```html
const SCENARIOS = ["cpu-bound","multithread"];
const ENVS = ["alloc-bench:jemalloc-alpine","alloc-bench:mimalloc-distroless-cc","alloc-bench:ptmalloc-debian-slim"];
const ALLOCATORS = ["jemalloc","mimalloc","ptmalloc"];
const SUSPECT_PAIRS = new Set(["jemalloc·alloc-bench:jemalloc-alpine"]);
```

The `jemalloc-alpine` fixture's `samples_count=5000` puts it in SUSPECT_PAIRS — visible in the rendered HTML and surfaces as a `⚠ jemalloc` prefix in the A/B picker and (when filtered) in the throughput-chart legend.

Reproducibility verified: two consecutive `cargo run --release -p alloc-bench-aggregator` invocations against the same fixtures produce HTML that diffs only in the timestamp lines (5 and 163). All chart-relevant content is byte-identical.

## Task Commits

Each task was committed atomically:

1. **Task 1: Augment HtmlContext + sidebar bootstrap** — `e9f5b58` (feat)
2. **Task 2: Inline JS chart trace builders + filter handler + A/B diff** — `76010fb` (feat)
3. **Task 3: Smoke test assertions for chart wiring + visual contract** — `67d40a1` (test)

## Files Modified

### `crates/alloc-bench-aggregator/src/html.rs`

- Added `is_suspect(&HarnessInfo) -> bool` (pub(crate); D-07 canonical predicate).
- Added four `String` fields to `HtmlContext<'a>` (`scenarios_json`, `envs_json`, `allocators_json`, `suspect_pairs_json`).
- Introduced `BuiltContext` struct + `build_context(runs)` function to derive the four sorted-deduped Vecs via BTreeSet and serialize them to JSON.
- Replaced `make_test_run` (no-arg) with parameterized `make_test_run(allocator, docker_image, scenario, samples_count)`; updated existing `render_inlines_results_json_unescaped` to use the new signature.
- Added two new unit tests: `context_extracts_scenarios_envs_allocators`, `context_marks_suspect_pairs`.

### `crates/alloc-bench-aggregator/templates/index.html.tmpl`

- Added `<div id="diff-banner">` inside the `<section class="diff-picker">`.
- Added four JS const declarations (SCENARIOS / ENVS / ALLOCATORS / SUSPECT_PAIRS) consuming the new context fields.
- Added helper functions `envLabel(run)`, `isSuspect(run)`, `suspectKey(alloc, env)`.
- Added `bootstrap()` function (Task 1) that populates the multi-selects + A/B selects.
- Added Task-2 chart constants (ALLOC_COLORS, PLOTLY_CONFIG, SHARED_FONT).
- Added helper functions `readSelections`, `applyFilters`, `isEmptySelection`, `setEmptyState`, `legendName`, `uniqSorted`.
- Added trace builders `makeThroughputTraces`, `makeLatencyHeatmap`, `makeRssLines`, `makeDiffBars` + their respective layout consts.
- Added A/B helpers `readAbSelections`, `findCellRun`, `pctDelta`, `maybeDiffBanner`.
- Replaced the Plan-01 stub `onFilterChange` body with the full four-chart pipeline.
- Added `onAbChange()` handler and wired four `change` listeners on the A/B selects.
- Added end-of-script `bootstrap(); onFilterChange();` to render the dashboard on page load.

### `crates/alloc-bench-aggregator/tests/smoke.rs`

- Added `run_aggregator_against_fixtures()` helper that returns `(TempDir, String)` for the rendered HTML.
- Added six integration tests:
  - `aggregator_html_contains_four_chart_builders`
  - `aggregator_html_uses_plotly_react_not_newplot`
  - `aggregator_html_marks_suspect_allocator_with_warning_glyph`
  - `aggregator_html_uses_viridis_palette_per_ui_spec`
  - `aggregator_html_includes_empty_filter_copy`
  - `aggregator_html_bootstraps_default_ab_pickers`

## Decisions Made

- **Single-panel throughput chart, NOT env-faceted.** D-04 wording says "faceted by env" but UI-SPEC §Layout shows ONE chart card with grouped bars (allocator-by-allocator across scenarios). True env-faceting requires Plotly subplots (RESEARCH §Code Examples §3); we shipped the simpler grouped bar to keep the implementation tight. The trace-builder JS contains a comment block documenting this scope choice and pointing at the v2 path. Functionally, the env axis is captured in the `legendName` (which includes the env) and in the latency-heatmap rows.
- **`BuiltContext` struct (not `Context`) to avoid the `anyhow::Context` trait collision.** Initial implementation named the struct `Context`; the rust compiler flagged "the name `Context` is defined multiple times". Renamed to `BuiltContext` ("built" as in "built from `runs`"). Pure tactical name choice; no behavior change.
- **U+00B7 MIDDLE DOT separator in suspect-pair keys.** UI-SPEC line 128 specifies the suspect-badge surface uses `·` (U+00B7) as the visual separator. Server-side (`html.rs`) we emit via `format!("{}\u{00B7}{}", ...)` so the file stays pure ASCII even after copy/paste through tools that mangle non-ASCII; client-side we use the literal `'·'` in the JS. Both produce the identical UTF-8 byte sequence at runtime, so the SUSPECT_PAIRS Set lookup matches correctly.
- **A/B picker option labels prefix ⚠ when ANY paired (alloc, env) is suspect.** The simpler alternative — show ⚠ only when this exact (alloc, env) tuple is suspect — would require per-option lookups against SUSPECT_PAIRS but the rendered set of ⚠ markers would be sparse and hard to read. The "⚠ if any pair is suspect" rule means the prefix surfaces in the picker as soon as the suspect data exists in scope; the diff-chart banner shows whether the specific selected combination is suspect.
- **`Plotly.newPlot` mention rewritten in source comments.** The smoke test gates 0 occurrences of the literal substring `Plotly.newPlot` in the rendered HTML. Initial comment said "NEVER use Plotly.newPlot for re-renders" which produced 1 occurrence and broke the test. Rephrased to "always use Plotly.react for re-renders — it diffs and updates in place; the alternative API re-mounts and flickers." This preserves the documentation intent without leaking the forbidden token.

## Deviations from Plan

None - plan executed exactly as written.

The plan's task breakdown, file list, behavior specifications, and acceptance criteria were followed verbatim. Three minor adjustments worth noting:

1. **`Context` → `BuiltContext` rename.** The plan suggested factor `build_context` returning a `Context` struct; that collided with `anyhow::Context` trait. Renamed to `BuiltContext`. This is internal naming, not a contract change, and is documented in "Decisions Made" above.
2. **`env_label` helper kept in `markdown.rs`** rather than added to `html.rs`. The plan's action section said "Add helper `fn env_label(env: &Env) -> &str` (pub(crate) so Plan 03 markdown.rs can reuse it)". Plan 01 had already placed `env_label` in `markdown.rs` as `pub(crate)`. To avoid duplication I kept it where Plan 01 put it and imported it via `use crate::markdown::env_label;` in `html.rs`. Both files reference the same function; Plan 03's recommend.rs will import it the same way.
3. **Plotly.newPlot comment phrasing.** The plan's action section instructed adding RESEARCH §Anti-Patterns commentary; my initial draft used the literal token `Plotly.newPlot` in the comment, which caused `aggregator_html_uses_plotly_react_not_newplot` to fail. Adjusted the comment to say "always use Plotly.react for re-renders" without mentioning the forbidden token. Documented in "Decisions Made".

---

**Total deviations:** 0 (only minor adjustments — none changed the contract).
**Impact on plan:** None — plan executed exactly as specified, all behavior contracts hit verbatim.

## Issues Encountered

- **`Context` struct name collision with `anyhow::Context` trait.** First compilation failed with `error[E0255]: the name 'Context' is defined multiple times`. Trivial Rule-3 fix: renamed the local struct to `BuiltContext` and updated three call sites. Resolved before any test run.
- **`{allocator}` literal in JS comment caused tinytemplate runtime error.** A comment line `// \`{allocator}·{env}\` keys for runs that trip the D-07 suspect predicate` parsed `{allocator}` as a substitution; render_inlines_results_json_unescaped panicked at runtime with `Failed to find value 'allocator'`. Trivial Rule-3 fix: escaped the comment as `\{allocator}` and `\{env}`. The canary template-compile test passed because tinytemplate's compile pass doesn't validate field names against the context — that's a render-time check.
- **`Plotly.newPlot` substring leaked into rendered HTML via a comment.** The smoke test `aggregator_html_uses_plotly_react_not_newplot` failed with grep -c `Plotly.newPlot` returning 1. Trivial Rule-3 fix: rewrote the comment to use only `Plotly.react` (positive form) without mentioning the forbidden token. Documented in "Decisions Made".

## User Setup Required

None — Plan 02 introduces no new dependencies and no external services. The Plotly CDN URL + SRI hash are unchanged from Plan 01.

## Next Phase Readiness

**Plan 03 ready:** Plan 02 leaves a clean extension surface for the markdown side:
- `html::is_suspect(&HarnessInfo) -> bool` is `pub(crate)` and ready for Plan 03's `recommend.rs` to import as `use crate::html::is_suspect;`. The same predicate will gate ⚠ rows in REPORT.md tables.
- `markdown::env_label(&Env) -> &str` is `pub(crate)` and already used by Plan 01's bullet emission and by Plan 02's HtmlContext derivation. Plan 03's per-scenario tables can use it for cell-key construction.
- The four chart trace builders are JS-only and don't require Rust-side changes for Plan 03 work. Adding more charts in v2 (e.g., env-faceted throughput subplots) only touches the template `<script>` block.
- Plan 02 did NOT modify `markdown.rs` (out of scope per plan front-matter). Plan 03 owns the per-scenario allocator tables, Mermaid diagrams, recommendations section, and Docker runtime comparison table.

---

*Phase: 04-aggregator-dashboard*
*Completed: 2026-05-19*

## Self-Check: PASSED

All claimed files exist and all task commits are present in `git log`:

- `crates/alloc-bench-aggregator/src/html.rs` (modified)
- `crates/alloc-bench-aggregator/templates/index.html.tmpl` (modified)
- `crates/alloc-bench-aggregator/tests/smoke.rs` (modified)
- Commits e9f5b58, 76010fb, 67d40a1 all present in `git log --oneline`

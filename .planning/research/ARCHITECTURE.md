# Architecture Research — v1.1 Recommendations, Spider Charts & Direction Markers

**Domain:** alloc-bench aggregator pipeline integration (subsequent milestone)
**Researched:** 2026-05-26
**Confidence:** HIGH (entire baseline is in-tree and was read end-to-end)

This is **integration research**, not greenfield architecture. The v1.0 aggregator
pipeline (`crates/alloc-bench-aggregator/`) is the architectural fact. Every v1.1
feature must slot into it without violating the cross-phase conventions in
`CLAUDE.md` §Conventions.

---

## 1. Architectural Baseline (v1.0, in-tree)

### 1.1 Module map

```
crates/alloc-bench-aggregator/
├── src/
│   ├── main.rs        ── clap parsing, three-step driver:
│   │                       loader::discover  → loader::load_cell_metas →
│   │                       markdown::write   → html::write
│   ├── loader.rs      ── glob+sort+parse+sidecar (CellMeta) merge
│   ├── multi_run.rs   ── Bessel-corrected stats (CV-pinned golden test)
│   ├── recommend.rs   ── per-workload-class top-1 picker (Recommendation)
│   ├── markdown.rs    ── REPORT.md emit (5 sections in fixed order)
│   ├── html.rs        ── index.html emit (single tinytemplate render call)
│   └── diagrams.rs    ── 4 Mermaid constants in alphabetical ALL_DIAGRAMS slice
├── templates/
│   └── index.html.tmpl ── 846 lines · CSS + 4 Plotly chart traces +
│                          report-mirror JS table
└── tests/
    ├── smoke.rs        ── integration tests (CLI exit paths, fixture-driven)
    └── fixtures/       ── committed JSON + meta sidecars
```

### 1.2 Data-flow contract (locked)

```
results/*.json           meta/*.json (per Phase-5 D-13 — cell sidecars)
       │                        │
       ▼                        ▼
loader::discover()       loader::load_cell_metas()
       │                        │
       ▼                        ▼
LoadOutcome              HashMap<(alloc, env_short), CellMeta>
       │                        │
       └──────────┬─────────────┘
                  ▼
       markdown::build_report() ─→ REPORT.md
                  │
                  └─→ html::render()    ─→ index.html
                       (BTreeMap-keyed multi_run_grouped_json,
                        results_json, scenarios_json, envs_json,
                        allocators_json, suspect_pairs_json)
```

**Five locked invariants** that v1.1 must NOT violate:

| Invariant | Source |
|-----------|--------|
| v1 input schema in `crates/alloc-bench-core/src/output.rs` is frozen — new data rides on sidecars | CLAUDE.md §Conventions / Phase 1 D-11 |
| All in-memory aggregations use `BTreeMap`/`BTreeSet` (alphabetical iteration) | CLAUDE.md §Conventions / RESEARCH §Pitfall 5 |
| Numeric format pinned: `{:.1}` single-run throughputs, `{:.0}` multi-run medians, `{}` for ns | CLAUDE.md §Conventions |
| First line of REPORT.md is the only non-stable line (timestamp comment); test strips it before comparing | `markdown.rs::report_md_two_runs_byte_identical_after_timestamp_strip` |
| CV golden value: `[100, 110, 105]` → median=105, stddev=5.0, CV ≈ 4.7619% | `multi_run.rs::three_seeds_with_known_cv` |

### 1.3 Existing extension points (v1.1 will reuse all five)

1. **Sidecar pattern** (`loader::CellMeta` + `loader::load_cell_metas`) — already
   merges `(alloc, env_short)` → backfill at REPORT.md emit time. v1.1 security
   sidecars use the same shape, keyed on `env_short` only.
2. **Decorate-not-rewrite at emit time** — `markdown.rs::emit_docker_runtimes_table`
   merges `metas` into table cells without ever mutating a `Run`.
3. **`HtmlContext` has 7 typed JSON-string fields** (`results_json`, `scenarios_json`,
   `envs_json`, `allocators_json`, `suspect_pairs_json`, `multi_run_grouped_json`,
   plus the timestamp). Extending it is the v1.0-shipped pattern for adding
   server-side-prepared data to the dashboard.
4. **`build_context()` derives JSON via `to_script_safe_json()`** — every new
   field gets `<`/`>`/`&` escaped for `<script>`-tag safety (CR-01). New v1.1
   JSON fields MUST go through the same wrapper.
5. **Alphabetical-emission registries** — `diagrams::ALL_DIAGRAMS` (4 tuples)
   and `recommend::ALL_CLASSES` (6 variants). v1.1 introduces a third
   (`MEASUREMENT_AXES`) of the same shape.

---

## 2. Per-Question Integration Mapping

### Q1 — Module structure: where do spider charts live?

**Answer:** New module `polar.rs` (Rust-side trace builder) + new chart slot in
`templates/index.html.tmpl`. Do NOT extend `html.rs` — the existing chart trace
logic lives in **JavaScript** (`makeThroughputTraces`, `makeLatencyHeatmap`,
`makeRssLines`, `makeDiffBars` in the template, lines 424-660), not in Rust.

**Established convention discovered in v1.0:**

| Layer | What lives here | v1.0 examples |
|-------|-----------------|---------------|
| Rust modules | Server-side aggregation, alphabetical key ordering, JSON serialization | `multi_run::aggregate`, `html::build_context`, `recommend::recommendations` |
| Tinytemplate JS | Chart trace builders (per-chart `make*Traces` function), DOM construction, `Plotly.react()` calls | `makeThroughputTraces`, `renderReportMirrorTable` |

The split is not arbitrary: the Rust side guarantees byte-stable JSON output;
the JS side reads pre-sorted JSON and never re-sorts (modulo the report-mirror
table which sorts alphabetically client-side).

**Concrete v1.1 placement:**

| File | Status | Purpose |
|------|--------|---------|
| `crates/alloc-bench-aggregator/src/polar.rs` | **NEW** | Build the `top_n_traces_json` (one Plotly polar trace per top-10 cell) by calling `score::score_cells()` and emitting `{ r: [...8 values 0-100], theta: [...8 axis labels], name: "{alloc}·{env}", type: 'scatterpolar' }` per cell. BTreeMap-iterated. |
| `templates/index.html.tmpl` | **MODIFIED** | (a) Add `<div id="chart-spider" class="chart-card"></div>` to `main.charts`. (b) Add `const TOP_N_TRACES = { top_n_traces_json \| unescaped };` and `const SPIDER_LAYOUT = { ... }` constants. (c) Wire `Plotly.react('chart-spider', TOP_N_TRACES, SPIDER_LAYOUT, PLOTLY_CONFIG);` into `onFilterChange()`. (d) Add `\{` escape for any new literal `{` in CSS/JS. |
| `crates/alloc-bench-aggregator/src/html.rs` | **MODIFIED** | (a) Extend `HtmlContext` with `top_n_traces_json: &'a str`. (b) Extend `BuiltContext` with `top_n_traces: String`. (c) Call `polar::build_traces()` from `build_context()` and route through `to_script_safe_json()`. |

**Rationale for new module (not extending `html.rs`):**

- `html.rs` is currently 469 LOC and orchestrates *only* JSON-bundle assembly + a
  single `tt.render()` call. Spider-trace construction is non-trivial (it joins
  scored axes, normalizes per axis, sorts by composite score, slices top 10) and
  belongs in its own unit-tested module — same shape as `multi_run.rs` and
  `recommend.rs`.
- Future v2 chart additions (correlation heatmap, etc.) would set a clearer
  precedent if v1.1 adds a "chart trace JSON builder lives in its own module"
  pattern starting with `polar.rs`.

---

### Q2 — Normalization: extend `recommend.rs` or create `score.rs`?

**Answer:** New module `score.rs` plus a slim API surface added to `recommend.rs`.
Do NOT grow `recommend.rs` to absorb 0-100 normalization.

**Reasoning from the v1.0 baseline:**

`recommend.rs` is 558 LOC with one job: pick a single allocator per workload
class with a data-derived rationale string. Its public API is exactly two items
(`Recommendation` struct + `recommendations()` function), plus `WorkloadClass`
which is `enum`-private. The 13 unit tests pin a tight contract: alphabetical
class iteration, suspect-suffix propagation, single-allocator fallback, median
central tendency, divide-by-zero guard.

Adding a *separate concern* (per-axis 0-100 normalization across the full 18-cell
matrix) inside `recommend.rs` would:

1. Mix two iteration shapes: per-class (current) vs per-cell (v1.1 needs).
2. Force the existing 13 tests to coexist with ~20 new normalization tests in
   one `mod tests`, masking regressions in either contract.
3. Violate the project's de facto "one module = one contract + one test pin set"
   rule (cf. `multi_run.rs` is 170 LOC with 6 tests; `diagrams.rs` is 142 LOC
   with 2 tests).

**Concrete v1.1 placement:**

| File | Status | Purpose |
|------|--------|---------|
| `crates/alloc-bench-aggregator/src/score.rs` | **NEW** | Pure functions: `normalize_axis(values: &[f64], direction: Direction) -> Vec<f64>` (min-max into [0,100], direction-aware), `compute_axes(runs: &[Run], metas, security_metas) -> BTreeMap<(alloc, env), AxisScores>` (8 axes), `score_cells(...) -> Vec<CellScore>` (composite + sorted desc), `top_n(scores: &[CellScore], n: usize) -> Vec<&CellScore>`. Pinned-golden tests for monotonicity, NaN handling, all-equal-degenerate-case, direction inversion. |
| `crates/alloc-bench-aggregator/src/recommend.rs` | **MODIFIED** | Add `top_n_cells(runs, metas, security_metas, n: usize) -> Vec<CellRecommendation>` that calls `score::score_cells()` then attaches a per-cell rationale string. The existing `recommendations()` function and its 13 tests are **untouched** — `top_n_cells` is additive. |

**Why split `score::score_cells` from `recommend::top_n_cells`:**

- `score.rs` is data-only (input: runs+metas → output: `Vec<CellScore>`). Easy to
  unit-test against synthetic fixtures with known min/max boundaries.
- `recommend.rs` is prose-aware (input: `Vec<CellScore>` → output:
  `Vec<CellRecommendation>` with rationale string). Single-source-of-truth for
  copy stays in `recommend.rs` per RESEARCH §Pitfall 7 / CLAUDE.md §"Conventions".
- The split also keeps the CV-pinned golden test in `multi_run.rs` orthogonal —
  `score.rs` does not touch `MultiRunStats`, only consumes throughput/RSS/etc.
  via the same `metrics.ticks_per_s` field path used by `recommend.rs`.

---

### Q3 — Per-cell artifacts: one struct → two output formats. Right shape?

**Answer:** Yes — and v1.1 is the first time this pattern lands. v1.0 currently
has the *opposite* problem: `recommend::Recommendation` powers REPORT.md but
the HTML's report-mirror table is rebuilt independently in JavaScript
(`renderReportMirrorTable` lines 731-829 of the template), causing drift risk
already flagged in WR-01 (alphabetical-tiebreak winner pick had to be patched
in three places). v1.1 should NOT replicate that mistake for the new
per-cell recommendation prose.

**Established v1.0 convention for "render once in two places":** Server-side
Rust struct → two formatters (one Markdown, one HTML), with HTML rendered via
**tinytemplate** if the structure is non-trivial, or via **inline JS reading
JSON** if it's a flat list. Shipped examples:

- `Recommendation` → `markdown::emit_recommendations` only (HTML mirrors via
  JS — but mirror table operates on RESULTS, not on Recommendation, so they are
  *both* server-derived and *both* go through their own central tendency code).
- `ALL_DIAGRAMS` → `markdown::emit_allocator_diagrams` only (Mermaid is
  Markdown-only by design — UI-SPEC §Mermaid Theme Contract excludes runtime
  bundling).

**Concrete v1.1 shape:**

```rust
// crates/alloc-bench-aggregator/src/recommend.rs (extended)
#[derive(Debug, Clone, Serialize)]  // serde::Serialize is NEW (Recommendation isn't Serialize today)
pub struct CellRecommendation {
    pub rank: u8,                // 1..=10
    pub alloc: String,
    pub env: String,             // env_short, e.g. "alpine"
    pub composite_score: f64,    // 0.0..=100.0
    pub axes: BTreeMap<&'static str, f64>,  // 8 axis_name → 0-100
    pub strengths: Vec<String>,  // axes where this cell ≥ 80
    pub weaknesses: Vec<String>, // axes where this cell ≤ 40
    pub prose: String,           // pre-rendered rationale paragraph
}

pub fn top_n_cells(
    runs: &[Run],
    metas: &HashMap<(String, String), CellMeta>,
    security_metas: &HashMap<String, SecurityMeta>,
    n: usize,
) -> Vec<CellRecommendation>;
```

This single struct feeds **two emitters** via tinytemplate (extending the
existing pattern):

| Output | Path | Producer |
|--------|------|----------|
| `report/recommend-{rank:02}-{alloc}-{env}.md` (10 files) | `report/` | New `markdown::emit_per_cell_recommendations` rendering `templates/recommend-cell.md.tmpl` against each `CellRecommendation` |
| HTML fragment embedded in `index.html` (10 panels) | inside `<section class="top-n-recommendations">` | New `html::render_per_cell_panels` rendering `templates/recommend-cell.html.tmpl`, then injecting the rendered string fragment into the main template via a new `{ recommendation_panels_html \| unescaped }` placeholder |

**Why two tinytemplate files (one .md.tmpl, one .html.tmpl), not one shared
template:**

- Markdown table syntax (`| col | col |`) and HTML grid layout
  (`<div class="axis-row">`) have no useful overlap.
- tinytemplate is plain string substitution; it doesn't have a polymorphic
  abstraction over output format.
- The two templates *share data*, not shape — and the data is `CellRecommendation`,
  emitted server-side once. That's enough to prevent the drift WR-01 found.

**Linking from index.html / REPORT.md to the per-cell artifacts:**

- `REPORT.md` adds a new `## Top 10 cells` section listing markdown links to
  `recommend-{rank:02}-{alloc}-{env}.md` (relative paths — works on GitHub
  rendered preview AND `file://` open).
- `index.html` adds an inline HTML fragment per panel (no separate file fetch
  needed) — the dashboard remains self-contained and CSP-compliant
  (`connect-src 'self'` blocks any cross-origin fetch anyway).

**File-naming convention:** zero-padded rank prefix is intentional — keeps
`ls report/` alphabetical-by-rank, mirroring the BTreeMap output discipline.

---

### Q4 — Sidecar loader: one more named sidecar, or a generic abstraction?

**Answer:** One more named sidecar (`load_security_metas()`). Two named sidecars
is **NOT** rule-of-three territory — premature abstraction here would cost more
than it saves.

**Reasoning:**

The existing `load_cell_metas` is 23 LOC of body code (lines 79-101 of
`loader.rs`). Adding a near-clone for security gives ~46 LOC of sidecar plumbing
total — well under the threshold where a generic `load_sidecars<T: Sidecar>`
abstraction would pay for itself. Furthermore the two sidecar shapes differ:

| Aspect | `CellMeta` (v1.0) | `SecurityMeta` (v1.1) |
|--------|-------------------|------------------------|
| Key | `(alloc, env_short)` 2-tuple — 18 entries | `env_short` only — 6 entries |
| Source of truth | CI runtime (`docker image inspect`) — auto-populated | Hand-curated heuristic — committed |
| Cardinality | 1 file per cell | 1 file per env |
| Schema fields | `image_size_bytes`, `image_size_mb`, `build_time_s?`, `captured_at?` | `cve_count_30d`, `attack_surface_score`, `update_cadence_score`, `composite_security_score` |

Trying to unify `(alloc, env)` keys with `env`-only keys, plus auto-populated vs
hand-curated semantics, would force the abstraction to thread either an enum
or a callback through every callsite — exactly the over-engineering the
existing `load_cell_metas()` deliberately avoids.

**Concrete v1.1 placement:**

```rust
// crates/alloc-bench-aggregator/src/loader.rs (extended)

#[derive(Debug, Deserialize)]
pub struct SecurityMeta {
    pub env: String,                          // alpine | debian-slim | ...
    pub composite_security_score: f64,        // 0.0..=100.0 (already-normalized)
    pub cve_count_30d: u32,                   // raw input for axis label tooltip
    pub attack_surface_score: f64,            // 0.0..=100.0 sub-score
    pub update_cadence_score: f64,            // 0.0..=100.0 sub-score
    pub source: String,                       // citation for the heuristic
    pub captured_at: String,                  // RFC-3339
}

pub fn load_security_metas(pattern: &str) -> Result<HashMap<String, SecurityMeta>>;
```

**Mirrors `load_cell_metas` line-for-line:** empty pattern → empty map; glob +
sort + parse; per-file failures log `warn:` and skip-and-continue.

**`main.rs` wiring:** Add a third clap flag `--security` (defaulting to
empty string for byte-identical-output preservation when absent), call
`loader::load_security_metas(&cli.security)?`, thread the result into both
`markdown::write` and `html::write` via a new `&HashMap<String, SecurityMeta>`
parameter.

**Generic sidecar abstraction is a v2 concern**, not v1.1 — flagged in
`PITFALLS.md §3 Anti-Pattern: Premature sidecar abstraction`.

---

### Q5 — Direction-marker registry: central or per-call?

**Answer:** Central registry (`MEASUREMENT_AXES: [(name, direction); 8]`) in a
new module `axes.rs`. Per-call literals would invite the same drift WR-01
already exposed for tiebreak winner selection.

**Established v1.0 convention discovered:**

The v1.0 codebase already uses centralized alphabetical-emission registries for
exactly this sort of cross-emitter consistency. Two examples:

- `diagrams::ALL_DIAGRAMS: [(&str, &str); 4]` — name + body, alphabetical, used
  by `markdown.rs::emit_allocator_diagrams` line 320.
- `recommend::ALL_CLASSES: [WorkloadClass; 6]` — alphabetical iteration order
  consumed by `recommendations()` line 96.

Both are `const` arrays of tuples/enum variants; both are imported into
exactly one consumer; both have a unit test pinning their iteration order
(`diagrams::diagrams_in_alphabetical_emission_order`,
`recommend::winner_picker_alphabetical_class_order`).

**Concrete v1.1 placement:**

| File | Status | Purpose |
|------|--------|---------|
| `crates/alloc-bench-aggregator/src/axes.rs` | **NEW** | `pub enum Direction { Higher, Lower }`. `pub struct AxisSpec { pub key: &'static str, pub label: &'static str, pub direction: Direction }`. `pub const MEASUREMENT_AXES: [AxisSpec; 8] = [...]` — alphabetical. Helper `pub fn arrow(direction: Direction) -> &'static str` returning `"\u{2191}"` (↑) or `"\u{2193}"` (↓). Unit tests pin iteration order, arrow chars, and exhaustiveness vs the 8 spider-axis names. |
| `markdown.rs` | **MODIFIED** | Per-scenario table header builder reads `MEASUREMENT_AXES` to append `↑`/`↓` to each column header. |
| `polar.rs` | **NEW (re-uses)** | Spider-trace builder reads `MEASUREMENT_AXES` so the `theta` array (8 axis labels with arrows) and the `r` array (per-cell normalized values) stay in lockstep with the REPORT.md table headers and the `score::compute_axes()` direction logic. |
| `score.rs` | **NEW (re-uses)** | `normalize_axis()` reads `direction` from `MEASUREMENT_AXES`: for `Lower`-better metrics (latency, RSS, image size) it inverts the min-max so 100 = best regardless of raw direction. |
| Template `index.html.tmpl` | **MODIFIED** | Existing static layout titles (e.g. `'throughput (per scenario unit, …)'`) become server-injected — new tinytemplate placeholders `{ axis_label_throughput }`, `{ axis_label_latency }`, etc. — so the arrow markers stay in lockstep with REPORT.md. |

**The 8 axes (alphabetical key order, mapping to the milestone goals):**

| Key | Label | Direction | Source |
|-----|-------|-----------|--------|
| `channel-throughput` | Channel throughput | Higher | mean of spmc/mpsc/mpmc `ticks_per_s` |
| `cpu-bound` | CPU-bound throughput | Higher | cpu-bound `ticks_per_s` |
| `image-size` | Image size efficiency | Lower (smaller=better, inverted) | `meta.image_size_mb` |
| `memory` | Memory efficiency | Lower (smaller=better, inverted) | `metrics.peak_rss_kb` (or fragmentation ratio when available) |
| `multithread` | Multi-thread allocation | Higher | multithread `ticks_per_s` |
| `resilience` | Resilience (lock-contention) | Higher | contention `ticks_per_s` |
| `security` | Security posture | Higher (already 0-100) | `security_meta.composite_security_score` |
| `web` | Web service throughput | Higher | web `ticks_per_s` |

**Tradeoff acknowledged:** the central registry tightly couples `axes.rs` to
`recommend.rs`'s `WorkloadClass` enum (channel-heavy/contention/cpu-bound/
memory-bound/web — the perf 5 of the 8 axes overlap). The coupling is
*intentional* — it's the single source of truth that prevents WR-01-style
drift between `recommend.rs`'s rationale prose and the spider chart's
axis labels.

---

### Q6 — Build order

```
                ┌─────────────────────────────────────┐
                │ Phase A: Foundations (no deps)      │
                │  ─ axes.rs (registry)               │
                │  ─ loader::load_security_metas()    │
                │  ─ tests/fixtures/security/*.json    │
                └────────────────┬────────────────────┘
                                 │
                                 ▼
                ┌─────────────────────────────────────┐
                │ Phase B: Scoring + per-cell prose   │
                │  ─ score.rs (normalize, compute,    │
                │              top_n)                  │
                │  ─ recommend::top_n_cells           │
                │     (returns Vec<CellRecommendation>)│
                └────────────────┬────────────────────┘
                                 │
                                 ▼
                ┌─────────────────────────────────────┐
                │ Phase C: Emit per-cell artifacts    │
                │  ─ templates/recommend-cell.md.tmpl │
                │  ─ templates/recommend-cell.html.tmpl│
                │  ─ markdown::emit_per_cell_recommendations
                │  ─ html::render_per_cell_panels      │
                │  ─ index.html.tmpl spider <div> +    │
                │    panel section markup              │
                └────────────────┬────────────────────┘
                                 │
                                 ▼
                ┌─────────────────────────────────────┐
                │ Phase D: Spider chart               │
                │  ─ polar.rs (top_n traces builder)  │
                │  ─ html.rs HtmlContext extension    │
                │  ─ template Plotly.react wiring     │
                └────────────────┬────────────────────┘
                                 │
                                 ▼
                ┌─────────────────────────────────────┐
                │ Phase E: Direction markers wired    │
                │  ─ markdown.rs table headers        │
                │  ─ index.html.tmpl axis labels      │
                │  ─ "↑/↓ legend" 1-liner per surface  │
                └────────────────┬────────────────────┘
                                 │
                                 ▼
                ┌─────────────────────────────────────┐
                │ Phase F: Golden-fixture regen       │
                │  ─ tests/smoke.rs assertions        │
                │  ─ Once-only fixture refresh        │
                │  ─ Byte-identical thereafter        │
                └─────────────────────────────────────┘
```

**Ordering rationale (each Phase blocks the next):**

| Phase | Blocks | Why |
|-------|--------|-----|
| A | B, D, E | `score.rs` reads `axes.rs::MEASUREMENT_AXES`; `polar.rs` reads `axes.rs`; markdown direction-marker headers read `axes.rs`. Security sidecars must exist before `score::compute_axes` can normalize the security axis. |
| B | C, D | `CellRecommendation` is the single source of truth for both per-cell artifacts (Phase C) and spider trace JSON (Phase D); both consumers must wait until the struct exists. |
| C | F (independent of D) | Once C is done, the per-cell `.md` and HTML panel emission is fixture-pinnable. |
| D | F | Spider trace JSON contributes to the byte-identical test surface. |
| E | F | Direction-marker arrows change column-header bytes — the golden fixture cannot be regenerated until Phase E lands. |
| F | (release gate) | One-time regeneration; after Phase F the fixtures are pinned. |

**Why Phase A bundles three apparently-unrelated tasks:** they share the property
of being "leaf" additions (no consumers downstream until Phases B-E exist), and
landing them together gives Phase B a complete fixture set to test against.

---

## 3. Refactoring of v1.0 modules

**Required:** none. Every v1.1 feature can land via additive extensions
(new modules, new functions, new struct fields with `#[serde(default)]` or
new template placeholders).

**Tactical micro-changes** (not refactoring):

| File | Change | Justification |
|------|--------|---------------|
| `recommend.rs` | Add `serde::Serialize` to `Recommendation` (struct exists today without it). | Required only if v1.1 also wants to surface the *workload-class* recommendations table in HTML — currently it's Markdown-only. The new `CellRecommendation` struct needs `Serialize` regardless. |
| `html.rs::HtmlContext` | Add 2-3 new `&'a str` fields (`top_n_traces_json`, `recommendation_panels_html`, `axis_labels_json`). | Same pattern as Plan 02 / Plan 03 added 4 + 1 fields respectively — this is the v1.0-shipped extension shape. |
| `main.rs::Cli` | Add `--security` flag mirroring `--meta` (defaults to empty). | Single-line clap addition; backwards-compatible (`--security` absent → no security data → security axis = em-dash like the docker_runtimes table does today). |

**Risk audit:** the only structural risk is **template-placeholder count growth**
(currently 9 placeholders in `index.html.tmpl`; v1.1 adds ~4 more). tinytemplate
has no formal limit, but readers of the template need a clear comment block.
PLAN should require an updated header doc-comment in `html.rs` enumerating
every placeholder. (Lines 16-19 of `html.rs` already do this for v1.0 — extend
the list.)

---

## 4. New files vs Modified files (consolidated)

### New (Rust)

| Path | Purpose | LOC estimate |
|------|---------|--------------|
| `crates/alloc-bench-aggregator/src/axes.rs` | Direction-marker registry + `MEASUREMENT_AXES` const | ~80 |
| `crates/alloc-bench-aggregator/src/score.rs` | 0-100 normalization, `score_cells`, `top_n` | ~200 |
| `crates/alloc-bench-aggregator/src/polar.rs` | Spider-trace JSON builder | ~120 |

### New (templates)

| Path | Purpose |
|------|---------|
| `crates/alloc-bench-aggregator/templates/recommend-cell.md.tmpl` | Per-cell Markdown body (rank, axes table, prose, strengths/weaknesses) |
| `crates/alloc-bench-aggregator/templates/recommend-cell.html.tmpl` | Per-cell HTML panel fragment (CSS class hooks, no `<html>`/`<body>` wrapper — embedded into the main template) |

### New (fixtures)

| Path | Purpose |
|------|---------|
| `crates/alloc-bench-aggregator/tests/fixtures/security/{alpine,debian-slim,distroless-cc,distroless-static,scratch,wolfi}.json` (6 files) | Hand-curated heuristic security scores |

### Modified (Rust)

| Path | What changes |
|------|--------------|
| `src/main.rs` | `Cli`: add `--security` flag. `main()`: thread security metas through to writers. Plus matching unit tests. |
| `src/loader.rs` | Add `SecurityMeta` struct + `load_security_metas()` function + 4 unit tests. |
| `src/recommend.rs` | Add `CellRecommendation` struct + `top_n_cells()` function. Add `Serialize` derive on existing `Recommendation`. Existing 13 tests untouched. |
| `src/markdown.rs` | (a) Extend `write()` + `build_report()` signatures to take security metas. (b) Add `emit_per_cell_recommendations()` and a new `## Top 10 cells` section between Recommendations and Skipped. (c) Header builders read `axes::MEASUREMENT_AXES` for arrows. (d) Re-pin byte-identical-output test fixture. |
| `src/html.rs` | (a) Extend `HtmlContext` and `BuiltContext` with new fields. (b) Add `render_per_cell_panels()` that calls each `recommend-cell.html.tmpl` render then concatenates. (c) `build_context()` calls `polar::build_traces()` + `recommend::top_n_cells()`. |

### Modified (template)

| Path | What changes |
|------|--------------|
| `templates/index.html.tmpl` | (a) New `<div id="chart-spider" class="chart-card"></div>`. (b) New `<section class="top-n-recommendations">{ recommendation_panels_html \| unescaped }</section>`. (c) Existing axis-label hard-coded titles become `{ axis_label_* }` placeholders. (d) Top-of-file comment block enumerates *all* placeholders (existing + new). |

---

## 5. Quality-gate self-check

Mapping back to the `<quality_gate>` items:

- [x] Each v1.1 feature is mapped to a specific module — Q1 (polar.rs), Q2 (score.rs+recommend.rs), Q3 (recommend.rs CellRecommendation + 2 templates), Q4 (loader.rs SecurityMeta), Q5 (axes.rs).
- [x] New vs Modified files explicit — see §4 (3 new Rust modules + 2 new templates + 6 new fixtures; 5 modified Rust modules + 1 modified template).
- [x] Build order considers dependencies — see §2 Q6: A (registry+sidecar) → B (scoring+prose struct) → C (per-cell artifacts) → D (spider chart) → E (direction markers) → F (golden-fixture regen).
- [x] Refactoring justified — none required (§3); tactical micro-changes only.
- [x] Decorate-not-rewrite respected — §1.2 invariants explicitly preserved; v1 schema in `crates/alloc-bench-core/src/output.rs` is untouched. All v1.1 inputs ride on sidecars.
- [x] CV-pinned golden test preserved — §1.2; `multi_run.rs` is not in the v1.1 modify list. `score.rs` is orthogonal.

---

## 6. Open questions for the roadmapper

These are *not* blockers, but the roadmap should resolve them in PLAN.md:

1. **Does `--security` default to an empty string (matching `--meta`'s ergonomics) or to a sensible default like `meta/security/*.json`?** Empty preserves byte-identical output for existing local invocations; default-glob makes the dashboard show security data out of the box. Probably empty (PHASE-5 D-13 precedent).
2. **Should the spider chart appear in the standalone HTML even when fewer than 10 cells have data?** The 18-cell matrix is the canonical envelope — partial runs (CI smoke during development) would otherwise produce a sparse spider. Suggest: always emit, render up to N cells where N = min(10, available).
3. **What's the empty-pattern fallback for the security axis?** Mirror the docker_runtimes em-dash pattern: missing security_meta → security axis renders as 0 with an em-dash tooltip. Or skip the axis entirely (drop from 8 to 7)? The first option is more byte-identical-friendly.
4. **Should `axes.rs` co-locate with `recommend.rs` or live separately?** §Q5 recommends separate. Confirms with the v1.0 precedent (`diagrams.rs` separate from `markdown.rs` even though only one consumer).

---

## Sources

- `/Users/marc.carre/src/rust-benchmark-glibc-musl-mimalloc/CLAUDE.md` §Conventions (HIGH — locked policy)
- `/Users/marc.carre/src/rust-benchmark-glibc-musl-mimalloc/.planning/PROJECT.md` §"Current Milestone: v1.1" (HIGH)
- `/Users/marc.carre/src/rust-benchmark-glibc-musl-mimalloc/.planning/milestones/v1.0-research/ARCHITECTURE.md` (HIGH — v1.0 baseline)
- `/Users/marc.carre/src/rust-benchmark-glibc-musl-mimalloc/crates/alloc-bench-aggregator/src/{main,loader,multi_run,recommend,markdown,html,diagrams}.rs` (HIGH — read end-to-end)
- `/Users/marc.carre/src/rust-benchmark-glibc-musl-mimalloc/crates/alloc-bench-aggregator/templates/index.html.tmpl` (HIGH — 846 LOC, all sections inspected)
- `/Users/marc.carre/src/rust-benchmark-glibc-musl-mimalloc/crates/alloc-bench-aggregator/tests/smoke.rs` (HIGH — integration-test surface)

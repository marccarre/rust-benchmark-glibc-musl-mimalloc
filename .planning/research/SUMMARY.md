# Project Research Summary

**Project:** rust-benchmark-glibc-musl-mimalloc
**Milestone:** v1.1 — Recommendations, Spider Charts & Direction Markers (additive over shipped v1.0)
**Domain:** Aggregator pipeline extension (Rust + tinytemplate + Plotly) for an allocator-benchmark dashboard
**Researched:** 2026-05-26
**Confidence:** HIGH

## Executive Summary

v1.1 is **integration research, not greenfield architecture.** The v1.0 aggregator pipeline (`crates/alloc-bench-aggregator/` — 7 Rust modules + a single 846-LOC tinytemplate) is the architectural fact, and every v1.1 feature must slot into it without violating the five locked invariants in `CLAUDE.md` §Conventions: frozen v1 input schema in `output.rs`, alphabetical iteration via `BTreeMap`/`BTreeSet`, pinned numeric formatting (`{:.1}` / `{:.0}` / `{}`), single-timestamp-only non-stable line in REPORT.md, and the `[100, 110, 105]` → CV ≈ 4.7619% golden-value pin in `multi_run.rs`. Crucially, **the v1.0 stack is sufficient as-is**: zero new runtime crate dependencies, no Plotly upgrade (the pinned 2.35.3 bundle already ships `scatterpolar`), no SRI hash change, no Cargo.toml edits. All v1.1 features compose entirely on top of what v1.0 already shipped.

The recommended approach is **decorate-not-rewrite via three new Rust modules** (`axes.rs` for the direction-marker registry, `score.rs` for 0–100 normalization, `polar.rs` for spider-trace JSON), **two new tinytemplate fragments** (`recommend-cell.md.tmpl` and `recommend-cell.html.tmpl`), and **one new sidecar shape** (`meta/security/{env}.json`, mirroring the existing `CellMeta` plumbing). The build order is strictly serial — A: foundations (registry + security sidecar) → B: scoring + per-cell prose struct → C: per-cell artifacts (Markdown + HTML emit) → D: spider chart wiring → E: direction markers → F: one-time golden-fixture regeneration — because each phase produces the symbols the next consumes, and golden-fixture regeneration must be a standalone PR per the v1.0 byte-identical-output discipline. Tier display is opinionated: top-3 spider charts above the fold, top-5 in the recommendations table, top-10 reachable on demand (Cowan's 4 ± 1 working-memory bound).

The highest-leverage risks are silent-correctness traps that **pass schema-level tests while corrupting rankings**: direction-unaware normalization (latency renders inverted), p5/p95 winsorization is mathematically a no-op at N=18 (`floor(0.05 × 18) = 0`, `floor(0.95 × 18) = 17` — pure min/max), `HashMap` iteration order leaking into floating-point summation order in composite scores, the `↑`/`↓` glyph collision with column-sort indicators in BI tools, hard-coded prose strings desynchronizing the Markdown card from the HTML panel (the WR-01-style drift the v1.0 winner-tiebreak fix already exposed once), and any temptation to add `security_score` to `output.rs` instead of riding it on a sidecar. Each is preventable with a named test in `score.rs::tests` / `polar.rs::tests` / `loader.rs::tests` — listed in the "Looks Done But Isn't" checklist of PITFALLS.md and reproduced under "Gaps to Address" below.

## Key Findings

### Recommended Stack

**Zero new runtime crates.** v1.0's locked stack — `tinytemplate` 1, `serde_json` 1, `glob` 0.3, `chrono` 0.4, `tikv-jemallocator` 0.6.1, `mimalloc` 0.1.43, `hdrhistogram` 7.5, `axum` 0.8, `tokio` 1, `crossbeam-channel` 0.5, `vergen` 9, Plotly 2.35.3 via CDN with pinned SRI hash `sha384-MqL7Cy3it…lhPykM` — covers every v1.1 need. The pinned Plotly bundle already ships `scatterpolar`; the existing `tinytemplate` default formatter HTML-escapes `<`/`>`/`&`/`"`, which suffices for single-sentence per-cell prose with `*(suspect)*` italic markers; the `BTreeMap`/`BTreeSet` discipline carries forward unchanged.

**Core technologies (extended, not replaced):**
- **Plotly 2.35.3** (`scatterpolar` trace) — radar chart rendering; canonical idiom is 9-element `r`/`theta` arrays (close the polygon by repeating `r[0]` and `theta[0]`), `fill: 'toself'`, `polar.radialaxis.range: [0, 100]`, opacity 0.55–0.7 for overlay readability.
- **tinytemplate 1** — single-source-of-truth contract for both Markdown and HTML emission of the per-cell `CellRecommendation` struct (avoids the WR-01 drift v1.0 hit when JS rebuilt the report-mirror table independently).
- **Hand-rolled normalization (~15 LOC in `score.rs`)** — direction-aware min-max into `[0, 100]`; rejects `statrs`/`ndarray-stats` as overkill for arithmetic the project already does inline in `multi_run::aggregate`.
- **Hard-coded `\u{2191}` / `\u{2193}` constants** for direction markers — codebase already uses `\u{2014}` (em-dash) and `\u{00B7}` (middle dot); rejects `unicode-arrows` for two glyphs.
- **Optional v1.2 hedge:** `pulldown-cmark` 0.13.4 only if recommendation prose grows beyond single-sentence rationale (links, lists, headings); explicitly **deferred** for v1.1.

See [STACK.md](./STACK.md) for full per-question verdicts and integration points.

### Expected Features

**Must have (table stakes — milestone fails without these):**
- **Per-cell radar chart for top-N (env, alloc) cells** — single biggest UX delta over v1.0's tables.
- **0–100 normalization with direction-aware inversion** — without it, throughput-in-ops/s and latency-in-ns can't share an axis.
- **Direction markers in column headers** (REPORT.md + HTML) — non-negotiable for honesty; `Throughput ↑ (ops/s)`, `Latency p99 ↓ (ns)`.
- **Per-cell recommendation prose card** (TL;DR → strengths → weaknesses → recommended-for / avoid-for, 80–150 words, data-derived).
- **Single-line legend** above each table and chart explaining `↑`/`↓`/`⚠`.
- **Suspect-flag propagation** into spider charts and prose cards (mirrors v1.0's `*(suspect)*` rationale-suffix).
- **`meta/security/{env}.json` sidecars** (6 hand-curated files) for the security axis.

**Should have (competitive differentiators):**
- **Small-multiples spider grid** (top-3 in foreground, matrix-mean reference polygon overlay, 25% alpha fill).
- **Composite weighted-sum overall score with surfaced formula** in the prose card (equal weights = 1/8 per axis).
- **Visually distinct heuristic axes** (`(heuristic)` suffix + dashed gridline for image-size and security).
- **"Why this cell?" data-derived rationale** — `+{delta}% on {best_scenario}; -{delta}% on {worst_scenario}`.

**Defer (v1.2+):**
- **Pareto-front overlay** on the recommendations table (P2 — ship if budget permits).
- **JS axis-weighting slider** (P3 — breaks the static-`file://` contract).
- **Cross-version diff radar** (needs persisted historical results).
- **pulldown-cmark integration** (only if prose grows markdown features).

**Anti-features explicitly rejected:** all top-10 cells overlaid on one radar (>3 polygons becomes occluded), direction markers on every cell (visual noise + breaks `{:.1}` byte-stable formatting), heuristic axes without visual distinction (destroys credibility), hard-coded prose, raw min/max without winsorization, z-score normalization, top-10 above the fold (exceeds Cowan's 4 ± 1 bound).

See [FEATURES.md](./FEATURES.md) for the full landscape, prioritization matrix, and competitor analysis (mimalloc-bench, jemalloc paper, TechEmpower, hyperfine all lack the per-cell radar + auto-prose combination — v1.1 is genuinely novel in the space).

### Architecture Approach

**Three new Rust modules, two new templates, one new sidecar — every change additive.** The v1 input schema in `crates/alloc-bench-core/src/output.rs` is **NOT modified**; all new data rides on sidecars or is computed in `alloc-bench-aggregator` from existing v1 fields. `recommend.rs` gains a new `top_n_cells()` function and `CellRecommendation` struct (the existing `recommendations()` and 13 unit tests are untouched). The split between Rust modules (server-side aggregation, alphabetical key ordering, JSON serialization) and tinytemplate JS (per-chart `make*Traces`, DOM construction, `Plotly.react()`) is the v1.0-shipped pattern — `polar.rs` builds JSON server-side; the template's `makeSpiderTraces` reads it.

**Major components:**
1. **`axes.rs` (NEW, ~80 LOC)** — `MEASUREMENT_AXES: [AxisSpec; 8]` const registry with alphabetical key order; `Direction::{Higher, Lower}` enum; `arrow()` helper returning `\u{2191}` / `\u{2193}`. Single source of truth consumed by `score.rs`, `polar.rs`, and `markdown.rs` table-header builders — preventing the WR-01-style drift between rationale prose and chart axis labels.
2. **`score.rs` (NEW, ~200 LOC)** — pure-data module: `normalize_axis(values, direction)`, `compute_axes(runs, metas, security_metas)`, `score_cells(...)`, `top_n(scores, n)`. No prose; no template coupling. Pinned-golden tests for monotonicity, NaN handling, all-equal degenerate case, direction inversion, and alphabetical tie-breaking.
3. **`polar.rs` (NEW, ~120 LOC)** — spider-trace JSON builder; emits `{ r: [...9], theta: [...9], type: 'scatterpolar' }` per cell (9 not 8 — closes the polygon).
4. **`recommend.rs` (extended)** — adds `CellRecommendation` struct + `top_n_cells()` function + named constants `TOP_N_SPIDER = 3` / `TOP_N_TABLE = 5` / `TOP_N_TOTAL = 10` (single source of truth across both emitters).
5. **`loader.rs` (extended)** — `SecurityMeta` struct + `load_security_metas()` mirroring `load_cell_metas` line-for-line; **returns `BTreeMap<String, SecurityMeta>` not `HashMap`** (byte-identical-output discipline).
6. **`markdown.rs` + `html.rs` (extended)** — additive emit functions; `HtmlContext` gains `top_n_traces_json`, `recommendation_panels_html`, `axis_labels_json`. Both Markdown and HTML render the **same `CellRecommendation` struct** through two tinytemplate files (`recommend-cell.md.tmpl`, `recommend-cell.html.tmpl`) — the single struct prevents drift; the test `cell_templates_both_reference_all_fields` enforces field coverage.
7. **`templates/index.html.tmpl` (extended)** — new `<div id="chart-spider">`, new `<section class="top-n-recommendations">{ recommendation_panels_html | unescaped }</section>`, axis-label hardcoded titles converted to `{ axis_label_* }` placeholders.

See [ARCHITECTURE.md](./ARCHITECTURE.md) for the per-question integration mapping, the file-by-file change matrix (3 new + 5 modified Rust modules, 2 new + 1 modified template, 6 new fixtures), and the four open questions for the roadmapper to resolve in PLAN.md.

### Critical Pitfalls

Five **silent-correctness** traps that pass schema-level tests but corrupt rankings:

1. **Direction-unaware normalization** — `score.rs::normalize_axis` must take `Direction` as a non-optional parameter, not a boolean. Symptom: ptmalloc (worst latency, worst RSS) ranks above jemalloc on memory-sensitive axes; spider chart still looks plausible. Guard: `score::tests::lower_is_better_axis_inverts_correctly` — input `[100, 200, 300]` with `Direction::Lower` → `[100.0, 50.0, 0.0]`.

2. **p5/p95 winsorization is a no-op at N=18** — `floor(0.05 × 18) = 0`, `floor(0.95 × 18) = 17`, which are min and max. The textbook formula adopted from FEATURES.md §2 silently degrades to raw min/max, defeating the entire technique. Fix: use **p10/p90** (`floor(0.1 × 18) = 1` clips one cell per tail) or skip winsorization at N ≤ 20 with a documented decision comment. Guard: `score::tests::normalize_axis_p10_p90_clips_one_outlier_each_tail_at_n18`.

3. **Byte-identical-output drift via `HashMap` and FP summation order** — three sub-pitfalls compound: (a) `load_security_metas` must return `BTreeMap<String, SecurityMeta>` not `HashMap` (else iteration order varies across Rust toolchains); (b) `score_cells` composite-score sum must traverse `MEASUREMENT_AXES` constant order, not collect into a `Vec` (single-ULP drift from non-deterministic order corrupts ties); (c) the `score_cells` sort needs an alphabetical `(alloc, env)` tiebreak (`partial_cmp` returns `None` for NaN, and pdqsort is not stable for ties). Symptom: golden fixture passes locally, fails on a different OS or Rust version. Guards: `loader::tests::load_security_metas_returns_btreemap_sorted_by_env`, `score::tests::composite_score_summation_order_matches_axes_rs_constant_order`, `score::tests::tied_cells_break_alphabetically_for_determinism`.

4. **Per-cell template struct-field sync failure** — `CellRecommendation` feeds two tinytemplate files (`recommend-cell.md.tmpl`, `recommend-cell.html.tmpl`); tinytemplate silently ignores struct fields that aren't referenced. Adding a field (e.g., `pareto_front`) to one template but not the other produces inconsistent Markdown and HTML output without any compile-time error — the WR-01 drift pattern. Guard: `html::tests::cell_templates_both_reference_all_fields` renders both with sentinel values and asserts both outputs contain them.

5. **Mutating `crates/alloc-bench-core/src/output.rs` for security data** — the v1 schema is locked (CLAUDE.md §Conventions, Phase 1 D-11). Adding `security_score: Option<u8>` to `Run` would invalidate every committed `results/*.json` fixture and break readability of historical bench-runner output. Sidecar (`meta/security/{env}.json`) is the only correct path. Guard: `smoke::tests::v1_schema_output_rs_is_frozen` — pin a SHA-256 of `output.rs` at v1.0 freeze.

Six secondary pitfalls (MEDIUM severity): NaN propagation from `multi_run` raw `Run` data into `score.rs`, `(heuristic)` label is not a sufficient guardrail (heuristic axes contribute 25/100 = 25% of composite under equal weights — a cell can rise rank 4→1 on security alone), Plotly polygon not closed (missing 9th element in `r`/`theta`), Plotly CDN upgrade without trace-API audit (test: `plotly_sri_hash_unchanged`), top-N hierarchy depth duplicated as magic numbers, `↑`/`↓` collision with column-sort-indicator convention in BI tools (mitigation: 1-line legend explicitly disclaiming sort).

See [PITFALLS.md](./PITFALLS.md) for the full Pitfall-to-Phase mapping (22 named pitfalls, each with a named guard test) and the "Looks Done But Isn't" checklist that gates Phase F.

## Implications for Roadmap

The build order is **strictly serial** because each phase produces the symbols the next consumes, and Phase F (golden-fixture regeneration) must be a standalone PR per the v1.0 byte-identical-output discipline. The cross-cutting principle: **foundations → scoring → per-cell artifacts → spider chart → direction markers → golden regen.**

### Phase 1: Foundations (registry + security sidecar)
**Rationale:** `axes.rs` is the single source of truth consumed by Phases 2, 4, and 5; without it, drift is inevitable. Security sidecars are leaf additions (no consumers) so landing them here gives Phase 2 a complete fixture set. Frozen-schema test must be added now to prevent accidental `output.rs` mutation later.
**Delivers:** `axes.rs` (`MEASUREMENT_AXES` const + `Direction` enum + `arrow()` helper), `loader::SecurityMeta` + `load_security_metas` returning `BTreeMap`, 6 hand-curated `tests/fixtures/security/*.json` files, `--security` clap flag in `main.rs`.
**Addresses:** Security axis (FEATURES.md table-stake), direction-marker registry foundation.
**Avoids:** Pitfalls #5 (output.rs mutation), #3a (HashMap iteration), #16 (security score per-allocator misattribution).

### Phase 2: Scoring + per-cell prose struct
**Rationale:** `score.rs` is the keystone — every visual artifact depends on its output. Build it before any emitter. The split between data-only `score.rs` and prose-aware `recommend.rs::top_n_cells` keeps the existing 13 tests in `recommend.rs` untouched.
**Delivers:** `score.rs` (`normalize_axis`, `compute_axes`, `score_cells`, `top_n`), `recommend.rs::CellRecommendation` struct + `top_n_cells()` function, named constants `TOP_N_SPIDER` / `TOP_N_TABLE` / `TOP_N_TOTAL`.
**Uses:** Hand-rolled ~15-LOC normalizer (no new crate), `BTreeMap`/`BTreeSet` discipline, `MEASUREMENT_AXES` summation order.
**Implements:** 0–100 normalization with direction-aware inversion, composite scoring, top-10 generation set with alphabetical tiebreak.
**Avoids:** Pitfalls #1 (direction-unaware normalize), #2 (NaN propagation), #3 p5/p95-at-N=18 collapse (use p10/p90 or skip), #3b (FP summation order), #3c (NaN sort corruption), #5 tie-break non-determinism, #6 cross-axis normalization.

### Phase 3: Per-cell artifacts (Markdown + HTML emit)
**Rationale:** Single struct → two emitters via tinytemplate. Pre-empts WR-01-style drift the v1.0 winner-tiebreak fix already exposed once. Both templates must reference every field of `CellRecommendation` — enforced by a compile-time sentinel test.
**Delivers:** `templates/recommend-cell.md.tmpl`, `templates/recommend-cell.html.tmpl`, `markdown::emit_per_cell_recommendations`, `html::render_per_cell_panels`, `## Top 10 cells` section in REPORT.md, `<section class="top-n-recommendations">` in `index.html`.
**Implements:** Tier-display contract (top-3 / top-5 / top-10 via named constants), suspect-flag propagation, `(heuristic)` qualifier discipline.
**Avoids:** Pitfalls #11 (hard-coded prose), #12 (template field-sync), #10 (tier depth duplicated as magic numbers), #17 (absolute security-score framing).

### Phase 4: Spider chart wiring
**Rationale:** `polar.rs` consumes `score.rs` output; the chart `<div>` and Plotly trace builder slot into `index.html.tmpl`'s existing `make*Traces` pattern.
**Delivers:** `polar.rs` (top-N traces builder, **9-element `r`/`theta` arrays — close the polygon**), `HtmlContext.top_n_traces_json`, `templates/index.html.tmpl` `<div id="chart-spider">` + `makeSpiderTraces` JS function + `Plotly.react()` wiring.
**Uses:** Plotly 2.35.3 `scatterpolar` (already in pinned bundle — no SRI change), reference matrix-mean polygon overlay, per-allocator color map from v1.0 `ALLOC_COLORS`.
**Implements:** Top-3 above-the-fold spider grid, dashed gridline for heuristic axes.
**Avoids:** Pitfalls #4 (`(heuristic)` formula guardrail), #8 (open polygon), #9 (Plotly CDN upgrade without trace-API audit — pin SRI hash test).

### Phase 5: Direction markers wired
**Rationale:** Direction markers are the leaf surface visible to readers; wiring them last lets every upstream module use `axes.rs` consistently. Markdown table headers become arrow-decorated; HTML axis labels become server-injected from `axes.rs`.
**Delivers:** `markdown.rs` per-scenario table headers carrying `↑`/`↓`, `index.html.tmpl` axis labels via `{ axis_label_* }` placeholders, single-line legend above every table and chart, `aria-label` wrappers in HTML for WCAG 2.1 SC 1.3.3.
**Implements:** Direction-marker placement in column headers only (cells stay numeric — preserves byte-stable `{:.1}` formatting).
**Avoids:** Pitfalls #13 (sort-indicator collision — legend disclaims), #14 (font rendering variance — `aria-label` future-proofs), #15 (GitHub Markdown table width — keep arrow, drop unit suffix to legend).

### Phase 6: Golden-fixture regeneration
**Rationale:** **Standalone PR with no production code.** Phase F must be visibly separate from Phases 1–5 so reviewers can verify the regeneration was intentional. Direction-marker arrows change column-header bytes; spider-trace JSON contributes to the byte-identical surface — neither can be pinned until Phase 5 lands.
**Delivers:** Refreshed `tests/fixtures/*.json`, refreshed `tests/smoke.rs` golden assertions, PR description listing the byte count of each updated fixture and the `just aggregate` invocation used.
**Avoids:** Pitfall #19 (silent absorption of fixture diff inside production-code PR).

### Phase Ordering Rationale

- **A blocks B, D, E** because `score.rs` and `polar.rs` and markdown header builders all read `MEASUREMENT_AXES`; security sidecars must exist before `score::compute_axes` can normalize the security axis.
- **B blocks C, D** because `CellRecommendation` is the single source of truth for both per-cell artifacts (Phase C) and spider trace JSON (Phase D).
- **C and D are independent of each other** but both depend on B — they could in principle run in parallel, but landing them serially keeps the PR review surface small and gives Phase E a single rebase target.
- **E blocks F** because direction-marker arrows change column-header bytes — the golden fixture cannot be regenerated until Phase E lands.
- **F is the release gate.** Standalone PR convention is non-negotiable.

### Research Flags

**Phases that need deeper research during planning** (recommend `/gsd:plan-phase --research-phase <N>`):

- **Phase 2 (Scoring):** the p5/p95-vs-p10/p90-vs-skip-at-N=18 decision is unresolved in FEATURES.md — needs a numerical-stability research pass against the 18-cell canonical matrix, ideally with a dry-run on the v1.0 committed fixtures to see which technique gives the most informative spread.
- **Phase 4 (Spider chart):** Plotly's per-axis gridline-style support is unclear — `polar.radialaxis` does not directly support per-axis dash, so the heuristic-axis visual distinction may need a workaround (per-tick `tickfont` overrides, or a separate angular-axis-tickvals trick). Worth verifying against Plotly v2.35.3 docs before committing the API.
- **Phase 6 (Golden-fixture regen):** the convention that fixture-regen PRs must be standalone is currently informal — should be promoted to a CLAUDE.md §Conventions entry before Phase A starts.

**Phases with standard patterns** (skip research-phase, follow established v1.0 conventions):

- **Phase 1 (Foundations):** `loader.rs` sidecar pattern is shipped; `axes.rs` mirrors `diagrams::ALL_DIAGRAMS` and `recommend::ALL_CLASSES`. Pure mechanical extension.
- **Phase 3 (Per-cell artifacts):** tinytemplate emit pattern is shipped (`emit_recommendations`, `emit_docker_runtimes_table`); two new templates is mechanical.
- **Phase 5 (Direction markers):** column-header decoration is a string-builder edit in `markdown.rs`; `index.html.tmpl` placeholder injection mirrors v1.0 `{ generated_at }` / `{ results_json }`.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Plotly `scatterpolar` verified in v2.35.3 source tree (`src/traces/scatterpolar/index.js`); SRI hash unchanged; tinytemplate default-escape behavior verified in `html.rs:88-93` doc comment; pulldown-cmark deferred decision grounded in existing v1.0 escape pipeline |
| Features | HIGH-MEDIUM | 8-axis count, top-3/5/10 tiers, min-max with winsorization, `↑`/`↓` in headers all converge from multiple authoritative sources (Wikipedia, datavizcatalogue, Spotfire, Cowan working-memory revision); per-cell prose card structure synthesized from Lighthouse + AWS Trusted Advisor + web.dev with no single canonical spec |
| Architecture | HIGH | Entire v1.0 baseline read end-to-end (846-LOC template + 7 Rust modules + tests); every v1.1 file placement grounded in a named v1.0 precedent; "decorate-not-rewrite" invariants enumerated in §1.2 of ARCHITECTURE.md |
| Pitfalls | HIGH | Every pitfall references a named existing or proposed file; named guard tests proposed for each; mapped to phases with no orphans |

**Overall confidence:** HIGH. v1.0 is in-tree and was read in full; v1.1 is purely additive over a frozen schema; every cross-cutting risk has a named test guard.

### Gaps to Address

- **Winsorization choice at N=18** is unresolved (Phase 2 research flag). FEATURES.md §2 says p5/p95; PITFALLS.md #3 proves this is mathematically a no-op. Roadmap PLAN.md must pick: (a) p10/p90 — clips one cell per tail; (b) skip winsorization, document at N ≤ 20; (c) fixed clamp at p15/p85. Recommendation: (a) or (b); decide before implementing `score.rs::normalize_axis`.

- **Heuristic-axis weight cap** under equal weights — a cell can rise from rank 4 to rank 1 by scoring well on the two heuristic axes alone (25% of composite). Roadmap should decide whether to keep equal weights (milestone spec) or cap aggregate heuristic weight at 12.5% (one axis worth). Test `score::tests::heuristic_axes_cannot_promote_worst_measured_cell_to_top_1` is intentionally designed to force this discussion before Phase 2 ships.

- **Plotly per-axis gridline styling** for heuristic axes — `polar.radialaxis` does not directly support per-axis dash; workaround needed via `tickfont` overrides or angular-axis trick. Phase 4 research flag.

- **Empty-pattern fallback for the security axis** when `meta/security/*.json` is absent — should missing security_meta render as 0 with an em-dash tooltip (mirrors docker_runtimes), or drop the axis entirely (8→7)? ARCHITECTURE.md §6 leaves this open. Recommendation: em-dash fallback for byte-identical-output preservation.

- **`--security` default value** — empty string (matches `--meta`) preserves byte-identical output when absent; `meta/security/*.json` glob default ships richer dashboards out of the box. Phase 1 decision; recommend empty string per Phase-5 D-13 precedent.

## Sources

### Primary (HIGH confidence)
- `crates/alloc-bench-aggregator/src/{main,loader,multi_run,recommend,markdown,html,diagrams}.rs` — read end-to-end; every architectural claim grounded in a named symbol.
- `crates/alloc-bench-aggregator/templates/index.html.tmpl` — 846 LOC, all sections inspected.
- `crates/alloc-bench-aggregator/tests/smoke.rs` — integration-test surface and golden-fixture pattern.
- `CLAUDE.md` §Conventions — locked policy: byte-identical output, `BTreeMap` mandate, decorate-not-rewrite, suspect-flag definition, frozen v1 schema.
- `.planning/PROJECT.md` §"Current Milestone: v1.1" — milestone scope, eight-axis selection, six target Docker images.
- Plotly v2.35.3 source — `https://github.com/plotly/plotly.js/blob/v2.35.3/src/traces/scatterpolar/index.js` (verifies `scatterpolar` ships in pinned bundle).
- Context7 `/plotly/graphing-library-docs` — canonical multi-trace radar pattern (`fill: 'toself'` + `polar.radialaxis.range` + `polar.angularaxis.direction`).
- Context7 `/pulldown-cmark/pulldown-cmark` — 0.13.4 API for the v1.2 deferred path.
- Wikipedia: _Radar chart_, _Feature scaling_, _Winsorizing_, _Multi-criteria decision analysis_, _Pareto efficiency_, _Working memory_ (Cowan revision), _Miller's law_, comparison tables for TLS implementations and RDBMS.

### Secondary (MEDIUM-HIGH confidence)
- The Data Visualisation Catalogue (datavizcatalogue.com), Spotfire glossary, Storytelling-with-Data, Scott Logic — radar-chart axis-count and overlap conventions converge on the same advice.
- Lighthouse Opportunities + Diagnostics, AWS Trusted Advisor, web.dev Core Web Vitals, hyperfine "Relative" column — recommendation-card structural pattern (definition → diagnosis → action).
- `.planning/milestones/v1.0-research/{STACK,ARCHITECTURE,FEATURES,PITFALLS}.md` — v1.0 baseline (this milestone delta is layered on top).

### Tertiary (LOW confidence)
- CIS Docker Benchmark — exists as PDF benchmark with no public JSON scoring schema; the verdict here is **not to mirror it**, so the schema gap doesn't change the answer.
- Spider-axis ordering rationale (alphabetical vs domain-grouped) — Scott Logic's critique flags ordering ambiguity but does not specify a canonical resolution. Alphabetical chosen to match v1.0 BTreeMap discipline.

---
*Research completed: 2026-05-26*
*Ready for roadmap: yes*

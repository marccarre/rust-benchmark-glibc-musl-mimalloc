# Roadmap: rust-benchmark-glibc-musl-mimalloc

## Milestones

- ✅ **v1.0 MVP** — Phases 1-5 (shipped 2026-05-19)
- 🛠 **Post-ship surgical fixes** — Phase 5.1 (UAT gap closure, opened 2026-05-23, completed 2026-05-22)
- 🚧 **v1.1 Recommendations, Spider Charts & Direction Markers** — Phases 6-11 (active, started 2026-05-26)

## v1.1 Phases (active)

Phase numbering continues from v1.0 (last phase = 5; surgical patch = 5.1) → v1.1 starts at Phase 6. Build order is **strictly serial**: Phase 6 blocks 7+9+10 (axes registry consumed by all three); Phase 7 blocks 8+9 (CellRecommendation + score outputs); Phase 10 blocks 11 (direction-marker arrows change column-header bytes; golden fixture cannot be regenerated until Phase 10 lands).

- [ ] **Phase 6: Foundations** — Axes registry, security sidecars, frozen-schema guard
- [ ] **Phase 7: Scoring & Top-N** — Direction-aware normalization, composite scoring, recommendation struct
- [ ] **Phase 8: Per-cell Artifacts** — Markdown + HTML cards via two templates with sync sentinel
- [ ] **Phase 9: Spider Chart** — `polar.rs` scatterpolar trace builder + chart wiring + Pareto overlay
- [ ] **Phase 10: Direction Markers** — Column headers + axis labels + legend + a11y
- [ ] **Phase 11: Golden-fixture Regen** — Standalone PR; byte-identical pinning

## v1.1 Phase Details

### Phase 6: Foundations ✅

**Goal:** Land the leaf additions that every downstream phase consumes — the `MEASUREMENT_AXES` registry, the security sidecar plumbing, and the frozen-schema CI gate that prevents accidental v1 schema mutation. No consumers downstream until Phase 7+ exist; landing them together gives Phase 7 a complete fixture set to test against.
**Depends on:** Nothing (first v1.1 phase; v1.0 + 5.1 already shipped)
**Requirements:** AXES-01, AXES-02, SEC-01, SEC-02, SEC-03, GUARD-01
**Success Criteria** (what must be TRUE):

  1. User reads `crates/alloc-bench-aggregator/src/axes.rs` and finds `MEASUREMENT_AXES: [AxisSpec; 8]` (alphabetical key order) plus `Direction::{Higher, Lower}` enum and `arrow()` helper returning `\u{2191}` / `\u{2193}`
  2. User runs `just aggregate --security 'meta/security/*.json'` and the aggregator loads six hand-curated `meta/security/{env}.json` sidecars (alpine, debian-slim, distroless-cc, distroless-static, scratch, wolfi) via `loader::load_security_metas()` returning `BTreeMap<String, SecurityMeta>` (NOT `HashMap`)
  3. User runs `just aggregate` without `--security` and the aggregator falls back to `score = 0` with em-dash tooltip in the security axis (mirrors v1.0 docker_runtimes em-dash convention)
  4. User runs `cargo test smoke::tests::v1_schema_output_rs_is_frozen` and the test pins a SHA-256 of `crates/alloc-bench-core/src/output.rs` to its v1.0 freeze — guards against accidental v1 schema mutation

**Plans:** 3 plans
Plans:
**Wave 1**

- [x] 06-01-PLAN.md — `axes.rs` registry + Direction enum + arrow() helper (AXES-01, AXES-02)
- [x] 06-03-PLAN.md — Frozen-schema gate (`smoke::tests::v1_schema_output_rs_is_frozen` + `sha2` dev-dep) (GUARD-01)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 06-02-PLAN.md — Security loader (`SecurityMeta` + `load_security_metas`) + `--security` CLI flag + 6 committed sidecars (SEC-01, SEC-02, SEC-03)

**Open Questions** (defer to `/gsd:plan-phase 6`):

  - `--security` flag default value: empty string (matches `--meta` ergonomics, preserves byte-identical output when absent) vs. `meta/security/*.json` glob (richer dashboard out of the box). Recommend empty string per Phase-5 D-13 precedent.
  - Empty-pattern fallback for the security axis when `meta/security/*.json` is absent: render `score = 0` with em-dash tooltip vs. drop the axis entirely (8 → 7). Recommend em-dash fallback for byte-identical-output preservation and stable 8-axis spider shape.

### Phase 7: Scoring & Top-N

**Goal:** Build the scoring keystone — `score.rs` (direction-aware normalization, composite scoring with summation-order discipline, alphabetical tiebreak) and the `CellRecommendation` struct + `top_n_cells()` extension to `recommend.rs`. Every visual artifact in Phases 8 and 9 depends on this output. The split between data-only `score.rs` and prose-aware `recommend.rs::top_n_cells` keeps the existing 13 tests in `recommend.rs` untouched.
**Depends on:** Phase 6 (consumes `MEASUREMENT_AXES` constant order, `Direction` enum, `SecurityMeta` BTreeMap)
**Requirements:** SCORE-01, SCORE-02, SCORE-03, SCORE-04, REC-01, REC-02, TEST-03, TEST-04, TEST-05
**Success Criteria** (what must be TRUE):

  1. User runs `cargo test score::tests::lower_is_better_axis_inverts_correctly` and the test passes — input `[100.0, 200.0, 300.0]` with `Direction::Lower` yields `[100.0, 50.0, 0.0]`
  2. User runs `cargo test score::tests::normalize_axis_p10_p90_clips_one_outlier_each_tail_at_n18` and the test passes — confirms p10/p90 winsorization clips one cell per tail at N=18 (where p5/p95 would collapse to raw min/max)
  3. User runs `cargo test score::tests::composite_score_summation_order_matches_axes_rs_constant_order` and the test passes — composite score sums via `MEASUREMENT_AXES` constant traversal, not via a collected `Vec`
  4. User runs `cargo test score::tests::tied_cells_break_alphabetically_for_determinism` and the test passes — composite-score ties broken by `(alloc, env)` alphabetical secondary sort
  5. User reads `crates/alloc-bench-aggregator/src/recommend.rs` and finds the new `CellRecommendation` struct (rank, alloc, env, composite_score, axes, tldr, strengths, weaknesses, recommended_for, avoid_for, suspect_flag), `top_n_cells()` function, and named constants `TOP_N_SPIDER = 3` / `TOP_N_TABLE = 5` / `TOP_N_TOTAL = 10` — existing 13 unit tests for `recommendations()` are untouched

**Plans:** TBD
**Open Questions** (defer to `/gsd:plan-phase 7`):

  - Dry-run the chosen p10/p90 winsorization on the v1.0 committed fixtures to verify spread is informative (not flat). If p10/p90 still compresses meaningfully, document fixed-clamp fallback as a TODO.
  - Heuristic-axis weight cap: equal weights = 1/8 per axis means the two heuristic axes (image-size, security) contribute 25/100 = 25% of composite. Test `score::tests::heuristic_axes_cannot_promote_worst_measured_cell_to_top_1` is intentionally designed to force this discussion before scoring ships. Decide: keep equal weights (milestone spec, ship as-is) vs. cap aggregate heuristic weight at 12.5% (one axis worth — defers to v1.2).

### Phase 8: Per-cell Artifacts

**Goal:** Render `CellRecommendation` through two tinytemplate files (`recommend-cell.md.tmpl` for Markdown card, `recommend-cell.html.tmpl` for HTML panel) — single struct → two outputs → drift caught at compile time. Pre-empts WR-01-style drift the v1.0 winner-tiebreak fix already exposed once. Adds a `## Top 10 cells` section to REPORT.md and a `<section class="top-n-recommendations">` to `index.html`.
**Depends on:** Phase 7 (consumes `CellRecommendation` struct + `TOP_N_*` named constants)
**Requirements:** CELL-01, CELL-02, CELL-03, CELL-04, CELL-05
**Success Criteria** (what must be TRUE):

  1. User reads `crates/alloc-bench-aggregator/templates/recommend-cell.md.tmpl` and `crates/alloc-bench-aggregator/templates/recommend-cell.html.tmpl` and finds two tinytemplate files driven by the same `CellRecommendation` struct — Markdown card and HTML panel are field-by-field identical
  2. User runs `cargo test html::tests::cell_templates_both_reference_all_fields` and the test passes — renders both templates with sentinel values and asserts both outputs contain every sentinel (the WR-01 pattern)
  3. User runs `just aggregate` and the aggregator writes ten Markdown files `report/recommend-{rank:02d}-{alloc}-{env}.md` and ten HTML fragments `report/recommend-{rank:02d}-{alloc}-{env}.html` (rank zero-padded for natural filename sort)
  4. User reads `report/REPORT.md` and finds a new `## Top 10 cells` section with the top-5 recommendation cards above the fold and the remaining 5 inside a collapsible `<details>` block (Cowan's 4±1 working-memory bound)
  5. User reads each per-cell artifact and finds the structure: TL;DR (1 sentence) → Strengths → Weaknesses → Recommended-for → Avoid-for, 80–150 words total, data-derived (no hand-edited prose strings — the `*(suspect)*` italic suffix from v1.0 is the only allowed annotation)

**Plans:** TBD
**UI hint**: yes

### Phase 9: Spider Chart

**Goal:** Build `polar.rs` — server-side `scatterpolar` trace JSON builder consumed by `index.html.tmpl`'s new `<div id="chart-spider">`. Top-3 cells render above the fold as a small-multiples grid; matrix-mean reference polygon overlaid at 25% alpha for context; heuristic axes visually distinguished; Plotly SRI hash pinned by test to prevent silent trace-API drift on Plotly upgrade.
**Depends on:** Phase 6 (consumes `MEASUREMENT_AXES`), Phase 7 (consumes `CellScore` / `CellRecommendation` for top-N selection)
**Requirements:** POLAR-01, POLAR-02, POLAR-03, POLAR-04, POLAR-05
**Success Criteria** (what must be TRUE):

  1. User runs `cargo test polar::tests::trace_closes_polygon_with_9_elements` and the test passes — confirms 9-element `r`/`theta` arrays (not 8) closing the polygon by repeating `r[0]` and `theta[0]`
  2. User opens `report/index.html` and sees a new `<div id="chart-spider">` rendering the top-3 cells above the fold as a small-multiples grid, with a matrix-mean reference polygon overlaid at 25% alpha
  3. User opens `report/index.html` and sees the spider chart's heuristic axes (image-size efficiency, security posture) visually distinguished — `(heuristic)` suffix on the axis label and dashed gridline (or equivalent Plotly per-axis styling — Phase 9 plan resolves the exact mechanism)
  4. User runs `cargo test html::tests::plotly_sri_hash_unchanged` and the test passes — pins the Plotly CDN URL + SRI hash to v2.35.3, guarding against a future contributor "upgrading" Plotly without verifying `scatterpolar` trace shape
  5. User opens `report/index.html` and finds the v1.0 Recommendations table extended with a Pareto-front overlay column (P2 differentiator — cells on the Pareto front of `composite_score` vs `image_size_mb` carry a marker)

**Plans:** TBD
**UI hint**: yes
**Open Questions** (defer to `/gsd:plan-phase 9`):

  - Plotly per-axis gridline styling for heuristic axes: `polar.radialaxis` does not directly support per-axis dash. Workaround needed via `tickfont` overrides or angular-axis-tickvals trick. Verify scatterpolar API in Plotly v2.35.3 docs before committing trace shape.
  - Should the spider chart appear in the standalone HTML even when fewer than 10 cells have data? Suggest: always emit, render up to N cells where N = min(10, available).
  - Pareto-front overlay (POLAR-05) is P2 — defer-friendly if Phase 9 budget tight; may move to v1.2 if heuristic-axis distinction (POLAR-03) takes longer than estimated.

### Phase 10: Direction Markers

**Goal:** Wire the leaf surface readers see — every measurement column header in REPORT.md and every chart axis label in `index.html` carries `↑` / `↓` glyphs drawn from `axes.rs::arrow()`. One-line legend above every per-scenario table explicitly disclaims that arrows are direction markers, not column-sort indicators. WCAG 2.1 SC 1.3.3 satisfied via `aria-label` wrappers in HTML. Markers live in column headers only — cells stay numeric (preserves byte-stable `{:.1}` / `{:.0}` / `{}` formatting).
**Depends on:** Phase 6 (consumes `axes::arrow()` helper)
**Requirements:** DIR-01, DIR-02, DIR-03, DIR-04, DIR-05
**Success Criteria** (what must be TRUE):

  1. User reads any per-scenario allocator-comparison table in REPORT.md and finds `↑` or `↓` in every measurement column header — e.g., `Throughput ↑ (ops/s)`, `Latency p99 ↓ (ns)`, `Peak RSS ↓ (MB)`
  2. User reads REPORT.md and finds a one-line legend above every per-scenario table: `↑ higher is better · ↓ lower is better · ⚠ suspect run` — explicitly disclaims that the arrows are direction markers, not column-sort indicators
  3. User opens `report/index.html` and finds every Plotly chart's axis label injected from `axes.rs` via `{ axis_label_* }` template placeholders carrying the same `↑` / `↓` glyphs as REPORT.md (single source of truth — no hard-coded labels in `index.html.tmpl`)
  4. User reads `index.html` source and finds each direction-marker glyph wrapped in `<span aria-label="higher is better">↑</span>` (or `lower is better` for `↓`) for WCAG 2.1 SC 1.3.3 screen-reader accessibility
  5. User reads REPORT.md cells and finds them unchanged from v1.0 byte-stable formatting — `{:.0}` for medians in multi-run cells, `{:.1}` for throughputs in single-run cells, `{}` for ns latencies (direction markers live in headers only, never in cells)

**Plans:** TBD
**UI hint**: yes

### Phase 11: Golden-fixture Regen

**Goal:** **Standalone PR with no production code.** Direction-marker arrows change column-header bytes; spider-trace JSON contributes to the byte-identical surface — neither can be pinned until Phases 7-10 land. The PR description must list the byte count of each updated fixture and the `just aggregate` invocation used. This is the v1.1 release gate.
**Depends on:** Phase 10 (direction-marker arrows are the last byte-changing addition; golden fixture stable only after Phase 10)
**Requirements:** TEST-01, TEST-02
**Success Criteria** (what must be TRUE):

  1. User runs `cargo test` and **all v1.0 byte-identical-output golden tests still pass** — every byte that v1.0 emitted is unchanged for inputs that don't include security sidecars (security-axis em-dash fallback applies)
  2. User reads the v1.1 PR list and finds Phase 11 (golden-fixture regeneration) shipped as a single standalone PR with no production code — Phases 6-10 PRs each carry no fixture-byte changes (test fails loudly until Phase 11 lands), so reviewer can verify the regeneration was intentional

**Plans:** TBD
**Open Questions** (defer to `/gsd:plan-phase 11`):

  - The standalone-PR convention should be promoted to a CLAUDE.md §Conventions entry before Phase 11 ships, codifying the rule for future milestones (e.g., v1.2 spider-chart additions, v2 allocator-matrix expansion).

## Phases

<details>
<summary>✅ v1.0 MVP (Phases 1-5) — SHIPPED 2026-05-19</summary>

- [x] Phase 1: Foundation MVP Slice (2/2 plans) — completed 2026-05-18
- [x] Phase 2: Scenario Fan-Out (3/3 plans) — completed 2026-05-18
- [~] Phase 3: Docker Matrix & Local Orchestration (3/5 plans; 03-04 + 03-05 deferred to Phase 5 CI per commit 9cab5c2)
- [x] Phase 4: Aggregator & Dashboard (3/3 plans) — completed 2026-05-19
- [x] Phase 5: CI, Image-Size Gate & Public Polish (4/4 plans) — completed 2026-05-19

See `.planning/milestones/v1.0-ROADMAP.md` for full phase details and `.planning/milestones/v1.0-MILESTONE-AUDIT.md` for the audit report.

</details>

### Phase 5.1: UAT Gap Closure (post-v1.0)

**Goal:** Close the two UAT blockers found 2026-05-23 by `/gsd:verify-work` against the deferred Phase 5 UAT items: (1) all 18 cells SIGSEGV on Apple Silicon (Rosetta+v3 incompatibility — REPR-01); (2) GHA aggregate-report job fails because `Reorganize artifacts` step's `mv` patterns target the wrong directory level (ORCH-04). v1.0 archive is read-only; fix plans live in `.planning/phases/05.1-uat-gap-closure/` and ship as a 5.1 surgical patch on top of the v1.0 release.

**Requirements:** REPR-01, ORCH-04

**Plans:** 2/2 plans complete

Plans:

- [x] 05.1-01-PLAN.md — Apple Silicon Rosetta+v3 SIGSEGV fix (Dockerfiles + justfile + README)
- [x] 05.1-02-PLAN.md — GHA aggregate-step mv-source path fix (.github/workflows/bench.yml)

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Foundation MVP Slice | v1.0 | 2/2 | Complete | 2026-05-18 |
| 2. Scenario Fan-Out | v1.0 | 3/3 | Complete | 2026-05-18 |
| 3. Docker Matrix & Local Orchestration | v1.0 | 3/5 | Partial (deferred to Phase 5 CI) | 2026-05-19 |
| 4. Aggregator & Dashboard | v1.0 | 3/3 | Complete | 2026-05-19 |
| 5. CI, Image-Size Gate & Public Polish | v1.0 | 4/4 | Complete | 2026-05-19 |
| 5.1. UAT Gap Closure | post-v1.0 | 2/2 | Complete   | 2026-05-22 |
| 6. Foundations | v1.1 | 0/3 | Not started | - |
| 7. Scoring & Top-N | v1.1 | 1/2 | In Progress|  |
| 8. Per-cell Artifacts | v1.1 | 0/? | Not started | - |
| 9. Spider Chart | v1.1 | 0/? | Not started | - |
| 10. Direction Markers | v1.1 | 0/? | Not started | - |
| 11. Golden-fixture Regen | v1.1 | 0/? | Not started | - |

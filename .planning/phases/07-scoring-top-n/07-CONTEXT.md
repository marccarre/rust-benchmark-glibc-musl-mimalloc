---
phase: 7
phase_name: Scoring & Top-N
gathered: 2026-05-26
status: Ready for planning
---

# Phase 7: Scoring & Top-N - Context

**Gathered:** 2026-05-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Build the scoring keystone for v1.1:

1. NEW file `crates/alloc-bench-aggregator/src/score.rs` — direction-aware normalization, p10/p90 winsorization, composite weighted-sum scoring with `MEASUREMENT_AXES` constant-order summation, top-N selection with `(alloc, env)` alphabetical tiebreak
2. EXTEND `crates/alloc-bench-aggregator/src/recommend.rs` — add `CellRecommendation` struct + `top_n_cells()` function + named constants `TOP_N_SPIDER = 3 / TOP_N_TABLE = 5 / TOP_N_TOTAL = 10`. The existing `recommendations()` function and its 13 unit tests are untouched
3. NEW unit-test set covering SCORE-01..04, REC-01, REC-02, TEST-03, TEST-04, TEST-05

`score.rs` produces data only — no prose, no rendering. `recommend.rs::top_n_cells()` is the prose-aware layer that consumes scores and produces `CellRecommendation` records (axis breakdown + tldr/strengths/weaknesses/recommended_for/avoid_for/suspect_flag). The split keeps the existing 13 `recommendations()` tests untouched and isolates Phase 8/9 template churn from Phase 7 scoring math.

**Out of scope:** Templates / per-cell artifacts (Phase 8), spider chart rendering (Phase 9), direction-marker glyphs in headers / legends (Phase 10), golden-fixture regeneration (Phase 11), heuristic-axis weight cap (V12-07), workload-shape weighted profiles (V12-05), JSON-driven re-weighting slider (V12-01).

</domain>

<decisions>
## Implementation Decisions

### Heuristic Weight Policy (SCORE-03)
- **Equal weights ship as-is**: `weight_per_axis = 0.125` (1/8) across all 8 axes. Heuristic axes (image_size_efficiency + security_posture) contribute 2/8 = 25% combined. Per milestone v1.1 spec.
- Test `score::tests::heuristic_axes_cannot_promote_worst_measured_cell_to_top_1` defends the worst-case: a cell whose 6 measured axes all rank near-bottom must NOT be promoted to #1 by perfect heuristic scores. Construct a synthetic 18-cell fixture where one cell has measured-bottom + heuristic-100; assert it does not appear at rank 1.
- V12-07 (cap heuristic at 12.5% aggregate) is explicitly deferred. Do NOT add a `weight_hint` field to anything; do NOT introduce per-axis weight overrides.

### Winsorization & Spread Discipline (SCORE-02)
- **p10/p90 winsorization** before min-max normalization. At N=18, `floor(0.10 × 18) = 1` clips one cell per tail; `floor(0.05 × 18) = 0` would collapse to raw min/max — that's why p5/p95 was rejected.
- **Dry-run gate during planning**: Plan must include a one-shot dry-run task that runs the chosen normalization on the v1.0 committed multi-run fixture (`tests/fixtures/multi-run-results/*.json`) and prints per-axis distinct-score counts. If any axis produces < 5 distinct scores at N=18, plan adds a `TODO(V12-04)` comment in `score.rs::normalize_axis` flagging fixed-clamp axis-specific fallback as a v1.2 follow-up — but **still ships p10/p90 in code for v1.1**. No fixed-clamp ranges in v1.1 (insufficient empirical data for thresholds).
- Single normalization mechanism in code; the TODO is a documentation marker, not a code path.

### Per-cell Prose Source (REC-01)
- `tldr / strengths / weaknesses / recommended_for / avoid_for` are **all axis-derived**. No hand-curated string tables, no per-(alloc, env) lookup, no reuse of `recommend.rs::pick_rationale_scenario` strings (those are tuned for class-winners, not cell ranks).
- Derivation rules (lock these in `recommend.rs` helpers):
  - `strengths: Vec<&'static str>` — top-2 axes by normalized score, alphabetical tiebreak on `axis_key`. Stored as `MEASUREMENT_AXES[i].label` (NOT `key`) so prose is human-facing.
  - `weaknesses: Vec<&'static str>` — bottom-2 axes by normalized score, alphabetical tiebreak.
  - `recommended_for: Vec<WorkloadClass>` — every `WorkloadClass` where this `(alloc, env)` cell wins under the existing `recommend.rs::recommendations()` logic. Re-uses winner detection; does NOT re-use the rationale strings.
  - `avoid_for: Vec<WorkloadClass>` — every `WorkloadClass` where this `(alloc, env)` cell finishes in the bottom 2 across the 18-cell ranking for that class.
  - `tldr: String` — single-sentence templated assembly: `format!("{alloc}/{env} — strong on {top_strength_label}, weak on {bottom_weakness_label}.")`. Pure formatting; no narrative.
- All cell-prose helpers in `recommend.rs`. `score.rs` knows nothing about prose.

### Suspect-Flag Propagation (CELL-04 prerequisite, REC-01)
- `CellRecommendation.suspect_flag: bool` — true if ANY axis for this cell has `samples_count < 1000` OR `warmup_duration_s < 5.0` (mirrors the v1.0 / Phase 5 convention from CLAUDE.md "Suspect run flagging").
- Aggregation via OR across axes (any-axis-suspect promotes the whole cell to suspect). Templates render `*(suspect)*` italic suffix per the v1.0 convention. NO per-axis breakdown in the struct (V12 deferral; Phase 9 polar.rs gets per-axis context from `MEASUREMENT_AXES.is_heuristic`, not from a suspect set).
- Helper: `recommend::cell_is_suspect(cell_runs: &[Run]) -> bool` — short-circuit OR over axes.

### Composite Score Determinism (SCORE-03)
- Composite score sums via `MEASUREMENT_AXES.iter()` constant traversal — **NOT** via a collected `Vec<(key, score)>`. Single-ULP drift from non-deterministic order corrupts ties; tied cells then break alphabetically by `(alloc, env)`. Test `score::tests::composite_score_summation_order_matches_axes_rs_constant_order` pins the order.
- Tiebreak: stable sort by `(composite_score DESC, alloc ASC, env ASC)`. Test `score::tests::tied_cells_break_alphabetically_for_determinism` pins this (synthetic fixture with two cells at identical composite scores).

### Top-N Constants (REC-02)
- `pub const TOP_N_SPIDER: usize = 3` — top 3 cells overlaid on Phase 9 spider chart
- `pub const TOP_N_TABLE: usize = 5` — top 5 cells in the above-the-fold REPORT.md table (Phase 8)
- `pub const TOP_N_TOTAL: usize = 10` — total cards / fragments emitted (Phase 8)
- Single source of truth; templates pull these constants, never magic numbers. Lives in `recommend.rs` (consumed by Phase 8 templates + Phase 9 polar.rs).

### Scoring Module Architecture
- `score.rs` exports: `normalize_axis()`, `compute_axes()`, `score_cells()`, `top_n()`, plus structs `CellAxes` and `CellScore`. NO prose, NO rendering, NO BTreeMap consumption beyond reading inputs (it consumes `&[Run]` + `&BTreeMap<String, CellMeta>` + `&BTreeMap<String, SecurityMeta>` from Phase 6).
- `recommend.rs` adds: `CellRecommendation` struct (all fields locked in REQUIREMENTS REC-01), `top_n_cells()` taking `Vec<CellScore>` + `&[Run]` and returning `Vec<CellRecommendation>` for the top `TOP_N_TOTAL` cells.
- `score.rs::Score` and `recommend.rs::Recommendation` are **separate** types; nothing in `score.rs` is exposed as `pub use` from `recommend.rs`. Decorate-not-rewrite; no churn to existing 13 `recommendations()` tests.

### Claude's Discretion
- Exact layout of `CellAxes` struct (intermediate type) — likely `BTreeMap<&'static str, f64>` keyed by axis_key, but can be a fixed-array of length 8 for stack discipline. Pick whichever is cleaner.
- Empty-input guard for `normalize_axis(&[])` — return empty Vec, no panic.
- Single-value-input edge: `normalize_axis([42.0])` → score must be deterministic (suggest 50.0 mid-range; pin via test).
- All-equal-input edge: `normalize_axis([7.0, 7.0, 7.0])` → all 50.0 mid-range (avoid div-by-zero); pin via test.
- Whether `top_n` returns `Vec<CellScore>` (data) and `top_n_cells` returns `Vec<CellRecommendation>` (prose), or whether `top_n_cells` is the only public top-N function — recommend keeping both: `score::top_n` for Phase 9 polar.rs (data only, no prose overhead).

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/alloc-bench-aggregator/src/axes.rs` — Phase 6 deliverable. Exports `MEASUREMENT_AXES: [AxisSpec; 8]`, `Direction::{Higher, Lower}`, `Direction::arrow()`. **Iterate this constant, do not collect into a Vec.** Score normalization reads `direction` and `is_heuristic` per axis.
- `crates/alloc-bench-aggregator/src/loader.rs` — Phase 6 deliverable. Exports `CellMeta` + `load_cell_metas` (image-size sidecars), `SecurityMeta` + `load_security_metas` (security sidecars). `score.rs::compute_axes()` consumes both `BTreeMap`s.
- `crates/alloc-bench-aggregator/src/recommend.rs` — existing `WorkloadClass` enum (6 classes: web, channel, multithread, cpu, memory, resilience), `Recommendation` struct, `recommendations(runs: &[Run]) -> Vec<Recommendation>`, `pick_rationale_scenario` helper. **`top_n_cells()` reuses `recommendations()` to derive `recommended_for`** — no rewrite, no churn to the 13 existing tests.
- `crates/alloc-bench-core/src/output.rs` — locked v1 schema. Phase 6 GUARD-01 gate (SHA-256 frozen) prevents mutation. Read fields: `Run { alloc, env, scenario, throughput, p50/p90/p99/p999, samples_count, warmup_duration_s, ... }`.
- `crates/alloc-bench-aggregator/src/multi_run.rs` — multi-run aggregation (median + Bessel sample stddev + CV). `compute_axes` likely consumes the median values per `(alloc, env, scenario)` triple, not raw individual runs.

### Established Patterns
- Decorate-not-rewrite: `output.rs` v1 schema is locked. New fields ride on sidecars OR are computed by the aggregator from existing v1 fields. SCORE-* and REC-* additions are aggregator-side computations.
- Byte-identical output discipline: alphabetical iteration via `BTreeMap` / `BTreeSet` only. `MEASUREMENT_AXES` is alphabetical by `key` (Phase 6 invariant). Tiebreaks alphabetical.
- Numeric formatting: `{:.1}` for throughputs in single-run cells, `{:.0}` for medians in multi-run cells, `{}` for ns latencies. Composite scores will follow `{:.1}` (one decimal of precision is enough for top-N ordering).
- Suspect-run flagging: `samples_count < 1000` OR `warmup_duration_s < 5.0` triggers `⚠ suspect` (read-time decoration, not a render filter; row still emitted).
- Multi-run statistics: Bessel sample stddev (n-1 denominator); CV > 10% flags `⚠ high variance`; CV undefined when `|mean| ≤ 1e-9`.

### Integration Points
- `score.rs` consumes Phase 6's `MEASUREMENT_AXES` constant + `Direction` enum + `SecurityMeta` BTreeMap (already on main).
- Phase 8 templates consume `CellRecommendation` struct from `recommend.rs::top_n_cells()` (this phase). Single struct → two outputs (md.tmpl + html.tmpl); WR-01-pattern test `html::tests::cell_templates_both_reference_all_fields` defends drift.
- Phase 9 polar.rs consumes `score::top_n()` (data only) for top-3 spider overlay; `MEASUREMENT_AXES.is_heuristic` drives dashed gridline.
- Phase 10 markdown.rs consumes `Direction::arrow()` for column-header glyphs (read-time decoration).

</code_context>

<canonical_refs>
## Canonical References

| Path | Why this is canonical |
|------|----------------------|
| `.planning/REQUIREMENTS.md` (lines 19-27) | SCORE-01..04 + REC-01..02 locked requirements |
| `.planning/ROADMAP.md` (Phase 7 entry) | Phase goal, dependencies, success criteria, open questions |
| `.planning/PROJECT.md` | Project-level decisions (decorate-not-rewrite, BTreeMap discipline, equal-weights spec) |
| `.planning/phases/06-foundations/06-CONTEXT.md` | Phase 6 locked decisions consumed here (`MEASUREMENT_AXES` const, `Direction` enum, `SecurityMeta`) |
| `.planning/phases/06-foundations/06-01-SUMMARY.md` | What `axes.rs` actually shipped (8 keys + Direction + arrow) |
| `.planning/phases/06-foundations/06-02-SUMMARY.md` | What `loader.rs::load_security_metas` ships (BTreeMap, empty-pattern guard, skip-and-continue) |
| `crates/alloc-bench-aggregator/src/axes.rs` | Source of truth for axis order, direction, is_heuristic |
| `crates/alloc-bench-aggregator/src/recommend.rs` | Existing `WorkloadClass` enum + `recommendations()` + 13 tests (untouched) |
| `crates/alloc-bench-core/src/output.rs` | Locked v1 input schema (frozen by Phase 6 GUARD-01) |
| `./CLAUDE.md` (Conventions section) | Multi-run statistics, suspect-run flag thresholds, byte-identical-output discipline |

</canonical_refs>

<specifics>
## Specific Ideas

- The dry-run for p10/p90 informativeness should run during planning (gsd-phase-researcher task), not at code time. The output goes into RESEARCH.md as a small table: `| axis_key | distinct_scores_at_n18 | spread_qualitative |`. If any axis is < 5 distinct scores, the planner emits the V12-04 TODO line in the `score.rs::normalize_axis` doc comment.
- All five `CellRecommendation` prose fields are pure functions of the axis-score breakdown + the existing `recommendations()` output. They must be reproducible from a `(Vec<CellScore>, &[Run])` input pair — so unit tests can pass synthetic inputs and assert exact prose strings.
- `top_n_cells()` returns `Vec<CellRecommendation>` of length `min(TOP_N_TOTAL, scored_cells.len())`. With 18 cells, length is always 10; the `min()` is defensive for fixtures.
- Phase 9 needs `CellScore` (data-only) for the spider; do not force Phase 9 to consume `CellRecommendation` (prose overhead). Public API: `score::top_n(scores, TOP_N_SPIDER) -> Vec<CellScore>` is the Phase 9 hook.

</specifics>

<deferred>
## Deferred Ideas

- Heuristic-axis weight cap (≤12.5% aggregate) — **V12-07** (v1.2)
- Workload-shape weighted scoring profiles — **V12-05** (v1.2)
- Confidence intervals on composite scores — **V12-06** (v1.2)
- JSON-driven re-weighting slider — **V12-01** (v1.2)
- Cross-version diff radar — **V12-02** (v1.2)
- Per-axis fixed-clamp normalization fallback — **V12-04 candidate** (v1.2 IF dry-run shows compression at p10/p90)
- Per-axis suspect breakdown (`suspect_axes: BTreeSet<&str>` on `CellRecommendation`) — deferred; v1.1 ships cell-level bool only
- Hand-curated per-cell prose lookup tables — explicitly rejected by REQUIREMENTS REC-01 ("data-derived (no hand-edited prose strings)")

</deferred>

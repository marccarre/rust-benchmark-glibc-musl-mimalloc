# Pitfalls Research — v1.1 Recommendations, Spider Charts & Direction Markers

**Domain:** Aggregator pipeline extension — normalization, radar charts, per-cell prose, direction markers, security sidecars
**Researched:** 2026-05-26
**Confidence:** HIGH (all findings grounded in in-tree code read end-to-end; every component reference is a named existing file or a named proposed file from ARCHITECTURE.md)

---

## Critical Pitfalls

### Pitfall 1: Direction-Unaware Normalization in `score.rs::normalize_axis`

**Severity:** HIGH

**What goes wrong:**
`score.rs::normalize_axis` receives raw values from heterogeneous sources — throughput (`ticks_per_s`, higher = better), latency/RSS (`peak_rss_kb`, lower = better), and image size (`meta.image_size_mb`, lower = better). If the direction parameter is ignored or applied inconsistently, high latency cells score as 100 and low latency cells score as 0, silently inverting the entire recommendation ranking. The spider chart will *look* plausible (all polygons, all values in 0–100) so visual inspection will not catch the error. The only symptom is the overall composite scores in `score.rs::score_cells` placing ptmalloc (highest RSS, highest latency) above jemalloc on every memory-sensitive axis.

**Why it happens:**
The call site in `score.rs::compute_axes` iterates over `MEASUREMENT_AXES` from `axes.rs` to extract per-axis raw values for all 18 cells, then calls `normalize_axis` once per axis. It is easy to pass the extracted slice without consulting `axis.direction`, especially when copy-pasting the throughput-axis call as the template for the lower-is-better axes. The resulting bug is category-preserving (scores remain in 0–100) and will pass any schema-level test.

**How to avoid:**
In `score.rs::normalize_axis`, make direction a non-optional parameter, not a boolean flag:

```rust
pub fn normalize_axis(values: &[f64], dir: axes::Direction) -> Vec<f64>
```

The call site in `compute_axes` must destructure each `AxisSpec` and pass `spec.direction` explicitly — making it impossible to forget. Add a golden-value unit test in `score.rs::tests`:

```
// test: lower_is_better_axis_inverts_correctly
// Input: [100.0, 200.0, 300.0], Direction::Lower
// Expected: [100.0, 50.0, 0.0]  (300 → worst → 0; 100 → best → 100)
```

This test name becomes the named guard for PR review.

**Warning sign:**
Any cell that is the worst performer on throughput (e.g., musl-mallocng on the web scenario) scoring above 80 on the `web` axis; or mimalloc (lowest RSS) scoring below 20 on the `memory` axis. Both are detectable in `just aggregate` output before the golden fixture is regenerated.

**Phase to address:** Phase B (scoring + per-cell prose).

---

### Pitfall 2: NaN/Infinity Propagation from `multi_run::aggregate` into `score.rs`

**Severity:** HIGH

**What goes wrong:**
`multi_run::aggregate` returns `None` when any sample is non-finite (`rejects_nan_input` test in `multi_run.rs:164`). The call sites in `recommend.rs` and `markdown.rs` already handle `None` correctly (em-dash fallback). But `score.rs::compute_axes` extracts axis values from `Run` records *before* grouping by multi-run statistics, using fields like `run.metrics.ticks_per_s` directly. If a run produced a `NaN` (e.g., a crashed scenario whose JSON was written with `null` coerced to `0.0/0.0`), that `NaN` enters `normalize_axis`'s `values: &[f64]` slice. `f64::min` and `f64::max` fold operations with `NaN` produce `NaN` (NaN-poisoning), so `span = max - min = NaN`, and every cell on that axis scores `NaN`. `NaN` flows into `CellRecommendation::composite_score`, which `BTreeMap`-sorts the cells by score to build the top-10 — and `f64::partial_cmp` returns `None` for NaN comparisons, which will panic at the `unwrap()` in `sorted.sort_by(|a,b| a.partial_cmp(b).unwrap())` if the sort uses that pattern.

**Why it happens:**
`multi_run.rs::aggregate` is clean (NaN-rejecting at the statistics layer). But `score.rs` operates at the raw `Run` layer, one level below `multi_run`. The NaN guard exists in the wrong module from `score.rs`'s perspective.

**How to avoid:**
In `score.rs::compute_axes`, add an `is_finite()` guard before inserting each raw metric value into the per-axis accumulator. This mirrors the existing pattern in `multi_run.rs:67`:

```rust
if !v.is_finite() { continue; }  // skip non-finite runs — same guard as multi_run::aggregate
```

Add a named test: `score::tests::nan_run_is_skipped_not_propagated`. The sort in `score_cells` should use `a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)` — the same tie-breaking idiom already used in `multi_run.rs:87`.

**Warning sign:**
`just aggregate` panics with `called Option::unwrap() on a None value` inside the sort in `score.rs`, OR `index.html` renders a spider chart with one or more axes blank (Plotly renders `NaN` in `r` arrays as a gap in the polygon, not a crash — so visual inspection can miss this).

**Phase to address:** Phase B (scoring). The NaN-guard test should be gated by Phase A's fixture setup because the test requires a fixture with a non-finite metric value.

---

### Pitfall 3: p5/p95 Winsorization With N=18 Cells Collapses to Raw Min/Max

**Severity:** HIGH

**What goes wrong:**
FEATURES.md §2 specifies "5th/95th percentile winsorization before min-max." With exactly 18 cells in the canonical matrix, the 5th percentile index is `floor(0.05 × 18) = 0`, and the 95th percentile index is `floor(0.95 × 18) = 17` — which are the minimum and maximum values respectively. Winsorization at p5/p95 on 18 data points is mathematically equivalent to no winsorization, defeating the entire purpose of the technique (suppressing outliers before normalization).

**Why it happens:**
The winsorization formula is designed for large samples (Wikipedia: Winsorizing assumes "commonly applied to the lowest 5 and highest 5 percent of the values," meaning N must be at least ~40 for meaningful clipping). The FEATURES.md spec adopted the standard textbook formula without noting the small-N failure mode.

**How to avoid:**
In `score.rs::normalize_axis`, use a **fixed clamp**: skip winsorization when N ≤ 20 and fall back to pure min-max. Document this as a named decision comment. Alternatively, use a tighter percentile — p10/p90 gives `floor(0.1 × 18) = 1`, clipping one cell per tail, which is meaningful. The golden-value unit test for `normalize_axis` should assert the p10/p90 behavior explicitly:

```
// test: normalize_axis_p10_p90_clips_one_outlier_each_tail_at_n18
// Input 18 values [0, 1, 2, ... 16, 100], Direction::Higher
// With p10/p90 winsorization: outlier 100 is clipped to p90 value
// Without winsorization: outlier 100 compresses all other cells into 0–16/100
```

**Warning sign:**
All 18 cells except the best and worst scoring between 48 and 52 on any axis — which is the compression signature of a single outlier dominating the range. This is detectable by inspecting `score.rs::compute_axes` output before the radar chart renders.

**Phase to address:** Phase B (scoring). The test fixture must include a synthetic outlier cell to validate the clipping behavior.

---

### Pitfall 4: `(heuristic)` Suffix Is Not a Sufficient Guardrail for Mixed-Axis Spider Charts

**Severity:** HIGH

**What goes wrong:**
FEATURES.md §6 and ARCHITECTURE.md §Q5 propose appending `(heuristic)` to the `image-size` and `security` axis labels in the `polar.rs` spider trace `theta` array and in `MEASUREMENT_AXES` in `axes.rs`. This is a **label** guardrail, not a **formula** guardrail. A reader who sees `Security posture (heuristic): 87/100` alongside `Web service throughput: 87/100` will visually equate them because they are rendered at identical scale, identical radial distance, identical fill opacity. The label is easily missed on a small screen or in a screenshot where the full axis label is truncated by Plotly's angular axis label truncation.

Additionally, with equal weights (1/8 per axis), the two heuristic axes together contribute 25/100 points (25%) of the composite score in `score.rs::score_cells`. A cell can rise from rank 4 to rank 1 by having high hand-curated security scores alone, even if it is the worst performer on all six measured axes. This is not a bug — equal weights are the milestone spec — but it is a credibility pitfall that will not be caught by any existing test.

**Why it happens:**
MCDA literature endorses mixing objective and subjective criteria (FEATURES.md §6) without requiring visual distinction beyond a label. The issue is that the v1.1 dashboard will be the *first* place readers see this combination, and there is no existing convention in the codebase for "this value is curated, not measured."

**How to avoid:**

1. In `polar.rs`, set `line.dash = 'dot'` for heuristic-axis gridlines. Plotly's `polar.radialaxis` does not support per-axis gridline styles directly, but the `angular` axis tick labels can be styled individually via `polar.angularaxis.tickfont` overrides for specific tick indices. Alternatively, append `*` to heuristic axis labels and add a legend footnote.

2. In `recommend-cell.md.tmpl` and `recommend-cell.html.tmpl`, emit a callout block whenever the heuristic axes contribute more than 20 points to the composite score gap between rank N and rank N+1:

   ```
   Note: {heuristic_delta:.0} of this cell's {score:.0}/100 composite score
   derives from hand-curated image-size and security heuristics.
   See meta/security/{env}.json for the scoring rationale.
   ```

3. Add a named test `score::tests::heuristic_axes_cannot_promote_worst_measured_cell_to_top_1` using a synthetic fixture where one cell scores 0 on all six measured axes but 100 on both heuristic axes — verify that such a cell does not appear in `top_n(1)`. This test is intentionally designed to fail under equal weights, triggering a discussion about whether equal weights are the correct policy before the feature ships.

**Warning sign:**
The overall top-1 cell is one that performs poorly on throughput scenarios but has a perfect security score. Detectable by comparing `score.rs::score_cells` output with the v1.0 `recommend.rs::recommendations` top picks — if they disagree strongly, heuristic axes are likely the cause.

**Phase to address:** Phase B (scoring decision) and Phase D (spider chart visual distinction).

---

### Pitfall 5: `recommend.rs::top_n_cells` Tie-Breaking Is Non-Deterministic Without an Explicit Tiebreaker

**Severity:** HIGH

**What goes wrong:**
`score.rs::score_cells` produces a `Vec<CellScore>` sorted descending by `composite_score: f64`. When two cells have identical composite scores (common when both heuristic axes score the same value for the same env, e.g., all jemalloc cells in the same Docker image share the same `security_meta.composite_security_score`), the sort is unstable — the relative order of tied cells depends on the order they appeared in the input `BTreeMap` iteration, which is deterministic, but `sort_by` in Rust is not guaranteed to preserve insertion order for equal elements (it uses pattern-defeating quicksort, which is not stable). Two identical composite scores from jemalloc·alpine and mimalloc·alpine could swap ranks between runs if the floating-point sum is computed in different orders.

**Why it happens:**
The CLAUDE.md byte-identical-output discipline mandates `BTreeMap` iteration for *collection traversal*, but the sort step in `score_cells` is a secondary sort that can silently produce non-deterministic output for tied scores. The existing `multi_run.rs::aggregate` uses `.sort_by(|a,b| a.partial_cmp(b).unwrap_or(Equal))` which is also not stable, but its output (median, min, max) is commutative with respect to input order, so the non-stability doesn't matter there.

**How to avoid:**
In `score.rs::score_cells`, use `.sort_by(|a, b| b.composite_score.partial_cmp(&a.composite_score).unwrap_or(Equal).then_with(|| a.alloc.cmp(&b.alloc)).then_with(|| a.env.cmp(&b.env)))`. The `then_with` alphabetical tie-break is the same strategy already used in v1.0's `recommend.rs::winner_picker_alphabetical_class_order` test. Name the test:

```
// test: score::tests::tied_cells_break_alphabetically_for_determinism
// Two cells with identical composite_score → lower (alloc,env) alphabetically wins
```

**Warning sign:**
Golden fixture in `tests/smoke.rs` fails intermittently (not consistently). If the test passes on one run and fails on the next with the same inputs, the sort is non-deterministic. In practice this surfaces first during Phase F golden-fixture regeneration.

**Phase to address:** Phase B (scoring), before Phase F fixture regeneration.

---

## Normalization-Specific Pitfalls

### Pitfall 6: Cross-Axis Normalization (Mixing Units Before Per-Axis Normalization)

**Severity:** HIGH

**What goes wrong:**
`score.rs::compute_axes` builds a `BTreeMap<(alloc, env), AxisScores>` where each `AxisScores` has 8 values (one per axis, in raw units: ops/s, ns, KB, MB, score-0-100). If a developer accidentally normalizes across axes (e.g., normalizes all 8 values for a single cell together, treating the 8 values as one vector) instead of normalizing per axis across all 18 cells, the resulting scores are meaningless — a cell with throughput 1_000_000 ops/s will dominate a cell with RSS 200 KB simply because the throughput value is numerically larger.

**Why it happens:**
The STACK.md §Q2 hand-rolled normalization recipe shows `values: &[f64]` as the input — the developer must ensure they pass the per-axis slice (all 18 cells' values for one axis) and not the per-cell slice (all 8 axis values for one cell). The difference is a transposition of the data access pattern, and without a type-level distinction the compiler cannot catch it.

**How to avoid:**
In `score.rs`, enforce the call shape by declaring typed newtypes for the two access patterns:

```rust
// CORRECT: per-axis vector (length = N cells)
struct AxisValues(Vec<f64>);  // passed to normalize_axis

// WRONG shape (length = N axes) — would be caught at the type boundary
struct CellValues(Vec<f64>);  // used only internally; never passed to normalize_axis
```

Alternatively, name the test assertion to catch the mistake:

```
// test: score::tests::normalize_axis_called_per_axis_not_per_cell
// Builds a 3-cell × 2-axis matrix with distinct row and column ranges
// Verifies the normalized output for axis-0 is independent of axis-1 values
```

**Warning sign:**
All cells score near 100 on throughput axes and near 0 on latency/RSS axes — cross-axis normalization makes throughput-heavy allocators dominate because raw throughput values are orders of magnitude larger than raw latency values.

**Phase to address:** Phase B (scoring).

---

## Byte-Identical Output Pitfalls

### Pitfall 7a: HashMap Iteration in `load_security_metas`

**Severity:** HIGH

**What goes wrong:**
STACK.md §Q5 shows the proposed `load_security_metas` returning `HashMap<String, SecurityMeta>`. If the aggregator iterates this `HashMap` to build the security axis values array (e.g., to collect all 6 security scores before normalizing), the iteration order is non-deterministic across Rust versions and platforms. The security axis values fed into `score.rs::normalize_axis` will be in a different order on each run, and while the normalized scores are commutative with respect to input order (min-max normalization does not depend on iteration order), the downstream `score_cells` sort will encounter values that were assembled from a non-deterministic traversal — which risks subtle float divergence from summation order.

**Why it happens:**
The STACK.md code sample uses `HashMap` because the lookup by `env: String` key does not need order. But there is a downstream step where the collected values are iterated to build the `AxisValues` slice for `normalize_axis` — this step requires a defined order.

**How to avoid:**
Either:
(a) Return `BTreeMap<String, SecurityMeta>` from `load_security_metas` — the same discipline applied to all other aggregator collections. This is the simpler fix and aligns with CLAUDE.md §Conventions ("alphabetical iteration via `BTreeMap`/`BTreeSet`").
(b) Or sort the collected values before passing to `normalize_axis`: `values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Equal))` — but this destroys the cell-to-value correspondence needed for the per-cell score assignment.

Use (a). Name the test: `loader::tests::load_security_metas_returns_btreemap_sorted_by_env`.

**Warning sign:**
`cargo test` byte-identical output tests pass locally but fail on a different OS or different Rust toolchain version — the classic HashMap-iteration non-determinism symptom.

**Phase to address:** Phase A (foundations — `loader.rs` security sidecar).

---

### Pitfall 7b: Floating-Point Summation Order in `score.rs::score_cells` Composite Score

**Severity:** HIGH

**What goes wrong:**
`score.rs::score_cells` computes `composite_score = mean(8 axis scores)` as `axes.values().sum::<f64>() / 8.0`. If `axes` is a `BTreeMap<&'static str, f64>`, iteration order is alphabetical and deterministic — but if axes are collected into a `Vec<f64>` before summing (a common micro-optimization to avoid repeated BTreeMap lookups), the order of summation depends on the insertion order into the Vec, which depends on the order of the `compute_axes` loop. With 8 f64 values in the 0–100 range, summation order affects the result by at most a few ULPs — but any ULP difference in `composite_score` changes the sort order for tied cells, which changes the golden fixture.

**Why it happens:**
The developer reaches for `Vec<f64>` for performance, not realizing that the summation result must be exactly reproducible. The `multi_run.rs::aggregate` sum (`samples.iter().sum::<f64>()`) is deterministic because `iter()` over a `Vec` always iterates in insertion order — but `BTreeMap::values().sum()` iterates alphabetically, and `HashMap::values().sum()` iterates non-deterministically.

**How to avoid:**
In `score.rs::score_cells`, sum axes by explicit alphabetical key traversal:

```rust
let composite_score = MEASUREMENT_AXES.iter()
    .map(|spec| cell_axes.get(spec.key).copied().unwrap_or(50.0))
    .sum::<f64>() / MEASUREMENT_AXES.len() as f64;
```

`MEASUREMENT_AXES` is a `const` array in `axes.rs` with a fixed alphabetical key order — this makes the summation order a compile-time constant. Name the test: `score::tests::composite_score_summation_order_matches_axes_rs_constant_order`.

**Warning sign:**
Golden fixture in `tests/smoke.rs` fails with a diff of a single ULP in a composite score value. The diff is stable (always the same two values) but platform-dependent (different on Linux vs macOS).

**Phase to address:** Phase B (scoring).

---

### Pitfall 7c: NaN or Infinity in Composite Score Corrupts Sort

**Severity:** HIGH

**What goes wrong:**
Pitfall 2 describes NaN entering `normalize_axis`. If a NaN escapes the normalize step and enters `composite_score`, then `score_cells`' sort (`a.partial_cmp(&b)`) returns `None` for any comparison involving the NaN cell. Using `.unwrap_or(Equal)` makes the NaN cell sort as equal to everything, placing it at a random position. Using `.unwrap()` panics. Either way the top-10 ranking is corrupted.

**Why it happens:**
The `is_finite()` guard in `compute_axes` may be present for the raw metric extraction but absent for the composite computation step, especially if `normalize_axis` returns `50.0` for degenerate inputs (span == 0) but the caller did not handle the case where all inputs were filtered out (empty slice passed to `normalize_axis`, which returns an empty `Vec`, and mean of an empty `Vec` is `NaN` via division by zero).

**How to avoid:**
In `score.rs::normalize_axis`, explicitly guard the empty-input case:

```rust
if values.is_empty() { return vec![]; }
```

In `score_cells`, after computing `composite_score`, add:

```rust
if !composite_score.is_finite() { continue; }  // skip cells with degenerate axis data
```

Name the test: `score::tests::cell_with_all_nan_runs_is_excluded_from_top_n`.

**Warning sign:**
`score::score_cells` returns fewer than 18 cells in the ranked output — which is expected for cells with no data, but should be logged at `warn:` level so the developer notices the exclusion during development.

**Phase to address:** Phase B.

---

### Pitfall 7d: Tie-Breaking Non-Determinism in `top_n_cells` Already Covered in Pitfall 5

(See Pitfall 5 — alphabetical `(alloc, env)` secondary sort is the prevention.)

---

### Pitfall 7e: Timestamp Leakage from `SecurityMeta::captured_at` into the Deterministic Output Surface

**Severity:** MEDIUM

**What goes wrong:**
`SecurityMeta` (STACK.md §Q5, ARCHITECTURE.md §Q4) includes a `captured_at: String` field (RFC-3339 timestamp). If `polar.rs::build_traces` or `markdown.rs::emit_per_cell_recommendations` embeds `captured_at` in any emitted string — e.g., in a spider chart hover tooltip (`text: "Security score captured 2026-05-26T00:00:00Z"`) or in `recommend-cell.md.tmpl` — and the template evaluator substitutes the field directly, the output becomes non-deterministic when a developer re-curates the security sidecars and regenerates without bumping the golden fixture. This breaks the "byte-identical output is the default invariant, not an exception" principle.

**Why it happens:**
`CellMeta` has the same `captured_at` field (ARCHITECTURE.md §Q4) and v1.0 already emits it in the Docker runtimes table. The v1.0 approach is correct: `captured_at` is displayed as metadata, not as part of the byte-identical output surface. But `security_meta.captured_at` is in a *new* emitter (`recommend-cell.md.tmpl` / `recommend-cell.html.tmpl`) written by a developer who may not realize the v1.0 convention.

**How to avoid:**
Annotate the `captured_at` field with `#[allow(dead_code)]` in the Rust struct (as STACK.md already proposes) AND add a note to both templates:

```
<!-- NOTICE: Do NOT reference { security_meta.captured_at } in this template.
     captured_at is metadata only and is not part of the byte-identical output surface.
     Displaying it would require golden-fixture regen on every sidecar refresh. -->
```

Name the CI gate: add a `grep` step in `tests/smoke.rs` or a compile-time test that asserts `captured_at` does not appear in any rendered output string.

**Warning sign:**
Golden fixture diff in `tests/smoke.rs` contains a date string that matches the format `\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z`. Any such string in a REPORT.md or index.html snapshot (beyond the existing single timestamp comment at line 1 of REPORT.md) is a leakage.

**Phase to address:** Phase C (per-cell artifact templates) + Phase A (sidecar struct definition).

---

## Spider Chart Anti-Patterns

### Pitfall 8: Plotly `scatterpolar` Polygon Not Closed — Missing Final Point

**Severity:** MEDIUM

**What goes wrong:**
STACK.md §Q1 documents the canonical Plotly scatterpolar idiom: the `r` array must repeat `r[0]` as `r[8]`, and `theta` must repeat `theta[0]` as `theta[8]`, to close the polygon. If `polar.rs::build_traces` uses an 8-element `r` and `theta` array (one entry per axis) without appending the closing element, Plotly renders an open polygon — the fill area leaves a wedge open between the last axis and the first axis. This is visually subtle on a full 8-axis chart (the gap is a narrow wedge at 12 o'clock) and is easily overlooked in visual testing.

**Why it happens:**
STACK.md shows the correct 9-element pattern in the verified Plotly trace shape. But a developer implementing `polar.rs` from scratch will typically produce an 8-element `r`/`theta` pair (one-per-axis is the natural loop), and both 8- and 9-element inputs are accepted by Plotly without error — the difference is only visual.

**How to avoid:**
In `polar.rs::build_trace`, after building the 8-element `r_values` and `theta_labels` vecs, explicitly append the closing elements:

```rust
r_values.push(r_values[0]);       // close polygon
theta_labels.push(theta_labels[0]); // close polygon
```

Name the unit test: `polar::tests::trace_r_and_theta_are_closed_polygon` — assert `r.len() == 9 && r[0] == r[8] && theta[0] == theta[8]`.

**Warning sign:**
Plotly renders a wedge-shaped gap at the first axis position in every spider chart. Not caught by any non-visual test; requires a manual screenshot comparison.

**Phase to address:** Phase D (spider chart).

---

### Pitfall 9: Plotly CDN Upgrade That Silently Changes `scatterpolar` Trace Shape

**Severity:** MEDIUM

**What goes wrong:**
The v1.0 Plotly CDN pin is `plotly-2.35.3.min.js` with SRI hash `sha384-MqL7Cy3it…lhPykM` (STACK.md §Q1; `html.rs:37-45`). STACK.md confirms `scatterpolar` exists in this bundle. If a developer "upgrades" Plotly by updating the CDN URL and SRI hash in `html.rs` without verifying that the `scatterpolar` trace configuration (specifically: `fill: 'toself'`, `polar.radialaxis.range`, `polar.angularaxis.direction: 'clockwise'`, `polar.angularaxis.rotation: 90`) remains unchanged in the new version, the radar charts may render incorrectly. Plotly has historically made breaking changes in polar-trace defaults between minor versions (e.g., `rotation` default changed between v2.x releases).

**Why it happens:**
The Plotly CDN URL looks like a one-line change. The SRI hash being different makes it obvious that the JS file changed, but a developer who does not understand what the hash protects may update the hash without auditing the trace API.

**How to avoid:**
Add a named CI gate: `test_plotly_sri_hash_unchanged` — a `cargo test` integration test that reads `html.rs` and asserts the CDN URL and SRI hash match the v1.0-pinned values exactly:

```rust
// tests/smoke.rs: assert CDN line unchanged
const EXPECTED_PLOTLY_SRI: &str = "sha384-MqL7Cy3it…lhPykM";
assert!(html_source.contains(EXPECTED_PLOTLY_SRI), "Plotly CDN hash changed — verify scatterpolar trace shape before updating");
```

Any Plotly upgrade must:
1. Update the hash in `html.rs`.
2. Update the `EXPECTED_PLOTLY_SRI` constant in `smoke.rs`.
3. Add a PR checklist item: "Verified `scatterpolar` with `fill: 'toself'` and `polar.angularaxis.direction: 'clockwise'` in the new bundle via the Plotly changelog."

**Warning sign:**
Plotly bundle CDN URL appears in a PR diff alongside the SRI hash with no PR checklist item verifying polar trace API compatibility.

**Phase to address:** Phase D (spider chart) — the test should be added when `polar.rs` is first wired into `html.rs`.

---

## Recommendation Prose Pitfalls

### Pitfall 10: Top-N Hierarchy Failure — Top-3 Display vs Top-10 Generation Set

**Severity:** MEDIUM

**What goes wrong:**
FEATURES.md §5 and ARCHITECTURE.md §Q3 establish a three-tier display: top-3 spider charts above the fold, top-5 in the recommendations table, top-10 in a collapsed `<details>` section. `recommend.rs::top_n_cells(runs, metas, security_metas, n=10)` generates 10 `CellRecommendation` records. `markdown.rs::emit_per_cell_recommendations` and `html.rs::render_per_cell_panels` each receive the same `Vec<CellRecommendation>` and must independently filter/slice to their tier depth (top-3 for charts, top-5 for the table, top-10 for the collapsed section). If either emitter slices by `[..3]`, `[..5]`, or `[..10]` with a hardcoded literal, the tier counts will drift when someone decides to change one without updating the other.

**Why it happens:**
Two separate emitters (`markdown.rs` and `html.rs`) consuming the same struct with different slice depths — the ARCHITECTURE.md §Q3 note on WR-01 drift applies here: "v1.0 currently has the opposite problem: `recommend::Recommendation` powers REPORT.md but the HTML's report-mirror table is rebuilt independently in JavaScript, causing drift risk."

**How to avoid:**
Define named constants in `recommend.rs`:

```rust
pub const TOP_N_SPIDER: usize = 3;   // charts above the fold
pub const TOP_N_TABLE: usize = 5;    // recommendations table rows
pub const TOP_N_TOTAL: usize = 10;   // generation set / collapsed section
```

Both `markdown.rs` and `html.rs` import and use these constants. A PR checklist item: "Changing any `TOP_N_*` constant requires updating the PR description and bumping the golden fixture."

**Warning sign:**
REPORT.md shows 3 prose cards while `index.html` shows 5 radar charts, or vice versa. Also: golden fixture diff after a tier-count change touches only one file, not both — which is the signal that constants were duplicated rather than shared.

**Phase to address:** Phase C (per-cell artifact templates).

---

### Pitfall 11: Prose Staleness — Hard-Coded Sentences in Per-Cell Templates

**Severity:** HIGH

**What goes wrong:**
`recommend-cell.md.tmpl` and `recommend-cell.html.tmpl` use tinytemplate substitutions to produce data-derived prose. If a developer edits the templates to include a sentence that looks like a substitution but is actually hard-coded copy — e.g., `{alloc} is the best allocator for production use.` where the sentence content is fixed regardless of the actual score — then re-running `just aggregate` with different benchmark data will produce the same sentence even when it is no longer true. This violates the locked v1.0 convention: "all prose is template-substitution from `Run` data; no hard-coded copy."

**Why it happens:**
tinytemplate does not distinguish between `{ alloc }` (data substitution) and literal text. A developer writing the template manually can accidentally introduce marketing copy that reads as if it were data-derived.

**How to avoid:**
Every sentence in both templates must be traceable to a named field in `CellRecommendation`. Add a named compile-time test that renders both templates against a synthetic `CellRecommendation` with sentinel values (e.g., `alloc: "ALLOC_SENTINEL"`, `composite_score: -999.0`) and asserts that every occurrence of allocator-specific language in the output contains the sentinel string. This catches any sentence that embeds allocator names as literals rather than as template substitutions.

Name the test: `html::tests::template_has_no_hardcoded_allocator_prose`.

**Warning sign:**
`report/recommend-01-*.md` contains the same sentence verbatim as `report/recommend-02-*.md` for a field that should differ (e.g., the TL;DR sentence). This is detectable by diffing two generated per-cell files.

**Phase to address:** Phase C (per-cell artifact templates).

---

### Pitfall 12: Per-Cell Template Struct-Field Sync Failure (Markdown + HTML Emitted from Same Struct)

**Severity:** MEDIUM

**What goes wrong:**
`CellRecommendation` (ARCHITECTURE.md §Q3) is the single struct feeding both `recommend-cell.md.tmpl` and `recommend-cell.html.tmpl`. When `CellRecommendation` adds a new field (e.g., `pareto_front: bool` in v1.2), a developer may update only one template. tinytemplate does not fail on unused struct fields — it silently ignores them. So the Markdown template gains the new column but the HTML template does not, producing inconsistent output without any compile-time or test-time error.

**Why it happens:**
tinytemplate's rendering engine does not enforce exhaustive template coverage of struct fields. It renders what is referenced and ignores the rest. The `recommend-cell.md.tmpl` and `recommend-cell.html.tmpl` are two separate files with no cross-reference.

**How to avoid:**
Add a compile-time test that renders both templates and checks that a sentinel field value (a unique string injected into the synthetic `CellRecommendation`) appears in both outputs:

```rust
// test: html::tests::both_cell_templates_reference_composite_score_field
// Render both templates with composite_score = 99999.0
// Assert both rendered outputs contain "99999"
```

This must be updated whenever a new field is added to `CellRecommendation`. Name the test: `html::tests::cell_templates_both_reference_all_fields`.

Additionally, add a PR checklist item: "If `CellRecommendation` gains a new field, both `recommend-cell.md.tmpl` and `recommend-cell.html.tmpl` must be updated in the same commit. Verified by running `cargo test html::tests::cell_templates_both_reference_all_fields`."

**Warning sign:**
The Markdown per-cell card and the HTML per-cell panel show different information for the same cell (e.g., the HTML panel shows a "Pareto-optimal" badge but the Markdown card does not). Detectable by diffing the Markdown and HTML output for the same cell rank.

**Phase to address:** Phase C (per-cell artifact templates). The test should be written when the first version of both templates is committed.

---

## Direction-Marker Pitfalls

### Pitfall 13: Direction Marker as a Column-Sort Indicator (Collision With v1.0 "Winner Bold" Convention)

**Severity:** MEDIUM

**What goes wrong:**
FEATURES.md §4 and ARCHITECTURE.md §Q5 specify placing `↑` and `↓` in REPORT.md column headers to denote "higher is better" and "lower is better." In common web and analytics dashboards, `↑` and `↓` in column headers denote *sort direction* (sort ascending/descending). A reader familiar with Tableau, Excel, or database UIs will interpret `Throughput ↑` as "this column is sorted ascending" rather than "higher throughput is better." This collision is particularly confusing because REPORT.md already uses **bold text** for the winner cell — the reader now sees bold cells (winner), `↑`/`↓` arrows (direction of good), and possibly also the suspect `⚠` marker, creating three overlapping visual annotations on the same table.

**Why it happens:**
The FEATURES.md research correctly notes `↑`/`↓` are the dominant comparison-table convention (Wikipedia). But the sort-indicator collision is not addressed in either FEATURES.md or STACK.md.

**How to avoid:**
In `axes.rs::AxisSpec::label`, append a parenthetical after the arrow to eliminate the ambiguity: `Throughput ↑ (higher=better)` is unambiguous; `Throughput ↑` alone is ambiguous. Shorten to `↑ good` if space is tight. The 1-line legend at the top of each table (required by FEATURES.md) is also a mitigation: `Legend: ↑ = higher is better (not a sort indicator), ↓ = lower is better.`

**Warning sign:**
A reader in the GitHub-rendered REPORT.md preview asks "why are all throughput columns sorted ascending?" in a PR comment. This is a documentation failure detectable only in user testing.

**Phase to address:** Phase E (direction markers).

---

### Pitfall 14: macOS Safari vs Linux Chrome Font Rendering of `↑`/`↓` Glyphs

**Severity:** LOW

**What goes wrong:**
U+2191 (↑) and U+2193 (↓) are in the Unicode Basic Arrows block (U+2190..U+21FF), universally supported since Unicode 1.0. STACK.md §Q4 correctly notes all three font stacks in `index.html.tmpl` cover these glyphs. However, on macOS Safari, the system font (San Francisco) renders U+2191 with a serrated arrow head (slightly different glyph weight) compared to Chrome on Linux (Roboto/DejaVu). This is cosmetic, not functional, but may cause pixel-diff failures in automated screenshot regression tests if such tests are added in v1.2.

**Why it happens:**
Font stack rendering of Basic Arrows block glyphs varies cosmetically across OS/browser combinations. The glyphs are semantically identical across all renderers.

**How to avoid:**
Use `aria-label` attributes on `<span>` wrappers in `index.html.tmpl` (STACK.md §Q4 already recommends this for WCAG 2.1 SC 1.3.3 compliance). This also future-proofs any screenshot regression test against font rendering variance. The `aria-label` is the semantic label; the glyph is the visual decoration:

```html
<span aria-label="higher is better">↑</span>
```

In `axes.rs`, add an `aria_label` field to `AxisSpec` to keep the HTML attribution in sync with the Rust-side direction registry.

**Warning sign:**
A pixel-diff test fails only on macOS runners but passes on Linux. Not a v1.1 risk (no screenshot tests are planned), but a v1.2 risk if visual regression testing is added.

**Phase to address:** Phase E (direction markers). The `aria-label` wrapper is low-cost to add upfront and prevents a future regression.

---

### Pitfall 15: GitHub Markdown Rendering of `↑`/`↓` in REPORT.md Pipe Tables

**Severity:** LOW

**What goes wrong:**
GitHub's Markdown renderer (CommonMark with GFM extensions) renders `| Throughput ↑ (ops/s) |` correctly in pipe tables. However, the `↑` and `↓` glyphs increase the visual width of the column header, which can cause GitHub's table auto-sizing to make the header column wider than the data columns, reducing readability for dense tables. The current REPORT.md tables already pack densely (CLAUDE.md §Conventions notes the `{:.1}` vs `{:.0}` formatting is optimized for narrow columns).

**Why it happens:**
GitHub does not proportionally auto-size table columns based on content; it uses fixed column widths determined by the widest cell. Adding a Unicode arrow to a header cell adds 2–4 pixels of width.

**How to avoid:**
This is a minor cosmetic tradeoff. The mitigation is to keep the arrow and drop the unit suffix from the header, putting the unit in a legend instead: `| ↑ Throughput |` rather than `| Throughput ↑ (ops/s) |`. Reduces header width by ~8–10 characters. Add a note to `markdown.rs::emit_per_scenario_tables` doc-comment.

**Warning sign:**
REPORT.md tables rendered on GitHub preview appear misaligned compared to the local Markdown viewer. Visual inspection during Phase E development.

**Phase to address:** Phase E (direction markers).

---

## Security Sidecar Misuse Pitfalls

### Pitfall 16: Misattribution — Security Score Applied Per-Allocator Instead of Per-Env

**Severity:** HIGH

**What goes wrong:**
`SecurityMeta` is keyed by `env: String` (ARCHITECTURE.md §Q4: "Security score is per-image, not per-build"). But in `score.rs::compute_axes`, when extracting the security axis value for a `(alloc, env)` cell, a developer may accidentally look up `security_metas.get(&(alloc.clone(), env.clone()))` — the `CellMeta` key shape — instead of `security_metas.get(&env)`. The result is silent failure (missing security meta) for all cells, defaulting to 0 or 50, which distorts the spider chart silently.

Worse: if `SecurityMeta` were embedded inside `CellMeta` in an earlier design (the ARCHITECTURE.md §Q4 explicitly rejects this but explains why it's tempting), the security score for `jemalloc·alpine` would be treated as potentially different from `mimalloc·alpine`, which is semantically wrong — both run on the same Alpine Docker image with the same CVE surface.

**Why it happens:**
`loader.rs` has two loading functions with similar signatures (`load_cell_metas` and `load_security_metas`) but different key types. Copy-pasting the lookup call pattern from `CellMeta` to `SecurityMeta` without noticing the key type difference is the most likely error path.

**How to avoid:**
Make the key type difference visible at the type system level. `load_security_metas` returns `HashMap<String, SecurityMeta>` while `load_cell_metas` returns `HashMap<(String, String), CellMeta>`. The lookup call in `compute_axes` will fail to compile if the developer passes `&(alloc, env)` as the key to a `HashMap<String, SecurityMeta>`.

Add a named test: `score::tests::security_score_is_same_for_all_allocators_in_same_env`. Use a fixture with 3 allocators all in "alpine" env with one security meta for "alpine" — assert all three cells receive the same security axis score.

**Warning sign:**
All 18 cells score 0 on the security axis (the fallback for missing key lookup). Detectable by printing `compute_axes` output during development before the golden fixture is established.

**Phase to address:** Phase A (sidecar loader) and Phase B (scoring — `compute_axes` key lookup).

---

### Pitfall 17: "Absolute" vs "Directional" Framing of Security Scores

**Severity:** MEDIUM

**What goes wrong:**
`SecurityMeta::composite_security_score` is a hand-curated 0–100 value (ARCHITECTURE.md §Q4). If the `recommend-cell.md.tmpl` prose formats this as `"Security score: 78/100"`, a reader interprets this as an absolute security guarantee: "Alpine Docker images are 78% secure." This is incorrect — the score is a *relative* ordering tool (Alpine has a better security posture than Debian-slim in this context, based on the listed criteria). Using absolute framing invites misuse: a reader who deploys Alpine in production based on the `78/100` score and then suffers a CVE will blame the benchmark.

**Why it happens:**
The `0–100` range creates an implicit "percentage of maximum security" interpretation. The `rationale` field in `SecurityMeta` is intended to provide directional context, but the template developer may display only the numeric score for compactness.

**How to avoid:**
In `recommend-cell.md.tmpl` and `recommend-cell.html.tmpl`, always emit the security score alongside its rationale and a framing qualifier:

```
Security posture (heuristic): {security_score}/100 — relative ranking only.
Source: meta/security/{env}.json
Rationale: {security_rationale}
```

The `(heuristic)` suffix on the axis label (FEATURES.md §6) reinforces this, but the per-cell card is the place where the disclaimer can be most explicit.

Name a compile-time test: `html::tests::security_prose_always_includes_heuristic_qualifier` — render the cell template and assert the rendered output contains the string "(heuristic)" at least once per cell.

**Warning sign:**
The per-cell card in REPORT.md reads `Security: 78/100` with no qualifier or source link. This is detectable by reviewing the template render output before Phase F golden-fixture regeneration.

**Phase to address:** Phase C (per-cell artifact templates).

---

## Decorate-Not-Rewrite Pitfall

### Pitfall 18: Mutating `crates/alloc-bench-core/src/output.rs` for a `security_score` Field

**Severity:** HIGH

**What goes wrong:**
The v1 input schema in `crates/alloc-bench-core/src/output.rs` is locked (CLAUDE.md §Conventions, Phase 1 D-11). A developer implementing the security axis may be tempted to add `security_score: Option<u8>` to the `Run` struct in `output.rs` so that security scores travel with the run record and are available wherever `Run` is used. This would require re-generating all existing `results/*.json` fixtures, invalidating the `tests/smoke.rs` golden fixture, and breaking the contract that v1 JSON files produced by any prior `just bench-all` invocation remain readable by the current aggregator.

**Why it happens:**
Adding a field to `output.rs` is the intuitive "add the data where it belongs" move. The sidecar pattern (separate JSON files loaded by `loader.rs`) feels like indirection. The convenience of having security data on `Run` is real.

**How to avoid:**
The `CLAUDE.md §Conventions` line — "Never mutate the bench-runner output shape" — is the guard. Enforce it with a CI gate: a `grep`-based test in `tests/smoke.rs` that asserts `crates/alloc-bench-core/src/output.rs` is byte-identical to the committed baseline. This is already conceptually present in the v1.0 byte-identical discipline but should be made explicit as a test:

```rust
// tests/smoke.rs: schema freeze guard
const SCHEMA_SHA256: &str = "[hash of output.rs at the time of v1.0 freeze]";
// assert sha256(output.rs) == SCHEMA_SHA256
```

Name the test: `smoke::tests::v1_schema_output_rs_is_frozen`.

PR checklist item: "Does this PR modify `crates/alloc-bench-core/src/output.rs`? If yes, stop — contact the team before proceeding."

**Warning sign:**
A PR diff includes changes to `crates/alloc-bench-core/src/output.rs`. This is a hard stop.

**Phase to address:** Phase A (foundations). The frozen-schema test should be added at the start of v1.1 work to prevent accidental mutation.

---

## Golden-Fixture Pitfalls

### Pitfall 19: Silent Golden-Fixture Regeneration (Absorbed Without PR Review)

**Severity:** HIGH

**What goes wrong:**
`tests/smoke.rs` contains byte-identical output tests driven by committed fixtures in `tests/fixtures/`. Phase F requires a one-time intentional regeneration of these fixtures to incorporate the v1.1 output additions (spider traces, per-cell cards, direction-marker headers). If the regeneration is performed by running `UPDATE_FIXTURES=1 cargo test` during development and the updated fixtures are committed in the same PR as the production code changes, reviewers cannot easily verify that the fixture change was intentional — they see a large binary-looking diff and assume it is correct. This is the same risk that caused the v1.0 `multi_run.rs::three_seeds_with_known_cv` golden value to be well-documented: the test was written before the implementation to create a checksum, not after.

**Why it happens:**
The convenience of `UPDATE_FIXTURES=1 cargo test` makes it easy to regenerate fixtures as a side effect of any code change. Without a separate PR, the diff between "intentional Phase F regen" and "accidentally absorbed side-effect of a bug" is invisible to reviewers.

**How to avoid:**
Phase F must be a standalone PR containing only fixture updates and no production code changes. The PR description must include:
- The specific version of `just aggregate` command used to regenerate
- The byte count of each updated fixture file (before and after)
- The assertion that `cargo test` passes before and after the fixture update in the same commit

Name the process: "Golden-Fixture Regeneration PR" — a named procedure in the CLAUDE.md conventions section after v1.1 ships.

Additionally, add a `#[test]` that asserts the fixture files were last modified by a `git commit` whose message matches the pattern `feat(06)` or `chore(06)` — i.e., the fixture regen commit must use the correct phase prefix.

**Warning sign:**
A PR that touches both `src/score.rs` and `tests/fixtures/*.json` in the same commit diff. This is a signal that production code and fixture regen were conflated.

**Phase to address:** Phase F (golden-fixture regeneration). The convention must be documented before Phase A begins.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Use `HashMap` instead of `BTreeMap` in `load_security_metas` | Faster lookup by env key | Non-deterministic iteration when collecting axis values; byte-identical output breaks across Rust versions | Never — use `BTreeMap` to match the v1.0 `load_cell_metas` pattern |
| Embed `SecurityMeta` inside `CellMeta` | One sidecar type instead of two | Security score duplicated N times (once per allocator per env); `(alloc, env)` key misrepresents that security is per-image not per-build | Never — key spaces are semantically different |
| Hard-code `n=10` in `top_n_cells` call | Simpler function signature | `TOP_N_SPIDER`, `TOP_N_TABLE`, `TOP_N_TOTAL` become magic numbers scattered across `markdown.rs` and `html.rs` | Never — use named constants from `recommend.rs` |
| Skip winsorization for the 18-cell matrix | Simpler implementation | Raw min/max normalization collapses the 0–100 range when one pathological cell dominates | Acceptable only at N=18 if documented as a known limitation; the TODO note must be in `score.rs` |
| Inline axis definitions in `polar.rs` instead of using `axes.rs::MEASUREMENT_AXES` | Avoid dependency on Phase A | Spider chart axis order drifts from REPORT.md column order; WR-01-style drift risk | Never |
| Skip `aria-label` on direction markers | Fewer template changes | WCAG 2.1 SC 1.3.3 violation; screen readers read raw Unicode codepoint name ("upwards arrow") not "higher is better" | Acceptable only if the project explicitly declares it does not target WCAG compliance (currently undeclared) |

---

## "Looks Done But Isn't" Checklist

- [ ] **Spider chart polygon closure:** `polar.rs` trace builder verified to emit 9-element `r`/`theta` arrays. Test: `polar::tests::trace_r_and_theta_are_closed_polygon`.
- [ ] **Direction-aware normalization:** `score.rs::normalize_axis` inverts for `Direction::Lower`. Test: `score::tests::lower_is_better_axis_inverts_correctly`.
- [ ] **NaN guard in scoring:** `score.rs::compute_axes` filters non-finite metrics before `normalize_axis`. Test: `score::tests::nan_run_is_skipped_not_propagated`.
- [ ] **Alphabetical tie-breaking in `score_cells`:** composite-score ties broken by `(alloc, env)` string sort. Test: `score::tests::tied_cells_break_alphabetically_for_determinism`.
- [ ] **`BTreeMap` in `load_security_metas`:** return type is `BTreeMap<String, SecurityMeta>`, not `HashMap`. Test: `loader::tests::load_security_metas_returns_btreemap_sorted_by_env`.
- [ ] **Summation order in composite score:** `score_cells` sums axes by iterating `MEASUREMENT_AXES` constant order. Test: `score::tests::composite_score_summation_order_matches_axes_rs_constant_order`.
- [ ] **`captured_at` not in emitted output:** both cell templates verified not to reference `security_meta.captured_at`. Test: grep in `smoke.rs` for date-format pattern in rendered output.
- [ ] **Both cell templates updated together:** `recommend-cell.md.tmpl` and `recommend-cell.html.tmpl` both reference every field of `CellRecommendation`. Test: `html::tests::cell_templates_both_reference_all_fields`.
- [ ] **Security score framing:** per-cell card always includes `(heuristic)` qualifier. Test: `html::tests::security_prose_always_includes_heuristic_qualifier`.
- [ ] **Frozen v1 schema:** `crates/alloc-bench-core/src/output.rs` unmodified. Test: `smoke::tests::v1_schema_output_rs_is_frozen`.
- [ ] **Plotly SRI hash unchanged:** `html.rs` CDN URL and hash match the v1.0-pinned values. Test: `smoke::tests::plotly_sri_hash_unchanged`.
- [ ] **Golden fixture regeneration is a standalone PR:** no production code changes in the Phase F commit.

---

## Pitfall-to-Phase Mapping

| Pitfall | Severity | Prevention Phase | Named Guard |
|---------|----------|-----------------|-------------|
| Direction-unaware `normalize_axis` | HIGH | Phase B | `score::tests::lower_is_better_axis_inverts_correctly` |
| NaN propagation from `multi_run` into `score.rs` | HIGH | Phase B (gate Phase A fixture) | `score::tests::nan_run_is_skipped_not_propagated` |
| p5/p95 winsorization collapses at N=18 | HIGH | Phase B | `score::tests::normalize_axis_p10_p90_clips_one_outlier_each_tail_at_n18` |
| `(heuristic)` label insufficient — formula guardrail needed | HIGH | Phase B + Phase D | `score::tests::heuristic_axes_cannot_promote_worst_measured_cell_to_top_1` |
| Non-deterministic tie-breaking in `score_cells` | HIGH | Phase B | `score::tests::tied_cells_break_alphabetically_for_determinism` |
| Cross-axis normalization (per-cell not per-axis) | HIGH | Phase B | `score::tests::normalize_axis_called_per_axis_not_per_cell` |
| `HashMap` in `load_security_metas` | HIGH | Phase A | `loader::tests::load_security_metas_returns_btreemap_sorted_by_env` |
| FP summation order in composite score | HIGH | Phase B | `score::tests::composite_score_summation_order_matches_axes_rs_constant_order` |
| NaN in composite score corrupts sort | HIGH | Phase B | `score::tests::cell_with_all_nan_runs_is_excluded_from_top_n` |
| `captured_at` leaks into deterministic output | MEDIUM | Phase A + Phase C | grep pattern test in `smoke.rs` |
| `scatterpolar` polygon not closed | MEDIUM | Phase D | `polar::tests::trace_r_and_theta_are_closed_polygon` |
| Plotly CDN upgrade without trace API audit | MEDIUM | Phase D | `smoke::tests::plotly_sri_hash_unchanged` |
| Top-N tier depth hardcoded literals | MEDIUM | Phase C | Named constants `TOP_N_SPIDER` / `TOP_N_TABLE` / `TOP_N_TOTAL` in `recommend.rs` |
| Prose staleness via hard-coded copy | HIGH | Phase C | `html::tests::template_has_no_hardcoded_allocator_prose` |
| Per-cell template struct-field sync failure | MEDIUM | Phase C | `html::tests::cell_templates_both_reference_all_fields` |
| Direction marker / sort-indicator collision | MEDIUM | Phase E | Doc-comment + legend wording |
| macOS Safari font rendering variance | LOW | Phase E | `aria-label` wrapper; no test needed for v1.1 |
| GitHub Markdown table width | LOW | Phase E | Doc-comment in `markdown.rs` |
| Security score per-allocator misattribution | HIGH | Phase A + Phase B | `score::tests::security_score_is_same_for_all_allocators_in_same_env` |
| Absolute security score framing | MEDIUM | Phase C | `html::tests::security_prose_always_includes_heuristic_qualifier` |
| Mutating `output.rs` for security data | HIGH | Phase A | `smoke::tests::v1_schema_output_rs_is_frozen` |
| Silent golden-fixture regeneration | HIGH | Phase F | Standalone-PR convention; fixture-regen PR checklist |

---

## Sources

- `crates/alloc-bench-aggregator/src/multi_run.rs` — NaN guard pattern (`rejects_nan_input`), sort pattern (`sort_by partial_cmp unwrap_or Equal`), golden-value test discipline (HIGH confidence — read in full)
- `crates/alloc-bench-aggregator/src/recommend.rs` — single-source-of-truth prose contract, alphabetical class iteration, `WR-01` tiebreak drift risk (HIGH — read in full)
- `.planning/research/ARCHITECTURE.md` — proposed `axes.rs`, `score.rs`, `polar.rs`, `recommend-cell.md.tmpl`, `recommend-cell.html.tmpl`, Phase A→F build order, `CellRecommendation` struct, `load_security_metas` pattern (HIGH — read in full)
- `.planning/research/FEATURES.md` — 8-axis spec, p5/p95 winsorization, top-3/5/10 tier display, `(heuristic)` suffix requirement, anti-feature analysis (HIGH — read in full)
- `.planning/research/STACK.md` — `normalize_axis` 15-LOC hand-roll, direction-aware implementation, `SecurityMeta` struct, `ARROW_UP`/`ARROW_DOWN` constants, Plotly SRI hash immutability (HIGH — read in full)
- `CLAUDE.md §Conventions` — byte-identical output discipline, decorate-not-rewrite, BTreeMap mandate, suspect-flag definition, locked v1 schema (HIGH — canonical)
- `.planning/milestones/v1.0-research/PITFALLS.md` — v1.0 baseline pitfalls (not repeated here; v1.1 builds on top) (HIGH)
- Wikipedia (Winsorizing) — N-dependent effectiveness of p5/p95 percentile clipping (MEDIUM — inference from the formula applied to N=18)
- Wikipedia (Multi-criteria_decision_analysis) — heuristic/objective axis mixing precedent; equal-weight composite scoring (HIGH)
- Plotly.js polar-chart docs — 9-element `r`/`theta` closing convention for `fill: 'toself'`; `polar.angularaxis.direction` default change history (HIGH — verified against Context7)

---

*Pitfalls research for: rust-benchmark-glibc-musl-mimalloc v1.1 (Recommendations, Spider Charts & Direction Markers)*
*Researched: 2026-05-26*

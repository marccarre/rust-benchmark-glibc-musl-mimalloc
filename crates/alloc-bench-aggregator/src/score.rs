//! Phase 7 / SCORE-01..04 + TEST-04 + TEST-05 — direction-aware
//! normalization, p10/p90 winsorization, composite weighted-sum scoring with
//! `MEASUREMENT_AXES` constant-order summation, and top-N selection with
//! `(alloc, env)` alphabetical tiebreak.
//!
//! Data-only. No prose. No rendering. `recommend.rs::top_n_cells` is the
//! prose-aware layer that decorates `CellScore` into `CellRecommendation`
//! (Plan 07-02).
//!
//! p5/p95 was rejected because `floor(0.05 * 18) = 0` collapses to raw
//! min/max; p10/p90 (`floor(0.10 * 18) = 1`, `floor(0.90 * 18) = 16`) clips
//! one cell per tail at N=18.

use std::collections::{BTreeMap, HashMap};

use alloc_bench_core::output::Run;

use crate::axes::{Direction, MEASUREMENT_AXES};
use crate::loader::{CellMeta, SecurityMeta};

/// Per-axis normalized score for a single (alloc, env) cell. Keys are the
/// `MEASUREMENT_AXES[i].key` strings; values are 0.0..=100.0 floats produced
/// by `normalize_axis`. `BTreeMap` (NOT `HashMap`) per byte-identical-output
/// discipline (CLAUDE.md Conventions).
#[derive(Debug, Clone, PartialEq)]
pub struct CellAxes {
    pub alloc: String,
    pub env: String,
    pub axes: BTreeMap<&'static str, f64>,
}

/// Composite score per cell. `composite` is the equal-weighted (1/8) sum
/// across all 8 axes, traversed in `MEASUREMENT_AXES.iter()` constant order
/// (NOT collected — single-ULP-drift hazard; see RESEARCH §5).
#[derive(Debug, Clone, PartialEq)]
pub struct CellScore {
    pub alloc: String,
    pub env: String,
    pub composite: f64,
    pub axes: BTreeMap<&'static str, f64>,
}

/// p10/p90 winsorize → direction-aware min-max → 0..=100. Empty input →
/// empty `Vec`; single value → `vec![50.0]`; all-equal (after winsorization)
/// → `vec![50.0; n]`. Non-finite inputs (NaN, infinity) are replaced with
/// `0.0` per slot before winsorization, mirroring `multi_run::aggregate`'s
/// `is_finite` guard convention.
///
/// RESEARCH §1 dry-run on the v1.0 production fixtures verified that all 8
/// axes produce ≥6 distinct scores at N=18 (channel=16, cpu=15,
/// image_size=6, memory=16, multithread=15, resilience=15, security=6,
/// web=16) — well above the ≥5 distinct-scores threshold. No
/// `TODO(V12-04)` marker is needed; per-axis fixed-clamp fallback remains
/// a v1.2 candidate only IF future runs show compression.
///
/// Algorithm (locked):
/// 1. Replace any non-finite element with `0.0`.
/// 2. Empty input → return `Vec::new()`.
/// 3. Single-value input → return `vec![50.0]` (deterministic mid-range).
/// 4. Sort copy ascending; compute `p10 = sorted[floor(0.10 * n)]` and
///    `p90 = sorted[floor(0.90 * n).min(n-1)]`. Clamp each input to
///    `[p10, p90]`.
/// 5. If `(p90 - p10).abs() <= 1e-12`: all-equal path → return
///    `vec![50.0; n]` (avoid div-by-zero).
/// 6. `score = (clamped - p10) / (p90 - p10) * 100.0`.
/// 7. `Direction::Lower` → `score = 100.0 - score`.
/// 8. Hard clamp to `[0.0, 100.0]`.
pub fn normalize_axis(values: &[f64], direction: Direction) -> Vec<f64> {
    let n = values.len();
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![50.0];
    }

    // Step 1: replace non-finite slots with 0.0 (mirrors multi_run.rs:67).
    let cleaned: Vec<f64> = values
        .iter()
        .map(|x| if x.is_finite() { *x } else { 0.0 })
        .collect();

    // Step 4: sort + p10/p90 indices.
    let mut sorted: Vec<f64> = cleaned.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let lo_idx = ((n as f64) * 0.10).floor() as usize;
    let hi_idx = (((n as f64) * 0.90).floor() as usize).min(n - 1);
    let p10 = sorted[lo_idx];
    let p90 = sorted[hi_idx];

    // Step 5: degenerate-range guard (avoids div-by-zero on all-equal /
    // near-equal inputs).
    let span = p90 - p10;
    if span.abs() <= 1e-12 {
        return vec![50.0; n];
    }

    cleaned
        .into_iter()
        .map(|v| {
            // Step 6: winsorize to [p10, p90] then min-max.
            let clamped = v.max(p10).min(p90);
            let raw = (clamped - p10) / span * 100.0;
            // Step 7: direction inversion.
            let directed = match direction {
                Direction::Higher => raw,
                Direction::Lower => 100.0 - raw,
            };
            // Step 8: hard clamp to [0.0, 100.0].
            directed.max(0.0).min(100.0)
        })
        .collect()
}

/// Build per-cell normalized axes by joining runs with the image-size and
/// security sidecars.
///
/// RED stub: returns an empty `Vec`. Task 2 GREEN replaces this.
pub fn compute_axes(
    _runs: &[Run],
    _cell_metas: &HashMap<(String, String), CellMeta>,
    _security_metas: &BTreeMap<String, SecurityMeta>,
) -> Vec<CellAxes> {
    Vec::new()
}

/// Equal-weighted composite (1/8 per axis), summed via
/// `MEASUREMENT_AXES.iter()` constant traversal.
///
/// RED stub: returns scores with composite = 0.0. Task 2 GREEN replaces this.
pub fn score_cells(_cell_axes: Vec<CellAxes>) -> Vec<CellScore> {
    Vec::new()
}

/// Stable sort by `(composite DESC, alloc ASC, env ASC)`; truncate to first n.
///
/// RED stub: returns the input unchanged. Task 2 GREEN replaces this.
pub fn top_n(scores: Vec<CellScore>, _n: usize) -> Vec<CellScore> {
    scores
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tolerance for elementwise float comparison; mirrors the
    /// `multi_run::tests` 1e-9 convention.
    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn assert_vec_approx(actual: &[f64], expected: &[f64]) {
        assert_eq!(actual.len(), expected.len(), "vec length mismatch");
        for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
            assert!(
                approx_eq(*a, *e),
                "index {i}: expected {e}, got {a} (diff {})",
                (a - e).abs()
            );
        }
    }

    /// SCORE-01: `Direction::Higher` keeps the natural order — the smallest
    /// raw value normalizes to 0.0, the largest to 100.0.
    #[test]
    fn higher_is_better_axis_keeps_order() {
        let out = normalize_axis(&[100.0, 200.0, 300.0], Direction::Higher);
        assert_vec_approx(&out, &[0.0, 50.0, 100.0]);
    }

    /// SCORE-01: `Direction::Lower` inverts the natural order — the
    /// smallest raw value normalizes to 100.0, the largest to 0.0. This is
    /// the verbatim ROADMAP success-criterion #1 fixture.
    #[test]
    fn lower_is_better_axis_inverts_correctly() {
        let out = normalize_axis(&[100.0, 200.0, 300.0], Direction::Lower);
        assert_vec_approx(&out, &[100.0, 50.0, 0.0]);
    }

    /// SCORE-01: every output element is hard-clamped into `[0.0, 100.0]`.
    /// At N=18 the inner 16 values fall strictly inside the bounds; the
    /// p10/p90 boundary cells touch 0.0 and 100.0 exactly.
    #[test]
    fn normalized_scores_are_clamped_to_0_100() {
        let inputs: Vec<f64> = (1..=18).map(|i| i as f64).collect();
        let out = normalize_axis(&inputs, Direction::Higher);
        assert_eq!(out.len(), 18);
        for (i, v) in out.iter().enumerate() {
            assert!(
                (0.0..=100.0).contains(v),
                "index {i}: value {v} outside [0.0, 100.0]"
            );
        }
    }

    /// SCORE-02: at N=18, `floor(0.10 * 18) = 1` selects index 1 (the
    /// SECOND-smallest sorted value) as p10, and `floor(0.90 * 18) = 16`
    /// selects index 16 (the SECOND-largest sorted value) as p90. So one
    /// extreme outlier per tail is winsorized and maps to the boundary
    /// score; the inner 16 values fill `(0, 100)` monotone increasing.
    #[test]
    fn normalize_axis_p10_p90_clips_one_outlier_each_tail_at_n18() {
        // Sorted layout: [-1e9, 1, 2, 3, ..., 16, 1e9].
        let mut inputs: Vec<f64> = vec![-1.0e9];
        inputs.extend((1..=16).map(|i| i as f64));
        inputs.push(1.0e9);
        assert_eq!(inputs.len(), 18);

        let out = normalize_axis(&inputs, Direction::Higher);
        assert_eq!(out.len(), 18);

        // Extreme low outlier was clipped to p10 (= sorted[1] = 1.0) →
        // (1.0 - 1.0) / (16.0 - 1.0) * 100.0 = 0.0.
        assert!(
            approx_eq(out[0], 0.0),
            "extreme low outlier should clip to 0.0, got {}",
            out[0]
        );
        // Extreme high outlier was clipped to p90 (= sorted[16] = 16.0) →
        // (16.0 - 1.0) / (16.0 - 1.0) * 100.0 = 100.0.
        assert!(
            approx_eq(out[17], 100.0),
            "extreme high outlier should clip to 100.0, got {}",
            out[17]
        );

        // Inner monotone strictness: idx 2..=15 should be strictly inside
        // (0.0, 100.0) since their raw values 2..=15 are strictly inside
        // [p10=1, p90=16].
        for i in 2..=15usize {
            let v = out[i];
            assert!(
                v > 0.0 && v < 100.0,
                "inner value at idx {i}: {v} expected strictly in (0.0, 100.0)"
            );
        }
        // And full monotone non-decreasing across the inner block.
        for i in 2..=16usize {
            assert!(
                out[i] >= out[i - 1] - 1e-9,
                "inner values must be monotone increasing: idx {} = {}, idx {} = {}",
                i - 1,
                out[i - 1],
                i,
                out[i]
            );
        }
    }

    /// Edge case (CONTEXT §Claude's Discretion, lock #1): empty input
    /// returns `Vec::new()` — no panic, no out-of-bounds index.
    #[test]
    fn normalize_axis_empty_input_returns_empty() {
        let out = normalize_axis(&[], Direction::Higher);
        assert!(out.is_empty(), "expected empty Vec, got {out:?}");
    }

    /// Edge case (CONTEXT §Claude's Discretion, lock #2): single-value
    /// input is deterministically mid-range. Mirrors the convention used
    /// for all-equal inputs.
    #[test]
    fn normalize_axis_single_value_returns_mid_range() {
        let out = normalize_axis(&[42.0], Direction::Higher);
        assert_vec_approx(&out, &[50.0]);
    }

    /// Edge case (CONTEXT §Claude's Discretion, lock #3): all-equal input
    /// yields `[50.0; n]` to avoid the `(p90 - p10) = 0` divide-by-zero
    /// path in min-max normalization.
    #[test]
    fn normalize_axis_all_equal_returns_mid_range() {
        let out = normalize_axis(&[7.0, 7.0, 7.0], Direction::Higher);
        assert_vec_approx(&out, &[50.0, 50.0, 50.0]);
    }

    // ---- Task 2 helpers + tests below ----

    use alloc_bench_core::output::{
        Build, Env, HarnessInfo, LatencyNs, Metrics, Rusage, ScenarioInfo,
    };
    use alloc_bench_core::SCHEMA_VERSION;

    /// Build one normalized `CellAxes` with every axis set to the same
    /// score `v`. Used by composite-summation and tiebreak tests.
    fn synth_cell_axes(alloc: &str, env: &str, v: f64) -> CellAxes {
        let mut axes: BTreeMap<&'static str, f64> = BTreeMap::new();
        for spec in MEASUREMENT_AXES.iter() {
            axes.insert(spec.key, v);
        }
        CellAxes {
            alloc: alloc.into(),
            env: env.into(),
            axes,
        }
    }

    /// Build a `CellAxes` with a per-axis-key score map. Missing keys are
    /// filled with 0.0 so callers can spike a single axis cheaply.
    fn synth_cell_axes_keyed(
        alloc: &str,
        env: &str,
        overrides: &[(&'static str, f64)],
    ) -> CellAxes {
        let mut axes: BTreeMap<&'static str, f64> = BTreeMap::new();
        for spec in MEASUREMENT_AXES.iter() {
            axes.insert(spec.key, 0.0);
        }
        for (k, v) in overrides {
            axes.insert(*k, *v);
        }
        CellAxes {
            alloc: alloc.into(),
            env: env.into(),
            axes,
        }
    }

    /// Synthetic Run builder used by `compute_axes_consumes_runs_metas_and_security_alphabetically`.
    /// Mirrors `recommend.rs::tests::synth_run` shape but populates
    /// `env.docker_image` so the env-extraction path in `compute_axes`
    /// resolves correctly: `"alloc-bench:{alloc}-{env}"`.
    fn synth_run_for_score(alloc: &str, env: &str, scenario: &str, throughput: f64) -> Run {
        Run {
            schema_version: SCHEMA_VERSION,
            run_id: format!("synth-{alloc}-{env}-{scenario}"),
            env: Env {
                os: "linux".into(),
                os_version: "test".into(),
                docker_image: Some(format!("alloc-bench:{alloc}-{env}")),
                cpu_model: "test-cpu".into(),
                cpu_count: 1,
                memory_total_kb: 1,
            },
            build: Build {
                allocator: alloc.into(),
                rustc_version: "1.83.0".into(),
                target_triple: "x86_64-unknown-linux-gnu".into(),
                host_triple: "x86_64-unknown-linux-gnu".into(),
                profile: "release".into(),
                git_sha: "0".repeat(40),
                git_dirty: false,
                build_timestamp: "2026-05-19T00:00:00Z".into(),
                rustflags: "".into(),
            },
            scenario: ScenarioInfo {
                name: scenario.into(),
                config: serde_json::json!({}),
                unit: None,
            },
            harness: HarnessInfo {
                warmup_duration_s: 5.0,
                measurement_duration_s: 5.0,
                samples_count: 50_000,
            },
            metrics: Metrics {
                ticks_per_s: throughput,
                allocations_per_tick: 100,
                tick_latency_ns: LatencyNs {
                    p50: 1000,
                    p95: 2000,
                    p99: 3000,
                    p999: 5000,
                    max: 10000,
                },
                peak_rss_kb: 1000,
                rss_growth_samples: vec![],
                rusage: Rusage {
                    user_time_s: 0.0,
                    sys_time_s: 0.0,
                    minor_faults: 0,
                    major_faults: 0,
                    voluntary_csw: 0,
                    involuntary_csw: 0,
                    peak_rss_kb: 1000,
                },
                allocator_stats: serde_json::json!({}),
            },
            status: Some("success".into()),
            error: None,
        }
    }

    /// SCORE-03: a cell with every axis at 100.0 yields a composite of
    /// exactly `8 * 100.0 * 0.125 = 100.0`.
    #[test]
    fn composite_uses_equal_weights_one_eighth_per_axis() {
        let cells = vec![synth_cell_axes("jemalloc", "alpine", 100.0)];
        let scores = score_cells(cells);
        assert_eq!(scores.len(), 1);
        assert!(
            (scores[0].composite - 100.0).abs() < 1e-9,
            "expected composite ≈ 100.0, got {}",
            scores[0].composite
        );
    }

    /// TEST-04 / SCORE-03: composite summation traverses
    /// `MEASUREMENT_AXES.iter()` in constant declaration order. Two cells
    /// — one spiked on the first axis only, one spiked on the last axis
    /// only — must produce bit-equal composites of exactly 12.5
    /// (`100.0 * 0.125`). Any deviation from the locked iterator order
    /// would FP-accumulate to a slightly different value.
    #[test]
    fn composite_score_summation_order_matches_axes_rs_constant_order() {
        let first_key = MEASUREMENT_AXES[0].key;
        let last_key = MEASUREMENT_AXES[7].key;
        let cells = vec![
            synth_cell_axes_keyed("a", "alpha", &[(first_key, 100.0)]),
            synth_cell_axes_keyed("b", "beta", &[(last_key, 100.0)]),
        ];
        let scores = score_cells(cells);
        assert_eq!(scores.len(), 2);
        // Both spike-of-100 cells must produce identical composites.
        assert_eq!(
            scores[0].composite, scores[1].composite,
            "spike-on-first vs spike-on-last must produce bit-equal composites"
        );
        // And both equal exactly 12.5 (single 100.0 * 0.125).
        assert_eq!(
            scores[0].composite, 12.5,
            "single-axis spike must yield exactly 12.5, got {}",
            scores[0].composite
        );
    }

    /// SCORE-03 worst-case guard: a cell with measured axes near-bottom
    /// but heuristic axes at 100.0 must not be promoted to rank 1 by the
    /// equal-weight composite. Builds an 18-cell synthetic fixture; one
    /// "ptmalloc/wolfi" cell has measured-bottom + heuristic-100, the
    /// other 17 have measured spread `[40, 90]` and heuristic `[0, 50]`.
    /// Asserts the heuristic-100 cell is NOT rank 1.
    #[test]
    fn heuristic_axes_cannot_promote_worst_measured_cell_to_top_1() {
        let measured_keys: Vec<&'static str> = MEASUREMENT_AXES
            .iter()
            .filter(|s| !s.is_heuristic)
            .map(|s| s.key)
            .collect();
        let heuristic_keys: Vec<&'static str> = MEASUREMENT_AXES
            .iter()
            .filter(|s| s.is_heuristic)
            .map(|s| s.key)
            .collect();
        assert_eq!(measured_keys.len(), 6, "RESEARCH §1: 6 measured axes");
        assert_eq!(heuristic_keys.len(), 2, "RESEARCH §1: 2 heuristic axes");

        let mut cells: Vec<CellAxes> = Vec::with_capacity(18);
        // The decoy: heuristic-100 + measured-5.
        let mut decoy_overrides: Vec<(&'static str, f64)> =
            measured_keys.iter().map(|k| (*k, 5.0)).collect();
        for k in &heuristic_keys {
            decoy_overrides.push((*k, 100.0));
        }
        cells.push(synth_cell_axes_keyed("ptmalloc", "wolfi", &decoy_overrides));

        // The other 17: measured spread [40, 90] linearly across cells;
        // heuristic spread [0, 50] linearly. Use synthetic alloc/env names
        // that sort AFTER "ptmalloc/wolfi" to defeat the alphabetical
        // tiebreak — composites must dominate, not the alloc name.
        for i in 0..17 {
            let frac = i as f64 / 16.0; // 0..=1 across the 17 cells
            let measured_v = 40.0 + frac * 50.0; // 40..=90
            let heuristic_v = frac * 50.0; // 0..=50
            let mut overrides: Vec<(&'static str, f64)> =
                measured_keys.iter().map(|k| (*k, measured_v)).collect();
            for k in &heuristic_keys {
                overrides.push((*k, heuristic_v));
            }
            // Use names that sort AFTER ("ptmalloc", "wolfi") so the
            // alphabetical tiebreak alone could not promote the decoy.
            let alloc = format!("zalloc{:02}", i);
            let env = format!("zenv{:02}", i);
            cells.push(synth_cell_axes_keyed(&alloc, &env, &overrides));
        }
        assert_eq!(cells.len(), 18);

        let scores = score_cells(cells);
        let top = top_n(scores, 1);
        assert_eq!(top.len(), 1);
        assert!(
            !(top[0].alloc == "ptmalloc" && top[0].env == "wolfi"),
            "the heuristic-100 cell must NOT be rank 1; got ({}, {}) with composite {}",
            top[0].alloc,
            top[0].env,
            top[0].composite,
        );
    }

    /// SCORE-04: tied composites break alphabetically by `(alloc ASC,
    /// env ASC)`. Two cells with composite = 50.0 each: `(jemalloc,
    /// alpine)` lex-precedes `(ptmalloc, wolfi)`, so it must occupy
    /// `top_n[0]`.
    #[test]
    fn tied_cells_break_alphabetically_for_determinism() {
        let cells = vec![
            synth_cell_axes("ptmalloc", "wolfi", 50.0),
            synth_cell_axes("jemalloc", "alpine", 50.0),
        ];
        let scores = score_cells(cells);
        let top = top_n(scores, 2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].alloc, "jemalloc");
        assert_eq!(top[0].env, "alpine");
        assert_eq!(top[1].alloc, "ptmalloc");
        assert_eq!(top[1].env, "wolfi");
        // Both composites should equal exactly 50.0.
        assert!((top[0].composite - 50.0).abs() < 1e-9);
        assert!((top[1].composite - 50.0).abs() < 1e-9);
    }

    /// SCORE-04: `top_n` truncates to the first `n` after sorting by
    /// composite DESC. 18 cells with composites `1..=18` reduce to the
    /// last five (`14..=18`) in descending order.
    #[test]
    fn top_n_returns_at_most_n_elements() {
        let cells: Vec<CellAxes> = (1..=18)
            .map(|i| {
                // Use lex-distinct (alloc, env) so tiebreak never fires.
                let alloc = format!("alloc{:02}", i);
                let env = format!("env{:02}", i);
                synth_cell_axes(&alloc, &env, i as f64)
            })
            .collect();
        let scores = score_cells(cells);
        let top = top_n(scores, 5);
        assert_eq!(top.len(), 5);
        let composites: Vec<f64> = top.iter().map(|s| s.composite).collect();
        // Composites must be strictly descending: 18, 17, 16, 15, 14.
        let expected = vec![18.0, 17.0, 16.0, 15.0, 14.0];
        for (i, (got, want)) in composites.iter().zip(expected.iter()).enumerate() {
            assert!(
                (got - want).abs() < 1e-9,
                "rank {i}: expected composite {want}, got {got}"
            );
        }
    }

    /// SCORE-04 edge case: empty input → empty output (no panic).
    #[test]
    fn top_n_of_empty_returns_empty() {
        let out = top_n(Vec::new(), 5);
        assert!(out.is_empty());
    }

    /// TEST-05: a cell with composite = NaN must NOT silently float to
    /// rank 1. `partial_cmp` on NaN returns `None`; `unwrap_or(Equal)`
    /// falls through to the alphabetical secondary sort. The NaN cell
    /// MUST never outrank a finite-composite cell.
    #[test]
    fn nan_input_does_not_corrupt_score() {
        let mut zero_axes: BTreeMap<&'static str, f64> = BTreeMap::new();
        for spec in MEASUREMENT_AXES.iter() {
            zero_axes.insert(spec.key, 0.0);
        }
        // Composites computed manually so the test exercises top_n alone.
        let a = CellScore {
            alloc: "a-alloc".into(),
            env: "a-env".into(),
            composite: 90.0,
            axes: zero_axes.clone(),
        };
        let b = CellScore {
            alloc: "b-alloc".into(),
            env: "b-env".into(),
            composite: f64::NAN,
            axes: zero_axes.clone(),
        };
        let c = CellScore {
            alloc: "c-alloc".into(),
            env: "c-env".into(),
            composite: 80.0,
            axes: zero_axes,
        };
        let scores = vec![a, b, c];
        let out = top_n(scores, 3);
        assert_eq!(out.len(), 3);
        // Rank 1 must be the finite-90.0 cell.
        assert!(
            (out[0].composite - 90.0).abs() < 1e-9,
            "rank 1 composite: expected 90.0, got {} (alloc={}, env={})",
            out[0].composite,
            out[0].alloc,
            out[0].env,
        );
        // Rank 2 must be the finite-80.0 cell.
        assert!(
            (out[1].composite - 80.0).abs() < 1e-9,
            "rank 2 composite: expected 80.0, got {} (alloc={}, env={})",
            out[1].composite,
            out[1].alloc,
            out[1].env,
        );
        // Rank 3 must be the NaN cell — and the NaN must still be NaN.
        assert!(
            out[2].composite.is_nan(),
            "rank 3 composite: expected NaN, got {} (alloc={}, env={})",
            out[2].composite,
            out[2].alloc,
            out[2].env,
        );
    }

    /// `compute_axes` smoke test: 6 synthetic Runs across 2 cells →
    /// `Vec<CellAxes>` of length 2 in alphabetical `(alloc, env)` order.
    /// Each entry has all 8 axis keys in its `axes` map.
    #[test]
    fn compute_axes_consumes_runs_metas_and_security_alphabetically() {
        let runs = vec![
            // ("jemalloc", "alpine") — three scenarios.
            synth_run_for_score("jemalloc", "alpine", "cpu-bound", 200.0),
            synth_run_for_score("jemalloc", "alpine", "web", 1500.0),
            synth_run_for_score("jemalloc", "alpine", "multithread", 700.0),
            // ("mimalloc", "wolfi") — three scenarios.
            synth_run_for_score("mimalloc", "wolfi", "cpu-bound", 180.0),
            synth_run_for_score("mimalloc", "wolfi", "web", 1700.0),
            synth_run_for_score("mimalloc", "wolfi", "multithread", 800.0),
        ];
        let cell_metas: HashMap<(String, String), CellMeta> = HashMap::new();
        let mut security_metas: BTreeMap<String, SecurityMeta> = BTreeMap::new();
        security_metas.insert(
            "alpine".into(),
            SecurityMeta {
                env: "alpine".into(),
                score: 60,
                rationale: "synth".into(),
                captured_at: "2026-05-26".into(),
            },
        );
        security_metas.insert(
            "wolfi".into(),
            SecurityMeta {
                env: "wolfi".into(),
                score: 75,
                rationale: "synth".into(),
                captured_at: "2026-05-26".into(),
            },
        );

        let out = compute_axes(&runs, &cell_metas, &security_metas);
        assert_eq!(out.len(), 2, "expected 2 cells, got {}", out.len());
        assert_eq!(out[0].alloc, "jemalloc");
        assert_eq!(out[0].env, "alpine");
        assert_eq!(out[1].alloc, "mimalloc");
        assert_eq!(out[1].env, "wolfi");
        // Each cell's BTreeMap must contain all 8 axis keys.
        for (i, cell) in out.iter().enumerate() {
            for spec in MEASUREMENT_AXES.iter() {
                assert!(
                    cell.axes.contains_key(spec.key),
                    "cell {i}: missing axis key {}",
                    spec.key
                );
                let v = cell.axes[spec.key];
                assert!(
                    (0.0..=100.0).contains(&v),
                    "cell {i} axis {}: value {v} outside [0.0, 100.0]",
                    spec.key
                );
            }
        }
    }
}

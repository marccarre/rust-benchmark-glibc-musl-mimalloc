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

use std::collections::{BTreeMap, BTreeSet, HashMap};

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

/// Private duplicate of recommend.rs::env_short_name (Phase 7 / v1.1). Both
/// copies must agree byte-for-byte on env-extraction. v1.2 may consolidate
/// to crate::env::short_name. See Plan 07-02 §Step 5 (DUPLICATE-AVOIDANCE
/// NOTE).
///
/// Extraction recipe: split `r.env.docker_image` on `:` (taking the right
/// half), then split that on `-` and take element `[1]`. So
/// `"alloc-bench:jemalloc-alpine"` → `"alpine"`. Defensive fallback on any
/// missing/malformed segment is the literal `"host"` (matches the
/// `markdown::env_label` host-fallback convention).
fn env_short_name(r: &Run) -> String {
    let image = match r.env.docker_image.as_deref() {
        Some(s) => s,
        None => return "host".to_string(),
    };
    // "alloc-bench:jemalloc-alpine" → after_colon = "jemalloc-alpine"
    let after_colon = match image.split_once(':') {
        Some((_, right)) => right,
        None => return "host".to_string(),
    };
    // "jemalloc-alpine" → take element [1] = "alpine"
    let mut parts = after_colon.splitn(2, '-');
    let _alloc = parts.next();
    match parts.next() {
        Some(env) if !env.is_empty() => env.to_string(),
        _ => "host".to_string(),
    }
}

/// Compute the median throughput for one scenario name within a cell's
/// runs. Returns `None` when no run matches OR when `multi_run::aggregate`
/// rejects the sample (NaN-poisoned, <2 samples). Single-sample is
/// special-cased to return that single value (matches the v1.1
/// single-seed-per-cell production reality where `aggregate` would return
/// `None` for n=1).
fn cell_scenario_throughput_median(runs: &[&Run], scenario: &str) -> Option<f64> {
    let samples: Vec<f64> = runs
        .iter()
        .filter(|r| r.scenario.name == scenario)
        .map(|r| r.metrics.ticks_per_s)
        .collect();
    if samples.is_empty() {
        return None;
    }
    if samples.len() == 1 {
        return Some(samples[0]);
    }
    crate::multi_run::aggregate(&samples).map(|s| s.median)
}

/// Same as `cell_scenario_throughput_median` but pulls `peak_rss_kb`
/// instead of `ticks_per_s` (used for the `memory_fragmentation` axis,
/// Lower-is-better).
fn cell_scenario_peak_rss_median(runs: &[&Run], scenario: &str) -> Option<f64> {
    let samples: Vec<f64> = runs
        .iter()
        .filter(|r| r.scenario.name == scenario)
        .map(|r| r.metrics.peak_rss_kb as f64)
        .collect();
    if samples.is_empty() {
        return None;
    }
    if samples.len() == 1 {
        return Some(samples[0]);
    }
    crate::multi_run::aggregate(&samples).map(|s| s.median)
}

/// Mean of the present per-scenario medians; absent scenarios are skipped
/// (NOT treated as 0.0). When ALL scenarios are absent, returns `0.0`
/// (sentinel — feeds into normalize_axis's degenerate-range guard).
fn mean_of_present_medians(values: &[Option<f64>]) -> f64 {
    let present: Vec<f64> = values.iter().filter_map(|x| *x).collect();
    if present.is_empty() {
        return 0.0;
    }
    let sum: f64 = present.iter().sum();
    sum / present.len() as f64
}

/// Build per-cell normalized axes by joining runs with the image-size and
/// security sidecars.
///
/// Algorithm (locked per RESEARCH §1 axis-to-scenario mapping):
/// 1. Group runs into a `BTreeMap<(alloc, env_short), Vec<&Run>>` so the
///    return order is alphabetical by `(alloc, env_short)`.
/// 2. For each cell, compute the 6 measured-axis raw values:
///    - `channel_throughput` = mean(spmc, mpsc, mpmc) medians
///    - `cpu_bound_throughput` = `cpu-bound` median
///    - `memory_fragmentation` = mean(mem-bound, fragmentation-soak) peak_rss
///    - `multithread_throughput` = `multithread` median
///    - `resilience` = mean(realloc-storm, contention) medians
///    - `web_throughput` = `web` median
/// 3. Heuristic axes:
///    - `image_size_efficiency` = `cell_metas[(alloc, env_short)].image_size_mb`
///       (raw MB; effective_direction = Lower because smaller MB = better).
///    - `security_posture` = `security_metas[env_short].score as f64`
///       (0..=100, effective_direction = Higher).
///    - Either falling back to `0.0` when the sidecar is absent.
/// 4. Build per-axis input vectors of length N (one entry per cell, in
///    alphabetical cell order). Call `normalize_axis(&inputs, effective_dir)`
///    once per axis.
/// 5. Stitch results back: for each cell, build its `axes: BTreeMap` keyed
///    by `MEASUREMENT_AXES[i].key`.
pub fn compute_axes(
    runs: &[Run],
    cell_metas: &HashMap<(String, String), CellMeta>,
    security_metas: &BTreeMap<String, SecurityMeta>,
) -> Vec<CellAxes> {
    // Step 1 — group by (alloc, env_short) into BTreeMap (alphabetical).
    let mut grouped: BTreeMap<(String, String), Vec<&Run>> = BTreeMap::new();
    for r in runs {
        let key = (r.build.allocator.clone(), env_short_name(r));
        grouped.entry(key).or_default().push(r);
    }

    if grouped.is_empty() {
        return Vec::new();
    }

    let n = grouped.len();
    let cell_keys: Vec<(String, String)> = grouped.keys().cloned().collect();

    // Step 2/3 — collect per-cell raw values for each of the 8 axes,
    // alphabetical cell order. Index `i` of the inner Vec corresponds to
    // `cell_keys[i]`.
    let mut raw_per_axis: BTreeMap<&'static str, Vec<f64>> = BTreeMap::new();
    for spec in MEASUREMENT_AXES.iter() {
        raw_per_axis.insert(spec.key, Vec::with_capacity(n));
    }

    for (alloc, env) in cell_keys.iter() {
        let cell_runs = grouped.get(&(alloc.clone(), env.clone())).expect("present");

        // channel_throughput = mean(spmc, mpsc, mpmc) medians.
        let channel = mean_of_present_medians(&[
            cell_scenario_throughput_median(cell_runs, "spmc"),
            cell_scenario_throughput_median(cell_runs, "mpsc"),
            cell_scenario_throughput_median(cell_runs, "mpmc"),
        ]);
        // cpu_bound_throughput.
        let cpu = cell_scenario_throughput_median(cell_runs, "cpu-bound").unwrap_or(0.0);
        // memory_fragmentation = mean(mem-bound, fragmentation-soak) peak_rss.
        let mem = mean_of_present_medians(&[
            cell_scenario_peak_rss_median(cell_runs, "mem-bound"),
            cell_scenario_peak_rss_median(cell_runs, "fragmentation-soak"),
        ]);
        // multithread_throughput.
        let multithread =
            cell_scenario_throughput_median(cell_runs, "multithread").unwrap_or(0.0);
        // resilience = mean(realloc-storm, contention) medians.
        let resilience = mean_of_present_medians(&[
            cell_scenario_throughput_median(cell_runs, "realloc-storm"),
            cell_scenario_throughput_median(cell_runs, "contention"),
        ]);
        // web_throughput.
        let web = cell_scenario_throughput_median(cell_runs, "web").unwrap_or(0.0);

        // image_size_efficiency raw = image_size_mb (Lower = better via
        // effective_direction below).
        let image_mb = cell_metas
            .get(&(alloc.clone(), env.clone()))
            .map(|m| m.image_size_mb)
            .unwrap_or(0.0);

        // security_posture raw = security score (0..=100, Higher better).
        let security = security_metas
            .get(env)
            .map(|m| m.score as f64)
            .unwrap_or(0.0);

        raw_per_axis.get_mut("channel_throughput").unwrap().push(channel);
        raw_per_axis.get_mut("cpu_bound_throughput").unwrap().push(cpu);
        raw_per_axis.get_mut("image_size_efficiency").unwrap().push(image_mb);
        raw_per_axis.get_mut("memory_fragmentation").unwrap().push(mem);
        raw_per_axis.get_mut("multithread_throughput").unwrap().push(multithread);
        raw_per_axis.get_mut("resilience").unwrap().push(resilience);
        raw_per_axis.get_mut("security_posture").unwrap().push(security);
        raw_per_axis.get_mut("web_throughput").unwrap().push(web);
    }

    // Step 4 — normalize each axis with the effective direction. The
    // effective direction differs from `spec.direction` for two cases:
    // - `image_size_efficiency`: spec.direction is `Higher` (the *axis*
    //   represents efficiency, where higher score is better) but the *raw*
    //   input is image_size_mb where smaller is better. So we feed
    //   `Direction::Lower` to flip the min-max output sign.
    // - `security_posture`: spec.direction is `Higher` AND raw is the
    //   sidecar score (0..=100, higher is better) — pass `Direction::Higher`.
    let mut normalized_per_axis: BTreeMap<&'static str, Vec<f64>> = BTreeMap::new();
    for spec in MEASUREMENT_AXES.iter() {
        let raw = raw_per_axis.get(spec.key).expect("populated above");
        let effective_dir = match spec.key {
            "image_size_efficiency" => Direction::Lower,
            "security_posture" => Direction::Higher,
            _ => spec.direction,
        };
        normalized_per_axis.insert(spec.key, normalize_axis(raw, effective_dir));
    }

    // Step 5 — stitch per-cell BTreeMaps from the i-th element of each
    // normalized vector. Iteration order over cell_keys is alphabetical
    // (Step 1 grouped via BTreeMap).
    let mut out: Vec<CellAxes> = Vec::with_capacity(n);
    for (i, (alloc, env)) in cell_keys.iter().enumerate() {
        let mut axes: BTreeMap<&'static str, f64> = BTreeMap::new();
        for spec in MEASUREMENT_AXES.iter() {
            let v = normalized_per_axis
                .get(spec.key)
                .and_then(|vec| vec.get(i).copied())
                .unwrap_or(0.0);
            axes.insert(spec.key, v);
        }
        out.push(CellAxes {
            alloc: alloc.clone(),
            env: env.clone(),
            axes,
        });
    }
    out
}

/// Equal-weighted composite (1/8 per axis), summed via
/// `MEASUREMENT_AXES.iter()` constant traversal — NOT a collected pair-Vec
/// (single-ULP-drift hazard, RESEARCH §5). Output preserves input order.
pub fn score_cells(cell_axes: Vec<CellAxes>) -> Vec<CellScore> {
    cell_axes
        .into_iter()
        .map(|cell| {
            let composite: f64 = MEASUREMENT_AXES
                .iter()
                .map(|spec| cell.axes.get(spec.key).copied().unwrap_or(0.0) * 0.125)
                .sum();
            CellScore {
                alloc: cell.alloc,
                env: cell.env,
                composite,
                axes: cell.axes,
            }
        })
        .collect()
}

/// Stable sort by `(composite DESC, alloc ASC, env ASC)` then truncate to
/// the first `n`. NaN-poisoning guard via `partial_cmp(...).unwrap_or(Equal)`
/// — NaN composites fall through to the alphabetical secondary sort, never
/// silently floating to first place.
pub fn top_n(scores: Vec<CellScore>, n: usize) -> Vec<CellScore> {
    let mut scores = scores;
    scores.sort_by(|a, b| {
        // Primary: composite DESC. `b.partial_cmp(&a)` for descending.
        b.composite
            .partial_cmp(&a.composite)
            .unwrap_or(std::cmp::Ordering::Equal)
            // Secondary: alloc ASC.
            .then_with(|| a.alloc.cmp(&b.alloc))
            // Tertiary: env ASC.
            .then_with(|| a.env.cmp(&b.env))
    });
    scores.truncate(n);
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
    /// rank 1 or rank 2 above a finite-composite cell. `partial_cmp` on NaN
    /// returns `None`; `unwrap_or(Equal)` falls through to the alphabetical
    /// secondary sort. NAMING NOTE: the NaN cell ("z-alloc", "z-env") is
    /// chosen so the alphabetical tiebreak on `(alloc, env)` naturally
    /// places NaN last — exercising the plan's locked `top_n` spec
    /// (07-01-PLAN §interfaces) where NaN sinks to rank-N via the secondary
    /// alphabetical sort, not via a separate NaN-aware comparator. With
    /// alloc names `a-, b-, z-` the test pins the strong assertion
    /// `top_n[2].composite.is_nan()` (07-01-PLAN behavior line 260).
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
            composite: 80.0,
            axes: zero_axes.clone(),
        };
        let z = CellScore {
            alloc: "z-alloc".into(),
            env: "z-env".into(),
            composite: f64::NAN,
            axes: zero_axes,
        };
        let scores = vec![a, b, z];
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
        // Rank 2 must be the finite-80.0 cell — NaN must NOT outrank it.
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

    // ------------------------------------------------------------------
    // Phase 9 / POLAR-05 — pareto_front sibling fn tests (added by Plan
    // 09-02 Task 1).
    //
    // Algorithm reference: 09-RESEARCH.md §4 (full O(n²) sweep). macOS
    // host (cells whose env is absent from `image_sizes`) is excluded
    // from the front per 09-CONTEXT.md §"Pareto-front computation
    // location".
    // ------------------------------------------------------------------

    /// Build a minimal `CellScore` for Pareto-front tests. The algorithm
    /// only consults `composite` and `image_sizes` — so `axes` is left
    /// empty and `alloc`/`env` are kept short and lex-stable.
    fn synth_cell(alloc: &str, env: &str, composite: f64) -> CellScore {
        CellScore {
            alloc: alloc.into(),
            env: env.into(),
            composite,
            axes: BTreeMap::new(),
        }
    }

    /// POLAR-05: A cell strictly dominated on BOTH axes (lower composite
    /// AND larger image) must NOT be on the front. Cell A
    /// (composite=0.9, env="alpine", image=50.0) dominates cell B
    /// (composite=0.5, env="debian-slim", image=200.0) on both axes →
    /// only A is on the front.
    #[test]
    fn pareto_front_strictly_dominated_cell_excluded() {
        let cells = vec![
            synth_cell("alloc", "alpine", 0.9),
            synth_cell("alloc", "debian-slim", 0.5),
        ];
        let mut image_sizes: BTreeMap<String, f64> = BTreeMap::new();
        image_sizes.insert("alpine".into(), 50.0);
        image_sizes.insert("debian-slim".into(), 200.0);

        let front = pareto_front(&cells, &image_sizes);
        assert_eq!(front.len(), 1);
        assert!(front.contains(&("alloc".to_string(), "alpine".to_string())));
        assert!(!front.contains(&("alloc".to_string(), "debian-slim".to_string())));
    }

    /// POLAR-05: Two non-dominated cells (one wins on composite, the
    /// other wins on image size) must BOTH be on the front. A
    /// (composite=0.9, image=200.0) and B (composite=0.5, image=10.0):
    /// neither dominates the other.
    #[test]
    fn pareto_front_non_dominated_pair_both_on_front() {
        let cells = vec![
            synth_cell("alloc", "alpine", 0.9),
            synth_cell("alloc", "scratch", 0.5),
        ];
        let mut image_sizes: BTreeMap<String, f64> = BTreeMap::new();
        image_sizes.insert("alpine".into(), 200.0);
        image_sizes.insert("scratch".into(), 10.0);

        let front = pareto_front(&cells, &image_sizes);
        assert_eq!(front.len(), 2);
        assert!(front.contains(&("alloc".to_string(), "alpine".to_string())));
        assert!(front.contains(&("alloc".to_string(), "scratch".to_string())));
    }

    /// POLAR-05: Cells whose env is absent from `image_sizes` (macOS
    /// host case) must be excluded from the front. With three cells —
    /// A (alpine, 0.9), B (host, 0.95, no image), C (debian-slim, 0.5)
    /// — and `image_sizes = { alpine: 50.0, debian-slim: 200.0 }`:
    /// B is excluded because no image entry exists; C is strictly
    /// dominated by A (higher composite + smaller image). Only A
    /// remains on the front.
    #[test]
    fn pareto_front_excludes_cells_without_image_size() {
        let cells = vec![
            synth_cell("alloc", "alpine", 0.9),
            synth_cell("alloc", "host", 0.95),
            synth_cell("alloc", "debian-slim", 0.5),
        ];
        let mut image_sizes: BTreeMap<String, f64> = BTreeMap::new();
        image_sizes.insert("alpine".into(), 50.0);
        image_sizes.insert("debian-slim".into(), 200.0);
        // No entry for "host" — macOS host case.

        let front = pareto_front(&cells, &image_sizes);
        assert_eq!(front.len(), 1);
        assert!(front.contains(&("alloc".to_string(), "alpine".to_string())));
        assert!(!front.contains(&("alloc".to_string(), "host".to_string())));
        assert!(!front.contains(&("alloc".to_string(), "debian-slim".to_string())));
    }

    /// POLAR-05 (RESEARCH §4): Pareto domination requires STRICT
    /// inequality on at least one axis. Two cells with EQUAL composite
    /// AND EQUAL image_size do NOT dominate each other — both stay on
    /// the front.
    #[test]
    fn pareto_front_dominates_only_when_strictly_better() {
        let cells = vec![
            synth_cell("jemalloc", "alpine", 0.7),
            synth_cell("mimalloc", "wolfi", 0.7),
        ];
        let mut image_sizes: BTreeMap<String, f64> = BTreeMap::new();
        image_sizes.insert("alpine".into(), 100.0);
        image_sizes.insert("wolfi".into(), 100.0);

        let front = pareto_front(&cells, &image_sizes);
        assert_eq!(front.len(), 2);
        assert!(front.contains(&("jemalloc".to_string(), "alpine".to_string())));
        assert!(front.contains(&("mimalloc".to_string(), "wolfi".to_string())));
    }

    /// POLAR-05 edge case: empty input returns an empty `BTreeSet` —
    /// no panic, no allocation churn.
    #[test]
    fn pareto_front_empty_input_returns_empty_set() {
        let front = pareto_front(&[], &BTreeMap::new());
        assert!(front.is_empty(), "expected empty set, got {front:?}");
    }

    /// POLAR-05: the return type is `BTreeSet<(String, String)>` so
    /// iteration is alphabetical — preserves byte-identical-output
    /// discipline (CLAUDE.md Conventions). Three on-front cells in
    /// unsorted insertion order; the iter().collect() result must be
    /// sorted ascending by (alloc, env).
    #[test]
    fn pareto_front_returns_btreeset_for_byte_identical_iteration() {
        // Insertion order is (z, m, a) — but BTreeSet iter must yield
        // (a, m, z). All cells have the SAME image_size → strict
        // domination never triggers, so all 3 stay on the front.
        let cells = vec![
            synth_cell("z-alloc", "z-env", 0.5),
            synth_cell("m-alloc", "m-env", 0.5),
            synth_cell("a-alloc", "a-env", 0.5),
        ];
        let mut image_sizes: BTreeMap<String, f64> = BTreeMap::new();
        image_sizes.insert("z-env".into(), 100.0);
        image_sizes.insert("m-env".into(), 100.0);
        image_sizes.insert("a-env".into(), 100.0);

        let front = pareto_front(&cells, &image_sizes);
        assert_eq!(front.len(), 3);
        let collected: Vec<(String, String)> = front.iter().cloned().collect();
        let mut expected = collected.clone();
        expected.sort();
        assert_eq!(
            collected, expected,
            "BTreeSet iteration must be alphabetically sorted"
        );
        // Spot-check the first element.
        assert_eq!(collected[0], ("a-alloc".to_string(), "a-env".to_string()));
        assert_eq!(collected[2], ("z-alloc".to_string(), "z-env".to_string()));
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

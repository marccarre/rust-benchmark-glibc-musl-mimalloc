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

use std::collections::BTreeMap;

use crate::axes::Direction;

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

/// p10/p90 winsorize → direction-aware min-max → 0..=100.
///
/// RED stub: returns an empty `Vec` regardless of input. The Task 1 GREEN
/// commit replaces this with the real algorithm.
pub fn normalize_axis(_values: &[f64], _direction: Direction) -> Vec<f64> {
    Vec::new()
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
}

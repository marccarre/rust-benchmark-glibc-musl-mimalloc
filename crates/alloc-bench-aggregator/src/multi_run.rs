//! Multi-run statistics module — CONTEXT.md D-11 / D-12.
//!
//! Pure-stdlib computation of multi-run summary statistics over a set of
//! `&[f64]` samples drawn from the `(alloc, env, scenario)` 3-tuple group
//! of `Run` records. Used by the aggregator to decorate REPORT.md /
//! `index.html` cells with median, range, and a high-variance warning when
//! coefficient-of-variation crosses a threshold.
//!
//! Decision pointers:
//! - **D-11** (PHASE 5 CONTEXT.md): swap mean → median for central tendency
//!   when ≥3 runs share `(alloc, env, scenario)`; surface `(min..max)` range
//!   alongside the median.
//! - **D-12** (PHASE 5 CONTEXT.md): high-variance flag — CV > 10%.
//! - **RESEARCH §Pattern 5: Multi-run statistics in Rust** — canonical 80-LOC
//!   skeleton for this module (function signatures, formula, test names).
//! - **RESEARCH §Pitfall 7: Sample stddev confusion at n=3** — Bessel-
//!   corrected (n-1) denominator is the unbiased sample-stddev convention;
//!   population stddev (n denominator) underestimates true variance at small
//!   n. The 10% threshold (D-12) was chosen with the Bessel-corrected number
//!   in mind.
//!
//! Dependency contract: pure-stdlib + `serde::Serialize` (already in the
//! aggregator's workspace deps via `serde = { workspace = true }`). No
//! `statrs`, no `nalgebra`, no `ndarray` — keeps Phase 5 dep-free per
//! RESEARCH §"Package Legitimacy Audit".
//!
//! Unit-test gates (6 total — all enforced by `cargo test --lib
//! multi_run::tests`):
//! - `three_identical_samples_have_zero_variance` — stddev=0, CV=Some(0%).
//! - `three_seeds_with_known_cv` — `[100,110,105]` → median=105, Bessel
//!   stddev=5.0, CV ≈ 4.76% (golden value pinning the formula version).
//! - `high_variance_flagged_when_cv_above_10pct` — `[100,130,90]` → CV
//!   ≈ 19.5% → `is_high_variance` returns true.
//! - `cv_undefined_when_mean_is_zero` — `[0,0,0]` → cv_pct=None.
//! - `rejects_nan_input` — `[100, NaN, 105]` → returns None.
//! - `requires_at_least_two_samples` — `[100]` → returns None.

use serde::Serialize;

/// Multi-run statistics across ≥2 runs of the same `(alloc, env, scenario)`
/// tuple. Stored only in the aggregator's REPORT.md / HTML output —
/// NEVER in the v1 JSON schema (CONTEXT.md D-14 / D-20).
///
/// Plan 03 wires `MultiRunStats`, `aggregate`, and `is_high_variance` into
/// markdown.rs (per-scenario throughput-cell decoration + central tendency
/// for winner-picking), recommend.rs (median-with-mean-fallback central
/// tendency), and html.rs (Plotly `error_y` whiskers + ⚠ high variance
/// legend label).
#[derive(Debug, Clone, Serialize)]
pub struct MultiRunStats {
    pub n: usize,
    pub mean: f64,
    pub median: f64,
    pub min: f64,
    pub max: f64,
    /// Sample standard deviation (Bessel-corrected, n-1 denominator).
    pub stddev: f64,
    /// Coefficient of variation as a percentage: stddev / mean × 100.
    /// `None` when mean is 0 or non-finite (CV is undefined).
    pub cv_pct: Option<f64>,
}

/// Compute multi-run statistics. Returns `None` if `samples` has fewer than
/// 2 values (sample stddev requires n ≥ 2) or if any sample is non-finite
/// (NaN-poisoning guard — see `<threat_model>` T-05-03 in PLAN.md).
pub fn aggregate(samples: &[f64]) -> Option<MultiRunStats> {
    if samples.len() < 2 || samples.iter().any(|x| !x.is_finite()) {
        return None;
    }
    let n = samples.len();
    let n_f = n as f64;
    let mean = samples.iter().sum::<f64>() / n_f;

    // Bessel-corrected sample stddev — see RESEARCH §Pitfall 7.
    let variance = samples
        .iter()
        .map(|x| {
            let d = x - mean;
            d * d
        })
        .sum::<f64>()
        / (n_f - 1.0);
    let stddev = variance.sqrt();

    // Median via sort + middle index.
    let mut sorted: Vec<f64> = samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    };

    let min = sorted[0];
    let max = sorted[n - 1];

    // CV is undefined when mean is zero or non-finite — Wikipedia near-zero
    // edge case ("approaches infinity, sensitive to small changes in mean").
    let cv_pct = if mean.abs() > 1e-9 && mean.is_finite() {
        Some((stddev / mean) * 100.0)
    } else {
        None
    };

    Some(MultiRunStats {
        n,
        mean,
        median,
        min,
        max,
        stddev,
        cv_pct,
    })
}

/// CONTEXT.md D-12: high-variance flag — CV > 10%.
pub fn is_high_variance(stats: &MultiRunStats) -> bool {
    matches!(stats.cv_pct, Some(cv) if cv > 10.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_identical_samples_have_zero_variance() {
        let stats = aggregate(&[100.0, 100.0, 100.0]).unwrap();
        assert_eq!(stats.median, 100.0);
        assert_eq!(stats.stddev, 0.0);
        assert_eq!(stats.cv_pct, Some(0.0));
        assert!(!is_high_variance(&stats));
    }

    #[test]
    fn three_seeds_with_known_cv() {
        // Throughput samples: 100, 110, 105. mean = 105, sample stddev (n-1) =
        // sqrt(((100-105)^2 + (110-105)^2 + (105-105)^2) / 2) = sqrt(50/2) = 5.0.
        // CV = 5.0 / 105.0 × 100 ≈ 4.76%.
        let stats = aggregate(&[100.0, 110.0, 105.0]).unwrap();
        assert!((stats.median - 105.0).abs() < 1e-9);
        assert!((stats.stddev - 5.0).abs() < 1e-9);
        let cv = stats.cv_pct.expect("cv defined");
        assert!((cv - 4.7619).abs() < 1e-3);
        assert!(!is_high_variance(&stats));
    }

    #[test]
    fn high_variance_flagged_when_cv_above_10pct() {
        // 100, 130, 90 → mean 106.67, stddev 20.82, CV ≈ 19.5%
        let stats = aggregate(&[100.0, 130.0, 90.0]).unwrap();
        assert!(is_high_variance(&stats));
    }

    #[test]
    fn cv_undefined_when_mean_is_zero() {
        let stats = aggregate(&[0.0, 0.0, 0.0]).unwrap();
        assert_eq!(stats.cv_pct, None);
        assert!(!is_high_variance(&stats));
    }

    #[test]
    fn rejects_nan_input() {
        assert!(aggregate(&[100.0, f64::NAN, 105.0]).is_none());
    }

    #[test]
    fn requires_at_least_two_samples() {
        assert!(aggregate(&[100.0]).is_none());
    }
}

//! Phase 9 / POLAR-01..04 — server-side `scatterpolar` trace JSON builder
//! for the spider chart small-multiples grid in `index.html.tmpl`. Pure data
//! transform; no IO; no rendering. Sibling to `score.rs`.
//!
//! Public surface (consumed by `html.rs` in Plan 09-03):
//!   - `pub fn build_trace(score: &CellScore) -> serde_json::Value`
//!     POLAR-01 / POLAR-02: returns a `scatterpolar` trace with 9-element
//!     `r` / `theta` arrays where `r[0] == r[8]` and `theta[0] == theta[8]`
//!     (polygon closure). `type: "scatterpolar"`, `fill: "toself"`,
//!     `name: "{alloc}/{env}"`.
//!   - `pub fn build_reference_trace(scores: &[CellScore]) -> serde_json::Value`
//!     POLAR-04: matrix-mean reference polygon at 25% alpha
//!     (`fillcolor: "rgba(128,128,128,0.25)"`, `line.color:
//!     "rgba(128,128,128,0.5)"`). Hard-coded `name: "Matrix mean (n=18)"`
//!     per UI-SPEC §Trace data array — the matrix is locked at 18 cells
//!     by CLAUDE.md cross-libc rejection.
//!   - `pub fn axis_label_for_chart(spec: &AxisSpec) -> String`
//!     POLAR-03: appends the exact 11-byte ` (heuristic)` suffix (single
//!     leading U+0020 space) for `image_size_efficiency` and
//!     `security_posture`; returns the plain `spec.label` for the other
//!     six measured axes.
//!
//! Locked invariants:
//!   - `MEASUREMENT_AXES` iteration is `.iter()` on the const array (NOT a
//!     collected `Vec`) — preserves Phase 7's summation-order discipline so
//!     the spider trace's r/theta indices line up with composite-score
//!     iteration. Index 8 always closes the polygon by repeating index 0.
//!   - Phase 6's `MEASUREMENT_AXES` registry is NOT mutated — `(heuristic)`
//!     suffix is a render-time decoration only.
//!   - JSON shape: `serde_json::json!({...})` macro (NOT a typed
//!     `PolarTrace` struct) per CONTEXT.md — tinytemplate consumes the
//!     pre-serialized JSON string, not a typed value.

use serde_json::{json, Value};

use crate::axes::{AxisSpec, MEASUREMENT_AXES};
use crate::score::CellScore;

/// Render-time decoration that appends ` (heuristic)` (exact 11-byte suffix
/// with a single leading U+0020 space) to the labels of the two heuristic
/// axes — `image_size_efficiency` and `security_posture` — and returns the
/// plain registry label for the other six measured axes. POLAR-03.
pub fn axis_label_for_chart(spec: &AxisSpec) -> String {
    if spec.is_heuristic {
        format!("{} (heuristic)", spec.label)
    } else {
        spec.label.to_string()
    }
}

/// Build a `scatterpolar` trace JSON value for one `CellScore`. POLAR-01 +
/// POLAR-02: 9-element `r` / `theta` arrays with `r[0] == r[8]` and
/// `theta[0] == theta[8]` (polygon closure). The first 8 indices traverse
/// `MEASUREMENT_AXES` in constant alphabetical-by-key order (Phase 6 lock);
/// index 8 repeats index 0.
///
/// Missing axis values fall back to `0.0` (mirroring `score::score_cells`'
/// `cell.axes.get(spec.key).copied().unwrap_or(0.0)` pattern). Phase 7's
/// `compute_axes` already guarantees every cell carries all 8 keys; this
/// fallback is a defensive no-panic guard, not the expected path.
pub fn build_trace(score: &CellScore) -> Value {
    let mut r: Vec<f64> = MEASUREMENT_AXES
        .iter()
        .map(|spec| score.axes.get(spec.key).copied().unwrap_or(0.0))
        .collect();
    let mut theta: Vec<String> = MEASUREMENT_AXES
        .iter()
        .map(axis_label_for_chart)
        .collect();
    // POLAR-02 polygon closure: repeat index 0 at index 8.
    r.push(r[0]);
    theta.push(theta[0].clone());

    json!({
        "type": "scatterpolar",
        "r": r,
        "theta": theta,
        "fill": "toself",
        "name": format!("{}/{}", score.alloc, score.env),
        "opacity": 1.0,
    })
}

/// Build the matrix-mean reference polygon at 25% fill / 50% stroke alpha.
/// POLAR-04: averages each axis across the input scores; renders with the
/// hard-coded literal `"Matrix mean (n=18)"` name (the matrix is locked at
/// 18 cells by CLAUDE.md cross-libc rejection — do NOT interpolate
/// `scores.len()` into the name string). Empty input → 9-element zero
/// arrays (degenerate dot at origin; still closes the polygon).
pub fn build_reference_trace(scores: &[CellScore]) -> Value {
    let n = scores.len();
    let mut r: Vec<f64> = MEASUREMENT_AXES
        .iter()
        .map(|spec| {
            if n == 0 {
                0.0
            } else {
                let sum: f64 = scores
                    .iter()
                    .map(|s| s.axes.get(spec.key).copied().unwrap_or(0.0))
                    .sum();
                sum / n as f64
            }
        })
        .collect();
    let mut theta: Vec<String> = MEASUREMENT_AXES
        .iter()
        .map(axis_label_for_chart)
        .collect();
    // POLAR-02 polygon closure (also applies to the reference polygon).
    r.push(r[0]);
    theta.push(theta[0].clone());

    json!({
        "type": "scatterpolar",
        "r": r,
        "theta": theta,
        "fill": "toself",
        "fillcolor": "rgba(128,128,128,0.25)",
        "line": { "color": "rgba(128,128,128,0.5)" },
        "name": "Matrix mean (n=18)",
        "opacity": 0.25,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// Synth helper: build a `CellScore` with all 8 axis keys populated
    /// from a fixed `[f64; 8]` ordered alphabetically by key (matching
    /// `MEASUREMENT_AXES`). Mirrors the `score::tests::synth_cell_axes_keyed`
    /// pattern.
    fn synth_score(alloc: &str, env: &str, vals: [f64; 8]) -> CellScore {
        let mut axes: BTreeMap<&'static str, f64> = BTreeMap::new();
        for (spec, v) in MEASUREMENT_AXES.iter().zip(vals.iter()) {
            axes.insert(spec.key, *v);
        }
        CellScore {
            alloc: alloc.into(),
            env: env.into(),
            composite: 0.0,
            axes,
        }
    }

    /// POLAR-03: `image_size_efficiency` and `security_posture` get the
    /// exact 11-byte ` (heuristic)` suffix appended to their registry
    /// labels. (Indices 2 and 6 of the alphabetical `MEASUREMENT_AXES`.)
    #[test]
    fn axis_label_for_chart_appends_heuristic_suffix_for_image_size_efficiency_and_security_posture() {
        // Index 2 is `image_size_efficiency`.
        let img = &MEASUREMENT_AXES[2];
        assert_eq!(img.key, "image_size_efficiency");
        assert!(img.is_heuristic);
        assert_eq!(
            axis_label_for_chart(img),
            format!("{} (heuristic)", img.label)
        );

        // Index 6 is `security_posture`.
        let sec = &MEASUREMENT_AXES[6];
        assert_eq!(sec.key, "security_posture");
        assert!(sec.is_heuristic);
        assert_eq!(
            axis_label_for_chart(sec),
            format!("{} (heuristic)", sec.label)
        );
    }

    /// POLAR-03 negative case: the six measured axes return their plain
    /// registry labels with NO suffix appended. Bool-flip safety net per
    /// CONTEXT.md "Heuristic test".
    #[test]
    fn axis_label_for_chart_returns_plain_label_for_real_measurement_axes() {
        for spec in MEASUREMENT_AXES.iter().filter(|s| !s.is_heuristic) {
            let got = axis_label_for_chart(spec);
            assert_eq!(
                got, spec.label,
                "non-heuristic axis {} must render plain label, got {:?}",
                spec.key, got
            );
            assert!(
                !got.contains("(heuristic)"),
                "non-heuristic axis {} must NOT contain '(heuristic)', got {:?}",
                spec.key,
                got
            );
        }
    }

    /// POLAR-03 ordering safety net: iterating `MEASUREMENT_AXES.iter()` and
    /// collecting `axis_label_for_chart(spec)` produces a `Vec<String>` of
    /// length 8 where indices 2 and 6 carry ` (heuristic)` and the other
    /// six do not — flips the bool on either side breaks the assertion.
    #[test]
    fn axis_label_for_chart_handles_all_eight_axes_in_constant_order() {
        let labels: Vec<String> = MEASUREMENT_AXES
            .iter()
            .map(axis_label_for_chart)
            .collect();
        assert_eq!(labels.len(), 8);
        for (i, label) in labels.iter().enumerate() {
            let has_suffix = label.contains(" (heuristic)");
            let expected = i == 2 || i == 6;
            assert_eq!(
                has_suffix, expected,
                "index {i} ({}): suffix presence mismatch — got {has_suffix}, expected {expected} (label {:?})",
                MEASUREMENT_AXES[i].key, label,
            );
        }
    }

    // ---------- Task 2: build_trace tests (POLAR-01 / POLAR-02) ----------

    /// POLAR-01 + POLAR-02 verbatim: `build_trace` returns a `scatterpolar`
    /// trace with 9-element `r` / `theta` arrays where `r[0] == r[8]` and
    /// `theta[0] == theta[8]` (polygon closure). `type == "scatterpolar"`,
    /// `fill == "toself"`. This is the canonical lock for the trace shape.
    #[test]
    fn trace_closes_polygon_with_9_elements() {
        let score = synth_score(
            "jemalloc",
            "alpine",
            [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
        );
        let trace = build_trace(&score);

        let r = trace["r"].as_array().expect("r is an array");
        let theta = trace["theta"].as_array().expect("theta is an array");
        assert_eq!(r.len(), 9, "r must have 9 elements (8 axes + closure)");
        assert_eq!(
            theta.len(),
            9,
            "theta must have 9 elements (8 axes + closure)"
        );
        // Polygon closure: index 0 == index 8 for both arrays.
        assert_eq!(r[0], r[8], "polygon closure: r[0] must equal r[8]");
        assert_eq!(
            theta[0], theta[8],
            "polygon closure: theta[0] must equal theta[8]"
        );
        // Trace type and fill must match POLAR-01.
        assert_eq!(trace["type"].as_str(), Some("scatterpolar"));
        assert_eq!(trace["fill"].as_str(), Some("toself"));
    }

    /// `build_trace` carries the `name: "{alloc}/{env}"` per-cell title used
    /// by the small-multiples grid in `index.html.tmpl`.
    #[test]
    fn trace_carries_alloc_env_name_field() {
        let score = synth_score(
            "mimalloc",
            "debian-slim",
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        );
        let trace = build_trace(&score);
        assert_eq!(trace["name"].as_str(), Some("mimalloc/debian-slim"));
    }

    /// POLAR-03 + POLAR-02: the `theta` array carries `axis_label_for_chart`
    /// output for each axis. Indices 2 and 6 carry the ` (heuristic)` suffix
    /// (image_size_efficiency, security_posture); the other six carry plain
    /// labels. Index 8 equals index 0 (polygon closure).
    #[test]
    fn trace_uses_axis_label_for_chart_for_theta() {
        let score = synth_score(
            "ptmalloc",
            "wolfi",
            [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
        );
        let trace = build_trace(&score);
        let theta = trace["theta"].as_array().expect("theta is an array");
        assert_eq!(theta.len(), 9);

        // Index 2 (image_size_efficiency) carries ` (heuristic)` suffix.
        assert!(
            theta[2]
                .as_str()
                .expect("theta[2] is string")
                .contains(" (heuristic)"),
            "theta[2] must contain ' (heuristic)' suffix; got {:?}",
            theta[2]
        );
        // Index 6 (security_posture) carries ` (heuristic)` suffix.
        assert!(
            theta[6]
                .as_str()
                .expect("theta[6] is string")
                .contains(" (heuristic)"),
            "theta[6] must contain ' (heuristic)' suffix; got {:?}",
            theta[6]
        );
        // Indices 0, 1, 3, 4, 5, 7 are plain (no suffix).
        for i in [0usize, 1, 3, 4, 5, 7] {
            let s = theta[i].as_str().expect("theta[i] is string");
            assert!(
                !s.contains("(heuristic)"),
                "theta[{i}] must NOT contain '(heuristic)'; got {:?}",
                s
            );
        }
        // Polygon closure: theta[0] == theta[8].
        assert_eq!(theta[0], theta[8]);
    }

    // ---------- Task 2: build_reference_trace tests (POLAR-04) ----------

    /// POLAR-04: the matrix-mean reference trace carries the locked alpha
    /// literals — `fillcolor: "rgba(128,128,128,0.25)"` (25% fill) and
    /// `line.color: "rgba(128,128,128,0.5)"` (50% stroke) — plus the
    /// hard-coded `name: "Matrix mean (n=18)"` per UI-SPEC §Trace data array.
    #[test]
    fn reference_trace_carries_25_percent_alpha_fill_and_50_percent_alpha_stroke() {
        let scores = vec![
            synth_score(
                "jemalloc",
                "alpine",
                [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            ),
            synth_score(
                "mimalloc",
                "wolfi",
                [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5],
            ),
            synth_score(
                "ptmalloc",
                "debian-slim",
                [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
            ),
        ];
        let trace = build_reference_trace(&scores);

        assert_eq!(
            trace["fillcolor"].as_str(),
            Some("rgba(128,128,128,0.25)"),
            "POLAR-04: fillcolor literal locked at 25% alpha"
        );
        assert_eq!(
            trace["line"]["color"].as_str(),
            Some("rgba(128,128,128,0.5)"),
            "POLAR-04: line.color literal locked at 50% alpha"
        );
        assert_eq!(
            trace["name"].as_str(),
            Some("Matrix mean (n=18)"),
            "POLAR-04: name literal hard-coded — matrix locked at 18 cells",
        );
        assert_eq!(trace["fill"].as_str(), Some("toself"));
        assert_eq!(trace["type"].as_str(), Some("scatterpolar"));
    }

    /// POLAR-04: each `r[i]` is the mean across all input scores for the
    /// i-th axis in `MEASUREMENT_AXES`. With three CellScores at axis values
    /// [0.0, 0.5, 1.0], the mean is 0.5 for every axis. Polygon closure:
    /// `r[8] == r[0]`.
    #[test]
    fn reference_trace_averages_each_axis_across_input_scores() {
        let scores = vec![
            synth_score("a", "x", [0.0; 8]),
            synth_score("b", "y", [0.5; 8]),
            synth_score("c", "z", [1.0; 8]),
        ];
        let trace = build_reference_trace(&scores);
        let r = trace["r"].as_array().expect("r is an array");
        assert_eq!(r.len(), 9, "9 elements (8 axes + closure)");
        for i in 0..8usize {
            let v = r[i].as_f64().expect("r[i] is float");
            assert!(
                (v - 0.5).abs() < 1e-9,
                "r[{i}] must be mean(0.0, 0.5, 1.0) = 0.5; got {v}"
            );
        }
        // Polygon closure.
        assert_eq!(r[0], r[8], "r[0] must equal r[8] for polygon closure");
    }

    /// Edge case: empty input → 9-element zero `r` array and 9-element
    /// `theta` array. Still closes the polygon (renders a degenerate dot
    /// at origin; never panics on empty matrix).
    #[test]
    fn reference_trace_returns_zeros_when_input_empty() {
        let trace = build_reference_trace(&[]);
        let r = trace["r"].as_array().expect("r is an array");
        let theta = trace["theta"].as_array().expect("theta is an array");
        assert_eq!(r.len(), 9, "r must have 9 elements even when empty");
        assert_eq!(theta.len(), 9, "theta must have 9 elements even when empty");
        for (i, v) in r.iter().enumerate() {
            let f = v.as_f64().expect("r[i] is float");
            assert_eq!(f, 0.0, "r[{i}] must be 0.0 when input is empty; got {f}");
        }
        // Polygon closure still holds.
        assert_eq!(r[0], r[8]);
        assert_eq!(theta[0], theta[8]);
    }
}

//! Phase 9 / POLAR-01..04 — server-side `scatterpolar` trace JSON builder
//! for the spider chart small-multiples grid in `index.html.tmpl`. Pure data
//! transform; no IO; no rendering. Sibling to `score.rs`.
//!
//! Public surface (consumed by `html.rs` in Plan 09-03):
//!   - `pub fn build_trace(score: &CellScore) -> serde_json::Value`
//!     POLAR-01 / POLAR-02: returns a `scatterpolar` trace with 9-element
//!     `r` / `theta` arrays where `r[0] == r[8]` and `theta[0] == theta[8]`
//!     (polygon closure). `type: "scatterpolar"`, `fill: "toself"`,
//!     `name: "{alloc}/{env}"`. (Lands in Plan 09-01 Task 2.)
//!   - `pub fn build_reference_trace(scores: &[CellScore]) -> serde_json::Value`
//!     POLAR-04: matrix-mean reference polygon at 25% alpha
//!     (`fillcolor: "rgba(128,128,128,0.25)"`, `line.color:
//!     "rgba(128,128,128,0.5)"`). Hard-coded `name: "Matrix mean (n=18)"`
//!     per UI-SPEC §Trace data array — the matrix is locked at 18 cells
//!     by CLAUDE.md cross-libc rejection. (Lands in Plan 09-01 Task 2.)
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

use crate::axes::AxisSpec;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::MEASUREMENT_AXES;

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
}

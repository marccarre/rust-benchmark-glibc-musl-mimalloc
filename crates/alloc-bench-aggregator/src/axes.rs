//! Phase 6 / AXES-01 / AXES-02 — locked compile-time registry of the 8
//! measurement axes used by the spider chart and direction-marker columns.
//!
//! Single source of truth for direction-marker glyphs across `score.rs`
//! (Phase 7), `polar.rs` (Phase 9), and `markdown.rs` (Phase 10). Downstream
//! phases consume `MEASUREMENT_AXES` to drive normalization, spider geometry,
//! and column-header `↑` / `↓` glyphs (with a `(heuristic)` suffix and dashed
//! gridline whenever `is_heuristic == true` per POLAR-03 / DIR-01).
//!
//! Locked decisions (06-CONTEXT §Registry Architecture):
//!   - `MEASUREMENT_AXES` is a `pub const` array — NOT `lazy_static!` /
//!     `OnceCell` / `once_cell::sync::Lazy`. Compile-time only; zero runtime
//!     cost.
//!   - `Direction::arrow(self) -> char` is a `const fn` returning hard-coded
//!     Unicode literals `'\u{2191}'` (U+2191 UPWARDS ARROW) and `'\u{2193}'`
//!     (U+2193 DOWNWARDS ARROW). No external `unicode-arrows` crate.
//!   - `AxisSpec` carries no `weight_hint` field — V12-05 / V12-07 deferred;
//!     all 8 axes contribute equally to the composite score in Phase 7.
//!   - Array order is alphabetical by `key`; uniqueness, length, alphabetical
//!     order, heuristic-flag set, and arrow glyphs are gated by the unit
//!     tests below (T-06-01-T mitigation).

/// Whether higher or lower raw values represent better allocator behaviour
/// on a given axis. Drives normalization sign in `score.rs` (Phase 7) and
/// the `↑` / `↓` glyph in column headers (Phase 10) and spider legends
/// (Phase 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Higher,
    Lower,
}

impl Direction {
    /// The Unicode arrow glyph that decorates this direction in column
    /// headers and spider legends. `const fn` per CONTEXT lock — callers
    /// in Phase 9 / 10 use this in `const`-evaluated contexts.
    pub const fn arrow(self) -> char {
        match self {
            Direction::Higher => '\u{2191}',
            Direction::Lower => '\u{2193}',
        }
    }
}

/// One axis of the 8-axis measurement registry. All four fields are
/// `Copy`-friendly so the whole struct is `Copy + Clone + Debug` — this
/// lets downstream consumers iterate `MEASUREMENT_AXES` by value without
/// borrow ceremony.
#[derive(Debug, Clone, Copy)]
pub struct AxisSpec {
    /// Alphabetical sort key — also the JSON / template field name used by
    /// `polar.rs` (Phase 9) and `markdown.rs` (Phase 10).
    pub key: &'static str,
    /// Human-facing label rendered in the spider legend (Phase 9) and the
    /// REPORT.md column header (Phase 10).
    pub label: &'static str,
    /// Whether higher or lower raw values are better (drives `↑` / `↓`).
    pub direction: Direction,
    /// `true` → `(heuristic)` suffix + dashed gridline (POLAR-03, DIR-01).
    /// `false` → measured-value axis (solid gridline, no suffix).
    pub is_heuristic: bool,
}

/// The 8 measurement axes, alphabetical by `key`. Length, alphabetical
/// ordering, key uniqueness, the heuristic-flag set, and the arrow glyphs
/// are gated by the unit tests in this module (mitigation T-06-01-T).
pub const MEASUREMENT_AXES: [AxisSpec; 8] = [
    AxisSpec { key: "channel_throughput",     label: "Channel throughput",     direction: Direction::Higher, is_heuristic: false },
    AxisSpec { key: "cpu_bound_throughput",   label: "CPU-bound throughput",   direction: Direction::Higher, is_heuristic: false },
    AxisSpec { key: "image_size_efficiency",  label: "Image-size efficiency",  direction: Direction::Higher, is_heuristic: true  },
    AxisSpec { key: "memory_fragmentation",   label: "Memory / fragmentation", direction: Direction::Lower,  is_heuristic: false },
    AxisSpec { key: "multithread_throughput", label: "Multithread throughput", direction: Direction::Higher, is_heuristic: false },
    AxisSpec { key: "resilience",             label: "Resilience",             direction: Direction::Higher, is_heuristic: false },
    AxisSpec { key: "security_posture",       label: "Security posture",       direction: Direction::Higher, is_heuristic: true  },
    AxisSpec { key: "web_throughput",         label: "Web throughput",         direction: Direction::Higher, is_heuristic: false },
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// The 8-axis spider shape is locked: any reorder/add/remove breaks
    /// every downstream consumer (Phase 7 score, Phase 9 polar, Phase 10
    /// markdown headers). Mitigation T-06-01-T.
    #[test]
    fn axes_count_is_exactly_eight() {
        assert_eq!(
            MEASUREMENT_AXES.len(),
            8,
            "the 8-axis spider shape is locked"
        );
    }

    /// Byte-identical-output discipline (CLAUDE.md): consumers iterate the
    /// registry in declaration order and assume alphabetical key ordering.
    /// Sort the cloned key vector and compare for equality — any drift
    /// trips the assertion.
    #[test]
    fn axes_keys_are_alphabetically_sorted() {
        let keys: Vec<&str> = MEASUREMENT_AXES.iter().map(|a| a.key).collect();
        let mut sorted = keys.clone();
        sorted.sort_unstable();
        assert_eq!(
            keys, sorted,
            "MEASUREMENT_AXES must be sorted by `key`"
        );
    }

    /// `BTreeSet` (NOT `HashSet`) per CLAUDE.md byte-identical-iteration
    /// discipline — applies even in tests. Duplicate keys would silently
    /// collapse downstream lookups.
    #[test]
    fn axes_keys_are_unique() {
        let unique: BTreeSet<&str> = MEASUREMENT_AXES.iter().map(|a| a.key).collect();
        assert_eq!(
            unique.len(),
            MEASUREMENT_AXES.len(),
            "duplicate axis keys"
        );
    }

    /// Phase 9 (polar.rs) and Phase 10 (markdown.rs) hard-code these glyphs
    /// in column headers and spider legends. The `const fn` contract means
    /// any drift fails at compile time for downstream `const`-evaluated
    /// contexts as well as at this unit test.
    #[test]
    fn arrow_glyphs_match_unicode_literals() {
        assert_eq!(Direction::Higher.arrow(), '\u{2191}');
        assert_eq!(Direction::Lower.arrow(), '\u{2193}');
    }

    /// POLAR-03 / DIR-01: only `image_size_efficiency` and
    /// `security_posture` are heuristic axes (rendered with `(heuristic)`
    /// suffix + dashed gridline). All six other axes are measured.
    #[test]
    fn heuristic_axes_are_image_size_and_security() {
        let heuristic_keys: Vec<&str> = MEASUREMENT_AXES
            .iter()
            .filter(|a| a.is_heuristic)
            .map(|a| a.key)
            .collect();
        assert_eq!(
            heuristic_keys,
            vec!["image_size_efficiency", "security_posture"]
        );
    }
}

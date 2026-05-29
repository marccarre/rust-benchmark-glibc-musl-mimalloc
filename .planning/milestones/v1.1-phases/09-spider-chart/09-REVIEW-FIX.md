---
phase: 09-spider-chart
fixed_at: 2026-05-28T12:55:00Z
review_path: .planning/phases/09-spider-chart/09-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 9: Code Review Fix Report

**Fixed at:** 2026-05-28T12:55:00Z
**Source review:** `.planning/phases/09-spider-chart/09-REVIEW.md`
**Iteration:** 1

**Summary:**
- Findings in scope: 5 (1 Critical + 4 Warning; IN-* findings out of scope)
- Fixed: 5
- Skipped: 0
- Workspace test suite: `cargo test --workspace` passes — 248/248 tests
  green (131 + 30 + 3 + 1 + 1 + 81 + 1, plus 1 ignored doc-test).

## Fixed Issues

### CR-01: Suspect-threshold split-brain — inline JS uses obsolete `< 10000` while Rust uses `< 1_000`

**Files modified:**
- `crates/alloc-bench-aggregator/templates/index.html.tmpl`
- `crates/alloc-bench-aggregator/src/html.rs`

**Commit:** `06bd85f`
**Applied fix:** Updated all three `10000` references in `index.html.tmpl`
(line 325 comment, line 338 `function isSuspect`, line 853 `low` check in
`renderReportMirrorTable`) to mirror the canonical D-07 threshold
`< 1_000`. Added `template_has_no_obsolete_10000_samples_threshold`
regression test in `html.rs::tests` that scans the entire `TEMPLATE`
constant for the literal `10000` substring — a future drift back to the
obsolete cutoff trips at `cargo test` time. The cross-surface
byte-identity contract documented in `html.rs:9-13` ("the report and the
dashboard agree on which runs are flagged") now holds for any run with
`1_000 ≤ samples_count < 10_000`.

### WR-01: Hard-coded `Matrix mean (n=18)` literal — silently lies on partial inputs

**File modified:** `crates/alloc-bench-aggregator/src/polar.rs`
**Commit:** `b2ab7a9`
**Applied fix:** Replaced the hard-coded `name: "Matrix mean (n=18)"`
literal in `build_reference_trace` with `name: format!("Matrix mean
(n={n})")` where `n = scores.len()`. The doc-comments at lines 11-19
and 86-89 (now 104-116) were updated to document interpolation as the
canonical contract, with a note that the production rendering of
`"Matrix mean (n=18)"` is now an emergent property of the locked 18-cell
input rather than a hard-coded literal. The existing test fixture
`reference_trace_carries_25_percent_alpha_fill_and_50_percent_alpha_stroke`
asserts `n=3` (matching the 3 scores it feeds), and a new degenerate
test `reference_trace_name_interpolates_zero_for_empty_input` pins the
`Matrix mean (n=0)` boundary so a future regression that re-hard-codes
the literal trips at `cargo test` time.

### WR-02: NaN-unsafe HashMap iteration in `build_image_sizes`

**File modified:** `crates/alloc-bench-aggregator/src/main.rs`
**Commit:** `70cdc3a`
**Applied fix:** `build_image_sizes` now (1) sorts the `metas` keys
alphabetically before reducing per-env so iteration order is stable
across runs (CLAUDE.md Conventions §"Byte-identical-output discipline":
"alphabetical iteration via `BTreeMap` / `BTreeSet` (never `HashMap` /
`HashSet`)"), and (2) rejects non-finite `image_size_mb` values
(NaN/+inf/-inf) explicitly via `meta.image_size_mb.is_finite()`,
preventing a poisoned meta sidecar from propagating NaN into
`score::pareto_front` (where NaN-vs-finite comparisons silently inflate
front membership). Added `image_sizes_rejects_non_finite_values` test
feeding a NaN, +inf, and finite 52.0 across three envs: alpine keeps the
finite 52.0; wolfi (only NaN meta) is absent; debian-slim (only +inf
meta) is absent. The existing three POLAR-05 tests continue to pass
unchanged.

### WR-03: Pareto front computed on truncated top-N — column header is ambiguous

**File modified:** `crates/alloc-bench-aggregator/src/recommend.rs`
**Commit:** `c4d15c1`
**Applied fix:** Per the review's Option 2 recommendation, switched
`top_n_cells` to compute `score::pareto_front(&scores, image_sizes)` on
the FULL `scores` slice BEFORE truncation, rather than on the truncated
top-N output. The `is_pareto` flag and the `★` glyph in REPORT.md /
HTML now carry "globally on the front" semantics (non-dominated across
the entire 18-cell sweep). Cost: one O(n²) sweep on n=18 (324
comparisons), negligible. This aligns the implementation with
`09-CONTEXT.md §"Pareto-front data flow"` which already specified
`score::pareto_front(&scores, &image_sizes)` (the full slice). Added
`top_n_cells_pareto_front_uses_full_sweep_not_truncated_top_n`
regression test: 12 cells where alloc12/small (composite tied with
alloc09/big, image strictly smaller) is truncated out of the top-10 by
alloc-name tiebreak but still strictly dominates alloc09/big on the
(composite ↑, image ↓) plane. Under the OLD truncated-front semantics
alloc09 would be flagged Pareto; under the NEW full-sweep semantics
alloc09.is_pareto MUST be false. The existing
`top_n_cells_populates_is_pareto_for_pareto_front_cells` test (3 cells,
no truncation) continues to pass — full-sweep equals truncated-sweep
when `scores.len() ≤ TOP_N_TOTAL`.

### WR-04: `axis_label_for_chart` allocates `String` even on non-heuristic path

**File modified:** `crates/alloc-bench-aggregator/src/polar.rs`
**Commit:** `1cb51a5`
**Applied fix:** Changed `axis_label_for_chart` return type from
`String` to `Cow<'static, str>`. Six of the eight measurement axes are
non-heuristic and now return `Cow::Borrowed(spec.label)` (zero
allocation); the two heuristic axes
(`image_size_efficiency`, `security_posture`) continue to return
`Cow::Owned(format!("{} (heuristic)", spec.label))`. The two trace
builders (`build_trace`, `build_reference_trace`) need owned `String`
values for serde_json's `Vec<String>` boundary, so they materialize each
`Cow` via `.into_owned()`. Net effect: heap allocations drop from
8/render to 2/render in those two builders. The ordering test was
updated to collect into `Vec<Cow<'static, str>>` (a one-line change —
`Cow::contains` works transparently via `Deref<Target = str>`); the
other two `axis_label_for_chart` tests use `assert_eq!` against `&str`
or `String` and work transparently via `PartialEq` impls.

---

_Fixed: 2026-05-28T12:55:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_

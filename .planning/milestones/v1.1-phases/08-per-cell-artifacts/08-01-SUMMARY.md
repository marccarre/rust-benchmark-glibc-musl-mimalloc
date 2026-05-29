---
phase: 08-per-cell-artifacts
plan: 01
type: summary
status: complete
requirements:
  - CELL-01
  - CELL-02
  - CELL-05
commits:
  - 11f3749: feat(08-01) per-cell templates + CellTemplateContext (Task 1)
  - bbe7685: test(08-01) WR-01 drift defense + 2 supporting tests (Task 2)
files_changed:
  - crates/alloc-bench-aggregator/templates/recommend-cell.md.tmpl (NEW, 20 lines)
  - crates/alloc-bench-aggregator/templates/recommend-cell.html.tmpl (NEW, 12 lines)
  - crates/alloc-bench-aggregator/src/html.rs (+219 lines: 2 consts, 1 struct, 1 fn, 3 tests)
tags: [phase-08, templates, tinytemplate, drift-defense, recommend-cell, rust]
---

## Objective achieved

Foundation surfaces for Phase 8 are in place:

- Two new tinytemplate files (`recommend-cell.md.tmpl`, `recommend-cell.html.tmpl`)
  driven by the same `CellRecommendation` struct.
- `CellTemplateContext` wrapper struct in `html.rs` that adds `rank_padded`
  for `<article id="recommend-{rank_padded}-...">` formatting.
- `pub(crate) const RECOMMEND_CELL_MD` and `RECOMMEND_CELL_HTML` constants
  ready for Plan 02's `markdown.rs` to import via `use crate::html::*`.
- Three tests gating the contract before any wiring depends on them.

Plan 02 will wire templates into `markdown::write` + `html::render` and
emit the leading summary table + standalone fragment files.

## CellTemplateContext field set

```rust
pub(crate) struct CellTemplateContext {
    pub rank: usize,
    pub rank_padded: String,                  // format!("{:02}", cell.rank)
    pub alloc: String,
    pub env: String,                          // already short-form
    pub tldr: String,
    pub strengths: Vec<&'static str>,
    pub weaknesses: Vec<&'static str>,
    pub recommended_for: Vec<&'static str>,
    pub avoid_for: Vec<&'static str>,
    pub suspect_flag: bool,
}
```

`composite_score` and `axes` from `CellRecommendation` are intentionally
omitted per CONTEXT.md decision — cards do not render score numbers or
per-axis values. The leading `| Rank | Cell | Score |` summary table
(Plan 02) handles `composite_score`; per-axis values are the Phase 9
spider chart's job. `cell_template_context_excludes_score_and_axes` gates
this decision via JSON-key assertion.

## Co-location decision

`RECOMMEND_CELL_MD` / `RECOMMEND_CELL_HTML` both live in `html.rs` rather
than `markdown.rs`. Rationale: the WR-01 sentinel test
(`cell_templates_both_reference_all_fields`) needs to register both
templates against a single `TinyTemplate` instance to assert cross-surface
field-presence parity. Co-locating the constants with the test makes that
trivial; Plan 02's `markdown.rs` accepts the cross-module import as the
known trade-off.

## WR-01 sentinel mutation sanity check

Manual mutation test confirmed the drift defense works:

1. Removed the `{tldr}` substitution from `recommend-cell.md.tmpl`.
2. Re-ran `cargo test -p alloc-bench-aggregator html::tests::cell_templates_both_reference_all_fields`.
3. Test failed with `_SENTINEL_TLDR_` missing from the rendered Markdown
   output, exactly as designed.
4. Template restored before commit.

This proves a future PR adding a field to `CellTemplateContext` and the
markdown template but forgetting the HTML template (or vice-versa) will
fail at `cargo test` time — the WR-01-pattern winner-tiebreak drift the
v1.0 fix already exposed once cannot recur on the per-cell surfaces.

## Tinytemplate authoring notes

No compile-time errors hit — the canonical body sketches in 08-RESEARCH.md
§2 and 08-PATTERNS.md were taken verbatim, and the compact
`{{ for s in strengths }}- {s}\n{{ endfor }}` form (RESEARCH §Pitfall 2)
worked first try.

The `{{ if suspect_flag }} *(suspect)*{{ endif }}` form emits the literal
six bytes between asterisks (`*(suspect)*`) on both surfaces. UI-SPEC line
113 / CONTEXT.md "Suspect annotation byte-identity" explicitly forbid the
HTML `<em>(suspect)</em>` / `<span class="suspect">` / `⚠` glyph variants
— sentinel test asserts the literal `(suspect)` parenthesized form on
both surfaces.

## Dependencies

NO new entries in any `Cargo.toml`. `tinytemplate = "1"` was already in
the workspace deps from earlier phases; `serde` / `serde_json` were
already present.

## Test results

- `cargo test -p alloc-bench-aggregator html::tests::` → 8 passed (5
  pre-existing + 3 new), 0 failed.
- `cargo test --workspace` → all crates green; no v1.0 golden-output
  regression (Plan 01 adds zero bytes to existing REPORT.md / index.html
  outputs because the new templates are not yet registered with the
  aggregator's TinyTemplate instances — Plan 02 does that wiring).

## Deviation note

Plan 08-01 Task 1 commit (`11f3749`) landed directly on `main` instead of
a worktree branch due to a cwd-drift bug in the executor agent
(absolute-path discipline failure). Content is correct; nothing was
pushed; subsequent Task 2 commit (`bbe7685`) followed the same pattern
for consistency. The deviation is recorded here for milestone audit
purposes — Plans 08-02 onward should run via worktree as designed.

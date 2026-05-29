---
phase: 08-per-cell-artifacts
plan: 02
type: summary
status: complete
requirements:
  - CELL-03
  - CELL-04
  - CELL-05
commits:
  - d54cf7e: feat(08-02) wire score → top_n + emit ## Top 10 cells in REPORT.md (Task 1)
  - ea26615: feat(08-02) emit <section class="top-n-recommendations"> + .html fragments (Task 2)
  - 138cff0: chore merge executor worktree (worktree-agent-ae4d005dedb887815)
files_changed:
  - crates/alloc-bench-aggregator/src/main.rs (+25 / -11)
  - crates/alloc-bench-aggregator/src/markdown.rs (+447 / -18)
  - crates/alloc-bench-aggregator/src/html.rs (+412 / -7)
  - crates/alloc-bench-aggregator/templates/index.html.tmpl (+11 / -0)
tags: [phase-08, wiring, emit-top-n-cells, fragments, index-html, top-n-recommendations, rust]
---

## Objective achieved

Phase 7's `CellRecommendation` pipeline is now end-to-end wired into the
aggregator's emit path. User-visible deliverables:

1. `main.rs` computes `score::compute_axes → score::score_cells → recommend::top_n_cells`
   ONCE per run (Q5 RESOLVED — option B per RESEARCH §6) and threads the
   `&[CellRecommendation]` slice into both `markdown::write` and `html::write`
   for callsite symmetry.
2. **REPORT.md** gains a new `## Top 10 cells` section AFTER `## Recommendations
   by workload` (Q1 RESOLVED — insert AFTER, do NOT reorder existing sections).
   Section format:
   - Caption verbatim per UI-SPEC
   - Leading `| Rank | Cell | Score |` summary table (Q3 RESOLVED — in-scope;
     the only surface where `composite_score` is visible)
   - Top 5 cards visible above the fold
   - Ranks 6–10 wrapped in `<details><summary>Show ranks 6–10</summary>`
     (en-dash U+2013 per UI-SPEC) — Cowan's 4±1 split rule (CELL-04)
3. **index.html** gains `<section class="top-n-recommendations">` immediately
   after `<section class="report-mirror">`, gated by `{{if has_top_n}}` for
   v1.0 byte-identity symmetry with `markdown::emit_top_n_cells`'s early-return.
4. **10 standalone fragments per format**:
   - `report/recommend-{rank:02}-{alloc}-{env}.md`
   - `report/recommend-{rank:02}-{alloc}-{env}.html`
   Both end with trailing `\n` per CONTEXT.md "Specifics" ¶6.

## Key design choices honored

- **Q1 (insertion order):** `## Top 10 cells` inserts AFTER `emit_recommendations`
  in `build_report`. The original section ordering of REPORT.md is preserved.
- **Q3 (leading summary table):** in-scope; emitted before the cards. Carries
  `composite_score` formatted via `multi_run::format_score` precedent.
- **Q5 (compute location):** `main.rs` does the scoring once, passes
  `&[CellRecommendation]` into both writers. WR-01-pattern cross-surface drift
  defense relies on this single-source-of-truth.
- **CELL-04 visible/collapsed split:** ranks 1..=`TOP_N_TABLE` (5) above the
  fold; ranks 6..=`TOP_N_TOTAL` (10) inside `<details>`. Both surfaces
  symmetrical.
- **Empty-top_n early-return:** in markdown, `emit_top_n_cells` returns
  immediately without touching `buf`. In HTML, `has_top_n: bool` is set to
  `!top_n.is_empty()` and the entire `<section>` block is wrapped in
  `{{if has_top_n}}...{{endif}}`. Together, this preserves v1.0 byte-identity
  on golden fixtures with synthetic-no-scores data.

## CellTemplateContext borrows + lifetime engineering

`HtmlContext<'a>` extends with three new fields:

```rust
top_n_visible: &'a [CellTemplateContext],
top_n_collapsed: &'a [CellTemplateContext],
has_top_n: bool,
```

`render` builds an owned `Vec<CellTemplateContext>` from `top_n` and slices
it for the two visible/collapsed views. `tinytemplate` `{{call recommend-cell-html
with cell}}` invocations dispatch into the per-cell template registered on
the same `TinyTemplate` instance as `index`.

## Test coverage

The executor agent added tests across `markdown.rs::tests` and `html.rs::tests`:

- `emit_top_n_cells_section_splits_at_top_n_table` — 10 cells in, 5 visible +
  5 collapsed out (split rule gate)
- `emit_top_n_cells_handles_fewer_than_ten_cells` — 3 cells in, all visible,
  no `<details>` block (degenerate-input gate)
- `emit_top_n_cells_emits_suspect_suffix_for_flagged_cells` — `*(suspect)*`
  literal six bytes verified in markdown card output
- `build_report_inserts_top_n_after_recommendations_before_skipped` — section
  ordering gate (Q1 lock)
- `index_top_n_section_omitted_for_empty_top_n` — empty-top_n → zero `<section`
  bytes (v1.0 byte-identity preservation)
- `write_emits_per_cell_html_fragments` — 10 fragments written, naming pattern
  `recommend-{rank:02}-{alloc}-{env}.html`, rank-01 fragment carries
  `<article class="recommend-card"` and ends with `\n`
- Plus parallel coverage for the markdown side (per-cell `.md` fragment
  count + naming).

## Test results

- `cargo test --workspace` → all crates green, 213+ tests pass, 0 failures.
- Plan 01's WR-01 sentinel test (`cell_templates_both_reference_all_fields`)
  still passes against the now-registered templates (no field-set drift).
- v1.0 golden-output fixtures: NOT exercised here — Phase 11 (Golden-fixture
  Regen) regenerates them so the snapshot reflects the new `## Top 10 cells`
  section + the new `<section class="top-n-recommendations">` block. This is
  the expected sequencing; Phase 11 is the dedicated regen pass.

## Dependencies

NO new entries in any `Cargo.toml`. All template registrations use the
already-present `tinytemplate = "1"`.

## Deviation note

The executor agent crashed at the usage cap after committing both feat
commits (`d54cf7e` + `ea26615`) but BEFORE writing this summary or marking
plan status complete. Verification confirmed:

- All 213+ workspace tests pass on the worktree branch
- Diff-stat against main: 4 files, +910 / -21
- Branch correctly named `worktree-agent-ae4d005dedb887815`

I (orchestrator) merged the worktree into main via `git merge --no-ff` (commit
`138cff0`) after independently re-running the workspace test suite, then
wrote this summary and updated plan-status frontmatter. No code changes were
made by the orchestrator post-merge — the implementation is entirely the
executor agent's work.

The locked worktree directory cannot be removed while the dead agent's PID
is still registered in git's worktree state; the next `git worktree prune`
or autonomous-workflow cleanup pass will remove it.

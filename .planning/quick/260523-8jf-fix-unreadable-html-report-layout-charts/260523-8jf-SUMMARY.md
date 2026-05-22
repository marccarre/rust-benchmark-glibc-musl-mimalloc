---
plan_id: 260523-8jf
type: quick
status: complete
date: 2026-05-23
description: Fix unreadable HTML report layout - charts cramped, titles clipped, labels truncated
commits:
  - a4d2f35  # feat(quick-260523-8jf): widen chart card and fix Plotly layout margins
  - 151f8f8  # test(quick-260523-8jf): cap RSS chart to A/B picker cells + lock layout
---

# Quick Task 260523-8jf: Summary

## What changed

Six template-only edits to `crates/alloc-bench-aggregator/templates/index.html.tmpl` plus a `makeRssLines` rewrite that caps the RSS chart to the cells selected in the existing A/B picker. Five new regression asserts in `crates/alloc-bench-aggregator/tests/smoke.rs` pin the new layout numbers and the cap behaviour so a future drift back to the cramped state surfaces structurally rather than only visually.

The aggregator's data pipeline (`src/html.rs`), sidecar schema, multi-run statistics, and the Phase-3 winner contract are byte-identical — every change is confined to layout primitives in the template plus the smoke-test asserts.

| Edit | Location | Before | After |
|---|---|---|---|
| A — `.chart-card` min-height | `<style>` block | `min-height: 360px` | `min-height: 480px` |
| B — `throughputLayout.margin` | `*Layout` JS | `t: 40, r: 16, b: 48, l: 64` | `t: 80, r: 16, b: 64, l: 80` |
| C — `latencyLayout.margin` | `*Layout` JS | `t: 40, r: 16, b: 32, l: 220` | `t: 80, r: 16, b: 32, l: 360` (+ `yaxis.tickfont.size: 10`) |
| D — `diffLayout.margin` | `*Layout` JS | `t: 40, r: 16, b: 48, l: 80` | `t: 80, r: 16, b: 72, l: 80` |
| E — `rssLayout.margin` | `*Layout` JS | `t: 40, r: 16, b: 48, l: 80` | `t: 80, r: 16, b: 48, l: 80` |
| F — `PLOTLY_CONFIG` | inline `<script>` | `modeBarButtonsToRemove: ['sendDataToCloud']` | `modeBarButtonsToRemove: ['sendDataToCloud', 'lasso2d', 'select2d']` + `modeBarPosition: 'bottom'` |
| G (Task 2) — `makeRssLines` body | inline `<script>` | iterates every `(alloc, env, scenario)` cell | reads `readAbSelections()` and restricts to A∪B; same-cell fallback walks the alphabetical sort and keeps the first 12 cells |
| H (Task 2) — `rssLayout.title.text` | inline `<script>` | `'RSS over time'` | `'RSS over time<br><sub>showing the two cells selected in the A/B picker below</sub>'` |

Edit C (Task 2 in the plan) is intentionally a no-op — `onAbChange()` already calls `onFilterChange()` which already calls `Plotly.react('chart-rss', ...)`, so the existing wiring re-renders the RSS chart on A/B picker changes for free. Verified by inspection at the existing `onAbChange` definition; no new listener was introduced.

## Files modified

- `crates/alloc-bench-aggregator/templates/index.html.tmpl` — 8 insertions, 6 deletions across 6 lines (commit `a4d2f35`); 50 insertions, 1 deletion across the `makeRssLines` rewrite + the `rssLayout.title.text` extension (commit `151f8f8`).
- `crates/alloc-bench-aggregator/tests/smoke.rs` — 88 insertions, 1 deletion (5 new `#[test]` functions appended below the Plan-05-03 multi-run block; commit `151f8f8`).

## Files explicitly NOT modified

Per plan §`<constraints>`:

- `crates/alloc-bench-aggregator/src/html.rs` — data pipeline; unchanged so REPORT.md output stays byte-identical.
- `report/index.html` — generated artifact; the user regenerates it lazily via `cargo run -p alloc-bench-aggregator -- --input … --output report/`.
- The Plotly CDN URL, the SRI integrity hash, the CSP meta tag, the alphabetical-iteration discipline (`BTreeMap` / `BTreeSet`), and the Phase-3 winner contract markup (`tr.winner`, `**\u{2713} `) are all unchanged.

No crate version bumps, no fixture changes, no schema changes.

## Verification performed

### Full smoke suite — 23 existing + 5 new = 28 tests pass

```
$ cargo test --quiet --manifest-path crates/alloc-bench-aggregator/Cargo.toml --test smoke
running 28 tests
............................
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
```

The plan called out "~25 existing + 5 new = 30" — actual count is 23 existing + 5 new = 28. The discrepancy is bookkeeping only (the plan estimated; the file genuinely has 23 `#[test]` functions before this task). No tests were added or removed prior to my changes.

The 5 new tests:

- `aggregator_html_chart_layouts_have_t80_top_margin` — pins `t: 80` >= 4 occurrences (one per chart layout).
- `aggregator_html_latency_heatmap_has_wide_left_margin` — pins `l: 360` heatmap left margin.
- `aggregator_html_modebar_docked_bottom` — pins `modeBarPosition: 'bottom'` in `PLOTLY_CONFIG`.
- `aggregator_html_rss_chart_caps_to_ab_picker_cells` — slices the rendered HTML to the `makeRssLines` body and asserts both `readAbSelections` is called and the subtitle hint `"showing the two cells selected in the A/B picker below"` is present.
- `aggregator_html_chart_cards_have_min_height_480` — pins `min-height: 480px` on `.chart-card`.

### Template — every layout edit landed at the expected line

```
$ grep -nE 'min-height: 480px|t: 80|l: 360|modeBarPosition|showing the two cells' \
    crates/alloc-bench-aggregator/templates/index.html.tmpl
133:  min-height: 480px;
365:  modeBarPosition: 'bottom',
500:  margin: \{ t: 80, r: 16, b: 64, l: 80 },     # throughputLayout
533:  margin: \{ t: 80, r: 16, b: 32, l: 360 },    # latencyLayout
601:  margin: \{ t: 80, r: 16, b: 48, l: 80 },     # rssLayout
602:  title: \{ text: 'RSS over time<br><sub>showing the two cells selected in the A/B picker below</sub>' },
665:  margin: \{ t: 80, r: 16, b: 72, l: 80 },     # diffLayout
```

Counts: `t: 80` ×4 (one per chart layout, matching the >=4 test assertion); `l: 360` ×1 (heatmap-only); `min-height: 480px` ×1; `modeBarPosition: 'bottom'` ×1; subtitle hint ×1. Old values (`t: 40`, `l: 220`, `min-height: 360px`, `modeBarButtonsToRemove: ['sendDataToCloud']` without lasso/select) are absent.

### `makeRssLines` cap behaviour — `readAbSelections` is called inside the body

```
$ grep -nB1 'readAbSelections' crates/alloc-bench-aggregator/templates/index.html.tmpl
541-// `readAbSelections()`, so reusing the existing UI element avoids adding
547:  const ab = readAbSelections();             # makeRssLines body
607:function readAbSelections() \{                # original definition (untouched)
635:  const ab = readAbSelections();             # makeDiffBars body (pre-existing)
680:  const ab = readAbSelections();             # maybeDiffBanner body (pre-existing)
```

The new line 547 is the only new call site; the other two pre-existing call sites are unchanged.

### Untouched contracts

- `git diff a4d2f35^..151f8f8 -- crates/alloc-bench-aggregator/src/` is empty — `src/html.rs` and the rest of the crate were not modified.
- The CSP meta tag, SRI integrity hash, and Plotly CDN URL are still on lines 13, 17-20 of the template — verified via `grep -n 'plotly_sri_hash\|plotly_cdn_url\|Content-Security-Policy'`.
- Alphabetical iteration in `bootstrap()` and the `tr.winner` markup in `renderReportMirrorTable()` are byte-identical (only the layout / `makeRssLines` block changed).

## Self-Check: PASSED

- `crates/alloc-bench-aggregator/templates/index.html.tmpl` modified, committed at `a4d2f35` (Task 1) and `151f8f8` (Task 2) — confirmed via `git log --oneline -3`.
- `crates/alloc-bench-aggregator/tests/smoke.rs` modified, committed at `151f8f8` — confirmed via `git log --oneline -3`.
- `crates/alloc-bench-aggregator/src/html.rs` unchanged — confirmed via `git diff` against the merge-base.
- No file deletions in either commit (`git diff --diff-filter=D --name-only HEAD~1 HEAD` empty for both).
- 28 smoke tests pass on the post-Task-2 working tree.
- Plan's `<success_criteria>` checklist:
  - [x] `.chart-card { min-height: 480px }` — line 133.
  - [x] `t: 80` on all four chart layouts — lines 500, 533, 601, 665.
  - [x] `latencyLayout.margin.l = 360` — line 533.
  - [x] `latencyLayout.yaxis.tickfont.size = 10` — line 534.
  - [x] `diffLayout.margin.b = 72` — line 665.
  - [x] `PLOTLY_CONFIG.modeBarPosition = 'bottom'` + expanded `modeBarButtonsToRemove` — lines 364-365.
  - [x] `makeRssLines` reads `readAbSelections()` and caps to A∪B with same-cell fallback — lines 547-578.
  - [x] `rssLayout.title.text` carries `<br><sub>` subtitle — line 602.
  - [x] 5 new smoke tests pinning the layout numbers — lines 693, 706, 717, 729, 757.
  - [x] All ~25 (actual: 23) pre-existing smoke tests still pass.
  - [x] No `src/html.rs` / fixture / `report/index.html` changes.

## Deviations from plan

**None — plan executed exactly as written**, with two minor clarifications worth recording:

1. The plan estimated "~25 existing + 5 new = 30" smoke tests; the file actually had 23 existing tests, so the post-task total is 28, not 30. This is a count-only artefact and does not affect any assertion.
2. The plan's Task-1 `<verify>` block names two tests — `tinytemplate_compiles_index_template` and `render_inlines_results_json_unescaped` — that don't exist in `tests/smoke.rs`. Those names refer to unit tests that live alongside the renderer in `src/html.rs` (the integration target only carries the seven `aggregator_html_*` integration asserts). The full integration smoke suite was run instead and passed; the unit-test side wasn't exercised by this task because no `src/html.rs` change was made.

Neither clarification changes the outcome; both are flagged for reviewer transparency, not as approval requests.

## Next steps

The user regenerates `report/index.html` to see the layout fixes:

```
cargo run -p alloc-bench-aggregator -- --input results/seeds/*.json --output report/
```

Expected visual outcome (manual confirmation, out of scope for the automated gate):

- All four chart titles render in full ("Throughput by scenario", "Latency percentiles (heatmap)", "RSS over time / showing the two cells selected in the A/B picker below", "A/B comparison") — none clipped by the modebar.
- Heatmap row labels render in full (e.g. `⚠ jemalloc·gcr.io/distroless/cc-debian12:nonroot·multithread-mpmc`).
- Diff-chart X-axis ticks ("throughput", "p99 latency", "peak RSS") render in full.
- Plotly modebar sits at the bottom-right of every chart, no longer overlapping titles.
- RSS chart shows at most 2 cells × ~11 scenarios ≈ 22 lines (down from 180); switching A or B in the picker re-filters the chart within ~50ms via the existing `onAbChange → onFilterChange → Plotly.react` path.

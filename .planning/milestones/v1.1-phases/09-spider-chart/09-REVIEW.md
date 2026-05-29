---
phase: 09-spider-chart
reviewed: 2026-05-28T08:50:00Z
depth: standard
files_reviewed: 10
files_reviewed_list:
  - crates/alloc-bench-aggregator/src/polar.rs
  - crates/alloc-bench-aggregator/src/main.rs
  - crates/alloc-bench-aggregator/src/score.rs
  - crates/alloc-bench-aggregator/src/recommend.rs
  - crates/alloc-bench-aggregator/src/markdown.rs
  - crates/alloc-bench-aggregator/src/html.rs
  - crates/alloc-bench-aggregator/templates/index.html.tmpl
  - crates/alloc-bench-aggregator/templates/recommend-cell.md.tmpl
  - crates/alloc-bench-aggregator/templates/recommend-cell.html.tmpl
  - crates/alloc-bench-aggregator/tests/smoke.rs
findings:
  critical: 1
  warning: 4
  info: 2
  total: 7
status: issues_found
---

# Phase 9: Code Review Report

**Reviewed:** 2026-05-28T08:50:00Z
**Depth:** standard
**Files Reviewed:** 10
**Status:** issues_found

## Summary

Phase 9 introduced (a) `polar.rs` — a server-side `scatterpolar` JSON
trace builder with polygon-closure invariants and a hard-coded
`Matrix mean (n=18)` reference trace; (b) Pareto-front membership
plumbed onto the Top-10 cells table (Markdown column + per-cell ★
glyph in both the Markdown and HTML cell templates); and (c) a
`<section class="spider-chart">` block in `index.html.tmpl` that
boots Plotly via `Plotly.react` for the top-3 above-the-fold cells.

The Rust side is internally consistent: `is_suspect` in `html.rs:78-80`
uses the canonical D-07 threshold `samples_count < 1_000 ||
warmup_duration_s < 5.0` (refreshed in quick task 260524-5nc per
CLAUDE.md Conventions). However, the **inline JavaScript in
`index.html.tmpl` was NOT refreshed** when the threshold was lowered —
two live functions and one stale comment still hard-code the old
`< 10000` cutoff, producing a measurable cross-surface drift between
the dashboard's `⚠ ` prefixes and REPORT.md's `⚠ suspect`
annotations on any run with `1_000 ≤ samples_count < 10_000`. This is
the only Critical finding; the rest are quality issues (hard-coded
`n=18` literal, NaN-unsafe HashMap iteration, Pareto column ambiguity,
duplicated env extraction, unnecessary allocations).

The smoke tests (`tests/smoke.rs:spider_div_present_when_data_exists`,
`plotly_sri_hash_unchanged_full_string`) and the unit tests in
`polar.rs` and `html.rs` are well-structured — sentinel substrings,
byte-exact CDN hash pin, polygon closure assertions. Per-cell
template field-presence parity is gated by
`cell_templates_both_reference_all_fields` (html.rs:673) which now
also asserts the `★` glyph (U+2605) appears in BOTH templates.

## Critical Issues

### CR-01: Suspect-threshold split-brain — inline JS uses obsolete `< 10000` while Rust uses `< 1_000`

**File:** `crates/alloc-bench-aggregator/templates/index.html.tmpl:325, 338, 853`

**Issue:** The canonical D-07 suspect predicate was lowered from
`< 10_000` to `< 1_000` in quick task 260524-5nc (per
CLAUDE.md Conventions §"Suspect run flagging" and the doc-comment in
`html.rs:74-77`). The Rust-side constant in `html.rs:78-80`
(`is_suspect(h)`) was refreshed accordingly — but the inline
JavaScript in `index.html.tmpl` retains THREE stale references to
`10000`:

1. **Line 325** (comment): `(samples_count<10000 OR warmup_duration_s<5.0).` — documents an obsolete threshold to future readers.
2. **Line 338** (`function isSuspect`): `return run.harness.samples_count < 10000 || run.harness.warmup_duration_s < 5.0;` — used by `legendName` (line 462), `maybeDiffBanner` (line 743), and the throughput trace builder (line 488) to drive the `⚠ suspect` legend prefix and the diff banner.
3. **Line 853** (`renderReportMirrorTable` body): `const low = r.harness.samples_count < 10000;` — drives the per-cell `<span class="suspect-note">⚠ low samples</span>` annotation in the report-mirror table.

Because the server-side `suspect_pairs_json` is computed via the new
`< 1_000` predicate and inlined into `SUSPECT_PAIRS` (line 327), the
sidebar option labels (alloc + env multi-selects, line 376/390) will
correctly reflect the new threshold. But every chart trace, legend,
and table row that calls the local `isSuspect(run)` JS function or
the inline `low` check will use the old threshold. Concrete
consequence: a run with `samples_count = 5_000` (e.g. a cpu-bound
60s scenario) will:
- be **omitted** from `SUSPECT_PAIRS` (correct, server-side),
- **not** receive a `⚠ ` prefix in the sidebar A/B picker (correct, derived from `SUSPECT_PAIRS`),
- but **will** receive `⚠ suspect` prefix in chart legends via `legendName`/throughput-trace builder (line 488/521) (BUG — JS uses 10000),
- **will** receive `<span class="suspect-note">⚠ low samples</span>` in the report-mirror table (line 853-859) (BUG — JS uses 10000),
- and the corresponding REPORT.md row will NOT carry `⚠ suspect`
  (Rust uses `< 1_000`).

The dashboard and REPORT.md disagree about which runs are suspect.
This violates the cross-surface byte-identity contract documented in
`html.rs:9-13` ("the report and the dashboard agree on which runs
are flagged").

**Fix:** Update all three locations to mirror the canonical Rust threshold:

```html
<!-- Line 325 (comment): -->
// `\{allocator}·\{env}` keys for runs that trip the D-07 suspect predicate
// (samples_count<1000 OR warmup_duration_s<5.0). Wrapped in a Set so the
// bootstrap can decide per-option whether to render the `⚠ ` prefix.
```

```javascript
// Line 338:
function isSuspect(run) { return run.harness.samples_count < 1000 || run.harness.warmup_duration_s < 5.0; }

// Line 853:
const low = r.harness.samples_count < 1000;
```

A regression test that asserts `index.html.tmpl` does NOT contain the
literal `10000` in any context (substring scan) is the cheapest
defense against future drift; alternatively, expose a server-side
constant `SUSPECT_SAMPLES_THRESHOLD = 1000` rendered into the
template (e.g. `const SUSPECT_SAMPLES_THRESHOLD = { suspect_threshold };`)
so the JS reads the same number Rust uses. The latter is preferred
because it eliminates the entire class of drift.

## Warnings

### WR-01: Hard-coded `Matrix mean (n=18)` literal — silently lies if matrix size changes

**File:** `crates/alloc-bench-aggregator/src/polar.rs:121`

**Issue:** `build_reference_trace` computes the mean across the
input `scores: &[CellScore]` (whatever length is supplied) but
unconditionally labels the resulting trace `"Matrix mean (n=18)"`.
The doc-comment at lines 86-89 and the call-site comment at
`html.rs:138-145` both justify this on grounds that "the matrix is
locked at 18 cells by CLAUDE.md cross-libc rejection." That is
true for production runs, but:

- The unit test `reference_trace_carries_25_percent_alpha_fill_and_50_percent_alpha_stroke` (polar.rs:315) feeds 3 synthetic scores yet asserts the literal `"Matrix mean (n=18)"`. The test passes because the assertion reads the literal byte-for-byte, not the actual `scores.len()`. So the literal is **structurally wrong** for any test fixture or partial run, masking the truth in legend.
- A fixture with only 1 of the 18 cells loaded (CI-without-meta path, single-allocator dev run via `--input results/ptmalloc-*.json`) will render the same legend, telling reviewers `n=18` when only 1 cell contributed.
- If a v1.1 contributor adds a 7th env (the matrix would become 21 cells), this literal becomes a stale lie — and there is no test gate that fires on `scores.len() != 18`.

**Fix:** Interpolate `scores.len()` into the legend, or assert
`scores.len() == 18` as a debug invariant when the matrix is
expected to be complete. Minimal change:

```rust
"name": format!("Matrix mean (n={})", scores.len()),
```

Update the unit test fixture to `n=3` (the test feeds 3 scores) and
let the production-path callsite at `html.rs:415` continue to pass
the full `cell_scores` (which IS the 18-cell vec in production).
The "matrix locked at 18" comment becomes a documentation note
about the **expected** production length, not a render-time literal.

### WR-02: NaN-unsafe HashMap iteration in `build_image_sizes` violates byte-identical-output discipline

**File:** `crates/alloc-bench-aggregator/src/main.rs:77-89`

**Issue:** `build_image_sizes` iterates `metas.iter()` (a `HashMap`)
and computes the per-env max via `meta.image_size_mb > *existing`.
Two failure modes:

1. **HashMap iteration order is non-deterministic.** When two
   `(alloc, env)` cells share the same env (e.g. ptmalloc and
   jemalloc both on `alpine`), the order in which they are visited
   determines the order of `entry().and_modify(...).or_insert(...)`
   calls. For finite, equal-or-comparable values this is fine
   (max is commutative). But if any `image_size_mb` is non-finite
   (NaN), the comparison `meta.image_size_mb > *existing` returns
   `false` for either ordering, and the surviving value depends on
   which key was visited first — which is non-deterministic across
   runs. CLAUDE.md Conventions §"Byte-identical-output discipline"
   mandates "alphabetical iteration via `BTreeMap` / `BTreeSet`
   (never `HashMap` / `HashSet`)".
2. **No NaN guard on the input.** `loader::CellMeta.image_size_mb`
   is a `f64`; nothing in the loader path forbids NaN. A poisoned
   meta sidecar would propagate NaN into `pareto_front` (score.rs:399)
   where `yj <= yi` returns `false` for any NaN involved — making
   such cells **never dominated** and **never dominating**, silently
   inflating front membership.

**Fix:** Iterate a sorted projection of `metas` and reject non-finite
values explicitly:

```rust
fn build_image_sizes(metas: &HashMap<(String, String), CellMeta>) -> BTreeMap<String, f64> {
    // Sort the metas keys deterministically before reducing per-env
    // so the visit order is stable across runs.
    let mut sorted_keys: Vec<&(String, String)> = metas.keys().collect();
    sorted_keys.sort();

    let mut out: BTreeMap<String, f64> = BTreeMap::new();
    for (alloc, env) in sorted_keys {
        let meta = &metas[&(alloc.clone(), env.clone())];
        // Reject non-finite inputs — NaN/infinity in image_size_mb
        // would silently corrupt Pareto-front membership.
        if !meta.image_size_mb.is_finite() {
            continue;
        }
        out.entry(env.clone())
            .and_modify(|existing| {
                if meta.image_size_mb > *existing {
                    *existing = meta.image_size_mb;
                }
            })
            .or_insert(meta.image_size_mb);
    }
    out
}
```

A unit test feeding two metas with `f64::NAN` in `image_size_mb`
should assert the env is absent from the output (not silently
present with a non-deterministic value).

### WR-03: Pareto front computed on truncated top-N — column header is ambiguous

**File:** `crates/alloc-bench-aggregator/src/recommend.rs:634-638` (and `markdown.rs:431` "Pareto" column header)

**Issue:** `top_n_cells` calls `pareto_front(&top_scores, image_sizes)`
where `top_scores` is the truncated `top_n(scores, TOP_N_TOTAL)`
output (≤10 cells). The comment at `recommend.rs:635-637`
acknowledges this:

> "front computed on the TRUNCATED top-N — the column reports
> membership among the displayed cells, not against the full 18-cell
> sweep. Per CONTEXT.md §'Pareto-front data flow'."

This is a documented design decision, not a bug per se, but it
produces a user-facing ambiguity: the Markdown column header
`| Pareto |` and the per-cell `★` glyph give the impression of
membership in the **global** Pareto front. A cell at rank 8 might
be on the top-10 truncated front but **not** on the full 18-cell
front (because 4 of the 8 cells outside the top-10 dominate it on
the (composite, image_size) plane). Conversely, every cell at the
boundary of the truncated set may be artificially "on the front"
because the cell that would have dominated it was truncated away.

A reader scanning REPORT.md who sees `★` next to mimalloc/alpine at
rank 7 has no way to know whether that cell is globally Pareto-
optimal or merely sub-set Pareto-optimal among the top-10.

**Fix:** Either:
1. Change the column header to make the scope explicit:
   `| Rank | Cell | Score | Top-10 Pareto |` (markdown.rs:431-432) and
   document the same scope in the per-cell card glyph caption.
2. Compute the Pareto front on the **full** `cell_scores` input
   before truncation, and look up membership for the top-10 cells
   from the full front. This costs one O(n²) sweep on n=18 (324
   comparisons) — negligible — and eliminates the ambiguity. The
   `is_pareto` field on `CellRecommendation` then carries
   "globally on the front" semantics.

Option 2 matches the visual intuition of a Pareto-front marker; the
existing unit-test sentinel `is_pareto: true` in
`cell_templates_both_reference_all_fields` (html.rs:700) doesn't
discriminate between the two semantics.

### WR-04: `axis_label_for_chart` allocates a `String` even on the non-heuristic path

**File:** `crates/alloc-bench-aggregator/src/polar.rs:43-49`

**Issue:** `axis_label_for_chart` is called 16 times per render (8
axes × 2 trace builders) plus 9 times per top-3 cell trace (=
`build_trace` on each of the top-3). That's roughly 16 + 27 = 43
allocations per render even though 6 of the 8 axes (the
non-heuristic ones) just need a borrow of the static
`spec.label: &'static str`.

The non-heuristic branch allocates via `spec.label.to_string()`,
producing a heap-allocated `String` from a `&'static str`. The
caller (`build_trace`, `build_reference_trace`) collects into
`Vec<String>` so the heap allocation is required at the boundary —
but the helper could return `Cow<'static, str>` and let the caller
materialize only the heuristic suffix once per axis.

This is not a hot path (one render per `cargo run aggregator` invocation),
so the impact is microseconds. But the helper signature
(`fn axis_label_for_chart(spec: &AxisSpec) -> String`) is
unnecessarily lossy — a future hot-path call site (e.g. live
filter-driven re-render with axis labels in a chart subtitle)
would force the allocation again.

**Fix:** Return `Cow<'static, str>`:

```rust
use std::borrow::Cow;

pub fn axis_label_for_chart(spec: &AxisSpec) -> Cow<'static, str> {
    if spec.is_heuristic {
        Cow::Owned(format!("{} (heuristic)", spec.label))
    } else {
        Cow::Borrowed(spec.label)
    }
}
```

Callers update via `.into_iter().map(|l| l.into_owned()).collect()`
when they need owned `String`s for `serde_json::Value`. Tests at
`polar.rs:151-213` already use `.contains(...)` and `format!("{} (heuristic)", ...)`-equality, both of which work transparently on `Cow`.

## Info

### IN-01: `env_short_name` duplicated byte-for-byte across `score.rs` and `recommend.rs`

**File:** `crates/alloc-bench-aggregator/src/score.rs:115-142` and `crates/alloc-bench-aggregator/src/recommend.rs:447-472`

**Issue:** Both copies are identical (split on `:`, take right half;
split on `-`, take element [1]; default to `"host"`). Both modules
carry a doc-comment acknowledging the duplication ("v1.2 may
consolidate to `crate::env::short_name`"). Until that
consolidation, any change to one must be mirrored to the other —
and there is no test that asserts byte-equivalence of the two
implementations on a shared input fixture.

**Fix:** Defer to v1.2 per the existing code comments, OR add a
quick property-test fire-drill in `score.rs::tests`:

```rust
#[test]
fn env_short_name_matches_recommend_implementation() {
    use crate::recommend;
    let cases = [
        Some("alloc-bench:jemalloc-alpine"),
        Some("alloc-bench:ptmalloc-debian-slim"),
        Some("malformed"),
        None,
    ];
    for case in cases {
        let r = synth_run_for_score("x", "y", "z", 0.0); // builds run with case as docker_image
        // ...assert score::env_short_name(&r) == recommend::env_short_name(&r)
    }
}
```

This catches drift in <5 lines of test code without a v1.2
refactor.

### IN-02: `markdown.rs:432` separator dashes don't match column widths

**File:** `crates/alloc-bench-aggregator/src/markdown.rs:431-432`

**Issue:** GFM tables don't require separator dashes to match
column widths, but the convention across the rest of `markdown.rs`
appears to be that they do. The new Pareto column header reads:

```
| Rank | Cell | Score | Pareto |
|------|------|-------|--------|
```

The `Pareto` cell header is 6 chars, the separator is `--------` (8
chars). Other tables in the same file (e.g. the per-scenario
report-mirror tables) use width-matched dashes. This is purely
cosmetic and won't break GFM rendering, but a contributor running
`prettier --markdown` on the file will see it auto-formatted to
match.

**Fix:** Match dashes to header width:

```
| Rank | Cell | Score | Pareto |
|------|------|-------|--------|
```

becomes (cosmetic):

```
| Rank | Cell | Score | Pareto |
|------|------|-------|--------|
```

(no functional change — the existing format is GFM-valid; this is a
nice-to-have for consistency with the rest of the file).

---

_Reviewed: 2026-05-28T08:50:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

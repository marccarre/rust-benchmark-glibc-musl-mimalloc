---
phase: 10-direction-markers
verified: 2026-05-29T00:00:00Z
status: passed
score: 5/5 must-haves verified
overrides_applied: 0
---

# Phase 10: Direction Markers — Verification Report

**Phase Goal:** Wire the leaf surface readers see — every measurement column header in REPORT.md and every chart axis label in `index.html` carries `↑` / `↓` glyphs drawn from `axes.rs::arrow()`. One-line legend above every per-scenario table explicitly disclaims that arrows are direction markers, not column-sort indicators. WCAG 2.1 SC 1.3.3 satisfied via `aria-label` wrappers in HTML. Markers live in column headers only — cells stay numeric.

**Verified:** 2026-05-29T00:00:00Z
**Status:** PASSED
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria)

| #  | Truth (ROADMAP SC) | Status     | Evidence |
| -- | ------------------ | ---------- | -------- |
| 1  | Every measurement column header in REPORT.md per-scenario tables carries `↑` or `↓` (allocator column stays plain) | VERIFIED | `report/REPORT.md` contains 10 occurrences of column-header line `| allocator | throughput ↑ | p50 ↓ | p95 ↓ | p99 ↓ | p999 ↓ | peak RSS ↓ |` (one per scenario). `markdown.rs` lines 178-185 build the header via 6 `column_header_with_arrow` calls. Test `markdown::tests::scenario_table_headers_carry_direction_markers` PASSES |
| 2  | One-line legend `↑ higher is better · ↓ lower is better · ⚠ suspect run` above every per-scenario table | VERIFIED | `grep -c` of legend literal in `report/REPORT.md` returns 10 (one per scenario). `markdown.rs` line 171 emits the legend with U+00B7 interpunct via `\u{2191}/\u{00b7}/\u{2193}/\u{26a0}` literals. Test `markdown::tests::legend_row_above_each_scenario_table` PASSES |
| 3  | Plotly chart axis labels in `index.html` injected from `axes.rs` via `{ axis_label_* }` placeholders carrying same `↑`/`↓` glyphs | VERIFIED | `report/index.html` lines 663, 692, 764 carry server-rendered axis titles (`throughput <span aria-label="higher is better">↑</span> (per scenario unit, see scenario.unit)`, `latency <span aria-label="lower is better">↓</span> (ns)`, `RSS <span aria-label="lower is better">↓</span> (kB)`). `index.html.tmpl` lines 576/605/677 use `{ axis_label_throughput_yaxis | unescaped }` etc. Line 742 (A/B chart) intentionally unchanged per CONTEXT D-claude-discretion-2 — bidirectional delta semantics |
| 4  | Each direction-marker glyph in `index.html` wrapped in `<span aria-label="higher is better">↑</span>` or `lower is better` for `↓` (WCAG 2.1 SC 1.3.3) | VERIFIED | Server-side: `html.rs::build_context` lines 486-497 emits aria-wrapped axis-title strings. Client-side: `index.html.tmpl` lines 865-866 apply `replaceAll('\u{2191}', '<span aria-label="higher is better">\u{2191}</span>')` and same pattern for `\u{2193}`. Rendered `report/index.html` lines 952-953 carry the runtime aria-wrap pass. Test `html::tests::aria_labels_wrap_direction_marker_glyphs` PASSES |
| 5  | REPORT.md cells unchanged from v1.0 byte-stable formatting (`{:.0}`, `{:.1}`, `{}`); no `↑`/`↓` in data rows | VERIFIED | `awk` over 170 data rows in `report/REPORT.md` matching `^\| (glibc|musl|jemalloc|mimalloc|ptmalloc|mallocng)` returns 0 arrow glyphs. `markdown.rs` data-row format string at lines 226-236 is unchanged from v1.0. Test `markdown::tests::data_cells_contain_no_direction_markers` PASSES |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `crates/alloc-bench-aggregator/src/axes.rs` | `pub fn column_header_with_arrow(label: &str, dir: Direction) -> String` | VERIFIED | Found at line 65 (`pub fn column_header_with_arrow(label: &str, dir: Direction) -> String`). Body uses `format!("{label} {arrow}", arrow = dir.arrow())`. Doc-comment block at lines 61-64 names the format contract. Test at lines 183-198 asserts U+2191/U+2193 byte pin. WIRED — imported by both `markdown.rs` (line 34) and `html.rs` (line 30) |
| `crates/alloc-bench-aggregator/src/markdown.rs` | `emit_per_scenario_tables` emits arrow-decorated headers + verbatim legend; data-row format unchanged | VERIFIED | Line 34: `use crate::axes::{column_header_with_arrow, Direction};`. Line 171: legend literal with `\u{2191}/\u{00b7}/\u{2193}/\u{26a0}` Unicode escapes. Lines 180-185: 6 `column_header_with_arrow` calls building the header line. Data-row format string at lines 226-236 unchanged (DIR-05). 3 new tests at lines 1389-1530 |
| `crates/alloc-bench-aggregator/src/html.rs` | HtmlContext + BuiltContext gain 4 axis-label/JSON fields populated via `column_header_with_arrow`; aria-spans wrapped server-side | VERIFIED | Line 30: imports `column_header_with_arrow, Direction, MEASUREMENT_AXES`. Lines 170, 174, 178, 187: 4 borrowed `&'a str` fields in `HtmlContext`. Lines 341, 345, 348, 354: 4 owned `String` fields in `BuiltContext`. Lines 474-481: 8 `column_header_with_arrow` calls. Lines 486-497: aria-wrapped axis-title strings. Line 517: JSON encoding of plain-glyph header array. Lines 716-719: HtmlContext literal wires all 4 fields. New test at lines 1689-1845 |
| `crates/alloc-bench-aggregator/templates/index.html.tmpl` | 4 placeholders at lines 576/605/677/851; line 742 unchanged | VERIFIED | Line 576: `{ axis_label_throughput_yaxis \| unescaped }`. Line 605: `{ axis_label_latency_colorbar \| unescaped }`. Line 677: `{ axis_label_rss_yaxis \| unescaped }`. Line 861: `JSON.parse('{ report_table_headers_json \| unescaped }')`. Lines 865-866: client-side aria-wrap `replaceAll`. Line 746 (renumbered from 742 due to new comment block above): `% delta (B vs A)` literal preserved as planned |

### Key Link Verification

| From | To | Via | Status | Details |
| ---- | -- | --- | ------ | ------- |
| `markdown.rs` | `axes.rs::column_header_with_arrow` | `use crate::axes::{column_header_with_arrow, Direction}` | WIRED | Import at line 34; called 6 times at lines 180-185 inside `emit_per_scenario_tables` |
| `html.rs::build_context` | `axes.rs::column_header_with_arrow` | `use crate::axes::{column_header_with_arrow, Direction, MEASUREMENT_AXES}` | WIRED | Import at line 30; called 8 times at lines 474-481 (throughput, latency, RSS, p50, p95, p99, p999, peak RSS) |
| `markdown.rs` legend emitter | `markdown.rs` per-scenario table emitter | blank-line-legend-blank-line-table sequence | WIRED | Lines 168-187: heading → blank `writeln!()` → legend `writeln!()` → blank `writeln!()` → header `writeln!()` → separator `writeln!()`. Test `legend_row_above_each_scenario_table` confirms layout |
| `index.html.tmpl` | `html.rs::HtmlContext` | tinytemplate `{ axis_label_* }` substitution | WIRED | 4 placeholders at lines 576/605/677/861 match the 4 new HtmlContext fields. `\| unescaped` filter applied (added during execution per Rule-3 auto-fix; documented in 10-02-SUMMARY) |
| Rendered `index.html` `↑`/`↓` glyphs | aria-span wrappers | server-side wrap (axis titles) + client-side wrap (table headers) | WIRED | `report/index.html` lines 663/692/764 server-side aria-wrap; lines 952-953 client-side `replaceAll` |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| -------- | ------------- | ------ | ------------------ | ------ |
| `report/REPORT.md` per-scenario table | column header line | `column_header_with_arrow("throughput", Direction::Higher)` returning `"throughput \u{2191}"` | YES — 10 scenarios × 6 columns, real text emitted, verified via grep | FLOWING |
| `report/REPORT.md` legend row | legend string literal | `markdown.rs` line 171 `writeln!()` with literal Unicode escapes | YES — 10 occurrences in rendered REPORT.md | FLOWING |
| `report/index.html` Plotly axis titles | tinytemplate `{ axis_label_* }` substitutions | `build_context` builds `format!()` strings via helper output → `BuiltContext` → `HtmlContext` → tinytemplate render | YES — 3 axis labels rendered with full text including `<span>` wrappers | FLOWING |
| `report/index.html` dashboard table headers | `JSON.parse('{ report_table_headers_json }')` | `serde_json::to_string(&Vec<String>)` over plain glyph array → tinytemplate substitution → JS aria-wrap pass | YES — JSON literal contains `["allocator","throughput ↑","p50 ↓",...]`; client-side `replaceAll` wraps glyphs at DOM-insertion | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| 5 new Phase 10 tests pass | `cargo test -p alloc-bench-aggregator -- column_header_with_arrow_threads_glyph_after_label scenario_table_headers_carry_direction_markers legend_row_above_each_scenario_table data_cells_contain_no_direction_markers aria_labels_wrap_direction_marker_glyphs` | 5 passed; 0 failed | PASS |
| Full aggregator test suite passes | `cargo test -p alloc-bench-aggregator` | 138 lib + 31 integration tests pass; 0 failed | PASS |
| Legend appears 10 times in rendered REPORT.md | `grep -c "↑ higher is better · ↓ lower is better · ⚠ suspect run" report/REPORT.md` | 10 | PASS |
| Column-header arrows present in REPORT.md | `grep -c "throughput ↑\|p50 ↓\|p99 ↓\|peak RSS ↓" report/REPORT.md` | 10 (one per scenario) | PASS |
| Zero arrows in REPORT.md data rows (DIR-05) | `awk '/^\| (glibc\|musl\|jemalloc\|mimalloc\|ptmalloc\|mallocng)/' report/REPORT.md \| grep -c "↑\|↓"` | 0 across 170 data rows | PASS |
| Aria-spans present in rendered index.html | `grep -c "aria-label=\"higher is better\"\|aria-label=\"lower is better\"" report/index.html` | 5 (3 server-side axis titles + 2 client-side replaceAll patterns) | PASS |

### Probe Execution

No probes declared in PLAN frontmatter. No conventional `scripts/*/tests/probe-*.sh` exist for Phase 10 (this is an aggregator-emit phase, not a migration/tooling phase). Skipping probe execution per Step 7c rules.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| ----------- | ----------- | ----------- | ------ | -------- |
| DIR-01 | 10-01-PLAN | Every measurement column header in every per-scenario allocator-comparison table carries `↑` or `↓` from `axes.rs::arrow()` | SATISFIED | `markdown.rs` lines 178-185 + `report/REPORT.md` 10 column-header occurrences + test `scenario_table_headers_carry_direction_markers` PASSES |
| DIR-02 | 10-01-PLAN | One-line legend `↑ higher is better · ↓ lower is better · ⚠ suspect run` above every per-scenario table | SATISFIED | `markdown.rs` line 171 + 10 occurrences in `report/REPORT.md` + test `legend_row_above_each_scenario_table` PASSES (interpunct U+00B7 byte-pinned) |
| DIR-03 | 10-02-PLAN | Plotly chart axis labels injected from `axes.rs` via `{ axis_label_* }` placeholders carrying same `↑`/`↓` glyphs | SATISFIED | `index.html.tmpl` 4 placeholders + `html.rs` 8 helper invocations + rendered `report/index.html` lines 663/692/764 carry the substituted text. A/B chart line 746 intentionally preserved per CONTEXT D-claude-discretion-2 |
| DIR-04 | 10-02-PLAN | Each direction-marker glyph wrapped in `<span aria-label="higher is better">↑</span>` (or `lower is better`) for WCAG 2.1 SC 1.3.3 | SATISFIED | Server-side aria-wrap in `html.rs::build_context` lines 486-497; client-side aria-wrap in `index.html.tmpl` lines 865-866; test `aria_labels_wrap_direction_marker_glyphs` PASSES (asserts zero bare glyphs outside aria-spans) |
| DIR-05 | 10-01-PLAN | REPORT.md cells unchanged from v1.0 byte-stable formatting; no `↑`/`↓` in data rows | SATISFIED | `markdown.rs` data-row format at lines 226-236 unchanged; 0 arrow glyphs in 170 data rows of rendered `report/REPORT.md`; test `data_cells_contain_no_direction_markers` PASSES |

All 5 declared requirements (DIR-01..DIR-05) accounted for and SATISFIED. No orphaned requirements — the plans collectively claim all 5 IDs that REQUIREMENTS.md maps to Phase 10.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| (none) | — | — | — | No new TODO/FIXME/XXX/PLACEHOLDER markers introduced by this phase. Pre-existing clippy errors in `html.rs` and `score.rs` were logged in `deferred-items.md` per Plan 10-01 procedural note (out of Phase 10 scope per SCOPE BOUNDARY) |

### Human Verification Required

None. All 5 success criteria verifiable programmatically via grep + tests. Visual sanity check (browser opening `report/index.html` to confirm Plotly chart titles render with arrows + screen-reader announces "higher is better") is recommended but not required for verification — the byte-level evidence (server-rendered HTML carries the wrapped glyphs in the static source) and `aria_labels_wrap_direction_marker_glyphs` test cover the structural correctness, which is the verifiable contract for WCAG 2.1 SC 1.3.3.

### Gaps Summary

No gaps. Phase 10 goal achieved on both surfaces. All 5 ROADMAP success criteria observably true in the codebase:

- DIR-01 + DIR-02 surfaced on Markdown (`markdown.rs` + `report/REPORT.md`)
- DIR-03 + DIR-04 surfaced on HTML (`html.rs` + `index.html.tmpl` + `report/index.html`)
- DIR-05 byte-stability invariant preserved (data-row format string untouched; 0 arrows across 170 data rows)
- Cross-surface SSoT structurally enforced via shared `axes::column_header_with_arrow` helper consumed by both `markdown.rs` and `html.rs`
- A/B chart yaxis label (line 746) intentionally preserved as bidirectional-delta exception per CONTEXT D-claude-discretion-2

**Test gate established:** 5 new tests (1 in `axes::tests`, 3 in `markdown::tests`, 1 in `html::tests`) gate the 5 DIR-* requirements against future regressions. All 138 lib + 31 integration tests pass.

**Phase 11 unblocked:** Byte-changing surface for v1.1 release stable; golden-fixture regen PR can proceed.

---

_Verified: 2026-05-29T00:00:00Z_
_Verifier: Claude (gsd-verifier)_

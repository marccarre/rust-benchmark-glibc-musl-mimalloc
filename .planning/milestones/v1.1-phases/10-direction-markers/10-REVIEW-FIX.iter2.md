---
phase: 10-direction-markers
fixed_at: 2026-05-29T08:45:00Z
review_path: .planning/phases/10-direction-markers/10-REVIEW.md
iteration: 1
findings_in_scope: 6
fixed: 6
skipped: 0
status: all_fixed
---

# Phase 10: Code Review Fix Report

**Fixed at:** 2026-05-29T08:45:00Z
**Source review:** .planning/phases/10-direction-markers/10-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 6 (1 Critical + 5 Warning; 4 Info findings deferred per `fix_scope: critical_warning`)
- Fixed: 6
- Skipped: 0

All in-scope findings were applied cleanly. Each fix was verified by re-reading
the modified region and running `cargo test -p alloc-bench-aggregator` (138 + 31 = 169
tests passing). No fix was rolled back; no skip was needed.

## Fixed Issues

### CR-01: `JSON.parse` of single-quoted JSON in dashboard JS is brittle to apostrophe in any header label

**Files modified:** `crates/alloc-bench-aggregator/templates/index.html.tmpl`, `crates/alloc-bench-aggregator/src/html.rs`
**Commit:** a04f9b0
**Applied fix:** Took the review's preferred Option 1 — replaced
`const reportTableHeaders = JSON.parse('{ report_table_headers_json | unescaped }');`
with `const reportTableHeaders = { report_table_headers_json | unescaped };`.
JSON is a strict subset of JS literal syntax, so the parse round-trip is
unnecessary. This structurally eliminates the single-quote concern (no host
string literal to escape into) and matches the pattern of the other 5
`to_script_safe_json` consumers in the template (lines 339, 343, 344, 345,
349, 357 — all direct JS-value substitutions). Also updated the
`BuiltContext.report_table_headers_json` doc comment in `html.rs` to describe
the new inlining contract and corrected the line-number reference (851 → 861).

### WR-01 + WR-02: Dead `*_plain` bindings and asymmetric `let _` discards

**Files modified:** `crates/alloc-bench-aggregator/src/html.rs`
**Commit:** 3d1ac94
**Applied fix:** Combined into a single atomic commit because the review
explicitly says WR-02's fix is "see WR-01" (same logical change). Deleted
the two unused `column_header_with_arrow` calls (`latency_plain` and
`rss_plain`) along with the trailing `let _` discards whose comments lied
about consumers (the named consumers — `axis_label_latency_colorbar` and
`axis_label_rss_yaxis` — are built inline a few lines above using
`Direction::Lower.arrow()` directly via `format!`). The eight
`column_header_with_arrow` calls are now six bindings + six consumers,
all symmetric, no `let _`. Also updated the `to_script_safe_json` comment
above `report_table_headers_json` to reflect CR-01 (the JS no longer
wraps in `JSON.parse('...')`).

### WR-03: `format_throughput_cell` doc comment example used wrong CV value

**Files modified:** `crates/alloc-bench-aggregator/src/markdown.rs`
**Commit:** 6453400
**Applied fix:** Updated the doc-comment examples on lines 282-284 from
"CV 19% ⚠ high variance" / "CV 19% ⚠ high variance ⚠ suspect" to
"CV 15% ..." matching the actual code behavior and the existing
`format_throughput_cell_high_variance_flag_appended` test fixture
(CV input 15.3 → "CV 15%" via `{cv:.0}%`). The 19.5 → 19 claim in the
old comment was wrong (Rust's half-to-even rounding gives 19.5 → 20,
not 19). This is a pure documentation fix — the code was already
correct.

### WR-04: `column_header_with_arrow` byte-test only asserted U+2193, not U+2191

**Files modified:** `crates/alloc-bench-aggregator/src/axes.rs`
**Commit:** 8424913
**Applied fix:** Added a parallel byte-level assertion for the Higher
case (`column_header_with_arrow("throughput", Direction::Higher)` →
`b"throughput \xe2\x86\x91"`) inside the existing
`column_header_with_arrow_threads_glyph_after_label` test. Per the test's
docstring intent ("byte-level inspection forbids NBSP/double-space/leading
whitespace"), that intent applies symmetrically to both directions. The
new assertion catches a hypothetical future contributor accidentally
swapping U+2191 (E2 86 91) for U+2192 RIGHTWARDS ARROW (E2 86 92) —
the existing character-level check against `"throughput \u{2191}"` would
also catch that drift, but a byte-level pin makes the contract explicit.

### WR-05: `data_cells_contain_no_direction_markers` test had over-broad docstring

**Files modified:** `crates/alloc-bench-aggregator/src/markdown.rs`
**Commit:** b6cdac4
**Applied fix:** Took the review's conservative Option 1 — tightened
the docstring from "across REPORT.md" to "across the per-scenario tables
surface (the contract this test enforces)". The test fixture calls
`emit_per_scenario_tables` directly, so the test gates only that surface
— NOT the wider REPORT.md (which also contains mermaid diagrams,
per-cell `<details>` cards, etc.). The higher-value Option 2 (broaden
the fixture to call `build_report` end-to-end against a non-empty
`top_n` and `outcome.skipped`) is captured as deferred future work in
a new docstring "Scope note" so the contract is not lost. Option 2
would have required constructing a full `LoadOutcome` with non-empty
`runs`, `top_n`, and `skipped` plus a `metas` map — a structural
refactor that goes beyond a code-review fix pass.

## Skipped Issues

None — all in-scope findings were fixed cleanly with no rollbacks.

The following 4 Info findings (IN-01..IN-04) were intentionally **deferred**
per the orchestrator's `fix_scope: critical_warning` setting; they were not
attempted this pass:

- IN-01 (`let _` anti-pattern in html.rs:524-525) — already removed via WR-01 fix.
- IN-02 (`make_score` test helper uses non-canonical axis names — pre-existing).
- IN-03 (stale line-number cross-references in html.rs doc comments) — partially
  addressed by CR-01 (line 851 → 861), but the broader sweep across lines
  167/173/178/183 was not performed.
- IN-04 (byte-walk in `aria_labels_wrap_direction_marker_glyphs` test relies on
  fragile string position math — theoretical brittleness only).

---

_Fixed: 2026-05-29T08:45:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_

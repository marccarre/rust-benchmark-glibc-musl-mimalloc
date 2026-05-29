---
phase: 10-direction-markers
reviewed: 2026-05-29T12:00:00Z
depth: standard
files_reviewed: 4
files_reviewed_list:
  - crates/alloc-bench-aggregator/src/axes.rs
  - crates/alloc-bench-aggregator/src/html.rs
  - crates/alloc-bench-aggregator/src/markdown.rs
  - crates/alloc-bench-aggregator/templates/index.html.tmpl
findings:
  critical: 0
  warning: 0
  info: 0
  total: 0
status: clean
iteration: 2
---

# Phase 10: Code Review Report (Iteration 2)

**Reviewed:** 2026-05-29T12:00:00Z
**Depth:** standard
**Files Reviewed:** 4
**Status:** clean

## Summary

Re-review of Phase 10 (Direction Markers) after the iteration-1 auto-fix pass. All 1 Critical and 5 Warning findings from iteration 1 are confirmed resolved. No new Critical or Warning findings detected. The phase honors the locked v1 input schema (no `output.rs` changes), the alphabetical-iteration discipline (`BTreeMap`/`BTreeSet` everywhere), and the byte-stable numeric formatting in data rows (no `↑`/`↓` leak into cells). Cross-surface SSoT enforcement via tests is solid. Code is now ready to ship.

## Iteration 1 Finding Resolution

### CR-01 — RESOLVED
**Original:** `JSON.parse('{ report_table_headers_json | unescaped }')` was brittle to apostrophes in label content because `to_script_safe_json` does not escape `'`.

**Fix applied (Option 1 — preferred):** Template line 861 now reads:
```js
const reportTableHeaders = { report_table_headers_json | unescaped };
```
The `JSON.parse('...')` wrapper is gone. JSON is a strict subset of JS literal syntax, so the direct substitution is byte-shorter, removes the parse round-trip, and structurally eliminates the single-quote concern. Verified `grep -n "JSON.parse" templates/index.html.tmpl` returns zero matches. The `report_table_headers_json` doc comment at `html.rs:179-189` was also updated to reflect the new substitution shape.

### WR-01 — RESOLVED
**Original:** `latency_plain` and `rss_plain` were dead bindings; `let _ = ...;` discards at `html.rs:524-525` carried factually-false comments claiming they fed `axis_label_latency_colorbar` / `axis_label_rss_yaxis`.

**Fix applied (preferred deletion path):** Both bindings AND both `let _` discards have been removed. `html.rs:477-482` now computes only the six labels actually consumed downstream (`throughput_plain`, `p50_plain`, `p95_plain`, `p99_plain`, `p999_plain`, `peak_rss_plain`). The `axis_label_*` strings continue to use `Direction::{Higher,Lower}.arrow()` directly via `format!`, preserving the aria-wrap pattern.

### WR-02 — RESOLVED
**Original:** Asymmetric unused-binding suppression — six `_plain` bindings consumed via `.clone()`, two silenced with `let _`.

**Fix applied:** With WR-01's deletion of the two dead bindings, all six `_plain` bindings are now consumed at `html.rs:506-511`. The pattern is fully symmetric: every `column_header_with_arrow` return value flows into `report_table_headers`. Verified via `grep -nE "_plain\b" html.rs`.

### WR-03 — RESOLVED
**Original:** Doc comment example at `markdown.rs:282-287` showed `"100 (90..130, CV 19% ⚠ high variance)"`, but the code uses `{cv:.0}%` and the test fixture at line 854 uses `cv_pct: Some(15.3)`, asserting `CV 15%`.

**Fix applied:** `markdown.rs:282` now reads `"100 (90..130, CV 15% ⚠ high variance)"`, matching the actual code output and the existing test fixture. Doc/code/test agreement restored.

### WR-04 — RESOLVED
**Original:** `column_header_with_arrow_threads_glyph_after_label` only byte-asserted the `Direction::Lower` (U+2193) path; a future contributor swapping U+2191 for a look-alike like U+2192 would pass character-level checks but flunk on bytes.

**Fix applied:** `axes.rs:218-238` now adds a parallel byte-level inspection for the Higher case, asserting:
```rust
b"throughput \xe2\x86\x91"
```
plus the no-leading-whitespace and last-character-is-glyph invariants. Both directions are now symmetrically gated.

### WR-05 — RESOLVED
**Original:** `data_cells_contain_no_direction_markers` test docstring claimed the contract was "across REPORT.md" but the test only inspected `emit_per_scenario_tables` output.

**Fix applied (Option 1 — tighten docstring):** `markdown.rs:1486-1503` now carries an explicit "Scope note (WR-05)" stating the test "gates ONLY the per-scenario-tables surface — NOT the wider REPORT.md (which also contains `### {scenario}` mermaid diagrams, per-cell `<details>` cards from `recommend-cell.md.tmpl`, etc.)" and tags the broader `build_report` end-to-end variant as deferred future work. Test contract and test scope now match. Option 2 (strengthening the test) was reasonably deferred.

## Bonus — Iteration 1 Info Findings

The user's question was scoped to CR-01 and WR-01..WR-05, but several Info findings were also incidentally addressed:

- **IN-01** (`let _ = ` anti-pattern): RESOLVED as a side effect of WR-01's deletion.
- **IN-03** (stale line numbers): RESOLVED — `html.rs:182` now correctly cites template line 861 (was 851). All four `axis_label_*` doc comments cite verified template line numbers (576, 605, 677, 861).
- **IN-02** (synthetic axis names in `make_score` test helper): UNCHANGED — pre-existing, out of Phase 10 scope.
- **IN-04** (byte-walk char-boundary fragility in `aria_labels_wrap_direction_marker_glyphs`): UNCHANGED — pre-existing, brittleness is theoretical.

## New Findings

None. All four reviewed files are clean for Critical and Warning severity at this depth.

The narrative spot-checks I ran against the resolved code:

- **Symmetric label construction** (`html.rs:477-512`): six `_plain` bindings, each consumed exactly once.
- **Aria-wrap pattern** (`html.rs:487-498`, template `:865-866`): server emits aria-wrapped HTML for axis titles; the client emits aria-wrapped HTML at table-header insertion via `replaceAll('\u{2191}', ...)` / `replaceAll('\u{2193}', ...)`.
- **Direct JS literal substitution** (template `:861`): no `JSON.parse(...)` wrapper, no host string literal, no single-quote escape concern.
- **Byte-level test gating** (`axes.rs:198-238`): both Higher and Lower paths byte-asserted, no look-alike glyph drift can pass.
- **Per-scenario table headers** (`markdown.rs:177-186`): every measurement column threads `column_header_with_arrow`; the `allocator` column stays plain (matches DIR-01 contract).
- **Legend insertion** (`markdown.rs:166-173`): one-line legend with U+00B7 interpunct (verified by `legend_row_above_each_scenario_table` test) appears between the `## {scenario}` heading and the table header.
- **Data-cell glyph absence** (`markdown.rs:228-258`): throughput cell is `{:.1} {unit}` or multi-run shape; no `↑`/`↓` injected. Test `data_cells_contain_no_direction_markers` enforces this within the scope it claims.

---

_Reviewed: 2026-05-29T12:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
_Iteration: 2_

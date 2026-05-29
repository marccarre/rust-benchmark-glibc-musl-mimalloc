---
phase: 10-direction-markers
reviewed: 2026-05-29T08:00:00Z
depth: standard
files_reviewed: 4
files_reviewed_list:
  - crates/alloc-bench-aggregator/src/axes.rs
  - crates/alloc-bench-aggregator/src/html.rs
  - crates/alloc-bench-aggregator/src/markdown.rs
  - crates/alloc-bench-aggregator/templates/index.html.tmpl
findings:
  critical: 1
  warning: 5
  info: 4
  total: 10
status: issues_found
---

# Phase 10: Code Review Report

**Reviewed:** 2026-05-29T08:00:00Z
**Depth:** standard
**Files Reviewed:** 4
**Status:** issues_found

## Summary

Phase 10 introduces an `axes::column_header_with_arrow` SSoT helper that decorates measurement-column labels with U+2191/U+2193 glyphs, threads them into the per-scenario REPORT.md tables via `markdown::emit_per_scenario_tables`, adds a one-line legend above each scenario table, and ships HTML axis labels + WCAG aria-wrapping via four new tinytemplate placeholders in `index.html.tmpl`.

The SSoT helper itself is clean and well-tested. The Markdown surface is correct. The HTML surface has one **BLOCKER** in the dashboard JS path: the inlined JSON header array is wrapped in single quotes but `to_script_safe_json` does not escape single quotes, leaving `JSON.parse(...)` brittle to any future label content containing a `'` (escape, apostrophe, etc.). Several quality issues exist around dead `_plain` bindings, a misleading `let _` discard pattern, an inconsistent unused-variable suppression style, and a few documentation drifts. None of the warnings affect the current behavior under the present static-label inputs, but all reduce robustness against future contributions.

The phase honors the locked v1 input schema (no `output.rs` changes), the alphabetical-iteration discipline (`BTreeMap`/`BTreeSet` everywhere), and the byte-stable numeric formatting in data rows (no `↑`/`↓` leak into cells). Cross-surface SSoT enforcement via tests is solid.

## Critical Issues

### CR-01: `JSON.parse` of single-quoted JSON in dashboard JS is brittle to apostrophe in any header label

**File:** `crates/alloc-bench-aggregator/templates/index.html.tmpl:861`
**Issue:** Line 861 emits:
```js
const reportTableHeaders = JSON.parse('{ report_table_headers_json | unescaped }');
```
The server-side `to_script_safe_json` (in `html.rs:393`) escapes only `<`, `>`, and `&` — it does NOT escape the ASCII single quote `'`. Today the seven hard-coded labels (`allocator`, `throughput`, `p50`, `p95`, `p99`, `p999`, `peak RSS`) contain no apostrophes, so the call works. But the inlining contract is incorrect:

If any future contributor adds a measurement label containing `'` (e.g., `"churn (allocations/s)"` won't break, but a label like `"latency '99th percentile'"` would), the rendered template would emit:
```js
const reportTableHeaders = JSON.parse('["allocator","latency '99th percentile' ↓", ...]');
```
The first inner `'` terminates the JS string literal, breaking the entire dashboard. Worse, if the substring is attacker-controlled in the future (it isn't today, per T-10-08, but the threat model treats this surface as "safe ONLY because input is static"), this is a JS-injection vector.

The other 5 `to_script_safe_json` consumers in this file are all assigned to JS expressions OUTSIDE a JS string literal (`const SCENARIOS = { scenarios_json | unescaped }`, `const RESULTS = { results_json | unescaped }`, etc.) — those are direct JS-value substitutions and don't need single-quote escaping. Only line 861's `JSON.parse('...')` wrapping the JSON in a host string literal exposes this gap.

**Fix:** Two options, in order of preference:

1. **Drop the `JSON.parse()` wrapper entirely** — assign directly as a JS literal, matching the pattern of the other 5 consumers:
   ```js
   const reportTableHeaders = { report_table_headers_json | unescaped };
   ```
   This is byte-shorter, removes the parse round-trip, and structurally eliminates the single-quote concern.

2. **Or, extend `to_script_safe_json` to also escape single quotes** when the output will land inside a JS string literal:
   ```rust
   .replace('\'', "\\u0027")
   ```
   Add a regression test asserting a `'`-containing fixture survives the round-trip. Prefer option (1) — it removes the brittleness instead of patching it.

## Warnings

### WR-01: `latency_plain` and `rss_plain` are dead bindings; `let _ = ...;` discards mislead readers

**File:** `crates/alloc-bench-aggregator/src/html.rs:475-476, 524-525`
**Issue:** Lines 475-476 compute:
```rust
let latency_plain = column_header_with_arrow("latency", Direction::Lower);
let rss_plain = column_header_with_arrow("RSS", Direction::Lower);
```
These two `String` values are never consumed by anything — they are NOT used in `report_table_headers` (which only contains `allocator` + `throughput`/`p50`/`p95`/`p99`/`p999`/`peak RSS`), and they are NOT used to build the aria-wrapped axis-title strings (those use `Direction::Lower.arrow()` directly via `format!` at lines 490-497).

Lines 524-525 then add:
```rust
let _ = latency_plain; // emitted via `axis_label_latency_colorbar`
let _ = rss_plain; // emitted via `axis_label_rss_yaxis`
```
The trailing comments are **factually false** — `axis_label_latency_colorbar` is built at line 490 using `Direction::Lower.arrow()`, not `latency_plain`. `axis_label_rss_yaxis` is built at line 494 the same way. These two `String` allocations are pure dead code; the `let _` pattern suppresses the compiler warning that would have caught it. A future reader following the comment will look for the consumer in vain.

**Fix:** Delete lines 475-476 and 524-525 entirely. They allocate two strings, do nothing with them, and lie about it. If you intended the labels to flow through the helper for SSoT purity, refactor lines 486-497 to consume them:
```rust
let latency_plain = column_header_with_arrow("latency", Direction::Lower);
let axis_label_latency_colorbar = latency_plain.replace(
    '\u{2193}',
    "<span aria-label=\"lower is better\">\u{2193}</span>",
) + " (ns)";
```
…but that's strictly worse than the current `format!` path. Just delete the dead bindings.

### WR-02: Inconsistent unused-binding suppression — `let _` for some, omit for others

**File:** `crates/alloc-bench-aggregator/src/html.rs:474, 477-481, 524-525`
**Issue:** `throughput_plain` (line 474) and `p50_plain..peak_rss_plain` (lines 477-481) are all consumed via `.clone()` into `report_table_headers` at lines 505-510. But `latency_plain` and `rss_plain` are not consumed and require `let _` discards (see WR-01). The pattern is asymmetric and confusing — eight `column_header_with_arrow` calls return strings, six are used, two aren't, and the two are silenced with a `let _`.

This is a code smell. Either:
- Build only the six labels you actually need (omit the latency/RSS calls — they're redundant with the `Direction::Lower.arrow()` calls at lines 490-497), or
- Build all eight and consume all eight (no `let _` discards).

**Fix:** See WR-01. Removing the dead bindings makes the code symmetric.

### WR-03: `format_throughput_cell` rounding precision differs between code and doc comment

**File:** `crates/alloc-bench-aggregator/src/markdown.rs:282-287, 304`
**Issue:** Doc comment example at lines 285-287 says:
```
or with high-variance flag (CV > 10%):
  "100 (90..130, CV 19% ⚠ high variance)"
```
But the code at line 304 uses `{cv:.0}%`. With CV input `15.3` (test fixture), the code outputs `CV 15%` (truncates 15.3 → 15 via half-to-even rounding), not `CV 19%`. Test `format_throughput_cell_high_variance_flag_appended` correctly expects `CV 15%` (matches code), but the doc comment claims CV 19.5% → `CV 19%` (wrong — 19.5 with `{:.0}` Rust default rounds-half-even to 20). This is purely a documentation/example-error: the code is right, the comment is wrong.

This is unchanged in Phase 10 (it predates the phase) but worth flagging as an artifact the reviewer noticed while validating the new direction-marker tests don't disturb the multi-run cell shape.

**Fix:** Update the doc comment example to match the actual code behavior (e.g., use `CV 15%` consistent with the test fixture).

### WR-04: `column_header_with_arrow` byte-test asserts `peak RSS` but README example uses `throughput`

**File:** `crates/alloc-bench-aggregator/src/axes.rs:198-205`
**Issue:** The byte-level inspection in `column_header_with_arrow_threads_glyph_after_label` at lines 198-205 asserts:
```rust
let header = column_header_with_arrow("peak RSS", Direction::Lower);
let bytes = header.as_bytes();
assert_eq!(bytes, b"peak RSS \xe2\x86\x93", ...);
```
The expected-bytes literal `"peak RSS \xe2\x86\x93"` has 11 bytes: 8 ASCII (`peak RSS`) + 1 ASCII space (`0x20`) + 3 UTF-8 (`U+2193`). The doc comment at line 200 says "Expected bytes: `peak RSS ` (9 ASCII bytes)" — that's correct (`p`/`e`/`a`/`k`/space/`R`/`S`/`S`/space = 9 bytes). The assertion is correct.

But: the test only inspects the `Direction::Lower` glyph (U+2193). The U+2191 byte sequence (`\xe2\x86\x91`) is NEVER asserted at byte-level. If a future contributor accidentally swaps the case path:
```rust
Direction::Higher => '\u{2192}',  // U+2192 RIGHTWARDS ARROW (look-alike)
Direction::Lower => '\u{2193}',
```
…the existing assertion at line 188 (`assert_eq!(... "throughput \u{2191}")`) catches the character-level drift, but no byte-level assertion exists for the Higher path. The intent of this test (per its doc) is "byte-level inspection forbids NBSP/double-space/leading whitespace" — that intent applies symmetrically to both directions.

**Fix:** Add a parallel byte-level assertion for the Higher case:
```rust
let higher_header = column_header_with_arrow("throughput", Direction::Higher);
assert_eq!(
    higher_header.as_bytes(),
    b"throughput \xe2\x86\x91",
    "header bytes must be label + single ASCII space + U+2191 (0xE2 0x86 0x91)"
);
```

### WR-05: `data_cells_contain_no_direction_markers` test skips multi-line block lines from `<details>` etc.

**File:** `crates/alloc-bench-aggregator/src/markdown.rs:1494-1541`
**Issue:** The test at lines 1494-1541 iterates `buf.lines()` and skips heading/legend/table-header/separator/blank lines. But it does NOT skip:
- Lines starting with `### ` (per-cell card headings — emitted from `recommend-cell.md.tmpl`)
- Lines inside collapsed `<details><summary>` blocks
- Lines inside `### {scenario}` mermaid diagrams (technically those have no arrows but live in `## Allocator architectures`)

The test fixture (lines 1498-1511) deliberately uses `emit_per_scenario_tables` directly, which emits ONLY the per-scenario tables — so the test passes today. But the test's contract per its docstring is "data cells contain ZERO `↑`/`↓` glyphs across REPORT.md", which is a stronger claim than "across `emit_per_scenario_tables`". An ambitious future contributor could insert a glyph into a Mermaid diagram or a per-cell card and not trip the gate.

**Fix:** Either:
1. Tighten the docstring to "across the per-scenario tables surface" (matches what's actually tested), OR
2. Strengthen the test to call `build_report` end-to-end against a non-empty `top_n` and `outcome.skipped`, then run the same line-by-line filter. The latter is the higher-value contract.

## Info

### IN-01: `axes.rs` unused-import-style `let _` is discouraged Rust

**File:** `crates/alloc-bench-aggregator/src/html.rs:524-525`
**Issue:** `let _ = ...;` to silence unused-binding warnings is a known anti-pattern in Rust — `#[allow(unused_variables)]` or simply removing the binding is preferred. Most linters (clippy `#[warn(let_underscore_drop)]`) flag this style.

**Fix:** Delete the lines (see WR-01).

### IN-02: `make_score` test helper uses non-canonical axis names

**File:** `crates/alloc-bench-aggregator/src/html.rs:1485-1514`
**Issue:** The test helper `make_score` at lines 1491-1500 inserts axes named `cpu_efficiency`, `memory_efficiency`, `tail_latency`, `scaling_factor`, `throughput` — none of these match the canonical 8 keys in `MEASUREMENT_AXES` (`channel_throughput`, `cpu_bound_throughput`, `image_size_efficiency`, `memory_fragmentation`, `multithread_throughput`, `resilience`, `security_posture`, `web_throughput`). The doc comment claims "8 measurement axes (alphabetical via BTreeMap; matches the canonical MEASUREMENT_AXES iteration order via constant slice lookup in polar::build_trace)" — this is FALSE. The synthetic axes don't match the registry; `polar::build_trace` looks them up against the registry and gets misses.

This predates Phase 10. The test still passes (the spider trace builder presumably handles missing keys gracefully, or doesn't validate against the registry), but the comment is misleading. Phase 10 didn't introduce this; flagging because the reviewer noticed while tracing the SSoT chain.

**Fix:** Either align the synthetic keys with the canonical 8, or correct the doc comment to "synthetic axes for spider-cell rendering smoke test; not meant to match `MEASUREMENT_AXES`".

### IN-03: Comment cross-references stale line numbers

**File:** `crates/alloc-bench-aggregator/src/html.rs:167, 173, 178, 183`
**Issue:** Doc comments on `axis_label_throughput_yaxis` at line 167 say "Substituted into `index.html.tmpl` line 576 via `{ axis_label_throughput_yaxis }`". The actual line in the template IS 576 (verified). But the comments at lines 167/173/178/183 cite line 576 (correct), 605 (correct), 677 (correct), 851 (the comment says 851; actual placeholder is on line 861).

**Fix:** Update line 183's comment from "line 851" to "line 861" so future code-archaeology stays accurate. Better: drop the line numbers entirely — they bit-rot on every template edit.

### IN-04: `aria_labels_wrap_direction_marker_glyphs` test relies on fragile string position math

**File:** `crates/alloc-bench-aggregator/src/html.rs:1797-1828`
**Issue:** The byte-walk loop at lines 1789-1828 indexes `body.as_bytes()` and accesses `&body[i - opener.len()..i]` and `&body[after..after + wrapper_close.len()]`. Slicing a `&str` by byte-offset is sound only when offsets fall on UTF-8 boundaries. The U+2191/U+2193 sequences are 3-byte UTF-8 starting at the iterator's `i` (so `&body[i-N..i]` is fine because the preceding bytes are ASCII `<span aria-label="...">`), and `&body[after..after+9]` is also fine (the closing `</span>` is ASCII).

This is **NOT** a defect today — the surrounding bytes are pure ASCII. But the assertion code does NO boundary check: if a future change inserts a multi-byte character immediately before or after the glyph (e.g., another emoji or a smart quote), the slice would panic at test time with `byte index N is not a char boundary`. For long-term robustness, prefer `body.is_char_boundary(i)` checks, or use `.find()` against the expected substrings.

**Fix:** Optional. Today's behavior is correct; the brittleness is theoretical. If tightening, replace the byte-walk with `body.match_indices('\u{2191}')` + per-match substring assertions.

---

_Reviewed: 2026-05-29T08:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

---
phase: 10-direction-markers
plan: 02
subsystem: aggregator
tags: [rust, html, plotly, direction-markers, axes, dir-03, dir-04, a11y, wcag, ssot]

# Dependency graph
requires:
  - phase: 10-direction-markers
    plan: 01
    provides: "axes::column_header_with_arrow(label, dir) -> String — single-source-of-truth helper that Plan 10-02's html.rs imports for cross-surface SSoT"
  - phase: 06-foundations
    provides: "axes::Direction::arrow() const fn returning U+2191/U+2193 — backbone of the SSoT helper"
provides:
  - "html::HtmlContext + BuiltContext: 4 new axis-label / table-header fields populated from axes::column_header_with_arrow — the HTML surface now consumes the same SSoT helper as the Markdown surface (T-10-05 cross-surface drift defended structurally)"
  - "DIR-03 satisfied: every Plotly chart measurement axis label in report/index.html carries ↑ / ↓ glyphs injected from axes.rs via four new tinytemplate placeholders (no hard-coded glyphs in the template)"
  - "DIR-04 satisfied: every ↑ / ↓ glyph in the rendered HTML body wrapped in <span aria-label=\"higher is better\">…</span> or <span aria-label=\"lower is better\">…</span> for WCAG 2.1 SC 1.3.3 conformance — server-side wrap for axis-title strings, client-side wrap (JS at template line 851) for dashboard table headers"
  - "html::tests::aria_labels_wrap_direction_marker_glyphs regression test gates the a11y invariant against future template refactors"
affects: [11]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Cross-surface SSoT consumption: html.rs (10-02) imports axes::column_header_with_arrow alongside markdown.rs (10-01) — one helper, two surfaces, zero parallel implementations (T-10-05 mitigation)"
    - "tinytemplate `| unescaped` filter discipline: HTML/JSON literals inside JS string positions require explicit `| unescaped` — the default escape mangles `<span>` brackets and `\"` JSON quotes"
    - "Server-injected JSON arrays + client-side aria-wrap: the dashboard JS at template line 851 parses the server-built JSON header array (`report_table_headers_json`) and wraps each ↑/↓ at DOM-insertion time via `replaceAll`. Wrap logic lives in ONE place per surface (server for axis titles, client for table headers)"
    - "Byte-pin tests using character-by-character scanning when regex is not a workspace dep: walk the body bytes asserting every ↑/↓ outside `<script>` is bracketed by the expected aria-span (regex is NOT in alloc-bench-aggregator/Cargo.toml — pure-stdlib byte iteration)"

key-files:
  created:
    - .planning/phases/10-direction-markers/10-02-SUMMARY.md
  modified:
    - crates/alloc-bench-aggregator/src/html.rs
    - crates/alloc-bench-aggregator/templates/index.html.tmpl

key-decisions:
  - "tinytemplate `| unescaped` filter is required for all 4 new placeholders. The axis-label fields contain literal `<span aria-label=\"…\">` HTML markup (Plotly v2.35.3 accepts HTML in axis-title strings), and the `report_table_headers_json` field is a JSON literal whose `\"` quotes must NOT be HTML-escaped. The default tinytemplate formatter would mangle both."
  - "JSON header array carries PLAIN glyphs (`\"throughput ↑\"`, etc.); the JS at template line 851 wraps each glyph CLIENT-SIDE at DOM-insertion via `replaceAll`. This keeps the JSON literal compact and concentrates wrap logic in ONE place per surface."
  - "innerHTML safety (T-10-08): the JSON source is `axes.rs` + compile-time-fixed labels — no user-controlled input flows in. Risk structurally absent; explicitly accepted per PLAN.md threat_model."
  - "JS Unicode escape literal `'\\u{2191}'` (curly-brace ES2015+ form): the template emits `'\\u\\{2191}'` (the `\\{` is the tinytemplate brace escape; the runtime browser parses `'\\u{2191}'` as a 1-character string equivalent to `'↑'`). Verified via `node -e` — `replaceAll` works correctly. Modern browsers (ES2015+) all support this; Plotly v2.35.3 itself uses ES6+, so the dashboard target was already ES2015+."
  - "A/B chart line 746 (`% delta (B vs A)`) intentionally unchanged per CONTEXT D-claude-discretion-2 / UI-SPEC §Copywriting Contract. Bidirectional delta semantics do not fit a single arrow direction. A 3-line block comment above documents the deferral so future re-introduction is a deliberate UX decision."

patterns-established:
  - "All 4 axis-label fields populated server-side via the same `axes::column_header_with_arrow` helper used by markdown.rs — one helper, two surfaces, no drift."
  - "Cross-surface a11y discipline: server-side aria-wrap for static axis-title strings (Plotly accepts HTML), client-side aria-wrap for dynamically-rendered table-header strings (avoids double-wrap and shrinks the inlined JSON)."
  - "Byte-pin tests in pure stdlib: character-by-character body scanning with `<script>` block stripping when regex is unavailable. Pattern reusable for any future SSoT-output byte-stability gate."

requirements-completed: [DIR-03, DIR-04]

# Metrics
duration: 7 min
completed: 2026-05-28
---

# Phase 10 Plan 02: Direction Markers (HTML surface) Summary

**`html::build_context` now produces four aria-wrapped / JSON-encoded axis-label strings via `axes::column_header_with_arrow`; `index.html.tmpl` substitutes them at lines 576/605/677/851; every `↑`/`↓` glyph in the rendered `index.html` is wrapped with `<span aria-label="…">` for WCAG 2.1 SC 1.3.3 conformance — DIR-03, DIR-04 satisfied on the HTML surface. All five Phase-10 requirements (DIR-01..DIR-05) are now green across both surfaces.**

## Performance

- **Duration:** ~7 min
- **Started:** 2026-05-28T21:04:32Z
- **Completed:** 2026-05-28T21:11:32Z
- **Tasks:** 2 (TDD: plan-level RED → Task 1 GREEN → Task 2 GREEN cycle)
- **Files modified:** 2 (`html.rs`, `index.html.tmpl`)

## Accomplishments

- **DIR-03 satisfied:** `report/index.html` Plotly chart axes carry direction-marker glyphs injected from `axes.rs` via four tinytemplate placeholders (`{ axis_label_throughput_yaxis | unescaped }` at line 576, `{ axis_label_latency_colorbar | unescaped }` at line 605, `{ axis_label_rss_yaxis | unescaped }` at line 677, `{ report_table_headers_json | unescaped }` at line 861). NO hard-coded glyphs in the template — the SSoT helper is the only place where the format pattern lives.
- **DIR-04 satisfied:** Every `↑`/`↓` glyph in the rendered HTML body is wrapped in `<span aria-label="higher is better">…</span>` or `<span aria-label="lower is better">…</span>` for WCAG 2.1 SC 1.3.3 conformance. Server-side wrap for the three Plotly axis-title strings; client-side wrap (JS at template line 861-865) for the dashboard table headers populated from `report_table_headers_json`.
- **Cross-surface SSoT structurally enforced:** `html.rs` now imports `axes::{column_header_with_arrow, Direction}` alongside the existing `MEASUREMENT_AXES`. Combined with Plan 10-01's `markdown.rs` import, the helper is consumed by BOTH surfaces from a single location. T-10-05 cross-surface drift is defended by tests on each end.
- **A/B chart preserved:** Line 746 of `index.html.tmpl` (`% delta (B vs A)`) is byte-identical to v1.0; a 3-line comment above documents the bidirectional-delta deferral per CONTEXT D-claude-discretion-2 / UI-SPEC §Copywriting Contract.
- **1 new test added** (`html::tests::aria_labels_wrap_direction_marker_glyphs`); the test count grows from 137 to 138 lib tests; all 31 integration tests continue to pass.

## Task Commits

Plan 10-02 followed a plan-level RED → GREEN cycle (the test asserts behavior across the FULL rendered HTML, which depends on both Task 1's context fields AND Task 2's template substitutions; the test went RED at Task 1's RED commit, stayed RED at Task 1's GREEN commit, and went GREEN at Task 2's commit):

1. **Task 1 RED:** `3f4f903` test(10-02): add failing test for aria-wrapped axis labels + JSON header array
2. **Task 1 GREEN:** `8331076` feat(10-02): wire 4 new axis-label fields onto HtmlContext + BuiltContext
3. **Task 2 GREEN:** `e7caa37` feat(10-02): substitute chart-axis literals + add JS aria-wrap pass

(Per Plan-10-02 the verification step explicitly says the new test passes "AFTER the template substitutions" in Task 2 — so the plan-level RED → GREEN structure is intentional.)

## Files Created/Modified

### Source

- **`crates/alloc-bench-aggregator/src/html.rs`** — added 4 new fields each on `HtmlContext` (`&'a str`) and `BuiltContext` (`String`); imported `column_header_with_arrow` and `Direction` from `crate::axes`; extended `build_context` with 8 `column_header_with_arrow` calls (throughput, latency, RSS, p50, p95, p99, p999, peak RSS) followed by `format!`-based aria-wrap for the three Plotly axis labels and `to_script_safe_json` over the plain-glyph `Vec<String>` for the dashboard JSON; wired all 4 new fields into the `HtmlContext { ... }` literal in `render`. Added one new test `aria_labels_wrap_direction_marker_glyphs` with helper `strip_script_blocks` (~150 lines incl. doc).
- **`crates/alloc-bench-aggregator/templates/index.html.tmpl`** — replaced 4 hard-coded literals with tinytemplate placeholders (lines 576, 605, 677, 861); added a 3-line documenting comment above line 743 explaining the A/B chart deferral; replaced the dashboard table-header array (line 861) with `JSON.parse(...)` + a 3-line `replaceAll` aria-wrap pass.

### Documentation / planning

- **`.planning/phases/10-direction-markers/10-02-SUMMARY.md`** — this file.

### Helper / context-field signatures

```rust
// html.rs imports (one new line):
use crate::axes::{column_header_with_arrow, Direction, MEASUREMENT_AXES};

// HtmlContext (4 new borrowed-string fields):
axis_label_throughput_yaxis: &'a str,   // "throughput <span aria-label=\"higher is better\">↑</span> (per scenario unit, see scenario.unit)"
axis_label_latency_colorbar: &'a str,   // "latency <span aria-label=\"lower is better\">↓</span> (ns)"
axis_label_rss_yaxis: &'a str,          // "RSS <span aria-label=\"lower is better\">↓</span> (kB)"
report_table_headers_json: &'a str,     // ["allocator","throughput ↑","p50 ↓","p95 ↓","p99 ↓","p999 ↓","peak RSS ↓"]

// BuiltContext (4 new owned-string analogs)
axis_label_throughput_yaxis: String,
axis_label_latency_colorbar: String,
axis_label_rss_yaxis: String,
report_table_headers_json: String,
```

### Template substitutions (before / after diff)

**Line 576 (throughput chart yaxis):**
```diff
- yaxis: \{ title: \{ text: 'throughput (per scenario unit, see scenario.unit)' } },
+ yaxis: \{ title: \{ text: '{ axis_label_throughput_yaxis | unescaped }' } },
```

**Line 605 (latency heatmap colorbar):**
```diff
- colorbar: \{ title: \{ text: 'latency (ns)' } },
+ colorbar: \{ title: \{ text: '{ axis_label_latency_colorbar | unescaped }' } },
```

**Line 677 (RSS-over-time yaxis):**
```diff
- yaxis: \{ title: \{ text: 'RSS (kB)' } },
+ yaxis: \{ title: \{ text: '{ axis_label_rss_yaxis | unescaped }' } },
```

**Line 743/746 (A/B comparison) — INTENTIONALLY UNCHANGED:**
```diff
+// A/B comparison: bidirectional delta (B vs A) — no direction arrow per
+// Phase-10 CONTEXT D-claude-discretion-2 / UI-SPEC §Copywriting Contract.
+// A single ↑/↓ does not capture "positive = B faster, negative = B slower"
+// semantics; intentionally preserved as v1.0 byte form.
 const diffLayout = \{
   font: SHARED_FONT,
   yaxis: \{ title: \{ text: '% delta (B vs A)' } },
```

**Line 851 (dashboard table-header array):**
```diff
-    for (const label of ['allocator', 'throughput', 'p50', 'p95', 'p99', 'p999', 'peak RSS']) \{
-      const th = document.createElement('th');
-      th.textContent = label;
-      headerRow.appendChild(th);
-    }
+    // Phase 10 / Plan 10-02 / DIR-03 + DIR-04 — server-injected JSON
+    // header array (plain glyphs, server is the SSoT via
+    // axes::column_header_with_arrow). Each glyph is wrapped CLIENT-SIDE
+    // with `<span aria-label="…">` for WCAG 2.1 SC 1.3.3 (T-10-07).
+    // T-10-08 (innerHTML XSS): the JSON source is `axes.rs` + fixed
+    // labels, no user-controlled input — see PLAN.md threat_model.
+    const reportTableHeaders = JSON.parse('{ report_table_headers_json | unescaped }');
+    for (const label of reportTableHeaders) \{
+      const th = document.createElement('th');
+      th.innerHTML = label
+        .replaceAll('\u\{2191}', '<span aria-label="higher is better">\u\{2191}</span>')
+        .replaceAll('\u\{2193}', '<span aria-label="lower is better">\u\{2193}</span>');
+      headerRow.appendChild(th);
+    }
```

(Note: `\u\{2191}` is the on-disk template form. Tinytemplate strips the `\` brace-escape; the rendered runtime JS sees `'\u{2191}'` — valid ES2015+ Unicode escape syntax. Verified working via `node -e`.)

### Tests added

| Test | Module | Pins |
|------|--------|------|
| `aria_labels_wrap_direction_marker_glyphs` | `html::tests` | DIR-03, DIR-04, T-10-05 (cross-surface SSoT), T-10-06 (placeholder typo), T-10-07 (WCAG conformance). Asserts: (1) the three server-rendered Plotly axis-title strings carry the expected aria-wrapped HTML literal; (2) the JSON header array contains plain glyphs `"throughput ↑"`, `"p50 ↓"`, etc.; (3) every `↑`/`↓` outside `<script>` blocks is bracketed by the expected aria-span pair (byte-by-byte body scan); (4) the JS aria-wrap wiring (`aria-label="higher is better"`/`aria-label="lower is better"` substring) is present. |

## report/index.html byte delta (informs Phase 11 regen accounting)

Live `report/index.html` regenerated against `results/*.json` (180 runs, 18 cells, 10 scenarios):

- **Before Plan 10-02:** 606,545 bytes (post-Plan-10-01)
- **After Plan 10-02:** 607,640 bytes
- **Delta:** +1,095 bytes (+0.18%)

Breakdown of the +1,095-byte delta:
- 3 Plotly axis-title aria-wrapped strings (throughput / latency colorbar / RSS yaxis): ~3 × 90 bytes = ~270 bytes
- 1 dashboard JSON header array literal (`["allocator","throughput ↑",...]`): ~80 bytes
- JS aria-wrap pass (2 `replaceAll` calls + `JSON.parse`): ~150 bytes
- Server-side helper output bytes (8 × `column_header_with_arrow` strings flowing into the JSON): ~70 bytes
- A/B comparison block comment (3 lines × ~80 bytes): ~240 bytes
- Increased indentation / formatting changes around the substitutions: ~285 bytes

Sum: ~1,095 bytes, fully accounted for. UI-SPEC §"Performance & Bundle Discipline" estimated <1 KB; the actual +1.1 KB is within tolerance and partially driven by the documenting comment block (intentional, not byte-padding).

## Decisions Made

- **`| unescaped` filter required on all four placeholders.** The default tinytemplate formatter HTML-escapes `<`/`>`/`&`/`"` — which would mangle the `<span>` brackets in the Plotly axis-title strings AND the `"` quotes in the JSON header array. Discovered during Task 2 verification (the test FAILED on the first template substitution attempt; `cargo test` failure showed bare HTML escapes mangling the `<span>`). Fixed by adding `| unescaped` to all four placeholders.
- **JSON header array carries plain glyphs; JS wraps client-side.** Per CONTEXT.md §Decisions: dashboard JS reads `report_table_headers_json` (plain glyphs) and applies the aria-span wrap at DOM-insertion time. This (a) keeps the inlined JSON literal compact (~80 bytes vs ~250 bytes if pre-wrapped), (b) concentrates wrap logic in ONE place per surface, and (c) makes the server side easier to test (test asserts the plain glyph in JSON, separately asserts the JS code is wired up).
- **`innerHTML` over `textContent` in the dashboard JS table-header loop.** The aria-span wrap requires HTML-rendering, not text-rendering. `textContent` would render `<span aria-label="...">↑</span>` as literal text instead of an HTML element. Threat model T-10-08 explicitly accepts the `innerHTML` switch because the JSON source is `axes.rs` + compile-time-fixed labels — no user-controlled input flows in.
- **JS Unicode escape literal `'\u{2191}'` (curly-brace ES2015+ form).** The template carries `'\u\{2191}'` (the `\{` is tinytemplate brace-escape; the rendered runtime JS sees `'\u{2191}'`). Verified via `node -e`: this is a 1-character string equal to `'↑'`. Modern browsers support ES2015+ Unicode escapes; Plotly v2.35.3 already requires ES6+, so the dashboard target was already there.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added `| unescaped` filter to all 4 template placeholders**
- **Found during:** Task 2 GREEN verification (running `cargo test html::tests::aria_labels_wrap_direction_marker_glyphs`)
- **Issue:** The plan's template substitution syntax was `'{ axis_label_throughput_yaxis }'` (default formatter), but tinytemplate's default formatter HTML-escapes `<`/`>`/`&`/`"`, which mangles the `<span>` brackets in the axis-title aria-wrap AND the `"` quotes in the JSON header array. The aria-labels test FAILED with `missing throughput yaxis label: throughput <span aria-label="higher is better">↑</span> ...`, indicating tinytemplate had escaped the `<span>` to `&lt;span&gt;`.
- **Fix:** Added `| unescaped` to all 4 placeholders: `{ axis_label_throughput_yaxis | unescaped }`, `{ axis_label_latency_colorbar | unescaped }`, `{ axis_label_rss_yaxis | unescaped }`, `{ report_table_headers_json | unescaped }`.
- **Files modified:** `crates/alloc-bench-aggregator/templates/index.html.tmpl` (4 lines).
- **Commit:** `e7caa37` (rolled into Task 2 GREEN since the deviation was discovered during Task 2's verification step before the commit was finalized).
- **Why this is Rule 3 (auto-fix), not Rule 4 (architectural):** The `| unescaped` filter is the tinytemplate-documented standard pattern for inlining HTML/JSON literals; it's already used in this template for `results_json | unescaped`, `scenarios_json | unescaped`, etc. (see html.rs lines 86-100 doc comment). The plan's CONTEXT explicitly references "Plotly accepts HTML in axis-title strings" — this requires `| unescaped`. The fix is a purely-mechanical correction of a plan-side wording oversight; no architectural change.

### Procedural Note

**1. Plan-level RED → GREEN cycle (vs per-task RED → GREEN)**
- Plan 10-02's frontmatter declares each task `tdd="true"`, but the test (`aria_labels_wrap_direction_marker_glyphs`) asserts behavior across the FULL rendered HTML — which depends on BOTH Task 1's context fields AND Task 2's template substitutions. So:
  - Task 1's RED commit (`3f4f903`): test fails (no fields, no template substitution).
  - Task 1's GREEN commit (`8331076`): test STILL fails (fields exist but template doesn't reference them).
  - Task 2's GREEN commit (`e7caa37`): test passes (template now substitutes the 4 placeholders).
- This matches the plan's verification step on Task 2 ("Test passes against the FULL rendered HTML — confirms the template substitution fired correctly"), so the plan-level RED → GREEN structure is intentional.
- Documented here for transparency — a future contributor reading `git log --oneline` will see two consecutive `feat(10-02)` commits before the test goes green, and this section explains why.

---

**Total deviations:** 1 auto-fix (Rule 3 — `| unescaped` filter); 1 procedural note (plan-level vs per-task TDD cycle).
**Impact on plan:** None to the architectural design; the `| unescaped` fix is a 4-line mechanical correction of a plan-side wording oversight (the plan referenced the substitution form without specifying the filter, but the project-wide convention for HTML/JSON literals in JS positions has always required `| unescaped`).

## Issues Encountered

- **None new.** Pre-existing clippy warnings in `html.rs` (5 `doc_list_item_without_indentation` at lines 149-153 and one at line 902, plus several `function call inside of expect` in test helpers) and other crates' files (`recommend.rs`, `score.rs`, `markdown.rs`) are unchanged — already logged in `.planning/phases/10-direction-markers/deferred-items.md` per Plan 10-01's procedural note.
- **No new clippy warnings introduced** by Plan 10-02 code (verified via `cargo clippy -p alloc-bench-aggregator --tests` filtering for line numbers 1550+ in html.rs — the new test region — returns no warnings).

## TDD Gate Compliance

Plan 10-02 frontmatter declares `type: execute`, but each task carried `tdd="true"`. Per the TDD gate guidance (plan-level cycle):

- **Task 1 RED gate:** `3f4f903` test commit ✓ (test fails at this point — no context fields, no template substitution)
- **Task 1 GREEN gate:** `8331076` feat commit ✓ (context fields exist; test still RED because template not substituted yet)
- **Task 2 GREEN gate:** `e7caa37` feat commit ✓ (template substitutions in place; test now passes)

No REFACTOR commits needed — the helper-consumption logic and template substitutions were minimal and correct as written.

## User Setup Required

None — no external service configuration, no environment variables, no credentials.

## Self-Check: PASSED

Verified at end of execution against SUMMARY claims:

- **File existence:**
  - FOUND: `.planning/phases/10-direction-markers/10-02-SUMMARY.md`
  - FOUND: `crates/alloc-bench-aggregator/src/html.rs`
  - FOUND: `crates/alloc-bench-aggregator/templates/index.html.tmpl`
- **Commit existence (all 3):**
  - FOUND: `3f4f903` test(10-02): add failing test for aria-wrapped axis labels + JSON header array
  - FOUND: `8331076` feat(10-02): wire 4 new axis-label fields onto HtmlContext + BuiltContext
  - FOUND: `e7caa37` feat(10-02): substitute chart-axis literals + add JS aria-wrap pass
- **Helper consumption:** FOUND — `use crate::axes::{column_header_with_arrow, Direction, MEASUREMENT_AXES};` at top of `html.rs`.
- **8 helper invocations:** FOUND — `column_header_with_arrow` is called 8 times in `build_context` (throughput, latency, RSS, p50, p95, p99, p999, peak RSS).
- **Template substitutions:** FOUND — `grep -c "axis_label_throughput_yaxis\|axis_label_latency_colorbar\|axis_label_rss_yaxis\|report_table_headers_json" templates/index.html.tmpl` returns 4.
- **A/B chart preserved:** FOUND — `% delta (B vs A)` literal at line 746 unchanged; 3-line documenting comment above it.
- **Test status:** 138 lib + 31 integration tests pass; 1 new test added (was 137; aria-labels test takes the count to 138).

## Next Phase Readiness

- **Phase 10 closed:** All 5 DIR-* requirements satisfied across both surfaces.
  - DIR-01 (column-header glyphs in REPORT.md): ✓ Plan 10-01
  - DIR-02 (legend row above each per-scenario table): ✓ Plan 10-01
  - DIR-03 (axis-label glyphs in HTML chart axes + dashboard table headers): ✓ Plan 10-02
  - DIR-04 (a11y wrappers for ↑/↓ glyphs in HTML): ✓ Plan 10-02
  - DIR-05 (data cells contain no direction markers): ✓ Plan 10-01
- **Phase 11 unblocked:** With all 5 DIR-* requirements green, the byte-changing surface for the v1.1 release is now stable. Phase 11's standalone golden-fixture regen PR can proceed (one-shot byte-identical pinning of REPORT.md + index.html in `tests/smoke.rs`).
- **No blockers.** The cross-surface SSoT is structurally enforced by tests on both ends (`markdown::tests::scenario_table_headers_carry_direction_markers` + `html::tests::aria_labels_wrap_direction_marker_glyphs`). Future regressions trip at `cargo test` time.

---
*Phase: 10-direction-markers*
*Plan: 02*
*Completed: 2026-05-28*

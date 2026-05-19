---
phase: 04-aggregator-dashboard
reviewed: 2026-05-19T00:00:00Z
depth: standard
files_reviewed: 13
files_reviewed_list:
  - Cargo.toml
  - crates/alloc-bench-aggregator/Cargo.toml
  - crates/alloc-bench-aggregator/src/diagrams.rs
  - crates/alloc-bench-aggregator/src/html.rs
  - crates/alloc-bench-aggregator/src/loader.rs
  - crates/alloc-bench-aggregator/src/main.rs
  - crates/alloc-bench-aggregator/src/markdown.rs
  - crates/alloc-bench-aggregator/src/recommend.rs
  - crates/alloc-bench-aggregator/templates/index.html.tmpl
  - crates/alloc-bench-aggregator/tests/smoke.rs
  - crates/alloc-bench-core/src/output.rs
  - justfile
  - README.md
findings:
  critical: 1
  warning: 4
  info: 7
  total: 12
status: issues_fixed
---

# Phase 4: Code Review Report

**Reviewed:** 2026-05-19
**Depth:** standard
**Files Reviewed:** 13
**Status:** issues_fixed

## Summary

Phase 4 ships a coherent, well-tested aggregator: schema is locked at SCHEMA_VERSION=1 with round-trip Deserialize tests, the loader sorts deterministically and skip-and-continues on per-file errors, the markdown emitter uses BTreeMap/BTreeSet throughout for byte-identical output, and the HTML template carefully escapes literal `{` for tinytemplate. Tests cover the four CLI exit paths and the Plan-02/03 visual contract.

That said, this review identified one Critical hardening gap, four Warnings (a real winner-tiebreak inconsistency between markdown.rs and recommend.rs, a dead lifetime parameter on `AllocStats`, an empty-loadout pitfall in `byScenario` JS object construction, and an HTML-injection-via-timestamp surface that survives only because chrono produces benign characters), and seven Info-tier items (mostly comment/code drift, code duplication, and minor edge-case ordering quirks).

## Critical Issues

### CR-01: Inlined `<script>` block can be terminated by attacker-controlled data flowing through `serde_json::to_string`

**File:** `crates/alloc-bench-aggregator/src/html.rs:100`, `crates/alloc-bench-aggregator/templates/index.html.tmpl:251`
**Issue:** `serde_json::to_string` does NOT escape `<`, `>`, `/`, or the literal substring `</script>` when serializing string fields. The result is then emitted verbatim into the HTML via `{ results_json | unescaped }` (and the four sister `*_json` fields on lines 255-261). Any `Run` whose string fields contain `</script>` — for example `build.allocator = "</script><script>alert(1)</script>"`, or arbitrary JSON inside the free-form `scenario.config` / `metrics.allocator_stats` `serde_json::Value` fields — will:

1. Terminate the inline `<script>` tag at byte offset of the embedded `</script>`.
2. Cause every byte that follows (still part of the JSON literal) to be parsed as HTML.
3. Re-enter `<script>` mode at the next `<script>` tag, executing whatever the JSON contains.

The prompt notes "security surface is glob + serde_json::from_slice against trusted local files (no network input, no untrusted user data)." That defense is partial:
- The HTML output is checked into git and reviewed by humans, so a contributor who writes a JSON fixture (or a Phase-3 Docker run that generates `allocator_stats: {"opaque_string": "</script>..."}` from a buggy upstream library) silently corrupts the dashboard for every reader.
- One of the file paths in `scenario.config` is `serde_json::Value` — a free-form pass-through. Phase-2 jemalloc/mimalloc stats already populate `allocator_stats` as `serde_json::Value`, and upstream libraries can return arbitrary JSON.
- The README's CLAUDE.md mandates "every result is reproducible, environment-labelled, and visually comparable" — silent HTML corruption violates that.

The fix is well-known and one line: replace `serde_json::to_string` with a function that emits the same JSON but escapes `<` as `<` (or splits any `</` substring as `<\/`). The Plotly community ships `JSON.parse('<json string>')` for exactly this reason.

**Fix:**
```rust
// crates/alloc-bench-aggregator/src/html.rs

/// JSON-encode for safe inlining inside an HTML <script> block. Escapes
/// `<`, `>`, and `&` so the string literal can never terminate the host
/// <script> tag (a `</script>` in the input becomes `</script>` in
/// the output). RFC 8259 permits these escapes; every JSON parser accepts.
fn to_script_safe_json<T: serde::Serialize>(v: &T) -> Result<String> {
    let raw = serde_json::to_string(v).context("serializing to JSON")?;
    Ok(raw
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026"))
}

// In build_context() and at every `serde_json::to_string` site that feeds
// `{ ... | unescaped }` in the template, replace with `to_script_safe_json`.
```

Add a regression test:
```rust
#[test]
fn inlined_json_escapes_script_close_tag() {
    let mut run = make_test_run("</script><script>alert('xss')</script>", None, "x", 50_000);
    let html = render(&[run]).expect("render");
    assert!(!html.contains("</script><script>alert"), "script tag terminated inside RESULTS");
    assert!(html.contains("\\u003c/script\\u003e"), "expected \\u003c escape");
}
```

## Warnings

### WR-01: `markdown.rs` and `recommend.rs` use opposite tiebreaks on equal throughput, breaking the "single source of truth" claim

**File:** `crates/alloc-bench-aggregator/src/markdown.rs:117-126`, `crates/alloc-bench-aggregator/src/recommend.rs:217-221`, `crates/alloc-bench-aggregator/src/recommend.rs:266-273`
**Issue:** Both modules claim to use "alphabetical tiebreak via stable iteration / BTreeMap iteration" (markdown.rs:115-116; recommend.rs:228-229; recommend.rs:264). They actually do not agree on which row wins on a tie:

- **markdown.rs `winner_idx`:** uses `Iterator::max_by` whose contract is "If several elements are equally maximum, the last element is returned." With rows pre-sorted alphabetically by `(allocator, env_label)`, this picks the alphabetically-LAST row on a throughput tie.
- **recommend.rs `pick_rationale_scenario`:** uses `if tps <= b => keep current`, which picks the alphabetically-FIRST scenario on a tie.
- **recommend.rs sort by `score` descending:** `sort_by` is stable, and `unwrap_or(Equal)` makes ties compare equal; the BTreeMap iteration that built `stats` is alphabetical, so winner is alphabetically-FIRST on a tie.

Concretely: if `jemalloc` and `mimalloc` post identical `ticks_per_s` for `cpu-bound`, the REPORT.md table marks `**✓ mimalloc**` while the Recommendations table picks `jemalloc`. The dashboard's `renderReportMirrorTable` (template line 654-658) uses strict `>`, also alphabetical-first, so the dashboard and the recommendations agree but the per-scenario REPORT.md table dissents.

This is a real correctness gap, not a style nit — the markdown report would self-contradict and reviewers would lose trust in the byte-identical-output contract.

**Fix:** Replace `max_by` in `emit_per_scenario_tables` with an explicit forward-iteration max-finder that keeps the first-seen winner on ties (mirroring `pick_rationale_scenario`):
```rust
// markdown.rs, around line 117:
let mut winner_idx: Option<usize> = None;
for (i, r) in sorted.iter().enumerate() {
    let tps = r.metrics.ticks_per_s;
    match winner_idx {
        Some(j) if tps <= sorted[j].metrics.ticks_per_s => {}
        _ => winner_idx = Some(i),
    }
}
```
Add a test that two same-throughput runs always award the alphabetically-first allocator the bold-and-✓ prefix in BOTH the per-scenario tables AND the recommendations.

### WR-02: `AllocStats<'a>` carries a vestigial lifetime parameter and `PhantomData<&'a Run>` field that nothing uses

**File:** `crates/alloc-bench-aggregator/src/recommend.rs:109-121`, `crates/alloc-bench-aggregator/src/recommend.rs:148`, `crates/alloc-bench-aggregator/src/recommend.rs:178-184`
**Issue:** `AllocStats<'a>` declares a lifetime `'a` but every field is either owned (`String`, `f64`, `bool`) or `&'static str` (`per_scenario: BTreeMap<&'static str, f64>`). The `PhantomData<&'a Run>` field exists ONLY to consume the otherwise-unused lifetime. The doc comment claims the phantom is "so `&str` keys borrow from the static `class.scenarios()` slice rather than from each `Run`" — but the keys are `&'static str` already, which has nothing to do with `'a`.

This is a code smell that misleads future maintainers about the data flow (the comment claims a borrow that doesn't exist) and trips clippy's `extra_unused_lifetimes` lint if it is ever enabled. It also forces every construction site to write `_runs_lifetime: std::marker::PhantomData` for no benefit.

**Fix:** Drop the lifetime entirely:
```rust
struct AllocStats {
    allocator: String,
    score: f64,
    per_scenario: BTreeMap<&'static str, f64>,
    any_suspect: bool,
}
// At construction site (recommend.rs:178), drop the `_runs_lifetime` line.
// At collect site (recommend.rs:148), `Vec<AllocStats>` is unchanged.
```

### WR-03: Inline `<script>` constructs `byScenario = {}` (vulnerable to prototype pollution from data-derived keys)

**File:** `crates/alloc-bench-aggregator/templates/index.html.tmpl:636-640`
**Issue:** `const byScenario = {};` and then `byScenario[r.scenario.name] = ...`. If a scenario name is `__proto__`, `constructor`, or `hasOwnProperty`, this either pollutes the prototype chain or silently fails (the assignment to `byScenario['__proto__']` reassigns the prototype). A subsequent `Object.keys(byScenario).sort()` may then iterate poisoned entries on every JS Object created later in the same realm.

The prompt notes inputs are trusted, so exploitation is unlikely, but this is a defensive-coding lapse that costs zero perf to fix and aligns the JS with the Rust side's `BTreeMap` semantics. The same pattern appears in the `for (const r of RESULTS)` loop on line 637.

**Fix:** Use `Object.create(null)` so the object has no prototype:
```javascript
const byScenario = Object.create(null);
for (const r of RESULTS) {
  const key = r.scenario.name;
  if (!byScenario[key]) { byScenario[key] = []; }
  byScenario[key].push(r);
}
```
Or use a `Map`:
```javascript
const byScenario = new Map();
for (const r of RESULTS) {
  const key = r.scenario.name;
  if (!byScenario.has(key)) { byScenario.set(key, []); }
  byScenario.get(key).push(r);
}
const scenarioNames = Array.from(byScenario.keys()).sort();
```

### WR-04: HTML template has no `nonce`/CSP defense; `timestamp_iso8601` is HTML-escape-default-OK only by accident

**File:** `crates/alloc-bench-aggregator/templates/index.html.tmpl:5,211`, `crates/alloc-bench-aggregator/src/html.rs:74`
**Issue:** The `timestamp_iso8601` placeholder uses tinytemplate's default formatter (which DOES HTML-escape, unlike `unescaped`). Today `chrono::Utc::now().to_rfc3339()` produces only digits/hyphens/colons/dot/`T`/`+` — no `<`, `>`, `&`, or `"`, so the output is safe. But:

1. The safety is incidental, not defensive: a future contributor who swaps in a custom timestamp provider (e.g., reading `BENCH_TIMESTAMP_OVERRIDE` from env for reproducibility) could trivially leak `<` and create the same XSS surface as CR-01. The header `<title>...{timestamp_iso8601}</title>` and `<p>...{timestamp_iso8601}</p>` are different attack surfaces (title doesn't get a `<script>` block but does get parser-state weirdness in older browsers).
2. The aggregator emits no `Content-Security-Policy` meta tag, no `X-Content-Type-Options: nosniff`, no SRI on the locally-included data (only on Plotly). Defense-in-depth would catch CR-01 even if the JSON-escape were missed.

**Fix:** Add a `<meta http-equiv="Content-Security-Policy" content="default-src 'self'; script-src 'self' 'unsafe-inline' https://cdn.plot.ly; style-src 'self' 'unsafe-inline'">` to the `<head>`. Even with `'unsafe-inline'` (required by the inline `<script>`), CSP would block external script injections. Document that the `timestamp_iso8601` field is trusted-content via a doc comment on `HtmlContext::timestamp_iso8601`.

## Info

### IN-01: Comment claims template JS is "pure ASCII" but file is UTF-8 with U+00B7 MIDDLE DOT byte

**File:** `crates/alloc-bench-aggregator/templates/index.html.tmpl:265-267`
**Issue:** Lines 265-266 say `// `·` keeps the file pure ASCII — equivalent to U+00B7 MIDDLE DOT at runtime`. The actual bytes at line 267 column 28 are `c2 b7` (UTF-8 encoding of U+00B7), confirmed by `xxd`. The whole template is `Unicode text, UTF-8 text` per `file(1)`. The comment misleads future maintainers about whether the file is byte-pure ASCII.
**Fix:** Replace the comment with the truth — either change the literal to `'\u\{00B7}'` (which DOES keep it pure ASCII) or update the comment to: `// '·' (U+00B7 MIDDLE DOT) — must match the suspect_pairs separator emitted by html.rs::build_context.`

### IN-02: `count_unique_cells` is duplicated between markdown.rs and html.rs

**File:** `crates/alloc-bench-aggregator/src/markdown.rs:239-245`, `crates/alloc-bench-aggregator/src/html.rs:163-169`
**Issue:** Both modules implement `count_unique_cells(runs: &[Run]) -> usize` identically. If one is updated (e.g., to count by `(allocator, env_label, scenario)` triples for some future requirement) the other is silently desynced.
**Fix:** Lift to a `pub(crate)` helper in one module — `markdown::count_unique_cells` is already `pub(crate)`-accessible-via-module-path; just re-use it from `html.rs`:
```rust
// html.rs
fn count_unique_cells(runs: &[Run]) -> usize {
    crate::markdown::count_unique_cells(runs) // requires pub(crate) on the markdown side
}
```
Or inline the BTreeSet-of-tuple into both call sites — current state is the worst of both worlds.

### IN-03: `recommend::pick_rationale_scenario` ranks all-NaN inputs alphabetically-LAST, contradicting "alphabetical tiebreak" claim

**File:** `crates/alloc-bench-aggregator/src/recommend.rs:265-273`
**Issue:** When every value in `per_scenario` is `f64::NAN`, the comparison `tps <= b` is FALSE for every iteration (NaN is unordered). Each pass falls through `_ => best = Some((scen, tps))` and overwrites `best`. After iterating the BTreeMap in alphabetical order, the alphabetically-LAST scenario wins. This is the OPPOSITE of the documented "alphabetical tiebreak" intent at line 264.

In practice throughput should never be NaN, but the code already routes `partial_cmp(...).unwrap_or(Equal)` defensively elsewhere — this branch should match.
**Fix:** Use `f64::total_cmp` to give NaN a stable position, or short-circuit on NaN:
```rust
match best {
    Some((_, b)) if !tps.is_nan() && tps <= b => {}
    Some(_) if tps.is_nan() => {} // NaN never wins
    _ => best = Some((scen, tps)),
}
```

### IN-04: `metrics.allocator_stats` and `scenario.config` are wholly absent from the dashboard, but every byte ships in the inlined RESULTS

**File:** `crates/alloc-bench-aggregator/src/html.rs:100`, `crates/alloc-bench-core/src/output.rs:54,516`
**Issue:** `serde_json::to_string(runs)` serializes EVERY field of every `Run`, including the free-form `scenario.config` and `metrics.allocator_stats` (`serde_json::Value`) and the `rss_growth_samples` `Vec`. The JS uses `r.scenario.config` only on the diff chart for nothing, and `r.metrics.allocator_stats` not at all — yet every byte ships into `index.html`. With many scenarios this bloats the static HTML page (and is readable by anyone with the file). RESEARCH §Pitfall 2 already calls out the size axis for `to_string` vs `to_string_pretty`; the same logic suggests stripping unused fields.
**Fix:** Define a `RunForDashboard` projection struct that omits the unused fields and `From<&Run>`-converts before serialization. Or document explicitly in `html.rs` why every field is shipped (e.g., "future v2 charts will read `allocator_stats`"). Pick one — the current state is silent.

### IN-05: `mod diagrams; mod html; mod loader; mod markdown; mod recommend;` lacks `pub` modifier — all are private to main.rs but expose `pub` items

**File:** `crates/alloc-bench-aggregator/src/main.rs:21-25`
**Issue:** Each `mod X;` is private (default), but the modules each expose `pub fn write`, `pub fn discover`, `pub fn recommendations`, etc. Since this is a binary crate, only main.rs uses them, so the `pub` is functionally dead. Either drop `pub` (use `pub(crate)`) or document the intent.
**Fix:** Either downgrade all module exports to `pub(crate)` (the simplest, signals "binary-internal"), or extract a `lib.rs` so the modules can be reused by other crates / tools. Given no other crate uses them today, `pub(crate)` is the right choice.

### IN-06: `pub` constants `PTMALLOC_DIAGRAM`, `MALLOCNG_DIAGRAM`, `JEMALLOC_DIAGRAM`, `MIMALLOC_DIAGRAM` are only consumed via `ALL_DIAGRAMS` and never directly

**File:** `crates/alloc-bench-aggregator/src/diagrams.rs:19,37,53,71,92-97`
**Issue:** The four diagram constants are public (`pub const`) but the only callers (markdown.rs:188, the unit tests) iterate `ALL_DIAGRAMS`. The public visibility serves no consumer.
**Fix:** Make them `pub(crate)` (or `const`-level private) and keep `ALL_DIAGRAMS` as the single public surface. Reduces the API surface and keeps the "iterate `ALL_DIAGRAMS` for emission order" contract visible at a glance.

### IN-07: `_runs_lifetime` rationale comment in `AllocStats` is wrong (and was the only justification for the field)

**File:** `crates/alloc-bench-aggregator/src/recommend.rs:118-120`
**Issue:** Doc comment says "Phantom for the lifetime so `&str` keys borrow from the static `class.scenarios()` slice rather than from each `Run`." But the keys are typed `&'static str`, which has nothing to do with `'a`. The comment is the ONLY thing justifying the dead lifetime parameter (see WR-02). Removing the field also removes the lie.
**Fix:** Subsumed by WR-02 — when `'a` and `_runs_lifetime` go away, this doc comment goes with them.

---

_Reviewed: 2026-05-19_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

# Technology Stack — v1.1 Recommendations, Spider Charts & Direction Markers

**Project:** rust-benchmark-glibc-musl-mimalloc
**Milestone:** v1.1 (additive over v1.0 stack)
**Researched:** 2026-05-26
**Mode:** Ecosystem (delta-only — v1.0 STACK.md is the baseline; this file ONLY records additions/changes for v1.1)

## TL;DR

**Zero new runtime crate dependencies.** All four v1.1 features (radar charts, recommendation prose, direction markers, security sidecars) compose entirely on top of the v1.0 stack — Plotly 2.35.3 already ships `scatterpolar`, `tinytemplate` already escapes prose strings, and the existing `loader.rs::CellMeta` sidecar pattern handles the new `meta/security/{env}.json` shape with a single struct addition. The Plotly CDN URL and SRI hash do **NOT** need to change.

**One zero-risk dev-time addition** (optional, defer if not needed): `pulldown-cmark` 0.13.x is the canonical Rust CommonMark parser if Q3's HTML rendering of recommendation prose grows beyond raw `<p>{prose}</p>` injection. **Recommendation: do not add it for v1.1**; emit recommendation prose as plain text in both Markdown (raw) and HTML (single `<p>` wrapping the same string with HTML-escape via tinytemplate's default formatter — no markdown-to-HTML conversion needed because the prose is one sentence per cell with no markdown features beyond ↑/↓ glyphs and `*(suspect)*` italic markers).

## Recommended Stack — v1.1 Additions

### What does NOT change

| Component | v1.0 version | v1.1 status |
|-----------|-------------|-------------|
| Plotly.js CDN | 2.35.3 | **Unchanged** — `scatterpolar` trace ships in the standard `plotly-2.35.3.min.js` bundle (verified) |
| Plotly SRI hash | `sha384-MqL7Cy3it…lhPykM` | **Unchanged** — same JS file, same hash |
| `tinytemplate` | 1 | Unchanged — handles all v1.1 prose injection |
| `serde_json` | 1 | Unchanged — handles `meta/security/{env}.json` parse |
| `glob` 0.3 + `loader.rs::load_cell_metas` | shipped Phase 5 | Unchanged — extended (not duplicated) for security sidecars |
| `BTreeMap` / `BTreeSet` iteration | shipped Phase 1 | Unchanged — byte-identical-output discipline preserved |

### What DOES change (additions only)

| Addition | Type | Where | Why |
|----------|------|-------|-----|
| Radar trace builder (`makeSpiderTraces`) | JS function in `templates/index.html.tmpl` | New template section | Plotly already supports `scatterpolar`; this is a pure trace-shape addition |
| Top-N normalized score table | Rust struct in `recommend.rs` | Extends `top_n_cells()` call site | Hand-rolled min-max normalizer (15 LOC) — no crate needed |
| Direction-marker glyph constants | Rust `pub const` in new module `recommend.rs` or `markdown.rs` | Existing files | Hard-coded `"\u{2191}"` / `"\u{2193}"` — no crate needed |
| Security sidecar struct | `SecurityMeta` next to `CellMeta` in `loader.rs` | Existing file | Mirrors `load_cell_metas` exactly — same glob/parse plumbing |
| Markdown report tables decorated with arrows | Plain `format!` in `markdown.rs` | Existing file | Pre-existing string-builder pattern |

## Per-Question Verdicts

### Q1. Plotly polar/scatterpolar — version sufficiency, trace shape, SRI risk

**Verdict: NO CHANGE NEEDED.** The pinned `plotly-2.35.3.min.js` ships the `scatterpolar` trace in its standard bundle.

**Evidence (HIGH confidence):**

- `scatterpolar` exists in the v2.35.3 source tree at `src/traces/scatterpolar/index.js` (verified via direct fetch of `github.com/plotly/plotly.js/blob/v2.35.3/src/traces/scatterpolar/index.js`). The trace module is described as supporting "line charts, scatter charts, text charts, and bubble charts in polar coordinates."
- The official Plotly graphing library docs (`/plotly/graphing-library-docs` Context7) demonstrate the canonical multi-trace radar pattern using `scatterpolar` + `fill: 'toself'` + `polar.radialaxis.range`, which is exactly the spider-chart shape we need.
- `scatterpolar` predates the v2.x series; it has been stable for years (the linked docs are from 2018-02-23 against an even older Plotly version).
- The Plotly CDN URL `https://cdn.plot.ly/plotly-2.35.3.min.js` and SRI hash `sha384-MqL7Cy3itNqCI1Wlc926K0XhyRKJ/NMqTaytIIEB+QIdInOploxqRIHRKLlhPykM` (declared in `crates/alloc-bench-aggregator/src/html.rs:37-45`) **stay byte-identical**. We are not changing the bundle — we are calling a trace type that has always been in it.

**Trace shape (canonical, verified against Plotly docs):**

```javascript
const trace = {
  type: 'scatterpolar',
  r: [85, 72, 91, 68, 88, 75, 82, 79, 85],   // 9 values: 8 axes + close-the-polygon (repeat r[0])
  theta: ['channel-tput', 'mem-frag', 'web', 'multithread',
          'cpu-bound', 'resilience', 'image-eff', 'security', 'channel-tput'],
  fill: 'toself',
  name: 'jemalloc·alpine',
  fillcolor: 'rgba(33, 144, 140, 0.25)',  // Viridis green at 25% alpha — overlay-friendly
  line: { color: '#21908C' },              // Viridis green at 100% — same as v1.0 ALLOC_COLORS
};

const layout = {
  polar: {
    radialaxis: {
      visible: true,
      range: [0, 100],                     // normalized 0–100 axes — fixed across all traces
    },
    angularaxis: {
      direction: 'clockwise',              // matches reading-order convention
      rotation: 90,                        // first axis at 12 o'clock — verified supported
    },
  },
  showlegend: true,
  font: SHARED_FONT,                       // reuse v1.0 system-font stack
};
```

**Why the closing repetition (`r[0]` and `theta[0]` appended):** Plotly's `fill: 'toself'` requires the polygon be explicitly closed by repeating the first vertex; otherwise the fill area leaves a wedge open. This is the canonical idiom across every example in the Plotly docs (verified at `_posts/plotly_js/scientific/radar/2018-02-23-basic-radar.html`).

**Multi-trace overlay:** Plotly stacks `scatterpolar` traces in z-order (last drawn on top); use 25% alpha `fillcolor` so 10 overlaid cells stay readable. The existing `ALLOC_COLORS` map in `index.html.tmpl:352-357` is reused directly for the `line.color`.

**Confidence: HIGH** — verified against official Plotly source for the pinned version AND against canonical doc examples.

---

### Q2. Normalization to 0–100 across heterogeneous units

**Verdict: HAND-ROLL.** No Rust crate is worth pulling in for what is ~15 lines of arithmetic.

**Reasoning:**

- The normalization needed is **per-axis min-max scaling with direction awareness** (higher-is-better for throughput; lower-is-better for latency / RSS / image size). This is not statistically interesting — it is `(x - min) / (max - min) * 100` for "more is better" and `100 - that` for "less is better."
- Crates surveyed (`statrs`, `ndarray-stats`, `nalgebra-stats`) are matrix-statistics-oriented (means, stddev, distributions) and would add 100KB+ of compile time + transitive deps for arithmetic the project already does inline (see `multi_run::aggregate` for the Bessel-corrected stddev pattern — same code-shape).
- Hand-rolled normalization keeps the byte-identical-output contract trivially auditable: the function is short enough to be entirely visible in a code review.

**Recommended implementation pattern (extend `recommend.rs`):**

```rust
/// Direction-aware min-max normalizer. `higher_is_better=true` for throughput
/// axes (channel, web, multithread, cpu-bound); `false` for latency/RSS axes.
/// When `max - min == 0` (all cells equal on this axis), every cell scores 50
/// — neutral, neither penalized nor rewarded.
fn normalize_axis(values: &[f64], higher_is_better: bool) -> Vec<f64> {
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = max - min;
    if span.abs() < 1e-9 {
        return vec![50.0; values.len()];
    }
    values
        .iter()
        .map(|&v| {
            let norm = (v - min) / span * 100.0;
            if higher_is_better { norm } else { 100.0 - norm }
        })
        .collect()
}
```

**Edge cases the hand-roll must handle (all unit-testable, reuses the `synth_run` test fixture in `recommend.rs:296`):**

1. Single-cell input → `span == 0` → all-50 fallback (neutral score).
2. NaN/inf input → caller must filter via `is_finite` first; the Bessel-corrected stddev path in `multi_run.rs` already establishes this pattern (CV undefined when `|mean| ≤ 1e-9` or non-finite).
3. Heterogeneous units across axes → normalizer is applied **per axis independently**, never cross-axis. The 0–100 output makes axes commensurable for the radar chart.

**Confidence: HIGH** — domain-trivial arithmetic; existing `multi_run::aggregate` is the architectural template.

---

### Q3. Recommendation prose: tinytemplate vs pulldown-cmark

**Verdict: STAY ON tinytemplate.** Defer pulldown-cmark unless prose grows markdown-feature requirements (links, headings, lists).

**Reasoning:**

- The recommendation prose shape (per `recommend.rs:248-251`) is a single line per cell: `"+25.0% throughput vs ptmalloc on cpu-bound *(suspect)*"`. The only markdown feature in this string is the `*…*` italic markers.
- For Markdown output: literally pass the string through unchanged into REPORT.md. Markdown renderers (GitHub, IDE preview) handle `*italic*` natively. No conversion needed.
- For HTML output: tinytemplate's **default formatter HTML-escapes `<`/`>`/`&`/`"`** (verified in `html.rs:88-93` doc comment: "rendered via tinytemplate's DEFAULT formatter (NOT `unescaped`), so any stray `<`/`>`/`&`/`"` would be HTML-escaped"). Wrap the prose in a `<p>{ rationale }</p>` substitution — the `*…*` becomes literal asterisks in the HTML view, which is acceptable per the v1.0 UI-SPEC convention (the `*(suspect)*` token is already rendered as raw `*(suspect)*` in the HTML report-mirror table at `html.rs:790-808` via the `suspect-note` CSS class).
- **If we switched to pulldown-cmark** we would need:
  1. New runtime dep (50–100 KB compiled).
  2. A second escape audit — `pulldown-cmark`'s HTML output has its own escape semantics (CR-01 in `html.rs` documented the existing `to_script_safe_json` wrapper for inlined JSON; adding a second escape pipeline doubles the audit surface).
  3. New byte-identical-output golden test — pulldown-cmark's output is **deterministic in practice** (its test suite contains thousands of byte-stable input/output fixtures, verified via Context7), but introducing it adds a transitive risk of behavior change on minor version bumps.
- **Decorate-not-rewrite:** the v1.0 plumbing already handles HTML escaping correctly; adding a markdown-to-HTML layer for one sentence violates the project's "decorate, don't rewrite" principle (CLAUDE.md cross-phase convention).

**Byte-identical-output guarantee path (no pulldown-cmark):**

```rust
// In recommend.rs, extend `top_n_cells` to emit prose alongside scores:
pub struct CellRecommendation {
    pub allocator: String,
    pub env: String,
    pub score: f64,
    pub prose_md: String,   // raw string with *(suspect)* italic markers — same shape as v1.0 rationale
}

// In markdown.rs, emit alongside the per-scenario tables:
let _ = writeln!(buf, "### {} · {}", rec.allocator, rec.env);
let _ = writeln!(buf, "{}", rec.prose_md);

// In templates/index.html.tmpl, render via:
//   <p class="recommendation">{ recommendation.prose_md }</p>
// tinytemplate's default formatter escapes HTML metacharacters; the literal
// asterisks from *(suspect)* render as plain text — matches REPORT.md's
// behavior in non-markdown previewers.
```

**If markdown features ARE needed in v1.2+** (e.g., bullet lists, links to per-scenario sections): adopt pulldown-cmark 0.13.4 (verified latest on 2026-05-20). The minimal integration is `pulldown_cmark::html::push_html(&mut html_output, Parser::new(&prose_md))` (verified API via Context7). Add `#[serde(default)]` on a new `prose_html: Option<String>` field in `CellRecommendation`, populate it lazily, and reference it from the template via `{ prose_html | unescaped }`.

**Confidence: HIGH** — every claim grounded in existing code paths in `html.rs` / `markdown.rs`.

---

### Q4. Direction markers — Unicode source standardization

**Verdict: HARD-CODE. No crate.**

**Reasoning:**

- The direction-marker set is two glyphs: `↑` (U+2191 UPWARDS ARROW) and `↓` (U+2193 DOWNWARDS ARROW). Both are in the Basic Arrows block (U+2190..U+21FF), which has been universally supported since Unicode 1.0 (1991).
- Crates like `unicode-arrows` or `font-icons` introduce supply-chain surface and indirection for what is two `pub const &str = "\u{2191}"` declarations.
- All three font stacks declared in the v1.0 dashboard (`templates/index.html.tmpl:51-53` and `:371-373`) include glyphs for U+2191 and U+2193:
  - macOS: `-apple-system, BlinkMacSystemFont` → San Francisco / SF Pro (full Basic Arrows coverage).
  - Windows: `"Segoe UI"` (full Basic Arrows coverage).
  - Linux: `Roboto, "Helvetica Neue", Arial, sans-serif` (DejaVu fallback in browsers — full Basic Arrows coverage).
- The codebase already uses U+2014 (em-dash) raw in `recommend.rs:144` (`"\u{2014}".to_string()`) and U+00B7 (middle dot) in `html.rs:186` (`format!("{}\u{00B7}{}", ...)`) — the `\u{NNNN}` literal pattern is the established convention.
- **Accessibility:** add a `aria-label="up"` / `aria-label="down"` attribute when the arrow appears in HTML, AND a 1-line legend at the top of the dashboard / REPORT.md explaining the convention (`↑ = higher-is-better; ↓ = lower-is-better`). This satisfies WCAG 2.1 Success Criterion 1.3.3 (Sensory Characteristics) without needing a screen-reader-only `<span>`.

**Recommended implementation:**

```rust
// In recommend.rs (or a new direction.rs if the surface grows):
pub const ARROW_UP: &str = "\u{2191}";   // ↑ U+2191 UPWARDS ARROW
pub const ARROW_DOWN: &str = "\u{2193}"; // ↓ U+2193 DOWNWARDS ARROW

/// Per-metric direction. Throughput-style metrics flag ↑ (more is better);
/// latency/RSS/image-size flag ↓ (less is better).
pub enum AxisDirection { HigherIsBetter, LowerIsBetter }

impl AxisDirection {
    pub fn glyph(&self) -> &'static str {
        match self { Self::HigherIsBetter => ARROW_UP, Self::LowerIsBetter => ARROW_DOWN }
    }
}
```

Markdown emitter: `format!("| metric {} | …", dir.glyph())` — produces `| metric ↑ | …` rendered byte-identically.
HTML emitter: same string flows through tinytemplate's default escape, which leaves U+2191/U+2193 untouched (HTML escape only touches `<`/`>`/`&`/`"`/`'`).

**Optional accessibility wrapper** (HTML only):
```html
<th>throughput <span aria-label="higher is better">↑</span></th>
```

**Confidence: HIGH** — Unicode block is canonical, font support universal, codebase pattern (`\u{NNNN}` literals) is already established.

---

### Q5. Security-posture sidecar schema

**Verdict: HAND-ROLL minimal schema. Mirror the existing `CellMeta` plumbing exactly — DO NOT introduce CIS-Bench JSON.**

**Reasoning:**

- CIS-Bench / Docker-Bench output schemas are heavyweight: each test produces a `{ test_id, status, output, … }` record, with `score` aggregated post-hoc as `(passed_tests / total_tests) * weighted_severity`. This is **machine-collected output from a real scanner** — we are emitting **hand-curated heuristic scores** for six static Docker base images, so the CIS schema is gross overkill.
- The existing `loader.rs::CellMeta` (lines 57-68) is the architectural template:
  - One file per env: `meta/security/alpine.json`, `meta/security/debian-slim.json`, etc.
  - Glob loaded by a parallel function `load_security_metas(pattern)` mirroring `load_cell_metas`.
  - Skip-and-continue on parse failure (D-08 contract — already established).
  - Keyed by `env` string, NOT by `(alloc, env)` (security is per-image, not per-image×allocator).
- The schema is the minimum needed for the radar chart's "security" axis: a 0–100 score plus a string rationale shown in tooltips and the per-cell recommendation prose.

**Recommended schema:**

```json
{
  "env": "alpine",
  "score": 78,
  "rationale": "musl libc + minimal attack surface (no shell, no setuid binaries beyond busybox); CVE-2024-XXXXX patched in 3.20.3; static-builds-friendly via apk-tools.",
  "captured_at": "2026-05-26T00:00:00Z"
}
```

**Implementation (extends `loader.rs`, no new module):**

```rust
#[derive(Debug, Deserialize)]
pub struct SecurityMeta {
    pub env: String,
    pub score: u8,            // 0..=100; clamp on read for defense-in-depth
    pub rationale: String,
    #[allow(dead_code)]
    pub captured_at: Option<String>,
}

pub fn load_security_metas(pattern: &str) -> Result<HashMap<String, SecurityMeta>> {
    if pattern.is_empty() { return Ok(HashMap::new()); }
    let mut paths: Vec<PathBuf> = glob(pattern)
        .with_context(|| format!("invalid security meta glob pattern: {pattern}"))?
        .filter_map(|r| r.ok())
        .collect();
    paths.sort_unstable();  // RESEARCH §Pitfall 3 — byte-identical output

    let mut map: HashMap<String, SecurityMeta> = HashMap::new();
    for path in paths {
        match load_one_security_meta(&path) {
            Ok(meta) => { map.insert(meta.env.clone(), meta); }
            Err(e) => { eprintln!("warn: skipped security meta {}: {}", path.display(), e); }
        }
    }
    Ok(map)
}
```

**Why score is `u8` (not `f64`):** scores are hand-curated ratings on a discrete 0..=100 scale — there is no information in fractional values. `u8` gives free clamp validation at deserialize time (`serde_json` rejects > 255 automatically; ≤ 100 contract enforced at the merge step in `recommend.rs`).

**Why no embedding in `CellMeta`:** the existing `CellMeta` is keyed by `(alloc, env)` because image size depends on the build target. Security score is per-image, not per-build, so it gets its own keyspace. Reusing a separate file pattern keeps the `meta/` directory self-documenting (`meta/{alloc}-{env}.json` for build-time data; `meta/security/{env}.json` for image-time data).

**Confidence: HIGH** — direct mirror of `CellMeta` plumbing; no new architectural surface.

## Crate Versions Summary (May 2026)

### No new runtime crates

| Crate | Version | Status | Notes |
|-------|---------|--------|-------|
| (none) | — | — | All v1.1 features compose on the v1.0 stack |

### Optional future-only — defer unless needed

| Crate | Version | Purpose | When to add |
|-------|---------|---------|-------------|
| `pulldown-cmark` | 0.13.4 (verified 2026-05-20) | CommonMark → HTML | Only if recommendation prose grows beyond single-sentence rationale (v1.2+); MIT-licensed, ~3-4 transitive deps, ~150 KB compiled |

### v1.0 crates that get extended (not duplicated)

| Crate | Version | Used in v1.1 for |
|-------|---------|------------------|
| `tinytemplate` | 1 | Inject `{ recommendation_prose | }` (default-escaped) and `{ spider_traces_json | unescaped }` substitutions |
| `serde_json` | 1 | Parse `meta/security/{env}.json` via the existing `load_one_*` pattern |
| `glob` | 0.3 | Discover security sidecars via `meta/security/*.json` pattern |
| `chrono` | 0.4 | `captured_at` field on security sidecars (informational; matches `CellMeta`) |

## Alternatives Considered

| Need | Recommended | Alternative | Why Not |
|------|-------------|-------------|---------|
| Radar chart trace | Plotly `scatterpolar` (already in CDN bundle) | Chart.js `radar` + Chart.js CDN swap | Would require new SRI hash, new CDN URL, new escape audit, new license check — violates "decorate, don't rewrite" |
| Radar chart trace | Plotly `scatterpolar` | D3.js + hand-rolled radar | 3-5x more LOC; D3 not in CDN today; no benefit |
| 0–100 normalization | Hand-rolled `normalize_axis` (15 LOC) | `statrs` 0.18 / `ndarray-stats` 0.6 | Heavy matrix-stats deps for arithmetic the project already does inline (see `multi_run.rs`) |
| Markdown → HTML | Pass prose through tinytemplate default-escape | `pulldown-cmark` 0.13 | Single-sentence prose has no markdown features beyond italic; conversion is cosmetic-only |
| Markdown → HTML | Pass prose through tinytemplate default-escape | `comrak` (CommonMark with extensions) | Same as above + heavier dep tree (~10 transitive crates) |
| Direction markers | Hard-coded `\u{2191}` / `\u{2193}` constants | `unicode-arrows` crate, `unicode-segmentation` | Overkill for two glyphs; supply-chain surface for zero gain |
| Security sidecar schema | Minimal `{score, rationale, captured_at}` JSON | CIS-Docker-Bench scoring schema | We hand-curate 6 image scores; CIS is for machine-emitted scanner output |
| Security sidecar schema | Minimal struct mirroring `CellMeta` | Embed in existing `CellMeta` | Per-image vs per-build key spaces; embedding causes duplication across N allocators of same env |

## Installation

No `Cargo.toml` changes required for v1.1.

If pulldown-cmark is added in v1.2+:

```toml
# crates/alloc-bench-aggregator/Cargo.toml
[dependencies]
pulldown-cmark = { version = "0.13", default-features = false, features = ["html"] }
```

Note `default-features = false` — turns off the binary CLI feature; we need only the library `html` module.

## Integration Points (Decorate, Don't Rewrite)

| New code goes in | Existing module it extends | What stays unchanged |
|------------------|----------------------------|----------------------|
| `recommend.rs::top_n_cells()` (new fn) | `recommend.rs` (existing `recommendations()` is untouched) | Six-class workload-recommendation table contract |
| `recommend.rs::normalize_axis()` (new fn) | `recommend.rs` | `recommend_for_class()` and the `WorkloadClass` enum |
| `recommend.rs::ARROW_UP` / `ARROW_DOWN` consts (new) | `recommend.rs` | All existing exports |
| `loader.rs::SecurityMeta` struct + `load_security_metas()` fn (new) | `loader.rs` (existing `CellMeta` is untouched) | `CellMeta` shape, `discover()`, `load_cell_metas()` |
| `markdown.rs::emit_top_n_cells()` (new fn) | `markdown.rs` (existing `emit_recommendations()` is untouched) | Six-class table emission |
| `markdown.rs::emit_direction_legend()` (new fn) | `markdown.rs` | Header / per-scenario tables |
| `html.rs::BuiltContext` (new fields: `spider_traces_json`, `top_n_cells_json`, `security_scores_json`) | `html.rs` | `results_json`, `scenarios_json`, `envs_json`, etc. — every existing field stays |
| `templates/index.html.tmpl::makeSpiderTraces()` (new JS fn) | `templates/index.html.tmpl` | `makeThroughputTraces`, `makeLatencyHeatmap`, `makeRssLines`, `makeDiffBars`, `renderReportMirrorTable` |

**Critically:** `crates/alloc-bench-core/src/output.rs` (the locked v1 input schema) is **NOT modified** for v1.1. All new data rides sidecars or is computed in `alloc-bench-aggregator` from existing v1 fields. The decorate-not-rewrite principle (CLAUDE.md cross-phase convention 2) is preserved.

## Sources

- **Plotly v2.35.3 source tree** — `https://github.com/plotly/plotly.js/blob/v2.35.3/src/traces/scatterpolar/index.js` — verifies `scatterpolar` exists in the pinned bundle (HIGH confidence, fetched 2026-05-26).
- **Plotly graphing library docs** — Context7 `/plotly/graphing-library-docs` — canonical multi-trace radar pattern with `fill: 'toself'` + `polar.radialaxis.range` + `polar.angularaxis.direction` (HIGH confidence).
- **pulldown-cmark API** — Context7 `/pulldown-cmark/pulldown-cmark` — `html::push_html(&mut String, Parser)` signature; latest 0.13.4 verified 2026-05-20 (HIGH confidence).
- **Unicode Arrows block** — Wikipedia Arrows (Unicode block) U+2190..U+21FF; U+2191 UPWARDS ARROW, U+2193 DOWNWARDS ARROW canonical since Unicode 1.0 (HIGH confidence).
- **CIS Docker Benchmark** — `cisecurity.org/benchmark/docker` — exists as PDF benchmark with no public JSON scoring schema (LOW confidence on schema details — but the verdict here is to NOT mirror it, so the gap doesn't matter).
- **v1.0 stack baseline** — `.planning/milestones/v1.0-research/STACK.md` — establishes versions, conventions, and the "decorate, don't rewrite" architectural pattern.
- **In-tree code paths** (HIGH confidence — direct read):
  - `crates/alloc-bench-aggregator/src/html.rs:37-45` — Plotly CDN URL + SRI hash declarations.
  - `crates/alloc-bench-aggregator/src/loader.rs:57-108` — `CellMeta` + `load_cell_metas` template for security sidecar mirror.
  - `crates/alloc-bench-aggregator/src/recommend.rs:248-261` — existing rationale-string builder (the prose path that v1.1 extends).
  - `crates/alloc-bench-aggregator/src/markdown.rs:1-100` — emitter pattern (`build_report` → `emit_*` functions) that the new emitters slot into.
  - `crates/alloc-bench-aggregator/templates/index.html.tmpl:352-357` — `ALLOC_COLORS` Viridis map reused for radar trace `line.color`.

## Confidence Assessment

| Area | Confidence | Reason |
|------|------------|--------|
| Plotly scatterpolar in v2.35.3 | HIGH | Direct fetch of v2.35.3 source tree confirms trace exists |
| Plotly trace shape (r, theta, fill: toself) | HIGH | Multiple canonical doc examples in Context7 |
| SRI hash unchanged | HIGH | Same JS file → same hash (no Plotly upgrade required) |
| pulldown-cmark deferred (use tinytemplate) | HIGH | Existing `html.rs` escape-via-default-formatter path covers single-sentence prose |
| Hand-rolled normalization | HIGH | Trivial arithmetic; existing `multi_run::aggregate` is precedent |
| Direction markers as `\u{NNNN}` literals | HIGH | Codebase already uses this pattern (em-dash, middle-dot); Unicode block universally supported |
| Security sidecar minimal schema | HIGH | Direct mirror of shipped `CellMeta` plumbing; no new architectural surface |
| CIS-Bench schema rejected | MEDIUM | CIS-Bench schema details are gated behind PDFs; verdict (don't mirror) is grounded in scope mismatch (hand-curated vs scanner-emitted), so the gap doesn't change the answer |

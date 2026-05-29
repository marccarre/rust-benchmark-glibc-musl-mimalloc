# Phase 8 — Per-cell artifacts: RESEARCH

> Discovery output for `/gsd-discuss-phase 08-per-cell-artifacts`.
> Generated 2026-05-27. Confidence levels: HIGH = verified against codebase, MEDIUM = inferred from analogous patterns, LOW = stated in CONTEXT.md but not yet codebase-verified.

---

## Section 1 — Executive Summary

Phase 8 ships **per-cell artifacts** — Markdown cards (`recommend-cell.md`) and HTML panels (`recommend-cell.html.tmpl`) — driven by `tinytemplate` v1 (already in workspace). The phase plumbs the Phase 7 `top_n_cells` output into both Markdown and HTML emit paths and writes 10+10 standalone fragment files (one per top-10 cell) under `recommendations/cells/`. Phase 8 also adds a leading `| Rank | Cell | Score |` summary table at the top of `## Top 10 cells` and the parallel HTML `<section class="top-n-recommendations">`, both wrapped in conditional emit so empty-`top_n` runs preserve v1.0 byte-identity.

**Key facts:**
- Phase 7 already produced `CellRecommendation` (11 fields) + `top_n_cells(&runs, n=10)` returning `Vec<CellRecommendation>`. Phase 8 consumes only.
- `tinytemplate = "1"` is already in `Cargo.toml` (line 32) — used by `html.rs::write_index_html` to render `templates/index.html.tmpl`. No new deps.
- Existing template uses ONLY simple `{var}` substitutions + `\{` escapes for raw braces. Phase 8 introduces `{{for}}` (loop), `{{if}}` (conditional), and `{{call}}` (template-from-template) directives — all v1-supported.
- WR-01 drift defense pattern (sentinel-render + cross-surface field-presence assertion) is locked from Phase 4; Phase 8 reuses it verbatim.

**Phase budget:** Two plans, 2-3 tasks each, ~50% context per plan. Plan 1 = templates + CellTemplateContext + WR-01 sentinel test. Plan 2 = wiring (`emit_top_n_cells` in markdown.rs, `<section class="top-n-recommendations">` in html.rs + `index.html.tmpl`, fragment-file writers, leading summary table, conditional emit for empty top_n).

---

## Section 2 — Tinytemplate v1 Reference

(Cited from `https://docs.rs/tinytemplate/latest/tinytemplate/syntax/index.html` — WebFetch 2026-05-27)

### Directives

- **Substitution:** `{var}` or `{path.to.field}` — substitutes serde-serialized value, HTML-escaped by default in HTML templates.
- **Raw substitution:** `{var | unescaped}` — bypasses HTML escaping (use for pre-escaped HTML).
- **Loop:** `{{ for item in collection }} ... {{ endfor }}` — iterates over arrays.
- **Conditional:** `{{ if condition }} ... {{ else }} ... {{ endif }}` — truthy if non-zero / non-empty / `true`.
- **Template call:** `{{ call template_name with context }}` — recursively renders a registered template, passing a sub-context.
- **Escapes:** `\{` for literal `{`, `\}` for literal `}`.
- **Whitespace control:** `{{- ... -}}` to trim surrounding whitespace.
- **Custom formatters:** `tt.add_formatter("name", |val, out| { ... })` — registered before `render()`.

### Phase 8 directives plan

| Directive | Where | Purpose |
|-----------|-------|---------|
| `{{ for cell in top_n }}` | `index.html.tmpl` (new section) | Loop over 10 cells |
| `{{ call recommend_cell with cell }}` | inside `{{for}}` | Render each card via shared template |
| `{{ if has_top_n }}` | `index.html.tmpl` (around new section) | Conditional emit when top_n is non-empty (v1.0 byte-identity preservation for runs with `runs_count < 10`) |
| `{{ if cell.is_suspect }}` | `recommend-cell.{md,html}.tmpl` | Render `*(suspect)*` annotation only when flagged |
| `\{` and `\}` | template bodies | Escape literal braces in Markdown code blocks (none expected, but guard against future) |

---

## Section 3 — Codebase Map (verified line-by-line)

### `crates/alloc-bench-aggregator/src/html.rs`

| Line | Item | Phase 8 use |
|------|------|-------------|
| 14   | `pub(crate) const INDEX_HTML_TEMPLATE: &str = include_str!("../templates/index.html.tmpl");` | Will register two more `include_str!` constants (`RECOMMEND_CELL_HTML`) |
| 60-96 | `struct HtmlContext { ... }` (locked v1.0 fields) | Adds 3 new fields: `top_n: Vec<CellTemplateContext>`, `has_top_n: bool`, `top_n_table: Vec<TopNTableRow>` |
| 60-96 | `#[derive(Serialize)]` on HtmlContext | CellTemplateContext + TopNTableRow must also derive Serialize |
| 228-230 | `tt.add_template("index", INDEX_HTML_TEMPLATE)` | Will add `tt.add_template("recommend_cell", RECOMMEND_CELL_HTML)` adjacent |
| 339-343 | `#[test] fn template_renders_with_known_inputs` | Pattern source for WR-01 sentinel test (Phase 8 adds parallel sentinel test for both new templates) |

### `crates/alloc-bench-aggregator/src/markdown.rs`

| Line | Item | Phase 8 use |
|------|------|-------------|
| 56-70 | `fn build_report(...) -> String` orchestration | Insert `emit_top_n_cells(&mut buf, top_n)?` after `emit_recommendations` (line ~66); changes return type to `Result<String>` to propagate render errors |
| 340-350 | `fn emit_recommendations(...)` | Direct precedent for `fn emit_top_n_cells(buf: &mut String, top_n: &[CellRecommendation]) -> Result<()>` |
| 408-414 | `fn suspect_note(...)` | Returns `*(suspect)*` six-byte literal for byte-identity with HTML side |

### `crates/alloc-bench-aggregator/src/recommend.rs`

| Line | Item | Phase 8 use |
|------|------|-------------|
| 42-50 | `pub const TOP_N_DEFAULT: usize = 10;` and weight constants | Phase 8 calls `top_n_cells(&runs, TOP_N_DEFAULT)` |
| 138-151 | `pub struct CellRecommendation { 11 fields }` | Wrapped by `CellTemplateContext` (adds `rank_padded`, `env_short`); CellRecommendation itself remains untouched |
| 619-671 | `pub fn top_n_cells(runs: &[Run], n: usize) -> Vec<CellRecommendation>` | Called once from `main.rs::main`, result passed to both `markdown::write` and `html::write` |

### `crates/alloc-bench-aggregator/src/main.rs`

| Line | Item | Phase 8 use |
|------|------|-------------|
| 69-70 | `markdown::write(..., &outcome, &metas)?; html::write(..., &outcome, &metas)?;` | Add `let top_n = recommend::top_n_cells(&runs, recommend::TOP_N_DEFAULT);` before line 69; pass `&top_n` as new parameter to both writers (option B per resolved Q5) |

### `crates/alloc-bench-aggregator/templates/index.html.tmpl`

| Line | Item | Phase 8 use |
|------|------|-------------|
| 1-846 | Existing template, simple substitutions only | Append new `{{ if has_top_n }}<section class="top-n-recommendations"> ... </section>{{ endif }}` block AFTER recommendations section, BEFORE per-scenario tables — matches markdown.rs `emit_top_n_cells` placement after `emit_recommendations` |

---

## Section 4 — WR-01 Drift Defense Pattern (Phase 4 precedent)

The "sentinel-render + cross-surface field-presence assertion" pattern was locked in Phase 4 for `image_size_mb` byte-identity between markdown and HTML emits. Phase 8 reuses it for `recommend-cell.md.tmpl` ↔ `recommend-cell.html.tmpl` parallelism.

### Pattern shape

1. **Single sentinel input:** A hand-crafted `CellRecommendation` instance with exotic but realistic field values.
2. **Render both surfaces:** Call the markdown template renderer and the HTML template renderer with identical context.
3. **Assert field presence in BOTH outputs:** Each of the 11 `CellRecommendation` fields appears (in some surface-appropriate form) in BOTH the markdown and HTML renderings.
4. **Asymmetric formatting allowed:** `latency_p50_ns: 12_345` may render as `12345 ns` in markdown and `<dd class="latency-p50">12345 ns</dd>` in HTML — the test checks for `12345` substring presence in both, not byte-equal output.

### Phase 8 sentinel test sketch

Locked module path: `crates::aggregator::html::tests::recommend_cell_templates_render_all_fields`. The test registers BOTH new templates (`recommend_cell_md` and `recommend_cell_html`) in the same `tinytemplate::TinyTemplate` instance, renders the sentinel context against both, and asserts all 11 fields' values appear in both outputs. Sentinel field values use distinctive base-13 / Mersenne / unicode-tagged strings so that any template variable substitution drift is loud-failing.

---

## Section 5 — Pitfalls Identified

### From Phase 7 SUMMARY (Phase 7 plan-02 SUMMARY.md)

- **Deferred imports:** Phase 7 had ONE auto-fixed deviation — `use crate::score::CellScore` was deferred to a later task because the import would have triggered an unused-import warning at Task 1 build time. **Phase 8 lesson:** the `RECOMMEND_CELL_MD` / `RECOMMEND_CELL_HTML` constants and the `tt.add_template()` calls should land in the same task to avoid that warning class.

### From CLAUDE.md byte-identical-output discipline

- **No HashMap/HashSet** — alphabetical iteration via `BTreeMap` / `BTreeSet`. Phase 8's `top_n` is already a `Vec<CellRecommendation>` in deterministic composite-score order (Phase 7 SUMMARY), so no concern.
- **Numeric formatting:** `{:.1}` for throughputs in single-run cells, `{:.0}` for medians in multi-run cells, `{}` for ns latencies. Phase 8 templates must use the same formatters as `emit_recommendations` produces. The aggregator builds the `CellTemplateContext` with **pre-formatted strings** (e.g., `latency_p50_str: "12345 ns"`) rather than raw integers, so tinytemplate doesn't need custom formatters.
- **Suspect annotation:** the six-byte literal `*(suspect)*` (no nbsp, no zero-width-space) must appear in both surfaces. Phase 8 templates emit it via the same conditional check (`{{ if cell.is_suspect }}*(suspect)*{{ endif }}`).
- **Single timestamp comment:** REPORT.md keeps its existing single timestamp comment at top — Phase 8 does NOT add another. The `## Top 10 cells` section emits below the existing recommendations cluster.

### From CONTEXT.md "Out of Scope"

Per CONTEXT.md "Out of Scope" and the v1.1 milestone discipline ("Hand-roll 15-LOC normalizer; hard-code `↑`/`↓` Unicode literals; security sidecar reuses `serde_json` 1 (no `statrs`, no `unicode-arrows`, no `pulldown-cmark` for v1.1)"). Phase 8 uses ONLY tinytemplate (already in workspace) + std + existing deps. No new entries in `Cargo.toml`.

### Lessons from Phase 7

Per the Phase 7 plan-02 SUMMARY.md (read in full), Phase 7 had ONE auto-fixed deviation: a deferred import (`use crate::score::CellScore`) due to unused-import warning at Task 1 build time. **Lesson for Phase 8:** Add imports only at the task that consumes them. If Plan 8 splits "scaffold templates + register" from "wire into emitters", the `RECOMMEND_CELL_MD` / `RECOMMEND_CELL_HTML` constants and the `add_template` calls should land in the same task.

---

## Section 6 — Open Questions (RESOLVED)

All five questions have been resolved during `/gsd:discuss-phase 08-per-cell-artifacts` and the subsequent plan-checker pass. Decisions are locked in `08-CONTEXT.md` and reflected in `08-01-PLAN.md` / `08-02-PLAN.md`.

1. **REPORT.md section ordering discrepancy** (flagged in Section 3, markdown.rs map)
   - CONTEXT.md says: "after the existing `## Recommendations` table and before the per-scenario tables"
   - Actual `build_report` order (markdown.rs:61-65): `header → per-scenario tables → docker-runtimes → diagrams → recommendations → skipped`
   - **RESOLVED:** Insert `emit_top_n_cells` AFTER `emit_recommendations` (line 66 in `build_report`). This honors the spirit of CONTEXT.md (Top 10 cells is in the recommendations cluster, not buried by per-scenario detail) without reordering existing v1.0 sections (which would break byte-identity for everything between). The literal "before the per-scenario tables" reading is **superseded** — relocating per-scenario tables is out of scope and would shatter v1.0 byte-identity. Plan 02 Task 1 step 6 implements this placement.

2. **Filename `{env}` interpretation** (flagged in Section 3, recommend.rs map)
   - CONTEXT.md gives examples like `linux-glibc-jemalloc` and `linux-musl-mallocng` for the filename's `{env}` token, claiming it's "the full env name as it appears in `Run.env`"
   - But `Run.env` is a struct, not a kebab-case string. The "linux-glibc" form is a kebab-case env identifier used in the GHA matrix and the justfile, NOT a field on `Run.env`.
   - The naturally available short form (`alpine`, `debian-slim`, `distroless-cc`, `distroless-static`, `scratch`, `wolfi`) is what `recommend.rs::env_short_name` extracts and what `CellRecommendation.env` already contains.
   - **RESOLVED:** Use `cell.env` (short form) — unambiguous in the 7-env matrix (six Linux + macOS host), grep-friendly, and round-trippable to `Run.env.docker_image` via the inverse extraction. Filename pattern is `recommendations/cells/{rank_padded}-{alloc}-{env}.{md,html}` per CELL-04. Plan 02 Task 2 implements this.

3. **Leading `| Rank | Cell | Score |` summary table at top of `## Top 10 cells`** (flagged in CONTEXT.md "Specific Ideas")
   - CONTEXT.md says: "Plan 8 to decide if this leading table is in scope or deferred — recommend in-scope (it's the navigation index for the cards section)."
   - This is a small extra bit of work (10 rows × 3 cols) that lives in `markdown.rs::emit_top_n_cells` directly (not in the per-cell template). HTML side would be a parallel `<table>` block in the new `<section class="top-n-recommendations">`.
   - **RESOLVED — IN SCOPE.** The leading table is the navigational index for the cards section and the only surface where `composite_score` is visible (per CELL-05 the cards themselves don't show it). Markdown header `| Rank | Cell | Score |` with separator `|------|------|-------|`, rows formatted as rank `{:02}`, cell label `{alloc} on {env}`, composite score `{:.3}` (matches CONTEXT.md sentinel `0.789`). HTML parallel renders a `<table>` with matching column order and the same row formatters. Plan 02 Task 1 step 5 (emit before cards) and Test 5 (`top_n_section_starts_with_summary_table`) implement this.

4. **`env_short_name` visibility** (minor, flagged in Section 3)
   - Phase 8 needs the short env name to (a) build card titles, (b) build filenames. `CellRecommendation.env` already contains it (verified via Phase 7 plumbing).
   - **RESOLVED — NO ACTION REQUIRED.** `CellRecommendation.env` carries the short form (verified Phase 7 plumbing). Phase 8 does NOT call `env_short_name` directly on a `Run`; the value is read from `cell.env` everywhere. Visibility unchanged.

5. **Wiring location for `score_cells → top_n_cells` invocation**
   - Three options identified (Section 3, main.rs map). Recommendation: option B (compute in main.rs, pass via new parameter to both writers).
   - **RESOLVED — OPTION B.** Compute `top_n` once in `main.rs::main` immediately before line 69, then pass `&top_n` as a new parameter to both `markdown::write` and `html::write`. Single computation site, two consumers, no double-work, no cross-module mutable state. Plan 02 Task 1 (markdown signature change) and Task 3 (html signature change) implement this.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| `CellRecommendation` data | Aggregator (`recommend.rs`) | — | Already shipped Phase 7; Phase 8 consumes only |
| `CellTemplateContext` builder | Aggregator (`recommend.rs` or `markdown.rs`/`html.rs`) | — | Adds `rank_padded` / `env_short` for template substitution; must be `serde::Serialize` |
| Markdown card rendering | Aggregator (`markdown.rs`) | tinytemplate (lib) | `markdown::write` orchestrates; tinytemplate does field substitution |
| HTML card rendering | Aggregator (`html.rs`) | tinytemplate (lib) | Parallel pattern to Markdown side |
| `## Top 10 cells` section emit | Aggregator (`markdown.rs::emit_top_n_cells`) | — | Mirrors `emit_recommendations` precedent (line 340) |
| `<section class="top-n-recommendations">` | Aggregator (`html.rs` + `index.html.tmpl`) | tinytemplate `{{for}}` + `{{call}}` | New template directives — first use in project |
| Standalone fragment files | Aggregator (`markdown::write`, `html::write`) | std::fs | Per-cell loop writes 10 files of each type |
| Drift defense (CELL-02 sentinel test) | Aggregator (`html.rs::tests`) | — | Locked test name and module path per REQUIREMENTS |

---

## Sources

### Primary (HIGH confidence)
- `crates/alloc-bench-aggregator/src/html.rs` — verified line-for-line; tinytemplate registration at 228-230, compile test at 339-343, context struct at 60-96 [VERIFIED: codebase Read]
- `crates/alloc-bench-aggregator/src/markdown.rs` — verified line-for-line; `build_report` orchestration at 56-70, `emit_recommendations` at 340-350, `suspect_note` at 408-414 [VERIFIED: codebase Read]
- `crates/alloc-bench-aggregator/src/recommend.rs` — verified full file; `CellRecommendation` at 138-151, `top_n_cells` at 619-671, constants at 42-50 [VERIFIED: codebase Read]
- `crates/alloc-bench-aggregator/templates/index.html.tmpl` — 846 lines; ONLY simple substitutions + `\{` escapes; no `{{for}}`/`{{if}}` directives exist [VERIFIED: codebase grep]
- `Cargo.toml` line 32 — `tinytemplate = "1"` [VERIFIED: codebase Read]
- `.planning/phases/07-scoring-top-n/07-02-SUMMARY.md` — Phase 7 deliverable summary; 21 `recommend::tests` pass; `top_n_cells` not yet wired into main.rs [VERIFIED: file Read]
- `crates/alloc-bench-aggregator/src/main.rs` lines 69-70 — orchestration order `markdown::write → html::write` [VERIFIED: grep]

### Secondary (MEDIUM confidence)
- `https://docs.rs/tinytemplate/latest/tinytemplate/syntax/index.html` — official syntax reference for `{{for}}`, `{{if}}`, `{{call}}`, `\{` escape, `add_formatter` [CITED: WebFetch 2026-05-27]

### Tertiary (LOW confidence)
- None — all critical claims verified against codebase or official docs.

---

## Metadata

**Confidence breakdown:**
- Existing code map: HIGH — every line number verified against the working tree
- Tinytemplate API: HIGH — official docs.rs cited; CLI fallback (ctx7) unavailable but WebFetch authoritative
- WR-01 test pattern: HIGH — closest precedent identified at html.rs:339; new sentinel-render shape sketched
- Pitfalls: HIGH — derived from Phase 7 SUMMARY (one auto-fixed deviation), CLAUDE.md byte-identical-output discipline, and tinytemplate documented behavior
- Open Questions: ALL FIVE RESOLVED — see Section 6 (resolutions locked in `08-CONTEXT.md` and reflected in `08-01-PLAN.md` / `08-02-PLAN.md` per the plan-checker revision pass)

**Research date:** 2026-05-27
**Valid until:** 2026-06-27 (30 days; tinytemplate version pin is stable, codebase moves forward but Phase 8 plug-in points are stable)

## RESEARCH COMPLETE

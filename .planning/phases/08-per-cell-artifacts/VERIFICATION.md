---
phase: 08-per-cell-artifacts
verified: 2026-05-27T00:00:00Z
status: passed
score: 5/5
overrides_applied: 0
---

# Phase 8: Per-cell Artifacts — Verification Report

**Phase Goal:** Render `CellRecommendation` through two tinytemplate files (`recommend-cell.md.tmpl` for Markdown card, `recommend-cell.html.tmpl` for HTML panel) — single struct → two outputs → drift caught at compile time. Adds a `## Top 10 cells` section to REPORT.md and a `<section class="top-n-recommendations">` to `index.html`. Writes ten standalone Markdown files and ten standalone HTML fragments to `report/recommend-{rank:02d}-{alloc}-{env}.{md,html}`.

**Verified:** 2026-05-27
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (REQUIREMENTS CELL-01..CELL-05)

| #   | Truth (Requirement) | Status     | Evidence       |
| --- | ------------------- | ---------- | -------------- |
| 1   | **CELL-01**: Two tinytemplate files driven by the same `CellRecommendation` struct — Markdown card and HTML panel field-by-field identical | ✓ VERIFIED | `templates/recommend-cell.md.tmpl` (20 lines) and `templates/recommend-cell.html.tmpl` (12 lines) both reference identical field set: `rank`, `rank_padded` (HTML only — id attr), `alloc`, `env`, `tldr`, `strengths`, `weaknesses`, `recommended_for`, `avoid_for`, `suspect_flag`. Same `CellTemplateContext` (html.rs:154-170) feeds both via `build_cell_template_context()` (html.rs:177-190). |
| 2   | **CELL-02**: Test `html::tests::cell_templates_both_reference_all_fields` renders both with sentinels and asserts both contain every sentinel (WR-01 pattern) | ✓ VERIFIED | Test at html.rs:553-617. Asserts 9 sentinel substrings + literal `(suspect)` + rank prefix `7. ` + rank-padded HTML id `recommend-07-...`. `cargo test html::tests::cell_templates_both_reference_all_fields` exits 0. SUMMARY records mutation sanity check confirming the test catches `{tldr}` removal from md template. |
| 3   | **CELL-03**: `just aggregate` writes ten `recommend-{rank:02d}-{alloc}-{env}.{md,html}` (rank zero-padded) | ✓ VERIFIED | markdown.rs:64-72 writes per-cell `.md` fragments via `format!("recommend-{:02}-{}-{}.md", cell.rank, cell.alloc, cell.env)`. html.rs:227-235 writes per-cell `.html` fragments same pattern. Smoke run on `results/*.json` (180 runs, 7 cells available) emits 7 `.md` + 7 `.html` files at `/tmp/phase08-verify/recommend-XX-*-*.{md,html}` — pattern verified, rank zero-padded `01..07`, content correct (e.g., `recommend-01-jemalloc-slim.md` carries `### 1. jemalloc/slim` heading). When 10 cells exist, all 10 emitted (proven by `write_emits_per_cell_html_fragments` test at html.rs:998-1069 with `make_top_n(10)`). |
| 4   | **CELL-04**: `## Top 10 cells` section in REPORT.md with top-5 above the fold, ranks 6-10 inside `<details>` (Cowan's 4±1) | ✓ VERIFIED | markdown.rs:403-470 `emit_top_n_cells` emits `## Top 10 cells` heading, caption, leading `\| Rank \| Cell \| Score \|` table, top-5 cards separated by `---`, then `<details><summary>Show ranks 6–10</summary>` (en-dash U+2013 at line 455 — `\u{2013}`) wrapping ranks 6-10. Symmetrical HTML side: index.html.tmpl:257-266 wraps `<section class="top-n-recommendations">` in `{{ if has_top_n }}…{{ endif }}` with two `{{for}}` loops over `top_n_visible` / `top_n_collapsed` calling `{{ call recommend-cell-html with cell }}`. Smoke run confirms: REPORT.md line 337 `## Top 10 cells` after line 326 `## Recommendations by workload`; ranks 1-5 visible (lines 351-468), `<details>` block lines 471-523. index.html: `<section class="top-n-recommendations">` line 257, 5 `<article>` cards before line 316 `<details>`, 2 collapsed cards inside `<details>...</details>` lines 318-340. |
| 5   | **CELL-05**: TL;DR → Strengths → Weaknesses → Recommended-for → Avoid-for, 80–150 words, data-derived (only `*(suspect)*` annotation) | ✓ VERIFIED | Both templates emit fields in declared order: TL;DR (`{tldr}` paragraph), `**Strengths**` + bullet list, `**Weaknesses**` + bullet list, `**Recommended for**` + bullet list, `**Avoid for**` + bullet list. Suspect annotation `{{ if suspect_flag }} *(suspect)*{{ endif }}` emits literal six bytes (no `<em>`, no badge). Verified by `emit_top_n_cells_emits_suspect_suffix_for_flagged_cells` at markdown.rs:1203-1214 (asserts `*(suspect)*` substring) and `cell_templates_both_reference_all_fields` (asserts `(suspect)` in BOTH md and html outputs). Spot-check of `/tmp/phase08-verify/recommend-01-jemalloc-slim.md` and `.html` confirms exact field order: TL;DR sentence, Strengths bullets, Weaknesses bullets, Recommended for bullets, Avoid for bullets. No hand-edited prose strings. |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact | Expected    | Status | Details |
| -------- | ----------- | ------ | ------- |
| `crates/alloc-bench-aggregator/templates/recommend-cell.md.tmpl` | New tinytemplate Markdown card body (CELL-01) | ✓ VERIFIED | 20 lines; references `{rank}`, `{alloc}`, `{env}`, `{tldr}`, `{{ if suspect_flag }}`, 4 `{{ for }}` loops (strengths, weaknesses, recommended_for, avoid_for); first line matches `### {rank}. {alloc}/{env}{{ if suspect_flag }} *(suspect)*{{ endif }}`; field-only substitution per CELL-02 contract |
| `crates/alloc-bench-aggregator/templates/recommend-cell.html.tmpl` | New tinytemplate HTML card body (CELL-01) | ✓ VERIFIED | 12 lines; outer `<article class="recommend-card" id="recommend-{rank_padded}-{alloc}-{env}">`; identical field set to `.md.tmpl` (modulo `rank_padded` for the id attr); 4 `{{ for }}` loops; same suspect conditional; closing `</article>` |
| `crates/alloc-bench-aggregator/src/html.rs` | RECOMMEND_CELL_MD/HTML consts, CellTemplateContext, build_cell_template_context, render_cell_html, top_n_visible/collapsed/has_top_n on HtmlContext, render_cell_html, 5 new tests | ✓ VERIFIED | line 47: `pub(crate) const RECOMMEND_CELL_MD: &str = include_str!(...)`; line 55: `pub(crate) const RECOMMEND_CELL_HTML`; lines 123-135: `top_n_visible: &'a [CellTemplateContext]` / `top_n_collapsed: &'a [CellTemplateContext]` / `has_top_n: bool`; lines 153-170: `pub(crate) struct CellTemplateContext` with exactly the 10 documented fields (no `composite_score`, no `axes`); lines 177-190: `pub(crate) fn build_cell_template_context`; lines 215-237: `pub fn write` extended with `top_n: &[CellRecommendation]` arg + per-cell fragment loop; lines 346-398: `fn render` extended with `top_n` arg, registers all three templates (`index`, `recommend-cell-html`, `recommend-cell-md`), computes `visible_ctxs` / `collapsed_ctxs` split via `top_n.len().min(TOP_N_TABLE)`, sets `has_top_n: !top_n.is_empty()`; lines 408-418: `fn render_cell_html` per-cell render |
| `crates/alloc-bench-aggregator/src/markdown.rs` | emit_top_n_cells, render_cell_md, build_report Result<String>, write extended, per-cell fragment loop, leading summary table, 5 new tests | ✓ VERIFIED | line 35: `use crate::html::{build_cell_template_context, is_suspect, RECOMMEND_CELL_MD};`; line 49: `pub fn write(... top_n: &[CellRecommendation], out_dir: &Path) -> Result<()>`; lines 64-72: per-cell `.md` fragment write loop with format `recommend-{:02}-{}-{}.md`; lines 84-104: `pub(crate) fn build_report` returns `Result<String>`, calls `emit_top_n_cells(&mut buf, top_n)?` after `emit_recommendations` (Q1 lock); lines 403-470: `fn emit_top_n_cells -> Result<()>` with early-return on empty, section heading, caption, `\| Rank \| Cell \| Score \|` table with `{:.3}` precision, 5 visible cards separated by `---`, `<details><summary>Show ranks 6–10</summary>` block (U+2013 at line 455 via `\u{2013}` escape) for ranks 6-10; lines 479-489: `fn render_cell_md` |
| `crates/alloc-bench-aggregator/src/main.rs` | score → top_n pipeline + threading into both writers | ✓ VERIFIED | line 64: `let security_metas = loader::load_security_metas(&cli.security)?;` (no leading underscore — Plan 02 promotion); lines 85-87: `let cell_axes = score::compute_axes(...); let cell_scores = score::score_cells(cell_axes); let top_n = recommend::top_n_cells(cell_scores, &outcome.runs);`; lines 89-90: `markdown::write(&outcome, &metas, &top_n, out_dir)?; html::write(&outcome, &metas, &top_n, out_dir)?;` |
| `crates/alloc-bench-aggregator/templates/index.html.tmpl` | `<section class="top-n-recommendations">` block gated by `{{if has_top_n}}` after `<section class="report-mirror">` | ✓ VERIFIED | lines 257-266: `{{ if has_top_n }}` wraps `<section class="top-n-recommendations">` containing `<h2>Top 10 cells</h2>`, caption `<p>`, `{{ for cell in top_n_visible }}{{ call recommend-cell-html with cell }}{{ endfor }}`, `<details><summary>Show ranks 6–10</summary>` (U+2013), `{{ for cell in top_n_collapsed }}…{{ endfor }}`, `</details>`, `</section>`, `{{ endif }}`. Inserted immediately after the closing `</section>` of `<section class="report-mirror">` (line 256) |

### Key Link Verification

| From | To  | Via | Status | Details |
| ---- | --- | --- | ------ | ------- |
| `recommend-cell.md.tmpl` + `recommend-cell.html.tmpl` | `CellTemplateContext` | tinytemplate field substitution by name | ✓ WIRED | Both templates reference exactly the field set declared on `CellTemplateContext` (rank, rank_padded, alloc, env, tldr, strengths, weaknesses, recommended_for, avoid_for, suspect_flag). Compile-time gate: `tinytemplate_compiles_recommend_cell_templates` test at html.rs:534. Cross-surface drift gate: `cell_templates_both_reference_all_fields` (CELL-02). |
| `html::tests::cell_templates_both_reference_all_fields` | both templates | sentinel-render + cross-surface substring assertion | ✓ WIRED | Test renders both templates against same `CellTemplateContext` built from a sentinel-laden `CellRecommendation`; asserts 9 sentinels + `(suspect)` + rank prefix in BOTH outputs. Mutation sanity check (per SUMMARY) confirmed the test fails when `{tldr}` is removed from md template. |
| `main.rs::main` | `markdown::write` + `html::write` | `&[CellRecommendation]` threaded as new positional arg | ✓ WIRED | main.rs:85-87 computes `top_n` once via `compute_axes → score_cells → top_n_cells`. main.rs:89-90 passes `&top_n` to both writers. Single source of truth → both surfaces render the same ranking. |
| `markdown.rs::build_report` | `emit_top_n_cells` | buf-append after `emit_recommendations`; Result propagated via `?` | ✓ WIRED | markdown.rs:94 calls `emit_recommendations(&mut buf, &outcome.runs);`, then markdown.rs:99 calls `emit_top_n_cells(&mut buf, top_n)?;`. Q1 lock honored: `## Top 10 cells` lands AFTER `## Recommendations by workload` (proven by `build_report_inserts_top_n_after_recommendations_before_skipped` test at markdown.rs:1219-1261). |
| `index.html.tmpl` | `recommend-cell.html.tmpl` | tinytemplate `{{ call recommend-cell-html with cell }}` inside `{{for}}` loops, gated by `{{if has_top_n}}` | ✓ WIRED | index.html.tmpl:260, 263 invoke `{{ call recommend-cell-html with cell }}` inside `{{for}}` loops. html.rs:358-361 registers `recommend-cell-html` against the same `TinyTemplate` instance as `index` so `{{call}}` resolves at render time. Gate: `tinytemplate_renders_index_with_top_n_section` test at html.rs:1077-1096 asserts 10 rank-padded ids appear in rendered HTML. |
| `markdown::write` + `html::write` | `report/recommend-{rank:02d}-{alloc}-{env}.{md,html}` | `std::fs::write` per-cell loop with `format!("recommend-{:02}-{}-{}", ...)` | ✓ WIRED | markdown.rs:64-72 + html.rs:227-235 iterate `top_n.iter()`, render via `render_cell_md` / `render_cell_html`, write to `out_dir.join(...)` with rank zero-padded. Smoke run on `results/*.json` produced 7 `.md` + 7 `.html` files (only 7 cells in fixture). `write_emits_per_cell_html_fragments` test at html.rs:998-1069 confirms 10 fragments emitted when `top_n.len() == 10`. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| -------- | ------------- | ------ | ------------------ | ------ |
| `markdown.rs::emit_top_n_cells` | `top_n: &[CellRecommendation]` | `recommend::top_n_cells(score_cells(compute_axes(runs, metas, security_metas)))` from main.rs:85-87 | ✓ Yes (180 input runs → 7 ranked cells in smoke run) | ✓ FLOWING |
| `html.rs::render` | `top_n_visible` / `top_n_collapsed` slices over `CellTemplateContext` | Same `top_n` threaded via `pub fn write` from main.rs | ✓ Yes (per-cell HTML fragments populated with real allocator names, env labels, tldr, strengths/weaknesses) | ✓ FLOWING |
| `report/recommend-XX-*.{md,html}` fragments | Per-cell `CellRecommendation` field substitutions | `render_cell_md` / `render_cell_html` invoke tinytemplate against `build_cell_template_context(cell)` | ✓ Yes (smoke run produced files with real prose: e.g., "jemalloc/slim — strong on CPU-bound throughput, weak on Image-size efficiency.") | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| `cargo test --workspace` passes 0 failures | `cargo test --workspace 2>&1 \| grep "test result:"` | All 8 result lines show `0 failed`; totals: 98 + 81 + 28 + 3 + 1 + 1 + 1 + 1 = 214 tests pass | ✓ PASS |
| All 8 Phase 8 specific html tests pass | `cargo test -p alloc-bench-aggregator html::tests::` | 13 passed; 0 failed (8 Phase 8 tests + 5 pre-existing) | ✓ PASS |
| All 5 Phase 8 specific markdown tests pass | `cargo test -p alloc-bench-aggregator markdown::tests::` | 20 passed; 0 failed (5 Phase 8 tests + 15 pre-existing) | ✓ PASS |
| Smoke aggregate emits per-cell fragments | `cargo run -p alloc-bench-aggregator -- --input "results/*.json" --output /tmp/phase08-verify/` then `ls /tmp/phase08-verify/recommend-*.md \| wc -l` | 7 `.md` + 7 `.html` files written (180 runs aggregated, 7 cells available); names follow `recommend-XX-{alloc}-{env}.{md,html}` pattern | ✓ PASS |
| `## Top 10 cells` section emitted in REPORT.md after `## Recommendations by workload` | `grep -n "Top 10 cells\|Recommendations" /tmp/phase08-verify/REPORT.md` | line 326 `## Recommendations by workload`; line 337 `## Top 10 cells`; line 471 `<details>`; line 472 `<summary>Show ranks 6–10</summary>` (en-dash); line 523 `</details>` | ✓ PASS |
| `<section class="top-n-recommendations">` emitted in index.html | `grep -n "top-n-recommendations\|Top 10 cells\|Show ranks 6" /tmp/phase08-verify/index.html` | line 257 `<section class="top-n-recommendations">`; line 258 `<h2>Top 10 cells</h2>`; 5 `<article class="recommend-card"` blocks before `<details>` (line 316); 2 collapsed inside details (lines 318, 329) — only 2 because fixture has only 7 cells | ✓ PASS |

### Probe Execution

No conventional `scripts/*/tests/probe-*.sh` exists in this project. PLAN frontmatter does not declare any probes. `<verify>` blocks in PLANs reference `cargo test` and `cargo build` directly — those have been executed under the Behavioral Spot-Checks section above.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| ----------- | ---------- | ----------- | ------ | -------- |
| CELL-01 | 08-01-PLAN.md | Two tinytemplate files driven by same CellRecommendation | ✓ SATISFIED | `templates/recommend-cell.{md,html}.tmpl` exist; both reference identical field set via `CellTemplateContext` |
| CELL-02 | 08-01-PLAN.md | WR-01 sentinel test — drift defense | ✓ SATISFIED | `html::tests::cell_templates_both_reference_all_fields` exists at html.rs:553-617; passes; mutation sanity check confirmed |
| CELL-03 | 08-02-PLAN.md | 10 standalone .md + 10 standalone .html fragments | ✓ SATISFIED | Per-cell write loops in markdown.rs:64-72 + html.rs:227-235; `write_emits_per_cell_html_fragments` test asserts 10 `.html` files with rank-zero-padded names; smoke run produced 7 each (only 7 cells in fixture) |
| CELL-04 | 08-02-PLAN.md | `## Top 10 cells` section in REPORT.md (top-5 above fold + collapsed details for 6-10) | ✓ SATISFIED | `emit_top_n_cells` at markdown.rs:403-470 implements the full section; symmetrical HTML side via `<section class="top-n-recommendations">` gated by `{{if has_top_n}}`; both surfaces verified in smoke output |
| CELL-05 | 08-01-PLAN.md + 08-02-PLAN.md | TL;DR → Strengths → Weaknesses → Recommended-for → Avoid-for, data-derived, only `*(suspect)*` annotation | ✓ SATISFIED | Card structure proven by template body order (recommend-cell.md.tmpl lines 1-20); `*(suspect)*` literal verified by `cell_templates_both_reference_all_fields` (asserts `(suspect)` in both outputs) and `emit_top_n_cells_emits_suspect_suffix_for_flagged_cells` (asserts `*(suspect)*` substring); CONTEXT.md decision gate via `cell_template_context_excludes_score_and_axes` (composite_score / axes intentionally absent from card body) |

**Coverage:** 5/5 Phase 8 requirements satisfied. No orphans (REQUIREMENTS.md maps CELL-01..CELL-05 to Phase 8; all 5 covered).

### Decision Coverage (08-CONTEXT.md)

Reviewed all decisions in 08-CONTEXT.md and confirmed implementation:

| Decision | Implementation | Status |
| -------- | -------------- | ------ |
| Heading level: `### {rank}. {alloc}/{env_short}` (CELL-01) | recommend-cell.md.tmpl:1; recommend-cell.html.tmpl:2 (`<h3>`) | ✓ Implemented |
| Suspect annotation: literal `*(suspect)*` six bytes (no `<em>`, no badge) | Both templates use `{{ if suspect_flag }} *(suspect)*{{ endif }}`; sentinel test asserts `(suspect)` in BOTH outputs | ✓ Implemented |
| Section order TL;DR → Strengths → Weaknesses → Recommended for → Avoid for | recommend-cell.md.tmpl emits in declared order (lines 3, 5-8, 9-12, 13-16, 17-20); html template mirrors | ✓ Implemented |
| Strengths/Weaknesses bullet list of axis labels only — NO scores | Templates iterate `{{ for s in strengths }}- {s}{{ endfor }}`; no axis-score lookups; CONTEXT decision gated by `cell_template_context_excludes_score_and_axes` | ✓ Implemented |
| Filename pattern: `recommend-{rank:02d}-{alloc}-{env}.{md,html}` (CELL-03) | markdown.rs:67 + html.rs:230 use `format!("recommend-{:02}-{}-{}.{ext}", cell.rank, cell.alloc, cell.env)`; smoke run confirms `recommend-01-jemalloc-slim.{md,html}` etc. | ✓ Implemented |
| File body: each fragment is the single card body, no frontmatter, no file-level title | Smoke run inspection: `recommend-01-jemalloc-slim.md` starts with `### 1. jemalloc/slim`, no other heading; `.html` fragment is just a single `<article>` with no `<html>`/`<head>`/`<body>` wrapper | ✓ Implemented |
| HTML fragment outer: `<article class="recommend-card" id="recommend-{rank_padded}-{alloc}-{env}">` | recommend-cell.html.tmpl:1; rank_padded gates the id via `format!("{:02}", cell.rank)` in build_cell_template_context (html.rs:180) | ✓ Implemented |
| REPORT.md inlining via direct template invocation 10 times concatenated with `---` (no `{include}`) | markdown.rs:439-444 iterates visible slice calling `render_cell_md(cell)?`, separates with `writeln!(buf, "---")`; collapsed slice same pattern lines 458-463 | ✓ Implemented |
| Section position: AFTER `## Recommendations`, BEFORE per-scenario tables (Q1 lock) | markdown.rs:94-99: `emit_recommendations` then `emit_top_n_cells`. `build_report_inserts_top_n_after_recommendations_before_skipped` test at markdown.rs:1219-1261 asserts the order; smoke run confirms (line 326 → 337). Note: 08-CONTEXT.md actually says "AFTER `## Recommendations` AND BEFORE per-scenario tables" — but per Plan 02 Q1 lock and the actual implementation, the REPORT.md flow is `emit_per_scenario_tables` → `emit_docker_runtimes_table` → `emit_allocator_diagrams` → `emit_recommendations` → `emit_top_n_cells` → `emit_skipped`. The "before per-scenario" framing in CONTEXT was overridden by Q1's "do NOT reorder existing sections" lock; the implementation correctly inserts AFTER recommendations only. This is a deliberate, documented deviation. | ✓ Implemented (per Q1 lock) |
| Collapsible split rule: top-5 visible + ranks 6-10 inside `<details>` (CELL-04 / Cowan's 4±1) | markdown.rs:434 `let split = top_n.len().min(TOP_N_TABLE);`; visible slice lines 438-444; `<details>` block lines 449-466. Symmetric HTML in render() lines 371-396 | ✓ Implemented |
| Card separators: `---` Markdown horizontal rule between consecutive cards | markdown.rs:441-443 + 460-462 emit `\n---\n` between cards (NOT after the final card) | ✓ Implemented |
| Section header line: `## Top 10 cells` exactly | markdown.rs:408: `writeln!(buf, "## Top 10 cells")` | ✓ Implemented |
| Caption verbatim: `Ranked 1-10 by composite score (equal-weighted across 8 axes). Cards 6-10 collapsed by default.` | markdown.rs:412 emits the exact string; index.html.tmpl:259 contains the same caption inside a `<p>` tag | ✓ Implemented |
| index.html section placement: after `<section class="report-mirror">`, before per-scenario chart blocks | index.html.tmpl:253-256 is the report-mirror section; lines 257-266 immediately after contain the new section. Smoke run line 257 confirms | ✓ Implemented |
| Internal layout: top-5 visible always + `<details>` for ranks 6-10; static, no JS | index.html.tmpl:260 visible loop; lines 261-264 `<details>` wraps collapsed loop; no `<script>` calls in the new section | ✓ Implemented |
| Two template files identical field references; sentinel test enforces parity | `cell_templates_both_reference_all_fields` test asserts 9 sentinels + `(suspect)` in BOTH md and html outputs | ✓ Implemented |
| Template registration in html.rs::TT_REGISTRY (or equivalent) | html.rs:347-361 in `render`: registers `index`, `recommend-cell-html`, `recommend-cell-md` against the same `TinyTemplate`; `tinytemplate_compiles_recommend_cell_templates` test at html.rs:534 gates compile-time | ✓ Implemented |
| Tinytemplate `\{` escape discipline; compile-time test extension | Existing `tinytemplate_compiles_index_template` updated to register all three templates (html.rs:521-528). New `tinytemplate_compiles_recommend_cell_templates` at html.rs:534 covers the Phase 8 templates in isolation | ✓ Implemented |
| Default formatter (HTML-escapes `<`/`>`/`&`/`"`); fields are plain text | Templates use bare `{tldr}`, `{s}`, `{w}`, etc. without `\| unescaped` modifier. Default-escape is in effect | ✓ Implemented |
| Per-card invocation pattern: `tt.render("recommend-cell-{md,html}", &ctx)`; no inlined template strings | markdown.rs:485 + html.rs:414 use `tt.render(...)` + `build_cell_template_context(cell)`; no string formatting outside tinytemplate | ✓ Implemented |
| Standalone files written inside `markdown::write` and `html::write` (not in main.rs) | markdown.rs:64-72 + html.rs:227-235; per-cell loops live alongside `REPORT.md` / `index.html` writes | ✓ Implemented |
| Filename ASCII safety; no sanitization needed | `cell.alloc` and `cell.env` are kebab-case ASCII per v1 schema invariants; `format!` writes them verbatim | ✓ Implemented |
| Idempotency: per-cell file writes overwrite on re-run | `std::fs::write` overwrites by default; matches existing REPORT.md / index.html behavior | ✓ Implemented |
| Trailing `\n` on every fragment | markdown.rs:487: `s.push('\n');`; html.rs:416: `s.push('\n');`; smoke run inspection confirms | ✓ Implemented |
| Empty-top_n early-return preserves v1.0 byte-identity | markdown.rs:404-406: `if top_n.is_empty() { return Ok(()); }`; html.rs:395 + index.html.tmpl:257-266 `{{if has_top_n}}` wrapper. Symmetric across both surfaces | ✓ Implemented |
| Q3 leading summary table (in-scope per CONTEXT "Specifics"): `\| Rank \| Cell \| Score \|` | markdown.rs:421-429: emits header, separator, 1 row per cell with `{:02}` rank, `{alloc} on {env}` cell, `{:.3}` score. `top_n_section_starts_with_summary_table` test gates this | ✓ Implemented |
| Q5 wiring choice: compute in main.rs, thread `&top_n` (option B) | main.rs:85-90: pipeline + threading; both writers receive `&top_n` | ✓ Implemented |

**No unimplemented decisions found.**

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| (none) | — | — | — | — |

`grep -E "TBD\|FIXME\|XXX"` on all Phase 8-modified files returned zero matches (the two false positives in html.rs:257 and html.rs:736 are documentation comments referencing `\uXXXX` escape sequences, not debt markers). `grep -E "TODO\|HACK\|PLACEHOLDER"` returned zero matches across the same set. No unresolved debt markers — the phase is auditable.

### Human Verification Required

None. All Phase 8 requirements are mechanically verifiable through code inspection, automated tests, and smoke-run filesystem inspection. No visual / UX / external-service behaviors that would require human eyes — the Phase 9 spider chart is the visual artifact, and that's a separate phase.

### Gaps Summary

No gaps found. All 5 observable truths (CELL-01..CELL-05) are verified, all artifacts exist with substantive content (templates ≥12 lines, source-code modifications +900 LOC across html.rs / markdown.rs / main.rs / index.html.tmpl), all key links are wired (template → context → render → write → fragment files), data flows end-to-end (180 input runs → 7 ranked cells → 7 .md + 7 .html fragments + Top 10 cells sections in both REPORT.md and index.html in the smoke run), the WR-01 sentinel test catches drift (mutation sanity check confirmed), `cargo test --workspace` is 0 failures (214 tests pass), and no unresolved debt markers exist.

The SUMMARY.md "deviation note" about a worktree merge by the orchestrator and an executor cwd-drift bug in Plan 01 are process observations, not gaps — the code itself is correct, tests pass, and the implementation matches REQUIREMENTS / CONTEXT / PLAN intent.

The Phase 8 ROADMAP entry can be marked `[x]` and the project can transition to Phase 9 (Spider Chart). Phase 11 will regenerate the v1.0 golden fixtures to capture the new `## Top 10 cells` section bytes — that's the expected sequencing per the PLAN.

---

**Recommendation:** ready-to-transition

_Verified: 2026-05-27_
_Verifier: Claude (gsd-verifier)_

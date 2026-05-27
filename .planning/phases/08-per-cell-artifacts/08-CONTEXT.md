---
phase: 8
phase_name: Per-cell Artifacts
gathered: 2026-05-27
status: Ready for planning
---

# Phase 8: Per-cell Artifacts - Context

**Gathered:** 2026-05-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Render `Vec<CellRecommendation>` (Phase 7 deliverable) through two new tinytemplate files into Markdown cards and HTML panels — single struct → two outputs → drift caught at compile time. Adds a new `## Top 10 cells` section to `report/REPORT.md` and a new `<section class="top-n-recommendations">` to `report/index.html`. Writes ten standalone Markdown files and ten standalone HTML fragments to `report/recommend-{rank:02d}-{alloc}-{env}.{md,html}`.

**In scope (CELL-01 through CELL-05):**

1. NEW `crates/alloc-bench-aggregator/templates/recommend-cell.md.tmpl` — single-card Markdown fragment driven by the `CellRecommendation` struct fields
2. NEW `crates/alloc-bench-aggregator/templates/recommend-cell.html.tmpl` — single-card HTML `<article>` fragment (field-by-field identical to the Markdown card)
3. EXTEND `crates/alloc-bench-aggregator/src/markdown.rs` — emit `## Top 10 cells` section in REPORT.md (top-5 always visible + `<details>` for ranks 6-10); also write the 10 standalone `.md` files
4. EXTEND `crates/alloc-bench-aggregator/src/html.rs` — register both `recommend-cell.md` and `recommend-cell.html` templates; emit the `<section class="top-n-recommendations">` block via a `{{for}}` loop in `index.html.tmpl`; also write the 10 standalone `.html` fragments
5. NEW WR-01-pattern test `html::tests::cell_templates_both_reference_all_fields` — sentinel-render both templates against the same `CellRecommendation` and assert every sentinel value appears in both outputs (drift-defense for CELL-02)
6. EXTEND `crates/alloc-bench-aggregator/src/main.rs` — invoke the new emitters from the orchestration layer (between existing `markdown::write` and `html::write` or inside them — exact wiring at Claude's discretion)

**Out of scope:** Spider chart trace JSON / `polar.rs` (Phase 9 — consumes `score::top_n` directly, not these prose templates); direction-marker glyphs in column headers / chart axis labels (Phase 10 — the `↑` / `↓` glyphs from `axes.rs::arrow()` go in headers, not in prose cards); golden-fixture regeneration (Phase 11 — TEST-01/TEST-02 are the v1.1 release gate); Pareto-front overlay column on the Recommendations table (POLAR-05, Phase 9); per-cell drilldown navigation (V12-04, deferred to v1.2).

</domain>

<decisions>
## Implementation Decisions

### Card Structure & Prose Format (CELL-01, CELL-05)

- **Heading level:** `### {rank}. {alloc}/{env_short}` in Markdown card; matching `<h3>` in HTML card. Fits inside the parent `## Top 10 cells` (h2) and works correctly inside `<details>`. `env_short` is the result of `recommend.rs::env_short_name()` (short form e.g., `debian`, `alpine`) so the title is compact.
- **Suspect annotation:** When `suspect_flag = true`, append `*(suspect)*` to the card title verbatim — exact bytes `*(suspect)*` (six chars between asterisks). Matches v1.0 multi-run convention (CLAUDE.md "Suspect run flagging" section). Both Markdown and HTML emit those exact bytes — no `<em>(suspect)</em>` wrapper in HTML, no `<span class="suspect">⚠</span>` badge. Byte-comparable across surfaces (the WR-01 sentinel test relies on identical-byte rendering).
- **Section order inside each card (CELL-05 requirement):**
  1. **TL;DR** — single sentence from `cell.tldr`, rendered as a paragraph (no leading label needed; prose already self-introduces with `{alloc}/{env}` per `format_tldr` from Phase 7)
  2. **Strengths** — `**Strengths**` bold label followed by a bulleted list (one bullet per `&'static str` in `cell.strengths`). Top-2 axis labels from `derive_strengths()`.
  3. **Weaknesses** — `**Weaknesses**` bold label + bulleted list of `cell.weaknesses` (bottom-2 axis labels)
  4. **Recommended for** — `**Recommended for**` bold label + bulleted list of `cell.recommended_for` (already strings of workload-class names from Phase 7 `winners_by_class`)
  5. **Avoid for** — `**Avoid for**` bold label + bulleted list of `cell.avoid_for`
  - 80-150 words total target; cards naturally land in this range because all fields are short axis labels / class names. Per CELL-05: NO hand-edited prose strings; ONLY the `*(suspect)*` italic suffix is allowed as annotation.
- **Strengths/Weaknesses format:** Bullet list of axis labels only — NO scores in parentheses. The composite score number sits in the `## Top 10 cells` table (above each card title) but is NOT in the prose. Reason: Phase 7 derived prose is "data-derived axis labels" not "axis labels with scores"; adding scores would require re-fetching `cell.axes[axis_key]` lookups in the template, which violates the "single struct, two outputs" discipline (no template logic beyond field substitution).

### Filenames & Per-cell Artifacts (CELL-03)

- **Filename pattern:** `report/recommend-{rank:02d}-{alloc}-{env}.{md,html}` — exact REQUIREMENTS spec. `rank` is 01-10 zero-padded for natural filename sort. `{alloc}` is the full allocator name (`mimalloc`, `jemalloc`, `mallocng`, `ptmalloc`). `{env}` is the **full env name** as it appears in `Run.env` (e.g., `linux-glibc-jemalloc`, `linux-musl-mallocng`, `macos-libmalloc`). Full env names keep filenames unambiguous, round-trippable to source `Run` records, and grep-friendly.
- **File body:** Each `.md` file contains exactly the single card body (TL;DR + 4 sections + suspect suffix if applicable). NO `## Top 10 cells` heading, NO frontmatter, NO file-level title. The file IS the card; it's a fragment for future embedding (V12-04 drilldown deferral) and direct linking. Renders correctly when opened standalone in GitHub/IDE Markdown previewers because `### {rank}. …` is a valid top-level heading for a fragment.
- **HTML fragment body:** Each `.html` file contains exactly `<article class="recommend-card" id="recommend-{rank:02d}-{alloc}-{env}">…</article>` — single article element, no `<html>`/`<head>`/`<body>` wrapper, no inline `<style>`. Reason: fragments are designed to be linked from `index.html` (V12-04) and read standalone — wrapping them as full HTML pages adds noise. The `class` and `id` attributes are namespaced (`recommend-card`, matching id-pattern) for future CSS hooks without requiring dynamic JS.
- **REPORT.md inlining mechanism:** The `## Top 10 cells` section in `REPORT.md` is rendered by **directly invoking the same `recommend-cell.md.tmpl` template 10 times** (one per `CellRecommendation` in `top_n_cells()`'s output) and concatenating the rendered strings — separated by `---` horizontal rules. NO `{include "fragment"}` directive (tinytemplate doesn't support file includes). Same approach for `index.html`: the `{{for cell in top_n_cells}}…{{endfor}}` loop in `index.html.tmpl` invokes the same compiled template body. Single template → many invocations → one source of truth.

### REPORT.md Section Placement & Collapse (CELL-04)

- **Section position:** Insert `## Top 10 cells` **after the existing `## Recommendations` table** (the per-workload-class winners) and **before the per-scenario tables**. Reading flow: high-level workload winners → ranked cells (the new section) → per-scenario detail. Mirrored in `index.html` (Area 4 below).
- **Collapsible split rule:** Top-`TOP_N_TABLE` (5) cards always above the fold (rendered inline). Ranks `TOP_N_TABLE+1..=TOP_N_TOTAL` (6..=10) inside a single `<details><summary>Show ranks 6–10</summary>…</details>` block. Per Cowan's 4±1 working-memory bound (REQUIREMENTS CELL-04 rationale). Constants from `recommend.rs` drive the cutoff — NO magic numbers in `markdown.rs`.
- **Card separators:** `---` Markdown horizontal rule between consecutive cards (matches existing REPORT.md per-scenario separator convention; renders as `<hr/>` in GitHub MD).
- **Suspect annotation byte-identity:** Both REPORT.md and the `<details>` block emit `*(suspect)*` literal asterisks (no `<em>` in HTML). The same six bytes ship across both surfaces; the WR-01 sentinel test asserts both outputs contain the literal `(suspect)` substring when `suspect_flag = true`.
- **Section header line:** `## Top 10 cells` exactly (no anchor explicit; GFM auto-generates `#top-10-cells`).
- **Above the section header:** A single explanatory line — `Ranked 1-10 by composite score (equal-weighted across 8 axes). Cards 6-10 collapsed by default.` — keeps the section self-contained and matches the explanatory-line convention used by the existing Recommendations table caption.

### index.html Integration (CELL-04)

- **Section placement:** `<section class="top-n-recommendations">` sits **after the existing Recommendations table block** in `index.html.tmpl`, **before the per-scenario chart blocks**. Eye-path is identical to REPORT.md.
- **Internal layout:** Top-5 cards always visible (one per row, full-width via parent CSS — no new CSS rules in this phase; existing `.report-content` wrapper handles spacing); ranks 6-10 inside `<details><summary>Show ranks 6–10</summary>…</details>`. Static `file://`-friendly, no JS, matches D-02 dashboard discipline.
- **Rendering mechanism:** `index.html.tmpl` gains a `{{for cell in top_n_cells}}…{{endfor}}` loop that invokes the compiled `recommend-cell.html` template body inline (per cell). Tinytemplate's `{{for}}` directive is the standard pattern (already used elsewhere in index.html.tmpl for scenario blocks). The split between "first 5" and "details 6-10" is computed Rust-side: the context passed to the template carries two collections — `top_n_visible: Vec<CellRecommendation>` (ranks 1-5) and `top_n_collapsed: Vec<CellRecommendation>` (ranks 6-10) — so the template body has two `{{for}}` loops separated by the `<details>` wrapper.
- **Standalone HTML fragments are kept:** The 10 `report/recommend-{rank:02d}-{alloc}-{env}.html` files are written every aggregate run per CELL-03 (the requirement is non-negotiable). They serve as future drilldown targets (V12-04) and direct-link surfaces. Cost: 10 small writes per `just aggregate` (negligible vs. existing per-scenario writes).
- **Section-level explanatory line:** `<p>Ranked 1-10 by composite score (equal-weighted across 8 axes). Cards 6-10 collapsed by default.</p>` immediately after `<h2>Top 10 cells</h2>` — matches REPORT.md caption byte-for-byte (with HTML-tag substitution).

### Template Engineering & Drift Defense (CELL-02)

- **Two template files, identical field references:** `recommend-cell.md.tmpl` and `recommend-cell.html.tmpl` MUST reference exactly the same set of `CellRecommendation` field names. The WR-01-pattern test enumerates every field of `CellRecommendation` (rank, alloc, env, composite_score, axes, tldr, strengths, weaknesses, recommended_for, avoid_for, suspect_flag), constructs a sentinel `CellRecommendation` instance with unique recognizable string values for each field (e.g., `"_SENTINEL_TLDR_"`, `"_SENTINEL_STRENGTH_"`), renders both templates, and asserts every sentinel substring appears in both rendered outputs. Drift between the two templates fails the test loudly — pre-empts the WR-01 winner-tiebreak drift the v1.0 fix already exposed once.
- **Template registration in html.rs:** `html::TT_REGISTRY` (or whatever the existing tinytemplate setup is) gains two new `add_template` calls: `tt.add_template("recommend-cell-md", RECOMMEND_CELL_MD)` and `tt.add_template("recommend-cell-html", RECOMMEND_CELL_HTML)` — both `include_str!`'d at compile time. Reuses the existing `tinytemplate_compiles_index_template` test pattern (extends it to all three templates so unescaped `{` braces in the new templates also fail at compile time).
- **Tinytemplate `{` escape discipline:** Both `.tmpl` files MUST escape literal `{` as `\{` per the Phase 4 D-01 / RESEARCH §Pitfall 1 contract. The compile-time test extension above catches missed escapes. The Markdown template is unlikely to contain `{` (text content); the HTML template has `class="…"` attributes and CSS-like syntax to watch.
- **Tinytemplate value-substitution defaults:** Use the DEFAULT formatter (HTML-escapes `<`/`>`/`&`/`"`) for all fields except where the field is intentionally HTML-bearing (e.g., the `<details>` wrapper that's NOT in the cell template — that's in `index.html.tmpl` proper). Cell-level fields (tldr, strengths items, etc.) are plain text per Phase 7 derivation rules — DEFAULT-escape is correct.
- **Per-card invocation pattern:** `markdown::emit_top_n_cells(buf, &top_n)` and `html::render_top_n_cells_section(top_n)` both call into `tt.render("recommend-cell-md", &cell)` / `tt.render("recommend-cell-html", &cell)` once per cell — no inlined template strings, no string formatting outside the tinytemplate engine. Compile-time field validation through tinytemplate's render-error path.

### File Writing & Orchestration

- **Where the 10 standalone files are written:** Inside the existing `markdown::write` and `html::write` entry points (not in `main.rs`). Each emitter receives the `Vec<CellRecommendation>` and writes (a) the in-document section, then (b) iterates the 10 cells and writes the standalone files. Reason: keeps file-writing colocated with format knowledge (markdown.rs writes .md, html.rs writes .html), matches existing per-scenario emit pattern.
- **Filename ASCII safety:** `{alloc}` and `{env}` come from `Run.alloc` / `Run.env`, which are validated kebab-case ASCII at ingest time (existing v1 schema invariants). No filename sanitization needed in Phase 8 — write the strings verbatim.
- **Directory creation:** `report/` already exists by the time Phase 8 emitters run (created by existing `write_report` orchestrator). No new `mkdir` calls.
- **Idempotency:** Per-cell file writes overwrite on re-run (matches existing `REPORT.md` / `index.html` overwrite behavior). Stale files from previous aggregates with different top-10 sets are NOT cleaned — out of scope for Phase 8 (deferred to v1.2 if it surfaces; in practice the 18-cell matrix is stable so stale files don't accumulate).

### Claude's Discretion

- Exact field-by-field layout inside `recommend-cell.md.tmpl` (e.g., whether the bold label `**Strengths**` is followed by a colon then list, or by a newline then list — pick whichever renders cleanest in GitHub Markdown previewer).
- Whether the WR-01-pattern test lives in `html.rs::tests` (per CELL-02 verbatim test name `html::tests::cell_templates_both_reference_all_fields`) or in a new `templates_test.rs` module — REQUIREMENTS pins the path, so it lives in `html.rs::tests` to comply.
- Exact wording of the section caption line — proposed text `Ranked 1-10 by composite score (equal-weighted across 8 axes). Cards 6-10 collapsed by default.` is a recommendation, not a verbatim REQUIREMENTS pin.
- Whether `top_n_visible` / `top_n_collapsed` are split before or after `tt.render` calls — at Claude's discretion (both work; splitting before the loop is slightly cleaner).
- Whether the standalone fragment files include a leading byte-order-mark / trailing newline — recommend trailing `\n` for POSIX file convention; no BOM (matches existing REPORT.md / index.html style).

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/alloc-bench-aggregator/src/recommend.rs` — Phase 7 deliverable. Exports `pub fn top_n_cells(scores: Vec<CellScore>, runs: &[Run]) -> Vec<CellRecommendation>`, `pub struct CellRecommendation` (11 locked fields), `pub const TOP_N_SPIDER: usize = 3`, `TOP_N_TABLE: usize = 5`, `TOP_N_TOTAL: usize = 10`. Phase 8 consumes this directly.
- `crates/alloc-bench-aggregator/src/score.rs` — Phase 7 Plan 01 deliverable. Exports `score_cells(...) -> Vec<CellScore>` plus `top_n(scores, n) -> Vec<CellScore>` for Phase 9. Phase 8 invokes `score_cells` then passes the `Vec<CellScore>` into `recommend::top_n_cells` to get the prose-aware `Vec<CellRecommendation>`.
- `crates/alloc-bench-aggregator/src/html.rs` — already uses `tinytemplate::TinyTemplate` with `tt.add_template("index", TEMPLATE)`. Lines 28-34 show the `include_str!` pattern. Lines 228-230 show the orchestration entry. Phase 8 extends this with two more `add_template` calls + render entry points.
- `crates/alloc-bench-aggregator/src/markdown.rs` — `emit_recommendations(buf, runs)` at line 340 is the precedent for in-buffer section emission. Phase 8 adds `emit_top_n_cells(buf, top_n)` next to it.
- `crates/alloc-bench-aggregator/templates/index.html.tmpl` — existing `{{for}}` loops for per-scenario blocks. Phase 8 adds a new `{{for}}` loop for top-N cards.
- `recommend.rs::env_short_name()` — private helper (Phase 7 Plan 02), returns short env labels for card titles. Phase 8 needs `pub(crate)` visibility (or a re-export through markdown.rs's existing helper module) — minor visibility tweak; Plan 8 will resolve exact path.
- `recommend.rs::cell_is_suspect()` — private helper, but its OUTPUT (`suspect_flag: bool`) is already on `CellRecommendation`. Phase 8 templates only reference the field, never the helper.

### Established Patterns
- Decorate-not-rewrite: NO mutation of `crates/alloc-bench-core/src/output.rs`. All Phase 8 work is aggregator-side rendering of Phase 7's already-computed `CellRecommendation`.
- Byte-identical output discipline: BTreeMap/BTreeSet alphabetical iteration (already enforced upstream in `top_n_cells`); deterministic numeric formatting per CLAUDE.md Conventions; the `{:.1}` for composite scores in the section table (matches Phase 7 plan).
- Single source of truth for prose: REC-01 prose fields are computed in Phase 7's `recommend.rs`; Phase 8 templates do nothing beyond field substitution. NO new prose-derivation logic in markdown.rs / html.rs / templates.
- Suspect-run flagging convention: `*(suspect)*` italic suffix on `(alloc, env)` cell rows; same six bytes across surfaces.
- Tinytemplate brace escape: `\{` for literal `{`. Phase 4 RESEARCH §Pitfall 1 contract; existing `tinytemplate_compiles_index_template` test extends to cover the two new templates.
- WR-01-pattern drift defense: render two views of the same struct, sentinel-check both contain every field (REQUIREMENTS CELL-02 verbatim test name).
- File overwrite semantics: existing emitters overwrite on re-run; stale files not cleaned. Phase 8 follows.

### Integration Points
- `main.rs` orchestration order: `markdown::write(out_dir, &outcome)` → `html::write(out_dir, &outcome, ...)`. Phase 8 inserts `score::score_cells` + `recommend::top_n_cells` calls UPSTREAM of both writers (or threads the resulting `Vec<CellRecommendation>` through `outcome` — exact wiring is Plan 8 to determine; recommend threading via a new field on the orchestration context struct).
- `markdown::write_report` already builds the REPORT.md string sequentially (line 65: `emit_recommendations(&mut buf, &outcome.runs);`). Phase 8 adds `emit_top_n_cells(&mut buf, &outcome.top_n)` immediately after that line.
- `html::write` constructs the tinytemplate context object. Phase 8 extends the context with `top_n_visible` and `top_n_collapsed` fields, both `Vec<CellRecommendation>`.
- Phase 7 already shipped 21 `recommend::tests` (10 untouched + 11 new); Phase 8 adds the 1 WR-01-pattern test in `html::tests` plus any markdown-emit golden tests.

</code_context>

<canonical_refs>
## Canonical References

| Path | Why this is canonical |
|------|----------------------|
| `.planning/REQUIREMENTS.md` (lines 31-37, CELL-01..05) | Locked requirements for Phase 8 |
| `.planning/ROADMAP.md` (Phase 8 entry) | Phase goal, dependencies, success criteria |
| `.planning/PROJECT.md` | Decorate-not-rewrite + BTreeMap discipline + suspect-run convention |
| `.planning/phases/07-scoring-top-n/07-02-SUMMARY.md` | What `recommend.rs` actually shipped (CellRecommendation fields, TOP_N_*, helpers) |
| `crates/alloc-bench-aggregator/src/recommend.rs` | Source of truth for `CellRecommendation` struct + `top_n_cells` + TOP_N_* constants |
| `crates/alloc-bench-aggregator/src/html.rs` (lines 28-34, 228-230, 339-345) | Tinytemplate registration + brace-escape compile-time test pattern |
| `crates/alloc-bench-aggregator/src/markdown.rs` (lines 35-50, 340-365) | `emit_recommendations` precedent for in-buffer section emission |
| `crates/alloc-bench-aggregator/templates/index.html.tmpl` | Existing `{{for}}` loop conventions to follow |
| `./CLAUDE.md` (Conventions section) | Suspect-flag thresholds + byte-identical-output + numeric formatting rules |

</canonical_refs>

<specifics>
## Specific Ideas

- The two template files MUST reference field names verbatim from `CellRecommendation` (rank, alloc, env, composite_score, axes, tldr, strengths, weaknesses, recommended_for, avoid_for, suspect_flag). Tinytemplate compile-error fails the build if a template references a non-existent field. The WR-01-pattern test then asserts BOTH templates reference EVERY field by sentinel-rendering and substring-checking.
- The `{{for}}` loops over `strengths` / `weaknesses` / `recommended_for` / `avoid_for` need tinytemplate's iteration syntax: `{{ for item in strengths }}- {item}\n{{ endfor }}`. The `\n` is a literal newline (tinytemplate preserves whitespace inside `{{for}}` bodies).
- The `*(suspect)*` annotation is conditional: `{{ if suspect_flag }} *(suspect)*{{ endif }}` — leading space (so it doesn't run into the title), exact six-byte literal between asterisks. Both templates use the same conditional pattern.
- The composite_score number is NOT rendered inside the card body (per Area 1 Q4 decision). It IS rendered in the section's leading table (above the cards): `| Rank | Cell | Score |` table summarizing all 10 cells, then the cards below. Plan 8 to decide if this leading table is in scope or deferred — recommend in-scope (it's the navigation index for the cards section).
- The HTML template's `<article class="recommend-card" id="recommend-{rank:02d}-{alloc}-{env}">` uses tinytemplate's value substitution; the `{rank:02d}` zero-pad needs care — tinytemplate's default formatter is `Display` for `usize`, which outputs `1` not `01`. Solution: pre-format the rank as a `String` Rust-side (`format!("{:02}", rank)`) and pass it as a separate `rank_padded` field in the template context, OR add a custom tinytemplate formatter `pad02` registered via `tt.add_formatter`. Recommend the former (simpler, no new formatters).
- Standalone fragment files: each `.md` and `.html` file's tail `\n` makes them concatenable / cat-able cleanly; recommend writing with `format!("{}\n", rendered)`.

</specifics>

<deferred>
## Deferred Ideas

- **Per-cell drilldown navigation** (clicking a spider chart navigates to its `recommend-{rank:02d}-{alloc}-{env}.html` panel) — **V12-04** (v1.2)
- **Stale-fragment cleanup** (when the top-10 set changes between aggregates) — out of scope; no observed accumulation issue with stable 18-cell matrix
- **Rich Markdown features** (links, nested headers, tables inside cards) — `tinytemplate` default formatter is sufficient for v1.1 plain-text bullets; pulldown-cmark integration is **V12-03** (v1.2)
- **Custom CSS for `.recommend-card`** (visual polish on the index.html dashboard) — out of scope for v1.1; existing dashboard CSS handles spacing; cards inherit the `.report-content` wrapper styling
- **Per-cell expandable axis-score breakdowns** (clicking "Strengths" reveals all 8 axis scores) — out of scope; the spider chart in Phase 9 IS the per-axis visual
- **Pareto-front overlay column** on the Recommendations table — **POLAR-05** (Phase 9, P2 differentiator)

</deferred>

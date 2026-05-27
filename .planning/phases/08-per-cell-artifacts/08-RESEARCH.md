# Phase 8: Per-cell Artifacts - Research

**Researched:** 2026-05-27
**Domain:** tinytemplate-driven static rendering of `CellRecommendation` to Markdown + HTML
**Confidence:** HIGH

## Summary

Phase 8 ships two new tinytemplate files (`recommend-cell.md.tmpl`, `recommend-cell.html.tmpl`) that render the same `CellRecommendation` struct (already shipped by Phase 7) into byte-aligned Markdown and HTML cards. The infrastructure all exists: tinytemplate is already a workspace dep at version 1, `html.rs::render` already does `tt.add_template("index", TEMPLATE)` and `tt.render`, `recommend.rs::top_n_cells` already produces `Vec<CellRecommendation>` of length ≤10, and `markdown.rs::emit_recommendations` is a clean precedent for in-buffer section emission. **Phase 7's `top_n_cells` is NOT yet wired into `main.rs`** — Plan 8 must thread `score_cells → top_n_cells` upstream of both writers.

**Primary recommendation:** Use `{{ call recommend-cell-md with cell }}` (tinytemplate's nested-template invocation) inside parent `{{for cell in top_n_visible}}` loops in `index.html.tmpl`. For REPORT.md, render the per-cell template 10 times in Rust (`tt.render("recommend-cell-md", &cell)`) and concatenate strings with `---\n` separators, since REPORT.md is built imperatively in Rust (not by a parent template).

The `## Top 10 cells` section is pure additive content — it doesn't touch any of the four existing v1.0 sections in REPORT.md or any existing block in `index.html`. The WR-01 sentinel test pattern is novel (not already in the codebase) but the closest precedent is `html::tests::tinytemplate_compiles_index_template` (line 339 of html.rs) which gives the structural template for the new `cell_templates_both_reference_all_fields` test.

## User Constraints (from CONTEXT.md)

### Locked Decisions

All Phase 8 decisions are locked by `08-CONTEXT.md` and `08-UI-SPEC.md`. Highlights the planner MUST honor:

- **Two template files** at `crates/alloc-bench-aggregator/templates/recommend-cell.{md,html}.tmpl`, both driven by the same `CellRecommendation` struct (CELL-01, CELL-02).
- **Card section order:** TL;DR → Strengths → Weaknesses → Recommended for → Avoid for (CELL-05).
- **Card title:** `### {rank}. {alloc}/{env_short}` with optional `*(suspect)*` suffix (six-byte literal, identical bytes across surfaces).
- **Filename pattern:** `report/recommend-{rank:02d}-{alloc}-{env}.{md,html}` — rank zero-padded; full env name from `Run.env` (CELL-03).
- **REPORT.md split:** Top-5 always visible; ranks 6-10 inside `<details><summary>Show ranks 6–10</summary>...</details>` (CELL-04).
- **`index.html` integration:** New `<section class="top-n-recommendations">` after the existing `<section class="report-mirror">` block, before per-scenario chart blocks. `top_n_visible` (1-5) and `top_n_collapsed` (6-10) are split Rust-side; the template body has two `{{for}}` loops separated by the `<details>` wrapper.
- **Caption text:** `Ranked 1-10 by composite score (equal-weighted across 8 axes). Cards 6-10 collapsed by default.` — verbatim across surfaces.
- **WR-01 sentinel test:** `html::tests::cell_templates_both_reference_all_fields` (path locked by REQUIREMENTS).
- **No new prose-derivation logic** in markdown.rs / html.rs / templates — Phase 8 is field-substitution only.
- **No mutation** of `crates/alloc-bench-core/src/output.rs` (decorate-not-rewrite).

### Claude's Discretion (research recommends)

- **Bold-label formatting in Markdown:** Newline between `**Strengths**` and the bullet list (cleaner GFM render than `**Strengths:**` inline).
- **Visible/collapsed split location:** Split before `tt.render` (cleaner — pass two `Vec<CellRecommendation>` slices into the context).
- **Rank zero-padding:** Pre-format Rust-side as a `String` field (`rank_padded: String`) on the per-cell context — simpler than registering a custom `pad02` formatter via `tt.add_formatter`.
- **Trailing newline on fragment files:** `\n` (POSIX convention; matches existing emit style).
- **Test name spelling:** `cell_templates_both_reference_all_fields` (REQUIREMENTS-pinned).

### Deferred Ideas (OUT OF SCOPE)

- Per-cell drilldown navigation (V12-04, v1.2)
- Stale-fragment cleanup
- pulldown-cmark integration (V12-03)
- Custom CSS for `.recommend-card`
- Per-cell expandable axis-score breakdowns
- Pareto-front overlay column (POLAR-05, Phase 9)

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CELL-01 | Two tinytemplate files (`recommend-cell.md.tmpl`, `recommend-cell.html.tmpl`) driven by same struct | Section 2 (tinytemplate API), Section 3 (existing render pattern in html.rs:227-248) |
| CELL-02 | WR-01-pattern test `html::tests::cell_templates_both_reference_all_fields` | Section 4 (drift-test pattern; closest existing analog at html.rs:339) |
| CELL-03 | Ten `.md` + ten `.html` standalone fragments at `report/recommend-{rank:02d}-{alloc}-{env}.{md,html}` | Section 3 (file-write pattern in `markdown::write` / `html::write`) |
| CELL-04 | `## Top 10 cells` section in REPORT.md (top-5 visible, 6-10 in `<details>`); `<section class="top-n-recommendations">` in index.html | Section 3 (REPORT.md plug-in point at markdown.rs:65; HTML context at html.rs:60-96) |
| CELL-05 | Card structure TL;DR → Strengths → Weaknesses → Recommended-for → Avoid-for, 80-150 words, data-derived only | Section 2 (tinytemplate `{{for}}`/`{{if}}` syntax); Section 5 (suspect-bytes invariant) |

---

## Section 1 — Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (Rust 2021, rust-toolchain.toml pin = 1.95) |
| Config file | `Cargo.toml` workspace + `crates/alloc-bench-aggregator/Cargo.toml` |
| Quick run command | `cargo test -p alloc-bench-aggregator --lib` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CELL-01 | Two `.tmpl` files exist & compile against `CellRecommendation` | unit | `cargo test -p alloc-bench-aggregator html::tests::tinytemplate_compiles_recommend_cell -x` | ❌ Wave 0 (new test extending existing precedent at html.rs:339) |
| CELL-02 | Sentinel-render both templates; assert every `CellRecommendation` field's sentinel substring appears in BOTH outputs | unit | `cargo test -p alloc-bench-aggregator html::tests::cell_templates_both_reference_all_fields -x` | ❌ Wave 0 (the WR-01-pattern test — REQUIREMENTS-pinned name + path) |
| CELL-03 | `just aggregate` writes 10 `.md` + 10 `.html` fragments at expected paths | integration | `cargo test -p alloc-bench-aggregator --test aggregate_writes_per_cell_fragments` (or similar) | ❌ Wave 0 |
| CELL-04 | REPORT.md contains `## Top 10 cells` + `<details>` block with ranks 6-10 | unit | `cargo test -p alloc-bench-aggregator markdown::tests::emit_top_n_cells_section_splits_at_top_n_table` | ❌ Wave 0 |
| CELL-04 | `index.html` contains `<section class="top-n-recommendations">` after `.report-mirror` | unit | `cargo test -p alloc-bench-aggregator html::tests::index_contains_top_n_recommendations_section` | ❌ Wave 0 |
| CELL-05 | Each card body has TL;DR + 4 sections (no hand-edited prose; only `*(suspect)*` annotation allowed) | unit | covered by CELL-02 sentinel test (substring-presence proves field-substitution discipline) | ✓ via CELL-02 |
| (regression gate) | All v1.0 byte-identical golden tests still pass | integration | `cargo test --workspace` (existing aggregator suite of 113 tests) | ✓ |

### Sampling Rate

- **Per task commit:** `cargo test -p alloc-bench-aggregator --lib` (~5-15s; covers all unit tests in markdown.rs, html.rs, recommend.rs)
- **Per wave merge:** `cargo test --workspace` (~1-2 min; full suite incl. 28 integration tests)
- **Phase gate:** Full suite green + `just aggregate` produces the 20 expected files (10 .md + 10 .html) before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `crates/alloc-bench-aggregator/templates/recommend-cell.md.tmpl` — new template file
- [ ] `crates/alloc-bench-aggregator/templates/recommend-cell.html.tmpl` — new template file
- [ ] `crates/alloc-bench-aggregator/src/markdown.rs::tests::emit_top_n_cells_*` — new tests (none exist for this section yet)
- [ ] `crates/alloc-bench-aggregator/src/html.rs::tests::cell_templates_both_reference_all_fields` — REQUIREMENTS-pinned test (the WR-01 sentinel)
- [ ] `crates/alloc-bench-aggregator/src/html.rs::tests::tinytemplate_compiles_recommend_cell_md` and `_html` — extends the existing `tinytemplate_compiles_index_template` precedent (html.rs:339)
- [ ] No framework install needed (tinytemplate already at workspace dep `tinytemplate = "1"` per Cargo.toml line 32)

---

## Section 2 — Library Surface (tinytemplate)

### Version & Project Usage

- **Workspace pin:** `tinytemplate = "1"` (Cargo.toml root, line 32) — cited from Phase 4. Active project usage:
  - `crates/alloc-bench-aggregator/src/html.rs:28` — `use tinytemplate::TinyTemplate;`
  - `crates/alloc-bench-aggregator/src/html.rs:34` — `const TEMPLATE: &str = include_str!("../templates/index.html.tmpl");`
  - `crates/alloc-bench-aggregator/src/html.rs:228-230` — `let mut tt = TinyTemplate::new(); tt.add_template("index", TEMPLATE).context("compiling index.html.tmpl")?;`
  - `crates/alloc-bench-aggregator/src/html.rs:247` — `tt.render("index", &ctx)`

`templates/index.html.tmpl` is the only existing template file. **No `{{for}}` or `{{if}}` directives appear in the existing template** (verified by `grep` for `{{ for|{{ if|{{ endfor|{{ endif`). Phase 8 introduces these patterns to the project for the first time. [VERIFIED: codebase grep]

### Syntax Reference (CITED: docs.rs/tinytemplate)

| Feature | Syntax | Example |
|---------|--------|---------|
| Value substitution | `{ field_name }` | `{tldr}` — DEFAULT formatter HTML-escapes `<`/`>`/`&`/`"` |
| Unescaped substitution | `{ field \| unescaped }` | already used for inlined JSON in `index.html.tmpl` (lines 261-279) |
| Custom formatter | `{ field \| pipe_to_named_formatter }` | requires `tt.add_formatter("name", \|val, out\| ...)` Rust-side |
| Iteration | `{{ for value_name in path }}...{{ endfor }}` | `{{ for s in strengths }}- {s}\n{{ endfor }}` |
| Conditional | `{{ if path }}...{{ endif }}` (optional `{{ else }}`) | `{{ if suspect_flag }} *(suspect)*{{ endif }}` |
| Negation | `{{ if not path }}` | not needed for Phase 8 |
| Nested template call | `{{ call template_name with path }}` | `{{ call recommend-cell-html with cell }}` — looks up `cell` in current context, renders that template against it |
| Literal `{` escape | `\{` | already used in `index.html.tmpl:281,282,290,291...` for JS object literals |

[CITED: https://docs.rs/tinytemplate/latest/tinytemplate/syntax/index.html — confirmed via WebFetch 2026-05-27]

### Phase 8 Application

**`recommend-cell.md.tmpl` (sketch):**
```text
### {rank}. {alloc}/{env_short}{{ if suspect_flag }} *(suspect)*{{ endif }}

{tldr}

**Strengths**

{{ for s in strengths }}- {s}
{{ endfor }}
**Weaknesses**

{{ for w in weaknesses }}- {w}
{{ endfor }}
**Recommended for**

{{ for c in recommended_for }}- {c}
{{ endfor }}
**Avoid for**

{{ for c in avoid_for }}- {c}
{{ endfor }}
```

**`recommend-cell.html.tmpl` (sketch):**
```text
<article class="recommend-card" id="recommend-{rank_padded}-{alloc}-{env}">
  <h3>{rank}. {alloc}/{env_short}{{ if suspect_flag }} *(suspect)*{{ endif }}</h3>
  <p>{tldr}</p>
  <p><strong>Strengths</strong></p>
  <ul>{{ for s in strengths }}<li>{s}</li>{{ endfor }}</ul>
  <p><strong>Weaknesses</strong></p>
  <ul>{{ for w in weaknesses }}<li>{w}</li>{{ endfor }}</ul>
  <p><strong>Recommended for</strong></p>
  <ul>{{ for c in recommended_for }}<li>{c}</li>{{ endfor }}</ul>
  <p><strong>Avoid for</strong></p>
  <ul>{{ for c in avoid_for }}<li>{c}</li>{{ endfor }}</ul>
</article>
```

**Field-name discipline:** Both templates reference field names verbatim from `CellRecommendation`: `rank`, `alloc`, `env`, `composite_score`, `axes`, `tldr`, `strengths`, `weaknesses`, `recommended_for`, `avoid_for`, `suspect_flag`. The HTML template additionally references `env_short` and `rank_padded` — these are NOT fields on `CellRecommendation`. Two options:

1. **Pre-compute Rust-side and pass via a wrapper struct** that implements `serde::Serialize` with the extra fields. Recommended — keeps the template field-substitution-only (no helper formatters in tinytemplate's namespace), and is explicit about which strings are derived where.
2. **Add `env_short` and `rank_padded` as fields on `CellRecommendation`.** Rejected — mutates a Phase 7 struct that's already locked + tested with 21 unit tests; the WR-01 sentinel test would also need to assert these new fields appear in both outputs, expanding the test surface unnecessarily.

**Recommendation:** Option 1 — a private `CellTemplateContext { rank, rank_padded, alloc, env, env_short, composite_score, axes, tldr, strengths, weaknesses, recommended_for, avoid_for, suspect_flag }` struct in `html.rs` (and a parallel one in `markdown.rs` if needed; both can share a `pub(crate)` builder fn). The WR-01 sentinel still asserts against the underlying `CellRecommendation` field substrings.

### `add_formatter` API (FYI, NOT NEEDED)

```rust
tt.add_formatter("pad02", |val, out| {
    if let serde_json::Value::Number(n) = val {
        if let Some(u) = n.as_u64() {
            return write!(out, "{:02}", u).map_err(|e| e.into());
        }
    }
    Err(tinytemplate::error::Error::GenericError { msg: "pad02 expects integer".to_string() })
});
```

Used as `{ rank | pad02 }`. **Not recommended for Phase 8** — pre-formatting Rust-side is simpler and matches existing patterns (no custom formatters are registered in the project today; only the built-in `unescaped` is used).

---

## Section 3 — Existing Code Map

Exact paths, function names, and line numbers for plug-in points. All values verified against the working tree at commit `93b893a`.

### `crates/alloc-bench-aggregator/Cargo.toml`
- Line 20: `tinytemplate = { workspace = true }`

### Cargo.toml (workspace root)
- Line 32: `tinytemplate = "1"`

### `crates/alloc-bench-aggregator/src/main.rs`
- **Line 69-70 — orchestration order (CRITICAL plug-in point):**
  ```rust
  markdown::write(&outcome, &metas, out_dir)?;
  html::write(&outcome, &metas, out_dir)?;
  ```
  Phase 8 must compute `score_cells(...) → top_n_cells(...)` BEFORE these calls and thread the resulting `Vec<CellRecommendation>` into both writers. Two wiring options (Plan 8 to choose):
  - **A:** Add a new field `top_n: Vec<CellRecommendation>` to `LoadOutcome` (defined in `loader.rs`). Cleanest from caller perspective but mutates a load-only type with rendering data.
  - **B:** Compute it in `main.rs` and pass as a new parameter to both writers: `markdown::write(&outcome, &metas, &top_n, out_dir)?` / `html::write(&outcome, &metas, &top_n, out_dir)?`. Recommended — explicit, no type pollution.
  - **C (CONTEXT.md notes):** Compute inside each writer (`markdown::write` calls `score_cells` + `top_n_cells` itself). Rejected — duplicates work + makes the two writers dependent on `score.rs` (currently they aren't).

### `crates/alloc-bench-aggregator/src/markdown.rs`
- **Line 33-37 — imports:** `use crate::diagrams::ALL_DIAGRAMS; use crate::html::is_suspect; use crate::loader::{CellMeta, LoadOutcome}; use crate::multi_run::{aggregate as mr_aggregate, is_high_variance, MultiRunStats}; use crate::recommend::recommendations;`
  Phase 8 adds: `use crate::recommend::{top_n_cells, CellRecommendation, TOP_N_TABLE, TOP_N_TOTAL};` (or `score_cells` if Plan 8 chooses to compute inside `write` rather than thread through `main.rs`).
- **Line 43-52 — `pub fn write(outcome: &LoadOutcome, metas: &HashMap<...>, out_dir: &Path) -> Result<()>`** — entry point. Phase 8 either extends the signature (option B above) or assumes top_n is on `LoadOutcome` (option A).
- **Line 56-70 — `pub(crate) fn build_report(outcome: &LoadOutcome, metas: &HashMap<...>) -> String`** — sequential section emitter. **Line 65 is the precise insertion point** for the new section:
  ```rust
  emit_recommendations(&mut buf, &outcome.runs);
  // ↓ ADD HERE: emit_top_n_cells(&mut buf, &top_n);
  if !outcome.skipped.is_empty() {
      emit_skipped(&mut buf, &outcome.skipped);
  }
  ```
  Per CONTEXT.md: section sits AFTER `## Recommendations` (currently line 65 emits recommendations first) and BEFORE per-scenario tables — but `build_report` already emits per-scenario tables FIRST (line 62), then docker-runtimes, diagrams, and recommendations. Re-read the order:
  ```
  Line 61: emit_header
  Line 62: emit_per_scenario_tables          ← per-scenario already FIRST
  Line 63: emit_docker_runtimes_table
  Line 64: emit_allocator_diagrams
  Line 65: emit_recommendations               ← Recommendations LAST among content
  Line 66-68: emit_skipped (conditional)
  ```
  **The CONTEXT.md decision says "after the Recommendations table, before per-scenario tables" but the existing REPORT.md ordering puts per-scenario FIRST.** This is a discrepancy worth flagging in Open Questions (Section 6) — the planner needs to decide whether to (a) honor CONTEXT.md verbatim and reorder existing sections (NOT recommended — breaks v1.0 byte-identity for everything between), or (b) interpret CONTEXT.md as "right after `emit_recommendations` finishes" (line 66 in `build_report`). **Recommend option b** — minimal disruption to v1.0 reader expectations and keeps ordering sequential through `build_report`.
- **Line 340-350 — `fn emit_recommendations(buf: &mut String, runs: &[Run])`** — the canonical precedent for in-buffer section emission. Phase 8's `emit_top_n_cells` mirrors this exact shape: write a heading, blank line, optional caption, body, trailing blank line.
- **Line 408-414 — `fn suspect_note(reason: &SuspectReason) -> &'static str`** — emits `*(\u{26A0} suspect: low samples)*` etc. **NOT what Phase 8 cards use.** Per CONTEXT.md and UI-SPEC.md, Phase 8 cards emit the simpler six-byte form `*(suspect)*` (no `⚠`, no reason qualifier). The cells in per-scenario tables continue to use `suspect_note`'s richer form; Phase 8 cards use the title-suffix pattern. **No code reuse here** — new literal `*(suspect)*` byte sequence in the per-cell template.
- **Line 366-368 — `pub(crate) fn env_label(env: &Env) -> &str`** — returns the docker_image string or `"host"`. Phase 8 needs the SHORT form (`debian`, `alpine`) instead — that lives in `recommend.rs::env_short_name` at line 451-466 (currently private). **Visibility tweak required:** make `env_short_name` `pub(crate)` so `markdown.rs` and `html.rs` can call it when building the per-cell template context. Or, equivalently, add `env_short` as a String to the new `CellTemplateContext` builder fn that lives in `recommend.rs` (cleanest — keeps `env_short_name` private; Phase 8 just consumes whatever Phase 7 already exports indirectly through the builder).

### `crates/alloc-bench-aggregator/src/html.rs`
- **Line 23-32 — imports:** `use std::collections::{BTreeMap, BTreeSet, HashMap}; use std::path::Path; use alloc_bench_core::output::{HarnessInfo, Run}; use anyhow::{Context, Result}; use tinytemplate::TinyTemplate; use crate::loader::{CellMeta, LoadOutcome}; use crate::markdown::env_label; use crate::multi_run::{aggregate as mr_aggregate, MultiRunStats};`
  Phase 8 adds: `use crate::recommend::{CellRecommendation, TOP_N_TABLE};`
- **Line 34 — `const TEMPLATE: &str = include_str!("../templates/index.html.tmpl");`**
  Phase 8 adds two parallel consts:
  ```rust
  const RECOMMEND_CELL_HTML: &str = include_str!("../templates/recommend-cell.html.tmpl");
  // and (if used here for parity, otherwise lives in markdown.rs):
  const RECOMMEND_CELL_MD: &str = include_str!("../templates/recommend-cell.md.tmpl");
  ```
- **Line 60-96 — `struct HtmlContext<'a>`** — the tinytemplate context. Phase 8 extends this with two new fields:
  ```rust
  top_n_visible: &'a [CellTemplateContext],   // ranks 1..=TOP_N_TABLE
  top_n_collapsed: &'a [CellTemplateContext], // ranks TOP_N_TABLE+1..=TOP_N_TOTAL
  ```
  (or pass the full top-N list and let the template slice — but tinytemplate's `{{for}}` doesn't support index-based slicing, so the split MUST happen Rust-side per CONTEXT.md.)
- **Line 109-118 — `pub fn write(outcome: &LoadOutcome, _metas: &HashMap<...>, out_dir: &Path) -> Result<()>`** — entry point. Phase 8 extends signature to accept `top_n: &[CellRecommendation]` (or threads via `LoadOutcome`).
- **Line 155-225 — `fn build_context(runs: &[Run]) -> Result<BuiltContext>`** — JSON-context builder. Phase 8 extends to also build the per-cell template-context vector (split into visible + collapsed). New helper: `fn build_cell_template_contexts(top_n: &[CellRecommendation]) -> (Vec<CellTemplateContext>, Vec<CellTemplateContext>)` (split at index `TOP_N_TABLE`).
- **Line 227-248 — `fn render(runs: &[Run]) -> Result<String>`** — registers + renders. Phase 8 adds two `tt.add_template` calls:
  ```rust
  tt.add_template("recommend-cell-html", RECOMMEND_CELL_HTML)
      .context("compiling recommend-cell.html.tmpl")?;
  ```
  `index.html.tmpl` invokes the per-cell template via `{{ call recommend-cell-html with cell }}` inside its `{{for cell in top_n_visible}}` loops — this lookup-by-name is what the parent `tt.render("index", ...)` resolves at render time. `tt.add_template` MUST be called for both BEFORE `tt.render`.
- **Line 339-343 — `fn tinytemplate_compiles_index_template`** — the canonical brace-escape compile-time test. Phase 8 ADDS two parallel tests (or extends this one to register all three templates): `tinytemplate_compiles_recommend_cell_md` and `tinytemplate_compiles_recommend_cell_html`.

### `crates/alloc-bench-aggregator/src/recommend.rs`
- **Line 30 — `use crate::score::CellScore;`** — Phase 7 wired this. Phase 8 may add `use crate::score::score_cells` (or call the canonical entry point if Plan 8 invokes it from main.rs).
- **Line 42-50 — `pub const TOP_N_SPIDER: usize = 3; pub const TOP_N_TABLE: usize = 5; pub const TOP_N_TOTAL: usize = 10;`** — the constants Phase 8 templates and emitters consume. NO magic numbers in Phase 8.
- **Line 138-151 — `pub struct CellRecommendation`** — 11 locked fields (rank, alloc, env, composite_score, axes, tldr, strengths, weaknesses, recommended_for, avoid_for, suspect_flag). The WR-01 sentinel test asserts every one of these field names produces a substring match in BOTH rendered outputs.
- **Line 619-671 — `pub fn top_n_cells(scores: Vec<CellScore>, runs: &[Run]) -> Vec<CellRecommendation>`** — the function Phase 8 invokes (probably from main.rs upstream of both writers).
- **Line 451-466 — `fn env_short_name(r: &Run) -> String`** — currently private. Plan 8 may need to expose as `pub(crate)` OR Phase 7's existing `top_n_cells` already populates `cell.env` with the short form (line 658: `env: cell.env.clone()` where `cell` is `CellScore` and `score.rs::CellScore.env` is set via the `env_short_name` extractor in score.rs:125). **Verified by inspection of `recommend.rs:619-671`:** `cell.env` IS the short form ("alpine", "debian-slim", etc.) because `score::top_n` returns `CellScore`s whose `env` field was populated by `score.rs::env_short_name`. Therefore **`CellRecommendation.env` already contains the short form** — Phase 8 templates can use `{env}` directly for the title without a separate `env_short` field. CONTEXT.md's "env_short" naming is interchangeable with `cell.env` here.

  **However** — the filename pattern in CELL-03 is `recommend-{rank:02d}-{alloc}-{env}.{md,html}`. If `cell.env` is short-form (`debian`), filenames are `recommend-01-mimalloc-debian.md`. CONTEXT.md says "full env name as it appears in `Run.env` (e.g., `linux-glibc-jemalloc`, `linux-musl-mallocng`, `macos-libmalloc`). Full env names keep filenames unambiguous, round-trippable to source `Run` records, and grep-friendly." This is a **discrepancy** — `Run.env` is a struct (`Env { os, os_version, docker_image, ... }`), not a kebab-case string. The "linux-glibc-jemalloc" example doesn't match any field. Two interpretations:
  1. CONTEXT.md author meant `Run.env.docker_image` (string like `alloc-bench:jemalloc-alpine`) — but that has a colon and prefix.
  2. CONTEXT.md author meant the kebab-case env identifier used elsewhere in the codebase (e.g., `linux-glibc`, `linux-musl`, `linux-distroless-cc`).

  **Recommendation:** Plan 8 should clarify this with the user. For now, research notes that `cell.env` (short form, e.g., `alpine`, `debian-slim`, `wolfi`, `host`, `distroless-cc`, `distroless-static`, `scratch`) matches what `recommend.rs::env_short_name` already extracts — these are still unambiguous (each is unique within the 6-env Linux matrix + macOS host) and grep-friendly. **Use `cell.env` directly** in filenames and titles; flag in Open Questions if the user wanted the longer form.

### `crates/alloc-bench-aggregator/templates/index.html.tmpl`
- 846 lines total. Existing template uses ONLY simple `{ name | unescaped }` substitutions (lines 261-279) and `\{` escapes for JS object literals. **No `{{for}}` / `{{if}}` / `{{call}}` directives exist.** Phase 8 introduces these patterns. The new `<section class="top-n-recommendations">` with two `{{for}}` loops + nested `{{call}}` invocation will be the first use in the project — extra care on the `tinytemplate_compiles_index_template` test (line 339 in html.rs) to catch mis-matched `{{endfor}}`/`{{endif}}` at compile time.

---

## Section 4 — WR-01 Drift Test Pattern

### Closest Existing Analog

The CONTEXT.md repeatedly references the "WR-01 winner-tiebreak drift" fix from v1.0. Search results confirm the v1.0 WR-01 issue was a winner-picker tiebreak drift (not a template drift). The closest **template-related** drift test in the existing codebase is:

**`crates/alloc-bench-aggregator/src/html.rs:339-343`:**
```rust
/// RESEARCH §Pitfall 1: a missed `\{` escape produces a TinyTemplate
/// compile error. This test catches the regression at `cargo test`
/// time instead of leaving it to a runtime mystery.
#[test]
fn tinytemplate_compiles_index_template() {
    let mut tt = TinyTemplate::new();
    tt.add_template("index", TEMPLATE)
        .expect("template should compile — missed `\\{` escape?");
}
```

This is a structural template — Phase 8 extends it to cover all three templates. The new `cell_templates_both_reference_all_fields` test is a **new pattern in the codebase** (sentinel-render + cross-surface field-presence assertion). No existing test does this.

### Recommended Test Shape (REQUIREMENTS-pinned name)

```rust
#[test]
fn cell_templates_both_reference_all_fields() {
    // 1. Build a CellRecommendation with sentinel substring values for
    //    every scalar field. Each sentinel is unique enough to grep for.
    let cell = CellRecommendation {
        rank: 7,                                     // numeric — rendered as "7"
        alloc: "_SENTINEL_ALLOC_".into(),
        env: "_SENTINEL_ENV_".into(),
        composite_score: 42.42,
        axes: {
            let mut m = BTreeMap::new();
            m.insert("channel_throughput", 99.0);
            // ... all 8 keys for completeness; values are not sentinel-grepped
            m
        },
        tldr: "_SENTINEL_TLDR_".into(),
        strengths: vec!["_SENTINEL_STRENGTH_A_", "_SENTINEL_STRENGTH_B_"],
        weaknesses: vec!["_SENTINEL_WEAKNESS_A_", "_SENTINEL_WEAKNESS_B_"],
        recommended_for: vec!["_SENTINEL_RECCLASS_"],
        avoid_for: vec!["_SENTINEL_AVOIDCLASS_"],
        suspect_flag: true,  // forces the {{ if suspect_flag }} branch
    };

    // 2. Build a CellTemplateContext from `cell` (adds env_short, rank_padded).
    let ctx = build_cell_template_context(&cell);  // pub(crate) helper

    // 3. Register both templates and render.
    let mut tt = TinyTemplate::new();
    tt.add_template("recommend-cell-md", RECOMMEND_CELL_MD).unwrap();
    tt.add_template("recommend-cell-html", RECOMMEND_CELL_HTML).unwrap();
    let md = tt.render("recommend-cell-md", &ctx).expect("md render");
    let html = tt.render("recommend-cell-html", &ctx).expect("html render");

    // 4. Assert every sentinel substring appears in BOTH outputs.
    for sentinel in &[
        "_SENTINEL_ALLOC_",
        "_SENTINEL_ENV_",
        "_SENTINEL_TLDR_",
        "_SENTINEL_STRENGTH_A_",
        "_SENTINEL_STRENGTH_B_",
        "_SENTINEL_WEAKNESS_A_",
        "_SENTINEL_WEAKNESS_B_",
        "_SENTINEL_RECCLASS_",
        "_SENTINEL_AVOIDCLASS_",
        "(suspect)",  // suspect_flag = true → both surfaces emit literal
    ] {
        assert!(md.contains(sentinel), "MD missing {sentinel}: {md}");
        assert!(html.contains(sentinel), "HTML missing {sentinel}: {html}");
    }

    // 5. Assert numeric fields appear in both (no sentinel — direct value match).
    assert!(md.contains("7. "), "MD missing rank prefix");      // "7. _SENTINEL_ALLOC_/_SENTINEL_ENV_"
    assert!(html.contains("7. "), "HTML missing rank prefix");
    // composite_score and axes are NOT in the per-cell template body
    // (per CONTEXT.md decision — they live in the section-leading table,
    // not the cards). So the test does NOT assert those fields.
}
```

**Coverage gap intentional:** `composite_score` and `axes` are NOT rendered inside the per-cell card per CONTEXT.md. The WR-01 test asserts every field that IS rendered. If a future contributor adds composite_score to the card (e.g., as a header annotation), the test must be expanded — but Phase 8 leaves it out per the locked decision.

### Companion Test (extends existing precedent)

```rust
#[test]
fn tinytemplate_compiles_recommend_cell_templates() {
    let mut tt = TinyTemplate::new();
    tt.add_template("recommend-cell-md", RECOMMEND_CELL_MD)
        .expect("md template should compile — missed `\\{` escape or `{{endfor}}`?");
    tt.add_template("recommend-cell-html", RECOMMEND_CELL_HTML)
        .expect("html template should compile — missed `\\{` escape or `{{endfor}}`?");
}
```

This extends the existing `tinytemplate_compiles_index_template` (line 339) to catch missed escapes and unbalanced `{{for}}` / `{{if}}` blocks at `cargo test` time.

---

## Section 5 — Pitfalls and Constraints

### Pitfall 1: Mismatched `{{endfor}}` / `{{endif}}` blocks

**What goes wrong:** A template body with `{{ for s in strengths }}` but no matching `{{ endfor }}` produces a runtime tinytemplate compile error — silent until `tt.add_template` is called.

**Why it happens:** Templates are pure text files; the editor doesn't validate them. The existing `index.html.tmpl` doesn't use loops, so the project has no muscle memory for this.

**How to avoid:** The new `tinytemplate_compiles_recommend_cell_templates` test catches this at `cargo test` time. Place it adjacent to the existing `tinytemplate_compiles_index_template` (line 339) so the precedent is obvious.

### Pitfall 2: Whitespace inside `{{for}}` bodies

**What goes wrong:** Tinytemplate preserves whitespace inside `{{for}}` bodies. A template like:
```
{{ for s in strengths }}
- {s}
{{ endfor }}
```
emits a leading blank line before the first bullet and a trailing blank line after the last one. Bullet lists rendered with trailing blank lines still parse as GFM, but the byte-identity contract is sensitive.

**How to avoid:** Use the compact form `{{ for s in strengths }}- {s}\n{{ endfor }}` — newline at the end of the bullet line, NO leading whitespace before `{{ for }}`. The template-render output then matches: `- a\n- b\n- c\n`.

Verify by inspecting the WR-01 sentinel-render output in test failures (the `{md}` / `{html}` debug-print includes the rendered bytes).

### Pitfall 3: HTML default-escape vs. unescaped on prose fields

**What goes wrong:** The default tinytemplate formatter HTML-escapes `<`, `>`, `&`, `"`. If a workload-class string contains `&` (e.g., `cpu&memory-bound` — hypothetical), the HTML emits `cpu&amp;memory-bound`. The Markdown side does not escape, so the bytes differ.

**Why this is OK for Phase 8:** The data-derived strings in `CellRecommendation` are all axis labels (`MEASUREMENT_AXES[i].label`, e.g., "CPU-bound throughput", "Memory / fragmentation") and workload-class names (`channel-heavy`, `cpu-bound`, `web-ser-de`). None contain HTML-meta characters. Verified by `axes.rs::MEASUREMENT_AXES` (Phase 6) and `recommend.rs::WorkloadClass::label` enum.

**Discipline:** The WR-01 sentinel test uses ASCII-safe sentinel strings (`_SENTINEL_STRENGTH_A_`) so the escape behavior never trips it. If a future axis or class label gains HTML-meta characters, the substring match would still work (sentinel ≠ label), but the byte-identity contract would weaken — flag in code review.

### Pitfall 4: Suspect annotation byte-identity

**What goes wrong:** The temptation is to emit the suspect suffix as `<em>(suspect)</em>` in HTML and `*(suspect)*` in Markdown. Bytes differ; the WR-01 test fails.

**The discipline (CONTEXT.md):** Both surfaces emit the **exact six bytes** `*(suspect)*` (asterisk + parenthetical + asterisk). In HTML, the asterisks render as plain text inside the `<h3>` — no `<em>` wrapper, no `<span class="suspect">` badge. The browser displays `*(suspect)*` literally. UI-SPEC.md confirms this is intentional.

**Distinction from per-scenario tables:** `markdown.rs::suspect_note` (line 408-414) emits the richer `*(\u{26A0} suspect: low samples)*` form for table cells. Phase 8 cards use the simpler form. The two forms coexist; Phase 8 does NOT replace `suspect_note` anywhere.

### Pitfall 5: BTreeMap iteration order in `axes` field

**What goes wrong:** `CellRecommendation.axes: BTreeMap<&'static str, f64>` iterates alphabetically by axis key (Phase 7 invariant). If the per-cell template includes an `{{ for (k, v) in axes }}` loop, it would render axes in alphabetical key order — not the `MEASUREMENT_AXES` declaration order.

**Why this matters:** Per CONTEXT.md decision, `axes` is NOT rendered in the per-cell card body (composite_score sits in a leading table; per-axis values sit in the Phase 9 spider chart). Phase 8 templates **do not iterate `axes`**. The WR-01 test does not need to assert axis-iteration order.

**However:** If Phase 8 adds the leading `## Top 10 cells` table (rank | cell | score) per CONTEXT.md "Specific Ideas", that table is rendered in `markdown.rs::emit_top_n_cells` directly (not via the per-cell template) and uses regular `for cell in top_n.iter()` Rust iteration. Top-N order from `top_n_cells` is composite-DESC, alphabetical-tiebreak — the natural sort.

### Pitfall 6: Filename collision risk on stale rerun

**What goes wrong:** The 18-cell × Phase 7 ranking is stable across runs given identical input data. But if the user re-aggregates with a different fixture (e.g., adds a new env), the top-10 set could shift. Files `recommend-{rank:02d}-{alloc}-{env}.md` from the old run are NOT cleaned up — they persist with stale rank numbers.

**Stance (CONTEXT.md):** Stale-fragment cleanup is OUT OF SCOPE for Phase 8 (deferred to v1.2). Acceptable because the 18-cell matrix is stable and cleanup is a polish item.

**Implementation note:** Each `.md` / `.html` write uses `std::fs::write` with overwrite semantics (matches the existing `markdown::write` and `html::write` orchestrators). No `OpenOptions::new().create_new(true)` — Plan 8 must verify the writer used preserves overwrite-on-rerun.

### Constraint: Byte-identical-output discipline (CLAUDE.md)

- **Alphabetical iteration:** `BTreeMap` / `BTreeSet`, never `HashMap` / `HashSet`. Phase 7 `top_n_cells` already does this; Phase 8 inherits naturally. The new `top_n_visible` / `top_n_collapsed` Vec slices from the top-N list preserve composite-DESC order (alphabetical tiebreak) — no new sort introduced.
- **Numeric formatting:** `{:.1}` for throughputs (single-run cells), `{:.0}` for medians (multi-run cells), `{}` for ns latencies. Phase 8 cards do NOT render numeric fields — the composite_score number sits in the leading table (if Plan 8 ships it; CONTEXT.md says "recommend in-scope"). If included, format as `{:.1}`.
- **Single non-stable line:** `<!-- generated by alloc-bench-aggregator at ... -->` at the top of REPORT.md is the only timestamped line. Phase 8 adds NO timestamps to per-cell fragments or to the new `## Top 10 cells` section.

### Constraint: NO new runtime dependencies

Per CONTEXT.md "Out of Scope" and the v1.1 milestone discipline ("Hand-roll 15-LOC normalizer; hard-code `↑`/`↓` Unicode literals; security sidecar reuses `serde_json` 1 (no `statrs`, no `unicode-arrows`, no `pulldown-cmark` for v1.1)"). Phase 8 uses ONLY tinytemplate (already in workspace) + std + existing deps. No new entries in `Cargo.toml`.

### Lessons from Phase 7

Per the Phase 7 plan-02 SUMMARY.md (read in full), Phase 7 had ONE auto-fixed deviation: a deferred import (`use crate::score::CellScore`) due to unused-import warning at Task 1 build time. **Lesson for Phase 8:** Add imports only at the task that consumes them. If Plan 8 splits "scaffold templates + register" from "wire into emitters", the `RECOMMEND_CELL_MD` / `RECOMMEND_CELL_HTML` constants and the `add_template` calls should land in the same task.

---

## Section 6 — Open Questions

1. **REPORT.md section ordering discrepancy** (flagged in Section 3, markdown.rs map)
   - CONTEXT.md says: "after the existing `## Recommendations` table and before the per-scenario tables"
   - Actual `build_report` order (markdown.rs:61-65): `header → per-scenario tables → docker-runtimes → diagrams → recommendations → skipped`
   - **Recommendation:** Insert `emit_top_n_cells` AFTER `emit_recommendations` (line 66 in `build_report`). This honors the spirit of CONTEXT.md (Top 10 cells is in the recommendations cluster, not buried by per-scenario detail) without reordering existing v1.0 sections (which would break byte-identity for everything between).
   - **Action for planner:** Confirm with user before locking — the literal "before the per-scenario tables" reading would require relocating per-scenario tables to the bottom, which is out of scope.

2. **Filename `{env}` interpretation** (flagged in Section 3, recommend.rs map)
   - CONTEXT.md gives examples like `linux-glibc-jemalloc` and `linux-musl-mallocng` for the filename's `{env}` token, claiming it's "the full env name as it appears in `Run.env`"
   - But `Run.env` is a struct, not a kebab-case string. The "linux-glibc" form is a kebab-case env identifier used in the GHA matrix and the justfile, NOT a field on `Run.env`.
   - The naturally available short form (`alpine`, `debian-slim`, `distroless-cc`, `distroless-static`, `scratch`, `wolfi`) is what `recommend.rs::env_short_name` extracts and what `CellRecommendation.env` already contains.
   - **Recommendation:** Use `cell.env` (short form) — unambiguous in the 7-env matrix (six Linux + macOS host), grep-friendly, and round-trippable to `Run.env.docker_image` via the inverse extraction. Plan 8 should clarify with user if longer form was intended.

3. **Leading `| Rank | Cell | Score |` summary table at top of `## Top 10 cells`** (flagged in CONTEXT.md "Specific Ideas")
   - CONTEXT.md says: "Plan 8 to decide if this leading table is in scope or deferred — recommend in-scope (it's the navigation index for the cards section)."
   - This is a small extra bit of work (10 rows × 3 cols) that lives in `markdown.rs::emit_top_n_cells` directly (not in the per-cell template). HTML side would be a parallel `<table>` block in the new `<section class="top-n-recommendations">`.
   - **Recommendation:** In-scope — small, navigationally useful, and keeps the composite_score visible somewhere (per CELL-05 the cards themselves don't show it).
   - **Action for planner:** If user disagrees, drop the leading table and surface composite_score elsewhere (or leave it out entirely).

4. **`env_short_name` visibility** (minor, flagged in Section 3)
   - Phase 8 needs the short env name to (a) build card titles, (b) build filenames. `CellRecommendation.env` already contains it (verified via Phase 7 plumbing).
   - **No action required** unless Plan 8 needs to call `env_short_name` directly on a `Run` (e.g., to filter runs to a specific cell). If so, expose as `pub(crate)`.

5. **Wiring location for `score_cells → top_n_cells` invocation**
   - Three options identified (Section 3, main.rs map). Recommendation: option B (compute in main.rs, pass via new parameter to both writers).
   - **Action for planner:** Pick one in Plan 8.

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
- Open Questions: MEDIUM — three of five are genuine ambiguities in CONTEXT.md (Q1, Q2, Q3); two are minor wiring choices

**Research date:** 2026-05-27
**Valid until:** 2026-06-27 (30 days; tinytemplate version pin is stable, codebase moves forward but Phase 8 plug-in points are stable)

## RESEARCH COMPLETE

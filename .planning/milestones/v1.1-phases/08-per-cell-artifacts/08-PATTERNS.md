# Phase 8: Per-cell Artifacts - Pattern Map

**Mapped:** 2026-05-27
**Files analyzed:** 6 (2 NEW templates, 4 MODIFY .rs files)
**Analogs found:** 6 / 6

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/alloc-bench-aggregator/templates/recommend-cell.md.tmpl` (NEW) | template | transform | `crates/alloc-bench-aggregator/templates/index.html.tmpl` (existing tinytemplate) | role-match (different output dialect: MD vs HTML) |
| `crates/alloc-bench-aggregator/templates/recommend-cell.html.tmpl` (NEW) | template | transform | `crates/alloc-bench-aggregator/templates/index.html.tmpl` | exact (HTML/tinytemplate) |
| `crates/alloc-bench-aggregator/src/markdown.rs` (MODIFY: add `emit_top_n_cells` + extend `build_report`/`write`) | renderer | transform | `markdown.rs::emit_recommendations` (lines 340-350) | exact (in-buffer section emit precedent) |
| `crates/alloc-bench-aggregator/src/html.rs` (MODIFY: add 2 templates + `CellTemplateContext` + extend `HtmlContext`/`render`) | renderer | transform | `html.rs::render` (lines 227-248) + `html.rs::HtmlContext` (lines 60-96) | exact (tinytemplate context-and-render precedent) |
| `crates/alloc-bench-aggregator/templates/index.html.tmpl` (MODIFY: add `<section class="top-n-recommendations">`) | template | transform | own `<section class="report-mirror">` block (lines 253-256) | exact (sibling section) |
| `crates/alloc-bench-aggregator/src/main.rs` (MODIFY: wire `compute_axes → score_cells → top_n_cells` upstream of writers) | orchestrator | request-response | `main.rs::main` (lines 56-77, current pipeline) | exact (sequential pipeline, same module style) |

## Pattern Assignments

### `crates/alloc-bench-aggregator/templates/recommend-cell.md.tmpl` (template, transform — NEW)

**Analog:** `crates/alloc-bench-aggregator/templates/index.html.tmpl` (only existing tinytemplate file in project; no `{{for}}`/`{{if}}` precedent — Phase 8 introduces these directives for the first time).

**Tinytemplate value-substitution pattern** (from `index.html.tmpl` line 261):
```text
const RESULTS = { results_json | unescaped };
```
- The default formatter HTML-escapes `<`/`>`/`&`/`"`. Phase 8 Markdown template uses **default-escape** (Markdown body escapes nothing meaningful in plain text). RESEARCH §Pitfall 3: axis labels and workload-class strings are ASCII-safe — no escape collisions.
- `{ field_name }` (single brace) is the substitution syntax. Phase 8 uses this for `rank`, `alloc`, `env`, `tldr`.

**Tinytemplate `\{` escape discipline** (from `index.html.tmpl` line 22):
```text
:root \{
```
- Every literal `{` MUST be escaped as `\{`. Markdown card body is unlikely to contain `{` (text content), but the compile-time test `tinytemplate_compiles_recommend_cell_*` (extends html.rs:339) catches any regression.

**Phase 8 application** (RESEARCH §2 sketch — verbatim canonical form):
```text
### {rank}. {alloc}/{env}{{ if suspect_flag }} *(suspect)*{{ endif }}

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

**Field-name discipline:** References `rank`, `alloc`, `env`, `tldr`, `strengths`, `weaknesses`, `recommended_for`, `avoid_for`, `suspect_flag` — these names MUST exactly match `CellRecommendation` field names (or the wrapping `CellTemplateContext` field names). RESEARCH §Section 2 recommends Option 1: a wrapper context struct adds `rank_padded` (and reuses `cell.env` directly, which is already the short form per Phase 7 Plan 02 — verified in `recommend.rs::top_n_cells` line 659 + `score::env_short_name`).

**Pitfall 2 (whitespace in `{{for}}`):** Use compact form `{{ for s in strengths }}- {s}\n{{ endfor }}` — newline at end of bullet line, NO leading whitespace before `{{ for }}`. Tinytemplate preserves whitespace inside loops; misplaced indentation produces leading/trailing blank lines.

---

### `crates/alloc-bench-aggregator/templates/recommend-cell.html.tmpl` (template, transform — NEW)

**Analog:** `crates/alloc-bench-aggregator/templates/index.html.tmpl` (HTML structure precedent).

**HTML semantic-element pattern** (from `index.html.tmpl` line 253-256):
```text
<section class="report-mirror">
  <h2>Per-scenario allocator comparison</h2>
  <div id="report-mirror-tables"></div>
</section>
```
- Section-level container with class hook for CSS, semantic h2 inside. Phase 8 mirrors this for `<article class="recommend-card">` with h3 inside.

**Phase 8 application** (RESEARCH §2 sketch — verbatim canonical form):
```text
<article class="recommend-card" id="recommend-{rank_padded}-{alloc}-{env}">
  <h3>{rank}. {alloc}/{env}{{ if suspect_flag }} *(suspect)*{{ endif }}</h3>
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

**`rank_padded` discipline:** Tinytemplate's default `Display` formatter outputs `1` (not `01`) for `usize`. Per CONTEXT.md "Specifics" ¶6 + RESEARCH §Section 2 Option 1 recommendation, pre-format Rust-side as `String` field on the wrapper context — DO NOT register a custom `pad02` formatter. Existing project has zero `add_formatter` calls; preserves discipline.

**Suspect-bytes invariant** (CONTEXT.md decision + UI-SPEC line 113): Both templates emit the literal six bytes `(suspect)` between asterisks — NO `<em>` wrapper, NO `<span class="suspect">` badge, NO `⚠` glyph. Byte-comparable across surfaces (the WR-01 sentinel test relies on identical bytes). The `markdown.rs::suspect_note` (line 408-414) richer form `*(\u{26A0} suspect: low samples)*` is for table cells — Phase 8 cards use the simpler form.

---

### `crates/alloc-bench-aggregator/src/markdown.rs` (renderer, transform — MODIFY)

**Analog:** `markdown.rs::emit_recommendations` (lines 340-350).

**Imports pattern** (lines 26-37):
```rust
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;
use std::path::Path;

use alloc_bench_core::output::{Env, HarnessInfo, Run};
use anyhow::{Context, Result};

use crate::diagrams::ALL_DIAGRAMS;
use crate::html::is_suspect;
use crate::loader::{CellMeta, LoadOutcome};
use crate::multi_run::{aggregate as mr_aggregate, is_high_variance, MultiRunStats};
use crate::recommend::recommendations;
```
**Phase 8 adds:** `use crate::recommend::{top_n_cells, CellRecommendation, TOP_N_TABLE, TOP_N_TOTAL};`

**Section-emit precedent** (lines 340-350):
```rust
fn emit_recommendations(buf: &mut String, runs: &[Run]) {
    let recs = recommendations(runs);
    let _ = writeln!(buf, "## Recommendations by workload");
    let _ = writeln!(buf);
    let _ = writeln!(buf, "| Workload | Recommended | Rationale |");
    let _ = writeln!(buf, "|---|---|---|");
    for r in recs.iter() {
        let _ = writeln!(buf, "| {} | {} | {} |", r.class, r.allocator, r.rationale);
    }
    let _ = writeln!(buf);
}
```
**Phase 8 mirrors this shape:** new `fn emit_top_n_cells(buf: &mut String, top_n: &[CellRecommendation])` — emits `## Top 10 cells` heading + caption + 5 visible cards + `<details>` block + 5 collapsed cards. Per CONTEXT.md "Section position": insert call AFTER `emit_recommendations(&mut buf, &outcome.runs);` (line 65) — RESEARCH §Section 6 Open Question 1 recommends "after emit_recommendations" rather than reordering existing sections.

**Build_report orchestration pattern** (lines 56-70):
```rust
pub(crate) fn build_report(
    outcome: &LoadOutcome,
    metas: &HashMap<(String, String), CellMeta>,
) -> String {
    let mut buf = String::new();
    emit_header(&mut buf, outcome);
    emit_per_scenario_tables(&mut buf, &outcome.runs);
    emit_docker_runtimes_table(&mut buf, &outcome.runs, metas);
    emit_allocator_diagrams(&mut buf);
    emit_recommendations(&mut buf, &outcome.runs);
    if !outcome.skipped.is_empty() {
        emit_skipped(&mut buf, &outcome.skipped);
    }
    buf
}
```
**Phase 8 modification:** thread `top_n: &[CellRecommendation]` parameter into `build_report` + `write`; insert `emit_top_n_cells(&mut buf, top_n);` between line 65 (`emit_recommendations`) and line 66 (`if !outcome.skipped.is_empty()`).

**Write entry-point pattern** (lines 43-52):
```rust
pub fn write(
    outcome: &LoadOutcome,
    metas: &HashMap<(String, String), CellMeta>,
    out_dir: &Path,
) -> Result<()> {
    let buf = build_report(outcome, metas);
    let out_path = out_dir.join("REPORT.md");
    std::fs::write(&out_path, &buf).with_context(|| format!("writing {}", out_path.display()))?;
    Ok(())
}
```
**Phase 8 modification:** extend signature to accept `top_n: &[CellRecommendation]` (recommended option B per RESEARCH §Section 3); add per-cell file writes after the `std::fs::write(&out_path, &buf)`:
```rust
// Per-cell standalone .md fragments (CELL-03).
for cell in top_n.iter() {
    let frag = render_cell_md(cell)?;  // calls tt.render via per-cell context
    let frag_path = out_dir.join(format!(
        "recommend-{:02}-{}-{}.md",
        cell.rank, cell.alloc, cell.env
    ));
    std::fs::write(&frag_path, &frag)
        .with_context(|| format!("writing {}", frag_path.display()))?;
}
```

**Per-cell tinytemplate render pattern** (NEW — but mirrors `html::render` shape from html.rs:227-248):
```rust
fn render_cell_md(cell: &CellRecommendation) -> Result<String> {
    let mut tt = TinyTemplate::new();
    tt.add_template("recommend-cell-md", RECOMMEND_CELL_MD)
        .context("compiling recommend-cell.md.tmpl")?;
    let ctx = build_cell_template_context(cell);  // pub(crate) helper adds rank_padded
    let mut s = tt.render("recommend-cell-md", &ctx)
        .context("rendering recommend-cell.md")?;
    s.push('\n');  // POSIX trailing newline (CONTEXT.md "Specifics" ¶6)
    Ok(s)
}
```
*Note:* template registration could be hoisted to a single `TinyTemplate` instance shared across the 10 calls — RESEARCH §Pitfall 1 lesson is that compile-time tests cover registration, so reuse is safe.

**Numeric-formatting discipline** (CLAUDE.md Conventions, applied at line 77 + line 196 + line 250):
- `{:.0}` for medians in multi-run cells (line 250)
- `{:.1}` for throughputs in single-run cells (line 196)
- `{}` for ns latencies (line 196)
- For Phase 8 `## Top 10 cells` leading table (if in scope per Open Question 3): `{:.1}` for `composite_score`.

**BTreeMap/BTreeSet alphabetical-iteration discipline** (lines 273, 278, 372 — pervasive throughout):
```rust
let envs: BTreeSet<String> = runs.iter().map(|r| env_label(&r.env).to_string()).collect();
let mut by_docker_image: BTreeMap<String, f64> = BTreeMap::new();
```
**Phase 8 inherits naturally:** `top_n_cells` already returns `Vec<CellRecommendation>` in `(composite DESC, alloc ASC, env ASC)` order — no new BTree* needed in Phase 8 emitters.

**Test pattern** (lines 612-637 — `report_md_two_runs_byte_identical_after_timestamp_strip`):
```rust
#[test]
fn report_md_two_runs_byte_identical_after_timestamp_strip() {
    let runs = vec![ /* synthetic */ ];
    let outcome = LoadOutcome { runs, skipped: vec![] };
    let metas: HashMap<(String, String), CellMeta> = HashMap::new();
    let a = build_report(&outcome, &metas);
    let b = build_report(&outcome, &metas);
    let strip_first = |s: &str| -> String {
        let mut lines = s.splitn(2, '\n');
        lines.next();
        lines.next().unwrap_or("").to_string()
    };
    assert_eq!(strip_first(&a), strip_first(&b));
}
```
**Phase 8 mirrors this:** new `markdown::tests::emit_top_n_cells_section_splits_at_top_n_table` — synth a `Vec<CellRecommendation>` of length 10, call `emit_top_n_cells(&mut buf, &top_n)`, assert the buffer contains `## Top 10 cells`, the visible 5 cards before any `<details>`, the `<details><summary>` line, and the collapsed 5 cards inside.

---

### `crates/alloc-bench-aggregator/src/html.rs` (renderer, transform — MODIFY)

**Analog:** `html.rs::render` (lines 227-248) + `HtmlContext` struct (lines 60-96) + `tinytemplate_compiles_index_template` test (lines 339-343).

**Imports pattern** (lines 23-32):
```rust
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use alloc_bench_core::output::{HarnessInfo, Run};
use anyhow::{Context, Result};
use tinytemplate::TinyTemplate;

use crate::loader::{CellMeta, LoadOutcome};
use crate::markdown::env_label;
use crate::multi_run::{aggregate as mr_aggregate, MultiRunStats};
```
**Phase 8 adds:** `use crate::recommend::{CellRecommendation, TOP_N_TABLE};`

**Template constant pattern** (line 34):
```rust
const TEMPLATE: &str = include_str!("../templates/index.html.tmpl");
```
**Phase 8 adds two parallel consts:**
```rust
const RECOMMEND_CELL_HTML: &str = include_str!("../templates/recommend-cell.html.tmpl");
const RECOMMEND_CELL_MD: &str = include_str!("../templates/recommend-cell.md.tmpl");
```
*Visibility note:* `RECOMMEND_CELL_MD` may live in `markdown.rs` instead — RESEARCH §Section 3 recommends keeping format-specific templates colocated with the format renderer. Plan 8 picks final location; the WR-01 sentinel test in `html::tests` needs visibility to both, so either expose as `pub(crate)` from `markdown.rs` or define both in `html.rs`.

**Tinytemplate context-struct pattern** (lines 59-96):
```rust
#[derive(serde::Serialize)]
struct HtmlContext<'a> {
    results_json: &'a str,
    scenarios_json: &'a str,
    envs_json: &'a str,
    allocators_json: &'a str,
    suspect_pairs_json: &'a str,
    multi_run_grouped_json: &'a str,
    run_count: usize,
    cell_count: usize,
    timestamp_iso8601: &'a str,
    plotly_cdn_url: &'a str,
    plotly_sri_hash: &'a str,
}
```
**Phase 8 extends `HtmlContext`** with two new fields (per CONTEXT.md "index.html Integration"):
```rust
top_n_visible: &'a [CellTemplateContext],   // ranks 1..=TOP_N_TABLE
top_n_collapsed: &'a [CellTemplateContext], // ranks TOP_N_TABLE+1..=TOP_N_TOTAL
```
**Phase 8 adds a parallel context struct:**
```rust
#[derive(serde::Serialize)]
pub(crate) struct CellTemplateContext {
    pub rank: usize,
    pub rank_padded: String,  // pre-formatted "{:02}" — see Pitfall §Section 2
    pub alloc: String,
    pub env: String,           // already short-form per Phase 7 (recommend.rs:659)
    pub tldr: String,
    pub strengths: Vec<&'static str>,
    pub weaknesses: Vec<&'static str>,
    pub recommended_for: Vec<&'static str>,
    pub avoid_for: Vec<&'static str>,
    pub suspect_flag: bool,
}
```
**Builder pattern** (parallel to `build_context` lines 155-225):
```rust
pub(crate) fn build_cell_template_context(cell: &CellRecommendation) -> CellTemplateContext {
    CellTemplateContext {
        rank: cell.rank,
        rank_padded: format!("{:02}", cell.rank),
        alloc: cell.alloc.clone(),
        env: cell.env.clone(),
        tldr: cell.tldr.clone(),
        strengths: cell.strengths.clone(),
        weaknesses: cell.weaknesses.clone(),
        recommended_for: cell.recommended_for.clone(),
        avoid_for: cell.avoid_for.clone(),
        suspect_flag: cell.suspect_flag,
    }
}
```

**Render entry-point pattern** (lines 227-248):
```rust
fn render(runs: &[Run]) -> Result<String> {
    let mut tt = TinyTemplate::new();
    tt.add_template("index", TEMPLATE)
        .context("compiling index.html.tmpl")?;
    let ctx_owned = build_context(runs)?;
    let timestamp = chrono::Utc::now().to_rfc3339();
    let cell_count = count_unique_cells(runs);
    let ctx = HtmlContext {
        // ... fields ...
    };
    tt.render("index", &ctx).context("rendering index.html")
}
```
**Phase 8 extends:** add two more `tt.add_template` calls AFTER the existing one — order matters because `index.html.tmpl` will reference `recommend-cell-html` via `{{ call recommend-cell-html with cell }}` (RESEARCH §Section 2 confirmed `{{call}}` is supported by tinytemplate v1):
```rust
tt.add_template("recommend-cell-html", RECOMMEND_CELL_HTML)
    .context("compiling recommend-cell.html.tmpl")?;
tt.add_template("recommend-cell-md", RECOMMEND_CELL_MD)
    .context("compiling recommend-cell.md.tmpl")?;
```

**Write entry-point pattern** (lines 109-118):
```rust
pub fn write(
    outcome: &LoadOutcome,
    _metas: &HashMap<(String, String), CellMeta>,
    out_dir: &Path,
) -> Result<()> {
    let html = render(&outcome.runs)?;
    let out_path = out_dir.join("index.html");
    std::fs::write(&out_path, &html).with_context(|| format!("writing {}", out_path.display()))?;
    Ok(())
}
```
**Phase 8 modification:** extend signature to accept `top_n: &[CellRecommendation]`; thread into `render`; iterate top_n and write 10 standalone fragments (parallel pattern to markdown.rs::write modification — same `recommend-{rank:02}-{alloc}-{env}.html` filename pattern with trailing `\n`):
```rust
// Per-cell standalone .html fragments (CELL-03).
for cell in top_n.iter() {
    let frag_html = render_cell_html(cell)?;  // separate TinyTemplate render
    let frag_path = out_dir.join(format!(
        "recommend-{:02}-{}-{}.html",
        cell.rank, cell.alloc, cell.env
    ));
    std::fs::write(&frag_path, &frag_html)
        .with_context(|| format!("writing {}", frag_path.display()))?;
}
```

**Compile-time test pattern** (lines 339-343 — the WR-01 precedent):
```rust
#[test]
fn tinytemplate_compiles_index_template() {
    let mut tt = TinyTemplate::new();
    tt.add_template("index", TEMPLATE)
        .expect("template should compile — missed `\\{` escape?");
}
```
**Phase 8 extends with two new tests** (RESEARCH §Section 4 Companion Test):
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

**WR-01 sentinel test pattern (NEW — locked test name)** (RESEARCH §Section 4 — REQUIREMENTS-pinned `cell_templates_both_reference_all_fields`):
```rust
#[test]
fn cell_templates_both_reference_all_fields() {
    let mut axes = BTreeMap::new();
    axes.insert("channel_throughput", 99.0);
    // ... 8 axes for completeness ...
    let cell = CellRecommendation {
        rank: 7,
        alloc: "_SENTINEL_ALLOC_".into(),
        env: "_SENTINEL_ENV_".into(),
        composite_score: 42.42,
        axes,
        tldr: "_SENTINEL_TLDR_".into(),
        strengths: vec!["_SENTINEL_STRENGTH_A_", "_SENTINEL_STRENGTH_B_"],
        weaknesses: vec!["_SENTINEL_WEAKNESS_A_", "_SENTINEL_WEAKNESS_B_"],
        recommended_for: vec!["_SENTINEL_RECCLASS_"],
        avoid_for: vec!["_SENTINEL_AVOIDCLASS_"],
        suspect_flag: true,
    };
    let ctx = build_cell_template_context(&cell);
    let mut tt = TinyTemplate::new();
    tt.add_template("recommend-cell-md", RECOMMEND_CELL_MD).unwrap();
    tt.add_template("recommend-cell-html", RECOMMEND_CELL_HTML).unwrap();
    let md = tt.render("recommend-cell-md", &ctx).expect("md render");
    let html = tt.render("recommend-cell-html", &ctx).expect("html render");
    for sentinel in &[
        "_SENTINEL_ALLOC_", "_SENTINEL_ENV_", "_SENTINEL_TLDR_",
        "_SENTINEL_STRENGTH_A_", "_SENTINEL_STRENGTH_B_",
        "_SENTINEL_WEAKNESS_A_", "_SENTINEL_WEAKNESS_B_",
        "_SENTINEL_RECCLASS_", "_SENTINEL_AVOIDCLASS_",
        "(suspect)",  // suspect_flag = true → both surfaces emit literal
    ] {
        assert!(md.contains(sentinel), "MD missing {sentinel}: {md}");
        assert!(html.contains(sentinel), "HTML missing {sentinel}: {html}");
    }
    assert!(md.contains("7. ") && html.contains("7. "));
}
```

**Test fixture builder** (`html.rs::tests::make_test_run` at lines 269-333) — full Run with all fields populated; Phase 8 sentinel test does NOT need `Run` (renders against synthetic `CellRecommendation` directly), so no fixture builder reuse needed.

---

### `crates/alloc-bench-aggregator/templates/index.html.tmpl` (template, transform — MODIFY)

**Analog:** own existing `<section class="report-mirror">` block at lines 253-256.

**Existing section-block pattern** (lines 253-256):
```text
<section class="report-mirror">
  <h2>Per-scenario allocator comparison</h2>
  <div id="report-mirror-tables"></div>
</section>
```

**Phase 8 inserts new section IMMEDIATELY AFTER line 256** (per CONTEXT.md "Section placement" + UI-SPEC line 246):
```text
<section class="top-n-recommendations">
  <h2>Top 10 cells</h2>
  <p>Ranked 1-10 by composite score (equal-weighted across 8 axes). Cards 6-10 collapsed by default.</p>

  {{ for cell in top_n_visible }}{{ call recommend-cell-html with cell }}{{ endfor }}

  <details>
    <summary>Show ranks 6–10</summary>
    {{ for cell in top_n_collapsed }}{{ call recommend-cell-html with cell }}{{ endfor }}
  </details>
</section>
```

**`{{ call }}` invocation discipline** (RESEARCH §Section 2 — Phase 8 introduces this directive for the first time in the project):
- `{{ call template_name with path }}` looks up `cell` in current loop context, renders `recommend-cell-html` against it
- Both `{{for}}` and `{{call}}` are tinytemplate v1 features (verified RESEARCH §Section 2 via WebFetch 2026-05-27)
- Compile-time `tinytemplate_compiles_index_template` test (line 339 of html.rs) catches mis-matched `{{endfor}}` / `{{endif}}` — Phase 8 modification stays inside that test's coverage

**Tinytemplate `\{` escape continuity** (existing pattern from line 22 forward):
- The new section sits in the HTML body region (lines 244-256 are pure HTML, NO `\{` escapes needed)
- Risk: nothing — `<section>`, `<h2>`, `<p>`, `<details>`, `<summary>`, `<ul>`, `<li>` are pure HTML tags. No braces involved.

---

### `crates/alloc-bench-aggregator/src/main.rs` (orchestrator, request-response — MODIFY)

**Analog:** `main.rs::main` (lines 56-77 — the existing pipeline).

**Imports & module list** (lines 21-31):
```rust
mod axes;
mod diagrams;
mod html;
mod loader;
mod markdown;
mod multi_run;
mod recommend;
mod score;

use anyhow::{Context, Result};
use clap::Parser;
```
**Phase 8 modification:** add explicit imports for the wiring chain (Plan 8 to choose; recommended by RESEARCH §Section 3):
```rust
use score::{compute_axes, score_cells};
use recommend::top_n_cells;
```

**Pipeline pattern** (lines 56-77):
```rust
fn main() -> Result<()> {
    let cli = Cli::parse();
    let outcome = loader::discover(&cli.input)?;
    let metas = loader::load_cell_metas(&cli.meta)?;
    let _security_metas = loader::load_security_metas(&cli.security)?;
    let out_dir = std::path::Path::new(&cli.output);
    std::fs::create_dir_all(out_dir)
        .with_context(|| format!("creating output dir {}", cli.output))?;
    markdown::write(&outcome, &metas, out_dir)?;
    html::write(&outcome, &metas, out_dir)?;
    eprintln!(
        "aggregated {} runs, skipped {}",
        outcome.runs.len(),
        outcome.skipped.len()
    );
    Ok(())
}
```
**Phase 8 modification — option B (recommended by RESEARCH §Section 3):** insert score → top_n computation upstream of both writers + thread `top_n` as new parameter:
```rust
fn main() -> Result<()> {
    let cli = Cli::parse();
    let outcome = loader::discover(&cli.input)?;
    let metas = loader::load_cell_metas(&cli.meta)?;
    let security_metas = loader::load_security_metas(&cli.security)?;  // promote: no longer dormant
    let out_dir = std::path::Path::new(&cli.output);
    std::fs::create_dir_all(out_dir)
        .with_context(|| format!("creating output dir {}", cli.output))?;

    // Phase 7 deliverable wired through Phase 8 emitters.
    let cell_axes = score::compute_axes(&outcome.runs, &metas, &security_metas);
    let scores = score::score_cells(cell_axes);
    let top_n = recommend::top_n_cells(scores, &outcome.runs);

    markdown::write(&outcome, &metas, &top_n, out_dir)?;
    html::write(&outcome, &metas, &top_n, out_dir)?;
    eprintln!(
        "aggregated {} runs, skipped {}",
        outcome.runs.len(),
        outcome.skipped.len()
    );
    Ok(())
}
```
*Lesson from Phase 7 SUMMARY (RESEARCH §Section 5):* the `_security_metas` rename to `security_metas` (drop leading underscore) lands HERE because Phase 8's `compute_axes` call is the first non-test consumer. If Plan 8 splits this into a separate task, defer the rename to that task to avoid the unused-import warning Phase 7 hit.

**Tests pattern** (lines 86-137):
- Existing `cli_meta_flag_*`/`cli_security_flag_*` tests parse CLI args. Phase 8 doesn't add CLI flags, so no new clap tests needed.
- The Phase 8 wiring is exercised end-to-end by the new integration test `aggregate_writes_per_cell_fragments` (RESEARCH §Section 1 Wave 0 gap) — likely lives at `crates/alloc-bench-aggregator/tests/aggregate_writes_per_cell_fragments.rs` (test-binary pattern, not in `main.rs::tests`).

---

## Shared Patterns

### Tinytemplate Registration Discipline
**Source:** `html.rs::render` lines 228-230 + `html.rs::tests::tinytemplate_compiles_index_template` lines 339-343
**Apply to:** All template-bearing files (markdown.rs new render fn, html.rs render fn, both new tests)
```rust
let mut tt = TinyTemplate::new();
tt.add_template("name", CONST)
    .context("compiling name.tmpl")?;
// ... register every additional template BEFORE first render call ...
tt.render("name", &ctx).context("rendering name")
```
**Why pervasive:** every `{{ call template_name with cell }}` invocation requires the called template to be registered FIRST. Phase 8's `index.html.tmpl` references `recommend-cell-html` via `{{call}}` — register it before `tt.render("index", ...)`.

### BTreeMap/BTreeSet Alphabetical-Iteration Discipline
**Source:** Pervasive — `markdown.rs:104, 118, 273, 278, 372` + `html.rs:155, 167, 174, 195, 207, 251` + `recommend.rs:526, 568`
**Apply to:** Any aggregation, filter, or grouping in Phase 8 emitters
```rust
let envs: BTreeSet<String> = runs.iter().map(|r| env_label(&r.env).to_string()).collect();
let mut by_x: BTreeMap<K, V> = BTreeMap::new();
```
**Why pervasive:** byte-identical-output contract (CLAUDE.md Conventions). Phase 7's `top_n_cells` already returns sorted `Vec<CellRecommendation>` — Phase 8 inherits naturally. **DO NOT introduce `HashMap` / `HashSet` anywhere in new Phase 8 code.**

### Suspect-Flag Annotation Bytes
**Source:** UI-SPEC lines 110-114 + CONTEXT.md "Suspect annotation byte-identity" + Phase 8 distinction from `markdown.rs::suspect_note` (line 408-414)
**Apply to:** Both templates' `{{ if suspect_flag }} *(suspect)*{{ endif }}` clause + WR-01 sentinel test substring assertion
- Phase 8 cards emit the literal six bytes `(suspect)` between asterisks — leading space, no `<em>`, no `⚠`, no reason-qualifier
- Existing per-scenario table cells continue to use `markdown.rs::suspect_note`'s richer `*(\u{26A0} suspect: low samples)*` form — Phase 8 does NOT replace that

### Numeric-Formatting Discipline (CLAUDE.md Conventions)
**Source:** `markdown.rs:196` (`{:.1}` throughputs), `markdown.rs:250` (`{:.0}` multi-run medians), `markdown.rs:196` (`{}` ns latencies)
**Apply to:** any numeric rendering in Phase 8 (currently NONE in card body per CONTEXT.md decision; if leading `| Rank | Cell | Score |` table is in scope per Open Question 3, use `{:.1}` for composite_score)
**Inverted-direction note:** image_size_efficiency normalizes Lower-better → Higher-score. Phase 8 doesn't render axis values directly (axes are NOT in card body), so this doesn't apply at the template layer — but the strengths/weaknesses labels coming from `derive_strengths`/`derive_weaknesses` already encode direction correctly.

### Standalone File Write Pattern
**Source:** `markdown.rs::write` line 50 + `html.rs::write` line 116
```rust
std::fs::write(&out_path, &content).with_context(|| format!("writing {}", out_path.display()))?;
```
**Apply to:** all 20 new fragment writes (10 .md + 10 .html). Idempotent overwrite semantics — matches existing emitter behavior. NO `OpenOptions::new().create_new(true)` — stale-file accumulation is OUT OF SCOPE per CONTEXT.md.

### Trailing-Newline POSIX Convention
**Source:** RESEARCH §Section 2 + CONTEXT.md "Specifics" ¶6
```rust
let mut s = tt.render("recommend-cell-md", &ctx)?;
s.push('\n');  // POSIX file convention
```
**Apply to:** every standalone fragment write (10 .md + 10 .html); trailing `\n` makes them concatenable / cat-friendly. Existing REPORT.md and index.html already follow this implicitly (their templates end with newlines).

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| (none) | — | — | All six Phase 8 surfaces have direct analogs in the existing codebase. The WR-01 sentinel test pattern (cross-surface field-presence assertion) is novel-to-the-project, but its closest precedent is `tinytemplate_compiles_index_template` (html.rs:339) which gives the structural test shape. The `{{call}}` directive is also new-to-the-project but is documented in tinytemplate v1 (RESEARCH §Section 2 cites docs.rs WebFetch 2026-05-27) — no codebase analog because no existing template needs nested invocation. |

## Metadata

**Analog search scope:**
- `crates/alloc-bench-aggregator/src/` (full directory — 8 .rs modules)
- `crates/alloc-bench-aggregator/templates/` (1 existing template: `index.html.tmpl`)
- `crates/alloc-bench-aggregator/tests/` (existing integration tests for context)
- `crates/alloc-bench-core/src/` (output schema sanity-check, not modified)

**Files scanned:** 6 primary (html.rs, markdown.rs, recommend.rs, score.rs, main.rs, loader.rs) + 1 template (index.html.tmpl)

**Pattern extraction date:** 2026-05-27

**Key cross-cutting findings:**
1. **Tinytemplate is single-template-per-render today** — Phase 8 introduces nested `{{call}}` invocation for the first time in the project. Compile-time tests (extending `tinytemplate_compiles_index_template`) gate this addition.
2. **`CellRecommendation.env` is already short-form** — verified via Phase 7 plumbing through `score::env_short_name → CellScore.env → recommend::top_n_cells line 659`. Filename pattern uses `cell.env` directly (RESEARCH §Section 3 Open Question 2 resolved by inspection — flag with user only if longer form was intended).
3. **Section-emit precedent is rock-solid** — `markdown.rs::emit_recommendations` (lines 340-350) is byte-for-byte the shape Phase 8's `emit_top_n_cells` needs (heading + caption + body + trailing blank line).
4. **`HtmlContext` is the canonical tinytemplate context-struct shape** — `serde::Serialize` derive + lifetimed `&'a str` references for borrowed JSON strings + owned types for everything else. Phase 8's `CellTemplateContext` follows the same recipe (owned strings + simple scalars).
5. **WR-01 sentinel test is REQUIREMENTS-pinned** — name `cell_templates_both_reference_all_fields`, location `html::tests`. NO discretion on path; Plan 8 places the test in `crates/alloc-bench-aggregator/src/html.rs` `#[cfg(test)] mod tests`.

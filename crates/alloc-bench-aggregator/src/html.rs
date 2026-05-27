//! Render `report/index.html` via tinytemplate against
//! `templates/index.html.tmpl` (D-01, D-02 / RESEARCH §Pattern 1).
//!
//! Plan 01 shipped the skeleton: pinned Plotly 2.35.3 CDN tag with SRI
//! integrity, four `<div id="chart-*">` slots, the filter sidebar shell,
//! and the inlined `RESULTS` array.
//!
//! Plan 02 augments `HtmlContext` with FOUR new JSON-string fields
//! (`scenarios_json`, `envs_json`, `allocators_json`, `suspect_pairs_json`)
//! that the template consumes to seed the multi-select / A/B-picker option
//! lists at page load. The canonical D-07 suspect predicate
//! (`samples_count < 1_000 || warmup_duration_s < 5.0`) lives here too
//! (`is_suspect`); Plan 03's recommend.rs will reuse it.
//!
//! Pitfall 1 (RESEARCH): tinytemplate parses `{` as a value substitution.
//! The template file MUST escape every literal `{` in CSS/JS bodies as
//! `\{`. Substitution placeholders include `{ results_json | unescaped }`,
//! `{ scenarios_json | unescaped }`, `{ envs_json | unescaped }`,
//! `{ allocators_json | unescaped }`, `{ suspect_pairs_json | unescaped }`.
//! The `tinytemplate_compiles_index_template` test catches an unescaped
//! `{` regression at `cargo test` time, not at runtime.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use alloc_bench_core::output::{HarnessInfo, Run};
use anyhow::{Context, Result};
use tinytemplate::TinyTemplate;

use crate::loader::{CellMeta, LoadOutcome};
use crate::markdown::env_label;
use crate::multi_run::{aggregate as mr_aggregate, MultiRunStats};
use crate::recommend::{CellRecommendation, TOP_N_TABLE};

const TEMPLATE: &str = include_str!("../templates/index.html.tmpl");

/// Phase 8 / Plan 01 — per-cell Markdown card body template.
///
/// Lives here (not in `markdown.rs`) so the WR-01 sentinel test
/// (`cell_templates_both_reference_all_fields`) can register both
/// templates against a single `TinyTemplate` instance and assert
/// cross-surface field-presence parity. Plan 02's `markdown.rs`
/// imports this constant via `use crate::html::RECOMMEND_CELL_MD`.
///
/// Acknowledged trade-off: cross-module template-string visibility,
/// accepted for test colocation.
pub(crate) const RECOMMEND_CELL_MD: &str = include_str!("../templates/recommend-cell.md.tmpl");

/// Phase 8 / Plan 01 — per-cell HTML card body template.
///
/// Renders a single `<article class="recommend-card">` body driven by
/// `CellTemplateContext`. Field-by-field identical to
/// `RECOMMEND_CELL_MD`; the WR-01 sentinel test enforces this at
/// `cargo test` time.
pub(crate) const RECOMMEND_CELL_HTML: &str = include_str!("../templates/recommend-cell.html.tmpl");

/// Pinned Plotly CDN URL (D-02 / RESEARCH §Pitfall 4 — never `latest`).
pub(crate) const PLOTLY_CDN_URL: &str = "https://cdn.plot.ly/plotly-2.35.3.min.js";

/// SRI integrity hash for `plotly-2.35.3.min.js` (RESEARCH §Code Examples §5).
/// Computed via:
///   curl -s 'https://cdn.plot.ly/plotly-2.35.3.min.js' \
///     | openssl dgst -sha384 -binary | base64
/// Verified live at research time (2026-05-19).
pub(crate) const PLOTLY_SRI_HASH: &str =
    "sha384-MqL7Cy3itNqCI1Wlc926K0XhyRKJ/NMqTaytIIEB+QIdInOploxqRIHRKLlhPykM";

/// Canonical D-07 suspect predicate. A run is suspect when its harness
/// shipped fewer than 1 000 samples or warmed up for less than 5 s.
/// Plan 03's `recommend.rs` is expected to import this exact function so
/// the report and the dashboard agree on which runs are flagged.
//
// Lowered from 10 000 in quick task 260524-5nc; the original threshold
// falsely flagged healthy 60s runs of slow scenarios (cpu-bound ~10 800
// samples, multithread under tick-latency pressure).
pub(crate) fn is_suspect(h: &HarnessInfo) -> bool {
    h.samples_count < 1_000 || h.warmup_duration_s < 5.0
}

#[derive(serde::Serialize)]
struct HtmlContext<'a> {
    /// Pre-serialized JSON. Rendered via `{ results_json | unescaped }` so
    /// tinytemplate doesn't HTML-escape the `<`/`>`/`&`/`"` inside the JSON.
    results_json: &'a str,
    /// Sorted, de-duplicated list of scenario names (JSON-encoded). Seeds
    /// the `#sel-scenarios` multi-select at page load.
    scenarios_json: &'a str,
    /// Sorted, de-duplicated list of env labels (JSON-encoded). Seeds
    /// `#sel-envs` and the `#ab-*-env` single-selects.
    envs_json: &'a str,
    /// Sorted, de-duplicated list of allocator names (JSON-encoded). Seeds
    /// `#sel-allocs` and the `#ab-*-alloc` single-selects.
    allocators_json: &'a str,
    /// Sorted, de-duplicated list of `{allocator}·{env}` keys for suspect
    /// runs (JSON-encoded). The bootstrap script wraps this in a `Set`
    /// and uses it to prefix option labels with `⚠ `.
    suspect_pairs_json: &'a str,
    /// D-11 / D-12: derived `{alloc|env|scenario} → MultiRunStats` map
    /// (JSON-encoded). Empty `{}` when no `(alloc, env, scenario)` triple
    /// has ≥2 runs. The Plotly trace builder reads this to render
    /// asymmetric `error_y` whiskers and the `⚠ high variance` legend
    /// flag when CV > 10%.
    multi_run_grouped_json: &'a str,
    run_count: usize,
    cell_count: usize,
    /// Wall-clock generation timestamp, RFC-3339-formatted via
    /// `chrono::Utc::now().to_rfc3339()`. WR-04: this field is rendered
    /// via tinytemplate's DEFAULT formatter (NOT `unescaped`), so any
    /// stray `<`/`>`/`&`/`"` would be HTML-escaped. Today the producer
    /// emits only digits/hyphens/colons/dot/`T`/`+` so the escape is a
    /// no-op, but if a future contributor swaps in a custom timestamp
    /// (e.g. reading `BENCH_TIMESTAMP_OVERRIDE` from env for repro), the
    /// default-escape behaviour MUST stay — never switch to `| unescaped`.
    timestamp_iso8601: &'a str,
    plotly_cdn_url: &'a str,
    plotly_sri_hash: &'a str,
    /// Phase 8 / Plan 02 / CELL-04 — ranks 1..=`TOP_N_TABLE` (≤5) cards,
    /// always-visible above the fold. Slice over pre-built
    /// `CellTemplateContext` so the template can call `{rank_padded}` and
    /// the field set is the field-substitution-only surface guarded by
    /// the WR-01 sentinel test (`cell_templates_both_reference_all_fields`).
    /// Template callsite: `{{ for cell in top_n_visible }}{{ call recommend-cell-html with cell }}{{ endfor }}`.
    top_n_visible: &'a [CellTemplateContext],
    /// Phase 8 / Plan 02 / CELL-04 — ranks (`TOP_N_TABLE`+1)..=`TOP_N_TOTAL`
    /// (6..=10) cards, rendered inside `<details>`. Empty slice when
    /// `top_n.len() <= TOP_N_TABLE`. Template callsite mirrors
    /// `top_n_visible` but inside the `<details><summary>` GFM disclosure.
    top_n_collapsed: &'a [CellTemplateContext],
    /// Phase 8 / Plan 02 / CELL-04 — gates the entire
    /// `<section class="top-n-recommendations">` block in `index.html.tmpl`
    /// via `{{ if has_top_n }}...{{ endif }}` wrapper. Set to
    /// `!top_n.is_empty()` in `render`. Mirrors `markdown.rs::emit_top_n_cells`'s
    /// early-return so v1.0 byte-identity is preserved symmetrically across
    /// both surfaces when `top_n` is empty (no `<section>` bytes emitted).
    has_top_n: bool,
}

/// Phase 8 / Plan 01 — render context for the per-cell Markdown card
/// (`RECOMMEND_CELL_MD`) and HTML panel (`RECOMMEND_CELL_HTML`).
///
/// Mirrors the renderable subset of `CellRecommendation` (Phase 7) and
/// adds `rank_padded` so `<article id="recommend-{rank_padded}-...">`
/// renders `01..10` with leading zero (tinytemplate's default `Display`
/// formatter on `usize` outputs `1`, not `01`).
///
/// Intentionally OMITS `composite_score` and `axes` — per CONTEXT.md
/// decision, the cards do NOT render score numbers or per-axis values.
/// The leading `| Rank | Cell | Score |` summary table (Plan 02) handles
/// `composite_score`; per-axis values are the Phase 9 spider chart's
/// job. The CELL-02 sentinel test
/// (`cell_template_context_excludes_score_and_axes`) gates this
/// decision.
#[derive(serde::Serialize)]
pub(crate) struct CellTemplateContext {
    pub rank: usize,
    /// `format!("{:02}", cell.rank)` — pre-formatted because tinytemplate's
    /// default `Display` formatter on `usize` cannot pad-zero.
    pub rank_padded: String,
    pub alloc: String,
    /// Already short-form (e.g. "alpine", "debian-slim", "host") via
    /// Phase 7 plumbing: `score::env_short_name → CellScore.env →
    /// CellRecommendation.env` (verified at `recommend.rs:659`).
    pub env: String,
    pub tldr: String,
    pub strengths: Vec<&'static str>,
    pub weaknesses: Vec<&'static str>,
    pub recommended_for: Vec<&'static str>,
    pub avoid_for: Vec<&'static str>,
    pub suspect_flag: bool,
}

/// Phase 8 / Plan 01 — convert a `CellRecommendation` (Phase 7
/// deliverable) into the template-render context. Pure data transform:
/// clones owned fields, pre-formats `rank_padded`, copies primitives by
/// value. Plan 02's `markdown.rs` and `html.rs` render paths invoke this
/// once per top-N cell.
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

/// Emit `index.html` from the loaded runs.
///
/// WR-03 (Phase-5 review): the `metas` parameter is intentionally
/// unused by the HTML emit path — `image_size_mb` and friends are
/// rendered ONLY in `REPORT.md` via `markdown::write`. The parameter
/// stays on this function's signature for callsite-symmetry with
/// `markdown::write` (both take `&LoadOutcome` + `&metas`), so the
/// `main.rs` driver can pass the same args to both writers without
/// branching. If a future contributor surfaces sidecar data in the
/// dashboard, thread `metas` into `render` and `BuiltContext` and
/// add a smoke test asserting the HTML contains the meta value.
///
/// Phase 8 / Plan 02 / CELL-04: `top_n` parameter wired through `render`
/// and `HtmlContext` so the `<section class="top-n-recommendations">` block
/// renders inside `index.html`, gated by `{{if has_top_n}}` (mirrors the
/// markdown side's early-return; v1.0 byte-identity preserved on empty top_n).
///
/// Phase 8 / Plan 02 / CELL-03: also writes 10 standalone single-card
/// `recommend-{rank:02d}-{alloc}-{env}.html` fragments alongside
/// `index.html` for future drilldown-link surfaces (V12-04 deferred).
/// Filename pattern locked: rank zero-padded for natural filename sort;
/// `cell.env` already short-form per Phase 7 plumbing (recommend.rs:659);
/// trailing `\n` per POSIX file convention (CONTEXT.md "Specifics" ¶6).
pub fn write(
    outcome: &LoadOutcome,
    _metas: &HashMap<(String, String), CellMeta>,
    top_n: &[CellRecommendation],
    out_dir: &Path,
) -> Result<()> {
    let html = render(&outcome.runs, top_n)?;
    let out_path = out_dir.join("index.html");
    std::fs::write(&out_path, &html).with_context(|| format!("writing {}", out_path.display()))?;

    // Phase 8 / Plan 02 / CELL-03 — per-cell standalone HTML fragments.
    // Symmetrical with `markdown::write`'s per-cell `.md` fragment loop.
    for cell in top_n.iter() {
        let frag = render_cell_html(cell)?;
        let frag_path = out_dir.join(format!(
            "recommend-{:02}-{}-{}.html",
            cell.rank, cell.alloc, cell.env
        ));
        std::fs::write(&frag_path, &frag)
            .with_context(|| format!("writing {}", frag_path.display()))?;
    }
    Ok(())
}

/// Bundle of JSON-string fields derived from the runs vec. Holding the
/// owned `String`s in a single struct keeps the render() lifetimes tidy
/// (the `HtmlContext` borrows `&str` from this).
struct BuiltContext {
    results: String,
    scenarios: String,
    envs: String,
    allocators: String,
    suspect_pairs: String,
    /// D-11 / D-12 derived map: keyed by `"alloc|env|scenario"` strings.
    /// Empty `"{}"` when no `(alloc, env, scenario)` triple has ≥2 runs.
    multi_run_grouped: String,
}

/// JSON-encode for safe inlining inside an HTML `<script>` block. Escapes
/// `<`, `>`, and `&` so the string literal can never terminate the host
/// `<script>` tag (a `</script>` substring in the input becomes the JSON
/// escape sequence `</script>` in the output). RFC 8259 permits
/// these `\uXXXX` escapes and every JSON parser accepts them, so the
/// decoded JS value is byte-identical to the unescaped form.
///
/// CR-01 (Phase-04 review): `serde_json::to_string` does NOT escape `<`,
/// `>`, `/`, or the literal substring `</script>` when serializing string
/// fields. Without this wrapper a `Run` whose JSON contains `</script>`
/// (e.g. inside the free-form `scenario.config` or `metrics.allocator_stats`
/// `serde_json::Value` fields) would terminate the inline `<script>` block
/// in the rendered dashboard.
fn to_script_safe_json<T: serde::Serialize + ?Sized>(v: &T) -> Result<String> {
    let raw = serde_json::to_string(v).context("serializing to JSON")?;
    Ok(raw
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026"))
}

fn build_context(runs: &[Run]) -> Result<BuiltContext> {
    // RESEARCH §Pitfall 2: use `to_string` (compact) — pretty-printed JSON
    // bloats the inlined RESULTS by ~3× without reader benefit.
    // CR-01: escape `<`/`>`/`&` so attacker-controlled string fields cannot
    // terminate the host `<script>` block.
    let results = to_script_safe_json(runs).context("serializing runs to JSON")?;

    // PATTERNS §"Sorted-output / byte-identical-output pattern" — use
    // `BTreeSet`, never `HashSet`, so iteration is alphabetical.
    let scenarios: Vec<String> = runs
        .iter()
        .map(|r| r.scenario.name.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let envs: Vec<String> = runs
        .iter()
        .map(|r| env_label(&r.env).to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let allocators: Vec<String> = runs
        .iter()
        .map(|r| r.build.allocator.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    // UI-SPEC line 128: separator is `·` (U+00B7 MIDDLE DOT).
    let suspect_pairs: Vec<String> = runs
        .iter()
        .filter(|r| is_suspect(&r.harness))
        .map(|r| format!("{}\u{00B7}{}", r.build.allocator, env_label(&r.env)))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    // D-11 / D-12: derive a flat map of `"alloc|env|scenario" → MultiRunStats`
    // for the JS trace builder. Group runs by 3-tuple, aggregate the
    // throughput axis, and key by `"alloc|env|scenario"` so the JS lookup
    // is O(1). BTreeMap → alphabetical iteration → byte-stable JSON.
    let mut throughput_groups: BTreeMap<(String, String, String), Vec<f64>> = BTreeMap::new();
    for r in runs {
        let key = (
            r.build.allocator.clone(),
            env_label(&r.env).to_string(),
            r.scenario.name.clone(),
        );
        throughput_groups
            .entry(key)
            .or_default()
            .push(r.metrics.ticks_per_s);
    }
    let mut multi_run_grouped: BTreeMap<String, MultiRunStats> = BTreeMap::new();
    for ((alloc, env, scen), samples) in throughput_groups.iter() {
        if let Some(stats) = mr_aggregate(samples) {
            let key = format!("{alloc}|{env}|{scen}");
            multi_run_grouped.insert(key, stats);
        }
    }

    Ok(BuiltContext {
        results,
        scenarios: to_script_safe_json(&scenarios).context("serializing scenarios to JSON")?,
        envs: to_script_safe_json(&envs).context("serializing envs to JSON")?,
        allocators: to_script_safe_json(&allocators).context("serializing allocators to JSON")?,
        suspect_pairs: to_script_safe_json(&suspect_pairs)
            .context("serializing suspect_pairs to JSON")?,
        multi_run_grouped: to_script_safe_json(&multi_run_grouped)
            .context("serializing multi_run_grouped to JSON")?,
    })
}

fn render(runs: &[Run], top_n: &[CellRecommendation]) -> Result<String> {
    let mut tt = TinyTemplate::new();
    tt.add_template("index", TEMPLATE)
        .context("compiling index.html.tmpl")?;
    // Phase 8 / Plan 02 — register BOTH per-cell templates against this
    // engine. `index.html.tmpl` `{{call recommend-cell-html with cell}}`'s
    // the HTML one inside two `{{for}}` loops; the markdown one is
    // registered for symmetry + supports a hypothetical future
    // `{{call recommend-cell-md ...}}` from another template surface
    // without re-plumbing. Without these two registrations, render-time
    // `{{call}}` returns `tinytemplate::Error::CalledTemplateNotFound`
    // (T-08-04 mitigation).
    tt.add_template("recommend-cell-html", RECOMMEND_CELL_HTML)
        .context("compiling recommend-cell.html.tmpl")?;
    tt.add_template("recommend-cell-md", RECOMMEND_CELL_MD)
        .context("compiling recommend-cell.md.tmpl")?;

    let ctx_owned = build_context(runs)?;
    let timestamp = chrono::Utc::now().to_rfc3339();
    let cell_count = count_unique_cells(runs);

    // Phase 8 / Plan 02 / CELL-04 — visible/collapsed split. `split` caps
    // at the actual top_n length so a fixture with `top_n.len() < TOP_N_TABLE`
    // produces a non-panicking empty `collapsed` slice (Test 3 covers
    // the empty-top_n branch via `has_top_n: false`).
    let split = top_n.len().min(TOP_N_TABLE);
    let visible_ctxs: Vec<CellTemplateContext> = top_n[..split]
        .iter()
        .map(build_cell_template_context)
        .collect();
    let collapsed_ctxs: Vec<CellTemplateContext> = top_n[split..]
        .iter()
        .map(build_cell_template_context)
        .collect();

    let ctx = HtmlContext {
        results_json: &ctx_owned.results,
        scenarios_json: &ctx_owned.scenarios,
        envs_json: &ctx_owned.envs,
        allocators_json: &ctx_owned.allocators,
        suspect_pairs_json: &ctx_owned.suspect_pairs,
        multi_run_grouped_json: &ctx_owned.multi_run_grouped,
        run_count: runs.len(),
        cell_count,
        timestamp_iso8601: &timestamp,
        plotly_cdn_url: PLOTLY_CDN_URL,
        plotly_sri_hash: PLOTLY_SRI_HASH,
        top_n_visible: &visible_ctxs,
        top_n_collapsed: &collapsed_ctxs,
        has_top_n: !top_n.is_empty(),
    };
    tt.render("index", &ctx).context("rendering index.html")
}

/// Phase 8 / Plan 02 / CELL-03 — render a single CellRecommendation
/// through the per-cell HTML template. Used by `pub fn write`'s per-cell
/// fragment loop to write `recommend-{rank:02d}-{alloc}-{env}.html`
/// alongside `index.html`. Symmetrical with
/// `markdown.rs::render_cell_md`.
///
/// Trailing `\n` on every fragment per CONTEXT.md "Specifics" ¶6
/// (POSIX file convention; concatenable / cat-friendly).
fn render_cell_html(cell: &CellRecommendation) -> Result<String> {
    let mut tt = TinyTemplate::new();
    tt.add_template("recommend-cell-html", RECOMMEND_CELL_HTML)
        .context("compiling recommend-cell.html.tmpl")?;
    let ctx = build_cell_template_context(cell);
    let mut s = tt
        .render("recommend-cell-html", &ctx)
        .context("rendering recommend-cell.html")?;
    s.push('\n');
    Ok(s)
}

fn count_unique_cells(runs: &[Run]) -> usize {
    let mut set = BTreeSet::new();
    for r in runs {
        set.insert((r.build.allocator.as_str(), env_label(&r.env)));
    }
    set.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc_bench_core::output::{
        Build, Env, HarnessInfo, LatencyNs, Metrics, Rusage, ScenarioInfo,
    };
    use alloc_bench_core::SCHEMA_VERSION;

    /// Builder used across the html.rs unit tests. Mirrors loader.rs's
    /// helper but parameterizes alloc/env/scenario/samples so the new
    /// context tests can exercise the suspect predicate explicitly.
    fn make_test_run(
        allocator: &str,
        docker_image: Option<&str>,
        scenario: &str,
        samples_count: u64,
    ) -> Run {
        Run {
            schema_version: SCHEMA_VERSION,
            run_id: format!("test-{allocator}-{scenario}"),
            env: Env {
                os: "linux".into(),
                os_version: "test".into(),
                docker_image: docker_image.map(|s| s.to_string()),
                cpu_model: "test-cpu".into(),
                cpu_count: 1,
                memory_total_kb: 1,
            },
            build: Build {
                allocator: allocator.into(),
                rustc_version: "1.83.0".into(),
                target_triple: "x86_64-unknown-linux-gnu".into(),
                host_triple: "x86_64-unknown-linux-gnu".into(),
                profile: "release".into(),
                git_sha: "0".repeat(40),
                git_dirty: false,
                build_timestamp: "2026-05-19T00:00:00Z".into(),
                rustflags: "".into(),
            },
            scenario: ScenarioInfo {
                name: scenario.into(),
                config: serde_json::json!({}),
                unit: None,
            },
            harness: HarnessInfo {
                warmup_duration_s: 5.0,
                measurement_duration_s: 5.0,
                samples_count,
            },
            metrics: Metrics {
                ticks_per_s: 100.0,
                allocations_per_tick: 100,
                tick_latency_ns: LatencyNs {
                    p50: 1000,
                    p95: 2000,
                    p99: 3000,
                    p999: 5000,
                    max: 10000,
                },
                peak_rss_kb: 1000,
                rss_growth_samples: vec![],
                rusage: Rusage {
                    user_time_s: 0.0,
                    sys_time_s: 0.0,
                    minor_faults: 0,
                    major_faults: 0,
                    voluntary_csw: 0,
                    involuntary_csw: 0,
                    peak_rss_kb: 1000,
                },
                allocator_stats: serde_json::json!({}),
            },
            status: Some("success".into()),
            error: None,
        }
    }

    /// RESEARCH §Pitfall 1: a missed `\{` escape produces a TinyTemplate
    /// compile error. This test catches the regression at `cargo test`
    /// time instead of leaving it to a runtime mystery.
    ///
    /// Phase 8 / Plan 02: `index.html.tmpl` now `{{call recommend-cell-html
    /// with cell}}`'s the per-cell template inside `{{for}}` loops. The
    /// `{{call}}` directive is parsed at template-compile time (not
    /// render time), but the **referenced template** must be registered
    /// before render or `tt.render("index", ...)` returns
    /// `tinytemplate::Error::CalledTemplateNotFound`. Registering all
    /// three templates here keeps the compile gate green AND defends
    /// against a future contributor adding `{{call recommend-cell-md ...}}`
    /// from another template (T-08-04 mitigation).
    #[test]
    fn tinytemplate_compiles_index_template() {
        let mut tt = TinyTemplate::new();
        tt.add_template("index", TEMPLATE)
            .expect("template should compile — missed `\\{` escape?");
        tt.add_template("recommend-cell-html", RECOMMEND_CELL_HTML)
            .expect("recommend-cell.html.tmpl should compile");
        tt.add_template("recommend-cell-md", RECOMMEND_CELL_MD)
            .expect("recommend-cell.md.tmpl should compile");
    }

    /// Phase 8 / Plan 01 — compile gate for the per-cell templates. Mirrors
    /// `tinytemplate_compiles_index_template`: registers both `.tmpl` files
    /// against a fresh `TinyTemplate` instance and fails at `cargo test`
    /// time on a missed `\{` escape or unbalanced `{{for}}` / `{{if}}` block.
    #[test]
    fn tinytemplate_compiles_recommend_cell_templates() {
        let mut tt = TinyTemplate::new();
        tt.add_template("recommend-cell-md", RECOMMEND_CELL_MD)
            .expect("recommend-cell.md.tmpl should compile — missed `\\{` escape or `{{endfor}}`?");
        tt.add_template("recommend-cell-html", RECOMMEND_CELL_HTML)
            .expect("recommend-cell.html.tmpl should compile — missed `\\{` escape or `{{endfor}}`?");
    }

    /// Phase 8 / Plan 01 / CELL-02 — WR-01 sentinel drift defense.
    ///
    /// Renders BOTH per-cell templates against a single
    /// `CellTemplateContext` whose every renderable field is a unique
    /// sentinel substring. Asserts each sentinel appears in BOTH outputs.
    /// If a future PR adds a field to `CellTemplateContext` and the markdown
    /// template but forgets the HTML template (or vice-versa), the omitted
    /// side fails on the substring miss. The literal `(suspect)` substring
    /// + the `7. ` rank prefix are also asserted in both surfaces (suspect
    /// byte-identity + rank substitution gates).
    #[test]
    fn cell_templates_both_reference_all_fields() {
        use crate::recommend::CellRecommendation;
        use std::collections::BTreeMap;

        // Synthetic CellRecommendation with sentinel substrings in every
        // renderable scalar field. `composite_score` and `axes` are NOT
        // rendered by either template — they receive arbitrary placeholder
        // values just to satisfy the type. Test 3 below asserts those two
        // fields are absent from CellTemplateContext.
        let mut axes: BTreeMap<&'static str, f64> = BTreeMap::new();
        axes.insert("throughput", 1.0);
        let cell = CellRecommendation {
            rank: 7,
            alloc: "_SENTINEL_ALLOC_".to_string(),
            env: "_SENTINEL_ENV_".to_string(),
            composite_score: 42.42,
            axes,
            tldr: "_SENTINEL_TLDR_".to_string(),
            strengths: vec!["_SENTINEL_STRENGTH_A_", "_SENTINEL_STRENGTH_B_"],
            weaknesses: vec!["_SENTINEL_WEAKNESS_A_", "_SENTINEL_WEAKNESS_B_"],
            recommended_for: vec!["_SENTINEL_RECCLASS_"],
            avoid_for: vec!["_SENTINEL_AVOIDCLASS_"],
            suspect_flag: true,
            // Phase 9 / POLAR-05: sentinel-template field-parity test does
            // not yet check the Pareto column (Plan 09-03 lands the row
            // emitter); explicit `false` keeps the literal byte-identical.
            is_pareto: false,
        };
        let ctx = build_cell_template_context(&cell);

        let mut tt = TinyTemplate::new();
        tt.add_template("recommend-cell-md", RECOMMEND_CELL_MD)
            .expect("md compile");
        tt.add_template("recommend-cell-html", RECOMMEND_CELL_HTML)
            .expect("html compile");
        let md = tt.render("recommend-cell-md", &ctx).expect("md render");
        let html = tt.render("recommend-cell-html", &ctx).expect("html render");

        // Cross-surface field-presence parity: every sentinel renders in
        // BOTH outputs. The literal `(suspect)` parenthesized form is the
        // suspect byte-identity gate (no `<em>`, no badge, no glyph).
        let expected_in_both = [
            "_SENTINEL_ALLOC_",
            "_SENTINEL_ENV_",
            "_SENTINEL_TLDR_",
            "_SENTINEL_STRENGTH_A_",
            "_SENTINEL_STRENGTH_B_",
            "_SENTINEL_WEAKNESS_A_",
            "_SENTINEL_WEAKNESS_B_",
            "_SENTINEL_RECCLASS_",
            "_SENTINEL_AVOIDCLASS_",
            "(suspect)",
        ];
        for s in expected_in_both {
            assert!(md.contains(s), "MD missing {s}: {md}");
            assert!(html.contains(s), "HTML missing {s}: {html}");
        }

        // Rank substitution in both surfaces.
        assert!(md.contains("7. "), "MD missing rank prefix `7. `: {md}");
        assert!(html.contains("7. "), "HTML missing rank prefix `7. `: {html}");

        // HTML id uses rank_padded (`07`), not rank (`7`). Bonus check
        // that the `{:02}` formatter wired through to the article id.
        assert!(
            html.contains(r#"id="recommend-07-_SENTINEL_ALLOC_-_SENTINEL_ENV_""#),
            "HTML missing rank_padded id `recommend-07-...`: {html}"
        );
    }

    /// Phase 8 / Plan 01 — gate the CONTEXT.md decision that per-cell
    /// cards do NOT render `composite_score` or `axes`. Serializes a
    /// freshly-built `CellTemplateContext` to JSON and asserts the key
    /// set is exactly the 10 documented fields, alphabetically sorted.
    /// A future PR that adds `composite_score` or `axes` to the struct
    /// (instead of routing it through the leading summary table or the
    /// Phase 9 spider chart) fails this test.
    #[test]
    fn cell_template_context_excludes_score_and_axes() {
        use crate::recommend::CellRecommendation;
        use std::collections::BTreeMap;

        let mut axes: BTreeMap<&'static str, f64> = BTreeMap::new();
        axes.insert("throughput", 1.0);
        let cell = CellRecommendation {
            rank: 1,
            alloc: "x".to_string(),
            env: "y".to_string(),
            composite_score: 0.0,
            axes,
            tldr: "z".to_string(),
            strengths: vec![],
            weaknesses: vec![],
            recommended_for: vec![],
            avoid_for: vec![],
            suspect_flag: false,
            // Phase 9 / POLAR-05: this test asserts CellTemplateContext
            // EXCLUDES composite_score and axes — `is_pareto` likewise
            // is not yet template-bound (Plan 09-03 wires the column).
            is_pareto: false,
        };
        let ctx = build_cell_template_context(&cell);
        let value = serde_json::to_value(&ctx).expect("serialize CellTemplateContext");
        let mut keys: Vec<String> = value
            .as_object()
            .expect("CellTemplateContext serializes to a JSON object")
            .keys()
            .cloned()
            .collect();
        keys.sort();
        assert_eq!(
            keys,
            vec![
                "alloc",
                "avoid_for",
                "env",
                "rank",
                "rank_padded",
                "recommended_for",
                "strengths",
                "suspect_flag",
                "tldr",
                "weaknesses",
            ],
            "CellTemplateContext field-set drifted — composite_score / axes must NOT be added here"
        );
    }

    /// `{ results_json | unescaped }` must NOT HTML-escape the JSON. The
    /// rendered string contains the literal `:` and `[` characters from
    /// the JSON, not `&#x3A;` / `&#x5B;`.
    ///
    /// Phase 8 / Plan 02: `render` signature now takes `top_n: &[CellRecommendation]`
    /// — pass empty slice here so the `<section class="top-n-recommendations">`
    /// block is omitted via `{{if has_top_n}}` (mirrors the empty-top_n
    /// branch). Test 3 (`index_top_n_section_omitted_for_empty_top_n`)
    /// asserts the omission explicitly.
    #[test]
    fn render_inlines_results_json_unescaped() {
        let run = make_test_run("system", None, "test", 50_000);
        let html = render(&[run], &[]).expect("render");
        assert!(
            html.contains("\"schema_version\":1"),
            "rendered html missing schema_version key in inlined JSON"
        );
        assert!(
            html.contains("const RESULTS = ["),
            "rendered html missing inlined RESULTS array"
        );
        assert!(
            html.contains(PLOTLY_SRI_HASH),
            "rendered html missing pinned Plotly SRI hash"
        );
    }

    /// `build_context` must derive sorted, de-duplicated arrays for
    /// scenarios / envs / allocators. Three synthetic runs with a known
    /// label cross-product land in the JSON in alphabetical order
    /// (D-09 byte-identical output).
    #[test]
    fn context_extracts_scenarios_envs_allocators() {
        let runs = vec![
            make_test_run(
                "ptmalloc",
                Some("alloc-bench:ptmalloc-debian-slim"),
                "multithread",
                50_000,
            ),
            make_test_run(
                "jemalloc",
                Some("alloc-bench:jemalloc-alpine"),
                "multithread",
                50_000,
            ),
            make_test_run(
                "mimalloc",
                Some("alloc-bench:mimalloc-distroless-cc"),
                "cpu-bound",
                50_000,
            ),
        ];
        let ctx = build_context(&runs).expect("build_context");
        assert_eq!(ctx.scenarios, r#"["cpu-bound","multithread"]"#);
        assert_eq!(
            ctx.envs,
            r#"["alloc-bench:jemalloc-alpine","alloc-bench:mimalloc-distroless-cc","alloc-bench:ptmalloc-debian-slim"]"#
        );
        assert_eq!(ctx.allocators, r#"["jemalloc","mimalloc","ptmalloc"]"#);
    }

    /// CR-01 regression: a `</script>` substring (or any `<`/`>`/`&`) in
    /// a string field MUST be `\uXXXX`-escaped in the inlined JSON so it
    /// cannot terminate the host `<script>` block. Both `scenario.config`
    /// and `metrics.allocator_stats` are free-form `serde_json::Value`
    /// pass-throughs — they get tested via the `Run.build.allocator`
    /// field below, but the same escape wrapper covers them and every
    /// other `String` field on `Run` (single code path through
    /// `to_script_safe_json`).
    #[test]
    fn inlined_json_escapes_script_close_tag() {
        let mut run = make_test_run(
            "</script><script>alert('xss')</script>",
            None,
            "test",
            50_000,
        );
        // Also exercise `scenario.config` (free-form serde_json::Value)
        // and `metrics.allocator_stats` (same) so we prove the wrapper
        // covers every byte the runs vec carries — not just `Run.build`.
        run.scenario.config = serde_json::json!({
            "opaque": "</script><script>alert('xss')</script>"
        });
        run.metrics.allocator_stats = serde_json::json!({
            "raw_dump": "</script><script>alert('xss')</script>"
        });
        // Phase 8 / Plan 02: empty top_n preserves CR-01 escape testing
        // surface — adding ranking data would obscure the inlined JSON
        // assertions below.
        let html = render(&[run], &[]).expect("render");
        // Negative: the unescaped script-terminator MUST NOT appear in
        // the rendered HTML — neither the literal `</script><script>`
        // (which would terminate the inline RESULTS block AND inject
        // a fresh script tag) nor the bare `</script>` substring.
        assert!(
            !html.contains("</script><script>alert"),
            "script tag terminated inside RESULTS — CR-01 escape failed"
        );
        // Positive: every `<` is escaped to `<` (and `>` → `>`).
        assert!(
            html.contains("\\u003c/script\\u003e"),
            "expected JSON `\\u003c/script\\u003e` escape in inlined RESULTS"
        );
    }

    /// `suspect_pairs_json` lists every `{allocator}·{env}` combo whose
    /// run trips `is_suspect`. Two synthetic runs: one suspect (samples=500),
    /// one clean (samples=50_000) — only the suspect pair appears.
    #[test]
    fn context_marks_suspect_pairs() {
        let runs = vec![
            // Suspect: samples_count < 1_000.
            make_test_run(
                "jemalloc",
                Some("alloc-bench:jemalloc-alpine"),
                "multithread",
                500,
            ),
            // Clean.
            make_test_run(
                "ptmalloc",
                Some("alloc-bench:ptmalloc-debian-slim"),
                "multithread",
                50_000,
            ),
        ];
        let ctx = build_context(&runs).expect("build_context");
        assert_eq!(
            ctx.suspect_pairs,
            r#"["jemalloc·alloc-bench:jemalloc-alpine"]"#
        );
    }

    // ---------------------------------------------------------------------
    // Phase 8 / Plan 02 / CELL-04 — `<section class="top-n-recommendations">`
    // tests. Synthesizes minimal `CellRecommendation` instances directly
    // (the per-cell template only references rank, alloc, env, tldr,
    // strengths/weaknesses/recommended_for/avoid_for, suspect_flag — empty
    // axes BTreeMap is OK).
    // ---------------------------------------------------------------------

    /// Build a synthetic `CellRecommendation` with the given rank/alloc/env.
    /// `composite_score` matches the CONTEXT.md sentinel pattern (descending
    /// from 0.789 by 0.033 per rank).
    fn make_cell(rank: usize, alloc: &str, env: &str) -> CellRecommendation {
        use std::collections::BTreeMap;
        CellRecommendation {
            rank,
            alloc: alloc.to_string(),
            env: env.to_string(),
            composite_score: 0.789 - 0.033 * (rank as f64 - 1.0),
            axes: BTreeMap::new(),
            tldr: format!("synthetic-tldr-rank-{rank}"),
            strengths: vec!["throughput", "latency"],
            weaknesses: vec!["memory", "image-size"],
            recommended_for: vec!["cpu-bound"],
            avoid_for: vec!["memory-bound"],
            suspect_flag: false,
            // Phase 9 / POLAR-05: HTML render-tests don't yet cover the
            // Pareto column — Plan 09-03 lands the row emitter glyph.
            is_pareto: false,
        }
    }

    /// Build a top_n vec of length `n` with deterministic alloc/env labels.
    fn make_top_n(n: usize) -> Vec<CellRecommendation> {
        let allocs = ["mimalloc", "jemalloc", "mallocng", "ptmalloc"];
        let envs = ["alpine", "debian-slim", "wolfi"];
        (1..=n)
            .map(|rank| {
                let alloc = allocs[(rank - 1) % allocs.len()];
                let env = envs[(rank - 1) % envs.len()];
                make_cell(rank, alloc, env)
            })
            .collect()
    }

    /// Test 1: rendered HTML contains `<section class="top-n-recommendations">`
    /// AFTER `<section class="report-mirror">`; `<h2>Top 10 cells</h2>` once;
    /// caption inside `<p>` tags.
    #[test]
    fn index_contains_top_n_recommendations_section() {
        let run = make_test_run(
            "jemalloc",
            Some("alloc-bench:jemalloc-alpine"),
            "multithread",
            50_000,
        );
        let top_n = make_top_n(10);
        let html = render(&[run], &top_n).expect("render");

        // Section exists.
        let top_n_idx = html
            .find(r#"<section class="top-n-recommendations">"#)
            .expect(&format!("missing top-n section in:\n{html}"));

        // Sits AFTER the existing report-mirror section (string-position).
        let report_mirror_idx = html
            .find(r#"<section class="report-mirror">"#)
            .expect(&format!("missing report-mirror section in:\n{html}"));
        assert!(
            report_mirror_idx < top_n_idx,
            "top-n section ({top_n_idx}) must come AFTER report-mirror ({report_mirror_idx})"
        );

        // Heading appears exactly once.
        let h2_count = html.matches("<h2>Top 10 cells</h2>").count();
        assert_eq!(
            h2_count, 1,
            "expected exactly one `<h2>Top 10 cells</h2>` in:\n{html}"
        );

        // Caption verbatim, inside `<p>` tags.
        assert!(
            html.contains("<p>Ranked 1-10 by composite score (equal-weighted across 8 axes). Cards 6-10 collapsed by default.</p>"),
            "missing caption in `<p>` tags in:\n{html}"
        );
    }

    /// Test 2: 10-card top_n produces exactly 5 `<article class="recommend-card"`
    /// openings BEFORE `<details>` (visible) and exactly 5 BETWEEN
    /// `<details>` and `</details>` (collapsed). One `<details>` block,
    /// one `</details>`, en-dash U+2013 in the summary.
    #[test]
    fn index_top_n_section_renders_visible_and_collapsed_cards() {
        let run = make_test_run(
            "jemalloc",
            Some("alloc-bench:jemalloc-alpine"),
            "multithread",
            50_000,
        );
        let top_n = make_top_n(10);
        let html = render(&[run], &top_n).expect("render");

        // Single details block.
        let details_open_count = html.matches("<details>").count();
        let details_close_count = html.matches("</details>").count();
        assert_eq!(
            details_open_count, 1,
            "expected exactly one `<details>` opening in:\n{html}"
        );
        assert_eq!(
            details_close_count, 1,
            "expected exactly one `</details>` closing in:\n{html}"
        );

        // Summary line with en-dash U+2013.
        let summary_count = html
            .matches("<summary>Show ranks 6\u{2013}10</summary>")
            .count();
        assert_eq!(
            summary_count, 1,
            "expected exactly one `<summary>Show ranks 6–10</summary>` in:\n{html}"
        );

        // Card-count split: 5 visible BEFORE <details>, 5 collapsed
        // BETWEEN <details> and </details>. Anchor only on the
        // `top-n-recommendations` section (the other surfaces in
        // index.html may also use `<article>`).
        let section_start = html
            .find(r#"<section class="top-n-recommendations">"#)
            .expect(&format!("missing top-n section in:\n{html}"));
        let section_end = html[section_start..]
            .find("</section>")
            .map(|i| section_start + i)
            .expect(&format!("missing closing </section> in:\n{html}"));
        let section = &html[section_start..section_end];

        let details_idx = section
            .find("<details>")
            .expect(&format!("missing <details> in section:\n{section}"));
        let close_idx = section
            .find("</details>")
            .expect(&format!("missing </details> in section:\n{section}"));

        let visible = &section[..details_idx];
        let collapsed = &section[details_idx..close_idx];

        let visible_card_count = visible.matches(r#"<article class="recommend-card""#).count();
        let collapsed_card_count = collapsed
            .matches(r#"<article class="recommend-card""#)
            .count();

        assert_eq!(
            visible_card_count, 5,
            "expected 5 visible cards before <details> in:\n{visible}"
        );
        assert_eq!(
            collapsed_card_count, 5,
            "expected 5 collapsed cards inside <details>...</details> in:\n{collapsed}"
        );
    }

    /// Test 3: empty top_n -> the entire `<section class="top-n-recommendations">`
    /// block is omitted via `{{if has_top_n}}` wrapper. v1.0 byte-identity
    /// preserved (mirrors markdown's early-return symmetrically).
    #[test]
    fn index_top_n_section_omitted_for_empty_top_n() {
        let run = make_test_run(
            "jemalloc",
            Some("alloc-bench:jemalloc-alpine"),
            "multithread",
            50_000,
        );
        let html = render(&[run], &[]).expect("render");

        assert!(
            !html.contains(r#"<section class="top-n-recommendations">"#),
            "expected NO top-n section for empty top_n, got:\n{html}"
        );
        assert!(
            !html.contains("<h2>Top 10 cells</h2>"),
            "expected NO `<h2>Top 10 cells</h2>` for empty top_n, got:\n{html}"
        );
        // Sanity: existing report-mirror section IS still present (we only
        // removed the new section, not anything pre-existing).
        assert!(
            html.contains(r#"<section class="report-mirror">"#),
            "expected pre-existing `<section class=\"report-mirror\">` to remain, got:\n{html}"
        );
    }

    /// Test 4: `pub fn write` writes 10 standalone `recommend-{rank:02d}-{alloc}-{env}.html`
    /// fragments alongside index.html. Spot-check one fragment contains
    /// `<article class="recommend-card"` and ends with `\n`.
    ///
    /// Uses `tempfile::tempdir()` (already in [dev-dependencies] per Cargo.toml).
    #[test]
    fn write_emits_per_cell_html_fragments() {
        let runs = vec![make_test_run(
            "jemalloc",
            Some("alloc-bench:jemalloc-alpine"),
            "multithread",
            50_000,
        )];
        let outcome = LoadOutcome {
            runs,
            skipped: vec![],
        };
        let metas: HashMap<(String, String), CellMeta> = HashMap::new();
        let top_n = make_top_n(10);

        let tmp = tempfile::tempdir().expect("tempdir");
        write(&outcome, &metas, &top_n, tmp.path()).expect("write");

        // Collect filenames matching `recommend-*.html`.
        let mut fragment_names: Vec<String> = std::fs::read_dir(tmp.path())
            .expect("read_dir")
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("recommend-") && name.ends_with(".html") {
                    Some(name)
                } else {
                    None
                }
            })
            .collect();
        fragment_names.sort();
        assert_eq!(
            fragment_names.len(),
            10,
            "expected exactly 10 recommend-*.html fragments, got: {fragment_names:?}"
        );

        // Filename pattern: rank zero-padded 01..=10.
        for (i, name) in fragment_names.iter().enumerate() {
            let expected_prefix = format!("recommend-{:02}-", i + 1);
            assert!(
                name.starts_with(&expected_prefix),
                "fragment {i} name `{name}` doesn't start with `{expected_prefix}`"
            );
            assert!(
                name.ends_with(".html"),
                "fragment {i} name `{name}` doesn't end with `.html`"
            );
        }

        // Spot-check rank-01 fragment contents.
        let rank_01_path = tmp
            .path()
            .read_dir()
            .expect("read_dir")
            .filter_map(|e| e.ok())
            .find(|e| {
                let n = e.file_name().to_string_lossy().to_string();
                n.starts_with("recommend-01-") && n.ends_with(".html")
            })
            .expect("rank-01 fragment");
        let body = std::fs::read_to_string(rank_01_path.path()).expect("read");
        assert!(
            body.contains(r#"<article class="recommend-card""#),
            "rank-01 fragment missing `<article class=\"recommend-card\"`:\n{body}"
        );
        assert!(
            body.ends_with('\n'),
            "rank-01 fragment must end with `\\n` (POSIX trailing newline):\n{body:?}"
        );
    }

    /// Test 5: full register-and-render path with a non-empty top_n
    /// exercises the `{{call recommend-cell-html with cell}}` directive.
    /// If a future contributor breaks the `{{call}}` (typo, missing
    /// registration, dropped `with cell` syntax), this test fails at
    /// `cargo test` time with a clear `tinytemplate::Error`.
    #[test]
    fn tinytemplate_renders_index_with_top_n_section() {
        let run = make_test_run(
            "jemalloc",
            Some("alloc-bench:jemalloc-alpine"),
            "multithread",
            50_000,
        );
        let top_n = make_top_n(10);
        let html = render(&[run], &top_n).expect("render with top_n must succeed");

        // The `{{call recommend-cell-html with cell}}` invocations
        // produced 10 articles with proper rank-padded ids.
        for rank in 1..=10 {
            let id_prefix = format!(r#"id="recommend-{:02}-"#, rank);
            assert!(
                html.contains(&id_prefix),
                "missing `{id_prefix}...` rank-padded article id in:\n{html}"
            );
        }
    }
}

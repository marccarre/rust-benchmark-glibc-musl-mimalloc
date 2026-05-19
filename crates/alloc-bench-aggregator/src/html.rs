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
//! (`samples_count < 10_000 || warmup_duration_s < 5.0`) lives here too
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

const TEMPLATE: &str = include_str!("../templates/index.html.tmpl");

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
/// shipped fewer than 10 000 samples or warmed up for less than 5 s.
/// Plan 03's `recommend.rs` is expected to import this exact function so
/// the report and the dashboard agree on which runs are flagged.
pub(crate) fn is_suspect(h: &HarnessInfo) -> bool {
    h.samples_count < 10_000 || h.warmup_duration_s < 5.0
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

fn render(runs: &[Run]) -> Result<String> {
    let mut tt = TinyTemplate::new();
    tt.add_template("index", TEMPLATE)
        .context("compiling index.html.tmpl")?;
    let ctx_owned = build_context(runs)?;
    let timestamp = chrono::Utc::now().to_rfc3339();
    let cell_count = count_unique_cells(runs);
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
    };
    tt.render("index", &ctx).context("rendering index.html")
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
    #[test]
    fn tinytemplate_compiles_index_template() {
        let mut tt = TinyTemplate::new();
        tt.add_template("index", TEMPLATE)
            .expect("template should compile — missed `\\{` escape?");
    }

    /// `{ results_json | unescaped }` must NOT HTML-escape the JSON. The
    /// rendered string contains the literal `:` and `[` characters from
    /// the JSON, not `&#x3A;` / `&#x5B;`.
    #[test]
    fn render_inlines_results_json_unescaped() {
        let run = make_test_run("system", None, "test", 50_000);
        let html = render(&[run]).expect("render");
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
        let html = render(&[run]).expect("render");
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
    /// run trips `is_suspect`. Two synthetic runs: one suspect (samples=5_000),
    /// one clean (samples=50_000) — only the suspect pair appears.
    #[test]
    fn context_marks_suspect_pairs() {
        let runs = vec![
            // Suspect: samples_count < 10_000.
            make_test_run(
                "jemalloc",
                Some("alloc-bench:jemalloc-alpine"),
                "multithread",
                5_000,
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
}

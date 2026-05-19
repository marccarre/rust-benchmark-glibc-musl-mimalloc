//! Render `report/index.html` via tinytemplate against
//! `templates/index.html.tmpl` (D-01, D-02 / RESEARCH §Pattern 1).
//!
//! Plan 01 ships the skeleton: pinned Plotly 2.35.3 CDN tag with SRI
//! integrity, four `<div id="chart-*">` slots, the filter sidebar shell,
//! and the inlined `RESULTS` array. Plan 02 fleshes out chart-trace
//! construction + filter handlers; Plan 03 wires the A/B picker.
//!
//! Pitfall 1 (RESEARCH): tinytemplate parses `{` as a value substitution.
//! The template file MUST escape every literal `{` in CSS/JS bodies as
//! `\{`. The single substitution placeholder is `{ results_json | unescaped }`.
//! The `tinytemplate_compiles_index_template` test catches an unescaped
//! `{` regression at `cargo test` time, not at runtime.

use std::path::Path;

use alloc_bench_core::output::Run;
use anyhow::{Context, Result};
use tinytemplate::TinyTemplate;

use crate::loader::LoadOutcome;
use crate::markdown::env_label;

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

#[derive(serde::Serialize)]
struct HtmlContext<'a> {
    /// Pre-serialized JSON. Rendered via `{ results_json | unescaped }` so
    /// tinytemplate doesn't HTML-escape the `<`/`>`/`&`/`"` inside the JSON.
    results_json: &'a str,
    run_count: usize,
    cell_count: usize,
    timestamp_iso8601: &'a str,
    plotly_cdn_url: &'a str,
    plotly_sri_hash: &'a str,
}

pub fn write(outcome: &LoadOutcome, out_dir: &Path) -> Result<()> {
    let html = render(&outcome.runs)?;
    let out_path = out_dir.join("index.html");
    std::fs::write(&out_path, &html).with_context(|| format!("writing {}", out_path.display()))?;
    Ok(())
}

fn render(runs: &[Run]) -> Result<String> {
    let mut tt = TinyTemplate::new();
    tt.add_template("index", TEMPLATE)
        .context("compiling index.html.tmpl")?;
    // RESEARCH §Pitfall 2: use `to_string` (compact) — pretty-printed JSON
    // bloats the inlined RESULTS by ~3× without reader benefit.
    let json = serde_json::to_string(runs).context("serializing runs to JSON")?;
    let timestamp = chrono::Utc::now().to_rfc3339();
    let cell_count = count_unique_cells(runs);
    let ctx = HtmlContext {
        results_json: &json,
        run_count: runs.len(),
        cell_count,
        timestamp_iso8601: &timestamp,
        plotly_cdn_url: PLOTLY_CDN_URL,
        plotly_sri_hash: PLOTLY_SRI_HASH,
    };
    tt.render("index", &ctx).context("rendering index.html")
}

fn count_unique_cells(runs: &[Run]) -> usize {
    use std::collections::BTreeSet;
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

    fn make_test_run() -> Run {
        Run {
            schema_version: SCHEMA_VERSION,
            run_id: "test".into(),
            env: Env {
                os: "linux".into(),
                os_version: "test".into(),
                docker_image: None,
                cpu_model: "test-cpu".into(),
                cpu_count: 1,
                memory_total_kb: 1,
            },
            build: Build {
                allocator: "system".into(),
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
                name: "test".into(),
                config: serde_json::json!({}),
                unit: None,
            },
            harness: HarnessInfo {
                warmup_duration_s: 5.0,
                measurement_duration_s: 5.0,
                samples_count: 50_000,
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
        let run = make_test_run();
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
}

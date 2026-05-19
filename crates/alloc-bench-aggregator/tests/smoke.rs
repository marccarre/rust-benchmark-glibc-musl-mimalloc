//! Phase 4 Plan 01 integration test (D-17). Drives the
//! `alloc-bench-aggregator` binary against committed fixtures and
//! exercises the four CLI exit paths:
//!
//!   1. `aggregator_emits_html_and_markdown_against_fixtures` — happy
//!      path: 5 runs across 3 fixtures (ptmalloc-debian-slim.json with
//!      2 runs, jemalloc-alpine.json with 2 runs including one suspect,
//!      mimalloc-distroless-cc-single.json with 1 single-Run-object) →
//!      both `index.html` and `REPORT.md` written; HTML carries the
//!      pinned Plotly 2.35.3 CDN tag + SRI integrity hash + inlined
//!      `const RESULTS = [` array; REPORT.md carries the H1 +
//!      schema-version comment + `## Runs` section.
//!   2. `aggregator_zero_glob_matches_exits_nonzero` — D-08: empty glob
//!      → bail with `error: no results found matching pattern …`.
//!   3. `aggregator_partial_failure_logs_skipped_file_and_continues` —
//!      D-08: one valid + one schema=999 file → exit 0, stderr lists
//!      the skipped file, REPORT.md contains the skipped section.
//!   4. `aggregator_all_files_fail_still_exits_zero_with_empty_report` —
//!      D-08: only-bad-files → exit 0, REPORT.md contains the
//!      `## Skipped inputs` section + the bad filename.
//!
//! Pattern follows `crates/alloc-bench-cli/tests/run_all_smoke.rs`
//! (assert_cmd::Command::cargo_bin + tempfile::tempdir). Adds
//! `predicates::str::contains` for stderr substring matching (Task 1
//! adds `predicates = "3"` to dev-deps).

use std::path::Path;

use alloc_bench_core::output::{
    Build, Env, HarnessInfo, LatencyNs, Metrics, Run, Rusage, ScenarioInfo,
};
use alloc_bench_core::SCHEMA_VERSION;
use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use tempfile::tempdir;

/// Synthetic Run builder used by the schema-version-mismatch + all-fail
/// tests below. Mirrors the loader.rs unit-test helper but lives in this
/// integration target (a separate compilation unit, so we can't share
/// the helper across files without a shared lib crate — duplicating is
/// fine for ~50 LOC).
fn make_synthetic_run(scenario_name: &str, schema_version: u32) -> Run {
    Run {
        schema_version,
        run_id: format!("synth-{scenario_name}"),
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
            name: scenario_name.into(),
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

/// Happy-path: drive the aggregator against the three committed
/// fixtures and assert both output files exist with the expected
/// substrings (pinned Plotly CDN + SRI hash + inlined RESULTS array
/// in HTML; H1 + schema_version comment + ## Runs in REPORT.md).
#[test]
fn aggregator_emits_html_and_markdown_against_fixtures() {
    let out_dir = tempdir().expect("tempdir");
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let pattern = format!("{}/*.json", fixtures.display());

    let mut cmd = Command::cargo_bin("alloc-bench-aggregator").expect("cargo bin");
    cmd.args(["--input"])
        .arg(&pattern)
        .args(["--output"])
        .arg(out_dir.path());
    cmd.assert().success();

    let html = std::fs::read_to_string(out_dir.path().join("index.html")).expect("read index.html");
    assert!(
        html.contains("https://cdn.plot.ly/plotly-2.35.3.min.js"),
        "HTML missing pinned Plotly 2.35.3 CDN URL"
    );
    // Use a prefix-match on the SRI to avoid line-wrap fragility.
    assert!(
        html.contains("sha384-MqL7Cy3i"),
        "HTML missing pinned Plotly SRI integrity hash"
    );
    assert!(
        html.contains("crossorigin=\"anonymous\""),
        "HTML missing crossorigin=\"anonymous\" on CDN <script>"
    );
    assert!(
        html.contains("const RESULTS = ["),
        "HTML missing inlined RESULTS array"
    );

    let md = std::fs::read_to_string(out_dir.path().join("REPORT.md")).expect("read REPORT.md");
    assert!(
        md.contains("# alloc-bench REPORT"),
        "REPORT.md missing top-level H1"
    );
    // Plan 03 replaced the Plan-01 `## Runs` bullet section with per-scenario
    // allocator comparison tables. Anchor on the Plan-03 emit set instead.
    assert!(
        md.contains("## Docker runtimes"),
        "REPORT.md missing ## Docker runtimes section"
    );
    assert!(
        md.contains("<!-- schema_version: 1"),
        "REPORT.md missing schema_version comment"
    );
}

/// D-08: empty glob → exit non-zero with the failed pattern in stderr.
#[test]
fn aggregator_zero_glob_matches_exits_nonzero() {
    let empty = tempdir().expect("tempdir");
    let pattern = format!("{}/*.json", empty.path().display());
    let mut cmd = Command::cargo_bin("alloc-bench-aggregator").expect("cargo bin");
    cmd.args(["--input"])
        .arg(&pattern)
        .args(["--output"])
        .arg(empty.path());
    cmd.assert().failure().stderr(contains("no results found"));
}

/// D-08: one valid + one schema=999 → exit 0, stderr lists the skipped
/// file with the schema_version mismatch reason. The valid file
/// produces a normal report; the bad file is documented in the
/// "Skipped inputs" section of REPORT.md.
#[test]
fn aggregator_partial_failure_logs_skipped_file_and_continues() {
    let dir = tempdir().expect("tempdir");
    // Good fixture — a valid Run array.
    std::fs::write(
        dir.path().join("good.json"),
        serde_json::to_string(&vec![make_synthetic_run("ok", SCHEMA_VERSION)]).unwrap(),
    )
    .unwrap();
    // Bad fixture — full Run with schema_version: 999. The serde
    // parse succeeds; the schema-version invariant in load_one bail!s,
    // which the discover loop catches and converts to skip-and-continue.
    std::fs::write(
        dir.path().join("bad.json"),
        serde_json::to_string(&vec![make_synthetic_run("future", 999)]).unwrap(),
    )
    .unwrap();

    let pattern = format!("{}/*.json", dir.path().display());
    let mut cmd = Command::cargo_bin("alloc-bench-aggregator").expect("cargo bin");
    cmd.args(["--input"])
        .arg(&pattern)
        .args(["--output"])
        .arg(dir.path());
    cmd.assert()
        .success()
        .stderr(contains("warn: skipped").and(contains("bad.json")))
        .stderr(contains("schema_version mismatch"));

    let md = std::fs::read_to_string(dir.path().join("REPORT.md")).expect("read REPORT.md");
    assert!(
        md.contains("## Skipped inputs"),
        "REPORT.md missing ## Skipped inputs section"
    );
    assert!(
        md.contains("bad.json"),
        "REPORT.md missing bad.json in skipped inputs"
    );
}

/// D-08: all-files-fail → still exit 0 with an empty-runs REPORT.md
/// that lists every bad file in `## Skipped inputs`. This is the
/// graceful-degradation contract: the aggregator never aborts; it
/// always produces SOME report, even if every input file was unreadable.
#[test]
fn aggregator_all_files_fail_still_exits_zero_with_empty_report() {
    let dir = tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("bad.json"),
        serde_json::to_string(&vec![make_synthetic_run("future", 999)]).unwrap(),
    )
    .unwrap();

    let pattern = format!("{}/*.json", dir.path().display());
    let mut cmd = Command::cargo_bin("alloc-bench-aggregator").expect("cargo bin");
    cmd.args(["--input"])
        .arg(&pattern)
        .args(["--output"])
        .arg(dir.path());
    cmd.assert().success();

    let md = std::fs::read_to_string(dir.path().join("REPORT.md")).expect("read REPORT.md");
    assert!(
        md.contains("## Skipped inputs"),
        "REPORT.md missing ## Skipped inputs section in all-fail case"
    );
    assert!(
        md.contains("bad.json"),
        "REPORT.md missing bad.json in skipped inputs"
    );
}

// ---------------------------------------------------------------------------
// Plan 02 Task 3: visual-contract smoke tests
// ---------------------------------------------------------------------------
//
// Each of the six tests below drives the aggregator against the committed
// fixtures, reads `index.html`, and asserts substring presence/absence to
// gate the UI-SPEC visual contract. They are additive — the four Plan-01
// tests above remain untouched so regressions surface separately.

/// Run the aggregator against the committed Plan-01 fixtures and return
/// the rendered index.html as a String. Factored out so the six new
/// tests share the setup; the four Plan-01 tests still inline the same
/// pattern to keep churn minimal.
fn run_aggregator_against_fixtures() -> (tempfile::TempDir, String) {
    let out_dir = tempdir().expect("tempdir");
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let pattern = format!("{}/*.json", fixtures.display());
    let mut cmd = Command::cargo_bin("alloc-bench-aggregator").expect("cargo bin");
    cmd.args(["--input"])
        .arg(&pattern)
        .args(["--output"])
        .arg(out_dir.path());
    cmd.assert().success();
    let html = std::fs::read_to_string(out_dir.path().join("index.html")).expect("read index.html");
    (out_dir, html)
}

/// Behavior 1: rendered HTML contains all four chart trace-builder
/// identifiers (`makeThroughputTraces`, `makeLatencyHeatmap`,
/// `makeRssLines`, `makeDiffBars`). Proves Task 2's chart wiring is
/// shipped end-to-end.
#[test]
fn aggregator_html_contains_four_chart_builders() {
    let (_dir, html) = run_aggregator_against_fixtures();
    assert!(
        html.contains("makeThroughputTraces"),
        "expected makeThroughputTraces in index.html"
    );
    assert!(
        html.contains("makeLatencyHeatmap"),
        "expected makeLatencyHeatmap in index.html"
    );
    assert!(
        html.contains("makeRssLines"),
        "expected makeRssLines in index.html"
    );
    assert!(
        html.contains("makeDiffBars"),
        "expected makeDiffBars in index.html"
    );
}

/// Behavior 2: rendered HTML uses `Plotly.react` for re-renders (>=4
/// invocations, one per chart card) and contains 0 instances of
/// `Plotly.newPlot` (RESEARCH §Anti-Patterns: NEVER use newPlot for
/// re-renders — it re-mounts the chart DOM, causing flicker; react
/// diffs in place).
#[test]
fn aggregator_html_uses_plotly_react_not_newplot() {
    let (_dir, html) = run_aggregator_against_fixtures();
    let react_count = html.matches("Plotly.react").count();
    assert!(
        react_count >= 4,
        "expected >=4 Plotly.react invocations, got {react_count}"
    );
    assert!(
        !html.contains("Plotly.newPlot"),
        "Plotly.newPlot must not appear — use Plotly.react for re-renders"
    );
}

/// Behavior 3: the suspect ⚠ glyph (U+26A0 WARNING SIGN) appears at
/// least once in the rendered HTML. Plan-01 fixtures include
/// jemalloc-alpine with samples_count=5_000 → suspect → server-side
/// bootstrap embeds `⚠ jemalloc·alloc-bench:jemalloc-alpine` in the
/// SUSPECT_PAIRS array per Task 1.
#[test]
fn aggregator_html_marks_suspect_allocator_with_warning_glyph() {
    let (_dir, html) = run_aggregator_against_fixtures();
    assert!(
        html.contains('\u{26A0}'),
        "expected ⚠ (U+26A0) in index.html — suspect-fixture run should surface"
    );
}

/// Behavior 4: rendered HTML carries the exact Viridis palette hex codes
/// from UI-SPEC line 92 (#440154 ptmalloc, #3B528B mallocng, #21908C
/// jemalloc, #5DC863 mimalloc). Colorblind-safe palette in use across
/// CSS variables AND the inline ALLOC_COLORS JS map.
#[test]
fn aggregator_html_uses_viridis_palette_per_ui_spec() {
    let (_dir, html) = run_aggregator_against_fixtures();
    assert!(html.contains("#440154"), "missing Viridis stop ptmalloc");
    assert!(html.contains("#3B528B"), "missing Viridis stop mallocng");
    assert!(html.contains("#21908C"), "missing Viridis stop jemalloc");
    assert!(html.contains("#5DC863"), "missing Viridis stop mimalloc");
}

/// Behavior 5: rendered HTML contains the empty-filter copy from
/// UI-SPEC line 155 verbatim. Lockdown the Copywriting Contract so a
/// later refactor can't silently drift the wording.
#[test]
fn aggregator_html_includes_empty_filter_copy() {
    let (_dir, html) = run_aggregator_against_fixtures();
    assert!(
        html.contains("No data in current filter"),
        "missing empty-filter heading copy"
    );
    assert!(
        html.contains("Select at least one scenario, environment, and allocator to render charts."),
        "missing empty-filter body copy"
    );
}

/// Behavior 6: rendered HTML's bootstrap function references the
/// canonical default-A/B index expressions (UI-SPEC line 256). We
/// assert the substrings rather than parse the JS so the test is
/// resilient to whitespace/format changes.
#[test]
fn aggregator_html_bootstraps_default_ab_pickers() {
    let (_dir, html) = run_aggregator_against_fixtures();
    assert!(
        html.contains("ALLOCATORS[0]"),
        "missing default A allocator (ALLOCATORS[0]) in bootstrap"
    );
    assert!(
        html.contains("ALLOCATORS[1]"),
        "missing default B allocator index (ALLOCATORS[1]) in bootstrap"
    );
    assert!(
        html.contains("ENVS[0]"),
        "missing default A/B env (ENVS[0]) in bootstrap"
    );
}

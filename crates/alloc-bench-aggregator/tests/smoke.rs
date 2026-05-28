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
/// jemalloc-alpine with samples_count=500 → suspect (low-samples arm)
/// → server-side bootstrap embeds
/// `⚠ jemalloc·alloc-bench:jemalloc-alpine` in the SUSPECT_PAIRS array
/// per Task 1.
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

/// WR-04 (Phase-04 review): rendered HTML carries a defense-in-depth
/// Content-Security-Policy meta tag in the <head>. Allows 'self' +
/// 'unsafe-inline' (required for the inlined RESULTS const) + the
/// pinned Plotly CDN host. Lockdown the contract so a future refactor
/// can't silently drop the tag.
#[test]
fn aggregator_html_includes_csp_meta_tag() {
    let (_dir, html) = run_aggregator_against_fixtures();
    assert!(
        html.contains("http-equiv=\"Content-Security-Policy\""),
        "missing CSP <meta http-equiv=\"Content-Security-Policy\"> tag"
    );
    assert!(
        html.contains("https://cdn.plot.ly"),
        "CSP must allow the pinned Plotly CDN host in script-src"
    );
    assert!(
        html.contains("'unsafe-inline'"),
        "CSP must allow 'unsafe-inline' so the inlined RESULTS <script> block runs"
    );
    assert!(
        html.contains("http-equiv=\"X-Content-Type-Options\""),
        "missing X-Content-Type-Options nosniff defense-in-depth tag"
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

// ---------------------------------------------------------------------------
// Plan 03 Task 6: REPORT.md richness + README.md system diagram smoke tests
// ---------------------------------------------------------------------------

/// Run the aggregator against the committed fixtures and return the
/// rendered REPORT.md as a String. Mirrors the HTML helper so the new
/// markdown-side tests share a consistent setup.
fn run_aggregator_and_read_markdown() -> (tempfile::TempDir, String) {
    let out_dir = tempdir().expect("tempdir");
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let pattern = format!("{}/*.json", fixtures.display());
    let mut cmd = Command::cargo_bin("alloc-bench-aggregator").expect("cargo bin");
    cmd.args(["--input"])
        .arg(&pattern)
        .args(["--output"])
        .arg(out_dir.path());
    cmd.assert().success();
    let md = std::fs::read_to_string(out_dir.path().join("REPORT.md")).expect("read REPORT.md");
    (out_dir, md)
}

/// AGG-06: REPORT.md contains four `flowchart TD` Mermaid blocks (one
/// per allocator architecture: jemalloc, mallocng, mimalloc, ptmalloc).
#[test]
fn aggregator_report_md_contains_four_mermaid_diagrams() {
    let (_dir, md) = run_aggregator_and_read_markdown();
    assert!(
        md.contains("flowchart TD"),
        "REPORT.md missing any flowchart TD"
    );
    let count = md.matches("flowchart TD").count();
    assert!(
        count >= 4,
        "expected ≥ 4 `flowchart TD` blocks in REPORT.md, got {count}"
    );
}

/// AGG-07: REPORT.md contains the `## Recommendations by workload`
/// section with data-derived rationale strings (`% throughput vs`)
/// AND every workload class label.
#[test]
fn aggregator_report_md_contains_recommendations_section() {
    let (_dir, md) = run_aggregator_and_read_markdown();
    for needle in [
        "## Recommendations by workload",
        "% throughput vs",
        "channel-heavy",
        "contention",
        "cpu-bound",
        "fragmentation-prone",
        "memory-bound",
        "web-ser-de",
    ] {
        assert!(
            md.contains(needle),
            "REPORT.md missing {needle:?} in Recommendations section"
        );
    }
}

/// AGG-05: REPORT.md `## Docker runtimes` table — column header,
/// em-dash cells, footnote anchor.
#[test]
fn aggregator_report_md_contains_docker_runtimes_table() {
    let (_dir, md) = run_aggregator_and_read_markdown();
    for needle in [
        "## Docker runtimes",
        "image_size_mb",
        "\u{2014}", // em-dash U+2014
        "Phase 5 CI via",
    ] {
        assert!(
            md.contains(needle),
            "REPORT.md missing {needle:?} in Docker runtimes table"
        );
    }
}

/// AGG-04: REPORT.md per-scenario tables emit the `**✓ ` bold-and-check
/// winner-prefix on at least one row. Plan-01 fixtures + Plan-03 logic
/// guarantee a winner row is rendered for every per-scenario table.
#[test]
fn aggregator_report_md_contains_winner_prefix() {
    let (_dir, md) = run_aggregator_and_read_markdown();
    assert!(
        md.contains("**\u{2713} "),
        "REPORT.md missing bold-and-check winner prefix `**\\u{{2713}} `"
    );
}

/// AGG-04 (Plan-03 update): REPORT.md surfaces the suspect signal when run
/// against the Phase-4 jemalloc-alpine fixture. Plan 03 introduced multi-run
/// cell collapsing — the existing fixture has TWO multithread+jemalloc-alpine
/// runs (one low-samples, one short-warmup) which now collapse into a SINGLE
/// multi-run cell with a `⚠ suspect` flag. The Phase-4 single-cell suspect
/// notes (`*(⚠ suspect: low samples)*` / `*(⚠ suspect: short warmup)*`) still
/// emit when a cell has only ONE run.
///
/// Plan-03 contract: the suspect signal must surface SOMEWHERE — either as
/// the legacy italic note (single-run cells) OR as the new multi-run flag
/// (≥2-run cells). We assert at least one shape is present.
#[test]
fn aggregator_report_md_contains_suspect_italic_notes() {
    let (_dir, md) = run_aggregator_and_read_markdown();
    let legacy_low = md.contains("*(\u{26A0} suspect: low samples)*");
    let legacy_short = md.contains("*(\u{26A0} suspect: short warmup)*");
    let multi_run_flag = md.contains("\u{26A0} suspect)");
    assert!(
        legacy_low || legacy_short || multi_run_flag,
        "REPORT.md missing any suspect signal (legacy italic notes or multi-run `⚠ suspect` flag):\n{md}"
    );
}

/// AGG-08: README.md contains the `## How memory allocation works on
/// Linux` heading AND a `flowchart TD` Mermaid block. The crate lives
/// at `crates/alloc-bench-aggregator/`, so the workspace README is two
/// directories up from `CARGO_MANIFEST_DIR`.
#[test]
fn readme_md_contains_system_diagram() {
    let readme_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../README.md");
    let readme = std::fs::read_to_string(&readme_path)
        .unwrap_or_else(|e| panic!("read {} failed: {e}", readme_path.display()));
    assert!(
        readme.contains("## How memory allocation works on Linux"),
        "README.md missing `## How memory allocation works on Linux` heading"
    );
    assert!(
        readme.contains("flowchart TD"),
        "README.md missing `flowchart TD` Mermaid block"
    );
}

// ---------------------------------------------------------------------------
// Plan 05-03 Task 5: multi-run + sidecar integration smoke tests
// ---------------------------------------------------------------------------
//
// Each test below drives the aggregator against the multi_run/seed-*.json
// fixture set (Plan 05-01) and asserts the Plan-03 wire-up is end-to-end
// observable in the output artifacts. The fixtures contain THREE seeded
// Run arrays (seed-1/2/3.json) covering two scenarios — multithread (CV
// ≈ 4.76%) and cpu-bound (CV ≈ 19.52%) — with a single sidecar
// meta/jemalloc-alpine.json carrying `image_size_mb: 26.55`.
//
// The helper `run_aggregator_with_multi_run_fixtures` mirrors
// `run_aggregator_against_fixtures` but points at the multi_run/ subdir
// for both `--input` and `--meta`.

/// Drive the aggregator against `tests/fixtures/multi_run/seed-*.json`
/// + `tests/fixtures/multi_run/meta/*.json`. Returns the tempdir handle
///   (kept alive for the test) plus the rendered HTML and REPORT.md.
fn run_aggregator_with_multi_run_fixtures() -> (tempfile::TempDir, String, String) {
    let out_dir = tempdir().expect("tempdir");
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/multi_run");
    let input_pattern = format!("{}/seed-*.json", fixtures.display());
    let meta_pattern = format!("{}/meta/*.json", fixtures.display());

    let mut cmd = Command::cargo_bin("alloc-bench-aggregator").expect("cargo bin");
    cmd.args(["--input"])
        .arg(&input_pattern)
        .args(["--meta"])
        .arg(&meta_pattern)
        .args(["--output"])
        .arg(out_dir.path());
    cmd.assert().success();

    let html = std::fs::read_to_string(out_dir.path().join("index.html")).expect("read index.html");
    let md = std::fs::read_to_string(out_dir.path().join("REPORT.md")).expect("read REPORT.md");
    (out_dir, html, md)
}

/// D-11: per-scenario throughput cells include the literal `, CV ` anchor
/// when ≥2 runs share an `(alloc, env, scenario)` triple. The multi_run
/// fixture set has 3 seeds for both multithread and cpu-bound, so both
/// scenario tables emit the multi-run shape.
#[test]
fn aggregator_multi_run_emits_cv_in_throughput_cell() {
    let (_dir, _html, md) = run_aggregator_with_multi_run_fixtures();
    assert!(
        md.contains(", CV "),
        "expected `, CV ` anchor in REPORT.md per-scenario throughput cells:\n{md}"
    );
}

/// D-11: per-scenario throughput cells include the `({min}..` range
/// substring. Multithread fixture: throughputs [100, 110, 105] →
/// min=100, max=110; the cell starts `105 (100..110, CV 5%)`.
#[test]
fn aggregator_multi_run_emits_min_max_range_in_cell() {
    let (_dir, _html, md) = run_aggregator_with_multi_run_fixtures();
    // Anchor on the `(100..` literal — the multithread scenario's min is
    // 100 across the 3 seeds. This pins the `({min:.0}..` shape and
    // implicitly proves min/max came through `mr_aggregate`.
    assert!(
        md.contains("(100.."),
        "expected `(100..` substring (min/max range) in REPORT.md:\n{md}"
    );
}

/// D-12: cells with CV > 10% surface the `⚠ high variance` flag.
/// cpu-bound fixture: throughputs [100, 130, 90] → CV ≈ 19.52% → flag.
#[test]
fn aggregator_high_variance_cell_marked_with_warning_glyph() {
    let (_dir, _html, md) = run_aggregator_with_multi_run_fixtures();
    assert!(
        md.contains("\u{26A0} high variance"),
        "expected `⚠ high variance` glyph in REPORT.md:\n{md}"
    );
}

/// D-11 / D-12 (Task 4): rendered index.html contains the `error_y`
/// Plotly field name (the asymmetric whisker contract). When multi-run
/// data is present, `makeThroughputTraces` always emits an `error_y`
/// block — even if every cell happens to be a single-run fallback the
/// block is still present (with all-zero arrays). The substring presence
/// is the structural pin.
#[test]
fn aggregator_html_contains_error_y_field() {
    let (_dir, html, _md) = run_aggregator_with_multi_run_fixtures();
    assert!(
        html.contains("error_y"),
        "expected `error_y` Plotly field in index.html"
    );
    assert!(
        html.contains("arrayminus"),
        "expected `arrayminus` (asymmetric whisker) in index.html"
    );
}

/// D-12 (Task 4): rendered index.html contains the legend label suffix
/// `high variance`. The cpu-bound fixture cell (CV ≈ 19.52%) trips the
/// `anyHighVariance` flag in `makeThroughputTraces`, so the legend gets
/// the suffix appended.
///
/// We anchor on the literal `high variance` substring so the assertion
/// is resilient to whitespace / glyph-formatting changes in the JS
/// `legendName` concatenation.
#[test]
fn aggregator_html_high_variance_appears_in_legend() {
    let (_dir, html, _md) = run_aggregator_with_multi_run_fixtures();
    assert!(
        html.contains("high variance"),
        "expected `high variance` substring in index.html (rendered into the JS legendName concatenation)"
    );
}

/// D-13 (Task 1 + Task 2): when `--meta` points at the multi-run sidecar
/// fixture, REPORT.md `## Docker runtimes` table populates `image_size_mb`
/// from the meta. The fixture sidecar carries `image_size_mb: 26.55`,
/// which Rust's `{:.1}` formatter renders as `26.6` (IEEE-754
/// half-up rounding — see deviation note in 05-03-SUMMARY.md).
///
/// Anchor on both `26.` (numeric value, formatting-direction-stable) AND
/// the env-row label so a future cell-shuffle can't false-pass.
#[test]
fn aggregator_meta_sidecar_populates_image_size_mb() {
    let (_dir, _html, md) = run_aggregator_with_multi_run_fixtures();
    // Numeric value present.
    assert!(
        md.contains("26."),
        "expected `26.` (image_size_mb) in REPORT.md Docker runtimes:\n{md}"
    );
    // Env-row label is the full docker_image tag synthesized from the
    // sidecar's (alloc, env) tuple.
    assert!(
        md.contains("alloc-bench:jemalloc-alpine"),
        "expected `alloc-bench:jemalloc-alpine` env-row label in REPORT.md:\n{md}"
    );
    // D-13 footnote wording switches when metas non-empty.
    assert!(
        md.contains("populated from CI sidecar"),
        "expected D-13 sidecar footnote when --meta is supplied:\n{md}"
    );
}

// ---------------------------------------------------------------------------
// 260523-8jf: layout-fix regression asserts (Task 1 + Task 2)
// ---------------------------------------------------------------------------
//
// Each assert pins ONE concrete edit so a future drift back to the cramped
// layout is caught structurally, not visually.

/// 260523-8jf Edit B (Task 1): rendered HTML carries the new clipping-fix
/// margin numbers on every chart layout. We anchor on the literal
/// `t: 80` substring (was `t: 40`) appearing at least 4 times — once per
/// chart layout — so a future drift back to `t: 40` is caught.
#[test]
fn aggregator_html_chart_layouts_have_t80_top_margin() {
    let (_dir, html) = run_aggregator_against_fixtures();
    let count = html.matches("t: 80").count();
    assert!(
        count >= 4,
        "expected >=4 `t: 80` occurrences (one per chart layout), got {count}"
    );
}

/// 260523-8jf Edit C (Task 1): latency heatmap left margin is wide enough
/// for the longest cell legend label (≈ 65 chars). The literal `l: 360`
/// substring is the structural pin.
#[test]
fn aggregator_html_latency_heatmap_has_wide_left_margin() {
    let (_dir, html) = run_aggregator_against_fixtures();
    assert!(
        html.contains("l: 360"),
        "expected `l: 360` (heatmap left margin) in index.html"
    );
}

/// 260523-8jf Edit F (Task 1): modebar docked at bottom so it cannot
/// overlap chart titles.
#[test]
fn aggregator_html_modebar_docked_bottom() {
    let (_dir, html) = run_aggregator_against_fixtures();
    assert!(
        html.contains("modeBarPosition: 'bottom'"),
        "expected `modeBarPosition: 'bottom'` in PLOTLY_CONFIG"
    );
}

/// 260523-8jf Task 2: RSS chart restricts itself to A/B picker selections.
/// The literal `readAbSelections` call inside `makeRssLines` is the
/// structural pin; if a future refactor removes the cap, this test fires.
#[test]
fn aggregator_html_rss_chart_caps_to_ab_picker_cells() {
    let (_dir, html) = run_aggregator_against_fixtures();
    // makeRssLines body must call readAbSelections() — proves the cap is
    // wired. Anchor on the function name + the same-cell-fallback comment.
    let make_rss = html
        .find("function makeRssLines")
        .expect("makeRssLines must be defined");
    let tail = &html[make_rss..];
    let next_function = tail
        .find("\nfunction ")
        .map(|i| make_rss + i)
        .unwrap_or(html.len());
    let body = &html[make_rss..next_function];
    assert!(
        body.contains("readAbSelections"),
        "makeRssLines must call readAbSelections() to cap series count"
    );
    // Hint subtitle wires into the chart title so users know about the cap.
    assert!(
        html.contains("showing the two cells selected in the A/B picker below"),
        "rssLayout title must surface the A/B-picker cap hint"
    );
}

/// 260523-8jf Task 1 (Edit A): chart-card min-height widened to 480px so
/// the 2x2 grid gives each chart enough vertical room for title + modebar
/// + plot area + legend.
#[test]
fn aggregator_html_chart_cards_have_min_height_480() {
    let (_dir, html) = run_aggregator_against_fixtures();
    assert!(
        html.contains("min-height: 480px"),
        "expected `min-height: 480px` on .chart-card"
    );
}

// ---------------------------------------------------------------------------
// Phase 9 / Plan 09-03 / Task 3 — spider chart visual-contract smoke tests.
//
// Both tests run end-to-end through `cargo bin alloc-bench-aggregator` against
// the committed fixtures, then assert substring presence on the rendered
// `index.html`. They are additive — every prior smoke test stays untouched.
// ---------------------------------------------------------------------------

/// POLAR-04: the spider chart `<div id="chart-spider"` block lands in the
/// rendered index.html when the fixture set produces at least one
/// CellRecommendation (the committed fixtures always do — 5 runs across
/// 3 fixtures with non-empty score axes guarantee a non-empty top_n).
#[test]
fn spider_div_present_when_data_exists() {
    let (_dir, html) = run_aggregator_against_fixtures();
    assert!(
        html.contains(r#"<div id="chart-spider""#),
        "expected `<div id=\"chart-spider\"` in rendered index.html"
    );
}

/// POLAR-04: the Plotly SRI hash literal is byte-pinned end-to-end. Extends
/// the existing prefix-only check (`sha384-MqL7Cy3i`) at line 128 to the
/// full hash to gate accidental partial-rotation drift. Re-verify upstream
/// via:
///   curl -s 'https://cdn.plot.ly/plotly-2.35.3.min.js' \
///     | openssl dgst -sha384 -binary | base64
/// (last verified 2026-05-19 per RESEARCH §Code Examples §5).
#[test]
fn plotly_sri_hash_unchanged_full_string() {
    let (_dir, html) = run_aggregator_against_fixtures();
    assert!(
        html.contains(
            "sha384-MqL7Cy3itNqCI1Wlc926K0XhyRKJ/NMqTaytIIEB+QIdInOploxqRIHRKLlhPykM"
        ),
        "expected full Plotly SRI hash literal in rendered index.html"
    );
}

/// Phase 9 / Plan 09-04 (UI-REVIEW BLOCKER fix) — end-to-end smoke
/// confirmation that the rendered `index.html` against the committed
/// fixtures carries EXACTLY THREE `<div class="spider-cell">` children
/// inside the outer `<div id="chart-spider" class="spider-grid">`
/// wrapper. Companion to the unit test
/// `html::tests::spider_section_emits_three_spider_cell_divs` — pins
/// the small-multiples grid contract through the full
/// `cargo bin alloc-bench-aggregator` execution path (load → score →
/// recommend → render → write).
///
/// The committed fixtures have 5 runs across 3 (alloc, env) cells, all
/// of which produce non-empty score axes (top_n is non-empty), so the
/// `{{ if has_spider }}` gate fires and the loop emits 3 cell divs.
#[test]
fn three_spider_cells_present_when_data_exists() {
    let (_dir, html) = run_aggregator_against_fixtures();
    let cell_count = html.matches(r#"class="spider-cell""#).count();
    assert_eq!(
        cell_count, 3,
        "expected exactly 3 `class=\"spider-cell\"` divs in rendered index.html, \
         got {cell_count} — small-multiples grid contract broken (UI-REVIEW BLOCKER)"
    );
    // Per-cell stable ids `spider-cell-1`, `-2`, `-3`.
    for n in 1..=3usize {
        let needle = format!(r#"id="spider-cell-{n}""#);
        assert!(
            html.contains(&needle),
            "missing `{needle}` per-cell id in rendered index.html"
        );
    }
    // Outer grid wrapper preserved.
    assert!(
        html.contains(r#"<div id="chart-spider" class="spider-grid""#),
        "missing OUTER `chart-spider` grid wrapper — \
         `spider_div_present_when_data_exists` would also break"
    );
    // Three independent `Plotly.react` calls keyed on `spider-cell-N`.
    for n in 1..=3usize {
        let needle = format!("Plotly.react('spider-cell-{n}'");
        assert!(
            html.contains(&needle),
            "missing `{needle}` independent Plotly.react call \
             — small-multiples bootstrap broken"
        );
    }
}

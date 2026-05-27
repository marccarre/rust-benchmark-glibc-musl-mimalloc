//! `alloc-bench-aggregator` — aggregate Phase-1/2/3 `results/*.json` into
//! `report/index.html` (Plotly dashboard) + `report/REPORT.md` (Markdown
//! comparison report).
//!
//! Plan 01 ships the end-to-end loop:
//!   1. clap parses `--input` (default `results/*.json`) + `--output`
//!      (default `report/`) per D-05.
//!   2. `loader::discover` globs + sorts + parses (Vec<Run> first, single
//!      Run fallback) + validates schema_version + skip-and-continues on
//!      per-file failures (D-06, D-08).
//!   3. `markdown::write` emits a minimal REPORT.md (Plan 02/03 expand it).
//!   4. `html::write` emits `index.html` via tinytemplate against
//!      `templates/index.html.tmpl` with a single `{ results_json |
//!      unescaped }` placeholder (D-01, D-02).
//!   5. Final stderr line `aggregated {N} runs, skipped {M}` summarizes.
//!
//! The aggregator is NOT a bench binary — CLAUDE.md's "all bench binaries
//! must print rustc version, target triple, allocator name at startup"
//! does NOT apply. No version banner is emitted.

mod axes;
mod diagrams;
mod html;
mod loader;
mod markdown;
mod multi_run;
mod polar;
mod recommend;
mod score;

use anyhow::{Context, Result};
use clap::Parser;

#[derive(Parser)]
#[command(
    name = "alloc-bench-aggregator",
    version,
    about = "Aggregate alloc-bench results into report/index.html + REPORT.md"
)]
struct Cli {
    /// Glob pattern for input JSON files (D-05).
    #[arg(long, default_value = "results/*.json")]
    input: String,
    /// Glob pattern for per-cell meta sidecars (image_size_mb / build_time_s).
    /// Empty = skip meta merge. CI populates via 'docker inspect' (D-13).
    #[arg(long, default_value = "")]
    meta: String,
    /// Glob pattern for per-env security posture sidecars (env-level score).
    /// Empty = security axis renders score=0 with em-dash tooltip (SEC-03).
    #[arg(long, default_value = "")]
    security: String,
    /// Output directory for index.html + REPORT.md (D-05).
    #[arg(long, default_value = "report/")]
    output: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let outcome = loader::discover(&cli.input)?;
    let metas = loader::load_cell_metas(&cli.meta)?;
    // Phase 7 / SEC-02 → Phase 8 wiring: security metas now thread into
    // `score::compute_axes` (security axis). Empty `--security ""` flag
    // produces an empty BTreeMap which `compute_axes` handles via the
    // `score=0 + em-dash tooltip` fallback (SEC-03 / Plan 07).
    let security_metas = loader::load_security_metas(&cli.security)?;
    let out_dir = std::path::Path::new(&cli.output);
    std::fs::create_dir_all(out_dir)
        .with_context(|| format!("creating output dir {}", cli.output))?;

    // Phase 8 / Plan 02 / CELL-04 — score → top_n pipeline.
    //
    // Pipeline: `compute_axes` derives 8 normalized axes per (alloc, env)
    // cell from the runs vec + sidecars. `score_cells` collapses to a
    // single composite_score (equal-weighted geometric mean across axes).
    // `top_n_cells` ranks 1..=TOP_N_TOTAL and decorates with strengths /
    // weaknesses / tldr / suspect_flag.
    //
    // Both writers receive the same `&top_n` so REPORT.md and index.html
    // surface byte-identical ranking + per-cell content (WR-01 cross-
    // surface drift defense, gated by `cell_templates_both_reference_all_fields`).
    //
    // Empty-runs / zero-cell case: `compute_axes` returns an empty Vec,
    // `score_cells` returns empty, `top_n_cells` returns empty, and both
    // writers' empty-top_n early-returns preserve v1.0 byte-identity for
    // synthetic-no-scores fixtures.
    let cell_axes = score::compute_axes(&outcome.runs, &metas, &security_metas);
    let cell_scores = score::score_cells(cell_axes);
    let top_n = recommend::top_n_cells(cell_scores, &outcome.runs);

    markdown::write(&outcome, &metas, &top_n, out_dir)?;
    html::write(&outcome, &metas, &top_n, out_dir)?;
    eprintln!(
        "aggregated {} runs, skipped {}",
        outcome.runs.len(),
        outcome.skipped.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D-13: the `--meta` flag defaults to an empty string so existing
    /// local `just aggregate` invocations keep producing the byte-identical
    /// Phase-4 REPORT.md output.
    #[test]
    fn cli_meta_flag_defaults_to_empty_string() {
        let cli = Cli::parse_from(["alloc-bench-aggregator"]);
        assert_eq!(cli.meta, "");
    }

    /// D-13: when the user passes `--meta meta/*.json`, the value lands
    /// in the CLI struct verbatim for `loader::load_cell_metas` to glob.
    #[test]
    fn cli_meta_flag_accepts_glob_pattern() {
        let cli = Cli::parse_from([
            "alloc-bench-aggregator",
            "--input",
            "results/*.json",
            "--meta",
            "meta/*.json",
            "--output",
            "report/",
        ]);
        assert_eq!(cli.meta, "meta/*.json");
        assert_eq!(cli.input, "results/*.json");
        assert_eq!(cli.output, "report/");
    }

    /// SEC-03: the `--security` flag defaults to an empty string so
    /// existing local `just aggregate` invocations keep producing the
    /// byte-identical Phase-4/5 REPORT.md output (mirrors the `--meta`
    /// Phase-5 D-13 precedent).
    #[test]
    fn cli_security_flag_defaults_to_empty_string() {
        let cli = Cli::parse_from(["alloc-bench-aggregator"]);
        assert_eq!(cli.security, "");
    }

    /// SEC-03: when the user passes `--security meta/security/*.json`,
    /// the value lands in the CLI struct verbatim for
    /// `loader::load_security_metas` to glob.
    #[test]
    fn cli_security_flag_accepts_glob_pattern() {
        let cli = Cli::parse_from([
            "alloc-bench-aggregator",
            "--input",
            "results/*.json",
            "--security",
            "meta/security/*.json",
            "--output",
            "report/",
        ]);
        assert_eq!(cli.security, "meta/security/*.json");
        assert_eq!(cli.input, "results/*.json");
        assert_eq!(cli.output, "report/");
    }
}

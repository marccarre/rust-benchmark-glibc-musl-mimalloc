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
    // SEC-02: security metas loaded but intentionally dormant in Phase 6.
    // Leading underscore signals Phase-7-pickup — that plan renames to
    // `security_metas` and threads it into `score::compute_axes`. Wiring
    // into markdown::write / html::write is explicitly forbidden in this
    // phase per RESEARCH §Risks #1 (byte-identical-output discipline).
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

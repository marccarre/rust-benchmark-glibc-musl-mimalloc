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

mod diagrams;
mod html;
mod loader;
mod markdown;
mod recommend;

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
    /// Output directory for index.html + REPORT.md (D-05).
    #[arg(long, default_value = "report/")]
    output: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let outcome = loader::discover(&cli.input)?;
    let out_dir = std::path::Path::new(&cli.output);
    std::fs::create_dir_all(out_dir)
        .with_context(|| format!("creating output dir {}", cli.output))?;
    markdown::write(&outcome, out_dir)?;
    html::write(&outcome, out_dir)?;
    eprintln!(
        "aggregated {} runs, skipped {}",
        outcome.runs.len(),
        outcome.skipped.len()
    );
    Ok(())
}

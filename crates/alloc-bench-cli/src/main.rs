mod allocator;
mod build_info;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "alloc-bench-cli", version, about = "Memory allocator benchmark suite")]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Print the version banner and exit (Walking Skeleton placeholder)
    Version,
    /// Multi-thread allocation stress
    Multithread {
        #[arg(long, default_value_t = num_cpus_default())]
        threads: usize,
        #[arg(long, default_value_t = 100_000)]
        objects: usize,
        #[arg(long, default_value = "uniform")]
        size_dist: String,
        #[arg(long, default_value_t = 16)]
        size_min: usize,
        #[arg(long, default_value_t = 1024)]
        size_max: usize,
        #[arg(long, default_value = "5s")]
        warmup: String,
        #[arg(long, default_value = "60s")]
        duration: String,
        #[arg(long, default_value_t = 0xDEADBEEF)]
        seed: u64,
        #[arg(long)]
        output: Option<String>,
    },
}

fn num_cpus_default() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

fn print_version_banner() {
    let sha = build_info::GIT_SHA;
    let sha8 = &sha[..sha.len().min(8)];
    let dirty = if build_info::GIT_DIRTY == "true" {
        "-dirty"
    } else {
        ""
    };
    eprintln!(
        "alloc-bench v{ver} (allocator={alloc}, rustc={rustc}, target={tgt}, host={host}, profile={prof}, git={sha}{dirty}, built={ts})",
        ver = build_info::CRATE_VERSION,
        alloc = allocator::name(),
        rustc = build_info::RUSTC_VERSION,
        tgt = build_info::TARGET_TRIPLE,
        host = build_info::HOST_TRIPLE,
        prof = build_info::PROFILE,
        sha = sha8,
        dirty = dirty,
        ts = build_info::BUILD_TIMESTAMP,
    );
}

fn main() -> Result<()> {
    print_version_banner();
    allocator::assert_mutual_exclusion();

    let cli = Cli::parse();
    match cli.cmd {
        None | Some(Cmd::Version) => Ok(()),
        Some(Cmd::Multithread { .. }) => {
            eprintln!("error: `multithread` subcommand is implemented in Plan 02");
            std::process::exit(2);
        }
    }
}

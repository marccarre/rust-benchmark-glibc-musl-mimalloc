mod allocator;
mod build_info;
mod run;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "alloc-bench-cli",
    version,
    about = "Memory allocator benchmark suite"
)]
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
    /// SPMC channel: 1 producer, N cloned receivers race for messages
    Spmc {
        #[arg(long, default_value_t = 1)]
        producers: usize,
        #[arg(long, default_value_t = 4)]
        consumers: usize,
        #[arg(long, default_value_t = 1024)]
        capacity: usize,
        #[arg(long, default_value_t = 1000)]
        objects_per_tick: u64,
        #[arg(long, default_value = "uniform")]
        payload_dist: String,
        #[arg(long, default_value = "5s")]
        warmup: String,
        #[arg(long, default_value = "60s")]
        duration: String,
        #[arg(long, default_value_t = 0xDEADBEEF)]
        seed: u64,
        #[arg(long)]
        output: Option<String>,
    },
    /// MPSC channel: N cloned senders, 1 receiver
    Mpsc {
        #[arg(long, default_value_t = 4)]
        producers: usize,
        #[arg(long, default_value_t = 1)]
        consumers: usize,
        #[arg(long, default_value_t = 1024)]
        capacity: usize,
        #[arg(long, default_value_t = 1000)]
        objects_per_tick: u64,
        #[arg(long, default_value = "uniform")]
        payload_dist: String,
        #[arg(long, default_value = "5s")]
        warmup: String,
        #[arg(long, default_value = "60s")]
        duration: String,
        #[arg(long, default_value_t = 0xDEADBEEF)]
        seed: u64,
        #[arg(long)]
        output: Option<String>,
    },
    /// MPMC channel: N senders × M receivers, both sides cloned
    Mpmc {
        #[arg(long, default_value_t = 4)]
        producers: usize,
        #[arg(long, default_value_t = 4)]
        consumers: usize,
        #[arg(long, default_value_t = 1024)]
        capacity: usize,
        #[arg(long, default_value_t = 1000)]
        objects_per_tick: u64,
        #[arg(long, default_value = "uniform")]
        payload_dist: String,
        #[arg(long, default_value = "5s")]
        warmup: String,
        #[arg(long, default_value = "60s")]
        duration: String,
        #[arg(long, default_value_t = 0xDEADBEEF)]
        seed: u64,
        #[arg(long)]
        output: Option<String>,
    },
    /// Lock-contention: high-thread-count tight alloc/free loop
    Contention {
        #[arg(long, default_value_t = 64)]
        threads: usize,
        #[arg(long, default_value_t = 64)]
        alloc_size: usize,
        #[arg(long, default_value_t = 10_000)]
        iters_per_tick: u64,
        #[arg(long, default_value = "5s")]
        warmup: String,
        #[arg(long, default_value = "60s")]
        duration: String,
        #[arg(long, default_value_t = 0xDEADBEEF)]
        seed: u64,
        #[arg(long)]
        output: Option<String>,
    },
    /// Memory-bound: linked-list (alloc-heavy) or strided-array (RSS+bandwidth)
    MemBound {
        #[arg(long, default_value = "linked-list")]
        mode: String,
        /// Working-set size in megabytes (--size MB).
        #[arg(long, default_value_t = 4)]
        size: usize,
        #[arg(long, default_value = "5s")]
        warmup: String,
        #[arg(long, default_value = "60s")]
        duration: String,
        #[arg(long, default_value_t = 0xDEADBEEF)]
        seed: u64,
        #[arg(long)]
        output: Option<String>,
    },
    /// Realloc-storm: Vec growth from capacity 0 to --target-size MB per tick
    ReallocStorm {
        /// Target Vec length in megabytes (--target-size MB).
        #[arg(long, default_value_t = 64)]
        target_size: usize,
        #[arg(long, default_value = "5s")]
        warmup: String,
        #[arg(long, default_value = "60s")]
        duration: String,
        #[arg(long, default_value_t = 0xDEADBEEF)]
        seed: u64,
        #[arg(long)]
        output: Option<String>,
    },
    /// Web service: in-process axum server + reqwest client load generator
    Web {
        #[arg(long, default_value_t = 4)]
        server_workers: usize,
        #[arg(long, default_value_t = 16)]
        client_workers: usize,
        #[arg(long, default_value = "5s")]
        warmup: String,
        #[arg(long, default_value = "60s")]
        duration: String,
        #[arg(long, default_value_t = 0xDEADBEEF)]
        seed: u64,
        #[arg(long)]
        output: Option<String>,
    },
    /// CPU-bound: parallel merge-sort with allocations in the merge step
    CpuBound {
        #[arg(long, default_value_t = num_cpus_default())]
        threads: usize,
        /// Input size in megabytes (--input-size MB).
        #[arg(long, default_value_t = 64)]
        input_size: usize,
        #[arg(long, default_value = "5s")]
        warmup: String,
        #[arg(long, default_value = "60s")]
        duration: String,
        #[arg(long, default_value_t = 0xDEADBEEF)]
        seed: u64,
        #[arg(long)]
        output: Option<String>,
    },
    /// Fragmentation-soak: 90/10 short/long-lived workload with capped
    /// long-lived state across ticks
    FragmentationSoak {
        #[arg(long, default_value_t = 10_000)]
        allocs_per_tick: u64,
        #[arg(long, default_value_t = 10_000)]
        long_lived_cap: usize,
        #[arg(long, default_value = "5s")]
        warmup: String,
        #[arg(long, default_value = "5m")]
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
    // WR-07: defer the banner until *after* clap has parsed. Printing it
    // before Cli::parse() leaked it onto stderr for `--help` and
    // `--version` (clap exits inside parse() before we'd otherwise reach
    // any subcommand dispatch), polluting CI logs and surprising scripts
    // that pipe --version output. The runtime mutual-exclusion check
    // stays before any benchmark work begins.
    allocator::assert_mutual_exclusion();

    let cli = Cli::parse();
    match cli.cmd {
        None | Some(Cmd::Version) => {
            print_version_banner();
            Ok(())
        }
        Some(Cmd::Multithread {
            threads,
            objects,
            size_dist,
            size_min,
            size_max,
            warmup,
            duration,
            seed,
            output,
        }) => {
            print_version_banner();
            run::run_multithread(
                threads,
                objects,
                &size_dist,
                size_min,
                size_max,
                &warmup,
                &duration,
                seed,
                output.as_deref(),
            )
        }
        Some(Cmd::Spmc {
            producers,
            consumers,
            capacity,
            objects_per_tick,
            payload_dist,
            warmup,
            duration,
            seed,
            output,
        }) => {
            print_version_banner();
            run::run_spmc(
                producers,
                consumers,
                capacity,
                objects_per_tick,
                &payload_dist,
                &warmup,
                &duration,
                seed,
                output.as_deref(),
            )
        }
        Some(Cmd::Mpsc {
            producers,
            consumers,
            capacity,
            objects_per_tick,
            payload_dist,
            warmup,
            duration,
            seed,
            output,
        }) => {
            print_version_banner();
            run::run_mpsc(
                producers,
                consumers,
                capacity,
                objects_per_tick,
                &payload_dist,
                &warmup,
                &duration,
                seed,
                output.as_deref(),
            )
        }
        Some(Cmd::Mpmc {
            producers,
            consumers,
            capacity,
            objects_per_tick,
            payload_dist,
            warmup,
            duration,
            seed,
            output,
        }) => {
            print_version_banner();
            run::run_mpmc(
                producers,
                consumers,
                capacity,
                objects_per_tick,
                &payload_dist,
                &warmup,
                &duration,
                seed,
                output.as_deref(),
            )
        }
        Some(Cmd::Contention {
            threads,
            alloc_size,
            iters_per_tick,
            warmup,
            duration,
            seed,
            output,
        }) => {
            print_version_banner();
            run::run_contention(
                threads,
                alloc_size,
                iters_per_tick,
                &warmup,
                &duration,
                seed,
                output.as_deref(),
            )
        }
        Some(Cmd::MemBound {
            mode,
            size,
            warmup,
            duration,
            seed,
            output,
        }) => {
            print_version_banner();
            run::run_mem_bound(&mode, size, &warmup, &duration, seed, output.as_deref())
        }
        Some(Cmd::ReallocStorm {
            target_size,
            warmup,
            duration,
            seed,
            output,
        }) => {
            print_version_banner();
            run::run_realloc_storm(target_size, &warmup, &duration, seed, output.as_deref())
        }
        Some(Cmd::Web {
            server_workers,
            client_workers,
            warmup,
            duration,
            seed,
            output,
        }) => {
            print_version_banner();
            run::run_web(
                server_workers,
                client_workers,
                &warmup,
                &duration,
                seed,
                output.as_deref(),
            )
        }
        Some(Cmd::CpuBound {
            threads,
            input_size,
            warmup,
            duration,
            seed,
            output,
        }) => {
            print_version_banner();
            run::run_cpu_bound(
                threads,
                input_size,
                &warmup,
                &duration,
                seed,
                output.as_deref(),
            )
        }
        Some(Cmd::FragmentationSoak {
            allocs_per_tick,
            long_lived_cap,
            warmup,
            duration,
            seed,
            output,
        }) => {
            print_version_banner();
            run::run_fragmentation_soak(
                allocs_per_tick,
                long_lived_cap,
                &warmup,
                &duration,
                seed,
                output.as_deref(),
            )
        }
    }
}

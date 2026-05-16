# Phase 1: Foundation MVP Slice — Research

**Phase:** 1 — Foundation MVP Slice
**Goal:** Deliver an end-to-end vertical slice for ONE allocator combo (glibc-jemalloc) proving Cargo workspace + harness + first scenario + populated results.json.
**Researched:** 2026-05-17
**Source:** Distilled from `.planning/research/{STACK,FEATURES,ARCHITECTURE,PITFALLS,SUMMARY}.md` + the user Q&A captured in CONTEXT.md.

## TL;DR

This phase is heavily front-loaded with research already done at the project level. The phase researcher's job is mostly to filter the project-level research down to "what does Phase 1 specifically need" and to flag concrete code-level patterns the planner should follow.

Every implementation choice for Phase 1 is locked by `01-CONTEXT.md` D-01..D-24. RESEARCH.md focuses on the **how-to** patterns the planner uses to instantiate those decisions.

## Architecture: workspace shape (D-01)

```
rust-benchmark-glibc-musl-mimalloc/
├── Cargo.toml                                    # workspace root
├── .planning/                                    # already exists
├── crates/
│   ├── alloc-bench-core/                         # library crate
│   │   ├── Cargo.toml
│   │   ├── build.rs                              # vergen + RUSTFLAGS capture
│   │   └── src/
│   │       ├── lib.rs                            # pub mod harness; pub mod scenarios; pub mod metrics; pub mod output;
│   │       ├── harness/
│   │       │   ├── mod.rs                        # `Harness`, `Scenario` trait
│   │       │   ├── warmup.rs
│   │       │   └── measure.rs
│   │       ├── scenarios/
│   │       │   ├── mod.rs                        # re-export each scenario
│   │       │   └── multithread.rs                # SCEN-01 implementation
│   │       ├── metrics/
│   │       │   ├── mod.rs                        # rusage + statm + alloc_stats unified
│   │       │   ├── rusage.rs                     # libc::getrusage wrapper
│   │       │   ├── statm.rs                      # /proc/self/statm parser
│   │       │   └── alloc_stats.rs                # serde_json::Value emitter
│   │       └── output.rs                         # results.json serde structs
│   ├── alloc-bench-cli/                          # binary crate
│   │   ├── Cargo.toml                            # has feature flags
│   │   ├── build.rs                              # same vergen as core (or shared)
│   │   └── src/
│   │       ├── main.rs                           # clap dispatch
│   │       ├── allocator.rs                      # #[global_allocator] + compile_error!
│   │       └── build_info.rs                     # env!() re-exports
│   └── alloc-bench-aggregator/                   # placeholder binary
│       ├── Cargo.toml                            # minimal — no real deps
│       └── src/main.rs                           # prints "Phase 4 — not yet implemented" and exits 0
└── README.md                                     # already exists (stub)
```

**Why three crates:** `alloc-bench-core` is library-only (no `#[global_allocator]` — Cargo forbids that in libs); `alloc-bench-cli` is the binary that owns allocator selection; `alloc-bench-aggregator` is a placeholder so the workspace shape is locked from Phase 1. Adding aggregator now means Phase 4 doesn't have to change the workspace structure.

## Cargo workspace root configuration

**`Cargo.toml` (workspace root):**

```toml
[workspace]
resolver = "2"
members  = ["crates/*"]

[workspace.package]
edition       = "2021"   # bumped to 2024 once tooling stabilises
rust-version  = "1.83"
license       = "MIT OR Apache-2.0"
repository    = "https://github.com/.../rust-benchmark-glibc-musl-mimalloc"
authors       = ["Marc Carré"]

[workspace.dependencies]
# Pinned for reproducibility across the matrix
clap            = { version = "4.5",  features = ["derive"] }
serde           = { version = "1",    features = ["derive"] }
serde_json      = "1"
hdrhistogram    = "7.5"
libc            = "0.2"
rand            = "0.8"
crossbeam       = "0.8"
chrono          = { version = "0.4", default-features = false, features = ["clock", "serde"] }
anyhow          = "1"
vergen          = { version = "9", features = ["build", "cargo", "rustc"] }
vergen-gitcl    = "1"
tikv-jemallocator = "0.6"
tikv-jemalloc-ctl = "0.6"
mimalloc        = { version = "0.1.43", default-features = false }

[profile.release]
lto             = "fat"
codegen-units   = 1
opt-level       = 3
strip           = "symbols"
debug           = false
panic           = "abort"
overflow-checks = false

[profile.bench-debug]                              # opt-in for troubleshooting
inherits        = "release"
strip           = "none"
debug           = "full"
lto             = "thin"

[profile.dev]
# default; used during development
```

(D-02 satisfied. `panic = "abort"` is conservative for a benchmark — avoids unwinding cost.)

## Allocator selection (D-03 to D-05, D-21)

**`crates/alloc-bench-cli/Cargo.toml`:**

```toml
[package]
name        = "alloc-bench-cli"
version     = "0.1.0"
edition.workspace      = true
license.workspace      = true
rust-version.workspace = true

[[bin]]
name = "alloc-bench-cli"
path = "src/main.rs"

[features]
default        = []
alloc-jemalloc = ["dep:tikv-jemallocator", "dep:tikv-jemalloc-ctl"]
alloc-mimalloc = ["dep:mimalloc"]

[dependencies]
alloc-bench-core = { path = "../alloc-bench-core" }
clap             = { workspace = true }
anyhow           = { workspace = true }
serde_json       = { workspace = true }

# allocator deps — gated by features
tikv-jemallocator = { workspace = true, optional = true }
tikv-jemalloc-ctl = { workspace = true, optional = true }
mimalloc          = { workspace = true, optional = true }

[build-dependencies]
vergen       = { workspace = true }
vergen-gitcl = { workspace = true }
```

**`crates/alloc-bench-cli/src/allocator.rs`:**

```rust
//! Compile-time global allocator selection.
//!
//! Cargo features `alloc-jemalloc` and `alloc-mimalloc` are mutually exclusive.
//! Enabling both is a hard compile error (D-04).

#[cfg(all(feature = "alloc-jemalloc", feature = "alloc-mimalloc"))]
compile_error!(
    "cargo features `alloc-jemalloc` and `alloc-mimalloc` are mutually exclusive. \
     Build with at most one allocator feature."
);

#[cfg(feature = "alloc-jemalloc")]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(feature = "alloc-mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// Returns the active allocator's canonical name. Used in the version banner
/// and in the `build.allocator` field of the results JSON.
pub const fn name() -> &'static str {
    #[cfg(feature = "alloc-jemalloc")] { "jemalloc" }
    #[cfg(all(feature = "alloc-mimalloc", not(feature = "alloc-jemalloc")))] { "mimalloc" }
    #[cfg(not(any(feature = "alloc-jemalloc", feature = "alloc-mimalloc")))] {
        if cfg!(target_env = "musl") { "mallocng" }
        else if cfg!(target_os = "macos") { "libmalloc" }
        else { "ptmalloc" }
    }
}

/// Defense-in-depth runtime check (D-04). Strictly redundant with the
/// `compile_error!` above but documents the runtime contract.
pub fn assert_mutual_exclusion() {
    if cfg!(all(feature = "alloc-jemalloc", feature = "alloc-mimalloc")) {
        panic!("mutually exclusive allocator features enabled at runtime");
    }
}

/// Emit allocator-internal stats as a `serde_json::Value` for the results JSON.
pub fn stats() -> serde_json::Value {
    #[cfg(feature = "alloc-jemalloc")]
    {
        use tikv_jemalloc_ctl::{epoch, stats};
        epoch::advance().ok();  // refresh counters
        return serde_json::json!({
            "kind":      "jemalloc",
            "allocated": stats::allocated::read().unwrap_or(0),
            "resident":  stats::resident::read().unwrap_or(0),
            "retained":  stats::retained::read().unwrap_or(0),
            "active":    stats::active::read().unwrap_or(0),
        });
    }
    #[cfg(feature = "alloc-mimalloc")]
    {
        // mimalloc 0.1.43 doesn't expose mi_stats_get bindings directly.
        // Phase 1: emit a kind discriminator + best-effort stub. Plan-phase
        // may extend this to `mi_stats_print` capture if the upstream API
        // is reachable. Otherwise this is fine — D-13 says allocator_stats
        // is `serde_json::Value` and each variant has its own shape.
        return serde_json::json!({ "kind": "mimalloc" });
    }
    #[allow(unreachable_code)]
    serde_json::json!({ "kind": "system" })
}
```

## Build metadata via vergen (D-06, D-07)

**`crates/alloc-bench-cli/build.rs`:**

```rust
use vergen_gitcl::{BuildBuilder, CargoBuilder, GitclBuilder, RustcBuilder, Emitter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let build  = BuildBuilder::all_build()?;
    let cargo  = CargoBuilder::all_cargo()?;
    let git    = GitclBuilder::all_git()?;
    let rustc  = RustcBuilder::all_rustc()?;

    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&cargo)?
        .add_instructions(&git)?
        .add_instructions(&rustc)?
        .emit()?;

    // Capture RUSTFLAGS — vergen 9 doesn't include this.
    let rustflags = std::env::var("CARGO_ENCODED_RUSTFLAGS")
        .map(|s| s.replace('\x1f', " "))                  // unit-separator → space
        .unwrap_or_default();
    println!("cargo:rustc-env=BUILD_RUSTFLAGS={}", rustflags);

    Ok(())
}
```

**`crates/alloc-bench-cli/src/build_info.rs`:**

```rust
pub const RUSTC_VERSION:    &str = env!("VERGEN_RUSTC_SEMVER");
pub const HOST_TRIPLE:      &str = env!("VERGEN_RUSTC_HOST_TRIPLE");
pub const TARGET_TRIPLE:    &str = env!("VERGEN_CARGO_TARGET_TRIPLE");
pub const CARGO_OPT_LEVEL:  &str = env!("VERGEN_CARGO_OPT_LEVEL");
pub const PROFILE:          &str = env!("VERGEN_CARGO_DEBUG"); // "false" in release
pub const GIT_SHA:          &str = env!("VERGEN_GIT_SHA");
pub const GIT_DIRTY:        &str = env!("VERGEN_GIT_DIRTY");
pub const BUILD_TIMESTAMP:  &str = env!("VERGEN_BUILD_TIMESTAMP");
pub const RUSTFLAGS:        &str = env!("BUILD_RUSTFLAGS");
pub const CRATE_VERSION:    &str = env!("CARGO_PKG_VERSION");
```

## Version banner format (D-23)

**`crates/alloc-bench-cli/src/main.rs` startup block:**

```rust
use alloc_bench_cli::{allocator, build_info};

fn print_version_banner() {
    eprintln!(
        "alloc-bench v{ver} (allocator={alloc}, rustc={rustc}, target={tgt}, host={host}, profile={prof}, git={sha8}{dirty}, built={ts})",
        ver   = build_info::CRATE_VERSION,
        alloc = allocator::name(),
        rustc = build_info::RUSTC_VERSION,
        tgt   = build_info::TARGET_TRIPLE,
        host  = build_info::HOST_TRIPLE,
        prof  = if build_info::PROFILE == "false" { "release" } else { "debug" },
        sha8  = &build_info::GIT_SHA[..build_info::GIT_SHA.len().min(8)],
        dirty = if build_info::GIT_DIRTY == "true" { "-dirty" } else { "" },
        ts    = build_info::BUILD_TIMESTAMP,
    );
}
```

Banner goes to **stderr** (D-24) so stdout stays clean for `--output -` JSON streaming.

## Harness contract (D-08, D-09, D-10)

**`crates/alloc-bench-core/src/harness/mod.rs`:**

```rust
use std::time::{Duration, Instant};
use anyhow::Result;
use hdrhistogram::Histogram;

use crate::metrics::{rusage::read_rusage, statm::read_rss_kb};
use crate::output::{HarnessInfo, LatencyNs, Metrics, RssGrowthSample};

/// Marker trait for any value the scenario `tick()` can return. Auto-implemented
/// for every `'static` type, so scenarios just return whatever they allocate.
pub trait SinkValue {}
impl<T: ?Sized + 'static> SinkValue for T {}

/// Contract for any benchmark scenario. The harness owns warm-up, measurement,
/// black-boxing, and metric collection. Scenarios just `tick()` and yield work.
pub trait Scenario {
    fn name(&self) -> &'static str;
    fn config_json(&self) -> serde_json::Value;
    fn setup(&mut self) -> Result<()>;
    fn tick(&mut self) -> Box<dyn SinkValue>;
    fn teardown(&mut self) {}
}

pub struct HarnessConfig {
    pub warmup:   Duration,
    pub measure:  Duration,
    pub seed:     u64,
}

pub struct HarnessResult {
    pub harness: HarnessInfo,
    pub metrics: Metrics,
}

pub fn run<S: Scenario>(scenario: &mut S, cfg: &HarnessConfig) -> Result<HarnessResult> {
    if cfg.warmup < Duration::from_secs(1) {
        anyhow::bail!("warm-up must be >= 1s; allocator caches need to populate (see PITFALLS.md §1.5)");
    }
    scenario.setup()?;

    // Phase 1: warm-up — black_box only, no recording.
    let warm_end = Instant::now() + cfg.warmup;
    while Instant::now() < warm_end {
        std::hint::black_box(scenario.tick());
    }

    // Phase 2: measurement.
    let mut hist = Histogram::<u64>::new_with_bounds(1, 60_000_000_000, 3)?;
    let mut rss_samples: Vec<RssGrowthSample> = Vec::new();
    let measure_start = Instant::now();
    let measure_end   = measure_start + cfg.measure;
    let mut last_rss  = measure_start;
    let mut samples_count: u64 = 0;

    while Instant::now() < measure_end {
        let t0 = Instant::now();
        std::hint::black_box(scenario.tick());
        hist.record(t0.elapsed().as_nanos() as u64).ok();
        samples_count += 1;
        if last_rss.elapsed() >= Duration::from_secs(1) {
            rss_samples.push(RssGrowthSample {
                t_s:     measure_start.elapsed().as_secs(),
                rss_kb:  read_rss_kb()?,
            });
            last_rss = Instant::now();
        }
    }

    let elapsed_s = measure_start.elapsed().as_secs_f64();
    scenario.teardown();

    let throughput = samples_count as f64 / elapsed_s;
    let latency = LatencyNs {
        p50:  hist.value_at_quantile(0.50),
        p95:  hist.value_at_quantile(0.95),
        p99:  hist.value_at_quantile(0.99),
        p999: hist.value_at_quantile(0.999),
        max:  hist.max(),
    };
    let rusage = read_rusage()?;

    Ok(HarnessResult {
        harness: HarnessInfo {
            warmup_duration_s:     cfg.warmup.as_secs(),
            measurement_duration_s: cfg.measure.as_secs(),
            samples_count,
        },
        metrics: Metrics {
            throughput_ops_per_s: throughput,
            latency_ns:           latency,
            peak_rss_kb:          rusage.peak_rss_kb,
            rss_growth_samples:   rss_samples,
            rusage,
            allocator_stats:      crate::metrics::alloc_stats::sample(),
        },
    })
}
```

The scenario provides the workload, the harness provides the discipline. PITFALLS.md §1.1 (DCE) is handled by `black_box(tick())`; §1.5 (warm-up) by the `bail!` and the warm-up loop.

## Metrics (D-14 to D-17)

**`crates/alloc-bench-core/src/metrics/rusage.rs`:**

```rust
use anyhow::{anyhow, Result};
use libc::{getrusage, rusage, RUSAGE_SELF};
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct Rusage {
    pub user_time_s:    f64,
    pub sys_time_s:     f64,
    pub peak_rss_kb:    u64,
    pub minor_faults:   u64,
    pub major_faults:   u64,
    pub voluntary_csw:  u64,
    pub involuntary_csw: u64,
}

pub fn read_rusage() -> Result<Rusage> {
    let mut ru: rusage = unsafe { std::mem::zeroed() };
    let rc = unsafe { getrusage(RUSAGE_SELF, &mut ru) };
    if rc != 0 {
        return Err(anyhow!("getrusage(RUSAGE_SELF) failed: {}", std::io::Error::last_os_error()));
    }
    Ok(Rusage {
        user_time_s:     ru.ru_utime.tv_sec as f64 + ru.ru_utime.tv_usec as f64 / 1e6,
        sys_time_s:      ru.ru_stime.tv_sec as f64 + ru.ru_stime.tv_usec as f64 / 1e6,
        peak_rss_kb:     ru.ru_maxrss as u64,            // Linux: kilobytes
        minor_faults:    ru.ru_minflt as u64,
        major_faults:    ru.ru_majflt as u64,
        voluntary_csw:   ru.ru_nvcsw as u64,
        involuntary_csw: ru.ru_nivcsw as u64,
    })
}
```

**`crates/alloc-bench-core/src/metrics/statm.rs`:**

```rust
use anyhow::{Context, Result};
use std::fs;

pub fn read_rss_kb() -> Result<u64> {
    let s = fs::read_to_string("/proc/self/statm").context("read /proc/self/statm")?;
    // statm fields (whitespace-separated, in pages):
    //   size resident shared text lib data dt
    let resident_pages: u64 = s.split_whitespace()
        .nth(1)
        .context("statm field 2")?
        .parse()?;
    let page_kb = page_size::get() / 1024;     // page_size crate, or libc::sysconf
    Ok(resident_pages * page_kb as u64)
}
```

(Use the `page_size` crate or call `libc::sysconf(libc::_SC_PAGESIZE)`. The latter avoids one extra dep — recommend that.)

## results.json schema (D-11, D-12)

**`crates/alloc-bench-core/src/output.rs`:**

```rust
use serde::Serialize;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Serialize)]
pub struct Run {
    pub schema_version: u32,
    pub run_id:         String,
    pub env:            Env,
    pub build:          Build,
    pub scenario:       ScenarioInfo,
    pub harness:        HarnessInfo,
    pub metrics:        Metrics,
}

#[derive(Serialize)]
pub struct Env {
    pub os:                 String,    // "linux"
    pub os_version:         String,    // kernel from `uname -r` or /proc/version
    pub docker_image:       Option<String>,  // None for host runs
    pub cpu_model:          String,    // /proc/cpuinfo "model name"
    pub cpu_count:          u32,       // num_cpus
    pub memory_total_kb:    u64,       // /proc/meminfo MemTotal
}

#[derive(Serialize)]
pub struct Build {
    pub allocator:        String,
    pub rustc_version:    String,
    pub target_triple:    String,
    pub host_triple:      String,
    pub profile:          String,        // "release"
    pub git_sha:          String,
    pub git_dirty:        bool,
    pub build_timestamp:  String,        // RFC3339
    pub rustflags:        String,
}

#[derive(Serialize)]
pub struct ScenarioInfo {
    pub name:    String,
    pub config:  serde_json::Value,
}

#[derive(Serialize)]
pub struct HarnessInfo {
    pub warmup_duration_s:      u64,
    pub measurement_duration_s: u64,
    pub samples_count:          u64,
}

#[derive(Serialize)]
pub struct LatencyNs {
    pub p50:  u64,
    pub p95:  u64,
    pub p99:  u64,
    pub p999: u64,
    pub max:  u64,
}

#[derive(Serialize)]
pub struct RssGrowthSample {
    pub t_s:    u64,
    pub rss_kb: u64,
}

#[derive(Serialize)]
pub struct Metrics {
    pub throughput_ops_per_s: f64,
    pub latency_ns:           LatencyNs,
    pub peak_rss_kb:          u64,
    pub rss_growth_samples:   Vec<RssGrowthSample>,
    pub rusage:               crate::metrics::rusage::Rusage,
    pub allocator_stats:      serde_json::Value,
}
```

`Env` collection lives in `crates/alloc-bench-core/src/metrics/env.rs` (read `/proc/cpuinfo`, `/proc/meminfo`, `/proc/version`; `cpu_count = num_cpus::get()` or read `/proc/cpuinfo` processor count to avoid a dep).

## First scenario: multithread (D-18 to D-20)

**`crates/alloc-bench-core/src/scenarios/multithread.rs`:**

```rust
use crate::harness::{Scenario, SinkValue};
use anyhow::Result;
use rand::{rngs::SmallRng, Rng, SeedableRng};
use serde_json::json;

pub struct MultithreadConfig {
    pub threads:        usize,
    pub objects:        usize,
    pub size_dist:      SizeDist,
    pub size_min:       usize,
    pub size_max:       usize,
    pub seed:           u64,
}

pub enum SizeDist { Uniform, Bimodal, Pareto }

pub struct Multithread {
    cfg: MultithreadConfig,
}

impl Multithread {
    pub fn new(cfg: MultithreadConfig) -> Self { Self { cfg } }
}

impl Scenario for Multithread {
    fn name(&self) -> &'static str { "multithread" }

    fn config_json(&self) -> serde_json::Value {
        json!({
            "threads":   self.cfg.threads,
            "objects":   self.cfg.objects,
            "size_dist": match self.cfg.size_dist {
                SizeDist::Uniform => "uniform",
                SizeDist::Bimodal => "bimodal",
                SizeDist::Pareto  => "pareto",
            },
            "size_min":  self.cfg.size_min,
            "size_max":  self.cfg.size_max,
            "seed":      self.cfg.seed,
        })
    }

    fn setup(&mut self) -> Result<()> { Ok(()) }

    fn tick(&mut self) -> Box<dyn SinkValue> {
        // One tick = one round of N threads × M allocations.
        std::thread::scope(|s| {
            let handles: Vec<_> = (0..self.cfg.threads).map(|t| {
                let cfg = &self.cfg;
                s.spawn(move || {
                    let mut rng = SmallRng::seed_from_u64(cfg.seed.wrapping_add(t as u64));
                    let mut sink: Vec<Box<[u8]>> = Vec::with_capacity(cfg.objects);
                    for _ in 0..cfg.objects {
                        let size = sample_size(&mut rng, cfg);
                        let mut b = vec![0u8; size].into_boxed_slice();
                        b[size / 2] = rng.gen::<u8>().max(1);   // PITFALLS.md §1.2
                        sink.push(std::hint::black_box(b));
                    }
                    sink                                            // dropped on join
                })
            }).collect();
            for h in handles { h.join().expect("thread panicked"); }
        });
        Box::new(()) as Box<dyn SinkValue>
    }
}

fn sample_size<R: Rng>(rng: &mut R, cfg: &MultithreadConfig) -> usize {
    match cfg.size_dist {
        SizeDist::Uniform => rng.gen_range(cfg.size_min..=cfg.size_max),
        SizeDist::Bimodal => if rng.gen::<f32>() < 0.9 { 16 } else { cfg.size_max },
        SizeDist::Pareto  => {
            // shape α=1.5; quick approximation via inverse CDF
            let u: f64 = rng.gen_range(1e-9..1.0);
            let v = (cfg.size_min as f64) * u.powf(-1.0/1.5);
            (v as usize).clamp(cfg.size_min, cfg.size_max)
        }
    }
}
```

Note: `tick()` is heavyweight (it drives a full N×M round). The harness measures the latency of each round, not each individual `Box::new`. That's fine — what we want is per-round throughput and per-round tail latency. Documented in CONTEXT.md D-19 wording.

## CLI shape

**`crates/alloc-bench-cli/src/main.rs` (sketch):**

```rust
use anyhow::Result;
use clap::{Parser, Subcommand};

mod allocator;
mod build_info;

#[derive(Parser)]
#[command(name = "alloc-bench-cli", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Multi-thread allocation stress (Phase 1 anchor scenario)
    Multithread {
        #[arg(long, default_value_t = num_cpus())]      threads: usize,
        #[arg(long, default_value_t = 100_000)]         objects: usize,
        #[arg(long, default_value = "uniform")]         size_dist: String,
        #[arg(long, default_value_t = 16)]              size_min: usize,
        #[arg(long, default_value_t = 1024)]            size_max: usize,
        #[arg(long, default_value = "5s")]              warmup: String,
        #[arg(long, default_value = "60s")]             duration: String,
        #[arg(long, default_value_t = 0xDEADBEEF)]      seed: u64,
        #[arg(long)]                                    output: Option<String>,
    },
    /// Print version-banner only and exit (also printed at startup of any subcommand)
    Version,
}

fn main() -> Result<()> {
    print_version_banner();                          // stderr
    allocator::assert_mutual_exclusion();            // defense-in-depth (D-04)
    assert_linux();                                  // D-22

    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Version => Ok(()),
        Cmd::Multithread { threads, objects, size_dist, size_min, size_max,
                           warmup, duration, seed, output } => {
            let cfg = parse_multithread_args(threads, objects, &size_dist,
                                             size_min, size_max, &warmup, &duration, seed)?;
            let run = run_scenario(cfg)?;
            emit(run, output.as_deref())
        }
    }
}

fn assert_linux() {
    if !cfg!(target_os = "linux") {
        eprintln!("error: Phase 1 supports x86_64-unknown-linux-gnu only; \
                   macOS host baseline arrives in Phase 3 (see ROADMAP.md Phase 3 success criterion 4).");
        std::process::exit(2);
    }
}
```

`run_scenario` builds `Env` + `Build` + `ScenarioInfo` + invokes `harness::run`, then assembles a `Run` and serializes to either stdout or `--output` path.

## Threats & gotchas (Phase-1-specific)

| ID | Pitfall | Phase 1 mitigation |
|----|---------|-------------------|
| P1.1 | DCE eliminating allocations | `black_box(tick())` in harness + `b[size/2] = ...` write inside scenario |
| P1.2 | Warm-up too short | `bail!` if `warmup < 1s`; default 5s |
| P1.3 | LTO breaking jemalloc/mimalloc link | Smoke-build both feature combos in Phase 1 verification |
| P1.4 | `target-cpu=native` portability | Phase 1 uses host build only — `target-cpu=native` is fine; Docker matrix changes this in Phase 3 |
| P1.5 | `#[global_allocator]` init order | Keep `allocator.rs` minimal — only `static GLOBAL` and pub fns; no `lazy_static` |
| P1.6 | `getrusage::ru_maxrss` units | Linux returns kilobytes — store as kB; document in schema |
| P1.7 | `/proc/self/statm` missing in non-Linux | Already gated by D-22 (Linux-only Phase 1) |
| P1.8 | `vergen` vs git availability | `vergen-gitcl` falls back gracefully when not in a git checkout — document in build.rs |

## Open questions for the planner

- **Test strategy:** Phase 1 deserves at least: a unit test for `read_rss_kb` (Linux), a unit test for `Multithread::sample_size` distributions, and an integration test that builds with `--features alloc-jemalloc` and parses the resulting JSON. Plan-phase decides depth.
- **`Cargo.toml [workspace.dependencies]` placement:** the example above puts `tikv-jemallocator` etc. in workspace deps. Plan-phase should confirm this works correctly when `alloc-bench-core` doesn't activate any allocator feature (deps stay optional).
- **Aggregator placeholder:** keep it as `cargo-build-but-do-nothing` to avoid a workspace member that fails to compile in Phase 1 verification.

## Required reading (mandatory for plan-phase)

- `.planning/research/STACK.md` (entire file)
- `.planning/research/ARCHITECTURE.md` (entire file)
- `.planning/research/PITFALLS.md` §1.1, §1.2, §1.5, §2.5, §5.1-5.3
- `.planning/phases/01-foundation-mvp-slice/01-CONTEXT.md` (locked decisions)
- `.planning/REQUIREMENTS.md` (REQ-IDs in scope: WS-01..05, HARN-01..08, SCEN-01, REPR-02)

## RESEARCH COMPLETE

Phase 1 research distills the project-level research bundle into Phase-1-actionable patterns. The planner can now write PLAN.md(s) with concrete file paths, code skeletons, dependency versions, and acceptance criteria.

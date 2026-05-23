use std::time::Duration;

use alloc_bench_core::metrics::env::read_env;
use alloc_bench_core::output::{Build, Run, ScenarioInfo};
use alloc_bench_core::scenarios::{
    ChannelConfig, ChannelKind, Contention, ContentionConfig, CpuBound, CpuBoundConfig,
    FragmentationConfig, FragmentationSoak, MemBound, MemBoundConfig, MemBoundMode, Mpmc, Mpsc,
    Multithread, MultithreadConfig, PayloadDist, ReallocStorm, ReallocStormConfig, SizeDist, Spmc,
    Web, WebConfig,
};
use alloc_bench_core::{run, HarnessConfig, SCHEMA_VERSION};
use anyhow::{anyhow, ensure, Context, Result};

use crate::{allocator, build_info};

/// Parse human-readable durations like "5s", "500ms", "2m". No external dep.
///
/// WR-09: suffix-check ordering is correctness-critical. "ms" must be
/// stripped before "s" because "5ms" ends with "s" too — reordering the
/// branches would parse "5ms" as "5m → 300s". The unit tests below pin
/// "5ms" and "5m" as adjacent assertions to fail fast on accidental
/// reordering.
pub fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim();
    // Order: longest/most-specific suffix first. Do not reorder.
    if let Some(rest) = s.strip_suffix("ms") {
        let n: u64 = rest
            .parse()
            .with_context(|| format!("invalid duration: {s}"))?;
        return Ok(Duration::from_millis(n));
    }
    if let Some(rest) = s.strip_suffix('s') {
        let n: u64 = rest
            .parse()
            .with_context(|| format!("invalid duration: {s}"))?;
        return Ok(Duration::from_secs(n));
    }
    if let Some(rest) = s.strip_suffix('m') {
        let n: u64 = rest
            .parse()
            .with_context(|| format!("invalid duration: {s}"))?;
        // WR-09: n * 60 wrapped silently for very large n. Use checked_mul
        // so a user passing e.g. '18446744073709551615m' gets a clear
        // error instead of a tiny wrapped Duration.
        let secs = n
            .checked_mul(60)
            .with_context(|| format!("duration too large: {s}"))?;
        return Ok(Duration::from_secs(secs));
    }
    Err(anyhow!(
        "invalid duration: {s} (expected suffix ms|s|m, e.g. 5s)"
    ))
}

/// Build a `Run` record from a finished `HarnessOutcome`. Centralises the
/// Build / Env / Run-record construction so single-scenario dispatch
/// (`run_<name>`) and the multi-scenario `run_all` path share one source of
/// truth — no drift in `git_sha`, `rustflags`, `run_id` shape, or schema
/// fields.
///
/// Phase-2 additions:
/// - `status` / `error` parameters control the Phase-2 additive `Run.status`
///   and `Run.error` fields. Single-scenario callers pass `None` for both
///   (legacy byte-identical shape preserved). `run_all` is the only caller
///   that ever passes `Some(...)`.
/// - `unit` controls the Phase-2 additive `ScenarioInfo.unit` label
///   (`"req_per_s"`, `"iters_per_s"`, etc.).
#[allow(clippy::too_many_arguments)]
pub(crate) fn assemble_run(
    scenario_name: &str,
    scenario_config: serde_json::Value,
    scenario_unit: Option<String>,
    outcome: alloc_bench_core::HarnessOutcome,
    status: Option<String>,
    error: Option<String>,
) -> Result<Run> {
    let env = read_env()?;
    // IN-01: shared truncation helper in build_info::short_sha.
    let sha8 = build_info::short_sha();
    let run_id = format!("{}-{sha8}", chrono::Utc::now().to_rfc3339());

    let scenario_info = ScenarioInfo {
        name: scenario_name.to_string(),
        config: scenario_config,
        unit: scenario_unit,
    };

    let build = Build {
        allocator: allocator::name().to_string(),
        rustc_version: build_info::RUSTC_VERSION.to_string(),
        target_triple: build_info::TARGET_TRIPLE.to_string(),
        host_triple: build_info::HOST_TRIPLE.to_string(),
        profile: build_info::PROFILE.to_string(),
        git_sha: build_info::GIT_SHA.to_string(),
        git_dirty: build_info::GIT_DIRTY == "true",
        build_timestamp: build_info::BUILD_TIMESTAMP.to_string(),
        rustflags: build_info::RUSTFLAGS.to_string(),
    };

    Ok(Run {
        schema_version: SCHEMA_VERSION,
        run_id,
        env,
        build,
        scenario: scenario_info,
        harness: outcome.harness,
        metrics: outcome.metrics,
        status,
        error,
    })
}

/// Serialize a single `Run` to pretty JSON and write it to either a file
/// (`Some(path)`) or stdout (`None`). Mirrors the Phase-1 single-scenario
/// emission path. `run_all` (Task 3) does NOT use this — it serialises the
/// `Vec<Run>` array directly.
pub(crate) fn write_or_print(run: &Run, output: Option<&str>) -> Result<()> {
    let json = serde_json::to_string_pretty(run)?;
    match output {
        Some(path) => {
            std::fs::write(path, &json).with_context(|| format!("writing results to {path}"))?
        }
        None => println!("{json}"),
    }
    Ok(())
}

/// Drive the harness for a single scenario, then assemble + emit the Run
/// record. Centralised so each `run_<name>` function below is just argument
/// parsing + scenario construction + this call. Avoids drift between
/// scenarios in how Build/Env/Run records are built.
fn drive_and_emit<S: alloc_bench_core::Scenario>(
    scenario: &mut S,
    name: &str,
    unit: Option<String>,
    cfg: &HarnessConfig,
    output: Option<&str>,
) -> Result<()> {
    let outcome = run(scenario, cfg, allocator::stats)?;
    // Single-scenario runs always produce a successful Run with `status:
    // None, error: None` so Phase-1 byte-identical JSON shape is preserved.
    let run_record = assemble_run(
        name,
        alloc_bench_core::Scenario::config_json(scenario),
        unit,
        outcome,
        None,
        None,
    )?;
    write_or_print(&run_record, output)
}

#[allow(clippy::too_many_arguments)]
pub fn run_multithread(
    threads: usize,
    objects: usize,
    size_dist: &str,
    size_min: usize,
    size_max: usize,
    warmup: &str,
    duration: &str,
    seed: u64,
    output: Option<&str>,
) -> Result<()> {
    let warmup = parse_duration(warmup)?;
    let measure = parse_duration(duration)?;
    let dist: SizeDist = size_dist.parse()?;

    // WR-02 / WR-03: validate inputs up-front so the worker hot path is
    // panic-free.
    let cfg = MultithreadConfig {
        threads,
        objects,
        size_dist: dist,
        size_min,
        size_max,
        seed,
    }
    .validated()?;
    let mut scenario = Multithread::new(cfg);

    let hcfg = HarnessConfig {
        warmup,
        measure,
        seed,
    };
    drive_and_emit(&mut scenario, "multithread", None, &hcfg, output)
}

// =============================================================================
// Phase-2 channel scenarios (SCEN-03/04/05)
// =============================================================================

#[allow(clippy::too_many_arguments)]
pub fn run_spmc(
    producers: usize,
    consumers: usize,
    capacity: usize,
    objects_per_tick: u64,
    payload_dist: &str,
    warmup: &str,
    duration: &str,
    seed: u64,
    output: Option<&str>,
) -> Result<()> {
    // Topology constraint: SPMC = "Single Producer, Multi Consumer". The
    // shared ChannelConfig is permissive enough to model all three
    // topologies, so we enforce the SP invariant at config-construction
    // via WR-06's `validated_for(ChannelKind::Spmc)`. The early CLI-level
    // check below stays for a clearer error message at the CLI surface.
    ensure!(
        producers == 1,
        "SPMC requires --producers 1 (got {producers})"
    );

    let warmup = parse_duration(warmup)?;
    let measure = parse_duration(duration)?;
    let dist: PayloadDist = payload_dist.parse()?;

    let cfg = ChannelConfig {
        producers,
        consumers,
        capacity,
        objects_per_tick,
        payload_dist: dist,
        seed,
    }
    .validated_for(ChannelKind::Spmc)?;
    let mut scenario = Spmc::new(cfg);

    let hcfg = HarnessConfig {
        warmup,
        measure,
        seed,
    };
    drive_and_emit(
        &mut scenario,
        "spmc",
        Some("iters_per_s".to_string()),
        &hcfg,
        output,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn run_mpsc(
    producers: usize,
    consumers: usize,
    capacity: usize,
    objects_per_tick: u64,
    payload_dist: &str,
    warmup: &str,
    duration: &str,
    seed: u64,
    output: Option<&str>,
) -> Result<()> {
    ensure!(
        consumers == 1,
        "MPSC requires --consumers 1 (got {consumers})"
    );

    let warmup = parse_duration(warmup)?;
    let measure = parse_duration(duration)?;
    let dist: PayloadDist = payload_dist.parse()?;

    let cfg = ChannelConfig {
        producers,
        consumers,
        capacity,
        objects_per_tick,
        payload_dist: dist,
        seed,
    }
    .validated_for(ChannelKind::Mpsc)?;
    let mut scenario = Mpsc::new(cfg);

    let hcfg = HarnessConfig {
        warmup,
        measure,
        seed,
    };
    drive_and_emit(
        &mut scenario,
        "mpsc",
        Some("iters_per_s".to_string()),
        &hcfg,
        output,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn run_mpmc(
    producers: usize,
    consumers: usize,
    capacity: usize,
    objects_per_tick: u64,
    payload_dist: &str,
    warmup: &str,
    duration: &str,
    seed: u64,
    output: Option<&str>,
) -> Result<()> {
    let warmup = parse_duration(warmup)?;
    let measure = parse_duration(duration)?;
    let dist: PayloadDist = payload_dist.parse()?;

    let cfg = ChannelConfig {
        producers,
        consumers,
        capacity,
        objects_per_tick,
        payload_dist: dist,
        seed,
    }
    .validated_for(ChannelKind::Mpmc)?;
    let mut scenario = Mpmc::new(cfg);

    let hcfg = HarnessConfig {
        warmup,
        measure,
        seed,
    };
    drive_and_emit(
        &mut scenario,
        "mpmc",
        Some("iters_per_s".to_string()),
        &hcfg,
        output,
    )
}

// =============================================================================
// Phase-2 contention / mem-bound / realloc-storm
// =============================================================================

#[allow(clippy::too_many_arguments)]
pub fn run_contention(
    threads: usize,
    alloc_size: usize,
    iters_per_tick: u64,
    warmup: &str,
    duration: &str,
    seed: u64,
    output: Option<&str>,
) -> Result<()> {
    let warmup = parse_duration(warmup)?;
    let measure = parse_duration(duration)?;

    let cfg = ContentionConfig {
        threads,
        alloc_size,
        iters_per_tick,
        seed,
    }
    .validated()?;
    let mut scenario = Contention::new(cfg);

    let hcfg = HarnessConfig {
        warmup,
        measure,
        seed,
    };
    drive_and_emit(&mut scenario, "contention", None, &hcfg, output)
}

pub fn run_mem_bound(
    mode: &str,
    size_mb: usize,
    warmup: &str,
    duration: &str,
    seed: u64,
    output: Option<&str>,
) -> Result<()> {
    let warmup = parse_duration(warmup)?;
    let measure = parse_duration(duration)?;
    let mode: MemBoundMode = mode.parse()?;

    let cfg = MemBoundConfig {
        mode,
        size_mb,
        seed,
    }
    .validated()?;
    let mut scenario = MemBound::new(cfg);

    let hcfg = HarnessConfig {
        warmup,
        measure,
        seed,
    };
    drive_and_emit(&mut scenario, "mem-bound", None, &hcfg, output)
}

pub fn run_realloc_storm(
    target_size_mb: usize,
    warmup: &str,
    duration: &str,
    seed: u64,
    output: Option<&str>,
) -> Result<()> {
    let warmup = parse_duration(warmup)?;
    let measure = parse_duration(duration)?;

    let cfg = ReallocStormConfig {
        target_size_mb,
        seed,
    }
    .validated()?;
    let mut scenario = ReallocStorm::new(cfg);

    let hcfg = HarnessConfig {
        warmup,
        measure,
        seed,
    };
    drive_and_emit(&mut scenario, "realloc-storm", None, &hcfg, output)
}

// =============================================================================
// Phase-2 Wave-2 — web / cpu-bound / fragmentation-soak (SCEN-02/06/09)
// =============================================================================

#[allow(clippy::too_many_arguments)]
pub fn run_web(
    server_workers: usize,
    client_workers: usize,
    warmup: &str,
    duration: &str,
    seed: u64,
    output: Option<&str>,
) -> Result<()> {
    let warmup = parse_duration(warmup)?;
    let measure = parse_duration(duration)?;

    let cfg = WebConfig {
        server_workers,
        client_workers,
        seed,
    }
    .validated()?;
    let mut scenario = Web::new(cfg);

    let hcfg = HarnessConfig {
        warmup,
        measure,
        seed,
    };
    // CONTEXT.md schema decision: web reports req/s as the throughput unit.
    drive_and_emit(
        &mut scenario,
        "web",
        Some("req_per_s".to_string()),
        &hcfg,
        output,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn run_cpu_bound(
    threads: usize,
    input_size_mb: usize,
    warmup: &str,
    duration: &str,
    seed: u64,
    output: Option<&str>,
) -> Result<()> {
    let warmup = parse_duration(warmup)?;
    let measure = parse_duration(duration)?;

    let cfg = CpuBoundConfig {
        threads,
        input_size_mb,
        seed,
    }
    .validated()?;
    let mut scenario = CpuBound::new(cfg);

    let hcfg = HarnessConfig {
        warmup,
        measure,
        seed,
    };
    drive_and_emit(&mut scenario, "cpu-bound", None, &hcfg, output)
}

#[allow(clippy::too_many_arguments)]
pub fn run_fragmentation_soak(
    allocs_per_tick: u64,
    long_lived_cap: usize,
    warmup: &str,
    duration: &str,
    seed: u64,
    output: Option<&str>,
) -> Result<()> {
    let warmup = parse_duration(warmup)?;
    let measure = parse_duration(duration)?;

    let cfg = FragmentationConfig {
        allocs_per_tick,
        long_lived_cap,
        seed,
    }
    .validated()?;
    let mut scenario = FragmentationSoak::new(cfg);

    let hcfg = HarnessConfig {
        warmup,
        measure,
        seed,
    };
    drive_and_emit(&mut scenario, "fragmentation-soak", None, &hcfg, output)
}

// =============================================================================
// Phase-2 Wave-3 — run-all (SCEN-11)
// =============================================================================

/// Builder closure that constructs a boxed scenario lazily. The closure
/// type is `FnOnce` returning `Box<dyn Scenario>` so the registry can
/// capture `seed` by move and run-all only pays the construction cost
/// for scenarios it actually invokes (and only once per invocation).
type ScenarioBuilder = Box<dyn FnOnce() -> Result<Box<dyn alloc_bench_core::Scenario>> + 'static>;

/// Returns the canonical 10-scenario registry used by `run-all`. Each entry
/// is `(name, optional unit label, builder)`. Order is the documented
/// execution order — multithread first (matches Phase-1), web LAST so its
/// port-bind work doesn't slow earlier scenarios. Ordering is asserted by
/// the integration test (`run_all_smoke`) as a set of unique names, not a
/// sequence — Phase-4 aggregator does its own canonical sort.
///
/// Default per-scenario configs are deliberately small: warmup=1s +
/// duration=5s per scenario × 10 scenarios ≈ 60s total, matching CONTEXT.md
/// "small, fast — finishes in ~60s".
fn default_scenarios(seed: u64) -> Vec<(&'static str, Option<String>, ScenarioBuilder)> {
    // IN-04 (Phase-2 review): scenario types are imported at the module
    // level (see top of file). The duplicate `use` block that lived
    // here was redundant and read as forgotten cleanup. All names
    // resolve via the module-level import.

    vec![
        // 1. Multithread (SCEN-01, Phase-1 baseline) — 4 threads × 10k objects.
        (
            "multithread",
            None,
            Box::new(move || {
                let cfg = MultithreadConfig {
                    threads: 4,
                    objects: 10_000,
                    size_dist: SizeDist::Uniform,
                    size_min: 16,
                    size_max: 1024,
                    seed,
                }
                .validated()?;
                Ok(Box::new(Multithread::new(cfg)) as Box<dyn alloc_bench_core::Scenario>)
            }) as ScenarioBuilder,
        ),
        // 2. SPMC (SCEN-03) — 1 producer, 4 consumers race for messages.
        //    WR-06: validated_for(ChannelKind::Spmc) enforces producers==1.
        (
            "spmc",
            Some("iters_per_s".to_string()),
            Box::new(move || {
                let cfg = ChannelConfig {
                    producers: 1,
                    consumers: 4,
                    capacity: 1024,
                    objects_per_tick: 1_000,
                    payload_dist: PayloadDist::Uniform,
                    seed,
                }
                .validated_for(ChannelKind::Spmc)?;
                Ok(Box::new(Spmc::new(cfg)) as Box<dyn alloc_bench_core::Scenario>)
            }) as ScenarioBuilder,
        ),
        // 3. MPSC (SCEN-04) — 4 producers, 1 receiver.
        //    WR-06: validated_for(ChannelKind::Mpsc) enforces consumers==1.
        (
            "mpsc",
            Some("iters_per_s".to_string()),
            Box::new(move || {
                let cfg = ChannelConfig {
                    producers: 4,
                    consumers: 1,
                    capacity: 1024,
                    objects_per_tick: 1_000,
                    payload_dist: PayloadDist::Uniform,
                    seed,
                }
                .validated_for(ChannelKind::Mpsc)?;
                Ok(Box::new(Mpsc::new(cfg)) as Box<dyn alloc_bench_core::Scenario>)
            }) as ScenarioBuilder,
        ),
        // 4. MPMC (SCEN-05) — 4 producers × 4 consumers, both sides cloned.
        //    WR-06: validated_for(ChannelKind::Mpmc) is a no-op topology check.
        (
            "mpmc",
            Some("iters_per_s".to_string()),
            Box::new(move || {
                let cfg = ChannelConfig {
                    producers: 4,
                    consumers: 4,
                    capacity: 1024,
                    objects_per_tick: 1_000,
                    payload_dist: PayloadDist::Uniform,
                    seed,
                }
                .validated_for(ChannelKind::Mpmc)?;
                Ok(Box::new(Mpmc::new(cfg)) as Box<dyn alloc_bench_core::Scenario>)
            }) as ScenarioBuilder,
        ),
        // 5. Contention (SCEN-08) — 8 threads × 1k iters/tick. CONTEXT.md
        //    default of 64 threads + 10k iters is too heavy for the
        //    run-all smoke; trimmed for the ~60s budget.
        (
            "contention",
            None,
            Box::new(move || {
                let cfg = ContentionConfig {
                    threads: 8,
                    alloc_size: 64,
                    iters_per_tick: 1_000,
                    seed,
                }
                .validated()?;
                Ok(Box::new(Contention::new(cfg)) as Box<dyn alloc_bench_core::Scenario>)
            }) as ScenarioBuilder,
        ),
        // 6. Mem-bound (SCEN-07) — picks LinkedList because it's the
        //    alloc-heavy mode (per RESEARCH.md §Mem-bound: "linked-list
        //    is the alloc-heavy one"). StridedArray's pre-allocated
        //    buffer doesn't exercise the allocator on tick boundaries.
        (
            "mem-bound",
            None,
            Box::new(move || {
                let cfg = MemBoundConfig {
                    mode: MemBoundMode::LinkedList,
                    size_mb: 2,
                    seed,
                }
                .validated()?;
                Ok(Box::new(MemBound::new(cfg)) as Box<dyn alloc_bench_core::Scenario>)
            }) as ScenarioBuilder,
        ),
        // 7. Realloc-storm (SCEN-10) — 4MB target keeps tick latency
        //    well under HIST_MAX_NS even on slow CI.
        (
            "realloc-storm",
            None,
            Box::new(move || {
                let cfg = ReallocStormConfig {
                    target_size_mb: 4,
                    seed,
                }
                .validated()?;
                Ok(Box::new(ReallocStorm::new(cfg)) as Box<dyn alloc_bench_core::Scenario>)
            }) as ScenarioBuilder,
        ),
        // 8. CPU-bound (SCEN-06) — 2 threads × 2MB input. Default 64MB
        //    input + N threads is overkill for the smoke budget.
        (
            "cpu-bound",
            None,
            Box::new(move || {
                let cfg = CpuBoundConfig {
                    threads: 2,
                    input_size_mb: 2,
                    seed,
                }
                .validated()?;
                Ok(Box::new(CpuBound::new(cfg)) as Box<dyn alloc_bench_core::Scenario>)
            }) as ScenarioBuilder,
        ),
        // 9. Fragmentation-soak (SCEN-09) — 1k allocs/tick, cap 500.
        //    Default 5min duration is replaced by run-all's 5s measure.
        (
            "fragmentation-soak",
            None,
            Box::new(move || {
                let cfg = FragmentationConfig {
                    allocs_per_tick: 1_000,
                    long_lived_cap: 500,
                    seed,
                }
                .validated()?;
                Ok(Box::new(FragmentationSoak::new(cfg)) as Box<dyn alloc_bench_core::Scenario>)
            }) as ScenarioBuilder,
        ),
        // 10. Web (SCEN-02) — placed LAST so its port-bind + tokio
        //     runtime construction doesn't delay the other scenarios.
        //     1 server worker + 2 client workers keeps loopback HTTP
        //     latency low and predictable.
        (
            "web",
            Some("req_per_s".to_string()),
            Box::new(move || {
                let cfg = WebConfig {
                    server_workers: 1,
                    client_workers: 2,
                    seed,
                }
                .validated()?;
                Ok(Box::new(Web::new(cfg)) as Box<dyn alloc_bench_core::Scenario>)
            }) as ScenarioBuilder,
        ),
    ]
}

/// Extract a human-readable message from a panic payload. `panic::catch_unwind`
/// returns `Err(Box<dyn Any + Send>)` whose payload is typically the panic
/// argument: a `&'static str`, a `String`, or something else opaque.
///
/// WR-07 (Phase-2 review): the parameter is `&(dyn Any + Send)` rather
/// than `&Box<dyn Any + Send>` to avoid the `clippy::borrowed_box` smell
/// — `&Box<T>` is almost always replaceable with `&T` and the call site
/// just dereferences the `Box` via `&*panic` (or `panic.as_ref()`).
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "panic with non-string payload".to_string()
    }
}

/// Build a degenerate `Run` record for a scenario that errored or panicked.
/// Metric numerics are zeroed (the consumer reads `error` for the truth).
/// Env + Build are still populated so Phase-4 aggregator can attribute the
/// failure to the right host / commit / allocator.
fn degenerate_failure_run(name: &str, error: String) -> Result<Run> {
    use alloc_bench_core::output::{HarnessInfo, LatencyNs, Metrics, Rusage};
    let outcome = alloc_bench_core::HarnessOutcome {
        harness: HarnessInfo {
            warmup_duration_s: 0.0,
            measurement_duration_s: 0.0,
            samples_count: 0,
        },
        metrics: Metrics {
            ticks_per_s: 0.0,
            allocations_per_tick: 0,
            tick_latency_ns: LatencyNs {
                p50: 0,
                p95: 0,
                p99: 0,
                p999: 0,
                max: 0,
            },
            peak_rss_kb: 0,
            rss_growth_samples: vec![],
            rusage: Rusage {
                user_time_s: 0.0,
                sys_time_s: 0.0,
                minor_faults: 0,
                major_faults: 0,
                voluntary_csw: 0,
                involuntary_csw: 0,
                peak_rss_kb: 0,
            },
            allocator_stats: serde_json::json!({"kind": allocator::name()}),
        },
    };
    assemble_run(
        name,
        serde_json::json!({}),
        None,
        outcome,
        Some("failed".to_string()),
        Some(error),
    )
}

/// Phase-2 SCEN-11. Run all 10 scenarios sequentially under a uniform
/// HarnessConfig and emit a JSON array of Run records. Defaults come from
/// the CLI layer (`--warmup 5s --duration 60s`); this function honors
/// whatever is passed in. Per CONTEXT.md decision, per-scenario panics
/// are caught via `panic::catch_unwind(AssertUnwindSafe(...))` so the
/// other scenarios still produce records — the run-all binary exits 0
/// even when scenarios fail. The `error` field on each Run is the source
/// of truth for failure cases; consumers reading `status == "failed"`
/// MUST also read `error`.
pub fn run_all(output: Option<&str>, seed: u64, warmup: &str, duration: &str) -> Result<()> {
    let cfg = HarnessConfig {
        warmup: parse_duration(warmup)?,
        measure: parse_duration(duration)?,
        seed,
    };

    let mut runs: Vec<Run> = Vec::new();

    for (name, unit, builder) in default_scenarios(seed) {
        eprintln!("[run-all] starting scenario: {name}");
        // CONTEXT.md decision: continue on per-scenario failure. The
        // closure mutably borrows scenario state (the boxed scenario
        // built inside) so AssertUnwindSafe is required (RESEARCH.md
        // §A8). The double-Result pattern (`Ok(Ok(_)) | Ok(Err(_)) |
        // Err(panic)`) distinguishes "panicked" from "anyhow-errored"
        // so we can record both as `status: "failed"` with the right
        // error message.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<Run> {
            let mut scenario = builder()?;
            let outcome = run(&mut scenario, &cfg, allocator::stats)?;
            let scenario_name = alloc_bench_core::Scenario::name(&scenario).to_string();
            let scenario_config = alloc_bench_core::Scenario::config_json(&scenario);
            assemble_run(
                &scenario_name,
                scenario_config,
                unit.clone(),
                outcome,
                Some("success".to_string()),
                None,
            )
        }));

        match result {
            Ok(Ok(run)) => {
                eprintln!("[run-all]   {name}: success");
                runs.push(run);
            }
            Ok(Err(e)) => {
                eprintln!("[run-all]   {name}: error — {e}");
                runs.push(degenerate_failure_run(name, e.to_string())?);
            }
            Err(panic) => {
                // WR-07 (Phase-2 review): pass the unboxed dyn Any directly
                // by deref'ing the Box.
                let msg = panic_message(&*panic);
                eprintln!("[run-all]   {name}: panicked — {msg}");
                runs.push(degenerate_failure_run(name, msg)?);
            }
        }
    }

    let json = serde_json::to_string_pretty(&runs)?;
    match output {
        Some(path) => std::fs::write(path, &json)
            .with_context(|| format!("writing run-all results to {path}"))?,
        None => println!("{json}"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duration_units() {
        assert_eq!(parse_duration("5s").unwrap(), Duration::from_secs(5));
        assert_eq!(parse_duration("500ms").unwrap(), Duration::from_millis(500));
        assert_eq!(parse_duration("2m").unwrap(), Duration::from_secs(120));
        assert!(parse_duration("garbage").is_err());
        assert!(parse_duration("5x").is_err());
    }

    /// WR-09: pin the suffix-precedence invariant. If a maintainer reorders
    /// the strip_suffix branches so "s" runs before "ms", "5ms" silently
    /// becomes "5m → 300s" — this test catches that fast.
    #[test]
    fn parse_duration_ms_takes_precedence_over_m() {
        assert_eq!(parse_duration("5ms").unwrap(), Duration::from_millis(5));
        assert_eq!(parse_duration("5m").unwrap(), Duration::from_secs(300));
        assert_ne!(
            parse_duration("5ms").unwrap(),
            parse_duration("5m").unwrap()
        );
    }

    #[test]
    fn parse_duration_minute_overflow_is_caught() {
        // u64::MAX minutes would overflow when multiplied by 60. Without
        // the checked_mul guard this returned a tiny wrapped Duration.
        let s = format!("{}m", u64::MAX);
        let err = parse_duration(&s).unwrap_err();
        assert!(
            err.to_string().contains("duration too large"),
            "expected 'duration too large' error, got: {err}"
        );
    }
}

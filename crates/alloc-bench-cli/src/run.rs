use std::time::Duration;

use alloc_bench_core::metrics::env::read_env;
use alloc_bench_core::output::{Build, Run, ScenarioInfo};
use alloc_bench_core::scenarios::{
    ChannelConfig, Contention, ContentionConfig, CpuBound, CpuBoundConfig, FragmentationConfig,
    FragmentationSoak, MemBound, MemBoundConfig, MemBoundMode, Mpmc, Mpsc, Multithread,
    MultithreadConfig, PayloadDist, ReallocStorm, ReallocStormConfig, SizeDist, Spmc, Web,
    WebConfig,
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

    let env = read_env()?;
    let sha = build_info::GIT_SHA;
    let sha8 = &sha[..sha.len().min(8)];
    let run_id = format!("{}-{sha8}", chrono::Utc::now().to_rfc3339());

    let scenario_info = ScenarioInfo {
        name: name.to_string(),
        config: alloc_bench_core::Scenario::config_json(scenario),
        // Phase-2 additive schema field. `None` is skipped on serialize so
        // existing Phase-1 JSON shapes stay byte-identical.
        unit,
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

    let run_record = Run {
        schema_version: SCHEMA_VERSION,
        run_id,
        env,
        build,
        scenario: scenario_info,
        harness: outcome.harness,
        metrics: outcome.metrics,
    };

    let json = serde_json::to_string_pretty(&run_record)?;
    match output {
        Some(path) => {
            std::fs::write(path, &json).with_context(|| format!("writing results to {path}"))?
        }
        None => println!("{json}"),
    }
    Ok(())
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
    // topologies, so we enforce the SP invariant here at the CLI surface.
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
    .validated()?;
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
    .validated()?;
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
    .validated()?;
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

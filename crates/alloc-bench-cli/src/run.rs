use std::time::Duration;

use alloc_bench_core::metrics::env::read_env;
use alloc_bench_core::output::{Build, Run, ScenarioInfo};
use alloc_bench_core::scenarios::{Multithread, MultithreadConfig, SizeDist};
use alloc_bench_core::{run, HarnessConfig, SCHEMA_VERSION};
use anyhow::{anyhow, Context, Result};

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

    let cfg = HarnessConfig {
        warmup,
        measure,
        seed,
    };
    let outcome = run(&mut scenario, &cfg, allocator::stats)?;

    let env = read_env()?;
    let sha = build_info::GIT_SHA;
    let sha8 = &sha[..sha.len().min(8)];
    let run_id = format!("{}-{sha8}", chrono::Utc::now().to_rfc3339());

    let scenario_info = ScenarioInfo {
        name: "multithread".to_string(),
        config: alloc_bench_core::Scenario::config_json(&scenario),
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
        assert_ne!(parse_duration("5ms").unwrap(), parse_duration("5m").unwrap());
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

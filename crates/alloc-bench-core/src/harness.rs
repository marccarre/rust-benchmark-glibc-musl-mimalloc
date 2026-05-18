use std::time::{Duration, Instant};

use anyhow::bail;
use hdrhistogram::Histogram;

use crate::metrics::{rusage::read_rusage, statm::read_rss_kb};
use crate::output::{HarnessInfo, LatencyNs, Metrics, RssGrowthSample};

pub trait SinkValue {}
impl<T: ?Sized + 'static> SinkValue for T {}

pub trait Scenario {
    fn name(&self) -> &'static str;
    fn config_json(&self) -> serde_json::Value;
    fn setup(&mut self) -> anyhow::Result<()>;
    fn tick(&mut self) -> Box<dyn SinkValue>;
    fn teardown(&mut self) {}
    /// Number of allocations performed per `tick()` invocation.
    /// WR-01: surfaced into `Metrics::allocations_per_tick` so consumers can
    /// derive the allocation rate as `ticks_per_s * allocations_per_tick`.
    fn allocations_per_tick(&self) -> u64;
}

/// Allow `Box<dyn Scenario>` to be driven by the generic `run<S: Scenario>`
/// signature. Phase-2 `run_all` (SCEN-11) builds a registry of
/// `Box<dyn Scenario>` so adding a new scenario only touches the registry —
/// it is the canonical Phase-2 dispatch shape per RESEARCH.md §Run-all.
///
/// Without this delegation impl, `&mut dyn Scenario` cannot satisfy
/// `S: Scenario` (because `dyn Scenario: !Sized`); but `Box<dyn Scenario>`
/// is `Sized`, so `Box<dyn Scenario>: Scenario` lets us pass
/// `&mut boxed_scenario` (where `boxed_scenario: Box<dyn Scenario>`)
/// straight to `run`.
impl Scenario for Box<dyn Scenario> {
    fn name(&self) -> &'static str {
        (**self).name()
    }
    fn config_json(&self) -> serde_json::Value {
        (**self).config_json()
    }
    fn setup(&mut self) -> anyhow::Result<()> {
        (**self).setup()
    }
    fn tick(&mut self) -> Box<dyn SinkValue> {
        (**self).tick()
    }
    fn teardown(&mut self) {
        (**self).teardown()
    }
    fn allocations_per_tick(&self) -> u64 {
        (**self).allocations_per_tick()
    }
}

pub struct HarnessConfig {
    pub warmup: Duration,
    pub measure: Duration,
    pub seed: u64,
}

#[derive(Debug)]
pub struct HarnessOutcome {
    pub harness: HarnessInfo,
    pub metrics: Metrics,
}

pub fn run<S: Scenario, F: Fn() -> serde_json::Value>(
    scenario: &mut S,
    cfg: &HarnessConfig,
    alloc_stats: F,
) -> anyhow::Result<HarnessOutcome> {
    if cfg.warmup < Duration::from_secs(1) {
        bail!("warm-up must be >= 1s; allocator caches need to populate (see PITFALLS.md §1.5)");
    }

    scenario.setup()?;

    // Warm-up phase — no measurement
    let warmup_end = Instant::now() + cfg.warmup;
    while Instant::now() < warmup_end {
        std::hint::black_box(scenario.tick());
    }
    let warmup_actual_s = cfg.warmup.as_secs_f64();

    // Measurement phase
    // WR-04: HDR ceiling. Per-tick fork/join can plausibly exceed 60s on
    // saturated CI runners or heavy `--threads --objects` configurations;
    // 5min head-room keeps us recording even when slow, and we still
    // saturate (clip) any sample above HIST_MAX_NS rather than aborting
    // the run mid-measurement (`hist.record(...)?` would otherwise return
    // an error on out-of-range values).
    const HIST_MAX_NS: u64 = 300_000_000_000;
    let mut hist = Histogram::<u64>::new_with_bounds(1, HIST_MAX_NS, 3)?;
    let mut rss_samples: Vec<RssGrowthSample> = Vec::new();
    let measure_start = Instant::now();
    let measure_end = measure_start + cfg.measure;
    let mut last_rss_t = measure_start;
    let mut t_s: u64 = 0;

    while Instant::now() < measure_end {
        let t0 = Instant::now();
        std::hint::black_box(scenario.tick());
        let elapsed_ns = t0.elapsed().as_nanos() as u64;
        // WR-04: saturating clip. We prefer a slightly-truncated tail to a
        // mid-run abort that loses everything collected so far.
        let clipped = elapsed_ns.clamp(1, HIST_MAX_NS);
        hist.record(clipped)?;

        if last_rss_t.elapsed() >= Duration::from_secs(1) {
            let rss_kb = read_rss_kb().unwrap_or(0);
            rss_samples.push(RssGrowthSample { t_s, rss_kb });
            last_rss_t = Instant::now();
            t_s += 1;
        }
    }

    let measurement_s = measure_start.elapsed().as_secs_f64();
    let samples_count = hist.len();
    // WR-01: this is the rate of *ticks*, not of allocations. Multiply by
    // `allocations_per_tick` to derive the allocation rate.
    let ticks_per_s = samples_count as f64 / measurement_s;
    let allocations_per_tick = scenario.allocations_per_tick();

    scenario.teardown();

    let rusage = read_rusage()?;
    let peak_rss_kb = rusage.peak_rss_kb;
    let alloc_stats_val = alloc_stats();

    let metrics = Metrics {
        ticks_per_s,
        allocations_per_tick,
        tick_latency_ns: LatencyNs {
            p50: hist.value_at_quantile(0.50),
            p95: hist.value_at_quantile(0.95),
            p99: hist.value_at_quantile(0.99),
            p999: hist.value_at_quantile(0.999),
            max: hist.max(),
        },
        peak_rss_kb,
        rss_growth_samples: rss_samples,
        rusage,
        allocator_stats: alloc_stats_val,
    };

    let harness = HarnessInfo {
        warmup_duration_s: warmup_actual_s,
        measurement_duration_s: measurement_s,
        samples_count,
    };

    Ok(HarnessOutcome { harness, metrics })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyScenario;
    impl Scenario for DummyScenario {
        fn name(&self) -> &'static str {
            "dummy"
        }
        fn config_json(&self) -> serde_json::Value {
            serde_json::json!({})
        }
        fn setup(&mut self) -> anyhow::Result<()> {
            Ok(())
        }
        fn tick(&mut self) -> Box<dyn SinkValue> {
            Box::new(vec![0u8; 64])
        }
        fn allocations_per_tick(&self) -> u64 {
            1
        }
    }

    #[test]
    fn warmup_too_short_returns_error() {
        let mut s = DummyScenario;
        let cfg = HarnessConfig {
            warmup: Duration::from_millis(500),
            measure: Duration::from_secs(1),
            seed: 0,
        };
        let err = run(&mut s, &cfg, || serde_json::json!({"kind":"system"})).unwrap_err();
        assert!(
            err.to_string().contains("warm-up must be >= 1s"),
            "unexpected error: {err}"
        );
    }
}

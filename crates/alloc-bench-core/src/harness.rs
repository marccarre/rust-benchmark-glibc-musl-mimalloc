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
    let mut hist = Histogram::<u64>::new_with_bounds(1, 60_000_000_000, 3)?;
    let mut rss_samples: Vec<RssGrowthSample> = Vec::new();
    let measure_start = Instant::now();
    let measure_end = measure_start + cfg.measure;
    let mut last_rss_t = measure_start;
    let mut t_s: u64 = 0;

    while Instant::now() < measure_end {
        let t0 = Instant::now();
        std::hint::black_box(scenario.tick());
        let elapsed_ns = t0.elapsed().as_nanos() as u64;
        hist.record(elapsed_ns.max(1))?;

        if last_rss_t.elapsed() >= Duration::from_secs(1) {
            let rss_kb = read_rss_kb().unwrap_or(0);
            rss_samples.push(RssGrowthSample { t_s, rss_kb });
            last_rss_t = Instant::now();
            t_s += 1;
        }
    }

    let measurement_s = measure_start.elapsed().as_secs_f64();
    let samples_count = hist.len();
    let throughput_ops_per_s = samples_count as f64 / measurement_s;

    scenario.teardown();

    let rusage = read_rusage()?;
    let peak_rss_kb = rusage.peak_rss_kb;
    let alloc_stats_val = alloc_stats();

    let metrics = Metrics {
        throughput_ops_per_s,
        latency_ns: LatencyNs {
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

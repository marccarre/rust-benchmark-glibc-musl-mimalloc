use std::str::FromStr;

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use serde::Serialize;

use crate::harness::{Scenario, SinkValue};

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SizeDist {
    Uniform,
    Bimodal,
    Pareto,
}

impl FromStr for SizeDist {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "uniform" => Ok(SizeDist::Uniform),
            "bimodal" => Ok(SizeDist::Bimodal),
            "pareto" => Ok(SizeDist::Pareto),
            other => anyhow::bail!("unknown size_dist: {other} (expected uniform|bimodal|pareto)"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MultithreadConfig {
    pub threads: usize,
    pub objects: usize,
    pub size_dist: SizeDist,
    pub size_min: usize,
    pub size_max: usize,
    pub seed: u64,
}

pub struct Multithread {
    cfg: MultithreadConfig,
}

impl Multithread {
    pub fn new(cfg: MultithreadConfig) -> Self {
        Self { cfg }
    }
}

fn sample_size(rng: &mut SmallRng, dist: SizeDist, min: usize, max: usize) -> usize {
    match dist {
        SizeDist::Uniform => rng.gen_range(min..=max),
        SizeDist::Bimodal => {
            // CR-01: Respect [min, max] bounds. Any sane-minimum guard belongs in
            // MultithreadConfig::validated, not silently rewritten in the hot path.
            if rng.gen::<f32>() < 0.9 {
                min
            } else {
                max
            }
        }
        SizeDist::Pareto => {
            let alpha = 1.5_f64;
            let u: f64 = rng.gen_range(0.0_f64..1.0_f64);
            let raw = (min as f64) * (1.0_f64 - u).powf(-1.0 / alpha);
            (raw.round() as usize).clamp(min, max)
        }
    }
}

impl Scenario for Multithread {
    fn name(&self) -> &'static str {
        "multithread"
    }

    fn config_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.cfg).unwrap_or_else(|_| serde_json::json!({}))
    }

    fn setup(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn tick(&mut self) -> Box<dyn SinkValue> {
        let cfg = self.cfg.clone();
        let mut all: Vec<Vec<Box<[u8]>>> = Vec::with_capacity(cfg.threads);

        std::thread::scope(|scope| {
            let mut handles = Vec::with_capacity(cfg.threads);
            for t in 0..cfg.threads {
                let cfg = cfg.clone();
                handles.push(scope.spawn(move || {
                    let mut rng = SmallRng::seed_from_u64(cfg.seed.wrapping_add(t as u64));
                    let mut bag: Vec<Box<[u8]>> = Vec::with_capacity(cfg.objects);
                    for _ in 0..cfg.objects {
                        let size = sample_size(&mut rng, cfg.size_dist, cfg.size_min, cfg.size_max);
                        let mut b: Box<[u8]> = vec![0u8; size].into_boxed_slice();
                        // PITFALLS.md §1.2: write to mid-buffer to defeat
                        // optimizer-driven allocation elision.
                        b[size / 2] = 0xAB;
                        bag.push(std::hint::black_box(b));
                    }
                    bag
                }));
            }
            for h in handles {
                // CR-02: propagate worker panics so the harness fails loudly
                // instead of recording bogus throughput from a partial run.
                match h.join() {
                    Ok(bag) => all.push(bag),
                    Err(payload) => std::panic::resume_unwind(payload),
                }
            }
        });

        Box::new(std::hint::black_box(all))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_dist_from_str() {
        assert!(matches!(
            "uniform".parse::<SizeDist>(),
            Ok(SizeDist::Uniform)
        ));
        assert!(matches!(
            "bimodal".parse::<SizeDist>(),
            Ok(SizeDist::Bimodal)
        ));
        assert!(matches!("pareto".parse::<SizeDist>(), Ok(SizeDist::Pareto)));
        assert!("xyz".parse::<SizeDist>().is_err());
    }

    #[test]
    fn sample_size_within_bounds() {
        let mut rng = SmallRng::seed_from_u64(42);
        for _ in 0..100 {
            let s = sample_size(&mut rng, SizeDist::Uniform, 16, 1024);
            assert!((16..=1024).contains(&s));
        }
    }
}

//! Realloc-storm scenario (SCEN-10) — `Vec` growth from capacity 0 every tick.
//!
//! Per CONTEXT.md decision: each tick starts with `Vec::with_capacity(0)`
//! and pushes bytes until length = `target_size_mb * 1024 * 1024`. This
//! exercises the standard `Vec` doubling growth strategy from scratch every
//! tick — every `tick()` triggers `log2(target_bytes)` reallocations plus
//! a single drop of the entire buffer at the end.
//!
//! Allocations per tick (approximation): the doubling-growth strategy
//! produces one realloc per power-of-two boundary. For a target of
//! `target_bytes` bytes this is `ceil(log2(target_bytes))` realloc calls
//! plus the final allocation that holds the full payload. We surface that
//! count via `allocations_per_tick()` so the Phase-4 aggregator can derive
//! the realloc rate as `ticks_per_s * allocations_per_tick`.
//!
//! Upper bound: not enforced. CONTEXT.md notes the user is responsible
//! for not pushing more than host RAM. The scenario will OOM gracefully
//! (process abort) if pushed beyond available memory — a benign failure
//! mode for a benchmark tool.

use serde::Serialize;

use crate::harness::{Scenario, SinkValue};

#[derive(Debug, Clone, Serialize)]
pub struct ReallocStormConfig {
    pub target_size_mb: usize,
    pub seed: u64,
}

impl ReallocStormConfig {
    pub fn validated(self) -> anyhow::Result<Self> {
        anyhow::ensure!(
            self.target_size_mb >= 1,
            "target_size_mb must be >= 1 (got {})",
            self.target_size_mb
        );
        // Upper bound left to the user — pushing more than host RAM is
        // their problem (CONTEXT.md decision).
        Ok(self)
    }
}

pub struct ReallocStorm {
    cfg: ReallocStormConfig,
}

impl ReallocStorm {
    pub fn new(cfg: ReallocStormConfig) -> Self {
        Self { cfg }
    }
}

impl Scenario for ReallocStorm {
    fn name(&self) -> &'static str {
        "realloc-storm"
    }

    fn config_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.cfg).unwrap_or_else(|_| serde_json::json!({}))
    }

    fn setup(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn allocations_per_tick(&self) -> u64 {
        // Approximate doubling-growth realloc count: log2(target_bytes).
        // Implementation: count trailing zeros of the next power of two of
        // target_bytes — for non-power-of-two values this gives the smallest
        // exponent k such that 2^k >= target_bytes, i.e. number of doubling
        // steps the Vec performs. For target_size_mb=64 this is 26.
        let target_bytes = (self.cfg.target_size_mb as u64) * 1024 * 1024;
        target_bytes.next_power_of_two().trailing_zeros() as u64
    }

    fn tick(&mut self) -> Box<dyn SinkValue> {
        let target_bytes = self.cfg.target_size_mb * 1024 * 1024;
        // CONTEXT.md decision: test growth from scratch every tick.
        let mut v: Vec<u8> = Vec::with_capacity(0);
        for i in 0..target_bytes {
            v.push((i & 0xFF) as u8);
        }
        // Mid-buffer read defeats DCE on the entire grow loop.
        std::hint::black_box(v[v.len() / 2]);
        // Drop on return → frees the entire buffer at once, exercising
        // the allocator's free path for one large block.
        Box::new(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(target_size_mb: usize) -> ReallocStormConfig {
        ReallocStormConfig {
            target_size_mb,
            seed: 1,
        }
    }

    #[test]
    fn validated_rejects_zero_target_size_mb() {
        let err = cfg(0).validated().unwrap_err();
        assert!(err.to_string().contains("target_size_mb must be >= 1"));
    }

    #[test]
    fn validated_accepts_well_formed_config() {
        assert!(cfg(64).validated().is_ok());
    }

    #[test]
    fn allocations_per_tick_log2_formula_lower_bound() {
        // 64MB → log2(64 * 2^20) = log2(2^26) = 26.
        let s = ReallocStorm::new(cfg(64));
        assert!(
            s.allocations_per_tick() >= 20,
            "expected >= 20 realloc steps for 64MB target, got {}",
            s.allocations_per_tick()
        );
        assert_eq!(s.allocations_per_tick(), 26);
    }

    #[test]
    fn allocations_per_tick_smaller_target() {
        // 1MB → log2(2^20) = 20.
        let s = ReallocStorm::new(cfg(1));
        assert_eq!(s.allocations_per_tick(), 20);
    }

    #[test]
    fn tick_smoke_does_not_panic() {
        // Use the smallest valid target — 1MB = 1_048_576 push iterations,
        // fast enough for a unit test.
        let c = cfg(1).validated().unwrap();
        let mut s = ReallocStorm::new(c);
        let _ = s.tick();
    }
}

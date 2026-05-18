//! Fragmentation-soak scenario (SCEN-09) — long-running mixed alloc/free
//! workload with state across ticks via `&mut self`.
//!
//! Per CONTEXT.md and RESEARCH.md §Fragmentation-soak the workload mixes:
//!  * **90% short-lived 16-byte buffers** — allocated and dropped within
//!    one tick (the implicit `Vec<Box<[u8]>>` returned by `tick()` drops
//!    on return).
//!  * **10% long-lived 4KB buffers** — pushed into `self.long_lived` and
//!    held across ticks. This is the *only* cross-tick mutable state in
//!    the benchmark suite so far; it's allowed because `Scenario::tick`
//!    takes `&mut self`.
//!
//! **CRITICAL (RESEARCH.md §Pitfall 8): the long-lived Vec is capped at
//! `long_lived_cap` entries with random eviction.** Without the cap, a
//! 5min soak with allocs_per_tick=10_000 at ~1000 ticks/s would push 6M
//! long-lived 4KB blocks ≈ 24GB. The cap of 10_000 holds peak_rss for the
//! long-lived state at ~40MB max regardless of duration.
//!
//! The fragmentation pressure comes from interleaving the short- and
//! long-lived size classes — the allocator's 16-byte slab and 4KB
//! large-page paths fight for the same arenas, and the survivors of
//! eviction force re-coalescing on each new long-lived push.

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use serde::Serialize;

use crate::harness::{Scenario, SinkValue};

#[derive(Debug, Clone, Serialize)]
pub struct FragmentationConfig {
    pub allocs_per_tick: u64,
    pub long_lived_cap: usize,
    pub seed: u64,
}

impl FragmentationConfig {
    pub fn validated(self) -> anyhow::Result<Self> {
        anyhow::ensure!(
            self.allocs_per_tick >= 1,
            "allocs_per_tick must be >= 1 (got {})",
            self.allocs_per_tick
        );
        anyhow::ensure!(
            self.long_lived_cap >= 1,
            "long_lived_cap must be >= 1 (got {})",
            self.long_lived_cap
        );
        Ok(self)
    }
}

pub struct FragmentationSoak {
    cfg: FragmentationConfig,
    long_lived: Vec<Box<[u8]>>,
    rng: SmallRng,
}

impl FragmentationSoak {
    pub fn new(cfg: FragmentationConfig) -> Self {
        let rng = SmallRng::seed_from_u64(cfg.seed);
        // Reserve up-front so the long_lived growth doesn't itself cause
        // benchmark noise via Vec reallocations.
        let cap = cfg.long_lived_cap;
        Self {
            cfg,
            long_lived: Vec::with_capacity(cap),
            rng,
        }
    }

    /// Test-only accessor for the long-lived bag length. Used by the
    /// cap-enforcement test to verify the eviction logic.
    #[cfg(test)]
    fn long_lived_len(&self) -> usize {
        self.long_lived.len()
    }
}

impl Scenario for FragmentationSoak {
    fn name(&self) -> &'static str {
        "fragmentation-soak"
    }

    fn config_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.cfg).unwrap_or_else(|_| serde_json::json!({}))
    }

    fn setup(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn allocations_per_tick(&self) -> u64 {
        self.cfg.allocs_per_tick
    }

    fn tick(&mut self) -> Box<dyn SinkValue> {
        let mut short: Vec<Box<[u8]>> = Vec::with_capacity(self.cfg.allocs_per_tick as usize);
        for _ in 0..self.cfg.allocs_per_tick {
            if self.rng.gen::<f32>() < 0.9 {
                // 90% — short-lived 16-byte buffer.
                let mut b = vec![0u8; 16].into_boxed_slice();
                // Mid-buffer write defeats DCE.
                b[8] = self.rng.gen::<u8>();
                short.push(b);
            } else {
                // 10% — long-lived 4KB buffer.
                let mut b = vec![0u8; 4096].into_boxed_slice();
                // Mid-buffer write defeats DCE.
                b[2048] = self.rng.gen::<u8>();
                // RESEARCH.md §Pitfall 8: cap long_lived to prevent
                // unbounded growth on long soaks. swap_remove is O(1)
                // and the eviction order doesn't matter for fragmentation
                // pressure (we just need a slot to free).
                if self.long_lived.len() >= self.cfg.long_lived_cap {
                    let evict_idx = self.rng.gen_range(0..self.long_lived.len());
                    self.long_lived.swap_remove(evict_idx);
                }
                self.long_lived.push(b);
            }
        }
        // Defeat DCE on the long-lived collection (without this, LLVM
        // could in theory prove the long-lived branch unobservable across
        // ticks).
        std::hint::black_box(&self.long_lived);
        // `short` drops at return → frees all 90% short-lived buffers.
        // `self.long_lived` persists for the next tick.
        Box::new(short)
    }

    fn teardown(&mut self) {
        // Explicit drop of the long-lived bag at end-of-run so the rusage
        // tail records the freeing work too.
        self.long_lived.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(allocs_per_tick: u64, long_lived_cap: usize) -> FragmentationConfig {
        FragmentationConfig {
            allocs_per_tick,
            long_lived_cap,
            seed: 1,
        }
    }

    #[test]
    fn validated_rejects_zero_allocs_per_tick() {
        let err = cfg(0, 100).validated().unwrap_err();
        assert!(err.to_string().contains("allocs_per_tick must be >= 1"));
    }

    #[test]
    fn validated_rejects_zero_long_lived_cap() {
        let err = cfg(1000, 0).validated().unwrap_err();
        assert!(err.to_string().contains("long_lived_cap must be >= 1"));
    }

    #[test]
    fn validated_accepts_well_formed_config() {
        assert!(cfg(10_000, 10_000).validated().is_ok());
    }

    #[test]
    fn long_lived_cap_is_enforced_across_ticks() {
        // Force lots of long-lived pushes (allocs_per_tick=10_000 ×
        // 10% long-lived ≈ 1000 long-lived per tick). cap=100 forces
        // eviction every tick after the first 100 long-lived pushes.
        let c = cfg(10_000, 100).validated().unwrap();
        let mut s = FragmentationSoak::new(c);
        for _ in 0..50 {
            let _ = s.tick();
            assert!(
                s.long_lived_len() <= 100,
                "long_lived breached cap=100, len={}",
                s.long_lived_len()
            );
        }
        // WR-11 (Phase-2 review): after many ticks we expect the cap to
        // be saturated. The probability of fewer than 100 long-lived
        // pushes across 50 ticks of allocs_per_tick=10_000 is
        // ≈ (0.9)^(50 * 10_000) ≈ 10^-22906 — astronomically small for
        // a well-formed PRNG. If this assertion ever flakes, SmallRng
        // produced a degenerate sequence for seed=1 (rotate the seed
        // in `cfg(allocs_per_tick, long_lived_cap)` before assuming the
        // cap-enforcement logic regressed). assert_eq! gives a clearer
        // diff than `assert!(... == ...)` on failure.
        assert_eq!(
            s.long_lived_len(),
            100,
            "expected long_lived to reach cap of 100; if not, SmallRng \
             sequence for seed=1 degenerated — try a different seed before \
             assuming the eviction logic regressed"
        );
    }

    #[test]
    fn tick_smoke_does_not_panic() {
        let c = cfg(100, 50).validated().unwrap();
        let mut s = FragmentationSoak::new(c);
        let _ = s.tick();
    }

    #[test]
    fn teardown_clears_long_lived() {
        let c = cfg(10_000, 100).validated().unwrap();
        let mut s = FragmentationSoak::new(c);
        for _ in 0..5 {
            let _ = s.tick();
        }
        assert!(s.long_lived_len() > 0, "expected non-empty long_lived");
        s.teardown();
        assert_eq!(s.long_lived_len(), 0);
    }

    #[test]
    fn allocations_per_tick_matches_config() {
        let s = FragmentationSoak::new(cfg(12345, 100));
        assert_eq!(s.allocations_per_tick(), 12345);
    }
}

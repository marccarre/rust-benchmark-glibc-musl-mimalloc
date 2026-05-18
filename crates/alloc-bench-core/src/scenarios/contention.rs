//! Contention scenario (SCEN-08) — high-thread-count tight alloc/free loop.
//!
//! Per CONTEXT.md and RESEARCH.md §Pitfall 7: this scenario's defining
//! property is that every allocated buffer is **dropped before the next
//! iteration**. If buffers were accumulated into a Vec, RSS would grow
//! linearly and we'd be measuring sustained alloc rate rather than
//! alloc/free contention. The doc-comment on `tick()` documents this
//! invariant; do not "optimise" by hoisting allocations.

use serde::Serialize;

use crate::harness::{Scenario, SinkValue};

#[derive(Debug, Clone, Serialize)]
pub struct ContentionConfig {
    pub threads: usize,
    pub alloc_size: usize,
    pub iters_per_tick: u64,
    pub seed: u64,
}

impl ContentionConfig {
    /// Reject malformed configs at construction time so worker threads stay
    /// panic-free.
    pub fn validated(self) -> anyhow::Result<Self> {
        anyhow::ensure!(
            self.threads >= 1,
            "threads must be >= 1 (got {})",
            self.threads
        );
        anyhow::ensure!(
            self.alloc_size >= 1,
            "alloc_size must be >= 1 (got {})",
            self.alloc_size
        );
        anyhow::ensure!(
            self.iters_per_tick >= 1,
            "iters_per_tick must be >= 1 (got {})",
            self.iters_per_tick
        );
        Ok(self)
    }
}

pub struct Contention {
    cfg: ContentionConfig,
}

impl Contention {
    pub fn new(cfg: ContentionConfig) -> Self {
        Self { cfg }
    }
}

impl Scenario for Contention {
    fn name(&self) -> &'static str {
        "contention"
    }

    fn config_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.cfg).unwrap_or_else(|_| serde_json::json!({}))
    }

    fn setup(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn allocations_per_tick(&self) -> u64 {
        // threads * iters_per_tick allocations per tick — each worker performs
        // one alloc + one drop per inner-loop iteration.
        (self.cfg.threads as u64).saturating_mul(self.cfg.iters_per_tick)
    }

    /// Each worker runs a tight alloc/drop loop:
    ///
    /// ```ignore
    /// for _ in 0..iters_per_tick {
    ///     let b: Box<[u8]> = vec![0u8; alloc_size].into_boxed_slice();
    ///     std::hint::black_box(&b);
    ///     count += b[alloc_size / 2] as u64;
    ///     // implicit drop(b) at end of loop body — DO NOT push to a Vec.
    /// }
    /// ```
    ///
    /// **Invariant:** the box MUST drop on every iteration. If a maintainer
    /// "optimises" by accumulating into a Vec the test silently flips into
    /// a memory-growth benchmark — the alloc/free contention path is the
    /// whole point. RESEARCH.md §Pitfall 7 spells this out.
    fn tick(&mut self) -> Box<dyn SinkValue> {
        let cfg = self.cfg.clone();
        std::thread::scope(|scope| {
            let mut handles = Vec::with_capacity(cfg.threads);
            for _ in 0..cfg.threads {
                let cfg = cfg.clone();
                handles.push(scope.spawn(move || {
                    let mut count = 0u64;
                    for _ in 0..cfg.iters_per_tick {
                        let b: Box<[u8]> = vec![0u8; cfg.alloc_size].into_boxed_slice();
                        // Defeat DCE: black_box the alloc, mid-buffer read.
                        std::hint::black_box(&b);
                        count = count.wrapping_add(b[cfg.alloc_size / 2] as u64);
                        // implicit drop(b) at end of loop body — see invariant above.
                    }
                    count
                }));
            }
            let mut total = 0u64;
            for h in handles {
                // Phase-1 CR-02: propagate worker panics so the harness
                // fails loudly rather than recording bogus throughput.
                match h.join() {
                    Ok(c) => total = total.wrapping_add(c),
                    Err(payload) => std::panic::resume_unwind(payload),
                }
            }
            std::hint::black_box(total);
        });
        Box::new(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(threads: usize, alloc_size: usize, iters_per_tick: u64) -> ContentionConfig {
        ContentionConfig {
            threads,
            alloc_size,
            iters_per_tick,
            seed: 1,
        }
    }

    #[test]
    fn validated_rejects_zero_threads() {
        let err = cfg(0, 64, 100).validated().unwrap_err();
        assert!(err.to_string().contains("threads must be >= 1"));
    }

    #[test]
    fn validated_rejects_zero_alloc_size() {
        let err = cfg(1, 0, 100).validated().unwrap_err();
        assert!(err.to_string().contains("alloc_size must be >= 1"));
    }

    #[test]
    fn validated_rejects_zero_iters_per_tick() {
        let err = cfg(1, 64, 0).validated().unwrap_err();
        assert!(err.to_string().contains("iters_per_tick must be >= 1"));
    }

    #[test]
    fn validated_accepts_well_formed_config() {
        assert!(cfg(8, 64, 1000).validated().is_ok());
    }

    #[test]
    fn tick_smoke_does_not_panic() {
        let c = cfg(4, 64, 100).validated().unwrap();
        let mut s = Contention::new(c);
        let _ = s.tick();
    }

    #[test]
    fn allocations_per_tick_is_threads_times_iters() {
        let c = cfg(4, 64, 100).validated().unwrap();
        let s = Contention::new(c);
        assert_eq!(s.allocations_per_tick(), 400);
    }
}

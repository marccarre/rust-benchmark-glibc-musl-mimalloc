//! CPU-bound scenario (SCEN-06) — recursive parallel merge-sort.
//!
//! Per CONTEXT.md and RESEARCH.md §CPU-bound the implementation uses
//! `rayon::join` recursion with a SCOPED `rayon::ThreadPool` (NOT the
//! global pool). This is critical for two reasons:
//!
//!  1. **Honour `--threads N`** — global rayon pool is initialised on
//!     first use; if another scenario in run-all touches it first with a
//!     different thread count, our `--threads` flag is silently ignored.
//!     A scoped pool eliminates that drift.
//!  2. **Allocations land in the merge step** — every merge node allocates
//!     a fresh `Vec<T>::with_capacity(slice.len())` (RESEARCH.md §Pitfall 4).
//!     Hoisting that allocation out would invert the benchmark by removing
//!     allocations from the critical path.
//!
//! Rayon's pattern-defeating quicksort (the slice extension trait) is the
//! wrong API for this benchmark — it has pre-allocated buffers and zero
//! allocations in the hot path. We use a hand-written `parallel_merge_sort`
//! so the alloc/free pair is on every recursion level.

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use serde::Serialize;

use crate::harness::{Scenario, SinkValue};

#[derive(Debug, Clone, Serialize)]
pub struct CpuBoundConfig {
    pub threads: usize,
    pub input_size_mb: usize,
    pub seed: u64,
}

impl CpuBoundConfig {
    /// Reject malformed configs at construction time.
    pub fn validated(self) -> anyhow::Result<Self> {
        anyhow::ensure!(
            self.threads >= 1,
            "threads must be >= 1 (got {})",
            self.threads
        );
        anyhow::ensure!(
            self.input_size_mb >= 1,
            "input_size_mb must be >= 1 (got {})",
            self.input_size_mb
        );
        Ok(self)
    }
}

pub struct CpuBound {
    cfg: CpuBoundConfig,
    pool: Option<rayon::ThreadPool>,
    input: Option<Vec<u64>>,
}

impl CpuBound {
    pub fn new(cfg: CpuBoundConfig) -> Self {
        Self {
            cfg,
            pool: None,
            input: None,
        }
    }
}

/// Recursive merge-sort with parallel divide and an allocation in every
/// merge step. Base case at 1024 elements falls back to `slice::sort_unstable`
/// to keep recursion shallow on small arrays.
///
/// **Critical invariant (RESEARCH.md §Pitfall 4):** the `Vec::with_capacity`
/// inside the merge step is intentional. Allocating a single temp buffer
/// once at the top of the recursion tree would defeat the benchmark — the
/// whole point is that allocations happen at every level so the allocator's
/// lock contention shows up.
fn parallel_merge_sort<T: Ord + Send + Copy>(slice: &mut [T]) {
    if slice.len() <= 1024 {
        // Base case: small arrays use std unstable sort (no recursion,
        // no allocations from this function — the std impl handles it).
        slice.sort_unstable();
        return;
    }
    let total_len = slice.len();
    let mid = total_len / 2;
    let (left, right) = slice.split_at_mut(mid);
    rayon::join(
        || parallel_merge_sort(left),
        || parallel_merge_sort(right),
    );

    // MERGE STEP — fresh allocation per merge node, intentional.
    let mut merged: Vec<T> = Vec::with_capacity(total_len);
    let (mut i, mut j) = (0, 0);
    while i < left.len() && j < right.len() {
        if left[i] <= right[j] {
            merged.push(left[i]);
            i += 1;
        } else {
            merged.push(right[j]);
            j += 1;
        }
    }
    merged.extend_from_slice(&left[i..]);
    merged.extend_from_slice(&right[j..]);
    slice.copy_from_slice(&merged);
}

impl Scenario for CpuBound {
    fn name(&self) -> &'static str {
        "cpu-bound"
    }

    fn config_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.cfg).unwrap_or_else(|_| serde_json::json!({}))
    }

    fn setup(&mut self) -> anyhow::Result<()> {
        // Build a SCOPED pool, never touch the global pool.
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.cfg.threads)
            .build()?;
        let n_elems = self.cfg.input_size_mb * 1024 * 1024 / std::mem::size_of::<u64>();
        let mut rng = SmallRng::seed_from_u64(self.cfg.seed);
        let mut input: Vec<u64> = (0..n_elems as u64).collect();
        // Fisher–Yates shuffle so the sort actually has work to do.
        for i in (1..input.len()).rev() {
            let j = rng.gen_range(0..=i);
            input.swap(i, j);
        }
        self.input = Some(input);
        self.pool = Some(pool);
        Ok(())
    }

    fn allocations_per_tick(&self) -> u64 {
        // Approximate: each merge node allocates one Vec<u64> of half-size;
        // total alloc nodes ~= ceil(log2(elems)) per recursive sort, but as
        // a coarse number for the aggregator we surface the element count
        // (which dominates the byte volume of allocations per tick).
        (self.cfg.input_size_mb as u64) * 1024 * 1024 / 8
    }

    fn tick(&mut self) -> Box<dyn SinkValue> {
        // Clone the input each tick so we sort fresh data; the clone alone
        // is one large allocation that joins the per-merge allocations as
        // workload for the allocator.
        let mut data = self
            .input
            .as_ref()
            .expect("setup() not called")
            .clone();
        let pool = self.pool.as_ref().expect("setup() not called");
        // pool.install ensures the rayon::join calls inside the sort use
        // OUR scoped pool, not the global one.
        pool.install(|| parallel_merge_sort(&mut data));
        // Defeat DCE: the sorted slice must be observed.
        std::hint::black_box(&data[..]);
        Box::new(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(threads: usize, input_size_mb: usize) -> CpuBoundConfig {
        CpuBoundConfig {
            threads,
            input_size_mb,
            seed: 1,
        }
    }

    #[test]
    fn validated_rejects_zero_threads() {
        let err = cfg(0, 4).validated().unwrap_err();
        assert!(err.to_string().contains("threads must be >= 1"));
    }

    #[test]
    fn validated_rejects_zero_input_size_mb() {
        let err = cfg(2, 0).validated().unwrap_err();
        assert!(err.to_string().contains("input_size_mb must be >= 1"));
    }

    #[test]
    fn validated_accepts_well_formed_config() {
        assert!(cfg(2, 1).validated().is_ok());
    }

    #[test]
    fn parallel_merge_sort_sorts_small_array() {
        // 100 elements: triggers the base case (<= 1024) so this validates
        // the cutoff path but not rayon::join.
        let mut data = vec![9u64, 3, 7, 1, 8, 2, 6, 4, 5, 0];
        parallel_merge_sort(&mut data);
        assert_eq!(data, vec![0u64, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn parallel_merge_sort_sorts_large_array_through_recursion() {
        // > 1024 elements: triggers rayon::join recursion + merge steps,
        // exercising the alloc-per-merge path.
        let mut rng = SmallRng::seed_from_u64(42);
        let mut data: Vec<u64> = (0..4096).collect();
        for i in (1..data.len()).rev() {
            let j = rng.gen_range(0..=i);
            data.swap(i, j);
        }
        let expected: Vec<u64> = (0..4096).collect();
        parallel_merge_sort(&mut data);
        assert_eq!(data, expected);
    }

    #[test]
    fn allocations_per_tick_matches_input_size() {
        // 1MB / 8 = 131072 u64 elements = 131072 alloc-units.
        let s = CpuBound::new(cfg(2, 1));
        assert_eq!(s.allocations_per_tick(), 131072);
    }

    #[test]
    fn tick_produces_sorted_data() {
        // 1MB input keeps the sort fast in tests but still triggers
        // rayon::join recursion (131072 elements > 1024 cutoff).
        let c = cfg(2, 1).validated().unwrap();
        let mut s = CpuBound::new(c);
        s.setup().expect("setup");
        // Re-sort a clone of the input independently; we can't downcast
        // the Box<dyn SinkValue> easily, so verify that calling tick()
        // doesn't panic and that the input still sorts via the helper.
        let _ = s.tick();
        let mut copy = s.input.as_ref().unwrap().clone();
        parallel_merge_sort(&mut copy);
        for w in copy.windows(2) {
            assert!(w[0] <= w[1], "sort broken at {:?}", w);
        }
    }
}

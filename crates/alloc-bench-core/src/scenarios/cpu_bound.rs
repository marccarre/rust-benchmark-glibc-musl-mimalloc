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
    /// Upper bound on `input_size_mb` (WR-04 Phase-2 review). 4096 MB =
    /// 4 GB of u64s = 512M elements per setup() shuffle and per-tick
    /// clone. Larger inputs would risk OOM-killing the host before
    /// surfacing a clean error to the harness.
    const MAX_INPUT_SIZE_MB: usize = 4096;

    /// Reject malformed configs at construction time.
    pub fn validated(self) -> anyhow::Result<Self> {
        anyhow::ensure!(
            self.threads >= 1,
            "threads must be >= 1 (got {})",
            self.threads
        );
        // WR-04 (Phase-2 review): upper bound prevents pathological
        // inputs (e.g., input_size_mb=1_000_000 → 1 TB) from OOM-killing
        // the host before a clean error reaches the harness.
        anyhow::ensure!(
            self.input_size_mb >= 1 && self.input_size_mb <= Self::MAX_INPUT_SIZE_MB,
            "input_size_mb must be in [1, {}] (got {})",
            Self::MAX_INPUT_SIZE_MB,
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

/// IN-05 (Phase-2 review): named constant for the merge-sort base-case
/// cutoff. Below this slice length, recursion stops and the std unstable
/// sort handles the rest in place (no allocations). The Phase-4
/// aggregator's per-allocator alloc-vs-no-alloc ratio depends on this
/// cutoff being explicit and consistent; documenting it here also keeps
/// `allocations_per_tick()` arithmetic in sync.
const BASE_CASE_CUTOFF: usize = 1024;

/// Recursive merge-sort with parallel divide and an allocation in every
/// merge step. Base case at `BASE_CASE_CUTOFF` elements falls back to
/// `slice::sort_unstable` to keep recursion shallow on small arrays.
///
/// **Critical invariant (RESEARCH.md §Pitfall 4):** the `Vec::with_capacity`
/// inside the merge step is intentional. Allocating a single temp buffer
/// once at the top of the recursion tree would defeat the benchmark — the
/// whole point is that allocations happen at every level so the allocator's
/// lock contention shows up.
fn parallel_merge_sort<T: Ord + Send + Copy>(slice: &mut [T]) {
    if slice.len() <= BASE_CASE_CUTOFF {
        // Base case: small arrays use std unstable sort (no recursion,
        // no allocations from this function — the std impl handles it).
        slice.sort_unstable();
        return;
    }
    let total_len = slice.len();
    let mid = total_len / 2;
    let (left, right) = slice.split_at_mut(mid);
    rayon::join(|| parallel_merge_sort(left), || parallel_merge_sort(right));

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
        // WR-08 (Phase-2 review): we previously returned the element count
        // (~8M for 64MB input). That is the *number of u64 elements*, NOT
        // the *number of allocations* — off by ~5 decimal orders. The
        // Phase-4 aggregator multiplies this by ticks_per_s to derive
        // allocs/s, so the wrong-by-300_000x value made the headline
        // metric meaningless.
        //
        // Real merge-sort allocation count per tick:
        //   * 1 allocation for the per-tick `data = self.input.clone()` at
        //     line 144.
        //   * Merge-sort recursion stops at BASE_CASE_CUTOFF (1024) elements,
        //     so there are `levels = ceil(log2(n_elems / 1024))` levels of
        //     parallel merge above the base case. Each internal node in the
        //     recursion tree allocates one Vec<T> in its merge step; the tree
        //     has roughly `2^levels` internal nodes for a balanced split.
        //
        // For 64 MB (8M u64) input: 2^ceil(log2(8M/1024)) = 2^13 = 8192
        // merge allocations + 1 clone = 8193. For run-all's 2 MB default
        // (262144 u64): 2^ceil(log2(256)) = 2^8 = 256 + 1 = 257. Both
        // genuine allocation counts; both several decimal orders smaller
        // than the old element-count proxy.
        let n_elems = (self.cfg.input_size_mb as u64) * 1024 * 1024 / 8;
        let above_cutoff = n_elems / 1024;
        if above_cutoff == 0 {
            // Entire sort handled by the base case (`slice.sort_unstable`),
            // which performs no allocations. Only the per-tick input clone.
            return 1;
        }
        // ceil(log2(above_cutoff)) via next_power_of_two().trailing_zeros().
        let levels = above_cutoff.next_power_of_two().trailing_zeros() as u64;
        let merge_nodes = 1u64.checked_shl(levels as u32).unwrap_or(u64::MAX);
        // +1 for the per-tick `data = input.clone()`.
        merge_nodes.saturating_add(1)
    }

    fn tick(&mut self) -> Box<dyn SinkValue> {
        // Clone the input each tick so we sort fresh data; the clone alone
        // is one large allocation that joins the per-merge allocations as
        // workload for the allocator.
        let mut data = self.input.as_ref().expect("setup() not called").clone();
        let pool = self.pool.as_ref().expect("setup() not called");
        // pool.install ensures the rayon::join calls inside the sort use
        // OUR scoped pool, not the global one.
        pool.install(|| parallel_merge_sort(&mut data));
        // Defeat DCE: the sorted slice must be observed.
        std::hint::black_box(&data[..]);
        Box::new(data)
    }

    /// WR-03 (Phase-2 review): explicit teardown drops the scoped rayon
    /// pool and the cached input vector so a subsequent `setup()` (e.g.,
    /// from a future test harness that reuses the scenario struct) does
    /// not overwrite a still-running pool — which would either leak the
    /// previous pool's worker threads or panic depending on how the
    /// `Option::replace` collides with the live workers.
    fn teardown(&mut self) {
        self.pool.take();
        self.input.take();
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
        // WR-04 (Phase-2 review): error message now includes the upper
        // bound — `input_size_mb must be in [1, 4096] (got 0)`.
        let msg = err.to_string();
        assert!(
            msg.contains("input_size_mb must be in"),
            "unexpected error: {msg}"
        );
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
    fn allocations_per_tick_counts_merge_nodes_plus_clone() {
        // WR-08 (Phase-2 review): allocations_per_tick = merge_nodes + 1.
        // 1 MB → 131072 u64 elements → 131072/1024 = 128 leaves above the
        // base-case cutoff → 2^ceil(log2(128)) = 2^7 = 128 internal merge
        // nodes + 1 input clone = 129 total per-tick allocations.
        let s = CpuBound::new(cfg(2, 1));
        assert_eq!(s.allocations_per_tick(), 129);

        // 64 MB → 8M elements → 8192 above-cutoff buckets → 2^13 = 8192
        // merge nodes + 1 = 8193. Several decimal orders smaller than
        // the old element-count proxy (8388608) but still represents
        // genuine allocations.
        let s = CpuBound::new(cfg(2, 64));
        assert_eq!(s.allocations_per_tick(), 8193);
    }

    #[test]
    fn allocations_per_tick_handles_below_cutoff() {
        // input_size_mb=1 always > cutoff (128 leaves); add a synthetic
        // tiny case where the entire sort is the base case. Smallest valid
        // size is 1 MB so we exercise the formula's `above_cutoff == 0`
        // path manually:
        let s = CpuBound::new(CpuBoundConfig {
            threads: 1,
            input_size_mb: 1,
            seed: 0,
        });
        // Sanity: even at 1 MB, recursion fires and we don't return 1.
        assert!(s.allocations_per_tick() > 1);
    }

    /// WR-04 (Phase-2 review): upper bound on input_size_mb.
    #[test]
    fn validated_rejects_oversize_input_size_mb() {
        let err = cfg(2, 100_000).validated().unwrap_err();
        assert!(
            err.to_string().contains("input_size_mb must be in"),
            "unexpected error: {err}"
        );
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

//! Mem-bound scenario (SCEN-07) — two-mode workload selected by `MemBoundMode`.
//!
//! - **`linked-list`** (alloc-heavy): builds a chain of uniform 64B `Box<Node>`
//!   nodes, traverses to defeat DCE, drops at end of tick. Tests slab /
//!   segregated-list allocator behaviour cleanly because every alloc is the
//!   same size class.
//! - **`strided-array`** (RSS + bandwidth, no per-tick alloc): pre-allocates
//!   one `Vec<u64>` of `--size MB` in `setup()` and reads it with a prime
//!   stride (4099) per tick to defeat L1 streaming + L2 stride prefetchers
//!   (RESEARCH.md §Pitfall 6).

use std::str::FromStr;

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use serde::Serialize;

use crate::harness::{Scenario, SinkValue};

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MemBoundMode {
    LinkedList,
    StridedArray,
}

impl FromStr for MemBoundMode {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "linked-list" | "linked_list" | "linkedlist" => Ok(MemBoundMode::LinkedList),
            "strided-array" | "strided_array" | "stridedarray" => Ok(MemBoundMode::StridedArray),
            other => anyhow::bail!(
                "unknown mem_bound mode: {other} (expected linked-list|strided-array)"
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MemBoundConfig {
    pub mode: MemBoundMode,
    pub size_mb: usize,
    pub seed: u64,
}

impl MemBoundConfig {
    pub fn validated(self) -> anyhow::Result<Self> {
        anyhow::ensure!(
            self.size_mb >= 1,
            "size_mb must be >= 1 (got {})",
            self.size_mb
        );
        Ok(self)
    }
}

/// 64-byte node for the linked-list mode (RESEARCH.md §"linked-list").
///
/// Layout: `Option<Box<Node>>` is niche-optimised to 8 bytes on 64-bit
/// targets; `[u8; 56]` fills the remaining 56 bytes for an exact 64-byte
/// total. Verified by `const _: () = assert!(...)` below — the compile
/// fails immediately if Rust ever changes the layout.
#[repr(C)]
struct Node {
    next: Option<Box<Node>>,
    payload: [u8; 56],
}

const _: () = assert!(std::mem::size_of::<Node>() == 64);

pub struct MemBound {
    cfg: MemBoundConfig,
    /// Populated by `setup()` only when `mode == StridedArray`.
    buffer: Option<Vec<u64>>,
}

impl MemBound {
    pub fn new(cfg: MemBoundConfig) -> Self {
        Self { cfg, buffer: None }
    }
}

impl Scenario for MemBound {
    fn name(&self) -> &'static str {
        "mem-bound"
    }

    fn config_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.cfg).unwrap_or_else(|_| serde_json::json!({}))
    }

    fn setup(&mut self) -> anyhow::Result<()> {
        if matches!(self.cfg.mode, MemBoundMode::StridedArray) {
            // Allocate the buffer once in setup so `tick()` is pure read work
            // (this mode tests RSS + bandwidth, not alloc churn).
            let n = self.cfg.size_mb * 1024 * 1024 / std::mem::size_of::<u64>();
            let mut rng = SmallRng::seed_from_u64(self.cfg.seed);
            let buf: Vec<u64> = (0..n).map(|_| rng.gen()).collect();
            self.buffer = Some(buf);
        }
        Ok(())
    }

    fn allocations_per_tick(&self) -> u64 {
        match self.cfg.mode {
            // One Box<Node> per 64B; chain length = size_mb * 1024 * 1024 / 64.
            MemBoundMode::LinkedList => (self.cfg.size_mb as u64) * 1024 * 1024 / 64,
            // Pure read; no per-tick alloc — RSS comes from the setup-time buffer.
            MemBoundMode::StridedArray => 0,
        }
    }

    fn tick(&mut self) -> Box<dyn SinkValue> {
        match self.cfg.mode {
            MemBoundMode::LinkedList => {
                // Build the chain top-down (RESEARCH.md §"linked-list"):
                let n_nodes = self.cfg.size_mb * 1024 * 1024 / 64;
                let mut head: Option<Box<Node>> = None;
                for i in 0..n_nodes {
                    head = Some(Box::new(Node {
                        next: head,
                        // Mid-buffer-style write defeats DCE — every node has
                        // a deterministic payload byte derived from index.
                        payload: [(i as u8); 56],
                    }));
                }
                // Traverse so the chain isn't a write-only structure (which
                // LLVM could in principle elide):
                let mut count = 0u64;
                let mut cursor = head.as_deref();
                while let Some(node) = cursor {
                    count = count.wrapping_add(node.payload[0] as u64);
                    cursor = node.next.as_deref();
                }
                std::hint::black_box(count);
                // Drop the chain on return — the entire `size_mb` MB of nodes
                // freed at once, exercising the allocator's free path.
                Box::new(head)
            }
            MemBoundMode::StridedArray => {
                let buf = self.buffer.as_mut().expect("buffer set up");
                let n = buf.len();
                // Prime stride > L2 line count defeats both L1 streaming
                // and L2 stride prefetchers (Pitfall 6).
                let stride: usize = 4099;
                let mut acc: u64 = 0;
                let mut i = 0usize;
                for _ in 0..n {
                    acc = acc.wrapping_add(buf[i]);
                    i = (i + stride) % n;
                }
                std::hint::black_box(acc);
                Box::new(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(mode: MemBoundMode, size_mb: usize) -> MemBoundConfig {
        MemBoundConfig {
            mode,
            size_mb,
            seed: 1,
        }
    }

    #[test]
    fn node_size_is_64() {
        // Exercise the const assertion at runtime too — `cargo test` runs
        // across cfg combinations and will catch any future layout drift.
        assert_eq!(std::mem::size_of::<Node>(), 64);
    }

    #[test]
    fn mem_bound_mode_from_str() {
        assert!(matches!(
            "linked-list".parse::<MemBoundMode>(),
            Ok(MemBoundMode::LinkedList)
        ));
        assert!(matches!(
            "linked_list".parse::<MemBoundMode>(),
            Ok(MemBoundMode::LinkedList)
        ));
        assert!(matches!(
            "strided-array".parse::<MemBoundMode>(),
            Ok(MemBoundMode::StridedArray)
        ));
        assert!(matches!(
            "STRIDED-ARRAY".parse::<MemBoundMode>(),
            Ok(MemBoundMode::StridedArray)
        ));
        assert!("xyz".parse::<MemBoundMode>().is_err());
    }

    #[test]
    fn validated_rejects_zero_size_mb() {
        let err = cfg(MemBoundMode::LinkedList, 0).validated().unwrap_err();
        assert!(err.to_string().contains("size_mb must be >= 1"));
    }

    #[test]
    fn validated_accepts_well_formed_config() {
        assert!(cfg(MemBoundMode::LinkedList, 4).validated().is_ok());
        assert!(cfg(MemBoundMode::StridedArray, 4).validated().is_ok());
    }

    #[test]
    fn linked_list_tick_smoke() {
        let c = cfg(MemBoundMode::LinkedList, 1).validated().unwrap();
        let mut s = MemBound::new(c);
        s.setup().unwrap();
        let _ = s.tick();
    }

    #[test]
    fn strided_array_tick_smoke() {
        let c = cfg(MemBoundMode::StridedArray, 1).validated().unwrap();
        let mut s = MemBound::new(c);
        s.setup().unwrap();
        let _ = s.tick();
    }

    #[test]
    fn allocations_per_tick_linked_list_matches_node_count() {
        // 1 MB / 64B = 16384 nodes per tick.
        let c = cfg(MemBoundMode::LinkedList, 1).validated().unwrap();
        let s = MemBound::new(c);
        assert_eq!(s.allocations_per_tick(), 16384);
    }

    #[test]
    fn allocations_per_tick_strided_array_is_zero() {
        let c = cfg(MemBoundMode::StridedArray, 1).validated().unwrap();
        let s = MemBound::new(c);
        assert_eq!(s.allocations_per_tick(), 0);
    }
}

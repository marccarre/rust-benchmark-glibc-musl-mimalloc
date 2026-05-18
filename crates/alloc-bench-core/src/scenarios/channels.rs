//! Channel scenarios — SPMC, MPSC, MPMC sharing one ChannelPayload + topology helper.
//!
//! Per RESEARCH.md §"Channel Scenarios" recommendation: a single file holds
//! all three because they share ~80% of the implementation (only the
//! sender/receiver cardinality differs).
//!
//! All three scenarios:
//! - Use `crossbeam_channel::bounded(capacity)` for backpressure semantics.
//! - Send `ChannelPayload { seq, data: Box<[u8]> }` — the `Box<[u8]>` is the
//!   allocation under test (256B–4KB depending on `--payload-dist`).
//! - Defeat DCE via mid-buffer write before send + `black_box(&msg)` before drop.
//! - Propagate worker panics via `std::panic::resume_unwind` (Phase-1 CR-02).
//! - Run producers flat-out (no `thread::sleep`) per RESEARCH.md §Pitfall 2 to
//!   maximise allocator stress and exercise channel back-pressure.
//!
//! Topology, in plain words:
//! - SPMC = 1 sender, N cloned receivers race for messages.
//! - MPSC = N cloned senders, 1 receiver.
//! - MPMC = N senders × M receivers, both sides cloned.

use std::str::FromStr;

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use serde::Serialize;

use crate::harness::{Scenario, SinkValue};

/// Payload size distribution. `Uniform` = `gen_range(256..=4096)`;
/// `Bimodal` = 90% small (256B), 10% large (4096B).
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PayloadDist {
    Uniform,
    Bimodal,
}

impl FromStr for PayloadDist {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "uniform" => Ok(PayloadDist::Uniform),
            "bimodal" => Ok(PayloadDist::Bimodal),
            other => anyhow::bail!("unknown payload_dist: {other} (expected uniform|bimodal)"),
        }
    }
}

/// One message flowing through the channel — the `data` `Box<[u8]>` is the
/// allocation under test.
#[allow(dead_code)] // `seq` defeats DCE on the payload — kept even when only `data` is read.
pub(crate) struct ChannelPayload {
    pub seq: u64,
    pub data: Box<[u8]>,
}

pub(crate) fn sample_payload_size(rng: &mut SmallRng, dist: PayloadDist) -> usize {
    match dist {
        PayloadDist::Uniform => rng.gen_range(256..=4096),
        PayloadDist::Bimodal => {
            if rng.gen::<f32>() < 0.9 {
                256
            } else {
                4096
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ChannelConfig {
    pub producers: usize,
    pub consumers: usize,
    pub capacity: usize,
    pub objects_per_tick: u64,
    pub payload_dist: PayloadDist,
    pub seed: u64,
}

/// WR-06 (Phase-2 review): topology tag carried into `validated_for(...)`
/// so SPMC/MPSC topology constraints are enforced at config-construction
/// time rather than only at the CLI surface. Bypassing the CLI (e.g.,
/// constructing `Mpsc::new(ChannelConfig{ consumers: 5, .. })` directly)
/// previously silently ran with the topology-illegal config: the Mpsc
/// tick body uses `cfg.consumers` for serialisation but only spawns one
/// receiver thread, so 4 of those 5 consumers do not exist — the
/// recorded JSON `consumers: 5` is then a lie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelKind {
    Spmc,
    Mpsc,
    Mpmc,
}

impl ChannelConfig {
    /// Reject malformed configs at construction time so the hot path
    /// (workers + RNG) stays panic-free. Mirrors MultithreadConfig::validated.
    pub fn validated(self) -> anyhow::Result<Self> {
        anyhow::ensure!(
            self.producers >= 1,
            "producers must be >= 1 (got {})",
            self.producers
        );
        anyhow::ensure!(
            self.consumers >= 1,
            "consumers must be >= 1 (got {})",
            self.consumers
        );
        anyhow::ensure!(
            self.capacity >= 1,
            "capacity must be >= 1 (got {})",
            self.capacity
        );
        anyhow::ensure!(
            self.objects_per_tick >= 1,
            "objects_per_tick must be >= 1 (got {})",
            self.objects_per_tick
        );
        Ok(self)
    }

    /// WR-06 (Phase-2 review): like `validated`, but additionally
    /// enforces topology constraints for the channel `kind`. SPMC must
    /// have exactly one producer; MPSC must have exactly one consumer;
    /// MPMC accepts any `>= 1`. Both `Mpsc::new` / `Spmc::new` callers
    /// (and `default_scenarios` in run.rs) should prefer this over
    /// `validated` so the topology constraint is enforced at the type-
    /// level entry point.
    pub fn validated_for(self, kind: ChannelKind) -> anyhow::Result<Self> {
        let cfg = self.validated()?;
        match kind {
            ChannelKind::Spmc => anyhow::ensure!(
                cfg.producers == 1,
                "SPMC requires producers == 1 (got {})",
                cfg.producers
            ),
            ChannelKind::Mpsc => anyhow::ensure!(
                cfg.consumers == 1,
                "MPSC requires consumers == 1 (got {})",
                cfg.consumers
            ),
            ChannelKind::Mpmc => {}
        }
        Ok(cfg)
    }
}

// =============================================================================
// SPMC — 1 sender, N cloned receivers race for messages
// =============================================================================

pub struct Spmc {
    cfg: ChannelConfig,
}

impl Spmc {
    pub fn new(cfg: ChannelConfig) -> Self {
        Self { cfg }
    }
}

impl Scenario for Spmc {
    fn name(&self) -> &'static str {
        "spmc"
    }

    fn config_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.cfg).unwrap_or_else(|_| serde_json::json!({}))
    }

    fn setup(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn allocations_per_tick(&self) -> u64 {
        // One Box<[u8]> per send.
        self.cfg.objects_per_tick
    }

    fn tick(&mut self) -> Box<dyn SinkValue> {
        let cfg = self.cfg.clone();
        let (s, r) = crossbeam_channel::bounded::<ChannelPayload>(cfg.capacity);

        std::thread::scope(|scope| {
            // Spawn N consumers, each holding a cloned receiver:
            let mut consumer_handles = Vec::with_capacity(cfg.consumers);
            for _ in 0..cfg.consumers {
                let r = r.clone();
                consumer_handles.push(scope.spawn(move || {
                    let mut received = 0u64;
                    while let Ok(msg) = r.recv() {
                        // Defeat DCE on the receive side.
                        std::hint::black_box(&msg);
                        received = received.wrapping_add(1);
                        drop(msg);
                    }
                    received
                }));
            }

            // Drop the original receiver so consumers terminate when the sender closes.
            drop(r);

            // 1 producer (this thread):
            let mut rng = SmallRng::seed_from_u64(cfg.seed);
            for seq in 0..cfg.objects_per_tick {
                let size = sample_payload_size(&mut rng, cfg.payload_dist);
                let mut data: Box<[u8]> = vec![0u8; size].into_boxed_slice();
                // PITFALLS §1.2: mid-buffer write defeats optimizer-driven elision.
                data[size / 2] = (seq & 0xFF) as u8;
                s.send(std::hint::black_box(ChannelPayload { seq, data }))
                    .expect("spmc send (consumers must outlive producer)");
            }
            drop(s); // close channel — consumers terminate after draining.

            let mut total = 0u64;
            for h in consumer_handles {
                // CR-02: propagate worker panics so the harness fails loudly.
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

// =============================================================================
// MPSC — N cloned senders, 1 receiver
// =============================================================================

pub struct Mpsc {
    cfg: ChannelConfig,
}

impl Mpsc {
    pub fn new(cfg: ChannelConfig) -> Self {
        Self { cfg }
    }
}

impl Scenario for Mpsc {
    fn name(&self) -> &'static str {
        "mpsc"
    }

    fn config_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.cfg).unwrap_or_else(|_| serde_json::json!({}))
    }

    fn setup(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn allocations_per_tick(&self) -> u64 {
        // CR-02 (Phase-2 review): per-tick work is split across producers
        // via integer division (`per_producer = objects_per_tick /
        // producers`). The *actual* allocations performed per tick are
        // `producers * per_producer`, not `objects_per_tick`. When
        // `objects_per_tick` is not an exact multiple of `producers`, the
        // truncated remainder must NOT be counted — otherwise allocator
        // throughput derived as `ticks_per_s * allocations_per_tick`
        // over-reports by up to `producers - 1` allocations per tick.
        let per_producer = self.cfg.objects_per_tick / self.cfg.producers as u64;
        per_producer.saturating_mul(self.cfg.producers as u64)
    }

    fn tick(&mut self) -> Box<dyn SinkValue> {
        let cfg = self.cfg.clone();
        let (s, r) = crossbeam_channel::bounded::<ChannelPayload>(cfg.capacity);

        // Spread `objects_per_tick` across producers. Integer division rounds
        // down so total may be slightly < objects_per_tick when the count
        // doesn't divide evenly — `allocations_per_tick()` reports the
        // truncated total (`producers * per_producer`), not the input.
        let per_producer = cfg.objects_per_tick / cfg.producers as u64;

        std::thread::scope(|scope| {
            // Spawn N producers, each holding a cloned sender:
            let mut producer_handles = Vec::with_capacity(cfg.producers);
            for p in 0..cfg.producers {
                let s = s.clone();
                let cfg = cfg.clone();
                producer_handles.push(scope.spawn(move || {
                    // Per-thread RNG mirrors MultithreadConfig seed-per-thread pattern.
                    let mut rng = SmallRng::seed_from_u64(cfg.seed.wrapping_add(p as u64));
                    for seq in 0..per_producer {
                        let size = sample_payload_size(&mut rng, cfg.payload_dist);
                        let mut data: Box<[u8]> = vec![0u8; size].into_boxed_slice();
                        data[size / 2] = (seq & 0xFF) as u8;
                        s.send(std::hint::black_box(ChannelPayload { seq, data }))
                            .expect("mpsc send (consumer must outlive producers)");
                    }
                }));
            }
            // Drop the original sender — channel closes once all producer
            // clones go out of scope.
            drop(s);

            // 1 consumer (this thread):
            let mut received = 0u64;
            while let Ok(msg) = r.recv() {
                std::hint::black_box(&msg);
                received = received.wrapping_add(1);
                drop(msg);
            }
            std::hint::black_box(received);

            for h in producer_handles {
                match h.join() {
                    Ok(()) => {}
                    Err(payload) => std::panic::resume_unwind(payload),
                }
            }
        });

        Box::new(())
    }
}

// =============================================================================
// MPMC — N senders × M receivers, both cloned
// =============================================================================

pub struct Mpmc {
    cfg: ChannelConfig,
}

impl Mpmc {
    pub fn new(cfg: ChannelConfig) -> Self {
        Self { cfg }
    }
}

impl Scenario for Mpmc {
    fn name(&self) -> &'static str {
        "mpmc"
    }

    fn config_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.cfg).unwrap_or_else(|_| serde_json::json!({}))
    }

    fn setup(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn allocations_per_tick(&self) -> u64 {
        // CR-02 (Phase-2 review): same per-producer truncation as Mpsc.
        // Reports `producers * per_producer`, not `objects_per_tick`,
        // so the derived allocation rate is faithful when
        // `objects_per_tick % producers != 0`.
        let per_producer = self.cfg.objects_per_tick / self.cfg.producers as u64;
        per_producer.saturating_mul(self.cfg.producers as u64)
    }

    fn tick(&mut self) -> Box<dyn SinkValue> {
        let cfg = self.cfg.clone();
        let (s, r) = crossbeam_channel::bounded::<ChannelPayload>(cfg.capacity);
        let per_producer = cfg.objects_per_tick / cfg.producers as u64;

        std::thread::scope(|scope| {
            // Spawn N producers:
            let mut producer_handles = Vec::with_capacity(cfg.producers);
            for p in 0..cfg.producers {
                let s = s.clone();
                let cfg = cfg.clone();
                producer_handles.push(scope.spawn(move || {
                    let mut rng = SmallRng::seed_from_u64(cfg.seed.wrapping_add(p as u64));
                    for seq in 0..per_producer {
                        let size = sample_payload_size(&mut rng, cfg.payload_dist);
                        let mut data: Box<[u8]> = vec![0u8; size].into_boxed_slice();
                        data[size / 2] = (seq & 0xFF) as u8;
                        s.send(std::hint::black_box(ChannelPayload { seq, data }))
                            .expect("mpmc send (consumers must outlive producers)");
                    }
                }));
            }
            // Drop the original sender so consumers see channel close after
            // all producer clones finish.
            drop(s);

            // Spawn M consumers:
            let mut consumer_handles = Vec::with_capacity(cfg.consumers);
            for _ in 0..cfg.consumers {
                let r = r.clone();
                consumer_handles.push(scope.spawn(move || {
                    let mut received = 0u64;
                    while let Ok(msg) = r.recv() {
                        std::hint::black_box(&msg);
                        received = received.wrapping_add(1);
                        drop(msg);
                    }
                    received
                }));
            }
            // Drop the original receiver — without this, consumers would
            // never terminate (receiver clone count > 0).
            drop(r);

            for h in producer_handles {
                match h.join() {
                    Ok(()) => {}
                    Err(payload) => std::panic::resume_unwind(payload),
                }
            }
            let mut total = 0u64;
            for h in consumer_handles {
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

    fn cfg(producers: usize, consumers: usize, capacity: usize) -> ChannelConfig {
        ChannelConfig {
            producers,
            consumers,
            capacity,
            objects_per_tick: 100,
            payload_dist: PayloadDist::Uniform,
            seed: 1,
        }
    }

    #[test]
    fn payload_dist_from_str() {
        assert!(matches!(
            "uniform".parse::<PayloadDist>(),
            Ok(PayloadDist::Uniform)
        ));
        assert!(matches!(
            "bimodal".parse::<PayloadDist>(),
            Ok(PayloadDist::Bimodal)
        ));
        assert!(matches!(
            "UNIFORM".parse::<PayloadDist>(),
            Ok(PayloadDist::Uniform)
        ));
        assert!("xyz".parse::<PayloadDist>().is_err());
    }

    #[test]
    fn validated_rejects_zero_producers() {
        let err = cfg(0, 1, 16).validated().unwrap_err();
        assert!(err.to_string().contains("producers must be >= 1"));
    }

    #[test]
    fn validated_rejects_zero_consumers() {
        let err = cfg(1, 0, 16).validated().unwrap_err();
        assert!(err.to_string().contains("consumers must be >= 1"));
    }

    #[test]
    fn validated_rejects_zero_capacity() {
        let err = cfg(1, 1, 0).validated().unwrap_err();
        assert!(err.to_string().contains("capacity must be >= 1"));
    }

    #[test]
    fn validated_rejects_zero_objects_per_tick() {
        let mut c = cfg(1, 1, 16);
        c.objects_per_tick = 0;
        let err = c.validated().unwrap_err();
        assert!(err.to_string().contains("objects_per_tick must be >= 1"));
    }

    #[test]
    fn validated_accepts_well_formed_config() {
        assert!(cfg(2, 4, 64).validated().is_ok());
    }

    #[test]
    fn sample_payload_size_uniform_within_bounds() {
        let mut rng = SmallRng::seed_from_u64(42);
        for _ in 0..200 {
            let s = sample_payload_size(&mut rng, PayloadDist::Uniform);
            assert!((256..=4096).contains(&s), "size {s} out of range");
        }
    }

    #[test]
    fn sample_payload_size_bimodal_picks_endpoints() {
        let mut rng = SmallRng::seed_from_u64(42);
        for _ in 0..50 {
            let s = sample_payload_size(&mut rng, PayloadDist::Bimodal);
            assert!(s == 256 || s == 4096, "bimodal size {s} not at endpoint");
        }
    }

    #[test]
    fn spmc_tick_smoke() {
        let c = cfg(1, 2, 8).validated().unwrap();
        let mut s = Spmc::new(c);
        // tick() returns Box<()>; we only assert it does not panic.
        let _ = s.tick();
    }

    #[test]
    fn mpsc_tick_smoke() {
        let c = cfg(2, 1, 8).validated().unwrap();
        let mut s = Mpsc::new(c);
        let _ = s.tick();
    }

    #[test]
    fn mpmc_tick_smoke() {
        let c = cfg(2, 2, 8).validated().unwrap();
        let mut s = Mpmc::new(c);
        let _ = s.tick();
    }

    /// CR-02: when `objects_per_tick % producers != 0`, the per-tick work is
    /// `producers * (objects_per_tick / producers)` (integer truncation).
    /// `allocations_per_tick()` MUST report this truncated total so the
    /// derived allocator throughput is faithful — over-reporting by the
    /// truncated remainder propagates through the Phase-4 aggregator as a
    /// wrong allocs/s number.
    #[test]
    fn mpsc_allocations_per_tick_accounts_for_truncation() {
        // 10 / 3 = 3 per producer × 3 producers = 9 actual allocs (NOT 10).
        let mut c = cfg(3, 1, 8);
        c.objects_per_tick = 10;
        let s = Mpsc::new(c.validated().unwrap());
        assert_eq!(s.allocations_per_tick(), 9);
    }

    #[test]
    fn mpsc_allocations_per_tick_exact_divisor() {
        // Even split: 12 / 4 = 3 per producer × 4 = 12 (no truncation).
        let mut c = cfg(4, 1, 8);
        c.objects_per_tick = 12;
        let s = Mpsc::new(c.validated().unwrap());
        assert_eq!(s.allocations_per_tick(), 12);
    }

    #[test]
    fn mpmc_allocations_per_tick_accounts_for_truncation() {
        // Same arithmetic for MPMC: 7 / 3 = 2 per producer × 3 = 6 (NOT 7).
        let mut c = cfg(3, 2, 8);
        c.objects_per_tick = 7;
        let s = Mpmc::new(c.validated().unwrap());
        assert_eq!(s.allocations_per_tick(), 6);
    }

    #[test]
    fn mpmc_allocations_per_tick_exact_divisor() {
        let mut c = cfg(4, 2, 8);
        c.objects_per_tick = 16;
        let s = Mpmc::new(c.validated().unwrap());
        assert_eq!(s.allocations_per_tick(), 16);
    }

    /// WR-06 (Phase-2 review): topology constraints enforced at config
    /// construction.
    #[test]
    fn validated_for_spmc_rejects_multi_producer() {
        let err = cfg(2, 4, 16).validated_for(ChannelKind::Spmc).unwrap_err();
        assert!(
            err.to_string().contains("SPMC requires producers == 1"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn validated_for_spmc_accepts_single_producer() {
        assert!(cfg(1, 4, 16).validated_for(ChannelKind::Spmc).is_ok());
    }

    #[test]
    fn validated_for_mpsc_rejects_multi_consumer() {
        let err = cfg(4, 2, 16).validated_for(ChannelKind::Mpsc).unwrap_err();
        assert!(
            err.to_string().contains("MPSC requires consumers == 1"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn validated_for_mpsc_accepts_single_consumer() {
        assert!(cfg(4, 1, 16).validated_for(ChannelKind::Mpsc).is_ok());
    }

    #[test]
    fn validated_for_mpmc_accepts_any_topology() {
        assert!(cfg(2, 2, 16).validated_for(ChannelKind::Mpmc).is_ok());
        assert!(cfg(1, 1, 16).validated_for(ChannelKind::Mpmc).is_ok());
        assert!(cfg(8, 4, 16).validated_for(ChannelKind::Mpmc).is_ok());
    }
}

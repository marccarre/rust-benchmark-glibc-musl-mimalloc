use serde::Serialize;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize)]
pub struct Run {
    pub schema_version: u32,
    pub run_id: String,
    pub env: Env,
    pub build: Build,
    pub scenario: ScenarioInfo,
    pub harness: HarnessInfo,
    pub metrics: Metrics,
}

#[derive(Debug, Serialize)]
pub struct Env {
    pub os: String,
    pub os_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docker_image: Option<String>,
    pub cpu_model: String,
    pub cpu_count: u32,
    pub memory_total_kb: u64,
}

#[derive(Debug, Serialize)]
pub struct Build {
    pub allocator: String,
    pub rustc_version: String,
    pub target_triple: String,
    pub host_triple: String,
    pub profile: String,
    pub git_sha: String,
    pub git_dirty: bool,
    pub build_timestamp: String,
    pub rustflags: String,
}

#[derive(Debug, Serialize)]
pub struct ScenarioInfo {
    pub name: String,
    pub config: serde_json::Value,
    /// Phase-2 additive field (CONTEXT.md D-CONTEXT). Optional throughput unit
    /// label consumed by the Phase-4 aggregator. `None` (default) → ticks/s;
    /// `Some("iters_per_s")` for channel scenarios; `Some("req_per_s")` for
    /// the upcoming Phase-2 web scenario. Skipped when serializing if `None`,
    /// so existing Phase-1 multithread JSON shapes remain byte-identical.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HarnessInfo {
    pub warmup_duration_s: f64,
    pub measurement_duration_s: f64,
    pub samples_count: u64,
}

/// Latency percentiles for a single `tick()` (one batched fork/join across
/// all worker threads), measured in nanoseconds. *Not* per-allocation latency.
#[derive(Debug, Serialize)]
pub struct LatencyNs {
    pub p50: u64,
    pub p95: u64,
    pub p99: u64,
    pub p999: u64,
    pub max: u64,
}

#[derive(Debug, Serialize)]
pub struct RssGrowthSample {
    pub t_s: u64,
    pub rss_kb: u64,
}

#[derive(Debug, Serialize)]
pub struct Rusage {
    pub user_time_s: f64,
    pub sys_time_s: f64,
    pub minor_faults: u64,
    pub major_faults: u64,
    pub voluntary_csw: u64,
    pub involuntary_csw: u64,
    pub peak_rss_kb: u64,
}

#[derive(Debug, Serialize)]
pub struct Metrics {
    /// Number of `Scenario::tick()` invocations completed per second
    /// during the measurement window. WR-01: this is *not* an allocation
    /// rate — divide `ticks_per_s * allocations_per_tick` to derive that.
    pub ticks_per_s: f64,
    /// Number of allocations a single `Scenario::tick()` performs.
    /// For Multithread: `threads * objects`. WR-01: lets consumers compute
    /// the allocation rate as `ticks_per_s * allocations_per_tick`.
    pub allocations_per_tick: u64,
    /// WR-01: per-tick fork/join latency, *not* per-allocation latency.
    /// See `LatencyNs` for full semantics.
    pub tick_latency_ns: LatencyNs,
    pub peak_rss_kb: u64,
    pub rss_growth_samples: Vec<RssGrowthSample>,
    pub rusage: Rusage,
    pub allocator_stats: serde_json::Value,
}

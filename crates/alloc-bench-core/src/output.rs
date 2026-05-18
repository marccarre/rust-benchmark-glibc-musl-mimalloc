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
}

#[derive(Debug, Serialize)]
pub struct HarnessInfo {
    pub warmup_duration_s: f64,
    pub measurement_duration_s: f64,
    pub samples_count: u64,
}

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
    pub throughput_ops_per_s: f64,
    pub latency_ns: LatencyNs,
    pub peak_rss_kb: u64,
    pub rss_growth_samples: Vec<RssGrowthSample>,
    pub rusage: Rusage,
    pub allocator_stats: serde_json::Value,
}

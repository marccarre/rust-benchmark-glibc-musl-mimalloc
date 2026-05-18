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
    /// Phase-2 additive field (CONTEXT.md schema-extension decision). Top-level
    /// status used by the `run-all` command to mark per-scenario success or
    /// failure. `None` (default) for legacy single-scenario runs — `serde`
    /// drops the key entirely via `skip_serializing_if`, keeping Phase-1 JSON
    /// shape byte-identical. `Some("success" | "failed")` only set by run-all.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Phase-2 additive field. Populated only when `status == Some("failed")`
    /// (e.g., a panicked scenario in run-all). `None` is dropped on serialize.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Phase-2 additive-only invariant: when `Run.status` and `Run.error`
    /// are both `None`, the serialized JSON MUST NOT contain `status` or
    /// `error` keys. This guards against a future maintainer defaulting
    /// `status: Some("success")` and silently breaking the byte-for-byte
    /// JSON shape that Phase-1 single-scenario consumers depend on.
    #[test]
    fn run_with_none_status_omits_status_and_error_keys() {
        let run = Run {
            schema_version: SCHEMA_VERSION,
            run_id: "stub".to_string(),
            env: Env {
                os: "x".into(),
                os_version: "y".into(),
                docker_image: None,
                cpu_model: "z".into(),
                cpu_count: 1,
                memory_total_kb: 1,
            },
            build: Build {
                allocator: "system".into(),
                rustc_version: "x".into(),
                target_triple: "x".into(),
                host_triple: "x".into(),
                profile: "x".into(),
                git_sha: "x".into(),
                git_dirty: false,
                build_timestamp: "x".into(),
                rustflags: "x".into(),
            },
            scenario: ScenarioInfo {
                name: "stub".into(),
                config: serde_json::json!({}),
                unit: None,
            },
            harness: HarnessInfo {
                warmup_duration_s: 0.0,
                measurement_duration_s: 0.0,
                samples_count: 0,
            },
            metrics: Metrics {
                ticks_per_s: 0.0,
                allocations_per_tick: 0,
                tick_latency_ns: LatencyNs {
                    p50: 0,
                    p95: 0,
                    p99: 0,
                    p999: 0,
                    max: 0,
                },
                peak_rss_kb: 0,
                rss_growth_samples: vec![],
                rusage: Rusage {
                    user_time_s: 0.0,
                    sys_time_s: 0.0,
                    minor_faults: 0,
                    major_faults: 0,
                    voluntary_csw: 0,
                    involuntary_csw: 0,
                    peak_rss_kb: 0,
                },
                allocator_stats: serde_json::json!({}),
            },
            status: None,
            error: None,
        };
        let json = serde_json::to_string(&run).expect("serialize");
        assert!(
            !json.contains("\"status\""),
            "expected no `status` key when Run.status is None — got {json}"
        );
        assert!(
            !json.contains("\"error\""),
            "expected no `error` key when Run.error is None — got {json}"
        );
    }

    /// WR-10 (Phase-2 review): canonical-shape snapshot. Enumerates every
    /// top-level key the schema is allowed to emit at SCHEMA_VERSION=1 so
    /// a future PR that adds a NEW required field (no
    /// `skip_serializing_if`) fails this test — forcing the author to
    /// either bump SCHEMA_VERSION in the same change or revert the field.
    /// Phase-1 byte-equivalence is asserted by the
    /// `run_with_none_status_omits_status_and_error_keys` test above;
    /// this one pins the additive-shape gate at the type level.
    ///
    /// If you intentionally bump SCHEMA_VERSION, update both the constant
    /// (output.rs:3) AND the `expected` slice below in the same commit.
    #[test]
    fn run_canonical_shape_snapshot() {
        let run = Run {
            schema_version: SCHEMA_VERSION,
            run_id: "stub".to_string(),
            env: Env {
                os: "x".into(),
                os_version: "y".into(),
                docker_image: None,
                cpu_model: "z".into(),
                cpu_count: 1,
                memory_total_kb: 1,
            },
            build: Build {
                allocator: "system".into(),
                rustc_version: "x".into(),
                target_triple: "x".into(),
                host_triple: "x".into(),
                profile: "x".into(),
                git_sha: "x".into(),
                git_dirty: false,
                build_timestamp: "x".into(),
                rustflags: "x".into(),
            },
            scenario: ScenarioInfo {
                name: "stub".into(),
                config: serde_json::json!({}),
                unit: Some("req_per_s".into()),
            },
            harness: HarnessInfo {
                warmup_duration_s: 0.0,
                measurement_duration_s: 0.0,
                samples_count: 0,
            },
            metrics: Metrics {
                ticks_per_s: 0.0,
                allocations_per_tick: 0,
                tick_latency_ns: LatencyNs {
                    p50: 0,
                    p95: 0,
                    p99: 0,
                    p999: 0,
                    max: 0,
                },
                peak_rss_kb: 0,
                rss_growth_samples: vec![],
                rusage: Rusage {
                    user_time_s: 0.0,
                    sys_time_s: 0.0,
                    minor_faults: 0,
                    major_faults: 0,
                    voluntary_csw: 0,
                    involuntary_csw: 0,
                    peak_rss_kb: 0,
                },
                allocator_stats: serde_json::json!({}),
            },
            // status: Some so the canonical-success run-all shape is
            // exercised; error stays None so it's still skipped.
            status: Some("success".into()),
            error: None,
        };
        let v: serde_json::Value = serde_json::to_value(&run).expect("serialize");
        let obj = v.as_object().expect("Run must be a JSON object");
        let mut keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
        keys.sort_unstable();

        // Canonical SCHEMA_VERSION=1 top-level keys for a successful
        // run-all entry. `error` is absent because it's None above
        // (skip_serializing_if drops it). If you add a NEW top-level
        // field to `Run`, update this slice AND bump SCHEMA_VERSION in
        // output.rs:3 in the SAME commit.
        let expected: &[&str] = &[
            "build",
            "env",
            "harness",
            "metrics",
            "run_id",
            "scenario",
            "schema_version",
            "status",
        ];
        assert_eq!(
            keys, expected,
            "Run JSON shape changed at SCHEMA_VERSION=1; bump SCHEMA_VERSION + update this test in lockstep"
        );
    }

    /// Conversely, when `status: Some(_)` and `error: Some(_)` are set,
    /// both keys MUST appear (the run-all path depends on this).
    #[test]
    fn run_with_failed_status_emits_status_and_error_keys() {
        let run = Run {
            schema_version: SCHEMA_VERSION,
            run_id: "stub".into(),
            env: Env {
                os: "x".into(),
                os_version: "y".into(),
                docker_image: None,
                cpu_model: "z".into(),
                cpu_count: 1,
                memory_total_kb: 1,
            },
            build: Build {
                allocator: "system".into(),
                rustc_version: "x".into(),
                target_triple: "x".into(),
                host_triple: "x".into(),
                profile: "x".into(),
                git_sha: "x".into(),
                git_dirty: false,
                build_timestamp: "x".into(),
                rustflags: "x".into(),
            },
            scenario: ScenarioInfo {
                name: "stub".into(),
                config: serde_json::json!({}),
                unit: None,
            },
            harness: HarnessInfo {
                warmup_duration_s: 0.0,
                measurement_duration_s: 0.0,
                samples_count: 0,
            },
            metrics: Metrics {
                ticks_per_s: 0.0,
                allocations_per_tick: 0,
                tick_latency_ns: LatencyNs {
                    p50: 0,
                    p95: 0,
                    p99: 0,
                    p999: 0,
                    max: 0,
                },
                peak_rss_kb: 0,
                rss_growth_samples: vec![],
                rusage: Rusage {
                    user_time_s: 0.0,
                    sys_time_s: 0.0,
                    minor_faults: 0,
                    major_faults: 0,
                    voluntary_csw: 0,
                    involuntary_csw: 0,
                    peak_rss_kb: 0,
                },
                allocator_stats: serde_json::json!({}),
            },
            status: Some("failed".into()),
            error: Some("boom".into()),
        };
        let json = serde_json::to_string(&run).expect("serialize");
        assert!(json.contains("\"status\":\"failed\""), "got {json}");
        assert!(json.contains("\"error\":\"boom\""), "got {json}");
    }
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

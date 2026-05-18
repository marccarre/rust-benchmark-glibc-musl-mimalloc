use std::path::PathBuf;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::tempdir;

#[test]
fn multithread_emits_schema_v1_results_json() {
    let dir = tempdir().expect("tempdir");
    let out: PathBuf = dir.path().join("run.json");

    let mut cmd = Command::cargo_bin("alloc-bench-cli").expect("cargo bin");
    cmd.args([
        "multithread",
        "--threads",
        "2",
        "--objects",
        "100",
        "--warmup",
        "1s",
        "--duration",
        "1s",
        "--output",
    ])
    .arg(&out);
    cmd.assert().success();

    let raw = std::fs::read_to_string(&out).expect("read run.json");
    let v: Value = serde_json::from_str(&raw).expect("parse run.json");

    assert_eq!(v["schema_version"].as_u64(), Some(1));
    assert_eq!(v["scenario"]["name"].as_str(), Some("multithread"));
    assert!(
        v["metrics"]["throughput_ops_per_s"].as_f64().unwrap_or(0.0) > 0.0,
        "throughput must be > 0"
    );
    assert!(
        v["metrics"]["latency_ns"]["p50"].as_u64().unwrap_or(0) > 0,
        "p50 latency must be > 0"
    );
    assert_eq!(v["harness"]["warmup_duration_s"].as_f64(), Some(1.0));

    let kind = v["metrics"]["allocator_stats"]["kind"]
        .as_str()
        .expect("allocator_stats.kind missing");
    assert!(
        matches!(kind, "system" | "libmalloc" | "jemalloc" | "mimalloc"),
        "unexpected allocator kind: {kind}"
    );
}

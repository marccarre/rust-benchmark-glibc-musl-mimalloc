//! Phase-2 SCEN-11 end-to-end gate. Drives `alloc-bench-cli run-all` and
//! validates the combined JSON shape:
//!   - JSON is an array of length 10 (one record per scenario)
//!   - Every entry has `schema_version: 1`
//!   - Every entry's `scenario.name` is in the canonical 10-name set,
//!     each appearing EXACTLY once (registry must not double-add or skip)
//!   - Every entry has `status` set to "success" or "failed"
//!   - Successful entries: `metrics.ticks_per_s > 0` AND
//!     `metrics.tick_latency_ns.p50 > 0`
//!   - Failed entries: `error` is a non-empty string
//!   - The two are mutually exclusive — never both metrics > 0 AND error set
//!   - `env.cpu_count > 0` and `build.allocator` non-empty (env + build
//!     populated regardless of status)
//!
//! Per Phase-1 SUMMARY pattern, this test runs against the release binary
//! built by cargo (`assert_cmd::Command::cargo_bin`) so LTO=fat / codegen-
//! units=1 / opt-level=3 are active — matching the contract for benchmark
//! correctness.

use std::collections::HashSet;
use std::path::PathBuf;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::tempdir;

const EXPECTED_SCENARIOS: [&str; 10] = [
    "multithread",
    "spmc",
    "mpsc",
    "mpmc",
    "contention",
    "mem-bound",
    "realloc-storm",
    "cpu-bound",
    "fragmentation-soak",
    "web",
];

#[test]
fn run_all_emits_one_record_per_scenario() {
    let dir = tempdir().expect("tempdir");
    let out: PathBuf = dir.path().join("all.json");

    let mut cmd = Command::cargo_bin("alloc-bench-cli").expect("cargo bin");
    cmd.args([
        "run-all",
        "--seed",
        "12345",
        "--warmup",
        "1s",
        "--duration",
        "5s",
        "--output",
    ])
    .arg(&out);
    // Smoke-shape config: 10 scenarios × (1s warmup + 5s measure) ≈ 60s,
    // plus ~ms-scale fixed overhead per scenario. The CLI defaults are
    // 5s/60s (canonical local-bench shape); this test pins the smoke
    // shape explicitly so it runs in ~60s on CI. assert_cmd's default
    // timeout is none; CI budgets typically allow 3-5min for a single
    // integration test which is well above the ~90s upper bound on a
    // slow host.
    cmd.assert().success();

    let raw = std::fs::read_to_string(&out).expect("read all.json");
    let v: Value = serde_json::from_str(&raw).expect("parse all.json");
    let arr = v.as_array().expect("run-all output must be a JSON array");

    // 1. Exactly 10 records.
    assert_eq!(
        arr.len(),
        10,
        "expected 10 records (one per scenario), got {}",
        arr.len()
    );

    // 2. Every name in the expected set, each exactly once. We assert the
    //    *set* (not order) so the registry can be reordered later without
    //    breaking this gate; ordering is a Phase-4 aggregator concern.
    let mut seen: HashSet<&str> = HashSet::new();
    for entry in arr {
        let name = entry["scenario"]["name"]
            .as_str()
            .expect("scenario.name must be a string");
        assert!(
            EXPECTED_SCENARIOS.contains(&name),
            "unexpected scenario name in run-all output: '{name}' (allowed: {:?})",
            EXPECTED_SCENARIOS
        );
        assert!(
            seen.insert(name),
            "duplicate scenario name in run-all output: '{name}'"
        );
    }
    let expected_set: HashSet<&str> = EXPECTED_SCENARIOS.iter().copied().collect();
    assert_eq!(
        seen, expected_set,
        "run-all scenario set mismatch: got {seen:?}, expected {expected_set:?}"
    );

    // 3. Per-record shape assertions.
    for entry in arr {
        let name = entry["scenario"]["name"].as_str().unwrap();

        // schema_version must be 1 — additive-only invariant.
        assert_eq!(
            entry["schema_version"].as_u64(),
            Some(1),
            "{name}: schema_version must be 1"
        );

        // env + build populated regardless of success/failure.
        assert!(
            entry["env"]["cpu_count"].as_u64().unwrap_or(0) > 0,
            "{name}: env.cpu_count must be > 0"
        );
        let allocator_str = entry["build"]["allocator"]
            .as_str()
            .expect("build.allocator must be a string");
        assert!(
            !allocator_str.is_empty(),
            "{name}: build.allocator must be non-empty"
        );

        // status must be "success" or "failed". For run-all entries we
        // require Some — the None case is the legacy single-scenario
        // shape which run-all never emits.
        let status = entry["status"]
            .as_str()
            .expect("run-all entries must have status set (not None)");
        assert!(
            status == "success" || status == "failed",
            "{name}: status must be 'success' or 'failed', got '{status}'"
        );

        let ticks = entry["metrics"]["ticks_per_s"].as_f64().unwrap_or(0.0);
        let p50 = entry["metrics"]["tick_latency_ns"]["p50"]
            .as_u64()
            .unwrap_or(0);
        let error_value = &entry["error"];

        if status == "success" {
            // Successful scenario: positive throughput + populated p50.
            assert!(
                ticks > 0.0,
                "{name}: status=success but ticks_per_s={ticks}"
            );
            assert!(p50 > 0, "{name}: status=success but tick_latency_ns.p50=0");
            // Mutual exclusion: success records MUST NOT have an error
            // field (it's None → omitted via skip_serializing_if).
            assert!(
                error_value.is_null(),
                "{name}: status=success must omit error, got {error_value:?}"
            );
        } else {
            // Failed scenario: error populated, metrics zeroed.
            let error_str = error_value
                .as_str()
                .unwrap_or_else(|| panic!("{name}: status=failed but error is not a string"));
            assert!(
                !error_str.is_empty(),
                "{name}: status=failed but error string is empty"
            );
            // Mutual exclusion: degenerate failure run zeros metrics.
            assert_eq!(
                ticks, 0.0,
                "{name}: status=failed must zero ticks_per_s, got {ticks}"
            );
        }
    }

    // 4. Sanity: the run-all binary should be allocator-agnostic on the
    //    host — assert at least one entry succeeded so the test catches
    //    catastrophic regressions where every scenario panics.
    let success_count = arr
        .iter()
        .filter(|e| e["status"].as_str() == Some("success"))
        .count();
    assert!(
        success_count >= 1,
        "expected at least 1 successful scenario; all 10 failed — likely a regression"
    );
}

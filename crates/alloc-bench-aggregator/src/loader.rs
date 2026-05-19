//! Discover, parse, and validate `results/*.json` inputs (D-06, D-08).
//!
//! Behavior summary (RESEARCH §Pattern 2):
//!   - `glob::glob(pattern)` → collect → sort_unstable. Sort is mandatory:
//!     the byte-identical-output contract (RESEARCH §Pitfall 3) requires
//!     deterministic file order, and `glob`'s iteration order is undefined.
//!   - Empty match set → `bail!("no results found matching pattern \"{pat}\"")`.
//!   - For each path: try `Vec<Run>` first (Phase-3 dominant case per
//!     `crates/alloc-bench-cli/tests/run_all_smoke.rs:57`); on JSON-parse
//!     failure fall back to `Run` (Phase-1 single-scenario emission shape).
//!   - Schema-version mismatch → `bail!` with the offending path.
//!   - Per-file failure → `eprintln!("warn: skipped {}: {}", ...)` and push
//!     a `SkippedFile` to the outcome's `skipped` list. Discovery NEVER
//!     fails-fast (D-08 — skip-and-continue).
//!
//! Phase-5 D-13 / D-14 / D-20 / RESEARCH §Pattern 4 — sidecar `meta.json`:
//!   - `CellMeta` carries the per-cell image-size + build-time backfill
//!     emitted by `just ci-bench-cell` (Plan 02). v1 input schema is NOT
//!     modified — the meta merge happens at REPORT.md emit time.
//!   - `load_cell_metas(pattern)` returns an empty `HashMap` when the
//!     pattern is empty (default `--meta` flag value). Otherwise globs +
//!     sorts + parses each file; per-file failures log a `warn:` line on
//!     stderr and skip-and-continue (matches `discover` behavior).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use alloc_bench_core::output::Run;
use alloc_bench_core::SCHEMA_VERSION;
use anyhow::{bail, Context, Result};
use glob::glob;
use serde::Deserialize;

/// Result of a discover() pass.
#[derive(Debug)]
pub struct LoadOutcome {
    pub runs: Vec<Run>,
    pub skipped: Vec<SkippedFile>,
}

/// Per-file failure record (D-08); markdown.rs renders these into a
/// "Skipped inputs" section so the user has visibility.
#[derive(Debug)]
pub struct SkippedFile {
    pub path: PathBuf,
    pub reason: String,
}

/// Per-cell metadata sidecar (D-13 / D-14). Emitted by Plan 02's
/// `just ci-bench-cell` recipe via `docker image inspect --format '{{.Size}}'`
/// after each cell build. Keeps the locked v1 input schema (Phase 1 D-11/D-12)
/// untouched — the meta carries the post-build backfill that the aggregator
/// merges at REPORT.md emit time.
///
/// Fields `build_time_s` and `captured_at` are optional so older sidecars
/// produced before those fields were added still parse cleanly.
#[derive(Debug, Deserialize)]
pub struct CellMeta {
    pub alloc: String,
    pub env: String,
    #[allow(dead_code)] // Reserved for v2 (informational; not rendered today).
    pub image_size_bytes: u64,
    pub image_size_mb: f64,
    #[allow(dead_code)] // Reserved for v2 (Docker runtimes table column not yet emitted).
    pub build_time_s: Option<f64>,
    #[allow(dead_code)] // Reserved for v2 (provenance; not rendered today).
    pub captured_at: Option<String>,
}

/// Load per-cell meta sidecars by globbing `pattern`. Empty pattern →
/// empty map (no error). Per-file parse failures log to stderr and are
/// skipped (matches `discover`'s skip-and-continue contract).
///
/// The map is keyed by `(alloc, env)` exactly as written in the sidecar.
/// `env` is the **short** env name (e.g. `"alpine"`) — NOT the full
/// `docker_image` tag. Callers that need to join against a Run's
/// `env.docker_image` (e.g. `"alloc-bench:jemalloc-alpine"`) compose
/// the join themselves; see `markdown.rs::emit_docker_runtimes_table`.
pub fn load_cell_metas(pattern: &str) -> Result<HashMap<(String, String), CellMeta>> {
    if pattern.is_empty() {
        return Ok(HashMap::new());
    }
    let mut paths: Vec<PathBuf> = glob(pattern)
        .with_context(|| format!("invalid meta glob pattern: {pattern}"))?
        .filter_map(|r| r.ok())
        .collect();
    paths.sort_unstable();

    let mut map: HashMap<(String, String), CellMeta> = HashMap::new();
    for path in paths {
        match load_one_meta(&path) {
            Ok(meta) => {
                map.insert((meta.alloc.clone(), meta.env.clone()), meta);
            }
            Err(e) => {
                eprintln!("warn: skipped meta {}: {}", path.display(), e);
            }
        }
    }
    Ok(map)
}

fn load_one_meta(path: &Path) -> Result<CellMeta> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let meta: CellMeta = serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing meta {}", path.display()))?;
    Ok(meta)
}

/// Glob the pattern, sort the matches lexicographically, then parse each
/// matched JSON file. Returns aggregated runs + per-file failures.
///
/// Errors only on:
///   - Invalid glob pattern (the `glob::glob()` call itself errors).
///   - Zero matches (bail with the pattern).
pub fn discover(pattern: &str) -> Result<LoadOutcome> {
    let mut paths: Vec<PathBuf> = glob(pattern)
        .with_context(|| format!("invalid glob pattern: {pattern}"))?
        .filter_map(|r| r.ok())
        .collect();
    if paths.is_empty() {
        bail!("no results found matching pattern \"{pattern}\"");
    }
    // RESEARCH §Pitfall 3: glob's iteration order is undefined. Sort here
    // so the byte-identical-output contract holds.
    paths.sort_unstable();

    let mut runs = Vec::new();
    let mut skipped = Vec::new();
    for path in paths {
        match load_one(&path) {
            Ok(mut more) => runs.append(&mut more),
            Err(e) => {
                eprintln!("warn: skipped {}: {}", path.display(), e);
                skipped.push(SkippedFile {
                    path,
                    reason: e.to_string(),
                });
            }
        }
    }
    Ok(LoadOutcome { runs, skipped })
}

fn load_one(path: &Path) -> Result<Vec<Run>> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;

    // RESEARCH §Pattern 2: try array first (Phase-3 `run-all` shape — see
    // crates/alloc-bench-cli/tests/run_all_smoke.rs:57). This is the
    // dominant case in production results/{alloc}-{env}.json files.
    if let Ok(arr) = serde_json::from_slice::<Vec<Run>>(&bytes) {
        for r in &arr {
            if r.schema_version != SCHEMA_VERSION {
                bail!(
                    "schema_version mismatch in {}: got {}, expected {}",
                    path.display(),
                    r.schema_version,
                    SCHEMA_VERSION
                );
            }
        }
        return Ok(arr);
    }
    // Fallback: single Run object — Phase-1 single-scenario emission shape.
    let single: Run = serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing {} (tried Vec<Run> then Run)", path.display()))?;
    if single.schema_version != SCHEMA_VERSION {
        bail!(
            "schema_version mismatch in {}: got {}, expected {}",
            path.display(),
            single.schema_version,
            SCHEMA_VERSION
        );
    }
    Ok(vec![single])
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc_bench_core::output::{
        Build, Env, HarnessInfo, LatencyNs, Metrics, Rusage, ScenarioInfo,
    };

    fn make_synthetic_run(scenario_name: &str) -> Run {
        Run {
            schema_version: SCHEMA_VERSION,
            run_id: format!("synth-{scenario_name}"),
            env: Env {
                os: "linux".into(),
                os_version: "test".into(),
                docker_image: None,
                cpu_model: "test-cpu".into(),
                cpu_count: 1,
                memory_total_kb: 1,
            },
            build: Build {
                allocator: "system".into(),
                rustc_version: "1.83.0".into(),
                target_triple: "x86_64-unknown-linux-gnu".into(),
                host_triple: "x86_64-unknown-linux-gnu".into(),
                profile: "release".into(),
                git_sha: "0".repeat(40),
                git_dirty: false,
                build_timestamp: "2026-05-19T00:00:00Z".into(),
                rustflags: "".into(),
            },
            scenario: ScenarioInfo {
                name: scenario_name.into(),
                config: serde_json::json!({}),
                unit: None,
            },
            harness: HarnessInfo {
                warmup_duration_s: 5.0,
                measurement_duration_s: 5.0,
                samples_count: 50_000,
            },
            metrics: Metrics {
                ticks_per_s: 100.0,
                allocations_per_tick: 100,
                tick_latency_ns: LatencyNs {
                    p50: 1000,
                    p95: 2000,
                    p99: 3000,
                    p999: 5000,
                    max: 10000,
                },
                peak_rss_kb: 1000,
                rss_growth_samples: vec![],
                rusage: Rusage {
                    user_time_s: 0.0,
                    sys_time_s: 0.0,
                    minor_faults: 0,
                    major_faults: 0,
                    voluntary_csw: 0,
                    involuntary_csw: 0,
                    peak_rss_kb: 1000,
                },
                allocator_stats: serde_json::json!({}),
            },
            status: Some("success".into()),
            error: None,
        }
    }

    /// RESEARCH §Pitfall 3: glob iteration order is undefined; we mandate
    /// sort_unstable. Two files written in `b.json`-then-`a.json` order
    /// must be processed in `a.json`-first order after discover.
    #[test]
    fn paths_sorted_lexicographically() {
        let dir = tempfile::tempdir().expect("tempdir");
        let a_path = dir.path().join("a.json");
        let b_path = dir.path().join("b.json");
        // Write b first so any non-sorting order would surface.
        std::fs::write(
            &b_path,
            serde_json::to_string(&vec![make_synthetic_run("from-b")]).unwrap(),
        )
        .unwrap();
        std::fs::write(
            &a_path,
            serde_json::to_string(&vec![make_synthetic_run("from-a")]).unwrap(),
        )
        .unwrap();

        let pattern = format!("{}/*.json", dir.path().display());
        let outcome = discover(&pattern).expect("discover");
        assert_eq!(outcome.runs.len(), 2);
        assert_eq!(outcome.runs[0].scenario.name, "from-a");
        assert_eq!(outcome.runs[1].scenario.name, "from-b");
    }

    /// Phase-3 dominant case: results/{alloc}-{env}.json is a Vec<Run>.
    #[test]
    fn vec_run_array_parses_as_n_runs() {
        let dir = tempfile::tempdir().expect("tempdir");
        let runs = vec![make_synthetic_run("scen-a"), make_synthetic_run("scen-b")];
        let path = dir.path().join("multi.json");
        std::fs::write(&path, serde_json::to_string(&runs).unwrap()).unwrap();

        let pattern = format!("{}/*.json", dir.path().display());
        let outcome = discover(&pattern).expect("discover");
        assert_eq!(outcome.runs.len(), 2);
        assert_eq!(outcome.skipped.len(), 0);
    }

    /// Phase-1 fallback: legacy single-scenario emission writes a single
    /// Run object, not an array. The loader's Vec<Run>-then-Run fallback
    /// must accept this shape.
    #[test]
    fn single_run_object_parses_as_one_run() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("single.json");
        std::fs::write(
            &path,
            serde_json::to_string(&make_synthetic_run("solo")).unwrap(),
        )
        .unwrap();

        let pattern = format!("{}/*.json", dir.path().display());
        let outcome = discover(&pattern).expect("discover");
        assert_eq!(outcome.runs.len(), 1);
        assert_eq!(outcome.runs[0].scenario.name, "solo");
    }

    /// D-06: schema_version mismatch is a hard reject (per file). The
    /// error message must name the offending file path AND state the
    /// expected version so the user can find the bad file.
    #[test]
    fn schema_version_mismatch_rejects_with_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("future.json");
        let mut bad = make_synthetic_run("future");
        bad.schema_version = 999;
        std::fs::write(&path, serde_json::to_string(&vec![bad]).unwrap()).unwrap();

        let pattern = format!("{}/*.json", dir.path().display());
        // discover never fails-fast; the bad file is skipped via the
        // load_one bail!. We assert the SkippedFile's reason carries the
        // expected diagnostic.
        let outcome = discover(&pattern).expect("discover");
        assert_eq!(outcome.runs.len(), 0);
        assert_eq!(outcome.skipped.len(), 1);
        let reason = &outcome.skipped[0].reason;
        assert!(reason.contains("future.json"), "got: {reason}");
        assert!(reason.contains("expected"), "got: {reason}");
        assert!(reason.contains('1'), "got: {reason}");
    }

    /// D-08: zero glob matches → bail! with the offending pattern in the
    /// error message so the user can fix the typo.
    #[test]
    fn glob_zero_matches_returns_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pattern = format!("{}/*.json", dir.path().display());
        let err = discover(&pattern).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("no results found"), "got: {msg}");
    }

    /// D-13 / D-14 / RESEARCH §Pattern 4: empty meta pattern → empty
    /// HashMap, no error. The default `--meta` value is empty so existing
    /// local `just aggregate` invocations continue to work unchanged.
    #[test]
    fn load_cell_metas_empty_pattern_returns_empty_map() {
        let metas = load_cell_metas("").expect("empty pattern is OK");
        assert!(metas.is_empty(), "empty pattern must yield empty map");
    }

    /// D-13: the multi-run fixture sidecar parses with the documented
    /// shape — keys `(alloc, env)` exactly as written; `image_size_mb`
    /// preserved as f64.
    #[test]
    fn load_cell_metas_parses_documented_fixture() {
        let fixtures =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/multi_run/meta");
        let pattern = format!("{}/*.json", fixtures.display());
        let metas = load_cell_metas(&pattern).expect("load_cell_metas");
        assert_eq!(metas.len(), 1, "expected exactly one fixture meta");
        let key = ("jemalloc".to_string(), "alpine".to_string());
        let meta = metas
            .get(&key)
            .expect("metas[(jemalloc, alpine)] should exist");
        assert!(
            (meta.image_size_mb - 26.55).abs() < 1e-6,
            "image_size_mb mismatch: {}",
            meta.image_size_mb
        );
    }

    /// D-08-style skip-and-continue: a malformed JSON file is logged on
    /// stderr and skipped; valid sidecars in the same glob still load.
    #[test]
    fn load_cell_metas_skips_malformed_json() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Good meta — minimal documented shape.
        std::fs::write(
            dir.path().join("good.json"),
            r#"{"alloc":"ptmalloc","env":"debian-slim","image_size_bytes":104857600,"image_size_mb":100.0}"#,
        )
        .unwrap();
        // Malformed meta — invalid JSON.
        std::fs::write(dir.path().join("bad.json"), "not-json: garbage").unwrap();

        let pattern = format!("{}/*.json", dir.path().display());
        let metas = load_cell_metas(&pattern).expect("load_cell_metas Ok");
        // Only the good meta survives.
        assert_eq!(metas.len(), 1, "expected exactly one valid meta");
        let key = ("ptmalloc".to_string(), "debian-slim".to_string());
        assert!(metas.contains_key(&key), "good meta must be present");
    }

    /// D-08: when one file in a multi-file glob fails to parse, the
    /// loader skips it (logging a stderr warning) and continues with
    /// the valid files. The exit path remains Ok with `skipped` populated.
    #[test]
    fn partial_failure_skips_and_continues() {
        let dir = tempfile::tempdir().expect("tempdir");
        let good = dir.path().join("good.json");
        let bad = dir.path().join("bad.json");
        std::fs::write(
            &good,
            serde_json::to_string(&vec![make_synthetic_run("ok")]).unwrap(),
        )
        .unwrap();
        std::fs::write(&bad, "not-json: garbage").unwrap();

        let pattern = format!("{}/*.json", dir.path().display());
        let outcome = discover(&pattern).expect("discover");
        assert_eq!(outcome.runs.len(), 1);
        assert_eq!(outcome.skipped.len(), 1);
        assert!(
            outcome.skipped[0]
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.ends_with("bad.json"))
                .unwrap_or(false),
            "skipped path should end with bad.json, got {:?}",
            outcome.skipped[0].path
        );
    }
}

//! Workload → allocator picker (D-12, AGG-07).
//!
//! Given the loaded `Run` set, return one `Recommendation` per workload
//! class. Each rationale string is data-derived from the measured runs:
//!
//!   - When two or more allocators have data for the class →
//!     `+{delta:.1}% throughput vs {runner_up} on {scenario}`.
//!   - When exactly one allocator has data → `(only X measured)` /
//!     `insufficient comparative data — only X measured`.
//!   - When no allocator has data → `—` / `no measurements`.
//!
//! Hard-coded prose is forbidden (RESEARCH §Pitfall 7) — every rationale
//! must be derivable from the input JSON. A unit-test suite gates the
//! contract (delta math, alphabetical class order, suspect-suffix
//! propagation, single-allocator fallback, zero-allocator branch,
//! channel-heavy mean-of-three logic, divide-by-zero guard).
//!
//! Ordering uses `BTreeMap` (alphabetical iteration) for the byte-
//! identical-output contract per RESEARCH §Pitfall 5; the public output
//! `Vec` is in the locked alphabetical class order:
//!     channel-heavy → contention → cpu-bound → fragmentation-prone →
//!     memory-bound → web-ser-de.
//!
use std::collections::BTreeMap;

use alloc_bench_core::output::Run;

use crate::html::is_suspect;

/// Public output row. Emitted into the REPORT.md `## Recommendations by
/// workload` table by `markdown::emit_recommendations`. The `class` field
/// is `&'static str` because it comes from `WorkloadClass::label()`; the
/// other two are owned `String`s assembled from per-run data.
#[derive(Debug, Clone, PartialEq)]
pub struct Recommendation {
    pub class: &'static str,
    pub allocator: String,
    pub rationale: String,
}

/// The locked workload class set (D-12, UI-SPEC §Copywriting Contract).
/// Variants are emitted alphabetically; `ALL_CLASSES` is the iteration
/// order used by `recommendations()`.
#[derive(Debug, Clone, Copy)]
enum WorkloadClass {
    ChannelHeavy,
    Contention,
    CpuBound,
    FragmentationProne,
    MemoryBound,
    WebSerDe,
}

impl WorkloadClass {
    /// Lowercase-hyphenated label used in the REPORT.md table cell.
    fn label(&self) -> &'static str {
        match self {
            Self::ChannelHeavy => "channel-heavy",
            Self::Contention => "contention",
            Self::CpuBound => "cpu-bound",
            Self::FragmentationProne => "fragmentation-prone",
            Self::MemoryBound => "memory-bound",
            Self::WebSerDe => "web-ser-de",
        }
    }

    /// Scenario names that map into this class. The aggregator filters
    /// runs by this set; an allocator with no runs in any scenario is
    /// "not measured for this class".
    fn scenarios(&self) -> &'static [&'static str] {
        match self {
            Self::ChannelHeavy => &["spmc", "mpsc", "mpmc"],
            Self::Contention => &["contention"],
            Self::CpuBound => &["cpu-bound"],
            Self::FragmentationProne => &["fragmentation-soak"],
            Self::MemoryBound => &["mem-bound"],
            Self::WebSerDe => &["web"],
        }
    }
}

/// Alphabetical iteration order; `recommendations()` returns one row per
/// class in this exact order so the REPORT.md table is byte-stable.
const ALL_CLASSES: [WorkloadClass; 6] = [
    WorkloadClass::ChannelHeavy,
    WorkloadClass::Contention,
    WorkloadClass::CpuBound,
    WorkloadClass::FragmentationProne,
    WorkloadClass::MemoryBound,
    WorkloadClass::WebSerDe,
];

/// Build per-class recommendations from the loaded Run set. Always
/// returns six rows (one per class) — never skips a class — so the
/// REPORT.md reader sees the full table even when data is missing.
pub fn recommendations(runs: &[Run]) -> Vec<Recommendation> {
    let mut out = Vec::with_capacity(ALL_CLASSES.len());
    for class in ALL_CLASSES.iter() {
        out.push(recommend_for_class(*class, runs));
    }
    out
}

/// Per-allocator measurement for one workload class. Holds the mean
/// throughput across whichever scenarios in `class.scenarios()` the
/// allocator was measured on, plus the per-scenario throughput map (used
/// to pick the "most representative" scenario name for the rationale
/// string when a class has multiple scenarios — channel-heavy).
///
/// WR-02 (Phase-04 review): owned-only, lifetime-free. Earlier revs
/// carried a vestigial `'a` parameter + `PhantomData<&'a Run>` field,
/// justified by a doc comment claiming the `&str` keys borrowed from
/// `class.scenarios()` — but the keys are `&'static str`, which has
/// nothing to do with any caller-provided lifetime.
struct AllocStats {
    allocator: String,
    score: f64,
    /// scenario_name → mean throughput on that scenario. Only contains
    /// scenarios in `class.scenarios()` that the allocator measured.
    /// `BTreeMap` so iteration is alphabetical (tie-break stability).
    per_scenario: BTreeMap<&'static str, f64>,
    /// Whether ANY of the contributing runs is suspect per `is_suspect`.
    any_suspect: bool,
}

fn recommend_for_class(class: WorkloadClass, runs: &[Run]) -> Recommendation {
    let scenarios = class.scenarios();
    // Group runs by allocator, restricted to scenarios in this class.
    // BTreeMap so iteration order is alphabetical.
    let mut by_alloc: BTreeMap<String, Vec<&Run>> = BTreeMap::new();
    for r in runs {
        if scenarios.contains(&r.scenario.name.as_str()) {
            by_alloc
                .entry(r.build.allocator.clone())
                .or_default()
                .push(r);
        }
    }

    if by_alloc.is_empty() {
        // No allocator measured for this class — em-dash + flat rationale.
        return Recommendation {
            class: class.label(),
            allocator: "\u{2014}".to_string(), // em-dash U+2014
            rationale: "no measurements".to_string(),
        };
    }

    // Build per-allocator AllocStats. Mean throughput across whichever
    // scenarios the allocator measured.
    let stats: Vec<AllocStats> = by_alloc
        .into_iter()
        .map(|(allocator, alloc_runs)| {
            // Per-scenario mean. (Schema is single-run-per-cell in v1; if
            // multiple runs share an alloc·scenario pair this averages.)
            let mut per_scenario: BTreeMap<&'static str, f64> = BTreeMap::new();
            let mut any_suspect = false;
            for &scen in scenarios.iter() {
                let matching: Vec<&&Run> = alloc_runs
                    .iter()
                    .filter(|r| r.scenario.name == scen)
                    .collect();
                if matching.is_empty() {
                    continue;
                }
                // D-11 (Plan 03): prefer median across runs (multi_run::aggregate);
                // fall back to mean for n<2 so existing single-run-per-cell
                // fixtures pick the same winner as in Phase 4. The mean fallback
                // arithmetic is identical to the previous code path.
                let throughputs: Vec<f64> =
                    matching.iter().map(|r| r.metrics.ticks_per_s).collect();
                let central_tendency = match crate::multi_run::aggregate(&throughputs) {
                    Some(stats) => stats.median,
                    None => throughputs.iter().sum::<f64>() / (throughputs.len() as f64).max(1.0),
                };
                per_scenario.insert(scen, central_tendency);
                if matching.iter().any(|r| is_suspect(&r.harness)) {
                    any_suspect = true;
                }
            }
            // Score = mean across whichever scenarios in `scenarios` were
            // measured. (Channel-heavy with 3 scenarios → mean of however
            // many the allocator hit.)
            let score = if per_scenario.is_empty() {
                0.0
            } else {
                per_scenario.values().sum::<f64>() / per_scenario.len() as f64
            };
            AllocStats {
                allocator,
                score,
                per_scenario,
                any_suspect,
            }
        })
        // Drop allocators whose runs were ALL outside `scenarios` (none
        // recorded in `per_scenario`) — they never measured this class.
        .filter(|s| !s.per_scenario.is_empty())
        .collect();

    if stats.is_empty() {
        // After filtering empty per_scenario rows, no measurements remain.
        return Recommendation {
            class: class.label(),
            allocator: "\u{2014}".to_string(),
            rationale: "no measurements".to_string(),
        };
    }

    if stats.len() == 1 {
        // Single-allocator fallback (D-12, RESEARCH §Pitfall 7). Still emit
        // the row so REPORT.md has all six entries.
        let only = &stats[0];
        return Recommendation {
            class: class.label(),
            allocator: format!("(only {} measured)", only.allocator),
            rationale: format!(
                "insufficient comparative data — only {} measured",
                only.allocator
            ),
        };
    }

    // Sort allocator-stats by score DESCENDING. Stable sort preserves
    // alphabetical (BTreeMap iteration) tiebreak.
    let mut sorted = stats;
    sorted.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let winner = &sorted[0];
    let runner_up = &sorted[1];

    // Pick the rationale scenario. Single-scenario class → that scenario.
    // Multi-scenario class → the scenario where the WINNER recorded its
    // maximum throughput (most representative win); alphabetical tiebreak
    // via BTreeMap iteration order.
    let scenario = pick_rationale_scenario(scenarios, &winner.per_scenario);

    let delta = if runner_up.score > 0.0 {
        ((winner.score - runner_up.score) / runner_up.score) * 100.0
    } else {
        0.0
    };

    let mut rationale = format!(
        "+{:.1}% throughput vs {} on {}",
        delta, runner_up.allocator, scenario
    );
    if winner.any_suspect || runner_up.any_suspect {
        rationale.push_str(" *(suspect)*");
    }

    Recommendation {
        class: class.label(),
        allocator: winner.allocator.clone(),
        rationale,
    }
}

/// Pick the scenario name to cite in a multi-scenario class's rationale.
/// Single-scenario classes skip the search; multi-scenario classes pick
/// the scenario where the winner recorded its highest throughput.
fn pick_rationale_scenario(
    scenarios: &'static [&'static str],
    per_scenario: &BTreeMap<&'static str, f64>,
) -> &'static str {
    if scenarios.len() == 1 {
        return scenarios[0];
    }
    // Find the (scenario, throughput) pair with the maximum throughput.
    // BTreeMap iteration is alphabetical — gives a stable tiebreak.
    let mut best: Option<(&'static str, f64)> = None;
    for (&scen, &tps) in per_scenario.iter() {
        match best {
            Some((_, b)) if tps <= b => {}
            _ => best = Some((scen, tps)),
        }
    }
    // Fallback: alphabetically-first scenario in the class.
    best.map(|(s, _)| s).unwrap_or(scenarios[0])
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc_bench_core::output::{
        Build, Env, HarnessInfo, LatencyNs, Metrics, Rusage, ScenarioInfo,
    };
    use alloc_bench_core::SCHEMA_VERSION;

    /// Synthetic `Run` builder. Builds a minimal v1-schema-compliant Run
    /// with the given allocator/scenario/throughput/harness fields and
    /// sentinel values everywhere else.
    fn synth_run(
        alloc: &str,
        scenario: &str,
        ticks_per_s: f64,
        samples_count: u64,
        warmup_s: f64,
    ) -> Run {
        Run {
            schema_version: SCHEMA_VERSION,
            run_id: format!("synth-{alloc}-{scenario}"),
            env: Env {
                os: "linux".into(),
                os_version: "test".into(),
                docker_image: None,
                cpu_model: "test-cpu".into(),
                cpu_count: 1,
                memory_total_kb: 1,
            },
            build: Build {
                allocator: alloc.into(),
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
                name: scenario.into(),
                config: serde_json::json!({}),
                unit: None,
            },
            harness: HarnessInfo {
                warmup_duration_s: warmup_s,
                measurement_duration_s: 5.0,
                samples_count,
            },
            metrics: Metrics {
                ticks_per_s,
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

    fn cpu_bound_recommendation(recs: &[Recommendation]) -> &Recommendation {
        recs.iter()
            .find(|r| r.class == "cpu-bound")
            .expect("cpu-bound recommendation present")
    }

    #[test]
    fn winner_picker_emits_data_derived_rationale_two_allocators() {
        let runs = vec![
            synth_run("jemalloc", "cpu-bound", 100.0, 50_000, 5.0),
            synth_run("ptmalloc", "cpu-bound", 80.0, 50_000, 5.0),
        ];
        let recs = recommendations(&runs);
        let r = cpu_bound_recommendation(&recs);
        assert_eq!(r.allocator, "jemalloc");
        // (100 - 80) / 80 * 100 = +25.0%
        assert!(
            r.rationale
                .starts_with("+25.0% throughput vs ptmalloc on cpu-bound"),
            "rationale was {:?}",
            r.rationale
        );
    }

    #[test]
    fn winner_picker_three_allocators_picks_top_with_runner_up() {
        let runs = vec![
            synth_run("jemalloc", "cpu-bound", 100.0, 50_000, 5.0),
            synth_run("mimalloc", "cpu-bound", 110.0, 50_000, 5.0),
            synth_run("ptmalloc", "cpu-bound", 80.0, 50_000, 5.0),
        ];
        let recs = recommendations(&runs);
        let r = cpu_bound_recommendation(&recs);
        assert_eq!(r.allocator, "mimalloc");
        // (110 - 100) / 100 * 100 = +10.0%; runner-up is jemalloc, NOT ptmalloc.
        assert!(
            r.rationale
                .starts_with("+10.0% throughput vs jemalloc on cpu-bound"),
            "rationale was {:?}",
            r.rationale
        );
    }

    #[test]
    fn winner_picker_single_allocator_fallback() {
        let runs = vec![synth_run("ptmalloc", "web", 100.0, 50_000, 5.0)];
        let recs = recommendations(&runs);
        let r = recs
            .iter()
            .find(|r| r.class == "web-ser-de")
            .expect("web-ser-de present");
        assert_eq!(r.allocator, "(only ptmalloc measured)");
        assert_eq!(
            r.rationale,
            "insufficient comparative data — only ptmalloc measured"
        );
    }

    #[test]
    fn winner_picker_no_runs_for_class_emits_no_measurements() {
        let runs: Vec<Run> = vec![];
        let recs = recommendations(&runs);
        assert_eq!(recs.len(), 6);
        for r in &recs {
            assert_eq!(
                r.allocator, "\u{2014}",
                "class {} should be em-dash",
                r.class
            );
            assert_eq!(r.rationale, "no measurements", "class {}", r.class);
        }
    }

    #[test]
    fn winner_picker_alphabetical_class_order() {
        let runs = vec![
            synth_run("jemalloc", "cpu-bound", 100.0, 50_000, 5.0),
            synth_run("mimalloc", "cpu-bound", 110.0, 50_000, 5.0),
        ];
        let recs = recommendations(&runs);
        assert_eq!(recs.len(), 6);
        assert_eq!(recs[0].class, "channel-heavy");
        assert_eq!(recs[1].class, "contention");
        assert_eq!(recs[2].class, "cpu-bound");
        assert_eq!(recs[3].class, "fragmentation-prone");
        assert_eq!(recs[4].class, "memory-bound");
        assert_eq!(recs[5].class, "web-ser-de");
    }

    #[test]
    fn winner_picker_suspect_winner_appends_suspect_suffix() {
        let runs = vec![
            // jemalloc winner with samples_count=5000 → suspect (low samples).
            synth_run("jemalloc", "cpu-bound", 100.0, 5_000, 5.0),
            synth_run("ptmalloc", "cpu-bound", 80.0, 50_000, 5.0),
        ];
        let recs = recommendations(&runs);
        let r = cpu_bound_recommendation(&recs);
        assert!(
            r.rationale.ends_with("*(suspect)*"),
            "rationale was {:?}",
            r.rationale
        );
    }

    #[test]
    fn winner_picker_suspect_runner_up_also_appends_suffix() {
        let runs = vec![
            synth_run("jemalloc", "cpu-bound", 100.0, 50_000, 5.0),
            // ptmalloc runner-up with warmup_duration_s=2.0 → suspect (short warmup).
            synth_run("ptmalloc", "cpu-bound", 80.0, 50_000, 2.0),
        ];
        let recs = recommendations(&runs);
        let r = cpu_bound_recommendation(&recs);
        assert!(
            r.rationale.ends_with("*(suspect)*"),
            "rationale was {:?}",
            r.rationale
        );
    }

    #[test]
    fn winner_picker_channel_heavy_means_three_scenarios() {
        let runs = vec![
            synth_run("jemalloc", "spmc", 100.0, 50_000, 5.0),
            synth_run("jemalloc", "mpsc", 200.0, 50_000, 5.0),
            synth_run("jemalloc", "mpmc", 300.0, 50_000, 5.0),
            synth_run("ptmalloc", "spmc", 50.0, 50_000, 5.0),
            synth_run("ptmalloc", "mpsc", 100.0, 50_000, 5.0),
            synth_run("ptmalloc", "mpmc", 150.0, 50_000, 5.0),
        ];
        let recs = recommendations(&runs);
        let r = recs
            .iter()
            .find(|r| r.class == "channel-heavy")
            .expect("channel-heavy present");
        assert_eq!(r.allocator, "jemalloc");
        // jemalloc mean = (100+200+300)/3 = 200; ptmalloc mean = (50+100+150)/3 = 100.
        // delta = (200 - 100) / 100 * 100 = +100.0%. winner-max scenario = mpmc (300).
        assert!(
            r.rationale
                .starts_with("+100.0% throughput vs ptmalloc on mpmc"),
            "rationale was {:?}",
            r.rationale
        );
    }

    /// D-11 (Plan 03 Task 3): when 3 seeded runs are present per
    /// `(alloc, scenario)` cell, the winner picker uses median across the
    /// runs as central tendency — not mean.
    ///
    /// Fixture: jemalloc-cpu-bound seeds [10, 100, 110] (one outlier low).
    /// - mean(jemalloc) = (10 + 100 + 110) / 3 = 73.33 → loses to ptmalloc 50.
    /// - median(jemalloc) = 100 → wins vs ptmalloc 50.
    ///
    /// Asserts the recommendation picks jemalloc (median-driven), proving
    /// the multi_run::aggregate central-tendency swap is wired.
    #[test]
    fn winner_picker_uses_median_when_three_seeds_present() {
        let runs = vec![
            // jemalloc-cpu-bound: 3 seeds with a low outlier (mean=73.3, median=100).
            synth_run("jemalloc", "cpu-bound", 10.0, 50_000, 5.0),
            synth_run("jemalloc", "cpu-bound", 100.0, 50_000, 5.0),
            synth_run("jemalloc", "cpu-bound", 110.0, 50_000, 5.0),
            // ptmalloc-cpu-bound: 3 stable seeds at 50.0 (mean=50, median=50).
            synth_run("ptmalloc", "cpu-bound", 50.0, 50_000, 5.0),
            synth_run("ptmalloc", "cpu-bound", 50.0, 50_000, 5.0),
            synth_run("ptmalloc", "cpu-bound", 50.0, 50_000, 5.0),
        ];
        let recs = recommendations(&runs);
        let r = cpu_bound_recommendation(&recs);
        assert_eq!(
            r.allocator, "jemalloc",
            "median-driven central tendency should pick jemalloc; mean-driven would pick ptmalloc. Got: {r:?}"
        );
    }

    #[test]
    fn winner_picker_handles_zero_throughput_runner_up_without_div_by_zero() {
        let runs = vec![
            synth_run("jemalloc", "cpu-bound", 100.0, 50_000, 5.0),
            synth_run("ptmalloc", "cpu-bound", 0.0, 50_000, 5.0),
        ];
        let recs = recommendations(&runs);
        let r = cpu_bound_recommendation(&recs);
        // Guard kicks in: runner_up.score == 0.0 → delta = 0.0. No panic.
        assert!(
            r.rationale
                .starts_with("+0.0% throughput vs ptmalloc on cpu-bound"),
            "rationale was {:?}",
            r.rationale
        );
    }
}

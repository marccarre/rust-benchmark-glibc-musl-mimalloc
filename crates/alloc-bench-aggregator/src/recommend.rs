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

use crate::axes::MEASUREMENT_AXES;
use crate::html::is_suspect;
// `use crate::score::CellScore;` — added in Task 2 alongside `top_n_cells`
// (the only consumer). Task 1 ships the prose-derivation helpers + struct
// definition that are CellScore-free.

// ----------------------------------------------------------------------
// Phase 7 / Plan 02 / REC-02 — top-N named constants (single source of
// truth shared with Phase 8 templates and Phase 9 polar.rs). No magic
// numbers in templates.
// ----------------------------------------------------------------------

/// Top-3 cells overlaid on the Phase 9 spider chart (small-multiples
/// grid above the fold). Phase 9 polar.rs uses
/// `score::top_n(scores, TOP_N_SPIDER)` to skip prose computation on
/// the chart hot path.
pub const TOP_N_SPIDER: usize = 3;

/// Top-5 cells in the above-the-fold REPORT.md table (Phase 8).
pub const TOP_N_TABLE: usize = 5;

/// Total cards / fragments emitted by Phase 8 templates
/// (`recommend-cell.{md,html}.tmpl`). Also pins the body length of
/// `top_n_cells`: `min(TOP_N_TOTAL, scores.len())`.
pub const TOP_N_TOTAL: usize = 10;

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

/// Phase 7 / REC-01 — prose-decorated top-N row. All five prose fields
/// (`tldr`, `strengths`, `weaknesses`, `recommended_for`, `avoid_for`)
/// are axis-derived; no hand-curated lookup table is consulted.
///
/// Field semantics (locked in 07-CONTEXT and 07-RESEARCH §2):
///   - `rank`: 1-indexed position after `(composite DESC, alloc ASC,
///     env ASC)` sort.
///   - `composite_score`: copied from the source `CellScore.composite`.
///   - `axes`: copied from `CellScore.axes` — `BTreeMap` keeps
///     alphabetical iteration matching `MEASUREMENT_AXES` declaration
///     order (CLAUDE.md byte-identical-output discipline).
///   - `tldr`: single sentence, `format_tldr`-derived.
///   - `strengths`: top-2 `MEASUREMENT_AXES[i].label` strings (DESC by
///     score, alphabetical tiebreak on key).
///   - `weaknesses`: bottom-2 labels (ASC by score, same tiebreak).
///   - `recommended_for`: alphabetical class labels where this `(alloc,
///     env)` cell is the per-class winner. Reuses winner detection
///     from the existing `recommendations()` logic but at `(alloc,
///     env)` granularity (not the `String` allocator name).
///   - `avoid_for`: alphabetical class labels where this cell is in
///     the bottom 2 of the per-class ranking.
///   - `suspect_flag`: OR aggregation of `html::is_suspect` over every
///     run contributing to this cell.
#[derive(Debug, Clone, PartialEq)]
pub struct CellRecommendation {
    pub rank: usize,
    pub alloc: String,
    pub env: String,
    pub composite_score: f64,
    pub axes: BTreeMap<&'static str, f64>,
    pub tldr: String,
    pub strengths: Vec<&'static str>,
    pub weaknesses: Vec<&'static str>,
    pub recommended_for: Vec<&'static str>,
    pub avoid_for: Vec<&'static str>,
    pub suspect_flag: bool,
}

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

// ----------------------------------------------------------------------
// Phase 7 / Plan 02 / Task 1 — REC-01 prose-derivation helpers.
//
// All four helpers are private to this module. Task 2 stitches them into
// the public `top_n_cells` entrypoint.
// ----------------------------------------------------------------------

/// REC-01 / suspect-flag aggregation. Short-circuit OR over
/// `html::is_suspect`. Generic `IntoIterator<Item = &Run>` form so
/// callers can pass a `&[Run]` slice OR a `Vec<&Run>` of borrows
/// (Run does not derive Clone per the v1 schema GUARD-01). `&[Run]`
/// satisfies the bound via the standard library's
/// `impl<'a, T> IntoIterator for &'a [T]`.
fn cell_is_suspect<'a, I: IntoIterator<Item = &'a Run>>(cell_runs: I) -> bool {
    cell_runs.into_iter().any(|r| is_suspect(&r.harness))
}

/// REC-01 — top-2 axis LABELS by normalized score DESCENDING with
/// alphabetical tiebreak on `axis_key`. Returns
/// `MEASUREMENT_AXES[i].label` (NOT `key`) — these strings are
/// human-facing prose consumed by the Phase 8 templates.
///
/// Defensive: if `axes` has fewer than 2 entries, returns whatever is
/// available (this path is unreachable for production data because
/// `compute_axes` always emits all 8 keys, but it guards test fixtures
/// with sparse axes).
fn derive_strengths(axes: &BTreeMap<&'static str, f64>) -> Vec<&'static str> {
    let mut pairs: Vec<(&'static str, f64)> = axes.iter().map(|(k, v)| (*k, *v)).collect();
    // DESC by score; ASC by key on tie (NaN → Equal, falls through to key sort).
    pairs.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(b.0))
    });
    pairs
        .into_iter()
        .take(2)
        .map(|(key, _)| {
            MEASUREMENT_AXES
                .iter()
                .find(|s| s.key == key)
                .map(|s| s.label)
                .unwrap_or("unknown")
        })
        .collect()
}

/// REC-01 — bottom-2 axis LABELS by normalized score ASCENDING with
/// alphabetical tiebreak on `axis_key`. Mirror of `derive_strengths`
/// with inverted primary sort.
fn derive_weaknesses(axes: &BTreeMap<&'static str, f64>) -> Vec<&'static str> {
    let mut pairs: Vec<(&'static str, f64)> = axes.iter().map(|(k, v)| (*k, *v)).collect();
    // ASC by score; ASC by key on tie.
    pairs.sort_by(|a, b| {
        a.1.partial_cmp(&b.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(b.0))
    });
    pairs
        .into_iter()
        .take(2)
        .map(|(key, _)| {
            MEASUREMENT_AXES
                .iter()
                .find(|s| s.key == key)
                .map(|s| s.label)
                .unwrap_or("unknown")
        })
        .collect()
}

/// REC-01 — single-sentence TLDR. Locked shape:
///   `"{alloc}/{env} \u{2014} strong on {top_strength}, weak on {bottom_weakness}."`
/// Em-dash glyph U+2014 matches the existing recommend.rs:144 / 205
/// "no measurements" path. Defensive: if `strengths`/`weaknesses` is
/// empty, the literal `"insufficient data"` is substituted (unreachable
/// for production data — `compute_axes` always emits 8 keys — but
/// guards sparse test fixtures).
fn format_tldr(
    alloc: &str,
    env: &str,
    strengths: &[&'static str],
    weaknesses: &[&'static str],
) -> String {
    let s = strengths.first().copied().unwrap_or("insufficient data");
    let w = weaknesses.first().copied().unwrap_or("insufficient data");
    format!("{}/{} \u{2014} strong on {}, weak on {}.", alloc, env, s, w)
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
            // jemalloc winner with samples_count=500 → suspect (low samples, < 1_000).
            synth_run("jemalloc", "cpu-bound", 100.0, 500, 5.0),
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

    // ------------------------------------------------------------------
    // Phase 7 / Plan 02 / Task 1 tests (REC-01 helpers + REC-02 constants)
    //
    // Six tests cover:
    //   - top_n_constants_match_locked_values            (REC-02)
    //   - cell_recommendation_strengths_top_2_alphabetical_tiebreak (REC-01)
    //   - cell_recommendation_weaknesses_bottom_2_alphabetical_tiebreak
    //   - cell_recommendation_tldr_is_templated_one_sentence
    //   - cell_recommendation_suspect_flag_true_when_any_run_suspect
    //   - cell_recommendation_suspect_flag_false_when_all_runs_healthy
    //
    // Helper `synth_run` (lines 297-362) is reused.
    // ------------------------------------------------------------------

    /// REC-02: the three top-N constants are pinned to the locked values
    /// `3 / 5 / 10` (Phase 9 polar.rs spider; Phase 8 above-the-fold table;
    /// Phase 8 total cards/fragments). Any drift breaks downstream consumers.
    #[test]
    fn top_n_constants_match_locked_values() {
        assert_eq!(TOP_N_SPIDER, 3);
        assert_eq!(TOP_N_TABLE, 5);
        assert_eq!(TOP_N_TOTAL, 10);
    }

    /// REC-01 helper: `derive_strengths` returns the top-2 axes by score
    /// DESCENDING with alphabetical tiebreak on `axis_key`. Two axes tied at
    /// 95.0 (`channel_throughput` and `web_throughput`) — alphabetical sort
    /// on the keys (NOT the labels) places `channel_throughput` first. The
    /// returned vector stores `MEASUREMENT_AXES[i].label` strings, NOT keys.
    #[test]
    fn cell_recommendation_strengths_top_2_alphabetical_tiebreak() {
        let mut axes: BTreeMap<&'static str, f64> = BTreeMap::new();
        axes.insert("channel_throughput", 95.0); // tied top
        axes.insert("cpu_bound_throughput", 50.0);
        axes.insert("image_size_efficiency", 50.0);
        axes.insert("memory_fragmentation", 50.0);
        axes.insert("multithread_throughput", 50.0);
        axes.insert("resilience", 50.0);
        axes.insert("security_posture", 50.0);
        axes.insert("web_throughput", 95.0); // tied top
        let out = derive_strengths(&axes);
        assert_eq!(
            out,
            vec!["Channel throughput", "Web throughput"],
            "tied tops break alphabetically on axis_key (channel < web)"
        );
    }

    /// REC-01 helper: `derive_weaknesses` returns the bottom-2 axes by score
    /// ASCENDING with alphabetical tiebreak on `axis_key`. Two axes tied at
    /// 5.0 (`cpu_bound_throughput` and `multithread_throughput`) — keys sort
    /// `cpu_bound_throughput` < `multithread_throughput`, so labels appear in
    /// the order "CPU-bound throughput", "Multithread throughput".
    #[test]
    fn cell_recommendation_weaknesses_bottom_2_alphabetical_tiebreak() {
        let mut axes: BTreeMap<&'static str, f64> = BTreeMap::new();
        axes.insert("channel_throughput", 50.0);
        axes.insert("cpu_bound_throughput", 5.0); // tied bottom
        axes.insert("image_size_efficiency", 50.0);
        axes.insert("memory_fragmentation", 50.0);
        axes.insert("multithread_throughput", 5.0); // tied bottom
        axes.insert("resilience", 50.0);
        axes.insert("security_posture", 50.0);
        axes.insert("web_throughput", 50.0);
        let out = derive_weaknesses(&axes);
        assert_eq!(
            out,
            vec!["CPU-bound throughput", "Multithread throughput"],
            "tied bottoms break alphabetically on axis_key"
        );
    }

    /// REC-01 helper: `format_tldr` produces exactly the locked single
    /// sentence shape: `"{alloc}/{env} \u{2014} strong on {top_strength}, weak on {bottom_weakness}."`.
    /// Em-dash glyph U+2014 (matches recommend.rs:144 / line 205 existing
    /// usage). Trailing period; comma between strong/weak. Strengths use
    /// LABELS (e.g. "CPU-bound throughput") not keys.
    #[test]
    fn cell_recommendation_tldr_is_templated_one_sentence() {
        let mut axes: BTreeMap<&'static str, f64> = BTreeMap::new();
        axes.insert("channel_throughput", 50.0);
        axes.insert("cpu_bound_throughput", 95.0); // top strength
        axes.insert("image_size_efficiency", 50.0);
        axes.insert("memory_fragmentation", 5.0); // bottom weakness
        axes.insert("multithread_throughput", 50.0);
        axes.insert("resilience", 50.0);
        axes.insert("security_posture", 50.0);
        axes.insert("web_throughput", 50.0);
        let strengths = derive_strengths(&axes);
        let weaknesses = derive_weaknesses(&axes);
        let out = format_tldr("jemalloc", "alpine", &strengths, &weaknesses);
        assert_eq!(
            out,
            "jemalloc/alpine \u{2014} strong on CPU-bound throughput, weak on Memory / fragmentation."
        );
    }

    /// REC-01 helper: `cell_is_suspect` returns true when ANY run in the
    /// cell trips the v1.0 `is_suspect` threshold (samples_count < 1_000 OR
    /// warmup_duration_s < 5.0). Mirrors the existing recommend_for_class
    /// suspect-suffix aggregation pattern at line 177.
    ///
    /// Iterator-form invocation (`runs.iter()`) exercises the generic
    /// `IntoIterator<Item = &Run>` signature locked in the plan interfaces.
    #[test]
    fn cell_recommendation_suspect_flag_true_when_any_run_suspect() {
        let runs = vec![
            // jemalloc-cpu-bound on alpine: low samples (500 < 1_000) → suspect.
            synth_run("jemalloc", "cpu-bound", 100.0, 500, 5.0),
            // jemalloc-web on alpine: healthy.
            synth_run("jemalloc", "web", 1500.0, 50_000, 5.0),
        ];
        // Iterator form (yields &Run).
        assert!(cell_is_suspect(runs.iter()));
        // Slice form (also satisfies the bound).
        assert!(cell_is_suspect(&runs[..]));
    }

    /// REC-01 helper: `cell_is_suspect` returns false when ALL runs in the
    /// cell are healthy (samples ≥ 1_000 AND warmup ≥ 5.0).
    #[test]
    fn cell_recommendation_suspect_flag_false_when_all_runs_healthy() {
        let runs = vec![
            synth_run("jemalloc", "cpu-bound", 100.0, 50_000, 5.0),
            synth_run("jemalloc", "web", 1500.0, 50_000, 5.0),
        ];
        assert!(!cell_is_suspect(runs.iter()));
        assert!(!cell_is_suspect(&runs[..]));
    }

    // ------------------------------------------------------------------
    // Phase 7 / Plan 02 / Task 2 tests (REC-01 integration — top_n_cells
    // and the winners_by_class / losers_by_class / env_short_name
    // helpers).
    //
    // Five tests:
    //   - cell_recommendation_populates_all_fields_from_axes
    //   - cell_recommendation_recommended_for_uses_existing_winners
    //   - cell_recommendation_avoid_for_is_bottom_2_class_rankings
    //   - top_n_cells_truncates_to_top_n_total_constant
    //   - top_n_cells_handles_fewer_than_top_n_total_input
    // ------------------------------------------------------------------

    use crate::score::CellScore as TestCellScore;

    /// Build a `Run` whose `env.docker_image` is set to
    /// `"alloc-bench:{alloc}-{env}"` so `recommend.rs::env_short_name` can
    /// extract the short env name (matches the format produced by the
    /// `justfile` Docker tagging recipe). All other fields are sentinel
    /// values; harness defaults to healthy (50_000 samples, 5.0 s warmup).
    fn synth_run_with_env(
        alloc: &str,
        env: &str,
        scenario: &str,
        throughput: f64,
        samples: u64,
        warmup: f64,
    ) -> Run {
        Run {
            schema_version: alloc_bench_core::SCHEMA_VERSION,
            run_id: format!("synth-{alloc}-{env}-{scenario}"),
            env: Env {
                os: "linux".into(),
                os_version: "test".into(),
                docker_image: Some(format!("alloc-bench:{alloc}-{env}")),
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
                warmup_duration_s: warmup,
                measurement_duration_s: 5.0,
                samples_count: samples,
            },
            metrics: Metrics {
                ticks_per_s: throughput,
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

    /// Build a `BTreeMap<&'static str, f64>` with all 8 axis keys populated
    /// from `[(key, value); 8]` overrides. Keys must be a subset of
    /// `MEASUREMENT_AXES`; missing keys default to 0.0.
    fn build_axes_btreemap(values: &[(&'static str, f64)]) -> BTreeMap<&'static str, f64> {
        let mut axes: BTreeMap<&'static str, f64> = BTreeMap::new();
        for spec in MEASUREMENT_AXES.iter() {
            axes.insert(spec.key, 0.0);
        }
        for (k, v) in values {
            axes.insert(*k, *v);
        }
        axes
    }

    /// Synthesize a `CellScore` with the given alloc/env/composite and a
    /// uniform per-axis score `axis_v`. Used by the truncation tests where
    /// per-axis values do not matter — only ranking does.
    fn synth_cell_score_uniform(
        alloc: &str,
        env: &str,
        composite: f64,
        axis_v: f64,
    ) -> TestCellScore {
        TestCellScore {
            alloc: alloc.into(),
            env: env.into(),
            composite,
            axes: build_axes_btreemap(&MEASUREMENT_AXES.map(|s| (s.key, axis_v))),
        }
    }

    /// REC-01 integration: `top_n_cells(scores, runs)[0]` populates all 11
    /// fields. We exercise the full surface — composite copy, axes copy,
    /// non-empty prose strings, strengths/weaknesses length 2, and a
    /// well-typed `suspect_flag`.
    #[test]
    fn cell_recommendation_populates_all_fields_from_axes() {
        // Single 1-cell fixture is sufficient to verify field plumbing.
        // Spike one strong axis and one weak axis so strengths[0] /
        // weaknesses[0] are deterministic.
        let axes = build_axes_btreemap(&[
            ("cpu_bound_throughput", 95.0),  // top
            ("memory_fragmentation", 5.0),   // bottom
            ("channel_throughput", 50.0),
            ("image_size_efficiency", 50.0),
            ("multithread_throughput", 50.0),
            ("resilience", 50.0),
            ("security_posture", 50.0),
            ("web_throughput", 50.0),
        ]);
        let composite = 50.0;
        let scores = vec![TestCellScore {
            alloc: "jemalloc".into(),
            env: "alpine".into(),
            composite,
            axes: axes.clone(),
        }];
        let runs = vec![synth_run_with_env(
            "jemalloc",
            "alpine",
            "cpu-bound",
            100.0,
            50_000,
            5.0,
        )];

        let recs = top_n_cells(scores, &runs);
        assert_eq!(recs.len(), 1);
        let r = &recs[0];

        assert_eq!(r.rank, 1, "rank is 1-indexed");
        assert_eq!(r.alloc, "jemalloc");
        assert_eq!(r.env, "alpine");
        assert!(
            (r.composite_score - composite).abs() < 1e-9,
            "composite_score should be copied: got {}",
            r.composite_score
        );
        assert_eq!(r.axes, axes, "axes should be copied entry-equal");
        assert!(!r.tldr.is_empty(), "tldr non-empty");
        assert_eq!(r.strengths.len(), 2, "strengths.len() == 2");
        assert_eq!(r.weaknesses.len(), 2, "weaknesses.len() == 2");
        // Strongest axis is cpu_bound_throughput → "CPU-bound throughput".
        assert_eq!(r.strengths[0], "CPU-bound throughput");
        // Weakest axis is memory_fragmentation → "Memory / fragmentation".
        assert_eq!(r.weaknesses[0], "Memory / fragmentation");
        // suspect_flag is a deterministic bool — healthy run → false.
        assert!(!r.suspect_flag);
    }

    /// REC-01: `recommended_for` reuses winner detection at `(alloc, env)`
    /// granularity. Synthesize runs where `(jemalloc, alpine)` dominates
    /// `cpu-bound` and `web`; assert recommended_for = ["cpu-bound",
    /// "web-ser-de"] (alphabetical). Other cells contribute at lower
    /// throughputs so the winner is unambiguous.
    #[test]
    fn cell_recommendation_recommended_for_uses_existing_winners() {
        // (jemalloc, alpine) dominates cpu-bound and web; (ptmalloc,
        // debian-slim) is also measured but lower.
        let runs = vec![
            // cpu-bound: jemalloc/alpine wins.
            synth_run_with_env("jemalloc", "alpine", "cpu-bound", 200.0, 50_000, 5.0),
            synth_run_with_env("ptmalloc", "debian-slim", "cpu-bound", 100.0, 50_000, 5.0),
            // web: jemalloc/alpine wins.
            synth_run_with_env("jemalloc", "alpine", "web", 1800.0, 50_000, 5.0),
            synth_run_with_env("ptmalloc", "debian-slim", "web", 1200.0, 50_000, 5.0),
        ];
        let scores = vec![
            // jemalloc/alpine ranked first.
            synth_cell_score_uniform("jemalloc", "alpine", 80.0, 80.0),
            // ptmalloc/debian-slim ranked second.
            synth_cell_score_uniform("ptmalloc", "debian-slim", 40.0, 40.0),
        ];

        let recs = top_n_cells(scores, &runs);
        let winner = recs
            .iter()
            .find(|r| r.alloc == "jemalloc" && r.env == "alpine")
            .expect("winner present");
        assert_eq!(
            winner.recommended_for,
            vec!["cpu-bound", "web-ser-de"],
            "should win cpu-bound + web-ser-de classes (alphabetical)"
        );
    }

    /// REC-01: `avoid_for` reports the per-class bottom-2 cells. 4-cell
    /// fixture where `(ptmalloc, debian-slim)` is the bottom of cpu-bound
    /// and web throughput. With 4 measured cells per class, bottom-2
    /// selects the two lowest. We pin `(ptmalloc, debian-slim)` AND
    /// `(jemalloc, debian-slim)` as the bottom-2 for both classes.
    #[test]
    fn cell_recommendation_avoid_for_is_bottom_2_class_rankings() {
        let runs = vec![
            // cpu-bound: (ptmalloc, debian-slim) lowest.
            synth_run_with_env("mimalloc", "wolfi", "cpu-bound", 200.0, 50_000, 5.0),
            synth_run_with_env("jemalloc", "alpine", "cpu-bound", 180.0, 50_000, 5.0),
            synth_run_with_env("jemalloc", "debian-slim", "cpu-bound", 120.0, 50_000, 5.0),
            synth_run_with_env("ptmalloc", "debian-slim", "cpu-bound", 100.0, 50_000, 5.0),
            // web: same bottom-2 ordering.
            synth_run_with_env("mimalloc", "wolfi", "web", 1800.0, 50_000, 5.0),
            synth_run_with_env("jemalloc", "alpine", "web", 1700.0, 50_000, 5.0),
            synth_run_with_env("jemalloc", "debian-slim", "web", 1300.0, 50_000, 5.0),
            synth_run_with_env("ptmalloc", "debian-slim", "web", 1200.0, 50_000, 5.0),
        ];
        let scores = vec![
            synth_cell_score_uniform("mimalloc", "wolfi", 80.0, 80.0),
            synth_cell_score_uniform("jemalloc", "alpine", 70.0, 70.0),
            synth_cell_score_uniform("jemalloc", "debian-slim", 40.0, 40.0),
            synth_cell_score_uniform("ptmalloc", "debian-slim", 20.0, 20.0),
        ];

        let recs = top_n_cells(scores, &runs);
        let loser = recs
            .iter()
            .find(|r| r.alloc == "ptmalloc" && r.env == "debian-slim")
            .expect("(ptmalloc, debian-slim) present in top-N");
        // (ptmalloc, debian-slim) is bottom of cpu-bound AND web → both
        // classes appear in avoid_for, alphabetical.
        assert!(
            loser.avoid_for.contains(&"cpu-bound"),
            "expected cpu-bound in avoid_for, got {:?}",
            loser.avoid_for
        );
        assert!(
            loser.avoid_for.contains(&"web-ser-de"),
            "expected web-ser-de in avoid_for, got {:?}",
            loser.avoid_for
        );
        // BTreeMap iteration → labels sorted alphabetically: "cpu-bound" <
        // "web-ser-de".
        let cpu_idx = loser.avoid_for.iter().position(|c| *c == "cpu-bound");
        let web_idx = loser.avoid_for.iter().position(|c| *c == "web-ser-de");
        assert!(
            cpu_idx < web_idx,
            "cpu-bound should sort before web-ser-de in avoid_for"
        );
    }

    /// REC-01 + REC-02: `top_n_cells` truncates 18 cells → 10 (TOP_N_TOTAL).
    #[test]
    fn top_n_cells_truncates_to_top_n_total_constant() {
        let scores: Vec<TestCellScore> = (1..=18)
            .map(|i| {
                let alloc = format!("alloc{:02}", i);
                let env = format!("env{:02}", i);
                synth_cell_score_uniform(&alloc, &env, i as f64, 50.0)
            })
            .collect();
        // Empty runs is acceptable here — recommended_for / avoid_for /
        // suspect_flag will all be empty/false. We test the truncation
        // boundary, not class detection.
        let runs: Vec<Run> = Vec::new();
        let recs = top_n_cells(scores, &runs);
        assert_eq!(recs.len(), TOP_N_TOTAL);
        assert_eq!(recs.len(), 10);
    }

    /// REC-01: `top_n_cells` handles fewer-than-TOP_N_TOTAL inputs (defensive
    /// `min(TOP_N_TOTAL, scores.len())`). 3 input cells → 3 output rows.
    #[test]
    fn top_n_cells_handles_fewer_than_top_n_total_input() {
        let scores: Vec<TestCellScore> = (1..=3)
            .map(|i| {
                let alloc = format!("alloc{:02}", i);
                let env = format!("env{:02}", i);
                synth_cell_score_uniform(&alloc, &env, i as f64, 50.0)
            })
            .collect();
        let runs: Vec<Run> = Vec::new();
        let recs = top_n_cells(scores, &runs);
        assert_eq!(recs.len(), 3);
    }
}

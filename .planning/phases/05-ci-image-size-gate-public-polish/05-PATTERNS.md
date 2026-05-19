# Phase 5: CI, Image-Size Gate & Public Polish - Pattern Map

**Mapped:** 2026-05-19
**Files analyzed:** 13 (8 new, 5 modified, 1 verify-only)
**Analogs found:** 9 / 13 (4 are GREENFIELD with no in-repo analog)

## Codebase Audit (gate facts that bind every plan)

Before any per-file pattern, three project-wide facts the planner must hold constant:

| Fact | Verified Location | Implication |
|------|-------------------|-------------|
| **`rust-toolchain.toml` already exists** with `channel = "1.91"` | `/Users/marc.carre/src/rust-benchmark-glibc-musl-mimalloc/rust-toolchain.toml:1-3` | Phase 5 does NOT create this file — task is "verify" not "add". CONTEXT.md `<specifics>` ¶7 saying "Phase 5 ADDS this file" is stale. |
| **All Dockerfiles pin `RUST_VERSION=1.91`** consistently | `docker/alpine.Dockerfile:5`, `docker/distroless-cc.Dockerfile:9`, `docker/distroless-static.Dockerfile:6`, `docker/debian-slim.Dockerfile:9`, `docker/scratch.Dockerfile:17`, `docker/wolfi.Dockerfile:16` (all six = `1.91`) | README "Reproducibility" + every action `dtolnay/rust-toolchain@1.91.0` reference MUST cite `1.91`. CONTEXT.md D-17 references `1.83` — that text is stale (Phase 3 design carry-over). |
| **`justfile` pins `RUST_VERSION=1.91`** in build args | `justfile:79` (`--build-arg RUST_VERSION=1.91`) | Same — `1.91` is the source of truth. |
| **`Cargo.toml` workspace declares `rust-version = "1.83"`** | `Cargo.toml:7` | This is the **MSRV** (minimum supported), not the build-time pin. Both fields can co-exist: MSRV = `1.83`, actual build = `1.91`. Plan-phase decides whether to bump MSRV to `1.91` (low-risk; nothing depends on `1.83`-only code). |
| **`Cargo.toml` already declares `repository` + `license`** | `Cargo.toml:8-9` (`license = "MIT OR Apache-2.0"`, `repository = "https://github.com/marccarre/rust-benchmark-glibc-musl-mimalloc"`) | D-15 / D-18 dual-license + badge URL: just commit the LICENSE files; the SPDX expression is already declared. Use the literal repo URL in the badge — NOT a `{owner}/{repo}` placeholder. |
| **No `.github/workflows/` directory exists** | `ls .github 2>&1` → exits non-zero | `.github/workflows/bench.yml` is greenfield — no in-repo workflow analog. |
| **Aggregator `Env` struct has NO `image_size_mb` field** | `crates/alloc-bench-core/src/output.rs` (Phase 1 schema; Pitfall 4 of RESEARCH.md verifies field absence) | Sidecar `meta.json` is the ONLY schema-preserving path. Do NOT modify `Env` (locked Phase 1 D-11 / D-12). |

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `.github/workflows/bench.yml` | ci-config | event-driven (push/PR/wd → matrix → artifact pipeline) | — | GREENFIELD |
| `.github/workflows/ci-validate.yml` (folded into bench.yml per RESEARCH §Project Structure recommendation) | ci-config | event-driven | — | GREENFIELD |
| `crates/alloc-bench-aggregator/src/multi_run.rs` | utility (statistics module) | transform (`&[f64]` → `MultiRunStats`) | `crates/alloc-bench-aggregator/src/recommend.rs` | role-match (computation module with serde-derived input + `validated()`-style invariant guards) |
| `crates/alloc-bench-aggregator/src/main.rs` (modify) | controller (CLI entry) | request-response (clap → load → write) | itself (extend) | exact (in-place extension; add `--meta` flag + `mod multi_run`) |
| `crates/alloc-bench-aggregator/src/markdown.rs` (modify) | service (REPORT.md emitter) | transform (`&[Run]` → markdown buffer) | itself (`emit_per_scenario_tables`, `emit_docker_runtimes_table`) | exact |
| `crates/alloc-bench-aggregator/src/recommend.rs` (modify) | service (workload-class picker) | transform (`&[Run]` → `Vec<Recommendation>`) | itself (`recommend_for_class`) | exact |
| `crates/alloc-bench-aggregator/templates/index.html.tmpl` (modify) | template (HTML/JS chart) | transform (filtered runs → Plotly traces) | itself (`makeThroughputTraces` lines 415-447) | exact |
| `crates/alloc-bench-aggregator/tests/smoke.rs` (modify) | test (integration) | request-response (cargo bin → assert outputs) | itself (`run_aggregator_against_fixtures`) | exact |
| `crates/alloc-bench-aggregator/tests/fixtures/multi_run/seed-{1,2,3}.json` | fixture (test data) | static input | `tests/fixtures/jemalloc-alpine.json` | exact (same JSON shape; vary only `ticks_per_s` per seed) |
| `justfile` (modify — add `ci-bench-cell`, `ci-aggregate`, `ci-validate`) | config (recipe runner) | request-response (CLI → shell) | `justfile:43-84` (`build`), `justfile:110-112` (`bench-cell`), `justfile:299-301` (`aggregate`) | exact |
| `LICENSE-MIT` | doc (license text) | static | — | GREENFIELD (canonical text from opensource.org/license/mit) |
| `LICENSE-APACHE` | doc (license text) | static | — | GREENFIELD (canonical text from apache.org/licenses/LICENSE-2.0.txt) |
| `README.md` (modify — prepend badge, append walkthrough/matrix/reproducibility/license) | doc (markdown) | static | `README.md:1-16` (Phase 4 system-diagram block — preserve verbatim) | partial (extend, do not rewrite) |
| `Cargo.toml` workspace metadata | config | static | itself (`Cargo.toml:5-10`) | already complete — verify only |

## Pattern Assignments

### `crates/alloc-bench-aggregator/src/multi_run.rs` (utility, transform) — NEW

**Analog:** `crates/alloc-bench-aggregator/src/recommend.rs` (same crate; same architectural tier — pure-stdlib computation over `&[Run]` / `&[f64]`; emits a `#[derive(Serialize)]` output struct).

**Module-doc preamble pattern** (lines 1-24 of `recommend.rs`):
```rust
//! Workload → allocator picker (D-12, AGG-07).
//!
//! Given the loaded `Run` set, return one `Recommendation` per workload
//! class. Each rationale string is data-derived from the measured runs:
//! [...]
//! Hard-coded prose is forbidden (RESEARCH §Pitfall 7) — every rationale
//! must be derivable from the input JSON. A unit-test suite gates the
//! contract [...]
```
Phase 5 mirrors this: cite **CONTEXT.md D-11/D-12** + **RESEARCH §Pattern 5 / §Pitfall 7** at the top, declare the contract (Bessel-corrected sample stddev, n ≥ 2, CV undefined when mean ≈ 0), and announce the unit-test gates.

**Imports pattern** (lines 24-28 of `recommend.rs`):
```rust
use std::collections::BTreeMap;

use alloc_bench_core::output::Run;

use crate::html::is_suspect;
```
Phase 5 `multi_run.rs` imports just `serde::Serialize` (output struct) + nothing from the workspace at module level. The function takes `&[f64]` so it does NOT depend on `alloc_bench_core::output::Run`. The Run-grouping helper that bridges `&[Run]` → `&[f64]` lives in `markdown.rs` (per RESEARCH §"Code Examples — Multi-run aggregator integration", function `group_runs_by_cell`).

**Public output-struct pattern** (`Recommendation`, lines 34-40 of `recommend.rs`):
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Recommendation {
    pub class: &'static str,
    pub allocator: String,
    pub rationale: String,
}
```
Phase 5 `MultiRunStats` follows the same shape — `#[derive(Debug, Clone, Serialize)]` (the `Serialize` is the only addition; the aggregator-derived `multi_run_stats` block emitted into HTML/JSON-for-template needs it):
```rust
#[derive(Debug, Clone, Serialize)]
pub struct MultiRunStats {
    pub n: usize,
    pub mean: f64,
    pub median: f64,
    pub min: f64,
    pub max: f64,
    pub stddev: f64,         // Bessel-corrected (n-1)
    pub cv_pct: Option<f64>, // None when mean ≈ 0 or non-finite
}
```

**Validated()-style invariant pattern** (already established in `recommend.rs`'s zero-throughput guard at lines 233-238, and divide-by-zero `if runner_up.score > 0.0`). Phase 5 reproduces the same defensive shape via two early returns:

1. Sample-count floor: `if samples.len() < 2 { return None; }` — sample stddev is undefined for n < 2.
2. NaN/inf rejection: `if samples.iter().any(|x| !x.is_finite()) { return None; }` — matches CONTEXT.md `<code_context>` "reject NaN/inf throughput, reject negative cv_pct".
3. CV-undefined branch: `if mean.abs() > 1e-9 && mean.is_finite() { Some(...) } else { None }` — Wikipedia near-zero edge case.

**Unit-test pattern** (`recommend.rs` lines 278-519, `synth_run` builder + 9 named test cases):

`recommend.rs` builds a `synth_run` helper (lines 289-354) returning a fully-populated `Run` with sentinel values, then runs 9 `#[test] fn ...` cases each named after the behavior under test. Phase 5 `multi_run.rs` follows the same structure with simpler input (`&[f64]` not `Run`), exactly as RESEARCH §Pattern 5 sketches: 6 tests (`three_identical_samples_have_zero_variance`, `three_seeds_with_known_cv`, `high_variance_flagged_when_cv_above_10pct`, `cv_undefined_when_mean_is_zero`, `rejects_nan_input`, `requires_at_least_two_samples`).

**Threshold helper pattern** (`recommend.rs` does not have an exact analog; closest is `html::is_suspect`, lines 50-52):
```rust
pub(crate) fn is_suspect(h: &HarnessInfo) -> bool {
    h.samples_count < 10_000 || h.warmup_duration_s < 5.0
}
```
Phase 5 mirrors with `pub fn is_high_variance(stats: &MultiRunStats) -> bool { matches!(stats.cv_pct, Some(cv) if cv > 10.0) }` — same single-purpose-predicate pattern + same constant-locking convention (the threshold lives next to the predicate, not as a magic-number constant).

---

### `crates/alloc-bench-aggregator/src/main.rs` (controller, modify)

**Analog:** itself (lines 1-59 — the entire file extends in place).

**Imports + clap struct extension pattern** (lines 21-43):
```rust
mod diagrams;
mod html;
mod loader;
mod markdown;
mod recommend;

use anyhow::{Context, Result};
use clap::Parser;

#[derive(Parser)]
#[command(
    name = "alloc-bench-aggregator",
    version,
    about = "Aggregate alloc-bench results into report/index.html + REPORT.md"
)]
struct Cli {
    /// Glob pattern for input JSON files (D-05).
    #[arg(long, default_value = "results/*.json")]
    input: String,
    /// Output directory for index.html + REPORT.md (D-05).
    #[arg(long, default_value = "report/")]
    output: String,
}
```

**Phase 5 additions:**
1. Add `mod multi_run;` to the top module block (alphabetical with existing modules).
2. Add a third clap field — RESEARCH §Open Questions ¶3 specifies a `--meta` flag (default `meta/*.json`) so the aggregator can locate sidecar `meta.json` files separately from per-run results:
   ```rust
   /// Glob pattern for per-cell meta sidecars (image_size_mb / build_time_s).
   /// Empty = skip meta merge. CI populates via `docker inspect` (D-13).
   #[arg(long, default_value = "")]
   meta: String,
   ```
   Default empty (not `meta/*.json`) so existing local `just aggregate` invocations continue to work unchanged — meta-merge is an opt-in CI feature.

**main() body extension pattern** (lines 45-59):
```rust
fn main() -> Result<()> {
    let cli = Cli::parse();
    let outcome = loader::discover(&cli.input)?;
    let out_dir = std::path::Path::new(&cli.output);
    std::fs::create_dir_all(out_dir)
        .with_context(|| format!("creating output dir {}", cli.output))?;
    markdown::write(&outcome, out_dir)?;
    html::write(&outcome, out_dir)?;
    eprintln!(
        "aggregated {} runs, skipped {}",
        outcome.runs.len(),
        outcome.skipped.len()
    );
    Ok(())
}
```

Phase 5 inserts a meta-load step between `discover` and `markdown::write`, gated on `!cli.meta.is_empty()`:
```rust
let metas = if cli.meta.is_empty() {
    Default::default()  // empty HashMap
} else {
    loader::load_cell_metas(&cli.meta)?
};
markdown::write(&outcome, &metas, out_dir)?;
html::write(&outcome, &metas, out_dir)?;
```

This adds a *single* parameter to two existing function signatures — preserve the rest of the call shape.

---

### `crates/alloc-bench-aggregator/src/markdown.rs` (service, modify)

**Analog:** itself (lines 86-164 — the per-scenario table emitter).

**Per-scenario row-emit pattern** (lines 138-161):
```rust
for (idx, r) in sorted.iter().enumerate() {
    let unit = r.scenario.unit.as_deref().unwrap_or("ticks/s");
    let alloc_cell = if Some(idx) == winner_idx {
        format!("**\u{2713} {}**", r.build.allocator)
    } else {
        r.build.allocator.clone()
    };
    let mut throughput_cell = format!("{:.1} {}", r.metrics.ticks_per_s, unit);
    if let Some(reason) = suspect_reason(&r.harness) {
        throughput_cell.push(' ');
        throughput_cell.push_str(suspect_note(&reason));
    }
    let _ = writeln!(
        buf,
        "| {alloc} | {tps} | {p50} ns | {p95} ns | {p99} ns | {p999} ns | {rss} kB |",
        alloc = alloc_cell,
        tps = throughput_cell,
        p50 = r.metrics.tick_latency_ns.p50,
        // ...
    );
}
```

**Phase 5 extension — multi-run cell decoration:** Build a `BTreeMap<(alloc, env, scenario), MultiRunStats>` once per scenario (group runs by 3-tuple, call `multi_run::aggregate`), then change the throughput cell from raw `{:.1} {}` to RESEARCH §"Code Examples — Multi-run aggregator integration" `format_throughput_cell`:
```rust
// pattern from RESEARCH §Code Examples
pub fn format_throughput_cell(s: &MultiRunStats, suspect: bool) -> String {
    let cv_str = match s.cv_pct {
        Some(cv) => format!("CV {:.0}%", cv),
        None     => "CV \u{2014}".to_string(),  // em-dash
    };
    let variance_flag = if is_high_variance(s) { " \u{26A0} high variance" } else { "" };
    let suspect_flag  = if suspect              { " \u{26A0} suspect"        } else { "" };
    format!(
        "{:.0} ({:.0}..{:.0}, {}{}{})",
        s.median, s.min, s.max, cv_str, variance_flag, suspect_flag
    )
}
```

**Suspect+variance concatenation rule** (CONTEXT.md `<specifics>` ¶5): when both flags fire, BOTH italic notes appear concatenated: `*(⚠ suspect: low samples; ⚠ high variance: CV 14%)*`. The existing `suspect_note` function (lines 284-290) already has the `*(⚠ suspect: ...)*` italic-wrap pattern; Phase 5 extends `suspect_note` with a `Both` variant or appends a second italic note in `format_throughput_cell`. **Recommendation:** keep `suspect_note` unchanged and append the variance flag *outside* the italic, so the existing byte-identical-output test (`per_scenario_table_marks_winner_with_check_prefix` lines 369-389, `report_md_two_runs_byte_identical_after_timestamp_strip` lines 487-511) continues to pass for runs with no multi-run data.

**Docker runtimes table backfill pattern** (lines 170-189):
```rust
fn emit_docker_runtimes_table(buf: &mut String, runs: &[Run]) {
    let envs: BTreeSet<String> = runs.iter().map(|r| env_label(&r.env).to_string()).collect();
    // [...]
    for env in envs.iter() {
        let _ = writeln!(buf, "| {env} | \u{2014} | \u{2014} | \u{2014} |");
    }
    // [...]
    "*image_size_mb / build_time_s / run_overhead_pct populated by Phase 5 CI via docker inspect (REPR-03).*"
}
```

**Phase 5 extension:** signature changes to `fn emit_docker_runtimes_table(buf: &mut String, runs: &[Run], metas: &HashMap<(String, String), CellMeta>)`. For each `env` in the BTreeSet, look up `metas.get(&(alloc, env))` for one allocator (or fold across allocs and pick the median value) and replace `\u{2014}` with `{:.1}` of the `image_size_mb` field. When the metas map is empty (local `just aggregate` invocation), emit em-dashes — this preserves backward compat with all existing tests.

**Footnote rewrite:** Once Phase 5 lands, the footnote becomes `*image_size_mb / build_time_s populated when CI is the source (REPR-03); em-dash for local runs.*` — Phase 4 D-10's "now-em-dash, future-CI-populated" semantic is now resolved.

---

### `crates/alloc-bench-aggregator/src/recommend.rs` (service, modify)

**Analog:** itself.

**Per-scenario mean computation pattern** (lines 156-180):
```rust
for &scen in scenarios.iter() {
    let matching: Vec<&&Run> = alloc_runs
        .iter()
        .filter(|r| r.scenario.name == scen)
        .collect();
    if matching.is_empty() {
        continue;
    }
    let mean: f64 = matching.iter().map(|r| r.metrics.ticks_per_s).sum::<f64>()
        / matching.len() as f64;
    per_scenario.insert(scen, mean);
    // [...]
}
```

**Phase 5 extension** (CONTEXT.md D-11): swap mean → median. When ≥3 runs share `(alloc, env, scenario)`, use `multi_run::aggregate(&samples).map(|s| s.median).unwrap_or_else(|| /* fallback to mean */)`. The fallback path matters because Phase 5 still needs to emit recommendations even when local `just aggregate` runs feed only 1 run per cell. Pattern:
```rust
let throughputs: Vec<f64> = matching.iter().map(|r| r.metrics.ticks_per_s).collect();
let central_tendency = match crate::multi_run::aggregate(&throughputs) {
    Some(stats) => stats.median,
    None        => throughputs.iter().sum::<f64>() / (throughputs.len() as f64).max(1.0),
};
per_scenario.insert(scen, central_tendency);
```

**Test-impact path:** Lines 478-501 (`winner_picker_channel_heavy_means_three_scenarios`) will continue to pass because each fixture has exactly 1 run per (alloc, scenario) → `multi_run::aggregate` returns `None` (n=1 < 2) → falls back to mean. Existing tests are byte-stable with the new code.

**New test to add** (gates the multi-run substitution): synthesize 3 `Run`s per (alloc, scenario) with varying `ticks_per_s`, assert the median (not mean) is what feeds the recommendation comparison.

---

### `crates/alloc-bench-aggregator/templates/index.html.tmpl` (template, modify)

**Analog:** itself, lines 415-447 (the `makeThroughputTraces` function).

**Existing throughput-bar trace pattern** (lines 419-436):
```javascript
for (const alloc of allocs) {
  const allocRuns = filtered.filter(r => r.build.allocator === alloc);
  let anySuspect = false;
  for (const r of allocRuns) {
    if (isSuspect(r)) { anySuspect = true; break; }
  }
  const y = scenarios.map(s => {
    const hit = allocRuns.find(r => r.scenario.name === s);
    return hit ? hit.metrics.ticks_per_s : null;
  });
  traces.push({
    type: 'bar',
    name: alloc + (anySuspect ? ' ⚠' : ''),  // ⚠ when any-suspect (escaped)
    x: scenarios,
    y: y,
    marker: { color: ALLOC_COLORS[alloc] || '#888' },
  });
}
```

**Phase 5 extension — Plotly `error_y` pattern.** Plotly's bar-chart errorbar field is documented at https://plotly.com/javascript/error-bars/ — the canonical shape for asymmetric error bars (min..max as `arrayminus` / `array`):

```javascript
// Phase 5 extension: add an `error_y` field per trace when multi-run data
// is present in `r._multiRun = { median, min, max, cv_pct, suspect }` (the
// derived `RESULTS_GROUPED` view per CONTEXT.md D-11).
const yMedians = scenarios.map(s => {
  const hit = allocRuns.find(r => r.scenario.name === s);
  return hit && hit._multiRun ? hit._multiRun.median : (hit ? hit.metrics.ticks_per_s : null);
});
const yMinusArr = scenarios.map(s => {
  const hit = allocRuns.find(r => r.scenario.name === s);
  return (hit && hit._multiRun) ? (hit._multiRun.median - hit._multiRun.min) : 0;
});
const yPlusArr = scenarios.map(s => {
  const hit = allocRuns.find(r => r.scenario.name === s);
  return (hit && hit._multiRun) ? (hit._multiRun.max - hit._multiRun.median) : 0;
});
const anyHighVariance = allocRuns.some(r => r._multiRun && r._multiRun.cv_pct > 10);
traces.push({
  type: 'bar',
  name: alloc
    + (anySuspect      ? ' ⚠ suspect'        : '')
    + (anyHighVariance ? ' ⚠ high variance'  : ''),
  x: scenarios,
  y: yMedians,
  error_y: {
    type: 'data',
    symmetric: false,
    array:      yPlusArr,    // upper bound (max - median)
    arrayminus: yMinusArr,   // lower bound (median - min)
    visible: true,
    color: '#1F2328',        // matches --color-text from line 27
    thickness: 1.0,
    width: 4,
  },
  marker: { color: ALLOC_COLORS[alloc] || '#888' },
});
```

**Backward compat:** when `r._multiRun` is `undefined` (local `just aggregate` with 1 run per cell), the existing single-value path runs unchanged — `yMinusArr` and `yPlusArr` arrays are full of zeros, which Plotly renders as no-error-bars (visible:true with all-zero arrays renders as zero-width whiskers; equivalent to `visible: false`). The existing 6 visual-contract smoke tests (`tests/smoke.rs:278-405`) remain green.

**Tinytemplate-escape gotcha** (lines 22, 60, etc. — every literal `{` in CSS/JS body is `\{`): the new error_y block follows the same convention. Ref: `html.rs:15-21` doc-comment on tinytemplate `{` parsing.

**Legend ⚠ glyph contract** (locked Phase 4): `name: alloc + ' ⚠'` already established at line 431. Phase 5 extends with two distinct `⚠` reasons (`suspect`, `high variance`) — both Unicode `⚠`. The label text differentiates them (matches REPORT.md italic-note vocabulary: `⚠ suspect`, `⚠ high variance`).

---

### `crates/alloc-bench-aggregator/tests/smoke.rs` (test, modify)

**Analog:** itself, lines 259-271 (the `run_aggregator_against_fixtures` helper) + lines 414-426 (`run_aggregator_and_read_markdown`).

**Helper-driven smoke pattern** (lines 259-271):
```rust
fn run_aggregator_against_fixtures() -> (tempfile::TempDir, String) {
    let out_dir = tempdir().expect("tempdir");
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let pattern = format!("{}/*.json", fixtures.display());
    let mut cmd = Command::cargo_bin("alloc-bench-aggregator").expect("cargo bin");
    cmd.args(["--input"])
        .arg(&pattern)
        .args(["--output"])
        .arg(out_dir.path());
    cmd.assert().success();
    let html = std::fs::read_to_string(out_dir.path().join("index.html")).expect("read index.html");
    (out_dir, html)
}
```

**Phase 5 extension pattern**: add a sister helper `run_aggregator_with_multi_run_fixtures()` that points `--input` at `tests/fixtures/multi_run/*.json` and additionally passes `--meta tests/fixtures/multi_run/meta/*.json` for the sidecar test. Mirror the `(tempdir, html)` return shape so tests can `assert!(html.contains("error_y"))`.

**New tests to add** (mirrors existing 6 visual-contract tests at lines 278-405):

| New test | Asserts |
|----------|---------|
| `aggregator_multi_run_emits_cv_in_throughput_cell` | REPORT.md contains `CV ` (with trailing space, e.g. `CV 4%`) on the per-scenario throughput cells |
| `aggregator_multi_run_emits_min_max_range_in_cell` | REPORT.md contains `(...0..` literal — the `({min}..{max})` range syntax |
| `aggregator_high_variance_cell_marked_with_warning_glyph` | REPORT.md contains `⚠ high variance` for a fixture with CV > 10% |
| `aggregator_html_contains_error_y_field` | `index.html` contains `error_y` (the Plotly errorbar field name) |
| `aggregator_html_high_variance_appears_in_legend` | `index.html` contains `⚠ high variance` (legend text) |
| `aggregator_meta_sidecar_populates_image_size_mb` | REPORT.md `## Docker runtimes` row for `alloc-bench:jemalloc-alpine` shows a numeric `image_size_mb` (not em-dash) when sidecar is present |

**Test fixture pattern (NEW files):** `tests/fixtures/multi_run/seed-1.json`, `seed-2.json`, `seed-3.json` — each is a `Vec<Run>` matching the existing `jemalloc-alpine.json` shape (lines 1-90 of that file is the canonical reference). Vary only `metrics.ticks_per_s` per seed:

| File | scenario | ticks_per_s | Purpose |
|------|----------|-------------|---------|
| seed-1.json | cpu-bound | 100.0 | seed 1 — canonical |
| seed-2.json | cpu-bound | 110.0 | seed 2 — perturbation; mean=105, stddev=5.0 (Bessel), CV ≈ 4.76% |
| seed-3.json | cpu-bound | 105.0 | seed 3 — perturbation; closes the 3-sample set |

Both contain at least one `(alloc, env, scenario)` tuple where CV > 10% to gate the high-variance test (e.g., seed-1 throughput=100, seed-2=130, seed-3=90 → mean ≈ 106.67, stddev ≈ 20.82, CV ≈ 19.5%).

**Sidecar fixture (NEW):** `tests/fixtures/multi_run/meta/jemalloc-alpine.json`:
```json
{
  "alloc": "jemalloc",
  "env": "alpine",
  "image_size_bytes": 27845632,
  "image_size_mb": 26.55,
  "build_time_s": 142.3,
  "captured_at": "2026-05-19T15:30:42Z"
}
```
Shape per RESEARCH §"Sidecar `meta.json` shape".

---

### `justfile` (config, modify — add `ci-bench-cell`, `ci-aggregate`, `ci-validate`)

**Analog:** the existing recipes are the strongest in-repo precedent for justfile style.

**`ci-bench-cell` analog** — `bench-cell` (lines 110-112) + `build` body (lines 43-84):
```just
# Build + run one cell sequentially.
bench-cell env alloc:
    just build {{env}} {{alloc}}
    just run {{env}} {{alloc}}
```

`ci-bench-cell` extends this with: (a) post-build `dive-check` invocation (already exists at line 232), (b) a 3-iteration loop with seeds 1/2/3 — re-using the `--seed` flag pattern from `run-all-smoke` (line 24, `--seed 7`) and the docker `run` body (lines 102-107), (c) `meta.json` capture via `docker inspect` (per CONTEXT.md `<specifics>` ¶4 and RESEARCH §Pattern 4).

**Recommended `ci-bench-cell` body:**
```just
# CI variant of bench-cell: build + dive-check + 3 seeded runs + meta.json sidecar.
# Used by the GHA matrix workflow (Phase 5).
ci-bench-cell env alloc:
    #!/usr/bin/env bash
    set -euo pipefail
    just build {{env}} {{alloc}}
    just dive-check {{env}} {{alloc}}
    mkdir -p results meta
    chmod 0777 results
    SIZE_BYTES=$(docker image inspect alloc-bench:{{alloc}}-{{env}} --format '{{ "{{" }}.Size{{ "}}" }}')
    SIZE_MB=$(awk "BEGIN { printf \"%.2f\", $SIZE_BYTES / 1024 / 1024 }")
    jq -n --argjson b "$SIZE_BYTES" --argjson m "$SIZE_MB" \
      '{
         alloc:            "{{alloc}}",
         env:              "{{env}}",
         image_size_bytes: $b,
         image_size_mb:    $m,
         captured_at:      now | todate
       }' > meta/{{alloc}}-{{env}}.json
    for seed in 1 2 3; do
      docker run --rm \
        --platform linux/amd64 \
        --cpus=4 --memory=4g --cpuset-cpus=0-3 \
        -v "$(pwd)/results:/out" \
        alloc-bench:{{alloc}}-{{env}} \
        run-all --output /out/{{alloc}}-{{env}}-seed${seed}.json --seed ${seed}
    done
```

Notes:
- Re-use the **exact same `--platform linux/amd64 --cpus=4 --memory=4g --cpuset-cpus=0-3` invariants** from `run` (lines 102-107). These flags are Phase 3 D-15 / D-16 locked.
- The `{{ "{{" }}.Size{{ "}}" }}` escape is a just-and-template double-escape: `{{ }}` is just-interpolation, the inner `"{{"` / `"}}"` literal strings emit `{{` / `}}` to the shell, and that `{{.Size}}` is the docker template syntax. Identical pattern is used in `clean-images` (line 121).
- The 3-seed loop intentionally matches CONTEXT.md `<specifics>` ¶2 (`--seed 1`, `--seed 2`, `--seed 3`).

**`ci-aggregate` analog** — `aggregate` (lines 299-301):
```just
aggregate:
    cargo run --release -p alloc-bench-aggregator -- \
        --input "results/*.json" --output report/
```

`ci-aggregate` extends with the new `--meta` flag (matching the main.rs CLI extension above):
```just
# CI variant of aggregate — also picks up sidecar meta.json files written
# by ci-bench-cell (Phase 5 D-13). Used by the GHA aggregate job.
ci-aggregate:
    cargo run --release -p alloc-bench-aggregator -- \
        --input "results/*.json" --meta "meta/*.json" --output report/
```

**`ci-validate` analog** — there is no existing recipe for "fmt + clippy + dce-check"; the closest precedents are:
- `dce-check` (lines 18-19): `@bash scripts/dce_check.sh {{ALLOCATOR}}`
- prek hooks (`prek.toml`): `cargo fmt` + `cargo clippy --all-targets`

`ci-validate` is a simple aggregate wrapper:
```just
# CI sanity check — fmt + clippy + dce-check. Mirrors the prek hook so CI
# catches the same regressions before invoking the matrix.
ci-validate:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    just dce-check system
```

**Recipe-doc-comment pattern** (lines 9-17 — the `dce-check` doc block):
```just
# Verify allocation calls survive --release --emit=llvm-ir (DCE gate).
# Phase-2 ROADMAP success criterion 4. Wraps scripts/dce_check.sh which
# greps the produced .ll files for `__rust_alloc` call sites.
#
# Usage:
#   just dce-check           # default: system allocator (libmalloc/ptmalloc/mallocng)
#   just dce-check system    # explicit
```

Phase 5 recipes follow the same convention: 3-line description + `# Usage:` block + 3-5 example invocations.

---

### `README.md` (doc, modify — additive sections only)

**Analog:** `README.md:1-16` (Phase 4 system-diagram block).

**Preserve-verbatim block** (lines 1-16):
```markdown
# rust-benchmark-glibc-musl-mimalloc

## How memory allocation works on Linux

```mermaid
flowchart TD
  app[Application code]
  std[Rust std::alloc]
  ga["#[global_allocator]<br/>jemalloc / mimalloc / system"]
  libc["libc malloc<br/>(ptmalloc on glibc, mallocng on musl)"]
  kernel["Kernel mmap / brk / sbrk"]
  phys[Physical memory]
  app --> std --> ga --> libc --> kernel --> phys
\```

When a Rust program calls `Vec::new()` [...]
```

This block has a **gating test** at `tests/smoke.rs:519-531` (`readme_md_contains_system_diagram`) that asserts the heading + `flowchart TD` are present. Phase 5 MUST NOT remove or alter the heading/diagram or the test breaks.

**Phase 5 README final structure** (CONTEXT.md D-15):
```markdown
[![CI](https://github.com/marccarre/rust-benchmark-glibc-musl-mimalloc/actions/workflows/bench.yml/badge.svg)](https://github.com/marccarre/rust-benchmark-glibc-musl-mimalloc/actions/workflows/bench.yml)

# rust-benchmark-glibc-musl-mimalloc

[1-line tagline + 3-4 sentence hero]

## How memory allocation works on Linux        <-- VERBATIM, preserve
[the existing 16 lines]

## Run it yourself                              <-- NEW (D-16)
[5-step recipe + Troubleshooting block]

## Allocator matrix overview                    <-- NEW (D-15 ¶5)
[Mermaid table or static list of 6 alloc combos × 6 envs]

## Reproducibility                              <-- NEW (D-17)
[rustc 1.91 pin + Docker tags + target-cpu=x86-64-v3 + GHA hardware notes + PITFALLS link]

## License                                      <-- NEW (D-15 ¶7)
Dual-licensed under Apache-2.0 OR MIT. See `LICENSE-APACHE` / `LICENSE-MIT`.
```

**Badge URL pattern** (RESEARCH §Pattern 7, sourced from `Cargo.toml:9`): use the LITERAL repo URL `https://github.com/marccarre/rust-benchmark-glibc-musl-mimalloc`, NOT the `{owner}/{repo}` placeholder text from CONTEXT.md D-18.

**Mermaid table convention** for `## Allocator matrix overview`: the existing `flowchart TD` style at lines 5-14 is the only Mermaid precedent. Phase 5 can use either a markdown `| alloc | env | libc |` table (more accessible) or a Mermaid `flowchart LR` (consistent with the existing diagram). Recommendation: markdown table for screen readers + 18 rows; the matrix is enumerable structure, not a flow.

**Reproducibility-section content sources** (D-17):
- rustc pin: cite `rust-toolchain.toml:2` → `1.91`. Do NOT cite CONTEXT.md's `1.83` (stale).
- Docker base images: cite each `docker/*.Dockerfile`'s `FROM` line for the pinned tag.
- Build flag: cite `.cargo/config.toml` (Phase 3 D-09 — `target-cpu=x86-64-v3`).
- GHA hardware: 4 vCPU / 16 GB RAM; cite RESEARCH §Architecture Patterns or PITFALLS.
- Link target: `.planning/research/PITFALLS.md`.

---

### `LICENSE-MIT` (doc, NEW) — GREENFIELD

**No analog.** Use canonical SPDX text from https://opensource.org/license/mit (cited in RESEARCH §"Code Examples — LICENSE-MIT canonical text").

Substitute:
- Year: `2026`
- Holder: `Marc Carré` (matches `Cargo.toml:10` authors field).

The text is verbatim — DO NOT paraphrase. SPDX-detection tooling (GitHub Linguist, cargo metadata) requires byte-exact match.

---

### `LICENSE-APACHE` (doc, NEW) — GREENFIELD

**No analog.** Plan-phase commits the canonical Apache 2.0 text from https://www.apache.org/licenses/LICENSE-2.0.txt verbatim (~11 KB). After the canonical text, append the Apache "How to apply the Apache License to your work" boilerplate appendix with substituted year/holder (RESEARCH §"Code Examples — LICENSE-APACHE canonical text").

The 11 KB body is too large to paste here; the planner reads from the cited URL or from a local `.cache/` of the canonical text.

---

### `.github/workflows/bench.yml` (ci-config, NEW) — GREENFIELD

**No analog in repo.** Source the entire structure from RESEARCH §"Code Examples — Full bench.yml skeleton" (lines 766-925 of `05-RESEARCH.md`). Verified against:
- `actions/checkout@v4` (RESEARCH §Standard Stack)
- `Swatinem/rust-cache@v2` (RESEARCH §Standard Stack)
- `docker/setup-buildx-action@v3` + `docker/build-push-action@v6` (RESEARCH §Standard Stack)
- `actions/upload-artifact@v4` + `actions/download-artifact@v4` (RESEARCH §Pattern 2)
- `dtolnay/rust-toolchain@1.91.0` (NOTE: pin is `1.91.0`, not `1.83.0` — RESEARCH §Pitfall 5)
- `extractions/setup-just@v2` (RESEARCH §Standard Stack)

**Three jobs** (RESEARCH §Architecture):
1. `pre-bench` — fmt + clippy + dce-check (single job, ~5 min)
2. `bench-matrix` — 18 cells parallel, each runs `just ci-bench-cell {env} {alloc}` then uploads `results-{alloc}-{env}` artifact
3. `aggregate` — `needs: bench-matrix`, `if: always()` (RESEARCH §Open Questions ¶1), downloads pattern `results-*` with `merge-multiple: true`, runs `just ci-aggregate`, uploads `bench-report-${{ github.run_id }}`

**18-cell matrix include block** — copy the ordered tuple list from `justfile:131-150` (the `_matrix_cells` block) so the GHA workflow and the local justfile stay in lockstep. RESEARCH §Pattern 1 has the canonical 18-line `include:` block.

**Concurrency block** (RESEARCH §Pattern 6):
```yaml
concurrency:
  group: bench-${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}
```

**Per-cell `meta.json` capture** — RESEARCH §Pattern 4 + the `ci-bench-cell` justfile recipe above. Plan-phase decides between in-recipe (cleaner; the recipe owns the docker invocation) or in-workflow (more visible in CI logs).

**Recommendation:** put `meta.json` capture inside `ci-bench-cell` so the local-machine reproduction path also produces the sidecar (a developer running `just ci-bench-cell debian-slim ptmalloc` locally gets the same outputs as CI).

**Anti-patterns to refuse** (RESEARCH §Anti-Patterns to Avoid):
- ❌ `actions/upload-artifact@v3` (deprecated; v4 required for unique-name-per-job)
- ❌ `target-cpu=native` in CI (locked Phase 3 D-09; runners share CPU types)
- ❌ Modifying `crates/alloc-bench-core/src/output.rs` (locked Phase 1 D-11/D-12)
- ❌ Re-running the matrix on tag pushes (use explicit `branches: ['**']`)

---

### `.github/workflows/ci-validate.yml` — DECISION POINT

CONTEXT.md says "or fold into bench.yml — plan-phase decides" and RESEARCH §Recommended Project Structure recommends folding (single-file workflow for readability of the artifact-flow contract). **Recommendation:** fold into `bench.yml` as the `pre-bench` job. No second YAML file.

If plan-phase chooses to split: the file is a 2-job mini-workflow (fmt-clippy-dce). Same standard stack as `bench.yml`. No new analog needed.

---

### `Cargo.toml` workspace metadata — VERIFY ONLY

**Already complete** at `Cargo.toml:5-10`:
```toml
[workspace.package]
edition = "2021"
rust-version = "1.83"
license = "MIT OR Apache-2.0"
repository = "https://github.com/marccarre/rust-benchmark-glibc-musl-mimalloc"
authors = ["Marc Carré"]
```

Phase 5 has nothing to add. Plan-phase decides whether to bump `rust-version = "1.83"` → `"1.91"` to match the actual build pin (low-risk). Recommended: keep `1.83` as the **MSRV** (minimum supported Rust version — what consumers need); the **build-time pin** (`rust-toolchain.toml`, Dockerfiles, justfile) is `1.91`. The two fields have distinct semantics.

---

## Shared Patterns

### Code excerpt: dive-check fallback to dockerized invocation

**Source:** `justfile:232-244`
**Apply to:** `bench.yml` matrix-cell job (the `Dive image-size gate` step)

```just
dive-check env alloc:
    #!/usr/bin/env bash
    set -euo pipefail
    if command -v dive >/dev/null 2>&1; then
        dive --ci alloc-bench:{{alloc}}-{{env}} --ci-config .dive-ci
    else
        docker run --rm \
            --platform linux/amd64 \
            -v /var/run/docker.sock:/var/run/docker.sock \
            -v "$(pwd)/.dive-ci:/.dive-ci:ro" \
            wagoodman/dive:latest \
            --ci alloc-bench:{{alloc}}-{{env}} --ci-config /.dive-ci
    fi
```

The `bench.yml` `Dive image-size gate` step just calls `just dive-check {{ matrix.env }} {{ matrix.alloc }}` — no separate dive install action. The fallback is already wired.

### Code excerpt: just escape pattern for double-brace docker templates

**Source:** `justfile:121` (`clean-images`):
```just
docker images --filter "reference=alloc-bench:*" --format '{{ "{{" }}.Repository{{ "}}" }}:{{ "{{" }}.Tag{{ "}}" }}'
```

**Apply to:** any GHA / justfile step that uses `docker inspect --format`. The `{{ "{{" }}` is just-interpolation emitting a literal `{{` to the shell.

### Code excerpt: BTreeMap-driven byte-identical-output discipline

**Source:** `markdown.rs:18-25` (module-doc preamble):
```rust
//! Byte-identical-output contract (D-09 / RESEARCH §Pitfall 5):
//!   - Sort everything alphabetically (BTreeMap / BTreeSet, never
//!     HashMap / HashSet).
//!   - Format throughput as `{:.1}` with the unit appended.
//!   - Format integers as `{}`, latency cells as `{} ns`, RSS as `{} kB`.
//!   - The single timestamp comment at the top is the only non-stable
//!     line — strippable in tests via first-line removal.
```

**Apply to:** `multi_run.rs` (output ordering for the derived `RESULTS_GROUPED` view) + the modified `markdown.rs` per-scenario tables (when grouping by `(alloc, env, scenario)` for multi-run lookup, use `BTreeMap`, not `HashMap`).

### Code excerpt: `validated()`-style invariant test pattern

**Source:** `recommend.rs:504-518` (`winner_picker_handles_zero_throughput_runner_up_without_div_by_zero`):
```rust
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
        r.rationale.starts_with("+0.0% throughput vs ptmalloc on cpu-bound"),
        "rationale was {:?}", r.rationale
    );
}
```

**Apply to:** `multi_run.rs` tests — write the matching guard tests for: `cv_undefined_when_mean_is_zero`, `requires_at_least_two_samples`, `rejects_nan_input`. RESEARCH §Pattern 5 has the exact code.

### Code excerpt: Conventional-commit prefixes

**Source:** CLAUDE.md / Phase 5 CONTEXT.md `<code_context>`:
> Phase 5 uses `feat(05)`, `chore(05)`, `docs(05)`, `test(05)`, `ci(05)`.

**Apply to:** every commit produced by every plan in Phase 5. Multi_run module → `feat(05)`. CI workflow → `ci(05)`. README → `docs(05)`. LICENSE files → `chore(05)`. Tests → `test(05)`.

---

## No Analog Found (GREENFIELD files)

Files with no close match in the codebase. The planner should consult the cited reference instead.

| File | Role | Data Flow | Reason | Reference |
|------|------|-----------|--------|-----------|
| `.github/workflows/bench.yml` | ci-config | event-driven | No `.github/workflows/` exists yet | RESEARCH §"Code Examples — Full bench.yml skeleton" (verbatim 160-line skeleton); RESEARCH §Pattern 1 (18-cell include); RESEARCH §Pattern 2 (artifact pipeline); RESEARCH §Pattern 3 (BuildKit cache scope); RESEARCH §Pattern 6 (concurrency); RESEARCH §Pattern 7 (badge) |
| `.github/workflows/ci-validate.yml` (if not folded) | ci-config | event-driven | Same | RESEARCH §"Code Examples — Full bench.yml skeleton" (the `pre-bench` job is a self-contained 2-step mini-workflow). |
| `LICENSE-MIT` | doc | static | No license text in repo today | https://opensource.org/license/mit (canonical SPDX); RESEARCH §"Code Examples — LICENSE-MIT canonical text" |
| `LICENSE-APACHE` | doc | static | No license text in repo today | https://www.apache.org/licenses/LICENSE-2.0.txt (canonical Apache 2.0); RESEARCH §"Code Examples — LICENSE-APACHE canonical text" |

For all four, the planner copies external canonical text verbatim. Do NOT paraphrase license text — SPDX detection tools require byte-exact matches.

## Metadata

**Analog search scope:**
- `crates/alloc-bench-aggregator/{src,tests,templates}/`
- `justfile` (full)
- `Cargo.toml` (workspace)
- `rust-toolchain.toml`
- `docker/*.Dockerfile`
- `.dive-ci`, `prek.toml`, `scripts/`
- `README.md`
- `.github/` (confirmed empty)

**Files scanned:** 14 source files + 4 fixtures + workspace + 6 Dockerfiles + 2 root configs

**Pattern extraction date:** 2026-05-19

**Key project-wide patterns identified:**
1. **Hand-written, no-deps stdlib computation modules** — `recommend.rs` is the canonical analog for `multi_run.rs`. Same crate, same tier, same `BTreeMap`-discipline, same `synth_run` test scaffold.
2. **Aggregator is decorate-not-rewrite** — every Phase 5 aggregator extension (markdown.rs, recommend.rs, index.html.tmpl) is in-place additive; the v1 schema (`alloc-bench-core/src/output.rs`) is locked.
3. **Justfile recipes are sequential `just X && just Y` compositions** — `bench-cell` is the model for `ci-bench-cell`; `aggregate` for `ci-aggregate`. Add the new recipes ALONGSIDE existing ones, not as replacements.
4. **Em-dash placeholder + Phase-X-backfill footnote** — Phase 4 already established the convention (`markdown.rs:181-188`). Phase 5 closes the loop by populating the cells when meta.json is present, but keeps em-dash as the no-data fallback.
5. **Tinytemplate literal-`{` escapes** — every `{` in the HTML/CSS/JS template body is `\{`. The `error_y` extension in index.html.tmpl follows the same rule.
6. **`rust-toolchain.toml` says `1.91`, Cargo.toml MSRV says `1.83`** — these are different fields with different semantics. Don't conflate them. CONTEXT.md's `1.83` reference is stale Phase-3-design carry-over; the codebase consensus is `1.91`.

---
phase: 04-aggregator-dashboard
plan: 01
subsystem: aggregator
tags: [tinytemplate, glob, plotly-cdn, sri-integrity, serde-deserialize, assert_cmd, predicates]

# Dependency graph
requires:
  - phase: 01-foundation-mvp-slice
    provides: v1 Run/Env/Build/ScenarioInfo/HarnessInfo/Metrics/LatencyNs/RssGrowthSample/Rusage schema in alloc-bench-core::output (Serialize-only); SCHEMA_VERSION=1 const.
  - phase: 02-scenario-fan-out
    provides: schema additive fields (scenario.unit, top-level status, top-level error); skip_serializing_if=Option::is_none preserves byte-identical Phase-1 output shape.
  - phase: 03-docker-matrix-local-orchestration
    provides: results/{alloc}-{env}.json flat-layout contract producing Vec<Run> arrays; justfile recipe convention (cargo run --release -p ...).
provides:
  - Working alloc-bench-aggregator binary with --input + --output CLI per D-05.
  - Read-side schema (Deserialize derive on all 9 v1 structs in alloc-bench-core::output) — one source of truth, no parallel struct hierarchy.
  - loader::discover (glob → sort_unstable → Vec<Run> primary parse + single Run fallback + schema_version reject + zero-match bail + skip-and-continue) per D-06, D-08.
  - Tinytemplate-rendered index.html skeleton with pinned Plotly 2.35.3 CDN URL, SRI integrity hash baked as Rust const, escaped CSS variables block from UI-SPEC, four chart-card slots, multi-select sidebar shell, A/B picker shell, stub onFilterChange (Plan 02 wires actual Plotly.react calls).
  - Minimal but reproducible REPORT.md emitter (single-timestamp comment, H1, run summary, alphabetically-sorted ## Runs bullet list, optional ## Skipped inputs section).
  - Three committed test fixtures covering Vec<Run> happy path, suspect run, and Phase-1 single-Run-object fallback.
  - Smoke integration test suite (4 tests) gating the four CLI exit paths.
  - just aggregate + just aggregate-smoke recipes (ORCH-03 / D-17 / D-18).
affects: [Phase 4 Plan 02 (chart trace builders), Phase 4 Plan 03 (per-scenario tables / Mermaid diagrams / Recommendations), Phase 5 (CI artifact upload + multi-run aggregation)]

# Tech tracking
tech-stack:
  added:
    - tinytemplate 1.2.1 (HTML template engine; single placeholder; criterion-author owned, 172M downloads)
    - glob 0.3.3 (Unix-glob input file discovery; rust-lang-owner, 441M downloads)
    - predicates 3.x (dev-dep; assert_cmd stderr substring matching with `.and(...)` boolean combinator)
  patterns:
    - One-source-of-truth schema via Deserialize derive on existing Serialize structs (no parallel struct hierarchy).
    - Tinytemplate brace-escaping pattern (`\{` for every literal `{` in CSS/JS bodies; `{ name | unescaped }` for the JSON blob).
    - Glob+sort_unstable (RESEARCH §Pitfall 3 — glob iteration order is undefined).
    - Vec<Run>-then-Run heterogeneous parse fallback for backwards compatibility with Phase-1 single-scenario emission.
    - Skip-and-continue per-file failure path (D-08 — partial failure does NOT fail-fast).
    - SRI integrity hash baked as Rust const for Plotly CDN tag (RESEARCH §Pitfall 4).
    - Compile-time template validation via `tinytemplate_compiles_index_template` unit test (catches missed `\{` escape at cargo test time, not runtime).

key-files:
  created:
    - crates/alloc-bench-aggregator/src/loader.rs (282 lines: glob+sort+parse+schema validation + 6 unit tests)
    - crates/alloc-bench-aggregator/src/markdown.rs (113 lines: minimal REPORT.md emitter)
    - crates/alloc-bench-aggregator/src/html.rs (188 lines: tinytemplate render + 2 unit tests)
    - crates/alloc-bench-aggregator/templates/index.html.tmpl (~190 lines: skeleton template with brace-escaping applied throughout)
    - crates/alloc-bench-aggregator/tests/fixtures/ptmalloc-debian-slim.json (Vec<Run> array, 2 scenarios)
    - crates/alloc-bench-aggregator/tests/fixtures/jemalloc-alpine.json (Vec<Run> array, 2 scenarios, one suspect)
    - crates/alloc-bench-aggregator/tests/fixtures/mimalloc-distroless-cc-single.json (single Run object, Phase-1 emission shape)
    - crates/alloc-bench-aggregator/tests/smoke.rs (4 integration tests: happy-path, zero-glob, partial-failure, all-files-fail)
  modified:
    - Cargo.toml (added tinytemplate=1, glob=0.3 to [workspace.dependencies] with slopcheck-false-positive comment)
    - Cargo.lock (auto-updated)
    - crates/alloc-bench-core/src/output.rs (added Deserialize to all 9 schema structs; added 2 round-trip tests)
    - crates/alloc-bench-aggregator/Cargo.toml (wired alloc-bench-core path dep + workspace anyhow/chrono/clap/glob/serde/serde_json/tinytemplate; dev-deps assert_cmd=2/predicates=3/tempfile=3)
    - crates/alloc-bench-aggregator/src/main.rs (wholesale rewrite: clap CLI + linear pipeline; NOT a bench binary so no version banner)
    - justfile (appended `aggregate` + `aggregate-smoke` recipes after check-matrix)

key-decisions:
  - Deserialize derive added to existing Serialize structs (one source of truth) instead of creating parallel `*Read` structs in the aggregator.
  - Tinytemplate template kept as a separate `.tmpl` file (not inline in Rust source) so brace-escaping is grep-auditable and the compile-test gate is meaningful.
  - schema_version mismatch handled via load_one bail! → discover loop catches → skip-and-continue (D-08 graceful degradation), not via top-level CLI failure. The user gets a SkippedFile entry in REPORT.md instead of a hard exit, even when ALL input files fail schema validation.
  - Pinned Plotly 2.35.3 CDN URL + SRI hash baked as Rust consts (`PLOTLY_CDN_URL`, `PLOTLY_SRI_HASH`) — single source of truth across the binary, the template substitution, and the smoke test assertions.
  - `predicates = "3"` added to dev-deps (analog `alloc-bench-cli` smoke does NOT depend on predicates; the aggregator smoke needs `.and(...)` for stderr substring combinations, so the dep is justified).

patterns-established:
  - "Brace escape gate: `tinytemplate_compiles_index_template` unit test catches missed `\\{` escapes at compile time (RESEARCH §Pitfall 1)"
  - "Glob → sort_unstable → process: byte-identical-output enforcement at the loader level (RESEARCH §Pitfall 3)"
  - "Vec<Run>-first parse, single-Run fallback: future input shapes can be added by extending the load_one ladder without disturbing existing callers"

requirements-completed: [AGG-01, AGG-02, AGG-03, ORCH-03]

# Metrics
duration: ~25min
completed: 2026-05-19
---

# Phase 4 Plan 1: alloc-bench-aggregator MVP Summary

**End-to-end aggregator binary: parses glob → loads Vec<Run> or single Run → emits Plotly-CDN-pinned + SRI-checked index.html and a reproducible REPORT.md, gated by a 4-test smoke suite and `just aggregate-smoke`.**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-05-19T00:53:40Z (worktree spawn)
- **Completed:** 2026-05-19T01:19:00Z (approx)
- **Tasks:** 3
- **Files created:** 8 (loader.rs, markdown.rs, html.rs, index.html.tmpl, 3 fixtures, smoke.rs)
- **Files modified:** 6 (Cargo.toml, Cargo.lock, output.rs, aggregator Cargo.toml, main.rs, justfile)

## Accomplishments

- Workspace dependencies wired: `tinytemplate = "1"` and `glob = "0.3"` added at workspace level (D-14). Aggregator opts in via `{ workspace = true }` on every dep. `predicates = "3"` added to dev-deps for `assert_cmd` stderr substring combinators.
- Read-side schema sealed: all 9 v1 structs in `alloc-bench-core::output` now derive `Deserialize` in addition to `Serialize`. Two new tests pin the round-trip invariant (`deserialize_round_trips_a_canonical_run`) and the D-06 forward-compat contract (`deserialize_ignores_unknown_fields`). No `#[serde(default)]` and no `#[serde(deny_unknown_fields)]` per RESEARCH §Pattern 2.
- Loader implementation matches RESEARCH §Pattern 2 verbatim: glob → filter_map(Result::ok) → collect → sort_unstable → for-each load_one → Ok/skip-and-continue. Six unit tests cover all six observable behaviors.
- HTML emitter renders a skeleton dashboard with the pinned Plotly 2.35.3 CDN URL + SRI integrity (`sha384-MqL7Cy3itNqCI1Wlc926K0XhyRKJ/NMqTaytIIEB+QIdInOploxqRIHRKLlhPykM`) + `crossorigin="anonymous"` baked in. The full UI-SPEC `:root` CSS variables block is embedded verbatim with every `{` escaped `\{`. Two unit tests gate the template at `cargo test` time.
- Markdown emitter ships a minimal but byte-deterministic REPORT.md (single timestamp comment, H1, summary line, alphabetically-sorted bullet list). Two consecutive runs produce byte-identical REPORT.md after stripping the timestamp.
- `just aggregate` + `just aggregate-smoke` recipes appended to the justfile (ORCH-03 / D-17 / D-18). Existing recipes are untouched.
- Smoke test suite gates four CLI exit paths: happy-path against fixtures, zero-glob exit-non-zero, partial-failure skip-and-continue (logs to stderr + ## Skipped inputs section in REPORT.md), all-files-fail graceful degradation (still exits 0 with empty-runs report).

## Aggregator Binary Surface

**CLI signature** (D-05, exact):
```
alloc-bench-aggregator --input "results/*.json" --output report/
```

- `--input` defaults to `results/*.json` (glob pattern; `glob` crate handles `*`/`?`/`**`).
- `--output` defaults to `report/` (directory; `create_dir_all` ensures it exists).

**Exit-code contract:**
- `0` — happy path; partial failure (some files skipped); all files fail (empty REPORT.md emitted).
- non-zero — invalid glob pattern; zero matches; HTML/REPORT.md write failure.

**stderr summary** (always emitted on success): `aggregated {N} runs, skipped {M}`.

**Output shapes:**
- `report/index.html` — tinytemplate-rendered dashboard skeleton; ~7KB; contains pinned Plotly CDN tag + SRI hash + inlined `const RESULTS = [...]` + four `<div id="chart-*">` cards + sidebar `<select multiple>` shells + A/B picker shells + stub `onFilterChange()`.
- `report/REPORT.md` — schema_version comment, `# alloc-bench REPORT`, `**N runs across M cells.**`, `## Runs` bulleted list (alphabetical by allocator·env·scenario), optional `## Skipped inputs` section.

## Pinned External References

```rust
pub(crate) const PLOTLY_CDN_URL: &str = "https://cdn.plot.ly/plotly-2.35.3.min.js";
pub(crate) const PLOTLY_SRI_HASH: &str =
    "sha384-MqL7Cy3itNqCI1Wlc926K0XhyRKJ/NMqTaytIIEB+QIdInOploxqRIHRKLlhPykM";
```

The hash was computed live during research (2026-05-19) via:
```bash
curl -s 'https://cdn.plot.ly/plotly-2.35.3.min.js' | openssl dgst -sha384 -binary | base64
```

The HTML script tag is rendered with `integrity="{plotly_sri_hash}" crossorigin="anonymous"` — both attributes are required because SRI is silently disabled in some browsers without `crossorigin`.

## Test Counts

- `alloc-bench-core` lib tests: 5 in `output.rs` (3 pre-existing + 2 new round-trip / forward-compat tests). All 5 pass plus 76 other lib tests across the crate.
- `alloc-bench-aggregator` unit tests in `loader.rs`: 6 (paths_sorted_lexicographically, vec_run_array_parses_as_n_runs, single_run_object_parses_as_one_run, schema_version_mismatch_rejects_with_path, glob_zero_matches_returns_error, partial_failure_skips_and_continues).
- `alloc-bench-aggregator` unit tests in `html.rs`: 2 (tinytemplate_compiles_index_template, render_inlines_results_json_unescaped).
- `alloc-bench-aggregator` integration tests in `tests/smoke.rs`: 4 (happy-path, zero-glob, partial-failure, all-files-fail).

**Total Plan 01 tests:** 12 new (8 unit + 4 integration) + 2 new pre-existing-file tests in output.rs.

## Sample stderr from `just aggregate` against fixtures

```
$ ./target/release/alloc-bench-aggregator \
    --input 'crates/alloc-bench-aggregator/tests/fixtures/*.json' \
    --output /tmp/agg-fixt-out
aggregated 5 runs, skipped 0
```

Output files: `/tmp/agg-fixt-out/index.html` (~7 KB) + `/tmp/agg-fixt-out/REPORT.md` (~600 bytes).

Sample REPORT.md:
```markdown
<!-- schema_version: 1 · generated by alloc-bench-aggregator at 2026-05-19T01:08:23+00:00 -->
# alloc-bench REPORT

**5 runs across 3 cells.**

## Runs

- jemalloc · alloc-bench:jemalloc-alpine · cpu-bound: 110.0 ticks/s
- jemalloc · alloc-bench:jemalloc-alpine · multithread: 100.0 ticks/s
- mimalloc · alloc-bench:mimalloc-distroless-cc · multithread: 110.0 ticks/s
- ptmalloc · alloc-bench:ptmalloc-debian-slim · cpu-bound: 60.0 ticks/s
- ptmalloc · alloc-bench:ptmalloc-debian-slim · multithread: 80.0 ticks/s
```

## Task Commits

Each task was committed atomically with conventional-commit prefix `feat(04-01)`:

1. **Task 1: Workspace + aggregator deps + Deserialize derives + sanity round-trip test** — `f2a8a61` (feat)
2. **Task 2: Loader + main.rs + minimal markdown.rs + minimal html.rs + tinytemplate template skeleton** — `8378ec1` (feat)
3. **Task 3: Test fixtures + smoke integration test + justfile recipes** — `79d8108` (feat)

## Files Created/Modified

### Created

- `crates/alloc-bench-aggregator/src/loader.rs` — discover() + load_one() + 6 unit tests; LoadOutcome / SkippedFile structs.
- `crates/alloc-bench-aggregator/src/markdown.rs` — write() emits a minimal REPORT.md; env_label() helper shared with html.rs; cell-counter via BTreeSet for byte-identical-output guarantees.
- `crates/alloc-bench-aggregator/src/html.rs` — write() + render() + count_unique_cells() + 2 unit tests; HtmlContext struct for the tinytemplate substitution; PLOTLY_CDN_URL + PLOTLY_SRI_HASH consts.
- `crates/alloc-bench-aggregator/templates/index.html.tmpl` — skeleton dashboard with brace-escaping applied throughout; CDN script tag uses {plotly_cdn_url} + {plotly_sri_hash} placeholders; the only `unescaped` placeholder is `{ results_json | unescaped }`.
- `crates/alloc-bench-aggregator/tests/fixtures/ptmalloc-debian-slim.json` — 2-scenario Vec<Run> array, status=success.
- `crates/alloc-bench-aggregator/tests/fixtures/jemalloc-alpine.json` — 2-scenario Vec<Run> array, one suspect run (samples_count=5000).
- `crates/alloc-bench-aggregator/tests/fixtures/mimalloc-distroless-cc-single.json` — single Run object (no top-level status/error keys), Phase-1 single-scenario emission shape.
- `crates/alloc-bench-aggregator/tests/smoke.rs` — 4 assert_cmd integration tests; reuses make_synthetic_run helper for the schema_version=999 fixture.

### Modified

- `Cargo.toml` — added tinytemplate=1 + glob=0.3 to [workspace.dependencies] with slopcheck-false-positive doc comment.
- `Cargo.lock` — auto-updated via cargo build.
- `crates/alloc-bench-core/src/output.rs` — Deserialize derive added to all 9 v1 structs; 2 new tests appended to mod tests.
- `crates/alloc-bench-aggregator/Cargo.toml` — alloc-bench-core path dep + 7 workspace deps + 3 dev-deps wired.
- `crates/alloc-bench-aggregator/src/main.rs` — wholesale-rewrite from Phase-1 placeholder eprintln to a real binary (clap CLI + linear pipeline + final stderr summary).
- `justfile` — appended `aggregate` + `aggregate-smoke` recipes (preserves existing recipes verbatim).

## Decisions Made

- **Schema-version mismatch is per-file skip-and-continue, not top-level fail-fast.** D-08 mandates "≥ 1 file parses but some files fail → continue with valid ones, exit 0; bad files listed in REPORT.md → Skipped Inputs". My implementation puts the bail! inside `load_one` (per RESEARCH §Pattern 2), then catches it in the discover loop. As a side effect, even when ALL input files fail (e.g., the all-files-fail smoke test), the aggregator still exits 0 with an empty-runs REPORT.md that lists every bad file. This matches D-08's graceful-degradation philosophy and lets a CI artifact downloader re-run without manual cleanup.
- **`predicates = "3"` added to dev-deps** even though the analog `alloc-bench-cli/tests/run_all_smoke.rs` doesn't use it. Justification: Task 3's smoke tests need stderr substring AND-combinations (e.g., assert stderr contains both "warn: skipped" AND "bad.json"); the `predicates::prelude::PredicateBooleanExt` trait provides the `.and(...)` combinator. Falling back to `cmd.output()` + manual `String::from_utf8` would have been more code and less idiomatic.
- **`PLOTLY_SRI_HASH` is `pub(crate)` not `pub`.** The smoke test asserts on a prefix (`sha384-MqL7Cy3i`) rather than importing the full constant, to avoid line-wrap fragility in PR review tools. Keeping the const crate-private signals "internal pin, not API surface" and matches the style of similar pinned constants elsewhere in the workspace.

## Deviations from Plan

None - plan executed exactly as written.

The plan's task breakdown, file list, behavior specifications, and acceptance criteria were followed verbatim. The only minor adjustment was the `predicates` import path: the plan suggested `use predicates::str::contains;` which suffices for single-substring checks but does not include the `.and(...)` combinator. Adding `use predicates::prelude::PredicateBooleanExt;` to the smoke test enables the boolean combinator the plan explicitly asks for in Task 3 acceptance criterion ("assert stderr contains 'warn: skipped' AND 'bad.json' AND 'schema_version mismatch'"). This is internal to Task 3 and matches the plan's stated intent — not a deviation requiring a Rule 3 auto-fix annotation.

---

**Total deviations:** 0
**Impact on plan:** None — plan executed exactly as specified.

## Issues Encountered

- **Initial `LoadOutcome` lacked `Debug` derive.** The `glob_zero_matches_returns_error` test calls `discover(...).unwrap_err()`, which requires `T: Debug` on `Result<T, E>`. Trivial Rule-3 fix: added `#[derive(Debug)]` to both `LoadOutcome` and `SkippedFile` (Task 2 commit, before any other test exercise). Resolved during initial test run.
- **fmt rearranged `use` statements** in `loader.rs`, `markdown.rs`, `html.rs`, `smoke.rs` after initial drafts (e.g., merged `alloc_bench_core::output::Run` and `alloc_bench_core::SCHEMA_VERSION` use lines into different orders than I wrote). Cosmetic; addressed via `cargo fmt --all` and committed cleanly.

## User Setup Required

None — Phase 4 Plan 01 introduces only build-time and runtime dependencies that resolve via `cargo build`. No external services, no secrets, no environment variables.

## Next Phase Readiness

**Plan 02 ready:** the aggregator's loader/markdown/html surfaces are stable and unit-tested. Plan 02 fleshes out:
- Plotly chart trace builders (throughput bar + grouped, latency heatmap, RSS-over-time line, A/B comparison-diff bar) — implemented in JS inside the template, driven by the inlined `RESULTS` array.
- Filter handler logic — vanilla-JS multi-select sidebar that filters `RESULTS` and calls `Plotly.react()` per chart card.
- Suspect-badge rendering — `harness.samples_count < 10000 OR warmup_duration_s < 5.0` predicate (D-07) renders a `⚠` glyph next to the affected legend entry / picker option.

**Plan 03 ready:** markdown.rs has a clean extension point for:
- Per-scenario allocator comparison tables with `✓` winner highlighting (D-09).
- Docker runtime comparison table (D-10) — emit `—` for the `image_size_mb` / `build_time_s` / `run_overhead_pct` columns absent from the v1 schema.
- Mermaid allocator architecture diagrams (D-11) — committed as `&'static str` constants in a new `diagrams.rs` module.
- Recommendations table (D-12) — workload-class → allocator picker that derives every rationale string from data.
- README.md system diagram (AGG-08, D-13) — manual edit.

**Phase 5 readiness:** the aggregator runs from a Docker-less host, so a Phase 5 CI artifact downloader can `cargo run --release -p alloc-bench-aggregator` directly against unpacked GH Actions artifacts. No blockers.

---

*Phase: 04-aggregator-dashboard*
*Completed: 2026-05-19*

## Self-Check: PASSED

All claimed files exist and all task commits are present in `git log`:

- crates/alloc-bench-aggregator/src/{loader,markdown,html}.rs
- crates/alloc-bench-aggregator/templates/index.html.tmpl
- crates/alloc-bench-aggregator/tests/fixtures/{ptmalloc-debian-slim,jemalloc-alpine,mimalloc-distroless-cc-single}.json
- crates/alloc-bench-aggregator/tests/smoke.rs
- Commits f2a8a61, 8378ec1, 79d8108


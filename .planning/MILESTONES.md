# Milestones

## v1.1 Recommendations, Spider Charts & Direction Markers (Shipped: 2026-05-30)

**Phases completed:** 6 phases (Phases 6-11), 14 plans, 27 tasks
**Requirements:** 32/32 satisfied (audit status: tech_debt — see `.planning/milestones/v1.1-MILESTONE-AUDIT.md`)
**Git range:** 261946c → 162161f (108 commits, 2026-05-26 → 2026-05-30)

**Key accomplishments:**

- **Phase 6 (Foundations):** `MEASUREMENT_AXES: [AxisSpec; 8]` registry + `Direction::{Higher, Lower}` enum + `arrow()` helper in `axes.rs`; six hand-curated `meta/security/{env}.json` sidecars loaded via `loader::load_security_metas() -> BTreeMap<String, SecurityMeta>`; `--security` CLI flag with em-dash fallback; SHA-256-pinned `v1_schema_output_rs_is_frozen` integration test guarding against accidental v1 schema mutation (AXES-01..02, SEC-01..03, GUARD-01).
- **Phase 7 (Scoring & Top-N):** `score.rs` data-only scoring module (`normalize_axis`, `compute_axes`, `score_cells`, `top_n`) with p10/p90 winsorization, equal-weight composite via `MEASUREMENT_AXES.iter()` constant traversal, alphabetical `(alloc, env)` tiebreak, and NaN-poisoning guard; `recommend.rs` extended with `CellRecommendation` struct + `top_n_cells()` + three locked constants `TOP_N_SPIDER=3 / TOP_N_TABLE=5 / TOP_N_TOTAL=10`; existing 13 `recommendations()` tests untouched (SCORE-01..04, REC-01..02, TEST-03..05).
- **Phase 8 (Per-cell Artifacts):** Two-template (`recommend-cell.md.tmpl` + `recommend-cell.html.tmpl`) driven by the same `CellRecommendation` — Markdown card + HTML panel field-by-field identical, drift caught at compile time via `cell_templates_both_reference_all_fields` sentinel test; ten per-cell `.md`/`.html` fragments emitted to `report/`; `## Top 10 cells` section in REPORT.md (top-5 above-fold, 5 in `<details>`); data-derived TL;DR → Strengths → Weaknesses → Recommended-for → Avoid-for prose (CELL-01..05).
- **Phase 9 (Spider Chart):** `polar.rs` shipped with `build_trace`, `build_reference_trace`, `axis_label_for_chart` and 9 unit tests locking the 9-element polygon-closure invariant; `<div id="chart-spider">` renders top-3 cells above-fold as small-multiples grid with matrix-mean reference polygon at 25% alpha; `(heuristic)` suffix + per-tick tickfont color #666 distinguishes heuristic axes; `PLOTLY_SRI_HASH` constant + `plotly_sri_hash_unchanged` test pin v2.35.3; `pareto_front` + `★` glyph add Pareto-front overlay column (POLAR-01..05).
- **Phase 10 (Direction Markers):** `column_header_with_arrow` SSoT helper added to `axes.rs`, consumed by both `markdown::emit_per_scenario_tables` (REPORT.md surface) and `html::build_context` (chart axis labels — index.html surface); verbatim `↑ higher is better · ↓ lower is better · ⚠ suspect run` legend above every per-scenario table; `<span aria-label="…">` aria-wrapping for WCAG 2.1 SC 1.3.3 conformance via server-side template + JS post-render pass; cells unchanged — markers live in headers only (DIR-01..05).
- **Phase 11 (Golden-fixture Regen):** v1.1 release gate — standalone doc-only PR pattern codified in CLAUDE.md §Conventions; v1.0 byte-identical-output golden tests still pass through Phase 6-10 additions; rendered-fixture byte-counts captured for the v1.1 lock (TEST-01..02).

**Technical debt deferred to v1.2:** Phase 7 missing `07-VERIFICATION.md` (code-verified via SUMMARYs + integration check; documentation gap only); `env_short_name` duplicated in `score.rs:125` + `recommend.rs:457` (acknowledged in code as v1.2 consolidation candidate).

**Known deferred items at close:** 8 (see STATE.md Deferred Items — 6 quick-task frontmatter status fields + 2 tech-debt items above).

---

## v1.0 MVP (Shipped: 2026-05-19)

**Phases completed:** 5 phases, 17 plans, 71 tasks

**Key accomplishments:**

- Cargo workspace with allocator feature flags, hand-rolled build metadata injection, and Walking Skeleton CLI printing the locked version banner
- Real harness loop with HDR-histogram percentile latency, getrusage + /proc/self/statm metrics, multithread allocation stress scenario, and locked v1 results.json schema emitted end-to-end
- Six new scenarios (SCEN-03/04/05/07/08/10) implementing the synchronous, non-async portion of Phase 2's fan-out on top of the locked Phase-1 harness contract, plus the additive `ScenarioInfo.unit` schema field.
- Three heavier Phase-2 scenarios (SCEN-02 web with in-process axum+tokio+reqwest, SCEN-06 cpu-bound with scoped rayon merge-sort, SCEN-09 fragmentation-soak with cross-tick state and capped long-lived buffers) implementing the async/heavy-dep portion of Phase 2 on top of Plan 01's foundation, plus the workspace deps for axum/tokio/tower/reqwest/rayon.
- SCEN-11 wires all 10 scenarios into a single `run-all` registry with `panic::catch_unwind` per-scenario isolation, adds the DCE verification recipe (Phase-2 ROADMAP success criterion 4), and an end-to-end integration test. Phase-2 closes with all 4 ROADMAP success criteria demonstrably met.
- Three glibc-family Dockerfiles (debian-slim, distroless-cc, wolfi) sharing a cargo-chef 3-stage builder, plus `.dockerignore` and `.dive-ci` — all linted clean via `docker buildx build --check --platform linux/amd64`.
- Three musl-family Dockerfiles (alpine dynamic, distroless-static, scratch) sharing a 4-stage cargo-chef builder with ALLOC=mallocng/jemalloc/mimalloc selection and OCI annotations.
- Justfile extended with 10 Phase-3 recipes (build, run, bench-cell, bench-all, bench-all-smoke, bench-host, dive-check, dive-check-all, clean-images, check-matrix) + an 18-cell `_matrix_cells` heredoc — wires the Wave-1 Dockerfiles into a sequential matrix loop with per-cell error capture, Apple Silicon platform pinning, distroless nonroot mount fix, and D-04 cross-libc rejection.
- End-to-end aggregator binary: parses glob → loads Vec<Run> or single Run → emits Plotly-CDN-pinned + SRI-checked index.html and a reproducible REPORT.md, gated by a 4-test smoke suite and `just aggregate-smoke`.
- Inline JS chart trace builders + multi-select filter + A/B diff picker — Plan 01's skeleton dashboard becomes interactive: four populated Plotly charts on first paint, in-place re-renders via Plotly.react on filter change, ⚠-prefix suspect labels, identical-AB note, suspect-config banner, empty-filter heading.
- Per-scenario allocator comparison tables (winner prefix + suspect italic notes), Docker runtime table with em-dash cells, four Mermaid allocator-architecture diagrams, data-derived recommendations table, README system diagram with locked verbatim paragraph — Plan 02's interactive dashboard becomes a readable benchmark report.
- Pure-stdlib `multi_run.rs` module providing Bessel-corrected sample stddev, median, min/max, and coefficient-of-variation with high-variance flag (CV > 10%) — plus three Vec<Run> seed fixtures and one sidecar meta fixture for downstream Plan 03 integration tests.
- 18-cell GHA matrix workflow (pre-bench → bench-matrix → aggregate) with dive --ci image-size enforcement, 3-seeded runs per cell, meta.json sidecar capture, and dual-licensed LICENSE files at repo root — all wired through three new justfile recipes that keep local-machine repro byte-identical to CI.
- Wires the Plan-01 multi_run statistics module + Plan-02 meta.json sidecar shape into the aggregator's REPORT.md emit, recommend.rs winner picker, and HTML dashboard. Decorates per-scenario throughput cells with `{median} ({min}..{max}, CV {N}%)` when ≥2 runs share an `(alloc, env, scenario)` triple, flags `⚠ high variance` when CV>10%, and backfills `image_size_mb` in the Docker runtimes table from the sidecar when `--meta` is supplied. The v1 input schema is NOT modified — every change is aggregator-output decoration.
- Polished `README.md` so a reader who has never seen the repo can reproduce a representative subset of results from scratch (REPR-01) — preserving the Phase 4 system diagram byte-identically while prepending the CI badge and appending `## Run it yourself`, `## Allocator matrix overview`, `## Reproducibility`, and `## License` sections. Also recorded the 9 cross-phase conventions established in Phases 1-5 in CLAUDE.md.

---

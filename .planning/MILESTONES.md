# Milestones

## v1.0 v1.0 MVP (Shipped: 2026-05-19)

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

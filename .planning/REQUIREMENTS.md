# Requirements: rust-benchmark-glibc-musl-mimalloc

**Defined:** 2026-05-17
**Core Value:** Every result is reproducible, environment-labelled, and visually comparable — so the reader can confidently recommend the right allocator for a given workload.

## v1 Requirements

### Workspace & Build

- [ ] **WS-01**: User can build the benchmark binary as a Cargo workspace with one binary crate (`alloc-bench-cli`) plus core library (`alloc-bench-core`) and aggregator (`alloc-bench-aggregator`)
- [ ] **WS-02**: User can select the global allocator at build time via Cargo features (`alloc-jemalloc`, `alloc-mimalloc`, default = system) — feature flags are mutually exclusive at runtime
- [ ] **WS-03**: User runs the benchmark and the binary prints rustc version, target triple, host triple, profile, git SHA, and build timestamp at startup (injected at compile time via `build.rs`)
- [ ] **WS-04**: User builds with `[profile.release] lto = "fat", codegen-units = 1, opt-level = 3, strip = "symbols", debug = false` configured in Cargo.toml
- [ ] **WS-05**: User builds Docker images with `RUSTFLAGS="-C target-cpu=x86-64-v3"` (portable across CI runners) and host builds with `target-cpu=native`

### Benchmark Harness & Metrics

- [ ] **HARN-01**: User runs a scenario and the harness performs a configurable warm-up (default 5s, minimum 1s) before measurement
- [ ] **HARN-02**: User runs a scenario and the harness records per-operation latency to an HDR histogram and emits p50/p95/p99/p999/max in nanoseconds
- [ ] **HARN-03**: User runs a scenario and the harness samples `/proc/self/statm` every 1s producing an RSS-over-time array
- [ ] **HARN-04**: User runs a scenario and the harness reads `getrusage(RUSAGE_SELF)` at end-of-run for peak RSS, user/sys CPU time, page faults, and context switches
- [ ] **HARN-05**: User runs a scenario with jemalloc and the harness emits jemalloc-internal stats (allocated, resident, retained, active) via `tikv_jemalloc_ctl`
- [ ] **HARN-06**: User runs a scenario with mimalloc and the harness emits mimalloc-internal stats via the extended-feature stats API
- [ ] **HARN-07**: User runs a scenario and the harness wraps every measured tick in `std::hint::black_box` to prevent dead-code elimination
- [ ] **HARN-08**: User runs a scenario and the harness emits a single results.json record matching the schema (env + build + scenario + harness + metrics)

### Benchmark Scenarios

- [ ] **SCEN-01**: User runs `alloc-bench multithread` with `--threads N --objects M --size-dist <uniform|bimodal|pareto>` (the anchor multi-thread allocation stress benchmark)
- [ ] **SCEN-02**: User runs `alloc-bench web` (axum + serde_json + tokio) with `--server-workers N --client-workers M --duration 60s`, measuring throughput (req/s) and p50/p95/p99/p999 latency
- [ ] **SCEN-03**: User runs `alloc-bench spmc` with `--producers 1 --consumers N --payload-dist <distribution>`
- [ ] **SCEN-04**: User runs `alloc-bench mpsc` with `--producers N --consumers 1 --payload-dist <distribution>`
- [ ] **SCEN-05**: User runs `alloc-bench mpmc` with `--producers N --consumers M --payload-dist <distribution>`
- [ ] **SCEN-06**: User runs `alloc-bench cpu-bound` (parallel merge-sort with allocations in the critical path) with `--threads N --input-size MB`
- [ ] **SCEN-07**: User runs `alloc-bench mem-bound` with `--mode <linked-list|strided-array> --size MB`
- [ ] **SCEN-08**: User runs `alloc-bench contention` (high thread count, same-size rapid alloc/free) with `--threads N`
- [ ] **SCEN-09**: User runs `alloc-bench fragmentation-soak` (long-running mixed alloc/free with biased sizes) with `--duration <minutes>`
- [ ] **SCEN-10**: User runs `alloc-bench realloc-storm` (Vec::push under pressure) with `--target-size MB`
- [ ] **SCEN-11**: User runs `alloc-bench run-all --output results/run.json` to execute all scenarios sequentially with default configs and emit a single combined results.json

### Docker & Environments

- [ ] **DOCK-01**: User builds the `alpine` image (musl dynamic) via `docker/alpine.Dockerfile` using cargo-chef multi-stage build with OCI annotations
- [ ] **DOCK-02**: User builds the `debian-slim` image (glibc dynamic) via `docker/debian-slim.Dockerfile`
- [ ] **DOCK-03**: User builds the `distroless-cc` image via `docker/distroless-cc.Dockerfile` (for ptmalloc/jemalloc/mimalloc on glibc)
- [ ] **DOCK-04**: User builds the `distroless-static` image via `docker/distroless-static.Dockerfile` (musl static binary)
- [ ] **DOCK-05**: User builds the `scratch` image via `docker/scratch.Dockerfile` (fully static musl + crt-static)
- [ ] **DOCK-06**: User builds the `chainguard-static` or `wolfi` image via `docker/chainguard.Dockerfile`
- [ ] **DOCK-07**: User runs `dive --ci` against any image and the wasted-bytes/efficiency thresholds pass
- [ ] **DOCK-08**: Every Docker image carries OCI annotations: `org.opencontainers.image.{title,description,source,version,revision,licenses,created,authors}`
- [ ] **DOCK-09**: User runs `docker run --cpus=4 --memory=4g --cpuset-cpus=0-3 alloc-bench:<allocator>-<env> run-all` and gets results.json on stdout / mounted volume

### Orchestration & CI

- [ ] **ORCH-01**: User runs `just bench-all` and the Justfile builds + runs the full (allocator × env) matrix, emitting one results.json per cell
- [ ] **ORCH-02**: User runs `just bench-host` and the Justfile builds + runs the bench natively on the host (macOS libmalloc baseline)
- [x] **ORCH-03**: User runs `just aggregate` and the Justfile invokes `alloc-bench-aggregator` to produce `report/index.html` + `REPORT.md`
- [x] **ORCH-04**: User pushes to GitHub and a CI workflow runs the matrix (excluding macOS-specific cells) on `ubuntu-24.04`, uploading `results/` and `report/` as artifacts
- [x] **ORCH-05**: CI runs `dive --ci` for each image and fails the build if image-size thresholds are exceeded

### Aggregator & Reporting

- [x] **AGG-01**: User runs `alloc-bench-aggregator --input "results/*.json" --output report/` and a `report/index.html` is generated with Plotly.js charts (results inlined; opens via `file://`)
- [x] **AGG-02**: The HTML dashboard supports filtering by scenario, env, allocator (multi-select sidebar) and supports side-by-side comparison of two configs
- [x] **AGG-03**: The HTML dashboard renders: throughput bar chart (grouped by scenario, colored by allocator, faceted by env), latency-percentile heatmap, RSS-over-time lines, and a comparison-diff bar chart
- [x] **AGG-04**: User reads `REPORT.md` and finds a Markdown side-by-side comparison table of all 4 allocators across all scenarios with the winner highlighted per row
- [x] **AGG-05**: User reads `REPORT.md` and finds a Docker runtime comparison table (image size, build time, run-time overhead)
- [x] **AGG-06**: User reads `REPORT.md` and finds Mermaid.js architecture diagrams for ptmalloc, mallocng, jemalloc, and mimalloc explaining each allocator's structure
- [x] **AGG-07**: User reads `REPORT.md` and finds a "Recommendations" section mapping workload-shape → recommended allocator
- [x] **AGG-08**: User reads `README.md` and finds an overall Mermaid.js system diagram of how memory allocation works on modern Linux

### Reproducibility & Documentation

- [x] **REPR-01**: User reads `README.md` and finds a complete "Run it yourself" walkthrough including Docker prerequisites, `just bench-all`, and how to view `report/index.html`
- [ ] **REPR-02**: Every results.json includes the cpu_model, cpu_count, kernel_version, docker_image, rustc_version, target_triple, git_sha, and rustflags so any reader can reproduce the run
- [x] **REPR-03**: Each matrix cell runs ≥ 3 times in CI; aggregator reports median + min/max range across runs

## v2 Requirements (deferred)

### Extended Allocators

- **V2-01**: Add `snmalloc` to the matrix (deferred — clean baseline first)
- **V2-02**: Add `tcmalloc` to the matrix
- **V2-03**: Add `rpmalloc` to the matrix

### Extended Workloads

- **V2-04**: NUMA-aware multi-socket scaling test
- **V2-05**: Compiler-style allocation pattern (mimic rustc / clang allocation profiles)
- **V2-06**: Database-style allocation pattern (long-lived vs short-lived mix matching real DB workloads)

### Extended Reporting

- **V2-07**: Marimo notebook output as alternative to Plotly HTML
- **V2-08**: Continuous benchmark tracking with regression detection (cargo-criterion-style trend lines)
- **V2-09**: Cross-architecture results (aarch64 in addition to x86_64)

## Out of Scope

| Feature | Reason |
|---------|--------|
| Windows targets | All allocator-comparable runs are Linux-only; Windows would need a separate matrix |
| Allocator runtime swap (LD_PRELOAD) | Compile-time selection gives cleaner, more reproducible results |
| Criterion HTML reports | Custom harness is the single source of truth; Criterion is microbench-shaped |
| Marimo notebooks | Plotly HTML covers interactive needs; Marimo deferred |
| Live web dashboard with backend | Static HTML opens via `file://`; no server complexity |
| Android / embedded targets | Out of scope for this allocator comparison |
| HTTPS in web bench | Localhost HTTP only; HTTPS would require bundled CA certs in scratch image |
| GUI for the benchmark runner | CLI is sufficient; HTML dashboard handles visualization |

## Traceability

Coverage: 49/49 v1 requirements mapped to exactly one phase. No orphans.

| Requirement | Phase | Status |
|-------------|-------|--------|
| WS-01 | Phase 1 | Pending |
| WS-02 | Phase 1 | Pending |
| WS-03 | Phase 1 | Pending |
| WS-04 | Phase 1 | Pending |
| WS-05 | Phase 1 | Pending |
| HARN-01 | Phase 1 | Pending |
| HARN-02 | Phase 1 | Pending |
| HARN-03 | Phase 1 | Pending |
| HARN-04 | Phase 1 | Pending |
| HARN-05 | Phase 1 | Pending |
| HARN-06 | Phase 1 | Pending |
| HARN-07 | Phase 1 | Pending |
| HARN-08 | Phase 1 | Pending |
| SCEN-01 | Phase 1 | Pending |
| SCEN-02 | Phase 2 | Pending |
| SCEN-03 | Phase 2 | Pending |
| SCEN-04 | Phase 2 | Pending |
| SCEN-05 | Phase 2 | Pending |
| SCEN-06 | Phase 2 | Pending |
| SCEN-07 | Phase 2 | Pending |
| SCEN-08 | Phase 2 | Pending |
| SCEN-09 | Phase 2 | Pending |
| SCEN-10 | Phase 2 | Pending |
| SCEN-11 | Phase 2 | Pending |
| DOCK-01 | Phase 3 | Pending |
| DOCK-02 | Phase 3 | Pending |
| DOCK-03 | Phase 3 | Pending |
| DOCK-04 | Phase 3 | Pending |
| DOCK-05 | Phase 3 | Pending |
| DOCK-06 | Phase 3 | Pending |
| DOCK-07 | Phase 3 | Pending |
| DOCK-08 | Phase 3 | Pending |
| DOCK-09 | Phase 3 | Pending |
| ORCH-01 | Phase 3 | Pending |
| ORCH-02 | Phase 3 | Pending |
| ORCH-03 | Phase 4 | Complete |
| AGG-01 | Phase 4 | Complete |
| AGG-02 | Phase 4 | Complete |
| AGG-03 | Phase 4 | Complete |
| AGG-04 | Phase 4 | Complete |
| AGG-05 | Phase 4 | Complete |
| AGG-06 | Phase 4 | Complete |
| AGG-07 | Phase 4 | Complete |
| AGG-08 | Phase 4 | Complete |
| ORCH-04 | Phase 5 | Complete |
| ORCH-05 | Phase 5 | Complete |
| REPR-01 | Phase 5 | Complete |
| REPR-02 | Phase 1 | Pending |
| REPR-03 | Phase 5 | Complete |

### Coverage by Phase

| Phase | Requirement Count | Requirements |
|-------|-------------------|--------------|
| Phase 1 — Foundation MVP Slice | 15 | WS-01..05, HARN-01..08, SCEN-01, REPR-02 |
| Phase 2 — Scenario Fan-Out | 10 | SCEN-02..11 |
| Phase 3 — Docker Matrix & Local Orchestration | 11 | DOCK-01..09, ORCH-01, ORCH-02 |
| Phase 4 — Aggregator & Dashboard | 9 | AGG-01..08, ORCH-03 |
| Phase 5 — CI, Image-Size Gate & Public Polish | 4 | ORCH-04, ORCH-05, REPR-01, REPR-03 |
| **Total** | **49** | — |

---
*Requirements defined: 2026-05-17*
*Last updated: 2026-05-17 after roadmap creation*

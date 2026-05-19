# Roadmap: rust-benchmark-glibc-musl-mimalloc

## Overview

Deliver a reproducible Rust allocator benchmark suite end-to-end in five MVP-shaped phases. Phase 1 stands up a complete vertical slice (workspace + harness + first scenario + results.json) for one allocator combo so the loop is provably real. Phase 2 fans out to all remaining scenarios on top of the same harness contract. Phase 3 wraps the matrix in Docker (six images) and the local Justfile so any combo runs reproducibly. Phase 4 turns `results/*.json` into a Plotly dashboard plus a Markdown report with Mermaid diagrams. Phase 5 wires the matrix into GitHub Actions CI with image-size gates and lands the public-facing README + recommendations.

## Phases

**Phase Numbering:**

- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Foundation MVP Slice** - Workspace, allocator features, harness, first scenario, results.json — one allocator end-to-end (completed 2026-05-18)
- [x] **Phase 2: Scenario Fan-Out** - Remaining ten benchmark scenarios on top of the harness contract (completed 2026-05-18)
- [ ] **Phase 3: Docker Matrix & Local Orchestration** - Six Dockerfiles, Justfile bench-all, NUMA/cgroup hardening
- [x] **Phase 4: Aggregator & Dashboard** - alloc-bench-aggregator, Plotly HTML, REPORT.md with Mermaid diagrams (completed 2026-05-19)
- [ ] **Phase 5: CI, Image-Size Gate & Public Polish** - GitHub Actions matrix, Dive gate, README walkthrough, recommendations

**Justification of phase shape:** The research-proposed 5-phase decomposition is preserved verbatim because it already matches coarse granularity (5 phases, each shippable in MVP mode) and because Phases 4 and 5 deliver independently verifiable artifacts (a working dashboard vs. a green CI matrix + public docs). Collapsing them would push 13 requirements into one phase — too coarse even for the coarse setting — and would block early review of the dashboard until CI is also green.

## Phase Details

### Phase 1: Foundation MVP Slice

**Goal**: User can build the workspace for one allocator combo, run the multi-thread allocation scenario, and get a fully-populated `results.json` proving the harness loop, the metrics pipeline, and the build-metadata injection all work end-to-end.
**Mode:** mvp
**Depends on**: Nothing (first phase)
**Requirements**: WS-01, WS-02, WS-03, WS-04, WS-05, HARN-01, HARN-02, HARN-03, HARN-04, HARN-05, HARN-06, HARN-07, HARN-08, SCEN-01, REPR-02
**Success Criteria** (what must be TRUE):

  1. User runs `cargo build --release --no-default-features --features alloc-jemalloc -p alloc-bench-cli` and gets a stripped, fat-LTO, single-codegen-unit binary that prints rustc version, target triple, host triple, profile, git SHA, and build timestamp on `--version` (or first line of stdout).
  2. User runs `alloc-bench-cli multithread --threads 8 --objects 100000 --size-dist uniform --warmup 5s --duration 30s --output run.json` and the binary executes a 5s warm-up followed by 30s of measured ticks with every value black-boxed.
  3. The emitted `run.json` validates against the schema and contains throughput, p50/p95/p99/p999/max latencies in ns, peak RSS from `getrusage`, an RSS-over-time array sampled every 1s from `/proc/self/statm`, page-fault and context-switch counts from `getrusage`, allocator-internal stats (jemalloc-ctl when built with `alloc-jemalloc`, mimalloc extended stats when built with `alloc-mimalloc`), and a fully-populated env+build block (cpu_model, cpu_count, kernel_version, rustc_version, target_triple, git_sha, rustflags) sufficient for any reader to reproduce the run.
  4. User attempts to build with both `alloc-jemalloc` and `alloc-mimalloc` enabled simultaneously and the binary panics on startup with a clear "mutually exclusive allocator features" message (mutual exclusion enforced).

**Plans**: TBD

### Phase 2: Scenario Fan-Out

**Goal**: User can run every benchmark scenario the project ships with, plus a single `run-all` command that executes them sequentially with default configs, all on top of the Phase 1 harness contract and emitting the same `results.json` schema.
**Mode:** mvp
**Depends on**: Phase 1
**Requirements**: SCEN-02, SCEN-03, SCEN-04, SCEN-05, SCEN-06, SCEN-07, SCEN-08, SCEN-09, SCEN-10, SCEN-11
**Success Criteria** (what must be TRUE):

  1. User runs `alloc-bench-cli web --server-workers 4 --client-workers 16 --duration 60s` and gets a results.json reporting axum+serde_json+tokio throughput in req/s with p50/p95/p99/p999 latency, where the in-process load generator and server share the same global allocator.
  2. User runs each of `spmc`, `mpsc`, `mpmc`, `cpu-bound`, `mem-bound` (with `--mode linked-list` and `--mode strided-array`), `contention`, `fragmentation-soak`, and `realloc-storm` with their documented CLI flags and each emits a schema-valid results.json with non-trivial throughput and percentile data.
  3. User runs `alloc-bench-cli run-all --output results/run.json` and a single combined results.json contains one record per scenario in execution order; total runtime is approximately the sum of per-scenario durations.
  4. User builds `cargo build --release --emit=llvm-ir` for each scenario and an automated grep verifies the allocation calls survive (no DCE), and a sanity test confirms RSS grows during a no-op-looking scenario (black_box discipline holds).

**Plans:** 3/3 plans complete

Plans:

- [x] 02-01-PLAN.md — 6 simple scenarios (channels, contention, mem-bound, realloc-storm) + crossbeam dep + ScenarioInfo.unit additive field
- [x] 02-02-PLAN.md — 3 heavy scenarios (web with axum/tokio/reqwest, cpu-bound with rayon, fragmentation-soak with state-across-ticks)
- [x] 02-03-PLAN.md — run-all registry (Box<dyn Scenario> + panic::catch_unwind) + DCE check (just-recipe + scripts/dce_check.sh) + integration test

### Phase 3: Docker Matrix & Local Orchestration

**Goal**: User can build any of the six allocator-runtime Docker images and run any cell of the matrix locally via `just`, with NUMA pinning and cgroup memory limits baked into the recipe so that mimalloc segment pre-allocation never OOM-kills.
**Mode:** mvp
**Depends on**: Phase 1
**Requirements**: DOCK-01, DOCK-02, DOCK-03, DOCK-04, DOCK-05, DOCK-06, DOCK-07, DOCK-08, DOCK-09, ORCH-01, ORCH-02
**Success Criteria** (what must be TRUE):

  1. User runs `just build alpine jemalloc`, `just build debian-slim ptmalloc`, `just build distroless-cc mimalloc`, `just build distroless-static mimalloc`, `just build scratch mallocng`, and `just build wolfi jemalloc` and each produces a tagged image whose `docker inspect` shows the full OCI annotation set (title, description, source, version, revision, licenses, created, authors).
  2. User runs `docker run --rm --cpus=4 --memory=4g --cpuset-cpus=0-3 -v $(pwd)/results:/out alloc-bench:jemalloc-alpine run-all --output /out/jemalloc-alpine.json` and gets a complete results.json with no OOM-kill and the env block correctly reporting `docker_image: "alpine:3.20"` and `target_triple: "x86_64-unknown-linux-musl"`.
  3. User runs `just bench-all` and the recipe builds and runs the full meaningful (allocator × env) cross-product (excluding nonsensical combos like mallocng-on-debian), emitting one results.json per cell into `results/`.
  4. User runs `just bench-host` on the macOS host and gets a `results/host-system.json` recording libmalloc as the 7th-environment baseline with a clear `docker_image: null` env marker.
  5. User runs `dive --ci alloc-bench:jemalloc-alpine` and the wasted-bytes / efficiency thresholds pass for every image in the matrix.

**Plans**: 5 plans

Plans:
**Wave 1**

- [x] 03-01-PLAN.md — glibc Dockerfiles (debian-slim, distroless-cc, wolfi) + .dockerignore + .dive-ci
- [x] 03-02-PLAN.md — musl Dockerfiles (alpine, distroless-static, scratch)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 03-03-PLAN.md — Justfile recipes (build/run/bench-cell/bench-all/bench-all-smoke/bench-host/dive-check/dive-check-all/clean-images)

**Wave 3** *(blocked on Wave 2 completion)*

- [ ] 03-04-PLAN.md — Anchor smoke + full matrix + bench-host + dive-check-all

**Wave 4** *(blocked on Wave 3 completion)*

- [ ] 03-05-PLAN.md — STATE.md / ROADMAP.md / CLAUDE.md closure

### Phase 4: Aggregator & Dashboard

**Goal**: User can point the aggregator at a directory of results.json files and get a self-contained `report/index.html` Plotly dashboard plus a `REPORT.md` with comparison tables, Mermaid allocator architecture diagrams, and a recommendations section.
**Mode:** mvp
**Depends on**: Phase 2
**Requirements**: AGG-01, AGG-02, AGG-03, AGG-04, AGG-05, AGG-06, AGG-07, AGG-08, ORCH-03
**Success Criteria** (what must be TRUE):

  1. User runs `cargo run --release -p alloc-bench-aggregator -- --input "results/*.json" --output report/` (or equivalently `just aggregate`) and the binary produces `report/index.html` with all results inlined as a `<script>const RESULTS = {...}</script>` block, openable directly via `file://` with no server.
  2. User opens `report/index.html` and can multi-select scenarios, environments, and allocators in a sidebar to drive a throughput bar chart (grouped by scenario, colored by allocator, faceted by env), a latency-percentile heatmap, RSS-over-time line charts (one line per allocator×env), and a side-by-side comparison-diff bar chart.
  3. User reads `report/REPORT.md` and finds a side-by-side comparison table of all four allocators across all scenarios with per-row winner highlighted, a Docker runtime comparison table (image size, build time, run-time overhead), Mermaid.js architecture diagrams for ptmalloc / mallocng / jemalloc / mimalloc, and a "Recommendations" section mapping workload-shape (CPU-bound, web ser/de, channel-heavy, fragmentation-prone, etc.) to a recommended allocator.
  4. The aggregator flags any input run with fewer than 10,000 latency samples or warmup_duration_s < 5 as "suspect" in both the HTML and the Markdown.
  5. User reads `README.md` and finds an overall Mermaid.js system diagram of how memory allocation works on modern Linux (kernel → libc → application allocator → user code).

**Plans**: TBD
**UI hint**: yes

### Phase 5: CI, Image-Size Gate & Public Polish

**Goal**: User pushes a commit and a GitHub Actions matrix run produces uploaded results/ + report/ artifacts (≥ 3 runs per cell, median + range), with Dive enforcing image-size budgets and the public README guiding any reader to reproduce the entire benchmark from scratch.
**Mode:** mvp
**Depends on**: Phase 4
**Requirements**: ORCH-04, ORCH-05, REPR-01, REPR-03
**Success Criteria** (what must be TRUE):

  1. User pushes a commit to a feature branch and a GitHub Actions workflow runs the full meaningful (allocator × env) matrix on `ubuntu-24.04` (excluding macOS-specific cells), runs each matrix cell ≥ 3 times with different seeds, and uploads `results/` and `report/` as workflow artifacts.
  2. CI fails the build if `dive --ci` exceeds the configured wasted-bytes / efficiency thresholds for any image in the matrix; the failure message points at the offending image and layer.
  3. The aggregator output (driven by CI) reports median + min/max range per matrix cell across the ≥ 3 runs, and any cell with a CV > X% is highlighted as "high variance" in REPORT.md.
  4. A reader who has never seen the repo follows the README "Run it yourself" walkthrough — Docker prerequisites, `just bench-all`, then opening `report/index.html` — and reproduces a representative subset of results without needing any out-of-band knowledge.

**Plans:** 2/4 plans executed

Plans:

**Wave 1**

- [x] 05-01-PLAN.md — Multi-run statistics module + fixtures (REPR-03 math layer)
- [x] 05-02-PLAN.md — GHA workflow + LICENSE files + ci-bench-cell + ci-validate (ORCH-04, ORCH-05)

**Wave 2** *(blocked on Wave 1 completion)*

- [ ] 05-03-PLAN.md — Aggregator integration: multi-run REPORT.md decoration + sidecar image_size_mb backfill (REPR-03 rendering layer + Phase 4 D-10 closure)

**Wave 3** *(blocked on Wave 2 completion)*

- [ ] 05-04-PLAN.md — README walkthrough + LICENSE citation + STATE/ROADMAP/CLAUDE closure (REPR-01)

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation MVP Slice | 2/2 | Complete   | 2026-05-18 |
| 2. Scenario Fan-Out | 3/3 | Complete   | 2026-05-18 |
| 3. Docker Matrix & Local Orchestration | 3/5 | In Progress|  |
| 4. Aggregator & Dashboard | 3/3 | Complete    | 2026-05-19 |
| 5. CI, Image-Size Gate & Public Polish | 2/4 | In Progress|  |

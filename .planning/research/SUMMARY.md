# SUMMARY.md — Research Synthesis

## Stack

- **Workspace:** single Cargo workspace; one binary crate built repeatedly with different `[features]` to select global allocator at compile time. Allocator combos: glibc-ptmalloc (default), glibc-jemalloc (`alloc-jemalloc`), glibc-mimalloc (`alloc-mimalloc`), musl-mallocng (default + musl target), musl-jemalloc, musl-mimalloc, plus macOS-libmalloc as 7th env baseline.
- **Allocator crates:** `tikv-jemallocator` 0.6.1 + `tikv-jemalloc-ctl` 0.6, `mimalloc` 0.1.43 (microsoft v3.x). Default-features-off on musl targets to dodge known linking quirks.
- **Harness:** custom (not Criterion). Duration-based with 5s warm-up minimum; per-op latency captured via `hdrhistogram` 7.5; `black_box` mandatory around scenario `tick()`.
- **Metrics:** throughput, p50/p95/p99/p999, peak RSS via `getrusage(RUSAGE_SELF).ru_maxrss`, RSS growth via `/proc/self/statm`, allocator-internal stats (jemalloc-ctl, mimalloc extended), `getrusage` page faults + context switches.
- **Web bench:** `axum` 0.8 + `tokio` 1 + `serde_json` 1, in-process load gen on second runtime, `reqwest` 0.12 client, ~1.5 KB request/response payloads.
- **Channel benches:** `crossbeam-channel` 0.5 with `Box<Payload>` carrying `Vec<u8>` of distribution-drawn size.
- **Build:** `[profile.release] lto = "fat"`, `codegen-units = 1`, `opt-level = 3`, `strip = "symbols"`, `debug = false`. Use `-C target-cpu=x86-64-v3` for portable Docker; `target-cpu=native` only for host.
- **Build metadata:** `vergen` 9 (or hand-rolled `build.rs`) injects rustc version + target + git SHA + timestamp via `env!` at compile time.
- **Docker:** multi-stage with `cargo-chef` for dependency caching. 6 runtime images: `alpine:3.20`, `debian:bookworm-slim`, `gcr.io/distroless/cc-debian12`, `gcr.io/distroless/static-debian12`, `scratch`, `cgr.dev/chainguard/static`. OCI annotations on every image.
- **Cross-compile:** Docker-builder-stage approach for matrix; `cargo-zigbuild` for local dev on macOS.
- **Orchestration:** Justfile cross-product matrix (`just bench-all`); GitHub Actions matrix CI for reproducibility.
- **Aggregator:** Rust binary (`alloc-bench-aggregator`) reads `results/*.json`, emits `report/index.html` (Plotly.js standalone, results inlined) + `REPORT.md` with comparison tables and Mermaid allocator diagrams.

## Table-stakes scenarios

1. **Multi-thread allocation stress** (the user's anchor scenario): N threads × M objects, configurable size distribution.
2. **Web service** (axum + serde_json + tokio).
3. **Channel benches** (SPMC, MPSC, MPMC) — `crossbeam-channel` with heap payloads.
4. **CPU-bound algorithm** — parallel merge-sort with allocations in critical path.
5. **Memory-bound algorithm** — pointer-chasing linked list (locality stress) + large strided array (control).
6. **Lock-contention / arena saturation** — high thread count, same-size allocations.

## Differentiating scenarios (added by research)

7. **Fragmentation soak** — long mixed alloc/free with biased sizes; measures resident-vs-allocated.
8. **Realloc storm** — `Vec::push()` under pressure; tests grow-in-place fast paths.

## Watch out for

- **DCE / black_box:** scenarios must wrap returned values in `std::hint::black_box`; smoke-test with `cargo build --emit=llvm-ir`.
- **Warm-up:** mimalloc and jemalloc are lazy; minimum 5s warm-up or all comparisons skew.
- **NUMA:** pin Docker to a single NUMA node via `--cpuset-cpus`; document in env block.
- **target-cpu:** never use `=native` in Docker matrix; CI runners have heterogeneous CPUs.
- **Static binaries on scratch / distroless-static:** require `+crt-static` + musl target; HTTPS would need bundled CA certs (avoid by keeping web bench on HTTP localhost).
- **macOS-vs-Linux:** macOS Docker runs through a VM; not 1:1 comparable. Label clearly in REPORT.md.
- **Statistics:** always p50/p95/p99/p999 with ≥ 10k samples per cell; ≥ 3 runs per matrix cell with median + range.
- **cgroup memory:** `--memory ≥ 4 GiB` to avoid OOM-kill on mimalloc segment pre-allocation.

## Anti-patterns to avoid

- Single-threaded micro-benches (don't differentiate allocators)
- Mean-only reporting
- Setup work inside measurement loop
- LD_PRELOAD-style runtime allocator swap (we use compile-time features for clean comparisons)
- Criterion-style microbench harness (wrong shape for throughput/latency-distribution)

## Phase shape (informs the roadmapper)

The research suggests this natural phase decomposition:

- **Phase 1 — Foundation:** Cargo workspace, `#[global_allocator]` feature scaffolding, build.rs metadata injection, custom harness skeleton, FIRST scenario (multi-thread allocation stress) end-to-end, `results.json` schema. Smoke build + run for one allocator combo proving the loop works.
- **Phase 2 — Scenarios:** All remaining benches (web, channels ×3, CPU-bound, memory-bound, lock-contention, fragmentation, realloc-storm). Each is independent given the harness contract; can parallelize as plans.
- **Phase 3 — Docker matrix:** 6 Dockerfiles + per-environment build verification. Smoke run for every (allocator × env) combo. Justfile bench-all recipe.
- **Phase 4 — Aggregator + Dashboard:** `alloc-bench-aggregator` binary, Plotly HTML template, JSON validation, Mermaid diagrams in REPORT.md.
- **Phase 5 — CI + Polish:** GitHub Actions matrix, Dive image-size gate, README.md with system Mermaid diagram, REPORT.md with side-by-side tables, recommendations section.

5 phases, MVP-mode (each phase delivers an end-to-end usable artifact), coarse granularity per the user's config.json.

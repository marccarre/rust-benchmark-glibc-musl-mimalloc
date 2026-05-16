# rust-benchmark-glibc-musl-mimalloc

## What This Is

A comprehensive Rust benchmark suite comparing four memory allocators (glibc/ptmalloc, musl/mallocng, jemalloc, mimalloc) across six libc×allocator combinations, seven runtime environments (macOS host + six Docker images), and eight benchmark scenarios (micro-allocation stress test, web service, SPMC/MPSC/MPMC channels, CPU-bound, memory-bound, and lock-contention). Results are aggregated into an interactive HTML dashboard with Plotly charts and a Markdown report with Mermaid.js allocator-architecture diagrams.

## Core Value

Every result is reproducible, environment-labelled, and visually comparable — so the reader can confidently recommend the right allocator for a given workload.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] Cargo workspace with one bench-runner crate and allocator selection via Cargo features at build time
- [ ] Six libc×allocator build targets: glibc-ptmalloc, glibc-jemalloc, glibc-mimalloc, musl-mallocng, musl-jemalloc, musl-mimalloc
- [ ] Micro-allocation benchmark: N threads each allocating M objects of random sizes (N, M, size range configurable via CLI)
- [ ] Web-service benchmark: axum + serde_json + tokio, saturating load for configurable duration, measuring req/s, p50/p95/p99 latency
- [ ] SPMC, MPSC, MPMC channel benchmarks exercising crossbeam or std channels with heap payloads
- [ ] CPU-bound benchmark (e.g. parallel merge-sort or matrix multiply) measuring wall-clock and throughput
- [ ] Memory-bound benchmark (e.g. pointer-chasing linked list or large array traversal) measuring peak RSS and bandwidth
- [ ] Lock-contention / arena benchmark to stress allocator scalability under high thread counts
- [ ] Custom harness: warm-up run, configurable duration, p50/p95/p99/p999 via hdrhistogram, peak RSS via /proc/self/statm (Linux) or getrusage, allocator-internal stats (jemalloc-ctl, mimalloc stats API)
- [ ] Compiler version + build metadata injected at compile time via build.rs (rustc version, target triple, allocator name, build timestamp)
- [ ] Release profile with LTO=fat, codegen-units=1, opt-level=3, RUSTFLAGS="-C target-cpu=native" (Linux builds)
- [ ] Justfile orchestrating the full 6×6 = 36-cell benchmark matrix locally; `just bench-all` runs everything
- [ ] GitHub Actions matrix CI producing results.json artifacts + REPORT.md on each push
- [ ] Seven Docker environments: macOS host as-is (libmalloc baseline), Alpine, Debian-slim, Distroless, Scratch (musl static), Wolfi, Chainguard static
- [ ] Docker multi-stage builds using Rust builder stage → minimal runtime stage; OCI labels (org.opencontainers.image.*) on all images
- [ ] Dive integration in CI to verify image layer efficiency (max wasted bytes threshold)
- [ ] Results emitted as results.json per run (env + allocator + scenario × metrics)
- [ ] Interactive HTML dashboard (Plotly) for side-by-side comparison, filtering by scenario/env/allocator, p-percentile selection
- [ ] REPORT.md with side-by-side allocator comparison table, Docker runtime comparison table, Mermaid.js allocator architecture diagrams
- [ ] README.md with Mermaid.js overall memory-allocation system diagram

### Out of Scope

- Windows or macOS cross-compilation for the allocator matrix — all allocator benchmarks run on Linux (Docker or Linux host)
- Criterion HTML reports — custom harness is the single source of truth
- Runtime LD_PRELOAD allocator swapping — allocator is baked in at compile time for clean results
- snmalloc / tcmalloc — deferred to v2 if jemalloc/mimalloc story is clean
- Marimo notebook output — Plotly HTML covers interactive exploration; Marimo deferred to v2
- GUI or web server for live dashboard — static HTML only
- Android / embedded targets — out of scope for this comparison

## Context

**Prior research (session, May 2026):**
- `tikv-jemallocator` 0.6.1 (Oct 2025) is the current standard jemalloc Rust binding. Provides `tikv_jemalloc_ctl` for stats. Dual Apache/MIT.
- `mimalloc` crate (microsoft/mimalloc v3.3.2, April 2026) is the current Rust wrapper. No official Rust-from-Microsoft binding; community `mimalloc` crate on crates.io is canonical.
- musl target: `x86_64-unknown-linux-musl` — statically links musl libc + mallocng (musl ≥ 1.2.1). Alpine Docker image uses musl dynamically.
- OCI labels canonical set: `org.opencontainers.image.{created,authors,url,documentation,source,version,revision,vendor,licenses,title,description,base.digest,base.name}`.
- User is on macOS (darwin). All allocator-comparable runs happen in Docker or on a Linux host. macOS host run measures libmalloc as a 7th environment for "dev-box baseline" only — not compared 1:1 with Linux combos.

**Design decisions locked in (user Q&A):**
- Structure: single CLI crate / Cargo workspace, multi-binary per allocator (Cargo features select allocator)
- Allocator matrix: full 6-combo (glibc-ptmalloc, glibc-jemalloc, glibc-mimalloc, musl-mallocng, musl-jemalloc, musl-mimalloc) + macOS libmalloc host
- Web stack: axum + serde_json + tokio
- Reporting: JSON output + Markdown report + interactive Plotly HTML dashboard
- Benchmark harness: custom (not criterion) — duration-based, warm-up, HDR-histogram latency, peak RSS, allocator stats APIs
- Metrics: peak RSS + memory growth, p50/p95/p99/p999 latency, allocator-internal stats, page faults / rusage
- Orchestration: Justfile + bench script + GitHub Actions matrix CI

## Constraints

- **Platform**: All allocator-vs-allocator benchmarks must run on Linux (Docker or Linux host) — macOS libmalloc run is a separate baseline only
- **Build**: Allocator selection is compile-time (Cargo feature flag) — no LD_PRELOAD; ensures fair, reproducible comparisons
- **Reproducibility**: Justfile and Docker builds must be fully self-contained — no manual steps beyond `just bench-all`
- **Image size**: Docker images should be as small as practical; Dive CI gate enforces no large unexpected layers
- **Performance build flags**: LTO=fat, codegen-units=1, opt-level=3 mandatory for benchmark binaries; debug symbols stripped from Docker images
- **Compiler version in output**: All bench binaries must print rustc version, target triple, allocator name at startup (injected at compile time)

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Cargo features select allocator at build time | Clean, reproducible, no LD_PRELOAD fragility; one feature per allocator combo | — Pending |
| Custom harness over Criterion | Criterion is microbench-shaped; throughput/latency-distribution benches need duration-based runs with warm-up and HDR histograms | — Pending |
| axum + serde_json + tokio for web bench | Most representative modern Rust async stack in 2026; maximises allocator-stress via Tokio runtime + serde heap work | — Pending |
| macOS host as 7th env (libmalloc) | User requested it as dev-box baseline; documented as not directly comparable to Linux 6-combo matrix | — Pending |
| Plotly HTML dashboard (not Marimo) | Marimo is Python-first; pure Plotly HTML is self-contained and zero-dependency for end consumers | — Pending |
| OCI annotations on all Docker images | Best practice per opencontainers/image-spec; community-standard as of 2025-2026 | — Pending |
| Dive CI gate for image efficiency | Prevents accidental large layers; enforces image-size discipline in a public benchmark repo | — Pending |

---
*Last updated: 2026-05-17 after initialization*

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

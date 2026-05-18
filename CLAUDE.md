<!-- GSD:project-start source:PROJECT.md -->
## Project

**rust-benchmark-glibc-musl-mimalloc**

A comprehensive Rust benchmark suite comparing four memory allocators (glibc/ptmalloc, musl/mallocng, jemalloc, mimalloc) across six libc×allocator combinations, seven runtime environments (macOS host + six Docker images), and eight benchmark scenarios (micro-allocation stress test, web service, SPMC/MPSC/MPMC channels, CPU-bound, memory-bound, and lock-contention). Results are aggregated into an interactive HTML dashboard with Plotly charts and a Markdown report with Mermaid.js allocator-architecture diagrams.

**Core Value:** Every result is reproducible, environment-labelled, and visually comparable — so the reader can confidently recommend the right allocator for a given workload.

### Constraints

- **Platform**: All allocator-vs-allocator benchmarks must run on Linux (Docker or Linux host) — macOS libmalloc run is a separate baseline only
- **Build**: Allocator selection is compile-time (Cargo feature flag) — no LD_PRELOAD; ensures fair, reproducible comparisons
- **Reproducibility**: Justfile and Docker builds must be fully self-contained — no manual steps beyond `just bench-all`
- **Image size**: Docker images should be as small as practical; Dive CI gate enforces no large unexpected layers
- **Performance build flags**: LTO=fat, codegen-units=1, opt-level=3 mandatory for benchmark binaries; debug symbols stripped from Docker images. `panic` is left at the toolchain default (`unwind`) so `alloc-bench-cli run-all`'s `std::panic::catch_unwind` per-scenario isolation contract holds in release builds — see Phase-2 review CR-01 for the trade-off (negligible binary-size overhead vs. losing the per-scenario `status:"failed"` Run record path entirely if `panic = "abort"`).
- **Compiler version in output**: All bench binaries must print rustc version, target triple, allocator name at startup (injected at compile time)
<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->
## Technology Stack

## TL;DR
## 1. Global allocator selection (Cargo features)
# Cargo.toml
# … other deps
#[cfg(feature = "alloc-jemalloc")]
#[global_allocator]
#[cfg(feature = "alloc-mimalloc")]
#[global_allocator]
## 2. tikv-jemallocator + tikv-jemalloc-ctl
- Latest: 0.6.1 (Oct 2025). Active fork after upstream `jemallocator` was abandoned. Dual Apache/MIT.
- Targets: x86_64-unknown-linux-gnu Tier 1; x86_64-unknown-linux-musl works (forced fallback to dlsym in some configs but functions). aarch64-linux-* works.
- **Stats access** at runtime via `tikv_jemalloc_ctl`:
#[cfg(feature = "alloc-jemalloc")]
## 3. mimalloc crate
- mimalloc upstream v3.3.2 (April 2026). Rust crate `mimalloc` on crates.io is the canonical wrapper (no official Microsoft binding).
- Build features: `secure` (slower, harder), `override` (overrides C/C++ malloc — not needed for Rust-only bench), `extended` (exposes additional stats/option APIs).
#[cfg(feature = "alloc-mimalloc")]
## 4. hdrhistogram
- Crate: `hdrhistogram` 7.x (latest stable since 2024). Stable API.
- Use `Histogram::<u64>::new_with_bounds(1, 60_000_000_000, 3)` for nanosecond latencies up to 60s with 3 sig figs.
## 5. Peak RSS on Linux (in Docker)
## 6. Compile-time metadata via build.rs
## 7. axum + tokio (web bench)
- axum 0.8.x (current stable as of 2025-2026), tokio 1.x (latest), tower 0.5, hyper 1.x.
- `serde_json` 1.x for ser/de.
- Test payload: nested JSON struct ~1–2KB request, ~1–2KB response — representative of typical microservice traffic; generates plenty of small/medium allocations per request.
## 8. crossbeam-channel for SPMC/MPSC/MPMC
- `crossbeam-channel` 0.5.x — bounded/unbounded MPMC channels. Single-producer / single-consumer semantics emerge from how senders/receivers are cloned.
- For SPMC: one Sender, multiple cloned Receivers (use `crossbeam::channel::bounded` and let consumers race for messages).
- For MPSC: std `mpsc` is fine but crossbeam is significantly faster. Use `crossbeam-channel::unbounded` with Senders cloned across threads.
- For MPMC: same crossbeam channel with both sides cloned.
## 9. Docker multi-stage with cargo-chef
# Stage 1: chef base
# Stage 2: prepare recipe
# Stage 3: cook deps (cached layer)
# Stage 4: runtime — different base per env
## 10. Cross-compilation to musl on macOS
## 11. Justfile cross-product matrix
## 12. GitHub Actions matrix CI
## 13. Plotly HTML dashboard (zero-server static)
## Summary of crate versions (May 2026 best-known)
| Crate | Version | Purpose |
|-------|---------|---------|
| tikv-jemallocator | 0.6.1 | jemalloc global allocator |
| tikv-jemalloc-ctl | 0.6 | jemalloc stats |
| mimalloc | 0.1.43 | mimalloc global allocator |
| hdrhistogram | 7.5 | Latency percentiles |
| axum | 0.8 | HTTP server |
| tokio | 1 | Async runtime |
| serde / serde_json | 1 | JSON ser/de |
| crossbeam-channel | 0.5 | SPMC/MPSC/MPMC channels |
| reqwest | 0.12 | HTTP client (load gen) |
| clap | 4 | CLI parsing |
| chrono | 0.4 | Timestamps |
| libc | 0.2 | getrusage |
| vergen | 9 | Compile-time build metadata (alt to hand-rolled build.rs) |
| tinytemplate | 1 | HTML report templating |
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

Conventions not yet established. Will populate as patterns emerge during development.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->
## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, `.github/skills/`, or `.codex/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->



<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->

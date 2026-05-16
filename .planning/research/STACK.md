# STACK.md — Rust Allocator Benchmark Stack (May 2026)

## TL;DR

Single Cargo workspace, allocator selected by Cargo features at compile time, custom duration-based harness with hdrhistogram, axum 0.8 + tokio 1.x for the web bench, crossbeam-channel for SPMC/MPSC/MPMC, cargo-chef for Docker layer caching, cargo-zigbuild for cross-compiling to musl on macOS, Justfile + GitHub Actions matrix for orchestration, vanilla Plotly.js standalone HTML for the dashboard.

## 1. Global allocator selection (Cargo features)

**Pattern:** one Cargo feature per allocator. The default feature targets the system allocator (glibc-ptmalloc on glibc Linux, mallocng on musl, libmalloc on macOS).

```toml
# Cargo.toml
[package]
name = "alloc-bench"
edition = "2024"

[features]
default = []
alloc-jemalloc = ["dep:tikv-jemallocator", "dep:tikv-jemalloc-ctl"]
alloc-mimalloc = ["dep:mimalloc"]

[dependencies]
tikv-jemallocator = { version = "0.6", optional = true }
tikv-jemalloc-ctl = { version = "0.6", optional = true }
mimalloc           = { version = "0.1.43", optional = true, default-features = false }
# … other deps
```

```rust
// src/allocator.rs
#[cfg(feature = "alloc-jemalloc")]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(feature = "alloc-mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

pub const ALLOCATOR_NAME: &str = {
    #[cfg(feature = "alloc-jemalloc")] { "jemalloc" }
    #[cfg(feature = "alloc-mimalloc")] { "mimalloc" }
    #[cfg(not(any(feature = "alloc-jemalloc", feature = "alloc-mimalloc")))] {
        if cfg!(target_env = "musl") { "mallocng" } else if cfg!(target_os = "macos") { "libmalloc" } else { "ptmalloc" }
    }
};
```

Build invocation: `cargo build --release --no-default-features --features alloc-jemalloc`.

**Confidence:** High — this is the canonical pattern; used by Tokio runtime examples, ripgrep, fd, etc.

## 2. tikv-jemallocator + tikv-jemalloc-ctl

- Latest: 0.6.1 (Oct 2025). Active fork after upstream `jemallocator` was abandoned. Dual Apache/MIT.
- Targets: x86_64-unknown-linux-gnu Tier 1; x86_64-unknown-linux-musl works (forced fallback to dlsym in some configs but functions). aarch64-linux-* works.
- **Stats access** at runtime via `tikv_jemalloc_ctl`:

```rust
#[cfg(feature = "alloc-jemalloc")]
pub fn jemalloc_stats() -> AllocStats {
    use tikv_jemalloc_ctl::{epoch, stats};
    epoch::advance().unwrap(); // refresh stats counters
    AllocStats {
        allocated: stats::allocated::read().unwrap(),
        resident:  stats::resident::read().unwrap(),
        retained:  stats::retained::read().unwrap(),
        active:    stats::active::read().unwrap(),
    }
}
```

**Confidence:** High — verified via crates.io / GitHub during prior session research.

## 3. mimalloc crate

- mimalloc upstream v3.3.2 (April 2026). Rust crate `mimalloc` on crates.io is the canonical wrapper (no official Microsoft binding).
- Build features: `secure` (slower, harder), `override` (overrides C/C++ malloc — not needed for Rust-only bench), `extended` (exposes additional stats/option APIs).

```toml
mimalloc = { version = "0.1.43", default-features = false, features = ["extended"] }
```

```rust
#[cfg(feature = "alloc-mimalloc")]
pub fn mimalloc_stats() -> AllocStats {
    // mi_stats_print writes to stderr; for structured stats use mi_stats_get via extended feature.
    // Simpler: read via mi_options or call mi_stats_print into a buffer.
    // For minimum-deps approach: capture only RSS via /proc/self/statm and let the
    // mimalloc-internal counters be sampled via mi_stats_print at end-of-run.
    AllocStats { /* stub or extended API */ }
}
```

**Note:** mimalloc's stats API in the `mimalloc` Rust crate is less ergonomic than jemalloc-ctl. Practical compromise: emit `mi_stats_print` to stderr at end-of-run; capture jemalloc/mimalloc structured stats only when the API is straightforward; rely on `/proc/self/statm` for the cross-allocator peak-RSS metric.

**Confidence:** Medium — version pinning verified; the extended-feature stats API is less battle-tested than tikv-jemalloc-ctl.

## 4. hdrhistogram

- Crate: `hdrhistogram` 7.x (latest stable since 2024). Stable API.
- Use `Histogram::<u64>::new_with_bounds(1, 60_000_000_000, 3)` for nanosecond latencies up to 60s with 3 sig figs.

```rust
let mut hist = hdrhistogram::Histogram::<u64>::new_with_bounds(1, 60_000_000_000, 3).unwrap();
let t0 = Instant::now();
do_work();
hist.record(t0.elapsed().as_nanos() as u64).unwrap();

let p50 = hist.value_at_quantile(0.50);
let p99 = hist.value_at_quantile(0.99);
let p999 = hist.value_at_quantile(0.999);
```

**Confidence:** High — well-known crate.

## 5. Peak RSS on Linux (in Docker)

Three options:

1. **`/proc/self/statm`** — read field 2 (resident pages) × `sysconf(_SC_PAGESIZE)`. Cheap, no deps, works in all containers that mount /proc (Alpine, Debian-slim, Distroless, Wolfi, Chainguard). Scratch + static binary: /proc must be present (Docker mounts it by default).
2. **`getrusage(RUSAGE_SELF)`** — `ru_maxrss` field. On Linux returns kilobytes. Available via `libc` crate. Single syscall; gives high-water mark across the process lifetime, which is what we want.
3. **`memory-stats` crate** — convenience wrapper but adds a dep. Not necessary.

**Recommendation:** use both — `/proc/self/statm` for sampled growth-curve over time, `getrusage(RUSAGE_SELF).ru_maxrss` for the peak-RSS final value.

```rust
use libc::{getrusage, rusage, RUSAGE_SELF};
unsafe {
    let mut ru: rusage = std::mem::zeroed();
    getrusage(RUSAGE_SELF, &mut ru);
    let peak_rss_kb = ru.ru_maxrss as u64; // kilobytes on Linux
}
```

**Confidence:** High.

## 6. Compile-time metadata via build.rs

```rust
// build.rs
use std::process::Command;
fn main() {
    let rustc_version = String::from_utf8(
        Command::new(std::env::var("RUSTC").unwrap()).arg("--version").output().unwrap().stdout
    ).unwrap();
    let target = std::env::var("TARGET").unwrap();
    let host   = std::env::var("HOST").unwrap();
    let profile = std::env::var("PROFILE").unwrap();
    let timestamp = chrono::Utc::now().to_rfc3339(); // or env::var("SOURCE_DATE_EPOCH") for reproducibility

    println!("cargo:rustc-env=BUILD_RUSTC_VERSION={}", rustc_version.trim());
    println!("cargo:rustc-env=BUILD_TARGET={}", target);
    println!("cargo:rustc-env=BUILD_HOST={}", host);
    println!("cargo:rustc-env=BUILD_PROFILE={}", profile);
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", timestamp);
    println!("cargo:rerun-if-changed=build.rs");
}
```

```rust
// src/build_info.rs
pub const RUSTC_VERSION: &str = env!("BUILD_RUSTC_VERSION");
pub const TARGET: &str = env!("BUILD_TARGET");
pub const HOST: &str = env!("BUILD_HOST");
pub const PROFILE: &str = env!("BUILD_PROFILE");
pub const TIMESTAMP: &str = env!("BUILD_TIMESTAMP");
```

Alternative: `vergen` crate provides this plus git SHA; adds a dep but produces consistent metadata. Recommended for this project (git SHA is useful for the Plotly dashboard).

**Confidence:** High.

## 7. axum + tokio (web bench)

- axum 0.8.x (current stable as of 2025-2026), tokio 1.x (latest), tower 0.5, hyper 1.x.
- `serde_json` 1.x for ser/de.
- Test payload: nested JSON struct ~1–2KB request, ~1–2KB response — representative of typical microservice traffic; generates plenty of small/medium allocations per request.

```toml
axum       = "0.8"
tokio      = { version = "1", features = ["full"] }
serde      = { version = "1", features = ["derive"] }
serde_json = "1"
tower      = "0.5"
```

Server runs in-process on one tokio runtime; load generator runs on another runtime (or threadpool) and hits 127.0.0.1:PORT via `reqwest` or a hand-rolled `hyper` client. Measure wall-clock latency per request and aggregate via hdrhistogram.

**Confidence:** High — this is the standard 2025-2026 stack.

## 8. crossbeam-channel for SPMC/MPSC/MPMC

- `crossbeam-channel` 0.5.x — bounded/unbounded MPMC channels. Single-producer / single-consumer semantics emerge from how senders/receivers are cloned.
- For SPMC: one Sender, multiple cloned Receivers (use `crossbeam::channel::bounded` and let consumers race for messages).
- For MPSC: std `mpsc` is fine but crossbeam is significantly faster. Use `crossbeam-channel::unbounded` with Senders cloned across threads.
- For MPMC: same crossbeam channel with both sides cloned.

```toml
crossbeam-channel = "0.5"
```

Payload: `Box<HeavyMessage>` where `HeavyMessage` contains `Vec<u8>` of varying sizes — forces heap allocation per message.

**Confidence:** High.

## 9. Docker multi-stage with cargo-chef

Three-stage pattern:

```dockerfile
# Stage 1: chef base
FROM rust:1.83-slim AS chef
RUN cargo install cargo-chef
WORKDIR /app

# Stage 2: prepare recipe
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: cook deps (cached layer)
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json --features alloc-jemalloc
COPY . .
RUN cargo build --release --no-default-features --features alloc-jemalloc

# Stage 4: runtime — different base per env
FROM debian:bookworm-slim AS runtime
COPY --from=builder /app/target/release/alloc-bench /usr/local/bin/alloc-bench
ENTRYPOINT ["/usr/local/bin/alloc-bench"]
```

Per-environment Dockerfiles share stages 1-3 and differ only in stage 4 base (alpine, debian-slim, gcr.io/distroless/cc-debian12, scratch, cgr.dev/chainguard/static, cgr.dev/chainguard/wolfi-base).

**For scratch:** must build a fully-static binary (musl target + `-C target-feature=+crt-static`).

**Confidence:** High.

## 10. Cross-compilation to musl on macOS

Three options ranked:

1. **`cargo-zigbuild`** (recommended) — uses Zig as the C linker; works on macOS host without privileged Docker. Single command: `cargo zigbuild --release --target x86_64-unknown-linux-musl --features alloc-jemalloc`. Handles glibc and musl targets.
2. **`cross`** — runs a Docker container per target. Reliable but adds Docker-in-Docker complications when CI is already in Docker.
3. **Build inside a Docker `rust:slim` builder image** — most reliable; fits the per-env Dockerfile pattern. **Recommend this for the Docker matrix; cargo-zigbuild for ad-hoc local dev.**

For this project: use Docker-builder-stage for all matrix runs; document `cargo-zigbuild` for local debugging.

**Confidence:** High.

## 11. Justfile cross-product matrix

```just
allocators := "ptmalloc jemalloc mimalloc mallocng"
envs       := "host alpine debian-slim distroless scratch wolfi"

bench-all:
    @for a in {{allocators}}; do for e in {{envs}}; do just bench $a $e; done; done

bench allocator env:
    @echo "::: {{allocator}} on {{env}}"
    docker build -f docker/{{env}}.Dockerfile --build-arg ALLOC={{allocator}} -t bench:{{allocator}}-{{env}} .
    docker run --rm -v $(pwd)/results:/out bench:{{allocator}}-{{env}} \
        run-all --output /out/{{allocator}}-{{env}}.json

aggregate:
    python3 scripts/aggregate.py results/*.json --out report/
```

**Confidence:** High.

## 12. GitHub Actions matrix CI

```yaml
strategy:
  fail-fast: false
  matrix:
    allocator: [ptmalloc, jemalloc, mimalloc, mallocng]
    env: [alpine, debian-slim, distroless, scratch, wolfi]
    exclude:
      # mallocng is the musl libc allocator; only meaningful on alpine/scratch
      - { allocator: mallocng, env: debian-slim }
      - { allocator: mallocng, env: distroless }
      - { allocator: mallocng, env: wolfi }
runs-on: ubuntu-24.04
```

Use `actions/cache@v4` keyed on Cargo.lock + Dockerfile hash for build cache.

**Confidence:** High.

## 13. Plotly HTML dashboard (zero-server static)

Two options:

1. **Pure Plotly.js standalone HTML** — single `report/index.html` that loads `results.json` via `fetch()` and renders charts client-side. Zero Python dependency. Requires CORS-friendly serving (use `python3 -m http.server` for local viewing, or ship as a self-contained HTML with results inlined).
2. **Python `plotly` library** — generates static HTML at aggregator time. Embedded data, no `fetch` needed. Adds a Python dependency.

**Recommend option 1** with results inlined as a `<script>const RESULTS = {...}</script>` block to avoid CORS for `file://` viewing. Aggregator written in Python (~50 lines using `pandas` + `plotly.express`) or Rust (using `tinytemplate` + Plotly.js CDN). Python is faster to write; Rust keeps the project zero-Python.

**Recommendation: aggregator in Rust** to keep the project allocator-language-pure. Use `tinytemplate` to inject results.json into a Plotly.js HTML template.

**Confidence:** Medium — final choice depends on whether the team prefers a small Python dep for ergonomics.

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

**Confidence:** High for major crates; pin exact versions during plan-phase.

# PITFALLS.md — Allocator Benchmarking Mistakes to Avoid

## 1. Measurement pitfalls

### 1.1 Dead-code elimination (DCE)

**Symptom:** allocator throughput numbers that are suspiciously close across allocators (e.g., all within 5%) — because the compiler optimized the allocations away entirely.

**Why it happens:** `Box::new(x)` whose result is dropped without being read can be DCE'd in release builds. LTO + codegen-units=1 amplifies this — exactly the build flags this project mandates.

**Prevention:**
- Wrap every allocated value in `std::hint::black_box(...)` before it leaves scope.
- Inside scenario `tick()` methods: return the allocated value, let harness `black_box` it.
- For threaded scenarios: pass allocations through a channel or atomic to a "sink" thread; channel send is a barrier.
- Periodically `cargo build --release --emit=llvm-ir` and grep for the allocation calls during phase-1 verification.

**Phase to address:** Phase 2 (scenario authoring). Add a smoke test that checks RSS grows during a no-op-looking scenario.

### 1.2 black_box insufficiency on read-only paths

**Symptom:** `black_box(value)` is sometimes not enough — LLVM may keep the value live in a register without forcing the memory allocation.

**Prevention:** for write-heavy scenarios, write into the allocated buffer (`buf[buf.len()/2] = noise()`) before `black_box(buf)`. Ensures the allocation must materialize in memory, not just exist conceptually.

**Phase to address:** Phase 2.

### 1.3 NUMA effects on multi-thread benches

**Symptom:** higher thread counts show counter-intuitive scaling where one allocator collapses past N=16 but recovers at N=32 — usually NUMA topology effects, not allocator behavior.

**Prevention:**
- Pin Docker container to a single NUMA node: `--cpuset-cpus=0-N` matching one socket.
- Document NUMA pinning in results.json env block.
- Single-NUMA-node measurement is the apples-to-apples comparison; cross-NUMA is a separate experiment.

**Phase to address:** Phase 3 (Docker matrix) — bake `--cpuset-cpus` into Justfile bench recipe.

### 1.4 CPU frequency scaling

**Symptom:** first run faster than subsequent runs (turbo boost on cold CPU) or slower (thermal throttling).

**Prevention:**
- Measurement duration ≥ 60s — long enough to reach thermal steady state.
- Run each cell ≥ 3 times, take median; report variance.
- Optionally: pin CPU governor to `performance` on the runner (CI: `cpupower frequency-set -g performance`). Not always possible in shared CI runners.
- Document CPU governor and runner type in results.json env block.

**Phase to address:** Phase 5 (CI wiring).

### 1.5 Insufficient warm-up

**Symptom:** mimalloc / jemalloc look 1.5–2× worse than steady-state in first second; ptmalloc looks "more consistent" because it has nothing to warm up.

**Prevention:**
- Mandatory 5s warm-up minimum across all scenarios. Configurable via CLI but default is 5s.
- Alloc-bench-core panics if `--warmup` is set below 1s.
- Aggregator flags any run with `warmup_duration_s < 5` as suspect.

**Phase to address:** Phase 2.

## 2. Cross-compile / musl pitfalls

### 2.1 jemalloc on musl

**Known issue:** historical issues with jemalloc + musl static linking around `MADV_DONTDUMP` and dlsym fallback. As of tikv-jemallocator 0.6.1, builds against musl work but require `--features unprefixed_malloc_on_supported_platforms` to be **off** (which is the default in 0.6).

**Prevention:**
- Test musl + jemalloc combo first in Phase 3.
- If linking fails: pin `tikv-jemalloc-sys` features explicitly: `tikv-jemalloc-sys = { version = "0.6", default-features = false, features = ["background_threads_runtime_support"] }`.

**Phase to address:** Phase 3 (Docker matrix), Phase 1 verification (musl smoke build).

### 2.2 mimalloc on musl + scratch

**Known issue:** mimalloc's segment allocator uses `mmap` flags that historically required glibc-specific feature detection. Recent versions (3.x) handle musl correctly, but the `mimalloc` crate must be built with `default-features = false` on musl to avoid pulling in optional features that break.

**Prevention:**
```toml
mimalloc = { version = "0.1.43", default-features = false }
```
Verify with a smoke build in Phase 3 before shipping the matrix.

### 2.3 Static linking for scratch image

**Issue:** scratch contains nothing — no `/etc/passwd`, no `/proc` (until Docker mounts it), no resolver, no time-zone data, no CA certs.

**Prevention:**
- Build with `RUSTFLAGS="-C target-feature=+crt-static" --target x86_64-unknown-linux-musl`.
- For axum web bench in scratch: HTTPS would need bundled CA certs — but our bench is HTTP-only on localhost, so this is moot. **Constrain web bench to HTTP for now; document the limitation.**
- Time zone: bench uses UTC only; no TZ data needed.
- /proc: Docker mounts /proc by default; verify with `docker run --rm scratch ls /proc` (will fail before our binary mounts it; binary itself works because Docker mounts /proc when starting container).

**Phase to address:** Phase 3 — write a smoke test that runs the scratch binary end-to-end before declaring it usable.

### 2.4 Distroless missing libc

**Issue:** `gcr.io/distroless/static` has no glibc; `gcr.io/distroless/cc` has glibc + libc++. Choose carefully:
- mallocng (musl) → distroless/static or scratch (must be static binary)
- ptmalloc (glibc) → distroless/cc
- jemalloc/mimalloc on glibc → distroless/cc
- jemalloc/mimalloc on musl → distroless/static (verify allocator's static linking works)

**Phase to address:** Phase 3.

### 2.5 #[global_allocator] init order

**Issue:** Rust's `#[global_allocator]` static is the very first thing initialized. If something in a build dependency (proc macro, build.rs) accidentally allocates before the global_allocator static is registered, you can hit ABI mismatches.

**Prevention:**
- Don't put `#[global_allocator]` in a library crate (Cargo enforces this) — keep it in the binary crate.
- Avoid `lazy_static!` / `OnceCell` in the global_allocator module.
- This is rarely an issue in practice — flagged here for awareness.

**Phase to address:** Phase 1 verification.

## 3. Docker / environment pitfalls

### 3.1 cgroup memory limits affecting allocator

**Issue:** mimalloc pre-allocates 64 MiB segments (configurable). If `--memory=64m` is set, OOM-kill on first allocation. jemalloc's `retained` memory can also trigger cgroup OOM if `MADV_DONTNEED` isn't honored.

**Prevention:**
- Use `--memory` ≥ 4 GiB for all matrix runs. Document in Justfile.
- Sample cgroup memory.current alongside RSS to detect divergence.

**Phase to address:** Phase 3 (Docker run config).

### 3.2 Docker on macOS overhead

**Issue:** Docker Desktop on macOS runs Linux in a VM; allocator measurements are not directly comparable to Linux-host runs because the VM adds memory virtualization overhead.

**Prevention:**
- Document explicitly: "macOS Docker results are not 1:1 comparable to Linux host."
- Recommend a Linux runner (CI / dedicated machine) for the canonical results.
- Include results from both — show the gap, label it.

**Phase to address:** Phase 5 (REPORT.md).

### 3.3 target-cpu=native in Docker

**Issue:** if you build with `-C target-cpu=native` on host A and run on host B (different CPU), you get illegal-instruction crashes. CI uses different CPUs across runs.

**Prevention:**
- Use **`-C target-cpu=x86-64-v3`** for all Docker builds (assumes AVX2 + BMI2, available since 2013-ish; covers Skylake and newer Intel, Zen and newer AMD).
- Use `-C target-cpu=native` only for `bench-host` recipe.

**Phase to address:** Phase 1 (build flags), Phase 3 (Dockerfile).

## 4. Reporting pitfalls

### 4.1 Mean-only statistics

**Issue:** mean throughput hides tail-latency divergence which is exactly where allocators differ.

**Prevention:** always report p50/p95/p99/p999 alongside mean. Aggregator highlights p99 difference, not mean difference.

**Phase to address:** Phase 4 (aggregator) + Phase 5 (REPORT.md).

### 4.2 Insufficient samples for high percentiles

**Issue:** p999 needs ≥ 10,000 samples to be statistically meaningful (rule of thumb: 10× the inverse of the percentile).

**Prevention:**
- Minimum 60s measurement at any reasonable throughput gives ≥ 100k samples for most scenarios.
- Aggregator flags any percentile reported from < 10,000 samples.
- For the web bench specifically: throughput might be ~10k req/s × 60s = 600k samples — fine.

**Phase to address:** Phase 4.

### 4.3 Single-run reporting

**Issue:** one run per cell is noise, not signal.

**Prevention:**
- Run each cell 3× with different seeds; aggregator reports median + min/max range.
- Document run count in results.json.

**Phase to address:** Phase 5 (CI matrix).

### 4.4 Confounders across Docker images

**Issue:** kernel version, Docker runtime version, host CPU, container memory limit, and cgroup driver all vary across CI runs.

**Prevention:**
- Capture all of these in env block of results.json.
- Aggregator only compares results from the same `(host_cpu, kernel, docker_version)` tuple — flags cross-host comparisons as "advisory."

**Phase to address:** Phase 4 + Phase 5.

## 5. Build pitfalls

### 5.1 LTO + #[global_allocator]

**Issue:** historically, fat LTO with custom global_allocator caused linker errors with some allocators. As of Rust 1.78+, both jemalloc and mimalloc work fine with `lto = "fat"`.

**Prevention:** test the LTO build in Phase 1; flag if an allocator fails to link.

### 5.2 Debug symbols in release

**Issue:** `[profile.release] debug = true` keeps debug symbols, which (a) bloats binaries and (b) can subtly affect inlining decisions.

**Prevention:**
- `[profile.release] debug = false, strip = "symbols"` for benchmark binaries.
- Optional separate profile `[profile.bench-debug]` with debug symbols, used only when troubleshooting.

**Phase to address:** Phase 1.

### 5.3 Codegen-units > 1

**Issue:** `codegen-units = 16` (default) prevents some inlining; for benchmark precision, set to 1.

**Prevention:** mandatory `[profile.release] codegen-units = 1`.

**Phase to address:** Phase 1.

### 5.4 Compiler version drift across matrix

**Issue:** if Docker images use different rustc versions, results aren't comparable.

**Prevention:**
- Pin rustc version in all Dockerfiles via `ARG RUST_VERSION=1.83`.
- Capture rustc version in results.json `build.rustc_version`; aggregator compares.

**Phase to address:** Phase 3.

## 6. Recommended Phase Mapping

| Pitfall | Phase to address |
|---------|------------------|
| DCE / black_box | Phase 1 (harness), Phase 2 (per-scenario verify) |
| black_box insufficiency on writes | Phase 2 |
| NUMA pinning | Phase 3 (Docker run config) |
| CPU frequency steady state | Phase 5 (CI) |
| Warm-up duration | Phase 1 (harness defaults) |
| jemalloc/mimalloc on musl | Phase 3 (smoke build) |
| Scratch missing files | Phase 3 (smoke run) |
| #[global_allocator] init | Phase 1 (verify) |
| cgroup OOM | Phase 3 (Docker run config) |
| macOS-vs-Linux comparability | Phase 5 (REPORT.md disclaimer) |
| target-cpu portability | Phase 1 (build flags), Phase 3 (Dockerfile) |
| Statistics: percentiles + samples | Phase 4 (aggregator) |
| Multiple runs per cell | Phase 5 (CI matrix) |
| Confounder capture | Phase 1 (build_info), Phase 3 (env capture), Phase 4 (aggregator) |
| LTO + global_allocator | Phase 1 (smoke build) |
| Debug symbols / codegen-units | Phase 1 (Cargo.toml) |
| Rustc version pinning | Phase 3 (Dockerfile ARG) |

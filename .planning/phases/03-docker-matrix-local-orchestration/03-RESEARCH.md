# Phase 3: Docker Matrix & Local Orchestration - Research

**Researched:** 2026-05-19
**Domain:** Reproducible Linux container matrix for Rust allocator benchmarking + local Justfile orchestration
**Confidence:** HIGH (Docker / cargo-chef / dive / OCI spec / image manifests verified live; Rust allocator crates pinned in Cargo.lock)

## Summary

Phase 3 ships six per-environment Dockerfiles (`alpine`, `debian-slim`, `distroless-cc`, `distroless-static`, `wolfi`, `scratch`) wrapped around the Phase-1/2 binary, parameterized by `ARG ALLOC` to select the Cargo feature, plus a Justfile that exposes a 12-cell allocator-runtime matrix (3 glibc envs × 3 glibc allocs + 3 musl envs × 3 musl allocs), a macOS host baseline, and a `dive --ci` image-size gate. All decisions in CONTEXT.md (D-01 .. D-23) are locked; this research provides the executable details the planner needs to convert each decision into tasks.

The single biggest unknown going into the phase is whether `jemalloc` and `mimalloc` link cleanly against `x86_64-unknown-linux-musl` with `crt-static` for the `scratch` and `distroless-static` cells. Decision D-01 already pre-empts this: build a 6-cell smoke matrix first (one alloc per env), confirm linkage, and only then commit to the 12-cell run. The PITFALLS.md §2.1 / §2.2 fixes (default-features=false, `unprefixed_malloc_on_supported_platforms` off) are already baked into the workspace Cargo.toml — no Cargo changes expected in this phase. Two confirmed musl/static gotchas: (1) `tikv-jemallocator` 0.6.1's default features are conservative (just `background_threads_runtime_support`) so the workspace's plain `tikv-jemallocator = "0.6"` declaration is musl-safe out of the box [VERIFIED: tikv/jemallocator Cargo.toml]; (2) `mimalloc` 0.1.50 with `default-features = false` (already in workspace) avoids the v3 segment-allocator paths that historically broke on musl [VERIFIED: purpleprotocol/mimalloc_rust Cargo.toml]. Both are smoke-tested in the first 6-cell build.

**Primary recommendation:** Two-pass build/run. (1) **Smoke pass:** one Dockerfile per env, default ALLOC, build + run the 6-cell smoke matrix to confirm cargo-chef + cross-compile + static-link + entrypoint all work end-to-end with the existing binary. (2) **Full pass:** parameterize each Dockerfile with `ARG ALLOC`, run the 12-cell matrix sequentially via `just bench-all` with `--cpus=4 --memory=4g --cpuset-cpus=0-3`. Fall back to documented cell-drops in SUMMARY.md if any musl-static jemalloc/mimalloc cell fails to link (D-01 escape hatch).

## User Constraints (from CONTEXT.md)

### Locked Decisions

**Matrix cell selection & tagging**
- **D-01:** 12-cell meaningful matrix — physically possible (env × alloc) combinations only:
  - glibc envs × glibc allocs (9 cells): `debian-slim`, `distroless-cc`, `wolfi` × `ptmalloc`, `jemalloc`, `mimalloc`
  - musl envs × musl allocs (9 cells): `alpine`, `distroless-static`, `scratch` × `mallocng`, `jemalloc`, `mimalloc`
  - Total 18 nominally; if `jemalloc-on-distroless-static` or `mimalloc-on-scratch` static-link smoke fails, drop those cells with documented reason.
- **D-02:** Image tag: `alloc-bench:{alloc}-{env}` (e.g., `alloc-bench:jemalloc-alpine`).
- **D-03:** Results filename: `results/{alloc}-{env}.json` (flat layout).
- **D-04:** Hard-skip cross-libc combos. mallocng on glibc, ptmalloc on musl — Justfile rejects with clear error.

**Base images & build strategy**
- **D-05:** Pinned base images: `alpine:3.20`, `debian:bookworm-slim`, `gcr.io/distroless/cc-debian12:nonroot`, `gcr.io/distroless/static-debian12:nonroot`, `cgr.dev/chainguard/wolfi-base:latest`, `scratch`.
- **D-06:** Builder bases: `rust:1.83-bookworm` (glibc), `rust:1.83-alpine` (musl). Pin via `ARG RUST_VERSION=1.83`.
- **D-07:** Six Dockerfiles, one per runtime env, parameterized by `ARG ALLOC=ptmalloc`. cargo-chef multi-stage. Plan-phase keeps Dockerfiles ≤ 80 lines.
- **D-08:** OCI annotations on every image: `org.opencontainers.image.{title, description, source, version, revision, licenses, created, authors}`. Default to LABEL lines parameterized by Dockerfile ARGs.
- **D-09:** `ENV RUSTFLAGS="-C target-cpu=x86-64-v3"` in builder; `bench-host` uses `target-cpu=native`.

**Justfile recipe surface**
- **D-10:** Positional-arg recipes: `just build {env} {alloc}`, `just run {env} {alloc}`, `just bench-cell {env} {alloc}`. Cap at 2 positional args.
- **D-11:** `just bench-all` runs the full 12+/18-cell matrix sequentially. Per-cell logs streamed with `[{alloc}-{env}]` prefix. Per-cell JSON written immediately to `results/{alloc}-{env}.json`.
- **D-12:** No aggregation in Phase 3 — `bench-all` ends with stdout summary table (`alloc, env, status, ticks_per_s_p50`).
- **D-13:** `just bench-all-smoke` runs the matrix with `--warmup 1s --duration 5s` per scenario.
- **D-14:** `just clean-images` removes all `alloc-bench:*` tags. `just dive-check {env} {alloc}` wraps `dive --ci`. `just dive-check-all` runs for every image.

**Runtime defaults (cgroup / NUMA)**
- **D-15:** Default `docker run` flags: `--cpus=4 --memory=4g --cpuset-cpus=0-3 --rm -v $(pwd)/results:/out`.
- **D-16:** NUMA pinning via `--cpuset-cpus` only. No `numactl` inside images.
- **D-17:** Override knobs via env vars: `BENCH_MEMORY=8g`, `BENCH_CPUSET=4-7`.

**macOS bench-host**
- **D-18:** `just bench-host` builds and runs natively on macOS using libmalloc only. Output `results/host-system.json`. Build flags: `-C target-cpu=native`, target = host. env block: `docker_image: null`, `target_triple: <host>`, `os: "macos"`.
- **D-19:** macOS does NOT run jemalloc/mimalloc. Libmalloc dev-box reference only.

**Bench durations**
- **D-20:** Phase-1 defaults preserved for matrix runs: `--warmup 5s --duration 60s` per scenario. Smoke recipe overrides.

**Image size & dive**
- **D-21:** `dive --ci` thresholds in `.dive-ci`: `lowestEfficiency: 0.95`, `highestUserWastedPercent: 0.05`, `highestWastedBytes: 50MB`.
- **D-22:** Image-size budgets per env (informational): `scratch ≤ 15MB`, `distroless-static ≤ 25MB`, `alpine ≤ 30MB`, `wolfi ≤ 35MB`, `distroless-cc ≤ 50MB`, `debian-slim ≤ 100MB`.

**Schema additions (none)**
- **D-23:** No results.json schema changes. Schema v1 locked. Phase 3 just populates the existing env block.

### Claude's Discretion

- Exact ordering of build stages within each Dockerfile.
- Whether `just bench-all` uses `set -euo pipefail`-style strict mode or per-cell error capture. **Recommended: per-cell error capture** so a single broken cell does not abort the rest of the matrix; capture the failure status and report in the stdout summary (D-12).
- Whether to add a separate `Dockerfile.builder` shared across runtime stages or duplicate the builder stage in each runtime Dockerfile. **Recommended: duplicate the builder stage** in each runtime Dockerfile — cargo-chef's BuildKit layer cache deduplicates the work across `docker buildx build` invocations sharing the same context, and a single shared builder Dockerfile complicates the per-Dockerfile `--target` selection (musl envs use `--target x86_64-unknown-linux-musl`, glibc envs use `--target x86_64-unknown-linux-gnu`).
- Whether `just run` mounts `./results` read-write or only writes via stdout redirection. **Recommended: mount `./results` to `/out` read-write** — matches success criterion 2 verbatim and is more ergonomic.
- How the macOS host bench builds for x86 vs. aarch64. **Recommended: detect via `uname -m` in the recipe body** and pass to `cargo build --release --target` only when the user is on Rosetta or unusual setups; otherwise omit `--target` (Cargo picks host).

### Deferred Ideas (OUT OF SCOPE)

- **Image-size CI enforcement** → Phase 5
- **Multiple runs per cell with median + min/max range** → Phase 5 (REPR-03)
- **Aggregator** → Phase 4
- **README "Run it yourself" walkthrough** → Phase 5 (REPR-01)
- **Cross-NUMA-node experiments** → v2
- **macOS-native jemalloc/mimalloc** → v2 (D-19)
- **`numactl --membind` inside images** → not planned (D-16)
- **Per-architecture matrix (aarch64 in addition to x86_64)** → v2 (REQUIREMENTS V2-09)
- **`snmalloc` / `tcmalloc` / `rpmalloc`** → v2

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DOCK-01 | Build alpine image (musl dynamic) via `docker/alpine.Dockerfile` with cargo-chef + OCI annotations | Standard Stack §1, §3, §4; Code Examples §1 |
| DOCK-02 | Build debian-slim image (glibc dynamic) via `docker/debian-slim.Dockerfile` | Same multi-stage pattern, glibc target |
| DOCK-03 | Build distroless-cc image (jemalloc/mimalloc on glibc) via `docker/distroless-cc.Dockerfile` | Pitfall §2: distroless-cc has glibc + libgcc1 [VERIFIED: docker manifest] |
| DOCK-04 | Build distroless-static image (musl static) via `docker/distroless-static.Dockerfile` | Static-linking research §2 + nonroot UID 65532 [VERIFIED] |
| DOCK-05 | Build scratch image (fully static musl + crt-static) via `docker/scratch.Dockerfile` | Static-linking research §2 + scratch-specific gotchas §3 |
| DOCK-06 | Build wolfi image via `docker/wolfi.Dockerfile` | Wolfi is glibc-based [VERIFIED: ldd output]; runs as UID 0 by default |
| DOCK-07 | `dive --ci` thresholds pass for every image | dive 0.13.1 + .dive-ci config §4; Code Examples §3 |
| DOCK-08 | OCI annotations on every image | OCI spec annotations [VERIFIED: opencontainers/image-spec]; Code Examples §4 |
| DOCK-09 | `docker run --cpus=4 --memory=4g --cpuset-cpus=0-3 ... run-all` produces results.json | NUMA + cgroup defaults baked into Justfile; Pitfall §3.1 (4 GiB ≥ mimalloc 64 MiB segment) |
| ORCH-01 | `just bench-all` builds + runs full meaningful matrix, one results.json per cell | Justfile patterns §5; Code Examples §5 |
| ORCH-02 | `just bench-host` builds + runs natively on macOS | Justfile macOS pattern §6; Code Examples §6 |

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Allocator selection (compile-time Cargo feature) | Build (Dockerfile) | — | Phase 1 contract: `--features alloc-jemalloc` / `alloc-mimalloc` / no-feature (system). Dockerfile only forwards `ARG ALLOC` → `--features` mapping. |
| Cross-libc target selection (musl vs glibc) | Build (Dockerfile) | — | Each Dockerfile pins `--target x86_64-unknown-linux-{musl,gnu}` based on which libc family its runtime base uses. |
| Static linking enforcement | Build (Dockerfile) | — | `RUSTFLAGS="-C target-feature=+crt-static"` in builder for `scratch` + `distroless-static` only; `alpine` uses dynamic musl. |
| Dependency caching | Build (cargo-chef) | — | `cargo chef prepare → cook → build` three-stage pattern; recipe.json is the cache key. |
| OCI metadata | Build (Dockerfile LABELs) | — | LABELs interpolate `ARG`s set by `--build-arg` from Justfile recipe. |
| NUMA + cgroup limits | Run (Justfile) | Docker daemon | `--cpus`, `--memory`, `--cpuset-cpus` are container-runtime concerns; `numactl` NOT used (D-16). |
| Per-cell results.json emit | Run (existing binary in container) | Container volume | Bench binary writes to `/out/{alloc}-{env}.json`; Justfile mounts `$(pwd)/results:/out`. |
| Matrix iteration | Local orchestration (Justfile) | — | `just bench-all` loops over hard-coded valid (env, alloc) tuples (D-04 hard-skips cross-libc). Sequential, not parallel. |
| Image-size verification | Local CI gate (`dive --ci`) | `.dive-ci` config | Wraps `dive` binary; not a Docker concern. |
| macOS host baseline | Native build + run (Justfile) | — | No Docker — direct `cargo run --release`. Output to `results/host-system.json`. |

## Standard Stack

### Core
| Tool / Library | Version | Purpose | Why Standard |
|----------------|---------|---------|--------------|
| Docker / BuildKit | 29.x + buildx 0.33+ | Container build + run | Universal; multi-arch via `--platform linux/amd64` |
| cargo-chef | 0.1.77 (Mar 2026) [VERIFIED: github.com/LukeMathWalker/cargo-chef] | Dependency-layer caching for Rust Docker builds | Canonical Rust Docker pattern; supports `--target` for cross-compile [VERIFIED: README] |
| just | 1.51.0 (host) [VERIFIED: `just --version`] | Justfile orchestration | Already adopted in Phase 1/2; positional args + shebang recipes [CITED: just.systems/man/en] |
| dive | 0.13.1 (Mar 2025) [VERIFIED: github.com/wagoodman/dive/releases] | Image-efficiency CI gate | Standard tool; `.dive-ci` config supports `lowestEfficiency`, `highestUserWastedPercent`, `highestWastedBytes` [VERIFIED] |
| rust:1.83-bookworm | tag exists (created 2024-12-25) [VERIFIED: docker manifest] | Glibc builder base | Pinned via `ARG RUST_VERSION=1.83`; ages out only when 1.83 tag is removed from Hub (rare for stable releases) |
| rust:1.83-alpine | tag exists (created 2024-12-05) [VERIFIED: docker manifest] | Musl builder base | Same pinning approach |
| alpine:3.20 | 3.20.10 [VERIFIED: `cat /etc/alpine-release`], musl libc [VERIFIED: `ld-musl-x86_64.so.1`] | Runtime base — musl dynamic | Matches CONTEXT.md success criterion 2 literal |
| debian:bookworm-slim | 12.13 [VERIFIED] | Runtime base — glibc dynamic | Standard Debian stable slim |
| gcr.io/distroless/cc-debian12:nonroot | UID 65532, WorkingDir=/home/nonroot, includes glibc+libgcc1 [VERIFIED: image config] | Runtime base — glibc minimal | Standard distroless variant for "mostly-static" Rust |
| gcr.io/distroless/static-debian12:nonroot | UID 65532, WorkingDir=/home/nonroot, no libc [VERIFIED: image config] | Runtime base — fully static | Requires `--target x86_64-unknown-linux-musl` + `+crt-static` |
| cgr.dev/chainguard/wolfi-base:latest | UID 0 (root), Wolfi distro, glibc [VERIFIED: `ldd` linker is `/lib/ld-linux-x86-64.so.2`] | Runtime base — glibc, modern alternative to debian-slim | NOT musl — confirms CONTEXT.md D-01 placement of wolfi in glibc family |
| scratch | empty, no /etc, no /proc-until-mounted | Runtime base — fully static | Smallest possible; binary runs as root |

### Supporting
| Crate | Version (locked) | Purpose | When to Use |
|-------|------------------|---------|-------------|
| tikv-jemallocator | 0.6.1 [VERIFIED: Cargo.lock] | jemalloc global allocator | When `--features alloc-jemalloc` |
| tikv-jemalloc-ctl | 0.6.1 [VERIFIED: Cargo.lock] | jemalloc stats | Same |
| mimalloc | 0.1.50 [VERIFIED: Cargo.lock] | mimalloc global allocator wrapper | When `--features alloc-mimalloc` |
| libmimalloc-sys | 0.1.47 [VERIFIED: Cargo.lock] | mimalloc native bindings | Transitive dep |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Per-Dockerfile builder stage duplication | Single shared `Dockerfile.builder` + `--from=builder` | Shared builder is cleaner conceptually but breaks per-target `--target` parameterization in cargo-chef cook step. Plan-phase Discretion already recommends keeping the builder per-Dockerfile; cargo-chef + BuildKit shared cache makes the "duplication" cost ~zero in practice. |
| `clux/muslrust` builder image | `rust:1.83-alpine` builder | clux/muslrust ships musl-tools + sccache + protoc — overkill for our pure-Rust workspace. `rust:1.83-alpine` is the minimal first-party choice (already produces musl-targeting toolchain because Alpine itself is musl). [CITED: github.com/clux/muslrust Dockerfile] |
| `cargo-zigbuild` | In-Docker cross-compile | zigbuild is convenient for ad-hoc dev on macOS but requires installing Zig + zigbuild on the host; CI also needs it. STACK.md §10 already recommends "Docker-builder-stage for all matrix runs; document `cargo-zigbuild` for local debugging." Phase 3 follows that advice. |
| LABEL-based OCI annotation injection | `docker buildx build --label org...=...` flags | LABELs in Dockerfile + ARG interpolation is one mechanism (declarative); `--label` flags in the build command is another (imperative). Both produce identical `docker inspect` output. **Recommended: LABELs in Dockerfile** (D-08) — keeps the Dockerfile self-describing and avoids a Justfile recipe that has to know all eight annotation keys per cell. |
| numactl --membind inside images | `--cpuset-cpus` only (CONTEXT D-16) | numactl needs root + adds 100s of KB to image; cpuset is honored uniformly across Docker Desktop (macOS) and Linux servers without root. CONTEXT D-16 already locks this. |
| `numactl` package via apt/apk | `--cpuset-cpus` only | Same — no install needed. |

**Installation (host machine — for local dev only; Docker images install cargo-chef inside themselves):**
```bash
# macOS (Apple Silicon)
brew install dive just              # both already verified above
# Docker, BuildKit, buildx: shipped with Docker Desktop / OrbStack
```

**Version verification:** Already done — `just 1.51.0`, `Docker 29.4.0`, `buildx 0.33.0` confirmed via `command -v`. `dive` not installed locally; planner adds an `install-dive` step or runs via Docker (`wagoodman/dive`).

## Package Legitimacy Audit

> Required: Phase 3 ships Docker images that install `cargo-chef` from crates.io (inside builder stage) and references the four allocator crates already pinned in Phase 1. No new Rust dependencies are added in Phase 3.

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `cargo-chef` | crates.io | 5+ yrs | 1M+/mo | github.com/LukeMathWalker/cargo-chef | [OK] | Approved |
| `tikv-jemallocator` | crates.io | 4+ yrs (fork active) | high | github.com/tikv/jemallocator | [OK] | Approved (already in Cargo.lock from Phase 1) |
| `tikv-jemalloc-ctl` | crates.io | 4+ yrs | high | github.com/tikv/jemallocator | [OK] | Approved (already in Cargo.lock) |
| `mimalloc` | crates.io | 5+ yrs | high | github.com/purpleprotocol/mimalloc_rust | [OK] | Approved (already in Cargo.lock) |
| `libmimalloc-sys` | crates.io | 5+ yrs | high | github.com/purpleprotocol/mimalloc_rust | [OK] | Approved (transitive of mimalloc) |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

> Verified via `slopcheck install --ecosystem crates.io cargo-chef tikv-jemallocator tikv-jemalloc-ctl mimalloc libmimalloc-sys` — all 5 returned `[OK]` against crates.io. (Test command: `slopcheck 0.6.1` against crates.io registry, 2026-05-19.)

**Image registries used (not slopcheck'd; instead verified via live `docker buildx imagetools inspect`):**

| Image | Tag | Verified | Notes |
|-------|-----|----------|-------|
| `rust:1.83-bookworm` | tag | [VERIFIED: docker manifest 2024-12-25] | Builder for glibc envs |
| `rust:1.83-alpine` | tag | [VERIFIED: docker manifest 2024-12-05] | Builder for musl envs |
| `alpine:3.20` | tag | [VERIFIED: 3.20.10 + musl libc] | Runtime base |
| `debian:bookworm-slim` | tag | [VERIFIED: 12.13 + glibc] | Runtime base |
| `gcr.io/distroless/cc-debian12:nonroot` | tag | [VERIFIED: UID 65532 + glibc] | Runtime base |
| `gcr.io/distroless/static-debian12:nonroot` | tag | [VERIFIED: UID 65532 + no libc] | Runtime base |
| `cgr.dev/chainguard/wolfi-base:latest` | tag | [VERIFIED: UID 0 + glibc] | Runtime base — see Pitfall §6 about `:latest` mutability |
| `scratch` | builtin | trivially valid | Runtime base |

## Architecture Patterns

### System Architecture Diagram

```
                 ┌─────────────────────────────────────────────────────┐
                 │  Developer (macOS host) / GitHub runner             │
                 │                                                     │
                 │  $ just bench-all  (or: just bench-cell <env> <a>)  │
                 └────────────────┬────────────────────────────────────┘
                                  │
                                  ▼
              ┌─────────────────────────────────────────┐
              │  justfile  (Phase 3 deliverable)        │
              │  - validates (env, alloc) tuple         │   D-04: hard-skip
              │  - hard-skips cross-libc combos         │       cross-libc
              │  - sets default --cpus / --memory /     │
              │    --cpuset-cpus  (D-15, D-16, D-17)    │
              │  - loops over valid tuples (D-11)       │
              └─────────┬─────────────────────┬─────────┘
                        │                     │
                build path             run path
                        │                     │
                        ▼                     ▼
        ┌──────────────────────┐   ┌──────────────────────────┐
        │ docker buildx build  │   │ docker run               │
        │   -f docker/<env>.df │   │   --cpus=4 --memory=4g   │
        │   --build-arg ALLOC= │   │   --cpuset-cpus=0-3      │
        │   --tag alloc-bench: │   │   -v $(pwd)/results:/out │
        │     <alloc>-<env>    │   │   alloc-bench:<a>-<env>  │
        └─────────┬────────────┘   │   run-all                │
                  │                │     --output /out/<a>-<env>.json
                  ▼                └────────────┬─────────────┘
        ┌─────────────────────────────────────────┐                    │
        │  Per-env Dockerfile (×6 — D-07)         │                    │
        │                                         │                    │
        │  Stage 1: chef base                     │                    │
        │   FROM rust:1.83-{bookworm|alpine}      │                    │
        │   RUN cargo install --locked cargo-chef │                    │
        │                                         │                    │
        │  Stage 2: planner                       │                    │
        │   COPY . . / RUN cargo chef prepare     │                    │
        │                                         │                    │
        │  Stage 3: builder                       │                    │
        │   ENV RUSTFLAGS="-C target-cpu=v3 [+   │                    │
        │     crt-static for scratch/static]"     │                    │
        │   RUN rustup target add <target>        │                    │
        │   RUN cargo chef cook --release          │                    │
        │       --target <target>                  │                    │
        │       [--features alloc-<alloc>]         │                    │
        │   COPY . .                              │                    │
        │   RUN cargo build --release              │                    │
        │       --target <target>                  │                    │
        │       -p alloc-bench-cli                 │                    │
        │       [--no-default-features              │                    │
        │        --features alloc-<alloc>]         │                    │
        │                                         │                    │
        │  Stage 4: runtime (per-env)             │                    │
        │   FROM <env-base>                       │                    │
        │   COPY --from=builder /app/target/      │                    │
        │     <target>/release/alloc-bench-cli    │                    │
        │     /usr/local/bin/  (or / for nonroot) │                    │
        │   LABEL org.opencontainers.image.* …    │                    │
        │   ENTRYPOINT ["…/alloc-bench-cli"]      │                    │
        └────────────┬────────────────────────────┘                    │
                     │                                                 │
        OCI image    │                                                 │
        alloc-bench: │                                                 │
        <alloc>-<env>│                                                 │
                     ▼                                                 ▼
                 ┌─────────────────────────────────────────────┐
                 │  Container runtime                          │
                 │   (constrained by --cpuset / --memory)      │
                 │                                             │
                 │   alloc-bench-cli run-all                   │
                 │     reads env (Phase-1 metrics::env)        │
                 │     runs 11 scenarios                       │
                 │     writes JSON to /out/<alloc>-<env>.json  │
                 └────────────┬────────────────────────────────┘
                              │
                              ▼
                 ┌─────────────────────────────────────────────┐
                 │ ./results/<alloc>-<env>.json (per cell)     │
                 │   — flat layout, glob-ready for Phase 4     │
                 └─────────────────────────────────────────────┘

                 ┌─────────────────────────────────────────────┐
                 │ ./results/host-system.json  (D-18)          │
                 │   — produced by just bench-host (no Docker) │
                 └─────────────────────────────────────────────┘
```

### Recommended Project Structure

```
/                          # repo root
├── docker/                # Phase 3 NEW — six per-env Dockerfiles
│   ├── alpine.Dockerfile
│   ├── debian-slim.Dockerfile
│   ├── distroless-cc.Dockerfile
│   ├── distroless-static.Dockerfile
│   ├── scratch.Dockerfile
│   └── wolfi.Dockerfile
├── scripts/               # existing — Phase 3 may add bench_all.sh
│   ├── dce_check.sh           # Phase 2 (existing)
│   ├── build_image.sh         # Phase 3 (new — optional, helps Justfile)
│   └── bench_all.sh           # Phase 3 (new — or inline in Justfile)
├── crates/                # unchanged
├── results/               # gitignored — bench output target
├── .dive-ci               # Phase 3 NEW — dive thresholds (D-21)
├── .dockerignore          # Phase 3 NEW — keep build context small
├── Cargo.toml             # unchanged
├── Cargo.lock             # unchanged
├── justfile               # extended — Phase 3 adds D-10 ... D-14 recipes
└── CLAUDE.md              # unchanged
```

### Pattern 1: cargo-chef multi-stage with cross-compile and feature selection

**What:** Three-stage Dockerfile (chef → planner → builder) plus a per-env runtime stage. Parameterizes target triple, feature flags, and runtime base via `ARG`.

**When to use:** Every Dockerfile in `docker/`.

**Example (canonical pattern, distroless-static variant):**
```dockerfile
# Source: github.com/LukeMathWalker/cargo-chef README + ARCHITECTURE.md §"Cross-compilation strategy"
ARG RUST_VERSION=1.83

# ─── Stage 1: chef base (musl) ──────────────────────────────────────────
FROM rust:${RUST_VERSION}-alpine AS chef
RUN apk add --no-cache musl-dev pkgconfig openssl-dev      # build deps for axum/tokio/reqwest cargo features
RUN cargo install --locked cargo-chef@0.1.77
WORKDIR /app

# ─── Stage 2: planner — compute the recipe.json fingerprint ───────────
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ─── Stage 3: builder — cook deps, then build the bin ─────────────────
FROM chef AS builder
ARG ALLOC=mallocng
ARG TARGET=x86_64-unknown-linux-musl
ENV RUSTFLAGS="-C target-cpu=x86-64-v3 -C target-feature=+crt-static"
RUN rustup target add ${TARGET}

# Map ALLOC → Cargo features. Note: ptmalloc / mallocng / libmalloc are
# NOT Cargo features — they are the system default when no feature is
# enabled. Only jemalloc / mimalloc need a --features flag.
COPY --from=planner /app/recipe.json recipe.json
RUN if [ "$ALLOC" = "jemalloc" ]; then \
        cargo chef cook --release --target ${TARGET} \
            --no-default-features --features alloc-jemalloc \
            -p alloc-bench-cli --recipe-path recipe.json; \
    elif [ "$ALLOC" = "mimalloc" ]; then \
        cargo chef cook --release --target ${TARGET} \
            --no-default-features --features alloc-mimalloc \
            -p alloc-bench-cli --recipe-path recipe.json; \
    else \
        cargo chef cook --release --target ${TARGET} \
            -p alloc-bench-cli --recipe-path recipe.json; \
    fi

COPY . .
RUN if [ "$ALLOC" = "jemalloc" ]; then \
        cargo build --release --target ${TARGET} \
            --no-default-features --features alloc-jemalloc \
            -p alloc-bench-cli; \
    elif [ "$ALLOC" = "mimalloc" ]; then \
        cargo build --release --target ${TARGET} \
            --no-default-features --features alloc-mimalloc \
            -p alloc-bench-cli; \
    else \
        cargo build --release --target ${TARGET} \
            -p alloc-bench-cli; \
    fi

# ─── Stage 4: runtime — distroless-static (UID 65532) ─────────────────
FROM gcr.io/distroless/static-debian12:nonroot AS runtime
ARG ALLOC=mallocng
ARG TARGET=x86_64-unknown-linux-musl

# OCI annotations (D-08). All eight required by DOCK-08.
ARG OCI_TITLE="alloc-bench"
ARG OCI_DESCRIPTION="Memory allocator benchmark (mallocng/jemalloc/mimalloc on musl static)"
ARG OCI_SOURCE="https://github.com/marccarre/rust-benchmark-glibc-musl-mimalloc"
ARG OCI_VERSION
ARG OCI_REVISION
ARG OCI_LICENSES="MIT OR Apache-2.0"
ARG OCI_CREATED
ARG OCI_AUTHORS="Marc Carré"
LABEL org.opencontainers.image.title="${OCI_TITLE}" \
      org.opencontainers.image.description="${OCI_DESCRIPTION}" \
      org.opencontainers.image.source="${OCI_SOURCE}" \
      org.opencontainers.image.version="${OCI_VERSION}" \
      org.opencontainers.image.revision="${OCI_REVISION}" \
      org.opencontainers.image.licenses="${OCI_LICENSES}" \
      org.opencontainers.image.created="${OCI_CREATED}" \
      org.opencontainers.image.authors="${OCI_AUTHORS}"

COPY --from=builder /app/target/${TARGET}/release/alloc-bench-cli /alloc-bench-cli
USER nonroot
WORKDIR /home/nonroot
ENTRYPOINT ["/alloc-bench-cli"]
```

### Pattern 2: Justfile cross-product matrix with hard-skip and per-cell error capture

**What:** A Justfile recipe that loops over a hard-coded list of valid (env, alloc) tuples — physically impossible combos are absent from the list, so D-04's "hard-skip" is structural, not conditional.

**When to use:** `just bench-all` (D-11), `just bench-all-smoke` (D-13).

**Example:**
```just
# justfile (Phase 3 additions)

# ──────────────────────────────────────────────────────────────────────
# Phase 3: Docker matrix recipes
# ──────────────────────────────────────────────────────────────────────

# Build one cell. Validates (env, alloc) and hard-rejects cross-libc combos.
build env alloc:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{env}}-{{alloc}}" in
        # Cross-libc rejections (D-04). Hard-error, not silent skip.
        debian-slim-mallocng|distroless-cc-mallocng|wolfi-mallocng) \
            echo "[ERR] mallocng is the musl libc allocator; cannot run on glibc env '{{env}}'" >&2; exit 1 ;;
        alpine-ptmalloc|distroless-static-ptmalloc|scratch-ptmalloc) \
            echo "[ERR] ptmalloc is the glibc libc allocator; cannot run on musl env '{{env}}'" >&2; exit 1 ;;
    esac
    # Map env → target triple
    case "{{env}}" in
        debian-slim|distroless-cc|wolfi) TARGET="x86_64-unknown-linux-gnu" ;;
        alpine|distroless-static|scratch) TARGET="x86_64-unknown-linux-musl" ;;
        *) echo "[ERR] unknown env '{{env}}'" >&2; exit 1 ;;
    esac
    docker buildx build \
        --platform linux/amd64 \
        -f docker/{{env}}.Dockerfile \
        --build-arg ALLOC={{alloc}} \
        --build-arg TARGET="$TARGET" \
        --build-arg RUST_VERSION=1.83 \
        --build-arg OCI_VERSION="$(grep -m1 '^version' crates/alloc-bench-cli/Cargo.toml | cut -d'"' -f2)" \
        --build-arg OCI_REVISION="$(git rev-parse HEAD)" \
        --build-arg OCI_CREATED="$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
        --tag alloc-bench:{{alloc}}-{{env}} \
        --load .

# Run one cell. Mounts ./results, applies cgroup + cpuset defaults.
run env alloc:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p results
    : "${BENCH_CPUS:=4}"
    : "${BENCH_MEMORY:=4g}"
    : "${BENCH_CPUSET:=0-3}"
    docker run --rm \
        --platform linux/amd64 \
        --cpus="${BENCH_CPUS}" --memory="${BENCH_MEMORY}" --cpuset-cpus="${BENCH_CPUSET}" \
        -v "$(pwd)/results:/out" \
        alloc-bench:{{alloc}}-{{env}} \
        run-all --output /out/{{alloc}}-{{env}}.json --seed 7

# Build + run one cell.
bench-cell env alloc:
    just build {{env}} {{alloc}}
    just run {{env}} {{alloc}}

# The 18-cell hard-coded valid tuple list (D-01, D-04). Cross-libc combos
# are absent — they cannot be selected. Format: "<env> <alloc>" per line.
# Order: glibc envs first (alphabetical by env, alphabetical by alloc),
# then musl envs.
_matrix_cells := '''
debian-slim ptmalloc
debian-slim jemalloc
debian-slim mimalloc
distroless-cc ptmalloc
distroless-cc jemalloc
distroless-cc mimalloc
wolfi ptmalloc
wolfi jemalloc
wolfi mimalloc
alpine mallocng
alpine jemalloc
alpine mimalloc
distroless-static mallocng
distroless-static jemalloc
distroless-static mimalloc
scratch mallocng
scratch jemalloc
scratch mimalloc
'''

# Run the full matrix sequentially, per-cell error capture (Discretion).
bench-all:
    #!/usr/bin/env bash
    set -uo pipefail   # -u and -o pipefail; NOT -e (we want to continue on per-cell failure)
    declare -a results=()
    while IFS= read -r line; do
        [[ -z "$line" ]] && continue
        env="${line%% *}"
        alloc="${line##* }"
        echo
        echo "════════════════════════════════════════════════════════"
        echo "[${alloc}-${env}] starting"
        echo "════════════════════════════════════════════════════════"
        if just bench-cell "$env" "$alloc" 2>&1 | sed "s/^/[${alloc}-${env}] /"; then
            results+=("OK   ${alloc}-${env}")
        else
            results+=("FAIL ${alloc}-${env}")
        fi
    done <<< '{{_matrix_cells}}'
    echo
    echo "════════════════════════════════════════════════════════"
    echo "Matrix summary"
    echo "════════════════════════════════════════════════════════"
    printf '%s\n' "${results[@]}"
    # D-12: stdout summary table — alloc, env, status, ticks_per_s_p50.
    # Where the JSON exists, jq the multithread scenario's ticks_per_s.
    echo
    echo "alloc env status ticks_per_s_p50"
    while IFS= read -r line; do
        [[ -z "$line" ]] && continue
        env="${line%% *}"
        alloc="${line##* }"
        json="results/${alloc}-${env}.json"
        if [[ -f "$json" ]]; then
            tps=$(jq -r '[.[] | select(.scenario.name=="multithread") | .metrics.ticks_per_s] | first // "n/a"' "$json")
            echo "${alloc} ${env} ok ${tps}"
        else
            echo "${alloc} ${env} fail -"
        fi
    done <<< '{{_matrix_cells}}'

# Smoke variant: 1s warmup + 5s measure per scenario (D-13). Same loop.
# Implemented by passing flags via env var to run-all (Phase 1 supports it
# implicitly through subcommand args — see Phase 1 review for env-var paths
# if absent). If run-all does NOT yet accept warmup/duration overrides,
# plan-phase replaces this with per-scenario overrides at Justfile level.
bench-all-smoke:
    BENCH_WARMUP=1s BENCH_DURATION=5s just bench-all

# Native macOS / Linux host bench — libmalloc / ptmalloc / mallocng baseline (D-18, D-19).
# No Docker. Builds with target-cpu=native.
bench-host:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p results
    # The .cargo/config.toml already sets RUSTFLAGS="-C target-cpu=native"
    # for native host builds (Phase 1 infrastructure). Just rebuild the bin.
    cargo build --release -p alloc-bench-cli
    target/release/alloc-bench-cli run-all --output results/host-system.json --seed 7
    echo "[host] wrote results/host-system.json (allocator=$(uname -s | tr '[:upper:]' '[:lower:]')-system)"

# dive image-efficiency check.
dive-check env alloc:
    dive --ci alloc-bench:{{alloc}}-{{env}} --ci-config .dive-ci

# Run dive against every image in the matrix.
dive-check-all:
    #!/usr/bin/env bash
    set -uo pipefail
    while IFS= read -r line; do
        [[ -z "$line" ]] && continue
        env="${line%% *}"
        alloc="${line##* }"
        echo "[dive] ${alloc}-${env}"
        just dive-check "$env" "$alloc" || echo "[dive] FAIL ${alloc}-${env}"
    done <<< '{{_matrix_cells}}'

# Cleanup.
clean-images:
    #!/usr/bin/env bash
    set -uo pipefail
    docker images --filter "reference=alloc-bench:*" --format '{{ "{{" }}.Repository{{ "}}" }}:{{ "{{" }}.Tag{{ "}}" }}' \
        | xargs -r docker rmi -f
```

> Note in the `clean-images` recipe: `{{ "{{" }}` is just's required escaping for emitting a literal `{{` into a shell command (because `{{` is just's variable-interpolation marker).

### Anti-Patterns to Avoid

- **Running matrix cells in parallel.** The kernel page cache, NUMA-node memory pressure, and CPU thermal state are shared resources. Two cells running concurrently on the same machine pollute each other's measurements (PITFALLS.md §1.3 + §1.4 spirit). D-11 locks **sequential** execution.
- **Inheriting `target-cpu=native` into Docker images.** `.cargo/config.toml` sets `target-cpu=native` for host builds. The Dockerfile must `ENV RUSTFLAGS="-C target-cpu=x86-64-v3"` to **override** this (PITFALLS §3.3). If you forget and rely on `.cargo/config.toml`, the image binary may use AVX-512 from your dev box and crash with `SIGILL` on a CI runner without it. Verify by running `objdump -d alloc-bench-cli | grep -E 'vmovdqa64|vpmullq'` in the smoke pass.
- **Hand-rolling OCI annotations as Dockerfile-static strings.** D-08 requires `version`, `revision`, and `created` to reflect the actual build — they MUST come from `--build-arg` set in the Justfile recipe (`OCI_VERSION` from Cargo.toml, `OCI_REVISION` from `git rev-parse HEAD`, `OCI_CREATED` from `date -u`). Hard-coded LABELs would lie to consumers.
- **Building for `linux/arm64` on Apple Silicon by accident.** OrbStack/Docker Desktop on Apple Silicon defaults to `linux/arm64`; the bench is a `linux/amd64`-only matrix in v1 (REQUIREMENTS V2-09 defers aarch64 to v2). Every `docker buildx build` and `docker run` MUST pass `--platform linux/amd64`. Confirmed via `docker info` showing `Architecture: aarch64` on the dev machine — this WOULD silently produce arm64 images without the platform flag.
- **`cargo install cargo-chef` without `--locked`.** cargo-chef's transitive deps occasionally pin breaking-change-prone crates. Always `cargo install --locked cargo-chef@0.1.77` (or whatever exact version is current at plan-phase time).
- **Mounting `./results` without `mkdir -p` first.** Docker on macOS will silently mount the path as `root`-owned if the host directory does not pre-exist; subsequent host writes fail. The `run` recipe's `mkdir -p results` is mandatory.
- **Running distroless `:nonroot` with the binary at `/usr/local/bin/alloc-bench-cli`.** distroless `:nonroot` images do not include `/usr/local/bin` in PATH for nonroot users guaranteed; place the binary at `/alloc-bench-cli` (root of FS) or `/home/nonroot/alloc-bench-cli` (the WorkingDir, UID 65532-owned). The Dockerfile pattern above uses `/alloc-bench-cli`.
- **Using `docker run --memory=64m` to "test the worst case."** mimalloc pre-allocates 64 MiB segments lazily; with `--memory=64m` the very first allocation OOM-kills (PITFALLS §3.1). D-15 locks `--memory=4g`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Multi-stage Rust Docker build with dep cache | A custom `cargo build --release` Dockerfile that re-downloads + re-compiles deps on every change | `cargo-chef` 0.1.77 [VERIFIED] | cargo-chef saves 5–20 minutes per build by caching the dep graph in a separate layer keyed by `recipe.json`. STACK.md §9 calls this "the canonical Rust Docker pattern." |
| Image-efficiency check | A custom `docker history` parser + bash math | `dive --ci` 0.13.1 with `.dive-ci` config | dive computes layer-by-layer waste, dedup ratio, and efficiency score; `.dive-ci` accepts the three thresholds CONTEXT.md D-21 specifies (`lowestEfficiency`, `highestUserWastedPercent`, `highestWastedBytes`) [VERIFIED]. Hand-rolling means re-implementing layer diffing. |
| OCI annotation injection | Bash sed/replace into a generated Dockerfile per cell | `LABEL` + `ARG` + `--build-arg` (Pattern 1) | Standard Dockerfile primitive; `docker inspect` reads them with no parsing. CONTEXT.md D-08 already locks this approach. |
| Cross-product matrix iteration | A bash array of strings + nested for-loops in Justfile | A single hard-coded list of valid (env, alloc) tuples (Pattern 2) | A nested for-loop forces conditional cross-libc skipping at runtime, which D-04 says should be hard-error. The hard-coded valid list makes the skip structural and the recipe shorter. |
| Cgroup memory limit selection | A heuristic that scales `--memory` based on RSS | Fixed `--memory=4g` (D-15) | mimalloc segment pre-allocation is 64 MiB; 4 GiB is a 64× headroom that covers every scenario. PITFALLS §3.1 already locked the floor. |
| NUMA pinning | `numactl --cpunodebind=0 --membind=0` inside container | `--cpuset-cpus=0-3` on `docker run` (D-16) | numactl needs root + adds a package; cpuset is a kernel-level cgroup constraint that works for the same purpose without either cost. |
| Static linking against musl | A bash script that munges `RUSTFLAGS` at build time | `ENV RUSTFLAGS="-C target-cpu=x86-64-v3 -C target-feature=+crt-static"` in builder stage | Standard. PITFALLS §2.3 already documents this exact flag combo. |

**Key insight:** Every problem this phase solves has a standard tool. The goal is composition (Dockerfile + Justfile + dive + cargo-chef + ARG/LABEL/RUSTFLAGS), not invention. The handful of bash glue is in the Justfile recipe bodies and is intentionally narrow (one hard-coded tuple list, one summary table, one validation case statement) — no recipe should grow beyond ~30 lines.

## Runtime State Inventory

> **Trigger check:** Phase 3 is **not** a rename/refactor/migration phase. It adds new files (`docker/*.Dockerfile`, `.dive-ci`, `.dockerignore`) and extends `justfile`. It does not rename anything in the existing codebase or rewrite any persistent state.

This section is included for completeness so a future maintainer running `/gsd:verify-work` can confirm "yes, this was checked":

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — bench writes ephemeral JSON to `./results/`, no databases. | none |
| Live service config | None — all configuration is in-repo (Cargo.toml, Justfile, Dockerfiles, `.dive-ci`). No external services with hidden state. | none |
| OS-registered state | None — no Task Scheduler / systemd / launchd / pm2 entries are created. Docker images and tags are the only OS-level state, and `just clean-images` removes them. | none |
| Secrets/env vars | `BENCH_CPUS`, `BENCH_MEMORY`, `BENCH_CPUSET` are user-overrides for the run recipe (D-17). Not secrets. The bench reads `DOCKER_IMAGE` env (Phase 1 `metrics::env::read_env`) — must be set in the Dockerfile or via `--env` to populate the `env.docker_image` JSON field. **Recommended:** set `ENV DOCKER_IMAGE="<base>:<tag>"` in each runtime Dockerfile so the bench picks it up automatically. | code-edit (Dockerfile only) |
| Build artifacts / installed packages | After Phase 3, the dev machine has `alloc-bench:*` Docker images. `just clean-images` removes them. No installed packages on host beyond optional `dive` (recommended via `brew install dive`). | document in SUMMARY |

**Nothing critical found in any category.** Phase 3 is a code-additive phase with one minor existing-code touch: the `metrics::env::read_env` function in `crates/alloc-bench-core/src/metrics/env.rs` already reads `DOCKER_IMAGE` env (line 7). Phase 3 must set it correctly per Dockerfile so the JSON env block is accurate (success criterion 2 expects `docker_image: "alpine:3.20"`).

## Common Pitfalls

### Pitfall 1: jemalloc/mimalloc fail to link statically against musl + crt-static

**What goes wrong:** Building `--features alloc-jemalloc` for `x86_64-unknown-linux-musl` with `+crt-static` fails at link time with errors like "undefined reference to `__tls_get_addr`" or "linking against libc.a not supported." Same can happen for `mimalloc`.

**Why it happens:** Historical issues with `tikv-jemalloc-sys`'s build script picking up dynamic-link assumptions on musl; mimalloc's segment-allocator code path historically used glibc-specific feature detection (PITFALLS §2.1, §2.2).

**How to avoid:**
- The workspace `Cargo.toml` already sets `mimalloc = { version = "0.1", default-features = false, features = ["extended"] }` — confirmed [VERIFIED: Cargo.toml lines 23–24].
- `tikv-jemallocator` 0.6.1's defaults are conservative (only `background_threads_runtime_support`) [VERIFIED: tikv/jemallocator Cargo.toml]; `unprefixed_malloc_on_supported_platforms` is **NOT** in defaults, which is the safe configuration for musl.
- D-01 already plans the escape hatch: build a 6-cell smoke matrix first (one alloc per env: ptmalloc-debian, jemalloc-debian, mimalloc-debian, mallocng-alpine, jemalloc-alpine, mimalloc-alpine), confirm linkage, and only then commit to the 18-cell run.

**Warning signs:** "undefined reference" or "cannot find -lc" in `cargo build` output; or the binary builds but `ldd` shows it's dynamic when you expected static.

### Pitfall 2: `target-cpu=native` leaks from `.cargo/config.toml` into Docker images

**What goes wrong:** The image is built using AVX-512 instructions specific to the dev machine's CPU; running it on a CI runner with older AVX yields `SIGILL`.

**Why it happens:** `.cargo/config.toml` at repo root sets `rustflags = ["-C", "target-cpu=native"]` for host builds (Phase 1 `WS-05`). cargo merges this with `RUSTFLAGS` env var.

**How to avoid:** Set `ENV RUSTFLAGS="-C target-cpu=x86-64-v3 …"` in the builder stage. The `ENV` directive overrides anything in `.cargo/config.toml` because `RUSTFLAGS` env wins over `[build] rustflags = …` (CONTEXT.md D-09).

**Warning signs:** `objdump -d /usr/local/bin/alloc-bench-cli | head` shows AVX-512 mnemonics (`vmovdqa64`, `vpmullq`); or `qemu-x86_64 -cpu Skylake-Server` returns `illegal instruction`.

### Pitfall 3: Apple Silicon / OrbStack defaults to `linux/arm64`

**What goes wrong:** `docker buildx build` produces an `arm64` image even though the matrix is x86_64-only in v1. Subsequent `docker run` on a CI x86_64 runner fails with "no matching manifest for linux/amd64."

**Why it happens:** OrbStack and Docker Desktop on Apple Silicon set the buildx default platform to the host architecture. Confirmed via `docker info`: `Architecture: aarch64` on the dev machine.

**How to avoid:** Every `docker buildx build` and `docker run` invocation in the Justfile MUST pass `--platform linux/amd64`. This is already in Pattern 2.

**Warning signs:** `docker inspect alloc-bench:jemalloc-alpine | jq '.[].Architecture'` shows `arm64`.

### Pitfall 4: distroless `:nonroot` cannot write to a host-mounted volume

**What goes wrong:** `docker run -v $(pwd)/results:/out alloc-bench:jemalloc-distroless-static run-all --output /out/jemalloc-distroless-static.json` fails with "permission denied" when the binary tries to create `/out/jemalloc-distroless-static.json`.

**Why it happens:** distroless `:nonroot` runs the binary as UID 65532. The host directory `./results` is owned by the developer's UID (501 on macOS / 1000 on Linux). The container UID can't write there.

**How to avoid:** Three options, in order of preference:
1. **Pre-create the host directory and `chmod 0777`** (or `chown 65532` on Linux). Simplest, lowest-friction.
2. **`docker run --user $(id -u):$(id -g)`** — overrides the image's `USER` directive. Works but breaks the "run as defined in image" guarantee.
3. **Have the binary write to stdout instead** and pipe to a host-side `tee`. Matches Phase 1 D-24 idiom but is more brittle.

**Recommended:** `mkdir -p results && chmod 0777 results` in the `run` recipe before `docker run`. macOS Docker Desktop / OrbStack actually mount as the host user by default (NFS-style), so this only matters on Linux runners — but doing it unconditionally is harmless.

**Warning signs:** "Permission denied (os error 13)" in container logs when opening the output file.

### Pitfall 5: scratch image can't print error messages because it has no `/etc/ld.so.cache` or terminfo

**What goes wrong:** A static binary in `scratch` panics or returns non-zero, but the container exits with no output beyond exit code.

**Why it happens:** `scratch` has nothing — no /etc, no /tmp, no /var. Rust's `eprintln!` works (writes to fd 2, no library lookup), but third-party panic hooks that try to read environment / TZ / locale will fail silently.

**How to avoid:**
- Bench-CLI's existing `print_version_banner` writes to `eprintln!` only — confirmed [VERIFIED: crates/alloc-bench-cli/src/main.rs:226].
- Confirm the binary doesn't attempt to read TZ data: `chrono` is in workspace deps with `default-features = false, features = ["clock", "serde"]` — `clock` uses `localtime_r` which reads `/etc/localtime`. **In scratch this returns UTC silently**, which is fine for our purposes (PITFALLS.md §2.3 explicitly says "bench uses UTC only; no TZ data needed").
- Smoke-test by running the scratch image against `run-all --output /out/test.json` early in plan-phase; if it crashes silently, add `--rm -it` to capture stderr.

**Warning signs:** Empty `docker logs <container>` despite non-zero exit.

### Pitfall 6: `cgr.dev/chainguard/wolfi-base:latest` is mutable

**What goes wrong:** A reproducible benchmark depends on the runtime base being byte-identical across runs. `:latest` is rebuilt frequently (last seen `created: 2026-05-14T15:05:17Z` [VERIFIED]); a re-run two weeks later may pick up a different build.

**Why it happens:** Chainguard rebuilds Wolfi images daily for CVE patching.

**How to avoid:** Pin by digest at plan-phase time. CONTEXT.md D-05 already calls this out: "or pinned digest at plan-phase time". Capture the current `latest` digest:
```bash
docker buildx imagetools inspect cgr.dev/chainguard/wolfi-base:latest --raw \
  | jq -r '.manifests[] | select(.platform.architecture=="amd64") | .digest'
```
…and use `cgr.dev/chainguard/wolfi-base@sha256:<digest>` in `docker/wolfi.Dockerfile`. Same applies to any `:latest` tag (none of the others in D-05 are `:latest` — alpine, debian, distroless are all version-pinned).

**Warning signs:** "Wolfi" run shows different `os_version` between two runs of the same git SHA.

### Pitfall 7: cargo-chef's recipe.json is sensitive to workspace member ordering

**What goes wrong:** A `cargo chef cook` step that worked yesterday now downloads and re-compiles all deps from scratch.

**Why it happens:** cargo-chef hashes the workspace's dep graph. If `Cargo.toml` member order changes, or if a `[patch.crates-io]` is added, the recipe.json hash changes and the cached layer is invalidated.

**How to avoid:**
- Don't reorder `members = ["crates/*"]` (a glob — already insensitive to order). [VERIFIED: workspace Cargo.toml line 3]
- Avoid `[patch]` sections.
- Use `cargo chef prepare --bin alloc-bench-cli` if you want to scope the recipe to one binary's deps (some cargo-chef versions support this).

**Warning signs:** Build time per Dockerfile suddenly goes from 1 min cached → 10+ min uncached on every commit.

### Pitfall 8: `.dockerignore` either too aggressive or too permissive

**What goes wrong:**
- Too permissive: `target/` (~500 MB) gets COPYed into the build context, slowing every build by minutes and bloating the BuildKit cache.
- Too aggressive: `Cargo.lock` is excluded, breaking reproducibility (cargo regenerates it but with different versions if `^x.y` semver allows).

**How to avoid:** A focused `.dockerignore` (Code Examples §7).

**Warning signs:** "Sending build context to Docker daemon  500MB" on every build (too permissive); or `Cargo.lock` regenerated inside the builder (too aggressive).

## Code Examples

### §1: alpine.Dockerfile (musl dynamic, simplest case)

```dockerfile
# Source: STACK.md §9, ARCHITECTURE.md §"Cross-compilation strategy",
# adapted for musl-dynamic (alpine) — no crt-static.
ARG RUST_VERSION=1.83

FROM rust:${RUST_VERSION}-alpine AS chef
RUN apk add --no-cache musl-dev pkgconfig
RUN cargo install --locked cargo-chef@0.1.77
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ARG ALLOC=mallocng
ARG TARGET=x86_64-unknown-linux-musl
ENV RUSTFLAGS="-C target-cpu=x86-64-v3"
RUN rustup target add ${TARGET}
COPY --from=planner /app/recipe.json recipe.json
RUN if [ "$ALLOC" = "jemalloc" ]; then \
        FEATURES="--no-default-features --features alloc-jemalloc"; \
    elif [ "$ALLOC" = "mimalloc" ]; then \
        FEATURES="--no-default-features --features alloc-mimalloc"; \
    else FEATURES=""; fi && \
    cargo chef cook --release --target ${TARGET} ${FEATURES} \
        -p alloc-bench-cli --recipe-path recipe.json
COPY . .
RUN if [ "$ALLOC" = "jemalloc" ]; then \
        FEATURES="--no-default-features --features alloc-jemalloc"; \
    elif [ "$ALLOC" = "mimalloc" ]; then \
        FEATURES="--no-default-features --features alloc-mimalloc"; \
    else FEATURES=""; fi && \
    cargo build --release --target ${TARGET} ${FEATURES} \
        -p alloc-bench-cli

FROM alpine:3.20 AS runtime
ARG OCI_VERSION
ARG OCI_REVISION
ARG OCI_CREATED
LABEL org.opencontainers.image.title="alloc-bench" \
      org.opencontainers.image.description="Memory allocator benchmark — alpine (musl dynamic)" \
      org.opencontainers.image.source="https://github.com/marccarre/rust-benchmark-glibc-musl-mimalloc" \
      org.opencontainers.image.version="${OCI_VERSION}" \
      org.opencontainers.image.revision="${OCI_REVISION}" \
      org.opencontainers.image.licenses="MIT OR Apache-2.0" \
      org.opencontainers.image.created="${OCI_CREATED}" \
      org.opencontainers.image.authors="Marc Carré"
ENV DOCKER_IMAGE=alpine:3.20
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/alloc-bench-cli /usr/local/bin/
ENTRYPOINT ["/usr/local/bin/alloc-bench-cli"]
```

### §2: scratch.Dockerfile (fully static, smallest)

```dockerfile
# Source: PITFALLS.md §2.3 + STACK.md §9 + ARCHITECTURE.md.
# scratch has no /etc, no shell, no resolver. Bench is HTTP-only on
# 127.0.0.1; no TLS / DNS / TZ data needed (PITFALLS §2.3 confirms).
ARG RUST_VERSION=1.83

FROM rust:${RUST_VERSION}-alpine AS chef
RUN apk add --no-cache musl-dev pkgconfig
RUN cargo install --locked cargo-chef@0.1.77
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ARG ALLOC=mallocng
ARG TARGET=x86_64-unknown-linux-musl
# +crt-static is the key delta vs alpine.Dockerfile — produces a
# fully-static binary that scratch can run.
ENV RUSTFLAGS="-C target-cpu=x86-64-v3 -C target-feature=+crt-static"
RUN rustup target add ${TARGET}
COPY --from=planner /app/recipe.json recipe.json
RUN if [ "$ALLOC" = "jemalloc" ]; then \
        FEATURES="--no-default-features --features alloc-jemalloc"; \
    elif [ "$ALLOC" = "mimalloc" ]; then \
        FEATURES="--no-default-features --features alloc-mimalloc"; \
    else FEATURES=""; fi && \
    cargo chef cook --release --target ${TARGET} ${FEATURES} \
        -p alloc-bench-cli --recipe-path recipe.json
COPY . .
RUN if [ "$ALLOC" = "jemalloc" ]; then \
        FEATURES="--no-default-features --features alloc-jemalloc"; \
    elif [ "$ALLOC" = "mimalloc" ]; then \
        FEATURES="--no-default-features --features alloc-mimalloc"; \
    else FEATURES=""; fi && \
    cargo build --release --target ${TARGET} ${FEATURES} \
        -p alloc-bench-cli

FROM scratch AS runtime
ARG OCI_VERSION
ARG OCI_REVISION
ARG OCI_CREATED
LABEL org.opencontainers.image.title="alloc-bench" \
      org.opencontainers.image.description="Memory allocator benchmark — scratch (musl static)" \
      org.opencontainers.image.source="https://github.com/marccarre/rust-benchmark-glibc-musl-mimalloc" \
      org.opencontainers.image.version="${OCI_VERSION}" \
      org.opencontainers.image.revision="${OCI_REVISION}" \
      org.opencontainers.image.licenses="MIT OR Apache-2.0" \
      org.opencontainers.image.created="${OCI_CREATED}" \
      org.opencontainers.image.authors="Marc Carré"
ENV DOCKER_IMAGE=scratch
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/alloc-bench-cli /alloc-bench-cli
ENTRYPOINT ["/alloc-bench-cli"]
```

> **Note:** scratch images do NOT need `/etc/passwd` or `/etc/ssl/certs` for this bench (HTTP-only on localhost; PITFALLS §2.3). Do not COPY them in — keeps image to ~7-15 MB.

### §3: .dive-ci

```yaml
# Source: github.com/wagoodman/dive README — verified 2026-05-19.
# CONTEXT.md D-21 thresholds.
rules:
  # Layer-stack efficiency: 1.0 = no duplication, no waste; 0.95 = up to 5% redundancy allowed.
  lowestEfficiency: 0.95
  # Maximum % of image size occupied by wasted bytes (excluding base layer).
  highestUserWastedPercent: 0.05
  # Absolute cap on wasted bytes.
  highestWastedBytes: 50MB
```

Run via `dive --ci alloc-bench:<alloc>-<env> --ci-config .dive-ci`. Returns exit 0 if pass, 1 if any threshold violated.

### §4: OCI annotations injection (LABEL + ARG + Justfile build-arg)

```dockerfile
# In each runtime stage:
ARG OCI_VERSION                 # set by Justfile from Cargo.toml
ARG OCI_REVISION                # set by Justfile from `git rev-parse HEAD`
ARG OCI_CREATED                 # set by Justfile from `date -u +%Y-%m-%dT%H:%M:%SZ`
LABEL org.opencontainers.image.title="alloc-bench" \
      org.opencontainers.image.description="Memory allocator benchmark — <env>" \
      org.opencontainers.image.source="https://github.com/marccarre/rust-benchmark-glibc-musl-mimalloc" \
      org.opencontainers.image.version="${OCI_VERSION}" \
      org.opencontainers.image.revision="${OCI_REVISION}" \
      org.opencontainers.image.licenses="MIT OR Apache-2.0" \
      org.opencontainers.image.created="${OCI_CREATED}" \
      org.opencontainers.image.authors="Marc Carré"
```

```bash
# Verify with:
docker inspect alloc-bench:jemalloc-alpine \
  --format '{{json .Config.Labels}}' | jq .
```

Expected output: an object with all eight `org.opencontainers.image.*` keys populated. If any key is empty, the corresponding `--build-arg` was not passed by the Justfile recipe.

[VERIFIED: opencontainers/image-spec annotations.md] — All eight key names match the canonical OCI spec. The spec defines the full list of 13 keys (`url`, `documentation`, `vendor`, `base.digest`, `base.name` are also canonical); CONTEXT.md D-08 selects the eight most-load-bearing for this bench.

### §5: Justfile bench-all summary table emit

See Pattern 2 above. Key shape:

```
alloc env status ticks_per_s_p50
ptmalloc debian-slim ok 4523412.5
ptmalloc distroless-cc ok 4498211.1
ptmalloc wolfi ok 4501023.3
jemalloc debian-slim ok 5712341.8
…
mimalloc scratch ok 5689012.4
```

A single `jq` over each per-cell JSON pulls the `multithread` scenario's `metrics.ticks_per_s`. D-12 is satisfied.

### §6: bench-host recipe (macOS native, libmalloc baseline)

```just
# Native host build. Uses .cargo/config.toml's target-cpu=native automatically
# (no override). Detects host triple — usually Cargo picks correctly without
# --target. WS-03 banner already prints rustc/target/host/profile.
bench-host:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p results
    cargo build --release -p alloc-bench-cli
    target/release/alloc-bench-cli run-all \
        --output results/host-system.json --seed 7
    echo "[host] target=$(rustc -vV | awk '/^host:/ {print $2}')"
    echo "[host] wrote results/host-system.json"
```

`run-all` will populate the env block with `os: "macos"`, `docker_image: null` (no env var set), `target_triple: "aarch64-apple-darwin"` (or `x86_64-apple-darwin` on Intel Macs) automatically via `metrics::env::read_env` and `build_info::TARGET_TRIPLE` — no Justfile-side intervention needed [VERIFIED: crates/alloc-bench-core/src/metrics/env.rs].

### §7: .dockerignore (focused for Rust workspace + cargo-chef)

```
# Build artifacts (huge — most important entry)
target/
**/target/

# Git, planning, IDE
.git/
.gitignore
.planning/
.cargo-ok
.idea/
.vscode/
*.swp

# Documentation that doesn't affect the build
docs/
*.md
!CLAUDE.md

# Test/bench results (generated, not part of source)
results/
report/

# Misc
.DS_Store
**/*.rs.bk

# IMPORTANT: do NOT exclude — needed for cargo-chef + reproducible build
# Cargo.toml          ← workspace + per-crate manifests
# Cargo.lock          ← reproducible exact-version pins
# crates/             ← all source
# rust-toolchain.toml ← rustc version pin
# .cargo/config.toml  ← rustflags for native (Phase 1) — but the Dockerfile
#                       overrides via ENV RUSTFLAGS, so leaving it in is safe.
# build.rs            ← inside crates/alloc-bench-cli/, scoped via crates/
```

### §8: Dive install (host) and Dockerized fallback

```bash
# macOS / Linux preferred:
brew install dive

# Or via Docker (no install) — used in the dive-check recipe if dive is absent:
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v "$(pwd)/.dive-ci:/.dive-ci" \
  wagoodman/dive:latest \
  --ci alloc-bench:jemalloc-alpine --ci-config /.dive-ci
```

The Justfile `dive-check` recipe can detect host availability and fall back to the Dockerized form: `command -v dive && dive --ci … || docker run …`. [CITED: github.com/wagoodman/dive README]

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `RUN cargo build` in a single stage with no dep cache | cargo-chef 3-stage (chef → planner → builder) | cargo-chef 0.1.0 (~2020) — now 0.1.77 | 10× faster incremental Rust Docker builds; canonical pattern. |
| `RUN apt install jemalloc-dev` + LD_PRELOAD | `#[global_allocator]` Cargo features at compile time | Project-locked since init (PROJECT.md "no LD_PRELOAD") | Cleaner per-binary allocator selection; no runtime config. |
| Hand-rolled OCI labels with hard-coded values | LABEL + ARG + `--build-arg` from Justfile | OCI image-spec stable since 2017; widely adopted ~2020 | Build-time injection produces accurate `version`/`revision`/`created`. |
| `numactl --cpunodebind` inside container | `docker run --cpuset-cpus` | cgroup-v2 unified hierarchy ~2020 | No root needed; works on macOS Docker Desktop and Linux uniformly. |
| `rust:slim` builder + Debian runtime | Per-env runtime base + matching builder libc family | Distroless mainstream since ~2020 | 10–100× smaller images; better security posture. |

**Deprecated/outdated:**
- `jemallocator` crate (upstream-abandoned ~2022) — replaced by `tikv-jemallocator` 0.6.x [VERIFIED: tikv/jemallocator readme]
- `vergen` 1.x → 9.x — workspace uses hand-rolled build.rs (build.rs comment line 1 explains why) [VERIFIED]
- `dockerfile2llb` direct usage → BuildKit's built-in Dockerfile frontend (BuildKit 0.10+ default)

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `rust:1.83-bookworm` and `rust:1.83-alpine` tags will remain pulleable for the lifetime of v1 | Standard Stack | If Docker Hub removes the tag (rare for stable), every `docker buildx build` fails until plan-phase pins to a digest. Mitigation: capture digests in a follow-up if plan-phase wants extra reproducibility. |
| A2 | `jemalloc` and `mimalloc` static-link successfully against musl + crt-static for `scratch` and `distroless-static` | Pitfall §1 | D-01 already builds in an escape hatch (drop the cell, document in SUMMARY). Risk is low because PITFALLS §2.1, §2.2 say "recent versions handle this correctly" and the workspace's `default-features = false` config is already the documented safe configuration. **Smoke-test in plan-phase.** |
| A3 | `--cpuset-cpus=0-3` lands all four cores on the same NUMA node on Docker Desktop (macOS) and standard Linux servers | CONTEXT D-15 | On a multi-socket Linux server with non-contiguous core numbering (e.g., interleaved socket layout), cores 0-3 might span NUMA nodes. Mitigation: PITFALLS §1.3 explicitly says "single-NUMA-node measurement is the apples-to-apples comparison" and CI runners on `ubuntu-24.04` are single-socket — the assumption holds for the locked target. v2 cross-NUMA experiments handle the rest. |
| A4 | `dive --ci` returns non-zero exit on threshold violation in v0.13.1 | Standard Stack | Dive docs say "mark as failed" — assume = exit 1. If wrong, Justfile recipe needs to grep stdout. Verify in smoke test. |
| A5 | The four allocator crates (`tikv-jemallocator`, `tikv-jemalloc-ctl`, `mimalloc`, `libmimalloc-sys`) link cleanly with `lto = "fat" + codegen-units = 1 + panic = "unwind"` for both glibc and musl | Pitfall §1 | Phase 1 already verified glibc; PITFALLS §5.1 says "as of Rust 1.78+, both jemalloc and mimalloc work fine with `lto = "fat"`". musl is the new axis Phase 3 introduces. Smoke test in plan-phase covers this. |
| A6 | `mimalloc` 0.1.50 (Apr 2026) [VERIFIED: Cargo.lock] introduces no regression vs 0.1.43 (the version mentioned in CLAUDE.md / SUMMARY.md) for the default-features-off configuration the workspace uses | Standard Stack | Issue #1282 (apr 2026) on microsoft/mimalloc reports a 26→47 MB RSS regression v3.1.5→3.3.0 on musl/Alpine. **This is potentially load-bearing for the bench results** — if real, it will SHOW UP in our results as expected (the bench measures RSS) and we can document it. NOT a build-time risk. |
| A7 | `cgr.dev/chainguard/wolfi-base:latest` is glibc-based [VERIFIED: live ldd check 2026-05-19] | Standard Stack | If Chainguard switches Wolfi to musl in a future rebuild, the wolfi runtime stage will break (glibc dyn-linked binary on musl). Mitigation: Pitfall §6 already recommends pinning by digest. |

**Discuss-phase user confirmation needed:** A6 (mimalloc 0.1.43 → 0.1.50 drift since the SUMMARY.md was written) is the only user-facing surprise; everything else is locked by CONTEXT.md or verified live. Plan-phase should note in SUMMARY.md "mimalloc resolved to 0.1.50" so the report accurately credits the version under test.

## Open Questions

1. **Will `wolfi-base` provide enough OS context for the `metrics::env::read_os_version` `/proc/version` read?**
   - What we know: Wolfi runs on Linux kernel via Docker; `/proc/version` should be present; mounted by Docker per spec.
   - What's unclear: Whether Wolfi's kernel string is human-readable (it's the host kernel exposed inside the container).
   - Recommendation: Run smoke test; expect `Linux version 6.x.y …` content. Acceptable as-is.

2. **Should `bench-all` run the matrix in glibc-then-musl order or alpha order by tag?**
   - What we know: Sequential (D-11). 18 cells, ~2.4 h total at 65s × 11 scenarios × 18.
   - What's unclear: Whether grouping all glibc together amortizes the BuildKit cache better than alpha order.
   - Recommendation: Group by libc family (musl together, glibc together). cargo-chef's recipe.json hash is the same for any glibc target with same features — reordering glibc alpha doesn't help. The win is putting all musl after all glibc so the BuildKit can re-use the chef base layer for each family. Pattern 2 already orders this way (glibc first).

3. **Where does `OCI_CREATED` come from when `bench-all` runs over multiple minutes?**
   - What we know: `date -u +%Y-%m-%dT%H:%M:%SZ` returns the moment of recipe invocation. If the Justfile loops 18 cells, each cell stamps a different `OCI_CREATED`.
   - What's unclear: Whether this is a problem (it slightly differs across cells in the same matrix run).
   - Recommendation: Acceptable. `OCI_CREATED` is per-image; different cells legitimately have different build moments. If reproducibility matters, capture a single `BUILD_TS=$(date -u …)` at the top of the `bench-all` recipe and pass it through to every cell — but this is a minor refinement that plan-phase can decide.

4. **Does `cargo install --locked cargo-chef@0.1.77` work on `rust:1.83-alpine` (which has only ~6 MB of free space in the default partition)?**
   - What we know: cargo-chef compiles from source; needs ~50 MB of build space.
   - What's unclear: Whether `rust:1.83-alpine` has enough.
   - Recommendation: Smoke-test. If fails, install via `apk add cargo-chef` if available, or use `LukeMathWalker/cargo-chef` precompiled binary (their CI publishes one). Alternatively, use a heavier `rust:1.83-bookworm` builder for musl too — cargo can target musl from a glibc builder via `rustup target add x86_64-unknown-linux-musl + linker` setup. This is a fallback if D-06's "rust:1.83-alpine for musl" doesn't pan out.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Docker daemon (Linux container runtime) | All Dockerfile builds + run | ✓ | 29.4.0 (OrbStack, aarch64 host but supports linux/amd64 emulation) | — |
| Docker buildx | Multi-stage, ARG, --platform linux/amd64 | ✓ | 0.33.0 | — |
| Just | Justfile recipes | ✓ | 1.51.0 | — |
| dive | DOCK-07 image-efficiency CI gate | ✗ | — | Use `docker run wagoodman/dive:latest --ci` (Code Examples §8) |
| Cargo (host, for `bench-host`) | ORCH-02 native macOS run | ✓ | (workspace) | — |
| jq | Justfile summary table emit (Pattern 2) | ✓ (assumed; standard on macOS dev boxes — confirm in plan-phase) | — | If absent, replace with `python3 -c '…'` or print raw JSON |
| Bash 4+ (for shebang recipes with arrays) | Justfile recipes using `declare -a`, `<<<` heredocs | ✓ on macOS via `/bin/bash` 3.2 (sufficient for our use of `<<<`) and `/opt/homebrew/bin/bash` 5.x | bash 3.2 / 5.x | macOS's old bash 3.2 supports `<<<` and arrays — verified pattern works |
| Git | OCI_REVISION = `git rev-parse HEAD` | ✓ | (workspace) | — |
| date with `+%Y-%m-%dT%H:%M:%SZ` | OCI_CREATED ISO-8601 timestamp | ✓ (BSD date on macOS, GNU date on Linux — both support this format) | — | — |
| Internet (for first build) | Pulling base images, crates.io | required first time, then cached | — | Pre-pull base images: `docker pull rust:1.83-bookworm rust:1.83-alpine alpine:3.20 …` |

**Missing dependencies with no fallback:** none.

**Missing dependencies with fallback:** dive (Dockerized run is a clean fallback).

## Validation Architecture

> **`workflow.nyquist_validation`:** `false` in `.planning/config.json` [VERIFIED]. **Section omitted per spec.**

## Security Domain

> Phase 3 ships Docker images and a local Justfile orchestrator. No authentication, sessions, access control, or cryptography are introduced. The benchmark binary itself was scoped in Phase 1/2 and runs only on `127.0.0.1` (web scenario) with no TLS (PITFALLS §2.3 — HTTP-only is locked).

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V1 Architecture | yes (informational) | Defense-in-depth via container isolation + cpuset + memory limits (D-15) |
| V2 Authentication | no | No user authentication in scope |
| V3 Session Management | no | No sessions |
| V4 Access Control | no | Local-only; no multi-user surface |
| V5 Input Validation | yes (low) | Justfile validates `(env, alloc)` tuple before invoking docker; rejects unknown values |
| V6 Cryptography | no | No crypto |
| V14 Configuration | yes | OCI annotations (DOCK-08), pinned base images (D-05), pinned `RUST_VERSION` (D-06), no `:latest` except wolfi (which CONTEXT.md notes "or pinned digest at plan-phase time") |

### Known Threat Patterns for {stack}

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Supply-chain: registry tag drift | Tampering | Pin base images by exact tag (D-05); plan-phase optionally pins by digest |
| Supply-chain: hallucinated package install | Tampering | slopcheck verified all 5 Rust crates against crates.io ([OK] / Approved); Phase 3 adds NO new crates |
| Supply-chain: cargo-chef binary install | Tampering | Use `cargo install --locked cargo-chef@0.1.77` (no version drift); never `--unstable` or `--git` |
| Container privilege escalation | EoP | Run as nonroot where possible: distroless `:nonroot` UID 65532 [VERIFIED]; alpine + scratch + wolfi run as root by default — informational, not a leak surface for a local benchmark binary |
| Outbound network exfil from build | I/D | Local matrix; CI runs in GHA `ubuntu-24.04` sandbox. Bench binary opens 127.0.0.1 only |
| `:latest` mutability | Tampering / Repudiation | Pitfall §6 — pin `wolfi-base` by digest for v1 release |
| Build-arg secret leak in LABEL | I/D | OCI ARGs are public metadata: `OCI_REVISION` = git SHA (public), `OCI_CREATED` = timestamp (public), `OCI_VERSION` = Cargo crate version (public). No secrets injected via ARG |
| Volume permissions / nonroot write | Repudiation | Pitfall §4 — `chmod 0777 results/` (or `chown 65532` on Linux) before mount |

## Sources

### Primary (HIGH confidence)
- **CONTEXT.md** — `.planning/phases/03-docker-matrix-local-orchestration/03-CONTEXT.md` (23 user decisions D-01..D-23)
- **REQUIREMENTS.md** — Phase 3 reqs DOCK-01..09, ORCH-01, ORCH-02
- **PITFALLS.md §1.3, §1.5, §2.1, §2.2, §2.3, §2.4, §3.1, §3.2, §3.3, §5.4** — known gotchas mapped to Phase 3
- **STACK.md §9** — cargo-chef pattern; **§10** — cross-compile to musl; **§11** — Justfile cross-product
- **ARCHITECTURE.md** — workspace layout, Cross-compilation strategy, Justfile matrix
- **Cargo.lock** — verified resolved versions: tikv-jemallocator 0.6.1, tikv-jemalloc-ctl 0.6.1, mimalloc 0.1.50, libmimalloc-sys 0.1.47
- **Live `docker buildx imagetools inspect`** runs (2026-05-19):
  - `rust:1.83-bookworm` — exists, multi-arch (amd64/arm64/etc)
  - `rust:1.83-alpine` — exists, multi-arch
  - `alpine:3.20` — verified version 3.20.10, musl libc
  - `debian:bookworm-slim` — verified version 12.13, glibc
  - `gcr.io/distroless/cc-debian12:nonroot` — verified UID 65532
  - `gcr.io/distroless/static-debian12:nonroot` — verified UID 65532, WorkingDir=/home/nonroot
  - `cgr.dev/chainguard/wolfi-base:latest` — verified UID 0, glibc (`/lib/ld-linux-x86-64.so.2`)
- **slopcheck install --ecosystem crates.io …** (2026-05-19): all 5 referenced Rust crates returned `[OK]`
- **`docker info`** — confirmed dev daemon is OrbStack on aarch64 host (informs `--platform linux/amd64` requirement)
- **OCI image-spec annotations** — github.com/opencontainers/image-spec/blob/main/annotations.md (8 keys verified)
- **dive 0.13.1 README** — `.dive-ci` config schema verified (lowestEfficiency, highestUserWastedPercent, highestWastedBytes)
- **cargo-chef 0.1.77 README** — multi-stage pattern + `--target` for cross-compile verified
- **just.systems/man/en** — recipe-parameters, conditional-expressions, shebang-recipes verified

### Secondary (MEDIUM confidence)
- **github.com/tikv/jemallocator/blob/main/jemallocator/Cargo.toml** — feature list verified; `unprefixed_malloc_on_supported_platforms` exists but NOT in defaults
- **github.com/purpleprotocol/mimalloc_rust** — current crate version 0.1.50 (April 2026); features default/secure/extended/override + v2/debug
- **github.com/microsoft/mimalloc** — upstream v3.3.2 (April 2026); musl flag `MI_LIBC_MUSL` documented
- **github.com/clux/muslrust Dockerfile** — confirms it does NOT bundle jemalloc/mimalloc; we don't need it
- **Issue #1282 microsoft/mimalloc** — RSS regression 26→47MB on Alpine v3.1.5→v3.3.0 (informational; not a Phase 3 blocker)

### Tertiary (LOW confidence — none load-bearing)
- WebSearch / training-knowledge for "best practices" generalities — superseded by primary sources at every step

## Metadata

**Confidence breakdown:**
- Standard stack: **HIGH** — every crate version pinned in Cargo.lock; every base image verified live via docker buildx imagetools inspect; all CONTEXT.md decisions read directly from the locked file
- Architecture (cargo-chef + per-env Dockerfile + Justfile loop): **HIGH** — STACK.md §9-11 + ARCHITECTURE.md provide the canonical pattern, cross-checked against cargo-chef README and just.systems docs
- Pitfalls: **HIGH for already-documented (PITFALLS.md §1-5); MEDIUM for new ones surfaced in this research (Pitfall §3 Apple Silicon platform default, §4 distroless nonroot UID, §6 wolfi `:latest` mutability)** — these were discovered through live verification (docker info, image inspect)
- Justfile recipe details (positional args, shebang `#!/usr/bin/env bash`, conditional case statements): **HIGH** — verified against just.systems documentation pages
- OCI annotations (8 keys, format requirements): **HIGH** — verified against opencontainers/image-spec (live fetch)
- mimalloc / jemalloc musl static-link success rate: **MEDIUM** — A2 in Assumptions Log; PITFALLS §2 already plans an escape hatch; smoke test will confirm

**Research date:** 2026-05-19
**Valid until:** ~2026-06-19 (30-day window — base image digests + cargo-chef version + crates.io pins are stable; only `wolfi:latest` mutates daily, which Pitfall §6 already addresses)

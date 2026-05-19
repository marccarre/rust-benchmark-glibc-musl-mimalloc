# Phase 3: Docker Matrix & Local Orchestration - Context

**Gathered:** 2026-05-19
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous)

<domain>
## Phase Boundary

Deliver the Docker matrix (six runtime images × the meaningful allocator combinations) plus the local Justfile orchestration that wires the Phase 1/2 binary into reproducible per-cell runs with NUMA + cgroup discipline. Concretely, the phase ships:

1. Six Dockerfiles (one per runtime env) using cargo-chef multi-stage builds, accepting `ARG ALLOC={ptmalloc|jemalloc|mimalloc|mallocng}` to select the Cargo feature.
2. A 12-cell allocator-runtime matrix (3 glibc envs × 3 glibc allocs + 3 musl envs × 3 musl allocs).
3. Justfile recipes: `just build {env} {alloc}`, `just run {env} {alloc}`, `just bench-all`, `just bench-host`, `just bench-all-smoke`, `just clean-images`, `just dive-check {env} {alloc}`.
4. NUMA + cgroup defaults baked into the run recipe (`--cpus=4 --memory=4g --cpuset-cpus=0-3`).
5. macOS host baseline (`bench-host` → `results/host-system.json`) — system libmalloc only.
6. Image-size verification via `dive --ci` against every image in the matrix.

Phase 3 does NOT add: the aggregator (Phase 4), CI matrix runs (Phase 5), or the public README walkthrough (Phase 5). It also does NOT modify the Phase 1 harness contract or Phase 2 scenarios.

</domain>

<decisions>
## Implementation Decisions

### Matrix cell selection & tagging
- **D-01:** **12-cell meaningful matrix** — physically possible (env × alloc) combinations only:
  - **glibc envs × glibc allocs (9 cells):** `debian-slim`, `distroless-cc`, `wolfi` × `ptmalloc`, `jemalloc`, `mimalloc`
  - **musl envs × musl allocs (9 cells nominally, kept down to 9 cells across 3 envs):** `alpine`, `distroless-static`, `scratch` × `mallocng`, `jemalloc`, `mimalloc`
  - **Total 18 nominally; if `jemalloc-on-distroless-static` or `mimalloc-on-scratch` static-link smoke fails, drop those cells with documented reason in CONTEXT/SUMMARY.** Plan-phase to first build the smoke matrix (1 alloc per env) and confirm linkage before committing to 18.
- **D-02:** Image tag: **`alloc-bench:{alloc}-{env}`** (e.g., `alloc-bench:jemalloc-alpine`). Matches success criterion 1 verbatim. `{alloc}` first because it is the more interesting axis.
- **D-03:** Results filename: **`results/{alloc}-{env}.json`** (flat layout). Easy to glob in Phase 4 (`results/*.json`); allocator-first sort groups same-alloc-different-env cells visually.
- **D-04:** **Hard-skip cross-libc combos.** mallocng is the libc allocator of musl — running it on a glibc env is physically impossible. ptmalloc is the libc allocator of glibc — running it on a musl env is physically impossible. The Justfile rejects these with a clear error.

### Base images & build strategy
- **D-05:** **Pinned base images:**
  - `alpine:3.20` (matches success criterion 2's literal `docker_image`)
  - `debian:bookworm-slim`
  - `gcr.io/distroless/cc-debian12:nonroot`
  - `gcr.io/distroless/static-debian12:nonroot`
  - `cgr.dev/chainguard/wolfi-base:latest` *(or pinned digest at plan-phase time)*
  - `scratch` (no tag)
- **D-06:** **Builder bases:** `rust:1.83-bookworm` for glibc family targets (debian-slim, distroless-cc, wolfi); `rust:1.83-alpine` for musl family targets (alpine, distroless-static, scratch). Pin RUST_VERSION via `ARG RUST_VERSION=1.83` (PITFALLS §5.4).
- **D-07:** **Six Dockerfiles**, one per runtime env, parameterized by `ARG ALLOC=ptmalloc`. Each uses **cargo-chef** multi-stage caching (CLAUDE.md §9). Each Dockerfile produces six logical cells via `--build-arg ALLOC=...`. Plan-phase keeps Dockerfiles ≤ ~80 lines.
- **D-08:** **OCI annotations** required on every image (DOCK-08): `org.opencontainers.image.{title, description, source, version, revision, licenses, created, authors}`. `version` = `CARGO_PKG_VERSION`; `revision` = full git SHA; `created` = ISO-8601 `BUILDKIT_INLINE_CACHE`-friendly timestamp. Plan-phase decides whether to inject via `LABEL` lines (simple) or `--label` flags + Docker BuildKit `--metadata-file` (more verbose but cleaner). Default to `LABEL` lines parameterized by Dockerfile `ARG`s.
- **D-09:** **Build flags injected via `ENV RUSTFLAGS`** in the builder stage: `-C target-cpu=x86-64-v3` for portability across CI runners (PITFALLS §3.3). The host-only `bench-host` recipe uses `-C target-cpu=native`.

### Justfile recipe surface
- **D-10:** **Positional-arg recipes:** `just build {env} {alloc}`, `just run {env} {alloc}`, `just bench-cell {env} {alloc}`. Matches the literal commands in success criterion 1. Cap at 2 positional args; further customization via `--memory=...`, `--cpus=...` env vars in run recipe.
- **D-11:** **`just bench-all`** runs the full 12+/18-cell matrix **sequentially**. Per-cell logs streamed with `[{alloc}-{env}]` prefix; per-cell JSON written immediately to `results/{alloc}-{env}.json`. Sequential is mandatory: parallel cells would multiplex allocators in the same kernel page cache and pollute measurements (PITFALLS §1.3 spirit).
- **D-12:** **No aggregation in Phase 3** — `bench-all` ends with a stdout summary table (`alloc, env, status, ticks_per_s_p50`); the actual aggregation/HTML/REPORT is Phase 4.
- **D-13:** **`just bench-all-smoke`** runs the matrix with `--warmup 1s --duration 5s` per scenario for fast dev iteration (~10 min vs. ~2.5h). Mandatory for plan-phase exit verification.
- **D-14:** **`just clean-images`** removes all `alloc-bench:*` tags. **`just dive-check {env} {alloc}`** wraps `dive --ci` for one cell; **`just dive-check-all`** runs it for every image in the matrix. CI gating in Phase 5 reuses `just dive-check-all`.

### Runtime defaults (cgroup / NUMA)
- **D-15:** **Default `docker run` flags** baked into `just run`: `--cpus=4 --memory=4g --cpuset-cpus=0-3 --rm -v $(pwd)/results:/out` plus the OCI-image-default entrypoint. Matches success criterion 2 verbatim. 4 GiB ≥ mimalloc's 64 MiB segment pre-allocation by 64× (PITFALLS §3.1); cpuset pins to first 4 cores → single NUMA node on most servers (PITFALLS §1.3).
- **D-16:** **NUMA pinning via `--cpuset-cpus` only.** Do NOT install `numactl` inside images. Reasoning: keeps images minimal; cpuset is honored on Docker Desktop (macOS) and Linux uniformly; `--membind` requires root in the container which conflicts with `nonroot` distroless variants.
- **D-17:** **Override knobs** via env vars in the run recipe: `BENCH_MEMORY=8g just run alpine jemalloc` overrides `--memory`; `BENCH_CPUSET=4-7 just run debian-slim mimalloc` for second-NUMA-node experiments. Defaults stay at the locked values.

### macOS bench-host
- **D-18:** **`just bench-host`** builds and runs the bench natively on macOS using the system allocator (libmalloc) only — single cell, output `results/host-system.json`. Build flags: `-C target-cpu=native`, target = host (`aarch64-apple-darwin` or `x86_64-apple-darwin`). The env block records `docker_image: null`, `target_triple: <host>`, `os: "macos"`. Documented as a baseline, not a comparison axis (PITFALLS §3.2).
- **D-19:** **macOS does NOT run jemalloc/mimalloc.** Reasoning: macOS-native jemalloc/mimalloc binaries would require macOS-specific Cargo build paths (different feature combinations than Linux) and the result is still not 1:1 comparable to Linux because Apple's Mach-O dynamic loader and malloc zones don't map onto the Linux glibc/musl model. Keep macOS purely as the libmalloc dev-box reference.

### Bench durations
- **D-20:** **Phase-1 defaults preserved for matrix runs:** `--warmup 5s --duration 60s` per scenario (PITFALLS §1.5, §1.4). 11 scenarios × 12 cells × 65s ≈ 2.4 h for `bench-all`. Acceptable for a once-a-day full matrix; smoke recipe (D-13) covers fast dev loops.

### Image size & dive
- **D-21:** **`dive --ci` thresholds** baked into a `.dive-ci` config at repo root: `lowestEfficiency: 0.95`, `highestUserWastedPercent: 0.05` (5% wasted bytes), `highestWastedBytes: 50MB`. Plan-phase tunes if any cell legitimately exceeds (e.g., distroless-cc may be tighter than alpine).
- **D-22:** **Image-size budget** documented per env (informational, not enforced in Phase 3 — Phase 5 CI enforces): `scratch ≤ 15MB`, `distroless-static ≤ 25MB`, `alpine ≤ 30MB`, `wolfi ≤ 35MB`, `distroless-cc ≤ 50MB`, `debian-slim ≤ 100MB`. Plan-phase records actual sizes in SUMMARY.md after build.

### Schema additions (none)
- **D-23:** **No results.json schema changes.** Phase 1 D-11 locked `schema_version: 1`. Phase 3 just populates the existing env block (`docker_image`, `cpu_model`, `cpu_count`, `kernel_version`, `target_triple`) — all fields already exist in the schema.

### Claude's Discretion
- Exact ordering of build stages within each Dockerfile (cargo-chef recipe → cook → build → minimal runtime). Plan-phase decides.
- Whether `just bench-all` uses `set -euo pipefail`-style strict mode or a loop with explicit per-cell error capture. Strict mode is recommended; per-cell error capture allows continuing past a single cell failure.
- Whether to add a separate `Dockerfile.builder` shared across runtime stages or duplicate the builder stage in each runtime Dockerfile. Single-builder is cleaner; duplication is more cargo-chef-friendly. Plan-phase chooses.
- Whether `just run` mounts `./results` read-write or only writes via stdout redirection. Mount is more ergonomic; stdout matches Phase 1 D-24 idiom. Default to mount.
- How the macOS host bench builds for x86 vs. aarch64 (host detection in Justfile). Plan-phase decides.

</decisions>

<canonical_refs>
## Canonical References

### Phase context
- `.planning/PROJECT.md` — overall project context, locked decisions
- `.planning/REQUIREMENTS.md` — Phase 3 requirements: DOCK-01..09, ORCH-01, ORCH-02 (11 reqs)
- `.planning/ROADMAP.md` §"Phase 3: Docker Matrix & Local Orchestration" — phase goal + success criteria
- `.planning/phases/01-foundation-mvp-slice/01-CONTEXT.md` — workspace shape, profile flags, build metadata
- `.planning/phases/02-scenario-fan-out/02-CONTEXT.md` — scenarios available for `run-all`, schema lock

### Research outputs (MANDATORY reading for plan-phase)
- `.planning/research/SUMMARY.md` — synthesis
- `.planning/research/STACK.md` §9 — Docker multi-stage with cargo-chef
- `.planning/research/STACK.md` §10 — cross-compilation to musl on macOS (informational; macOS host bench is libmalloc only)
- `.planning/research/STACK.md` §11 — Justfile cross-product matrix
- `.planning/research/PITFALLS.md` §1.3 — NUMA effects (cpuset pinning)
- `.planning/research/PITFALLS.md` §1.5 — warm-up duration
- `.planning/research/PITFALLS.md` §2.1 — jemalloc on musl (`unprefixed_malloc_on_supported_platforms` off)
- `.planning/research/PITFALLS.md` §2.2 — mimalloc on musl (`default-features = false`)
- `.planning/research/PITFALLS.md` §2.3 — static linking for scratch (`+crt-static`)
- `.planning/research/PITFALLS.md` §2.4 — distroless cc vs static
- `.planning/research/PITFALLS.md` §3.1 — cgroup memory ≥ 4 GiB for mimalloc
- `.planning/research/PITFALLS.md` §3.2 — macOS Docker is not 1:1 vs Linux host
- `.planning/research/PITFALLS.md` §3.3 — `target-cpu=x86-64-v3` for Docker
- `.planning/research/PITFALLS.md` §5.4 — pin rustc version in Dockerfile ARG

### External specifications
- https://github.com/LukeMathWalker/cargo-chef — cargo-chef multi-stage caching
- https://github.com/wagoodman/dive — dive CI gate
- https://specs.opencontainers.org/image-spec/annotations/ — OCI annotation keys
- https://hub.docker.com/_/alpine — alpine:3.20
- https://hub.docker.com/_/debian — debian:bookworm-slim
- https://github.com/GoogleContainerTools/distroless — distroless/cc and distroless/static
- https://github.com/chainguard-images/wolfi-base — wolfi-base

### Out-of-scope (Phase 3)
- Aggregator/HTML dashboard → Phase 4
- GitHub Actions matrix → Phase 5
- Image-size CI gate enforcement → Phase 5 (Phase 3 ships `.dive-ci` config + `just dive-check-all`; Phase 5 wires it to GHA)
- README "Run it yourself" walkthrough → Phase 5 (REPR-01)
- Multiple runs per cell w/ median + range → Phase 5 (REPR-03)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets (from Phases 1 + 2)
- `crates/alloc-bench-cli` produces a single binary `alloc-bench-cli` whose Cargo features (`alloc-jemalloc`, `alloc-mimalloc`) select the global allocator. Phase 3 just composes Dockerfiles around the existing build.
- `crates/alloc-bench-core::output::Run` schema is locked at v1 (Phase 1 D-11). Env block fields `docker_image`, `cpu_model`, `cpu_count`, `kernel_version`, `target_triple`, `os` are already present.
- `crates/alloc-bench-cli` `run-all` subcommand (Phase 2 SCEN-11) emits a JSON array of Run records with per-scenario `status: "success" | "failed"` — exactly the per-cell artifact `bench-all` needs.
- `justfile` already exists with `dce-check` and `run-all-smoke` recipes. Phase 3 extends it; does not rewrite it.
- `scripts/dce_check.sh` — example shell-out pattern for the new dive-check / build / run scripts.
- `prek.toml` pre-commit hooks. Plan-phase adds Dockerfile / shell linting if not already present.

### Established Patterns (from Phases 1 + 2)
- Conventional-commit prefixes (`docs:`, `feat:`, `chore:`); Phase 3 will use `feat(03):`, `chore(03):`, `docs(03):`.
- Workspace deps in root `Cargo.toml`. Phase 3 does not change Cargo deps (no new Rust crates expected — dive and docker are tools, not deps).
- `commit_docs = true` — `.planning/` artifacts are committed.
- `cargo fmt` + `cargo clippy --all-targets` before each commit (matches prek.toml).

### Integration Points
- **Each Dockerfile** copies the workspace into a builder stage, runs `cargo build --release --features alloc-${ALLOC}` (mapping `ALLOC=ptmalloc → no feature flag, system default`), and copies the resulting `target/release/alloc-bench-cli` into the runtime stage.
- **Each `just bench-cell` invocation** runs `docker run ... alloc-bench:{alloc}-{env} run-all --output /out/{alloc}-{env}.json` and writes to `./results/`.
- **`just bench-host`** runs `cargo run --release --bin alloc-bench-cli -- run-all --output results/host-system.json` natively (no Docker).
- **`just dive-check`** wraps `dive --ci alloc-bench:{alloc}-{env} --ci-config .dive-ci`.
- **Phase 4 aggregator** (next phase) will glob `results/*.json` — Phase 3's flat layout is the contract.

### New non-Cargo files added in Phase 3
- `docker/alpine.Dockerfile`
- `docker/debian-slim.Dockerfile`
- `docker/distroless-cc.Dockerfile`
- `docker/distroless-static.Dockerfile`
- `docker/scratch.Dockerfile`
- `docker/wolfi.Dockerfile`
- `.dive-ci` (image-efficiency config)
- `scripts/build_image.sh`, `scripts/run_cell.sh`, `scripts/bench_all.sh` *(or inline in Justfile if short enough — plan-phase decides)*
- `.dockerignore` (excludes `target/`, `.git/`, `.planning/` for fast COPY)
- `justfile` additions (recipes D-10..D-14)

</code_context>

<specifics>
## Specific Ideas

- **Image tag literal in success criterion 1:** the test for criterion 1 is `docker inspect alloc-bench:jemalloc-alpine` showing the OCI annotation set. Plan-phase must verify the literal label keys, not paraphrases.
- **Success criterion 2's `target_triple: "x86_64-unknown-linux-musl"`** is the literal expected value for alpine; the `bench_info` block must produce that exact string. Build flag must be `--target x86_64-unknown-linux-musl`.
- **Hard requirement: `bench-all` must skip cross-libc combos automatically** — no manual filtering. The Justfile loop iterates only over physically valid (env, alloc) tuples.
- **`results/host-system.json`** is the literal filename in success criterion 4 — bench-host writes exactly this path.
- **`.dockerignore` is mandatory** to keep build context small. Without it, every COPY scoops 500 MB of `target/`.
- **PITFALLS §2.1 jemalloc-on-musl:** plan-phase should add a smoke build for `jemalloc-alpine` *before* committing to all 18 cells; if linking fails, drop to glibc-only jemalloc and document it.
- **mimalloc default-features = false on musl** is already set in workspace `Cargo.toml` — no change needed.

</specifics>

<deferred>
## Deferred Ideas

- **Image-size CI enforcement** → Phase 5 (Phase 3 ships `.dive-ci` config and `just dive-check-all`; Phase 5 wires GHA to fail on dive thresholds)
- **Multiple runs per cell with median + min/max range** → Phase 5 (REPR-03)
- **Aggregator** → Phase 4 (Phase 3 emits flat JSON files matching the contract)
- **README "Run it yourself" walkthrough** → Phase 5 (REPR-01)
- **Cross-NUMA-node experiments** → v2 (PITFALLS §1.3 leaves single-NUMA-node measurement as the canonical case)
- **macOS-native jemalloc/mimalloc** → v2 (D-19 — macOS host is libmalloc-only baseline)
- **`numactl --membind` inside images** → not planned (D-16 — `--cpuset-cpus` is sufficient)
- **Per-architecture matrix (aarch64 in addition to x86_64)** → v2 (REQUIREMENTS V2-09)
- **`snmalloc` / `tcmalloc` / `rpmalloc`** → v2 (REQUIREMENTS V2-01..03)

</deferred>

---

*Phase: 3-Docker Matrix & Local Orchestration*
*Context gathered: 2026-05-19*

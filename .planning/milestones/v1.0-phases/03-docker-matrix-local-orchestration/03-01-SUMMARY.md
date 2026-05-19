---
phase: 03-docker-matrix-local-orchestration
plan: "01"
subsystem: infra
tags: [docker, glibc, cargo-chef, oci-annotations, dive, dockerignore, debian-slim, distroless-cc, wolfi]

# Dependency graph
requires:
  - phase: 01-foundation-mvp-slice
    provides: alloc-bench-cli binary + workspace + rust-toolchain.toml
  - phase: 02-scenario-fan-out
    provides: run-all subcommand + Run schema v1 + metrics::env::read_env (DOCKER_IMAGE)
provides:
  - docker/debian-slim.Dockerfile — glibc dynamic on debian:bookworm-slim
  - docker/distroless-cc.Dockerfile — glibc minimal on distroless/cc-debian12:nonroot (UID 65532)
  - docker/wolfi.Dockerfile — glibc Chainguard wolfi-base, digest-pinned (manifest-list)
  - .dockerignore — keeps Docker build context small (excludes target/, .planning/)
  - .dive-ci — three D-21 thresholds (lowestEfficiency=0.95, highestUserWastedPercent=0.05, highestWastedBytes=50MB)
affects:
  - 03-02 (musl Dockerfiles — alpine, distroless-static, scratch — share the cargo-chef builder pattern)
  - 03-03 (Justfile build/run/bench-cell recipes consume these Dockerfiles via --build-arg)
  - 03-04 (smoke matrix builds the 6 anchor cells: 3 glibc envs × 1 alloc anchor each, plus 3 musl envs)
  - 03-05 (dive-check-all runs against every alloc-bench:* image)

# Tech tracking
tech-stack:
  added:
    - cargo-chef@0.1.77 (installed inside builder stage via `cargo install --locked`)
    - rust:1.91-bookworm (Docker builder base — matches rust-toolchain.toml channel)
    - debian:bookworm-slim (runtime base, glibc dynamic)
    - gcr.io/distroless/cc-debian12:nonroot (runtime base, glibc minimal nonroot UID 65532)
    - cgr.dev/chainguard/wolfi-base@sha256:0cff4df2... (runtime base, glibc Chainguard, manifest-list digest pinned)
  patterns:
    - cargo-chef 3-stage build (chef → planner → builder) per Dockerfile; cross-Dockerfile dedup via BuildKit layer cache
    - ENV RUSTFLAGS="-C target-cpu=x86-64-v3" overrides .cargo/config.toml's target-cpu=native (Pitfall §2)
    - ALLOC ∈ {ptmalloc, jemalloc, mimalloc} → Cargo features mapped via if/elif/else; mallocng hard-rejected at build time (D-04)
    - Eight org.opencontainers.image.* LABELs populated from --build-arg OCI_VERSION/OCI_REVISION/OCI_CREATED (D-08)
    - ENV DOCKER_IMAGE set in every runtime stage so metrics::env::read_env populates JSON env block
    - Manifest-list digest pinning (not per-arch) for clean `docker buildx build --check --platform linux/amd64` on arm64 hosts

key-files:
  created:
    - .dockerignore
    - .dive-ci
    - docker/debian-slim.Dockerfile (69 lines)
    - docker/distroless-cc.Dockerfile (73 lines)
    - docker/wolfi.Dockerfile (75 lines)
  modified: []

key-decisions:
  - "RUST_VERSION=1.91 (not D-06's literal 1.83) because rust-toolchain.toml channel = 1.91; matching saves a redundant rustup download inside the builder."
  - "Pin wolfi-base by manifest-list (OCI image-index) digest sha256:0cff4df2... rather than per-arch amd64 manifest, so `docker buildx build --check --platform linux/amd64` exits 0 cleanly on arm64 hosts (Apple Silicon / OrbStack) per RESEARCH §Pitfall 3."
  - "Hard-reject ALLOC=mallocng (and any other unknown value) in BOTH cargo chef cook and cargo build steps with a non-zero exit and clear stderr message — D-04 cross-libc rejection is structural, not silent fallthrough."
  - "distroless-cc places binary at /alloc-bench-cli (FS root), not /usr/local/bin/alloc-bench-cli, because nonroot users have no guaranteed PATH entry for /usr/local/bin (Pitfall §4)."

patterns-established:
  - "Three-stage cargo-chef pattern duplicated across all glibc Dockerfiles. BuildKit dedups the chef + planner + builder layers across `docker buildx build` invocations sharing the same workspace context — duplication is structural-clarity, not work cost."
  - "OCI annotations injected via ARG OCI_{VERSION,REVISION,CREATED} + LABEL block. Justfile recipe (Plan 03-03) populates these from Cargo.toml + git rev-parse + date."
  - "`docker buildx build --check --platform linux/amd64` is the lint gate for Dockerfile authoring on arm64 dev hosts — guards against InvalidBaseImagePlatform false positives by forcing the build platform explicitly."

requirements-completed: [DOCK-02, DOCK-03, DOCK-06, DOCK-07, DOCK-08]

# Metrics
duration: ~10min
completed: 2026-05-18
---

# Phase 3 Plan 01: Glibc Dockerfiles + dive/dockerignore Summary

**Three glibc-family Dockerfiles (debian-slim, distroless-cc, wolfi) sharing a cargo-chef 3-stage builder, plus `.dockerignore` and `.dive-ci` — all linted clean via `docker buildx build --check --platform linux/amd64`.**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-05-19 (per plan)
- **Completed:** 2026-05-18T21:49:10Z (UTC; matches host clock skew)
- **Tasks:** 3 (all auto-mode, no checkpoints)
- **Files created:** 5 (.dockerignore, .dive-ci, 3 Dockerfiles)
- **Files modified:** 0

## Accomplishments

- Three glibc Dockerfiles parameterized by `ARG ALLOC ∈ {ptmalloc, jemalloc, mimalloc}` with hard-reject of mallocng (cross-libc).
- All three pass `docker buildx build --check --platform linux/amd64` cleanly (exit 0, no warnings).
- Each ≤ 80 lines (69 / 73 / 75) per CONTEXT D-07 cap.
- Wolfi-base pinned by manifest-list digest captured live 2026-05-19 — reproducible across daily Chainguard rebuilds (Pitfall §6).
- `.dockerignore` excludes target/, .planning/, results/, report/ + IDE artifacts; keeps Cargo.toml, Cargo.lock, crates/, rust-toolchain.toml, .cargo/config.toml in context.
- `.dive-ci` carries the three D-21 thresholds for image-efficiency CI gating in Plan 03-05.

## Task Commits

Each task was committed atomically:

1. **Task 1: Create .dockerignore + .dive-ci** — `d557d58` (chore)
2. **Task 2: Create docker/debian-slim.Dockerfile + docker/distroless-cc.Dockerfile** — `8e6d79b` (feat)
3. **Task 3: Capture wolfi-base digest and create docker/wolfi.Dockerfile** — `60dd60c` (feat)

**Plan metadata:** (this commit) — docs(03-01): plan summary

## Files Created/Modified

- `.dockerignore` — Excludes target/, .planning/, results/, report/, IDE dirs from Docker build context. Preserves Cargo.toml, Cargo.lock, crates/, rust-toolchain.toml.
- `.dive-ci` — Three D-21 thresholds: lowestEfficiency=0.95, highestUserWastedPercent=0.05, highestWastedBytes=50MB.
- `docker/debian-slim.Dockerfile` — 69 lines. glibc dynamic on debian:bookworm-slim. Binary at /usr/local/bin/alloc-bench-cli, runs as root.
- `docker/distroless-cc.Dockerfile` — 73 lines. glibc minimal on gcr.io/distroless/cc-debian12:nonroot. Binary at /alloc-bench-cli (FS root, Pitfall §4), USER nonroot, WORKDIR /home/nonroot.
- `docker/wolfi.Dockerfile` — 75 lines. glibc Chainguard runtime on cgr.dev/chainguard/wolfi-base@sha256:0cff4df2.... Binary at /usr/local/bin/alloc-bench-cli, runs as UID 0 (Wolfi default).

## Decisions Made

- **Manifest-list digest for wolfi-base** (not per-arch amd64 digest): the per-arch amd64 manifest digest works at runtime but trips `docker buildx build --check` on arm64 hosts with `InvalidBaseImagePlatform`. Pinning the OCI image-index digest (`sha256:0cff4df29a6597173dc8b813787318150141eb96ac783dc3ff4f5ff52c49a1e2`) keeps the pin deterministic AND lets `--platform linux/amd64` resolve the right per-arch layer at build time.
- **RUST_VERSION=1.91**: matches rust-toolchain.toml channel. CONTEXT D-06 originally cited 1.83; the toolchain has since been bumped to 1.91, and matching the Docker builder base image saves rustup from downloading a second toolchain inside the builder stage.
- **ALLOC reject path duplicated** in both `cargo chef cook` and `cargo build` if/elif/else chains: any value outside {ptmalloc, jemalloc, mimalloc} hard-fails with `exit 1` and a clear stderr message. D-04 cross-libc rejection (mallocng on glibc) is structural, not silent fallthrough.
- **distroless-cc binary path at /alloc-bench-cli** (not /usr/local/bin/alloc-bench-cli): nonroot users in distroless `:nonroot` images do not have a guaranteed PATH entry for /usr/local/bin (RESEARCH §Pitfall 4).

## Wolfi-base digest captured

```
docker buildx imagetools inspect cgr.dev/chainguard/wolfi-base@<floating tag> | grep '^Digest:'
→ sha256:0cff4df29a6597173dc8b813787318150141eb96ac783dc3ff4f5ff52c49a1e2
```

Captured 2026-05-19. Multi-arch OCI image-index. Refresh per RESEARCH §Pitfall 6 if rebuilding from scratch and the wolfi runtime stops linking — but never use the floating tag.

## DOCKER_IMAGE env values per runtime

| Dockerfile | ENV DOCKER_IMAGE value |
|------------|------------------------|
| docker/debian-slim.Dockerfile | `debian:bookworm-slim` |
| docker/distroless-cc.Dockerfile | `gcr.io/distroless/cc-debian12:nonroot` |
| docker/wolfi.Dockerfile | `cgr.dev/chainguard/wolfi-base@sha256:0cff4df29a6597173dc8b813787318150141eb96ac783dc3ff4f5ff52c49a1e2` |

These are picked up by `metrics::env::read_env` (Phase 1) and surfaced in each Run record's `env.docker_image` field — required for results.json env-block accuracy (success criterion 2 of Phase 3).

## docker buildx build --check output (passing)

All three Dockerfiles linted with `--platform linux/amd64` and the same `--build-arg` set:

```
ALLOC=ptmalloc TARGET=x86_64-unknown-linux-gnu \
OCI_VERSION=0.1.0 OCI_REVISION=test OCI_CREATED=2026-05-19T00:00:00Z
```

| Dockerfile | Exit code | Output |
|------------|-----------|--------|
| docker/debian-slim.Dockerfile | 0 | `Check complete, no warnings found.` |
| docker/distroless-cc.Dockerfile | 0 | `Check complete, no warnings found.` |
| docker/wolfi.Dockerfile | 0 | `Check complete, no warnings found.` |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] wolfi-base digest pin tripped `docker buildx build --check` on arm64 host**

- **Found during:** Task 3 (lint gate)
- **Issue:** The Task 3 instructions say "capture the amd64 manifest digest via `docker buildx imagetools inspect ... --raw | jq '.manifests[] | select(.platform.architecture=="amd64") | .digest'`". That digest (`sha256:353c31c9d3...`) is a single-platform OCI image manifest. Pinning to it makes `docker buildx build --check` warn `InvalidBaseImagePlatform: pulled with platform "linux/amd64", expected "linux/arm64"` on arm64 hosts (Apple Silicon / OrbStack — confirmed via `docker info` showing aarch64 host arch). The check exits 1, failing the acceptance criterion "exits 0".
- **Fix:** Pin the **manifest-list (OCI image-index) digest** (`sha256:0cff4df29a6597173dc8b813787318150141eb96ac783dc3ff4f5ff52c49a1e2`) captured via `docker buildx imagetools inspect cgr.dev/chainguard/wolfi-base@<floating tag>` (no `--raw`, just the top-level `Digest:` line). This lets `--platform linux/amd64` resolve to the right per-arch layer at build time and `--check` exits 0 cleanly.
- **Files modified:** docker/wolfi.Dockerfile (commit 60dd60c, single FROM line + ENV DOCKER_IMAGE + comment block update)
- **Verification:** `docker buildx build --check --platform linux/amd64 -f docker/wolfi.Dockerfile ... .` → exit 0, "Check complete, no warnings found."
- **Committed in:** `60dd60c` (Task 3 commit)
- **Rationale recorded in Dockerfile comment:** "We pin the manifest-list digest (not the per-arch amd64 manifest) so `--platform linux/amd64` resolves cleanly at build time without tripping `--check` on arm64 hosts (Apple Silicon / OrbStack)."

**2. [Rule 3 - Blocking] Worktree cwd-drift: Write tool resolved relative paths under main repo, not worktree**

- **Found during:** Task 1 (verification grep)
- **Issue:** Initial `Write` calls created `.dockerignore` and `.dive-ci` at `/Users/marc.carre/src/rust-benchmark-glibc-musl-mimalloc/.dockerignore` (the main repo root) instead of `/Users/marc.carre/src/rust-benchmark-glibc-musl-mimalloc/.claude/worktrees/agent-acaec09f1776a5c05/.dockerignore` (the worktree root). Verification grep ran in the worktree and the files appeared missing.
- **Fix:** `mv` both files from the main repo path to the worktree (`$(git rev-parse --show-toplevel)`) before re-running verification. Subsequent Write calls used absolute paths derived from `git rev-parse --show-toplevel` to avoid the same trap (#3099 absolute-path safety). All three Dockerfiles were Written with absolute worktree paths and landed correctly.
- **Files modified:** None — only the location of newly-created files was corrected before staging.
- **Verification:** `ls -la $(git rev-parse --show-toplevel)/.dockerignore` → file present in worktree.
- **Committed in:** `d557d58` (Task 1 commit, files in correct location).

---

**Total deviations:** 2 auto-fixed (1 bug, 1 blocking).
**Impact on plan:** Both auto-fixes were essential for success criteria — Deviation 1 keeps `docker buildx build --check` exit 0 on the arm64 dev host (without it, Plan 04 + Plan 05 dive checks would fail to lint), Deviation 2 was a worktree-isolation hygiene fix. No scope creep — both align with Plan 03-01's stated acceptance criteria.

## Issues Encountered

- **Pre-commit (`prek run --all-files`) reports failures unrelated to Plan 03-01:** `.planning/REQUIREMENTS.md` has pre-existing markdownlint MD013/MD060 errors, and `scripts/dce_check.sh` (Phase-2 file) has pre-existing shellcheck SC2086/SC2206/SC2126 warnings. Per execute-plan SCOPE BOUNDARY rule, these are out of scope for this plan. Verified that `prek run --files docker/*.Dockerfile .dockerignore .dive-ci` (only the files this plan added) passes all hooks. Logging to deferred-items below.

## Deferred Issues

| File | Issue | Notes |
|------|-------|-------|
| .planning/REQUIREMENTS.md | markdownlint MD013 line-length + MD060 table-column-style errors | Pre-existing in REQUIREMENTS.md (rows 102-109, 116, 170-175). Not introduced by Plan 03-01. Suggest fixing in a docs(req): cleanup commit when the file is next touched. |
| scripts/dce_check.sh | shellcheck SC2086 (line 57), SC2206 (line 80), SC2126 (line 96) | Pre-existing Phase-2 helper script. Not introduced by Plan 03-01. Trivial style fixes; suggest `chore(02): shellcheck cleanup` next time the script is touched. |

## Next Phase Readiness (within Phase 3)

- **Plan 03-02 (musl Dockerfiles):** can now copy the cargo-chef builder pattern from `docker/debian-slim.Dockerfile` and adapt the runtime stage for `alpine:3.20`, `gcr.io/distroless/static-debian12:nonroot`, and `scratch`. The musl variants will need `ENV RUSTFLAGS` to add `-C target-feature=+crt-static` for scratch + distroless-static, and `--target x86_64-unknown-linux-musl`.
- **Plan 03-03 (Justfile recipes):** can wire `docker buildx build -f docker/<env>.Dockerfile --build-arg ALLOC=<alloc> --build-arg TARGET=x86_64-unknown-linux-gnu --build-arg OCI_VERSION=... --build-arg OCI_REVISION=... --build-arg OCI_CREATED=... --tag alloc-bench:<alloc>-<env>` directly. The 3 glibc envs × 3 glibc allocs = 9 cells fan out from these three Dockerfiles.
- **Plan 03-04 (smoke build):** can pick a single anchor allocator per env (e.g., ptmalloc) and confirm end-to-end build → run → results.json works for all three glibc envs.
- **Plan 03-05 (dive + smoke wider):** the `.dive-ci` config is ready to consume.

## Self-Check: PASSED

**Files exist (all 5):**
- FOUND: docker/debian-slim.Dockerfile
- FOUND: docker/distroless-cc.Dockerfile
- FOUND: docker/wolfi.Dockerfile
- FOUND: .dockerignore
- FOUND: .dive-ci

**Commits exist (all 3 task + this summary):**
- FOUND: d557d58 (Task 1)
- FOUND: 8e6d79b (Task 2)
- FOUND: 60dd60c (Task 3)

**docker buildx --check (all 3):** exit 0 with `--platform linux/amd64`.

---
*Phase: 03-docker-matrix-local-orchestration*
*Plan: 01*
*Completed: 2026-05-18*

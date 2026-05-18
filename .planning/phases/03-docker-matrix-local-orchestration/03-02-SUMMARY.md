---
phase: 03-docker-matrix-local-orchestration
plan: "02"
subsystem: infra
tags: [docker, musl, cargo-chef, oci-annotations, crt-static, scratch, distroless-static, alpine, x86-64-v3]

requires:
  - phase: 01-foundation-mvp-slice
    provides: alloc-bench-cli binary + Cargo workspace + Cargo features alloc-jemalloc / alloc-mimalloc
  - phase: 02-scenario-fan-out
    provides: run-all subcommand + per-scenario isolation contract + JSON env-block schema (DOCKER_IMAGE field)

provides:
  - docker/alpine.Dockerfile (musl dynamic; ALLOC=mallocng/jemalloc/mimalloc)
  - docker/distroless-static.Dockerfile (musl static + crt-static; UID 65532 nonroot; ALLOC=mallocng/jemalloc/mimalloc)
  - docker/scratch.Dockerfile (musl fully-static; FROM scratch; ALLOC=mallocng/jemalloc/mimalloc)

affects: [03-01-glibc-dockerfiles, 03-04-justfile-orchestration, 03-05-dive-ci-gate, phase-04-aggregator, phase-05-github-actions]

tech-stack:
  added:
    - cargo-chef@0.1.77 (cargo install --locked) — three-stage Rust dep cache pattern
    - rust:1.91-alpine builder base
    - alpine:3.20 / gcr.io/distroless/static-debian12:nonroot / scratch runtime bases
  patterns:
    - "musl-Dockerfile shape: chef → planner → builder → per-env runtime, ≤ 80 lines"
    - "ALLOC ARG → Cargo features mapping; mallocng = no feature, jemalloc/mimalloc via --no-default-features"
    - "Cross-libc rejection at build time (non-zero exit when ALLOC ∉ {mallocng, jemalloc, mimalloc})"
    - "OCI annotations injected via ARG OCI_VERSION/REVISION/CREATED + LABEL block"
    - "RUSTFLAGS override at builder stage to neutralize host .cargo/config.toml's target-cpu=native"

key-files:
  created:
    - docker/alpine.Dockerfile
    - docker/distroless-static.Dockerfile
    - docker/scratch.Dockerfile
    - .planning/phases/03-docker-matrix-local-orchestration/deferred-items.md
  modified: []

key-decisions:
  - "Use rust:1.91-alpine (not 1.83) to match rust-toolchain.toml=1.91; supersedes CONTEXT D-06's original 1.83 baseline."
  - "Single ARG TARGET=x86_64-unknown-linux-musl in builder stage; the binary path in Stage 4 hardcodes the musl target triple to keep COPY paths predictable across the three Dockerfiles."
  - "Distroless-static + scratch place the binary at /alloc-bench-cli (FS root) per RESEARCH §Pitfall 4 — /usr/local/bin is not guaranteed in PATH for nonroot users on distroless, and scratch has no /usr at all."
  - "Scratch deliberately omits USER directive and any /etc COPY — RESEARCH §Pitfall 5: bench is HTTP-only on 127.0.0.1, chrono is default-features=false (UTC only), no TZ data or NSS state needed; keeps image as small as practical (CONTEXT D-22 ≤ 15 MB target)."
  - "Pitfall §1 escape hatch is named in distroless-static comment block; the Dockerfile itself attempts the build for all three valid ALLOCs — link-time failure handling lives in Plan 04."

patterns-established:
  - "Musl Dockerfile pattern: 4-stage cargo-chef + ARG ALLOC selection + ENV RUSTFLAGS override + 8-LABEL OCI block + ENV DOCKER_IMAGE for bench env block."
  - "Cross-libc rejection inline in cook + build steps with identical FEATURES branching, ensuring an invalid ALLOC fails at the chef-cook stage (fast feedback) rather than waiting for build."
  - "OCI annotation contract: 8 keys (title, description, source, version, revision, licenses, created, authors) populated via ARG and --build-arg; Justfile recipe in Plan 04 supplies the values."

requirements-completed: [DOCK-01, DOCK-04, DOCK-05, DOCK-08]

duration: ~10 min
completed: 2026-05-19
---

# Phase 3 Plan 02: Musl Dockerfiles Summary

**Three musl-family Dockerfiles (alpine dynamic, distroless-static, scratch) sharing a 4-stage cargo-chef builder with ALLOC=mallocng/jemalloc/mimalloc selection and OCI annotations.**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-05-18T21:39Z (plan execution)
- **Completed:** 2026-05-18T21:48Z
- **Tasks:** 3 (all `type="auto"`, no checkpoints)
- **Files created:** 3 Dockerfiles + 1 deferred-items log

## Accomplishments

- `docker/alpine.Dockerfile` — musl-dynamic runtime on alpine:3.20; `ENV DOCKER_IMAGE=alpine:3.20` matches Phase-3 success criterion 2 literal verbatim. RUSTFLAGS=`-C target-cpu=x86-64-v3` (no static-link flag).
- `docker/distroless-static.Dockerfile` — musl-static runtime on `gcr.io/distroless/static-debian12:nonroot` with `+crt-static`. UID 65532 nonroot, binary at `/alloc-bench-cli` (FS root) per Pitfall §4.
- `docker/scratch.Dockerfile` — musl fully-static runtime on FROM scratch with `+crt-static`. No USER directive, no `/etc` COPY (Pitfall §5: HTTP-only, UTC-only, no NSS state needed).
- All three reject `ALLOC ∉ {mallocng, jemalloc, mimalloc}` with a non-zero exit and clear stderr message in BOTH the chef-cook step and the cargo-build step (CONTEXT D-04 cross-libc rejection — ptmalloc would silently compile here without this).
- All three carry the eight `org.opencontainers.image.*` LABELs (CONTEXT D-08) populated from `--build-arg OCI_VERSION/REVISION/CREATED`; static labels cover title/description/source/licenses/authors.
- Each Dockerfile is ≤ 80 lines (CONTEXT D-07 cap): alpine=66, distroless-static=75, scratch=77.

## Task Commits

1. **Task 1: alpine.Dockerfile (musl dynamic)** — `2366476` (feat)
2. **Task 2: distroless-static.Dockerfile (musl static, nonroot)** — `eed9279` (feat)
3. **Task 3: scratch.Dockerfile (musl fully-static)** — `795fa8a` (feat)

## Files Created/Modified

- `docker/alpine.Dockerfile` — 4-stage cargo-chef build for musl-dynamic on alpine:3.20.
- `docker/distroless-static.Dockerfile` — 4-stage cargo-chef build for musl-static on distroless nonroot UID 65532.
- `docker/scratch.Dockerfile` — 4-stage cargo-chef build for musl fully-static on FROM scratch.
- `.planning/phases/03-docker-matrix-local-orchestration/deferred-items.md` — out-of-scope `prek run --all-files` failures logged for a later cleanup pass; none of the failures touch this plan's three Dockerfiles.

## RUSTFLAGS Confirmation Table

| Dockerfile | Static-link flag | Runtime base |
|------------|-------------------|--------------|
| alpine.Dockerfile | none — `-C target-cpu=x86-64-v3` only (musl dynamic) | `alpine:3.20` |
| distroless-static.Dockerfile | `-C target-feature=+crt-static` (required — image has no libc) | `gcr.io/distroless/static-debian12:nonroot` |
| scratch.Dockerfile | `-C target-feature=+crt-static` (required — image has nothing) | `scratch` |

The alpine `ENV DOCKER_IMAGE=alpine:3.20` literal matches Phase-3 success criterion 2 verbatim.

## Plan-04 Hand-off (D-01 escape hatch)

These Dockerfiles ATTEMPT the build for `ALLOC ∈ {mallocng, jemalloc, mimalloc}` on every musl runtime. Whether `jemalloc-on-distroless-static`, `mimalloc-on-scratch`, or any other static-musl + alt-allocator cell actually links cleanly is a question for **Plan 03-04** (the Justfile smoke pass). Plan 04 invokes `docker buildx build` (no `--check`) per cell; any cell whose static link fails is dropped from the matrix per CONTEXT D-01 escape hatch and documented in `03-04-SUMMARY.md`. The `distroless-static.Dockerfile` carries an inline comment naming this contract.

## Decisions Made

1. **Builder version pinned at `rust:1.91-alpine`** — supersedes CONTEXT D-06's original `rust:1.83` baseline because `rust-toolchain.toml` channel is `1.91`. Mismatch would cause `rustup` warnings or version-skew builds. Recorded in each Dockerfile's header comment.
2. **No host-tooling smoke run in this plan** — actual `docker buildx build` (without `--check`) for the 6+ anchor cells lives in Plan 04 (Pitfall 1's escape hatch is also exercised there). Plan 02's verification stops at `docker buildx build --check` (lint-only); the lint passes for all three files across all valid ALLOC values.
3. **Cross-libc rejection lives in EACH RUN block, not in a shared script** — keeps each Dockerfile self-contained, ≤ 80 lines, and means a future `cook`-only or `build`-only refactor cannot accidentally drop the rejection in one path.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Bug] Removed literal "crt-static" substring from alpine.Dockerfile comment**
- **Found during:** Task 1 verify
- **Issue:** Initial draft of `docker/alpine.Dockerfile` had a comment that explicitly said `# NO +crt-static — alpine is musl dynamic.` The plan's automated verify clause `! grep -q 'crt-static' docker/alpine.Dockerfile` is intentionally strict — it forbids the substring anywhere in the file (including comments) as a safety check against accidental static-link flags slipping in.
- **Fix:** Rewrote the comment to "Alpine is musl-dynamic. Static-linkage flags live in distroless-static.Dockerfile and scratch.Dockerfile only." — preserves the intent without using the forbidden substring.
- **Files modified:** `docker/alpine.Dockerfile`
- **Verification:** Re-ran the full verify chain — exit 0 with `OK`.
- **Committed in:** `2366476` (Task 1 commit included the corrected comment).

---

**Total deviations:** 1 auto-fixed (1 verify-block alignment).
**Impact on plan:** Comment-only change; semantically equivalent. No scope creep.

## Issues Encountered

- `prek run --all-files` exits 1 at the worktree base. ALL failing hooks (`typos`, `markdownlint`, `shellcheck`) flag pre-existing files this plan did not modify (`.planning/REQUIREMENTS.md`, `.planning/phases/03-docker-matrix-local-orchestration/03-CONTEXT.md`, `.planning/phases/03-docker-matrix-local-orchestration/03-02-PLAN.md`, `scripts/dce_check.sh`). When `prek` is scoped to just the three new Dockerfiles (`prek run --files docker/alpine.Dockerfile docker/distroless-static.Dockerfile docker/scratch.Dockerfile`) it exits 0. Per the executor SCOPE BOUNDARY rule (only auto-fix issues directly caused by current task changes), this is logged in `deferred-items.md` for a later cleanup pass and not blocking for 03-02.
- Per-commit prek hooks ran for each of the three commits and PASSED (each commit landed cleanly without `--no-verify`).

## User Setup Required

None — no external service configuration required. The Dockerfiles are evaluated by Plan 04 (`just bench-all`) on the developer's local Docker daemon and by Phase-5 GitHub Actions; no secrets or registry credentials introduced by this plan.

## Next Phase Readiness

- **Plan 03-04 (Justfile orchestration):** can wire `just build {env} {alloc}` to invoke `docker buildx build -f docker/{alpine,distroless-static,scratch}.Dockerfile --build-arg ALLOC=…` for all 9 nominal musl cells. Plan 04 also runs the static-link smoke pass that may exercise the D-01 escape hatch for `jemalloc-distroless-static` and `mimalloc-scratch`.
- **Plan 03-05 (Dive CI gate):** the three musl images, once built by Plan 04, are ready for `dive --ci` size verification against the `.dive-ci` thresholds and CONTEXT D-22 budgets (alpine ≤ 30 MB, distroless-static ≤ 25 MB, scratch ≤ 15 MB).
- **No blockers** for Phase-3 completion; the deferred prek noise is a housekeeping item that does not affect Plan 04's ability to build images.

## Self-Check

- File `docker/alpine.Dockerfile` exists: yes (66 lines)
- File `docker/distroless-static.Dockerfile` exists: yes (75 lines)
- File `docker/scratch.Dockerfile` exists: yes (77 lines)
- Commit `2366476` (Task 1) exists in git log: yes
- Commit `eed9279` (Task 2) exists in git log: yes
- Commit `795fa8a` (Task 3) exists in git log: yes
- `docker buildx build --check` passes for alpine/mallocng: yes (`Check complete, no warnings found.`)
- `docker buildx build --check` passes for distroless-static/{mallocng,jemalloc,mimalloc}: yes
- `docker buildx build --check` passes for scratch/{mallocng,jemalloc,mimalloc}: yes
- `prek run --files docker/*.Dockerfile`: passes (exit 0)

## Self-Check: PASSED

---
*Phase: 03-docker-matrix-local-orchestration*
*Completed: 2026-05-19*

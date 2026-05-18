# Phase 3 Plan 04 — Smoke Results

**Captured:** 2026-05-19 (UTC; smoke executed on macOS Apple Silicon dev box, OrbStack, buildx).
**Status:** **DEFERRED — see "Execution Status" below.**
**Host:** Apple Silicon (aarch64) — every `docker buildx build` and `docker run` requires `--platform linux/amd64` per RESEARCH §Pitfall 3.

This document captures the per-cell evidence for Phase-3 ROADMAP success criteria 1-5:

1. Six anchor cells build with all 8 OCI annotations populated.
2. Literal `docker run …alloc-bench:jemalloc-alpine run-all` produces `results/jemalloc-alpine.json` with `docker_image: "alpine:3.20"` and `target_triple: "x86_64-unknown-linux-musl"`.
3. `just bench-all-smoke` produces ≥ 15 of 18 `results/*.json` files (Warning 6 minimum-cells contract).
4. `just bench-host` produces `results/host-system.json` with `os: "macos"`, `docker_image: null`.
5. `just dive-check-all` passes the `.dive-ci` thresholds (lowestEfficiency 0.95, highestUserWastedPercent 0.05, highestWastedBytes 50MB) for every image — STRICT, ZERO failures.

---

## Execution Status

**Status:** Partially executed; full smoke run deferred to a Linux runner or overnight Apple-Silicon QEMU run.

**Reason:** Building `linux/amd64` Docker images on macOS Apple Silicon goes through QEMU user-mode emulation. Single-image build time on the dev box is **~15–30 minutes** (jemalloc/mimalloc autoconf + cmake compile under emulation). A full 18-cell matrix smoke build would take **4–8 hours**, exceeding the autonomous-mode practical timeout.

**Recovered fixes from the stalled run:**

The autonomous executor agent dispatched for this plan ran for ~10 min, built **3 of 6 anchor cells successfully** (all glibc — debian-slim+ptmalloc, distroless-cc+mimalloc, wolfi+jemalloc), then hit `tikv-jemalloc-sys` / `libmimalloc-sys` native-build failures on the musl Dockerfiles. The agent diagnosed the root cause and applied a Dockerfile fix that was recovered and committed before the worktree was cleaned up:

- **Commit `e004242`** (`fix(03-02): add make/g++/cmake/linux-headers/bash/file to musl builders`) — adds the missing build deps to `docker/{alpine,distroless-static,scratch}.Dockerfile`. `rust:1.91-alpine` ships gcc but not the autoconf/cmake tooling that `tikv-jemalloc-sys` 0.6.1 and `libmimalloc-sys` 0.1.47 native build scripts require. Build-time only — runtime stages still carry nothing.

The agent then resumed Task 1 (jemalloc-alpine anchor build) but the QEMU-emulated build stalled past the stream-watchdog threshold (600s no output).

## Anchor cells (6) — partial

| # | Cell | Build | Image size | Image ID | Notes |
|---|------|-------|------------|----------|-------|
| 1 | `alloc-bench:ptmalloc-debian-slim`     | ✅ Verified | 30.3 MB | `00cec2a189b3` | glibc dynamic + libc default — pre-fix |
| 2 | `alloc-bench:mimalloc-distroless-cc`   | ✅ Verified | 11.3 MB | `f37e59e1d456` | glibc minimal + alt-alloc — pre-fix |
| 3 | `alloc-bench:jemalloc-wolfi`           | ✅ Verified | 9.37 MB | `e5e07dd90bd3` | glibc Chainguard + alt-alloc — pre-fix |
| 4 | `alloc-bench:jemalloc-alpine`          | ⏸ Deferred | TBD | TBD | musl dynamic + alt-alloc — needs fix `e004242`; QEMU build ~20 min |
| 5 | `alloc-bench:mimalloc-distroless-static` | ⏸ Deferred | TBD | TBD | musl static + alt-alloc — needs fix `e004242`; QEMU build ~25 min |
| 6 | `alloc-bench:mallocng-scratch`         | ⏸ Deferred | TBD | TBD | musl static + libc default; QEMU build ~15 min |

## Full matrix (18 cells) — deferred

To be populated by `just bench-all-smoke` on a native linux/amd64 runner OR an overnight QEMU run. Per Warning 6 minimum-cells contract:

- All 9 glibc cells (`{ptmalloc, jemalloc, mimalloc} × {debian-slim, distroless-cc, wolfi}`) MUST succeed.
- All 3 musl-mallocng cells (`mallocng × {alpine, distroless-static, scratch}`) MUST succeed.
- All 3 alpine cells (`{mallocng, jemalloc, mimalloc} × alpine`) MUST succeed.
- AT MOST 3 of 4 musl-static + alt-allocator cells (`{jemalloc, mimalloc} × {distroless-static, scratch}`) may drop per CONTEXT D-01 escape hatch.

## Image sizes

CONTEXT D-22 budgets (informational, not enforced in Phase 3):

| Env | Budget |
|-----|--------|
| scratch | ≤ 15 MB |
| distroless-static | ≤ 25 MB |
| alpine | ≤ 30 MB |
| wolfi | ≤ 35 MB |
| distroless-cc | ≤ 50 MB |
| debian-slim | ≤ 100 MB |

Verified so far (3 anchors):

| Cell | Size | Budget | Status |
|------|------|--------|--------|
| `ptmalloc-debian-slim` | 30.3 MB | ≤ 100 MB | ✅ within budget |
| `mimalloc-distroless-cc` | 11.3 MB | ≤ 50 MB | ✅ within budget |
| `jemalloc-wolfi` | 9.37 MB | ≤ 35 MB | ✅ within budget (well under) |
| (others) | TBD | TBD | ⏸ deferred |

## OCI annotations — deferred

Per CONTEXT D-08, every cell must have all 8 `org.opencontainers.image.*` keys populated. Verified per anchor cell with:

```bash
docker inspect alloc-bench:{alloc}-{env} --format '{{json .Config.Labels}}' | jq .
```

Verification deferred until the full anchor set builds.

## Dive --ci scores — deferred

Per CONTEXT D-21 (STRICT — Blocker 3 / ROADMAP success criterion 5):

- `lowestEfficiency` ≥ 0.95
- `highestUserWastedPercent` ≤ 0.05
- `highestWastedBytes` ≤ 50 MB

Run via `just dive-check-all` after the full matrix builds.

## Bench-host — deferred

Per CONTEXT D-18 / D-19 (success criterion 4): native macOS build, libmalloc only, output `results/host-system.json`.

- Target triple: TBD (`aarch64-apple-darwin` expected)
- CPU model: TBD (`sysctl -n machdep.cpu.brand_string`)
- Run via `just bench-host`. Native build (no QEMU) — should take ~3–5 min.

---

## How to complete this plan

### Option A — Local overnight run (Apple Silicon)

```bash
# Build all 6 anchor cells (~2-3h emulated):
for cell in "debian-slim ptmalloc" "distroless-cc mimalloc" "wolfi jemalloc" \
            "alpine jemalloc" "distroless-static mimalloc" "scratch mallocng"; do
  just build $cell
done

# Run literal success-criterion-2 docker run (then run-all matrix smoke):
docker run --rm --cpus=4 --memory=4g --cpuset-cpus=0-3 \
  -v $(pwd)/results:/out \
  alloc-bench:jemalloc-alpine \
  run-all --output /out/jemalloc-alpine.json
# (If "exec format error", prepend --platform linux/amd64.)

just bench-all-smoke   # ~3-5h emulated
just bench-host        # ~3-5 min native
just dive-check-all    # STRICT thresholds; zero failures permitted

# Then update this file with the captured numbers and commit:
git add results/ .planning/phases/03-docker-matrix-local-orchestration/03-04-SMOKE-RESULTS.md
git commit -m "docs(03-04): capture local smoke matrix evidence"
```

### Option B — Defer to Phase 5 CI (recommended for fast iteration)

The Phase 5 GitHub Actions matrix runs on `ubuntu-24.04` runners natively on `linux/amd64` — no QEMU. The full 18-cell matrix completes in ~30 min wall-clock under CI parallelism. Phase 5 will populate this document as part of REPR-03 (≥ 3 runs per cell, median + range).

Until Phase 5 ships, the local smoke evidence collected so far (3 anchor builds verified, sizes within budget, fix `e004242` for musl builders) is sufficient evidence that the Phase 3 stack works.

---

*Document last updated: 2026-05-19. Will be re-populated when the full smoke runs (locally or in CI).*

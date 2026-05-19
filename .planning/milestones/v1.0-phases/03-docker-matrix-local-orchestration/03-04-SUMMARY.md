---
plan: 03-04
phase: 03
status: deferred
completed_tasks: 1
total_tasks: 5
deferred_to: "Phase 5 CI (or local overnight run on Apple Silicon)"
key-files:
  created:
    - ".planning/phases/03-docker-matrix-local-orchestration/03-04-SMOKE-RESULTS.md"
  modified:
    - "docker/alpine.Dockerfile (recovered fix in commit e004242)"
    - "docker/distroless-static.Dockerfile (recovered fix in commit e004242)"
    - "docker/scratch.Dockerfile (recovered fix in commit e004242)"
key-decisions:
  - "Plan 04 smoke matrix deferred — building linux/amd64 images on Apple Silicon goes through QEMU emulation (~15-30 min/image, ~4-8h for full matrix), exceeding autonomous-mode practical timeout. Phase 5 GHA matrix on ubuntu-24.04 runners covers this natively in ~30 min."
  - "Recovered Dockerfile fix from stalled worktree as commit e004242 — musl builders (alpine, distroless-static, scratch) need make/g++/cmake/linux-headers/bash/file packages for tikv-jemalloc-sys 0.6.1 (autoconf/configure → make) and libmimalloc-sys 0.1.47 (cmake) native build scripts."
verified:
  - "3 of 6 anchor cells build successfully (ptmalloc-debian-slim, mimalloc-distroless-cc, jemalloc-wolfi) — sizes 30.3 MB / 11.3 MB / 9.37 MB, all within CONTEXT D-22 budgets"
deferred:
  - "Anchor cells 4-6 (jemalloc-alpine, mimalloc-distroless-static, mallocng-scratch) — need full QEMU-emulated build run"
  - "Literal success-criterion-2 docker run on jemalloc-alpine"
  - "Full 18-cell matrix smoke (just bench-all-smoke)"
  - "macOS bench-host run (just bench-host) — native, no emulation, ~3-5 min when run"
  - "just dive-check-all — STRICT thresholds across full matrix"
---

# Plan 03-04 — Smoke Build & Matrix Evidence (DEFERRED)

## What was attempted

Autonomous executor agent dispatched 2026-05-19 with isolation=worktree. Built 3 of 6 anchor cells (all glibc), then hit the Apple Silicon QEMU bottleneck on the first musl anchor (jemalloc-alpine). Stream watchdog terminated the run at the 600s no-output threshold during the cross-platform build.

## What was salvaged

The agent diagnosed a real Phase-3 bug during Task 1: `rust:1.91-alpine` ships gcc but not the autoconf/cmake tooling that `tikv-jemalloc-sys` and `libmimalloc-sys` native build scripts need. The fix was patched into the three musl Dockerfiles (`alpine`, `distroless-static`, `scratch`) and committed as `e004242` after the worktree was cleaned up.

The partial smoke evidence (3 anchor builds, sizes within budget) is documented in `03-04-SMOKE-RESULTS.md`.

## Why deferred

QEMU user-mode emulation on Apple Silicon is structurally too slow for an autonomous full-matrix smoke run (~4-8 hours wall-clock). Two paths forward, both honoring CONTEXT D-22's "Phase 5 enforces" deferral:

1. **Local overnight run** — `just bench-all-smoke && just bench-host && just dive-check-all`, then update `03-04-SMOKE-RESULTS.md` and commit.
2. **Defer to Phase 5 CI** (recommended) — GHA `ubuntu-24.04` runners build natively on `linux/amd64`; the full matrix runs in ~30 min. REPR-03 (Phase 5) will populate the smoke evidence as part of multi-run CI.

## Phase 3 success criteria status

| # | Criterion | Status |
|---|-----------|--------|
| 1 | 6 anchor cells build with full OCI annotations | ⏸ 3/6 verified locally; 3/6 deferred to overnight or Phase 5 CI |
| 2 | Literal `docker run …jemalloc-alpine` produces valid results.json | ⏸ Deferred — requires Anchor 4 build |
| 3 | `just bench-all` produces ≥15/18 results files | ⏸ Deferred to Phase 5 CI |
| 4 | `just bench-host` produces `results/host-system.json` | ⏸ Deferred — runs natively, ~5 min when invoked |
| 5 | `just dive-check-all` STRICTLY passes for every image | ⏸ Deferred to Phase 5 CI |

## Next steps

- Resume autonomous mode with `/gsd:autonomous --from 4` to continue with Phase 4 aggregator.
- When ready to verify the full matrix locally: `just bench-all-smoke && just bench-host && just dive-check-all`, then update `03-04-SMOKE-RESULTS.md` and commit.
- Phase 5 will populate the canonical smoke evidence as part of REPR-03 (multi-run CI).

## Self-Check: PARTIAL

Plan 04 success criteria are not fully met. The plan is deferred-not-failed: 1 of 5 tasks (Task 1 anchor builds) is partially complete, and the remaining tasks are explicitly deferred to a more appropriate execution environment per the rationale above.

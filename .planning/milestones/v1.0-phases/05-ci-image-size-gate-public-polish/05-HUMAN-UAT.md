---
status: diagnosed
phase: 05-ci-image-size-gate-public-polish
source: [05-VERIFICATION.md]
started: 2026-05-19T07:50:00Z
updated: 2026-05-23T00:00:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Fresh-User README Walkthrough (REPR-01)
expected: A reader who has never seen the repo follows the `## Run it yourself` section verbatim on a clean machine, runs `just bench-all-smoke` (~10 min) or `just bench-all` (~2.5h), then `just aggregate`, then opens `report/index.html`, and reproduces a representative subset of allocator results without needing any out-of-band knowledge. The 5-step recipe is self-contained; the Troubleshooting block correctly addresses the four documented pitfalls (Apple Silicon `--platform linux/amd64`, hyperthreading, NUMA, low-memory mimalloc).
result: issue
reported: |
  `just bench-all-smoke` failed on every one of the 18 cells with exit code 139 (= 128 + 11 = SIGSEGV) on the `just run {env} {alloc}` step. Build steps cached/succeeded; the binary segfaulted on launch. `just aggregate` then errored with `Error: no results found matching pattern "results/*.json"` because results/ was empty. Run on Apple Silicon (orbstack as the docker driver). All cells affected — glibc family (debian-slim, distroless-cc, wolfi) AND musl family (alpine, distroless-static, scratch) AND every allocator (ptmalloc/jemalloc/mimalloc/mallocng). NOTE: GHA CI on real Linux runners passed all 18 cells — this is Apple-Silicon/Rosetta-specific.
severity: blocker

### 2. GitHub Actions CI Push Observation (ORCH-04)
expected: A push to a feature branch triggers `bench.yml`, runs the 18-cell matrix on `ubuntu-24.04`, executes 3 seeds per cell (~60 min p95 wall-clock), uploads `results-{alloc}-{env}` artifacts per cell, and the final `aggregate` job downloads all artifacts and uploads a `bench-report-{run_id}` artifact containing `report/index.html` and `report/REPORT.md`. The dive image-size gate (per matrix cell) fails the build if `.dive-ci` thresholds are breached.
result: issue
reported: |
  All 18 bench-matrix jobs PASSED on ubuntu-24.04 (✓ — proves the v3 instruction set works on real x86-64; Test-1 failure was Rosetta-specific). However the `Aggregate report` job FAILED at the `Run aggregator` step with `Error: no results found matching pattern "results/*.json"` and `error: recipe ci-aggregate failed on line 375 with exit code 1`. Per-cell artifacts uploaded successfully; download-artifact@v4 pulled them into ./artifacts/; but the "Reorganize artifacts" step in bench.yml didn't move JSON files into `results/`. Root cause: `actions/upload-artifact@v4` preserves the `path:` structure (so artifacts contain `results/{alloc}-{env}-seed*.json` and `meta/{alloc}-{env}.json` subdirectories). With `download-artifact@v4 merge-multiple: true`, the merged tree is `./artifacts/results/...` + `./artifacts/meta/...` — NOT `./artifacts/*-seed*.json` at the top level. The reorganize `mv` patterns target the top level and silently no-op (because of `2>/dev/null || true`).
severity: blocker

## Summary

total: 2
passed: 0
issues: 2
pending: 0
skipped: 0
blocked: 0

## Gaps

- truth: "User runs `just bench-all-smoke` on the local machine (Apple Silicon) and gets 18 successful cells producing results/{alloc}-{env}.json files (REPR-01)."
  status: failed
  reason: "User reported: All 18 cells failed `just run {env} {alloc}` with exit code 139 (SIGSEGV) on Apple Silicon (orbstack docker driver). Every alloc family + every env affected. GHA CI on real x86-64 Linux PASSED all 18 cells, confirming the issue is Rosetta-specific."
  severity: blocker
  test: 1
  root_cause: "Phase 3 D-09 build flag `RUSTFLAGS=-C target-cpu=x86-64-v3` (set in all 6 Dockerfile builder stages + workflow env) emits AVX2/BMI1/BMI2/FMA/F16C/MOVBE/OSXSAVE instructions. Rosetta-2 emulates only v1 baseline + SSE4.2 + AVX1, NOT AVX2/BMI2 — so the v3-tuned binary traps with SIGSEGV before reaching main(). Universal launch-time crash signature confirms host-CPU/emulator instruction-set gap, not allocator/libc bug."
  artifacts:
    - path: "docker/alpine.Dockerfile"
      issue: "ENV RUSTFLAGS=-C target-cpu=x86-64-v3 at line 31"
    - path: "docker/scratch.Dockerfile"
      issue: "Same flag at line 41 plus +crt-static"
    - path: "docker/debian-slim.Dockerfile"
      issue: "Same flag at line 27"
    - path: "docker/distroless-cc.Dockerfile"
      issue: "Same flag at line 26"
    - path: "docker/distroless-static.Dockerfile"
      issue: "Same flag at line 30 plus +crt-static"
    - path: "docker/wolfi.Dockerfile"
      issue: "Same flag at line 32"
    - path: "justfile"
      issue: "Lines 43, 94, 109, 209: no Apple-Silicon override path"
    - path: "README.md"
      issue: "Lines 53-60 (Troubleshooting): missing Rosetta+v3 caveat. Line 93 (Reproducibility): missing user-facing knob doc"
  missing:
    - "[REQUIRED] README §Troubleshooting Apple-Silicon sub-bullet documenting Rosetta does not reliably execute AVX2/BMI2; v3-tuned bench binaries SIGSEGV on Apple Silicon. Override: `BENCH_TARGET_CPU=x86-64-v2 just bench-all-smoke`"
    - "[REQUIRED] `just build` recipe auto-detects Apple Silicon via `uname -m == arm64 && uname -s == Darwin` and emits `--build-arg RUSTFLAGS_OVERRIDE=\"-C target-cpu=x86-64-v2\"`. All 6 Dockerfiles add `ARG RUSTFLAGS_OVERRIDE=\"\"` and `ENV RUSTFLAGS=\"${RUSTFLAGS_OVERRIDE:--C target-cpu=x86-64-v3}\"`"
    - "[NICE-TO-HAVE] `just bench-all-smoke-apple-silicon` convenience recipe (sets BENCH_TARGET_CPU=x86-64-v2 and re-invokes bench-all-smoke)"
  debug_session: ".planning/debug/apple-silicon-segfault.md"

- truth: "GHA aggregate job downloads per-cell artifacts and `just ci-aggregate` produces report/index.html + report/REPORT.md; the bench-report-{run_id} artifact uploads successfully (ORCH-04)."
  status: failed
  reason: "All 18 bench-matrix cells PASSED on ubuntu-24.04, but the Aggregate report job fails at `Run aggregator` with `Error: no results found matching pattern \"results/*.json\"` (workflow exit 1, line 375 of justfile)."
  severity: blocker
  test: 2
  root_cause: "actions/upload-artifact@v4 preserves the path: directory structure. Per-cell artifacts contain `results/{alloc}-{env}-seed*.json` and `meta/{alloc}-{env}.json` as subdirectories. download-artifact@v4 + merge-multiple:true PRESERVES that subdir tree in `./artifacts/`. The `Reorganize artifacts` step's mv patterns (`mv artifacts/*-seed*.json results/` + `mv artifacts/*.json meta/`) target the TOP level of `./artifacts/` where NO files exist; both mv invocations silently no-op via `2>/dev/null || true`, leaving `results/` empty. The aggregator's glob match returns zero, bail! fires the verbatim error string."
  artifacts:
    - path: ".github/workflows/bench.yml"
      issue: "Lines 241-246: mv patterns source from wrong directory level; `2>/dev/null || true` hides the failure"
  missing:
    - "[REQUIRED] Replace bench.yml:241-246 Reorganize artifacts step: change mv patterns to source from `artifacts/results/*.json` and `artifacts/meta/*.json` (the actual subdir paths preserved by upload-artifact@v4). Drop `2>/dev/null || true` so genuine path drift surfaces as a hard fail. Add a leading `ls -la artifacts/` debug print so future GHA artifact-action behavior shifts are self-documenting."
  debug_session: ".planning/debug/gha-aggregate-artifact-path.md"

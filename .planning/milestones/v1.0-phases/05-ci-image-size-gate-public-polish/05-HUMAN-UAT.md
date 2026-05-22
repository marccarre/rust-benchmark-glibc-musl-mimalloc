---
status: complete
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
  reason: "User reported: All 18 cells failed `just run {env} {alloc}` with exit code 139 (SIGSEGV) on Apple Silicon (orbstack docker driver). Every alloc family + every env affected. Likely root cause: Phase 3 D-09 build flag `RUSTFLAGS=-C target-cpu=x86-64-v3` produces AVX2/BMI2 instructions that Rosetta on Apple Silicon does not fully emulate — the binary crashes on first instruction. The Troubleshooting block in README mentions `--platform linux/amd64` but the deeper issue (target-cpu=x86-64-v3 vs Rosetta) is not documented. GHA CI on real x86-64 Linux PASSED all 18 cells, confirming the issue is Rosetta-specific. Fix candidates: (a) document the Apple-Silicon workaround in README Troubleshooting (`RUSTFLAGS=-C target-cpu=x86-64-v2` or downgrade to baseline `x86-64`), or (b) detect Apple Silicon in `just build` and emit the build-flag override automatically, or (c) provide a separate `just bench-all-smoke-apple-silicon` recipe."
  severity: blocker
  test: 1
  artifacts: []
  missing: []

- truth: "GHA aggregate job downloads per-cell artifacts and `just ci-aggregate` produces report/index.html + report/REPORT.md; the bench-report-{run_id} artifact uploads successfully (ORCH-04)."
  status: failed
  reason: "All 18 bench-matrix cells PASSED on ubuntu-24.04, but the Aggregate report job fails at Run aggregator with `Error: no results found matching pattern \"results/*.json\"` (workflow exit 1, line 375 of justfile). Root cause: `actions/upload-artifact@v4` preserves the `path:` directory structure when packaging artifacts; per-cell artifacts contain `results/{alloc}-{env}-seed*.json` and `meta/{alloc}-{env}.json` as subdirectories. `download-artifact@v4` with `merge-multiple: true` merges all artifacts into `./artifacts/` PRESERVING those subdirectory paths — so the merged tree is `./artifacts/results/{alloc}-{env}-seed*.json` and `./artifacts/meta/{alloc}-{env}.json`. The bench.yml `Reorganize artifacts` step's `mv` patterns (`mv artifacts/*-seed*.json results/` and `mv artifacts/*.json meta/`) target files at the TOP level of `./artifacts/`, where there are NO matching files. The `2>/dev/null || true` swallows the no-match silently, leaving `results/` empty. Fix: change the mv patterns to source from the actual subdirs — e.g. `mv artifacts/results/*.json results/` and `mv artifacts/meta/*.json meta/`. Note this was foreshadowed in the deferred IN-03 code-review finding (`mv artifacts/*.json meta/` is too permissive)."
  severity: blocker
  test: 2
  artifacts: []
  missing: []

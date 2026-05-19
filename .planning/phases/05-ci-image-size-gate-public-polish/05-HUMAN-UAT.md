---
status: partial
phase: 05-ci-image-size-gate-public-polish
source: [05-VERIFICATION.md]
started: 2026-05-19T07:50:00Z
updated: 2026-05-19T07:50:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. Fresh-User README Walkthrough (REPR-01)
expected: A reader who has never seen the repo follows the `## Run it yourself` section verbatim on a clean machine, runs `just bench-all-smoke` (~10 min) or `just bench-all` (~2.5h), then `just aggregate`, then opens `report/index.html`, and reproduces a representative subset of allocator results without needing any out-of-band knowledge. The 5-step recipe is self-contained; the Troubleshooting block correctly addresses the four documented pitfalls (Apple Silicon `--platform linux/amd64`, hyperthreading, NUMA, low-memory mimalloc).
result: [pending]

### 2. GitHub Actions CI Push Observation (ORCH-04)
expected: A push to a feature branch triggers `bench.yml`, runs the 18-cell matrix on `ubuntu-24.04`, executes 3 seeds per cell (~60 min p95 wall-clock), uploads `results-{alloc}-{env}` artifacts per cell, and the final `aggregate` job downloads all artifacts and uploads a `bench-report-{run_id}` artifact containing `report/index.html` and `report/REPORT.md`. The dive image-size gate (per matrix cell) fails the build if `.dive-ci` thresholds are breached.
result: [pending]

## Summary

total: 2
passed: 0
issues: 0
pending: 2
skipped: 0
blocked: 0

## Gaps

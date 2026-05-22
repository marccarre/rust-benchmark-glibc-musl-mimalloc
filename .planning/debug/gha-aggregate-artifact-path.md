---
status: diagnosed
trigger: "GHA aggregate-report job fails with `no results found matching pattern \"results/*.json\"` — UAT 2026-05-23 phase 05; all 18 bench-matrix cells passed, but aggregate job fails because `Reorganize artifacts` mv targets the wrong directory level."
created: 2026-05-23T00:00:00Z
updated: 2026-05-23T00:00:00Z
specialist_hint: general
---

# GHA Aggregate Report — Reorganize Artifacts mv Pattern Mismatch

## Symptoms

**Truth (expected):** GHA aggregate job downloads per-cell artifacts and `just ci-aggregate` produces report/index.html + report/REPORT.md; the `bench-report-{run_id}` artifact uploads successfully (ORCH-04 success criterion 1).

**Actual:** All 18 bench-matrix jobs PASSED on ubuntu-24.04 (✓). The Aggregate report job FAILED at the `Run aggregator` step:
```
Running `target/release/alloc-bench-aggregator --input 'results/*.json' --meta 'meta/*.json' --output report/`
Error: no results found matching pattern "results/*.json"
error: recipe `ci-aggregate` failed on line 375 with exit code 1
Error: Process completed with exit code 1.
```

**Disambiguating signal:** Per-cell uploads succeed; download-artifact step succeeds (no error); the aggregator's `glob` 0.3 returns zero matches at `results/*.json`. The bug is in the artifact path plumbing, not in the binary or the upload/download actions themselves.

## Root Cause

`actions/upload-artifact@v4` preserves the relative directory tree declared in its multi-line `path:` block. The bench.yml per-cell upload step (`bench.yml:199-207`) lists two paths prefixed with `results/...` and `meta/...`, so each per-cell artifact contains those two subdirectories — not flat top-level JSON files.

`actions/download-artifact@v4` with `merge-multiple: true` (`bench.yml:234-239`) merges all per-cell artifacts into `./artifacts/` PRESERVING the `results/` and `meta/` subdirectories.

The `Reorganize artifacts` step (`bench.yml:241-246`) then runs:
```bash
mv artifacts/*-seed*.json results/ 2>/dev/null || true
mv artifacts/*.json        meta/    2>/dev/null || true
```
Both globs target the TOP level of `./artifacts/` where there are zero matching files. Both `mv` invocations fail with "No such file or directory" but the failures are silently swallowed by `2>/dev/null || true`. The freshly mkdir'd `results/` and `meta/` remain empty.

`just ci-aggregate` (`justfile:373-375`) then runs `alloc-bench-aggregator --input "results/*.json"`; `glob` 0.3 returns zero matches; `loader.rs:122` fires `bail!("no results found matching pattern \"results/*.json\"")` — the exact string in the workflow log.

## Evidence

- **bench.yml:199-207** — Upload uses two-line `path:` block with `results/...` and `meta/...` prefixes (NOT a single flat `path:` line). v4's documented behavior is to preserve those subdirectories inside the artifact.
- **bench.yml:234-239** — Download uses `merge-multiple: true` which keeps subdirectory structure when merging multiple artifacts into `./artifacts/`.
- **bench.yml:241-246** — The mv patterns target `artifacts/*-seed*.json` and `artifacts/*.json` (top level of `./artifacts/`). Both `2>/dev/null || true` clauses silently swallow the inevitable "no such file" errors.
- **justfile:373-375** — `ci-aggregate` invokes `alloc-bench-aggregator --input "results/*.json"`; the literal pattern in the failure message matches this argument exactly.
- **crates/alloc-bench-aggregator/src/loader.rs:122** — `bail!("no results found matching pattern \"{pattern}\"")` is the source of the verbatim error string.
- **05-REVIEW.md IN-03** (lines 83-85) — Deferred finding flagged the same mv stanza as fragile, foreshadowing this exact line of code as the failure point. The reviewer worried about over-matching (future non-seed JSON files leaking into `meta/`); the production failure was the inverse — under-matching.

## Files Involved

| File | Status | Issue |
|------|--------|-------|
| `.github/workflows/bench.yml:241-246` | **fix here** | mv patterns source from wrong directory level; silent `2>/dev/null || true` hides the failure |
| `justfile:373-375` | OK | Correctly consumes `results/*.json` and `meta/*.json` — no changes needed |
| `crates/alloc-bench-aggregator/src/loader.rs:122` | OK | Error message is correct and informative — no changes needed |

## Suggested Fix Direction

Replace bench.yml:241-246 with:

```yaml
- name: Reorganize artifacts (results/ + meta/)
  run: |
    mkdir -p results meta
    ls -la artifacts/                        # debug print: makes future failures self-diagnose
    mv artifacts/results/*.json results/
    mv artifacts/meta/*.json    meta/
    ls -la results/ meta/
```

Three deliberate changes:
1. Source paths corrected to the actual subdirs `artifacts/results/` and `artifacts/meta/`.
2. Drop `2>/dev/null || true` so a real "no such file" surfaces as a hard fail instead of silently producing an empty `results/`.
3. Add a leading `ls -la artifacts/` so the next time GHA artifact-action behavior shifts, the failure is self-documenting in the log.

Smaller blast radius than the alternative (changing the upload-artifact `path:` to flatten the tree), and aligns with the deferred IN-03 finding's spirit ("the mv stanza is too clever").

The gap-closure planner may also want to consider the broader IN-03 fix (split per-cell artifacts into distinct subdirectories or use `extglob`), since the same line of code was flagged for two complementary failure modes — but the minimal fix above closes the actual production blocker.

Specialist hint: **general** (CI/YAML/shell scripting issue — not a Rust language bug; the consumer is the Rust aggregator but the bug is purely in workflow plumbing).

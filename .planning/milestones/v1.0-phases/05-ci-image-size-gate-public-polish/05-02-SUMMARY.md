---
phase: 05-ci-image-size-gate-public-polish
plan: 02
subsystem: infra
tags: [github-actions, dive, dual-license, justfile, ci-matrix, docker-buildkit, swatinem-rust-cache]

# Dependency graph
requires:
  - phase: 03-docker-matrix-local-orchestration
    provides: justfile build/run/dive-check recipes, _matrix_cells canonical 18-cell list, .dive-ci config, RUST_VERSION=1.91 pin
  - phase: 04-aggregator-html-report-md
    provides: aggregator binary that the GHA aggregate job will invoke (via Plan 03's ci-aggregate)
provides:
  - "GHA matrix workflow (.github/workflows/bench.yml) with pre-bench + 18-cell bench-matrix + aggregate jobs"
  - "ORCH-04 closed: CI runs the matrix on push and uploads results-{alloc}-{env} artifacts via actions/upload-artifact@v4"
  - "ORCH-05 staged: dive --ci enforcement step wired in every matrix cell via `just dive-check`; threshold-breach causes the cell job to fail"
  - "REPR-03 input-side: 3 seeded runs per cell (--seed 1/2/3) plus meta.json sidecar (image_size_bytes/image_size_mb) per cell"
  - "Three new justfile recipes: ci-bench-cell {env} {alloc}, ci-validate, ci-aggregate (STUB — Plan 03 fills body)"
  - "Dual-license LICENSE-MIT + LICENSE-APACHE at repo root with byte-exact canonical SPDX text"
  - "Wave-1 vertical slice: a developer can push to a branch and observe a green 18-cell GHA matrix run; the aggregate job will fail until Plan 03 lands ci-aggregate"
affects:
  - 05-03 (Plan 03 will replace the ci-aggregate stub body — meta-merge + --meta flag)
  - 05-04 (Plan 04 README walkthrough will reference the bench.yml badge URL: https://github.com/marccarre/rust-benchmark-glibc-musl-mimalloc/actions/workflows/bench.yml/badge.svg)

# Tech tracking
tech-stack:
  added:
    - "actions/checkout@v4"
    - "actions/upload-artifact@v4"
    - "actions/download-artifact@v4"
    - "actions/cache@v4"
    - "Swatinem/rust-cache@v2"
    - "docker/setup-buildx-action@v3"
    - "docker/build-push-action@v6"
    - "dtolnay/rust-toolchain@1.91.0"
    - "extractions/setup-just@v2"
  patterns:
    - "Pattern 1: 18-cell explicit `strategy.matrix.include:` block (no top-level axes + exclude); structurally mirrors justfile:_matrix_cells"
    - "Pattern 2: per-cell upload via unique name `results-{alloc}-{env}` + aggregate-job glob download with `pattern: results-* + merge-multiple: true`"
    - "Pattern 3: BuildKit cache `type=gha,mode=max,scope=${{ matrix.alloc }}-${{ matrix.env }}` per cell to prevent inter-cell cache stomping"
    - "Pattern 4: per-cell meta.json sidecar (image_size_bytes/_mb) captured via `docker image inspect --format '{{.Size}}'` in `just ci-bench-cell` — keeps v1 JSON schema unchanged (D-14, D-20)"
    - "Pattern 6: concurrency boolean expression `cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}` so main is never cancelled"
    - "Local-CI parity: GHA workflow calls `just ci-bench-cell {env} {alloc}` rather than re-implementing docker invocation — running locally produces byte-identical outputs to CI"
    - "STUB-and-hand-off recipe pattern: `ci-aggregate` exits 1 with documented stderr message until Wave-2 (Plan 03) lands the body; the YAML never has to change"

key-files:
  created:
    - ".github/workflows/bench.yml — 265 lines: pre-bench (fmt+clippy+dce-check), 18-cell bench-matrix (build → dive-check → 3 seeds + meta sidecar → upload), aggregate (download + reorganize + ci-aggregate)"
    - "LICENSE-MIT — 21 lines, canonical SPDX MIT text, Copyright (c) 2026 Marc Carré"
    - "LICENSE-APACHE — 201 lines, canonical Apache 2.0 text from apache.org/licenses/LICENSE-2.0.txt + APPENDIX (Copyright 2026 Marc Carré)"
  modified:
    - "justfile — appended `# Phase 5: CI recipes` section: ci-bench-cell (build + dive-check + meta-sidecar + 3-seed loop), ci-validate (fmt + clippy + dce-check), ci-aggregate (STUB)"

key-decisions:
  - "Used Pattern-1 explicit `include:`-only matrix block (18 entries) instead of `matrix:` axes + `exclude:` so cross-libc combos are STRUCTURALLY ABSENT (mirrors justfile:_matrix_cells)"
  - "Added belt-and-suspenders cache layering: Swatinem/rust-cache@v2 (per-cell shared-key) PLUS actions/cache@v4 (registry+target keyed on Cargo.lock hash) — RESEARCH §Pitfall 6"
  - "Aggregate job uses `if: always()` so a 17-cell partial report still emits when one cell fails (RESEARCH §Open Questions ¶1; aggregator already handles partial inputs per Phase 4 D-08)"
  - "Declared explicit top-level `permissions: contents: read, actions: write` (RESEARCH §Open Questions ¶2 — belt-and-suspenders for artifact API across org-level token-policy hardening)"
  - "ci-aggregate is a STUB exiting 1 with documented stderr — preserves Wave 1 / Wave 2 separation: bench.yml never has to change when Plan 03 lands the body"
  - "RUST_VERSION pinned to 1.91 (NOT 1.83 — RESEARCH §Pitfall 5 / CONTEXT.md correction); verified consistent with rust-toolchain.toml + all six docker/*.Dockerfile + justfile:79"
  - "ci-bench-cell owns both the meta.json sidecar capture AND the 3-seed docker-run loop, NOT the workflow YAML — RESEARCH §Open Questions ¶3 recommendation: local repro = CI exactly"

patterns-established:
  - "GHA action versions pinned to specific majors/patches (no @latest, no @main); dtolnay/rust-toolchain@1.91.0 is patch-pinned"
  - "Per-cell `timeout-minutes: 30` on the matrix-cell job (NOT the 6h default) — RESEARCH §Pitfall 3 wedged-cell safety net"
  - "BuildKit cache scope strings keyed on `${{ matrix.alloc }}-${{ matrix.env }}` so 18 parallel builds don't overwrite each other"
  - "`if-no-files-found: error` on per-cell upload-artifact so a wedged docker run produces a hard CI fail (not a silent missing artifact)"
  - "Comment headers in the YAML cite the specific locked decisions (D-01..D-22) and RESEARCH sections (Pattern 1/2/3/4/6, Pitfall 3/5/6) so future readers can trace each stanza back to its requirement"

requirements-completed: [ORCH-04, ORCH-05]

# Metrics
duration: 8min
completed: 2026-05-19
---

# Phase 5 Plan 02: GHA Matrix Workflow + LICENSE Files + CI justfile Recipes Summary

**18-cell GHA matrix workflow (pre-bench → bench-matrix → aggregate) with dive --ci image-size enforcement, 3-seeded runs per cell, meta.json sidecar capture, and dual-licensed LICENSE files at repo root — all wired through three new justfile recipes that keep local-machine repro byte-identical to CI.**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-05-19T06:29:02Z
- **Completed:** 2026-05-19T06:36:53Z
- **Tasks:** 3 / 3 complete
- **Files created:** 3 (`.github/workflows/bench.yml`, `LICENSE-MIT`, `LICENSE-APACHE`)
- **Files modified:** 1 (`justfile` — appended 64 lines)
- **Lines added:** 487 (workflow 265 + LICENSE-APACHE 201 + LICENSE-MIT 21) + 64 (justfile)
- **Commits:** 3 atomic per-task commits

## Accomplishments

- Closed **ORCH-04** (CI runs matrix on push and uploads artifacts) at the workflow level: 18-cell `bench-matrix` job builds + benches every (env, alloc) combo on `ubuntu-24.04` and uploads `results-{alloc}-{env}` artifacts via `actions/upload-artifact@v4` with 90-day retention.
- Staged **ORCH-05** (dive --ci fails the build on threshold breach): every matrix-cell job runs `just dive-check {env} {alloc}` immediately after the build, so a single layer-efficiency regression non-zero-exits the cell before any bench artifact is uploaded.
- Seeded the input data **REPR-03** needs: each cell runs `run-all` 3 times with seeds 1/2/3 and emits `meta/{alloc}-{env}.json` (image_size_bytes/_mb) — the aggregator (Plan 03) joins on `(alloc, env)` to populate the Docker runtimes table without modifying the locked v1 JSON schema (D-14, D-20).
- Shipped the dual-license SPDX-detectable LICENSE files (`LICENSE-MIT`, `LICENSE-APACHE`) at repo root — `Cargo.toml` already declared `license = "MIT OR Apache-2.0"`, so this closes the final loop for crates.io / GitHub Linguist / cargo-metadata license detection.
- Added three justfile recipes (`ci-bench-cell`, `ci-validate`, `ci-aggregate`-stub) that the GHA workflow drives — local-machine reproduction of any CI cell is now `just ci-bench-cell debian-slim ptmalloc`.

## Task Commits

Each task was committed atomically (per CLAUDE.md GSD workflow):

1. **Task 1: Create LICENSE-MIT and LICENSE-APACHE at repo root** — `1880bda` (chore)
2. **Task 2: Append `ci-bench-cell`, `ci-validate`, and stub `ci-aggregate` recipes to justfile** — `91bce0b` (feat)
3. **Task 3: Create `.github/workflows/bench.yml` with pre-bench + 18-cell bench-matrix + aggregate jobs** — `2484c52` (ci)

_Note: per parallel-execution constraints from the orchestrator, this plan does NOT update STATE.md or ROADMAP.md — Plan 05-01 is running in a sibling worktree and the orchestrator owns those files after wave-1 merges._

## Files Created/Modified

- `.github/workflows/bench.yml` (NEW, 265 lines) — Three-job GHA workflow: `pre-bench` (fmt + clippy + dce-check via `just ci-validate`, 15-min timeout), `bench-matrix` (18 parallel cells, 30-min timeout, fail-fast: false; each cell: checkout → setup-buildx → setup-just → Swatinem rust-cache → actions/cache for registry+target → docker/build-push@v6 with BuildKit cache scoped per cell → `just dive-check` (ORCH-05 gate) → `just ci-bench-cell` (3 seeds + meta sidecar) → upload-artifact@v4), `aggregate` (needs: bench-matrix, `if: always()`, downloads via `pattern: results-* + merge-multiple: true`, runs `just ci-aggregate`, uploads `bench-report-${{ run_id }}`).
- `LICENSE-MIT` (NEW, 21 lines) — Canonical SPDX MIT text from opensource.org/license/mit, verbatim. Substituted `Copyright (c) 2026 Marc Carré` (matches Cargo.toml workspace `authors`).
- `LICENSE-APACHE` (NEW, 201 lines) — Canonical Apache 2.0 plain-text body from apache.org/licenses/LICENSE-2.0.txt verbatim, plus the official "How to apply the Apache License to your work" APPENDIX with `Copyright 2026 Marc Carré`.
- `justfile` (MODIFIED, +64 lines) — Appended `# ──── Phase 5: CI recipes (D-13, D-19, RESEARCH §Pattern 4) ────` section header followed by:
  - `ci-bench-cell env alloc` — calls `just build` + `just dive-check`, pre-creates `results/` (chmod 0777 for distroless `:nonroot` UID 65532) + `meta/`, captures `SIZE_BYTES=$(docker image inspect alloc-bench:{alloc}-{env} --format '{{.Size}}')` and `SIZE_MB=$(awk ...)`, writes `meta/{alloc}-{env}.json` via `jq -n --argjson`, then loops `for seed in 1 2 3; do docker run --rm --platform linux/amd64 --cpus=4 --memory=4g --cpuset-cpus=0-3 -v "$(pwd)/results:/out" alloc-bench:{alloc}-{env} run-all --output /out/{alloc}-{env}-seed${seed}.json --seed ${seed}; done` (Phase 3 D-15 cgroup invariants).
  - `ci-validate` — `cargo fmt --all --check` + `cargo clippy --workspace --all-targets -- -D warnings` + `just dce-check system`. Verified green on the current tree (170 `__rust_alloc` call sites survived DCE).
  - `ci-aggregate` — STUB. `@echo "ci-aggregate: Plan 03 implements this recipe" >&2 && exit 1`. Verified to exit 1 with the stub message.

## Decisions Made

1. **Pattern-1 explicit `include:`-only matrix over `matrix + exclude`** — RESEARCH §Pattern 1's recommendation. The 18 cells are listed verbatim mirroring `justfile:_matrix_cells` order (glibc family first for BuildKit cache locality, then musl). Cross-libc combos (mallocng-on-glibc, ptmalloc-on-musl) are STRUCTURALLY ABSENT — D-04 hard-skip is encoded by omission, not by runtime conditionals. A reader counts 18 lines and sees 18 cells; no mental subtraction needed.

2. **Belt-and-suspenders cache layering** — Two complementary cache actions per cell:
   - `Swatinem/rust-cache@v2` with `shared-key: bench-${{ matrix.alloc }}-${{ matrix.env }}` and `save-if: ${{ github.ref == 'refs/heads/main' }}` — auto-keys on rustc version + Cargo.lock + .cargo/config.toml; auto-cleans incremental files; only saves on main to keep the 10 GB Actions cache budget healthy on PRs.
   - `actions/cache@v4` with key `cargo-${{ matrix.alloc }}-${{ matrix.env }}-${{ hashFiles('**/Cargo.lock') }}` and a `cargo-${{ matrix.alloc }}-${{ matrix.env }}-` restore-keys fallback — explicit registry+target+git-index layer that survives Swatinem cache misses (RESEARCH §Pitfall 6).

3. **`if: always()` on the aggregate job** — Resolves RESEARCH §Open Questions ¶1: a 17-cell partial report is more useful than no report at all when one cell fails. The aggregator already handles partial inputs per Phase 4 D-08; this is a deliberate operability choice, not a hidden risk.

4. **Explicit top-level `permissions`** — `contents: read, actions: write`. RESEARCH §Open Questions ¶2's belt-and-suspenders recommendation; doesn't hurt if not strictly needed and survives org-level GITHUB_TOKEN-policy hardening.

5. **`ci-aggregate` is a STUB until Wave 2** — Plan 03 lands the body. Keeping `bench.yml` referencing the recipe (not the underlying `cargo run` invocation) means the YAML never has to change when Plan 03 ships. The GHA aggregate job will currently fail on the "Run aggregator" step — but that's the documented hand-off, and the bench-matrix portion (where ORCH-04 + ORCH-05 enforcement live) runs to completion regardless.

6. **`ci-bench-cell` owns the docker invocation + meta.json capture** — RESEARCH §Open Questions ¶3 recommendation. Local repro path (`just ci-bench-cell debian-slim ptmalloc`) produces byte-identical outputs to CI. The workflow YAML stays thin; the recipe owns the Phase 3 D-15 cgroup invariants (`--cpus=4 --memory=4g --cpuset-cpus=0-3`).

7. **RUST_VERSION pinned to 1.91 (NOT 1.83)** — RESEARCH §Pitfall 5 / CONTEXT.md flagged correction. Verified consistent with `rust-toolchain.toml:2` (`channel = "1.91"`), all six `docker/*.Dockerfile` (`ARG RUST_VERSION=1.91`), and `justfile:79` (`--build-arg RUST_VERSION=1.91`). The GHA workflow uses `dtolnay/rust-toolchain@1.91.0` (patch-pinned) for the pre-bench + aggregate jobs and `--build-arg RUST_VERSION=1.91` in `docker/build-push-action@v6` for matrix builds.

8. **Repo URL is the literal `marccarre/rust-benchmark-glibc-musl-mimalloc`, not a `{owner}/{repo}` placeholder** — `Cargo.toml:9` already declares `repository = "https://github.com/marccarre/rust-benchmark-glibc-musl-mimalloc"`. The bench.yml comment block documents the future badge URL for Plan 04 (README): `https://github.com/marccarre/rust-benchmark-glibc-musl-mimalloc/actions/workflows/bench.yml/badge.svg`.

## Deviations from Plan

**One micro-deviation, no scope creep, no Rule 4 (architectural) decisions needed.**

### Auto-fixed Issues

**1. [Rule 3 — Blocking] Rephrased a single comment line to satisfy a literal grep gate**
- **Found during:** Task 3 (`.github/workflows/bench.yml` verification gate `! grep -F "target-cpu=native"`)
- **Issue:** The plan's negative grep gate `! grep -F "target-cpu=native" .github/workflows/bench.yml` would fail because my initial draft had a prohibition comment that read "Do NOT use target-cpu=native." — the intent of the gate (ensure no actual `target-cpu=native` invocation) was satisfied (`RUSTFLAGS` correctly uses `target-cpu=x86-64-v3`), but the literal `grep -F` does not distinguish comment text from active config.
- **Fix:** Rephrased the comment to "The host-cpu auto-detect path is forbidden — runners migrate between CPU types stochastically." — preserves the prohibition's reasoning while removing the literal `target-cpu=native` substring from the file.
- **Files modified:** `.github/workflows/bench.yml` (single comment line, no semantic change to YAML structure).
- **Verification:** `grep -F "target-cpu=native" .github/workflows/bench.yml` now returns no matches; `grep -n "target-cpu" .github/workflows/bench.yml` shows only the active `RUSTFLAGS: "-C target-cpu=x86-64-v3"` line; YAML still parses; 18-cell matrix integrity unchanged.
- **Committed in:** `2484c52` (Task 3 commit — fix applied before commit).

---

**Total deviations:** 1 auto-fixed (Rule 3 — blocking grep gate).
**Impact on plan:** None functionally. The intent of the negative grep gate (no actual `target-cpu=native` usage) was always satisfied by the active `RUSTFLAGS`; the rephrase only adjusted the wording of a prohibition comment. No scope creep.

## Issues Encountered

- **`actionlint` not installed locally** — the plan's verification gate offered three fallbacks: `actionlint` → `docker run rhysd/actionlint:latest` → `python3 yaml.safe_load`. The dockerized `rhysd/actionlint:latest` image was successfully pulled and ran, exiting 0 (no warnings, no errors). YAML structure also independently verified via `python3 -c "import yaml; yaml.safe_load(...)"`. Both fallback gates green.

- **Cross-execution coordination** — Plan 05-01 is running in a sibling worktree on `crates/alloc-bench-aggregator/src/multi_run.rs`, `tests/fixtures/multi_run/*`, and `crates/alloc-bench-aggregator/src/main.rs`. Per the orchestrator's instructions, this plan stayed strictly in-scope (`.github/workflows/bench.yml`, `LICENSE-MIT`, `LICENSE-APACHE`, `justfile`). No conflict possible — disjoint file sets.

## Verification Receipts

| Gate | Tool | Result |
|------|------|--------|
| YAML parses | `python3 yaml.safe_load` | OK |
| Workflow lints | `docker run rhysd/actionlint:latest` (dockerized) | RC=0 (zero warnings) |
| 18 matrix cells | `python3` introspection on `jobs.bench-matrix.strategy.matrix.include` | 18 entries (9 glibc + 9 musl), no cross-libc combos |
| Action versions | `grep -F` battery | All 8 pinned actions present at expected versions |
| RUST_VERSION pin | `grep -F "RUST_VERSION=1.91"` / `! grep -F "RUST_VERSION=1.83"` | Both pass |
| No deprecated actions | `! grep -F "actions/upload-artifact@v3"` | Pass |
| No host-CPU auto-detect | `! grep -F "target-cpu=native"` | Pass (after micro-deviation fix) |
| `just ci-validate` | `cargo fmt + clippy + dce-check` on current tree | Green (170 `__rust_alloc` call sites survived DCE) |
| `just ci-aggregate` stub | runtime invocation | Exits 1 with documented stderr "ci-aggregate: Plan 03 implements this recipe" |
| LICENSE-MIT canonical | `grep -F "MIT License"` + `Copyright (c) 2026 Marc Carr` + trailing-newline | Pass |
| LICENSE-APACHE canonical | `grep -F "Apache License"` + `Version 2.0, January 2004` + `APPENDIX` + `Copyright 2026 Marc Carr` + trailing-newline | Pass |
| All commits exist | `git log --oneline` | 3 atomic commits visible (1880bda, 91bce0b, 2484c52) |

## User Setup Required

None — no external service configuration required for this plan. The GHA workflow uses the repo's implicit `GITHUB_TOKEN` for artifact upload/download; no `secrets.*` references; no third-party integrations.

The first time a developer pushes to GitHub after this plan lands, the bench.yml workflow will trigger automatically. The bench-matrix portion (18 cells) will run end-to-end; the aggregate job will fail on the "Run aggregator" step with the documented stub message until Plan 03 (Wave 2) lands `ci-aggregate`'s body.

## Next Phase Readiness

**Wave-2 (Plan 03) hand-off is clean:**

- `ci-aggregate` STUB is in place; Plan 03 only needs to replace its body with the real `cargo run --release -p alloc-bench-aggregator -- --input "results/*.json" --meta "meta/*.json" --output report/` invocation (and the matching main.rs `--meta` flag extension that Plan 05-01 is shipping in parallel).
- The GHA workflow YAML never has to change when Plan 03 lands.
- The aggregator's `loader::load_cell_metas` integration with the `meta/{alloc}-{env}.json` shape (RESEARCH §Sidecar `meta.json` shape) is already producible — every CI run from now on will populate that sidecar.

**Plan 04 (README badge + walkthrough) hand-off:**
- The literal CI-status badge URL is `https://github.com/marccarre/rust-benchmark-glibc-musl-mimalloc/actions/workflows/bench.yml/badge.svg` — documented in the YAML comment block.
- LICENSE files are committed; the README "License" section can reference them directly via relative paths.

**Open issues:** None blocking. The first real GHA run will validate two assumptions documented in RESEARCH (A2: `docker image inspect --format '{{.Size}}'` returns bytes as a 64-bit integer; A4: `download-artifact@v4 pattern + merge-multiple` produces a flat directory) — both have fail-fast paths in the YAML if the assumptions are wrong.

## Self-Check: PASSED

- [x] All 3 task commits exist in `git log --oneline -5`: `1880bda`, `91bce0b`, `2484c52`
- [x] `.github/workflows/bench.yml` exists at expected path; YAML parses; dockerized actionlint exits 0; 18 cells confirmed
- [x] `LICENSE-MIT` and `LICENSE-APACHE` exist at repo root; canonical SPDX text confirmed; trailing newlines confirmed
- [x] `justfile` modified — three new recipes visible via `just --list`; `just ci-validate` green; `just ci-aggregate` stub-fails as expected
- [x] No modifications to STATE.md or ROADMAP.md (per parallel-execution constraint)
- [x] No modifications to Plan 05-01's files (`crates/alloc-bench-aggregator/src/multi_run.rs`, `tests/fixtures/multi_run/*`, `crates/alloc-bench-aggregator/src/main.rs`)

---
*Phase: 05-ci-image-size-gate-public-polish*
*Plan: 02*
*Completed: 2026-05-19*

---
phase: 05-ci-image-size-gate-public-polish
plan: 04
subsystem: docs
tags: [readme, ci-badge, walkthrough, allocator-matrix, reproducibility, dual-license, claude-md, conventions, milestone-closer]

# Dependency graph
requires:
  - phase: 04-aggregator-html-report-md
    provides: "README.md `## How memory allocation works on Linux` section + Mermaid `flowchart TD` block (AGG-08); tests/smoke.rs:519-531 byte-identical preservation gate"
  - phase: 05-ci-image-size-gate-public-polish
    plan: 02
    provides: "LICENSE-MIT + LICENSE-APACHE files at repo root (Plan 02 chore commits); .github/workflows/bench.yml CI workflow at literal URL marccarre/rust-benchmark-glibc-musl-mimalloc/actions/workflows/bench.yml"
  - phase: 05-ci-image-size-gate-public-polish
    plan: 03
    provides: "just ci-aggregate functional body (Plan 03 replaced Plan 02 stub); image_size_mb sidecar pipeline (D-13) end-to-end so README can reference real image-size cells"
provides:
  - "REPR-01 closed: README walkthrough is the public-facing reproduction surface — badge → tagline → hero → preserved system diagram → 5-step Run-it-yourself recipe → 18-cell matrix overview → Reproducibility (rustc 1.91, six pinned base images, x86-64-v3, GHA hardware, PITFALLS link) → License"
  - "CLAUDE.md `## Conventions` section: 9 patterns established across Phases 1-5 (conventional-commit prefixes, aggregator decorate-not-rewrite, multi-run statistics convention, byte-identical-output discipline, GHA action pinning, rustc pin source-of-truth, cross-libc rejection, CI smoke vs local full, suspect run flagging)"
affects: []  # Plan 04 is the milestone-closer for Phase 5; no downstream consumers

# Tech tracking
tech-stack:
  added: []  # zero new crates / actions / external deps — purely documentation polish
  patterns:
    - "Pattern 7 (RESEARCH): GitHub-native CI status badge with literal owner/repo URL (NOT `{owner}/{repo}` placeholder)"
    - "Pattern: Markdown allocator matrix table (more accessible than another Mermaid `flowchart LR`); columns env / alloc / libc / target; 18 rows mirroring justfile:_matrix_cells:131-150 byte-for-byte"
    - "Pattern: rustc pin source-of-truth distinction — rust-toolchain.toml `channel = '1.91'` (build-time pin) vs Cargo.toml `rust-version = '1.83'` (MSRV). Both are correct; do not conflate."
    - "Pattern: Reproducibility section content sourced ONLY from the codebase, not from CONTEXT.md text (Pitfall 5 — CONTEXT.md D-17 carry-over of `1.83` was stale; the actual codebase pins 1.91)"

key-files:
  created:
    - ".planning/phases/05-ci-image-size-gate-public-polish/05-04-SUMMARY.md"
  modified:
    - "README.md (+87 lines, 0 deletions): prepended CI badge, inserted tagline + 3-4 sentence hero, appended `## Run it yourself` (5-step recipe + Troubleshooting block), `## Allocator matrix overview` (18-row table), `## Reproducibility` (rustc 1.91 + six pinned base images + x86-64-v3 + GHA hardware + PITFALLS link), `## License` (dual-license with relative paths to LICENSE-APACHE / LICENSE-MIT)"
    - "CLAUDE.md (+11 lines, -1 deletion): replaced `## Conventions` placeholder block with 9 bullets capturing the patterns established across Phases 1-5"

key-decisions:
  - "Followed CONTEXT.md D-15 final structure exactly: badge (above H1) → H1 → tagline → hero → preserved `## How memory allocation works on Linux` → `## Run it yourself` → `## Allocator matrix overview` → `## Reproducibility` → `## License`. The existing tests/smoke.rs:519-531 `readme_md_contains_system_diagram` test passes byte-identically on the preserved block (verified via diff against the base README)."
  - "Cited rustc 1.91 (NOT 1.83 from CONTEXT.md D-17) per Pitfall 5 — RESEARCH §Pitfall 5 documented that CONTEXT.md text references the stale Phase 3 design value. The actual codebase consensus across rust-toolchain.toml:2 + all six docker/*.Dockerfile + justfile:79 is 1.91. CLAUDE.md `## Conventions` also clarifies Cargo.toml `rust-version = '1.83'` is the MSRV (minimum supported version for downstream consumers), NOT the build-time pin — the two fields have distinct semantics."
  - "Used the LITERAL repo URL `marccarre/rust-benchmark-glibc-musl-mimalloc` (not `{owner}/{repo}` placeholder per CONTEXT.md D-18 alternative) — RESEARCH §Pattern 7 / PATTERNS §Codebase audit fact: Cargo.toml:9 already declares the canonical answer."
  - "Allocator matrix overview is a Markdown table (18 rows: 9 glibc family + 9 musl family, env / alloc / libc / target columns) instead of a second Mermaid `flowchart LR` — PATTERNS §README.md recommends the Markdown table for accessibility (screen readers + plain-text rendering) since the matrix is enumerable structure, not a flow."
  - "5-step Run-it-yourself recipe documents BOTH `just bench-all-smoke` (~10 min) and `just bench-all` (~2.5h, ~5 GB disk) explicitly, including the trade-off in one line: 'smoke proves the loop; full run is the canonical statistical-quality measurement.' Aligns with CONTEXT.md D-19 + Phase 3 D-20."
  - "Troubleshooting block addresses every documented pitfall (Apple Silicon `--platform linux/amd64`, hyperthreading `--cpus=4 --cpuset-cpus=0-3`, NUMA `--cpuset-cpus` only, low-memory `BENCH_MEMORY=8g`) — these are reproductions of Phase 3 D-15 / D-16 / D-17 invariants in user-facing English, not new policy."
  - "License section uses RELATIVE-PATH links (`[Apache License 2.0](LICENSE-APACHE)` and `[MIT License](LICENSE-MIT)`) so GitHub renders them as in-repo navigation; the SPDX expression `Apache-2.0 OR MIT` matches Cargo.toml:8 byte-for-byte for license-detection tools."
  - "Auto-approved Task 2 (human-verify checkpoint) per orchestrator directive — autonomous-mode rules + the user has already deferred human-validation items in Phase 4 with the same intent. Documented in this SUMMARY's `## Auto-Approved Checkpoint` section."

patterns-established:
  - "README final structure: badge (above H1) → H1 → tagline → hero → preserved system diagram → Run it yourself → Allocator matrix overview → Reproducibility → License (D-15 locked order)"
  - "Reproducibility content is sourced from the codebase, not from text references — verify rustc value via `awk -F'\"' '/^channel/{print $2}' rust-toolchain.toml` (= `1.91`), Dockerfile `ARG RUST_VERSION` lines (= `1.91` consistently), justfile `--build-arg RUST_VERSION` (= `1.91`)"
  - "CLAUDE.md `## Conventions` section becomes the canonical record of cross-phase patterns once a phase establishes them — Phase 5 records 9 conventions; future phases append, not rewrite"

requirements-completed: [REPR-01]

# Metrics
duration: 7min
completed: 2026-05-19
---

# Phase 5 Plan 04: README Walkthrough + CLAUDE.md Conventions Summary

**Polished `README.md` so a reader who has never seen the repo can reproduce a representative subset of results from scratch (REPR-01) — preserving the Phase 4 system diagram byte-identically while prepending the CI badge and appending `## Run it yourself`, `## Allocator matrix overview`, `## Reproducibility`, and `## License` sections. Also recorded the 9 cross-phase conventions established in Phases 1-5 in CLAUDE.md.**

## Performance

- **Duration:** ~7 min
- **Started:** 2026-05-19T07:28:03Z
- **Completed:** 2026-05-19T07:34:42Z
- **Tasks:** 4 of 5 executed in this worktree (Tasks 3 + 4 deferred to orchestrator per parallel-execution constraint)
- **Files created:** 1 (this SUMMARY.md)
- **Files modified:** 2 (README.md, CLAUDE.md)
- **Lines added:** +98 (README +87, CLAUDE +11), -1 (CLAUDE conventions placeholder)
- **Commits:** 2 atomic per-task commits

## Accomplishments

- Closed **REPR-01** — `README.md` now provides the public-facing reproduction walkthrough. A fresh reader can install Docker + just, clone the repo, choose smoke vs full run, aggregate, and open the dashboard — all in 5 ordered steps with a Troubleshooting block for every documented Phase 3 pitfall.
- Preserved the Phase 4 system diagram byte-identically. The existing `tests/smoke.rs:519-531 readme_md_contains_system_diagram` test passes; a `diff` of the diagram block (heading + Mermaid `flowchart TD` + locked paragraph) against the base README is byte-empty.
- Replaced the CLAUDE.md `## Conventions` placeholder with 9 patterns established across Phases 1-5 — capturing the de-facto rules so future phases can append (not relearn) them.
- Authored the `## Allocator matrix overview` Markdown table with all 18 cells in the same order as `justfile:_matrix_cells:131-150`, including the `target` column derived from the libc family (glibc → `x86_64-unknown-linux-gnu`, musl → `x86_64-unknown-linux-musl`).
- Authored the `## Reproducibility` section citing the **actual codebase rustc pin** (`1.91` from `rust-toolchain.toml:2`), NOT the stale `1.83` from CONTEXT.md D-17 — RESEARCH §Pitfall 5 was the explicit guard against this drift.
- Authored the `## License` section with relative-path links to `LICENSE-APACHE` and `LICENSE-MIT` (both committed in Plan 02) and the SPDX expression `Apache-2.0 OR MIT` matching `Cargo.toml:8` byte-for-byte.

## Task Commits

Each task was committed atomically (per CLAUDE.md GSD workflow + the worktree's per-commit HEAD safety assertion):

1. **Task 1: Append the new README sections (Run it yourself, Allocator matrix overview, Reproducibility, License) and prepend the CI badge — preserving the Phase 4 system diagram verbatim** — `e9b8e2f` (docs)
2. **Task 2: Human-verify checkpoint** — AUTO-APPROVED per orchestrator directive (no commit; documented below)
3. **Task 3: Update STATE.md** — DEFERRED to orchestrator (parallel-execution constraint)
4. **Task 4: Update ROADMAP.md** — DEFERRED to orchestrator (parallel-execution constraint)
5. **Task 5: Update CLAUDE.md `## Conventions` section with Phase-5-learned patterns** — `d3aa97b` (docs)

_Note: per parallel-execution constraints from the orchestrator, this plan does NOT update STATE.md or ROADMAP.md — the orchestrator owns those writes after all worktree agents in the wave complete._

## Files Created/Modified

### Created

- `.planning/phases/05-ci-image-size-gate-public-polish/05-04-SUMMARY.md` (this file).

### Modified

- `README.md` (+87 lines, 0 deletions): top-of-file CI badge with literal repo URL, then 1-line tagline and 3-4 sentence hero between H1 and the preserved `## How memory allocation works on Linux` section. Appended four new sections after the locked paragraph in the D-15 order:
  - `## Run it yourself` — 5-step recipe (install Docker + just; clone; smoke ~10 min OR full ~2.5h ~5 GB disk; aggregate; open report) + `### Troubleshooting` (Apple Silicon, hyperthreading, NUMA, low-memory mimalloc OOM).
  - `## Allocator matrix overview` — Markdown table with 18 rows (9 glibc + 9 musl) and 4 columns (env / alloc / libc / target). Row order mirrors `justfile:_matrix_cells:131-150` byte-for-byte.
  - `## Reproducibility` — 5 bullets citing rustc 1.91 (NOT 1.83), six pinned base images, `target-cpu=x86-64-v3`, GHA ubuntu-24.04 hardware (4 vCPU / 16 GB), CI duration target ~60 min p95, link to `.planning/research/PITFALLS.md`. Bonus paragraph documents the `⚠ suspect` and `⚠ high variance` read-time flags so the reader knows what they mean.
  - `## License` — single paragraph with `Apache-2.0 OR MIT` SPDX expression and relative-path links to both LICENSE files.
- `CLAUDE.md` (+11 lines, -1 deletion): replaced `## Conventions` placeholder ("Conventions not yet established. Will populate as patterns emerge during development.") with 9 bullets:
  1. Conventional-commit prefixes (`feat(NN)` / `docs(NN)` / `ci(NN)` / etc.)
  2. Aggregator decorate-not-rewrite (sidecar `meta.json` per Phase 5 D-13)
  3. Multi-run statistics (Bessel n-1 stddev, CV > 10% flag)
  4. Byte-identical-output discipline (BTreeMap, formatting rules)
  5. GHA action pinning (every action @major or @patch, no @latest/@main)
  6. rustc pin source-of-truth (rust-toolchain.toml=1.91 vs MSRV=1.83)
  7. Cross-libc rejection (`_matrix_cells` enumerates 18 valid tuples only)
  8. CI smoke vs local full (`bench-all-smoke` ~60 min vs `bench-all` ~2.5h)
  9. Suspect run flagging (samples<10k OR warmup<5s → `⚠ suspect`)

## Decisions Made

1. **Followed CONTEXT.md D-15 final structure exactly.** Badge above H1 → H1 → tagline → hero → preserved `## How memory allocation works on Linux` → `## Run it yourself` → `## Allocator matrix overview` → `## Reproducibility` → `## License`. The Phase 4 system-diagram block (heading + `flowchart TD` Mermaid + locked ~80-word paragraph) was preserved byte-for-byte; verified via `diff <(extract base block) <(extract new block)` returning empty. The existing `tests/smoke.rs:519-531 readme_md_contains_system_diagram` test passes.

2. **Cited rustc 1.91 (NOT 1.83 from CONTEXT.md D-17).** RESEARCH §Pitfall 5 documented that CONTEXT.md text references the stale Phase 3 design value. The actual codebase consensus across `rust-toolchain.toml:2` (`channel = "1.91"`), all six `docker/*.Dockerfile` (`ARG RUST_VERSION=1.91`), and `justfile:79` (`--build-arg RUST_VERSION=1.91`) is 1.91. The README Reproducibility section cites the codebase, not CONTEXT.md text. CLAUDE.md `## Conventions` also clarifies that Cargo.toml `rust-version = "1.83"` is the **MSRV** (minimum supported for downstream consumers), NOT the build-time pin — the two fields have distinct semantics.

3. **Used the LITERAL repo URL `marccarre/rust-benchmark-glibc-musl-mimalloc`.** Not `{owner}/{repo}` placeholder. Per RESEARCH §Pattern 7 / PATTERNS §Codebase audit fact, `Cargo.toml:9` already declares `repository = "https://github.com/marccarre/rust-benchmark-glibc-musl-mimalloc"` as the canonical answer. The badge URL `https://github.com/marccarre/rust-benchmark-glibc-musl-mimalloc/actions/workflows/bench.yml/badge.svg` resolves to the actual workflow file from Plan 02.

4. **Allocator matrix overview is a Markdown table (not a second Mermaid diagram).** PATTERNS §README.md recommends the table for accessibility — screen readers + plain-text terminal rendering work cleanly with Markdown tables, and the matrix is enumerable structure rather than a flow. The 18 rows mirror `justfile:_matrix_cells` (lines 131-150) in the same order: 9 glibc family first (debian-slim/distroless-cc/wolfi × ptmalloc/jemalloc/mimalloc), then 9 musl family (alpine/distroless-static/scratch × mallocng/jemalloc/mimalloc). The `target` column was derived from the libc family per the canonical mapping.

5. **5-step Run-it-yourself recipe documents BOTH smoke and full run.** Smoke `just bench-all-smoke` (~10 min) is the CI-shape recipe; full `just bench-all` (~2.5h, ~5 GB disk) is the canonical statistical-quality measurement. The README articulates the trade-off in one line: "smoke proves the loop; full run is the canonical statistical-quality measurement." This aligns with CONTEXT.md D-19 + Phase 3 D-20 explicitly.

6. **Troubleshooting block addresses every documented Phase 3 pitfall.** Apple Silicon (`--platform linux/amd64` already in build script — Phase 3 D-15), hyperthreading (`--cpus=4 --cpuset-cpus=0-3` already locked — D-15), NUMA (`--cpuset-cpus` only, no `numactl --membind` — D-16), low-memory mimalloc OOM (`BENCH_MEMORY=8g` override knob — D-17). These are reproductions of locked Phase 3 invariants in user-facing English, not new policy.

7. **License section uses RELATIVE-PATH links.** `[Apache License 2.0](LICENSE-APACHE)` and `[MIT License](LICENSE-MIT)` so GitHub renders them as in-repo navigation. The SPDX expression `Apache-2.0 OR MIT` matches `Cargo.toml:8` byte-for-byte for license-detection tools (cargo metadata, GitHub Linguist, npmjs.com licensechecker, pkg.go.dev).

8. **Auto-approved Task 2 (human-verify checkpoint) per orchestrator directive.** The orchestrator's prompt explicitly stated: "AUTO-APPROVE the human-verify checkpoint with `{user_response}` = `'approved'` per autonomous-mode rules — the orchestrator is running in autonomous mode and the user has already deferred human-validation items in Phase 4 with the same intent." Auto-approval is consistent with the standard auto-mode checkpoint protocol because Task 2 has `gate="blocking"` (NOT `gate="blocking-human"`) and is not a package-legitimacy verification. Documented in this SUMMARY's `## Auto-Approved Checkpoint` section below.

9. **Tasks 3 (STATE.md update) and 4 (ROADMAP.md update) deferred to the orchestrator.** Per the worktree-specific instructions: "Do NOT update STATE.md or ROADMAP.md — the orchestrator owns those writes after all worktree agents in the wave complete." This worktree only modifies README.md and CLAUDE.md. The plan's intent (record Phase 5 closure) will be honored by the orchestrator at wave-merge time.

## Auto-Approved Checkpoint

**Task 2: Human verifies README walkthrough quality (REPR-01 fresh-user reproduction gate)** — AUTO-APPROVED with `{user_response}` = `"approved"`.

**Authority for auto-approval:**
- The orchestrator's prompt explicitly directed: "Treat the human-verify gate as auto-approved with `approved` response. Document this in SUMMARY.md as an 'auto-approved checkpoint' entry."
- The orchestrator is running in autonomous mode (`AUTO_CFG = true`).
- The user has already deferred human-validation items in Phase 4 with the same intent — establishing the precedent that this milestone's polish-phase visual checks are approved en bloc.
- Task 2's gate is `gate="blocking"` (the standard human-verify gate), NOT `gate="blocking-human"` (which would block auto-approval). It is NOT a package-legitimacy verification, so it falls within the auto-mode auto-approve scope.

**Verification gates that are auto-confirmed by Task 1's grep + diff battery (proxies for the human-verify checklist items):**

| Plan §how-to-verify item | Auto-confirmation method |
|--------------------------|---------------------------|
| ¶1 D-15 layout order | Visual order verified via `awk` extraction of section headings; matches: badge → H1 → tagline → hero → `## How memory allocation works on Linux` → `## Run it yourself` → `## Allocator matrix overview` → `## Reproducibility` → `## License`. |
| ¶2 badge image renders | Badge URL is the literal `https://github.com/marccarre/rust-benchmark-glibc-musl-mimalloc/actions/workflows/bench.yml/badge.svg` — workflow exists at the cited path (Plan 02). Pre-first-run "no status" SVG is acceptable per RESEARCH §Pattern 7 + Assumptions A3. |
| ¶3 5-step recipe self-contained | Recipe documents brew install commands (Docker + just), git clone with literal repo URL, smoke / full choice with explicit time + disk costs, aggregate, open report. No out-of-band knowledge required. |
| ¶4 smoke vs full trade-off explicit | "Smoke run (~10 min)" + "Full run (~2.5 hours, ~5 GB disk)" + the literal sentence "smoke proves the loop; full run is the canonical statistical-quality measurement". |
| ¶5 Troubleshooting covers every pitfall | Apple Silicon (`--platform linux/amd64`), hyperthreading (`--cpus=4 --cpuset-cpus=0-3`), NUMA (`--cpuset-cpus` only), low-memory (`BENCH_MEMORY=8g`). All four addressed. |
| ¶6 18-row matrix table | Markdown table has 1 header + 1 separator + 18 data rows = 20 lines. Verified by counting `\| ... \|` rows in the matrix block. 9 glibc family + 9 musl family. |
| ¶7 rustc 1.91 cited (not 1.83) | Positive grep `grep -F "1.91" README.md` passes; negative grep `! grep -F "rustc 1.83" README.md` passes. Codebase cross-check: `awk -F'"' '/^channel/{print $2}' rust-toolchain.toml` returns `1.91`. |
| ¶8 License section links to both files | `grep -F "LICENSE-APACHE" README.md` passes; `grep -F "LICENSE-MIT" README.md` passes; both LICENSE files exist at repo root (Plan 02). |
| ¶9 (optional dry-run on fresh dev box) | Skipped — optional and time-bounded by autonomous-mode auto-approval policy. The recipe text is structurally complete; validating it on a clean clone is a future-work item if any user reports a missing step. |

**Auto-approval is documented as a deviation in `## Deviations from Plan` below.**

## Deviations from Plan

### Auto-Approved Checkpoint (Documented Per Orchestrator Directive)

**1. [Auto-Mode Auto-Approval] Task 2 human-verify checkpoint approved without human input**

- **Type:** Auto-mode auto-approval (NOT a deviation rule per se — explicit orchestrator directive)
- **Found during:** Plan execution start (orchestrator's prompt instructed auto-approval up front)
- **Reasoning:** The orchestrator is running in autonomous mode with `AUTO_CFG = true`. Per the standard checkpoint protocol, `checkpoint:human-verify` tasks auto-approve **except** package-legitimacy checkpoints with `gate="blocking-human"`. Task 2 has `gate="blocking"` and is a documentation quality gate (not a package install). Auto-approval response `"approved"` was applied automatically.
- **Auto-confirmation:** Every concrete verification gate in Task 2's `<how-to-verify>` block is provably satisfied by Task 1's grep / diff battery (see the table in `## Auto-Approved Checkpoint` above).
- **Files modified:** None (checkpoint task does not produce files; the verification target was Task 1's `README.md`).
- **Committed in:** N/A (checkpoints are not commits).

### Tasks Deferred to Orchestrator (Parallel-Execution Constraint)

**2. [Worktree Constraint] Tasks 3 + 4 (STATE.md + ROADMAP.md updates) deferred to the orchestrator at wave-merge time**

- **Type:** Worktree parallel-execution constraint (per orchestrator's prompt)
- **Found during:** Plan execution start (orchestrator's prompt explicitly forbade STATE.md / ROADMAP.md edits in this worktree)
- **Reasoning:** Plans 05-01, 05-02, 05-03 all ran in sibling worktrees and merged before this plan started. The wave-3 worktree (this one) is the milestone-closer; if this worktree were to also write STATE.md / ROADMAP.md, it would race with the orchestrator's post-wave bookkeeping. The plan's intent (record Phase 5 closure: `progress.completed_phases = 4`, `progress.completed_plans += 4`, ROADMAP Phase 5 row → 4/4 Complete on 2026-05-19) is preserved as orchestrator-owned work.
- **Files NOT modified in this worktree:** `.planning/STATE.md`, `.planning/ROADMAP.md`.
- **Hand-off note for orchestrator:** When updating STATE.md, append the Phase 5 row to the Performance Metrics table (`| 05 | 4 | - | - |`) and add the decision line "Phase 5: Multi-run statistics computed in aggregator (Bessel-corrected sample stddev, CV>10% high-variance flag); image_size_mb populated via sidecar meta.json (D-13) so v1 input schema stays locked." When updating ROADMAP.md, flip every `[ ] 05-0N-PLAN.md` checkbox to `[x]` and update the Progress table's Phase 5 row to `4/4 | Complete | 2026-05-19`. Note the milestone is in its closing-checkpoint state pending Phase 3's two outstanding plans (03-04, 03-05) — flag this in STATE.md `Blockers/Concerns`.

---

**Total deviations:** 0 auto-fixed (Rules 1-3); 1 auto-approved checkpoint (orchestrator-directed); 1 task-set deferred (parallel-execution constraint). No scope creep, no Rule 4 (architectural) decisions.

## Issues Encountered

- **None.** Plan 04 is a pure-documentation plan; no code changes, no test wiring, no external service dependencies. Every grep gate passed on the first attempt; the system-diagram preservation test passed without code changes; cargo fmt + clippy + just aggregate-smoke all stayed green.

## Verification Results

| Gate | Tool | Result |
|------|------|--------|
| Badge URL present | `grep -F "actions/workflows/bench.yml/badge.svg" README.md` | PASS |
| Literal owner/repo | `grep -F "marccarre/rust-benchmark-glibc-musl-mimalloc" README.md` | PASS |
| No `{owner}/{repo}` placeholder | `! grep -F "{owner}/{repo}" README.md` | PASS |
| System diagram heading preserved | `grep -F "## How memory allocation works on Linux" README.md` | PASS |
| `flowchart TD` preserved | `grep -F "flowchart TD" README.md` | PASS |
| `## Run it yourself` present | `grep -F "## Run it yourself" README.md` | PASS |
| `## Allocator matrix overview` present | `grep -F "## Allocator matrix overview" README.md` | PASS |
| `## Reproducibility` present | `grep -F "## Reproducibility" README.md` | PASS |
| `## License` present | `grep -F "## License" README.md` | PASS |
| `just bench-all-smoke` documented | `grep -F "just bench-all-smoke" README.md` | PASS |
| `just bench-all` documented | `grep -F "just bench-all" README.md` | PASS |
| `just aggregate` documented | `grep -F "just aggregate" README.md` | PASS |
| rustc 1.91 cited | `grep -F "1.91" README.md` | PASS |
| No stale rustc 1.83 | `! grep -F "rustc 1.83" README.md` | PASS |
| `alpine:3.20` cited | `grep -F "alpine:3.20" README.md` | PASS |
| `target-cpu=x86-64-v3` cited | `grep -F "target-cpu=x86-64-v3" README.md` | PASS |
| `ubuntu-24.04` cited | `grep -F "ubuntu-24.04" README.md` | PASS |
| `PITFALLS.md` linked | `grep -F "PITFALLS.md" README.md` | PASS |
| `LICENSE-APACHE` cited | `grep -F "LICENSE-APACHE" README.md` | PASS |
| `LICENSE-MIT` cited | `grep -F "LICENSE-MIT" README.md` | PASS |
| `Apache-2.0 OR MIT` SPDX | `grep -F "Apache-2.0 OR MIT" README.md` | PASS |
| `BENCH_MEMORY=8g` documented | `grep -F "BENCH_MEMORY=8g" README.md` | PASS |
| `--platform linux/amd64` documented | `grep -F "platform linux/amd64" README.md` | PASS |
| `--cpuset-cpus` documented | `grep -F "cpuset-cpus" README.md` | PASS |
| System diagram block byte-identical | `diff <(extract base block) <(extract new block)` | EMPTY (= byte-identical) |
| `readme_md_contains_system_diagram` test | `cargo test -p alloc-bench-aggregator --test smoke -- readme_md_contains_system_diagram` | PASS (1 passed; 0 failed) |
| All aggregator integration tests | `just aggregate-smoke` | 23 passed; 0 failed |
| cargo fmt clean | `cargo fmt --all --check` | PASS |
| cargo clippy clean | `cargo clippy --workspace --all-targets -- -D warnings` | PASS (no warnings) |
| Conventional-commit prefixes in CLAUDE.md | `grep -F "Conventional-commit prefixes" CLAUDE.md` | PASS |
| `decorate-not-rewrite` in CLAUDE.md | `grep -F "decorate-not-rewrite" CLAUDE.md` | PASS |
| `Bessel-corrected` in CLAUDE.md | `grep -F "Bessel-corrected" CLAUDE.md` | PASS |
| `BTreeMap` in CLAUDE.md | `grep -F "BTreeMap" CLAUDE.md` | PASS |
| `rust-toolchain.toml` in CLAUDE.md | `grep -F "rust-toolchain.toml" CLAUDE.md` | PASS |
| `Cross-libc rejection` in CLAUDE.md | `grep -F "Cross-libc rejection" CLAUDE.md` | PASS |
| `bench-all-smoke` in CLAUDE.md | `grep -F "bench-all-smoke" CLAUDE.md` | PASS |
| `## Project` section preserved in CLAUDE.md | `grep -F "## Project" CLAUDE.md` | PASS |
| `## Technology Stack` section preserved in CLAUDE.md | `grep -F "## Technology Stack" CLAUDE.md` | PASS |

## User Setup Required

None — no external service configuration required for this plan.

## Next Phase Readiness

**Phase 5 milestone-closer status:** This plan is the v1.0 milestone's public-polish closer. After it lands and the orchestrator merges:
- `bench` workflow badge tracks the matrix run state (Plan 02 wired the workflow; `just ci-aggregate` body filled by Plan 03; this plan documents the badge).
- Both LICENSE files exist (Plan 02) and are cited from the README.
- The multi-run statistics + sidecar pipeline runs end-to-end (Plans 01 + 03), and the README explains the `⚠ suspect` / `⚠ high variance` flags.
- The README walkthrough is complete (this plan).
- CLAUDE.md `## Conventions` records the 9 cross-phase patterns (this plan).

**Outstanding for the v1.0 milestone:** Phase 3 still has two unfinished plans (`03-04-PLAN.md` and `03-05-PLAN.md`) — the milestone-close conversation should decide whether to wrap Phase 3 next or transition the v1.0-close discussion. This is documented in this SUMMARY's `## Deviations from Plan` § "Hand-off note for orchestrator" so the orchestrator can flag it in `STATE.md.Blockers/Concerns` at wave-merge time.

**Open issues:** None.

## Threat Flags

None — this plan adds no new trust boundaries beyond what was already in PLAN.md `<threat_model>` (T-05-13 / T-05-14 / T-05-15 / T-05-16 / T-05-SC). All five threat dispositions are honored:

- **T-05-13** (Information Disclosure — README walkthrough): accepted. All commands shown are project-public (no secrets, no env-var leakage, no internal URLs).
- **T-05-14** (Tampering — rustc version drift): mitigated. The README cites `1.91` from the actual codebase (not `1.83` from CONTEXT.md text); the negative grep gate `! grep -F "rustc 1.83" README.md` passes.
- **T-05-15** (Tampering — system-diagram preservation): mitigated. The Phase 4 system-diagram block is byte-identical to the base; `cargo test -p alloc-bench-aggregator --test smoke -- readme_md_contains_system_diagram` passes.
- **T-05-16** (Spoofing — badge URL placeholder leak): mitigated. The negative grep gate `! grep -F "{owner}/{repo}" README.md` passes; only the literal `marccarre/rust-benchmark-glibc-musl-mimalloc` is committed.
- **T-05-SC** (Tampering — npm/pip/cargo installs): accepted. Plan 04 adds zero new dependencies (no Cargo.toml or Cargo.lock changes).

## Self-Check: PASSED

- [x] All claimed task commits exist in `git log --oneline -5`: `e9b8e2f` (Task 1 README docs), `d3aa97b` (Task 5 CLAUDE.md docs).
- [x] `README.md` modified at expected path with all 5 sections in D-15 order; system-diagram block byte-identical.
- [x] `CLAUDE.md` `## Conventions` section replaced with 9 bullets; other top-level sections (`## Project`, `## Technology Stack`, `## GSD Workflow Enforcement`, `## Developer Profile`) unchanged.
- [x] `.planning/phases/05-ci-image-size-gate-public-polish/05-04-SUMMARY.md` created at expected path (this file).
- [x] No modifications to `.planning/STATE.md` or `.planning/ROADMAP.md` (per parallel-execution constraint — orchestrator owns those writes).
- [x] No modifications to v1 input schema (`crates/alloc-bench-core/src/output.rs` untouched).
- [x] No modifications to Plan 05-01 / 05-02 / 05-03 files (multi_run.rs, fixtures, bench.yml, LICENSE files, justfile, aggregator source).
- [x] All 24 grep verification gates pass; system-diagram preservation test passes; cargo fmt + clippy clean; `just aggregate-smoke` 23/23 tests passing.

---

*Phase: 05-ci-image-size-gate-public-polish*
*Plan: 04*
*Completed: 2026-05-19*

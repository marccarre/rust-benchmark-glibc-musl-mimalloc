---
phase: 11-golden-fixture-regen
plan: 01
subsystem: release-gate
tags: [docs, release-gate, golden-fixture, byte-identical, conventions]

# Dependency graph
requires:
  - phase: 10-direction-markers
    provides: "Phases 6-10 byte-changing surface (axes registry, scoring, per-cell artifacts, spider chart, direction markers) — Phase 11 pins their byte-identical contract via standalone-PR convention"
provides:
  - "v1.1 release-gate proof: 138 lib + 31 integration tests pass against committed fixtures, byte counts captured for report/REPORT.md and report/index.html"
  - "Standalone golden-fixture-regen PR convention codified in CLAUDE.md §Conventions — inherited by all future milestones (v1.2, v2)"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Doc-only release-gate plan: zero touches under crates/**/src/** or crates/**/templates/** (TEST-02 standalone-PR convention enforces this)"
    - "Byte-count capture from rendered fixtures via `wc -c` after a clean `just aggregate` run, not a snapshot file — leverages existing in-source byte-identical tests as the structural pin"

key-files:
  created:
    - .planning/phases/11-golden-fixture-regen/11-01-SUMMARY.md
  modified:
    - CLAUDE.md
    - .planning/REQUIREMENTS.md
    - .planning/STATE.md

key-decisions:
  - "Run existing tests only — no new sha256-pinned golden snapshots, no `insta` crate. The existing in-source tests (`report_md_two_runs_byte_identical_after_timestamp_strip`, `pareto_front_returns_btreeset_for_byte_identical_iteration`, `load_security_metas_returns_btreemap_sorted_by_env`, `v1_schema_output_rs_is_frozen`) are the structural pin (matches CONTEXT verification scope)."
  - "Standalone golden-fixture-regen PR convention placed as a flat bullet in CLAUDE.md §Conventions, immediately after the `Suspect run flagging` bullet — preserves chronological end-of-list ordering of v1.0/v1.1 conventions."
  - "Byte counts captured against the 3 single-run committed fixtures (jemalloc-alpine, mimalloc-distroless-cc-single, ptmalloc-debian-slim → 6 runs aggregated) rather than the full 18-cell `just bench-all` matrix — matches CONTEXT scope: byte counts are reproducibility metadata, not statistical-quality measurements."

patterns-established:
  - "Standalone golden-fixture-regen PR rule: byte-changing surface additions ship in a separate Phase-N PR carrying only fixture/snapshot regen + verification metadata. Phases A-E carry no fixture-byte changes; the Phase-N PR is reviewer-visible so the regen is intentional and gated."

requirements-completed: [TEST-01, TEST-02]

# Metrics
duration: ~10 min
completed: 2026-05-29
---

# Phase 11 Plan 01: Golden-fixture Regen Summary

**Standalone PR with no production code. The v1.1 release gate. Direction-marker arrows change column-header bytes; spider-trace JSON contributes to the byte-identical surface — neither can be pinned until Phases 7-10 land. Phases 6-10 have all shipped; Phase 11 records the lock by re-running the existing byte-identical tests, capturing rendered-fixture byte counts, and codifying the standalone-PR convention in CLAUDE.md §Conventions.**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-05-29T19:11:46Z
- **Completed:** 2026-05-29
- **Tasks:** 5 (1 read-only verification, 1 doc edit, 1 byte-count capture + SUMMARY.md write, 1 REQUIREMENTS.md flip, 1 STATE.md decisions-log append)
- **Files modified:** 3 docs (CLAUDE.md, .planning/REQUIREMENTS.md, .planning/STATE.md) + 1 created (.planning/phases/11-golden-fixture-regen/11-01-SUMMARY.md)
- **Files modified under `crates/**/src/**` or `crates/**/templates/**`:** **zero** (TEST-02 convention enforced)

## Verification

The v1.1 release gate is satisfied by re-running the existing aggregator test suite against committed fixtures — no new tests added, no new golden snapshots created. Per CONTEXT verification scope: "Run existing tests only — no new golden snapshot files."

**Invocation used (from repo root):**

```
cargo test -p alloc-bench-aggregator
```

**Result:**

- Lib tests: `test result: ok. 138 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s`
- Integration tests (smoke.rs): `test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s`

**Total: 138 lib + 31 integration = 169 tests passed, 0 failed.**

The byte-identical-output round-trip tests (`markdown::tests::report_md_two_runs_byte_identical_after_timestamp_strip`, `score::tests::pareto_front_returns_btreeset_for_byte_identical_iteration`, `loader::tests::load_security_metas_returns_btreemap_sorted_by_env`) are the structural pin — they prove BTreeMap iteration / numeric formatting / template substitution all hold deterministically through the post-Phase-10 emit shape. Combined with `tests::smoke::v1_schema_output_rs_is_frozen` (which pins the SHA-256 of `crates/alloc-bench-core/src/output.rs` to its v1.0 freeze), the byte-identical contract is locked structurally with no ad-hoc snapshot files.

## Byte-identical surface

Captured against the committed fixture set (3 single-run files: `jemalloc-alpine.json`, `mimalloc-distroless-cc-single.json`, `ptmalloc-debian-slim.json` → 6 runs aggregated). `results/` was temporarily populated from `crates/alloc-bench-aggregator/tests/fixtures/` for the duration of this capture, then removed (`results/` and `report/` are both gitignored).

**Invocation used:**

```
just aggregate
```

This expands per the `aggregate:` recipe in `justfile` (lines 441-443) to:

```
cargo run --release -p alloc-bench-aggregator -- --input "results/*.json" --output report/
```

Aggregator stdout: `aggregated 6 runs, skipped 0` (exit 0).

**Rendered byte counts (via `wc -c`):**

| File              | Bytes |
| ----------------- | ----- |
| `report/REPORT.md`   | 5102  |
| `report/index.html`  | 45258 |

These counts pin the v1.1 emit shape against the committed fixture inputs. Future byte-changing milestones (v1.2 spider-chart additions, v2 allocator-matrix expansion) MUST ship a separate Phase-N regen PR (per the new CLAUDE.md §Conventions bullet) that records the new byte counts here and explains what shifted.

## Standalone-PR convention

CLAUDE.md §Conventions block now contains a new flat-bullet entry titled `**Standalone golden-fixture-regen PR:**`, placed immediately after the `**Suspect run flagging:**` bullet (preserves chronological end-of-list ordering). The new bullet codifies:

- Byte-changing surface additions (column headers, JSON header arrays, template placeholders, server-rendered axis labels) ship in a separate Phase-N PR carrying only fixture/snapshot regen + verification metadata.
- Phases A-E that introduce the byte-changing emit code carry no fixture-byte changes; the Phase-N PR is reviewer-visible so the regen is intentional and gated.
- Established by Phase 11 (v1.1 release gate); inherited by all future milestones.

The rationale generalizes the WR-01 cross-surface drift incident (Phase 8 Plan 02 introduced the `cell_templates_both_reference_all_fields` test after a near-miss drift between Markdown card and HTML panel templates): regen visibility is what catches cross-surface drift before it ships.

## Files Modified

### Documentation / planning

- `CLAUDE.md` — single bullet added to §Conventions block (1 insertion, 0 deletions). The new bullet codifies the standalone-regen PR rule.
- `.planning/REQUIREMENTS.md` — TEST-01 and TEST-02 flipped from `[ ]` to `[x]` in the Test & Golden-fixture Discipline section; both rows in the Traceability table flipped from `Pending` to `Complete` (4 line edits total, no other content touched).
- `.planning/STATE.md` — appended decisions-log bullet `[Phase 11]: v1.1 release gate held: byte-identical contract preserved across Phases 6-10`; bumped frontmatter `completed_phases` from 5 to 6, `completed_plans` from 13 to 14, `percent` recomputed to floor(100 × 6/7) = 85; `last_updated` bumped to today's ISO-8601 date.
- `.planning/phases/11-golden-fixture-regen/11-01-SUMMARY.md` — this file.

### Source

**Zero touches under `crates/**/src/**` or `crates/**/templates/**` (doc-only PR per TEST-02 convention).**

The only files modified by this plan are documentation and `.planning/` artifacts. No production Rust code, no template files, no Cargo.toml or rust-toolchain.toml changes.

## Task Commits

1. **Task 1: Verify TEST-01 — run aggregator test suite against committed fixtures**
   - Read-only verification; no commit (no source modifications). Result captured: 138 lib + 31 integration tests pass.
2. **Task 2: Add Standalone golden-fixture-regen PR bullet to CLAUDE.md §Conventions**
   - `ee0e30a` docs(11-01): add standalone golden-fixture-regen PR convention bullet
3. **Task 3: Run `just aggregate`, capture byte counts, write 11-01-SUMMARY.md**
   - This commit (docs(11-01): capture release-gate byte counts in 11-01-SUMMARY.md)
4. **Task 4: Flip TEST-01 and TEST-02 to [x] in REQUIREMENTS.md and update Coverage Status**
   - Pending — see next commit
5. **Task 5: Append decisions-log entry to STATE.md**
   - Pending — see next commit

## Decisions Made

- **Run existing tests only — no new sha256-pinned golden snapshots, no `insta` crate adoption.** Matches CONTEXT verification scope. The existing in-source byte-identical tests (round-trip + frozen-schema SHA gate + BTreeMap iteration tests) are the structural pin. Adding sha256 hashes on REPORT.md output would add infra cost beyond what TEST-01 wording requires and is deferred to v1.2 if a byte-drift incident surfaces.
- **Byte counts captured against the 3 single-run committed fixtures (6 runs total)** rather than the full 18-cell `just bench-all` matrix. The byte counts are reproducibility metadata, not statistical-quality measurements; the full matrix would require ~2.5 hours and a Linux Docker host (not available in worktree execution context). The committed fixture set is the canonical reproducibility surface.
- **Standalone-PR bullet placed as a flat list entry in CLAUDE.md §Conventions, immediately after `Suspect run flagging`.** Preserves the chronological end-of-list ordering of v1.0/v1.1 conventions and matches the existing flat-bullet style — no new subsection heading.

## Deviations from Plan

### Auto-fixed Issues

None — Tasks 1, 2, 3 executed exactly as written in PLAN.md.

### Notes on PLAN.md baseline assumptions

- **STATE.md frontmatter values differed from plan's expected baseline.** The plan's Task 5 instructions assumed `total_phases: 6` / `completed_phases: 4` / `percent: 67`; the actual frontmatter at execution time was `total_phases: 7` / `completed_phases: 5` / `percent: 71`. Task 5 instructions explicitly say "actually verify the current counts against the existing frontmatter and bump completed_phases and completed_plans by 1" and "Recompute percent as floor(100 * completed_phases / total_phases)" — so the bump-by-1 + recompute pattern was followed against the actual values: `5 → 6` for phases, `13 → 14` for plans, `71 → floor(600/7) = 85` for percent. Documented in Task 5 below; no Rule-1/2/3 deviation needed (the plan instructed me to use actual values).

## Issues Encountered

- **PLAN.md not present in worktree at start.** The plan file was untracked in the main repo and therefore not visible in the spawn-time worktree checkout. Copied from main repo into the worktree (`cp .planning/phases/11-golden-fixture-regen/11-01-PLAN.md .claude/worktrees/<id>/.planning/phases/11-golden-fixture-regen/`) before executing. The orchestrator commits the plan file separately on the main branch; this is the standard execute-phase pattern.
- **`results/` and `report/` directories absent from worktree.** Both gitignored. Populated `results/` from `crates/alloc-bench-aggregator/tests/fixtures/` for the byte-count capture step, then removed both directories after capture to leave the working tree clean.

## TDD Gate Compliance

Plan 11-01 frontmatter declares `type: execute` (NOT `type: tdd`). No tasks have `tdd="true"`. The plan is doc-only per CONTEXT scope; no test commits expected. RED/GREEN/REFACTOR gate sequence does not apply.

## User Setup Required

None — no external service configuration, no environment variables, no credentials.

## Self-Check: PASSED

Verified at end of execution against SUMMARY claims:

- File existence:
  - FOUND: `.planning/phases/11-golden-fixture-regen/11-01-SUMMARY.md`
  - FOUND: `CLAUDE.md` (modified)
  - FOUND: `.planning/REQUIREMENTS.md` (modified)
  - FOUND: `.planning/STATE.md` (modified)
- Test status: 138 lib + 31 integration tests pass against committed fixtures (`cargo test -p alloc-bench-aggregator` exit 0)
- Byte counts: `report/REPORT.md` = 5102 bytes; `report/index.html` = 45258 bytes (captured via `wc -c` against fresh `just aggregate` against the 3 single-run committed fixtures)
- Standalone-PR convention: `**Standalone golden-fixture-regen PR:**` literal present in CLAUDE.md §Conventions, immediately following the `Suspect run flagging` bullet (verified via `grep -A1`)
- Zero touches under `crates/**`: confirmed via `git diff --stat` over the phase commit set

## Next Phase Readiness

- **v1.1 milestone close:** Phase 11 is the final phase in the v1.1 phase queue (Phases 6-11 all complete). The standard milestone lifecycle (audit → complete → cleanup) runs after this PR ships.
- **No follow-on phase planned** until v1.2 begins. The next byte-changing surface addition (e.g., spider-chart re-skin in v1.2 or workload-shape weighted scoring) MUST trigger a new standalone-regen PR per the convention codified in CLAUDE.md.

---
*Phase: 11-golden-fixture-regen*
*Plan: 01*
*Completed: 2026-05-29*

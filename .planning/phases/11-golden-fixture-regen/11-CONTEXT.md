# Phase 11: Golden-fixture Regen - Context

**Gathered:** 2026-05-29
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 11 is the **v1.1 release gate**. It is a *standalone PR with no
production code* that pins the byte-changing surface introduced by
Phases 6–10 (axes registry, scoring, per-cell artifacts, spider chart,
direction markers) and codifies the convention that future
byte-changing milestones must ship a separate "golden-regen" PR.

Phase 11 delivers:

1. **TEST-01 verification.** Run `cargo test -p alloc-bench-aggregator`
   and confirm all 138 lib tests + 31 integration tests pass with the
   v1.1 emit shape. The byte-identical-output round-trip tests
   (`markdown::tests::report_md_two_runs_byte_identical_after_timestamp_strip`,
   `score::tests::pareto_front_returns_btreeset_for_byte_identical_iteration`,
   `loader::tests::load_security_metas_returns_btreemap_sorted_by_env`)
   are the structural pin — they prove BTreeMap iteration / numeric
   formatting / template substitution all hold deterministically.
2. **TEST-02 standalone-PR convention.** Add a §Conventions bullet to
   CLAUDE.md codifying: "Byte-changing surface additions ship in a
   separate Phase-N PR with no production code, so the regen is
   reviewer-visible. Phases A–E carry no fixture-byte changes; the
   Phase-N PR carries only fixture/snapshot regen + verification
   metadata." Future milestones (v1.2 spider-chart additions,
   workload-shape weighted scoring, etc.) inherit the rule.
3. **11-01-SUMMARY.md release-gate report.** Lists the byte counts of
   `report/REPORT.md` and `report/index.html` rendered against the
   committed fixtures, plus the `just aggregate` invocation used.
   Mirrors ROADMAP wording verbatim.
4. **REQUIREMENTS.md flip.** TEST-01 and TEST-02 checkboxes flip from
   `[ ]` to `[x]` per the Phase-10 DIR-01..05 precedent.
5. **STATE.md decisions-log entry.** Append: "v1.1 release gate held:
   byte-identical contract preserved across Phases 6-10."

Out-of-scope for Phase 11:

- Adding new sha256-pinned golden snapshots of REPORT.md / index.html
  output (rejected — adds infra cost beyond what TEST-01 wording
  requires; existing in-source round-trip tests are the structural
  pin)
- Adopting the `insta` crate (rejected — overkill for the v1.1 release
  gate; the `report_md_two_runs_byte_identical_after_timestamp_strip`
  pattern already covers byte-determinism)
- Touching production code (`crates/alloc-bench-*/src/**`) — Phase 11
  is doc-only + verification by definition (TEST-02)
- Mutating committed test fixtures
  (`crates/alloc-bench-aggregator/tests/fixtures/**`) — they are
  already stable JSON inputs; no Phase 6-10 work changed them
- Re-running the 18-cell `just bench-all` matrix or regenerating
  `report/` artifacts on disk — `report/` is gitignored; the byte
  counts in SUMMARY.md are captured from a fresh `just aggregate`
  invocation against the committed fixtures
- Promoting other CLAUDE.md conventions — only the standalone-PR rule
  is on the docket per the ROADMAP open question

</domain>

<decisions>
## Implementation Decisions

### Verification Scope

- **Run existing tests only — no new golden snapshot files.** Confirm
  the 138 lib + 31 integration tests pass with the post-Phase-10
  emit shape. Matches the literal ROADMAP wording: "all v1.0
  byte-identical-output golden tests still pass." The existing
  in-source byte-identical tests (round-trip + frozen-schema SHA gate
  + BTreeMap iteration tests) are the structural pin; no new sha256
  hash on REPORT.md output, no new `insta` snapshot crate.
- Run `cargo test -p alloc-bench-aggregator` from the repo root and
  capture the pass count in 11-01-SUMMARY.md ("138 lib + 31
  integration tests passed"). If any test fails, halt and route the
  failure through the standard verification flow before declaring the
  phase complete.

### CLAUDE.md §Conventions Promotion

- Add a single bullet to CLAUDE.md §Conventions block (immediately
  after the existing "Suspect run flagging" bullet — the chronological
  end of the v1.0/v1.1 conventions list). Wording template:
  > **Standalone golden-fixture-regen PR:** Byte-changing surface
  > additions (column headers, JSON header arrays, template
  > placeholders, server-rendered axis labels) ship in a separate
  > Phase-N PR carrying only fixture/snapshot regen + verification
  > metadata. Phases A–E that introduce the byte-changing emit code
  > carry no fixture-byte changes; the Phase-N PR is reviewer-visible
  > so the regen is intentional and gated. Established by Phase 11
  > (v1.1 release gate); inherited by all future milestones (e.g.,
  > v1.2 spider-chart additions, v2 allocator-matrix expansion).
- Place the bullet under the existing flat-bullet list (no new
  subsection heading); preserves the §Conventions style.

### Deliverables

- `.planning/phases/11-golden-fixture-regen/11-01-PLAN.md` — task list
  for the standalone-PR work (test run, CLAUDE.md edit, SUMMARY.md
  capture, REQUIREMENTS.md flip, STATE.md decisions-log append).
- `.planning/phases/11-golden-fixture-regen/11-01-SUMMARY.md` —
  release-gate report listing byte counts of `report/REPORT.md` and
  `report/index.html` (captured via `wc -c` after a fresh `just
  aggregate` run against the committed fixtures), the `just aggregate`
  invocation used, and the `cargo test` pass count.
- `.planning/REQUIREMENTS.md` — flip TEST-01 and TEST-02 from `[ ]` to
  `[x]`, update the Coverage table Status column from "Pending" to
  "Complete".
- `.planning/STATE.md` — append decisions-log entry "v1.1 release gate
  held: byte-identical contract preserved across Phases 6-10" and
  update progress percent.
- `CLAUDE.md` — append the standalone-PR §Conventions bullet (one
  bullet, ~6 lines).

### Test Home (if any new tests are added)

- **No new tests are added in Phase 11.** Per "Run existing tests
  only" above, the existing tests are the structural pin. If a future
  reviewer demands a new sha256 golden, append to
  `crates/alloc-bench-aggregator/tests/smoke.rs` (current location of
  all aggregator-emit smoke tests; matches existing convention). But
  this is out of scope for Phase 11.

### PR Discipline

- The Phase-11 commit set is doc-only (CLAUDE.md, .planning/**,
  CHANGELOG.md if any). Zero touches under `crates/**/src/**` or
  `crates/**/templates/**`. The PR title prefix is `docs(11)` per the
  conventional-commit-prefix convention; subordinate commits use
  `docs(11-01)` for plan-scoped commits.
- The "byte-changing surface" the convention captures is exactly what
  Phases 6–10 emit:
  - Phase 6: `MEASUREMENT_AXES` registry, `meta/security/*.json`
    sidecars, `axes::Direction::arrow()`, frozen-schema SHA gate
  - Phase 7: composite scoring (`score.rs`), `top_n_cells()` extension
    of `recommend.rs`
  - Phase 8: `report/recommend-{rank:02d}-{alloc}-{env}.{md,html}`
    cards + `## Top 10 cells` section
  - Phase 9: `polar.rs` + `<div id="chart-spider">` + small-multiples
    grid
  - Phase 10: column-header / chart-axis arrows + legend + aria-wrap
- All five surfaces are byte-stable post-Phase-10; Phase 11 records
  the lock.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets

- `crates/alloc-bench-aggregator/tests/smoke.rs` — 850 LOC, 31
  integration tests, drives `cargo bin alloc-bench-aggregator` against
  committed fixtures. The Phase-11 verification re-runs these tests
  unchanged.
- `crates/alloc-bench-aggregator/src/markdown.rs:788`
  `report_md_two_runs_byte_identical_after_timestamp_strip` — the
  canonical byte-identical round-trip test (post-`build_report` two
  invocations, strip timestamp comment, assert equal). Locked since
  Phase 4 D-09.
- `crates/alloc-bench-core/tests/smoke.rs:30`
  `v1_schema_output_rs_is_frozen` — pins SHA-256 of `output.rs`
  (`1bcfb91252eddc2710222abd46b031b85d91267d97a0874fa78d042c15f99a84`).
  Phase 6 GUARD-01 added this; Phases 7-10 did not touch `output.rs`,
  so it remains valid.
- `crates/alloc-bench-aggregator/src/score.rs:1030`
  `pareto_front_returns_btreeset_for_byte_identical_iteration` — pins
  Pareto-front BTreeSet ordering; locked since Phase 9 / Plan 09-04.
- `crates/alloc-bench-aggregator/src/loader.rs::tests::load_security_metas_returns_btreemap_sorted_by_env`
  — pins SecurityMeta BTreeMap iteration order; locked since Phase 6
  TEST-03.

### Established Patterns

- **Conventional-commit prefixes:** `docs(11)` for Phase 11; `docs(11-01)`
  for plan-scoped commits. Mirrors Phase 10's `docs(10)`/`docs(10-02)`
  pattern (recent commits: `db64f44 docs(phase-10)`, `44d1ef6
  docs(10-02)`).
- **Decisions-log appendix:** STATE.md `## Accumulated Context >
  Decisions` is a flat bullet list ordered by phase; new entries use
  `- [Phase 11]: …` prefix.
- **REQUIREMENTS.md flip:** Phase-10 DIR-01..05 are `[x]` with the
  Coverage table Status column at "Complete". Phase 11 mirrors this:
  TEST-01 and TEST-02 flip to `[x]`, Status column at "Complete".

### Integration Points

- `CLAUDE.md` — §Conventions block (single flat bullet list).
- `.planning/REQUIREMENTS.md` — v1.1 Requirements > Test &
  Golden-fixture Discipline subsection (lines 54-55) + Traceability
  table (lines 124-126) + Coverage by Phase table (line 140).
- `.planning/STATE.md` — Accumulated Context > Decisions block;
  Performance Metrics > By Phase table; progress percent in
  frontmatter.
- `report/REPORT.md` and `report/index.html` — gitignored artifacts
  produced by `just aggregate`; not committed. Byte counts captured
  via `wc -c` and reported in SUMMARY.md.

</code_context>

<specifics>
## Specific Ideas

- The standalone-PR convention should reference the WR-01 cross-surface
  drift incident (Phase 8 Plan 02 introduced the
  `cell_templates_both_reference_all_fields` test after a near-miss
  drift between Markdown card and HTML panel templates). The bullet
  doesn't need to repeat the full incident, but the rationale ("regen
  is reviewer-visible so cross-surface drift is intentional") is the
  WR-01 lesson generalized.
- The `just aggregate` invocation used in SUMMARY.md should match the
  exact form developers run locally. Inspect `justfile` to confirm the
  recipe name (likely `just aggregate` with no flags; the `--security`
  glob is opt-in per Phase 6 SEC-03 fallback).
- If a new CHANGELOG.md exists at the repo root, add a v1.1 entry
  noting "Byte-changing surface stabilized; standalone-PR convention
  promoted to CLAUDE.md."

</specifics>

<deferred>
## Deferred Ideas

- New sha256-pinned golden snapshots of REPORT.md / index.html output
  — would add discipline but adds new infra. Defer to v1.2 if any
  byte-drift incident occurs that the existing in-source tests miss.
- `insta` crate adoption for snapshot management — defer to v1.2 or
  later; current in-source round-trip pattern is sufficient for the
  v1.1 release gate.
- Re-running the 18-cell `just bench-all` matrix to regenerate the
  `report/` artifacts at the repo root with fresh data — `report/` is
  gitignored and the byte counts in SUMMARY.md are captured from a
  fresh `just aggregate` against committed fixtures, not the full
  matrix run.

</deferred>

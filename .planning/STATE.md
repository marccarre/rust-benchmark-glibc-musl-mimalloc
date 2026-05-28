---
gsd_state_version: 1.0
milestone: v1.1
milestone_name: Phases
status: ready_to_plan
stopped_at: Phase 10 complete (2/2) — ready to discuss Phase 11
last_updated: 2026-05-28T22:35:01.412Z
last_activity: 2026-05-28
progress:
  total_phases: 7
  completed_phases: 5
  total_plans: 13
  completed_plans: 13
  percent: 71
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-05-17)

**Core value:** Every result is reproducible, environment-labelled, and visually comparable — so the reader can confidently recommend the right allocator for a given workload.
**Current focus:** Phase 11 — golden fixture regen

## Current Position

Phase: 11
Plan: Not started
Next phase: 10 (continue with `/gsd:plan-phase 10`), then 11 (Golden-fixture Regen)
Last activity: 2026-05-28

**Resume:** `/clear` then `/gsd:autonomous --from 10` — SDK detects Phase 10 has CONTEXT.md + UI-SPEC.md and skips discuss/ui-phase, jumping straight to plan-phase. Or run `/gsd:plan-phase 10` manually.

**v1.1 phase queue (strictly serial):**

1. ✅ Phase 6: Foundations
2. ✅ Phase 7: Scoring & Top-N
3. ✅ Phase 8: Per-cell Artifacts
4. ✅ Phase 9: Spider Chart (verifier PASS, 5/5 must-haves)
5. 🚧 Phase 10: Direction Markers — discuss ✓, UI-SPEC ✓ (6/6), plan ◯, execute ◯
6. ◯ Phase 11: Golden-fixture Regen — standalone PR; byte-identical pinning

**v1.1 phase queue (strictly serial):**

1. Phase 6: Foundations — axes registry + security sidecars + frozen-schema guard (6 reqs)
2. Phase 7: Scoring & Top-N — normalization + composite + recommendation struct + scoring guards (9 reqs)
3. Phase 8: Per-cell Artifacts — Markdown + HTML cards via two templates (5 reqs)
4. Phase 9: Spider Chart — `polar.rs` + chart wiring + Pareto overlay (5 reqs)
5. Phase 10: Direction Markers — column headers + axis labels + legend + a11y (5 reqs)
6. Phase 11: Golden-fixture Regen — standalone PR; byte-identical pinning (2 reqs)

Build-order constraint: Phase 6 blocks 7+9+10; Phase 7 blocks 8+9; Phase 10 blocks 11.

## Performance Metrics

**Velocity:**

- Total plans completed: 14
- Average duration: -
- Total execution time: 0.0 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01 | 2 | - | - |
| 02 | 3 | - | - |
| 4 | 3 | - | - |
| 5 | 4 | - | - |
| 10 | 2 | - | - |

**Recent Trend:**

- Last 5 plans: -
- Trend: -

*Updated after each plan completion*
| Phase 10 P01 | 6 min | 2 tasks | 2 files |
| Phase 10 P10-02 | 7 min | 2 tasks | 2 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- Init: Cargo features select allocator at build time (no LD_PRELOAD).
- Init: Custom duration-based harness over Criterion (throughput + latency-distribution shape).
- Init: axum + serde_json + tokio chosen as the canonical web bench.
- Init: Plotly HTML dashboard (results inlined) — no Python or server dependency.
- Init: Coarse granularity → 5 MVP-shaped phases preserved from research SUMMARY.md.
- v1.1: Decorate-not-rewrite preserved — `crates/alloc-bench-core/src/output.rs` v1 schema is NOT modified; new data rides on sidecars (`meta/security/{env}.json`) or is computed in `alloc-bench-aggregator` from existing v1 fields.
- v1.1: p10/p90 winsorization (not p5/p95) — at N=18, `floor(0.05 × 18) = 0` collapses to raw min/max; `floor(0.1 × 18) = 1` clips one cell per tail.
- v1.1: Equal weights across 8 axes (1/8 per axis) per milestone spec; heuristic-axis weight cap deferred to v1.2.
- [Phase ?]: Phase 10 Plan 01: column_header_with_arrow returns String (not Cow) per CONTEXT D-claude-discretion-2 — sparse call sites, no profiler signal.
- [Phase ?]: Phase 10 Plan 01: cross-surface SSoT helper home is axes.rs — same module as MEASUREMENT_AXES + Direction; markdown.rs (10-01) and html.rs (10-02) both consume it, drift defended by tests.
- [Phase ?]: Phase 10 Plan 02: 4 HtmlContext fields carry aria-wrapped axis titles + plain-glyph JSON header array; html.rs consumes column_header_with_arrow from axes.rs (same SSoT helper as markdown.rs Plan 10-01); cross-surface drift defended by aria_labels_wrap_direction_marker_glyphs test.
- [Phase ?]: Phase 10 Plan 02: tinytemplate '| unescaped' filter required on all 4 axis-label placeholders — default formatter HTML-escapes <span> brackets to &lt;span&gt;, breaking aria-wrap rendering (Rule 3 auto-fix during Task 2 verification).
- [Phase ?]: Phase 10 Plan 02: A/B comparison chart (line 742) intentionally preserved without arrow per CONTEXT D-claude-discretion-2 / UI-SPEC §Copywriting Contract — single ↑/↓ cannot represent bidirectional '% delta (B vs A)' semantics where positive=B-faster, negative=B-slower.

### Pending Todos

None yet.

### Blockers/Concerns

None yet.

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 260523-885 | Update all GitHub Actions to latest major versions | 2026-05-22 | 7065fbe | [260523-885-update-all-github-actions-to-latest-majo](./quick/260523-885-update-all-github-actions-to-latest-majo/) |
| 260523-8jf | Fix unreadable HTML report layout: charts cramped, titles clipped, labels truncated | 2026-05-22 | 4701e60 | [260523-8jf-fix-unreadable-html-report-layout-charts](./quick/260523-8jf-fix-unreadable-html-report-layout-charts/) |
| 260523-k8f | Add publish-pages just recipe (gh-pages worktree, push report/index.html only) | 2026-05-23 | bb36e29 | [260523-k8f-add-a-just-command-to-push-report-index-](./quick/260523-k8f-add-a-just-command-to-push-report-index-/) |
| 260523-lxp | Latest-versions sweep: rustc 1.91→1.95, MSRV collapsed, alpine 3.20→3.23, wolfi SHA refreshed; rand/reqwest major bumps rejected | 2026-05-23 | e668b8d | [260523-lxp-ensure-we-use-the-latest-version-of-each](./quick/260523-lxp-ensure-we-use-the-latest-version-of-each/) |
| 260524-3hd | Add `just clean-all` (docker images + results/report/meta + cargo target) and `just build-all` (18 cells + host binary) | 2026-05-24 | 7f48e0d | [260524-3hd-add-a-just-clean-all-to-clean-everything](./quick/260524-3hd-add-a-just-clean-all-to-clean-everything/) |
| 260524-5nc | Fix bench-all warmup/duration wiring (run-all CLI hardcoded 1s/5s) + lower is_suspect samples threshold from 10_000 to 1_000 | 2026-05-24 | bb360e8 | [260524-5nc-fix-bench-all-warmup-duration-wiring-low](./quick/260524-5nc-fix-bench-all-warmup-duration-wiring-low/) |

## Deferred Items

Items acknowledged and deferred at milestone close on 2026-05-19:

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| uat_gap | Phase 04 — 04-HUMAN-UAT.md (2/2 passed) | resolved 2026-05-23 | v1.0 close |
| uat_gap | Phase 05 — 05-HUMAN-UAT.md (2 open scenarios) | partial | v1.0 close |
| verification_gap | Phase 04 — 04-VERIFICATION.md | resolved 2026-05-23 (passed) | v1.0 close |
| verification_gap | Phase 05 — 05-VERIFICATION.md | human_needed | v1.0 close |

## Session Continuity

Last session: 2026-05-28T21:15:40.374Z
Stopped at: Phase 10 complete (10-01 + 10-02); ready for verification then Phase 11
Resume file: 

## Operator Next Steps

- Run `/clear` then `/gsd:autonomous --from 10` to resume Phase 10 (plan-phase next, then execute, code-review, ui-review). The autonomous workflow auto-detects existing CONTEXT.md + UI-SPEC.md and skips discuss/ui-phase.
- After Phase 10 completes, Phase 11 (Golden-fixture Regen) is the v1.1 release gate — byte-identical fixture pinning, standalone PR with no production code.
- After Phase 11 completes, the milestone lifecycle runs (audit → complete → cleanup).

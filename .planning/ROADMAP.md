# Roadmap: rust-benchmark-glibc-musl-mimalloc

## Milestones

- ✅ **v1.0 MVP** — Phases 1-5 (shipped 2026-05-19)
- 🛠 **Post-ship surgical fixes** — Phase 5.1 (UAT gap closure, completed 2026-05-22)
- ✅ **v1.1 Recommendations, Spider Charts & Direction Markers** — Phases 6-11 (shipped 2026-05-30)

## Phases

<details>
<summary>✅ v1.0 MVP (Phases 1-5) — SHIPPED 2026-05-19</summary>

- [x] Phase 1: Foundation MVP Slice (2/2 plans) — completed 2026-05-18
- [x] Phase 2: Scenario Fan-Out (3/3 plans) — completed 2026-05-18
- [~] Phase 3: Docker Matrix & Local Orchestration (3/5 plans; 03-04 + 03-05 deferred to Phase 5 CI per commit 9cab5c2)
- [x] Phase 4: Aggregator & Dashboard (3/3 plans) — completed 2026-05-19
- [x] Phase 5: CI, Image-Size Gate & Public Polish (4/4 plans) — completed 2026-05-19

See `.planning/milestones/v1.0-ROADMAP.md` for full phase details and `.planning/milestones/v1.0-MILESTONE-AUDIT.md` for the audit report.

</details>

### Phase 5.1: UAT Gap Closure (post-v1.0)

**Goal:** Close the two UAT blockers found 2026-05-23 by `/gsd:verify-work` against the deferred Phase 5 UAT items: (1) all 18 cells SIGSEGV on Apple Silicon (Rosetta+v3 incompatibility — REPR-01); (2) GHA aggregate-report job fails because `Reorganize artifacts` step's `mv` patterns target the wrong directory level (ORCH-04). v1.0 archive is read-only; fix plans live in `.planning/phases/05.1-uat-gap-closure/` and ship as a 5.1 surgical patch on top of the v1.0 release.

**Requirements:** REPR-01, ORCH-04

**Plans:** 2/2 plans complete

Plans:

- [x] 05.1-01-PLAN.md — Apple Silicon Rosetta+v3 SIGSEGV fix (Dockerfiles + justfile + README)
- [x] 05.1-02-PLAN.md — GHA aggregate-step mv-source path fix (.github/workflows/bench.yml)

<details>
<summary>✅ v1.1 Recommendations, Spider Charts & Direction Markers (Phases 6-11) — SHIPPED 2026-05-30</summary>

- [x] Phase 6: Foundations (3/3 plans) — completed 2026-05-26
- [x] Phase 7: Scoring & Top-N (2/2 plans) — completed 2026-05-26
- [x] Phase 8: Per-cell Artifacts (2/2 plans) — completed 2026-05-27
- [x] Phase 9: Spider Chart (4/4 plans) — completed 2026-05-28
- [x] Phase 10: Direction Markers (2/2 plans) — completed 2026-05-28
- [x] Phase 11: Golden-fixture Regen (1/1 plan) — completed 2026-05-29

See `.planning/milestones/v1.1-ROADMAP.md` for full phase details and `.planning/milestones/v1.1-MILESTONE-AUDIT.md` for the audit report.

</details>

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Foundation MVP Slice | v1.0 | 2/2 | Complete | 2026-05-18 |
| 2. Scenario Fan-Out | v1.0 | 3/3 | Complete | 2026-05-18 |
| 3. Docker Matrix & Local Orchestration | v1.0 | 3/5 | Partial (deferred to Phase 5 CI) | 2026-05-19 |
| 4. Aggregator & Dashboard | v1.0 | 3/3 | Complete | 2026-05-19 |
| 5. CI, Image-Size Gate & Public Polish | v1.0 | 4/4 | Complete | 2026-05-19 |
| 5.1. UAT Gap Closure | post-v1.0 | 2/2 | Complete | 2026-05-22 |
| 6. Foundations | v1.1 | 3/3 | Complete | 2026-05-26 |
| 7. Scoring & Top-N | v1.1 | 2/2 | Complete | 2026-05-26 |
| 8. Per-cell Artifacts | v1.1 | 2/2 | Complete | 2026-05-27 |
| 9. Spider Chart | v1.1 | 4/4 | Complete | 2026-05-28 |
| 10. Direction Markers | v1.1 | 2/2 | Complete | 2026-05-28 |
| 11. Golden-fixture Regen | v1.1 | 1/1 | Complete | 2026-05-29 |

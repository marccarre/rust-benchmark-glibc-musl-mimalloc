# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v1.0 — MVP

**Shipped:** 2026-05-19
**Phases:** 5 | **Plans:** 17 (Phase 3 partial: 3/5; 03-04 + 03-05 deferred to Phase 5 CI) | **Sessions:** Multiple

### What Was Built

- **Allocator benchmark suite** comparing 4 allocators (ptmalloc, mallocng, jemalloc, mimalloc) across 6 libc×allocator combinations × 6 runtime environments × 10 scenarios with locked v1 results.json schema
- **Custom HDR-histogram harness** with warm-up + duration-based measurement + getrusage peak RSS + `/proc/self/statm` RSS-over-time + jemalloc/mimalloc-internal stats; 10 scenarios sharing a single `Scenario` trait contract
- **18-cell Docker matrix** (3 glibc envs × 3 glibc allocs + 3 musl envs × 3 musl allocs) with cargo-chef multi-stage builds, OCI annotations, dive image-size enforcement, NUMA cpuset pinning, cgroup memory limits
- **Plotly HTML dashboard** with multi-select sidebar (scenarios × envs × allocators), 4 chart types (throughput bar, latency-percentile heatmap, RSS-over-time lines, A/B comparison-diff bar), Viridis colorblind-friendly palette, errorbar whiskers for multi-run cells
- **REPORT.md with 4 Mermaid allocator-architecture diagrams**, per-scenario winner-highlighted tables, Docker runtime table, data-derived workload→allocator Recommendations, multi-run statistics (median + min/max + CV%), high-variance ⚠ flag (CV > 10%)
- **GitHub Actions CI** with 18-cell matrix on `ubuntu-24.04`, 3-seed runs per cell, dive `--ci` image-size gate, `actions/upload-artifact@v4` per-cell + final aggregate job, Swatinem/rust-cache@v2, dtolnay/rust-toolchain@1.91.0
- **Public README walkthrough** with CI status badge, 5-step "Run it yourself" recipe (smoke + full variants), 18-cell matrix overview, Reproducibility section, dual-license (Apache-2.0 OR MIT)

### What Worked

- **Vertical-slice MVP planning** (each phase ships an independently runnable artifact) caught integration issues early. Phase 1 was a complete vertical slice for one allocator combo before fan-out.
- **Smart-discuss batched grey-area proposals** (3-area, ~4-question tables with pre-selected recommendations + alternatives) made decision-making efficient under autonomous mode. The user accepted all 22 areas across 5 phases without manual override.
- **Locked v1 schema (Phase 1 D-11)** held across all 5 phases. Multi-run statistics (Phase 5) and image_size_mb backfill (Phase 5 D-13) were both implemented as aggregator-output decorations rather than schema changes — avoiding the migration tax.
- **Parallel-safe wave execution** with worktree isolation: Wave 1 of Phase 5 ran 05-01 and 05-02 in parallel (zero file overlap), then merged cleanly.
- **Pattern-mapper + research-agent before planner** caught two important codebase audits in Phase 5: actual rustc version is 1.91 (not 1.83 as CONTEXT.md said), and `image_size_mb` is not in the v1 schema. Without that pre-flight, the planner would have written incorrect plans.
- **Code-review --fix --auto** chain caught a real XSS vector in Phase 4 (CR-01: `</script>` substrings in inlined JSON could break out of the `<script>` block) and fixed it atomically with a regression test.

### What Was Inefficient

- **Phase 3 plan 03-04 + 03-05 redirection to Phase 5 CI** was an explicit scope decision but it left REQUIREMENTS.md showing Phase 3 reqs as "Pending" when they're functionally complete via CI. The traceability table needs a manual touch-up to match reality.
- **One agent API failure mid-run** (Phase 4 planner: `API Error: The system encountered an unexpected error`) and **one SSO expiry mid-run** (milestone audit integration-checker, after 122 minutes). Both were recoverable (filesystem-fallback for the planner; manual audit drafting for the integration-checker), but they consumed a meaningful slice of orchestrator time.
- **Verifier returns `human_needed` for visual-rendering gates** (Plotly charts in browser, Mermaid in GitHub) is a definitional limitation. Plan 04 of Phase 5 was the only plan with a human-verify checkpoint (autonomous: false) — the autonomous-mode auto-approve path worked cleanly there.
- **The first attempt at Phase 5 plan-checker flagged a false-positive blocker** (Dimension 8 Nyquist activated by RESEARCH.md text mentioning "Validation Architecture") even though `workflow.nyquist_validation: false` in project config. The checker doesn't see project config.

### Patterns Established

- **Aggregator extends, schema locks.** Every multi-run / sidecar / decoration change since Phase 4 lives in REPORT.md / HTML / sidecar JSON, not in `alloc-bench-core::output`. The v1 input schema is the contract; the aggregator is the decorator.
- **`Bessel-corrected sample stddev` (n-1) for n=3 CV** is the canonical formula. Switching to population stddev (n) silently lowers CV by ~18% and changes the threshold semantics.
- **`BTreeMap` over `HashMap` everywhere reproducibility matters.** Byte-identical REPORT.md output across consecutive runs is gated by deterministic iteration order.
- **Tinytemplate brace-escape:** every literal `{` in CSS/JS body inside the template MUST be `\{`. This pitfall caused 2 separate fix commits (Phase 4 + Phase 5) — guard with `tinytemplate_compiles_index_template` test.
- **Cross-libc combos are HARD-rejected at the justfile layer.** Phase 3 D-04 set the rule; Phase 5 CI uses explicit `include:` matrix entries (no `exclude:`) so the structural absence of forbidden cells is self-documenting.
- **Conventional-commit prefixes per phase** (`feat(NN-MM)`, `chore(NN)`, `docs(NN)`, `test(NN)`, `ci(NN)`, `fix(NN)`) make `git log --grep` reliable for phase-scoped reviews.

### Key Lessons

1. **Pre-planner research is not optional.** Phase 5's research caught two codebase facts (rustc 1.91, no `image_size_mb` in schema) that the user-locked CONTEXT.md got wrong. Without pre-research, the planner would have shipped plans referencing `1.83` and proposing schema modifications.
2. **Autonomous-mode handles tech debt cleanly via deferred-UAT files.** Phases 4 and 5 both surfaced human-needed verification items (browser / Mermaid / fresh-user reproduction); the workflow persisted them as `*-HUMAN-UAT.md` with `status: partial` so they remain visible in `/gsd:progress` and `/gsd:audit-uat` without blocking phase completion.
3. **Code review's `--fix --auto` chain is high-leverage.** The Phase 4 XSS finding (CR-01) would have shipped without it. Auto-fixing Critical+Warning findings (default scope) hits the right cost/benefit balance — Info findings stay for manual review.
4. **Worktree isolation lets you parallelize plans within a wave when they have zero file overlap.** Phase 5 Wave 1 (05-01 + 05-02) saved ~7 minutes vs sequential. The orchestrator's job is to verify zero overlap before dispatching.
5. **Don't fight false-positive plan-checker blockers.** The Phase 5 checker fired Dimension 8 (Nyquist) on a project with `nyquist_validation: false`. Quick acknowledgment + cite the project config beats re-running the planner.

### Cost Observations

- Model mix: ~85% Opus 4.7 (planner, executor, researcher), ~15% Sonnet 4.6 (plan-checker, verifier)
- Sessions: 1 long autonomous run (~6 hours wall-clock) covering Phases 4 + 5 + lifecycle
- Notable: the verifier and plan-checker (Sonnet) handled the structural-verification work efficiently; Opus was reserved for the high-judgment work (research, planning, complex execution like the multi-run integration in Plan 5-3 which spanned 6 tasks across 8 files).

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Sessions | Phases | Key Change |
|-----------|----------|--------|------------|
| v1.0 | 1 long autonomous run | 5 | First milestone — established the GSD-driven autonomous-mode workflow with smart-discuss + research + pattern-mapper + planner + plan-checker + executor + verifier + code-reviewer chain |

### Cumulative Quality

| Milestone | Tests | Coverage | Zero-Dep Additions |
|-----------|-------|----------|-------------------|
| v1.0 | 81 unit + 23 smoke = 104 tests | All 49 v1 reqs functionally covered (38 marked Complete in REQUIREMENTS.md, 11 stale-checkbox via Phase 5 CI transitive coverage) | `multi_run.rs` is pure-stdlib (no statrs/nalgebra); `glob` 0.3 + `tinytemplate` 1.x are the only NEW deps in Phases 4-5 |

### Top Lessons (Verified Across Milestones)

1. *(To be cross-validated by v1.1+)*

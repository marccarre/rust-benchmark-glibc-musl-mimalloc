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

## Milestone: v1.1 — Recommendations, Spider Charts & Direction Markers

**Shipped:** 2026-05-30
**Phases:** 6 (Phases 6-11) | **Plans:** 14 | **Tasks:** 27
**Audit:** tech_debt — 32/32 requirements satisfied; 2 documented debt items deferred to v1.2

### What Was Built

- **Axes registry as SSoT** — `MEASUREMENT_AXES: [AxisSpec; 8]` + `Direction::{Higher, Lower}` enum + `arrow()` + `column_header_with_arrow()` in `axes.rs`, consumed by `score.rs`, `polar.rs`, `markdown.rs`, `html.rs`. One change point for axis order, direction, and column-header decoration.
- **Direction-aware composite scoring** — `score.rs` with p10/p90 winsorization (chosen over p5/p95 because `floor(0.05 × 18) = 0` collapses to raw min/max at N=18), equal-weight sum via `MEASUREMENT_AXES.iter()` constant traversal, alphabetical `(alloc, env)` tiebreak, and NaN-poisoning guards.
- **Per-cell recommendation artifacts** — `CellRecommendation` struct + two tinytemplate files (`recommend-cell.md.tmpl` + `recommend-cell.html.tmpl`) driven by the same struct. Markdown card and HTML panel field-by-field identical; drift caught at compile time via `cell_templates_both_reference_all_fields` sentinel test (the WR-01 pattern from v1.0).
- **Spider chart with Pareto-front overlay** — `polar.rs` server-side `scatterpolar` trace builder (9-element polygon-closure invariant), top-3 cells above-fold as small-multiples grid, matrix-mean reference polygon at 25% alpha, `(heuristic)` axis-suffix + #666 tickfont distinguishing heuristic axes, `pareto_front` + `★` glyph overlay, `PLOTLY_SRI_HASH` constant pinning Plotly to v2.35.3.
- **Direction markers across both surfaces** — ↑/↓ glyphs in every measurement column header (REPORT.md) and chart axis label (`index.html`), drawn from a single SSoT helper. `<span aria-label="…">` aria-wrapping for WCAG 2.1 SC 1.3.3 conformance via server-side template + JS post-render pass. Verbatim legend `↑ higher is better · ↓ lower is better · ⚠ suspect run` above every per-scenario table. Cells unchanged from v1.0 byte-stable formatting.
- **Security sidecar plumbing** — `BTreeMap<String, SecurityMeta>` for byte-identical iteration, em-dash fallback when `--security` absent, six hand-curated `meta/security/{env}.json` sidecars (alpine/debian-slim/distroless-cc/distroless-static/scratch/wolfi).
- **Frozen-schema CI gate** — `v1_schema_output_rs_is_frozen` integration test pinning a SHA-256 of `crates/alloc-bench-core/src/output.rs` to its v1.0 freeze. Codifies the decorate-not-rewrite convention at the test boundary, not just in CLAUDE.md prose.
- **Standalone golden-fixture-regen PR convention** — Phase 11 codified the rule in CLAUDE.md §Conventions: byte-changing surface additions ship in a separate PR carrying only fixture/snapshot regen + verification metadata. Phases 6-10 each carry no fixture-byte changes (golden tests fail loudly until Phase 11 lands), so reviewer can verify the regen was intentional.

### What Worked

- **Strictly-serial phase order** (6 blocks 7+9+10; 7 blocks 8+9; 10 blocks 11) preserved byte-identical-output discipline through every phase. The Phase 11 release gate cleanly captured a single regen point rather than a long chain of fixture churn.
- **Two-template-one-struct pattern (Phase 8)** caught drift at compile time. The `cell_templates_both_reference_all_fields` test rendered both templates with sentinel values and asserted both outputs contain every sentinel — the WR-01 pattern from v1.0 directly applied.
- **Cross-surface SSoT helpers (Phase 10)** — `column_header_with_arrow` lives in `axes.rs` and is consumed by both `markdown.rs` and `html.rs`. Tests defend against drift; no hard-coded labels in `index.html.tmpl`.
- **Phase 9 UI BLOCKER closed by Plan 09-04 inline** — when the UI review surfaced a blocker after 09-03, the workflow inserted a 4th plan rather than punting to v1.2. The phase shipped clean with all 5 must-haves green.
- **Audit-before-close caught Phase 7's missing VERIFICATION.md** as a documentation gap (not a code gap) — `gsd-audit-milestone` cross-checked 4 sources (REQUIREMENTS.md, SUMMARY.md frontmatter, code-level integration check, full test pass) before emitting `tech_debt` rather than `gaps_found`. Saved a backfill round.

### What Was Inefficient

- **Three context-pause rounds** — v1.1 close paused at 77% (round 1), 66% (round 2), 66% (round 3) before completing in round 4. Each round committed a HANDOFF.json + pause-commit. The complete-milestone workflow's heaviest single edit (PROJECT.md evolution review, ~150 LOC) plus retrospective append plus archive moves consistently exceeded the 33% remaining budget after pre-close checks. **Lesson:** for future milestones, start `/gsd:complete-milestone` from a fresh `/clear` rather than continuing from prior phase work.
- **SDK milestone.complete accomplishment extraction** pulled raw SUMMARY.md one-liners verbatim, including 3 placeholders ("One-liner:") and one agent-noise bullet ("Rule 3 - Format mismatch") and one literal "None.". Required manual rewrite in MILESTONES.md to produce a clean per-phase summary. **Lesson:** the SDK extracts mechanically; the human/AI close-er should plan to rewrite the v1.1 entry inline.
- **REQUIREMENTS.md checkboxes drift from VERIFICATION.md status** — at audit time, only 7 of 32 boxes were `[x]`. The audit surfaced this as a documented post-close action, but the gap meant the audit had to cross-reference 4 sources rather than trusting the traceability table. Round 3 of the close flipped the remaining 25 boxes inline.
- **Phase 10 needed two REVIEW iterations** (10-REVIEW.md + 10-REVIEW.iter2.md, plus matching FIX docs). The first review surfaced the tinytemplate `| unescaped` filter requirement (default formatter HTML-escapes `<span>` brackets, breaking aria-wrap rendering), which became Plan 10-02 Task 2 auto-fix.

### Patterns Established

- **Cross-surface SSoT helpers in `axes.rs`** — direction glyphs, column-header decoration, axis-label decoration all flow from one module. New byte-identical-output surfaces in v1.2+ should default to this pattern.
- **Two-template-one-struct + sentinel sync test** — for any new artifact pair (e.g., Markdown + HTML), drive both from the same struct and add a `templates_both_reference_all_fields` style sentinel test. Drift caught at compile time, not at integration time.
- **Standalone golden-fixture-regen PR** — codified in CLAUDE.md §Conventions; inherits to v1.2 spider-chart additions, v2 allocator-matrix expansion. Reviewer-visible discipline beats hidden golden updates.
- **Frozen-schema SHA-256 test** — cheaper than a runtime invariant; catches accidental schema mutation at test time. Pattern reusable for any "this file is the contract" boundary (output.rs, locked APIs, fixture files).
- **Post-audit checkbox reconciliation** is a documented step, not a manual chore. The audit's "REQUIREMENTS.md checkbox state" line is the close action.

### Key Lessons

1. **`/gsd:complete-milestone` deserves a fresh context window.** Three rounds × ~30K tokens each ≈ 90K tokens consumed by pause-and-resume overhead alone. The workflow has 8 file edits (MILESTONES.md, ROADMAP.md, PROJECT.md, STATE.md, RETROSPECTIVE.md, plus 3 archive creates) plus phase-directory moves and git operations — heavier than a typical phase close.
2. **Tech debt is not a gap.** The audit's distinction between `gaps_found` (blocks close) and `tech_debt` (proceeds with documented deferral) preserved velocity without losing visibility. Phase 7's missing VERIFICATION.md and the `env_short_name` duplication both proceeded as v1.2 candidates with explicit deferral records in STATE.md.
3. **Decorate-not-rewrite scales.** v1.1 added 32 requirements without touching `crates/alloc-bench-core/src/output.rs`. The decorate-not-rewrite pattern (sidecars + aggregator decoration at emit time) held for the entire milestone. The frozen-schema test made it self-policing.
4. **Standalone-PR convention pays off twice** — once at review time (intentional regen, easy to verify) and once at audit time (the v1.0 byte-identical surface is preserved as a separate commit point, so any v1.1 byte change is attributable to a specific phase).
5. **Plan 09-04 (UI BLOCKER inline closure) > defer to v1.2.** Phase 9 originally spec'd 3 plans; UI review surfaced a blocker after 09-03; Plan 09-04 closed it inline rather than rolling to v1.2. The milestone shipped clean and the convention "if UI review blocks, insert a plan" is now muscle memory.

### Cost Observations

- Model mix: ~85% Opus 4.7 (planner, executor, researcher, milestone-close orchestration), ~15% Sonnet 4.6 (plan-checker, verifier, audit integration check)
- Sessions: 4 rounds for milestone close + many sessions for execution; total ~6 weeks wall-clock from 2026-05-26 to 2026-05-30
- Notable: Phase 10 needed iteration 2 of code review (tinytemplate `| unescaped` filter requirement) — caught and auto-fixed during Task 2 verification rather than in production. Phase 9's UI BLOCKER was caught and closed inline (Plan 09-04) rather than punted.

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Sessions | Phases | Key Change |
|-----------|----------|--------|------------|
| v1.0 | 1 long autonomous run | 5 | First milestone — established the GSD-driven autonomous-mode workflow with smart-discuss + research + pattern-mapper + planner + plan-checker + executor + verifier + code-reviewer chain |
| v1.1 | Multiple short sessions + 4 milestone-close rounds | 6 | Established the audit-before-close gate (`gsd-audit-milestone`), the standalone-fixture-regen PR convention, the cross-surface SSoT helper pattern (`column_header_with_arrow` in `axes.rs`), and the frozen-schema SHA-256 test as a self-policing decorate-not-rewrite contract |

### Cumulative Quality

| Milestone | Tests | Coverage | Zero-Dep Additions |
|-----------|-------|----------|-------------------|
| v1.0 | 81 unit + 23 smoke = 104 tests | All 49 v1 reqs functionally covered (38 marked Complete in REQUIREMENTS.md, 11 stale-checkbox via Phase 5 CI transitive coverage) | `multi_run.rs` is pure-stdlib (no statrs/nalgebra); `glob` 0.3 + `tinytemplate` 1.x are the only NEW deps in Phases 4-5 |
| v1.1 | 256 tests passing (workspace) | 32/32 v1.1 reqs satisfied (audit `tech_debt`: 1 missing VERIFICATION.md doc + 1 cross-file code-duplication acknowledged for v1.2) | `sha2` added as dev-dep only (frozen-schema test); `axes.rs`, `score.rs`, `polar.rs`, `recommend-cell.{md,html}.tmpl` all pure-stdlib + tinytemplate; no new runtime deps |

### Top Lessons (Verified Across Milestones)

1. **Decorate-not-rewrite holds at scale.** v1.0 locked the v1 schema; v1.1 added 32 requirements without touching it. Sidecars + aggregator decoration + frozen-schema SHA-256 test = self-policing discipline.
2. **Two-template-one-struct + sentinel sync test catches drift at compile time.** v1.0 introduced the WR-01 pattern (winner tiebreak); v1.1 reapplied it to per-cell artifacts (Markdown + HTML cards from `CellRecommendation`).
3. **Audit-before-close distinguishes gap from debt.** v1.1's `tech_debt` audit verdict proceeded with documented deferral; a `gaps_found` verdict would have inserted closure phases. The distinction preserves velocity without losing visibility.
4. **Cross-surface SSoT helpers prevent drift.** v1.0 used `BTreeMap` over `HashMap` for iteration determinism; v1.1 added `column_header_with_arrow` SSoT in `axes.rs` consumed by both Markdown and HTML emitters. Pattern: one module owns the contract; tests defend the boundary.
5. **`/gsd:complete-milestone` deserves a fresh context.** v1.0 closed in one session; v1.1 needed 4 rounds. The workflow's heaviest single step (PROJECT.md evolution review) plus retrospective + archive moves consistently exceeded 33% remaining budget. Future milestones should start the close from `/clear`.

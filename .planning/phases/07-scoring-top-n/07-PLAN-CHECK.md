---
phase: 07-scoring-top-n
checked_at: 2026-05-26
plans_checked: 2
verdict: NEEDS_REVISION
blockers: 2
warnings: 4
---

# Phase 7 Plan Check

> Goal-backward verification of `07-01-PLAN.md` (score.rs, data-only) and `07-02-PLAN.md` (recommend.rs, prose-aware) against ROADMAP Phase 7 success criteria, REQUIREMENTS SCORE-01..04 + REC-01..02 + TEST-03..05, and the locked decisions in 07-CONTEXT.md.

## Per-Question Verdicts

### Q1: Goal coverage — 5 ROADMAP success criteria

| Success Criterion | Covered By | Verdict |
|---|---|---|
| `lower_is_better_axis_inverts_correctly` test | Plan 01, Task 1, `<behavior>` line 178-179 (verbatim test name; input `[100.0, 200.0, 300.0]` Lower → `[100.0, 50.0, 0.0]`) | PASS |
| `normalize_axis_p10_p90_clips_one_outlier_each_tail_at_n18` test | Plan 01, Task 1, `<behavior>` line 183 + `<action>` line 201 (`floor(0.10*n)` and `floor(0.90*n).min(n-1)`; at n=18: lo=1, hi=16) | PASS |
| `composite_score_summation_order_matches_axes_rs_constant_order` test | Plan 01, Task 2, `<behavior>` line 251 (TEST-04 verbatim; two axis-rotated 100.0 spike cells) | PASS |
| `tied_cells_break_alphabetically_for_determinism` test | Plan 01, Task 2, `<behavior>` line 255 (`(jemalloc, alpine)` < `(ptmalloc, wolfi)`) | PASS |
| `CellRecommendation` struct + `top_n_cells` + `TOP_N_*` constants | Plan 02, Task 1 (constants + struct) + Task 2 (`top_n_cells`) | PASS |

**Verdict: PASS** — Every ROADMAP success criterion is mapped to a verbatim-named test in one of the two plans.

---

### Q2: Requirement coverage — 9 requirements

| Requirement | Plan | Task | Verdict |
|---|---|---|---|
| SCORE-01 | 01 | 1 | PASS — `normalize_axis` with direction-aware inversion + 3 tests (higher_keeps, lower_inverts, clamped_0_100) |
| SCORE-02 | 01 | 1 | PASS — p10/p90 winsorization step in algorithm + verbatim test name |
| SCORE-03 | 01 | 2 | PASS — `score_cells` with `MEASUREMENT_AXES.iter()` + `composite_uses_equal_weights_one_eighth_per_axis` test |
| SCORE-04 | 01 | 2 | PASS — `top_n` with `(composite DESC, alloc ASC, env ASC)` stable sort + 3 tests |
| REC-01 | 02 | 1+2 | PASS — `CellRecommendation` struct (11 fields) + `top_n_cells` + 9 prose-derivation tests |
| REC-02 | 02 | 1 | PASS — `TOP_N_SPIDER=3, TOP_N_TABLE=5, TOP_N_TOTAL=10` + `top_n_constants_match_locked_values` test |
| TEST-03 | (Phase 6 inheritance) | — | PASS — Plan 02 frontmatter line 14 lists TEST-03 as inherited from Phase 6 (`loader::tests::load_security_metas_returns_btreemap_sorted_by_env`); RESEARCH.md §6 confirms shipped at `loader.rs:508`. No Phase 7 work needed. |
| TEST-04 | 01 | 2 | PASS — verbatim test name `composite_score_summation_order_matches_axes_rs_constant_order` |
| TEST-05 | 01 | 2 | PASS — verbatim test name `nan_input_does_not_corrupt_score` |

**Verdict: PASS** — All 9 requirements mapped. Plan 01 frontmatter `requirements:` lists SCORE-01..04 + TEST-04 + TEST-05; Plan 02 frontmatter `requirements:` lists REC-01 + REC-02 + TEST-03. Together they cover all 9.

---

### Q3: Module split fidelity

**Plan 01 (score.rs data-only):**
- Plan 01 `<action>` Task 1 line 213-214: "DO NOT include `CellRecommendation`, `top_n_cells`, `cell_is_suspect`, or `TOP_N_*` constants — Plan 07-02 owns those."
- Plan 01 acceptance criteria line 227: `grep -cE "weight_hint|TOP_N_|CellRecommendation|top_n_cells"` returns `0`.
- Plan 01 verification line 344: `grep -cE "tldr|strengths|weaknesses|recommended_for|avoid_for|CellRecommendation|TOP_N_"` returns `0`.

**Plan 02 (recommend.rs prose-aware):**
- Plan 02 Task 1 `<action>` line 252: "DO NOT modify `crates/alloc-bench-aggregator/src/score.rs` — Plan 07-01 owns it."
- Plan 02 Task 2 `<action>` line 339: "DO NOT modify `score.rs`."
- Plan 02 verification line 392-396: `cargo test -p alloc-bench-aggregator score::` regression check; `git diff --stat` shows ONE file touched.

**Verdict: PASS** — Both plans hard-fence the boundary with explicit "DO NOT" directives and grep-based acceptance gates.

---

### Q4: Existing-test preservation (10 tests in recommend.rs untouched)

Plan 02 references "10 existing tests" consistently (frontmatter line 21: "10 tests still pass"; objective line 47: "EXISTING 10 `recommendations()` unit tests MUST remain byte-unchanged"; Task 1 acceptance line 268-269: "10 existing recommend::tests::winner_picker_* tests still pass with NO body modifications: `cargo test -p alloc-bench-aggregator recommend::tests::winner_picker --no-fail-fast` reports '10 passed'").

The same gate is repeated in Task 2 acceptance line 352. RESEARCH §4 explicitly resolves the "13 vs 10" discrepancy (CONTEXT and original brief say 13; actual count is 10).

**Pre-commit `cargo test` proof:** Plan 02 Task 1 `<verify>` runs `cargo test -p alloc-bench-aggregator recommend::tests` and Task 2 `<verify>` repeats. Both gates require ≥16 (Task 1) and ≥21 (Task 2) tests passing including the 10 existing.

**Verdict: PASS** — The 10-tests-untouched gate is wired into both Task 1 and Task 2 acceptance criteria with explicit verbatim grep on `winner_picker` test names.

---

### Q5: Composite-score determinism

**`MEASUREMENT_AXES.iter()` constant traversal:**
- Plan 01 Task 2 `<action>` line 278: `MEASUREMENT_AXES.iter().map(|spec| cell.axes.get(spec.key).copied().unwrap_or(0.0) * 0.125).sum::<f64>()`. Iterator-traversal mandatory.
- Plan 01 acceptance line 301: `grep -c "MEASUREMENT_AXES.iter()" crates/alloc-bench-aggregator/src/score.rs` returns at least `1`.
- Plan 01 acceptance line 302: `grep -cE "\.collect::<Vec<\(.*key.*f64\)>"` returns `0` (NO collected pair-Vec — single-ULP-drift guard).

**Stable sort by `(composite DESC, alloc ASC, env ASC)`:**
- Plan 01 Task 2 `<action>` line 283: "Implementation EXACTLY as shown in `<interfaces>` block — `sort_by` with `partial_cmp(...).unwrap_or(Equal).then_with(alloc).then_with(env)`, then `truncate(n)`."
- `<interfaces>` block lines 141-151 spells out the sort closure verbatim.

**Verdict: PASS** — Both summation order and tiebreak chain are spelled out verbatim, with grep-based defense against future regression.

---

### Q6: Prose derivation rules

| Rule | Plan 02 Location | Verdict |
|---|---|---|
| Strengths = top-2 axes alphabetical key tiebreak, stored as `MEASUREMENT_AXES[i].label` | `<interfaces>` line 137 + Task 1 `<action>` line 234 (sort DESC + alphabetical key tiebreak) + label lookup line 234 | PASS |
| Weaknesses = bottom-2 axes alphabetical key tiebreak | `<interfaces>` line 140 + Task 1 `<action>` line 237 (sort ASC, same tiebreak) | PASS |
| `tldr` template `format!("{alloc}/{env} — strong on {top}, weak on {bottom}.")` | `<interfaces>` line 143 + Task 1 `<action>` line 241 (em-dash via `\u{2014}`) + verbatim test assertion line 209 | PASS — em-dash is `\u{2014}` not ASCII hyphen; test pin's exact string `"jemalloc/alpine — strong on CPU-bound throughput, weak on Memory / fragmentation."` |
| `recommended_for` reuses `recommendations()` winners | `<interfaces>` line 147 + Task 2 `<action>` line 298-302 (`winners_by_class` helper) | FLAG — see below |
| `avoid_for` = bottom-2 across class rankings | `<interfaces>` line 151 + Task 2 `<action>` line 304-306 (`losers_by_class` helper) | PASS |

**FLAG on `recommended_for`:** The CONTEXT.md decision says `recommended_for` "Re-uses winner detection". Plan 02 Task 2 `<action>` line 290 acknowledges this but explicitly diverges: "`recommendations()` returns `Recommendation.allocator` as a String, not `(alloc, env)`; the helper `winners_by_class` must determine the per-cell winner by `(alloc, env)` mean across class scenarios. **This is a NEW helper, not a wrapper of `recommendations()`.**"

This is technically correct (the existing `recommendations()` loses env granularity), but it means the Plan 02 implementation does NOT literally re-use `recommendations()` — it re-implements the per-class winner logic at `(alloc, env)` granularity. The behavior is consistent with the CONTEXT decision (winner detection re-used at the algorithmic level), but the wording in Task 2 line 162 of `<interfaces>` ("re-implement a thin per-class winner picker") may confuse the executor into thinking the existing `recommendations()` is structurally re-used. **WARNING (not blocker)** — the algorithmic intent is preserved (mean throughput per class, alphabetical tiebreak), but the wording should clarify "winner detection logic re-used at (alloc, env) granularity; the existing `recommendations()` String-based output is not consumed".

**Verdict: PASS with WARNING** on the `recommended_for` re-implementation wording.

---

### Q7: Suspect-flag aggregation

- Plan 02 Task 1 `<action>` line 230: `fn cell_is_suspect(cell_runs: &[Run]) -> bool` body: `cell_runs.iter().any(|r| is_suspect(&r.harness))`.
- Plan 02 Task 1 `<read_first>` line 199: References `html.rs:55-57` `pub(crate) fn is_suspect(h: &HarnessInfo) -> bool { h.samples_count < 1_000 || h.warmup_duration_s < 5.0 }`.
- RESEARCH §3 confirms `is_suspect` is already imported at `recommend.rs:28`.
- Plan 02 Task 2 `<action>` line 318-325 changes the signature to generic `IntoIterator` form so `top_n_cells` can pass `Vec<&Run>` without cloning Run (which doesn't derive Clone per output.rs).

**Verdict: PASS** — Cell-level OR aggregation reuses `html::is_suspect`; the Task 2 signature refinement is forced by the v1 schema's no-Clone constraint and is justified inline.

---

### Q8: Top-N constants

- Plan 02 Task 1 `<action>` line 225 + `<interfaces>` lines 91-98: `pub const TOP_N_SPIDER: usize = 3`, `pub const TOP_N_TABLE: usize = 5`, `pub const TOP_N_TOTAL: usize = 10`.
- Plan 02 acceptance lines 258-260: 3 grep checks on the verbatim constant declarations.
- Plan 02 verbatim test `top_n_constants_match_locked_values` at Task 1 `<behavior>` line 204 asserts all 3 values.

**Verdict: PASS** — Constants defined verbatim with grep-based + test-based double gate.

---

### Q9: Atomic-commit hygiene

| Plan | Tasks | Files Touched | Estimated LOC | Verdict |
|---|---|---|---|---|
| 01 | 2 | `score.rs` (NEW ~250 LOC) + `main.rs` (1-line edit) | ~250 | PASS — within 2-3 task target |
| 02 | 2 | `recommend.rs` (additive ~400 LOC) | ~400 | PASS — within 2-3 task target |

Plan 02 conventional-commit message at line 414: `feat(07-02): land CellRecommendation + top_n_cells + TOP_N_* constants (REC-01, REC-02)`. Plan 01 commit at line 359: `feat(07-01): land score.rs data-only normalization + composite + top_n (SCORE-01..04, TEST-04, TEST-05)`. Both follow the project's `feat(NN-PP)` convention from CLAUDE.md.

**Verdict: PASS** — Both plans within the 2-3 task budget; no mega-plans; conventional-commit prefixes correctly scoped.

---

### Q10: Edge-case coverage in Plan 01

Three edge-case tests in Task 1 `<behavior>` lines 186-188:

- `normalize_axis_empty_input_returns_empty` — `&[]` → `vec![]` (no panic)
- `normalize_axis_single_value_returns_mid_range` — `&[42.0]` → `vec![50.0]`
- `normalize_axis_all_equal_returns_mid_range` — `&[7.0, 7.0, 7.0]` → `vec![50.0, 50.0, 50.0]` (avoids div-by-zero)

Algorithm steps in `<interfaces>` lines 113-115:
2. Empty input → return `Vec::new()`.
3. Single-value input → return `vec![50.0]` (deterministic mid-range).
4. All-equal input (max - min ≤ 1e-12 after p10/p90) → return `vec![50.0; n]` (avoid div-by-zero).

**Verdict: PASS** — All three edge cases enumerated in CONTEXT §Claude's Discretion are covered with verbatim test names + algorithm steps.

---

### Q11: Heuristic-axis defense in Plan 01

Plan 01 Task 2 `<behavior>` line 252:
> `heuristic_axes_cannot_promote_worst_measured_cell_to_top_1`: 18-cell synthetic fixture. One cell ("ptmalloc", "wolfi") has the 6 measured axes all at `5.0` (near-bottom) AND both heuristic axes (`image_size_efficiency`, `security_posture`) at `100.0`. The other 17 cells have measured axes spread `[40.0, 90.0]` and heuristic axes `[0.0, 50.0]`. After `score_cells` + `top_n(_, 1)`, assert `top_n[0].alloc != "ptmalloc" || top_n[0].env != "wolfi"` — the heuristic-100 cell does NOT win rank 1.

Acceptance criterion line 306: explicit `cargo test ... ::heuristic_axes_cannot_promote_worst_measured_cell_to_top_1` gate.

**Verdict: PASS** — Verbatim test name from CONTEXT.md + 18-cell synthetic fixture with the exact worst-case construction the CONTEXT requires.

---

### Q12: No scope creep

**Plan 01 boundary (data-only):**
- `<action>` line 213-215: "DO NOT include `CellRecommendation`, `top_n_cells`, `cell_is_suspect`, or `TOP_N_*` constants"; "DO NOT add `pub use` re-exports."
- Task 2 `<action>` line 290-292: "DO NOT modify `recommend.rs` in this plan"; "DO NOT touch `crates/alloc-bench-core/src/output.rs`"; "DO NOT modify any other lines of `main.rs` — only the single `mod score;` insertion."
- Acceptance line 227: `grep -cE "weight_hint|TOP_N_|CellRecommendation|top_n_cells"` returns `0`.

**Plan 02 boundary (prose-aware):**
- Task 1 `<action>` line 252: "DO NOT modify `crates/alloc-bench-aggregator/src/score.rs`."
- Task 2 `<action>` line 339-340: "DO NOT modify `score.rs`. DO NOT modify `crates/alloc-bench-core/src/output.rs`."
- Verification line 398: `grep -cE "tinytemplate|render_html|recommend-cell\.md\.tmpl|recommend-cell\.html\.tmpl"` returns `0` (no template touch).

**Verdict: PASS** — Both plans rigidly scoped with explicit DO NOT directives + grep-based verification gates.

---

## BLOCKERS

### B-01: Plan 01 acceptance criterion threshold mismatch (Task 1)

**Location:** `07-01-PLAN.md` line 217 (Task 1 `<verify>` automated command).

**Issue:** The verify command's grep pattern is `grep -E "test result: ok\\. (6|7) passed"` — it accepts EITHER 6 OR 7 tests passing. But Task 1's `<behavior>` enumerates 6 NEW test names (3 SCORE-01 + 1 SCORE-02 + 3 edge-case = 7 tests, not 6). Counting:
1. `higher_is_better_axis_keeps_order`
2. `lower_is_better_axis_inverts_correctly`
3. `normalized_scores_are_clamped_to_0_100`
4. `normalize_axis_p10_p90_clips_one_outlier_each_tail_at_n18`
5. `normalize_axis_empty_input_returns_empty`
6. `normalize_axis_single_value_returns_mid_range`
7. `normalize_axis_all_equal_returns_mid_range`

That is **7 tests**, but the regex `(6|7) passed` allows 6 to pass. The `done` block at line 231 says "SCORE-01 + SCORE-02 + 3 edge-case tests" = 3+1+3 = 7. The acceptance criterion at line 228 says "7 score::tests pass". The verify-regex `(6|7)` is internally inconsistent with the success criterion of 7. If the executor accidentally drops one test, the verify gate still passes at 6.

**Severity:** BLOCKER — the verify regex creates a silent off-by-one slip path. A future contributor or executor following the plan to the letter could ship 6 tests and the gate passes.

**Fix:** Change line 217 from:
```
grep -E "test result: ok\\. (6|7) passed"
```
to:
```
grep -E "test result: ok\\. 7 passed"
```

---

### B-02: Plan 02 Task 2 cell_is_suspect signature thrash creates a Plan 01 dependency

**Location:** `07-02-PLAN.md` Task 2 `<action>` lines 318-325 (cell_is_suspect signature refinement) and Task 1 `<action>` line 230 (Task 1 cell_is_suspect signature).

**Issue:** Plan 02 Task 1 (line 230) defines `fn cell_is_suspect(cell_runs: &[Run]) -> bool`. Plan 02 Task 2 (line 324) then changes the signature to a generic `fn cell_is_suspect<'a, I: IntoIterator<Item = &'a Run>>(cell_runs: I) -> bool` to avoid cloning `Run` (which doesn't derive Clone per the v1 schema GUARD-01 freeze).

This is a **two-step task internal to Plan 02** — Task 1 lands the wrong signature, Task 2 retrofits it. The Task 1 acceptance criteria at line 270 demands "6 NEW Task 1 tests pass" using the simpler signature. If Task 2 changes the signature mid-plan, the Task 1 tests *should* still pass because `&[Run]` satisfies `IntoIterator<Item = &Run>` — but this is a subtle Rust trait coercion that depends on slice's `IntoIterator` impl yielding `&Run` (it does, but only because of the `impl<'a, T> IntoIterator for &'a [T]` standard library impl).

The risk is that the executor lands Task 1 with the simple signature, runs Task 1 tests (pass), then in Task 2 changes the signature and finds Task 1's tests now break in some subtle way (e.g., if the executor wrote `cell_is_suspect(&runs[..])` vs `cell_is_suspect(runs.as_slice())` — both should work but only with very specific lifetime annotations on the generic).

**Severity:** BLOCKER — the plan introduces a known signature thrash mid-plan that creates a fragile passing-state for Task 1 tests. The fragility is documented (Task 2 line 326 "Task 1's tests do NOT need to change because `&[Run]` continues to satisfy the bound") but it puts the executor in the position of having to verify a non-trivial trait coercion mid-plan.

**Fix:** Move the generic `IntoIterator` signature into Task 1 from the start. Update Task 1 `<action>` line 230 to:
```rust
fn cell_is_suspect<'a, I: IntoIterator<Item = &'a Run>>(cell_runs: I) -> bool {
    cell_runs.into_iter().any(|r| is_suspect(&r.harness))
}
```
Update Task 1's two suspect-flag tests (lines 212-213) to call `cell_is_suspect(&runs[..])` or `cell_is_suspect(runs.iter())` so they exercise the generic signature directly.

Task 2 then **removes** the signature-refinement step (line 323-325) entirely.

---

## WARNINGS

### W-01: Plan 02 Task 2 `recommended_for` algorithmic re-implementation wording (see Q6)

Tighten Task 2 `<action>` line 290 to clarify that "winner detection logic" (the algorithm) is re-used, not the existing `recommendations()` function output. Suggested replacement (line 290 of `07-02-PLAN.md`):

> The CONTEXT decision says `recommended_for` "Re-uses winner detection (does NOT re-use rationale strings)". The existing `recommendations()` returns `Recommendation.allocator` as a `String` and loses `(alloc, env)` granularity needed for cell-level recommendation. Therefore `winners_by_class` re-implements the **algorithm** (per-class scenario filtering + per-(alloc, env) mean throughput + max-mean-wins + alphabetical tiebreak) at the (alloc, env) granularity. The CONTEXT contract is satisfied because the *winner-detection logic* is identical to `recommendations()`'s underlying logic; only the output type differs.

### W-02: Plan 02 Task 2 `losers_by_class` not in CONTEXT decisions

**Location:** `07-02-PLAN.md` Task 2 `<action>` line 304-306.

**Issue:** The CONTEXT decision (REC-01 §Per-cell Prose Source line 47) says `avoid_for` is "every `WorkloadClass` where this `(alloc, env)` cell finishes in the **bottom 2** across the 18-cell ranking for that class." Plan 02 introduces a private `losers_by_class` helper to compute this. The helper is not in the CONTEXT.md (which doesn't dictate helper structure — that's Claude's discretion per CONTEXT line 71-77), but the algorithm choice ("bottom 2 by ASC mean, alphabetical tiebreak") needs to be locked in the algorithm spec.

The current Task 2 `<action>` line 305 says "sorted ASC by mean; alphabetical tiebreak by (alloc, env). If a class has fewer than 2 measured cells, insert empty BTreeSet." This is correct but is buried in prose. RESEARCH §6 doesn't pin the bottom-2 detection beyond "bottom-2 in class rankings".

**Fix (suggested):** Add an explicit acceptance criterion (line 348 area) that gates the `bottom_2` semantic:
```
- Test `cell_recommendation_avoid_for_is_bottom_2_class_rankings` (Task 2 line 291) constructs a 4-cell synthetic fixture where `(ptmalloc, debian-slim)` is bottom-2 in two classes; assertion that `avoid_for` contains exactly those 2 class labels in alphabetical order.
```

This is currently in `<behavior>` line 291 but not in `<acceptance_criteria>`. Promote to acceptance to harden the gate.

### W-03: `env_short_name` duplicated across `score.rs` and `recommend.rs`

**Location:** `07-02-PLAN.md` Task 2 `<action>` line 327-330.

**Issue:** Plan 02 Task 2 line 330 explicitly notes: "DUPLICATE-AVOIDANCE NOTE: this helper is also defined in `score.rs::compute_axes` per Plan 07-01. Both copies are private to their respective modules. Phase 7 ships two copies; v1.2 may consolidate to a single `crate::env::short_name` helper. NOT a Phase 7 deliverable — call out the duplication in the doc comment only."

This is a self-inflicted code duplication. The two copies must agree byte-for-byte on the env-extraction logic (split on `:`, then split on `-`, take `[1]`, fall back to `"host"`). Any drift between the two copies will break the cross-module key-lookup invariant in `top_n_cells` (which filters runs by `env_short_name(r) == cell.env`).

**Severity:** WARNING — the duplication is acknowledged and documented, but it's a known fragility. A future maintainer fixing a bug in one copy and not the other will silently break top_n_cells's suspect-flag aggregation.

**Fix:** Either (a) accept the duplication with a clear inline comment in BOTH copies referencing the other, OR (b) extract `env_short_name` to a small helper in `loader.rs` (which is consumed by both `score.rs` and `recommend.rs`) BEFORE Plan 07-01 lands. Option (b) is a cleaner architecture but adds scope to Plan 01. Option (a) is acceptable for v1.1 if the doc comment in BOTH `score.rs` and `recommend.rs` explicitly cross-references the duplicate.

Recommend (a) for Phase 7 — but Plan 01 Task 2 `<action>` (line 269) currently does NOT include the cross-reference doc comment in `score.rs`'s `env_short_name`. Add it.

### W-04: Plan 01 Task 2 verify-regex range too permissive

**Location:** `07-01-PLAN.md` Task 2 `<verify>` line 295.

**Issue:** The verify command's grep pattern is `grep -E "test result: ok\\. (1[2-9]|2[0-9]) passed"` — it accepts anywhere from 12 to 29 tests. The `<behavior>` and `<acceptance_criteria>` enumerate exactly:
- 7 from Task 1 (already running)
- 5 NEW from Task 2: composite_uses_equal_weights, composite_score_summation_order, heuristic_axes_cannot_promote, tied_cells_break_alphabetically, top_n_returns_at_most_n, top_n_of_empty_returns_empty, nan_input_does_not_corrupt_score, compute_axes_consumes_runs_metas_and_security_alphabetically
   = **8 NEW** tests (the spec line 305 says "5+ from Task 2: [8 names listed]" — undercounts).

Total: 7 + 8 = **15 tests**. The verify-regex `(1[2-9]|2[0-9])` is overly permissive at the low end (accepts 12) and overly generous at the high end (accepts 29). 

**Severity:** WARNING — correctness gate is too loose; could mask 3 dropped tests.

**Fix:** Change line 295 from:
```
grep -E "test result: ok\\. (1[2-9]|2[0-9]) passed"
```
to:
```
grep -E "test result: ok\\. 15 passed"
```

(or `(15|16) passed` if the executor adds defensive coverage; pick a tight upper bound).

Same issue exists at `07-02-PLAN.md` Task 1 line 255 (regex `(1[6-9]|2[0-9])` — 16 to 29) and Task 2 line 343 (regex `(2[1-9]|3[0-9])` — 21 to 39). Tighten both.

---

## Final Verdict: NEEDS_REVISION

**Blockers:** 2
- B-01: Plan 01 Task 1 verify-regex permits 6 OR 7 (off-by-one slip for SCORE-01 + SCORE-02 + edge-case test set).
- B-02: Plan 02 cell_is_suspect signature thrash mid-plan creates fragile Task 1 → Task 2 trait coercion path.

**Warnings:** 4
- W-01: `recommended_for` re-implementation wording could mislead executor (Q6).
- W-02: `losers_by_class` bottom-2 algorithm pinned in `<behavior>` only; promote to acceptance criteria.
- W-03: `env_short_name` duplicated across `score.rs` and `recommend.rs`; cross-reference doc comment missing in `score.rs` (Plan 01).
- W-04: All three plan verify-regexes (Plan 01 Task 1, Plan 01 Task 2, Plan 02 Task 1, Plan 02 Task 2) use overly permissive ranges; tighten to exact counts.

**PASS items:** 12 (Q1-Q12 all PASS on substance).

The plans are substantively complete and goal-aligned. The blockers are mechanical (verify-regex tightening + signature-thrash re-ordering), not architectural. After fixing B-01 and B-02 (and ideally W-01..W-04), Phase 7 plans are ready for execution.

**Recommendation:** Return to planner with the 2 blockers + 4 warnings; the planner can fix all 6 in a single revision pass since they're all line-level edits to the existing plans (no architectural changes needed).

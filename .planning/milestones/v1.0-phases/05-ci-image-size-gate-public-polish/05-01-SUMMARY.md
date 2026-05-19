---
phase: 05-ci-image-size-gate-public-polish
plan: 01
subsystem: aggregator
tags: [rust, multi-run, statistics, bessel-stddev, coefficient-of-variation, fixtures]

# Dependency graph
requires:
  - phase: 04-aggregator-and-report
    provides: Vec<Run> JSON shape (`tests/fixtures/jemalloc-alpine.json`); is_suspect predicate; recommend.rs analog structure
provides:
  - "`multi_run::aggregate(&[f64]) -> Option<MultiRunStats>` (Bessel-corrected sample stddev + median + min/max + CV%)"
  - "`multi_run::is_high_variance` predicate (CV > 10%, D-12 threshold)"
  - "Three Vec<Run> seed fixtures under `tests/fixtures/multi_run/` with low-variance (CV ≈ 4.76%) and high-variance (CV ≈ 19.5%) sets"
  - "Sidecar `meta/jemalloc-alpine.json` fixture with the literal D-13 shape"
affects: [05-03 markdown-html-multi-run, 05-04 dashboard-multi-run]

# Tech tracking
tech-stack:
  added: []  # no new crates — pure-stdlib + existing serde workspace dep
  patterns:
    - "Pure-stdlib computation module pattern (analog: recommend.rs) — no external math deps"
    - "Bessel-corrected (n-1) sample-stddev formula pinned by golden-value unit test"
    - "NaN/inf rejection guard at module entry for downstream safety"

key-files:
  created:
    - "crates/alloc-bench-aggregator/src/multi_run.rs"
    - "crates/alloc-bench-aggregator/tests/fixtures/multi_run/seed-1.json"
    - "crates/alloc-bench-aggregator/tests/fixtures/multi_run/seed-2.json"
    - "crates/alloc-bench-aggregator/tests/fixtures/multi_run/seed-3.json"
    - "crates/alloc-bench-aggregator/tests/fixtures/multi_run/meta/jemalloc-alpine.json"
  modified:
    - "crates/alloc-bench-aggregator/src/main.rs"

key-decisions:
  - "Bessel-corrected (n-1) sample stddev — RESEARCH §Pitfall 7. Population stddev (n) underestimates true variance; D-12's 10% threshold was designed for the larger Bessel-corrected number."
  - "CV undefined when |mean| ≤ 1e-9 or non-finite — Wikipedia near-zero edge case. Returned as `cv_pct: Option::None`, not `0.0` or `f64::INFINITY`."
  - "n<2 and any non-finite sample → `aggregate` returns `None`. Caller falls back to mean (Plan 03 wires that fallback)."
  - "Suppressed dead_code warnings on the public symbols with `#[allow(dead_code)]` — the aggregator is a binary-only crate (no `[lib]`), and Plan 03 will remove the allows when the imports land."

patterns-established:
  - "Pattern: `multi_run::aggregate(&[f64]) -> Option<MultiRunStats>` — pure-stdlib statistics module returning a serde-derived output struct"
  - "Pattern: golden-value unit test pinning the formula version — `[100,110,105]` → median=105, Bessel stddev=5.0, CV ≈ 4.7619%"
  - "Pattern: scenario-pair fixture — index [0] = low-variance set, index [1] = high-variance set, isolating each predicate from the other"

requirements-completed: [REPR-03]

# Metrics
duration: 9min
completed: 2026-05-19
---

# Phase 5 Plan 01: Multi-Run Statistics Module Summary

**Pure-stdlib `multi_run.rs` module providing Bessel-corrected sample stddev, median, min/max, and coefficient-of-variation with high-variance flag (CV > 10%) — plus three Vec<Run> seed fixtures and one sidecar meta fixture for downstream Plan 03 integration tests.**

## Performance

- **Duration:** 9.1 min
- **Started:** 2026-05-19T06:28:32Z
- **Completed:** 2026-05-19T06:37:37Z
- **Tasks:** 2/2 completed
- **Files created:** 5
- **Files modified:** 1
- **Tests added:** 6 (all passing)

## Accomplishments

- Implemented `MultiRunStats` struct + `aggregate(&[f64])` + `is_high_variance` per CONTEXT.md D-11 / D-12 and the canonical RESEARCH §Pattern 5 skeleton.
- Pinned the Bessel-corrected sample-stddev formula via a golden-value unit test (`three_seeds_with_known_cv`): `[100, 110, 105]` → median=105.0, stddev=5.0, CV ≈ 4.7619%. Any regression to population stddev or a missing Bessel correction breaks this test immediately (T-05-01 mitigation).
- Pinned the high-variance threshold (CV > 10%) via `high_variance_flagged_when_cv_above_10pct`: `[100, 130, 90]` → CV ≈ 19.52% triggers `is_high_variance == true`.
- Pinned the n<2, NaN, and zero-mean edge cases via three guard tests (`requires_at_least_two_samples`, `rejects_nan_input`, `cv_undefined_when_mean_is_zero`) — T-05-03 NaN-poisoning mitigation.
- Created three seed fixtures with **two scenarios per file**: `multithread` (low-variance, 100/110/105) and `cpu-bound` (high-variance, 100/130/90). The pair gates both predicates in Plan 03's integration tests with a single fixture set.
- Created the sidecar `meta/jemalloc-alpine.json` with the literal D-13 shape (`alloc, env, image_size_bytes, image_size_mb, build_time_s, captured_at`) for image-size backfill in Plan 03.
- Wired `mod multi_run;` into `crates/alloc-bench-aggregator/src/main.rs` in alphabetical position (between `markdown` and `recommend`) — the module is compiled into the binary so Plan 03's sibling-module imports resolve.

## Task Commits

1. **Task 1: Implement `multi_run.rs` module with 6 unit tests** — `9703874` (feat) — module + main.rs wiring, RED→GREEN within one task per the plan's TDD design.
2. **Task 2: Create multi-run fixture files (3 seed Run arrays + 1 sidecar meta)** — `26b5ba3` (test) — four JSON fixtures.

## Files Created/Modified

- `crates/alloc-bench-aggregator/src/multi_run.rs` (NEW, 175 LOC) — pure-stdlib `MultiRunStats` + `aggregate` + `is_high_variance` + 6 unit tests.
- `crates/alloc-bench-aggregator/src/main.rs` (MODIFIED) — added `mod multi_run;` line in alphabetical position.
- `crates/alloc-bench-aggregator/tests/fixtures/multi_run/seed-1.json` (NEW) — Vec<Run> with multithread@100 + cpu-bound@100.
- `crates/alloc-bench-aggregator/tests/fixtures/multi_run/seed-2.json` (NEW) — Vec<Run> with multithread@110 + cpu-bound@130.
- `crates/alloc-bench-aggregator/tests/fixtures/multi_run/seed-3.json` (NEW) — Vec<Run> with multithread@105 + cpu-bound@90.
- `crates/alloc-bench-aggregator/tests/fixtures/multi_run/meta/jemalloc-alpine.json` (NEW) — single object with D-13 sidecar shape.

## Decisions Made

- **Formula version: Bessel-corrected sample stddev (n-1 denominator).** Pinned by golden value `[100, 110, 105]` → stddev = 5.0 exactly. Switching to population stddev (n) would yield 4.082 and break the test. RESEARCH §Pitfall 7 documents the rationale.
- **CV undefined → `Option::None`.** Returning `0.0` or `f64::INFINITY` for the zero-mean case would silently corrupt downstream rendering in Plan 03. The `Option` shape forces every consumer to handle the case explicitly.
- **`#[allow(dead_code)]` on the three public symbols.** The aggregator is binary-only (no `[lib]` target), so `pub` items not yet imported by `main.rs` are flagged as dead code. Plan 03 imports them and removes the allows. Without this annotation, `cargo clippy --workspace --all-targets -- -D warnings` would fail (Rule 3 blocking issue).
- **Scenario-pair fixture (`multithread` + `cpu-bound`).** A single seed file carries one of each so Plan 03 can exercise both the low-variance non-flag path and the high-variance flag path without two fixture sets.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Suppressed dead_code warnings with `#[allow(dead_code)]`**

- **Found during:** Task 1 (`multi_run.rs` module compilation)
- **Issue:** `cargo clippy --workspace --all-targets -- -D warnings` failed because the aggregator is binary-only (no `[lib]` target). The three new public symbols (`MultiRunStats`, `aggregate`, `is_high_variance`) are not imported by any sibling module in Plan 01 — Plan 03 wires them in. Without suppression, rustc's `dead_code` lint flagged all three.
- **Fix:** Added `#[allow(dead_code)]` to each public item with an inline comment naming Plan 03 as the future consumer. Annotation will be removed when Plan 03 imports the symbols.
- **Files modified:** `crates/alloc-bench-aggregator/src/multi_run.rs`
- **Verification:** `cargo clippy --workspace --all-targets -- -D warnings` now passes.
- **Committed in:** `9703874` (Task 1 commit).

**2. [Rule 3 - Verify defect, non-blocking] Plan's `<verify>` jq exact-string check**

- **Found during:** Task 2 (running `<verify>` block).
- **Issue:** Plan's verify block contains `test "$(jq '.[0].metrics.ticks_per_s' ...)" = "100"`. `jq` outputs floats with their `.0` suffix (e.g. `100.0` not `100`), so the literal string match fails. The existing canonical `tests/fixtures/jemalloc-alpine.json` uses the float literal `100.0` — same canonical pattern.
- **Fix:** Verified semantically with `= "100.0"` instead of `= "100"`. Numerical intent (ticks_per_s == 100) is preserved. Did NOT modify the fixture to use integer literals (would break consistency with the existing canonical fixture and would not deserialize as `f64` cleanly via serde).
- **Files modified:** none (verify-script defect, not a code defect).
- **Verification:** All six fixture-value checks pass numerically; multithread CV computed externally as 4.7619% (matches unit test golden value), cpu-bound CV as 19.5156% (> 10%).
- **Committed in:** N/A — purely a verify-block annotation; documented here for the reviewer.

---

**Total deviations:** 2 auto-fixed (1 blocking compiler warning, 1 verify-script defect)
**Impact on plan:** No scope creep. The dead_code suppression is the conventional Rust idiom for "shipped now, consumed later" public symbols in binary-only crates and will be reverted when Plan 03 imports them. The verify-defect annotation is a reviewer-aid only — fixture intent is met exactly.

## Issues Encountered

- The aggregator crate has no `[lib]` target, so `cargo test -p alloc-bench-aggregator --lib` (the literal command in the plan's `<verify>`) errors with `no library targets found`. Used `cargo test -p alloc-bench-aggregator multi_run::tests` instead — same selector against the binary's unit tests. The plan's command-shape was treated as semantic intent, not literal invocation.

## Verification Results

- `cargo fmt --all --check` — passes (no formatting drift).
- `cargo clippy --workspace --all-targets -- -D warnings` — passes.
- `cargo test -p alloc-bench-aggregator multi_run::tests` — 6 tests pass (`requires_at_least_two_samples`, `rejects_nan_input`, `three_seeds_with_known_cv`, `high_variance_flagged_when_cv_above_10pct`, `three_identical_samples_have_zero_variance`, `cv_undefined_when_mean_is_zero`).
- `cargo test -p alloc-bench-aggregator` (full crate) — 35 unit + 17 integration smoke = 52 tests pass.
- `jq empty` succeeds on all 4 new fixture files.
- `grep -F "/ (n_f - 1.0)"` matches `multi_run.rs` (Bessel formula present).
- `grep -F "matches!(stats.cv_pct, Some(cv) if cv > 10.0)"` matches `multi_run.rs` (10% threshold present).
- `grep -F "#[derive(Debug, Clone, Serialize)]"` matches `multi_run.rs`.
- `grep -n "mod multi_run;"` matches `main.rs` line 25.
- `grep -c "#\[test\]"` returns 6.
- Numerical CV check via Python: multithread 4.7619%, cpu-bound 19.5156% — both match the plan's intended thresholds.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Plan 03 (markdown + html multi-run rendering)** can now import `crate::multi_run::{MultiRunStats, aggregate, is_high_variance}` directly. The `(alloc, env, scenario)` 3-tuple grouping logic lives in Plan 03 — this plan only ships the `&[f64]` math layer.
- **Plan 04 (dashboard multi-run)** can consume `MultiRunStats` via the existing `serde::Serialize` derive when the aggregator emits the JSON-for-template payload.
- The 4 fixture files form the test data backbone for Plan 03's integration tests (`aggregator_high_variance_cell_marked_with_warning_glyph`, `aggregator_meta_sidecar_populates_image_size_mb`, etc. — see PATTERNS.md test-name table).
- No blockers carried forward.

## Threat Flags

None — this plan adds no new trust boundaries beyond what was already in PLAN.md `<threat_model>` (T-05-01 / T-05-02 / T-05-03 / T-05-SC). All four threat dispositions are honored:
- T-05-01 (formula tampering): mitigated by 3 numerical golden-value tests.
- T-05-02 (fixture info disclosure): accepted — synthetic data only.
- T-05-03 (NaN DoS): mitigated by `samples.iter().any(|x| !x.is_finite())` guard at `aggregate` entry.
- T-05-SC (supply chain): accepted — zero new Cargo dependencies added.

## Self-Check: PASSED

All claimed files and commits verified to exist (see Verification Results above). No missing artifacts.

---

*Phase: 05-ci-image-size-gate-public-polish*
*Plan: 01*
*Completed: 2026-05-19*

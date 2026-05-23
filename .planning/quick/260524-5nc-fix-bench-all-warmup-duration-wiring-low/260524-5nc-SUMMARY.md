---
phase: quick-260524-5nc
plan: 01
type: summary
wave: 1
depends_on: []
tags:
  - cli
  - aggregator
  - justfile
  - claude-md
key-files:
  modified:
    - crates/alloc-bench-cli/src/main.rs
    - crates/alloc-bench-cli/src/run.rs
    - crates/alloc-bench-cli/tests/run_all_smoke.rs
    - crates/alloc-bench-aggregator/src/html.rs
    - crates/alloc-bench-aggregator/src/markdown.rs
    - crates/alloc-bench-aggregator/src/recommend.rs
    - crates/alloc-bench-aggregator/tests/fixtures/jemalloc-alpine.json
    - crates/alloc-bench-aggregator/tests/smoke.rs
    - justfile
    - CLAUDE.md
decisions:
  - "Threshold lowered: `is_suspect` `samples_count < 10_000` → `< 1_000` (the `< 5.0s` warmup arm is unchanged)."
  - "Justfile parameterization over env-var coupling: `run`, `bench-cell`, `bench-all` accept optional `warmup`/`duration` arguments with canonical 5s/60s defaults; `bench-all-smoke` is now a one-liner `just bench-all 1s 5s` (BENCH_SMOKE env-var dropped)."
  - "Synced `markdown::suspect_reason` to the lowered threshold (Rule 1) — `debug_assert_eq!(reason.is_some(), is_suspect(h))` enforces lockstep at runtime; without the sync that contract would silently break."
  - "Pinned `run_all_smoke.rs` integration test to explicit `--warmup 1s --duration 5s` (Rule 3) — the test validates JSON shape, not absolute throughput; the canonical-shape default change inflated it from ~60s to ~650s wall-clock."
metrics:
  duration: ~50min (single executor session)
  completed: 2026-05-24
  tests_total: 82 passing (49 aggregator unit + 28 aggregator integration + 3 cli unit + 1 multithread smoke + 1 run_all smoke)
---

# Quick Task 260524-5nc: Fix `bench-all` Warmup/Duration Wiring + Lower `is_suspect` Threshold Summary

CLI surface, justfile, and aggregator alignment fix: `alloc-bench-cli run-all` now exposes `--warmup` (default `5s`) and `--duration` (default `60s`); the justfile's `run`/`bench-cell`/`bench-all` recipes are parameterized so `just bench-all` is the canonical 5s/60s local-bench run (was silently 1s/5s) while `just bench-all-smoke` and `just ci-bench-cell` keep the smoke shape via explicit flags; the `is_suspect` predicate threshold drops from `< 10_000` to `< 1_000` so healthy 60s runs of slow scenarios no longer trip a false-positive warning.

## What Changed (Files & Commits)

| Commit    | Type      | Files                                                                                                                                  | Purpose                                                                                                                          |
| --------- | --------- | -------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `43f28ec` | feat      | `crates/alloc-bench-cli/src/main.rs`, `crates/alloc-bench-cli/src/run.rs`                                                              | Wire `--warmup`/`--duration` through the `run-all` CLI surface; replace hardcoded `Duration::from_secs(1)`/`(5)` literals.       |
| `5e5476e` | feat      | `crates/alloc-bench-aggregator/src/{html,markdown,recommend}.rs`, `tests/fixtures/jemalloc-alpine.json`, `tests/smoke.rs`              | Lower `is_suspect` `samples_count` threshold from `10_000` → `1_000`; sync the parallel `suspect_reason` low-samples arm; lower fixture/test sample-counts from `5_000` → `500`. |
| `1e5f39d` | refactor  | `justfile`, `CLAUDE.md`                                                                                                                | Parameterize `run`/`bench-cell`/`bench-all` with `warmup`/`duration` args (defaults `5s`/`60s`); `bench-all-smoke` → `just bench-all 1s 5s`; ci-bench-cell pins `1s`/`5s`; CLAUDE.md "Suspect run flagging" bullet updated to `< 1_000`. |
| `f7ec8b5` | test      | `crates/alloc-bench-cli/tests/run_all_smoke.rs`                                                                                        | Pin the integration test to explicit `--warmup 1s --duration 5s` so it stays a ~60s smoke gate (was 650s under the new defaults). |

**Files touched (10 total):**

- **CLI surface (3):** `main.rs`, `run.rs`, `tests/run_all_smoke.rs`
- **Aggregator (5):** `html.rs`, `markdown.rs`, `recommend.rs`, fixture `jemalloc-alpine.json`, `tests/smoke.rs`
- **Build/automation (1):** `justfile`
- **Documentation (1):** `CLAUDE.md`

## How the Pieces Compose

```
[ CLI defaults ]                    [ Justfile recipes (defaults) ]              [ Justfile recipes (smoke) ]
run-all --warmup 5s                 just bench-all      → 5s / 60s               just bench-all-smoke    → 1s / 5s
run-all --duration 60s              just bench-cell     → 5s / 60s               just ci-bench-cell      → 1s / 5s
                                    just run            → 5s / 60s               just run alpine jemalloc 1s 5s

[ Aggregator suspect predicate ]
samples_count < 1_000  OR  warmup_duration_s < 5.0   →   ⚠ suspect badge / italic note
```

The threshold lowering means a healthy 60s `bench-all` run (samples_count typically 6 000–60 000 on slow scenarios) no longer trips false-positive warnings — only genuinely under-sampled runs (`< 1_000`) do.

## Verification Battery (All Steps Passed)

| # | Step                                                              | Outcome                                                  |
| - | ----------------------------------------------------------------- | -------------------------------------------------------- |
| 1 | `cargo check -p alloc-bench-cli -p alloc-bench-aggregator`        | Clean (1.10s incremental).                               |
| 2 | `cargo test -p alloc-bench-aggregator`                            | 49 unit + 28 integration = 77 tests, all passing.        |
| 3 | `cargo test -p alloc-bench-cli`                                   | 3 unit + 1 multithread + 1 run_all = 5 tests, all passing (run_all_smoke: 60.26s). |
| 4 | `cargo build --release -p alloc-bench-cli`                        | Release build clean (19.28s).                            |
| 5 | `alloc-bench-cli run-all --help` advertises `--warmup` (default `5s`) and `--duration` (default `60s`). | OK — both flags + both default values.                  |
| 6 | `just --evaluate`                                                 | Exit 0 (justfile parses cleanly).                        |
| 7 | `just check-matrix`                                               | `[ok] _matrix_cells: 18 valid (env, alloc) tuples; zero cross-libc.` |
| 8 | `is_suspect` predicate body contains no `10_000` reference.       | OK (only a code comment in `markdown.rs` mentions the old value as historical context). |
| 9 | `CLAUDE.md` "Suspect run flagging" bullet says `samples_count < 1_000`. | OK — also includes a parenthetical pointer to this task. |

**Negative checks (must not match):**

- `Duration::from_secs(1)|Duration::from_secs(5)` literals inside the `run_all` function body → absent.
- `BENCH_SMOKE` env-var anywhere in the justfile → absent.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 – Bug] Synced `markdown.rs::suspect_reason` to the new threshold.**

- **Found during:** Task 2 cargo test sweep (3 unit-test failures: `markdown::tests::per_scenario_table_appends_suspect_note_to_low_samples_run`, `recommend::tests::winner_picker_suspect_winner_appends_suspect_suffix`, `html::tests::context_marks_suspect_pairs`).
- **Issue:** The plan's `<interfaces>` block scoped Task 2 strictly to `html.rs::is_suspect`, but `markdown.rs:390` (`let low = h.samples_count < 10_000`) is the parallel low-samples arm that classifies the suspect *reason* for italic notes. There is a `debug_assert_eq!(reason.is_some(), is_suspect(h))` runtime contract on line 402 of `markdown.rs` enforcing lockstep — leaving `markdown.rs` at `< 10_000` while `html.rs` flipped to `< 1_000` would silently break the contract for fixtures with `samples_count` in `[1_000, 10_000)`.
- **Fix:** Lowered `markdown.rs:390` from `< 10_000` to `< 1_000` with a single-source-of-truth comment pointing to `crate::html::is_suspect`. Lowered the synthetic test fixtures in `html::tests::context_marks_suspect_pairs`, `markdown::tests::per_scenario_table_appends_suspect_note_to_low_samples_run`, `markdown::tests::per_scenario_table_marks_both_suspect_predicates`, and `recommend::tests::winner_picker_suspect_winner_appends_suspect_suffix` from `5_000` → `500` (the same downshift the plan prescribed for the integration fixture in `jemalloc-alpine.json`).
- **Files modified:** `crates/alloc-bench-aggregator/src/{html,markdown,recommend}.rs` (in addition to the planned files).
- **Commit:** `5e5476e` (rolled into Task 2's commit).

**2. [Rule 3 – Blocking] Pinned `run_all_smoke.rs` to explicit `--warmup 1s --duration 5s`.**

- **Found during:** Task 4 cargo test sweep — `run_all_smoke` test wall-clock jumped from ~60s to **650.34s** (10 scenarios × 65s under the new 5s/60s defaults).
- **Issue:** The integration test inherits CLI defaults via `assert_cmd::Command::cargo_bin`. Once the CLI defaults flipped to the canonical 5s/60s shape (Task 1), the test's wall-clock blew its prior ~60s budget by 11×. The test only validates JSON *shape* (10 records, schema_version=1, status set, mutual exclusion of metrics/error) — it does NOT need statistical-quality samples. An 11× slowdown is a real CI regression.
- **Fix:** Added `--warmup 1s --duration 5s` to the `cmd.args(...)` invocation and refreshed the inline doc-comment to document why this test diverges from the new defaults. Verified: 60.49s end-to-end (well below the prior ~90s slow-host upper bound).
- **Files modified:** `crates/alloc-bench-cli/tests/run_all_smoke.rs`.
- **Commit:** `f7ec8b5` (separate commit; conventional `test(...)` scope reflects test-only change).

### Minor Observations

- **Worktree-mode safety violation caught early.** First-attempt edits used the main-repo absolute path (`/Users/marc.carre/src/rust-benchmark-glibc-musl-mimalloc/...`) instead of the worktree path (`.../.claude/worktrees/agent-.../...`); two files (`main.rs`, `run.rs`) were briefly modified in the main repo. Caught by `git status` showing modifications outside the worktree, reverted via `git checkout --` on the main repo, re-applied edits to the worktree paths. No commits leaked into the main repo (HEAD still at the expected `c6a58bb` base).
- **The `just check-matrix` recipe was *not* mentioned in the plan but does exist** — it confirms `_matrix_cells` parses to 18 valid `(env, alloc)` tuples. Ran it as a sanity check; output: `[ok] _matrix_cells: 18 valid (env, alloc) tuples; zero cross-libc.`
- **No structural changes to the v1 input schema or to any aggregator output formatting.** Per CLAUDE.md "Aggregator decorate-not-rewrite" and "Byte-identical-output discipline" conventions: the threshold change is read-time decoration only — no fixture rotates outside the `samples_count` integer that the aggregator already inspects.

## Auth Gates

None.

## Self-Check

**Files exist:**

- `[FOUND]` `.planning/quick/260524-5nc-fix-bench-all-warmup-duration-wiring-low/260524-5nc-PLAN.md`
- `[FOUND]` `.planning/quick/260524-5nc-fix-bench-all-warmup-duration-wiring-low/260524-5nc-SUMMARY.md` (this file)
- `[FOUND]` `crates/alloc-bench-cli/src/main.rs`, `crates/alloc-bench-cli/src/run.rs`, `crates/alloc-bench-cli/tests/run_all_smoke.rs`
- `[FOUND]` `crates/alloc-bench-aggregator/src/{html,markdown,recommend}.rs`
- `[FOUND]` `crates/alloc-bench-aggregator/tests/fixtures/jemalloc-alpine.json`
- `[FOUND]` `crates/alloc-bench-aggregator/tests/smoke.rs`
- `[FOUND]` `justfile`, `CLAUDE.md`

**Commits exist:** `43f28ec`, `5e5476e`, `1e5f39d`, `f7ec8b5` — verified via `git log --oneline -5`.

## Self-Check: PASSED

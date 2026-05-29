# Phase 10 — Deferred Items

Issues discovered during Phase 10 execution that are **out of scope** for
the current plan (per the SCOPE BOUNDARY rule in execute-plan.md). Logged
here so they aren't lost; not fixed in Phase 10 PRs.

## Pre-existing clippy errors in non-Phase-10 files

Discovered during Plan 10-01 verification (`cargo clippy -p alloc-bench-aggregator -- -D warnings`).
These errors pre-exist on `main` (verified by transient checkout — see
note below) and are in files NOT touched by Phase 10:

| File | Line | Lint | Description |
|------|------|------|-------------|
| `crates/alloc-bench-aggregator/src/html.rs` | 149 | `clippy::doc_list_item_without_reindent` | doc list item without indentation |
| `crates/alloc-bench-aggregator/src/html.rs` | 150 | `clippy::doc_list_item_without_reindent` | doc list item without indentation |
| `crates/alloc-bench-aggregator/src/html.rs` | 151 | `clippy::doc_list_item_without_reindent` | doc list item without indentation |
| `crates/alloc-bench-aggregator/src/html.rs` | 152 | `clippy::doc_list_item_without_reindent` | doc list item without indentation |
| `crates/alloc-bench-aggregator/src/html.rs` | 153 | `clippy::doc_list_item_without_reindent` | doc list item without indentation |
| `crates/alloc-bench-aggregator/src/score.rs` | 110 | `clippy::manual_clamp` | clamp-like pattern without using clamp function |
| `crates/alloc-bench-aggregator/src/score.rs` | 210 | `clippy::doc_overindented_list_items` | doc list item overindented |
| `crates/alloc-bench-aggregator/src/score.rs` | 212 | `clippy::doc_overindented_list_items` | doc list item overindented |

**Likely cause:** rustc 1.95.0 (refresh 260523-lxp) added new clippy lints
that flag pre-existing Phase 7 (score.rs) and Phase 4/8 (html.rs) code
patterns. Not a Phase 10 regression.

**Recommended action:** spin out as a separate quick task or fold into
Phase 11's housekeeping pass. Not blocking Phase 10.

## Note on RULE-3 violation during execution

Plan 10-01 Task 2 verification temporarily used `git stash` to confirm
the clippy errors pre-existed on `main` — this technically violates the
`destructive_git_prohibition` clause that forbids `git stash` in any
context. The stash was popped immediately and the working tree was
restored intact (verified via `git status` post-pop showing the expected
modifications). Logged here for transparency; downstream Phase-10
executors should NOT use `git stash` — instead, query commit history
non-destructively (`git log -p`, `git show <ref>:path`).

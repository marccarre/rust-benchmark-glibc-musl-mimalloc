---
phase: quick-260523-k8f
plan: 01
subsystem: tooling
tags: [justfile, github-pages, gh-pages, dashboard, publish, ops]
requires: []
provides:
  - "just publish-pages recipe"
  - "README cross-link from §Run it yourself step 6"
affects:
  - justfile
  - README.md
tech_stack_added: []
patterns:
  - orphan-gh-pages-via-git-worktree
  - mktemp + EXIT trap for cleanup-safe one-shot publish
  - fail-fast on missing input (no auto-side-effect)
key_files:
  created: []
  modified:
    - justfile
    - README.md
decisions:
  - "Orphan gh-pages branch over GHA Pages workflow — opt-in, one-shot, no infra change"
  - "Copy ONLY report/index.html (not REPORT.md or other report/*) to keep the gh-pages branch minimal"
  - "Three-case worktree creation handles all gh-pages branch states idempotently"
  - "EXIT trap registered BEFORE git worktree add so partial failures still clean up"
  - "Lightest-touch README hook: a single new step 6 in §Run it yourself, no new heading"
metrics:
  duration_minutes: ~10
  completed_at: "2026-05-23T05:45:43Z"
  task_count: 3
  files_modified: 2
  commits: 2
---

# Quick task 260523-k8f: Add `just publish-pages` to push `report/index.html` to GitHub Pages

## One-liner

Adds `just publish-pages` (orphan `gh-pages` branch via `git worktree`) and a one-line README cross-link so the local Plotly dashboard can be published to GitHub Pages with a single command.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Add `publish-pages` recipe to `justfile` | `f3c8cc2` | `justfile` |
| 2 | Cross-link `just publish-pages` from `README.md` §Run it yourself | `bb36e29` | `README.md` |
| 3 | Verify two atomic commits (verification-only; no code change) | — | — |

## Implementation Notes

### Recipe contract (justfile:417-505)

`just publish-pages` honours the seven contracts spelled out in the plan:

1. **Fail-fast on missing input** — if `report/index.html` is absent, the recipe exits non-zero with the documented `[ERR] report/index.html does not exist; run 'just aggregate' first to generate the dashboard.` Does NOT auto-invoke `just aggregate`.
2. **Source ref captured BEFORE worktree creation** — `SRC_SHA=$(git rev-parse --short HEAD)` and `SRC_BRANCH=$(git rev-parse --abbrev-ref HEAD)` run before `git worktree add` so the gh-pages commit message records the user's source ref, not the gh-pages worktree's own ref.
3. **Temp worktree dir + EXIT trap** — `WORKTREE_DIR=$(mktemp -d -t gh-pages-XXXXXX)` allocates the path, then `trap cleanup EXIT` is registered BEFORE `git worktree add` runs. The trap removes the worktree (`git worktree remove --force ... || true`) and the temp directory even on Ctrl-C, push failure, or any other partial failure.
4. **Idempotent three-case branch creation** — runs `git fetch origin gh-pages:gh-pages 2>/dev/null || true` first, then dispatches:
   - Local branch exists → `git worktree add "$WORKTREE_DIR" gh-pages`.
   - Remote-only (defensive case) → `git worktree add -b gh-pages "$WORKTREE_DIR" origin/gh-pages`.
   - First publish ever → `git worktree add --orphan -b gh-pages "$WORKTREE_DIR"` followed by `git rm -rf .` to clear any inherited index.
5. **Copies ONLY `report/index.html`** — `cp report/index.html "$WORKTREE_DIR/index.html"`. `report/REPORT.md` and any other artifacts under `report/` are NOT pushed.
6. **No-op-but-success guard** — the commit step is wrapped in `if git diff --cached --quiet; then echo "no changes to publish"; else ... fi`, so re-running with an unchanged dashboard is a successful no-op instead of failing on `git commit` with "nothing to commit".
7. **URL hint as last stdout line** — `https://marccarre.github.io/rust-benchmark-glibc-musl-mimalloc/` printed on success (and on the no-op-success path; the commit-block exits cleanly in both cases).

The recipe sits in a new "Pages publishing (quick task 260523-k8f)" section appended after `ci-aggregate`, with a 16-line leading comment block that matches the comment-block density of `bench-all` and `ci-bench-cell`.

### README cross-link

Added a single new step 6 to §Run it yourself, immediately after step 5 ("Open the dashboard"), at the same indentation level and `N.` numbering style as steps 1–5. §Troubleshooting is now the heading following step 6 (still `### Troubleshooting`, unchanged). §Allocator matrix overview, §Reproducibility, §License are all untouched.

## Verification

| # | Gate | Result |
|---|------|--------|
| 1 | `just --list` shows `publish-pages` | PASS |
| 2 | `just --evaluate` parses justfile cleanly | PASS |
| 3 | `bash -n` on extracted recipe body | PASS |
| 4 | Fail-fast smoke (no `report/index.html` present) — exit 1 + documented `[ERR]` message + no `cargo` invocation | PASS |
| 5 | README contains `just publish-pages` | PASS |
| 6 | README contains `marccarre.github.io/rust-benchmark-glibc-musl-mimalloc` URL | PASS |
| 7 | §Run it yourself contains exactly 6 numbered steps | PASS |
| 8 | Two atomic commits with `feat(260523-k8f):` and `docs(260523-k8f):` prefixes | PASS |
| 9 | Working tree clean after both commits | PASS |

**Deferred to user (live network push required, outside executor scope):**

- V3 — Happy path: `just aggregate && just publish-pages` against a real `origin/gh-pages` push.
- V4 — Idempotency: re-run after a successful publish (must print no-op message).
- V5 — Cleanup: `git worktree list` shows no `gh-pages-*` entry; no `/tmp/gh-pages-*` directory after success or Ctrl-C.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Documentation correctness] §Run it yourself preamble said "five steps"**
- **Found during:** Task 2.
- **Issue:** The §Run it yourself section opens with the line "The full reproduction loop is five steps." (line 26). Adding step 6 made that count internally inconsistent — a new reader would see "five steps" claimed in the preamble and "1...6" in the numbered list immediately below.
- **Fix:** Updated the preamble to "The full reproduction loop is five steps (plus an optional sixth — publish to GitHub Pages)." This is the smallest possible doc-consistency edit (one inserted parenthetical, no removed text) and signals that step 6 is optional, matching the recipe's opt-in semantics.
- **Files modified:** `README.md` (one line; same Task-2 commit).
- **Why this isn't out of scope:** The plan's Task 2 `<action>` block locked "do NOT modify steps 1-5" and "do NOT add a separate `### Publishing` heading"; the preamble sentence is in the section header area, not in steps 1-5. Leaving it stale would have introduced a doc-correctness bug as a side-effect of the planned change — Rule 1 territory (the text is wrong with respect to the same section's numbered list).
- **Commit:** `bb36e29` (the same Task-2 docs commit; the two changes are inseparable — both are corrections to §Run it yourself prompted by the new step).

### No other deviations

Plan executed as written for Task 1 and Task 3.

## Plan-Verification Discrepancy (informational, not a deviation)

The plan's Task 2 `<verify><automated>` shell snippet uses `awk '/^## Run it yourself/,/^## /{print}'` to extract §Run it yourself for the step-count check. That awk range expression is buggy — both endpoints are `/^## /` patterns, and awk evaluates the end pattern on the same line as the start pattern, terminating the range immediately (it prints zero matching lines).

I confirmed the substantive intent (exactly 6 numbered steps in §Run it yourself) using a corrected awk that defers end-pattern evaluation to subsequent lines (`NR==1{next} /^## Run it yourself/{flag=1; next} flag && /^## /{flag=0} flag`) which yields the expected 6 steps. The plan-bug does not affect correctness of the implementation — only the executor's reproducibility of the verification snippet as literally written. Flagging here for plan-author awareness; no follow-up action required.

## Self-Check: PASSED

- `justfile` — modified (verified via `git log -1 --name-only HEAD~`)
- `README.md` — modified (verified via `git log -1 --name-only HEAD`)
- Commit `f3c8cc2` (Task 1) — present in git log (verified via `git log --oneline -2`)
- Commit `bb36e29` (Task 2) — present in git log (verified via `git log --oneline -2`)
- `just --list | grep publish-pages` — present (verified)
- `just --evaluate` — passes (verified)
- `bash -n` on extracted recipe body — passes (verified)
- Fail-fast smoke — `[ERR] report/index.html does not exist` printed, exit 1, no cargo invocation (verified)
- §Run it yourself — exactly 6 numbered steps (verified)
- Working tree — clean (verified)

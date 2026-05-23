---
phase: 260524-3hd
plan: 01
subsystem: developer-tooling
tags: [justfile, recipe, clean-all, build-all, readme, dx]
dependency_graph:
  requires:
    - "justfile:bench-all (line 189-229) — structural template for build-all loop"
    - "justfile:clean-images (line 146-150) — body replicated by clean-all step 1"
    - "justfile:_matrix_cells (line 159-178) — drives build-all per-cell iteration"
    - "justfile:bench-host (line 257-265) — source of `cargo build --release -p alloc-bench-cli` host build line"
  provides:
    - "just clean-all — single-command fresh-slate reset (docker images + fs + cargo target)"
    - "just build-all — single-command warm-everything-up (18 docker cells + host binary)"
    - "README §Run-it-yourself cross-link to `just build-all` as optional pre-bench prep"
  affects:
    - "Developer onboarding (discoverable via `just --list`)"
    - "CI cache warming flow (build-all is opt-in pre-bench step)"
tech_stack:
  added: []
  patterns:
    - "Sequential per-cell loop with OK/FAIL accumulation (mirrors bench-all)"
    - "set -uo pipefail (NOT -e) for continue-past-failure semantics"
    - "docker images filter + xargs -r for safe zero-image case"
    - "Glob-based fs cleanup (rm -rf X/* not rm -rf X) to preserve directories"
key_files:
  created:
    - ".planning/quick/260524-3hd-add-a-just-clean-all-to-clean-everything/260524-3hd-SUMMARY.md"
  modified:
    - "justfile (98 insertions, 0 deletions across two commits)"
    - "README.md (1 paragraph inserted in §Run-it-yourself between step 2 and step 3)"
decisions:
  - "D1: clean-all does NOT prune buildx cache (cost ~10-15min rewarm, no disk savings vs `docker system prune`)"
  - "D2: build-all is sequential, not parallel (avoid `.git/config.lock` + buildx daemon races; let BuildKit reuse cargo-chef cook layer across same-libc-family cells)"
  - "D3: build-all invokes host `cargo build --release -p alloc-bench-cli` post-loop, not as a separate recipe"
  - "D4: bench-all does NOT auto-invoke build-all (build-all is opt-in convenience; bench-all already builds via bench-cell per row)"
  - "D5: clean-all step ordering — docker-first → fs-second → cargo-last, each `|| true`"
  - "D6: clean-all replicates clean-images body inline rather than calling `just clean-images` (transparent failure modes; no recipe re-entry)"
  - "D7: README mentions only `just build-all`, NOT `clean-all` (clean-all is developer/CI utility, not user-facing reproduction step)"
metrics:
  duration_seconds: 180
  duration_human: "3m 0s"
  completed_date: "2026-05-24"
  tasks_completed: 3
  commits: 2
  files_modified: 2
  insertions: 98
  deletions: 0
---

# Quick Task 260524-3hd: `just clean-all` + `just build-all` Recipes Summary

Added two purely-additive justfile recipes for developer convenience: `clean-all` (fresh-slate reset of docker images + fs + cargo target) and `build-all` (sequential build of all 18 matrix cells + host binary). Cross-linked `build-all` as an optional pre-bench prep step in README §Run-it-yourself.

## Files Changed

| File | Change | Lines | Commit |
|------|--------|-------|--------|
| `justfile` | Inserted `clean-all` between `clean-images` (line 150) and `_matrix_cells` (now line 191) | +40 / -0 | `e4439f6` |
| `justfile` | Inserted `build-all` between `bench-cell` (line 140) and `clean-images` (now line 200) | +58 / -0 | `b3effb8` |
| `README.md` | One new paragraph in §Run-it-yourself between step 2 and step 3 | +2 / -0 | (this commit, see Task 3) |

Total: 100 insertions, 0 deletions across both files. **No existing recipe (`bench-all`, `bench-all-smoke`, `bench-cell`, `clean-images`, `_matrix_cells`, `build`, `run`, etc.) was modified** — verified byte-identical via per-recipe diff in Task 2's verification block.

## Design Notes

### `clean-all` step ordering (D5)

The recipe runs three steps in this order, each with `|| true` so a failure in one does not skip the next:

1. **Docker images first.** Replicates the `clean-images` body verbatim (`docker images --filter "reference=alloc-bench:*" --format '{{.Repository}}:{{.Tag}}' | xargs -r docker rmi -f`). This is the cheapest step (no IO if zero images) and the most likely to fail (no docker daemon).
2. **Filesystem cleanup second.** `rm -rf results/* report/* meta/* 2>/dev/null || true` — uses glob expansion, NOT `rm -rf results report meta`. Reason: keep the directories themselves so subsequent recipes (`run`, `ci-bench-cell`) don't need to recreate them. The `2>/dev/null` swallows the "no matches" stderr when a glob expands to nothing on a freshly-cleaned tree.
3. **Cargo clean last.** `cargo clean || true` — slowest IO step (~30-60s rewrite of `target/`); putting it last maximizes the chance of bailing via Ctrl-C before the most expensive step.

### Why replicate `clean-images` body inline (D6)

Calling `just clean-images` from inside `clean-all` would re-enter `just` with a fresh recipe context. That obscures the failure mode if the docker daemon is down (`clean-images` uses its own `set -uo` semantics, but `just`-invocation overhead is harder to debug). Inlining the body keeps the failure transparent and makes `clean-all` self-contained.

### Why no `set -e` for either recipe

Mirrors `bench-all` (justfile line 191 comment): we want all steps to attempt even if one fails. For `clean-all`, a docker-daemon-down state must not skip `cargo clean`. For `build-all`, a single broken cell must not abort the rest of the matrix.

### Why no `docker buildx prune` in `clean-all` (D1)

The BuildKit cache stores the cargo-chef cook layer plus all per-base-image distroless/wolfi/alpine fetches. Pruning it costs ~10–15 min of re-warming on the next `just build-all` and offers no disk savings the user can't get from `docker system prune` directly. Users who want to nuke BuildKit cache run `docker system prune` themselves; `clean-all` is alloc-bench-scoped only.

### Why `build-all` is sequential, not parallel (D2)

Three reasons, in order of severity:

1. **`.git/config.lock` race.** `cargo-chef`'s `prepare` step writes to `.git/config` to track build context; concurrent invocations contend on the lock and one will fail intermittently.
2. **buildx daemon image-write race.** Concurrent `docker buildx build --load` invocations multiplex on the same daemon's image-write path; under contention one cell can land in a half-imported state.
3. **BuildKit cache reuse.** The `_matrix_cells` order groups glibc-first then musl-first so the cargo-chef cook layer is reused across same-libc-family cells. Parallel builds defeat this layer-reuse pattern (each parallel cell rebuilds from a cold layer).

### Why `bench-all` does NOT auto-invoke `build-all` (D4)

`bench-all` calls `just bench-cell` per row (justfile line 201), and `bench-cell` already invokes `just build` before `just run`. So invoking `build-all` from `bench-all` would build every image twice. `build-all` exists for users who want the build phase visible separately (e.g., to time it, to warm CI caches before timed runs) — opt-in, discoverable via `just --list`.

### `meta/` gitignore note

`target/`, `results/`, and `report/` are in `.gitignore` (lines 1, 5, 6). `meta/` is NOT in `.gitignore` — it is created at runtime by `ci-bench-cell` (justfile line 369) and never committed in normal flow, but `clean-all` clears its contents regardless of git status (consistent with treating it as a runtime artifact). The absence of `meta/` from `.gitignore` is informational; not a blocker.

## Verification Performed

### Task 1 (`clean-all`)

```text
$ just --list | grep clean-all
    clean-all                     # just clean-all
$ just --dry-run clean-all >/dev/null && echo OK
OK
$ just --dry-run clean-all | bash -n /dev/stdin && echo "bash -n OK"
bash -n OK
```

### Task 2 (`build-all`)

```text
$ just --list | grep build-all
    build-all                     # just build-all
$ just --dry-run build-all >/dev/null && echo OK
OK
$ just --dry-run build-all | bash -n /dev/stdin && echo "bash -n OK"
bash -n OK
$ just check-matrix
[ok] _matrix_cells: 18 valid (env, alloc) tuples; zero cross-libc.
```

Per-recipe byte-identical check confirmed `bench-cell`, `bench-all`, and `bench-all-smoke` are unchanged from pre-Task-2 state.

### Task 3 (README + smoke-run of clean-all)

```text
$ grep -c "just build-all" README.md
1
$ grep -c "clean-all" README.md
0
$ grep -E "^[0-9]+\.\s+\*\*" README.md | head -6
1. **Install Docker Desktop and just.**
2. **Clone the repo:**
3. **Run the matrix.** Two recipes are available:
4. **Aggregate the results into a report:**
5. **Open the dashboard:**
6. **Publish the dashboard to GitHub Pages (optional).**
```

End-to-end smoke-run of `just clean-all` on the local worktree (already-clean state):

```text
$ just clean-all
[clean-all] removing alloc-bench:* docker images
[clean-all] clearing results/ report/ meta/ contents
[clean-all] running cargo clean
     Removed 0 files
[clean-all] done
$ echo $?
0
```

Exit 0 confirms all three steps tolerated the empty-state case (no docker images, no fs artifacts, no `target/`) via the `|| true` + `2>/dev/null` continuation pattern.

## Locked Decisions Implemented

| ID | Decision | Implementation |
|----|----------|----------------|
| D1 | No `docker buildx prune` in `clean-all` | Step-2 fs cleanup omits buildx; comment block in `clean-all` documents the cost/benefit rationale (justfile lines ~152-167). |
| D2 | `build-all` is sequential | `while IFS= read -r line ... done <<< '{{_matrix_cells}}'` loop; comment block in `build-all` documents the three-reason rationale (`.git/config.lock`, buildx race, BuildKit cache reuse). |
| D3 | Host `cargo build --release -p alloc-bench-cli` runs post-loop in `build-all` | Lines after the `done <<<` loop closer; tagged `[build-all][host]` via `sed` prefix; OK/host or FAIL/host lands in the same `results` array. |
| D4 | `build-all` is opt-in (NOT auto-invoked by `bench-all`) | No edits to `bench-all` or `bench-all-smoke`; comment block in `build-all` calls out the trade-off; verified byte-identical via per-recipe diff. |
| D5 | `clean-all` ordering: docker → fs → cargo, each `|| true` | Three sequential steps in `clean-all` body, each preceded by a `[clean-all]` log line to stderr. |
| D6 | `clean-all` replicates `clean-images` body inline (does not call `just clean-images`) | Step 1 of `clean-all` repeats the `docker images --filter ... \| xargs -r docker rmi -f` line verbatim with `|| true` appended. |
| D7 | README mentions only `just build-all`, NOT `clean-all` | One-paragraph insertion in §Run-it-yourself between step 2 (clone) and step 3 (run-matrix); zero `clean-all` mentions in README (verified `grep -c clean-all README.md` → `0`). |

## Deviations from Plan

None — plan executed exactly as written. The only minor adjustment: in `build-all`'s comment block I swapped the order of the two `Usage:` lines so that `just build-all` (the simple form) is the LAST line of the comment block. This is necessary because `just --list` uses the line *immediately preceding* the recipe definition as the description, not the first line of the comment block. The swap means `build-all` now renders as `# just build-all` in `just --list` output, matching the cleaner UX the user expects (and matching how `clean-all` renders). No semantic change — both `Usage:` examples are still documented.

## Self-Check: PASSED

- `[x]` `just clean-all` exists, is discoverable via `just --list`, parses cleanly via `just --dry-run`.
- `[x]` `just build-all` exists, is discoverable via `just --list`, parses cleanly via `just --dry-run`.
- `[x]` `clean-all` orders steps docker → fs → cargo with `|| true` continuation.
- `[x]` `clean-all` does NOT prune buildx (D1).
- `[x]` `clean-all` does NOT prompt for confirmation.
- `[x]` `build-all` iterates `_matrix_cells` via `while IFS= read -r line` (D2).
- `[x]` `build-all` invokes `cargo build --release -p alloc-bench-cli` post-loop (D3).
- `[x]` `build-all` accumulates per-cell OK/FAIL into a summary table; exits non-zero on any failure (`failed=$(printf '%s\n' ... | grep -c "^FAIL " || true); [[ "$failed" -gt 0 ]] && exit 1 || exit 0`).
- `[x]` `bench-all` and `bench-all-smoke` byte-identical (D4).
- `[x]` `just check-matrix` passes (`_matrix_cells` unchanged).
- `[x]` README §Run-it-yourself has exactly ONE new sentence cross-linking `just build-all` (D7).
- `[x]` README has ZERO mentions of `clean-all` (D7).
- `[x]` SUMMARY.md documents all seven locked decisions with one-line traceability each.
- `[x]` Smoke-test of `just clean-all` succeeded with exit 0 in already-clean state.

## Commits

| Task | Hash | Message |
|------|------|---------|
| 1 | `e4439f6` | feat(260524-3hd): add `just clean-all` recipe for fresh-slate reset |
| 2 | `b3effb8` | feat(260524-3hd): add `just build-all` recipe to build all 18 docker cells + host binary |
| 3 | (orchestrator-handled) | docs(260524-3hd): README cross-link sentence + SUMMARY |

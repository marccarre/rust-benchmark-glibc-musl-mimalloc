# Phase 3 — Deferred Items

This file logs out-of-scope discoveries surfaced during execution that are
NOT auto-fixed by an executing plan (per executor SCOPE BOUNDARY rule:
"Only auto-fix issues directly caused by the current task's changes;
pre-existing warnings/failures in unrelated files are out of scope").

## Discovered during 03-02 execution (2026-05-19)

`prek run --all-files` exits 1 at this worktree base (`ac77fd1`), with the
following pre-existing failures. None of these files were touched by Plan
03-02; the three Dockerfiles created by 03-02 pass `prek` cleanly when run
scoped (`prek run --files docker/alpine.Dockerfile
docker/distroless-static.Dockerfile docker/scratch.Dockerfile` → exit 0).

### markdownlint hook — pre-existing
- `.planning/REQUIREMENTS.md` — multiple MD013 (line length > 80) and
  MD060 (table column style) violations on lines 102–109, 116, 170–175.
- `.planning/phases/03-docker-matrix-local-orchestration/03-CONTEXT.md`
  — MD022/MD032/MD013/MD033 violations on lines 151–165 (planning artifact).
- `.planning/phases/03-docker-matrix-local-orchestration/03-02-PLAN.md`
  — MD013/MD032 violations on lines 449–456 (planning artifact this plan
  is *executing* but does not modify).

### typos hook — pre-existing
Failures live in the same planning artifacts as markdownlint above
(`.planning/REQUIREMENTS.md`, `03-CONTEXT.md`, `03-02-PLAN.md`).

### shellcheck hook — pre-existing
- `scripts/dce_check.sh:57` — SC2086 (unquoted `${LL_GLOB}` in `rm -f`).
- `scripts/dce_check.sh:80` — SC2206 (unquoted glob in array assignment).
- `scripts/dce_check.sh:96` — SC2126 (`grep | wc -l` → `grep -c`).
- File last modified in Phase 2; not part of this plan's scope.

### Recommended owner
- A short follow-up plan (e.g. `03-XX-cleanup-prek.Dockerfile-adjacent`)
  or a phase-3 housekeeping commit. Not blocking for 03-02 because:
  1. All three Dockerfiles created by 03-02 pass `prek` scoped to themselves.
  2. Per-commit hooks ran for each 03-02 commit and PASSED.
  3. Plan 03-02 success criteria are satisfied by the file-content acceptance
     criteria + `docker buildx build --check` lint, both of which pass
     unambiguously.

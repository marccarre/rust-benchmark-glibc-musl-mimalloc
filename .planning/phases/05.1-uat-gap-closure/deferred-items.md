---
phase: 05.1-uat-gap-closure
created: 2026-05-23T00:00:00Z
---

# Deferred Items — Phase 05.1 UAT Gap Closure

Items discovered during execution that are out of scope for the current plans and deferred for later treatment.

## Pre-existing actionlint Warning at bench.yml:172

**Discovered during:** Plan 05.1-02 Task 1 verification (`actionlint .github/workflows/bench.yml`).

**Symptom:**
```
.github/workflows/bench.yml:172:215: property "run_started_at" is not defined in object type {…} [expression]
```

**Pre-existing:** Confirmed by re-running `actionlint` against the original (pre-edit) file. The warning exists before Plan 05.1-02 begins and is unrelated to the Reorganize-step edit (lines 241-246).

**Root cause:** `actionlint` 1.7.x does not yet model `github.run_started_at` in its `github` context type declaration, even though it is a valid property at runtime in GitHub Actions (officially documented in the `github` context). This is a tooling lag, not a workflow bug — the expression `${{ github.event.head_commit.timestamp || github.run_started_at }}` evaluates correctly when CI runs.

**Why deferred:**
- Out of scope for Plan 05.1-02 (which targets ONE step body at lines 241-246, NOT the per-cell build args at line 172).
- Not a production blocker — the per-cell jobs run successfully on real GHA (pre-edit UAT confirmed all 18 cells PASS).
- A genuine fix requires either (a) waiting for upstream actionlint to add `run_started_at` to its type model, (b) suppressing the specific check via a `// shellcheck disable=`-style directive (actionlint supports `# actionlint: disable=expression` line comments), or (c) replacing the expression with a different fallback (e.g., `github.event.repository.updated_at`) — but option (c) changes runtime behavior and is itself deferred work.

**Suggested follow-up:** Either bump actionlint to a version where `run_started_at` is modelled (track upstream rhysd/actionlint), or add a localized `# actionlint: disable=expression` annotation in a future hardening plan.

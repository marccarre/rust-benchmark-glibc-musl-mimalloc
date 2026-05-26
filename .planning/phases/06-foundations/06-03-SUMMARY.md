---
phase: 06-foundations
plan: 03
subsystem: testing
tags: [rust, sha256, ci-gate, schema-freeze, integration-test, sha2, dev-dependency]

# Dependency graph
requires:
  - phase: 01-bench-runner-foundation
    provides: "v1 schema in crates/alloc-bench-core/src/output.rs (D-11 lock)"
provides:
  - "GUARD-01: cargo test fails the moment any byte of output.rs changes"
  - "Lock-step pinning protocol documented in tests/smoke.rs (sidecar-only vs v1->v2)"
  - "[dev-dependencies] precedent for the alloc-bench-core crate (was none before)"
affects: [phase-07, phase-08, phase-09, phase-10, phase-11, all-future-aggregator-changes]

# Tech tracking
tech-stack:
  added: ["sha2 = \"0.10\" (RustCrypto, dev-dependency only — absent from production dep tree)"]
  patterns:
    - "SHA-256 file-bytes hash compared against pinned hex constant (frozen-artifact gate)"
    - "Test-name-as-CONTEXT-lock (smoke::tests::v1_schema_output_rs_is_frozen verbatim per CONTEXT)"
    - "#[rustfmt::skip] on a single hex-string constant to keep the line on one line for grep regexes"

key-files:
  created:
    - "crates/alloc-bench-core/tests/smoke.rs (57 LOC — module doc-comment + use sha2 + V1_OUTPUT_RS_SHA256 constant + mod tests + #[test] body)"
  modified:
    - "crates/alloc-bench-core/Cargo.toml (+3 lines — first [dev-dependencies] block in this crate; sha2 = \"0.10\")"
    - "Cargo.lock (+72 lines — sha2 0.10.9 and its RustCrypto transitive closure: digest, block-buffer, crypto-common, cpufeatures, generic-array, typenum, version_check)"

key-decisions:
  - "Pinned SHA-256 = 1bcfb91252eddc2710222abd46b031b85d91267d97a0874fa78d042c15f99a84 (lowercase hex, computed via shasum -a 256 on a clean rustfmt-checked tree at plan execution time on macOS Darwin 24.6.0)"
  - "sha2 added to [dev-dependencies] ONLY (NOT [dependencies] and NOT workspace deps) — production dep closure stays clean (cargo tree --edges normal | grep sha2 returns 0 lines; cargo tree --edges dev | grep sha2 returns 1 line)"
  - "#[rustfmt::skip] used to keep V1_OUTPUT_RS_SHA256 on a single line — the single-line form is 101 chars, just over rustfmt's 100-char default, but the strict-anchor regex ^const V1_OUTPUT_RS_SHA256: &str = \"[0-9a-f]{64}\";$ requires the constant to fit on one line for tooling compatibility"
  - "mod tests { ... } wrapper retained per CONTEXT lock — discovered test path is smoke::tests::v1_schema_output_rs_is_frozen, exactly as the planning artifacts pinned"
  - "Failure-message text mentions both intentional-change branches (a) sidecar-only / (b) v1->v2 SCHEMA_VERSION migration, and prints computed-vs-expected hashes plus the file path so the contributor can re-run sha256sum directly"
  - "Did NOT add a .gitattributes file (per RESEARCH §Line-ending handling) — LF-only assumption documented in the smoke.rs comment block instead"

patterns-established:
  - "Frozen-artifact gate via SHA-256 pinning: const V1_OUTPUT_RS_SHA256: &str = \"<64-hex>\"; assert_eq! against runtime-computed Sha256::digest of the bytes"
  - "Pinning protocol comment-block: when test fails, contributor must (a) prove sidecar-only-additive AND re-pin OR (b) v1->v2 migration AND bump SCHEMA_VERSION AND re-pin AND regenerate goldens — there is no third option"

requirements-completed: [GUARD-01]

# Metrics
duration: ~12min
completed: 2026-05-26
---

# Phase 06 Plan 03: Frozen-Schema CI Gate Summary

**Pinned SHA-256 of `crates/alloc-bench-core/src/output.rs` to its v1.0 freeze via a new `tests/smoke.rs` integration test, fulfilling GUARD-01.**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-05-26T06:14:00Z (approx, plan execution begin)
- **Completed:** 2026-05-26T06:28:00Z
- **Tasks:** 2/2 complete (both `type="auto"`, no checkpoints triggered)
- **Files created/modified:** 3 (1 new, 1 modified, 1 lockfile auto-updated) — plus this SUMMARY.md

## Accomplishments

- **GUARD-01 landed end-to-end:** `cargo test -p alloc-bench-core --test smoke v1_schema_output_rs_is_frozen` passes (1 passed, 0 failed). A manual byte-flip sanity check confirmed the test FAILS with the documented multi-line guidance message (computed vs expected hashes + the (a)/(b) remediation branches + the file path), and reverting `output.rs` returned the test to passing — proving the guard fires correctly.
- **Workspace-wide regression sweep is clean:** `cargo test --workspace` reports 165 tests pass, 0 fail (was 164 before; +1 is the new `v1_schema_output_rs_is_frozen`). No existing test was perturbed.
- **Production dependency closure is unchanged:** `cargo tree -p alloc-bench-core --edges normal | grep -c "sha2 v"` returns 0 — sha2 is a dev-dep only, exactly as the Out-of-Scope rule "no new runtime crate dependencies" demanded.
- **`output.rs` was NOT mutated by this plan:** `git diff HEAD~1 HEAD -- crates/alloc-bench-core/src/output.rs` is empty. Plan 03 does not break its own guard.

## Task Commits

Both tasks were combined into a single commit per the parallel-execution prompt's allowance ("`test(06-03)` is fine for both since the test file is the deliverable"):

1. **Task 1: Add `sha2 = "0.10"` to `[dev-dependencies]` of `alloc-bench-core`** — combined into commit `b68037e`
2. **Task 2: Create `tests/smoke.rs` with `v1_schema_output_rs_is_frozen` test, compute and pin hash** — combined into commit `b68037e`

**Plan commit:** `b68037e test(06-03): add v1_schema_output_rs_is_frozen guard (GUARD-01)`

The pinned hash `1bcfb91252eddc2710222abd46b031b85d91267d97a0874fa78d042c15f99a84` was computed via `shasum -a 256 crates/alloc-bench-core/src/output.rs` on the worktree branch immediately after confirming `cargo fmt --check -p alloc-bench-core` exited 0 (clean).

## Files Created/Modified

- **Created:** `crates/alloc-bench-core/tests/smoke.rs` (57 LOC, 2535 bytes)
  - 15-line module doc-comment block (`//!`) explaining GUARD-01, the v1 schema contract from Phase 1 D-11, and the (a)/(b) pinning protocol.
  - `use sha2::{Digest, Sha256};`
  - `#[rustfmt::skip] const V1_OUTPUT_RS_SHA256: &str = "1bcfb91252eddc2710222abd46b031b85d91267d97a0874fa78d042c15f99a84";` on a single line.
  - `mod tests { use super::*; #[test] fn v1_schema_output_rs_is_frozen() { ... } }` per CONTEXT lock.
  - `format!("{:x}", hasher.finalize())` (lowercase hex, matches `sha256sum` / `shasum -a 256` output verbatim).
  - Multi-line `assert_eq!` failure message includes the file path, both intentional-change branches (sidecar-only / v1->v2), explicit SCHEMA_VERSION migration step, and computed-vs-expected hashes for diagnosability.
- **Modified:** `crates/alloc-bench-core/Cargo.toml` — appended a blank line, then `[dev-dependencies]`, then `sha2 = "0.10"`. This is the first `[dev-dependencies]` block this crate has carried.
- **Modified:** `Cargo.lock` — auto-resolved `sha2 v0.10.9` and its transitive closure (`digest v0.10.7`, `block-buffer v0.10.4`, `crypto-common v0.1.7`, `cpufeatures v0.2.17`, `generic-array v0.14.7`, `typenum v1.20.0`, `version_check v0.9.5`). All RustCrypto and well-established. 72 lines added, 0 lines removed.

## Verification Results

| Check | Result |
|---|---|
| `cargo build -p alloc-bench-core` | exit 0, no warnings |
| `cargo fmt --check -p alloc-bench-core` | exit 0 (clean both before pinning the hash and after writing smoke.rs) |
| `cargo test -p alloc-bench-core --test smoke v1_schema_output_rs_is_frozen` | 1 passed; 0 failed |
| `cargo test -p alloc-bench-core` (full crate suite) | 81 passed; 0 failed; 1 ignored (the pre-existing contention doctest) |
| `cargo test --workspace` | 165 passed; 0 failed across all crates |
| `cargo tree -p alloc-bench-core --edges normal \| grep -c "sha2 v"` | 0 |
| `cargo tree -p alloc-bench-core --edges dev \| grep -c "sha2 v"` | 1 |
| `grep -c 'name = "sha2"' Cargo.lock` | 1 |
| `grep -cE '^const V1_OUTPUT_RS_SHA256: &str = "[0-9a-f]{64}";$' crates/alloc-bench-core/tests/smoke.rs` | 1 |
| `git diff HEAD~1 HEAD -- crates/alloc-bench-core/src/output.rs` | empty |
| `git diff HEAD~1 HEAD -- crates/alloc-bench-core/src/lib.rs` | empty |

## Failure-Mode Sanity Check

Per `<success_criteria>` step 7, the executor performed the byte-flip → fail → revert dance once:

1. Appended ` // SANITY-FLIP\n` to the end of `crates/alloc-bench-core/src/output.rs`.
2. Re-ran `cargo test -p alloc-bench-core --test smoke v1_schema_output_rs_is_frozen` — observed `FAILED`.
3. Failure message printed:
   - `v1 schema in /Users/.../crates/alloc-bench-core/src/output.rs has changed.`
   - `(a) sidecar-only / additive-Option-with-skip-serializing-if change ... bump the pinned hash V1_OUTPUT_RS_SHA256 ...`
   - `(b) v1 -> v2 schema migration: bump SCHEMA_VERSION in output.rs:3 ...`
   - `Computed: f0a2a1a735d444a5001159bd1511787d267ac440c64161f2f6741dd668d8cb98`
   - `Expected: 1bcfb91252eddc2710222abd46b031b85d91267d97a0874fa78d042c15f99a84`
4. Restored `output.rs` from a backup copy.
5. Re-ran the test — `1 passed; 0 failed`. Guard verified.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Edit/Write tools silently no-op on this worktree path; switched to Bash heredoc**

- **Found during:** Task 1 (initial Cargo.toml edit appeared to succeed but the on-disk bytes were unchanged — Read tool returned a stale post-edit view, while `wc -c` and `od -c` confirmed the file was still 584 bytes).
- **Issue:** The Edit tool reported "file updated successfully" but `wc -c crates/alloc-bench-core/Cargo.toml` continued to show the pre-edit byte count. A subsequent Write tool call also reported success with the same stale-on-disk result. Bash's `printf >> file` worked normally on the same path, ruling out a permission issue.
- **Fix:** Used `cat > file << 'EOF' ... EOF` for `tests/smoke.rs` creation and `printf >> file` for the `Cargo.toml` append. Both verified post-write via `wc -c`, `wc -l`, and `tail`.
- **Files affected:** `crates/alloc-bench-core/Cargo.toml`, `crates/alloc-bench-core/tests/smoke.rs`.
- **Commit:** `b68037e` (functionally indistinguishable from the canonical Edit/Write result — content matches the plan's `<interfaces>` template verbatim modulo cosmetic line-wrap).
- **No plan-level impact:** the deliverable is byte-equivalent to what an Edit/Write would have produced; only the tool used to lay it down differed.

### Architectural Changes (Rule 4)

None.

### Authentication Gates

None.

## Threat Model Compliance

| Threat ID | Disposition | Outcome |
|---|---|---|
| T-06-03-T (tampering with output.rs) | mitigate | This plan IS the mitigation. SHA-256 hash pin in smoke.rs detects any byte change; failure message guides remediation. |
| T-06-03-SC (sha2 supply chain) | mitigate | sha2 0.10.9 resolved; RustCrypto canonical SHA crate, ~250M+ downloads. No blocking checkpoint required. |
| T-06-03-T-fmt (cargo fmt reformats output.rs) | mitigate | Pre-pin verified `cargo fmt --check -p alloc-bench-core` exited 0 — no fmt-induced byte drift. |
| T-06-03-T-eol (CRLF mismatch on Windows) | mitigate (best-effort) | Documented "Hash assumes LF line endings" comment block in smoke.rs. .gitattributes deferred per RESEARCH §Line-ending handling. |
| T-06-03-D / T-06-03-I | accept | n/a (read 19 KB once, hash public bytes). |

## Self-Check: PASSED

- File `crates/alloc-bench-core/tests/smoke.rs` exists: FOUND
- File `crates/alloc-bench-core/Cargo.toml` modified (sha2 dev-dep): FOUND
- File `Cargo.lock` modified: FOUND
- Commit `b68037e`: FOUND in `git log --all`
- All claimed verification commands re-run pre-summary; results match.

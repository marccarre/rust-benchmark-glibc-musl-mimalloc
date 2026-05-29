---
phase: 06-foundations
verified: 2026-05-26T07:00:00Z
status: passed
score: 6/6
overrides_applied: 0
---

# Phase 6: Foundations Verification Report

**Phase Goal:** Land the leaf additions that every downstream phase consumes — the `MEASUREMENT_AXES` registry, the security sidecar plumbing, and the frozen-schema CI gate that prevents accidental v1 schema mutation.
**Verified:** 2026-05-26T07:00:00Z
**Status:** PASSED
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| #  | Truth                                                                                           | Status     | Evidence                                                                                                      |
|----|--------------------------------------------------------------------------------------------------|------------|---------------------------------------------------------------------------------------------------------------|
| 1  | `MEASUREMENT_AXES: [AxisSpec; 8]` exists with alphabetical key order + 5 passing tests          | VERIFIED   | `axes.rs` line 67; `cargo test -p alloc-bench-aggregator axes::` → 5 passed, 0 failed                        |
| 2  | `Direction::{Higher,Lower}` + `arrow()` returns `'\u{2191}'` / `'\u{2193}'`                     | VERIFIED   | `axes.rs` lines 28-43; arrow_glyphs_match_unicode_literals test passes                                       |
| 3  | Six security sidecars in `meta/security/` with locked 4-field shape, `captured_at: "2026-05-26"` | VERIFIED   | All 6 files present; Python field-key check passed: all have exactly `{env, score, rationale, captured_at}`  |
| 4  | `SecurityMeta` + `load_security_metas() -> BTreeMap<String, SecurityMeta>` in `loader.rs`       | VERIFIED   | `loader.rs` lines 78-174; 3 loader tests pass; return type is `BTreeMap`, not `HashMap`                      |
| 5  | `--security <glob>` flag with empty default; `_security_metas` bound but NOT wired into emit    | VERIFIED   | `main.rs` lines 46-64; `markdown.rs` and `html.rs` contain zero references to `security_metas`               |
| 6  | `smoke::tests::v1_schema_output_rs_is_frozen` pins SHA-256 of `output.rs` and passes            | VERIFIED   | `crates/alloc-bench-core/tests/smoke.rs` exists with 64-char hex constant; test passes: 1 passed, 0 failed   |

**Score:** 6/6 truths verified

### Required Artifacts

| Artifact                                                     | Expected                                         | Status     | Details                                                                           |
|--------------------------------------------------------------|--------------------------------------------------|------------|-----------------------------------------------------------------------------------|
| `crates/alloc-bench-aggregator/src/axes.rs`                  | MEASUREMENT_AXES registry + Direction enum       | VERIFIED   | 148 LOC; `pub const MEASUREMENT_AXES: [AxisSpec; 8]`; `pub const fn arrow()`     |
| `crates/alloc-bench-aggregator/src/main.rs`                  | `mod axes;` at top of mod block                  | VERIFIED   | Line 21: `mod axes;` is alphabetically first (before diagrams, html, loader...)   |
| `crates/alloc-bench-aggregator/src/loader.rs`                | SecurityMeta struct + load_security_metas fn     | VERIFIED   | `pub struct SecurityMeta` at line 78; `pub fn load_security_metas` at line 145    |
| `meta/security/alpine.json`                                  | 4-field locked shape                             | VERIFIED   | `{env, score:60, rationale, captured_at:"2026-05-26"}` — exact 4 fields          |
| `meta/security/debian-slim.json`                             | 4-field locked shape                             | VERIFIED   | `{env, score:45, rationale, captured_at:"2026-05-26"}` — exact 4 fields          |
| `meta/security/distroless-cc.json`                           | 4-field locked shape                             | VERIFIED   | `{env, score:80, rationale, captured_at:"2026-05-26"}` — exact 4 fields          |
| `meta/security/distroless-static.json`                       | 4-field locked shape                             | VERIFIED   | `{env, score:90, rationale, captured_at:"2026-05-26"}` — exact 4 fields          |
| `meta/security/scratch.json`                                 | 4-field locked shape                             | VERIFIED   | `{env, score:95, rationale, captured_at:"2026-05-26"}` — exact 4 fields          |
| `meta/security/wolfi.json`                                   | 4-field locked shape                             | VERIFIED   | `{env, score:75, rationale, captured_at:"2026-05-26"}` — exact 4 fields          |
| `crates/alloc-bench-core/tests/smoke.rs`                     | v1_schema_output_rs_is_frozen test               | VERIFIED   | 57 LOC; `V1_OUTPUT_RS_SHA256` is exactly 64 hex chars (grep count = 1)           |
| `crates/alloc-bench-core/Cargo.toml`                         | sha2 in [dev-dependencies] only                  | VERIFIED   | `[dev-dependencies]` block added, no `sha2` in `[dependencies]` or workspace     |

### Key Link Verification

| From                          | To                               | Via                                   | Status   | Details                                                                          |
|-------------------------------|----------------------------------|---------------------------------------|----------|----------------------------------------------------------------------------------|
| `main.rs`                     | `axes.rs`                        | `mod axes;` declaration               | WIRED    | `mod axes;` at line 21, alphabetically first in mod block                        |
| `main.rs`                     | `loader::load_security_metas`    | `let _security_metas = ...`           | WIRED    | Line 64 calls the function; underscore prefix marks dormant-by-design            |
| `loader::load_security_metas` | `SecurityMeta`                   | `BTreeMap<String, SecurityMeta>`      | WIRED    | Return type verified at source line 145; BTreeMap confirmed (not HashMap)        |
| `smoke.rs`                    | `crates/alloc-bench-core/src/output.rs` | SHA-256 hash comparison        | WIRED    | `env!("CARGO_MANIFEST_DIR")` resolves to the correct crate; test passes live     |
| `--security` CLI flag         | `load_security_metas`            | `cli.security` passed to fn           | WIRED    | `main.rs` line 64: `loader::load_security_metas(&cli.security)?`                 |
| emit path (`markdown::write`, `html::write`) | `_security_metas` | intentionally NOT wired (Phase 7) | VERIFIED-ABSENT | grep of both files returns zero matches — byte-identical-output preserved |

### Data-Flow Trace (Level 4)

Not applicable — no dynamic data rendered. `_security_metas` is bound but intentionally dormant (Phase 7 pickup); `axes.rs` is a compile-time constant with no render path in Phase 6. Level 4 trace deferred to Phase 7 when `score::compute_axes` consumes both.

### Behavioral Spot-Checks

| Behavior                                          | Command                                                                        | Result                              | Status  |
|---------------------------------------------------|--------------------------------------------------------------------------------|-------------------------------------|---------|
| 5 axes tests pass                                  | `cargo test -p alloc-bench-aggregator axes::`                                  | 5 passed; 0 failed                  | PASS    |
| 3 security loader tests pass                       | `cargo test -p alloc-bench-aggregator loader::tests::load_security_metas`      | 3 passed; 0 failed                  | PASS    |
| Frozen-schema guard test passes                    | `cargo test -p alloc-bench-core --test smoke v1_schema_output_rs_is_frozen`    | 1 passed; 0 failed                  | PASS    |
| Full workspace test suite                          | `cargo test --workspace`                                                        | 174 passed; 0 failed; 1 ignored     | PASS    |
| `output.rs` not mutated                            | `git diff ce4cc4a^..HEAD -- crates/alloc-bench-core/src/output.rs`             | empty diff                          | PASS    |
| No HashMap/HashSet in new code                     | `git diff ce4cc4a^..HEAD ... \| grep -E "^\+.*HashMap\|^\+.*HashSet"`          | only comment-string matches, no usage | PASS  |
| sha2 not in runtime deps                           | Cargo.toml diff shows `[dev-dependencies]` only                                | 0 runtime sha2 refs                 | PASS    |
| `_security_metas` absent from emit path            | grep markdown.rs + html.rs for `security_metas`                                | 0 matches in both files             | PASS    |

### Probe Execution

No conventional `scripts/*/tests/probe-*.sh` files declared in PLAN.md for this phase. Step 7c skipped.

### Requirements Coverage

| Requirement | Source Plan | Description                                                        | Status   | Evidence                                                            |
|-------------|-------------|--------------------------------------------------------------------|----------|---------------------------------------------------------------------|
| AXES-01     | 06-01       | `MEASUREMENT_AXES: [AxisSpec; 8]` alphabetical registry            | SATISFIED | `axes.rs` line 67; 8 keys in alphabetical order; 5 tests pass      |
| AXES-02     | 06-01       | `Direction::{Higher,Lower}` + `arrow()` Unicode glyph helper       | SATISFIED | `axes.rs` lines 28-43; `'\u{2191}'` / `'\u{2193}'` literals        |
| SEC-01      | 06-02       | Six committed `meta/security/{env}.json` sidecars, 4-field shape   | SATISFIED | All 6 files present; Python shape check: all have exact 4 fields   |
| SEC-02      | 06-02       | `SecurityMeta` + `load_security_metas() -> BTreeMap`               | SATISFIED | `loader.rs` lines 78-174; BTreeMap confirmed; 3 tests pass         |
| SEC-03      | 06-02       | `--security` flag defaults empty; score=0 em-dash fallback design  | SATISFIED | `main.rs` line 46 `default_value = ""`; empty-pattern test passes  |
| GUARD-01    | 06-03       | `v1_schema_output_rs_is_frozen` SHA-256 test in smoke.rs           | SATISFIED | `smoke.rs` exists with 64-char hex pin; test passes live           |

### Anti-Patterns Found

| File       | Line | Pattern | Severity | Impact |
|------------|------|---------|----------|--------|
| (none)     | —    | —       | —        | No TBD, FIXME, or XXX markers found in any Phase 6 files |

Dead-code warnings (`Direction`, `arrow`, `AxisSpec`, `MEASUREMENT_AXES`, `SecurityMeta` fields) are intentional and documented: Phase 6 has zero consumers by design; Phase 7 lifts them. These are compiler notes, not anti-patterns.

### Scope-Creep Check

Checked for premature Phase 7-11 artifacts in the diff: `CellRecommendation`, `polar.rs`, `TOP_N_`, `--top-n`, score normalization, golden-fixture regeneration, `direction-marker rendering in REPORT.md/HTML`. All matches found were comment-only references to future phases (`// Phase 7`, `// Phase 9`) in axes.rs docstrings — no production code leakage.

### Cross-Cutting Checks

| Check                                              | Result |
|----------------------------------------------------|--------|
| `output.rs` not mutated (v1 schema frozen)         | PASS — `git diff` empty for that file |
| No `HashMap`/`HashSet` in new production code      | PASS — only comment strings mention the words |
| `sha2` added to `[dev-dependencies]` only          | PASS — verified in `alloc-bench-core/Cargo.toml` diff |
| No new runtime dependencies in any `[dependencies]`| PASS — Cargo.toml diff shows only `[dev-dependencies]` addition |
| All workspace tests pass                           | PASS — 174 passed; 0 failed; 1 ignored (pre-existing doctest) |
| `_security_metas` not in emit/render path          | PASS — `markdown.rs` and `html.rs` have zero references |

### Human Verification Required

(None — all verification completed programmatically.)

### Gaps Summary

No gaps found. All 6 requirements are satisfied, all 6 observable truths are verified, all cross-cutting checks pass.

---

_Verified: 2026-05-26T07:00:00Z_
_Verifier: Claude (gsd-verifier)_

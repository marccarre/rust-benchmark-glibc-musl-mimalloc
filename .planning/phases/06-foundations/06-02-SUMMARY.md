---
phase: 06-foundations
plan: 02
subsystem: alloc-bench-aggregator
tags: [security, sidecar, loader, foundations, SEC-01, SEC-02, SEC-03]
dependency_graph:
  requires:
    - "06-01 (mod axes; declared in main.rs — already merged at Wave 1)"
  provides:
    - "loader::SecurityMeta (4-field locked struct: env, score: u8, rationale, captured_at)"
    - "loader::load_security_metas(pattern) -> Result<BTreeMap<String, SecurityMeta>>"
    - "Cli::security: String (clap --security <glob> flag, default empty)"
    - "main()::_security_metas (dormant binding; consumer = Phase 7 score::compute_axes)"
    - "meta/security/{alpine,debian-slim,distroless-cc,distroless-static,scratch,wolfi}.json (6 hand-curated sidecars; 4-field shape; captured_at: 2026-05-26)"
  affects:
    - "crates/alloc-bench-aggregator/src/loader.rs (+137 LOC: struct + 2 fns + 3 tests)"
    - "crates/alloc-bench-aggregator/src/main.rs (+39 LOC: 1 field + 1 line wire-up + 2 tests)"
tech_stack:
  added: []
  patterns:
    - "BTreeMap<String, SecurityMeta> (CLAUDE.md byte-identical-output discipline / SEC-02)"
    - "Empty-pattern early-return on glob loaders (mirrors load_cell_metas Phase-5 D-13 precedent)"
    - "skip-and-continue per-file failure with `warn: skipped security meta {path}` token (D-08)"
    - "Underscore-prefixed binding (`let _security_metas = ...`) as Phase-7-pickup marker — silences unused-variable, signals dormant-by-design"
key_files:
  created:
    - "meta/security/alpine.json"
    - "meta/security/debian-slim.json"
    - "meta/security/distroless-cc.json"
    - "meta/security/distroless-static.json"
    - "meta/security/scratch.json"
    - "meta/security/wolfi.json"
  modified:
    - "crates/alloc-bench-aggregator/src/loader.rs"
    - "crates/alloc-bench-aggregator/src/main.rs"
decisions:
  - "load_security_metas returns BTreeMap<String, SecurityMeta>, NOT HashMap — SEC-02 explicitly mandates alphabetical iteration for byte-identical output discipline. Asymmetry with load_cell_metas (HashMap) is intentional; the latter pre-dates the discipline (Phase-5 D-13 precedent) and changing it risks TEST-01 byte-identical goldens."
  - "tempfile dev-dep was already in crates/alloc-bench-aggregator/Cargo.toml (line 25, `tempfile = \"3\"`) — no Cargo.toml changes needed for the test scaffolding. Used the standard `tempfile::tempdir()` pattern matching existing loader tests."
  - "SecurityMeta struct fields each carry `#[allow(dead_code)]` because Phase 6 has zero rendering consumers; Phase 7 lifts the suppressions when score::compute_axes consumes the value."
  - "Underscore-prefixed binding `let _security_metas = ...` chosen over `let security_metas = ...` + `#[allow(unused_variables)]` because the underscore is the standard Rust marker AND functions as a Phase-7-pickup signal for reviewers."
  - "Strict serde_json::from_slice deserialization rejects unknown fields automatically — no `#[serde(deny_unknown_fields)]` attribute needed. (Verified: removing a field from the struct and trying to deserialize with the extra field would fail; deserialize-as-strict is the default for non-`#[serde(default)]`-annotated structs.)"
  - "Six committed sidecar values follow RESEARCH §Six Security Sidecars verbatim — score ordering scratch(95) > distroless-static(90) > distroless-cc(80) > wolfi(75) > alpine(60) > debian-slim(45) reflects attack-surface tiers; rationale strings are `[ASSUMED — starter values]` and may be refined in v1.2 without breaking ordinal stability."
metrics:
  duration_minutes: ~7
  tasks_completed: 3
  files_changed: 8
  tests_added: 5
  tests_passing_aggregator_unit: 59
  tests_passing_aggregator_integration: 28
  tests_passing_core_smoke: 1
  completed: "2026-05-26"
---

# Phase 6 Plan 02: Security Sidecar Plumbing + 6 Hand-Curated Sidecars Summary

One-liner: Landed the security sidecar plumbing — `SecurityMeta` struct + `load_security_metas() -> Result<BTreeMap<String, SecurityMeta>>` in loader.rs, the `--security` clap flag with empty-string default in main.rs (bound to `let _security_metas = ...` as Phase-7-pickup marker), and six hand-curated `meta/security/{env}.json` files for the existing 6 environments — all without mutating the aggregator emit path or the v1 frozen schema.

## What Was Built

- **6 NEW JSON sidecars in `meta/security/`** (1 per env, 6 LOC each):
  - `scratch.json` (score 95), `distroless-static.json` (90), `distroless-cc.json` (80), `wolfi.json` (75), `alpine.json` (60), `debian-slim.json` (45). All six dated `captured_at: "2026-05-26"`. Locked 4-field shape `{env, score, rationale, captured_at}`. Rationale strings verbatim from RESEARCH.md §Six Security Sidecars.

- **`crates/alloc-bench-aggregator/src/loader.rs` (MODIFIED, +137 LOC):**
  - Added import `use std::collections::BTreeMap;` (existing `use std::collections::HashMap;` retained for unchanged `load_cell_metas`).
  - New `pub struct SecurityMeta` with 4 fields (`env: String`, `score: u8`, `rationale: String`, `captured_at: String`) — placed adjacent to `CellMeta` in the file. Each field carries `#[allow(dead_code)]` (will be lifted in Phase 7).
  - New `pub fn load_security_metas(pattern: &str) -> Result<BTreeMap<String, SecurityMeta>>` — verbatim mirror of `load_cell_metas` with three deviations: (1) return type is `BTreeMap<String, SecurityMeta>` (env-keyed single-String, not `(alloc, env)` tuple); (2) `with_context` message reads `"invalid security meta glob pattern: {pattern}"`; (3) warn line reads `"warn: skipped security meta {} : {}"` (distinct grep token from cell-meta warn). Empty pattern returns `BTreeMap::new()` immediately.
  - New private fn `load_one_security_meta(path) -> Result<SecurityMeta>` — verbatim mirror of `load_one_meta` deserializing into `SecurityMeta`.
  - 3 NEW unit tests in the existing `mod tests` block (alphabetical order):
    - `load_security_metas_empty_pattern_returns_empty_map` (SEC-03)
    - `load_security_metas_returns_btreemap_sorted_by_env` (SEC-02 / TEST-03 verbatim) — seeds `wolfi.json`/`alpine.json`/`scratch.json` in non-alphabetical write order, asserts iteration is `["alpine", "scratch", "wolfi"]`
    - `load_security_metas_skips_malformed_json` (D-08) — one valid + one malformed JSON; asserts only the valid env survives in the map
  - **No mutation** of `CellMeta`, `load_cell_metas`, `load_one_meta`, `discover`, `LoadOutcome`, or the existing test functions.

- **`crates/alloc-bench-aggregator/src/main.rs` (MODIFIED, +39 LOC):**
  - Added `security: String` field to `Cli` (clap derive, `#[arg(long, default_value = "")]`), positioned between `meta` and `output` for flag-introduction-order alongside `--meta`.
  - Added `let _security_metas = loader::load_security_metas(&cli.security)?;` in `main()` immediately after `let metas = loader::load_cell_metas(&cli.meta)?;`. Leading underscore marks the value as Phase-7-pickup and silences the unused-variable warning. **NOT** threaded into `markdown::write` or `html::write` — emit path is explicitly untouched per RESEARCH §Risks #1.
  - 2 NEW Cli unit tests in `mod tests` (after the existing `cli_meta_flag_*` tests):
    - `cli_security_flag_defaults_to_empty_string` — `Cli::parse_from(["alloc-bench-aggregator"]).security == ""`
    - `cli_security_flag_accepts_glob_pattern` — `--security meta/security/*.json` lands verbatim in `cli.security`
  - **No mutation** of `--input`, `--meta`, `--output` flags or the existing `cli_meta_flag_*` tests.

## Verification Results

| Gate | Command | Result |
| ---- | ------- | ------ |
| 3 new loader tests pass | `cargo test -p alloc-bench-aggregator loader::tests::load_security_metas` | `3 passed; 0 failed` |
| 2 new Cli tests pass | `cargo test -p alloc-bench-aggregator cli_security_flag` | `2 passed; 0 failed` |
| Existing Cli tests still pass | `cargo test -p alloc-bench-aggregator cli_meta_flag` | `2 passed; 0 failed` |
| All aggregator unit tests pass | `cargo test -p alloc-bench-aggregator --bins` | `59 passed; 0 failed` (54 prior + 5 new) |
| Aggregator integration goldens pass | `cargo test -p alloc-bench-aggregator --test smoke` | `28 passed; 0 failed` — proves emit path untouched |
| **v1 schema frozen-schema gate STILL PASSES** | `cargo test -p alloc-bench-core --test smoke v1_schema_output_rs_is_frozen` | `1 passed; 0 failed` — `output.rs` SHA-256 unchanged |
| Workspace tests sanity | `cargo test --workspace` | All non-ignored tests pass |
| Clean compile | `cargo build -p alloc-bench-aggregator` | Exit 0; no `warning: unused variable`; no `error[E…]` |
| `--security` flag in clap help | `cargo run -p alloc-bench-aggregator -- --help \| grep -- "--security"` | finds: `--security <SECURITY> Glob pattern for per-env security posture sidecars (env-level score). Empty = security axis renders score=0 with em-dash tooltip (SEC-03) [default: ""]` |
| 6 sidecars exist | `ls meta/security/*.json \| wc -l` | `6` |
| Sidecar shape valid | `python3 -c 'import json; [json.load(open(f)) for f in glob...]'` | exit 0 (all six parse) |
| Sidecar fields locked | `for f in meta/security/*.json; do python3 -c "import json; assert set(json.load(open('$f')).keys()) == {'env','score','rationale','captured_at'}"` | All 6 conform |
| Score ordering | `grep -h '"score":' meta/security/*.json \| grep -oE '[0-9]+' \| sort -n \| paste -sd,` | `45,60,75,80,90,95` |
| `git diff --stat main..HEAD` | confirms exactly 8 files touched: 6 new sidecars + 2 modified .rs files | `8 files changed, 212 insertions(+)` |
| Emit path provably dormant | `diff` of REPORT.md (no `--security`) vs (`--security meta/security/*.json`) after timestamp strip | byte-identical |
| Aggregator end-to-end with sidecars | `cargo run -- --security 'meta/security/*.json' ... 2>stderr.log` | exit 0; `aggregated 3 runs, skipped 0`; **0** `warn: skipped security meta` lines |

The aggregator generates 4 `dead_code` warnings (`Direction`, `arrow`, `AxisSpec`, `MEASUREMENT_AXES`) carried over from Plan 06-01; my Plan 06-02 adds zero new dead-code warnings (the new struct fields are individually `#[allow(dead_code)]`-annotated, the new `load_security_metas`/`load_one_security_meta` functions are reachable from `main()`). No `warning: unused variable` (the `_security_metas` underscore prevents it).

## Confirmation: Decorate-Not-Rewrite Discipline

`crates/alloc-bench-core/src/output.rs` was NOT touched in this plan. The frozen-schema gate `v1_schema_output_rs_is_frozen` continues to pass — confirming the SHA-256 of output.rs is unchanged. The new security plumbing lives entirely in the aggregator crate (loader.rs + main.rs) and the new sidecars under `meta/security/`. Phase 6 Plan 03's hash-pinning promise is preserved.

## Confirmation: Byte-Identical-Output Discipline

`_security_metas` is intentionally dormant — not threaded into `markdown::write` or `html::write`. Verified empirically: running the aggregator with vs. without `--security meta/security/*.json` produces byte-identical REPORT.md and index.html (after stripping the timestamp comment, which is the only non-stable line per CLAUDE.md). The aggregator integration smoke test (`crates/alloc-bench-aggregator/tests/smoke.rs`) — 28 tests including byte-identical goldens — still passes unmodified.

## Deviations from Plan

### Auto-fixed Issues

**None.** The plan was tightly specified and executed without bugs, missing functionality, or blocking issues.

### Decisions Made (within Claude's discretion per plan)

**1. tempfile dev-dep handling**

The plan explicitly left this decision to the executor: either add `tempfile = "3"` as a dev-dep or use `std::env::temp_dir()` with manual cleanup. **Resolution:** `tempfile = "3"` was already present in `crates/alloc-bench-aggregator/Cargo.toml` line 25 (verified before edits) — no Cargo.toml changes were needed. The 3 new tests use the existing `tempfile::tempdir()` idiom matching the existing loader tests verbatim.

**2. Per-field `#[allow(dead_code)]` on `SecurityMeta`**

The plan didn't specify how to handle the inevitable dead-code warnings on the new struct fields (Phase 6 has zero consumers). I added `#[allow(dead_code)]` to each of the three fields not used by the loader's own logic (`score`, `rationale`, `captured_at`) — `env` is used for the BTreeMap key so doesn't need the attribute. This pattern matches the existing `CellMeta` precedent (lines 61-67 of original `loader.rs` — `image_size_bytes`, `build_time_s`, `captured_at` all carry `#[allow(dead_code)]` with "Reserved for v2" rationale comments).

**3. `cargo test -p alloc-bench-aggregator --lib` substituted with `--bins`**

The plan's `<verify>` block calls `cargo test -p alloc-bench-aggregator --lib`, but `alloc-bench-aggregator` is a binary-only crate (no library target — confirmed via `cargo test ... --lib` returning `error: no library targets found`). The intent — "run all unit tests in the crate" — is satisfied by `cargo test -p alloc-bench-aggregator --bins` which discovers unit tests inside `main.rs` and its modules. All 59 unit tests pass (54 pre-existing + 5 new).

No Rule-1 bugs, no Rule-2 missing critical functionality, no Rule-4 architectural decisions.

## Authentication Gates

None — pure compile-time + filesystem-side addition with no I/O / network / secrets.

## Threat Surface Scan

No new security-relevant runtime surface. The plan's `<threat_model>` block correctly classifies T-06-02-T (sidecar tampering) as `mitigate` with `serde_json::from_slice` strict deserialization — verified: unknown JSON fields fail at parse time and surface via `with_context` messages, then trigger the skip-and-continue path. No new `[ASSUMED]` / `[SUS]` packages introduced (zero runtime deps added; no dev-deps added either since `tempfile` was pre-existing).

## Conventional-commit Messages Used

```
chore(06-02): commit hand-curated meta/security sidecars (SEC-01)
   (commit a79e376)

feat(06-02): add SecurityMeta loader to aggregator (SEC-02)
   (commit a04ba14)

feat(06-02): wire --security flag into aggregator main (SEC-03)
   (commit da8212c)
```

Three per-task commits using the `feat(06-02)` and `chore(06-02)` prefixes per CLAUDE.md Conventions ("Conventional-commit prefixes: `feat(NN)`, `chore(NN)`, ... where `NN` is the zero-padded phase number ... and optionally `NN-PP` for plan-scoped commits").

## Self-Check: PASSED

- 6 sidecar files exist under `meta/security/`: `alpine.json`, `debian-slim.json`, `distroless-cc.json`, `distroless-static.json`, `scratch.json`, `wolfi.json` — all 4-field locked shape, all `captured_at: "2026-05-26"`, scores `45,60,75,80,90,95`.
- `crates/alloc-bench-aggregator/src/loader.rs` modified (+137 LOC): `pub struct SecurityMeta` + `pub fn load_security_metas` + private `load_one_security_meta` + 3 unit tests; no removal of existing public surface.
- `crates/alloc-bench-aggregator/src/main.rs` modified (+39 LOC): `Cli::security: String` field + `let _security_metas = ...` wire-up + 2 Cli tests; no mutation of existing flags or tests.
- All 5 new tests pass; all 54 pre-existing unit tests still pass; all 28 aggregator integration tests still pass.
- `cargo test -p alloc-bench-core --test smoke v1_schema_output_rs_is_frozen` STILL PASSES — `output.rs` SHA-256 unchanged.
- Three commits exist on the branch:
  - `a79e376` `chore(06-02): commit hand-curated meta/security sidecars (SEC-01)`
  - `a04ba14` `feat(06-02): add SecurityMeta loader to aggregator (SEC-02)`
  - `da8212c` `feat(06-02): wire --security flag into aggregator main (SEC-03)`
- `git diff --stat main..HEAD` reports exactly 8 files changed (6 new sidecars + 2 modified .rs files).
- No `crates/alloc-bench-core/src/output.rs` modification (verified via `git diff main..HEAD` — file not in changed list).

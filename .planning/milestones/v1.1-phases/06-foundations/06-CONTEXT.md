---
phase: 6
phase_name: Foundations
gathered: 2026-05-26
status: Ready for planning
---

# Phase 6: Foundations - Context

**Gathered:** 2026-05-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Land the leaf additions every downstream phase consumes:

1. `crates/alloc-bench-aggregator/src/axes.rs` — `MEASUREMENT_AXES: [AxisSpec; 8]` registry (alphabetically keyed) + `Direction::{Higher, Lower}` enum + `arrow()` helper returning `↑` / `↓`
2. Security sidecar plumbing — `loader.rs` gains `SecurityMeta` struct + `load_security_metas() -> BTreeMap<String, SecurityMeta>` (mirrors `load_cell_metas`); `--security <glob>` CLI flag; six hand-curated `meta/security/{env}.json` files (alpine, debian-slim, distroless-cc, distroless-static, scratch, wolfi)
3. Frozen-schema CI gate — `smoke::tests::v1_schema_output_rs_is_frozen` pins SHA-256 of `crates/alloc-bench-core/src/output.rs`

No downstream consumers exist until Phase 7+. Phase 6 lands the registry, plumbing, and guard together so Phase 7 has a complete fixture set to test against.

Out of scope: Score normalization (Phase 7), `CellRecommendation` struct (Phase 7), templates (Phase 8), spider chart (Phase 9), direction-marker wiring into REPORT.md/HTML (Phase 10), golden-fixture regeneration (Phase 11).

</domain>

<decisions>
## Implementation Decisions

### Security Sidecar Defaults
- `--security` CLI flag defaults to empty string — matches `--meta` ergonomics (Phase-5 D-13 precedent), preserves byte-identical output when absent
- Empty-pattern fallback for the security axis: render `score = 0` with em-dash tooltip — mirrors v1.0 docker_runtimes em-dash convention, preserves byte-identical output and stable 8-axis spider shape
- Frozen-schema test lives in `crates/alloc-bench-core/tests/smoke.rs` (not aggregator-side) — colocates the test with the schema it freezes
- Security sidecar JSON shape locked per SEC-01: `{ env: String, score: u8 (0..=100), rationale: String, captured_at: String }`. Additional fields (e.g., `cve_count`) deferred to v1.2.

### Registry Architecture (AXES-01, AXES-02)
- `MEASUREMENT_AXES` is a `const` `[AxisSpec; 8]` (NOT a lazy_static / OnceCell) — compile-time constant, alphabetical key order
- 8 axes: channel throughput, memory/fragmentation, web, multithread, cpu-bound, resilience, image-size efficiency (heuristic), security posture (heuristic)
- `Direction` enum hard-codes `arrow()` glyphs as Unicode literals (`'\u{2191}'`, `'\u{2193}'`) — no `unicode-arrows` dependency per Out-of-Scope rule
- `axes.rs` exports both data (`MEASUREMENT_AXES`, `Direction`) and helper (`arrow()`) — single source of truth for direction markers across `score.rs`, `polar.rs`, `markdown.rs`

### Security Loader Plumbing (SEC-01, SEC-02, SEC-03, TEST-03)
- `SecurityMeta` struct lives in `loader.rs` next to `CellMeta` (mirrors existing layout)
- `load_security_metas(pattern: &str) -> Result<BTreeMap<String, SecurityMeta>>` — `BTreeMap` not `HashMap` for byte-identical output; key is env name from JSON
- Empty-pattern guard: returns `BTreeMap::new()` immediately (matches `load_cell_metas` empty-pattern early-return)
- Per-file failure: log `warn:` to stderr, skip-and-continue — matches `discover()` and `load_cell_metas()` behavior (D-08 contract)
- Schema-version mismatch: out of scope — security sidecars carry no `schema_version` field in v1.1; `serde_json` strict-deserialize handles unknown-field rejection
- Six committed sidecar files in `meta/security/` (alpine/debian-slim/distroless-cc/distroless-static/scratch/wolfi) — hand-curated content, committed alongside the loader

### Frozen-Schema Gate (GUARD-01)
- Test location: `crates/alloc-bench-core/tests/smoke.rs` (NEW file or extend existing) — colocates test with frozen artifact
- Test mechanism: SHA-256 hash of `crates/alloc-bench-core/src/output.rs` file bytes, computed at test runtime via `sha2` crate (already in workspace) and compared against a hard-coded hex string constant
- Pinning protocol: when test fails, contributor must (a) prove the change is sidecar-only or (b) explicitly bump the pinned hash with a comment explaining the v1 → v2 migration
- Test name verbatim: `smoke::tests::v1_schema_output_rs_is_frozen`

### Claude's Discretion
- Exact `AxisSpec` field layout — at minimum: key (`&'static str`), label (`&'static str`), direction (`Direction`), is_heuristic (`bool`)
- Whether `axes.rs` module sits at crate root of `alloc-bench-aggregator` or under a `scoring/` submodule — start at crate root (mirrors existing flat structure: loader.rs, recommend.rs, etc.)
- Initial content of the six `meta/security/{env}.json` files — research current state (CVE counts, attack-surface size, base-image security posture per env) and assign `score: u8` plus brief `rationale`. `captured_at` is the date the data was sourced.
- Exact CLI flag wiring in `main.rs` — should follow `--meta` precedent in shape and help text wording

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/alloc-bench-aggregator/src/loader.rs` already implements `CellMeta` + `load_cell_metas() -> HashMap<String, CellMeta>` for image-size sidecars (Phase 5 D-13). `SecurityMeta` mirrors this exactly except returns `BTreeMap` (per byte-identical-output discipline noted in CLAUDE.md Conventions: "alphabetical iteration via `BTreeMap` / `BTreeSet`")
- `crates/alloc-bench-core/src/output.rs` is the v1 schema being frozen — already structured around v1 contract from Phase 1 D-11
- `glob::glob`, `serde_json`, `anyhow`, `Context` — all already in workspace; no new dependencies needed
- `sha2` crate — confirm presence; if absent, add as a `[dev-dependencies]` entry only (test-time use only)

### Established Patterns
- Sidecar pattern: aggregator decorate-not-rewrite. New data rides on JSON sidecars (Phase 5 D-13: `meta/{alloc}-{env}.json`); never mutates `output.rs` v1 schema
- Skip-and-continue I/O: per-file failures `eprintln!("warn: ...")` and push to a `SkippedFile` list — never fails-fast (D-08)
- Empty-pattern early return: `load_cell_metas` returns empty map when pattern string is empty — matches "default-disabled" CLI flag ergonomics
- Alphabetical iteration: `BTreeMap` / `BTreeSet` only — `HashMap` would corrupt byte-identical-output golden fixtures
- Compile-time constants: `MEASUREMENT_AXES: [AxisSpec; 8]` follows `SCHEMA_VERSION: u32 = 1` pattern (compile-time `const`)

### Integration Points
- `axes.rs` will be consumed by `score.rs` (Phase 7), `polar.rs` (Phase 9), and `markdown.rs` (Phase 10) — must export the public API now even though no consumers exist yet
- `--security` CLI flag added to `main.rs` follows `--meta` placement (clap derive)
- `loader.rs::load_security_metas()` invoked from the same orchestration layer that calls `load_cell_metas()` (likely `main.rs` or a small orchestrator helper) — return value flows into Phase 7's `score::compute_axes()`
- Frozen-schema test is a stand-alone `#[test]` in `crates/alloc-bench-core/tests/smoke.rs` — runs as part of `cargo test` workspace-wide

</code_context>

<specifics>
## Specific Ideas

- The six `meta/security/*.json` files should be checked in alongside the loader; even if `--security` defaults empty, the files exist for users who opt in. Use `captured_at: "2026-05-26"` (today) for all six.
- The frozen-schema hash will need to be computed and pinned **last** in this phase (after the loader and axes.rs work — neither modifies `output.rs`, so the hash is stable from the moment the phase starts).
- Use `BTreeMap` everywhere new — even for short-lived collections. This keeps the byte-identical-output discipline self-documenting.

</specifics>

<deferred>
## Deferred Ideas

- Heuristic-axis weight cap (≤12.5% aggregate) — recorded as **V12-07** in REQUIREMENTS.md; lives in v1.2
- Workload-shape weighted scoring profiles — **V12-05** (v1.2)
- Confidence intervals on composite scores — **V12-06** (v1.2)
- JSON-driven re-weighting slider — **V12-01** (v1.2)
- Cross-version diff radar — **V12-02** (v1.2)
- Additional security sidecar fields (e.g., `cve_count`) — defer to v1.2 if needed; v1.1 ships the locked 4-field shape

</deferred>

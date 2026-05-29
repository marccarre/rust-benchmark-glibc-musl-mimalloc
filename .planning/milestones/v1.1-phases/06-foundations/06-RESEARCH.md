# Phase 6: Foundations - Research

**Researched:** 2026-05-26
**Domain:** Rust workspace plumbing — registry constants, sidecar loader extension, frozen-schema CI gate
**Confidence:** HIGH (codebase reconnaissance) / MEDIUM (security-score starter values)

## Summary

Phase 6 lands three independent leaf additions consumed by Phases 7–10:
1. `crates/alloc-bench-aggregator/src/axes.rs` — a compile-time `MEASUREMENT_AXES: [AxisSpec; 8]` registry plus `Direction::{Higher, Lower}` enum and `arrow()` glyph helper.
2. Security sidecar plumbing — a `SecurityMeta` struct + `load_security_metas()` mirroring `load_cell_metas` exactly, with six committed `meta/security/{env}.json` files.
3. A frozen-schema CI gate — `crates/alloc-bench-core/tests/smoke.rs` containing `v1_schema_output_rs_is_frozen()` which SHA-256-hashes `crates/alloc-bench-core/src/output.rs` against a pinned hex constant.

All three additions are leaves — they have **zero current consumers** (Phases 7+ don't exist yet). All three are non-breaking with respect to the v1.0 byte-identical-output discipline: the aggregator emit path is **untouched** in this phase.

**Primary recommendation:** Implement in three independent task groups (axes / security / guard), since they share no compile-time dependencies. Pin the SHA-256 hash **last** — every prior task in this phase has zero impact on `output.rs`, so the hash is stable from phase start.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Security Sidecar Defaults**
- `--security` CLI flag defaults to empty string — matches `--meta` ergonomics (Phase-5 D-13 precedent), preserves byte-identical output when absent
- Empty-pattern fallback for the security axis: render `score = 0` with em-dash tooltip — mirrors v1.0 docker_runtimes em-dash convention, preserves byte-identical output and stable 8-axis spider shape
- Frozen-schema test lives in `crates/alloc-bench-core/tests/smoke.rs` (not aggregator-side) — colocates the test with the schema it freezes
- Security sidecar JSON shape locked per SEC-01: `{ env: String, score: u8 (0..=100), rationale: String, captured_at: String }`. Additional fields (e.g., `cve_count`) deferred to v1.2.

**Registry Architecture (AXES-01, AXES-02)**
- `MEASUREMENT_AXES` is a `const` `[AxisSpec; 8]` (NOT a lazy_static / OnceCell) — compile-time constant, alphabetical key order
- 8 axes: channel throughput, memory/fragmentation, web, multithread, cpu-bound, resilience, image-size efficiency (heuristic), security posture (heuristic)
- `Direction` enum hard-codes `arrow()` glyphs as Unicode literals (`'\u{2191}'`, `'\u{2193}'`) — no `unicode-arrows` dependency per Out-of-Scope rule
- `axes.rs` exports both data (`MEASUREMENT_AXES`, `Direction`) and helper (`arrow()`) — single source of truth for direction markers across `score.rs`, `polar.rs`, `markdown.rs`

**Security Loader Plumbing (SEC-01, SEC-02, SEC-03, TEST-03)**
- `SecurityMeta` struct lives in `loader.rs` next to `CellMeta` (mirrors existing layout)
- `load_security_metas(pattern: &str) -> Result<BTreeMap<String, SecurityMeta>>` — `BTreeMap` not `HashMap` for byte-identical output; key is env name from JSON
- Empty-pattern guard: returns `BTreeMap::new()` immediately (matches `load_cell_metas` empty-pattern early-return)
- Per-file failure: log `warn:` to stderr, skip-and-continue — matches `discover()` and `load_cell_metas()` behavior (D-08 contract)
- Schema-version mismatch: out of scope — security sidecars carry no `schema_version` field in v1.1; `serde_json` strict-deserialize handles unknown-field rejection
- Six committed sidecar files in `meta/security/` (alpine/debian-slim/distroless-cc/distroless-static/scratch/wolfi) — hand-curated content, committed alongside the loader

**Frozen-Schema Gate (GUARD-01)**
- Test location: `crates/alloc-bench-core/tests/smoke.rs` (NEW file or extend existing) — colocates test with frozen artifact
- Test mechanism: SHA-256 hash of `crates/alloc-bench-core/src/output.rs` file bytes, computed at test runtime via `sha2` crate (already in workspace) and compared against a hard-coded hex string constant
- Pinning protocol: when test fails, contributor must (a) prove the change is sidecar-only or (b) explicitly bump the pinned hash with a comment explaining the v1 → v2 migration
- Test name verbatim: `smoke::tests::v1_schema_output_rs_is_frozen`

### Claude's Discretion
- Exact `AxisSpec` field layout — at minimum: key (`&'static str`), label (`&'static str`), direction (`Direction`), is_heuristic (`bool`)
- Whether `axes.rs` module sits at crate root of `alloc-bench-aggregator` or under a `scoring/` submodule — start at crate root (mirrors existing flat structure: loader.rs, recommend.rs, etc.)
- Initial content of the six `meta/security/{env}.json` files — research current state (CVE counts, attack-surface size, base-image security posture per env) and assign `score: u8` plus brief `rationale`. `captured_at` is the date the data was sourced.
- Exact CLI flag wiring in `main.rs` — should follow `--meta` precedent in shape and help text wording

### Deferred Ideas (OUT OF SCOPE)
- Heuristic-axis weight cap (≤12.5% aggregate) — recorded as **V12-07** in REQUIREMENTS.md; lives in v1.2
- Workload-shape weighted scoring profiles — **V12-05** (v1.2)
- Confidence intervals on composite scores — **V12-06** (v1.2)
- JSON-driven re-weighting slider — **V12-01** (v1.2)
- Cross-version diff radar — **V12-02** (v1.2)
- Additional security sidecar fields (e.g., `cve_count`) — defer to v1.2 if needed; v1.1 ships the locked 4-field shape
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| AXES-01 | `axes.rs` exports `MEASUREMENT_AXES: [AxisSpec; 8]` const registry — alphabetically keyed | §axes.rs Design (compile-time const-array layout, alphabetical-key static_assert recipe) |
| AXES-02 | `axes.rs` exports `Direction::{Higher, Lower}` + `arrow()` returning `'\u{2191}'`/`'\u{2193}'` | §axes.rs Design (`arrow()` match pattern; const eligibility analysis) |
| SEC-01 | Six committed `meta/security/{env}.json` sidecars with shape `{ env, score: u8, rationale, captured_at }` | §Six Security Sidecars (per-env starter scores + rationale anchors) |
| SEC-02 | `SecurityMeta` struct + `load_security_metas() -> BTreeMap<String, SecurityMeta>` mirroring `load_cell_metas` | §SecurityMeta Plumbing (verbatim mirror of `load_cell_metas` body) |
| SEC-03 | Without `--security`, aggregator falls back to `score = 0` with em-dash tooltip | §SecurityMeta Plumbing (empty-pattern early return; downstream Phase-7 contract noted) |
| GUARD-01 | `cargo test` runs `smoke::tests::v1_schema_output_rs_is_frozen` pinning SHA-256 of `output.rs` | §Frozen-Schema Gate (sha2 dev-dep add; test-body sketch; line-ending policy) |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

The Conventions block of CLAUDE.md governs every Phase 6 task. The directly applicable items:

- **Conventional-commit prefix:** `feat(06)` for code, `docs(06)` for docs, `test(06)` for the smoke test, `chore(06)` for the sidecar JSON files (or `feat(06)` if the planner prefers a single commit per task group). Plan-scoped commits use `feat(06-NN)` where `NN` is the plan number.
- **Aggregator decorate-not-rewrite:** Phase 6 must NOT mutate `crates/alloc-bench-core/src/output.rs`. The frozen-schema test exists *because* the v1 schema is locked.
- **Byte-identical-output discipline:** alphabetical iteration via `BTreeMap` / `BTreeSet` (never `HashMap`/`HashSet`) — explicitly mandated by SEC-02. Note that `load_cell_metas` currently uses `HashMap<(String, String), CellMeta>` (loader.rs:79) — that is a Phase-5 precedent that pre-dates the byte-identical-iteration rule. Phase 6's `load_security_metas` MUST use `BTreeMap` regardless.
- **GHA action pinning / rustc pin:** N/A — Phase 6 doesn't touch CI workflow files or rust-toolchain.toml.
- **No new runtime crate dependencies:** `sha2` lands as a `[dev-dependencies]` entry only (Out-of-Scope rule from REQUIREMENTS.md: "New runtime crate dependencies").

## Codebase Reconnaissance

### `load_cell_metas` — the verbatim mirror target
`crates/alloc-bench-aggregator/src/loader.rs:79–101` defines:
```rust
pub fn load_cell_metas(pattern: &str) -> Result<HashMap<(String, String), CellMeta>> {
    if pattern.is_empty() {
        return Ok(HashMap::new());
    }
    let mut paths: Vec<PathBuf> = glob(pattern)
        .with_context(|| format!("invalid meta glob pattern: {pattern}"))?
        .filter_map(|r| r.ok())
        .collect();
    paths.sort_unstable();

    let mut map: HashMap<(String, String), CellMeta> = HashMap::new();
    for path in paths {
        match load_one_meta(&path) {
            Ok(meta) => {
                map.insert((meta.alloc.clone(), meta.env.clone()), meta);
            }
            Err(e) => {
                eprintln!("warn: skipped meta {}: {}", path.display(), e);
            }
        }
    }
    Ok(map)
}
```
The helper `load_one_meta(path: &Path) -> Result<CellMeta>` lives at lines 103–108. `CellMeta` itself is at lines 57–68.

`SecurityMeta` mirrors this **exactly** with three deviations: (1) returns `BTreeMap<String, SecurityMeta>` keyed on `env`, (2) the warn line should read `"warn: skipped security meta {}: {}"` for grep-distinguishability, (3) `load_one_security_meta` deserialises into the new struct.

### `--meta` clap declaration — the verbatim mirror target
`crates/alloc-bench-aggregator/src/main.rs:42–44`:
```rust
/// Glob pattern for per-cell meta sidecars (image_size_mb / build_time_s).
/// Empty = skip meta merge. CI populates via 'docker inspect' (D-13).
#[arg(long, default_value = "")]
meta: String,
```
The `--security` flag mirrors this precisely:
```rust
/// Glob pattern for per-env security posture sidecars (env-level score).
/// Empty = security axis renders score=0 with em-dash tooltip (SEC-03).
#[arg(long, default_value = "")]
security: String,
```
The two existing `Cli` tests (`cli_meta_flag_defaults_to_empty_string`, `cli_meta_flag_accepts_glob_pattern` at main.rs:74–96) provide the test pattern Phase 6 must add for `--security`. `main()` invokes `load_cell_metas(&cli.meta)` at main.rs:53 — Phase 6 adds a parallel call `load_security_metas(&cli.security)` immediately below it. The result is held in a local but not yet consumed in this phase (the value flows into Phase 7's `score::compute_axes(runs, metas, security_metas)` per CONTEXT integration-points).

### `output.rs` structure — what the SHA-256 freezes
`crates/alloc-bench-core/src/output.rs`: 517 lines, 18,839 bytes. Top-level items:
- `pub const SCHEMA_VERSION: u32 = 1;` (line 3) — the value pinned by `Run.schema_version` validation
- Public structs: `Run`, `Env`, `Build`, `ScenarioInfo`, `HarnessInfo`, `LatencyNs`, `RssGrowthSample`, `Rusage`, `Metrics` — all `#[derive(Debug, Serialize, Deserialize)]`
- `#[serde(skip_serializing_if = "Option::is_none")]` on optional additive fields: `Run.status`, `Run.error`, `Env.docker_image`, `ScenarioInfo.unit`
- An in-file `mod tests` block (lines 99–498) with five round-trip / canonical-shape tests including `run_canonical_shape_snapshot` (lines 191–276) — this latter is the type-level companion to the byte-level GUARD-01 we're about to add

The test must hash the **entire file bytes** including the embedded `mod tests` block. This is fine — the in-file tests are stable and shouldn't change when production code is stable; if they do change, the hash bumps with the same protocol.

### `tests/smoke.rs` status
- `crates/alloc-bench-core/tests/` directory does **NOT exist** — Phase 6 creates it.
- `crates/alloc-bench-core/tests/smoke.rs` does **NOT exist** — Phase 6 creates it.
- An unrelated `crates/alloc-bench-aggregator/tests/smoke.rs` already exists (764 lines, integration test for the aggregator binary). The CONTEXT explicitly locks the new test to **alloc-bench-core**, not the aggregator side — colocation rationale.

### `sha2` dependency status
- `sha2` is **NOT** in `Cargo.lock` (verified via `grep "name = \"sha"`).
- `sha2` is **NOT** in `Cargo.toml` workspace deps.
- `sha2` is **NOT** transitively pulled in by any current dep (would have appeared in Cargo.lock).
- **Action required:** Add `sha2` as `[dev-dependencies]` in `crates/alloc-bench-core/Cargo.toml`. Per CLAUDE.md "no new runtime crate dependencies" rule and Out-of-Scope item, dev-deps are explicitly permitted — the test compiles only under `cargo test`, never into production binaries.

> CONTEXT.md says "`sha2` crate (already in workspace)" — this is **incorrect** at the time of research. The planner must add it as a dev-dependency. This is a benign correction; the locked decision still holds (test mechanism = SHA-256 via `sha2`).

### Aggregator dep audit (Cargo.toml)
- `glob = "0.3"` (workspace dep, line 34 of root Cargo.toml) ✓ already in aggregator
- `serde_json = "1"` (workspace dep) ✓ already in aggregator
- `anyhow = "1"` (workspace dep) ✓ already in aggregator
- `serde = { version = "1", features = ["derive"] }` ✓ already in aggregator
- **No new aggregator deps required for Phase 6.**

### `meta/` directory status
- `meta/` does **NOT exist** at the repo root yet — the existing image-size sidecar fixtures live under `crates/alloc-bench-aggregator/tests/fixtures/multi_run/meta/`. Phase 6 creates the canonical `meta/security/` directory at the repo root for the SEC-01 production sidecars (the aggregator integration tests can ship a parallel fixture set under `tests/fixtures/security/` if needed).

## axes.rs Design

### Module location and visibility
- File: `crates/alloc-bench-aggregator/src/axes.rs` (crate root, mirrors flat layout: `loader.rs`, `recommend.rs`, `multi_run.rs`, etc.)
- `mod axes;` declared in `main.rs` between the existing `mod html;` and `mod loader;` (alphabetical to match existing ordering at main.rs:21–26)
- Public surface: `pub use axes::{MEASUREMENT_AXES, AxisSpec, Direction};` — `arrow()` is reachable via `Direction::Higher.arrow()` (associated method) so we don't need to re-export a free function.

### Direction enum + `arrow()`
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Higher,
    Lower,
}

impl Direction {
    pub const fn arrow(self) -> char {
        match self {
            Direction::Higher => '\u{2191}', // ↑
            Direction::Lower  => '\u{2193}', // ↓
        }
    }
}
```
`const fn` is eligible because `match` on a fieldless enum is supported in const context since Rust 1.46. `char` literals are const-eligible. This lets a future caller use `arrow()` in `const` initialisers if desired.

### AxisSpec layout (Claude's Discretion — recommended)
```rust
#[derive(Debug, Clone, Copy)]
pub struct AxisSpec {
    pub key: &'static str,         // alphabetical sort key — also the JSON / template field name
    pub label: &'static str,       // human-facing label (e.g. "CPU-bound throughput")
    pub direction: Direction,      // higher- or lower-is-better
    pub is_heuristic: bool,        // true → render with `(heuristic)` suffix and dashed gridline (POLAR-03, DIR-01)
}
```
**No optional `weight_hint` field.** The milestone v1.1 spec is explicit equal weights (1/8 each); a `weight_hint` would invite a v1.1 contributor to deviate. Weighting variation is deferred to V12-05 / V12-07 in v1.2.

### The 8 axes — alphabetical key order

| Index | `key` (sort key) | `label` | `direction` | `is_heuristic` | Source for direction |
|-------|------------------|---------|-------------|----------------|-----------------------|
| 0 | `"channel_throughput"` | "Channel throughput" | `Higher` | `false` | ticks_per_s — higher better |
| 1 | `"cpu_bound_throughput"` | "CPU-bound throughput" | `Higher` | `false` | ticks_per_s |
| 2 | `"image_size_efficiency"` | "Image-size efficiency" | `Higher` | `true` | normalised inverse of MB; smaller image → higher efficiency score |
| 3 | `"memory_fragmentation"` | "Memory / fragmentation" | `Lower` | `false` | peak_rss_kb — lower better |
| 4 | `"multithread_throughput"` | "Multithread throughput" | `Higher` | `false` | ticks_per_s |
| 5 | `"resilience"` | "Resilience" | `Higher` | `false` | inverse-failure-rate or stable-completion proxy — Phase 7 fixes the metric |
| 6 | `"security_posture"` | "Security posture" | `Higher` | `true` | sidecar score — higher better |
| 7 | `"web_throughput"` | "Web throughput" | `Higher` | `false` | req_per_s |

### Compile-time invariants

```rust
pub const MEASUREMENT_AXES: [AxisSpec; 8] = [
    AxisSpec { key: "channel_throughput",   label: "Channel throughput",   direction: Direction::Higher, is_heuristic: false },
    AxisSpec { key: "cpu_bound_throughput", label: "CPU-bound throughput", direction: Direction::Higher, is_heuristic: false },
    AxisSpec { key: "image_size_efficiency",label: "Image-size efficiency",direction: Direction::Higher, is_heuristic: true  },
    AxisSpec { key: "memory_fragmentation", label: "Memory / fragmentation", direction: Direction::Lower, is_heuristic: false },
    AxisSpec { key: "multithread_throughput",label:"Multithread throughput",direction: Direction::Higher, is_heuristic: false },
    AxisSpec { key: "resilience",           label: "Resilience",           direction: Direction::Higher, is_heuristic: false },
    AxisSpec { key: "security_posture",     label: "Security posture",     direction: Direction::Higher, is_heuristic: true  },
    AxisSpec { key: "web_throughput",       label: "Web throughput",       direction: Direction::Higher, is_heuristic: false },
];
```

### Alphabetical-key static_assert recipe

A unit test in `axes::tests` proves alphabetical order at compile-test time (`cargo test` is the gate):

```rust
#[test]
fn axes_keys_are_alphabetically_sorted() {
    let keys: Vec<&str> = MEASUREMENT_AXES.iter().map(|a| a.key).collect();
    let mut sorted = keys.clone();
    sorted.sort_unstable();
    assert_eq!(keys, sorted, "MEASUREMENT_AXES must be sorted by `key`");
}

#[test]
fn axes_keys_are_unique() {
    use std::collections::BTreeSet;
    let set: BTreeSet<&str> = MEASUREMENT_AXES.iter().map(|a| a.key).collect();
    assert_eq!(set.len(), MEASUREMENT_AXES.len(), "duplicate axis keys");
}

#[test]
fn axes_count_is_exactly_eight() {
    assert_eq!(MEASUREMENT_AXES.len(), 8, "the 8-axis spider shape is locked");
}

#[test]
fn arrow_glyphs_match_unicode_literals() {
    assert_eq!(Direction::Higher.arrow(), '\u{2191}');
    assert_eq!(Direction::Lower.arrow(),  '\u{2193}');
}

#[test]
fn heuristic_axes_are_image_size_and_security() {
    let heuristics: Vec<&str> = MEASUREMENT_AXES.iter().filter(|a| a.is_heuristic).map(|a| a.key).collect();
    assert_eq!(heuristics, vec!["image_size_efficiency", "security_posture"]);
}
```

A true compile-time `const_assert!` (e.g., the `static_assertions` crate) is **not** worth a new dependency for what `cargo test` already enforces. Out-of-Scope rule "no new runtime crate dependencies" applies here too.

## SecurityMeta Plumbing

### Struct definition (place adjacent to `CellMeta` in loader.rs:57–68)
```rust
/// Per-environment security posture sidecar (SEC-01). Hand-curated
/// content — six files in `meta/security/{env}.json`. v1.1 ships
/// the 4-field locked shape; additional fields (e.g. `cve_count`)
/// deferred to v1.2.
#[derive(Debug, Deserialize)]
pub struct SecurityMeta {
    pub env: String,
    /// Score 0..=100 — higher is better posture. Loader does NOT
    /// validate the range at parse time; downstream Phase-7
    /// `score::compute_axes` clamps via min-max normalization.
    pub score: u8,
    pub rationale: String,
    pub captured_at: String,
}
```
**Range validation:** kept out of the loader so a stale-but-valid sidecar with `score: 250` (typo) isn't a hard reject. The min-max normalisation in Phase 7 caps to `[0, 100]` regardless. If the planner wants a stricter check, add a debug-assert in `load_one_security_meta`.

### `load_security_metas` — verbatim mirror of `load_cell_metas`

```rust
pub fn load_security_metas(pattern: &str) -> Result<BTreeMap<String, SecurityMeta>> {
    if pattern.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut paths: Vec<PathBuf> = glob(pattern)
        .with_context(|| format!("invalid security meta glob pattern: {pattern}"))?
        .filter_map(|r| r.ok())
        .collect();
    paths.sort_unstable();

    let mut map: BTreeMap<String, SecurityMeta> = BTreeMap::new();
    for path in paths {
        match load_one_security_meta(&path) {
            Ok(meta) => {
                map.insert(meta.env.clone(), meta);
            }
            Err(e) => {
                eprintln!("warn: skipped security meta {}: {}", path.display(), e);
            }
        }
    }
    Ok(map)
}

fn load_one_security_meta(path: &Path) -> Result<SecurityMeta> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let meta: SecurityMeta = serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing security meta {}", path.display()))?;
    Ok(meta)
}
```

**Key differences from `load_cell_metas`:**
- Return type: `BTreeMap<String, SecurityMeta>` (env-keyed) — explicitly required by SEC-02; mandated by CLAUDE.md byte-identical-iteration discipline.
- Key: `meta.env.clone()` (single String, not a tuple) — security is per-env, not per-cell.
- Imports: add `use std::collections::BTreeMap;` to loader.rs (currently uses only `HashMap`).
- Warn message: `"warn: skipped security meta {}"` — distinguishable from the cell-meta warn.

### --security clap snippet (place adjacent to `--meta` in main.rs:42–44)
```rust
/// Glob pattern for per-env security posture sidecars (env-level score).
/// Empty = security axis renders score=0 with em-dash tooltip (SEC-03).
#[arg(long, default_value = "")]
security: String,
```
Wire-up in `main()`:
```rust
let security_metas = loader::load_security_metas(&cli.security)?;
```
Place this immediately after the existing `let metas = loader::load_cell_metas(&cli.meta)?;` (main.rs:53). The variable is unused in Phase 6 (downstream Phase 7 consumes it via `score::compute_axes`); silence the unused-variable warning with a leading underscore: `let _security_metas = ...` OR pass through to existing `markdown::write` / `html::write` signatures as `&BTreeMap::new()`-equivalent stubs that Phase 7 fills in.

**Recommendation:** use `let _security_metas = ...` for now — it's the smallest blast radius and the underscore is a clear Phase-7-pickup marker. The Phase 7 task will rename to `let security_metas` and pass into the new `compute_axes` call.

## Six Security Sidecars

Each file lives at `meta/security/{env}.json` (NEW directory at repo root). All six use `captured_at: "2026-05-26"` per CONTEXT specifics.

> **Confidence: MEDIUM.** These are **starter values** anchored on widely-published distroless/Wolfi/Alpine security posture summaries. The planner / discuss-phase may refine ratings during Plan 03 (sidecar authoring) — final ratings are not load-bearing for Phase 6's correctness, only for Phase 7 onward downstream rendering.

### 1. `meta/security/scratch.json` — score: 95
```json
{
  "env": "scratch",
  "score": 95,
  "rationale": "Empty base image — zero packages, zero shell, zero CVE surface beyond the static binary itself. Highest theoretical posture but trades off observability and debugging.",
  "captured_at": "2026-05-26"
}
```

### 2. `meta/security/distroless-static.json` — score: 90
```json
{
  "env": "distroless-static",
  "score": 90,
  "rationale": "Google distroless static base — ~2 MiB; only CA certs, /etc/passwd, tzdata. No shell, no package manager. Smaller surface than distroless-cc but requires statically linked binaries.",
  "captured_at": "2026-05-26"
}
```

### 3. `meta/security/distroless-cc.json` — score: 80
```json
{
  "env": "distroless-cc",
  "score": 80,
  "rationale": "Google distroless cc base — adds glibc + libgcc + libstdc++ for dynamically linked C/C++ binaries. Slightly larger CVE surface than distroless-static due to glibc inclusion; still no shell or package manager.",
  "captured_at": "2026-05-26"
}
```

### 4. `meta/security/wolfi.json` — score: 75
```json
{
  "env": "wolfi",
  "score": 75,
  "rationale": "Chainguard Wolfi — distroless-style minimalism with daily-rebuild fresh packages. Designed for low CVE drift. Includes apk-tools and a minimal package set; larger surface than scratch/distroless but actively patched.",
  "captured_at": "2026-05-26"
}
```

### 5. `meta/security/alpine.json` — score: 60
```json
{
  "env": "alpine",
  "score": 60,
  "rationale": "Alpine Linux 3.x — small (~5 MiB) musl-based distro with apk package manager and busybox shell. Active CVE patching but larger attack surface than distroless: shell, BusyBox utilities, and full package manager are all present.",
  "captured_at": "2026-05-26"
}
```

### 6. `meta/security/debian-slim.json` — score: 45
```json
{
  "env": "debian-slim",
  "score": 45,
  "rationale": "Debian slim — full glibc, dpkg, apt, bash. ~30 MiB. Largest attack surface in the matrix: full package manager, shell, and standard Unix utilities. Mature CVE tracking but slowest-moving security-update cadence among the six.",
  "captured_at": "2026-05-26"
}
```

**Score ordering rationale (95 > 90 > 80 > 75 > 60 > 45):** Each step reflects roughly +1 attack-surface tier added: scratch (none) → distroless-static (CA certs + tzdata) → distroless-cc (+ glibc) → wolfi (+ minimal pkg manager) → alpine (+ shell, busybox) → debian-slim (+ full apt + bash). Score gaps (5–15 points) leave room for v1.2 refinement based on monthly CVE-count delta sourcing without breaking ordinal stability.

**Provenance tags:**
- distroless-static < ~2 MiB / distroless-cc adds glibc — `[CITED: github.com/GoogleContainerTools/distroless]`
- alpine apk + busybox shell, debian-slim full apt — `[VERIFIED: standard Docker base image documentation]`
- Wolfi daily-rebuild posture — `[ASSUMED]`
- Exact score numerics (45, 60, 75, 80, 90, 95) — `[ASSUMED — starter values, refine in plan]`

## Frozen-Schema Gate

### sha2 dev-dep add
**Add to `crates/alloc-bench-core/Cargo.toml`** (currently has no `[dev-dependencies]` section — Phase 6 introduces one):
```toml
[dev-dependencies]
sha2 = "0.10"
```
The `0.10` line has been the `sha2` stable line since 2021 — no breaking changes anticipated. The planner should run `cargo add --dev sha2 -p alloc-bench-core` at plan time to lock the exact patch version into Cargo.lock.

### Test body sketch (`crates/alloc-bench-core/tests/smoke.rs`, NEW file)
```rust
//! Phase 6 GUARD-01: pin SHA-256 of `crates/alloc-bench-core/src/output.rs`
//! to its v1.0 freeze. Guards the v1 schema contract (Phase 1 D-11 +
//! CLAUDE.md Conventions: "Aggregator decorate-not-rewrite").
//!
//! When this test fails, the contributor must either:
//!   (a) prove the diff to output.rs is sidecar-only / additive-with-skip-
//!       serializing-if (and the existing `run_canonical_shape_snapshot`
//!       in output.rs's in-file `mod tests` still passes byte-equivalence),
//!       AND bump the pinned hash with a one-line commit message
//!       explaining the additive change; OR
//!   (b) explicitly migrate the schema to v2 (bump SCHEMA_VERSION to 2 in
//!       output.rs:3 in the SAME commit, regenerate goldens, and bump the
//!       pinned hash).
//!
//! There is no third option; "I just refactored, please trust me" is not.

use sha2::{Digest, Sha256};

/// Pinned SHA-256 of `crates/alloc-bench-core/src/output.rs` at v1 freeze.
/// To recompute (after an intentional, justified change):
///   `sha256sum crates/alloc-bench-core/src/output.rs`
const V1_OUTPUT_RS_SHA256: &str = "<HEX-COMPUTED-AT-PLAN-TIME>";

#[test]
fn v1_schema_output_rs_is_frozen() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/output.rs");
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("reading {} for SHA-256 freeze test: {e}", path.display()));
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let actual = format!("{:x}", hasher.finalize());

    assert_eq!(
        actual, V1_OUTPUT_RS_SHA256,
        "\n\nv1 schema in {} has changed.\n\
         If this is intentional:\n\
           (a) sidecar-only / additive-Option-with-skip-serializing-if change \
               that preserves byte-equivalence of all v1.0 fixtures: bump the \
               pinned hash and document in the commit message; OR\n\
           (b) v1 → v2 schema migration: bump SCHEMA_VERSION in output.rs:3 \
               AND the pinned hash AND regenerate all golden fixtures in \
               the SAME commit.\n\
         Computed: {}\n\
         Expected: {}\n",
        path.display(),
        actual,
        V1_OUTPUT_RS_SHA256,
    );
}
```

The test name **must** be `v1_schema_output_rs_is_frozen` per CONTEXT. The integration-test target file path is `crates/alloc-bench-core/tests/smoke.rs`, so the cargo-test-discovered path is `smoke::v1_schema_output_rs_is_frozen` (top-level `#[test]`) — **not** wrapped in `mod tests`. CONTEXT specifies "Test name verbatim: `smoke::tests::v1_schema_output_rs_is_frozen`" which implies a wrapping `mod tests { … }` block.

> **Convention reconciliation:** CONTEXT specifies the path `smoke::tests::v1_schema_output_rs_is_frozen` (with `::tests::`). Honor the lock — wrap the function in `#[cfg(test)] mod tests { … }` or simply `mod tests { … }` to match. The CONTEXT lock is authoritative.

### Final test layout matching CONTEXT verbatim
```rust
use sha2::{Digest, Sha256};

const V1_OUTPUT_RS_SHA256: &str = "<HEX-COMPUTED-AT-PLAN-TIME>";

mod tests {
    use super::*;

    #[test]
    fn v1_schema_output_rs_is_frozen() {
        // ... body as above
    }
}
```

Note: integration-test files do NOT need `#[cfg(test)]` (the whole file compiles only under `cargo test`), so a plain `mod tests { … }` suffices.

### Line-ending handling — recommended stance

**Recommendation:** Rely on the pre-existing `git config core.autocrlf` / system default. **Do not** add a `.gitattributes` file in Phase 6.

Justification:
- All current contributors are on macOS / Linux (project is `Cargo` + bash + `just`-based; macOS host listed in CLAUDE.md and a Linux Docker container matrix dominates the CI workflow). LF line endings are universal in this environment.
- The aggregator's existing byte-identical-output golden tests (Phase 4 D-17 `crates/alloc-bench-aggregator/tests/smoke.rs`) implicitly assume LF — they have not produced false failures, indicating no Windows contributor traffic.
- Adding a `.gitattributes` introduces a v1.1 deliverable beyond Phase 6 scope and risks a one-time forced re-checkout for contributors with mixed-line-endings caches.

**Document the assumption in the test:** add a single comment line under the pinned hash: `// Hash assumes LF line endings; verified on macOS/Linux via 'sha256sum'.`

If a Windows contributor surfaces and the test fires falsely, the fix is a follow-up `.gitattributes` shipping `* text=auto eol=lf` for `*.rs` — but it's out of scope here.

### Failure-message wording per CONTEXT pinning protocol
The test's `assert_eq!` message body (already drafted above) carries:
1. The two intentional-change scenarios (sidecar-only vs v1→v2 migration).
2. The exact remediation steps for each.
3. Computed-vs-expected hashes for diagnosability.
4. The file path so a contributor can run `sha256sum` directly.

This satisfies the CONTEXT pinning protocol verbatim ("when test fails, contributor must (a) prove the change is sidecar-only or (b) explicitly bump the pinned hash with a comment explaining the v1 → v2 migration").

### The hash-pinning order-of-operations
1. Phase 6 Plan 01 lands `axes.rs` (zero impact on `output.rs`).
2. Phase 6 Plan 02 lands the security loader + sidecars (zero impact on `output.rs`).
3. Phase 6 Plan 03 lands the smoke.rs test with `V1_OUTPUT_RS_SHA256 = "<TBD>"`, then runs `sha256sum crates/alloc-bench-core/src/output.rs`, pastes the result into the constant, and verifies `cargo test -p alloc-bench-core` passes.

The plan ordering is independent — the planner can also collapse all three into a single plan if the wave structure permits.

## Test Strategy Table

| REQ-ID | Minimum Test Name | Pass Condition |
|--------|-------------------|----------------|
| AXES-01 | `axes::tests::axes_count_is_exactly_eight` | `MEASUREMENT_AXES.len() == 8` |
| AXES-01 | `axes::tests::axes_keys_are_alphabetically_sorted` | iterating `.key` yields a strictly sorted sequence |
| AXES-01 | `axes::tests::axes_keys_are_unique` | no duplicate keys |
| AXES-01 | `axes::tests::heuristic_axes_are_image_size_and_security` | exactly `image_size_efficiency` + `security_posture` flagged `is_heuristic` |
| AXES-02 | `axes::tests::arrow_glyphs_match_unicode_literals` | `Direction::Higher.arrow() == '\u{2191}'` AND `Direction::Lower.arrow() == '\u{2193}'` |
| SEC-01 | `loader::tests::six_security_sidecars_parse_cleanly` | glob `meta/security/*.json` from manifest dir → exactly 6 entries; all 4 fields present per entry |
| SEC-01 | `loader::tests::security_score_is_in_zero_to_one_hundred_range` | every committed sidecar has `score <= 100` (loader doesn't enforce, but the goldens do) |
| SEC-02 | `loader::tests::load_security_metas_returns_btreemap_sorted_by_env` (TEST-03 verbatim) | iteration order is alphabetical by `env` key |
| SEC-02 | `loader::tests::load_security_metas_empty_pattern_returns_empty_map` | `load_security_metas("")` → empty BTreeMap, no error |
| SEC-02 | `loader::tests::load_security_metas_skips_malformed_json` | one good + one malformed sidecar → only good survives, stderr warn line |
| SEC-03 | `tests::cli_security_flag_defaults_to_empty_string` (in `main.rs::tests`) | `Cli::parse_from(["alloc-bench-aggregator"]).security == ""` |
| SEC-03 | `tests::cli_security_flag_accepts_glob_pattern` (in `main.rs::tests`) | `--security 'meta/security/*.json'` lands verbatim in `cli.security` |
| GUARD-01 | `smoke::tests::v1_schema_output_rs_is_frozen` (CONTEXT verbatim) | SHA-256 of `output.rs` bytes equals pinned hex |

**Phase 6 brings zero changes to existing v1.0 golden tests.** TEST-01 (Phase 11) requires that all v1.0 byte-stable fixtures still pass — Phase 6's additions are all *new* files / *new* unused variables / *new* unused public symbols; the aggregator emit path is not modified.

## Risks & Pitfalls

### 1. Byte-identical-output discipline — Phase 6 must NOT mutate aggregator emit
**Risk:** A contributor wires `load_security_metas` into `markdown::write` or `html::write` as a "while we're here" optimisation, regenerating goldens.
**Mitigation:** Phase 6 plan must explicitly forbid touching `markdown.rs` / `html.rs` / `recommend.rs`. The variable `_security_metas` (leading underscore) in `main()` is the **only** authorised consumer for this phase.
**Detection:** Phase 4 D-17 `aggregator_emits_html_and_markdown_against_fixtures` test (in `crates/alloc-bench-aggregator/tests/smoke.rs`) is byte-identical against committed fixtures. Any inadvertent mutation fires this test.

### 2. cargo-fmt-induced hash drift on `output.rs`
**Risk:** A contributor runs `cargo fmt` on the whole workspace; rustfmt reformats `output.rs` (e.g., shuffles trailing commas, breaks long lines); SHA-256 changes; CI fires `v1_schema_output_rs_is_frozen` even though the schema is structurally identical.
**Mitigation:**
- Pre-pin the hash by running `cargo fmt --check -p alloc-bench-core` at plan time. If `cargo fmt` produces a diff, apply the format **first** (separate commit `chore(06): cargo fmt output.rs`), THEN compute and pin the hash. This makes `output.rs` rustfmt-stable from the moment of pinning.
- Document in the smoke.rs comment: "Run `cargo fmt --check -p alloc-bench-core` before computing this hash to ensure rustfmt-stability."
**Phase 7+ ripple:** Phase 7's `score.rs` is a **new** file — it doesn't touch output.rs. Phase 7 is hash-safe.

### 3. Forward compat with Phase 7's `score::compute_axes` consumer
**Risk:** Phase 6 ships `MEASUREMENT_AXES` with the wrong shape for the Phase 7 SCORE-03 contract (`compute_axes(runs, metas, security_metas) -> Vec<CellAxes>`). Phase 7 has to back-port a struct change to axes.rs, breaking the Phase 6 hash-stable promise.
**Mitigation:** The `AxisSpec` field set was sized to satisfy SCORE-01 / SCORE-02 / SCORE-03 / DIR-01 / DIR-03 / POLAR-01 / POLAR-03 simultaneously:
- `key`: needed by `compute_axes` (alphabetical iteration), `polar.rs` theta labels, `markdown.rs` table headers (DIR-01 column-header builder), template placeholders (DIR-03)
- `label`: needed by `polar.rs` (theta labels with optional `(heuristic)` suffix per POLAR-03), `markdown.rs` direction-marker columns (DIR-01)
- `direction`: needed by `score::normalize_axis(values, direction)` per SCORE-01, plus `arrow()` injection (DIR-01 / DIR-03)
- `is_heuristic`: needed by `polar.rs` (dashed gridline styling, POLAR-03), `markdown.rs` (`(heuristic)` suffix)
**No fields are missing for Phases 7–10 against the locked v1.1 spec.** v1.2 weight-cap (V12-07) and workload-shape weights (V12-05) are out of scope and would extend AxisSpec at that point — no impact on Phase 6's hash-stability promise.

### 4. `HashMap` vs `BTreeMap` divergence between `load_cell_metas` and `load_security_metas`
**Risk:** Code reviewers ask "why does cell-meta use HashMap but security-meta use BTreeMap?" and request unification.
**Mitigation:** Document in `loader.rs` (Phase 6 plan) the asymmetry: `load_cell_metas`'s HashMap pre-dates the byte-identical-iteration discipline (Phase-5 D-13 precedent); SEC-02 explicitly mandates BTreeMap. **Don't change `load_cell_metas`** in Phase 6 — that's a Phase-11 goldens-regen task at the earliest. Adding even a "harmless" `.collect::<BTreeMap<_,_>>()` wrapper risks tripping TEST-01.

### 5. Sidecar fixture path collision with Phase 5
**Risk:** `meta/` at the repo root is currently empty, but Phase 5's image-size sidecars (`meta/{alloc}-{env}.json`) will populate it once CI runs. A collision (e.g., a file named `meta/alpine.json` could be read by both globs) is theoretically possible.
**Mitigation:** Phase 6 places security sidecars under `meta/security/` (distinct subdirectory). The `--meta` glob pattern from CI is `meta/*.json` (per Phase 5 D-13 precedent — though `_security_metas` is not enabled by `--meta`, only by `--security`); so the patterns `meta/*.json` and `meta/security/*.json` are non-overlapping. No risk.

### 6. SHA-256 hex-string formatting consistency
**Risk:** `sha256sum` (GNU coreutils, default on Linux/CI) produces lowercase hex; `shasum -a 256` (macOS BSD default) also produces lowercase; but `format!("{:x}", finalize())` in `sha2` also produces lowercase. All three agree → no risk if a contributor uses any of them. **Rust `{:X}` formatter would produce uppercase** — explicitly use `{:x}` (lowercase).
**Documentation:** Add a comment above the constant: `// Lowercase hex; matches both 'sha256sum' coreutils and 'shasum -a 256' macOS output.`

## Sources

### Primary (HIGH confidence — codebase reconnaissance)
- `crates/alloc-bench-aggregator/src/loader.rs:79–101` — `load_cell_metas` body (verbatim mirror target)
- `crates/alloc-bench-aggregator/src/loader.rs:57–68` — `CellMeta` struct (mirror layout)
- `crates/alloc-bench-aggregator/src/main.rs:42–44` — `--meta` clap declaration (verbatim mirror target)
- `crates/alloc-bench-aggregator/src/main.rs:74–96` — existing `Cli` test patterns
- `crates/alloc-bench-core/src/output.rs:1–517` — frozen file (517 lines, 18,839 bytes)
- `crates/alloc-bench-core/src/output.rs:191–276` — existing `run_canonical_shape_snapshot` (companion type-level guard)
- `crates/alloc-bench-aggregator/Cargo.toml` — confirms `glob`/`serde_json`/`anyhow`/`serde` already present
- `crates/alloc-bench-core/Cargo.toml` — confirms NO `[dev-dependencies]` section yet, NO `sha2`
- `Cargo.toml` workspace deps (root) — confirms `sha2` is NOT a transitive dep
- `Cargo.lock` — confirms `sha2` is NOT in the dependency closure
- `.planning/phases/06-foundations/06-CONTEXT.md` — locked decisions
- `.planning/REQUIREMENTS.md` — REQ-IDs and v1.1 phase mapping
- `CLAUDE.md` Conventions block — byte-identical iteration, decorate-not-rewrite, conventional-commit prefixes

### Secondary (MEDIUM confidence)
- distroless-static / distroless-cc relative size — `[CITED: github.com/GoogleContainerTools/distroless]` (~2 MiB confirmed)
- alpine + busybox + apk attack surface — `[VERIFIED: standard Docker base image documentation]`
- debian-slim full apt + bash — `[VERIFIED: standard Docker base image documentation]`

### Tertiary (LOW confidence — flagged for refinement in plan)
- Wolfi daily-rebuild posture — `[ASSUMED]` (training data; not verified in this session)
- Exact security-score numerics (45 / 60 / 75 / 80 / 90 / 95) — `[ASSUMED — starter values, refine in plan]`. Rank order is well-established but specific point gaps are subjective.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `sha2` v0.10.x is current stable line on crates.io | §Frozen-Schema Gate | Low — planner runs `cargo add` at plan time and the Cargo.lock pin replaces this assumption |
| A2 | Wolfi has a higher security posture than alpine due to daily rebuilds | §Six Security Sidecars #4 | Low — relative ranking, not absolute claim; planner can re-rank in plan |
| A3 | The exact security score gaps (e.g., 90 vs 95 for distroless-static vs scratch) are starter values | §Six Security Sidecars | None — sidecars are content artifacts; renderable end-to-end at any plausible score |
| A4 | Project contributors are macOS/Linux only — LF line endings universal | §Frozen-Schema Gate (line-ending handling) | Low — if a Windows contributor surfaces, fix is a `.gitattributes` follow-up |
| A5 | `cargo fmt -p alloc-bench-core` is currently clean against `output.rs` | §Risks & Pitfalls #2 | Low — verifiable at plan time; if dirty, plan adds `chore(06): cargo fmt output.rs` first |
| A6 | The CONTEXT-locked test name `smoke::tests::v1_schema_output_rs_is_frozen` (with `::tests::`) requires a wrapping `mod tests` in the integration test file | §Frozen-Schema Gate | None — plain `mod tests { ... }` works in integration-test targets |

## Metadata

**Confidence breakdown:**
- Codebase reconnaissance — HIGH (every file:line reference verified by `Read`)
- Loader plumbing design — HIGH (1:1 verbatim mirror of established Phase-5 D-13 pattern)
- axes.rs design — HIGH (compile-time const-array, no novel Rust mechanics)
- Security sidecar starter scores — MEDIUM (rank ordering well-established; absolute values flagged ASSUMED)
- Frozen-schema gate — HIGH (`sha2` 0.10 is well-known; sidecar test mechanism is standard practice)

**Research date:** 2026-05-26
**Valid until:** 2026-06-25 (30 days for stable Rust workspace plumbing)

## RESEARCH COMPLETE

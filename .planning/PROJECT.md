# rust-benchmark-glibc-musl-mimalloc

## What This Is

A comprehensive Rust benchmark suite comparing four memory allocators (glibc/ptmalloc, musl/mallocng, jemalloc, mimalloc) across six libc×allocator combinations, seven runtime environments (macOS host + six Docker images), and eight benchmark scenarios (micro-allocation stress test, web service, SPMC/MPSC/MPMC channels, CPU-bound, memory-bound, and lock-contention). Results are aggregated into an interactive HTML dashboard with Plotly charts and a Markdown report with Mermaid.js allocator-architecture diagrams.

## Core Value

Every result is reproducible, environment-labelled, and visually comparable — so the reader can confidently recommend the right allocator for a given workload.

## Requirements

### Validated

- ✓ Cargo workspace with `alloc-bench-cli`, `alloc-bench-core`, `alloc-bench-aggregator` crates and Cargo-feature allocator selection — v1.0
- ✓ Six libc×allocator build targets shipped via Phase 3 Dockerfiles (glibc-ptmalloc / glibc-jemalloc / glibc-mimalloc / musl-mallocng / musl-jemalloc / musl-mimalloc) — v1.0
- ✓ Multi-thread allocation stress benchmark (SCEN-01) with `--threads N --objects M --size-dist <…>` CLI — v1.0
- ✓ Web-service benchmark on axum + serde_json + tokio with `--server-workers/--client-workers/--duration` and req/s + p50/p95/p99/p999 latency — v1.0
- ✓ SPMC, MPSC, MPMC channel benchmarks via crossbeam-channel — v1.0
- ✓ CPU-bound parallel merge-sort benchmark — v1.0
- ✓ Memory-bound benchmark (linked-list + strided-array modes) — v1.0
- ✓ Lock-contention benchmark — v1.0
- ✓ Custom harness with HDR-histogram percentile latency + getrusage peak RSS + allocator-internal stats — v1.0
- ✓ Compile-time metadata injection via `vergen` (rustc version, target triple, host triple, profile, git SHA, build timestamp) — v1.0
- ✓ Release profile: `lto = "fat"`, `codegen-units = 1`, `opt-level = 3`, `strip = "symbols"`, `panic = unwind` (preserved per Phase-2 review CR-01) — v1.0
- ✓ Justfile orchestrating the 18-cell meaningful matrix (`just bench-all`) plus `just bench-host` for macOS libmalloc baseline — v1.0
- ✓ GitHub Actions CI on `ubuntu-24.04` running the 18-cell matrix × 3 seeds, with `dive --ci` image-size enforcement — v1.0 (definitional ORCH-04 push observation deferred to UAT)
- ✓ Six Docker environments: alpine, debian-slim, distroless-cc, distroless-static, scratch, wolfi (with cargo-chef multi-stage builds + OCI annotations) — v1.0
- ✓ `dive --ci` image-size gate wired to GHA per-cell — v1.0
- ✓ results.json per run with env + allocator + scenario + metrics (locked v1 schema) — v1.0
- ✓ Plotly HTML dashboard with multi-select sidebar (scenarios × envs × allocators), throughput bar / latency heatmap / RSS line / A/B diff charts — v1.0 (browser-rendering visual gate deferred to UAT)
- ✓ REPORT.md with per-scenario winner-highlighted comparison tables, Docker runtime table, four Mermaid allocator-architecture diagrams, multi-run statistics (median + min/max + CV%), high-variance flagging (CV > 10%), data-derived Recommendations — v1.0
- ✓ README.md system diagram (kernel → libc → application allocator → user code) + 5-step "Run it yourself" walkthrough + 18-cell matrix overview + Reproducibility section — v1.0 (fresh-user reproduction gate deferred to UAT)
- ✓ `MEASUREMENT_AXES: [AxisSpec; 8]` registry (alphabetical) + `Direction::{Higher, Lower}` enum + `arrow()` helper in `axes.rs` — single source of truth for direction-marker glyphs — v1.1
- ✓ Six hand-curated `meta/security/{env}.json` sidecars (alpine, debian-slim, distroless-cc, distroless-static, scratch, wolfi) loaded via `load_security_metas() -> BTreeMap<String, SecurityMeta>`; em-dash fallback when `--security` absent — v1.1
- ✓ `v1_schema_output_rs_is_frozen` integration test pinning SHA-256 of `crates/alloc-bench-core/src/output.rs` to its v1.0 freeze — guards against accidental v1 schema mutation — v1.1
- ✓ `score.rs` data-only scoring module: `normalize_axis` (direction-aware, p10/p90 winsorization), `compute_axes`, `score_cells` (equal-weight composite via `MEASUREMENT_AXES.iter()` constant traversal), `top_n` (alphabetical `(alloc, env)` tiebreak); NaN-poisoning short-circuits to em-dash, never silently sorts — v1.1
- ✓ `recommend.rs` extended with `CellRecommendation` struct + `top_n_cells()` + named constants `TOP_N_SPIDER=3 / TOP_N_TABLE=5 / TOP_N_TOTAL=10`; existing 13 `recommendations()` tests untouched — v1.1
- ✓ Per-cell artifacts: two tinytemplate files (`recommend-cell.md.tmpl` + `recommend-cell.html.tmpl`) driven by the same `CellRecommendation`; ten Markdown + ten HTML fragments emitted to `report/`; drift caught at compile time via `cell_templates_both_reference_all_fields` sentinel test (the WR-01 pattern) — v1.1
- ✓ `## Top 10 cells` section in REPORT.md with top-5 above-fold + 5 in collapsible `<details>` (Cowan's 4±1 working-memory bound); data-derived prose (TL;DR → Strengths → Weaknesses → Recommended-for → Avoid-for) — v1.1
- ✓ `polar.rs` server-side `scatterpolar` trace builder: 9-element polygon-closure invariant, top-3 cells above-fold as small-multiples grid, matrix-mean reference polygon at 25% alpha, `(heuristic)` axis suffix + #666 tickfont distinguishing heuristic axes, `pareto_front` + `★` glyph overlay — v1.1
- ✓ `PLOTLY_SRI_HASH` constant + `plotly_sri_hash_unchanged` test pinning Plotly CDN to v2.35.3 — guards against silent `scatterpolar` trace-API drift on upgrade — v1.1
- ✓ Direction markers (↑/↓) on every measurement column header in REPORT.md and every chart axis label in `index.html`, drawn from `axes::column_header_with_arrow` SSoT helper; cells unchanged from v1.0 byte-stable formatting — v1.1
- ✓ One-line legend `↑ higher is better · ↓ lower is better · ⚠ suspect run` above every per-scenario table — explicitly disclaims arrows are direction markers, not column-sort indicators — v1.1
- ✓ `<span aria-label="…">` aria-wrapping for every direction-marker glyph in `index.html` (server-side template + JS post-render pass) — WCAG 2.1 SC 1.3.3 conformance — v1.1
- ✓ Standalone golden-fixture-regen PR convention codified in CLAUDE.md §Conventions — byte-changing surface additions ship in a separate Phase-N PR carrying only fixture regen + verification metadata; inherits to v1.2+ milestones — v1.1

### Active

(None — v1.1 milestone shipped 2026-05-30. Run `/gsd:new-milestone` to define v1.2 requirements.)

### Out of Scope

- Windows or macOS cross-compilation for the allocator matrix — all allocator benchmarks run on Linux (Docker or Linux host)
- Criterion HTML reports — custom harness is the single source of truth
- Runtime LD_PRELOAD allocator swapping — allocator is baked in at compile time for clean results
- snmalloc / tcmalloc / rpmalloc — deferred to v2 if v1.1 ships clean (allocator matrix is locked at 4 for v1.x)
- Marimo notebook output — Plotly HTML covers interactive exploration; Marimo deferred to v2
- GUI or web server for live dashboard — static HTML only
- Android / embedded targets — out of scope for this comparison
- Mutating `crates/alloc-bench-core/src/output.rs` for `security_score` field — locked v1 schema (Phase 1 D-11); sidecar pattern is the only correct path
- Plotly upgrade past 2.35.3 — currently pinned bundle ships `scatterpolar`; future upgrade requires explicit trace-API audit gated by `plotly_sri_hash_unchanged` test
- All top-10 cells overlaid on one spider chart — >3 polygons becomes occluded; small-multiples (one chart per cell, top-3 above the fold) is the convention
- Direction markers on every cell — visual noise + breaks `{:.1}` byte-stable formatting; markers live in column headers only
- Hand-edited recommendation prose — must be data-derived (template-rendered from `CellRecommendation` fields); only the `*(suspect)*` v1.0 italic suffix is allowed
- Raw min/max normalization without winsorization — a single crashed/outlier run squashes the entire axis range; p10/p90 winsorization at N=18 is the floor
- z-score normalization — doesn't yield a 0–100 axis; min-max is the correct primitive for spider chart radial values
- GUI for editing security sidecars — 6 hand-curated JSON files; editing in `$EDITOR` is sufficient

## Context

**Prior research (session, May 2026):**
- `tikv-jemallocator` 0.6.1 (Oct 2025) is the current standard jemalloc Rust binding. Provides `tikv_jemalloc_ctl` for stats. Dual Apache/MIT.
- `mimalloc` crate (microsoft/mimalloc v3.3.2, April 2026) is the current Rust wrapper. No official Rust-from-Microsoft binding; community `mimalloc` crate on crates.io is canonical.
- musl target: `x86_64-unknown-linux-musl` — statically links musl libc + mallocng (musl ≥ 1.2.1). Alpine Docker image uses musl dynamically.
- OCI labels canonical set: `org.opencontainers.image.{created,authors,url,documentation,source,version,revision,vendor,licenses,title,description,base.digest,base.name}`.
- User is on macOS (darwin). All allocator-comparable runs happen in Docker or on a Linux host. macOS host run measures libmalloc as a 7th environment for "dev-box baseline" only — not compared 1:1 with Linux combos.

**Design decisions locked in (user Q&A):**
- Structure: single CLI crate / Cargo workspace, multi-binary per allocator (Cargo features select allocator)
- Allocator matrix: full 6-combo (glibc-ptmalloc, glibc-jemalloc, glibc-mimalloc, musl-mallocng, musl-jemalloc, musl-mimalloc) + macOS libmalloc host
- Web stack: axum + serde_json + tokio
- Reporting: JSON output + Markdown report + interactive Plotly HTML dashboard
- Benchmark harness: custom (not criterion) — duration-based, warm-up, HDR-histogram latency, peak RSS, allocator stats APIs
- Metrics: peak RSS + memory growth, p50/p95/p99/p999 latency, allocator-internal stats, page faults / rusage
- Orchestration: Justfile + bench script + GitHub Actions matrix CI

## Constraints

- **Platform**: All allocator-vs-allocator benchmarks must run on Linux (Docker or Linux host) — macOS libmalloc run is a separate baseline only
- **Build**: Allocator selection is compile-time (Cargo feature flag) — no LD_PRELOAD; ensures fair, reproducible comparisons
- **Reproducibility**: Justfile and Docker builds must be fully self-contained — no manual steps beyond `just bench-all`
- **Image size**: Docker images should be as small as practical; Dive CI gate enforces no large unexpected layers
- **Performance build flags**: LTO=fat, codegen-units=1, opt-level=3 mandatory for benchmark binaries; debug symbols stripped from Docker images
- **Compiler version in output**: All bench binaries must print rustc version, target triple, allocator name at startup (injected at compile time)

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Cargo features select allocator at build time | Clean, reproducible, no LD_PRELOAD fragility | ✓ Good — shipped in v1.0 |
| Custom harness over Criterion | Duration-based throughput/latency-distribution benches with warm-up + HDR histograms | ✓ Good — shipped in v1.0 (Phase 1 D-09) |
| axum + serde_json + tokio for web bench | Most representative modern Rust async stack in 2026 | ✓ Good — shipped (Phase 2 SCEN-02) |
| macOS host as 7th env (libmalloc) | Dev-box baseline; documented as not directly comparable to Linux 6-combo matrix | ✓ Good — `just bench-host` ships in v1.0 |
| Plotly HTML dashboard (not Marimo) | Self-contained, zero Python dependency, opens via `file://` | ✓ Good — shipped in v1.0 |
| Plotly via CDN (not inlined ~4MB) | Keeps committed `index.html` ~100KB; SRI hash + crossorigin=anonymous integrity | ✓ Good — shipped (Phase 4 D-02; CR-01 fix added defense-in-depth XSS escape) |
| tinytemplate for HTML rendering | Minimal Rust template engine; brace-escape pitfall guarded by compile-time test | ✓ Good — shipped (Phase 4 D-01) |
| Multi-run statistics: Bessel-corrected sample stddev + CV% > 10% high-variance flag | Captures CI runner variance; conservative threshold (10%) per industry conventions | ✓ Good — shipped (Phase 5 D-11/D-12) |
| Sidecar `meta.json` for image_size_mb backfill | Preserves locked v1 schema (Phase 1 D-11) while populating Docker runtimes table | ✓ Good — shipped (Phase 5 D-13) |
| OCI annotations on all Docker images | opencontainers/image-spec best practice | ✓ Good — shipped in v1.0 |
| Dive CI gate for image efficiency | Prevents accidental large layers in a public benchmark repo | ✓ Good — wired to GHA (Phase 5 D-08) |
| 18-cell meaningful matrix (not 36) | Skip cross-libc combos that are physically impossible (mallocng-on-glibc, ptmalloc-on-musl) | ✓ Good — shipped (Phase 3 D-04) |
| `target-cpu=x86-64-v3` (not native) for Docker | Portable across CI runners; native reserved for `just bench-host` | ✓ Good — shipped (Phase 3 D-09) |
| `panic = unwind` preserved at toolchain default | Preserves `std::panic::catch_unwind` per-scenario isolation in run-all (Phase-2 CR-01) | ✓ Good — preserved in v1.0 |
| 5-phase MVP decomposition | Each phase ships an independently verifiable artifact (skeleton → full scenarios → matrix → dashboard → CI) | ✓ Good — shipped 2026-05-19 |
| Decorate-not-rewrite preserved through v1.1 | Locked v1 schema in `output.rs` is NOT modified; v1.1 data rides on `meta/security/{env}.json` sidecars or is computed in `alloc-bench-aggregator` from existing v1 fields | ✓ Good — shipped in v1.1; SHA-256 frozen-schema test pins it (Phase 6 GUARD-01) |
| p10/p90 winsorization (not p5/p95) at N=18 | `floor(0.05 × 18) = 0` collapses to raw min/max; `floor(0.1 × 18) = 1` clips one cell per tail; pinned by `normalize_axis_p10_p90_clips_one_outlier_each_tail_at_n18` | ✓ Good — shipped (Phase 7 SCORE-02) |
| Equal weights across 8 axes (1/8 each) | Milestone v1.1 spec; heuristic-axis weight cap (≤12.5% aggregate) deferred to v1.2 if testing shows worst-perf cells ranking high on heuristics alone | — Pending — v1.2 review (V12-07) |
| BTreeMap for security sidecars (not HashMap) | Byte-identical-output discipline — deterministic alphabetical iteration; pinned by `load_security_metas_returns_btreemap_sorted_by_env` test | ✓ Good — shipped (Phase 6 SEC-02 + Phase 11 TEST-03) |
| `column_header_with_arrow` SSoT helper in `axes.rs` | Cross-surface drift defended — single helper consumed by both `markdown.rs` (REPORT.md) and `html.rs` (chart axis labels) | ✓ Good — shipped (Phase 10 DIR-03) |
| Aria-wrap direction-marker glyphs in HTML | WCAG 2.1 SC 1.3.3 conformance — `<span aria-label="higher is better">↑</span>` server-side + JS post-render pass | ✓ Good — shipped (Phase 10 DIR-04) |
| Per-cell artifacts via two templates with sentinel sync test | Markdown card + HTML panel driven by same `CellRecommendation` struct; `cell_templates_both_reference_all_fields` catches drift at compile time (the WR-01 pattern) | ✓ Good — shipped (Phase 8 CELL-01..02) |
| Standalone golden-fixture-regen PR convention | Byte-changing surface additions ship in a separate Phase-N PR carrying only fixture regen + verification metadata; reviewer-visible so regen is intentional and gated; codified in CLAUDE.md §Conventions for v1.2+ | ✓ Good — established Phase 11 (v1.1 release gate); inherits to all future milestones |
| 6-phase v1.1 decomposition (Phases 6-11) | Phase 6 blocks 7+9+10 (axes registry); Phase 7 blocks 8+9 (CellRecommendation); Phase 10 blocks 11 (direction markers change column-header bytes) — strictly serial build order | ✓ Good — shipped 2026-05-30 |

---
*Last updated: 2026-05-30 — v1.1 milestone shipped (Recommendations, Spider Charts & Direction Markers)*

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

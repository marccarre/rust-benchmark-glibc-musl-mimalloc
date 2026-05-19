---
phase: 05-ci-image-size-gate-public-polish
reviewed: 2026-05-19T18:30:00Z
depth: standard
files_reviewed: 11
files_reviewed_list:
  - .github/workflows/bench.yml
  - crates/alloc-bench-aggregator/src/html.rs
  - crates/alloc-bench-aggregator/src/loader.rs
  - crates/alloc-bench-aggregator/src/main.rs
  - crates/alloc-bench-aggregator/src/markdown.rs
  - crates/alloc-bench-aggregator/src/multi_run.rs
  - crates/alloc-bench-aggregator/src/recommend.rs
  - crates/alloc-bench-aggregator/templates/index.html.tmpl
  - crates/alloc-bench-aggregator/tests/smoke.rs
  - justfile
  - README.md
findings:
  critical: 0
  warning: 5
  info: 6
  total: 11
status: issues_found
---

# Phase 5: Code Review Report

## Summary

Phase 5 deliverables are largely solid. Headline-risk items (rustc 1.91 pin, 18-cell explicit `include:` matrix, no cross-libc cells, `target-cpu=x86-64-v3`, action SHAs, concurrency expression, Bessel-corrected stddev, `> 10.0` CV threshold, median for even-length arrays, sidecar parser, `--meta` defaults, byte-identical contracts, README literal repo URL) all check out correctly.

Five real defects exist:
1. The GHA workflow has a redundant `actions/cache@v4` layer that conflicts with `Swatinem/rust-cache@v2` and silently defeats the `save-if: main-only` budget guard.
2. The workflow drops `OCI_VERSION` and `OCI_CREATED` build-args, leaving those OCI labels empty in CI builds.
3. `html::render` accepts `metas` but discards it (`_metas`) — sidecar data never reaches the HTML dashboard despite the function signature suggesting otherwise.
4. README claims **eleven** scenarios in five places; the CLI ships **ten**.
5. Stale comment claims `just ci-aggregate` is a stub that exits 1; it is fully implemented.

No security-critical findings (no secret leakage, injection, or XSS regressions). The `to_script_safe_json` wrapper is correctly applied, CSP is present, SRI is pinned. No correctness defects in the multi_run statistics path.

## Warnings

### WR-01: Redundant `actions/cache@v4` defeats Swatinem cache budget guard

**File:** `.github/workflows/bench.yml:152-165`
**Issue:** Two cache layers compete for the same paths (`~/.cargo/registry`, `~/.cargo/git`, `target/`). `Swatinem/rust-cache@v2` has `save-if: ${{ github.ref == 'refs/heads/main' }}`; `actions/cache@v4` has no save-if guard. Every PR run writes ~1 GB+ via actions/cache, blowing through the 10 GB cache budget. With 18 matrix cells × N PRs, eviction churn becomes severe.
**Fix:** Remove the `actions/cache@v4` layer entirely (Swatinem is sufficient).

### WR-02: Workflow drops `OCI_VERSION` and `OCI_CREATED` build-args

**File:** `.github/workflows/bench.yml:179-185`
**Issue:** Dockerfiles declare `ARG OCI_VERSION`, `ARG OCI_CREATED` and emit them as `org.opencontainers.image.version`, `created` labels. The workflow only passes `OCI_REVISION`. CI-built images carry empty labels.
**Fix:** Pass `OCI_VERSION=${{ github.ref_name }}` and `OCI_CREATED=${{ github.event.head_commit.timestamp || github.run_started_at }}`.

### WR-03: `html::render` accepts `metas` but discards it

**File:** `crates/alloc-bench-aggregator/src/html.rs:212`
**Issue:** `fn render(runs: &[Run], _metas: &HashMap<(String, String), CellMeta>) -> Result<String>` — leading underscore suppresses unused-parameter warning. `image_size_mb` is rendered ONLY in REPORT.md, never in the HTML dashboard. Function signature lies about behavior.
**Fix:** Either drop the `metas` parameter entirely (if HTML-side is out of scope) or thread `metas` into `BuiltContext` and add a smoke test asserting the HTML contains the meta value.

### WR-04: README claims 11 scenarios; CLI ships 10

**File:** `README.md:5,7,22,37,94`
**Issue:** Five places say "eleven workload scenarios" / "11 scenarios." The actual scenario list in `crates/alloc-bench-cli/src/run.rs::default_scenarios` is **ten**: multithread, spmc, mpsc, mpmc, contention, mem-bound, realloc-storm, cpu-bound, fragmentation-soak, web. The justfile and matrix-overview taxonomy agree on 10.
**Fix:** Replace "eleven" / "11 scenarios" with "ten" / "10 scenarios" in five README locations.

### WR-05: Plotly errorbar `visible: true` with all-zero arrays renders cosmetic ticks on single-run cells

**File:** `crates/alloc-bench-aggregator/templates/index.html.tmpl:441-481`
**Issue:** Single-run cells push 0 to yMinus/yPlus. Plotly renders zero-length error bars as small ticks on the bar's centerline. Mixed dashboards (some cells multi-run, some not) show inconsistent appearance.
**Fix:** Track `hasErrorBars = yPlus.some(v => v > 0) || yMinus.some(v => v > 0)` and gate `visible` on that flag.

## Info

### IN-01: Stale comment claims `just ci-aggregate` is a stub
**File:** `.github/workflows/bench.yml:253-255` — Comment says recipe is a stub; `justfile:373-375` shows it is fully implemented.
**Fix:** Update the comment to describe current behavior.

### IN-02: `paths.sort_unstable()` is byte-lexicographic, not Unicode-collated
**File:** `crates/alloc-bench-aggregator/src/loader.rs:124`
**Fix:** Update doc comment from "alphabetical" to "byte-lexicographic" so future readers don't assume Unicode collation.

### IN-03: `mv artifacts/*.json meta/` is too permissive
**File:** `.github/workflows/bench.yml:246-251` — Future contributors adding non-seed JSON to per-cell artifacts would silently land them in `meta/`.
**Fix:** Either use `extglob` (`!(*-seed*).json`) or split per-cell artifacts into distinct subdirectories.

### IN-04: `central_tendency` doc comment misleads on empty-samples branch
**File:** `crates/alloc-bench-aggregator/src/markdown.rs:212-222`
**Fix:** Remove parenthetical "only possible if the caller passed `None`" — the empty-vec branch is also reachable.

### IN-05: `pick_rationale_scenario` lacks alphabetical-tiebreak unit test
**File:** `crates/alloc-bench-aggregator/src/recommend.rs:266-284`
**Fix:** Add a unit test pinning the first-on-tie contract, or rewrite to use `min_by(|a, b| b.cmp(a))` for clearer semantics.

### IN-06: `to_script_safe_json` does not escape `/`
**File:** `crates/alloc-bench-aggregator/src/html.rs:132-138`
**Issue:** Currently safe (escapes `<`, `>`, `&`), but defense-in-depth would add `/` → `/` per OWASP recommendation.
**Fix:** Add `.replace('/', "\\u002f")` to the wrapper.

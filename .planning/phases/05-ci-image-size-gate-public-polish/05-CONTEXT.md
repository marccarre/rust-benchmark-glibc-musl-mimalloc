# Phase 5: CI, Image-Size Gate & Public Polish - Context

**Gathered:** 2026-05-19
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous)

<domain>
## Phase Boundary

Wire the Phase 1–4 stack to GitHub Actions for reproducible push-driven runs, enforce image-size budgets via `dive --ci` in CI, surface multi-run median + min/max + CV% in REPORT.md, and ship the public-facing README "Run it yourself" walkthrough. This is the milestone-closing phase.

Concretely, the phase ships:

1. `.github/workflows/bench.yml` — push + PR + workflow_dispatch; full 18-cell (alloc × env) matrix on `ubuntu-24.04`; ≥3 runs per cell with seeds 1/2/3; per-image dive gate; final aggregator job that downloads all per-cell artifacts and emits a single combined `report/`.
2. Dive image-size enforcement using the existing `.dive-ci` config from Phase 3 — no new thresholds.
3. Aggregator extension: when ≥3 runs of the same `(alloc, env, scenario)` are present, compute median + min/max + CV%; flag CV > 10% as "⚠ high variance" in both REPORT.md and the HTML dashboard. Backfill `image_size_mb` field from `docker inspect` output (closes Phase 4 D-10 deferral).
4. README rewrite: hero / Phase-4-system-diagram (kept) / new `## Run it yourself` walkthrough / new `## Allocator matrix overview` / new `## Reproducibility` section. CI status badge at top.

Phase 5 does NOT add: new allocators (v2), new scenarios (v2), aarch64 axis (v2), Marimo notebooks (v2), or any breaking schema changes.

The aggregator is extended (multi-run statistics + image_size_mb backfill). The HTML dashboard mirrors the variance flag in chart legends.

</domain>

<decisions>
## Implementation Decisions

### GitHub Actions matrix topology
- **D-01:** **Triggers:** `push` (any branch) + `pull_request` (target = main) + `workflow_dispatch` (manual run). The push trigger covers the ROADMAP success-criterion 1 wording ("user pushes a commit"); workflow_dispatch lets the user re-run on demand without a code change.
- **D-02:** **Matrix scope: 18 cells** = 9 glibc-env×glibc-alloc + 9 musl-env×musl-alloc, exactly the meaningful matrix locked by Phase 3 D-01 (3 glibc envs × 3 glibc allocs + 3 musl envs × 3 musl allocs). `runs-on: ubuntu-24.04` per ROADMAP success-criterion 1. `fail-fast: false` so one cell failing doesn't kill the rest. The entire 18-cell matrix is a single GHA matrix; each cell is a separate job (parallel across runners).
- **D-03:** **Sequential within a job:** within a single matrix-cell job, the 11 scenarios run sequentially under `just bench-all-smoke` (or a CI-tuned recipe). Sequential is mandatory: parallel scenarios would multiplex allocators in the same kernel page cache (same constraint as Phase 3 D-11).
- **D-04:** **3 runs per cell with fixed seeds 1/2/3.** Matches REPR-03 floor. Seeds are deterministic so runs are reproducible across reruns. Each run produces a separate `results/{alloc}-{env}-seed{N}.json` file. The aggregator groups runs by `(alloc, env, scenario)` tuple and computes statistics across the 3.
- **D-05:** **Artifacts:**
  - Per-cell job uploads `results/` directory (3 JSON files per cell) via `actions/upload-artifact@v4`.
  - A final `aggregate` job declares `needs: [bench-matrix]`, downloads all per-cell artifacts via `actions/download-artifact@v4`, runs `just aggregate` over the merged `results/`, and uploads the resulting `report/` as a single artifact.
  - Retention: default 90 days for both. No long-term S3 storage in v1.
- **D-06:** **Concurrency:** `concurrency.group: bench-${{ github.ref }}` + `cancel-in-progress: true` for non-default branches; main is never cancelled. Prevents PR pushes from queueing 18-cell matrix runs.
- **D-07:** **Caching:** GHA `actions/cache@v4` for `~/.cargo/registry`, `~/.cargo/git`, and the cargo-chef recipe layer (`target/`). Docker BuildKit cache (`type=gha,mode=max`) for layer-level cache across runs. Plan-phase tunes cache keys.

### Dive image-size gate
- **D-08:** **Per-image dive check** runs immediately after every `just build {env} {alloc}` in the matrix job. The gate uses `just dive-check {env} {alloc}` (Phase 3 D-14 already wires this) which executes `dive --ci alloc-bench:{alloc}-{env} --ci-config .dive-ci`. Non-zero exit fails the matrix-cell job; failure message in the job log names the offending image and layer.
- **D-09:** **`.dive-ci` config from Phase 3 reused as-is.** No edits. Thresholds are: `lowestEfficiency: 0.95`, `highestUserWastedPercent: 0.05`, `highestWastedBytes: 50MB` (from Phase 3 D-21).
- **D-10:** **Tightened image-size budgets** documented in Phase 3 D-22 (`scratch ≤ 15MB`, `distroless-static ≤ 25MB`, `alpine ≤ 30MB`, `wolfi ≤ 35MB`, `distroless-cc ≤ 50MB`, `debian-slim ≤ 100MB`) are NOT enforced by dive — dive enforces efficiency/waste, not absolute size. Plan-phase decides whether to add a separate `du -sh` gate; recommended yes for visibility, no for blocking — surface as a CI summary annotation.

### Multi-run aggregation (aggregator extension)
- **D-11:** **Statistics computed** when ≥3 runs share `(alloc, env, scenario)` tuple: median (preferred over mean — robust to outliers), min, max, coefficient-of-variation (`stddev / mean × 100%`). Stored in REPORT.md per-scenario tables as `{median} (min..max, CV {N}%)` for the throughput column; latency columns use median across the 3 runs. The HTML dashboard also gains `RESULTS_GROUPED` (a derived `{alloc, env, scenario} → {median, min, max, cv_pct, suspect, runs: [Run]}` view) for chart rendering.
- **D-12:** **High-variance flag: CV > 10%.** Cells with CV > 10% are marked `⚠ high variance` in REPORT.md (italic note appended to the throughput cell, same vocabulary as Phase 4 suspect notes) and in HTML chart legends with the same ⚠ glyph contract from Phase 4. Threshold is conservative — 10% is a healthy benchmark variance ceiling per industry conventions; tighter would be noisier than useful in CI runs that share runner hardware.
- **D-13:** **`image_size_mb` field** in the v1 `Env` block is now populated when results.json is produced inside CI. The CI workflow runs `docker inspect alloc-bench:{alloc}-{env} --format '{{.Size}}'` after build and injects the value via env var `BENCH_IMAGE_SIZE_BYTES` consumed by the bench-runner at startup (or, simpler, by a small post-processing step that mutates the JSON before upload). Plan-phase chooses; recommended: post-processing via `jq` so the bench binary stays unchanged. Phase 4 D-10 em-dash now becomes a real value when CI is the source.
- **D-14:** **Schema additions (additive only):** `metrics` block gains optional fields `image_size_mb` (already in `env` per Phase 1 D-12 — confirm), `build_time_s`, and an `aggregator-only` derived block under a top-level optional key like `multi_run_stats` that the aggregator emits when emitting REPORT.md. The locked v1 input schema is NOT modified — these are aggregator-output decorations, not bench-runner outputs.

### README walkthrough
- **D-15:** **Final README structure (top-down):**
  1. CI status badge + project title + 1-line tagline
  2. Hero paragraph (3-4 sentences: what, why, link to report).
  3. `## How memory allocation works on Linux` — Phase 4 added; preserve verbatim.
  4. `## Run it yourself` — NEW (this phase). 5-step recipe.
  5. `## Allocator matrix overview` — NEW. Mermaid table or static list of all 6 allocator combos × 6 envs.
  6. `## Reproducibility` — NEW. rustc pinning, Docker SHA pinning, hardware notes, link to research/PITFALLS.md.
  7. `## License` — single line, MIT or Apache (pick at plan-phase based on CLAUDE.md crate table — both Apache and MIT are dual-licensed in transitive deps; recommend dual `Apache-2.0 OR MIT`).
- **D-16:** **"Run it yourself" 5-step recipe:**
  1. Install Docker Desktop (macOS: `brew install --cask docker` or colima) + just (`brew install just`).
  2. `git clone https://github.com/{owner}/{repo}.git && cd {repo}` (placeholder until repo is public).
  3. **Smoke run (~10 min):** `just bench-all-smoke` (per Phase 3 D-13).  **Full run (~2.5h, ~5GB disk):** `just bench-all`. (Document the trade-off explicitly.)
  4. `just aggregate`.
  5. `open report/index.html` (macOS) / `xdg-open report/index.html` (Linux).
  Include a `### Troubleshooting` block: Apple Silicon (`--platform linux/amd64` already in Phase 3 build script), hyperthreading (`--cpus=4 --cpuset-cpus=0-3` already locked Phase 3 D-15), NUMA (Phase 3 D-16), low memory ("if mimalloc OOM-kills, raise to `BENCH_MEMORY=8g`").
- **D-17:** **`## Reproducibility` section content:**
  - rustc version pinned via Phase 3 D-06 `RUST_VERSION=1.83` Dockerfile ARG; the bench binary records `rustc_version` per REPR-02.
  - Docker base images pinned per Phase 3 D-05 (alpine:3.20, debian:bookworm-slim, distroless tags, wolfi-base).
  - `target-cpu=x86-64-v3` build flag (Phase 3 D-09) — CI runners and Docker images use the same flag for cross-runner consistency.
  - Hardware notes: GHA ubuntu-24.04 runners use 4 vCPU / 16 GB RAM (free tier as of 2026-05); local Apple Silicon dev box yields different absolute numbers but stable relative ordering.
  - Link to `.planning/research/PITFALLS.md` for the "things that bias allocator benchmarks" list.
- **D-18:** **CI status badge:** `[![CI](https://github.com/{owner}/{repo}/actions/workflows/bench.yml/badge.svg)](https://github.com/{owner}/{repo}/actions/workflows/bench.yml)` at the top of README.md, before the project title. Use `{owner}/{repo}` as a placeholder; user fills in post-fork or post-publish. Phase 5 does NOT auto-detect the GitHub remote — a placeholder is more honest than a guess.

### CI bench duration
- **D-19:** **CI matrix uses Phase 3's "smoke" recipe** (`just bench-all-smoke` per cell — `--warmup 1s --duration 5s` per scenario). Reasoning: 18 cells × 11 scenarios × 6s × 3 runs ≈ 60 min wall-clock at full parallelism (assuming 6 GHA runners). The full `--warmup 5s --duration 60s` recipe would be ~10× longer (10h) — not feasible for push-driven CI. Statistical-quality runs (the "real benchmark") happen on the local user's machine via `just bench-all`. CI proves the pipeline works and catches regressions in relative ordering, not absolute numbers.
- **D-20:** **Schema-version contract preserved:** all CI-emitted results.json files use `schema_version: 1`. The aggregator-derived `multi_run_stats` block does NOT change the input schema — it's a post-processing decoration only.

### Documentation polish
- **D-21:** **CONTRIBUTING.md** — out of scope for v1. Phase 5 does not add a CONTRIBUTING.md; the README walkthrough is sufficient for a benchmark repo (no PR contributions expected from the public).
- **D-22:** **LICENSE file** — committed at repo root (`LICENSE-MIT` + `LICENSE-APACHE` for the dual-license recommended in D-15). Phase 5 ships these. Plan-phase picks license text content (canonical SPDX templates).

### Claude's Discretion
- The exact GHA cache key strategy (per-Cargo.lock hash vs per-branch). Plan-phase picks based on test runs; recommended: per `Cargo.lock` hash for cargo, per `Dockerfile` hash for image layers.
- Whether to publish the report/ artifact to GitHub Pages on every successful main-branch push. Recommended: skip for v1 (artifact upload + retention is sufficient); revisit once the project goes public.
- Whether `just bench-all` smoke includes the `dce-check` gate. Phase 2 keeps `dce-check` as a separate `just dce-check` recipe; Phase 5 wires it as a `pre-bench` step in the GHA workflow that runs once (not per-cell).
- Whether the high-variance flag also blocks the build. Recommended: warn-only — high variance is informational, not a regression signal in v1.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase context
- `.planning/PROJECT.md` — overall project context, locked decisions
- `.planning/REQUIREMENTS.md` — Phase 5 requirements: ORCH-04, ORCH-05, REPR-01, REPR-03 (4 reqs)
- `.planning/ROADMAP.md` §"Phase 5: CI, Image-Size Gate & Public Polish" — phase goal + 4 success criteria
- `.planning/phases/03-docker-matrix-local-orchestration/03-CONTEXT.md` — `.dive-ci` config (D-21), 18-cell matrix (D-01), image-size budgets (D-22), build flags (D-09), bench-all-smoke (D-13)
- `.planning/phases/04-aggregator-dashboard/04-CONTEXT.md` — aggregator schema, image_size_mb em-dash deferral (D-10)
- `.planning/phases/04-aggregator-dashboard/04-VERIFICATION.md` — verifies aggregator works against fixtures; CI extends it to real cell outputs

### Research outputs (MANDATORY reading for plan-phase)
- `.planning/research/SUMMARY.md` — synthesis
- `.planning/research/STACK.md` §12 — GitHub Actions matrix CI
- `.planning/research/PITFALLS.md` §1.4 — sample-count floor (CI smoke is below this; document why CI is "shape-of-curve" only)
- `.planning/research/PITFALLS.md` §3.3 — `target-cpu=x86-64-v3` portability across CI runners
- `.planning/research/PITFALLS.md` §4.3 — multi-run median + range (REPR-03 motivation)
- `.planning/research/PITFALLS.md` §5.4 — pin rustc version (already locked Phase 3)

### External specifications
- https://docs.github.com/en/actions/using-jobs/using-a-matrix-for-your-jobs — GHA matrix syntax
- https://github.com/actions/upload-artifact — v4 API
- https://github.com/actions/download-artifact — v4 API
- https://github.com/wagoodman/dive — dive --ci docs
- https://github.com/casey/just — just installation
- https://docs.github.com/en/actions/using-workflows/caching-dependencies-to-speed-up-workflows — actions/cache@v4

### Out-of-scope (Phase 5)
- aarch64 / Apple Silicon CI matrix → v2 (V2-09)
- snmalloc / tcmalloc / rpmalloc allocators → v2 (V2-01..03)
- Marimo notebook output → v2 (V2-07)
- Long-term result archival (S3, regression detection) → v2 (V2-08)
- CONTRIBUTING.md → not planned for v1
- GitHub Pages publishing → revisit when repo goes public

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets (from Phases 1 + 2 + 3 + 4)
- `justfile` — already has 11 recipes (`dce-check`, `run-all-smoke`, `build`, `run`, `bench-cell`, `bench-all`, `bench-all-smoke`, `bench-host`, `clean-images`, `dive-check`, `dive-check-all`, `aggregate`, `aggregate-smoke`). Phase 5 likely adds: `ci-bench-cell`, `ci-aggregate`, plus a single-pass `ci-validate` that runs dce-check + clippy + fmt as a "pre-bench" sanity check.
- `.dive-ci` — Phase 3 config; reused as-is.
- `.dockerignore` — Phase 3 added.
- `crates/alloc-bench-aggregator/` — the aggregator binary; Phase 5 extends `markdown.rs` + `recommend.rs` + the HTML template with multi-run statistics. The CV% computation is a small new module (`crates/alloc-bench-aggregator/src/multi_run.rs`).
- `crates/alloc-bench-core::output` — locked v1 schema; Phase 5 does NOT modify the input contract. Aggregator-emitted derived statistics live in REPORT.md and the dashboard, not in results.json.
- `prek.toml` pre-commit hooks — `cargo fmt` + `cargo clippy --all-targets`. CI mirrors these as a separate `just ci-validate` (or inline in the GHA workflow).

### Established Patterns (from Phases 1 + 2 + 3 + 4)
- Conventional-commit prefixes: Phase 5 uses `feat(05)`, `chore(05)`, `docs(05)`, `test(05)`, `ci(05)`.
- Workspace deps in root `Cargo.toml`. Phase 5 may add no new Rust deps — it's mostly YAML + markdown + small Rust additions to existing aggregator code.
- `commit_docs = true` — `.planning/` artifacts are committed.
- `cargo fmt --all --check` + `cargo clippy --workspace --all-targets -- -D warnings` before each commit.
- Validated configs (Phase 2) — multi_run statistics computation gets a `validated()`-style assertion path: reject NaN/inf throughput, reject negative `cv_pct` etc.

### Integration Points
- **GHA workflow** runs every existing recipe inside the matrix job. The recipes Phase 5 ADDS are CI conveniences:
  - `just ci-bench-cell {env} {alloc}` — wraps `just build + just dive-check + 3× just bench-cell with seeds`.
  - `just ci-aggregate` — wraps `cargo run -p alloc-bench-aggregator -- --input "results/**/*.json" --output report/` with the post-processing step that reads `image_size_mb` from each cell's `meta.json` (a tiny sidecar file CI emits per cell with image-size + build-time output).
- **Aggregator extension:** `recommend.rs` and `markdown.rs` learn the multi-run shape. New module `multi_run.rs` with `pub fn aggregate(runs: &[Run]) -> MultiRunStats`.
- **HTML dashboard:** `templates/index.html.tmpl` gains an "errorbar" rendering on the throughput chart (median + min/max range) when CI-aggregated data is loaded.
- **README.md:** Phase 4 inserted the system diagram. Phase 5 prepends the badge and appends the walkthrough + matrix overview + Reproducibility sections. The "How memory allocation works" section is preserved verbatim.

### New non-Cargo files added in Phase 5
- `.github/workflows/bench.yml` (the matrix workflow)
- `.github/workflows/ci-validate.yml` (or fold into bench.yml — plan-phase decides; standalone is more readable)
- `crates/alloc-bench-aggregator/src/multi_run.rs` (NEW — median, min, max, CV%)
- `crates/alloc-bench-aggregator/tests/fixtures/multi_run/seed-{1,2,3}.json` (3 fixtures for the multi-run unit tests)
- `LICENSE-MIT` (text file)
- `LICENSE-APACHE` (text file)
- `README.md` — extended (Phase 4 + Phase 5 additions)
- `justfile` — extended with `ci-bench-cell` + `ci-aggregate`

### Files that MUST NOT change
- v1 input schema in `crates/alloc-bench-core/src/output.rs` — locked Phase 1 D-11. CI-emitted JSON still uses schema_version=1.
- `.dive-ci` thresholds (Phase 3 D-21).
- The 4 Phase-4 chart types (HTML), Mermaid diagrams (REPORT.md), and the locked README "How memory allocation works" paragraph.

</code_context>

<specifics>
## Specific Ideas

- **CI duration target:** ~60 min p95 wall-clock for the full 18-cell × 11-scenario × 3-run matrix on `ubuntu-24.04`. Achieved by parallel matrix-cell jobs and the smoke recipe (`bench-all-smoke`) per cell. Document this number in `## Reproducibility`.
- **Seed convention:** `--seed 1`, `--seed 2`, `--seed 3` for the three CI runs per cell. Seed 1 = canonical, seeds 2 and 3 are perturbations. Phase 1 D-19 default is `0xDEADBEEF`; CI overrides via the existing `--seed` CLI flag.
- **Per-cell artifact name:** `results-{alloc}-{env}` so `actions/download-artifact@v4` can wildcard-glob them all.
- **Final aggregator artifact name:** `bench-report-{run_id}` (where run_id is the GHA run number) — easy to find in the run's Artifacts tab.
- **Image-size sidecar:** during the CI build step, run `docker inspect alloc-bench:{alloc}-{env} --format='{{.Size}}'` and write to `results/{alloc}-{env}-meta.json`. Aggregator merges meta + per-run JSON via the `(alloc, env)` join key. This avoids modifying the v1 schema struct.
- **CV%-vs-suspect interaction:** a run can be both suspect (low samples / short warmup) AND high-variance (across 3 runs). REPORT.md and HTML show both badges concatenated: `*(⚠ suspect: low samples; ⚠ high variance: CV 14%)*`.
- **High-variance threshold rationale (10%):** CI runners share hardware with neighbors → some warmth bleed-through. 10% CV on throughput is the documented "healthy" upper bound for synthetic micro-benchmarks per `criterion`'s default rejection threshold. Tighter would noise out reliable signal; looser would let real regressions slip.
- **rustc pin convention:** `rust-toolchain.toml` at repo root (already a Rust idiom). Phase 5 ADDS this file with `channel = "1.83.0"`. Currently the build relies on the system-default rustc — reproducibility argues for pinning.
- **Cache-buster prevention:** GHA cache keys MUST include `${{ hashFiles('Cargo.lock') }}` so dep updates don't get false cache hits. Plan-phase confirms.

</specifics>

<deferred>
## Deferred Ideas

- **GitHub Pages publishing of report/index.html on main-branch pushes** → v2 (revisit when repo goes public)
- **CONTRIBUTING.md** → out of scope for v1 benchmark repo
- **Multi-platform CI (macOS, Windows runners)** → v2; Linux runners only for v1
- **aarch64 CI matrix** → v2 (V2-09)
- **snmalloc / tcmalloc / rpmalloc** → v2 (V2-01..03)
- **Long-term result archival to S3** → v2 (V2-08)
- **Continuous regression detection (cargo-criterion-style trend lines)** → v2 (V2-08)
- **Marimo notebook output** → v2 (V2-07)
- **Slack / GitHub deployments integration** for benchmark notifications → v2
- **Tightening the dive thresholds** beyond Phase 3 defaults → revisit if any cell legitimately fails the existing thresholds
- **Absolute image-size budget enforcement** (Phase 3 D-22 numbers) → v2 (informational only in v1)
- **Self-hosted runners for stable benchmark hardware** → v2 (GHA runners shared-CPU is acceptable for "shape-of-curve" v1)
- **Tightening the high-variance CV threshold below 10%** → revisit after observing 1 month of CI variance
- **Phase 4 D-10 image_size_mb in the v1 schema** → CI populates it via sidecar (not schema change); v2 might add it to the Env block proper

</deferred>

---

*Phase: 5-CI, Image-Size Gate & Public Polish*
*Context gathered: 2026-05-19*

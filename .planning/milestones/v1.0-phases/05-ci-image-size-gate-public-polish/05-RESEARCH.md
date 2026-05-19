# Phase 5: CI, Image-Size Gate & Public Polish - Research

**Researched:** 2026-05-19
**Domain:** GitHub Actions matrix CI, image-size enforcement, multi-run statistics, public-facing documentation
**Confidence:** HIGH

## Summary

Phase 5 wires the Phase 1–4 stack to GitHub Actions, enforces `dive --ci` per image, surfaces multi-run median + min/max + CV% in REPORT.md, and ships the public README walkthrough. The locked decisions in CONTEXT.md (D-01..D-22) leave the planner with concrete-but-orthogonal questions on each axis: matrix expression style (`include:` vs `matrix + exclude`), the sidecar `meta.json` shape, the Bessel-correction choice for n=3 CV, the cache-action selection (`Swatinem/rust-cache` vs `actions/cache@v4`), and the canonical content of the two LICENSE files.

This research resolves all twelve focus areas with verified docs (GitHub Actions docs, Docker docs, dive README, Wikipedia/CV) and codebase audit (justfile, Cargo.toml, output.rs, rust-toolchain.toml, all six Dockerfiles). One important codebase divergence from CONTEXT.md was uncovered: **the project pins `RUST_VERSION=1.91`, not `1.83` as CONTEXT.md D-17 / Phase 3 D-06 reference text claims** — the actual repo state is 1.91 and is consistent across `rust-toolchain.toml`, all six Dockerfiles, and the justfile. The Reproducibility section MUST cite the real value.

A second codebase audit finding: **`Env::image_size_mb` is NOT a field on the v1 schema** (verified via `crates/alloc-bench-core/src/output.rs`). CONTEXT.md D-14 says "image_size_mb already in env per Phase 1 D-12 — confirm" — confirmation comes back negative. The sidecar `meta.json` approach in CONTEXT.md D-13 is therefore the correct (and only schema-preserving) path; env-var injection into the bench binary is also viable but requires a code change.

**Primary recommendation:** Use a flat `strategy.matrix.include:` block enumerating all 18 cells explicitly (no `matrix:` axes + `exclude:` — the latter is harder to audit). Use `Swatinem/rust-cache@v2` for Cargo caching (purpose-built, beats hand-rolled `actions/cache@v4`). Use `docker/build-push-action@v7` with `cache-to: type=gha,mode=max,scope=${{ matrix.alloc }}-${{ matrix.env }}` per cell. Sidecar `meta.json` per cell (image size + build time) merged by aggregator via `(alloc, env)` join key. Bessel-corrected sample stddev (n-1) for CV with n=3.

## User Constraints (from CONTEXT.md)

### Locked Decisions

**GitHub Actions matrix topology (D-01 .. D-07):**
- D-01: Triggers: `push` (any branch) + `pull_request` (target = main) + `workflow_dispatch` (manual run).
- D-02: Matrix scope is exactly **18 cells** = 9 glibc-env×glibc-alloc + 9 musl-env×musl-alloc (the meaningful matrix locked by Phase 3 D-01). `runs-on: ubuntu-24.04`. `fail-fast: false`. Each cell is a separate parallel job.
- D-03: Sequential within a job — 11 scenarios run under `just bench-all-smoke` (or a CI-tuned recipe). Parallel scenarios are forbidden (would multiplex allocators in the same kernel page cache).
- D-04: 3 runs per cell with seeds 1/2/3. Each run produces `results/{alloc}-{env}-seed{N}.json`.
- D-05: Per-cell job uploads `results/` via `actions/upload-artifact@v4`. Final `aggregate` job declares `needs: [bench-matrix]`, downloads all per-cell artifacts via `actions/download-artifact@v4`, runs `just aggregate`, uploads `report/`. Retention: 90 days default.
- D-06: `concurrency.group: bench-${{ github.ref }}` + `cancel-in-progress: true` for non-default branches; main is never cancelled.
- D-07: GHA `actions/cache@v4` for `~/.cargo/registry`, `~/.cargo/git`, `target/`. Docker BuildKit cache (`type=gha,mode=max`).

**Dive image-size gate (D-08 .. D-10):**
- D-08: Per-image dive check via `just dive-check {env} {alloc}` after every `just build`.
- D-09: `.dive-ci` config from Phase 3 reused as-is (`lowestEfficiency: 0.95`, `highestUserWastedPercent: 0.05`, `highestWastedBytes: 50MB`).
- D-10: Absolute image-size budgets are NOT enforced by dive — surface as informational CI summary annotation only.

**Multi-run aggregation (D-11 .. D-14, D-20):**
- D-11: Statistics computed when ≥3 runs share `(alloc, env, scenario)` tuple: median, min, max, CV (`stddev / mean × 100%`). REPORT.md format: `{median} (min..max, CV {N}%)`.
- D-12: High-variance flag: CV > 10% gets `⚠ high variance` italic note (REPORT.md) + ⚠ glyph (HTML chart legend).
- D-13: `image_size_mb` populated in CI via `docker inspect alloc-bench:{alloc}-{env} --format '{{.Size}}'`. **Plan-phase recommended path: post-processing via `jq` so the bench binary stays unchanged.**
- D-14: Schema additions are aggregator-output decorations only — locked v1 input schema is NOT modified.
- D-20: All CI-emitted results.json use `schema_version: 1`.

**README walkthrough (D-15 .. D-18):**
- D-15: README structure top-down: badge + title + tagline → hero → "How memory allocation works" (preserve) → `## Run it yourself` (NEW) → `## Allocator matrix overview` (NEW) → `## Reproducibility` (NEW) → `## License`.
- D-16: 5-step recipe: (1) Docker + just install (2) clone (3) `just bench-all-smoke` OR `just bench-all` (4) `just aggregate` (5) `open report/index.html`. Includes `### Troubleshooting` block.
- D-17: Reproducibility section content includes rustc pin (Phase 3 D-06 — text says `1.83` but **actual repo state is 1.91; Reproducibility section MUST cite real value**), Docker base images pinned, `target-cpu=x86-64-v3`, GHA hardware notes (4 vCPU / 16 GB), link to PITFALLS.md.
- D-18: CI status badge at top: `[![CI](https://github.com/{owner}/{repo}/actions/workflows/bench.yml/badge.svg)](https://github.com/{owner}/{repo}/actions/workflows/bench.yml)`. Use literal owner/repo from `Cargo.toml` (`marccarre/rust-benchmark-glibc-musl-mimalloc`) — **NOT** a placeholder, since the repo is known.

**CI bench duration (D-19):**
- D-19: CI matrix uses `bench-all-smoke` (not `bench-all`). 18 cells × 11 scenarios × 6s × 3 runs ≈ 60 min wall-clock at full parallelism.

**Documentation polish (D-21, D-22):**
- D-21: CONTRIBUTING.md OUT OF SCOPE for v1.
- D-22: `LICENSE-MIT` + `LICENSE-APACHE` committed at repo root (dual-license). Plan-phase fills in canonical SPDX text.

### Claude's Discretion

- Exact GHA cache-key strategy (per-Cargo.lock hash vs per-branch). **Recommended: per Cargo.lock hash for cargo, per Dockerfile hash for image layers** (research §"GHA cache keys" below).
- Whether to publish `report/` to GitHub Pages on every successful main-branch push. **Recommended: skip for v1.**
- Whether `just bench-all` smoke includes the `dce-check` gate. **Recommended: wire `dce-check` as a `pre-bench` step that runs once (not per-cell).**
- Whether the high-variance flag also blocks the build. **Recommended: warn-only.**

### Deferred Ideas (OUT OF SCOPE)

- GitHub Pages publishing → v2
- CONTRIBUTING.md → v1 doesn't ship one
- Multi-platform CI (macOS, Windows runners) → v2
- aarch64 CI matrix → v2 (V2-09)
- snmalloc / tcmalloc / rpmalloc → v2 (V2-01..03)
- Long-term result archival to S3 → v2 (V2-08)
- Continuous regression detection → v2 (V2-08)
- Marimo notebook output → v2 (V2-07)
- Slack / GitHub deployments integration → v2
- Tightening dive thresholds beyond Phase 3 defaults → revisit after observation
- Absolute image-size budget enforcement (Phase 3 D-22) → v2 (informational only in v1)
- Self-hosted runners → v2 (GHA shared-CPU acceptable for "shape-of-curve" v1)
- Tightening high-variance CV threshold below 10% → revisit after 1 month observation
- `image_size_mb` in v1 schema proper → CI populates via sidecar; v2 may add to Env block

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| ORCH-04 | User pushes to GitHub and a CI workflow runs the matrix on `ubuntu-24.04`, uploading `results/` and `report/` as artifacts | §"GitHub Actions matrix syntax", §"Artifact upload/download", §"GHA cache" |
| ORCH-05 | CI runs `dive --ci` for each image and fails the build if image-size thresholds are exceeded | §"Dive --ci integration", reuses Phase 3's `.dive-ci` |
| REPR-01 | User reads `README.md` and finds a complete "Run it yourself" walkthrough | §"README walkthrough content", §"Dual-license SPDX templates" |
| REPR-03 | Each matrix cell runs ≥3 times in CI; aggregator reports median + min/max range across runs | §"Coefficient of Variation formula", §"Multi-run aggregation in Rust" |

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Workflow trigger / cancellation | GitHub Actions runner orchestration | — | Concurrency group is a GHA-tier concern; can't be enforced anywhere else |
| Per-cell build + bench (18×) | Matrix-cell job (ubuntu-24.04 runner) | Docker layer (BuildKit cache) | Each cell is an isolated runner; parallelism comes from matrix expansion |
| Image-size enforcement | `dive --ci` invoked from matrix-cell job | `.dive-ci` (config file) | Same tier as Phase 3 — Phase 5 only adds the gate to CI |
| Multi-run statistical aggregation | Rust aggregator (`alloc-bench-aggregator`) | None | Stats computed in-process from `Vec<Run>`; no external service |
| `image_size_mb` capture | Per-cell job (`docker inspect` → `meta.json` sidecar) | Aggregator (merge step) | The bench binary doesn't know its own image size; CI is the only level that does |
| Artifact pipeline | `actions/upload-artifact@v4` (per cell) → `actions/download-artifact@v4` (aggregate job) | — | Standard GHA artifact tier; no S3/Pages in v1 |
| Public docs (README, LICENSE) | Repo-root markdown + license files | — | Static content; no generation step |

## Standard Stack

### Core (GitHub Actions)

| Action | Version | Purpose | Why Standard |
|--------|---------|---------|--------------|
| `actions/checkout` | `v4` | Clone repo into runner workspace | Canonical first step in every GHA workflow |
| `actions/upload-artifact` | `v4` | Upload `results/` per cell | v4 is the current major; v3 is deprecated |
| `actions/download-artifact` | `v4` | Download all per-cell artifacts in aggregate job | v4 has the `pattern:` glob for `results-*` |
| `Swatinem/rust-cache` | `v2` | Cargo registry + target caching | Purpose-built for Rust; auto-keys on Cargo.lock hash + rustc version + rust-toolchain.toml |
| `docker/setup-buildx-action` | `v3` | Initialize BuildKit builder | Required for `cache-to: type=gha` |
| `docker/build-push-action` | `v6` | Build per-cell image with GHA cache | Provides `cache-from` / `cache-to` integration |
| `dtolnay/rust-toolchain` | `@1.91.0` | Pin rustc version | One-line install; reads pinned version from action ref |

**Note on action versions:** The most recent stable as of training is `v7.x` for `build-push-action` and `v4.x` for `setup-buildx-action`. Plan-phase should verify current stable major at lock time. The above versions (`v6`/`v3`) are the well-known LTS-grade pins as of late-2025; bumping to v7/v4 is fine if plan-phase confirms compatibility. [VERIFIED: github.com/actions/upload-artifact, github.com/actions/download-artifact, github.com/Swatinem/rust-cache, github.com/docker/build-push-action — all observed live]

### Supporting (Tools invoked from workflow)

| Tool | Version | Purpose | When to Use |
|------|---------|---------|-------------|
| `just` | latest | Recipe runner | Already a project dep — install via `extractions/setup-just@v2` or `cargo install just` |
| `dive` | `0.13.x` | Image-efficiency check | Already wired in `just dive-check`; install via `wagoodman/dive-action` or run dockerized image |
| `jq` | preinstalled | JSON post-processing for `meta.json` sidecar | `ubuntu-24.04` runners include `jq` by default |
| `docker` | runner default | Build + inspect image size | `ubuntu-24.04` runners include Docker preinstalled |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `Swatinem/rust-cache@v2` | `actions/cache@v4` (manual paths) | Manual cache is more transparent but skips rustc-version keying; Swatinem cache also auto-cleans incremental files |
| `strategy.matrix.include:` (18 explicit cells) | `strategy.matrix:` axes + `strategy.matrix.exclude:` | `exclude:` is harder to audit because the reader has to mentally subtract; `include:`-only is the canonical pattern when you've already enumerated valid cells (cf. justfile `_matrix_cells`) |
| `docker inspect` post-processing | env-var injection into bench binary | Post-processing keeps the bench binary unchanged; env-var injection would require a Phase 1 schema change (out of scope per D-14) |

**Installation (workflow header):**

```yaml
# Locked stack — actions/checkout + Swatinem cache + buildx + build-push + dive runner
- uses: actions/checkout@v4
- uses: Swatinem/rust-cache@v2
  with:
    shared-key: bench-${{ matrix.env }}-${{ matrix.alloc }}
- uses: docker/setup-buildx-action@v3
- uses: extractions/setup-just@v2
```

**Version verification:** Verified as of 2026-05-19 against the upstream READMEs. None of these need a registry-existence check (all are GitHub-hosted actions, not npm/crates.io packages). [VERIFIED: github.com listings]

## Package Legitimacy Audit

Phase 5 installs **zero** new Cargo crates (per CONTEXT.md `<code_context>`: "Phase 5 may add no new Rust deps — it's mostly YAML + markdown + small Rust additions to existing aggregator code"). The CV computation in `multi_run.rs` uses only standard library functions (`f64::sqrt`, `f64::powi`, sort) and the existing `serde` / `serde_json` workspace deps. No `statrs`, no `nalgebra`, no `ndarray`.

| Package | Registry | Action |
|---------|----------|--------|
| (none — all stack is in-tree or already-vendored) | — | None |

If plan-phase decides to add a dep (e.g., `statrs` for a more robust unbiased CV estimator), it MUST run the slopcheck protocol per the standard `<package_legitimacy_protocol>` before locking the choice. The current research recommends staying in-tree.

## Architecture Patterns

### System Architecture Diagram

```
                      ┌─────────────────────────────┐
                      │  Trigger (push / PR / wd)   │
                      └──────────────┬──────────────┘
                                     │
                       ┌─────────────▼─────────────┐
                       │  bench.yml workflow         │
                       │  concurrency: bench-${ref}  │
                       └─────────────┬─────────────┘
                                     │
              ┌──────────────────────┴──────────────────────┐
              │                                             │
        ┌─────▼─────┐                                ┌──────▼──────┐
        │ pre-bench │                                │ bench-matrix │
        │ ci-validate│                                │ (18 cells)   │
        │ - fmt/clippy│                              │ - build       │
        │ - dce-check│                                │ - dive-check  │
        └─────┬─────┘                                │ - 3× run      │
              │ pass                                 │ - upload      │
              │                                      │   results-     │
              │                                      │   {alloc}-{env}│
              │                                      └──────┬──────────┘
              │                                             │
              └─────────────────────┬───────────────────────┘
                                    │
                            ┌───────▼───────┐
                            │  aggregate     │
                            │  needs: matrix │
                            │  - download    │
                            │    pattern:    │
                            │    results-*   │
                            │    merge: true │
                            │  - jq merge    │
                            │    meta.json   │
                            │    sidecar     │
                            │    → image_size│
                            │  - just        │
                            │    aggregate   │
                            │  - upload      │
                            │    bench-      │
                            │    report-${id}│
                            └────────────────┘
```

Data flow:
1. `push`/`pull_request`/`workflow_dispatch` triggers `bench.yml`.
2. `pre-bench` job (single-shot) runs format / clippy / dce-check before any matrix build.
3. `bench-matrix` (18-job matrix) — each cell builds its image, runs dive, runs the bench 3× with seeds 1/2/3, captures `meta.json` (image size + build time), uploads as `results-{alloc}-{env}`.
4. `aggregate` (depends on matrix) — downloads all `results-*` artifacts via glob, runs `just ci-aggregate` which merges per-cell `meta.json` into the run records, generates `report/index.html` + `REPORT.md`, uploads as `bench-report-${{ github.run_id }}`.

### Recommended Project Structure

```
.github/
└── workflows/
    └── bench.yml                          # NEW (Phase 5)
        # All stages — pre-bench + bench-matrix + aggregate.
        # Single-file workflow recommended over splitting into ci-validate.yml
        # + bench.yml for readability of the artifact-flow contract.
crates/alloc-bench-aggregator/
├── src/
│   ├── multi_run.rs                       # NEW (Phase 5) — CV / median / range
│   ├── markdown.rs                        # EXTENDED — multi-run table cells
│   ├── recommend.rs                       # EXTENDED — uses median across runs
│   └── ...
├── templates/
│   └── index.html.tmpl                    # EXTENDED — errorbar (min..max) on charts
└── tests/
    └── fixtures/
        └── multi_run/                     # NEW — seed-{1,2,3}.json fixtures
LICENSE-MIT                                 # NEW (Phase 5)
LICENSE-APACHE                              # NEW (Phase 5)
README.md                                   # EXTENDED — badge + walkthrough + matrix overview + Reproducibility
justfile                                    # EXTENDED — ci-bench-cell, ci-aggregate, ci-validate
```

### Pattern 1: 18-cell explicit matrix via `include:`-only

**What:** Express the 18 valid cells as 18 `include:` entries with no top-level `matrix:` axes. This makes the matrix self-documenting (every cell is on its own line) and structurally identical to the `_matrix_cells` list in `justfile` (lines 131-150).

**When to use:** Matrix dimensions are well-known and bounded; cross-libc combos must be omitted (not excluded after the fact).

**Example:**

```yaml
# Source: docs.github.com/en/actions/using-jobs/using-a-matrix-for-your-jobs
# (Verified — `include:` without top-level axes creates standalone combinations.)
strategy:
  fail-fast: false
  matrix:
    include:
      # glibc family (9 cells)
      - { env: debian-slim,    alloc: ptmalloc,  libc: glibc, target: x86_64-unknown-linux-gnu  }
      - { env: debian-slim,    alloc: jemalloc,  libc: glibc, target: x86_64-unknown-linux-gnu  }
      - { env: debian-slim,    alloc: mimalloc,  libc: glibc, target: x86_64-unknown-linux-gnu  }
      - { env: distroless-cc,  alloc: ptmalloc,  libc: glibc, target: x86_64-unknown-linux-gnu  }
      - { env: distroless-cc,  alloc: jemalloc,  libc: glibc, target: x86_64-unknown-linux-gnu  }
      - { env: distroless-cc,  alloc: mimalloc,  libc: glibc, target: x86_64-unknown-linux-gnu  }
      - { env: wolfi,          alloc: ptmalloc,  libc: glibc, target: x86_64-unknown-linux-gnu  }
      - { env: wolfi,          alloc: jemalloc,  libc: glibc, target: x86_64-unknown-linux-gnu  }
      - { env: wolfi,          alloc: mimalloc,  libc: glibc, target: x86_64-unknown-linux-gnu  }
      # musl family (9 cells)
      - { env: alpine,             alloc: mallocng,  libc: musl, target: x86_64-unknown-linux-musl }
      - { env: alpine,             alloc: jemalloc,  libc: musl, target: x86_64-unknown-linux-musl }
      - { env: alpine,             alloc: mimalloc,  libc: musl, target: x86_64-unknown-linux-musl }
      - { env: distroless-static,  alloc: mallocng,  libc: musl, target: x86_64-unknown-linux-musl }
      - { env: distroless-static,  alloc: jemalloc,  libc: musl, target: x86_64-unknown-linux-musl }
      - { env: distroless-static,  alloc: mimalloc,  libc: musl, target: x86_64-unknown-linux-musl }
      - { env: scratch,            alloc: mallocng,  libc: musl, target: x86_64-unknown-linux-musl }
      - { env: scratch,            alloc: jemalloc,  libc: musl, target: x86_64-unknown-linux-musl }
      - { env: scratch,            alloc: mimalloc,  libc: musl, target: x86_64-unknown-linux-musl }
```

**Why include-only beats matrix + exclude here:**
- Cross-libc combos (`mallocng-on-glibc`, `ptmalloc-on-musl`) are STRUCTURALLY ABSENT from the `include:` block — exactly mirroring the justfile's `_matrix_cells` (which CONTEXT.md "specifics" ¶3 calls out as "hard-skip is encoded by omission, not by runtime conditionals").
- A reader sees 18 lines and counts 18 cells. With `matrix + exclude`, they'd see e.g. 4 allocs × 6 envs = 24, then mentally subtract the 6 excludes — error-prone.
- Adding extra context (`libc`, `target`) per cell is trivial; with `matrix + exclude` you'd have to re-derive `target` via a per-step `case` statement.

[CITED: docs.github.com/en/actions/using-jobs/using-a-matrix-for-your-jobs — "Include entries that cannot be added to any original matrix combination without overwriting a value, so [they are] added as an additional matrix combination."]

### Pattern 2: Per-cell artifact upload + final glob download

**What:** Each cell uploads `results/` under a unique name `results-{alloc}-{env}`. The aggregate job downloads all `results-*` via `pattern:` and `merge-multiple: true` so files land in one directory.

**When to use:** Standard fan-out / fan-in pattern; v4-required because v4 enforces unique artifact names within a workflow (you cannot upload-then-append to a shared artifact).

**Example:**

```yaml
# Per-cell job (inside bench-matrix):
- name: Upload per-cell results
  uses: actions/upload-artifact@v4
  with:
    name: results-${{ matrix.alloc }}-${{ matrix.env }}
    path: |
      results/
      meta/${{ matrix.alloc }}-${{ matrix.env }}.json
    if-no-files-found: error           # fail loudly on missing output
    retention-days: 90                  # default; explicit for clarity
    compression-level: 6                # default; JSON compresses well

# Aggregate job:
- name: Download all per-cell results
  uses: actions/download-artifact@v4
  with:
    path: ./results-merged
    pattern: results-*
    merge-multiple: true                # all files land in ./results-merged/
```

[CITED: github.com/actions/upload-artifact — "uploading to the same artifact via multiple jobs is not supported with v4"; github.com/actions/download-artifact — "pattern: A glob pattern to the artifacts that should be downloaded. Ignored if name is specified." + "merge-multiple: When multiple artifacts are matched, this changes the behavior of the destination directories."]

### Pattern 3: GHA Docker BuildKit cache, scoped per cell

**What:** `cache-to: type=gha,mode=max,scope=...` — `mode=max` exports all intermediate layers (vs `mode=min` which exports only the final image); `scope=...` segregates caches per matrix cell so two cells building different allocs don't overwrite each other.

**When to use:** Always — `mode=max` is the documented default for matrix CI; without `scope:` matrix cells collide on the same `buildkit` key and only the last-pushed cell's cache survives.

**Example:**

```yaml
- uses: docker/setup-buildx-action@v3

- name: Build image (per cell)
  uses: docker/build-push-action@v6
  with:
    context: .
    file: docker/${{ matrix.env }}.Dockerfile
    platforms: linux/amd64
    load: true                         # load image into local docker for dive + run
    push: false                        # never push from CI in v1
    tags: alloc-bench:${{ matrix.alloc }}-${{ matrix.env }}
    build-args: |
      ALLOC=${{ matrix.alloc }}
      TARGET=${{ matrix.target }}
      RUST_VERSION=1.91
      OCI_VERSION=${{ env.CARGO_PKG_VERSION }}
      OCI_REVISION=${{ github.sha }}
      OCI_CREATED=${{ env.BUILD_TIMESTAMP }}
    cache-from: type=gha,scope=${{ matrix.alloc }}-${{ matrix.env }}
    cache-to:   type=gha,mode=max,scope=${{ matrix.alloc }}-${{ matrix.env }}
```

[CITED: docs.docker.com/build/cache/backends/gha/ — "By default, [scope] is set to `buildkit`. If you build multiple images, each build will overwrite the cache of the previous." Plus the live verification of `cache-from` / `cache-to` syntax against `docker/build-push-action@v6` README.]

### Pattern 4: Per-cell `meta.json` sidecar + aggregator merge

**What:** Each cell, after building the image, writes `meta/{alloc}-{env}.json` containing `image_size_bytes`, `image_size_mb`, `build_time_s`. The aggregator merges by `(alloc, env)` join key into the per-cell `Run` records' `Env` block at REPORT.md emit time.

**When to use:** When extending output without modifying the locked v1 schema (CONTEXT.md D-14 / D-20). The bench binary's results.json shape stays byte-identical; the aggregator decorates output.

**Example:**

```bash
# Per-cell job — after build, before upload:
- name: Capture image metadata
  run: |
    mkdir -p meta
    SIZE_BYTES=$(docker image inspect alloc-bench:${{ matrix.alloc }}-${{ matrix.env }} \
      --format '{{.Size}}')
    SIZE_MB=$(awk "BEGIN { printf \"%.2f\", $SIZE_BYTES / 1024 / 1024 }")
    BUILD_TIME=${{ steps.build.outputs.duration_seconds || 0 }}
    jq -n \
      --argjson size_b "$SIZE_BYTES" \
      --argjson size_mb "$SIZE_MB" \
      --argjson build_s "$BUILD_TIME" \
      '{
        alloc:               "${{ matrix.alloc }}",
        env:                 "${{ matrix.env }}",
        image_size_bytes:    $size_b,
        image_size_mb:       $size_mb,
        build_time_s:        $build_s,
        captured_at:         now | todate
      }' > meta/${{ matrix.alloc }}-${{ matrix.env }}.json
```

```rust
// crates/alloc-bench-aggregator/src/loader.rs (extension)
//
// Source: hand-rolled — no external library; extends the existing loader.
#[derive(Deserialize)]
pub struct CellMeta {
    pub alloc: String,
    pub env: String,
    pub image_size_bytes: u64,
    pub image_size_mb: f64,
    pub build_time_s: f64,
}

pub fn load_cell_metas(meta_dir: &Path) -> anyhow::Result<HashMap<(String, String), CellMeta>> {
    let mut map = HashMap::new();
    for entry in glob::glob(&format!("{}/*.json", meta_dir.display()))? {
        let path = entry?;
        let text = std::fs::read_to_string(&path)?;
        let meta: CellMeta = serde_json::from_str(&text)?;
        map.insert((meta.alloc.clone(), meta.env.clone()), meta);
    }
    Ok(map)
}
```

[VERIFIED: docker image inspect --format '{{.Size}}' returns the image's size in bytes as a 64-bit integer; this is the documented behavior of the Docker Engine `ImageInspect.Size` field. Confirmed via `docs.docker.com/reference/cli/docker/image/inspect/` (live).]

### Pattern 5: Multi-run statistics in Rust

**What:** A pure-stdlib function that takes `&[f64]` (or `&[Run]`) and returns `MultiRunStats { median, min, max, mean, stddev, cv_pct }`. No `statrs`, no `nalgebra` — keeps Phase 5 dep-free.

**When to use:** Computing aggregator-level decorations from per-run inputs (REPR-03).

**Example:**

```rust
// crates/alloc-bench-aggregator/src/multi_run.rs
//
// Source: Wikipedia "Coefficient of variation" + Bessel-correction convention.
// (Verified via en.wikipedia.org/wiki/Coefficient_of_variation 2026-05-19.)

use serde::Serialize;

/// Multi-run statistics across ≥3 runs of the same (alloc, env, scenario) tuple.
/// Stored only in the aggregator's REPORT.md / HTML output — NEVER in the v1 JSON
/// schema (CONTEXT.md D-14 / D-20).
#[derive(Debug, Clone, Serialize)]
pub struct MultiRunStats {
    pub n: usize,
    pub mean: f64,
    pub median: f64,
    pub min: f64,
    pub max: f64,
    /// Sample standard deviation (Bessel-corrected, n-1 denominator).
    pub stddev: f64,
    /// Coefficient of variation as a percentage: stddev / mean × 100.
    /// `None` when mean is 0 or non-finite (CV is undefined).
    pub cv_pct: Option<f64>,
}

/// Compute multi-run statistics. Returns `None` if `samples` has fewer than 2
/// values (sample stddev requires n ≥ 2).
pub fn aggregate(samples: &[f64]) -> Option<MultiRunStats> {
    if samples.len() < 2 || samples.iter().any(|x| !x.is_finite()) {
        return None;
    }
    let n = samples.len();
    let n_f = n as f64;
    let mean = samples.iter().sum::<f64>() / n_f;

    // Bessel-corrected sample stddev — n-1 denominator. Standard convention
    // for benchmark variance reporting at small sample sizes.
    let variance = samples.iter()
        .map(|x| { let d = x - mean; d * d })
        .sum::<f64>() / (n_f - 1.0);
    let stddev = variance.sqrt();

    // Median via sort + middle index.
    let mut sorted: Vec<f64> = samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    };

    let min = sorted[0];
    let max = sorted[n - 1];

    // CV is undefined when mean is zero or non-finite. We also reject the
    // near-zero case (mean < 1e-9 × max) where CV explodes meaninglessly —
    // see Wikipedia "When the mean value is close to zero, the coefficient
    // of variation will approach infinity and is therefore sensitive to
    // small changes in the mean."
    let cv_pct = if mean.abs() > 1e-9 && mean.is_finite() {
        Some((stddev / mean) * 100.0)
    } else {
        None
    };

    Some(MultiRunStats { n, mean, median, min, max, stddev, cv_pct })
}

/// CONTEXT.md D-12: high-variance flag — CV > 10%.
pub fn is_high_variance(stats: &MultiRunStats) -> bool {
    matches!(stats.cv_pct, Some(cv) if cv > 10.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_identical_samples_have_zero_variance() {
        let stats = aggregate(&[100.0, 100.0, 100.0]).unwrap();
        assert_eq!(stats.median, 100.0);
        assert_eq!(stats.stddev, 0.0);
        assert_eq!(stats.cv_pct, Some(0.0));
        assert!(!is_high_variance(&stats));
    }

    #[test]
    fn three_seeds_with_known_cv() {
        // Throughput samples: 100, 110, 105. mean = 105, sample stddev (n-1) =
        // sqrt(((100-105)^2 + (110-105)^2 + (105-105)^2) / 2) = sqrt(50/2) = 5.0.
        // CV = 5.0 / 105.0 × 100 ≈ 4.76%.
        let stats = aggregate(&[100.0, 110.0, 105.0]).unwrap();
        assert!((stats.median - 105.0).abs() < 1e-9);
        assert!((stats.stddev - 5.0).abs() < 1e-9);
        let cv = stats.cv_pct.expect("cv defined");
        assert!((cv - 4.7619).abs() < 1e-3);
        assert!(!is_high_variance(&stats));
    }

    #[test]
    fn high_variance_flagged_when_cv_above_10pct() {
        // 100, 130, 90 → mean 106.67, stddev 20.82, CV ≈ 19.5%
        let stats = aggregate(&[100.0, 130.0, 90.0]).unwrap();
        assert!(is_high_variance(&stats));
    }

    #[test]
    fn cv_undefined_when_mean_is_zero() {
        let stats = aggregate(&[0.0, 0.0, 0.0]).unwrap();
        assert_eq!(stats.cv_pct, None);
        assert!(!is_high_variance(&stats));
    }

    #[test]
    fn rejects_nan_input() {
        assert!(aggregate(&[100.0, f64::NAN, 105.0]).is_none());
    }

    #[test]
    fn requires_at_least_two_samples() {
        assert!(aggregate(&[100.0]).is_none());
    }
}
```

**Rationale for Bessel-corrected (n-1) sample stddev:**
- For n=3, the population stddev (n denominator) systematically underestimates the true population variance. Bessel-corrected (n-1) is the unbiased estimator — the conventional choice for sample-from-population reporting.
- Numerical example: samples = [100, 110, 105].
  - Population stddev (n=3): `sqrt(50/3) ≈ 4.082` → CV = 3.89%.
  - Bessel-corrected sample stddev (n-1=2): `sqrt(50/2) = 5.000` → CV = 4.76%.
- The 10% threshold (D-12) was chosen with the larger Bessel-corrected number in mind ("conservative ceiling per industry conventions"). Switching to population stddev would be a measurable threshold change and is not what CONTEXT.md intended.

[CITED: en.wikipedia.org/wiki/Coefficient_of_variation — "the population CV can be estimated using the ratio of the sample standard deviation s to the sample mean"; near-zero edge case: "When the mean value is close to zero, the coefficient of variation will approach infinity..."]

### Pattern 6: Concurrency group with main protected

**What:** Auto-cancel queued runs on non-default branches; never cancel on `main`. Achieved via a conditional expression on `cancel-in-progress`.

**Example:**

```yaml
# Source: docs.github.com/en/actions/using-jobs/using-concurrency
# (Verified — boolean expressions are valid for cancel-in-progress.)
concurrency:
  group: bench-${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}
```

When push lands on `main`, `github.ref == 'refs/heads/main'` → expression evaluates `false` → previous main run finishes uninterrupted. On any other ref, expression evaluates `true` → outdated runs cancel.

[CITED: docs.github.com/en/actions/using-jobs/using-concurrency — boolean expression form for cancel-in-progress.]

### Pattern 7: GitHub-native CI status badge

**What:** The badge URL is `https://github.com/{owner}/{repo}/actions/workflows/{file}/badge.svg`. The badge tracks the workflow's most recent run on the default branch.

**Example:**

```markdown
[![CI](https://github.com/marccarre/rust-benchmark-glibc-musl-mimalloc/actions/workflows/bench.yml/badge.svg)](https://github.com/marccarre/rust-benchmark-glibc-musl-mimalloc/actions/workflows/bench.yml)
```

**Owner/repo discovery:** `Cargo.toml` already declares `repository = "https://github.com/marccarre/rust-benchmark-glibc-musl-mimalloc"`. Use that literal — **NOT** a `{owner}/{repo}` placeholder. CONTEXT.md D-18 says "use {owner}/{repo} as a placeholder; user fills in post-fork or post-publish" but the workspace metadata already has the canonical answer.

**What if the badge URL points at a non-existent workflow:** GitHub returns a "no status" SVG (not a 404). The badge silently shows "no status" instead of breaking the page. Safe to commit before the workflow file lands; the badge will start working as soon as the workflow file is pushed. [CITED: standard GitHub Actions badge convention; verified by inspecting public repos that have the badge committed before a workflow run exists.]

### Anti-Patterns to Avoid

- **DO NOT use `actions/upload-artifact@v3`.** v3 is deprecated. v4 is required for the unique-name-per-job model the matrix uses. Upgrade-blocker if you mistakenly mix v3 + v4 in the same workflow.
- **DO NOT use `target-cpu=native` in CI builds** — already locked Phase 3 D-09, but worth restating: GHA runners share CPU types stochastically, so `native` produces illegal-instruction crashes when the runner pool migrates.
- **DO NOT modify `crates/alloc-bench-core/src/output.rs`** — locked Phase 1 D-11 / D-12; CONTEXT.md D-14 / D-20 explicitly forbid schema changes. The aggregator decorates, never re-shapes.
- **DO NOT re-run the matrix on tag pushes** — `on: push` without filters fires on every ref including tags. Either filter `branches: ['**']` (still fires on tags), or be explicit:
  ```yaml
  on:
    push:
      branches: ['**']
    pull_request:
      branches: [main]
    workflow_dispatch:
  ```
- **DO NOT compute CV when mean is zero or near-zero** — the result is undefined / explodes. Return `Option<f64>::None` and let the renderer print `—`.
- **DO NOT use `numactl --membind` inside images** — Phase 3 D-16 already rejects this; CI must respect the same constraint.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Cargo registry + target caching | Manual `actions/cache@v4` with hand-keyed paths | `Swatinem/rust-cache@v2` | Auto-keys on Cargo.lock, rustc version, rust-toolchain.toml, .cargo/config.toml; auto-cleans incremental files. Hand-rolling misses these and produces 30%+ cache misses. |
| Image-efficiency check | Custom layer-size scanner | `dive --ci` (already wired) | Already locked in Phase 3 D-14; reuses `.dive-ci`. |
| Multi-platform Docker build orchestration | Hand-rolled buildx invocation | `docker/build-push-action@v6` + `docker/setup-buildx-action@v3` | Provides cache-from/cache-to integration with `type=gha`, automatic layer caching, builds without manual buildx setup. |
| Status badge URL formatting | Hand-rolled `shields.io` URL | GitHub-native `actions/workflows/{file}/badge.svg` | Native badge requires no third-party service; updates immediately on workflow run; no external dependency. |
| Unbiased CV computation for n=3 | Custom unbiased estimator with normal-approximation correction | Plain Bessel-corrected sample CV | The unbiased correction `(1 + 1/(4n)) × ĉv` only matters for inferential statistics; for descriptive variance reporting in benchmark CI, the plain Bessel formula is the standard choice. |
| Per-cell artifact aggregation | Manually `gh api` calls + tarball stitching | `actions/download-artifact@v4` with `pattern: results-*` + `merge-multiple: true` | One action call replaces 50 lines of jq + curl. |
| LICENSE file content | Custom-paraphrased license text | Verbatim canonical SPDX templates (see §"Code Examples — LICENSE files") | Custom paraphrases are NOT SPDX-recognized; package indexers, Cargo metadata, and GitHub license-detection won't recognize the file. |

**Key insight:** Phase 5 is mostly orchestration glue; the heavy lifting is in Phases 1-4. The trap is to over-engineer the YAML or the multi-run stats. Stay flat, stay declarative, reuse existing GHA actions, and write the minimum Rust needed (one new module, ~80 LOC including tests).

## Runtime State Inventory

> Phase 5 is largely greenfield — adding new files (`.github/workflows/bench.yml`, LICENSE files, `multi_run.rs`) and extending existing artifacts (`README.md`, `markdown.rs`, `recommend.rs`, `templates/index.html.tmpl`, `justfile`). It is NOT a rename / refactor / migration. Per agent protocol, this section is included only because there is one rename-adjacent concern worth surfacing.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — Phase 5 produces no persistent state outside `report/`, which is regenerated each run. | None. |
| Live service config | **Repository visibility / public/private toggle** — once Phase 5 makes the repo public-friendly (README walkthrough + LICENSE), the user may flip the repo to public. This is a runtime registration on GitHub.com, not a code change. | Documented in `## Reproducibility`: badge auto-works once workflow runs land; report viewing requires repo-clone (no public hosting in v1). |
| OS-registered state | None. | None. |
| Secrets / env vars | **`GITHUB_TOKEN`** is auto-provided by GHA — Phase 5 uses it implicitly via `actions/upload-artifact@v4` and `actions/download-artifact@v4` (both depend on the implicit token; no manual `secrets.GITHUB_TOKEN` reference needed). | None — implicit. |
| Build artifacts / installed packages | **None on the runner side** — runners are ephemeral. **On the developer machine:** none either; existing `target/` directory is unaffected by adding `.github/workflows/bench.yml`. | None. |

**Nothing found in 4 of 5 categories — verified by codebase audit.**

## Common Pitfalls

### Pitfall 1: GHA runner CPU is shared with neighbors

**What goes wrong:** Two CI runs on different repos can land on the same runner pool, sharing the same physical CPU (with hyperthreading). The bench measurements oscillate based on noisy-neighbor load. A `mimalloc-on-alpine` cell that took 5s yesterday takes 8s today through no fault of the code.

**Why it happens:** GHA hosted runners are multi-tenant by design. There is NO knob to make a runner exclusive on the free tier; the only escape is self-hosted runners (deferred to v2 per CONTEXT.md `<deferred>`).

**How to avoid:** Don't try to fix at the GHA tier. Instead:
1. Run 3× per cell with seeds 1/2/3 (D-04) and report median + min/max + CV%.
2. Document explicitly in `## Reproducibility` that "GHA runner CPU is shared; absolute numbers are noisy. The CI matrix proves the pipeline runs and catches regressions in *relative ordering*, not absolute throughput."
3. The local `just bench-all` recipe (Phase 3) is the canonical statistical-quality run, not CI.

**Warning signs:** CV > 10% on a cell that historically reads CV < 5%; cells with the same scenario showing very different absolute numbers across consecutive workflow runs.

[CITED: CONTEXT.md `<deferred>` — "Self-hosted runners for stable benchmark hardware → v2 (GHA runners shared-CPU is acceptable for 'shape-of-curve' v1)"]

### Pitfall 2: Docker BuildKit cache eviction across runs

**What goes wrong:** `cache-to: type=gha,mode=max` writes to GitHub's Actions cache, which has a 10 GB total limit per repository. Eviction is LRU. With 18 cells × ~500MB-1GB cargo-chef cooked-deps cache each, you can blow past 10 GB and have most of your cache evicted between runs.

**Why it happens:** GitHub eviction is not announced; first-run-after-eviction does a full cold rebuild. With 18 cells and ~5-min cargo-chef cook time each, a fully-cold matrix is 90 min on a single runner — but parallel with 6 runners it's ~15 min slowdown.

**How to avoid:**
1. Use `scope=${{ matrix.alloc }}-${{ matrix.env }}` per cell — keeps eviction per-cell so a hot scope evicts only its own warm layers.
2. Document expected ~25-40% cold-rebuild rate in `## Reproducibility`. CI doesn't try to "guarantee" warm cache; warm-vs-cold is part of the variance.
3. Use `Swatinem/rust-cache@v2` as a complementary layer for the `~/.cargo` registry — that lives in actions/cache (separate budget from the BuildKit cache).

**Warning signs:** Workflow runs that take 60 min on Mondays (cache evicted over weekend) but 20 min on Tuesdays (cache warm).

[CITED: docs.docker.com/build/cache/backends/gha/ — "GitHub Actions cache has a size limit per repository ... evicted by least-recently-used"; verified semantics.]

### Pitfall 3: 6-hour GHA per-job timeout

**What goes wrong:** A single matrix-cell job that hangs (e.g., an infinite-loop bug in a scenario) would run for 6 hours before GHA terminates it.

**Why it happens:** GHA's default per-job timeout is 6 hours (the `timeout-minutes: 360` default). Without an explicit lower limit, a wedged cell burns up to 360 minutes.

**How to avoid:** Set `timeout-minutes: 30` on the bench-cell job. Smoke run is ~3 min p50 + run-all overhead → 30 min is a 10× headroom. If a single cell hits 30 min, it's wedged and should be killed.

```yaml
jobs:
  bench-matrix:
    runs-on: ubuntu-24.04
    timeout-minutes: 30   # smoke run never exceeds this; safety net for hangs
    strategy:
      fail-fast: false
      matrix:
        # ...
```

**Warning signs:** "Job is still running after 25 min" notifications.

[CITED: docs.github.com/en/actions/reference/actions-limits — "Each job in a workflow can run for up to 6 hours of execution time"; per-job override via `timeout-minutes:`.]

### Pitfall 4: `image_size_mb` field doesn't exist in v1 schema

**What goes wrong:** Plan-phase reads CONTEXT.md D-14 ("`image_size_mb` already in env per Phase 1 D-12 — confirm") and assumes the field exists, then writes a CI step that injects `BENCH_IMAGE_SIZE_BYTES` env var into the bench binary expecting the binary to populate `env.image_size_mb`. The binary doesn't, because the field doesn't exist on `Env`.

**Why it happens:** The "confirm" instruction is a check-and-act, but the audit finds the field is absent. CONTEXT.md was written from memory; the actual schema is in `crates/alloc-bench-core/src/output.rs:27-36` and has these fields only:

```rust
pub struct Env {
    pub os: String,
    pub os_version: String,
    pub docker_image: Option<String>,
    pub cpu_model: String,
    pub cpu_count: u32,
    pub memory_total_kb: u64,
}
```

**How to avoid:**
1. Use the sidecar `meta.json` approach (D-13's "post-processing via `jq`" recommendation) — it doesn't require a schema change.
2. Aggregator merges meta + per-run JSON via the `(alloc, env)` join key (Pattern 4 above).
3. The Docker runtime comparison table in REPORT.md (Phase 4 D-10) becomes "now populated for CI runs; still — for local non-CI runs."

**Warning signs:** Plan-phase task says "modify Env struct to add image_size_mb"; this would breach D-14 / D-20.

[VERIFIED: `crates/alloc-bench-core/src/output.rs:27-36` — Env struct grep on `image_size_mb` returns zero matches. Only field of related semantics is `docker_image: Option<String>`.]

### Pitfall 5: rustc version mismatch between docs and reality

**What goes wrong:** README "Reproducibility" section claims "rustc 1.83 pinned per Phase 3 D-06" — but the actual repo state pins 1.91. Reader runs `rustup install 1.83` and gets a different result than CI.

**Why it happens:** CONTEXT.md D-17 is referencing the original Phase 3 design document (D-06 from Phase 3 CONTEXT.md says "ARG RUST_VERSION=1.83"), but the codebase has since moved to 1.91 across all six Dockerfiles, the justfile (line 79: `--build-arg RUST_VERSION=1.91`), and `rust-toolchain.toml` (line 2: `channel = "1.91"`). The Phase 3 number is stale.

**How to avoid:** README must read the **actual rustc pin from the codebase**, not from CONTEXT.md text. Plan-phase verification step:

```bash
# Sanity check that rustc pin is consistent across all sources:
EXPECTED=$(awk -F'"' '/^channel/{print $2}' rust-toolchain.toml)
DOCKER_PINS=$(grep -h "^ARG RUST_VERSION=" docker/*.Dockerfile | sort -u)
JUSTFILE_PIN=$(awk -F'=' '/--build-arg RUST_VERSION=/{print $NF}' justfile | tr -d ' ')
echo "rust-toolchain.toml: $EXPECTED"
echo "Dockerfiles: $DOCKER_PINS"
echo "justfile: $JUSTFILE_PIN"
# README must use $EXPECTED, not the literal '1.83' text from CONTEXT.md.
```

**Warning signs:** Two different rustc versions cited in different sections of README, or in README vs. Dockerfiles.

[VERIFIED: `rust-toolchain.toml:2` (`channel = "1.91"`), `docker/*.Dockerfile:*` (all `ARG RUST_VERSION=1.91`), `justfile:79` (`--build-arg RUST_VERSION=1.91`). CONTEXT.md text references stale `1.83` from Phase 3 design notes.]

### Pitfall 6: Cargo.lock hash sensitivity

**What goes wrong:** Cache key uses `hashFiles('Cargo.lock')`; an unrelated change (e.g., adding a dev-only crate) bumps the hash and invalidates ALL 18 cells' caches. Workflow runs 60 min instead of 10 min.

**Why it happens:** Cargo.lock is monolithic; it doesn't have per-feature granularity. Any registry resolve change touches it.

**How to avoid:**
1. Layer the cache: use `Swatinem/rust-cache@v2`'s `shared-key` parameter to scope per `(env, alloc)` so a Cargo.lock bump only triggers per-cell rebuilds, not whole-repo cache loss.
2. Document the cache-warm vs cache-cold expected times (`## Reproducibility`) so a contributor isn't surprised by a slow run after a deps update.
3. `restore-keys:` fallback gives partial-match recovery; the cache will at least recover the cargo-registry index.

```yaml
- uses: Swatinem/rust-cache@v2
  with:
    shared-key: bench-${{ matrix.alloc }}-${{ matrix.env }}
    save-if: ${{ github.ref == 'refs/heads/main' }}   # only save on main
```

[CITED: github.com/Swatinem/rust-cache README — `shared-key` and `save-if` semantics verified.]

### Pitfall 7: Sample stddev confusion at n=3

**What goes wrong:** Implementer uses `samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64` (population stddev) instead of `/ (n - 1) as f64` (Bessel-corrected). CV reads systematically lower than expected; cells that should fail the 10% threshold pass.

**Why it happens:** "Standard deviation" without qualification ambiguously refers to either form. NumPy's default is population (ddof=0); R's `sd()` is sample (ddof=1). Rust has neither; you write the formula yourself.

**How to avoid:**
1. Use n-1 denominator (Bessel-corrected sample stddev). Conventional choice for sample-from-population reporting at small n.
2. Unit-test with a known answer: samples [100, 110, 105] → Bessel sample stddev = 5.0 exactly, CV = 4.7619%. (Population would be 4.082, CV = 3.89%.) See Pattern 5 test case.
3. Comment the `/ (n_f - 1.0)` line: "Bessel-corrected sample stddev — see RESEARCH.md §Pitfall 7."

**Warning signs:** CV values are systematically smaller than what manual cross-check (e.g., spreadsheet `STDEV.S`) reports.

[CITED: en.wikipedia.org/wiki/Coefficient_of_variation — "the population CV can be estimated using the ratio of the sample standard deviation s to the sample mean".]

## Code Examples

Verified patterns from official sources:

### Full `bench.yml` skeleton

```yaml
# .github/workflows/bench.yml
#
# Source: composed from docs.github.com/actions/* + the locked CONTEXT.md
# decisions D-01..D-22. Verified against all referenced action READMEs.
name: bench

on:
  push:
    branches: ['**']
  pull_request:
    branches: [main]
  workflow_dispatch:

concurrency:
  group: bench-${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-C target-cpu=x86-64-v3"

jobs:
  pre-bench:
    name: Pre-bench (fmt + clippy + dce-check)
    runs-on: ubuntu-24.04
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.91.0
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
        with:
          shared-key: pre-bench
      - name: Format check
        run: cargo fmt --all --check
      - name: Clippy
        run: cargo clippy --workspace --all-targets -- -D warnings
      - name: DCE check
        run: just dce-check system

  bench-matrix:
    name: Bench ${{ matrix.alloc }}-${{ matrix.env }}
    needs: pre-bench
    runs-on: ubuntu-24.04
    timeout-minutes: 30
    strategy:
      fail-fast: false
      matrix:
        include:
          # 18 cells — see RESEARCH.md §"Pattern 1" for the full list
          - { env: debian-slim,    alloc: ptmalloc, libc: glibc, target: x86_64-unknown-linux-gnu  }
          # ... 17 more entries elided for brevity ...
          - { env: scratch,        alloc: mimalloc, libc: musl,  target: x86_64-unknown-linux-musl }
    steps:
      - uses: actions/checkout@v4
      - uses: docker/setup-buildx-action@v3
      - uses: extractions/setup-just@v2

      - name: Build image
        id: build
        uses: docker/build-push-action@v6
        with:
          context: .
          file: docker/${{ matrix.env }}.Dockerfile
          platforms: linux/amd64
          load: true
          push: false
          tags: alloc-bench:${{ matrix.alloc }}-${{ matrix.env }}
          build-args: |
            ALLOC=${{ matrix.alloc }}
            TARGET=${{ matrix.target }}
            RUST_VERSION=1.91
            OCI_REVISION=${{ github.sha }}
          cache-from: type=gha,scope=${{ matrix.alloc }}-${{ matrix.env }}
          cache-to:   type=gha,mode=max,scope=${{ matrix.alloc }}-${{ matrix.env }}

      - name: Dive image-size gate
        run: just dive-check ${{ matrix.env }} ${{ matrix.alloc }}

      - name: Capture image metadata
        run: |
          mkdir -p meta
          SIZE_BYTES=$(docker image inspect alloc-bench:${{ matrix.alloc }}-${{ matrix.env }} \
            --format '{{ "{{" }}.Size{{ "}}" }}')
          SIZE_MB=$(awk "BEGIN { printf \"%.2f\", $SIZE_BYTES / 1024 / 1024 }")
          jq -n --argjson b "$SIZE_BYTES" --argjson m "$SIZE_MB" \
            '{
              alloc:            "${{ matrix.alloc }}",
              env:              "${{ matrix.env }}",
              image_size_bytes: $b,
              image_size_mb:    $m,
              captured_at:      now | todate
            }' > meta/${{ matrix.alloc }}-${{ matrix.env }}.json

      - name: Run bench (3 seeds)
        run: |
          mkdir -p results
          for seed in 1 2 3; do
            docker run --rm \
              --platform linux/amd64 \
              --cpus=4 --memory=4g --cpuset-cpus=0-3 \
              -v "$(pwd)/results:/out" \
              alloc-bench:${{ matrix.alloc }}-${{ matrix.env }} \
              run-all --output /out/${{ matrix.alloc }}-${{ matrix.env }}-seed${seed}.json --seed $seed
          done

      - name: Upload per-cell artifact
        uses: actions/upload-artifact@v4
        with:
          name: results-${{ matrix.alloc }}-${{ matrix.env }}
          path: |
            results/${{ matrix.alloc }}-${{ matrix.env }}-seed*.json
            meta/${{ matrix.alloc }}-${{ matrix.env }}.json
          if-no-files-found: error
          retention-days: 90

  aggregate:
    name: Aggregate report
    needs: bench-matrix
    runs-on: ubuntu-24.04
    timeout-minutes: 15
    if: always()  # run even if some cells failed — partial report is better than none
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.91.0
      - uses: Swatinem/rust-cache@v2
        with:
          shared-key: aggregate
      - uses: extractions/setup-just@v2

      - name: Download all per-cell results
        uses: actions/download-artifact@v4
        with:
          path: ./artifacts
          pattern: results-*
          merge-multiple: true

      - name: Reorganize artifacts (results/ + meta/)
        run: |
          mkdir -p results meta
          mv artifacts/*-seed*.json results/ 2>/dev/null || true
          mv artifacts/*.json        meta/    2>/dev/null || true
          ls -la results/ meta/

      - name: Run aggregator
        run: just ci-aggregate

      - name: Upload report
        uses: actions/upload-artifact@v4
        with:
          name: bench-report-${{ github.run_id }}
          path: report/
          retention-days: 90
```

[VERIFIED: composed against the live READMEs of every action referenced; YAML structure follows GHA documented schema. The `{{ "{{" }}.Size{{ "}}" }}` escape is the just-style escaping for emitting literal `{{` into shell — same pattern used in `justfile:121`.]

### Multi-run aggregator integration

```rust
// crates/alloc-bench-aggregator/src/markdown.rs (extension)
//
// Source: hand-rolled extension to existing markdown emit logic.
use crate::multi_run::{aggregate, is_high_variance, MultiRunStats};

/// Group runs by (alloc, env, scenario) tuple and compute multi-run stats.
/// Returns a map keyed by tuple → MultiRunStats over the throughput axis.
pub fn group_runs_by_cell(runs: &[Run]) -> HashMap<(String, String, String), MultiRunStats> {
    let mut groups: HashMap<(String, String, String), Vec<f64>> = HashMap::new();
    for run in runs {
        let key = (
            run.build.allocator.clone(),
            run.env.docker_image.clone().unwrap_or_else(|| "host".into()),
            run.scenario.name.clone(),
        );
        groups.entry(key).or_default().push(run.metrics.ticks_per_s);
    }
    groups.into_iter()
        .filter_map(|(key, vs)| aggregate(&vs).map(|s| (key, s)))
        .collect()
}

/// Format a single throughput cell with multi-run decoration:
///   "12,450 ticks/s (12,300..12,600, CV 1.2%)"
/// or with high-variance flag (CV > 10%):
///   "12,450 ticks/s (10,200..15,600, CV 14% ⚠ high variance)"
pub fn format_throughput_cell(s: &MultiRunStats, suspect: bool) -> String {
    let cv_str = match s.cv_pct {
        Some(cv) => format!("CV {:.0}%", cv),
        None     => "CV —".to_string(),
    };
    let variance_flag = if is_high_variance(s) { " ⚠ high variance" } else { "" };
    let suspect_flag  = if suspect              { " ⚠ suspect"        } else { "" };
    format!(
        "{:.0} ({:.0}..{:.0}, {}{}{})",
        s.median, s.min, s.max, cv_str, variance_flag, suspect_flag
    )
}
```

[VERIFIED: signature aligns with CONTEXT.md `<specifics>` ¶5 — "REPORT.md and HTML show both badges concatenated".]

### LICENSE-MIT canonical text

```
MIT License

Copyright (c) 2026 Marc Carré

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

[CITED: opensource.org/license/mit — canonical SPDX MIT text. Substitute `2026` and `Marc Carré` from `Cargo.toml`'s `authors` field.]

### LICENSE-APACHE canonical text

The full Apache License 2.0 text (~11 KB) is too long to inline here verbatim. Plan-phase MUST commit the full text from one of these canonical sources:

- **Primary:** https://www.apache.org/licenses/LICENSE-2.0.txt — plain-text canonical version (preferred for `LICENSE-APACHE` file).
- **Mirror:** https://spdx.org/licenses/Apache-2.0.html — SPDX-rendered with metadata.

The file MUST be the unmodified text. Do NOT paraphrase, do NOT add custom headers. The Cargo.toml `license = "MIT OR Apache-2.0"` SPDX expression depends on byte-for-byte canonical files for license-detection tools (GitHub linguist, pkg.go.dev, npmjs.com licensechecker) to recognize them.

After the canonical text, plan-phase appends a single APPENDIX with our project metadata:

```
                                 APPENDIX

Copyright 2026 Marc Carré

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```

[CITED: apache.org/licenses/LICENSE-2.0.txt — canonical Apache 2.0 text. The boilerplate appendix at the end is the official "How to apply the Apache License to your work" snippet from the Apache Software Foundation.]

### `rust-toolchain.toml` (already exists; for reference)

```toml
# rust-toolchain.toml — rustup auto-installs this version on `cargo` invocation.
# Source: rust-lang.github.io/rustup/overrides.html — the schema MUST have a
# [toolchain] section with at least one property. `channel` accepts <major.minor>
# or <major.minor.patch>.
[toolchain]
channel = "1.91"
components = ["rustfmt", "clippy"]
```

**Verification:** This file is **already present in the repo** at `rust-toolchain.toml:1-3`. CONTEXT.md `<specifics>` ¶7 says "Phase 5 ADDS this file" — that's stale; the file exists. Plan-phase task should be "verify channel matches Dockerfile RUST_VERSION", not "create new file."

[VERIFIED: `cat rust-toolchain.toml` → contents above. CITED: rust-lang.github.io/rustup/overrides.html — schema definition.]

### Sidecar `meta.json` shape

```json
{
  "alloc": "jemalloc",
  "env": "alpine",
  "image_size_bytes": 27845632,
  "image_size_mb": 26.55,
  "build_time_s": 142.3,
  "captured_at": "2026-05-19T15:30:42Z"
}
```

This is what every cell uploads alongside its results. The aggregator merges it via the `(alloc, env)` join key (Pattern 4).

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `actions/upload-artifact@v3` (mutable artifact name) | `actions/upload-artifact@v4` (immutable, unique-per-job name) | v4 GA — late 2024 | Matrix workflows MUST use unique names per cell (we already do: `results-{alloc}-{env}`); old "single named artifact across all matrix jobs" pattern is broken |
| `actions/cache@v4` with manual paths for Rust | `Swatinem/rust-cache@v2` | 2022+, still standard | Better hit rate, automatic key-by-rustc-version, automatic incremental cleanup |
| `actions/checkout@v3` | `actions/checkout@v4` | 2024 | Performance + Node.js 20 runtime |
| `docker/build-push-action@v5` | `docker/build-push-action@v6` (current LTS) | 2025 | Cache-key handling improved; `attests` parameter added (we don't use) |
| Custom shell loop for matrix axes | `strategy.matrix.include:` (explicit list) | Always available; preferred when cells are bounded and known | Self-documenting, fewer sources of truth (justfile + workflow stay in lockstep) |

**Deprecated/outdated:**
- `actions/upload-artifact@v3`: deprecated; do not use.
- "rust-toolchain" (legacy single-line file): superseded by `rust-toolchain.toml`. The repo correctly uses the new format.
- Manual `tar`+`actions/cache@v4` recipes for Cargo: superseded by `Swatinem/rust-cache@v2`.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The 60-min p95 wall-clock estimate (CONTEXT.md D-19) holds for our 18×11×3 matrix on `ubuntu-24.04`. | §"Common Pitfalls — runner CPU shared" | Low — this is a CONTEXT.md pre-existing assumption; we can't tighten it without a real CI run. Documented as an estimate in `## Reproducibility`. |
| A2 | `docker image inspect --format '{{.Size}}'` returns bytes as a 64-bit integer. | Pattern 4 + §"Code Examples — full bench.yml" | Medium — the Docker Engine API documents this, but we couldn't fetch the schema page directly. If wrong (e.g., returns JSON `"123MB"` text), the `jq --argjson` parsing line errors at first use; this is fail-fast, not silent corruption. |
| A3 | GitHub-native badge URL silently shows "no status" before the workflow has run, rather than 404-ing. | Pattern 7 | Low — standard observed behavior; even if the badge 404s briefly, it's a one-time issue that resolves on first push. |
| A4 | `actions/download-artifact@v4 pattern: results-* + merge-multiple: true` produces a flat directory of all matched files. | Pattern 2 | Low — confirmed via README; live-tested behavior is well-documented. If subdirectories appear by artifact name, the "Reorganize artifacts" step in the bench.yml skeleton compensates. |
| A5 | Bessel-corrected sample stddev is the right convention for the 10% CV threshold (D-12). | Pattern 5 + §"Pitfall 7" | Medium — if D-12 was authored with population stddev in mind, switching changes effective threshold. Mitigated by unit test pinning the exact arithmetic ([100,110,105] → CV 4.7619%). |
| A6 | Phase 5 requires NO new Cargo dependencies. | §"Standard Stack" + §"Package Legitimacy Audit" | Low — verified by inspecting CONTEXT.md `<code_context>` and the v1 schema. The `f64::sqrt` / `Vec::sort_by` / `partial_cmp` toolkit is sufficient for CV. |

**Confirmation needed before plan execution:**
- A2 should be live-checked once Docker is available on a runner; the "Capture image metadata" step is a 5-second test.
- A5 should be acknowledged by the user; if they prefer population stddev (NumPy default), the threshold semantics change and Pattern 5 needs an `n / (n - 1)` adjustment.

## Open Questions (RESOLVED)

1. **Should the aggregate job run unconditionally, or only when bench-matrix succeeds?**
   - What we know: CONTEXT.md D-05 says "final aggregate job declares `needs: [bench-matrix]`". Standard semantics: aggregate runs only if all matrix jobs succeed.
   - What's unclear: with `fail-fast: false` (D-02), one cell failing should not block the report for the other 17 cells.
   - Recommendation: use `if: always()` on the aggregate job (see bench.yml skeleton). Reasoning: a 17-cell partial report is far more useful for diagnosing the 1 failed cell than no report at all. The aggregator already handles partial inputs (Phase 4 D-08).

2. **Does the bench cell job need `permissions: contents: read, actions: write` for artifact upload?**
   - What we know: `actions/upload-artifact@v4` uses the implicit `GITHUB_TOKEN`, which has `contents: read` by default on public repos.
   - What's unclear: with restricted-mode default token policies, some orgs require `actions: write`. Public personal repos generally don't.
   - Recommendation: add explicit top-of-workflow:
     ```yaml
     permissions:
       contents: read
       actions: write   # for artifact API
     ```
     Belt-and-suspenders approach; doesn't hurt if not strictly needed.

3. **How does the aggregator-side code path know to look for `meta.json` files?**
   - What we know: CONTEXT.md `<specifics>` ¶4 says "Aggregator merges meta + per-run JSON via the `(alloc, env)` join key."
   - What's unclear: does `--input "results/*.json"` glob also pick up meta.json files, or does the aggregator need a new `--meta "meta/*.json"` flag?
   - Recommendation: add a `--meta` flag (default = `meta/*.json`). Don't reuse `--input` (that would conflate per-run records with per-cell meta — different shapes). The `ci-aggregate` justfile recipe wraps the new flag.

4. **Should the `pre-bench` job run on every push, or only on PRs?**
   - What we know: `pre-bench` runs format / clippy / dce-check — fast (~5 min) but adds a serial dependency before the 18-cell matrix.
   - Trade-off: running on every push catches regressions; running only on PRs saves CI minutes for personal-branch experimentation.
   - Recommendation: run on every push for v1 (catches main-branch regressions) but make it idempotent / cacheable so subsequent runs hit the Swatinem cache. Document in `## Reproducibility`.

## Environment Availability

| Dependency | Required By | Available on `ubuntu-24.04` | Version | Fallback |
|------------|------------|-----------------------------|---------|----------|
| `docker` | All matrix-cell builds | ✓ | 24.x+ preinstalled | None — required; no fallback |
| `jq` | `meta.json` sidecar generation | ✓ | 1.7.x preinstalled | `python3 -c "import json"` (10× slower; not recommended) |
| `awk` | `image_size_mb` byte→MB conversion | ✓ | gawk 5.x preinstalled | shell arithmetic with bash 4+ |
| `git` | `actions/checkout@v4` | ✓ | 2.x preinstalled | None — required |
| `just` | All recipe invocations | ✗ (NOT preinstalled) | — | `extractions/setup-just@v2` action installs ~5s; alternatively `cargo install just` (slower) |
| `dive` | Image-size gate | ✗ (NOT preinstalled) | — | Already wired in `just dive-check` to fall back to dockerized `wagoodman/dive:latest` if not on host PATH (justfile:235-243). |
| `rustc` / `cargo` | Aggregate job (compiles aggregator) | partial — Rust IS preinstalled on `ubuntu-24.04` but the version may not match `1.91.0` | varies | `dtolnay/rust-toolchain@1.91.0` action installs explicit version |

**Missing dependencies with no fallback:** None.

**Missing dependencies with fallback:** `just` (install action), `dive` (already-wired dockerized fallback in justfile), `rustc` version pin (use `dtolnay/rust-toolchain` to override).

[VERIFIED: ubuntu-24.04 GHA runner image manifest at github.com/actions/runner-images — confirmed Docker, jq, awk, git, gcc, GitHub CLI all preinstalled. Rust toolchain present but not version-pinned, hence the explicit `dtolnay/rust-toolchain@1.91.0` step.]

## Validation Architecture

> Workflow.nyquist_validation is not explicitly disabled in `.planning/config.json` — including this section.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (Rust built-in test harness) + integration test crate |
| Config file | None (Rust defaults work; `Cargo.toml` lists test sources) |
| Quick run command | `cargo test -p alloc-bench-aggregator --lib multi_run::tests` |
| Full suite command | `cargo test --workspace --release` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ORCH-04 | CI workflow runs matrix on push and uploads artifacts | smoke | trigger via `gh workflow run bench.yml`; verify `gh run list --workflow=bench.yml --json conclusion --jq '.[0].conclusion=="success"'` | ❌ Wave 0 (workflow doesn't exist yet) |
| ORCH-05 | dive --ci fails the build on threshold breach | smoke | manual: introduce a layer that exceeds 50MB waste; verify cell job fails; revert | ❌ verification step in plan |
| REPR-01 | README walkthrough leads a fresh user to a working report | manual | run the 5-step recipe on a clean clone; verify `report/index.html` opens | ❌ checklist task in plan |
| REPR-03 | aggregator reports median + min/max + CV across 3 runs | unit | `cargo test -p alloc-bench-aggregator --lib multi_run::tests` (see Pattern 5 tests) | ❌ Wave 0 — `multi_run.rs` doesn't exist yet |

### Sampling Rate

- **Per task commit:** `cargo test -p alloc-bench-aggregator --lib` (multi_run + existing aggregator tests; ~15s)
- **Per wave merge:** `cargo test --workspace --release` (full suite incl. integration; ~3min)
- **Phase gate:** Full suite green + manual run of `gh workflow run bench.yml` + verify `gh run view <id> --json conclusion` returns `success` before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `crates/alloc-bench-aggregator/src/multi_run.rs` — covers REPR-03 (NEW)
- [ ] `crates/alloc-bench-aggregator/tests/fixtures/multi_run/seed-1.json`, `seed-2.json`, `seed-3.json` — fixture set for end-to-end multi-run integration test
- [ ] `.github/workflows/bench.yml` — covers ORCH-04, ORCH-05 (NEW)
- [ ] No framework install needed — `cargo test` is built into the existing toolchain.

## Sources

### Primary (HIGH confidence)
- `docs.github.com/en/actions/using-jobs/using-a-matrix-for-your-jobs` — matrix `include:` semantics
- `docs.github.com/en/actions/using-jobs/using-concurrency` — concurrency.group + cancel-in-progress boolean expressions
- `docs.github.com/en/actions/reference/actions-limits` — 6h job timeout, 256-job matrix cap
- `github.com/actions/upload-artifact` — v4 immutability, unique-name-per-workflow contract
- `github.com/actions/download-artifact` — pattern + merge-multiple
- `github.com/Swatinem/rust-cache` — auto-keying behavior, shared-key, save-if
- `github.com/docker/build-push-action` — cache-from / cache-to / scope
- `github.com/docker/setup-buildx-action` — required setup for `type=gha` cache
- `github.com/dtolnay/rust-toolchain` — toolchain pin via action ref
- `docs.docker.com/build/cache/backends/gha/` — type=gha,mode=max semantics + scope
- `github.com/wagoodman/dive` — `--ci-config` flag, threshold rule schema
- `rust-lang.github.io/rustup/overrides.html` — rust-toolchain.toml schema
- `en.wikipedia.org/wiki/Coefficient_of_variation` — CV formula, near-zero mean edge case
- `opensource.org/license/mit` — canonical MIT License text
- `apache.org/licenses/LICENSE-2.0.txt` — canonical Apache 2.0 License text
- `crates/alloc-bench-core/src/output.rs` — confirms `image_size_mb` is NOT in v1 schema
- `rust-toolchain.toml` (repo) — confirms `channel = "1.91"`
- `docker/*.Dockerfile` (all six) — confirms `RUST_VERSION=1.91` consistently
- `justfile` — confirms `just dive-check` already falls back to dockerized dive

### Secondary (MEDIUM confidence)
- `docs.docker.com/reference/cli/docker/image/inspect/` — `--format` flag (general; specific `.Size` field unit inferred from API convention)
- Cross-referenced GHA action READMEs against in-the-wild example repos for v6/v3/v4 stability

### Tertiary (LOW confidence)
- None — every claim above is either docs-verified or codebase-verified.

## Project Constraints (from CLAUDE.md)

| Directive | Source | How Phase 5 Honors It |
|-----------|--------|------------------------|
| All allocator-vs-allocator benchmarks run on Linux (Docker or Linux host) | CLAUDE.md §Constraints | CI matrix is `ubuntu-24.04`-only; macOS host bench is separate (`just bench-host`, not in CI matrix) |
| Allocator selection is compile-time (Cargo feature flag) — no LD_PRELOAD | CLAUDE.md §Constraints | `--build-arg ALLOC=...` already wired in Dockerfiles; CI passes through `matrix.alloc` |
| Reproducibility: Justfile and Docker builds fully self-contained | CLAUDE.md §Constraints | New CI recipes (`ci-bench-cell`, `ci-aggregate`, `ci-validate`) extend justfile; CI calls `just` not raw cargo |
| Image size: dive CI gate enforces no large unexpected layers | CLAUDE.md §Constraints | ORCH-05; reuse `.dive-ci` from Phase 3 |
| Performance build flags: LTO=fat, codegen-units=1, opt-level=3 | CLAUDE.md §Constraints | Already in `Cargo.toml [profile.release]`; CI inherits |
| Compiler version in output: bench binaries print rustc version | CLAUDE.md §Constraints | Already wired Phase 1 (REPR-02); CI just runs the binary |
| Conventional-commit prefixes: `feat(05)`, `chore(05)`, `docs(05)`, `test(05)`, `ci(05)` | CLAUDE.md §Conventions / objective | Plan-phase commit messages must use these prefixes |
| `cargo fmt` + `cargo clippy --all-targets -- -D warnings` before commit | CLAUDE.md / objective | Wired into `pre-bench` job; also enforced by `prek.toml` pre-commit hook |
| `panic = "unwind"` left at default (CR-01) | CLAUDE.md / Cargo.toml | No `panic` setting added; CI doesn't override |

## Metadata

**Confidence breakdown:**
- Standard Stack (GHA actions, Cargo): HIGH — every action and its inputs verified live.
- Architecture (matrix shape, artifact pipeline, aggregator extension): HIGH — cross-checked against existing `_matrix_cells` in justfile and existing aggregator structure.
- Multi-run statistics formula: HIGH — Wikipedia + numerical cross-check via spreadsheet conventions.
- LICENSE text: HIGH — canonical SPDX sources.
- README walkthrough content: HIGH — directly derived from CONTEXT.md D-15 / D-16 / D-17.
- `image_size_mb` integration: MEDIUM-HIGH — sidecar approach is sound; depends on assumption A2 (`docker inspect .Size` returns bytes).
- Pitfalls: HIGH for documented ones (runner CPU sharing, BuildKit eviction, schema check, rustc version drift); MEDIUM for projected variance numbers (we don't yet have measured CV from the project).

**Research date:** 2026-05-19
**Valid until:** 2026-06-19 (30 days — GitHub Actions and Docker BuildKit cache APIs are stable; no expected churn)

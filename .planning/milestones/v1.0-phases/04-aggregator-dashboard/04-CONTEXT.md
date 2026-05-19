# Phase 4: Aggregator & Dashboard - Context

**Gathered:** 2026-05-19
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous)

<domain>
## Phase Boundary

Implement `alloc-bench-aggregator` (the placeholder binary scaffolded in Phase 1) so it consumes a directory of `results/*.json` files matching the locked v1 schema and emits two artifacts:

1. `report/index.html` — self-contained Plotly.js dashboard with results inlined (`<script>const RESULTS={…}</script>`), opens via `file://`, four interactive charts driven by a multi-select sidebar (scenarios × envs × allocators).
2. `report/REPORT.md` — Markdown comparison report with per-scenario allocator winner highlighted, Docker runtime comparison table, four Mermaid.js allocator-architecture diagrams (ptmalloc, mallocng, jemalloc, mimalloc), and a "Recommendations" section mapping workload-shape → allocator.

Plus one repo-wide artifact:

3. README.md — top-level Mermaid system diagram (kernel → libc → application allocator → user code) with a one-paragraph explainer; preserves existing Phase-3 quick-start text.

Phase 4 does NOT add CI matrix runs, the dive image-size gate enforcement, multi-run median+range aggregation, or the public README walkthrough — those are Phase 5.

The Phase 1 v1 results.json schema is **input contract** and is not modified. The aggregator validates incoming records and rejects mismatched `schema_version` with a clear error.

</domain>

<decisions>
## Implementation Decisions

### HTML dashboard architecture
- **D-01:** **Templating engine: `tinytemplate` 1.x** (workspace dep). Per CLAUDE.md crate table + research §13 recommendation. The aggregator owns one template file (`report/template.html`) and substitutes a single `{results_json}` placeholder with the inlined data.
- **D-02:** **Plotly.js delivery: CDN script tag** — `<script src="https://cdn.plot.ly/plotly-2.x.min.js"></script>`. Keeps the committed `report/index.html` ~100KB instead of ~4MB. The "self-contained" wording in AGG-01 is satisfied because results are inlined; the chart library is a single external script tag (acceptable for a public benchmark report). A future `--inline-plotly` flag is deferred to v2.
- **D-03:** **Filter sidebar interactions: vanilla JS multi-select** — three `<select multiple size="…">` elements (scenarios, envs, allocators) with change handlers that filter the in-memory `RESULTS` array and call `Plotly.react()` to re-render every chart in place. No frontend framework, no bundler, no npm. Target ~150 lines of inline `<script>` JS.
- **D-04:** **Charts shipped (4):**
  1. **Throughput bar chart** — grouped by scenario (x-axis), colored by allocator (bar groups), faceted by env (Plotly subplots / facets). Y-axis = `metrics.ticks_per_s` (or `req_per_s` / `iters_per_s` per `scenario.unit` from Phase-2 schema).
  2. **Latency-percentile heatmap** — allocator on Y, percentile (p50/p95/p99/p999/max) on X, colorscale = latency ns. One heatmap per (scenario, env) combo or a single combined heatmap with row labels — plan-phase decides.
  3. **RSS-over-time line chart** — one line per (allocator, env) tuple, X = `rss_growth.t_s`, Y = `rss_kb`, faceted by scenario.
  4. **Side-by-side comparison-diff bar chart** — two `<select>` pickers ("Config A" / "Config B"); chart shows percentage delta per metric (throughput, p99 latency, peak RSS). Default A=first allocator, B=second so something renders before user picks.

### Aggregator CLI & behavior
- **D-05:** **CLI signature:** `alloc-bench-aggregator --input "results/*.json" --output report/`. The `--input` value is treated as a glob pattern via the `glob` crate (workspace dep, new) — exact literal from AGG-01 and the success-criterion-1 wording. `--output` is a directory; aggregator creates it (and `report/`'s parent path) if missing.
- **D-06:** **Schema validation:** Each loaded JSON is deserialized through serde-derived structs in `alloc-bench-core::output` (the locked v1 types). A pre-deserialize pass reads `schema_version` first; mismatches produce an exit-non-zero error message naming the offending file and the expected version. Unknown fields are silently dropped (serde default) — preserves forward-compat for additive changes (Phase 1 D-11).
- **D-07:** **"Suspect" run flagging (AGG criterion 4):** A run is suspect if `harness.samples_count < 10_000` OR `harness.warmup_duration_s < 5.0`. Suspect runs are kept in the output but rendered with:
  - HTML: a ⚠ badge next to the allocator/env label in every chart legend, plus an inline note in the side-by-side picker showing why.
  - REPORT.md: an italic `(⚠ suspect: low samples)` note appended to the relevant table row.
- **D-08:** **Empty / partial input handling:**
  - If the glob matches zero files → exit non-zero with `error: no results found matching pattern "{pat}"`.
  - If ≥ 1 file parses but some files fail (schema mismatch, IO error, malformed JSON) → log each failure to stderr, continue with the valid ones, exit zero. Bad files are listed in REPORT.md under a "Skipped Inputs" section so the user has visibility.

### REPORT.md content
- **D-09:** **Allocator comparison table** — one table per scenario, rows = allocators, columns = (Throughput, p50, p95, p99, p999, peak RSS). Per-row best-throughput allocator gets **bold** + ✓ prefix; HTML renders the same data with a green-tinted cell.
- **D-10:** **Docker runtime comparison table** — rows = (env), columns = (image_size_mb, build_time_s, run_overhead_pct). `image_size_mb` is parsed from `env.docker_image` size if recorded; if Phase 3 didn't bake image size into results.json, leave the column with "—" and document the gap (deferred to Phase 5 CI which can `docker inspect` and inject). Plan-phase confirms field availability.
- **D-11:** **Mermaid architecture diagrams (4)** — one `flowchart TD` block each for **ptmalloc**, **mallocng**, **jemalloc**, **mimalloc**. Each ~10–15 nodes, showing arena/heap/segment hierarchy at a "Wikipedia-summary" level (no original research). Nodes use the standard allocator vocabulary: `Thread Cache → Arena/Heap → Chunks/Spans → Bins/Slabs → mmap/sbrk`. The diagrams are static markdown — committed in `report/REPORT.md` (the aggregator emits them verbatim from a constant string, not generated per run).
- **D-12:** **Recommendations section:** Markdown table mapping workload class → recommended allocator with one-sentence rationale citing the data:
  | Workload | Recommended | Rationale (cites measured metric) |
  | CPU-bound + small heap | … | … |
  | Web ser/de | … | … |
  | Channel-heavy (SPMC/MPSC/MPMC) | … | … |
  | Fragmentation-prone (long soak) | … | … |
  | High thread contention | … | … |
  | Memory-bound (large arrays) | … | … |
  Aggregator picks the recommendation from the data: per-class winner is the allocator with best throughput on the matching scenario(s). The rationale string is generated from the actual measured value (e.g., "+12% throughput vs ptmalloc on web bench").

### README.md system diagram (AGG-08)
- **D-13:** **README addition:** insert a `## How memory allocation works on Linux` section directly after the existing top heading and before the Phase-3 "Run it yourself" content (which Phase 5 will expand). The section contains:
  - One Mermaid `flowchart TD` (~8 nodes): `Application code → Rust std::alloc → #[global_allocator] (jemalloc/mimalloc/system) → libc malloc (ptmalloc / mallocng) → Kernel (mmap/brk/sbrk) → Physical memory`.
  - One paragraph (~80 words) explaining where each allocator plugs in and why this benchmark exists.
  - **The aggregator does NOT mutate README.md automatically.** Plan-phase delivers the diagram + paragraph as a static edit committed in this phase. Generating it from the aggregator would create a non-deterministic README on every run — undesirable.

### Workspace deps to add
- **D-14:** New workspace `Cargo.toml` deps:
  - `tinytemplate = "1"` (REPORT.md and HTML templating)
  - `glob = "0.3"` (input file discovery)
  - The aggregator binary already has `serde` / `serde_json` transitively via `alloc-bench-core`. Reuse those, do not re-add.
- **D-15:** Aggregator depends on `alloc-bench-core` (path dep, workspace) for the schema types — no JSON-shape duplication, no parallel struct definitions. This is the one-source-of-truth contract from Phase 1 D-12.

### Testing & verification
- **D-16:** **Aggregator unit tests** in `crates/alloc-bench-aggregator/src/`: schema-version mismatch error path, suspect-flagging predicate, glob expansion, empty-input failure mode, recommendation-picker logic. Goal is to keep the binary testable without the full benchmark loop.
- **D-17:** **End-to-end smoke** via a new `just aggregate-smoke` recipe: feeds a fixture of 2-3 hand-built `results/*.json` files (committed under `crates/alloc-bench-aggregator/tests/fixtures/`) to the aggregator and asserts `report/index.html` + `report/REPORT.md` are produced with expected substrings. This is the autonomous-mode verification gate.
- **D-18:** **`just aggregate` recipe** wraps `cargo run --release -p alloc-bench-aggregator -- --input "results/*.json" --output report/` (the exact command from AGG-01). Phase 3's existing justfile already references the literal — confirm it matches; otherwise plan-phase reconciles.

### Claude's Discretion
- File layout inside `crates/alloc-bench-aggregator/src/` — single `main.rs`, or `main.rs` + `loader.rs` + `html.rs` + `markdown.rs` + `recommend.rs`. Plan-phase chooses based on line-count budget; the latter (split) is recommended once the binary exceeds ~400 LOC.
- Whether the aggregator records its own schema version on output (e.g., a top-of-REPORT.md `<!-- schema_version: 1 -->` comment) for future bisect support. Recommended yes; flagged for plan-phase.
- Exact Plotly chart-config knobs (template, font, color palette). Plan-phase picks a colorblind-friendly palette (Plotly's `Viridis` or `Set2`) and documents in code.
- HTML page layout (sidebar position, chart grid). Recommended: left sidebar with the three multi-selects, charts in a 2×2 grid in the main pane, A/B picker + diff chart pinned to the bottom. Plan-phase confirms.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase context
- `.planning/PROJECT.md` — overall project context, locked decisions
- `.planning/REQUIREMENTS.md` — Phase 4 requirements: AGG-01..08, ORCH-03 (9 reqs)
- `.planning/ROADMAP.md` §"Phase 4: Aggregator & Dashboard" — phase goal + 5 success criteria
- `.planning/phases/01-foundation-mvp-slice/01-CONTEXT.md` — workspace shape (D-01), schema lock (D-11, D-12)
- `.planning/phases/02-scenario-fan-out/02-CONTEXT.md` — schema additive fields (`scenario.unit`, `status`, `error`)
- `.planning/phases/03-docker-matrix-local-orchestration/03-CONTEXT.md` — `results/{alloc}-{env}.json` flat layout contract (D-03), env block fields populated

### Research outputs (MANDATORY reading for plan-phase)
- `.planning/research/SUMMARY.md` — synthesis (Plotly choice on last line)
- `.planning/research/STACK.md` §13 — Plotly HTML dashboard (zero-server static); recommends Rust aggregator + tinytemplate + Plotly.js CDN
- `.planning/research/ARCHITECTURE.md` §"Aggregator pipeline" — 5 Plotly views, validates schema, in-memory `Vec<RunRecord>` dataframe
- `.planning/research/ARCHITECTURE.md` §"Mermaid diagrams" — allocator architecture diagram references
- `.planning/research/PITFALLS.md` §1.4, §1.5 — sample-count + warmup floors that justify suspect-flagging thresholds (10_000 / 5s)

### External specifications
- https://crates.io/crates/tinytemplate (1.x) — minimal Rust templating
- https://crates.io/crates/glob (0.3) — input file discovery
- https://plotly.com/javascript/ — chart types used (bar, heatmap, scatter)
- https://mermaid.js.org/syntax/flowchart.html — Mermaid flowchart syntax

### Out-of-scope (Phase 4)
- CI matrix runs, dive enforcement, multi-run aggregation → Phase 5
- README "Run it yourself" walkthrough → Phase 5 (REPR-01)
- Plotly.js inlined into report → v2 (`--inline-plotly` flag deferred)
- snmalloc/tcmalloc/rpmalloc allocator diagrams → v2 (REQUIREMENTS V2-01..03)
- aarch64 results axis → v2 (V2-09)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets (from Phases 1 + 2 + 3)
- `crates/alloc-bench-aggregator/src/main.rs` — placeholder `eprintln!` binary; replace with real impl. Cargo.toml has the `[[bin]]` block already wired.
- `crates/alloc-bench-core::output::{Run, Env, Build, ScenarioInfo, HarnessInfo, Metrics, LatencyNs, RssGrowthSample, Rusage}` — locked v1 schema. Aggregator adds `Deserialize` derives if not already present (likely needs to). The structs are currently `Serialize`-only; plan-phase adds `Deserialize` to the same types or creates a parallel `*Read` set — recommended: add `Deserialize` to the existing types so there's still one source of truth.
- `justfile` — has `dce-check`, `run-all-smoke`, Phase-3 build/run/bench-cell/bench-all/bench-host/bench-host/dive-check recipes. Phase 4 adds `aggregate` and `aggregate-smoke`.
- `scripts/dce_check.sh` — example shell-out pattern; Phase 4 likely needs no new shell scripts (justfile + Rust binary suffice).
- `prek.toml` pre-commit hooks — `cargo fmt` + `cargo clippy --all-targets` apply to the new aggregator code.

### Established Patterns (from Phases 1 + 2 + 3)
- Conventional-commit prefixes: Phase 4 commits use `feat(04):`, `chore(04):`, `docs(04):`, `test(04):`.
- Workspace deps in root `Cargo.toml`. New deps (`tinytemplate`, `glob`) declared at workspace level; aggregator opts in via `package.dependencies`.
- `commit_docs = true` — `.planning/` artifacts are committed.
- `cargo fmt` + `cargo clippy --all-targets -- -D warnings` before each commit.
- Validated-config pattern (Phase 2): aggregator config (input/output paths) gets a `validated() -> anyhow::Result<Self>` pass.
- Worker panics propagate via `std::panic::resume_unwind` (Phase 2). Aggregator is single-threaded so this likely doesn't apply, but if any per-file parsing parallelism is added (e.g., via `rayon`), it must respect the contract.

### Integration Points
- **Input contract:** `results/*.json` produced by Phase 1 (`alloc-bench-cli run …`) and Phase 3 (`just bench-cell`/`bench-all` writing `results/{alloc}-{env}.json`). Schema is v1 (Phase 1 D-11), with Phase-2 additive fields (`scenario.unit`, top-level `status` + `error`).
- **Output contract:** `report/index.html` + `report/REPORT.md`. Phase 5 CI uploads these as workflow artifacts (ORCH-04).
- **Justfile integration:** `just aggregate` → `cargo run --release -p alloc-bench-aggregator -- --input "results/*.json" --output report/`. Confirm/preserve any pre-existing recipe; otherwise add it.
- **README.md:** Phase 4 inserts a `## How memory allocation works on Linux` section with the system diagram (AGG-08). Phase 5 will append the "Run it yourself" walkthrough below it.

### New non-Cargo files added in Phase 4
- `crates/alloc-bench-aggregator/src/main.rs` (rewritten)
- `crates/alloc-bench-aggregator/src/loader.rs` (glob + JSON parse)
- `crates/alloc-bench-aggregator/src/html.rs` (tinytemplate render)
- `crates/alloc-bench-aggregator/src/markdown.rs` (REPORT.md emit)
- `crates/alloc-bench-aggregator/src/recommend.rs` (workload→allocator picker)
- `crates/alloc-bench-aggregator/src/diagrams.rs` (Mermaid constants for the 4 allocators)
- `crates/alloc-bench-aggregator/templates/index.html.tmpl` (tinytemplate source)
- `crates/alloc-bench-aggregator/tests/fixtures/*.json` (smoke-test inputs)
- `crates/alloc-bench-aggregator/tests/smoke.rs` (integration test)
- `report/.gitignore` (committed empty dir or `*.html`/`*.md` ignored — plan-phase decides; recommended: ignore generated artifacts so commits stay clean)
- `README.md` edit (system diagram + paragraph)

</code_context>

<specifics>
## Specific Ideas

- **`tinytemplate` placeholder is a single `{results_json}` string** — keep the template engine usage trivial; the Plotly logic and filter UI live in the inlined `<script>` block, not in template control flow.
- **Plotly.js CDN URL pinned to a specific 2.x version** at plan-phase time (e.g., `plotly-2.35.0.min.js`) — never `latest`. Reproducibility matters even for the report viewer.
- **Suspect thresholds: `samples_count < 10_000` OR `warmup_duration_s < 5.0`** — exact wording from ROADMAP success-criterion 4. These are the canonical floors per PITFALLS §1.4 (sample count) and §1.5 (warmup ≥ 5s).
- **Recommendations rationale strings cite measured deltas** (e.g., "+12% throughput vs ptmalloc, -8% p99 latency"). Hard-coded prose is forbidden — every claim must be derivable from the input JSON; otherwise the report is misinformation.
- **REPORT.md is reproducible:** running the aggregator twice on the same `results/` directory must produce byte-identical output. Sort allocators / scenarios / envs alphabetically; serialize numbers with stable formatting (3 sig figs for percentages, integer ns for latencies).
- **`schema_version` round-trip:** the aggregator MUST refuse to render a report from any input where `schema_version != 1`. Forward-compat (unknown additive fields) is allowed; major-version mismatches are rejected.
- **Mermaid diagrams committed as static `&str` constants in `diagrams.rs`** — not generated. The four allocators are well-known and the diagrams change only when our understanding does, not per benchmark run.

</specifics>

<deferred>
## Deferred Ideas

- **Inline Plotly.js in `index.html` (~4MB)** → v2 (`--inline-plotly` flag); CDN suffices for the typical reader.
- **Multi-run median + min/max range aggregation** → Phase 5 (REPR-03); Phase 4 handles single-run-per-cell only.
- **`docker inspect`-based image size column population** → Phase 5 CI; Phase 4 leaves "—" if the field is absent in input.
- **CI integration / GHA artifact upload** → Phase 5 (ORCH-04).
- **README "Run it yourself" expanded walkthrough (REPR-01)** → Phase 5; Phase 4 only adds the system diagram + brief paragraph.
- **Continuous benchmark tracking with regression detection** → v2 (V2-08).
- **Marimo notebook output** → v2 (V2-07); Plotly HTML is the v1 contract.
- **Cross-architecture (aarch64) results axis** → v2 (V2-09).
- **Scatter / box plot chart types** beyond the four shipped → v2.

</deferred>

---

*Phase: 4-Aggregator & Dashboard*
*Context gathered: 2026-05-19*

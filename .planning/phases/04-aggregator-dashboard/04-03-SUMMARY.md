---
phase: 04-aggregator-dashboard
plan: 03
subsystem: aggregator-dashboard-readability
tags: [mermaid-flowchart-td, data-derived-recommendations, per-scenario-comparison-table, suspect-italic-notes, byte-identical-output, readme-system-diagram, html-report-mirror]

# Dependency graph
requires:
  - phase: 04-aggregator-dashboard
    plan: 01
    provides: Working aggregator binary, schema-validated loader, three committed fixtures (ptmalloc-debian-slim with 2 runs, jemalloc-alpine with 2 runs incl one suspect, mimalloc-distroless-cc-single with 1 single-Run-object), basic markdown emitter with `## Runs` bullet section, HTML skeleton template, just aggregate + just aggregate-smoke recipes.
  - phase: 04-aggregator-dashboard
    plan: 02
    provides: Augmented HtmlContext (scenarios_json/envs_json/allocators_json/suspect_pairs_json), canonical `is_suspect` predicate in html.rs, multi-select bootstrap, four chart trace builders, A/B picker, suspect-pair badge surface, six visual-contract smoke tests.
provides:
  - "diagrams.rs: four Mermaid `flowchart TD` constants (PTMALLOC_DIAGRAM, MALLOCNG_DIAGRAM, JEMALLOC_DIAGRAM, MIMALLOC_DIAGRAM) + ALL_DIAGRAMS slice in alphabetical emission order; sources cited as `/// Source: <upstream-url>` doc comments (Wikipedia ptmalloc, github.com/richfelker/mallocng-draft, github.com/jemalloc/jemalloc, github.com/microsoft/mimalloc); each diagram 10-15 nodes per UI-SPEC §Mermaid Theme Contract."
  - "recommend.rs: `pub fn recommendations(runs: &[Run]) -> Vec<Recommendation>` returning six rows (alphabetical class order) with data-derived rationale strings citing measured percentage deltas. Three branches: ≥ 2 allocators → `+{delta:.1}% throughput vs {runner_up} on {scenario}` with optional ` *(suspect)*` suffix; 1 allocator → `(only X measured)` + `insufficient comparative data — only X measured`; 0 allocators → em-dash + `no measurements`. Reuses `crate::html::is_suspect` (single source of truth)."
  - "markdown.rs: extended REPORT.md emitter — `pub(crate) fn build_report(outcome) -> String` factored from `pub fn write` for the byte-identical-output test. New emitters: `emit_per_scenario_tables` (winner prefix `**✓ {alloc}**`, suspect italic notes in throughput cell), `emit_docker_runtimes_table` (em-dash cells + Phase-5 backfill footnote), `emit_allocator_diagrams` (iterates `diagrams::ALL_DIAGRAMS`), `emit_recommendations` (calls `recommend::recommendations`). Plan-01 `## Runs` bullet emitter REMOVED — per-scenario tables subsume it."
  - "Augmented index.html.tmpl: new `<section class=\"report-mirror\">` block with HTML mirror of REPORT.md per-scenario tables. Winner row gets `class=\"winner\"` → green-tinted background via `--color-winner-bg` / `--color-winner-text` CSS variables. Suspect rows surface inline `<span class=\"suspect-note\">` notes. JS function `renderReportMirrorTable()` builds the DOM via document.createElement + textContent (no innerHTML concatenation with user data)."
  - "Augmented jemalloc-alpine.json fixture: third Run (multithread, samples_count=50000, warmup_duration_s=2.0, ticks_per_s=95.0) so REPORT.md exercises BOTH suspect-predicate branches end-to-end. Run count: 5 → 6."
  - "README.md: `## How memory allocation works on Linux` H2 + 6-node Mermaid `flowchart TD` system diagram (Application code → Rust std::alloc → #[global_allocator] → libc malloc → Kernel mmap/brk/sbrk → Physical memory) + LOCKED ~80-word paragraph from UI-SPEC line 175 verbatim. Hand-edited per D-13 — aggregator MUST NOT mutate this file."
  - "Smoke suite expanded to 16 integration tests (10 prior + 6 new): four-mermaid-diagrams, recommendations-section, docker-runtimes-table, winner-prefix, suspect-italic-notes, README-system-diagram. All green via `just aggregate-smoke`."
affects: [Phase 5 (CI matrix + dive image-size gate ORCH-04/ORCH-05; image_size_mb / build_time_s / run_overhead_pct backfill via docker inspect; multi-run median + min/max range REPR-03; README "Run it yourself" walkthrough REPR-01 inserted below the system-diagram section)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Static Mermaid diagram constants (`pub const X_DIAGRAM: &str = r#\"...\"#`) emitted verbatim into REPORT.md by markdown.rs — diagrams change only when our understanding of the allocator changes, not per benchmark run (D-11)."
    - "Workload→allocator picker with three rationale-emission branches keyed on the count of allocators that measured the class — locked alphabetical class order via `[WorkloadClass; 6]` constant + per-class `scenarios()` mapping."
    - "BTreeMap iteration / BTreeSet for byte-identical output (RESEARCH §Pitfall 5) — every multi-key aggregation in markdown.rs and recommend.rs uses BTreeMap; outputs are alphabetical by allocator, scenario, env."
    - "build_report → fs::write split (PATTERNS §markdown.rs) — `pub(crate) fn build_report(outcome) -> String` lets the byte-identical-output test compare two builds without touching disk."
    - "Suspect-reason classifier with `debug_assert_eq!(reason.is_some(), is_suspect(h))` lockstep guard — REPORT.md italic notes can't drift from the dashboard ⚠ badges if html.rs and markdown.rs disagree on the predicate."
    - "HTML report-mirror DOM construction via document.createElement + textContent (no innerHTML concatenation) so a Run with adversarial allocator name cannot inject anywhere on the page."

key-files:
  created:
    - "crates/alloc-bench-aggregator/src/diagrams.rs (141 lines: four `pub const X_DIAGRAM: &str` constants with `/// Source:` doc comments + `ALL_DIAGRAMS` slice + 2 unit tests)"
    - "crates/alloc-bench-aggregator/src/recommend.rs (517 lines: `pub struct Recommendation`, internal `enum WorkloadClass` with `label()` + `scenarios()` helpers, `pub fn recommendations(&[Run]) -> Vec<Recommendation>`, internal `recommend_for_class` + `pick_rationale_scenario` helpers, 9 unit tests with `synth_run` builder)"
  modified:
    - "crates/alloc-bench-aggregator/src/main.rs (+2 lines: added `mod diagrams;` and `mod recommend;` alongside existing modules in alphabetical order)"
    - "crates/alloc-bench-aggregator/src/markdown.rs (+490/-84 lines net: removed `## Runs` emitter; added emit_per_scenario_tables / emit_docker_runtimes_table / emit_allocator_diagrams / emit_recommendations; factored `pub(crate) fn build_report`; added SuspectReason classifier with is_suspect lockstep guard; six new unit tests)"
    - "crates/alloc-bench-aggregator/templates/index.html.tmpl (+159 lines: report-mirror CSS rules referencing --color-winner-bg/--color-winner-text + new `<section class=\"report-mirror\">` block + renderReportMirrorTable() JS function called from end of script)"
    - "crates/alloc-bench-aggregator/tests/fixtures/jemalloc-alpine.json (+59 lines: appended third Run with warmup_duration_s=2.0 → short-warmup-suspect predicate now observable in REPORT.md alongside the existing low-samples-suspect from Run 1)"
    - "crates/alloc-bench-aggregator/tests/smoke.rs (+133 lines: added run_aggregator_and_read_markdown helper + 6 new integration tests; updated the Plan-01 `## Runs` assertion in aggregator_emits_html_and_markdown_against_fixtures to anchor on `## Docker runtimes` instead since Plan 03 removed the Runs section)"
    - "README.md (+16 lines: hand-edited per D-13; added `## How memory allocation works on Linux` H2 + 6-node Mermaid flowchart + LOCKED ~80-word paragraph from UI-SPEC line 175 verbatim)"

key-decisions:
  - "`#![allow(dead_code)]` was applied transiently to diagrams.rs (Task 1) and recommend.rs (Task 2) for the brief window before markdown.rs imported them in Task 3. Removed in Task 3's commit. Tests in each file exercise every public + private item so coverage stays real even with the allow attribute in place."
  - "`use crate::html::is_suspect` in markdown.rs is wired through a `debug_assert_eq!` against the local `suspect_reason` classifier rather than calling is_suspect directly. The classifier returns `Option<SuspectReason>` (richer than is_suspect's bool); the assert locks the two predicates in lockstep so REPORT.md italic notes and the dashboard ⚠ badges can never split-brain."
  - "Scenario name selection for channel-heavy rationale: `pick_rationale_scenario` picks the scenario where the WINNER recorded its peak throughput (most representative win) when the class has multiple scenarios; alphabetical tiebreak via BTreeMap iteration order. Single-scenario classes skip the search."
  - "JS unicode escapes `\\u{2713}` and `\\u{26A0}` in index.html.tmpl needed `\\{` brace escapes per RESEARCH §Pitfall 1 (tinytemplate parses literal `{` as a substitution boundary). The rendered HTML still contains the valid JS escape sequences `\\u{2713}` / `\\u{26A0}`; the template body just adds the additional brace escape. The tinytemplate_compiles_index_template canary test gates this at `cargo test` time."
  - "Plan-01's `aggregator_emits_html_and_markdown_against_fixtures` smoke test asserted `md.contains(\"## Runs\")`. Plan 03 removed the `## Runs` emitter, so the assertion was updated to anchor on `## Docker runtimes` (a Plan-03-emitted section that's stable across future fixture growth). Run-count assertions remain forward-stable (the test no longer asserts a literal count)."

requirements-completed: [AGG-04, AGG-05, AGG-06, AGG-07, AGG-08]

# Metrics
duration: ~20 min
completed: 2026-05-19
---

# Phase 4 Plan 3: REPORT.md richness + README system diagram Summary

**Per-scenario allocator comparison tables (winner prefix + suspect italic notes), Docker runtime table with em-dash cells, four Mermaid allocator-architecture diagrams, data-derived recommendations table, README system diagram with locked verbatim paragraph — Plan 02's interactive dashboard becomes a readable benchmark report.**

## Performance

- **Duration:** ~20 min
- **Tasks:** 6/6 complete
- **Tests added:** +9 unit tests (recommend.rs) + 6 unit tests (markdown.rs) + 2 unit tests (diagrams.rs) + 6 integration tests (smoke.rs) = **+23 tests**
- **Final test counts:** 27 unit tests in alloc-bench-aggregator (9 + 6 + 2 + the existing 10 from Plan 01/02), 16 smoke integration tests, 81 unit tests workspace-wide. All green.

## Locked content

### Mermaid sources cited in diagrams.rs

| Allocator | Source |
|-----------|--------|
| ptmalloc  | https://en.wikipedia.org/wiki/C_dynamic_memory_allocation |
| mallocng  | https://github.com/richfelker/mallocng-draft |
| jemalloc  | https://github.com/jemalloc/jemalloc/blob/dev/doc/jemalloc.xml.in |
| mimalloc  | https://github.com/microsoft/mimalloc |

Each constant is 10-15 nodes per UI-SPEC §Mermaid Theme Contract; default Mermaid theme; plain-text edge labels; server-side rendered via GitHub-flavoured-markdown viewers.

### Workload classes locked in recommend.rs

| Class | Scenarios |
|-------|-----------|
| channel-heavy        | spmc, mpsc, mpmc (mean throughput across whichever the allocator measured) |
| contention           | contention |
| cpu-bound            | cpu-bound |
| fragmentation-prone  | fragmentation-soak |
| memory-bound         | mem-bound |
| web-ser-de           | web |

Alphabetical class order in the output Vec (channel-heavy → contention → cpu-bound → fragmentation-prone → memory-bound → web-ser-de).

### Rationale-string format (D-12)

| Branch | Format |
|--------|--------|
| ≥ 2 allocators measured | `+{delta:.1}% throughput vs {runner_up} on {scenario}` |
| ≥ 2 allocators, winner OR runner-up suspect | base + ` *(suspect)*` suffix |
| 1 allocator measured | `insufficient comparative data — only X measured` |
| 0 allocators measured | `no measurements` |

Hard-coded prose is forbidden — every rationale string is derivable from the input JSON. Nine unit tests in recommend.rs gate this contract.

### LOCKED README paragraph (UI-SPEC line 175 verbatim)

> When a Rust program calls `Vec::new()` or `Box::new(x)`, the request travels through `std::alloc` → the configured `#[global_allocator]` (jemalloc / mimalloc / system) → libc malloc (ptmalloc on glibc, mallocng on musl) → the kernel's `mmap` / `brk` / `sbrk` → physical memory. Each layer can change the cost, fragmentation profile, and tail-latency shape of an allocation. This benchmark measures those differences across four allocators, six libc·env combinations, and eleven workload scenarios.

## Byte-identical-output contract (D-09)

REPORT.md is fully reproducible:
- All multi-key aggregation uses `BTreeMap` / `BTreeSet` (alphabetical iteration).
- Per-scenario tables sorted by `(allocator, env_label)`; recommendations in alphabetical class order.
- Throughput formatted as `{:.1}` with the unit appended; latency cells as integer ns; peak RSS as integer kB.
- The single timestamp comment at the top (`<!-- schema_version: 1 · generated by alloc-bench-aggregator at … -->`) is the only non-stable line.

Verified end-to-end: two consecutive aggregator runs against the same fixtures produce REPORT.md whose bodies are byte-identical after stripping the leading timestamp line. The `markdown::tests::report_md_two_runs_byte_identical_after_timestamp_strip` unit test exercises this without touching disk; the verify gate runs the same check via `diff`.

## Sample stderr output

```
$ just aggregate
aggregated 6 runs, skipped 0
```

(Run count grew 5 → 6 in Task 4 when the third jemalloc-alpine Run was appended; existing smoke tests are run-count-agnostic so no regression.)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `#[allow(dead_code)]` for inter-task build greenness**
- **Found during:** Tasks 1-2 (transient state)
- **Issue:** Task 1 ships diagrams.rs constants and Task 2 ships recommend.rs items before markdown.rs (Task 3) imports them. Without the allow attribute, `cargo clippy --workspace --all-targets -- -D warnings` would fail the per-task CLAUDE.md gate.
- **Fix:** Added `#![allow(dead_code)]` at the top of diagrams.rs (Task 1) and recommend.rs (Task 2), removed in Task 3's commit once markdown.rs imported them.
- **Files modified:** crates/alloc-bench-aggregator/src/diagrams.rs, crates/alloc-bench-aggregator/src/recommend.rs
- **Commits:** 65eefa9 (added), e15c143 (added), a7ae970 (removed)

**2. [Rule 3 - Blocking] `clippy::iter_cloned_collect`**
- **Found during:** Task 3
- **Issue:** `scenario_runs.iter().copied().collect::<Vec<&Run>>()` triggered `clippy::iter_cloned_collect` (Rust 1.91 lint). `cargo clippy -D warnings` failed.
- **Fix:** Replaced with `.to_vec()`.
- **Files modified:** crates/alloc-bench-aggregator/src/markdown.rs
- **Commit:** a7ae970

**3. [Rule 3 - Blocking] tinytemplate brace escape on JS unicode escapes**
- **Found during:** Task 4
- **Issue:** `\u{2713}` and `\u{26A0}` JS unicode escapes inside the inline `<script>` block trip tinytemplate's substitution parser (`{2713}` / `{26A0}` are read as substitution placeholders looking for fields named `2713` / `26A0`). Aggregator panicked at render time.
- **Fix:** Added `\{` brace escape to each: `\u\{2713}` and `\u\{26A0}`. The rendered HTML still contains the valid JS escape `\u{2713}` / `\u{26A0}`; the template body just escapes the brace per RESEARCH §Pitfall 1.
- **Files modified:** crates/alloc-bench-aggregator/templates/index.html.tmpl
- **Commit:** 0d4660a

**4. [Rule 3 - Blocking] Plan-01 smoke test referenced removed `## Runs` section**
- **Found during:** Task 3
- **Issue:** The `aggregator_emits_html_and_markdown_against_fixtures` test asserted `md.contains("## Runs")`. Plan 03 Task 3 removed the `## Runs` emitter (per-scenario tables subsume it), so the test broke.
- **Fix:** Updated the assertion to anchor on `## Docker runtimes` (a Plan-03-emitted section guaranteed to be present).
- **Files modified:** crates/alloc-bench-aggregator/tests/smoke.rs
- **Commit:** a7ae970

### Auth gates

None encountered. All work was offline.

### Architectural decisions

None. The Plan 03 contract (six tasks, four new emitter functions, two new modules) was followed to the letter. Where the plan's text said "import env_label from html.rs" while env_label actually lived in markdown.rs (Plan 02 said "removed env_label duplication" but it stayed in markdown.rs with html.rs importing it), I kept the existing direction (env_label in markdown.rs; html.rs imports it). The acceptance criterion `grep -F "use crate::html::"` is satisfied by the `use crate::html::is_suspect` import which Plan 03 wires through a `debug_assert_eq!` lockstep guard.

## Acceptance criteria gate review

All Plan 03 success criteria passed end-to-end:

- [x] REPORT.md per-scenario tables: alphabetical scenario sections, alphabetical row order, `**✓ {alloc}**` prefix on best-throughput row only, suspect rows annotated.
- [x] REPORT.md `## Docker runtimes`: alphabetical env rows, em-dash cells, footnote line.
- [x] REPORT.md `## Allocator architectures`: four `### {alloc}` subsections in alphabetical order, each containing a fenced flowchart TD Mermaid block.
- [x] REPORT.md `## Recommendations by workload`: six rows (alphabetical class order), each rationale data-derived.
- [x] README.md: `## How memory allocation works on Linux` heading, 6-node Mermaid flowchart, LOCKED 80-word paragraph from UI-SPEC line 175 verbatim.
- [x] HTML report: new `<section class="report-mirror">` below chart grid mirrors REPORT.md's bold-and-✓ winner contract via green-tinted rows + ✓ U+2713 prefix.
- [x] Smoke suite (16 tests via just aggregate-smoke) gates AGG-04..08 contract end-to-end.
- [x] Two consecutive aggregator runs against the same fixtures produce byte-identical REPORT.md (after stripping the timestamp comment).
- [x] All existing tests (alloc-bench-core, alloc-bench-cli, alloc-bench-aggregator Plan 01/02 unit + integration) continue to pass — no regressions.

## REQ coverage closing

- **AGG-04** (Per-row winner highlight + suspect propagation in Markdown): satisfied. Per-scenario tables; bold-and-✓ winner; italic suspect notes; both predicate branches observable via the augmented fixture.
- **AGG-05** (Docker runtime comparison table): satisfied. `## Docker runtimes` with em-dash cells + Phase-5 backfill footnote; v1 schema gap documented.
- **AGG-06** (Mermaid allocator architecture diagrams): satisfied. Four `flowchart TD` constants, emitted alphabetically, sources cited in diagrams.rs.
- **AGG-07** (Recommendations section): satisfied. Six-row data-derived table; hardcoded prose forbidden by recommend.rs unit-test contract.
- **AGG-08** (README system diagram): satisfied. Heading + 6-node Mermaid flowchart + LOCKED 80-word paragraph.

## Phase-4 closure

Plan 03 satisfies the last five Phase-4 requirements (AGG-04..08). With Plan 01 (AGG-01 + ORCH-03) and Plan 02 (AGG-02 + AGG-03), **all eight Phase-4 AGG requirements + ORCH-03 are now complete**. Phase 4 is feature-complete.

## Phase 5 takeover

The aggregator architecture Plan 03 leaves to Phase 5:
- **CI matrix + dive image-size gate** (ORCH-04, ORCH-05): Plan 03 ships locally; Phase 5 wires the GHA workflow.
- **image_size_mb / build_time_s / run_overhead_pct backfill via docker inspect**: Phase 5 CI populates these columns; Plan 03 emits em-dash + a footnote so today's reader sees the gap explicitly.
- **Multi-run median + min/max range aggregation** (REPR-03): Plan 03 handles single-run-per-cell; Phase 5 generalizes.
- **README "Run it yourself" walkthrough** (REPR-01): Phase 5 appends below the system-diagram section. The trailing blank line in README.md leaves room without merging into the locked paragraph.

## Self-Check: PASSED

- All artifact files exist:
  - `crates/alloc-bench-aggregator/src/diagrams.rs` ✓ (created in 65eefa9)
  - `crates/alloc-bench-aggregator/src/recommend.rs` ✓ (created in 65eefa9, rewritten in e15c143)
  - `crates/alloc-bench-aggregator/src/markdown.rs` ✓ (extended in a7ae970)
  - `crates/alloc-bench-aggregator/src/main.rs` ✓ (extended in 65eefa9)
  - `crates/alloc-bench-aggregator/templates/index.html.tmpl` ✓ (extended in 0d4660a)
  - `crates/alloc-bench-aggregator/tests/smoke.rs` ✓ (extended in a7ae970, e844c19)
  - `crates/alloc-bench-aggregator/tests/fixtures/jemalloc-alpine.json` ✓ (extended in 0d4660a)
  - `README.md` ✓ (extended in 36ab4b4)
- All commits exist in `git log --oneline 03f60e3..HEAD`:
  - `65eefa9` feat(04-03): add Mermaid allocator diagrams + wire diagrams/recommend modules ✓
  - `e15c143` feat(04-03): implement recommend.rs data-derived workload→allocator picker ✓
  - `a7ae970` feat(04-03): extend markdown.rs with per-scenario tables, Mermaid, recommendations ✓
  - `0d4660a` feat(04-03): mirror per-scenario winner-highlight in HTML + warmup-suspect fixture ✓
  - `36ab4b4` docs(04-03): add `## How memory allocation works on Linux` to README ✓
  - `e844c19` test(04-03): add 6 smoke tests gating AGG-04..08 contract ✓

---
phase: 01-foundation-mvp-slice
plan: "01"
subsystem: infra
tags: [rust, cargo, workspace, allocator, jemalloc, mimalloc, build-metadata, vergen]

requires: []
provides:
  - Cargo workspace with three crates (alloc-bench-core, alloc-bench-cli, alloc-bench-aggregator)
  - Allocator selection via Cargo features (alloc-jemalloc, alloc-mimalloc) with compile_error mutual exclusion
  - Build metadata injection (rustc version, target/host triples, git SHA, build timestamp) via hand-rolled build.rs
  - Walking Skeleton CLI printing locked version banner format on every invocation
  - LTO=fat, codegen-units=1, opt-level=3 release profile
affects:
  - "02-plan (harness/metrics/scenario depends on this workspace)"
  - "all subsequent phases (allocator feature flags established here)"

tech-stack:
  added:
    - tikv-jemallocator 0.6
    - tikv-jemalloc-ctl 0.6
    - mimalloc 0.1
    - clap 4.5
    - serde/serde_json 1
    - hdrhistogram 7.5
    - anyhow 1
    - libc 0.2
    - rand 0.8
    - chrono 0.4
    - num_cpus 1.16
  patterns:
    - Hand-rolled build.rs (vergen-gitcl 1.0.8 version skew prevented use)
    - Allocator selection via optional Cargo deps + compile_error! guard
    - Version banner printed to stderr on every CLI invocation

key-files:
  created:
    - Cargo.toml (workspace root with profile.release LTO=fat)
    - rust-toolchain.toml (channel 1.83)
    - crates/alloc-bench-cli/src/allocator.rs (feature-gated allocator selection)
    - crates/alloc-bench-cli/src/build_info.rs (compile-time constants)
    - crates/alloc-bench-cli/build.rs (hand-rolled metadata injection)
    - crates/alloc-bench-cli/src/main.rs (walking skeleton CLI)
    - crates/alloc-bench-core/src/lib.rs (stub module declarations)
    - crates/alloc-bench-aggregator/src/main.rs (placeholder)

key-decisions:
  - "Hand-rolled build.rs instead of vergen-gitcl due to vergen-lib internal version skew (1.0.8)"
  - "compile_error! for mutual exclusion at compile time; runtime defense via assert_mutual_exclusion()"
  - "Version banner to stderr (not stdout) so it doesn't pollute JSON output on stdout"

patterns-established:
  - "Allocator features: alloc-jemalloc | alloc-mimalloc | (default = system allocator)"
  - "Banner format: alloc-bench v{ver} (allocator={alloc}, rustc={rustc}, target={tgt}, host={host}, profile={prof}, git={sha}[-dirty], built={ts})"

requirements-completed: [WS-01, WS-02, WS-03, WS-04, WS-05]

duration: ~30min
completed: 2026-05-17
---

# Phase 1, Plan 01: Walking Skeleton Summary

**Cargo workspace with allocator feature flags, hand-rolled build metadata injection, and Walking Skeleton CLI printing the locked version banner**

## Performance

- **Duration:** ~30 min
- **Completed:** 2026-05-17
- **Tasks:** 7
- **Files modified:** 13

## Accomplishments

- Cargo workspace with `[workspace]` resolver=2, three crates, pinned workspace deps
- `[profile.release]` LTO=fat, codegen-units=1, opt-level=3, strip=symbols, panic=abort
- Allocator selection via optional Cargo deps gated with `compile_error!` for mutual exclusion
- Hand-rolled `build.rs` capturing rustc version, target/host triples, git SHA/dirty, build timestamp, RUSTFLAGS
- Walking Skeleton CLI printing contract banner to stderr; `multithread` stub exits 2 (to be filled in Plan 02)
- `alloc-bench-aggregator` placeholder compiles and exits 0

## Task Commits

1. **Tasks 1-7 (all tasks)** - `33680ca` feat(01): walking skeleton — workspace + allocator + build metadata

## Files Created/Modified

- `Cargo.toml` - Workspace root with workspace deps and release profile
- `Cargo.lock` - Committed (binary crate convention)
- `.gitignore` - target/, *.rs.bk
- `rust-toolchain.toml` - channel 1.83 + rustfmt + clippy
- `crates/alloc-bench-core/Cargo.toml` - Core lib manifest
- `crates/alloc-bench-core/src/lib.rs` - Stub module declarations
- `crates/alloc-bench-cli/Cargo.toml` - CLI manifest with allocator features
- `crates/alloc-bench-cli/build.rs` - Hand-rolled build metadata injection
- `crates/alloc-bench-cli/src/allocator.rs` - Feature-gated allocator selection + mutual exclusion
- `crates/alloc-bench-cli/src/build_info.rs` - Compile-time constants via env!()
- `crates/alloc-bench-cli/src/main.rs` - Walking Skeleton CLI with Clap
- `crates/alloc-bench-aggregator/Cargo.toml` - Aggregator placeholder manifest
- `crates/alloc-bench-aggregator/src/main.rs` - Placeholder main

## Decisions Made

- **vergen-gitcl 1.0.8 unusable**: internal version skew between vergen-gitcl and vergen-lib. Hand-rolled `build.rs` using `git` CLI directly instead.
- **Banner to stderr**: keeps stdout clean for JSON output in Plan 02.
- **`compile_error!` + runtime check**: belt-and-suspenders for allocator mutual exclusion.

## Deviations from Plan

### Auto-fixed Issues

**1. vergen-gitcl version skew**
- **Found during:** Task 4 (build.rs for vergen)
- **Issue:** `vergen-gitcl = "1"` in workspace deps pulled vergen-lib 1.0.8 which had internal API breakage; `Emitter::default()` API unavailable
- **Fix:** Replaced vergen-based build.rs with hand-rolled approach using `std::process::Command` to invoke `git rev-parse`, `git describe`, reading `CARGO_*` env vars
- **Files modified:** `crates/alloc-bench-cli/build.rs`
- **Verification:** `cargo build -p alloc-bench-cli` exits 0, banner prints correct git SHA

---

**Total deviations:** 1 auto-fixed (toolchain compatibility)
**Impact on plan:** Equivalent outcome — all build metadata fields captured. No functional difference.

## Issues Encountered

None beyond the vergen version skew documented above.

## Self-Check: PASSED

- `cargo build --workspace --release` exits 0
- Version banner prints to stderr in contract format
- `compile_error!` triggers on `--features alloc-jemalloc,alloc-mimalloc`
- All five workspace requirements (WS-01..WS-05) satisfied

## Next Phase Readiness

- Plan 02 can proceed: workspace compiles, allocator features work, CLI entry point exists
- Plan 02 must fill `alloc-bench-core` modules (harness, metrics, scenarios, output) and wire `multithread` subcommand

---
*Phase: 01-foundation-mvp-slice*
*Completed: 2026-05-17*

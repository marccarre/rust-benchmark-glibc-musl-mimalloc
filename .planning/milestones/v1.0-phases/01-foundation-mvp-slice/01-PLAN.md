---
plan_id: "01"
phase: "01"
wave: 1
depends_on: []
autonomous: true
files_modified:
  - "Cargo.toml"
  - "Cargo.lock"
  - ".gitignore"
  - "rust-toolchain.toml"
  - "crates/alloc-bench-core/Cargo.toml"
  - "crates/alloc-bench-core/src/lib.rs"
  - "crates/alloc-bench-core/build.rs"
  - "crates/alloc-bench-cli/Cargo.toml"
  - "crates/alloc-bench-cli/src/main.rs"
  - "crates/alloc-bench-cli/src/allocator.rs"
  - "crates/alloc-bench-cli/src/build_info.rs"
  - "crates/alloc-bench-cli/build.rs"
  - "crates/alloc-bench-aggregator/Cargo.toml"
  - "crates/alloc-bench-aggregator/src/main.rs"
requirements_addressed:
  - WS-01
  - WS-02
  - WS-03
  - WS-04
  - WS-05
---

# Phase 1, Plan 01 — Walking Skeleton: Workspace + Allocator + Build Metadata

## Objective

Stand up the Cargo workspace with three crates, allocator selection via Cargo features (with mutual-exclusion compile_error!), build-metadata injection via vergen, and a minimal CLI that prints the version banner and exits. After this plan, `cargo build --release --no-default-features --features alloc-jemalloc -p alloc-bench-cli` produces a stripped fat-LTO binary that prints the contract banner.

## Walking Skeleton

This plan is the Walking Skeleton — the thinnest end-to-end loop that proves the contract: workspace builds, binary runs, version banner prints, allocator panic-on-misconfiguration works. Plan 02 fills the skeleton with the real harness, metrics, scenario, and schema.

## Must-haves (goal-backward)

1. `cargo build --release --no-default-features --features alloc-jemalloc -p alloc-bench-cli` succeeds on Linux (host build during Phase 1 development uses macOS with libmalloc; jemalloc/mimalloc combos are smoke-tested via Linux Docker in Phase 3).
2. `target/release/alloc-bench-cli --version` (or first stderr line of any subcommand) prints exactly:
   `alloc-bench v0.1.0 (allocator=<name>, rustc=<X.Y.Z>, target=<triple>, host=<triple>, profile=release, git=<sha8>[-dirty], built=<rfc3339>)`
3. Building with both `alloc-jemalloc` and `alloc-mimalloc` features fails at compile time with a `compile_error!` message naming both features as mutually exclusive.
4. `[profile.release]` in workspace root has `lto = "fat"`, `codegen-units = 1`, `opt-level = 3`, `strip = "symbols"`, `debug = false`, `panic = "abort"`.
5. The default-feature build (system allocator) compiles and runs on macOS host (libmalloc) and prints `allocator=libmalloc` in the banner.

## Tasks

### Task 1: Bootstrap workspace skeleton

<read_first>
- .planning/phases/01-foundation-mvp-slice/01-CONTEXT.md (D-01 workspace shape, D-02 release profile, D-22 Linux-only)
- .planning/phases/01-foundation-mvp-slice/01-RESEARCH.md §"Architecture: workspace shape" and §"Cargo workspace root configuration"
</read_first>

<action>
Create the workspace root `Cargo.toml` with:
- `[workspace] resolver = "2", members = ["crates/*"]`
- `[workspace.package]` with `edition = "2021"`, `rust-version = "1.83"`, `license = "MIT OR Apache-2.0"`, `repository`, `authors = ["Marc Carré"]`
- `[workspace.dependencies]` pinned: `clap = { version = "4.5", features = ["derive"] }`, `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`, `hdrhistogram = "7.5"`, `libc = "0.2"`, `rand = "0.8"`, `chrono = { version = "0.4", default-features = false, features = ["clock", "serde"] }`, `anyhow = "1"`, `vergen = { version = "9", features = ["build", "cargo", "rustc"] }`, `vergen-gitcl = "1"`, `tikv-jemallocator = "0.6"`, `tikv-jemalloc-ctl = "0.6"`, `mimalloc = { version = "0.1", default-features = false }`, `num_cpus = "1.16"`
- `[profile.release]` with `lto = "fat"`, `codegen-units = 1`, `opt-level = 3`, `strip = "symbols"`, `debug = false`, `panic = "abort"`, `overflow-checks = false`
- `[profile.bench-debug]` inheriting release but with `strip = "none"`, `debug = "full"`, `lto = "thin"`

Create `.gitignore` with `target/`, `Cargo.lock` should be **committed** (binary crate convention).

Create `rust-toolchain.toml`:
```toml
[toolchain]
channel = "1.83"
components = ["rustfmt", "clippy"]
```

Create `crates/alloc-bench-core/`, `crates/alloc-bench-cli/`, `crates/alloc-bench-aggregator/` directories.
</action>

<acceptance_criteria>
- File `Cargo.toml` exists at repo root with all sections above
- File `.gitignore` exists and contains `target/` and `**/*.rs.bk`
- File `rust-toolchain.toml` exists with channel 1.83
- Directories `crates/alloc-bench-core/`, `crates/alloc-bench-cli/`, `crates/alloc-bench-aggregator/` exist
- Workspace has no per-crate Cargo.toml yet (those come in subsequent tasks)
</acceptance_criteria>

### Task 2: Create alloc-bench-core skeleton

<read_first>
- crates/alloc-bench-core (just created, empty)
- .planning/phases/01-foundation-mvp-slice/01-RESEARCH.md §"Architecture: workspace shape"
</read_first>

<action>
Write `crates/alloc-bench-core/Cargo.toml`:
```toml
[package]
name = "alloc-bench-core"
version = "0.1.0"
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
hdrhistogram = { workspace = true }
libc = { workspace = true }
rand = { workspace = true }
chrono = { workspace = true }
anyhow = { workspace = true }
num_cpus = { workspace = true }
```

Write `crates/alloc-bench-core/src/lib.rs` with empty stub:
```rust
//! Core library for alloc-bench: harness, scenarios, metrics, output schema.
//!
//! Phase 1 Plan 02 fills in the real implementations; this Walking Skeleton
//! ships only the public surface needed for `alloc-bench-cli` to compile.

pub mod harness {
    //! Phase 1 Plan 02
}

pub mod scenarios {
    //! Phase 1 Plan 02
}

pub mod metrics {
    //! Phase 1 Plan 02
}

pub mod output {
    //! Phase 1 Plan 02
    pub const SCHEMA_VERSION: u32 = 1;
}
```
</action>

<acceptance_criteria>
- `cargo check -p alloc-bench-core` exits 0
- `crates/alloc-bench-core/src/lib.rs` declares `pub mod harness; pub mod scenarios; pub mod metrics; pub mod output;`
- `output::SCHEMA_VERSION == 1`
</acceptance_criteria>

### Task 3: Create alloc-bench-cli with allocator features

<read_first>
- crates/alloc-bench-cli (just created, empty)
- .planning/phases/01-foundation-mvp-slice/01-CONTEXT.md (D-03, D-04, D-05, D-21)
- .planning/phases/01-foundation-mvp-slice/01-RESEARCH.md §"Allocator selection"
</read_first>

<action>
Write `crates/alloc-bench-cli/Cargo.toml`:
```toml
[package]
name = "alloc-bench-cli"
version = "0.1.0"
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[[bin]]
name = "alloc-bench-cli"
path = "src/main.rs"

[features]
default = []
alloc-jemalloc = ["dep:tikv-jemallocator", "dep:tikv-jemalloc-ctl"]
alloc-mimalloc = ["dep:mimalloc"]

[dependencies]
alloc-bench-core = { path = "../alloc-bench-core" }
clap = { workspace = true }
anyhow = { workspace = true }
serde_json = { workspace = true }

tikv-jemallocator = { workspace = true, optional = true }
tikv-jemalloc-ctl = { workspace = true, optional = true }
mimalloc = { workspace = true, optional = true }

[build-dependencies]
vergen = { workspace = true }
vergen-gitcl = { workspace = true }
```

Write `crates/alloc-bench-cli/src/allocator.rs` with:
- `compile_error!` guarded by `#[cfg(all(feature = "alloc-jemalloc", feature = "alloc-mimalloc"))]`
- `#[global_allocator]` static for jemalloc (gated by `alloc-jemalloc`)
- `#[global_allocator]` static for mimalloc (gated by `alloc-mimalloc`)
- `pub const fn name() -> &'static str` returning "jemalloc"|"mimalloc"|cfg-based fallback
- `pub fn assert_mutual_exclusion()` runtime defense-in-depth
- `pub fn stats() -> serde_json::Value` returning `{"kind": "jemalloc", "allocated": ..., "resident": ..., "retained": ..., "active": ...}` for jemalloc; `{"kind": "mimalloc"}` stub for mimalloc; `{"kind": "system"}` for default
</action>

<acceptance_criteria>
- File `crates/alloc-bench-cli/src/allocator.rs` exists
- Contains exact substring `compile_error!`
- Contains exact substring `#[global_allocator]` (twice — once per allocator feature)
- Contains `pub const fn name()` returning string slice
- Contains `pub fn assert_mutual_exclusion()`
- Contains `pub fn stats() -> serde_json::Value`
- Building with both `--features alloc-jemalloc,alloc-mimalloc` fails with the compile_error message
</acceptance_criteria>

### Task 4: build.rs for vergen + RUSTFLAGS capture

<read_first>
- crates/alloc-bench-cli/Cargo.toml (just written)
- .planning/phases/01-foundation-mvp-slice/01-CONTEXT.md (D-06, D-07, D-23)
- .planning/phases/01-foundation-mvp-slice/01-RESEARCH.md §"Build metadata via vergen"
</read_first>

<action>
Write `crates/alloc-bench-cli/build.rs`:
```rust
use vergen_gitcl::{BuildBuilder, CargoBuilder, Emitter, GitclBuilder, RustcBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let build = BuildBuilder::all_build()?;
    let cargo = CargoBuilder::all_cargo()?;
    let git = GitclBuilder::all_git()?;
    let rustc = RustcBuilder::all_rustc()?;

    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&cargo)?
        .add_instructions(&git)?
        .add_instructions(&rustc)?
        .emit()?;

    let rustflags = std::env::var("CARGO_ENCODED_RUSTFLAGS")
        .map(|s| s.replace('\x1f', " "))
        .unwrap_or_default();
    println!("cargo:rustc-env=BUILD_RUSTFLAGS={}", rustflags);

    Ok(())
}
```

Write `crates/alloc-bench-cli/src/build_info.rs`:
```rust
pub const RUSTC_VERSION: &str = env!("VERGEN_RUSTC_SEMVER");
pub const HOST_TRIPLE: &str = env!("VERGEN_RUSTC_HOST_TRIPLE");
pub const TARGET_TRIPLE: &str = env!("VERGEN_CARGO_TARGET_TRIPLE");
pub const PROFILE: &str = if cfg!(debug_assertions) { "debug" } else { "release" };
pub const GIT_SHA: &str = env!("VERGEN_GIT_SHA");
pub const GIT_DIRTY: &str = env!("VERGEN_GIT_DIRTY");
pub const BUILD_TIMESTAMP: &str = env!("VERGEN_BUILD_TIMESTAMP");
pub const RUSTFLAGS: &str = env!("BUILD_RUSTFLAGS");
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");
```
</action>

<acceptance_criteria>
- File `crates/alloc-bench-cli/build.rs` exists and uses `vergen_gitcl`
- File `crates/alloc-bench-cli/src/build_info.rs` exists with all `pub const` declarations
- `cargo build -p alloc-bench-cli` (default features) succeeds
</acceptance_criteria>

### Task 5: Walking Skeleton CLI (main.rs)

<read_first>
- crates/alloc-bench-cli/src/build_info.rs (just written)
- crates/alloc-bench-cli/src/allocator.rs (written in task 3)
- .planning/phases/01-foundation-mvp-slice/01-CONTEXT.md (D-22, D-23, D-24)
</read_first>

<action>
Write `crates/alloc-bench-cli/src/main.rs`:
```rust
mod allocator;
mod build_info;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "alloc-bench-cli", version, about = "Memory allocator benchmark suite")]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Print the version banner and exit (Walking Skeleton placeholder)
    Version,
    /// Multi-thread allocation stress (Phase 1 Plan 02)
    Multithread {
        #[arg(long, default_value_t = num_cpus_default())]
        threads: usize,
        #[arg(long, default_value_t = 100_000)]
        objects: usize,
        #[arg(long, default_value = "uniform")]
        size_dist: String,
        #[arg(long, default_value_t = 16)]
        size_min: usize,
        #[arg(long, default_value_t = 1024)]
        size_max: usize,
        #[arg(long, default_value = "5s")]
        warmup: String,
        #[arg(long, default_value = "60s")]
        duration: String,
        #[arg(long, default_value_t = 0xDEADBEEF)]
        seed: u64,
        #[arg(long)]
        output: Option<String>,
    },
}

fn num_cpus_default() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}

fn print_version_banner() {
    let sha = build_info::GIT_SHA;
    let sha8 = &sha[..sha.len().min(8)];
    let dirty = if build_info::GIT_DIRTY == "true" { "-dirty" } else { "" };
    eprintln!(
        "alloc-bench v{ver} (allocator={alloc}, rustc={rustc}, target={tgt}, host={host}, profile={prof}, git={sha}{dirty}, built={ts})",
        ver = build_info::CRATE_VERSION,
        alloc = allocator::name(),
        rustc = build_info::RUSTC_VERSION,
        tgt = build_info::TARGET_TRIPLE,
        host = build_info::HOST_TRIPLE,
        prof = build_info::PROFILE,
        sha = sha8,
        dirty = dirty,
        ts = build_info::BUILD_TIMESTAMP,
    );
}

fn main() -> Result<()> {
    print_version_banner();
    allocator::assert_mutual_exclusion();

    let cli = Cli::parse();
    match cli.cmd {
        None | Some(Cmd::Version) => Ok(()),
        Some(Cmd::Multithread { .. }) => {
            // Phase 1 Plan 02 fills this in. Walking Skeleton: not yet implemented.
            eprintln!("error: `multithread` subcommand is implemented in Plan 02");
            std::process::exit(2);
        }
    }
}
```
</action>

<acceptance_criteria>
- File `crates/alloc-bench-cli/src/main.rs` exists
- `cargo build -p alloc-bench-cli` (default features) succeeds on macOS host
- `target/debug/alloc-bench-cli` runs and prints the banner to stderr matching the regex `^alloc-bench v\d+\.\d+\.\d+ \(allocator=\w+, rustc=\d+\.\d+\.\d+, target=\S+, host=\S+, profile=(debug|release), git=[a-f0-9]{1,8}(-dirty)?, built=\d{4}-\d{2}-\d{2}T`
- `cargo run -p alloc-bench-cli -- version` exits 0
- `cargo run -p alloc-bench-cli -- multithread` exits 2 with "implemented in Plan 02" message
</acceptance_criteria>

### Task 6: alloc-bench-aggregator placeholder

<read_first>
- crates/alloc-bench-aggregator (just created, empty)
- .planning/phases/01-foundation-mvp-slice/01-CONTEXT.md (D-01: aggregator placeholder for Phase 4)
</read_first>

<action>
Write `crates/alloc-bench-aggregator/Cargo.toml`:
```toml
[package]
name = "alloc-bench-aggregator"
version = "0.1.0"
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[[bin]]
name = "alloc-bench-aggregator"
path = "src/main.rs"
```

Write `crates/alloc-bench-aggregator/src/main.rs`:
```rust
fn main() {
    eprintln!("alloc-bench-aggregator: not yet implemented (Phase 4 — see .planning/ROADMAP.md)");
    std::process::exit(0);
}
```
</action>

<acceptance_criteria>
- File `crates/alloc-bench-aggregator/Cargo.toml` exists
- File `crates/alloc-bench-aggregator/src/main.rs` exists
- `cargo build --workspace` succeeds (compiles all three crates)
- `cargo run -p alloc-bench-aggregator` exits 0 with the placeholder message on stderr
</acceptance_criteria>

### Task 7: Smoke verification

<read_first>
- All files written by tasks 1-6
</read_first>

<action>
Run smoke checks:
```bash
cargo build --workspace --release
target/release/alloc-bench-cli --version 2>&1 | head -2
target/release/alloc-bench-aggregator
```

If LTO=fat causes issues on macOS host (which it shouldn't but has historical quirks), document and continue with the warning.

Run `cargo clippy --workspace -- -D warnings` and fix any clippy errors.

Run `cargo fmt --all` and verify the diff is clean.
</action>

<acceptance_criteria>
- `cargo build --workspace --release` exits 0
- `cargo build --workspace --release --no-default-features --features=alloc-bench-cli/alloc-jemalloc` exits 0 OR cleanly fails on macOS-glibc unavailability with documented reason (jemalloc on macOS uses different code paths; this is acceptable for Phase 1 host build — Phase 3 Docker matrix is the real target)
- `target/release/alloc-bench-cli` prints the banner on first stderr line
- `cargo clippy --workspace --all-targets` passes with `-D warnings`
- `cargo fmt --all --check` exits 0
</acceptance_criteria>

## Verification

After all tasks: commit with `feat(01): walking skeleton — workspace + allocator + build metadata`. Phase 1 Plan 02 then fills the harness, metrics, and multithread scenario.

## Risks

- **vergen-gitcl + Cargo.lock interaction:** vergen needs git access; verify the build.rs runs cleanly in the CI environment (Phase 5 concern but verifying locally now).
- **LTO=fat on macOS:** Some macOS toolchains historically had issues with fat LTO on certain crates. If `cargo build --release` fails, fall back to `lto = "thin"` for the host build profile and document.
- **mimalloc 0.1.x on macOS host:** mimalloc on macOS may have different compile flags; the Walking Skeleton smoke-tests system + jemalloc; mimalloc tests run in Phase 3 Docker.

## Dependencies

- None — this is the foundation plan.

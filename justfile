# alloc-bench Justfile — discoverable recipes for build, bench, and CI gates.
#
# Run `just --list` to see every available recipe.

# Default recipe: list all recipes when `just` is run with no args.
default:
    @just --list

# Verify allocation calls survive --release --emit=llvm-ir (DCE gate).
# Phase-2 ROADMAP success criterion 4. Wraps scripts/dce_check.sh which
# greps the produced .ll files for `__rust_alloc` call sites.
#
# Usage:
#   just dce-check           # default: system allocator (libmalloc/ptmalloc/mallocng)
#   just dce-check system    # explicit
#   just dce-check jemalloc  # requires --features alloc-jemalloc to link cleanly (Linux Docker)
#   just dce-check mimalloc  # requires --features alloc-mimalloc to link cleanly (Linux Docker)
dce-check ALLOCATOR='system':
    @bash scripts/dce_check.sh {{ALLOCATOR}}

# Run the smoke test for the run-all command — produces 10 records.
run-all-smoke OUTPUT='/tmp/alloc-bench-runall.json':
    cargo build --release --bin alloc-bench-cli
    target/release/alloc-bench-cli run-all --output {{OUTPUT}} --seed 7
    @echo "--- Run summary ---"
    @jq '[.[] | {name: .scenario.name, status: .status, ticks_per_s: .metrics.ticks_per_s}]' {{OUTPUT}}

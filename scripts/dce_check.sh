#!/usr/bin/env bash
# DCE verification — Phase-2 ROADMAP success criterion 4.
#
# Builds alloc-bench-cli with `--release --emit=llvm-ir` and verifies that
# the Rust allocation shim (`__rust_alloc`) appears in the LLVM IR. If the
# threshold (10 call sites — one per scenario as a healthy floor) is not
# met, the scenarios were elided by LLVM's release-mode DCE and the
# benchmark would silently report "0 allocs/s" on every workload.
#
# Usage:
#   bash scripts/dce_check.sh              # defaults to "system" allocator
#   bash scripts/dce_check.sh system       # no-default-features (libmalloc/ptmalloc/mallocng)
#   bash scripts/dce_check.sh jemalloc     # --features alloc-jemalloc
#   bash scripts/dce_check.sh mimalloc     # --features alloc-mimalloc
#
# NOTE: jemalloc + mimalloc cross-features may fail to link cleanly on macOS
# host. The canonical run is in Phase-3 Linux Docker. The "system" path is
# the host smoke that catches obvious LLVM-IR regressions across all
# platforms — RESEARCH.md confirms macOS LLVM IR is essentially identical
# to Linux for this purpose (the `__rust_alloc` shim is platform-stable).

set -euo pipefail

ALLOC=${1:-system}
case "$ALLOC" in
    jemalloc)
        FEATURE_FLAGS=(--no-default-features --features alloc-jemalloc)
        ;;
    mimalloc)
        FEATURE_FLAGS=(--no-default-features --features alloc-mimalloc)
        ;;
    system)
        FEATURE_FLAGS=(--no-default-features)
        ;;
    *)
        echo "ERROR: unknown allocator '$ALLOC' — expected one of: jemalloc, mimalloc, system" >&2
        exit 1
        ;;
esac

echo "=== DCE check: allocator=$ALLOC ==="
echo "[1/3] Cleaning prior LLVM-IR artifacts..."
rm -f target/release/deps/alloc_bench_cli-*.ll

echo "[2/3] Building with --release --emit=llvm-ir..."
# RESEARCH.md §"DCE Verification": cargo rustc --release ... -- --emit=llvm-ir
# emits .ll files into target/release/deps/. We capture the output and tail
# the last 5 lines so a build failure is immediately visible.
cargo rustc \
    --release \
    -p alloc-bench-cli \
    "${FEATURE_FLAGS[@]}" \
    --bin alloc-bench-cli \
    -- --emit=llvm-ir 2>&1 | tail -10

echo "[3/3] Searching for __rust_alloc call sites..."
# RESEARCH.md §"Grep gate hygiene": -h suppresses filenames so awk can
# cleanly sum per-file counts. Globbing the * is the canonical pattern
# because the hash suffix on alloc_bench_cli-<hash>.ll changes every build.
shopt -s nullglob
LL_FILES=(target/release/deps/alloc_bench_cli-*.ll)
shopt -u nullglob

if [ ${#LL_FILES[@]} -eq 0 ]; then
    echo "FAIL: no LLVM-IR files found at target/release/deps/alloc_bench_cli-*.ll" >&2
    echo "      Did the build actually emit IR? Re-run with verbose cargo to debug." >&2
    exit 1
fi

# RESEARCH.md §"Grep gate hygiene": grep -h then sum; -c on multiple files
# returns counts per-file separated by colons which awk can also sum, but
# -h keeps the output cleaner.
COUNT=$(grep -h 'call.*__rust_alloc' "${LL_FILES[@]}" | wc -l | tr -d ' ')

echo
echo "  Files inspected: ${#LL_FILES[@]}"
echo "  __rust_alloc call sites: $COUNT"
echo

# Threshold: >= 10 (one per run-all scenario, with healthy margin). Below
# this, the release build elided enough of the scenario surface to make
# the benchmark report meaningless allocator stats.
THRESHOLD=10
if [ "$COUNT" -lt "$THRESHOLD" ]; then
    echo "FAIL: only $COUNT __rust_alloc calls survived (expected >= $THRESHOLD)" >&2
    echo "      The release build's DCE elided allocations from one or more" >&2
    echo "      scenarios. Phase-1 mitigates this with std::hint::black_box on" >&2
    echo "      every tick(); per-scenario buffers also need black_box wrappers." >&2
    exit 1
fi

echo "PASS: $COUNT __rust_alloc calls survived release-mode DCE (threshold: >= $THRESHOLD)"
exit 0

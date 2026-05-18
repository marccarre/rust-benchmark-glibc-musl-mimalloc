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
# CR-03 (Phase-2 review): use a separate target dir so cargo's
# incremental fingerprint doesn't skip the rustc step that produces
# `--emit=llvm-ir`. Reusing `target/` could result in `cargo rustc ...
# -- --emit=llvm-ir` being a no-op (the binary may already be cached
# without IR), in which case the gate inspects no IR at all OR — worse
# — it inspects stale IR left over from a prior invocation that used
# different feature flags.
TARGET_DIR="target/dce-check-${ALLOC}"
LL_GLOB="${TARGET_DIR}/release/deps/alloc_bench_cli-*.ll"

echo "[1/3] Cleaning prior LLVM-IR artifacts (target dir: ${TARGET_DIR})..."
# Always clean BOTH the per-allocator target dir AND the legacy
# target/release/deps location so stale artifacts from older
# invocations of this script (which wrote to target/) cannot pass the
# gate by accident.
rm -f ${LL_GLOB}
rm -f target/release/deps/alloc_bench_cli-*.ll

echo "[2/3] Building with --release --emit=llvm-ir (target-dir=${TARGET_DIR})..."
# CR-03 (Phase-2 review): do NOT pipe to `tail` — that hides cargo's
# error chain (the user sees the trailing 10 lines, which are usually
# benign trailing diagnostics rather than the root error). Stream
# cargo's full output to stderr/stdout. CI logs already truncate.
# RESEARCH.md §"DCE Verification": cargo rustc --release ...
# -- --emit=llvm-ir emits .ll files into <target-dir>/release/deps/.
cargo rustc \
    --release \
    --target-dir "${TARGET_DIR}" \
    -p alloc-bench-cli \
    "${FEATURE_FLAGS[@]}" \
    --bin alloc-bench-cli \
    -- --emit=llvm-ir

echo "[3/3] Searching for __rust_alloc call sites..."
# RESEARCH.md §"Grep gate hygiene": -h suppresses filenames so awk can
# cleanly sum per-file counts. Globbing the * is the canonical pattern
# because the hash suffix on alloc_bench_cli-<hash>.ll changes every build.
shopt -s nullglob
LL_FILES=(${LL_GLOB})
shopt -u nullglob

if [ ${#LL_FILES[@]} -eq 0 ]; then
    echo "FAIL: no LLVM-IR files found at ${LL_GLOB}" >&2
    echo "      Did cargo actually emit IR? Re-run with 'cargo rustc -v' to debug." >&2
    echo "      A common cause is cargo's incremental cache skipping the" >&2
    echo "      rustc step — using --target-dir target/dce-check-* should" >&2
    echo "      avoid that, but a manual 'cargo clean -p alloc-bench-cli'" >&2
    echo "      may be needed if the cache is corrupt." >&2
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

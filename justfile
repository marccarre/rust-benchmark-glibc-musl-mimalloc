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

# ──────────────────────────────────────────────────────────────────────
# Phase 3: Docker matrix recipes (D-04, D-10, D-15, D-17 + RESEARCH §Pitfall 3/4)
# ──────────────────────────────────────────────────────────────────────

# Build one cell. Validates (env, alloc) and HARD-REJECTS cross-libc combos
# BEFORE invoking docker (D-04). Apple Silicon dev boxes default to arm64 —
# every docker buildx invocation must pass --platform linux/amd64 (Pitfall 3).
#
# Usage:
#   just build alpine jemalloc          # → alloc-bench:jemalloc-alpine
#   just build debian-slim ptmalloc     # → alloc-bench:ptmalloc-debian-slim
#   just build debian-slim mallocng     # ERROR: cross-libc rejected
#
# Caveat (Warning 8): if `docker buildx build --load` fails on a buildx setup
# without the docker driver, run `docker buildx use desktop-linux` first.
build env alloc:
    #!/usr/bin/env bash
    set -euo pipefail
    # D-04 cross-libc rejection — HARD ERROR before any docker invocation.
    case "{{env}}-{{alloc}}" in
        debian-slim-mallocng|distroless-cc-mallocng|wolfi-mallocng)
            echo "[ERR] mallocng is the musl libc allocator; cannot run on glibc env '{{env}}'" >&2
            exit 1
            ;;
        alpine-ptmalloc|distroless-static-ptmalloc|scratch-ptmalloc)
            echo "[ERR] ptmalloc is the glibc libc allocator; cannot run on musl env '{{env}}'" >&2
            exit 1
            ;;
    esac
    # Map env → target triple.
    case "{{env}}" in
        debian-slim|distroless-cc|wolfi)
            TARGET="x86_64-unknown-linux-gnu"
            ;;
        alpine|distroless-static|scratch)
            TARGET="x86_64-unknown-linux-musl"
            ;;
        *)
            echo "[ERR] unknown env '{{env}}'" >&2
            exit 1
            ;;
    esac
    # Compute OCI annotation values from authoritative sources.
    OCI_VERSION=$(grep -m1 '^version' crates/alloc-bench-cli/Cargo.toml | cut -d'"' -f2)
    OCI_REVISION=$(git rev-parse HEAD)
    OCI_CREATED=$(date -u +%Y-%m-%dT%H:%M:%SZ)
    docker buildx build \
        --platform linux/amd64 \
        -f docker/{{env}}.Dockerfile \
        --build-arg ALLOC={{alloc}} \
        --build-arg TARGET="$TARGET" \
        --build-arg RUST_VERSION=1.91 \
        --build-arg OCI_VERSION="$OCI_VERSION" \
        --build-arg OCI_REVISION="$OCI_REVISION" \
        --build-arg OCI_CREATED="$OCI_CREATED" \
        --tag alloc-bench:{{alloc}}-{{env}} \
        --load .

# Run one cell. Mounts ./results read-write, applies cgroup + cpuset defaults
# (D-15). Pre-creates ./results with 0777 perms so distroless `:nonroot`
# (UID 65532) can write the per-cell JSON (RESEARCH §Pitfall 4).
#
# Override knobs (D-17):
#   BENCH_CPUS=8 just run alpine jemalloc          # default 4
#   BENCH_MEMORY=8g just run debian-slim ptmalloc  # default 4g
#   BENCH_CPUSET=4-7 just run wolfi mimalloc       # default 0-3
run env alloc:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p results
    chmod 0777 results
    : "${BENCH_CPUS:=4}"
    : "${BENCH_MEMORY:=4g}"
    : "${BENCH_CPUSET:=0-3}"
    docker run --rm \
        --platform linux/amd64 \
        --cpus="${BENCH_CPUS}" --memory="${BENCH_MEMORY}" --cpuset-cpus="${BENCH_CPUSET}" \
        -v "$(pwd)/results:/out" \
        alloc-bench:{{alloc}}-{{env}} \
        run-all --output /out/{{alloc}}-{{env}}.json --seed 7

# Build + run one cell sequentially.
bench-cell env alloc:
    just build {{env}} {{alloc}}
    just run {{env}} {{alloc}}

# Remove all alloc-bench:* image tags. The {{ "{{" }} below is just's
# escaping for emitting a literal `{{` into the shell — `{{` is the
# variable-interpolation marker. `xargs -r` avoids invoking `docker rmi`
# with zero arguments when no matching images exist.
clean-images:
    #!/usr/bin/env bash
    set -uo pipefail
    docker images --filter "reference=alloc-bench:*" --format '{{ "{{" }}.Repository{{ "}}" }}:{{ "{{" }}.Tag{{ "}}" }}' \
        | xargs -r docker rmi -f

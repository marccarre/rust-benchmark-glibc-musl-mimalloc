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
    # UAT Phase 5.1 Gap 1: Apple Silicon Rosetta cannot reliably execute the AVX2/BMI2
    # instructions emitted by the Phase 3 D-09 `target-cpu=x86-64-v3` baseline; cells
    # SIGSEGV (exit 139) on launch. Honor a user-supplied BENCH_TARGET_CPU override
    # OR auto-detect Apple Silicon (arm64+Darwin → x86-64-v2) and forward the result
    # to the Dockerfile via --build-arg RUSTFLAGS_OVERRIDE. Empty override → Dockerfile
    # default (`-C target-cpu=x86-64-v3`) fires unchanged → v1.0 CI invariant preserved.
    : "${BENCH_TARGET_CPU:=}"
    if [ -z "$BENCH_TARGET_CPU" ] && [ "$(uname -m)" = "arm64" ] && [ "$(uname -s)" = "Darwin" ]; then
        BENCH_TARGET_CPU="x86-64-v2"
        echo "[apple-silicon] auto-detected; using target-cpu=x86-64-v2 (override BENCH_TARGET_CPU=... to change)" >&2
    fi
    # Compute the override string. Contract (a): the Dockerfile's ${RUSTFLAGS_OVERRIDE:-<default>}
    # REPLACES the entire RUSTFLAGS string when non-empty, so for distroless-static + scratch
    # we must re-append `-C target-feature=+crt-static` here (the static-libc Dockerfiles need
    # +crt-static or the binary won't link against the empty-libc runtime).
    if [ -n "$BENCH_TARGET_CPU" ]; then
        case "{{env}}" in
            distroless-static|scratch)
                RUSTFLAGS_OVERRIDE="-C target-cpu=${BENCH_TARGET_CPU} -C target-feature=+crt-static"
                ;;
            *)
                RUSTFLAGS_OVERRIDE="-C target-cpu=${BENCH_TARGET_CPU}"
                ;;
        esac
    else
        RUSTFLAGS_OVERRIDE=""
    fi
    # Compute OCI annotation values from authoritative sources.
    OCI_VERSION=$(grep -m1 '^version' crates/alloc-bench-cli/Cargo.toml | cut -d'"' -f2)
    OCI_REVISION=$(git rev-parse HEAD)
    OCI_CREATED=$(date -u +%Y-%m-%dT%H:%M:%SZ)
    docker buildx build \
        --platform linux/amd64 \
        -f docker/{{env}}.Dockerfile \
        --build-arg ALLOC={{alloc}} \
        --build-arg TARGET="$TARGET" \
        --build-arg RUST_VERSION=1.95 \
        --build-arg RUSTFLAGS_OVERRIDE="$RUSTFLAGS_OVERRIDE" \
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

# The 18-cell hard-coded valid (env, alloc) tuple list (D-01, D-04). Cross-libc
# combos (mallocng on glibc, ptmalloc on musl) are STRUCTURALLY ABSENT — D-04's
# hard-skip is encoded by omission, not by runtime conditionals.
#
# Order: glibc family first (3 envs × 3 allocs = 9 cells), then musl family
# (3 envs × 3 allocs = 9 cells). Grouping by libc lets the BuildKit cache reuse
# the chef base layer across same-family cells.
_matrix_cells := '''
debian-slim ptmalloc
debian-slim jemalloc
debian-slim mimalloc
distroless-cc ptmalloc
distroless-cc jemalloc
distroless-cc mimalloc
wolfi ptmalloc
wolfi jemalloc
wolfi mimalloc
alpine mallocng
alpine jemalloc
alpine mimalloc
distroless-static mallocng
distroless-static jemalloc
distroless-static mimalloc
scratch mallocng
scratch jemalloc
scratch mimalloc
'''

# Build + run the full 18-cell matrix sequentially (D-11), with per-cell error
# capture so a single broken cell doesn't abort the rest (Discretion). Per-cell
# logs are streamed with `[<alloc>-<env>]` prefix. Ends with the D-12 stdout
# summary table: `alloc env status ticks_per_s_p50` (jq the multithread
# scenario's metrics.ticks_per_s).
#
# Sequential is mandatory: parallel cells would multiplex allocators in the
# same kernel page cache + thermal envelope, polluting measurements
# (PITFALLS §1.3 spirit).
bench-all:
    #!/usr/bin/env bash
    set -uo pipefail   # NOT -e — we want to continue past per-cell failures.
    declare -a results=()
    while IFS= read -r line; do
        [[ -z "$line" ]] && continue
        env="${line%% *}"
        alloc="${line##* }"
        echo
        echo "════════════════════════════════════════════════════════"
        echo "[${alloc}-${env}] starting"
        echo "════════════════════════════════════════════════════════"
        if just bench-cell "$env" "$alloc" 2>&1 | sed "s/^/[${alloc}-${env}] /"; then
            results+=("OK   ${alloc}-${env}")
        else
            results+=("FAIL ${alloc}-${env}")
        fi
    done <<< '{{_matrix_cells}}'
    echo
    echo "════════════════════════════════════════════════════════"
    echo "Matrix summary"
    echo "════════════════════════════════════════════════════════"
    printf '%s\n' "${results[@]}"
    echo
    echo "alloc env status ticks_per_s_p50"
    while IFS= read -r line; do
        [[ -z "$line" ]] && continue
        env="${line%% *}"
        alloc="${line##* }"
        json="results/${alloc}-${env}.json"
        # Warning 9: an existing JSON file alone is not enough — check the run
        # produced at least 8 of 10 expected scenarios (run-all emits 10 per
        # crates/alloc-bench-cli/src/run.rs default_scenarios; allow ≤ 2
        # per-scenario failures before marking the cell as FAIL).
        if [[ -f "$json" ]] && [ "$(jq 'length' "$json")" -ge 8 ]; then
            tps=$(jq -r '[.[] | select(.scenario.name=="multithread") | .metrics.ticks_per_s] | first // "n/a"' "$json")
            echo "${alloc} ${env} ok ${tps}"
        else
            echo "${alloc} ${env} FAIL -"
        fi
    done <<< '{{_matrix_cells}}'

# Smoke variant — same loop, same per-cell defaults. Phase-2 run-all already
# defaults to warmup=1s + duration=5s per scenario (see
# crates/alloc-bench-cli/src/run.rs default_scenarios), so this recipe's
# contract is "the matrix runs end-to-end fast enough to iterate." BENCH_SMOKE
# is reserved for future per-scenario flag overrides; today it has no effect
# beyond signalling intent in shell history. (D-13 — see SUMMARY for rationale.)
bench-all-smoke:
    #!/usr/bin/env bash
    set -uo pipefail
    BENCH_SMOKE=1 just bench-all

# Convenience recipe for Apple Silicon hosts — sets BENCH_TARGET_CPU=x86-64-v2 to
# avoid the Rosetta+v3 SIGSEGV (UAT Phase 5.1 Gap 1; see README §Troubleshooting).
# The build recipe also auto-detects arm64+Darwin and applies the same v2 default,
# so this wrapper is purely a discoverable `just --list` entry — both
# `just bench-all-smoke` and `just bench-all-smoke-apple-silicon` produce
# identical Apple-Silicon behavior.
bench-all-smoke-apple-silicon:
    #!/usr/bin/env bash
    set -uo pipefail
    BENCH_TARGET_CPU=x86-64-v2 just bench-all-smoke

# Native macOS / Linux host bench — libmalloc / ptmalloc baseline (D-18, D-19).
# No Docker. .cargo/config.toml's `target-cpu=native` is honored automatically;
# Cargo picks the host triple. Output is `results/host-system.json` (D-18
# literal filename). Prints the host triple via `rustc -vV` for traceability.
bench-host:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p results
    cargo build --release -p alloc-bench-cli
    target/release/alloc-bench-cli run-all --output results/host-system.json --seed 7
    HOST=$(rustc -vV | awk '/^host:/ {print $2}')
    echo "[host] target=$HOST"
    echo "[host] wrote results/host-system.json"

# dive image-efficiency check for one cell (D-14, DOCK-07). Falls back to the
# dockerized `wagoodman/dive:latest` image if `dive` isn't on host PATH.
# Warning 10: --platform linux/amd64 on the dockerized fallback mirrors the
# build/run recipes — keeps everything coherent on Apple Silicon dev boxes.
dive-check env alloc:
    #!/usr/bin/env bash
    set -euo pipefail
    if command -v dive >/dev/null 2>&1; then
        dive --ci alloc-bench:{{alloc}}-{{env}} --ci-config .dive-ci
    else
        docker run --rm \
            --platform linux/amd64 \
            -v /var/run/docker.sock:/var/run/docker.sock \
            -v "$(pwd)/.dive-ci:/.dive-ci:ro" \
            wagoodman/dive:latest \
            --ci alloc-bench:{{alloc}}-{{env}} --ci-config /.dive-ci
    fi

# Run dive against every image in the matrix. Per-cell error capture so a
# single failing cell doesn't abort the rest of the gate.
dive-check-all:
    #!/usr/bin/env bash
    set -uo pipefail
    declare -a results=()
    while IFS= read -r line; do
        [[ -z "$line" ]] && continue
        env="${line%% *}"
        alloc="${line##* }"
        echo "[dive] ${alloc}-${env}"
        if just dive-check "$env" "$alloc"; then
            results+=("OK   ${alloc}-${env}")
        else
            results+=("FAIL ${alloc}-${env}")
        fi
    done <<< '{{_matrix_cells}}'
    echo
    echo "dive-check summary"
    printf '%s\n' "${results[@]}"

# Verify _matrix_cells has exactly 18 valid (env, alloc) tuples and contains
# zero cross-libc combos (Warning 7 reconciliation — replaces the fragile
# inline awk pipeline a verify block would otherwise need). Recipe-form is
# cleaner because `just check-matrix` is callable both interactively and
# from CI / pre-commit.
check-matrix:
    #!/usr/bin/env bash
    set -euo pipefail
    BODY=$(awk "/^_matrix_cells := '''$/{flag=1;next} /^'''$/{flag=0} flag" justfile)
    VALID=$(printf "%s\n" "$BODY" | grep -cE '^(debian-slim|distroless-cc|wolfi|alpine|distroless-static|scratch) (ptmalloc|jemalloc|mimalloc|mallocng)$' || true)
    if [ "$VALID" -ne 18 ]; then
        echo "[ERR] _matrix_cells has $VALID valid tuples; expected 18" >&2
        exit 1
    fi
    INVALID=$(printf "%s\n" "$BODY" | grep -E '^(debian-slim|distroless-cc|wolfi) mallocng$|^(alpine|distroless-static|scratch) ptmalloc$' || true)
    if [ -n "$INVALID" ]; then
        echo "[ERR] _matrix_cells contains forbidden cross-libc tuple(s):" >&2
        echo "$INVALID" >&2
        exit 1
    fi
    echo "[ok] _matrix_cells: 18 valid (env, alloc) tuples; zero cross-libc."

# ──────────────────────────────────────────────────────────────────────
# Phase 4: Aggregator (ORCH-03, AGG-01).
# ──────────────────────────────────────────────────────────────────────

# Aggregate results/*.json into report/index.html + report/REPORT.md (D-18).
# Reads results/*.json (the flat layout from Phase 3 D-03), emits both files
# into report/. Pinned Plotly 2.35.3 CDN URL + SRI integrity hash baked into
# index.html (RESEARCH §Pitfall 4). Suspect-run flagging at the
# samples_count<10000 OR warmup_duration_s<5.0 thresholds (D-07) lands in
# Plan 02; Plan 01 ships the end-to-end loop + skeleton template.
aggregate:
    cargo run --release -p alloc-bench-aggregator -- \
        --input "results/*.json" --output report/

# Smoke variant — runs the aggregator integration tests against committed
# fixtures (D-17). Useful for prek pre-commit gate: catches a broken template
# / loader regression before push.
aggregate-smoke:
    cargo test --release -p alloc-bench-aggregator --test smoke

# ──────────────────────────────────────────────────────────────────────
# Phase 5: CI recipes (D-13, D-19, RESEARCH §Pattern 4)
# ──────────────────────────────────────────────────────────────────────

# CI variant of bench-cell: build + dive-check + 3 seeded runs + meta.json
# sidecar. Used by the GHA matrix workflow's bench-matrix job (Phase 5
# D-13 / D-19, RESEARCH §Pattern 4). The 3-seed loop matches CONTEXT.md
# `<specifics>` ¶2 (`--seed 1`, `--seed 2`, `--seed 3`); the meta.json
# sidecar carries `image_size_bytes` + `image_size_mb` so the aggregator
# can populate the Docker runtimes table without modifying the locked v1
# JSON schema (D-14 / D-20). Cgroup / cpuset / memory invariants are
# Phase 3 D-15 locked (4 vCPUs, 4 GiB memory, cpuset 0-3).
#
# Usage:
#   just ci-bench-cell debian-slim ptmalloc   # → results/ptmalloc-debian-slim-seed{1,2,3}.json + meta/ptmalloc-debian-slim.json
#   just ci-bench-cell alpine jemalloc        # → results/jemalloc-alpine-seed{1,2,3}.json     + meta/jemalloc-alpine.json
ci-bench-cell env alloc:
    #!/usr/bin/env bash
    set -euo pipefail
    just build {{env}} {{alloc}}
    just dive-check {{env}} {{alloc}}
    mkdir -p results meta
    chmod 0777 results
    SIZE_BYTES=$(docker image inspect alloc-bench:{{alloc}}-{{env}} --format '{{ "{{" }}.Size{{ "}}" }}')
    SIZE_MB=$(awk "BEGIN { printf \"%.2f\", $SIZE_BYTES / 1024 / 1024 }")
    jq -n --argjson b "$SIZE_BYTES" --argjson m "$SIZE_MB" \
      '{
         alloc:            "{{alloc}}",
         env:              "{{env}}",
         image_size_bytes: $b,
         image_size_mb:    $m,
         captured_at:      now | todate
       }' > meta/{{alloc}}-{{env}}.json
    for seed in 1 2 3; do
      docker run --rm \
        --platform linux/amd64 \
        --cpus=4 --memory=4g --cpuset-cpus=0-3 \
        -v "$(pwd)/results:/out" \
        alloc-bench:{{alloc}}-{{env}} \
        run-all --output /out/{{alloc}}-{{env}}-seed${seed}.json --seed ${seed}
    done

# CI sanity gate — fmt + clippy + dce-check on a clean tree. Mirrors the
# prek pre-commit hook so CI catches the same regressions before invoking
# the matrix. Wired into the GHA `pre-bench` job (Phase 5 D-19).
#
# Usage:
#   just ci-validate                          # runs all three gates sequentially
ci-validate:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    just dce-check system

# CI variant of aggregate — also picks up sidecar meta.json files written
# by ci-bench-cell (Phase 5 D-13). Used by the GHA aggregate job.
#
# The `--meta "meta/*.json"` flag is what differentiates this from the
# Phase-4 `just aggregate` recipe: when CI populates the sidecar files,
# the aggregator joins them on `(alloc, env)` to backfill `image_size_mb`
# in REPORT.md `## Docker runtimes`. Local `just aggregate` invocations
# without sidecars continue to render em-dash placeholders byte-stably.
#
# Usage:
#   just ci-aggregate                         # CI mode — reads results/ + meta/
ci-aggregate:
    cargo run --release -p alloc-bench-aggregator -- \
        --input "results/*.json" --meta "meta/*.json" --output report/

# ──────────────────────────────────────────────────────────────────────
# Pages publishing (quick task 260523-k8f).
# ──────────────────────────────────────────────────────────────────────

# Publish the local Plotly HTML dashboard to GitHub Pages by pushing
# `report/index.html` to the orphan `gh-pages` branch.
#
# Why this exists: `report/` is .gitignored on `main` (so the rendered
# dashboard never enters the main-branch history), and the project
# deliberately ships no GHA Pages workflow — the publish step is a
# one-shot, opt-in, manual recipe. The orphan `gh-pages` branch carries
# nothing but `index.html` at its root, so GitHub Pages serves it as
# the site's landing page at:
#   https://marccarre.github.io/rust-benchmark-glibc-musl-mimalloc/
#
# Prerequisite: run `just aggregate` first to (re)generate
# `report/index.html`. This recipe will NOT auto-aggregate; it fails
# fast with a clear error if `report/index.html` is missing.
#
# Idempotency: works whether the `gh-pages` branch exists nowhere, only
# on `origin`, or both locally and remotely. The recipe uses a temporary
# `git worktree` to avoid disturbing the user's working tree, and an EXIT
# trap to remove the worktree even on partial failure or Ctrl-C.
#
# Re-running with no dashboard changes is a no-op-but-success (prints
# `[publish-pages] no changes to publish`).
#
# Usage:
#   just publish-pages
publish-pages:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ ! -f report/index.html ]; then
        echo "[ERR] report/index.html does not exist; run 'just aggregate' first to generate the dashboard." >&2
        exit 1
    fi
    # Capture source ref BEFORE creating the worktree so the values
    # reflect the user's working tree, not the gh-pages worktree.
    SRC_SHA=$(git rev-parse --short HEAD)
    SRC_BRANCH=$(git rev-parse --abbrev-ref HEAD)
    # Allocate the temp worktree path FIRST and register cleanup BEFORE
    # `git worktree add` runs — so a failure between `mktemp` and
    # `worktree add` (or anywhere afterwards, including Ctrl-C) is still
    # cleaned up.
    WORKTREE_DIR=$(mktemp -d -t gh-pages-XXXXXX)
    cleanup() {
        if git worktree list --porcelain 2>/dev/null | grep -q "$WORKTREE_DIR"; then
            git worktree remove --force "$WORKTREE_DIR" 2>/dev/null || true
        fi
        rm -rf "$WORKTREE_DIR"
    }
    trap cleanup EXIT
    # Fetch any remote `gh-pages` so we know its state. The `:gh-pages`
    # form creates the local branch tracking the remote. `|| true` is
    # critical — fresh repos have no remote `gh-pages` yet and the fetch
    # will fail; that's expected.
    git fetch origin gh-pages:gh-pages 2>/dev/null || true
    # Three-case worktree creation, idempotent across all branch states.
    if git show-ref --verify --quiet refs/heads/gh-pages; then
        # Branch exists locally → reuse it.
        git worktree add "$WORKTREE_DIR" gh-pages
    elif git ls-remote --exit-code --heads origin gh-pages >/dev/null 2>&1; then
        # Branch exists on remote but the fetch did not create the local
        # ref (rare; defensive) — bind a new local branch to origin's tip.
        git worktree add -b gh-pages "$WORKTREE_DIR" origin/gh-pages
    else
        # First publish ever — create an orphan branch with no history.
        git worktree add --orphan -b gh-pages "$WORKTREE_DIR"
        # Orphan branches can still inherit the working tree's index in
        # some git versions; clear it so the new commit contains only
        # `index.html`.
        (cd "$WORKTREE_DIR" && git rm -rf . 2>/dev/null || true)
    fi
    # Copy ONLY `report/index.html` — never `report/REPORT.md` or any
    # other file under `report/`. The gh-pages branch is the dashboard,
    # nothing else.
    cp report/index.html "$WORKTREE_DIR/index.html"
    # Commit + push from inside the worktree. The subshell keeps the
    # parent shell's CWD unchanged.
    (
        cd "$WORKTREE_DIR"
        git add index.html
        if git diff --cached --quiet; then
            echo "[publish-pages] no changes to publish (index.html identical to gh-pages HEAD)"
        else
            git commit -m "Publish dashboard from ${SRC_BRANCH}@${SRC_SHA}"
            git push origin gh-pages
        fi
    )
    echo "[publish-pages] published — visit https://marccarre.github.io/rust-benchmark-glibc-musl-mimalloc/ (may take a minute on first publish)"

# syntax=docker/dockerfile:1.7
# scratch.Dockerfile — fully-static musl runtime FROM scratch (smallest image).
# Phase 3 Plan 02 / Task 3.
#
# RESEARCH §Pitfall 5: scratch contains nothing — no /etc, no shell, no
# resolver, no terminfo. The bench's `eprintln!` works (writes to fd 2,
# no library lookup). chrono is configured with `default-features=false`
# in workspace Cargo.toml so no /etc/localtime read; the bench writes
# UTC only. No /etc/passwd or /etc/ssl/certs needed (HTTP-only on
# 127.0.0.1 per PITFALLS §2.3).
#
# We deliberately DO NOT COPY any /etc/* or /etc/ssl/* from the builder
# — keeps the image as small as practical (CONTEXT D-22 budget ≤ 15 MB).
# We deliberately DO NOT add a USER directive — scratch has no
# /etc/passwd; the binary runs as root by default which is fine for a
# benchmark.
ARG RUST_VERSION=1.91

# ─── Stage 1: chef base ────────────────────────────────────────────
FROM rust:${RUST_VERSION}-alpine AS chef
# musl-dev + pkgconfig: workspace base build deps (axum/tokio/reqwest crates).
# make + g++ + cmake + linux-headers + bash + file: required by tikv-jemalloc-sys
# 0.6.1 (autoconf/configure → make pipeline) and libmimalloc-sys 0.1.47 (cmake)
# native build scripts. Plan 03-04 Task 1 deviation Rule 3 (blocking issue):
# rust:1.91-alpine ships gcc but not make/cmake; adding them is a build-time
# requirement only — scratch runtime stage carries nothing.
RUN apk add --no-cache musl-dev pkgconfig make g++ cmake linux-headers bash file
RUN cargo install --locked cargo-chef@0.1.77
WORKDIR /app

# ─── Stage 2: planner — compute recipe.json ────────────────────────
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ─── Stage 3: builder ──────────────────────────────────────────────
FROM chef AS builder
ARG ALLOC=mallocng
ARG TARGET=x86_64-unknown-linux-musl
# REQUIRED: scratch has nothing; binary must be fully static.
# Override mechanism for Apple Silicon Rosetta — see UAT Phase 5.1 / debug/apple-silicon-segfault.md
# When --build-arg RUSTFLAGS_OVERRIDE is absent or empty, the ${VAR:-default} expansion
# falls through to the v3 + crt-static literal (preserves v1.0 CI invariant). When the
# justfile auto-detects Apple Silicon, it forwards a full
# `-C target-cpu=x86-64-v2 -C target-feature=+crt-static` override (the override
# REPLACES the whole RUSTFLAGS string, so +crt-static must be re-included).
ARG RUSTFLAGS_OVERRIDE=""
ENV RUSTFLAGS="${RUSTFLAGS_OVERRIDE:--C target-cpu=x86-64-v3 -C target-feature=+crt-static}"
RUN rustup target add ${TARGET}
COPY --from=planner /app/recipe.json recipe.json
RUN if [ "$ALLOC" = "jemalloc" ]; then \
        FEATURES="--no-default-features --features alloc-jemalloc"; \
    elif [ "$ALLOC" = "mimalloc" ]; then \
        FEATURES="--no-default-features --features alloc-mimalloc"; \
    elif [ "$ALLOC" = "mallocng" ]; then \
        FEATURES=""; \
    else \
        echo "ERROR: musl env only supports ALLOC in {mallocng, jemalloc, mimalloc}; got '$ALLOC'" >&2; exit 1; \
    fi && \
    cargo chef cook --release --target ${TARGET} ${FEATURES} \
        -p alloc-bench-cli --recipe-path recipe.json
COPY . .
RUN if [ "$ALLOC" = "jemalloc" ]; then \
        FEATURES="--no-default-features --features alloc-jemalloc"; \
    elif [ "$ALLOC" = "mimalloc" ]; then \
        FEATURES="--no-default-features --features alloc-mimalloc"; \
    elif [ "$ALLOC" = "mallocng" ]; then \
        FEATURES=""; \
    else \
        echo "ERROR: musl env only supports ALLOC in {mallocng, jemalloc, mimalloc}; got '$ALLOC'" >&2; exit 1; \
    fi && \
    cargo build --release --target ${TARGET} ${FEATURES} \
        -p alloc-bench-cli

# ─── Stage 4: runtime — scratch (smallest possible image) ─────────
FROM scratch AS runtime
ARG OCI_VERSION
ARG OCI_REVISION
ARG OCI_CREATED
LABEL org.opencontainers.image.title="alloc-bench" \
      org.opencontainers.image.description="Memory allocator benchmark — scratch (musl fully-static; mallocng/jemalloc/mimalloc)" \
      org.opencontainers.image.source="https://github.com/marccarre/rust-benchmark-glibc-musl-mimalloc" \
      org.opencontainers.image.version="${OCI_VERSION}" \
      org.opencontainers.image.revision="${OCI_REVISION}" \
      org.opencontainers.image.licenses="MIT OR Apache-2.0" \
      org.opencontainers.image.created="${OCI_CREATED}" \
      org.opencontainers.image.authors="Marc Carré"
ENV DOCKER_IMAGE=scratch
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/alloc-bench-cli /alloc-bench-cli
ENTRYPOINT ["/alloc-bench-cli"]

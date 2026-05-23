# syntax=docker/dockerfile:1.7
# alpine.Dockerfile — musl dynamic runtime (mallocng / jemalloc / mimalloc).
# Phase 3 Plan 02 / Task 1. Builder: rust:1.95-alpine (rust-toolchain.toml=1.95;
# CONTEXT D-06 originally said 1.83 — supersede on the rust-toolchain pin).
ARG RUST_VERSION=1.95

# ─── Stage 1: chef base ────────────────────────────────────────────
FROM rust:${RUST_VERSION}-alpine AS chef
# musl-dev + pkgconfig: workspace base build deps (axum/tokio/reqwest crates).
# make + g++ + cmake + linux-headers + bash + file: required by tikv-jemalloc-sys
# 0.6.1 (autoconf/configure → make pipeline) and libmimalloc-sys 0.1.47 (cmake)
# native build scripts. Plan 03-04 Task 1 deviation Rule 3 (blocking issue):
# rust:1.95-alpine ships gcc but not make/cmake; adding them is a build-time
# requirement, not a runtime concern (the binary is statically/dynamically
# linked against the allocator and doesn't need these tools at runtime).
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
# Alpine is musl-dynamic. Static-linkage flags live in
# distroless-static.Dockerfile and scratch.Dockerfile only.
# Override mechanism for Apple Silicon Rosetta — see UAT Phase 5.1 / debug/apple-silicon-segfault.md
# When --build-arg RUSTFLAGS_OVERRIDE is absent or empty, the ${VAR:-default} expansion
# falls through to `-C target-cpu=x86-64-v3` (preserves v1.0 CI invariant). When the
# justfile auto-detects Apple Silicon (arm64+Darwin), it forwards
# `--build-arg RUSTFLAGS_OVERRIDE="-C target-cpu=x86-64-v2"` so the resulting binary
# does not emit AVX2/BMI2 instructions that Rosetta-2 cannot execute (exit 139).
ARG RUSTFLAGS_OVERRIDE=""
ENV RUSTFLAGS="${RUSTFLAGS_OVERRIDE:--C target-cpu=x86-64-v3}"
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

# ─── Stage 4: runtime — alpine:3.23 (matches success criterion 2 literal) ──
FROM alpine:3.23 AS runtime
ARG OCI_VERSION
ARG OCI_REVISION
ARG OCI_CREATED
LABEL org.opencontainers.image.title="alloc-bench" \
      org.opencontainers.image.description="Memory allocator benchmark — alpine (musl dynamic; mallocng/jemalloc/mimalloc)" \
      org.opencontainers.image.source="https://github.com/marccarre/rust-benchmark-glibc-musl-mimalloc" \
      org.opencontainers.image.version="${OCI_VERSION}" \
      org.opencontainers.image.revision="${OCI_REVISION}" \
      org.opencontainers.image.licenses="MIT OR Apache-2.0" \
      org.opencontainers.image.created="${OCI_CREATED}" \
      org.opencontainers.image.authors="Marc Carré"
ENV DOCKER_IMAGE=alpine:3.23
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/alloc-bench-cli /usr/local/bin/alloc-bench-cli
ENTRYPOINT ["/usr/local/bin/alloc-bench-cli"]

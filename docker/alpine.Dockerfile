# syntax=docker/dockerfile:1.7
# alpine.Dockerfile — musl dynamic runtime (mallocng / jemalloc / mimalloc).
# Phase 3 Plan 02 / Task 1. Builder: rust:1.91-alpine (rust-toolchain.toml=1.91;
# CONTEXT D-06 originally said 1.83 — supersede on the rust-toolchain pin).
ARG RUST_VERSION=1.91

# ─── Stage 1: chef base ────────────────────────────────────────────
FROM rust:${RUST_VERSION}-alpine AS chef
RUN apk add --no-cache musl-dev pkgconfig
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
ENV RUSTFLAGS="-C target-cpu=x86-64-v3"
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

# ─── Stage 4: runtime — alpine:3.20 (matches success criterion 2 literal) ──
FROM alpine:3.20 AS runtime
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
ENV DOCKER_IMAGE=alpine:3.20
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/alloc-bench-cli /usr/local/bin/alloc-bench-cli
ENTRYPOINT ["/usr/local/bin/alloc-bench-cli"]

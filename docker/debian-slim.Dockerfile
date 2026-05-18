# docker/debian-slim.Dockerfile — glibc dynamic runtime on debian:bookworm-slim.
# Builder + runtime libc family: glibc. Target: x86_64-unknown-linux-gnu.
# Pattern: cargo-chef 3-stage (chef → planner → builder) + per-env runtime.
# Source: 03-RESEARCH.md §"Pattern 1" + §"Code Examples §1".

# RUST_VERSION matches rust-toolchain.toml (channel = "1.91"). The original
# CONTEXT D-06 cited 1.83; we use 1.91 because rustup honors rust-toolchain.toml
# at build time anyway — matching the toolchain saves one redundant download.
ARG RUST_VERSION=1.91

# ─── Stage 1: chef base ────────────────────────────────────────────
FROM rust:${RUST_VERSION}-bookworm AS chef
RUN cargo install --locked cargo-chef@0.1.77
WORKDIR /app

# ─── Stage 2: planner — compute recipe.json ────────────────────────
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ─── Stage 3: builder ──────────────────────────────────────────────
FROM chef AS builder
ARG ALLOC=ptmalloc
ARG TARGET=x86_64-unknown-linux-gnu
# Override .cargo/config.toml's target-cpu=native (Pitfall §2). x86-64-v3 is
# portable across modern CI runners; do NOT add +crt-static (glibc dynamic).
ENV RUSTFLAGS="-C target-cpu=x86-64-v3"
RUN rustup target add ${TARGET}
COPY --from=planner /app/recipe.json recipe.json
RUN if [ "$ALLOC" = "jemalloc" ]; then \
        FEATURES="--no-default-features --features alloc-jemalloc"; \
    elif [ "$ALLOC" = "mimalloc" ]; then \
        FEATURES="--no-default-features --features alloc-mimalloc"; \
    elif [ "$ALLOC" = "ptmalloc" ]; then \
        FEATURES=""; \
    else \
        echo "ERROR: glibc env only supports ALLOC in {ptmalloc, jemalloc, mimalloc}; got '$ALLOC'" >&2; exit 1; \
    fi && \
    cargo chef cook --release --target ${TARGET} ${FEATURES} \
        -p alloc-bench-cli --recipe-path recipe.json
COPY . .
RUN if [ "$ALLOC" = "jemalloc" ]; then \
        FEATURES="--no-default-features --features alloc-jemalloc"; \
    elif [ "$ALLOC" = "mimalloc" ]; then \
        FEATURES="--no-default-features --features alloc-mimalloc"; \
    elif [ "$ALLOC" = "ptmalloc" ]; then \
        FEATURES=""; \
    else \
        echo "ERROR: glibc env only supports ALLOC in {ptmalloc, jemalloc, mimalloc}; got '$ALLOC'" >&2; exit 1; \
    fi && \
    cargo build --release --target ${TARGET} ${FEATURES} \
        -p alloc-bench-cli

# ─── Stage 4: runtime — debian:bookworm-slim (glibc dynamic) ───────
FROM debian:bookworm-slim AS runtime
ARG OCI_VERSION
ARG OCI_REVISION
ARG OCI_CREATED
LABEL org.opencontainers.image.title="alloc-bench" \
      org.opencontainers.image.description="Memory allocator benchmark — debian-slim (glibc dynamic; ptmalloc/jemalloc/mimalloc)" \
      org.opencontainers.image.source="https://github.com/marccarre/rust-benchmark-glibc-musl-mimalloc" \
      org.opencontainers.image.version="${OCI_VERSION}" \
      org.opencontainers.image.revision="${OCI_REVISION}" \
      org.opencontainers.image.licenses="MIT OR Apache-2.0" \
      org.opencontainers.image.created="${OCI_CREATED}" \
      org.opencontainers.image.authors="Marc Carré"
ENV DOCKER_IMAGE=debian:bookworm-slim
COPY --from=builder /app/target/x86_64-unknown-linux-gnu/release/alloc-bench-cli /usr/local/bin/alloc-bench-cli
ENTRYPOINT ["/usr/local/bin/alloc-bench-cli"]

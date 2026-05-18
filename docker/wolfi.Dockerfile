# docker/wolfi.Dockerfile — glibc Wolfi runtime on cgr.dev/chainguard/wolfi-base.
# Builder + runtime libc family: glibc. Target: x86_64-unknown-linux-gnu.
# Pattern: cargo-chef 3-stage (chef → planner → builder) + per-env runtime.
# Source: 03-RESEARCH.md §"Pattern 1" + §"Pitfall 6 — wolfi-base mutable-tag mutability".
#
# Wolfi-base manifest-list (OCI image-index) digest captured 2026-05-19 via:
#   docker buildx imagetools inspect cgr.dev/chainguard/wolfi-base@<floating tag> | grep '^Digest:'
# → sha256:0cff4df29a6597173dc8b813787318150141eb96ac783dc3ff4f5ff52c49a1e2
# We pin the manifest-list digest (not the per-arch amd64 manifest) so `--platform
# linux/amd64` resolves cleanly at build time without tripping `--check` on arm64
# hosts (Apple Silicon / OrbStack). Per RESEARCH §Pitfall 6: refresh this digest
# if rebuilding and the wolfi runtime stops linking — but never use the floating
# tag (Plan 04's `dive --ci` relies on bit-identical bases across runs).

# RUST_VERSION matches rust-toolchain.toml (channel = "1.91"). See debian-slim.Dockerfile.
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

# ─── Stage 4: runtime — wolfi-base (glibc, digest-pinned) ──────────
# wolfi-base runs as UID 0 by default per RESEARCH §"Standard Stack" — no USER.
FROM cgr.dev/chainguard/wolfi-base@sha256:0cff4df29a6597173dc8b813787318150141eb96ac783dc3ff4f5ff52c49a1e2 AS runtime
ARG OCI_VERSION
ARG OCI_REVISION
ARG OCI_CREATED
LABEL org.opencontainers.image.title="alloc-bench" \
      org.opencontainers.image.description="Memory allocator benchmark — wolfi (glibc Chainguard; ptmalloc/jemalloc/mimalloc)" \
      org.opencontainers.image.source="https://github.com/marccarre/rust-benchmark-glibc-musl-mimalloc" \
      org.opencontainers.image.version="${OCI_VERSION}" \
      org.opencontainers.image.revision="${OCI_REVISION}" \
      org.opencontainers.image.licenses="MIT OR Apache-2.0" \
      org.opencontainers.image.created="${OCI_CREATED}" \
      org.opencontainers.image.authors="Marc Carré"
ENV DOCKER_IMAGE=cgr.dev/chainguard/wolfi-base@sha256:0cff4df29a6597173dc8b813787318150141eb96ac783dc3ff4f5ff52c49a1e2
COPY --from=builder /app/target/x86_64-unknown-linux-gnu/release/alloc-bench-cli /usr/local/bin/alloc-bench-cli
ENTRYPOINT ["/usr/local/bin/alloc-bench-cli"]

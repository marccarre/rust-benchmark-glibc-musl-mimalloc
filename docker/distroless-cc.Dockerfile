# docker/distroless-cc.Dockerfile — glibc minimal runtime on distroless/cc-debian12 (UID 65532 nonroot).
# Builder + runtime libc family: glibc. Target: x86_64-unknown-linux-gnu.
# Pattern: cargo-chef 3-stage (chef → planner → builder) + per-env runtime.
# Source: 03-RESEARCH.md §"Pattern 1" + §"Pitfall 4 — distroless nonroot".

# RUST_VERSION matches rust-toolchain.toml (channel = "1.95"). Matches the
# toolchain channel so rustup does not download a second toolchain at build
# time (the original CONTEXT D-06 listed 1.83 before the toolchain was bumped).
ARG RUST_VERSION=1.95

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
# Override .cargo/config.toml's target-cpu=native (Pitfall §2). Glibc dynamic.
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

# ─── Stage 4: runtime — distroless/cc-debian12:nonroot (UID 65532) ──
# Pitfall §4: binary at /alloc-bench-cli (FS root); /usr/local/bin is NOT
# guaranteed in PATH for nonroot. Plan 03 host-side `chmod 0777 results`
# handles volume permissions.
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
ARG OCI_VERSION
ARG OCI_REVISION
ARG OCI_CREATED
LABEL org.opencontainers.image.title="alloc-bench" \
      org.opencontainers.image.description="Memory allocator benchmark — distroless-cc (glibc minimal nonroot; ptmalloc/jemalloc/mimalloc)" \
      org.opencontainers.image.source="https://github.com/marccarre/rust-benchmark-glibc-musl-mimalloc" \
      org.opencontainers.image.version="${OCI_VERSION}" \
      org.opencontainers.image.revision="${OCI_REVISION}" \
      org.opencontainers.image.licenses="MIT OR Apache-2.0" \
      org.opencontainers.image.created="${OCI_CREATED}" \
      org.opencontainers.image.authors="Marc Carré"
ENV DOCKER_IMAGE=gcr.io/distroless/cc-debian12:nonroot
COPY --from=builder /app/target/x86_64-unknown-linux-gnu/release/alloc-bench-cli /alloc-bench-cli
USER nonroot
WORKDIR /home/nonroot
ENTRYPOINT ["/alloc-bench-cli"]

# syntax=docker/dockerfile:1.7
# ---------------------------------------------------------------------------
# Kosmocrates pse-server container.
#
# Multi-stage build:
#   1. `chef`     prepares a dependency-only recipe (cargo-chef) so the
#                 dependency graph is cached independently of source edits.
#   2. `planner`  produces the recipe.json from the current workspace.
#   3. `builder`  cooks the dependency cache, then builds the pse-server
#                 binary against the locked workspace.
#   4. `runtime`  is a minimal debian-slim image carrying ca-certificates
#                 and libssl3 (for reqwest's default TLS backend) plus a
#                 non-root user.
#
# Pinning rust-version to the workspace MSRV (1.82) keeps reproducibility.
# ---------------------------------------------------------------------------

ARG RUST_VERSION=1.82
ARG DEBIAN_VERSION=bookworm

# ─── stage 1: chef ─────────────────────────────────────────────────────────
FROM rust:${RUST_VERSION}-slim-${DEBIAN_VERSION} AS chef
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install --locked cargo-chef
WORKDIR /workspace

# ─── stage 2: planner ──────────────────────────────────────────────────────
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ─── stage 3: builder ──────────────────────────────────────────────────────
FROM chef AS builder
COPY --from=planner /workspace/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json --bin pse-server
COPY . .
RUN cargo build --release --locked --bin pse-server

# ─── stage 4: runtime ──────────────────────────────────────────────────────
FROM debian:${DEBIAN_VERSION}-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
        curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 10001 kosmocrates \
    && useradd  --system --uid 10001 --gid kosmocrates --home /var/lib/kosmocrates kosmocrates \
    && mkdir -p /var/lib/kosmocrates \
    && chown -R kosmocrates:kosmocrates /var/lib/kosmocrates

COPY --from=builder /workspace/target/release/pse-server /usr/local/bin/pse-server

USER kosmocrates
WORKDIR /var/lib/kosmocrates

# pse-server reads PSE_SERVER_HOST / PSE_SERVER_PORT from the environment.
# Defaults match the server's own defaults (0.0.0.0:8765).
EXPOSE 8765
ENV PSE_SERVER_HOST=0.0.0.0 \
    PSE_SERVER_PORT=8765

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -fsS "http://127.0.0.1:${PSE_SERVER_PORT}/health" || exit 1

ENTRYPOINT ["/usr/local/bin/pse-server"]

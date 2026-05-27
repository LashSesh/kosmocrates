# Kosmocrates dev tasks. Install with: `cargo install just`
#
# Run with: `just <task>`  (or `just` to list).
# Tasks mirror the gates enforced by .github/workflows/ci.yml so
# `just ci` is the single-command answer to "is this PR-ready".

set shell := ["bash", "-cu"]

default:
    @just --list

# Format check (CI gate).
fmt:
    cargo fmt --all -- --check

# Auto-format every Rust file.
fmt-fix:
    cargo fmt --all

# Lint with clippy at CI strictness.
clippy:
    cargo clippy --workspace --all-targets --locked -- -D warnings

# Full workspace test run (including doc-tests).
test:
    cargo test --workspace --locked --all-targets
    cargo test --workspace --locked --doc

# Doc build with the same flags CI enforces.
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked

# Verify MSRV (matches .github/workflows/msrv.yml).
msrv:
    @msrv=$(grep -E '^rust-version' Cargo.toml | head -n1 | sed -E 's/.*"([0-9.]+)".*/\1/'); \
      echo "MSRV is $$msrv"; \
      rustup toolchain install $$msrv --profile minimal; \
      cargo +$$msrv check --workspace --locked --all-targets

# Supply-chain gates (matches .github/workflows/security.yml).
deny:
    cargo deny check advisories
    cargo deny check licenses
    cargo deny check bans
    cargo deny check sources

# The full CI gate, locally. Run before pushing.
ci: fmt clippy test doc

# 30-second engine smoke test.
demo:
    cargo run --release -p pse-demo

# End-to-end quickstart example with determinism assertion.
quickstart:
    cargo run --release -p pse --example quickstart

# Run the HTTP server with a generated dev token. Bound to localhost.
server-dev:
    PSE_SERVER_HOST=127.0.0.1 \
    PSE_SERVER_TOKEN="$$(openssl rand -hex 16)" \
    RUST_LOG=info \
    cargo run --release -p pse-server

# Build the docker image locally. Requires docker buildx.
docker-build:
    docker build -f docker/Dockerfile -t kosmocrates/pse-server:dev .

# Coverage report (LCOV at coverage/lcov.info). Requires cargo-llvm-cov.
coverage:
    cargo llvm-cov --workspace --locked \
        --lcov --output-path coverage/lcov.info \
        --ignore-filename-regex 'tools/.*|vendors/.*|.*/tests/.*'
    cargo llvm-cov report --summary-only

# Quick wins to clean local state. Does NOT touch git.
clean:
    cargo clean
    rm -rf coverage

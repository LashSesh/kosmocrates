#!/usr/bin/env bash
# Kosmocrates substrate tools — install script
#
# Installs kosmo-substrate and kosmo-server from the current workspace
# or from the upstream git repository.
#
# Usage:
#   ./install.sh                   # install from local workspace (requires Rust)
#   ./install.sh --git             # install from upstream git repo
#   ./install.sh --help

set -euo pipefail

REPO="https://github.com/LashSesh/kosmocrates"
TOOLS=(kosmo-substrate kosmo-server)

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[0;33m'
CYAN='\033[0;36m'; BOLD='\033[1m'; RESET='\033[0m'

info()    { printf "${CYAN}info${RESET}  %s\n" "$*"; }
ok()      { printf "${GREEN}ok${RESET}    %s\n" "$*"; }
warn()    { printf "${YELLOW}warn${RESET}  %s\n" "$*"; }
die()     { printf "${RED}error${RESET} %s\n" "$*" >&2; exit 1; }

# ─── Check Rust ───────────────────────────────────────────────────────────────
if ! command -v cargo &>/dev/null; then
    die "cargo not found. Install Rust via https://rustup.rs then re-run this script."
fi

RUST_VER=$(rustc --version 2>/dev/null | awk '{print $2}')
info "Rust ${RUST_VER} found"

# ─── Parse args ───────────────────────────────────────────────────────────────
FROM_GIT=false
for arg in "$@"; do
    case "$arg" in
        --git)   FROM_GIT=true ;;
        --help|-h)
            cat <<EOF
${BOLD}kosmo install.sh${RESET} — install Kosmocrates substrate tools

Usage:
  ./install.sh          Install from local workspace (default)
  ./install.sh --git    Install from ${REPO}

Installs: ${TOOLS[*]}

After installation, add ~/.cargo/bin to your PATH if not already present.
EOF
            exit 0 ;;
        *)  die "Unknown flag: $arg  (run --help)" ;;
    esac
done

# ─── Install ──────────────────────────────────────────────────────────────────
printf "\n${BOLD}Installing Kosmocrates substrate tools${RESET}\n\n"

if $FROM_GIT; then
    info "Installing from ${REPO}"
    for tool in "${TOOLS[@]}"; do
        info "  cargo install --locked --git ${REPO} ${tool}"
        cargo install --locked --git "${REPO}" "${tool}"
        ok "${tool} installed"
    done
else
    # Detect if we're running from the repo root
    if [[ ! -f "Cargo.toml" ]] || ! grep -q '"tools/kosmo-substrate"' Cargo.toml 2>/dev/null; then
        die "Run this script from the kosmocrates workspace root, or use --git to install from upstream."
    fi
    info "Installing from local workspace"
    for tool in "${TOOLS[@]}"; do
        info "  cargo install --locked --path tools/${tool} ${tool}"
        cargo install --locked --path "tools/${tool}"
        ok "${tool} installed"
    done
fi

# ─── Done ─────────────────────────────────────────────────────────────────────
CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin"
printf "\n${GREEN}${BOLD}Done!${RESET} Installed to ${CARGO_BIN}\n\n"

if [[ ":$PATH:" != *":${CARGO_BIN}:"* ]]; then
    warn "${CARGO_BIN} is not in your PATH."
    warn "Add it:  export PATH=\"\$PATH:${CARGO_BIN}\""
    echo
fi

cat <<EOF
${BOLD}Quick start:${RESET}

  # Analyse a workspace (terminal output)
  kosmo-substrate /path/to/workspace

  # Enable all analysis layers with crystal store persistence
  kosmo-substrate . --all --operator --store ~/.kosmo/cadlib.jsonl

  # Generate a Markdown report for your PR
  kosmo-substrate . --output markdown > TOPOLOGY_REPORT.md

  # CI gate (exit 1 on Reject)
  kosmo-substrate . --output summary --fail-on-reject

  # Start the browser UI
  kosmo-server --open

  # API usage
  curl -s -X POST http://localhost:7777/api/analyse \\
    -H 'Content-Type: application/json' \\
    -d '{"path":".","flags":{"crystals":true}}'

For the interactive TUI:  cargo run -p kosmo-tui -- /path/to/workspace
Docker:  docker build -f docker/Dockerfile.kosmo -t kosmo-substrate .
         docker run --rm -v \$(pwd):/workspace kosmo-substrate
EOF

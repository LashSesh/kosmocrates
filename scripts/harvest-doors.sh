#!/usr/bin/env bash
# Harvest the whole ecosystem's docking surface into one federated,
# content-addressed inventory.
#
# Every door-speaking binary emits its own catalog (CLIs via a `doors`
# subcommand or a `--doors` flag; servers via the same `--doors` flag, so
# nothing has to be started or curled). `kosmo-run --doors-merge` then
# unites them — verifying each by content address — into a single
# inventory whose catalog id is deterministic given the surfaces present.
#
# Usage:
#   scripts/harvest-doors.sh [--release] [--out <file>]
#
# Exit non-zero if any expected surface is missing or fails verification.
set -euo pipefail

PROFILE="debug"
CARGO_PROFILE_FLAG=""
OUT="doors-inventory.json"
while [ $# -gt 0 ]; do
  case "$1" in
    --release) PROFILE="release"; CARGO_PROFILE_FLAG="--release"; shift ;;
    --out) OUT="$2"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
BIN="$ROOT/target/$PROFILE"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# Every surface and how it speaks its catalog: "binary:invocation".
# `doors --json` for subcommand CLIs; `--doors --json` for the rest.
SURFACES=(
  "kosmo-run:--doors --json"
  "kosmo-server:--doors"
  "kosmo-substrate:--doors --output json"
  "kosmo-promote:--doors --json"
  "kosmo-tui:--doors"
  "kosmo-eval:--doors --json"
  "pse:doors --json"
  "pse-server:--doors"
  "pse-metatron:doors --json"
  "pse-eval-matrix:doors --json"
  "pse-validate:doors --json"
  "phase-matrix:doors --json"
  "nxalien:doors --json"
  "pse-traverse-cli:doors --json"
  "pse-traverse-horizon:doors --json"
  "pse-traverse-cognition:doors --json"
  "pse-traverse-topology:doors --json"
  "pse-traverse-lpcm:doors --json"
)

echo "Building the door-speaking binaries ($PROFILE)…" >&2
# One cargo invocation builds every binary we harvest.
cargo build $CARGO_PROFILE_FLAG --locked \
  -p kosmo-run -p kosmo-server -p kosmo-substrate -p kosmo-promote \
  -p kosmo-tui -p kosmo-eval -p pse-cli -p pse-server -p pse-metatron-cli \
  -p pse-eval-matrix-cli -p pse-validation-runner-cli -p pse-phase-matrix-cli \
  -p nxalien-cli -p pse-traverse-cli -p pse-traverse-horizon-cli \
  -p pse-traverse-cognition-cli -p pse-traverse-topology-cli \
  -p pse-traverse-lpcm-cli >&2

FILES=()
MISSING=0
for entry in "${SURFACES[@]}"; do
  name="${entry%%:*}"
  invocation="${entry#*:}"
  exe="$BIN/$name"
  if [ ! -x "$exe" ]; then
    echo "MISSING binary: $name" >&2
    MISSING=1
    continue
  fi
  out="$WORK/$name.json"
  # shellcheck disable=SC2086
  if "$exe" $invocation > "$out" 2>/dev/null && [ -s "$out" ]; then
    FILES+=("$out")
  else
    echo "FAILED to harvest: $name" >&2
    MISSING=1
  fi
done

if [ "$MISSING" -ne 0 ]; then
  echo "harvest incomplete — some surfaces did not answer" >&2
  exit 1
fi

# kosmo-run federates its own catalog with every harvested file, verifying
# each by content address; the merged inventory is written to $OUT.
MERGE="$(IFS=,; echo "${FILES[*]}")"
"$BIN/kosmo-run" --doors-merge "$MERGE" --json > "$OUT"

DOORS=$(grep -c '"door_id"' "$OUT" || true)
CATALOG=$(grep -m1 '"catalog_id"' "$OUT" | grep -oE '[0-9a-f]{16}' | head -1 || true)
echo "harvested ${#FILES[@]} surfaces → $OUT  ($DOORS doors · catalog ${CATALOG}…)" >&2

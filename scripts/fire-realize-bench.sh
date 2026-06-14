#!/usr/bin/env bash
# Fire the realization benchmark against a real provider and write bench.json.
#
# Prerequisites (set in the ENVIRONMENT config, never in chat), then start a
# FRESH session so they take effect:
#   - full network egress (or api.openai.com on the allowlist)
#   - KOSMO_LLM_API_KEY  = your provider key   (secret environment variable)
#
# Optional overrides (sensible OpenAI defaults if unset):
#   - KOSMO_LLM_PROVIDER (default: openai)
#   - KOSMO_LLM_BASE_URL (default: https://api.openai.com/v1)
#   - KOSMO_LLM_MODEL    (default: gpt-5.5-2026-04-23)
#
# The key never appears in this script's output; only its presence is used.
set -euo pipefail
cd "$(dirname "$0")/.."

: "${KOSMO_LLM_PROVIDER:=openai}"
: "${KOSMO_LLM_BASE_URL:=https://api.openai.com/v1}"
: "${KOSMO_LLM_MODEL:=gpt-5.5-2026-04-23}"
export KOSMO_LLM_PROVIDER KOSMO_LLM_BASE_URL KOSMO_LLM_MODEL

if [ -z "${KOSMO_LLM_API_KEY:-}" ]; then
  echo "error: KOSMO_LLM_API_KEY is not set in this session." >&2
  echo "       Add it as a secret env var and start a FRESH session." >&2
  exit 2
fi

echo "provider=$KOSMO_LLM_PROVIDER  model=$KOSMO_LLM_MODEL  base=$KOSMO_LLM_BASE_URL"

# 1) Cheap validation — one tiny call proves the key works AND the model name
#    is accepted, BEFORE the ~15-min benchmark spends real money on a typo.
echo "validating key + model with one tiny call…"
resp=$(curl -s -w $'\n%{http_code}' --max-time 60 \
  -H "Authorization: Bearer ${KOSMO_LLM_API_KEY}" \
  -H "Content-Type: application/json" \
  "${KOSMO_LLM_BASE_URL%/}/chat/completions" \
  -d "{\"model\":\"${KOSMO_LLM_MODEL}\",\"messages\":[{\"role\":\"user\",\"content\":\"reply with the single word OK\"}]}")
code=$(printf '%s' "$resp" | tail -n1)
if [ "$code" != "200" ]; then
  echo "validation FAILED (http ${code}):" >&2
  printf '%s\n' "$resp" | sed '$d' >&2
  echo "→ fix the key or the model flag, then re-run. No benchmark money spent." >&2
  exit 3
fi
echo "validation OK (http 200) — key and model accepted."

# 2) The real thing. ~15-20 min, ~50-70 provider calls; cost scales with what
#    the model CANNOT do. Verdict is execution, never the model's word.
echo "firing the realization benchmark — this takes a while…"
cargo run --quiet -p kosmo-run -- \
  --provider env --realize-bench --realize-bench-report bench.json --no-color

echo
echo "done — full content-addressed report in: bench.json"

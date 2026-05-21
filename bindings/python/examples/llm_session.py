#!/usr/bin/env python3
"""
PSE × LLM cross-session memory demo — Python edition.

Mirrors tools/pse-llm-demo but uses the pse_core Python bindings.
Run twice to see cross-session memory in action.

Requirements:
    pip install pse-core   # or: maturin develop (from bindings/python/)

Usage:
    PSE_LLM_API_KEY=<key> python examples/llm_session.py
    PSE_LLM_API_KEY=<key> python examples/llm_session.py   # session 2: hits!

Environment variables:
    PSE_LLM_API_KEY   API key (required)
    PSE_LLM_BASE_URL  API base URL (default: https://api.cerebras.ai/v1)
    PSE_LLM_MODEL     Model name   (default: llama3.1-8b)
    PSE_LLM_MEMORY    Memory file  (default: pse-python-memory.json)
"""

import json
import os
import pathlib
import time
import urllib.error
import urllib.request

import pse_core

# ── Configuration ─────────────────────────────────────────────────────────────

BASE_URL    = os.environ.get("PSE_LLM_BASE_URL", "https://api.cerebras.ai/v1")
API_KEY     = os.environ.get("PSE_LLM_API_KEY", "")
MODEL       = os.environ.get("PSE_LLM_MODEL", "llama3.1-8b")
MEMORY_FILE = pathlib.Path(os.environ.get("PSE_LLM_MEMORY", "pse-python-memory.json"))

QUESTIONS = [
    (
        "Explain the concept of entropy in thermodynamics and information theory. "
        "What structural properties do both interpretations share?"
    ),
    (
        "How does irreversibility in thermodynamics connect to the arrow of time? "
        "What constraints does entropy impose on physical processes?"
    ),
    (
        "What is the relationship between information compression and the second "
        "law of thermodynamics? How does Maxwell's demon relate to entropy?"
    ),
]

# ── LLM call ─────────────────────────────────────────────────────────────────


def llm_complete(question: str) -> str:
    if not API_KEY:
        raise RuntimeError(
            "Set PSE_LLM_API_KEY (and optionally PSE_LLM_BASE_URL, PSE_LLM_MODEL)."
        )
    payload = json.dumps(
        {
            "model": MODEL,
            "messages": [{"role": "user", "content": question}],
            "max_tokens": 512,
        }
    ).encode()
    req = urllib.request.Request(
        f"{BASE_URL}/chat/completions",
        data=payload,
        headers={
            "Authorization": f"Bearer {API_KEY}",
            "Content-Type": "application/json",
        },
    )
    with urllib.request.urlopen(req, timeout=60) as resp:
        data = json.loads(resp.read())
    return data["choices"][0]["message"]["content"]


# ── Memory persistence ────────────────────────────────────────────────────────


def load_memory() -> dict:
    if MEMORY_FILE.exists():
        return json.loads(MEMORY_FILE.read_text(encoding="utf-8"))
    return {"crystals_json": "[]", "prior_responses": []}


def save_memory(mem: dict) -> None:
    MEMORY_FILE.write_text(json.dumps(mem), encoding="utf-8")


# ── Main ─────────────────────────────────────────────────────────────────────


def main() -> None:
    print()
    print("PSE × LLM — Python Bindings Demo")
    print("═" * 64)
    print(f"  Model   : {MODEL}")
    print(f"  Endpoint: {BASE_URL}")
    print(f"  Memory  : {MEMORY_FILE}")
    print()

    mem = load_memory()
    prior_responses: list[str] = mem.get("prior_responses", [])
    session = len(prior_responses) + 1

    print(
        f"  Session {session}: {'COLD START' if session == 1 else 'WARM START'}"
        f"  ({len(json.loads(mem['crystals_json']))} crystal(s) in memory)"
    )
    print()

    # Warm-start state from saved crystals
    state = pse_core.PseState(memory_json=mem["crystals_json"])

    if state.crystal_count() > 0:
        print(
            f"  PatternMemory: {state.crystal_count()} signature(s) loaded"
            " from prior sessions"
        )
        print()

    # ── Replay prior responses ─────────────────────────────────────────────
    replay_hits = 0
    if session >= 2:
        print("─" * 64)
        print(f"  Re-processing {len(prior_responses)} prior response(s) through PSE…")
        t0 = time.monotonic()
        for prior in prior_responses:
            state.process_text(prior)
        replay_hits = state.pattern_hits()
        elapsed_ms = int((time.monotonic() - t0) * 1000)

        print(f"  Replay memory hits : {replay_hits}")
        print(f"  Replay time        : {elapsed_ms}ms")

        if replay_hits > 0:
            print()
            print(f"  ✓ PSE recognised topology from session {session - 1}.")
            print("    Identical text → identical observation graph →")
            print("    canonical-class match in PatternMemory.")
        print()

    # ── LLM query ─────────────────────────────────────────────────────────
    question = QUESTIONS[(session - 1) % len(QUESTIONS)]
    print("─" * 64)
    print(f'  Q: "{question[:90]}…"')
    print()

    t0 = time.monotonic()
    response = llm_complete(question)
    llm_ms = int((time.monotonic() - t0) * 1000)

    preview = response[:300] + ("…" if len(response) > 300 else "")
    print(f'  A: "{preview}"')
    print()
    print(f"  LLM time: {llm_ms}ms")
    print()

    # ── PSE ingestion ──────────────────────────────────────────────────────
    print("─" * 64)
    hits_before = state.pattern_hits()
    t0 = time.monotonic()
    new_crystals = state.process_text(response)
    pse_ms = int((time.monotonic() - t0) * 1000)
    new_hits = state.pattern_hits() - hits_before

    print(f"  New crystals : {len(new_crystals)}")
    for c in new_crystals:
        print(f"    {c}")
    print(f"  PSE time     : {pse_ms}ms")
    if new_hits:
        print(f"  New-response hits: {new_hits} (PSE found overlap with prior memory)")
    print()

    # ── Persist ────────────────────────────────────────────────────────────
    mem["crystals_json"] = state.save_memory()
    mem["prior_responses"] = prior_responses + [response]
    save_memory(mem)

    print("═" * 64)
    print(f"  Memory saved: {state.crystal_count()} crystal(s) → {MEMORY_FILE}")
    print()
    if session == 1:
        print("  Session 1 complete. Run again to see cross-session memory.")
    else:
        print(f"  Cross-session replay hits : {replay_hits}  (prior topology recognised)")
        print(f"  Total memory             : {state.crystal_count()} crystals accumulated")
        print(
            f"  PSE substrate claim      : "
            f"{'PROVEN ✓' if replay_hits > 0 else 'needs more sessions — run again'}"
        )
    print()


if __name__ == "__main__":
    main()

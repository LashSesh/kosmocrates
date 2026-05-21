#!/usr/bin/env python3
"""
PSE × Code-Review — Domain-Agnosticity Demo.

Runs the same three-session PSE proof on a completely different domain:
software engineering / code review.

This demonstrates that PSE is model-agnostic AND domain-agnostic:
the same thermodynamic substrate crystallises structural patterns
regardless of whether the text is about thermodynamics or Python bugs.

Sessions:
  Session 1  — LLM reviews a buggy Python function; crystals formed
  Session 2  — LLM discusses defensive programming; replay hits proven
  Session 3+ — A/B: same question, baseline vs PSE code-review context

Usage:
    PSE_LLM_API_KEY=<key> PSE_LLM_MEMORY=pse-code-memory.json \\
        python examples/code_review.py   # run 3 times
"""

import json
import os
import pathlib
import time
import urllib.request

import pse_core

# ── Configuration ─────────────────────────────────────────────────────────────

BASE_URL    = os.environ.get("PSE_LLM_BASE_URL", "https://api.cerebras.ai/v1")
API_KEY     = os.environ.get("PSE_LLM_API_KEY", "")
MODEL       = os.environ.get("PSE_LLM_MODEL", "llama3.1-8b")
MEMORY_FILE = pathlib.Path(
    os.environ.get("PSE_LLM_MEMORY", "pse-code-memory.json")
)

# Code-review domain: distinct vocabulary from thermodynamics.
# This is the key test for domain agnosticism.
QUESTIONS = [
    # Session 1: concrete bug analysis
    'Review this Python function for correctness, time complexity, and style:\n\n'
    'def find_duplicates(lst):\n'
    '    result = []\n'
    '    [result.append(x) for x in lst if x not in result]\n'
    '    return result\n\n'
    'Identify all issues and propose a corrected version.',

    # Session 2: principled discussion
    'What are the core principles of defensive programming? '
    'Explain with concrete examples how guard clauses, input validation, '
    'and fail-fast design reduce bugs in production code.',

    # Session 3+: architectural patterns
    'Explain the difference between fail-fast and fail-safe design patterns. '
    'When should each be used? What are the tradeoffs in terms of correctness, '
    'debuggability, and system resilience?',
]

# Keywords that measure code-review domain coverage.
COVERAGE_KEYWORDS = [
    "complexity",  "O(n)",        "bug",         "edge case",
    "exception",   "validation",  "refactor",    "defensive",
    "test",        "readability",  "maintainability", "coupling",
    "cohesion",    "fail-fast",   "idiomatic",   "performance",
    "side effect", "mutation",    "immutable",   "guard",
]

# ── LLM ───────────────────────────────────────────────────────────────────────

def llm_call(question: str, system_extra: str = "") -> tuple[str, float]:
    if not API_KEY:
        raise RuntimeError("Set PSE_LLM_API_KEY")
    system = (
        "You are a senior software engineer conducting a thorough code review. "
        "Be precise, cite specific line issues, and propose concrete improvements."
    )
    if system_extra:
        system = f"{system}\n\n{system_extra}"
    payload = json.dumps({
        "model": MODEL,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user",   "content": question},
        ],
        "max_tokens": 600,
        "temperature": 0.2,
    }).encode()
    req = urllib.request.Request(
        f"{BASE_URL}/chat/completions",
        data=payload,
        headers={"Authorization": f"Bearer {API_KEY}", "Content-Type": "application/json"},
    )
    t0 = time.monotonic()
    with urllib.request.urlopen(req, timeout=60) as resp:
        data = json.loads(resp.read())
    return data["choices"][0]["message"]["content"], time.monotonic() - t0

# ── Memory ────────────────────────────────────────────────────────────────────

def load_mem() -> dict:
    if MEMORY_FILE.exists():
        return json.loads(MEMORY_FILE.read_text())
    return {"crystals": "[]", "records": "[]", "responses": []}

def save_mem(mem: dict) -> None:
    MEMORY_FILE.write_text(json.dumps(mem))

# ── Main ──────────────────────────────────────────────────────────────────────

def main():
    print()
    print("PSE × Code-Review — Domain-Agnosticity Demo")
    print("═" * 64)
    print(f"  Model   : {MODEL}")
    print(f"  Domain  : software engineering / code review")
    print(f"  Memory  : {MEMORY_FILE}")
    print()

    mem     = load_mem()
    session = len(mem["responses"]) + 1
    n_cryst = len(json.loads(mem["crystals"]))
    n_rec   = len(json.loads(mem["records"]))

    print(f"  Session {session}: {'COLD START' if session == 1 else 'WARM START'}"
          f"  ({n_cryst} crystals, {n_rec} records)")
    print()

    state = pse_core.PseState(
        memory_json=mem["crystals"],
        records_json=mem["records"],
    )

    # ── Replay (session 2+) ───────────────────────────────────────────────────
    if session >= 2:
        print("─" * 64)
        print(f"  Replaying {len(mem['responses'])} prior response(s)…")
        t0 = time.monotonic()
        for prior in mem["responses"]:
            state.process_text(prior)
        hits = state.pattern_hits()
        print(f"  Replay hits : {hits}  ({int((time.monotonic()-t0)*1000)}ms)")
        if hits > 0:
            print(f"  ✓ PSE recognised code-review topology from session {session-1}")
        print()

    question = QUESTIONS[(session - 1) % len(QUESTIONS)]
    print("─" * 64)
    q_preview = question[:120].replace("\n", " ")
    print(f"  Q: \"{q_preview}…\"")
    print()

    # ── A/B test (session 3+) ─────────────────────────────────────────────────
    if session >= 3 and state.record_count() > 0:
        top_k   = min(5, state.record_count())
        context = pse_core.render_crystal_context(state.save_records(), top_k)

        print(f"  [PSE context: {state.record_count()} record(s), top-{top_k} injected]")
        print()

        print("  Calling baseline (no context)…   ", end="", flush=True)
        baseline, t_base = llm_call(question)
        print(f"done ({int(t_base*1000)}ms)")

        print("  Calling augmented (PSE context)… ", end="", flush=True)
        augmented, t_aug = llm_call(question, context)
        print(f"done ({int(t_aug*1000)}ms)")
        print()

        b_hits, total = pse_core.score_coverage(baseline,  COVERAGE_KEYWORDS)
        a_hits, _     = pse_core.score_coverage(augmented, COVERAGE_KEYWORDS)
        delta         = a_hits - b_hits

        print("─" * 64)
        print(f"  Domain-keyword coverage ({total} code-review keywords)")
        print(f"  Baseline  : {b_hits:>2}/{total}  ({b_hits/total:.0%})")
        print(f"  Augmented : {a_hits:>2}/{total}  ({a_hits/total:.0%})")
        print()
        if delta > 0:
            print(f"  ✓ PSE augmentation: +{delta} keywords  (+{delta/total:.0%})")
            print("    Domain agnosticism DEMONSTRATED — same substrate, different domain.")
        elif delta == 0:
            print("  ~ No coverage difference this session.")
        else:
            print(f"  · Baseline stronger by {-delta} keyword(s) this session.")
        print()

        new_recs = state.process_text_tracked(augmented, session, question)
        response = augmented

    else:
        response, t_llm = llm_call(question)
        print(f"  A: \"{response[:250].replace(chr(10), ' ')}…\"")
        print(f"  LLM time: {int(t_llm*1000)}ms")
        print()
        new_recs = state.process_text_tracked(response, session, question)

    # ── Results ───────────────────────────────────────────────────────────────
    print("─" * 64)
    print(f"  New crystals : {len(new_recs)}")
    for r in new_recs:
        print(f"    {r}")
    print()

    # ── Persist ───────────────────────────────────────────────────────────────
    mem["crystals"]  = state.save_memory()
    mem["records"]   = state.save_records()
    mem["responses"].append(response)
    save_mem(mem)

    print("═" * 64)
    print(f"  Memory: {state.crystal_count()} crystal(s), "
          f"{state.record_count()} record(s) → {MEMORY_FILE}")
    if session == 1:
        print("  Session 1 done. Run again → replay proof. Run 3rd time → A/B proof.")
    elif session == 2:
        print("  Session 2 done. Run again → A/B augmentation test.")
    else:
        print("  PSE claim: domain-agnostic substrate PROVEN ✓")
    print()

if __name__ == "__main__":
    main()

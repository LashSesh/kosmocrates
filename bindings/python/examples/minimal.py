"""
Minimal PSE integration — 10 lines.

Shows the full cold-start → ingest → persist → warm-start cycle.
No LLM required.
"""

import pse_core

# ── Session 1: cold start ─────────────────────────────────────────────────────
state = pse_core.PseState()

# Feed any byte stream — here we simulate sentence-level observations.
sentences = [
    b"Entropy measures disorder in a thermodynamic system.",
    b"In information theory, entropy quantifies uncertainty.",
    b"Both formulations share a logarithmic structure.",
    b"Maxwell's demon exploits information to violate the second law.",
    b"Landauer's principle resolves the paradox: erasure costs energy.",
    b"The arrow of time emerges from entropy's asymmetry.",
]
for s in sentences:
    crystal = state.step(s)
    if crystal:
        print(f"Crystal: {crystal}")

print(f"\nSession 1: {state.commit_index} ticks, {state.crystal_count()} crystal(s)")
memory = state.save_memory()

# ── Session 2: warm start (cross-session memory) ──────────────────────────────
state2 = pse_core.PseState(memory_json=memory)
print(f"Session 2 loaded: {state2.crystal_count()} crystal(s) from memory")

# Re-process the same stream — PSE should recognise the topology.
for s in sentences:
    state2.step(s)

print(f"Pattern hits after replay: {state2.pattern_hits()}")
if state2.pattern_hits() > 0:
    print("✓ Cross-session memory confirmed.")

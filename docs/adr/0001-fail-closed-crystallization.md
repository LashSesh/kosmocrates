# 0001. Fail-closed crystallization as the default contract

- **Status:** Accepted
- **Date:** 2024-Q4 (back-filled)
- **Deciders:** @LashSesh

## Context

Kosmocrates ingests untrusted observation streams and converts them
into content-addressed `SemanticCrystal` records that downstream
consumers treat as ground truth. Two failure modes were on the table
when the crystallization pipeline was being designed:

1. **Fail-open:** if the structural-stability gate is uncertain,
   commit the crystal anyway (with a low confidence score). Loses no
   data; relies on downstream filtering.
2. **Fail-closed:** if the gate cannot prove stability, emit no
   crystal. The observation is dropped without trace at the commit
   boundary.

For an *epistemic* memory system — one that downstream agents query
to ground their reasoning — a fail-open default would poison the
knowledge graph with low-quality crystals indistinguishable from
high-quality ones. A downstream LLM has no way to reconstruct *why*
a crystal was admitted; it has to trust the boundary.

## Decision

We will make **fail-closed** the default and the only supported
crystallization contract. A `SemanticCrystal` is committed if and
only if the gate proves topological stability against the
falsification cascade. Uncertain observations produce no commit.

## Alternatives considered

| Option | Trade-off | Why rejected |
|---|---|---|
| Fail-open with confidence scores | Loses no data. | Downstream consumers (LLMs, retrieval) have no reliable way to filter — confidence drift over time silently corrupts the knowledge field. |
| Fail-closed default, fail-open opt-in | Same safety + escape hatch. | Two contracts is one too many. Every consumer would have to handle both modes, and a misconfigured opt-in flag would silently bypass the safety. |
| Fail-closed with a "rejected-crystals" sidecar log | Same safety + observability. | Reasonable; deferred. Can be added without breaking the contract. Tracked as a roadmap item. |

## Consequences

What becomes easier:

- Downstream consumers (retrieval, agenda, LLM grounding) can treat
  every committed crystal as structurally vetted by construction —
  no per-crystal confidence threshold to tune.
- The replay invariant (ADR-0002) is provable: a crystal that
  survives replay is by definition gate-admissible.
- QTIC conformance classification has a stable input population.

What becomes harder:

- Streams with high noise produce few or zero crystals on day one,
  which can look like "the engine isn't doing anything" to a new
  user. The 30-second `pse-demo` is tuned to make commits visible
  immediately to compensate.
- Debugging "why did this stream produce no crystals" requires
  observability at the gate level (`tracing::debug!` calls in
  `crates/pse-core/src/lib.rs:702-738`).

What is now explicitly out of scope:

- A "soft" commit mode that admits crystals at reduced confidence.
- Per-consumer override of the gate decision.

## Implementation notes

The contract is enforced at two layers:

- `crates/pse-core/src/lib.rs` — the gate check returns
  early without writing to `state.archive` if `kairos` is false.
- `tests/integration/replay_byte_identity.rs` — verifies that a
  byte-identical input produces a byte-identical archive across
  runs, which implicitly tests that the gate decision is itself
  deterministic.

`CONTRIBUTING.md` lists "fail-closed contract" as one of the
non-negotiables every PR is reviewed against.

## References

- Internal: `crates/pse-core/src/lib.rs`,
  `tests/integration/replay_byte_identity.rs`,
  `CONTRIBUTING.md` § "Ground rules",
  `SECURITY.md` § "Security-relevant components" (`pse-traverse`).
- Specs: `specs/QTIC.pdf`, `specs/PSP_Formal_Specification_v1.0.0_Sebastian_Klemm.pdf`.

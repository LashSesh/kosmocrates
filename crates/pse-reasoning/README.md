# pse-reasoning

Epistemic Thunderbolt Vector — D=ψ·ρ·ω guided reasoning over the IL knowledge graph

`pse-reasoning` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

Epistemic Thunderbolt Vector — D = ψ · ρ · ω guided reasoning.

The Pfauenthron++ retrieval formula `D = ψ · ρ · ω` is not just a
retrieval metric — it is a *generalized epistemic energy function*.

Applied to reasoning:

- **Retrieval**: select the highest-D crystal from the store once.
- **Reasoning**: follow the gradient of D *across multiple hops* through
  the knowledge graph.  Each step uses the previous crystal's `vector8`
  as the next query, tracing a path of maximum epistemic coherence.

The metaphor: lightning follows the path of least electrical resistance.
The Epistemic Thunderbolt follows the path of highest epistemic energy
through the IL knowledge graph — it is *attractor-constrained reasoning*.

## Algorithm

```text
query_text
  │
  └─ text_to_vector8(query) ──► current_vec

loop step 0..max_steps:
  hits = score_tripolar(current_vec)        // D = ψ · ρ · ω for all crystals
  best = first hit not already in chain      // loop prevention
  if best.D < min_d_threshold  → terminate(MinThreshold)
  chain.push(best)
  current_vec = crystal_vector8(best.id)    // follow the crystal's embedding

terminate(MaxSteps)
```

The chain terminates when:
- `max_steps` is reached (configurable, default 6)
- The highest available D drops below `min_d_threshold` (default 0.01)
- No unvisited crystals remain (loop exhaustion)

## Add to your project

```toml
[dependencies]
pse-reasoning = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-reasoning --open`
(once published, also available on [docs.rs](https://docs.rs/pse-reasoning)).

## License

MIT — see [`LICENSE`](../../LICENSE).

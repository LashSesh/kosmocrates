# pse-exploratory

Exploratory Ledger — negative-ψ crystals as epistemic hypotheses (hallucination-as-instrument)

`pse-exploratory` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

Exploratory Ledger — hallucination as epistemic instrument.

A crystal with coherence potential ψ < 0 is *exploratory*: it sits in a
high-free-energy region of the epistemic space, far from any confirmed
attractor.  The model produced it, but no evidence has yet grounded it.

Instead of discarding these crystals, the exploratory ledger parks them as
*hypotheses* and waits for evidence to either confirm (land) or decay them.

## Lifecycle

```text
  commit, ψ < 0  ──►  Pending   ──►  same rule_id, new ψ ≥ 0  ──►  Landed
                             │
                             └──►  run > added_at + decay_after  ──►  Decayed
```

- **Pending** → hypothesis lives in the ledger, surfaces as `UnknownSlot`
- **Landed**  → evidence arrived; crystal promoted to grounded knowledge
- **Decayed** → evidence never came; surfaces as `Stale` `UnknownSlot`

## ψ as a coherence signal

ψ = kuramoto_coherence − (1 − stability_score)

Negative ψ means the crystal's self-organising pressure is outweighed by its
instability — it cannot sustain itself as an attractor.  Positive ψ means
the crystal has become a self-reinforcing knowledge node.

The landing threshold is ψ ≥ 0: the crystal has crossed the attractor
boundary and is now contributing positive coherence to the system.

## Add to your project

```toml
[dependencies]
pse-exploratory = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-exploratory --open`
(once published, also available on [docs.rs](https://docs.rs/pse-exploratory)).

## License

MIT — see [`LICENSE`](../../LICENSE).

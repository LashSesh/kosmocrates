# pse-traverse

PSE Traversal Agent v0.1 — post-symbolic traversal layer over PSE (FieldCube, DoFGraph, CollapsePlan, fail-closed gating)

`pse-traverse` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

PSE Traversal Agent v0.1

A post-symbolic traversal layer over PSE. The kernel models a problem
space as a [`FieldCube`] (dimensions, constraints, couplings, paths,
carriers, evidence), constructs a [`DoFGraph`] (degree-of-freedom
graph), produces a deterministic [`CollapsePlan`], detects
[`PathExcision`] (formal options without admissible operational
paths), gates candidates **fail-closed**, and binds successful
candidates to PSE crystals via the [`bridge`] module — which uses
`pse_core::macro_step` as the only legitimate source of
`SemanticCrystal`s.

Determinism is a structural invariant, not a convention: every map
that affects canonical output is a `BTreeMap`, every list that
affects canonical output is sorted before serialisation, and every
report is round-tripped through JCS (RFC 8785) before its content
address is computed.

See `docs/POST_SYMBOLIC.md` for the paradigm definition this layer
operates within. See `specs/pse_traversal_agent_spec_v0_1_REUPLOAD.pdf` for the
spec this implementation realises.

## Add to your project

```toml
[dependencies]
pse-traverse = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-traverse --open`
(once published, also available on [docs.rs](https://docs.rs/pse-traverse)).

## License

MIT — see [`LICENSE`](../../LICENSE).

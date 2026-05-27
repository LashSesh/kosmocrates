# pse-adapter-il

PSE × Infinity Ledger bridge: converts SemanticCrystals into IL TIC blocks, 8D HNSW vectors, and file-based ledger. Optionally drives MEFCore::process() directly.

A domain adapter for the [Kosmocrates](https://github.com/lashsesh/pse)
workspace. Implements the `ObservationAdapter` trait (defined in
[`pse-graph`](../../crates/pse-graph)) so the core engine can ingest
the corresponding data stream through the standard `macro_step` loop.

## What it does

PSE → Infinity Ledger bridge.

Converts `SemanticCrystal` + provenance into IL-compatible artifacts:
- `ILPayload`: a JSON representation of a `CompactTic` ledger block
- An 8D vector: [fixpoint[0..5], ψ, ρ, ω] normalised to unit length

`ILStore` persists those blocks on disk in IL ledger format, provides
cosine-similarity search, and maintains an HDAG of all committed crystals.

## HDAG
The HDAG is always active (no feature flag required).  It implements
Sebastian Klemm's original HDAG v1.0 specification:
- Nodes are 5D resonance tensors [temporal, morphic, relational, topological, entropic]
- Edges are phase-gradient transitions Φ_ij = (T_j − T_i) / ‖T_j − T_i‖
- Acyclicity emerges from the coherence potential gate, not explicit checks

## QTIC
Every crystal committed via `ILStore::commit_with_feedback` receives a
`QticCertificate` classifying it as Q0–Q5 per the QTIC specification
("Quasi-Zeitinformationskristalle", Sebastian Klemm, 2026).  Only Q5
crystals satisfy the full QTIC definition (path-invariant, seam-stable,
replayable information attractor).

## Features
- `il-pipeline`: drive MEFCore::process() for authoritative TIC generation
- `full`:        il-pipeline enabled

## Add to your project

```toml
[dependencies]
pse-adapter-il = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-adapter-il --open`.
For the adapter contract, see [`crates/pse-graph`](../../crates/pse-graph).

## License

MIT — see [`LICENSE`](../../LICENSE).

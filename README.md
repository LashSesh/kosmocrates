# Kosmocrates

> **A post-symbolic multi-layer epistemic operating system for AI agents
> and autonomous systems.** Deterministic, content-addressed, fail-closed
> by construction.

[![CI](https://github.com/lashsesh/pse/actions/workflows/ci.yml/badge.svg)](https://github.com/lashsesh/pse/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust 1.82+](https://img.shields.io/badge/rust-1.82+-orange.svg)](Cargo.toml)

Kosmocrates turns stateless LLM and agent interactions into persistent,
verifiable, topology-aware knowledge structures: content-addressed
**crystals**, ledgered evidence, causal retrieval paths, QTIC
conformance certificates, and self-maintaining epistemic health signals.

It is **not** another vector store, prompt wrapper, or RAG convenience
layer. It is an execution substrate for deciding *what may become
memory*, *why it was allowed to persist*, *how it can be replayed*,
*where it sits in a causal knowledge graph*, and *what must be repaired
before the knowledge base can move toward closure*.

For the full architecture, theory background, and feature reference,
see [`docs/OVERVIEW.md`](docs/OVERVIEW.md).

---

## What you get

- **Persistent memory** across sessions, runs, and model versions.
- **Content-addressed evidence** for everything processed and committed.
- **Deterministic replay**: byte-identical inputs produce byte-identical outputs.
- **Fail-closed crystallization**: unstable patterns emit no persistent memory.
- **Topology-aware knowledge** instead of flat chunks or unverified embeddings.
- **Causal retrieval** over ancestors, descendants, refinements, and related crystals.
- **QTIC conformance certificates** classifying committed knowledge from Q0 to Q5.
- **Constitutional pre-commit checks** for blocking, required, and advisory rules.
- **Lifecycle management** for stale, redundant, aging, and vital crystals.

---

## 60-second quickstart

```bash
# Prerequisites: Rust >= 1.82 (rustup.rs)
git clone https://github.com/lashsesh/pse
cd pse

# 30-second engine smoke test
cargo run --release -p pse-demo

# A minimal end-to-end Rust example using the umbrella crate
cargo run --release -p pse --example quickstart
```

Or run the server in a container:

```bash
docker compose up
# pse-server listens on http://localhost:8765 — try `curl localhost:8765/health`
```

For the full three-session LLM-memory walkthrough (Rust + Cerebras
API), see [`DEMO.md`](DEMO.md).

---

## Where to go next

| If you want to… | Read |
|---|---|
| Understand the layered architecture | [`docs/OVERVIEW.md`](docs/OVERVIEW.md) |
| See the 3-session LLM proof | [`DEMO.md`](DEMO.md) |
| Integrate from Python | [`bindings/python/README.md`](bindings/python/README.md) |
| Integrate from Node / browser | [`bindings/node/README.md`](bindings/node/README.md) |
| Run the HTTP server | [`tools/pse-server/src/main.rs`](tools/pse-server/src/main.rs) (docs in module header) |
| Understand the compliance posture | [`docs/COMPLIANCE.md`](docs/COMPLIANCE.md) |
| Read the formal specs | [`specs/`](specs/) |
| Contribute | [`CONTRIBUTING.md`](CONTRIBUTING.md) |
| Report a vulnerability | [`SECURITY.md`](SECURITY.md) |
| Understand project governance | [`GOVERNANCE.md`](GOVERNANCE.md) |
| Plan a release / build a Docker image | [`RELEASING.md`](RELEASING.md) |
| Know what is on the roadmap | [`ROADMAP.md`](ROADMAP.md) |

---

## Architecture at a glance

| Layer | Crates | Role |
|-------|--------|------|
| **Core engine** | `pse-core`, `pse-cascade`, `pse-graph`, … | Crystallization: observation → fail-closed gate → `SemanticCrystal` |
| **Traversal Agent Stack** | `pse-traverse`, `pse-metatron`, `phase-matrix`, … | Eight deterministic cognitive layers above the core engine |
| **Infinity Ledger + HDAG + QTIC** | `pse-adapter-il` (via `vendors/infinityledger`) | Persistent ledger, 5D resonance graph, Q0–Q5 conformance certificates |
| **PSE+IL Intelligence Layer** | directions 1–10 in `pse-adapter-il` | Active epistemic system: health, lifecycle, agenda, retrieval |
| **NxAlien** | `pse-nxalien-*`, `nxalien-cli` | Agent-context exoskeleton: governance → HDAG → epistemic signal |
| **Adapters** | `pse-adapter-*` | Domain adapters: seismo, vitals, binance, ENTSO-E, syslog, … |

All layers are **fail-closed**: `SemanticCrystal` records flow only
through the PSE-Bridge. No subsystem constructs or commits crystals on
its own.

---

## Project status

Kosmocrates is **pre-1.0**. The core API is stable enough for serious
integration work but may shift between minor releases — see
[`RELEASING.md`](RELEASING.md) §1 for the versioning policy and
[`ROADMAP.md`](ROADMAP.md) for what is gated on the `1.0.0` cut.

- 695+ workspace tests, deterministic replay test included.
- CI: format / clippy / build × 3 OS / test / doc / supply-chain.
- Single-maintainer project — see [`GOVERNANCE.md`](GOVERNANCE.md) for
  what that means for response times and merge cadence.

---

## License

MIT. See [`LICENSE`](LICENSE). Contributions are accepted under the
same licence — see [`CONTRIBUTING.md`](CONTRIBUTING.md).

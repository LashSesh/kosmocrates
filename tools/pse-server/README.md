# pse-server

PSE REST API server — exposes the Kosmocrates over HTTP/JSON

A command-line tool that ships with the [Kosmocrates](https://github.com/lashsesh/pse)
workspace.

## What it does

PSE REST API server.

Exposes the Kosmocrates over HTTP/JSON so any language can use
it without a Rust or Python toolchain.

**Design:** mostly stateless — the client carries state between calls via
`memory_json` fields.  The server holds no sessions.
Exceptions: the IL store (activated by `PSE_IL_STORE`) and the nxalien
governance state (activated by `PSE_NXALIEN_STATE`) are persistent,
file-backed stores shared across all requests.

**Endpoints:**

  GET  /health                  — liveness check
  POST /ingest                  — process text through PSE; returns crystals + state
  POST /context                 — render crystal records into an LLM prompt block
  POST /coverage                — count domain keywords in a response

  GET  /il/status               — IL + HDAG health (active?, block_count, mean ψ)
  POST /il/retrieve             — Pfauenthron++ unified retrieval D=ψ·ρ·ω
  GET  /il/hdag/coherence       — mean coherence potential + edge counts
  GET  /il/hdag/order           — topological order of HDAG nodes

  POST /nxalien/bundle          — ingest a governance bundle from any repo;
                                  commits rules to IL, updates cross-repo
                                  attractor, returns EpistemicSignal
  GET  /nxalien/signal          — current cross-repo EpistemicSignal
  POST /nxalien/validate        — dry-run constitutional check (no state change)
  GET  /nxalien/rules/current   — evolved rule set after last signal

  POST /constitutional/check    — evaluate an agent action against the live
                                  constitutional rule set; returns
                                  Allow / Block / Warn decision.
                                  Strict mode auto-activates when signal is
                                  Drifting or Diverging.

  GET  /exploratory/status      — pending hypotheses (negative-ψ crystals),
                                  landing events, decay log, UnknownSlots.

  POST /reasoning/guide         — Epistemic Thunderbolt Vector: D = ψ·ρ·ω
                                  guided multi-hop traversal of the IL graph.
                                  Returns a scored reasoning chain following
                                  the path of highest epistemic energy.

**Configuration (env vars):**

  `PSE_SERVER_HOST`     bind address (default: `0.0.0.0`)
  `PSE_SERVER_PORT`     port         (default: `8765`)
  `PSE_IL_STORE`        path to IL ledger directory (optional; activates IL routes)
  `PSE_NXALIEN_STATE`   path to nxalien state directory (optional; activates
                        /nxalien/* routes and cross-repo attractor)

**Quickstart:**
```text
  PSE_NXALIEN_STATE=/var/pse/nxalien cargo run --release -p pse-server

  # Push a bundle from any repo
  curl -s -X POST http://localhost:8765/nxalien/bundle \
    -H 'Content-Type: application/json' \
    -d @nxalien.manifest.json | jq .signal.stability

  # Current cross-repo signal
  curl http://localhost:8765/nxalien/signal | jq .
```

## Run

```bash
# From the workspace root:
cargo run --release -p pse-server
# or, after `cargo install --path .`:
pse-server
```

## Documentation

For the layered architecture this tool operates on, see the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md).

## License

MIT — see [`LICENSE`](../../LICENSE).

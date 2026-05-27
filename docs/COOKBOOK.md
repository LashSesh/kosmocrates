# Cookbook

Task-oriented recipes for common Kosmocrates use cases. Each recipe
is self-contained: copy, paste, run.

For the layered architecture context, see
[`docs/OVERVIEW.md`](OVERVIEW.md). For the API reference of any
crate mentioned here, run `cargo doc -p <crate> --open`.

> The recipes in this cookbook assume Rust ≥ 1.82 and a clone of the
> Kosmocrates workspace. Crate-specific install instructions live in
> the per-crate README (e.g. [`crates/pse-core/README.md`](../crates/pse-core/README.md)).

## Table of contents

1. [Ingest text and get crystals back](#1-ingest-text-and-get-crystals-back)
2. [Replay a corpus and prove determinism](#2-replay-a-corpus-and-prove-determinism)
3. [Persist memory across processes](#3-persist-memory-across-processes)
4. [Add a custom domain adapter](#4-add-a-custom-domain-adapter)
5. [Run the HTTP server with bearer-token auth](#5-run-the-http-server-with-bearer-token-auth)
6. [Call the HTTP server from Python](#6-call-the-http-server-from-python)
7. [Call the engine from a browser via WASM](#7-call-the-engine-from-a-browser-via-wasm)
8. [Investigate why a stream produced no crystals](#8-investigate-why-a-stream-produced-no-crystals)
9. [Generate a CycloneDX SBOM for a release](#9-generate-a-cyclonedx-sbom-for-a-release)
10. [Wire a constitutional rule into pse-server](#10-wire-a-constitutional-rule-into-pse-server)

---

## 1. Ingest text and get crystals back

The shortest path from a stream of text to a list of content-addressed
crystals.

```rust
use pse::core::{macro_step, GlobalState};
use pse::graph::PassthroughAdapter;
use pse::types::Config;

let config = Config::default();
let adapter = PassthroughAdapter::new("my-domain");
let mut state = GlobalState::new(&config);

let observations: Vec<Vec<u8>> = vec![
    b"sensor-1 reading 1.42 at t=0".to_vec(),
    b"sensor-1 reading 1.41 at t=1".to_vec(),
    b"sensor-1 reading 1.43 at t=2".to_vec(),
];

let _ = macro_step(&mut state, &observations, &config, &adapter);

println!("{} crystals committed", state.archive.len());
for c in &state.archive {
    println!("  id={}  stability={:.3}", c.id, c.stability_score);
}
```

The default `Config` has conservative thresholds — see
[recipe 8](#8-investigate-why-a-stream-produced-no-crystals) if you
get zero crystals.

---

## 2. Replay a corpus and prove determinism

The contract that makes Kosmocrates auditable: byte-identical input
→ byte-identical archive. The full proof lives in
[`tests/integration/replay_byte_identity.rs`](../tests/integration/);
this is the minimal in-process version.

```rust
use pse::core::{macro_step, GlobalState};
use pse::graph::PassthroughAdapter;
use pse::types::Config;

fn run(observations: &[Vec<u8>]) -> Vec<String> {
    let config = Config::default();
    let adapter = PassthroughAdapter::new("replay-check");
    let mut state = GlobalState::new(&config);
    let _ = macro_step(&mut state, observations, &config, &adapter);
    state.archive.iter().map(|c| c.id.clone()).collect()
}

let obs: Vec<Vec<u8>> = (0..200)
    .map(|i| format!(r#"{{"tick":{i},"value":{}}}"#, (i as f64 * 0.1).sin()).into_bytes())
    .collect();

let ids_1 = run(&obs);
let ids_2 = run(&obs);
assert_eq!(ids_1, ids_2, "two replays must produce identical crystal IDs");
```

If this assertion ever fails, you've found the highest-priority class
of bug in this codebase. See [`SUPPORT.md`](../SUPPORT.md) §
"Determinism / replay impact" for how to file it.

---

## 3. Persist memory across processes

`GlobalState` is in-memory by default. To carry pattern memory across
process boundaries, serialise the relevant fields to JSON and feed
them back via `load_memory_from_crystals`.

```rust
use pse::core::{load_memory_from_crystals, macro_step, GlobalState};
use pse::graph::PassthroughAdapter;
use pse::types::Config;

// Session 1: produce crystals, dump them.
let config = Config::default();
let adapter = PassthroughAdapter::new("session-1");
let mut state = GlobalState::new(&config);
let _ = macro_step(&mut state, &[/* observations */], &config, &adapter);

let memory_json = serde_json::to_string(&state.archive).unwrap();
std::fs::write("memory.json", &memory_json).unwrap();

// Session 2 (later process): warm-start from the dump.
let crystals: Vec<_> = serde_json::from_str(&std::fs::read_to_string("memory.json").unwrap()).unwrap();
let mut state = GlobalState::new(&config);
let n_loaded = load_memory_from_crystals(&mut state, &crystals);
println!("warm-started with {n_loaded} crystals");
```

For a server-side persistent store (Infinity Ledger), see
[recipe 5](#5-run-the-http-server-with-bearer-token-auth) and the
`PSE_IL_STORE` env var.

---

## 4. Add a custom domain adapter

The trait surface is intentionally tiny — see `crates/pse-graph` for
the canonical definition and `adapters/pse-adapter-airquality` for a
clean reference implementation.

```rust
use pse::graph::{ObservationAdapter, ObserveError};
use pse::types::{Observation, MeasurementContext, ProvenanceEnvelope, content_address_raw};

pub struct MyAdapter {
    source_id: String,
}

impl ObservationAdapter for MyAdapter {
    fn source_id(&self) -> &str { &self.source_id }

    fn canonicalize(
        &self,
        raw: &[u8],
        context: &MeasurementContext,
    ) -> Result<Observation, ObserveError> {
        let payload = raw.to_vec();
        let digest = content_address_raw(&payload);
        Ok(Observation {
            timestamp: 0.0,
            source_id: self.source_id.clone(),
            provenance: ProvenanceEnvelope {
                origin: self.source_id.clone(),
                chain: Vec::new(),
                sig: None,
            },
            payload,
            context: context.clone(),
            digest,
            schema_version: "1.0.0".to_string(),
            phase_hint: None,
        })
    }
}
```

For the full adapter checklist (offline dataset, integration test,
workspace member registration), see
[`CONTRIBUTING.md`](../CONTRIBUTING.md) § "Adding a new domain adapter".

---

## 5. Run the HTTP server with bearer-token auth

`pse-server` exposes the engine over HTTP/JSON. Default startup is
open; setting `PSE_SERVER_TOKEN` enables the bearer-token middleware
documented in [`HARDENING.md`](../HARDENING.md) § "Authentication".

```bash
export PSE_SERVER_HOST=127.0.0.1
export PSE_SERVER_PORT=8765
export PSE_SERVER_TOKEN="$(openssl rand -hex 32)"
export PSE_IL_STORE=/var/lib/kosmocrates/il_store
export RUST_LOG=info

cargo run --release -p pse-server
```

From another shell:

```bash
TOKEN="<the value printed at startup>"

# Probe (always open):
curl -s http://127.0.0.1:8765/health | jq .

# Ingest (requires the token):
curl -s -X POST http://127.0.0.1:8765/ingest \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"text": "sensor reading 1.42", "session": 1, "question": "anomaly?"}' | jq .new_crystal_count
```

For a hardened container deployment (read-only rootfs, dropped caps),
see [`HARDENING.md`](../HARDENING.md) § "Container deployment".

---

## 6. Call the HTTP server from Python

Pure standard-library HTTP — no SDK needed. The full Python binding
(`pse-core` via PyO3) is in [`bindings/python/`](../bindings/python/);
this recipe is for environments where in-process binding is overkill.

```python
import json
import os
import urllib.request

URL = "http://127.0.0.1:8765/ingest"
TOKEN = os.environ["PSE_SERVER_TOKEN"]

def ingest(text: str) -> dict:
    body = json.dumps({"text": text, "session": 1, "question": ""}).encode()
    req = urllib.request.Request(
        URL,
        data=body,
        headers={
            "Authorization": f"Bearer {TOKEN}",
            "Content-Type": "application/json",
        },
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=10) as resp:
        return json.loads(resp.read())

print(ingest("sensor-1 reading 1.42 at t=0")["new_crystal_count"])
```

For in-process Python use:

```bash
cd bindings/python
maturin develop --release
python -c "import pse_core; s = pse_core.PseState(); print(s)"
```

See [`bindings/python/README.md`](../bindings/python/README.md) for
the full Python API.

---

## 7. Call the engine from a browser via WASM

The browser binding is `@kosmocrates/pse-wasm-web`. Build locally
(until the first npm release) with `wasm-pack`:

```bash
wasm-pack build crates/pse-wasm \
  --target web --release \
  --out-dir ../../web/pkg --scope kosmocrates
```

Use from a page (the `web/index.html` demo is wired this way):

```html
<script type="module">
  import init, { PseWasm } from './pkg/pse_wasm.js';
  await init();

  const pse = new PseWasm();
  pse.run(100);
  console.log(JSON.parse(pse.status()));
</script>
```

For Node-side use, swap `--target web` for `--target nodejs` and see
[`bindings/node/README.md`](../bindings/node/README.md).

ADR-0003 explains why we do not maintain a native N-API binding.

---

## 8. Investigate why a stream produced no crystals

The default `Config` thresholds are conservative — a low-signal stream
can legitimately produce zero crystals. To distinguish "thresholds
too high" from "engine is broken":

```bash
# Run any binary with gate-level tracing on.
RUST_LOG=pse_core=debug cargo run --release -p pse-demo
```

You'll see one `tracing::debug!` event per Kairos gate decision
(`crates/pse-core/src/lib.rs:702-738`). Reject events name which
threshold blocked the tick:

```
DEBUG pse_core: kairos rejected tick=42 d=0.12 d_min=0.30
```

That `d_min=0.30` is the gate's distance-of-stability threshold. To
loosen it for your stream, fork `Config::default()`:

```rust
let mut config = Config::default();
config.gate.d_min = 0.10;  // looser; expect more (but less stable) crystals
```

For the rigorous version (sweeping thresholds against a labelled
corpus), the `tools/pse-bench-bbo` and `tools/pse-paper-bench`
binaries are purpose-built.

---

## 9. Generate a CycloneDX SBOM for a release

The release workflow generates this automatically on every tag (see
[`RELEASING.md`](../RELEASING.md) § 2 and `.github/workflows/release.yml`).
To produce one locally:

```bash
cargo install --locked cargo-cyclonedx
cargo cyclonedx --format json --override-filename kosmocrates-sbom
```

The output is per-crate; for a single workspace-level SBOM, collect:

```bash
find . -name 'kosmocrates-sbom.cdx.json' -print0 \
  | tar --null --files-from=- -czf kosmocrates-sbom.tar.gz
sha256sum kosmocrates-sbom.tar.gz
```

This is exactly the artifact that gets attached to GitHub releases.

---

## 10. Wire a constitutional rule into pse-server

Constitutional rules are the governance layer that decides whether an
agent action is admissible against the current `EpistemicSignal`. The
canonical wiring path is via the `/nxalien/bundle` endpoint — push a
bundle that includes `rules`, and the server's live
`ConstitutionalEvaluator` is updated atomically.

```bash
cat > my-bundle.json <<'EOF'
{
  "metadata": { "source": "my-repo", "version": "0.1.0" },
  "rules": [
    {
      "id": "no-untrusted-network",
      "severity": "Blocking",
      "predicate": "action.type == \"network\" && !action.target.trusted",
      "rationale": "Refuse network actions to untrusted targets while signal is drifting.",
      "evidence": []
    }
  ]
}
EOF

curl -s -X POST http://127.0.0.1:8765/nxalien/bundle \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d @my-bundle.json | jq '.signal.stability'
```

Then check an action against the live rule set:

```bash
curl -s -X POST http://127.0.0.1:8765/constitutional/check \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"action": {"type": "network", "target": {"trusted": false}}}' | jq .
```

A response of `{"decision": "Block", ...}` means the rule fired. For
the dry-run (no state change) variant, see `POST /nxalien/validate`
in the `pse-server` startup banner.

---

## Contributing recipes

If you have a recipe that other users would benefit from, open a PR
adding it to this file. Keep each recipe self-contained and runnable
— no "see README for context" — and end with a link to deeper
documentation for the next level of detail.

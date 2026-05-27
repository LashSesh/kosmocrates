# pse-net

Distributed swarm networking for PSE — TCP peer-to-peer crystal propagation with Kuramoto-inspired acceptance

`pse-net` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

# pse-net — Distributed Swarm Networking for PSE

Provides TCP peer-to-peer crystal propagation for the Kosmocrates.
Multiple PSE instances can share crystals over the network using gossip-based
propagation with Kuramoto-inspired acceptance criteria.

## Architecture

- **SwarmNode**: Main coordinator managing TCP connections and crystal flow
- **CrystalEnvelope**: Content-addressed wrapper for network-propagated crystals
- **Kuramoto acceptance**: Phase-based acceptance criterion using spectral gap alignment
- **Transport**: Length-prefixed JSON framing over TCP with rate limiting

## Example

```no_run
use pse_net::{SwarmNode, SwarmConfig};

let mut config = SwarmConfig::default();
config.listen_addr = "127.0.0.1:0".to_string();
let mut node = SwarmNode::new(config);
node.start().expect("start");
println!("Listening on {:?}", node.local_addr());
```

## Add to your project

```toml
[dependencies]
pse-net = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-net --open`
(once published, also available on [docs.rs](https://docs.rs/pse-net)).

## License

MIT — see [`LICENSE`](../../LICENSE).

# Kosmocrates language bindings

This directory hosts the official non-Rust integrations for Kosmocrates.
Every binding is built from the same Rust core (`crates/pse-*`) and
inherits its determinism, content-addressing, and fail-closed contracts.

| Binding | Package | Source | Status |
|---|---|---|---|
| **Python** | [`pse-core`](https://pypi.org/project/pse-core/) on PyPI | [`bindings/python/`](python/) | abi3-py38 wheel — one wheel covers every CPython ≥ 3.8 |
| **Browser / ES module** | [`@kosmocrates/pse-wasm-web`](https://www.npmjs.com/package/@kosmocrates/pse-wasm-web) on npm | [`crates/pse-wasm/`](../crates/pse-wasm/) (build via wasm-pack `--target web`) | ES module + WASM payload |
| **Node.js** | [`@kosmocrates/pse-wasm-node`](https://www.npmjs.com/package/@kosmocrates/pse-wasm-node) on npm | [`crates/pse-wasm/`](../crates/pse-wasm/) (build via wasm-pack `--target nodejs`) | CommonJS module + WASM payload — see [`node/`](node/) for a Node-side quickstart |

> Until the first PyPI / npm publish, install from source per binding —
> the [Python README](python/README.md) and [Node README](node/README.md)
> cover that path. The release-tagged GitHub Actions workflows in
> [`.github/workflows/python.yml`](../.github/workflows/python.yml) and
> [`.github/workflows/npm.yml`](../.github/workflows/npm.yml) handle the
> publish step once the corresponding registry credentials are wired
> into the `pypi` / `npm` GitHub environments.

## Versioning policy

Every binding follows the workspace version in `Cargo.toml` —
`[workspace.package].version`. A tag `v0.2.0` produces binding releases
`pse-core==0.2.0` (PyPI) and `@kosmocrates/pse-wasm-{web,node}@0.2.0`
(npm) in lockstep with the Rust crates.

Until Kosmocrates ships `1.0.0`, minor bumps may carry breaking changes
across **every** binding. After `1.0.0`, semver is honoured for the
public surface of each binding.

## Adding a new binding

The accepted criteria for a new binding to live in this directory:

1. It is built from the existing Rust core — no separate reimplementation.
2. Its release process is reproducible from CI and gated behind a
   protected GitHub environment (no manual `cargo publish` / `twine` /
   `npm publish` from a workstation).
3. It carries a `README.md` covering install, a runnable hello-world,
   and the link back to the canonical Rust API documentation.
4. Its package version is sourced from the workspace `Cargo.toml`
   (or set in lockstep at release-time, as the npm/PyPI publishes do).

Native N-API bindings for Node are intentionally **not** maintained —
the WASM binding (`@kosmocrates/pse-wasm-node`) is the supported path
for Node consumers and covers every deterministic guarantee the core
provides.

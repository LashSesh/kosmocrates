# 0003. WASM (not native N-API) as the supported Node binding

- **Status:** Accepted
- **Date:** 2025-Q2 (back-filled)
- **Deciders:** @LashSesh

## Context

`crates/pse-wasm` already produces a browser-targeted WASM artifact
via `wasm-pack`. Node.js consumers historically had two paths to a
Rust engine:

1. **Native N-API** bindings (via `napi-rs`). Best raw throughput,
   but pulls a native build into `npm install`: a working C toolchain,
   per-platform prebuilds, GitHub-runner matrix for prebuild artifacts,
   and a separate version/release cadence from the workspace.
2. **WASM** via the existing `pse-wasm` crate compiled with
   `wasm-pack --target nodejs`. Slower than native (~3-5× on a tight
   inner loop), but ships as a single architecture-independent
   `.wasm` blob inside the npm package — no build at install time, no
   native toolchain at the consumer.

Practical questions for Kosmocrates specifically:

- The hot loop (`macro_step`) is allocation-light and runs in
  bursts, not continuously. The WASM ↔ JS marshalling overhead is
  paid per `process_text` call, not per crystal.
- Every determinism guarantee in ADR-0002 is preserved through the
  WASM compilation path; this was verified end-to-end via the same
  `replay_byte_identity` test driven from the Node side.
- The project is single-maintainer (`GOVERNANCE.md`). Maintaining
  two binding shapes — N-API + WASM, with separate release
  workflows, prebuild artifacts, and platform matrices — would
  double the release surface area.

## Decision

We will publish the official Node.js binding as a WASM package
(`@kosmocrates/pse-wasm-node`), compiled from the same `pse-wasm`
crate that powers the browser binding. Native N-API is **not
maintained** and PRs to add it will be declined absent a profile
showing the WASM ↔ JS overhead is the bottleneck for the requestor's
workload.

## Alternatives considered

| Option | Trade-off | Why rejected |
|---|---|---|
| Native N-API only | Best throughput. | Doubles release surface; native build on every consumer's machine. |
| WASM + N-API in parallel | Best of both. | Maintenance burden. Bug-fixes have to land in two places; release coordination is fragile. |
| WASM-only (chosen) | Slower hot loop; single binding. | The cost is in a layer (marshalling) that does not block deterministic correctness, the central project contract. Easy to revisit if a real workload surfaces. |

## Consequences

What becomes easier:

- `npm install @kosmocrates/pse-wasm-node` works on every platform
  Node runs on, including ones we have never tested (BSDs, IBM
  power architectures, etc.) — no per-platform prebuild matrix.
- Release coordination is a single `wasm-pack` step in
  `.github/workflows/npm.yml`; the package version stays in lockstep
  with the workspace version (per ADR-implicit policy in
  `bindings/README.md`).
- Determinism is preserved by construction: the WASM target is the
  same Rust source as native, just a different codegen backend.

What becomes harder:

- A Node user with a hot inner loop pays a ~3-5× marshalling tax
  per call vs native. For Kosmocrates-shaped workloads (bursts of
  observation ingestion, not millions of calls/sec) this is well
  below the noise floor.
- N-API-specific features (zero-copy Buffer access, native libuv
  integration) are not available. The WASM binding round-trips
  through `Uint8Array`.

What is now explicitly out of scope:

- A native N-API binding maintained alongside WASM.
- Per-release benchmarks comparing WASM vs hypothetical native
  throughput.

## Implementation notes

- `crates/pse-wasm/` — the single Rust source.
- `bindings/node/` — Node-facing README, hello-world example,
  install path.
- `.github/workflows/npm.yml` — builds two npm packages
  (`@kosmocrates/pse-wasm-web` for the browser, `@kosmocrates/pse-wasm-node`
  for Node) from the same crate via `wasm-pack --target {web,nodejs}`.
- `bindings/README.md` — explicitly calls out the "not maintained"
  status of native N-API so contributors do not invest in PRs that
  will be declined.

## References

- Internal: `crates/pse-wasm/`, `bindings/node/README.md`,
  `bindings/README.md`, `.github/workflows/npm.yml`.
- External: [wasm-pack documentation](https://rustwasm.github.io/docs/wasm-pack/),
  [napi-rs project](https://napi.rs) (the road not taken).

# Roadmap

A living document of where Kosmocrates is heading. Items are grouped
by their distance from the current state, not by calendar date —
this is a single-maintainer project (see `GOVERNANCE.md`) and dates
are unreliable.

> Cross-reference: every item that requires a release-cycle change
> is also listed in `RELEASING.md` § "Roadmap to 1.0".

## Near-term (next minor cycle)

These are committed and in active design:

- **API surface metadata sweep.** Per-crate `description`, `license`,
  `repository`, `readme`, and `[package.metadata.docs.rs]`. Adds
  `version =` to every workspace path dependency. Prerequisite for
  the gated `publish-crates` job in `release.yml`.
- **Supply-chain CI: `cargo-deny`.** Replaces the non-blocking
  `cargo audit` job with `cargo-deny check {advisories, licenses,
  bans, sources}` as a required check.
- **MSRV CI job.** Verifies that the workspace actually compiles on
  the declared `rust-version = "1.82"`.
- **Server hardening: AuthN.** Bearer-token middleware behind a feature
  flag in `tools/pse-server`. Documented in `HARDENING.md`.
- **OpenAPI generation for `pse-server`.** `utoipa`-based; served on
  `/openapi.json` and `/docs` (Swagger UI).
- **Observability conventions.** Workspace-wide tracing span naming
  and structured-logging contract documented in `OBSERVABILITY.md`,
  with `pse-server` adopting it first.

## Mid-term

These are scoped but not started:

- **First crates.io publish.** Conditional on the API metadata sweep
  landing and CI staying green for a full minor cycle. Initial
  publish order documented in `RELEASING.md` §5.
- **PyPI + npm first release.** Conditional on the `pypi` / `npm`
  GitHub environments being configured with credentials.
- **`cargo-semver-checks` in CI.** Required check on PRs that touch
  any `crates/*` library.
- **`#![deny(missing_docs)]` on core public-API crates.** `pse-core`,
  `pse-types`, `pse`, `pse-traverse`, `pse-evidence`, `pse-replay`.
- **`pse-gateway` deprecation or merge with `pse-server`.** Two HTTP
  surfaces is a maintenance burden; the more complete `pse-server`
  is the supported path.
- **End-user CLI consolidation.** The 18 binaries under `tools/` are
  mostly research / development utilities. The user-facing surface
  is `pse`, `nxalien`, `pse-server`, `pse-demo`, `pse-llm-demo` — the
  rest move under a `dev-tools` workspace feature so they do not
  appear in `--help` for downstream consumers.

## Long-term

Direction-setting, not committed:

- **`1.0.0`.** Strict semver across every published crate / package.
  Triggered when all near- and mid-term items above are green for a
  full minor cycle without regression.
- **Native Distroless container** for `pse-server` (currently
  `debian:bookworm-slim`). Requires switching `reqwest` to `rustls`
  to drop the OpenSSL runtime dependency.
- **Formal verification scaffolding** for the determinism contract,
  beyond the existing `replay_byte_identity` test (Kani / Creusot
  exploration).
- **Hosted reference deployment** of `pse-server` so downstream
  evaluators can try the API without local builds.

## Out of scope

Listed so contributors know not to send these as PRs:

- **Native Node N-API bindings.** The WASM binding
  (`@kosmocrates/pse-wasm-node`) is the supported path. See
  `bindings/README.md`.
- **Go / .NET / Ruby bindings** for the time being. Reconsidered if
  there is sustained downstream demand.
- **Refactoring away the determinism contract.** Pattern-memory crystals
  are byte-identical by construction; this is not negotiable.
- **A vector-database storage backend.** Kosmocrates is a
  topology-memory engine, not a similarity store. The Infinity
  Ledger semantic axis (`ψ`) is the supported semantic surface.

## How to influence the roadmap

Open an issue tagged `roadmap` with:

- The specific item.
- Your use case and why it matters.
- Whether you are offering to contribute.

The maintainer commits to responding within 14 days. Items with
contributor commitment move up in priority.

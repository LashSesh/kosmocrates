# 0002. Deterministic replay as a non-negotiable invariant

- **Status:** Accepted
- **Date:** 2024-Q4 (back-filled)
- **Deciders:** @LashSesh

## Context

Kosmocrates is positioned as the substrate that records *why* a piece
of knowledge was admitted into the memory field. That positioning is
worthless if running the same input twice can produce different
crystals — the audit trail becomes hand-wavy and the compliance story
(`docs/COMPLIANCE.md`) collapses.

Determinism is also the only mechanism that lets a downstream
investigator reproduce a production incident from the inputs alone,
without snapshotting the runtime state.

## Decision

We will treat **byte-identical replay** as a non-negotiable
project-wide invariant. Two runs of the engine over the same
`ProblemSpec` (or stream snippet) MUST produce identical
`TraversalRunReport` bytes after JCS canonicalisation. The
`replay_byte_identity` integration test enforces this in CI.

Concretely:

- No wall-clock time, no `HashMap` iteration order, no unseeded RNG
  in any code path that contributes to a content hash.
- `BTreeMap` over `HashMap` for keyed collections that are hashed.
- Floating-point values that end up in a content address pass through
  `CanonicalNumber::quantize_default` (scale-9 banker's rounding)
  first.

## Alternatives considered

| Option | Trade-off | Why rejected |
|---|---|---|
| Best-effort determinism + tolerance windows | Lets us use stdlib `HashMap` etc. without ordering discipline. | "Roughly the same" hashes are not addresses. The whole content-addressing story collapses if two runs of the same input give two different `crystal_id`s. |
| Deterministic in release, not in dev | Cheaper dev builds. | The contract would be silently violated whenever someone develops without rebuilding in release. A regression would only surface in CI, not at the call site that introduced it. |
| Per-crate determinism opt-in | Lets non-core crates use idiomatic Rust. | Crystals flow across crate boundaries through the PSE-Bridge. A single non-deterministic crate in the chain destroys the invariant for everyone. The bar has to be workspace-wide. |

## Consequences

What becomes easier:

- Content addressing actually works as advertised: `crystal_id` is
  the input's fingerprint, not a near-fingerprint.
- Cross-version regression detection is mechanical: rerun the corpus,
  diff the report bytes.
- Compliance audits (EU AI Act, see `docs/COMPLIANCE.md`) have a
  bit-level reproducibility argument, not a hand-wavy one.

What becomes harder:

- Contributors have to be alert to the determinism contract every
  time they pick a collection type, an RNG, or a parallel iterator.
  `CONTRIBUTING.md` § "Ground rules" lists the recurring pitfalls.
- Parallel-iteration speedups (`rayon`) require explicit care to
  produce results in a deterministic order, not the natural
  whatever-finishes-first order.

What is now explicitly out of scope:

- A "stochastic mode" with intentional non-determinism (e.g. for
  Monte-Carlo exploration). Any such mode must run *outside* the
  crystallization path and not produce `SemanticCrystal` records.

## Implementation notes

- `tests/integration/replay_byte_identity.rs` — the gate. CI runs
  it on every PR; a failure is the highest-priority regression class
  in the project per `CONTRIBUTING.md` § "Reporting bugs".
- `crates/pse-types/src/canonical.rs` — `CanonicalNumber` quantisation
  for floats.
- `crates/pse-evidence/src/lib.rs` — JCS canonicalisation
  (RFC 8785) over SHA-256 for crystal IDs.
- `crates/pse-capsule/src/lib.rs` — sealed-transport replay-detector
  ensures sealed payloads themselves cannot be replayed across keys.

## References

- Internal: `tests/integration/replay_byte_identity.rs`,
  `crates/pse-types/src/canonical.rs`,
  `crates/pse-evidence/src/lib.rs`,
  `CONTRIBUTING.md` § "Ground rules",
  `docs/COMPLIANCE.md`.
- Standards: [RFC 8785 — JCS](https://www.rfc-editor.org/rfc/rfc8785),
  [SHA-256 (FIPS 180-4)](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf).
- Specs: `specs/PSP_Formal_Specification_v1.0.0_Sebastian_Klemm.pdf`,
  `specs/HDAG_bySebastianKlemm_v1.0.pdf`.

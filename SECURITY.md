# Security policy

## Supported versions

PSE is pre-1.0. The project does **not** carry a security support
guarantee for any released version. Security fixes land on `main`
and roll into the next tagged release; older tags are not patched.

| Version | Status |
|---|---|
| `0.1.x` | Active development on `main` |
| `< 0.1` | Not supported |

## Reporting a vulnerability

**Please do not open a public GitHub issue for security reports.**

If you believe you have found a vulnerability — for example a
deterministic-replay bypass, a sealed-capsule counter-reuse not
caught by the detector, an unauthenticated path through the gateway,
or a falsifier-bypass that fabricates a `Crystal` — open a private
report via GitHub's "Report a vulnerability" surface on this
repository, or email the maintainer at the address listed under
`authors` in `Cargo.toml`.

Please include:

* A clear description of the vulnerability and the affected component
  (crate name + module path).
* A minimal reproducer if possible.
* Your assessment of impact (data integrity, replay determinism,
  confidentiality, availability).

We aim to acknowledge reports within **14 days**. Genuine
vulnerabilities will be triaged, fixed on a private branch, and
disclosed coordinated with you.

## Security-relevant components

If you are auditing PSE, these are the places that warrant the
closest look:

* `crates/pse-capsule` — AES-256-GCM sealed-transport with a
  counter-reuse detector. The contract is that any nonce reuse
  within a key's lifetime is detected and rejected.
* `crates/pse-evidence` — content-addressing (SHA-256 over JCS).
  A break here would mean two distinct evidence chains can collide
  on `crystal_id`.
* `crates/pse-replay` — deterministic replay invariant. A break here
  would mean a `Crystal` can be produced that does not survive
  replay, which violates the EU AI Act compliance proof sketch in
  `docs/COMPLIANCE.md`.
* `crates/pse-gateway` — HTTP gateway. There is no authentication
  layer on the default build; do not expose the gateway to the
  public internet without one.
* `crates/pse-traverse` — fail-closed contract: a gate failure NEVER
  produces a commit. A break here would mean the traversal agent
  emits a `SemanticCrystal` that PSE itself would have rejected.

## Threat model (informal)

In scope:

* An attacker with access to PSE inputs trying to make the engine
  produce a crystal that survives `pse-audit` or `replay` checks
  but is not actually deterministic.
* An attacker tampering with a transported sealed capsule.
* An attacker submitting crafted observations to force the engine
  into a state that bypasses falsification.

Out of scope:

* Denial of service via large inputs (PSE is a streaming engine; an
  operator who exposes it without bounds is the responsible party).
* Side-channel attacks on the host (timing, cache).
* Compromise of the host operating system, the build toolchain, or
  the `cargo` registry. Use `cargo audit` and verify checksums.

## Cryptographic primitives in use

| Use | Primitive | Crate |
|---|---|---|
| Content addressing | SHA-256 over JCS (RFC 8785) | `sha2`, `serde_jcs` |
| Sealed transport | AES-256-GCM | `aes-gcm` |
| Key derivation | HKDF-SHA-256 | `hkdf` |
| Determinism | xorshift64 (seeded) | hand-rolled, single source |

All are standard, audited primitives. The HKDF / AES-GCM choice
follows the seal-and-sign pattern in `pse-capsule`. PSE does not
roll its own crypto.

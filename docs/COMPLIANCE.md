# EU AI Act Compliance — Formal Proof Sketch for PSE

**Status:** research-grade proof sketch, not a legal compliance claim. This document maps PSE's mechanisms to the requirements of Regulation (EU) 2024/1689 (the EU AI Act), with explicit code references and the mathematical/cryptographic assumptions each mapping rests on. It is intended as the foundation an actual Conformity Assessment Body would build a compliance argument *on*, not the assessment itself.

---

## 1. What PSE is, in compliance terms

PSE is a **deterministic, content-addressed, replayable stream-processing system** with cryptographically-anchored audit artifacts ("crystals"). Three properties make it relevant to AI-Act high-risk-system requirements:

1. Every observation, every decision, and every committed artifact is **content-addressed via SHA-256 over RFC 8785 JCS canonical bytes** (`pse_types::content_address`, `crates/pse-types/src/lib.rs:629-635`). This is the substrate for record-keeping (Art. 12) and traceability (Art. 13).

2. Every committed crystal carries a **`CommitProof` with the full gate-snapshot, dual-consensus result, PoR trace, evidence-chain digests, and (optionally) a surrogate-data falsification p-value** (`crates/pse-types/src/lib.rs:303-340`). This is the substrate for human oversight (Art. 14) and accuracy/robustness (Art. 15).

3. Every run is fully reproducible from a `RunDescriptor` (`crates/pse-types/src/lib.rs:387-397`); two replays of the same descriptor over the same observation log produce **bit-identical crystal sequences** (Inv I4, asserted by `pse_replay::verify_determinism`, `crates/pse-replay/src/lib.rs:18-22`). This is the substrate for technical documentation (Art. 11) and the QMS replay requirement (Art. 17).

What PSE is **not**: a legal certification, a guarantee that any *deployed* system satisfies the Act, or an exemption from a Conformity Assessment. It is the engineering substrate.

---

## 2. Article-by-article mapping

Each subsection states (a) what the article requires in plain operational terms, (b) which PSE mechanism addresses it, with file/line references, and (c) the **formal assumptions** under which the mapping holds. Open gaps are listed where they exist.

### Article 9 — Risk Management System

**Requirement.** A risk management system must be established that identifies, estimates, and mitigates risks throughout the system's life cycle.

**PSE mechanism.** The constitutional constraint vocabulary (`pse_types::ConstitutionalConstraint`, `crates/pse-types/src/lib.rs:222-230`) lets the operator declare risk-management invariants as machine-checkable predicates carried inside the genesis crystal (`pse_types::GenesisMetadata`, lines 247-253). The audit pipeline (`tools/pse-audit/src/lib.rs`) enumerates and re-checks these on every artifact.

**Assumption.** The invariants chosen by the operator actually correspond to the risks the deployment introduces — PSE provides the *frame* for risk encoding, not the risks themselves.

**Open gap.** PSE does not currently *generate* candidate risks from a domain analysis. That step remains a human responsibility upstream of the engine.

### Article 11 — Technical Documentation

**Requirement.** Documentation must enable assessment of the system's compliance with the Act, including descriptions of components, capabilities, and the data on which the system was built.

**PSE mechanism.** A `RunDescriptor` (`crates/pse-types/src/lib.rs:387-397`) carries the full configuration, pinned operator versions, initial state digest, registry digests, and seed. Operator versions are recorded in `Registry` entries (`crates/pse-registry/src/lib.rs`) keyed by content-addressed `RegistryEntry` digests.

**Assumption.** SHA-256 collision-resistance (Art. 11 records become forgeable iff SHA-256 is broken) and JCS canonicality (RFC 8785).

### Article 12 — Record-Keeping (Logging)

**Requirement.** High-risk AI systems must automatically record events ("logs") over their lifetime. Logs must enable retroactive analysis of system behaviour.

**PSE mechanism.** This is the article PSE most directly addresses.

- Every `Observation` carries a payload digest, source-id, timestamp, provenance envelope, and schema version (`crates/pse-types/src/lib.rs:170-178`). The `EvidenceChain` field of every crystal hashes its observations together in order (`crates/pse-types/src/lib.rs:259-267`); `pse_evidence::build_evidence_chain` (`crates/pse-evidence/src/lib.rs:38-52`) chains entry digests, and `pse_evidence::verify_crystal` (lines 56-109) re-verifies the chain on every audit pass.
- The `Archive` (`crates/pse-evidence/src/lib.rs:164-222`) is **append-only by type signature** — `append(&mut self, …)` is the only mutation method. There is no public `remove` or `update`, so log integrity is enforced by the type system, not by a runtime check that could be disabled. This is **Inv I10** in PSE's invariant catalogue.
- Macro-step state transitions are recorded via the `EngineState` field on `GlobalState` (`crates/pse-core/src/lib.rs:51-70`); the 16-state FSM corresponds 1:1 to the canonical PSE phenomenology.
- Cross-tick reproducibility is verified by `pse_replay::compare_crystal_sequences` (`crates/pse-replay/src/lib.rs:25-35`).

**Assumption.** SHA-256 collision-resistance. (Forgery of a log entry that retains its existing digest requires breaking SHA-256.)

**Note on the boundary.** PSE does not write logs to disk on the operator's behalf — that is the role of `pse-store` and the deployment's choice of backing store. PSE provides the *content-addressed payloads* and the *integrity guarantee that any persisted form can be verified post-hoc* (`pse_audit::AuditReport::integrity_check`, `tools/pse-audit/src/lib.rs:76-108`).

### Article 13 — Transparency and Provision of Information

**Requirement.** High-risk AI systems must be designed so that operators can interpret their output, including by providing information about capabilities and limitations.

**PSE mechanism.** Three layered facilities:

1. **`resonance_fingerprint`** (`crates/pse-core/src/query.rs:79-145`) is the engine's read-modality. It returns the structured topology + Mandorla state + top-K resonant crystals + per-axis deviation field of the current moment, **without altering memory statistics** (test `fingerprint_query_does_not_alter_memory_stats`). An operator can ask the engine "what are you currently seeing?" and receive a deterministic, inspectable answer.

2. **`SemanticCrystal::commit_proof`** (`crates/pse-types/src/lib.rs:303-340`) carries the full gate-snapshot, the dual-consensus result with primal/dual scores and Mirror Consistency Index, the PoR FSM trace, and (optionally) the falsification p-value. Every committed artifact carries the *full evidence of why it was committed*.

3. **`describe_crystal`** functions in each domain adapter (e.g. `adapters/pse-adapter-seismo/src/lib.rs:372-390`, `adapters/pse-adapter-vitals/src/lib.rs:177-182`) provide human-readable narration with the medical/financial/seismological disclaimer baked in.

**Assumption.** The operator understands the meaning of the fields exposed (this is itself a documentation requirement under Art. 11, satisfied by THIS document and by the inline doc-comments on every public type).

### Article 14 — Human Oversight

**Requirement.** High-risk AI systems must be designed to enable effective human oversight, including the ability to override or interrupt the system.

**PSE mechanism.** Three reject-paths in `macro_step` (`crates/pse-core/src/lib.rs:386-697`):

1. **Kairos gate** (lines 384-398): if any of the eight gate metrics falls below its configured threshold, the engine sets `engine_state = Rejected("kairos failed")` and returns `Ok(None)` — the artifact is **not** committed.
2. **Seam check** (lines 498-501): if `n_seam < threshold`, reject.
3. **Dual consensus** (lines 531-537): if either path's stability score or the MCI is below threshold, reject.
4. **Surrogate falsification** (lines 540-577, when enabled): if the empirical p-value ≥ alpha and `gate_on_fail`, reject.

Each reject-path is operator-configurable via `Config.thresholds`, `Config.consensus`, and `Config.falsification`. The operator can tune the system to be arbitrarily conservative (no commits at all in the limit) or arbitrarily permissive.

**Assumption.** The operator's chosen thresholds correspond to their risk tolerance. The Engine provides the *gate*; the operator provides the *threshold*.

### Article 15 — Accuracy, Robustness, Cybersecurity

**Requirement.** The system must achieve appropriate levels of accuracy, robustness, and cybersecurity in light of its intended purpose, and operate consistently across its life cycle.

**PSE mechanism.**

- **Accuracy** is domain-specific and outside the engine's scope; PSE provides the substrate, the ground-truth benchmark `tools/pse-bench-gt` provides the measurement apparatus.
- **Robustness** is addressed by surrogate-data falsification (`pse_core::falsify::falsify_with_surrogates`, `crates/pse-core/src/falsify.rs`). When enabled, every crystal that commits has been compared against `k` permuted versions of its own evidence; the resulting p-value is the empirical statistical significance of its resonance peak vs. shuffled null. **An unattacked Engine and an Engine fed adversarially-crafted observations yield different falsification signatures**.
- **Cybersecurity** rests on:
  - **Content-addressed integrity**: tampering with a crystal payload changes its SHA-256, which is verified by `verify_content_address` (`crates/pse-evidence/src/lib.rs:112-137`).
  - **Hash-chained evidence**: tampering with one observation invalidates every downstream entry's `prev` digest (`crates/pse-evidence/src/lib.rs:38-52`).
  - **AES-256-GCM-encapsulated capsules** for confidential artifacts (`crates/pse-capsule/src/lib.rs:106-143`), with HKDF-derived session keys bound to the run descriptor digest.
  - **Deterministic replay**: any tampering is detectable by re-running with the same `RunDescriptor` and comparing crystal IDs (`pse_replay::compare_crystal_sequences`, `crates/pse-replay/src/lib.rs:25-35`).

**Critical assumption.** The capsule `nonce` is derived deterministically as `SHA-256(run_id || seal_counter)[0..12]` (`crates/pse-capsule/src/lib.rs`). This is **IND-CPA-secure under AES-GCM if and only if the (run_id, seal_counter) pair is unique per session_key**. Operators MUST ensure `seal_counter` monotonicity per `run_id`; resetting the counter while reusing a session_key is catastrophic for AES-GCM. PSE's persistence layer (`pse-store`) is responsible for enforcing this invariant; any deployment that bypasses `pse-store` MUST replicate the guarantee.

**Open gap.** No counter-reuse detector is currently embedded in `pse-capsule`. Adding one is the recommended hardening for any production deployment under Art. 15.

### Article 17 — Quality Management System (Replayability)

**Requirement.** Providers of high-risk AI systems shall put in place a quality management system that ensures compliance, including post-market monitoring.

**PSE mechanism.** The `RunDescriptor` + `Archive` pair gives a closed-form replay primitive: any historical run can be re-executed from its descriptor and its crystal sequence verified against the archive. `pse_replay::verify_determinism` (`crates/pse-replay/src/lib.rs:18-22`) returns `true` iff the descriptor's content-address is stable; `compare_crystal_sequences` (lines 25-35) returns a structured `ReplayResult` indicating which crystal IDs match between two runs.

For post-market monitoring, the `Archive` and `PatternMemory` together provide **cross-session evidence accumulation** without the system needing online learning: a deployed instance can persist crystals after each run, load them back via `load_memory_from_crystals` (`crates/pse-core/src/lib.rs:153-155`), and have its pattern recognition compound across deployments while every individual decision remains traceable to the specific run that produced it.

**Assumption.** The persistence layer is itself audit-able. PSE does not specify the storage medium; the operator's QMS must.

---

## 3. Cross-cutting cryptographic and mathematical assumptions

The compliance argument above rests on a small, explicit set of formal assumptions:

| # | Assumption | Used in |
|---|------------|---------|
| C1 | SHA-256 is collision-resistant | Art. 11, 12, 14, 15, 17 |
| C2 | JCS (RFC 8785) is canonical: equivalent JSON values produce identical byte serializations | Art. 11, 12, 17 |
| C3 | AES-256-GCM is IND-CPA secure under unique nonces | Art. 15 (capsules) |
| C4 | HKDF-SHA-256 produces session keys that are computationally indistinguishable from random under independent inputs | Art. 15 (capsules) |
| C5 | `(run_id, seal_counter)` is unique per session_key throughout the deployment's lifetime | Art. 15 (capsules) — operator-enforced |
| C6 | The `Archive::append` invariant (Inv I10) is preserved by all paths that touch the archive | Art. 12 — type-system-enforced |
| C7 | Floating-point determinism on the target platform: `cargo test --release` produces identical bit patterns across runs on the same hardware/OS | Art. 17 |

Assumptions C1–C4 are standard cryptographic primitives used as black boxes. Assumption C5 is the **only** operator-side obligation that PSE cannot itself enforce without integration with `pse-store`. Assumption C6 is enforced by Rust's borrow-checker against the public API. Assumption C7 holds for IEEE-754 deterministic implementations of f64 arithmetic on x86_64 / aarch64; the workspace test `embedding_is_deterministic_on_same_input` (`crates/pse-graph/src/lib.rs`) is the regression check.

---

## 4. What this document is NOT

- It is **not** a Conformity Assessment under Annex VI or VII of the AI Act.
- It is **not** a claim that any deployment of PSE is automatically compliant — Articles 9, 14, and 17 each impose operator-side obligations that PSE provides the substrate for, not the substance.
- It is **not** legal advice. Engagement of a qualified Notified Body remains the operator's responsibility for any high-risk AI system placed on the EU market.

---

## 5. References

- Regulation (EU) 2024/1689 — AI Act (the *Artificial Intelligence Act*).
- RFC 8785 — JSON Canonicalization Scheme (JCS).
- FIPS 180-4 — Secure Hash Standard (SHA-256).
- NIST SP 800-38D — AES-GCM.
- RFC 5869 — HMAC-based Extract-and-Expand Key Derivation Function (HKDF).
- ADAMANT Protocol — PSE's constitutional governance frame (Zenodo, CC BY 4.0).

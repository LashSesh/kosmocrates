# Implementation Decisions

Records architectural decisions, narrowings, and deferrals relative to the spec corpus.

---

## AD-001 — New `kosmo-*` crates vs. extending existing `pse-*` crates

**Decision:** Create new `crates/kosmo-core`, `crates/kosmo-workbench`, `crates/kosmo-hyphae`,
`crates/kosmo-systemcube` crates as specified in the handoff `§6 Spec-to-Module Map`.

**Rationale:** The existing `pse-*` crates implement the PSE substrate with different type surfaces
(e.g. `RunDescriptor` in `pse-types` is PSE-domain specific, not HYPHAE-specific).
Mixing would blur the spec boundary. The spec explicitly names `kosmo-*` module paths.
New crates can depend on existing `pse-types` where primitives overlap (e.g. `Hash256`,
`content_address`).

**Constraint:** The new crates must not copy/re-implement existing `pse-*` behavior.
Where `pse-types` already provides a working primitive, re-export or alias it.

---

## AD-002 — `Digest` as newtype over `Hash256`

**Decision:** Implement `Digest` as a newtype `struct Digest([u8; 32])` in `kosmo-core`,
wrapping the existing `Hash256 = [u8; 32]` from `pse-types`. The canonical serialization
function delegates to `pse_types::content_address` (JCS + SHA-256).

**Rationale:** Spec requires a named `Digest` type with explicit constructors. A newtype
prevents accidental misuse (e.g. passing a raw `Hash256` where a `Digest` is expected),
while reusing the already-tested hashing infrastructure.

---

## AD-003 — Fixed-point `Q16` implementation

**Decision:** Implement `Q16` as `struct Q16(i64)` representing value × 2^16.
Use `i64` to allow negative values (e.g. score deltas). Provide `from_f64`, `to_f64`,
and arithmetic with overflow checking. No `f32`/`f64` in gate/audit comparison paths.

**Rationale:** CROSS-007 and handoff §4 MVP-0 forbid floats in gate/audit paths.
`i64` representation with 16 fractional bits gives range ±140737 with precision ~0.0000153.
Sufficient for density/score/fitness values in the spec.

---

## AD-004 — `ImplementationMode` default is `ReportOnly`

**Decision:** `PolicyProfile::default()` must set `mode = ImplementationMode::ReportOnly`
and all `allow_*` booleans to `false`. Any construction path that omits a `PolicyProfile`
argument must use the default (fail-closed).

**Rationale:** CROSS-001, CROSS-002; handoff §3. The system must be inert without
an explicit policy escalation.

---

## AD-005 — `EvidenceBundle` is distinct from `EvidenceChain`

**Decision:** `EvidenceBundle` (new, in `kosmo-core`) is a structured collection of typed
evidence refs with a bundle digest and replay status. It is distinct from `EvidenceChain`
(existing, in `pse-evidence`) which is a hash-chained list of raw entries. HYPHAE will use
`EvidenceBundle`; existing PSE layers continue to use `EvidenceChain`.

**Rationale:** The spec defines `EvidenceBundle` as the unit of evidence for HYPHAE operations.
It must be content-addressed, carry a `ReplayStatus`, and be policy-scoped. The existing
`EvidenceChain` is lower-level and PSE-internal.

---

## AD-006 — `kosmo-hyphae` as single crate with submodules

**Decision:** All HYPHAE v0.3 through v0.4.2 code lives in `crates/kosmo-hyphae` as
submodules (`host`, `cube`, `swarm`, `corpus`, `metatron`, `lpcm`), consistent with
the handoff §6 module map. `kosmo-systemcube` is a separate crate per spec.

**Rationale:** Keeps HYPHAE phases co-located for easy cross-module references while
respecting the crate boundary to SystemCube (which is an independent export layer).

---

## AD-007 — No mutation of host project files until Phase 11

**Decision:** All phases up to and including Phase 10 produce report-only or dry-run
artifacts. No code modifies files outside the kosmocrates workspace itself.
Phase 11 (OperatorApproved materialization) is blocked until explicit user authorization.

**Rationale:** Hard safety rules in the system prompt and handoff §2.

---

## AD-008 — Vendor ledger (`vendors/infinityledger`) not treated as HYPHAE SourceCube

**Decision:** The `vendors/infinityledger` code is not imported as a SourceCube source
in any automated HYPHAE run. It is treated as an existing vendored dependency of the PSE
substrate, separate from the HYPHAE assimilation pipeline.

**Rationale:** Hard safety rule §2 item 1: no raw external source code enters default
ContextPack. Vendored code requires explicit policy and Foundry gate before inclusion.

---

## AD-009 — Existing `pse-metatron` vs. new Metatron v0.4.1 layer

**Decision:** The existing `pse-metatron` crate implements graph-theoretic Metatron scan
(scaffold invariants, spectral analysis, platonic solids — a different scope). The new
`kosmo-hyphae/src/metatron/` module implements the v0.4.1 `MetatronScanKernel`, region
extraction, micrograph lifting, and microtopology diagnostics. The two are separate.

**Rationale:** The v0.4.1 spec defines HYPHAE-specific microtopology tooling. The existing
`pse-metatron` is a lower-level graph analysis library used by the PSE engine. They serve
different purposes and must not be conflated.

---

## AD-010 — Real executors live in separate host-capability crates

**Decision:** `kosmo-foundry` (process execution), `kosmo-store` (disk persistence), `kosmo-parseback` (cargo metadata + filesystem walk), and `kosmo-operator` (orchestration) each live in their own crate outside `kosmo-core`.

**Rationale:** `kosmo-core` is wasm-portable and process-free. Host capabilities (spawn, filesystem I/O, cargo invocation) require OS access that must not be available in wasm builds. The same isolation principle already applied to `kosmo-pse-bridge`.

---

## AD-011 — Disk persistence requires `allow_host_write`; DryRun cannot persist

**Decision:** `JsonlCartographyStore::append()` checks `policy.allow_host_write` in addition to the `ReportOnly` check. This means `DryRun` (which sets `allow_host_write = false`) cannot persist to disk; only `OperatorApproved` can.

**Rationale:** A durable append is a host write. The policy bit `allow_host_write` already governs the Foundry sandbox. Extending the same bit to the persistence layer creates a single invariant: `allow_host_write = false` means no host mutations anywhere in the stack (execution or storage).

---

## AD-012 — ParseBack snapshots crate-level topology from `cargo metadata`

**Decision:** `kosmo-parseback` uses `cargo metadata --no-deps` to capture the crate dependency graph and file topology. The snapshot is content-addressed at the crate level (name × sorted source files × sorted dep names). No Rust AST parsing.

**Rationale:** AST-level topology would require a full Rust parser dependency and would be prohibitively slow for a pre/post diff. Crate-level topology captures the structurally meaningful changes (crate added/removed, dep edge added/removed, file count changed) that ParseBack is defined to detect.

---

## AD-013 — TopologyChangeKind severity mapping is fail-closed

**Decision:** `NodeRemoved` and `EdgeRemoved` map to `ParseBackSeverity::Critical` (→ `Failed`); `NodeAdded` and `EdgeAdded` map to `Warning`; `NodeModified` maps to `Info`. Any `Critical` delta drives the overall `ParseBackOutcome` to `Failed`.

**Rationale:** Destructive structural changes (a crate or dependency disappearing) are more likely to indicate a regression or corruption than additive changes. Fail-closed means questionable state is never silently promoted.

---

## AD-014 — OperationReport is content-addressed over all sub-report IDs

**Decision:** `OperationReport.report_id = SHA256(JCS({plan_id, foundry_report_id, parseback_report_id, closure_report_id, elapsed_ms, persisted}))`. It does not include the full sub-report payloads, only their IDs.

**Rationale:** Content-addressing by ID rather than payload keeps the OperationReport lightweight while preserving INVARIANT-007 (identical inputs → identical ID). Any change in any sub-report cascades up through its ID to change the operation report ID.

---

## AD-015 — Unified tripolar energy kernel (`D = ψ · ρ · ω`) in `kosmo-core`

**Decision:** Add `kosmo-core::energy` with `TripolarEnergy { psi, rho, omega }`, `EnergyFactors` (gate/taint/license/foundry/seam/contradiction), `EnergyKernel`, a content-addressed evidence-bound `EnergyAssessment`, and `rank_by_energy`. The selection energy is `D · ∏ factors`, computed entirely in `Q16` integer arithmetic. This is the single ranking core the substrate's previously-fragmented heuristics (`SourceCube.support_score`, LPCM support-mass majority, SystemCube D-density, `NormGene` fitness) can converge onto.

**Rationale:** The vision requires one consistent "tripolar energy" selection kernel rather than per-layer heuristics. The PSE retrieval layer already uses `D = ψ·ρ·ω` (`pse-adapter-il::score_tripolar`) but in `f64`; the kosmo substrate keeps it in `Q16` because the value participates in gate-adjacent selection and CROSS-007 forbids floats there. The kernel is additive — it does not rip out existing scores, so adoption is incremental and low-risk.

**Hard invariant (non-bypass, CROSS-010):** Energy *ranks*; it never *gates*. The module exposes no method turning an energy value into a decision or policy escalation. The only gate interaction is the `gate` factor, a floor enforcer: a `Reject` `GateResult` forces that factor to zero, so a rejected candidate's energy is zero and can never out-rank a passing one — but a high `D` can never flip a `Reject` into an `Accept`. Selection by energy is always a choice *among* candidates that have independently passed their gates. This is enforced by tests (`energy_ranks_but_never_bypasses_gate`, `rx-energy-reject-gate-never-bypassed`).

---

## AD-016 — Real code topology via dependency-free lexical extraction

**Decision:** Upgrade `kosmo-hyphae::code_hdag` from the one-node skeleton to a real extractor, `CodeHDAG::extract_from_rust_source`, that lexically scans Rust source line-by-line and emits real nodes (modules, imports, fn/type/test definitions) and edges (`Imports`, `Contains`, `Tests`, `Implements`). It uses no parser dependency (`syn`/`tree-sitter`); each observation's `fragment_digest` is bound to its `location:line:text` so the graph is content-addressed down to the source line.

**Rationale:** The first link of the vision chain ("echte Topologie rein") was missing — topology was crate/file-skeletal everywhere. A full AST parser would add a heavy dependency and would be slow for the pre/post diff use case. A lexical extractor captures the structurally meaningful skeleton (module/import/definition graph) cheaply, deterministically (INVARIANT-007), and wasm-portably, while honestly *not* claiming to resolve macros, multi-line signatures, or call graphs.

**Topology → energy bridge:** `CodeHDAG::rho_coherence()` and `omega_phase()` derive the structural poles ρ and ω of the tripolar energy directly from the extracted graph (test coverage of definitions; presence of imports/definitions/tests). The semantic pole ψ is **not** derived from syntax — meaning cannot come from structure alone, so the caller supplies it. The derivations feed ranking only and never act as a gate.

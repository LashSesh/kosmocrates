# Kosmocrates / Workbench / HYPHAE Spec Corpus — Implementation Handoff

**Purpose:** This document gives the uploaded specification bundle a single red thread for implementation.  
It is not a replacement for the specs. It is the coordination layer that tells an implementation agent how to read the corpus, in which order to build it, which documents are canonical for which layer, and which safety boundaries must remain intact.

**Primary implementation rule:** build from the substrate upward, and keep the first implementation report-only/dry-run until Foundry-backed validation and policy gates exist.

---

## 0. Canonical Reading Order

Use the documents in this order.

### Canonical target specs

1. `kosmocrates_workbench_master_spec_v0_1.pdf`  
   Root production substrate: Workbench, Foundry, capability locks, ContextPack, TaskSpec, operator/runtime model.

2. `kosmocrates_hyphae_master_spec_v0_3.pdf`  
   Canonical run-local HYPHAE assimilation engine: HostCube, SourceCube, CubeSwarm, HostTargetDelta, StructuralYield, GateCascade.

3. `kosmocrates_hyphae_master_spec_v0_4.pdf`  
   Persistent morphogenic extension: CorpusCartography, StructuralCrystal, AssimilationCertificate, NormGene, HostTargetCollapsePlan, MorphogenicCorpusUpdate.

4. `kosmocrates_hyphae_metatron_extension_v0_4_1.pdf`  
   Microtopology extension: MetatronScanKernel, RegionExtraction, MicrographLifting, SemanticLossRecord, MicroTopologyDiagnostic, planning-only SurgeryOptions.

5. `kosmocrates_hyphae_master_spec_v0_4_2.pdf`  
   LPCM extension: controlled fragmentation, support mass, local condensation, seam percolation, passive collapse reports.

6. `kosmocrates_hyphae_master_spec_v0_4_3.pdf`  
   SystemCube extension: exportable SystemCube / `.kcube`, D-density, blueprint condensation, dry-run materialization path.

### Historical / reference-only specs

- `kosmocrates_hyphae_master_spec_v0_1.pdf`
- `kosmocrates_hyphae_master_spec_v0_2.pdf`

Use these to understand evolution and terminology. Do not implement outdated names, surfaces, or behaviors when contradicted by v0.3+.

---

## 1. Stack Position

The implementation stack is:

```text
Kosmocrates / PSE
  content addressing, evidence, ledger, authority, taint, governance, replay

Workbench / Foundry
  workspace model, TaskSpec, ContextPack, isolated operations,
  artifact generation, build/test/lint/typecheck/security/parse-back validation

HYPHAE v0.3
  host-bound run-local topology assimilation:
  HostCube -> SourceCubes -> CubeSwarm -> StructuralYield -> GateCascade

HYPHAE v0.4
  persistent morphogenic layer:
  CorpusCartography -> StructuralCrystals -> NormGenes -> CollapsePlans

HYPHAE v0.4.1 Metatron
  bounded microtopology:
  Region -> Micrograph -> Fingerprint -> Diagnostic -> planning-only SurgeryOption

HYPHAE v0.4.2 LPCM
  controlled fragmentation and local-percolative collapse:
  FragmentField -> SupportMass -> LocalCondensation -> SeamGraph -> report

HYPHAE v0.4.3 SystemCube
  exportable topological blueprint:
  SystemCube -> D-density -> Compatibility -> dry-run materialization path
```

The lower layer always constrains the upper layer. Higher layers may guide, score, plan, or propose; they must not become authority by existence.

---

## 2. Hard Boundaries

These rules override all implementation convenience.

```text
1. No raw external source code enters default prompts, host patches, trusted memory,
   governance, tool authorization, or SystemCube packages.

2. No acquired repository is executed by default.

3. No host file is modified during early implementation phases.

4. CorpusCartography is not trusted memory.

5. NormGene is not a trusted norm.

6. StructuralCrystal is not executable code.

7. CollapsePlan is not mutation authority.

8. SurgeryOption is not a patch.

9. LPCM "patch" means candidate direction, not file patch.

10. SystemCube materialization is dry-run or operator-confirmed by default.

11. SyntheticSourceCube is disabled by default until evidence, gates, and feedback loops exist.

12. Every durable object must be evidence-bound, content-addressed, policy-scoped,
    and replayable or explicitly marked replay-incomplete.
```

---

## 3. Global Implementation Mode

Begin with:

```text
ImplementationMode = ReportOnly
```

Allowed in `ReportOnly`:

- scan workspace;
- compute digests;
- build HostCube skeleton;
- produce VoidMap / DeficiencyVector;
- produce SourceFrontier proposals;
- produce EvidenceBundle;
- produce GateTrace;
- produce diagnostic reports;
- produce planning artifacts;
- write ledger events for reports.

Forbidden in `ReportOnly`:

- editing host files;
- executing acquired repositories;
- importing raw external code;
- promoting trusted memory;
- materializing SystemCubes;
- automatic Workbench patching;
- bypassing Foundry.

Only move from `ReportOnly` to `DryRun`, `OperatorApproved`, or `AutonomousBounded` through an explicit `PolicyProfile`.

---

## 4. One MVP, Not Many MVPs

The corpus contains several local MVPs. For implementation, use this unified MVP ladder.

### MVP-0 — Substrate Boot

Goal: establish common types and deterministic serialization.

Includes:

- `Digest`
- `EvidenceRef`
- `EvidenceBundle`
- `AuthorityLabel`
- `TaintLabel`
- `LicenseStatus`
- `CapabilityLock`
- `PolicyProfile`
- `RunDescriptor`
- `GateResult`
- `LedgerEvent`
- `FoundryCheckResult`
- canonical serialization profile

Definition of Done:

- stable digest for deterministic structures;
- no unordered map iteration in digest path;
- no floats in audit path; use fixed-point / Q16 or rational values;
- policy can force report-only behavior.

---

### MVP-1 — Workbench Dry-Run Substrate

Goal: create the production substrate HYPHAE will depend on.

Includes:

- workspace index;
- ContextPack with permitted-use boundaries;
- TaskSpec;
- isolated worktree / dry-run command model;
- Foundry command runner interface;
- event/ledger sink;
- operator-readable run report.

Definition of Done:

- a workspace can be scanned;
- a TaskSpec can be created;
- Foundry can run configured checks in dry-run or isolated mode;
- EvidenceBundle is emitted;
- no host mutation occurs without policy.

---

### MVP-2 — HYPHAE v0.3 Passive Run

Goal: implement the run-local topology assimilation skeleton without external risk.

Includes:

- HostBinding;
- HostCube skeleton;
- TopologicalVoidMap;
- DeficiencyVector;
- SourceFrontierGraph;
- bounded SourceEvidence model;
- Rust-first CodeObservation / CodeHDAG;
- StructuralYield;
- GateCascade;
- CandidateEvidenceRecord.

Definition of Done:

- a local host repository produces a HostCube and VoidMap;
- MissingTestFiber-like deficiencies can be represented;
- a StructuralYield cannot be Workbench-usable without HostVoid / Deficiency reference;
- every yield passes GateCascade;
- raw external code never enters default ContextPack.

---

### MVP-3 — CubeSwarm MVP

Goal: implement the core v0.3 assimilation structure.

Includes:

- SourceCubeWorker;
- SourceCube;
- CubeDimensionProfile;
- CubeSwarm;
- CubeMandorla;
- CompositeSupportCube;
- HostTargetDelta;
- deterministic merge ordering.

Definition of Done:

- two worker completion orders produce the same semantic output;
- CompositeSupportCube contains topology, mesh, fibers, phase, motifs, conflicts, scores, evidence only;
- no raw code is merged;
- NoConvergence / PartialEvidence is emitted when support is insufficient.

---

### MVP-4 — HYPHAE v0.4 Persistence and Certification Skeleton

Goal: make v0.3 outputs reusable without turning them into authority.

Includes:

- CorpusCartography;
- SourceCubeIndex;
- MotifIndex;
- NegativeEvidenceIndex;
- CartographyPrecheck;
- StructuralCrystalCandidate;
- AssimilationCertificate;
- ReplayProof;
- NormGeneCandidate;
- HostTargetCollapsePlan;
- MorphogenicCorpusUpdate minimal.

Definition of Done:

- second run can reuse prior negative evidence for ranking;
- StructuralCrystalCandidate requires evidence and certificate;
- NormGeneCandidate is not trusted norm;
- CollapsePlan is planning guidance, not mutation;
- CorpusCartographyUpdate has before/after digest and replay manifest.

---

### MVP-5 — Metatron v0.4.1 M1/M2

Goal: add bounded microtopology without overclaiming semantics.

Includes:

- TopologyRegionRef;
- RegionExtractionProfile;
- ProjectionProfile;
- SemanticLossRecord;
- MetatronMicrograph;
- MicrographLiftReport;
- MetatronRegionFingerprint;
- MicroTopologyDiagnostic;
- TopologyAmbiguityProfile;
- ComplementVoidHypothesis;
- MicroTopologyIndex.

Definition of Done:

- a HostVoidRegion can be lifted into a bounded MetatronMicrograph;
- every micrograph preserves HDAG node/edge backrefs;
- lossy projection emits SemanticLossRecord;
- canonical hash is deterministic;
- fingerprint alone cannot certify or prove semantic equivalence;
- MicroTopologyIndex groups recurring local forms.

---

### MVP-6 — LPCM v0.4.2 Passive Report

Goal: implement controlled fragmentation as reporting, not patching.

Includes:

- FragmentField;
- CandidateDirection;
- SupportMassVector;
- LocalCondensationCandidate;
- SeamGraph;
- DoFContractionReport;
- planning-only CollapsePlanStep.

Definition of Done:

- deterministic fragments;
- support vector normalized and gate-aware;
- 51 percent dominance emits candidate only, never truth;
- seam compatibility thresholds are explicit;
- LPCM does not write host files;
- report is useful before any materialization exists.

---

### MVP-7 — SystemCube v0.4.3 Passive Export

Goal: export topological blueprints without materialization.

Includes:

- SystemCube manifest;
- canonical package digest;
- BlueprintUnit;
- D-density / contradiction-energy report;
- compatibility profile;
- validation templates;
- dry-run build/export CLI.

Definition of Done:

- `host.kcube` can be built from a host snapshot;
- manifest round-trips with identical digest;
- opaque blueprint units are rejected;
- projection loss is recorded;
- materialization is dry-run or blocked by policy.

---

### MVP-8 — Controlled Workbench Materialization

Goal: allow operator-approved planning artifacts to become Workbench tasks and Foundry-validated changes.

Includes:

- WorkbenchPlanningArtifact;
- SurgeryWorkbenchTask;
- CollapseValidationPlan;
- SystemCube materialization request;
- Foundry execution;
- parse-back topology;
- outcome learning.

Definition of Done:

- materialization requires policy and operator approval unless explicitly allowed;
- generated changes go through Workbench task flow;
- Foundry passes or failure is captured;
- parse-back compares expected topology;
- failures become NegativeEvidence.

---

## 5. Phase Dependency Graph

```text
MVP-0 Core Types
  -> MVP-1 Workbench Dry-Run
      -> MVP-2 HYPHAE v0.3 Passive
          -> MVP-3 CubeSwarm
              -> MVP-4 v0.4 Persistence/Certification
                  -> MVP-5 Metatron Microtopology
                      -> MVP-6 LPCM Passive
                          -> MVP-7 SystemCube Passive Export
                              -> MVP-8 Controlled Materialization
```

Do not start a phase that depends on durable evidence before the evidence layer exists.  
Do not start materialization before Foundry and parse-back exist.

---

## 6. Spec-to-Module Map

### Core / PSE

```text
crates/kosmo-core/
  digest.rs
  canonical.rs
  fixed_point.rs
  policy.rs
  authority.rs
  taint.rs
  license.rs
  capability.rs
  error.rs
```

### Evidence / Ledger

```text
crates/kosmo-evidence/
  evidence_bundle.rs
  evidence_ref.rs
  ledger_event.rs
  replay_manifest.rs
  provenance.rs
```

### Workbench

```text
crates/kosmo-workbench/
  workspace.rs
  task_spec.rs
  context_pack.rs
  foundry.rs
  artifact_ir.rs
  operator.rs
  report.rs
```

### HYPHAE v0.3

```text
crates/kosmo-hyphae/
  run.rs
  host.rs
  void_map.rs
  deficiency.rs
  frontier.rs
  acquisition.rs
  source_evidence.rs
  parsing.rs
  code_hdag.rs
  cube.rs
  cube_swarm.rs
  motif.rs
  structural_yield.rs
  gates.rs
  assimilation.rs
  integration.rs
  metrics.rs
```

### HYPHAE v0.4

```text
crates/kosmo-hyphae/
  corpus_cartography.rs
  corpus_entity.rs
  corpus_relation.rs
  cartography_precheck.rs
  cartography_update.rs
  tritemporal_clock.rs
  phase_ladder.rs
  carrier_migration.rs
  structural_crystal.rs
  dual_fabric.rs
  assimilation_certificate.rs
  norm_gene.rs
  norm_fitness.rs
  collapse_plan.rs
  morphogenic_update.rs
  retention.rs
  decay.rs
  supersession.rs
  quarantine.rs
```

### Metatron v0.4.1

```text
crates/kosmo-hyphae/src/metatron/
  region_extraction.rs
  projection.rs
  micrograph.rs
  fingerprint.rs
  diagnostic.rs
  surgery.rs
  gates.rs
  certificate.rs
  corpus_index.rs
  outcome.rs
```

### LPCM v0.4.2

```text
crates/kosmo-hyphae/src/lpcm/
  fragment_field.rs
  candidate_direction.rs
  support_mass.rs
  local_condensation.rs
  seam_graph.rs
  coarse_grain.rs
  contraction_report.rs
```

### SystemCube v0.4.3

```text
crates/kosmo-systemcube/
  manifest.rs
  blueprint_unit.rs
  topology.rs
  energy.rs
  compatibility.rs
  adapter.rs
  accretion.rs
  materialization.rs
  parseback.rs
  cli.rs
```

Start as modules inside fewer crates if needed. Boundary clarity matters more than crate count.

---

## 7. PolicyProfile Must Exist Early

Implement this early enough that all subsystems can consume it.

```rust
pub enum ImplementationMode {
    ReportOnly,
    DryRun,
    OperatorApproved,
    AutonomousBounded,
}

pub struct PolicyProfile {
    pub id: Digest,
    pub mode: ImplementationMode,

    pub allow_network: bool,
    pub allow_external_acquisition: bool,
    pub allow_acquired_repo_execution: bool,
    pub allow_host_write: bool,
    pub allow_context_injection_from_external: bool,

    pub allow_synthetic_sourcecube: bool,
    pub allow_metatron_surgery_planning: bool,
    pub allow_lpcm_materialization: bool,
    pub allow_systemcube_materialization: bool,
    pub allow_memory_promotion: bool,

    pub require_foundry_for_executable_effects: bool,
    pub require_parseback_for_topology_changes: bool,
    pub require_operator_approval_for_materialization: bool,
}
```

Default:

```text
mode = ReportOnly
allow_network = false
allow_external_acquisition = false
allow_acquired_repo_execution = false
allow_host_write = false
allow_synthetic_sourcecube = false
allow_systemcube_materialization = false
allow_memory_promotion = false
require_foundry_for_executable_effects = true
require_parseback_for_topology_changes = true
require_operator_approval_for_materialization = true
```

---

## 8. Agent Task Template

When asking an implementation agent to implement a phase, use this shape.

```markdown
# Task: <phase/module>

## Source specs
- <PDF name + sections>

## Goal
<one paragraph>

## Implement
- <type/module/interface list>

## Do not implement
- <explicit exclusions>

## Required behavior
- <rules>

## Tests
- <unit/integration tests>

## Completion criteria
- <DoD>
```

Example:

```markdown
# Task: MVP-5 Metatron M1/M2

## Source specs
- kosmocrates_hyphae_metatron_extension_v0_4_1.pdf
- kosmocrates_hyphae_master_spec_v0_4.pdf

## Goal
Implement bounded HYPHAE region extraction, micrograph lifting,
fingerprinting, semantic-loss tracking, and diagnostic output.

## Implement
- TopologyRegionRef
- RegionExtractionProfile
- ProjectionProfile
- SemanticLossRecord
- MetatronMicrograph
- MicrographLiftReport
- MetatronRegionFingerprint
- MicroTopologyDiagnostic
- MicroTopologyIndex

## Do not implement
- host file edits
- automatic surgery materialization
- StructuralCrystal certification based only on hash
- trusted memory promotion

## Tests
- deterministic canonical hash
- backrefs preserved
- high semantic loss downgrades evidence
- fingerprint alone cannot certify
```

---

## 9. Cross-Cutting Acceptance Tests

These must remain true across all phases.

```text
CROSS-001:
Run without policy profile defaults to ReportOnly.

CROSS-002:
Host mutation is impossible unless PolicyProfile allows it.

CROSS-003:
External acquisition without capability is blocked.

CROSS-004:
Acquired source never executes by default.

CROSS-005:
Raw external code never enters default ContextPack.

CROSS-006:
Every durable object has digest, evidence refs, policy scope and replay status.

CROSS-007:
Every gate-relevant numeric value uses fixed-point or rational representation.

CROSS-008:
Every materialization path declares Foundry checks.

CROSS-009:
Every topology-changing materialization declares parse-back expectations.

CROSS-010:
No support count, resonance, norm fitness, D-density, 51% majority or catalog match bypasses gates.

CROSS-011:
Synthetic artifacts are low-authority and tainted.

CROSS-012:
Negative evidence is persisted and can affect future ranking.

CROSS-013:
Report-only mode produces useful diagnostics without writing host files.

CROSS-014:
Implementation can resume/replay from content-addressed artifacts.

CROSS-015:
If replay cannot be guaranteed, object is marked replay-incomplete.
```

---

## 10. Recommended First Claude Code Prompt

Use this before uploading all PDFs as active implementation targets.

```text
You are implementing the Kosmocrates Workbench + HYPHAE specification bundle.

Read the specs as a layered corpus, not as independent systems.

Canonical implementation order:
1. Workbench v0.1
2. HYPHAE v0.3
3. HYPHAE v0.4
4. Metatron v0.4.1
5. LPCM v0.4.2
6. SystemCube v0.4.3

Historical specs v0.1/v0.2 are context only unless a later spec explicitly preserves a concept.

Default implementation mode is ReportOnly.
Do not write host files, execute acquired repositories, import raw external code,
promote trusted memory, or materialize SystemCubes until the required policies,
EvidenceBundles, GateTraces, Foundry checks and parse-back validation exist.

Start by creating core shared types:
Digest, CanonicalSerializationProfile, Q16, EvidenceRef, EvidenceBundle,
AuthorityLabel, TaintLabel, LicenseStatus, CapabilityLock, PolicyProfile,
RunDescriptor, GateResult, LedgerEvent and FoundryCheckResult.

Then implement MVP-0 and stop for review.
```

---

## 11. Implementation Review Gates

After each phase, stop and review.

| Phase | Stop condition |
|---|---|
| MVP-0 | shared types compile; digest tests pass |
| MVP-1 | Workbench dry-run can scan workspace and emit EvidenceBundle |
| MVP-2 | HYPHAE passive run emits HostCube, VoidMap, DeficiencyVector, GateTrace |
| MVP-3 | deterministic CubeSwarm digest under worker-order variation |
| MVP-4 | CorpusCartography update has replay manifest; NormGeneCandidate not trusted |
| MVP-5 | Metatron micrograph preserves backrefs and loss records |
| MVP-6 | LPCM report is deterministic; no host write |
| MVP-7 | SystemCube export round-trips digest; no materialization |
| MVP-8 | Workbench task + Foundry + parse-back closes loop |

Do not proceed when a stop condition fails.

---

## 12. Final Build Maxim

```text
Build the skeleton before the intelligence.
Build evidence before memory.
Build gates before candidates.
Build reports before materialization.
Build Foundry before execution.
Build parse-back before trusting outcomes.
Build CorpusCartography as memory-shaped evidence, not authority.
```

The intended result is not one giant autonomous tool.  
It is a layered, fail-closed, content-addressed production substrate where every increasingly powerful layer remains grounded by evidence, policy, replay and validation.

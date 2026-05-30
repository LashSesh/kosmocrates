# Safety Policy

This document defines the default constraints for all Kosmocrates / HYPHAE implementation phases.

## Default Implementation Mode

```
ImplementationMode = ReportOnly
```

This is the mandatory default until an explicit `PolicyProfile` with an elevated mode is constructed
and confirmed by an authorized operator.

## What is Allowed in `ReportOnly` Mode

- Scan workspace and index files
- Compute content-addressed digests
- Build `HostCube` skeleton and `TopologicalVoidMap`
- Produce `DeficiencyVector` / `SourceFrontierGraph` proposals
- Emit `EvidenceBundle` and `GateTrace`
- Produce diagnostic reports and planning artifacts
- Write `LedgerEvent` entries for reports
- Append to `CorpusCartography` (read/plan only, not mutation authority)
- Produce `HostTargetDelta` as report-only artifact
- Produce `HostTargetCollapsePlan` as planning guidance only

## What is Forbidden in `ReportOnly` Mode

- Editing host project files
- Executing acquired repositories
- Importing raw external source code into ContextPack or prompts
- Promoting CorpusCartography to trusted memory
- Materializing SystemCubes
- Automatic Workbench patching
- Bypassing Foundry validation
- Treating NormGeneCandidate as trusted norm
- Treating StructuralCrystal as executable code
- Treating CollapsePlan as mutation authority
- Treating Metatron fingerprint equality as semantic equivalence
- Treating LPCM 51% majority as truth
- Enabling SyntheticSourceCube
- Network acquisition

## Policy Escalation Path

```
ReportOnly → DryRun → OperatorApproved → AutonomousBounded
```

Each transition requires:
1. An explicit `PolicyProfile` with the elevated `ImplementationMode`
2. Operator confirmation (for `OperatorApproved` and above)
3. Foundry validation for any executable effects
4. Parse-back topology check for topology-changing operations

## Hard Safety Rules (Invariants)

These rules hold across all phases and all modes:

1. No raw external source code enters default prompts, host patches, trusted memory, or ContextPack.
2. No acquired repository is executed by default.
3. No host file is modified during Phases 0–10.
4. CorpusCartography is not trusted memory.
5. NormGene is not a trusted norm.
6. StructuralCrystal is not executable code.
7. CollapsePlan is not mutation authority.
8. SurgeryOption is not a code patch.
9. LPCM "patch" means candidate direction, not file patch.
10. SystemCube materialization is dry-run or operator-confirmed by default.
11. SyntheticSourceCube is disabled by default.
12. Every durable object must be evidence-bound, content-addressed, policy-scoped, and replayable
    or explicitly marked replay-incomplete.

## Default `PolicyProfile` Values

```rust
PolicyProfile {
    mode: ImplementationMode::ReportOnly,
    allow_network: false,
    allow_external_acquisition: false,
    allow_acquired_repo_execution: false,
    allow_host_write: false,
    allow_context_injection_from_external: false,
    allow_synthetic_sourcecube: false,
    allow_metatron_surgery_planning: false,
    allow_lpcm_materialization: false,
    allow_systemcube_materialization: false,
    allow_memory_promotion: false,
    require_foundry_for_executable_effects: true,
    require_parseback_for_topology_changes: true,
    require_operator_approval_for_materialization: true,
}
```

## Phase 11 Gate

Phase 11 (OperatorApproved Materialization) is BLOCKED until:
- All Phases 0–10 exit criteria are met
- User explicitly authorizes Phase 11 in this conversation
- A `PolicyProfile` with `mode = OperatorApproved` is explicitly constructed
- Foundry validation infrastructure is confirmed operational
- Parse-back topology check infrastructure exists

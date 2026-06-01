# Changelog

All notable changes to PSE are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Pre-1.0 caveat: while every change tries to preserve the
content-addressing and replay contracts, the public Rust API surface
is still open to breaking changes between 0.x releases. Crystal IDs
and report bytes will only break across minor versions if a release
note explicitly says so.

## [Unreleased]

### Added

* **`CrystalRecordStore` — durable JSONL-backed CAD library persistence**

  Crystal records now survive across integration runs; the CAD library can be
  pre-loaded into `IntegrationRunOptions::prior_crystals` from the previous session.

  - `StructuralCrystalRecord::verify_id()` — recomputes the `record_id` from fields
    for integrity checking; used by the store on open and in `verify_integrity`.
  - `CrystalRecordStore::open(path)` — replays the JSONL file, verifying every
    `record_id`; returns `Err(IntegrityViolation)` on any tampered record.
  - `CrystalRecordStore::append(record, policy)` — same host-write invariant as
    `JsonlCartographyStore`: `ReportOnly` and `DryRun` are denied; only
    `OperatorApproved` (or a profile with `allow_host_write`) can persist. Dedup
    by `record_id` — re-appending an already-stored record is a silent no-op.
  - `CrystalRecordStore::records()` → `&[StructuralCrystalRecord]` for direct use
    as `IntegrationRunOptions::prior_crystals` without an extra copy.
  - `CrystalRecordStore::verify_integrity()` → re-verifies every record after reload.
  - `CrystalStoreError` — simple error enum with manual `Display`/`Error` impl
    (no thiserror dependency added to `kosmo-store`).
  - `kosmo-hyphae` added as a dependency of `kosmo-store`.
  - 7 new store tests (14 total); 1 new eval scenario (141 total, 886 substrate tests).

* **Crystal-boosted SourceCube scoring — `crystal_resonance` dimension**

  Closes the CAD library feedback loop: prior certified crystal records now influence
  the energy ranking of current-run SourceCubes via structural proximity.

  - Pipeline Step 2b: when `prior_crystals` is non-empty and source content is available,
    the best structural resonance between the current void's HDAG (rho/omega signals) and
    every prior crystal record is computed and stored as `crystal_resonance` dimension in
    the `CubeDimensionProfile`.
  - Uses the same rho/omega proximity formula as `Resonite::from_records`; pure Q16
    arithmetic, no floats (CROSS-007).
  - The dimension contributes to `ρ (coherence)` in the tripolar energy assessment —
    voids that match a known certified pattern rank higher in the void-fill plan.
  - `crystal_resonance` only appears when both HDAG and prior_crystals are present
    (no false-zero baseline: runs without prior crystals are unchanged).
  - 2 new pipeline tests (102 total); 1 new eval scenario (140 total, 872 substrate tests).

* **Crystal structural fingerprint + Resonite pipeline wiring (Step 5e-resonite)**

  Closes the loop between code structure and the CAD library: certified crystal records
  now carry structural provenance, and cross-run pattern proximity is computed via Resonite.

  - `StructuralCrystalCandidate`: new fields `source_void_id: Option<Digest>`,
    `rho_coherence: Q16`, `omega_phase: Q16`; both participate in `candidate_id`
    content-addressing so HDAG-enriched candidates differ from file-presence-only ones.
  - `StructuralCrystalCandidate::from_decision_with_signals(decision, void_id, rho, omega)` —
    builds a candidate with code-structure signals. `from_decision` now delegates to it
    with defaults `(None, ONE, ONE)`.
  - `StructuralCrystalRecord`: new fields `source_void_id`, `rho_coherence`, `omega_phase`
    propagated from the candidate at certification time; all three participate in `record_id`.
  - `StructuralCrystalRecord::from_certificate(cert, candidate)` — updated signature
    (second argument carries the structural provenance).
  - `Resonite::from_records(a, b, policy_id)` — structural proximity score:
    `((ONE - |ρ_a - ρ_b|) + (ONE - |ω_a - ω_b|)) / 2`; symmetric, Q16, no floats (CROSS-007).
  - Pipeline Step 5d: candidates built with HDAG signals via `from_decision_with_signals`
    (intent's `target_void_id` + `hdag_by_void_id` lookup).
  - Pipeline Step 5e-resonite: pairwise `Resonite` between every current certified crystal
    and every prior crystal; `resonite_count` participates in `report_id`.
  - `IntegrationRunReport.resonite_map: Vec<Resonite>` — covered by `verify_policy_consistency`.
  - 6 new `crystal.rs` tests (197 total); 6 new pipeline tests (100 total); 3 new eval scenarios
    (`rx-crystal-*` ×2, `rx-pipeline-resonite-*` ×1) → 139 total, 855 substrate tests.

* **CodeHDAG pipeline integration — code-structure-aware void severity and SourceCube dimensions**

  Topology observation deepened from file-presence to code-structure. When workspace entries
  carry source content (via `scan_path_with_content`), the pipeline now extracts `CodeHDAG`
  per source file and wires structural signals into the hyphae + pipeline layers.

  - `WorkspaceEntry.content: Option<String>` (`#[serde(skip)]`) — source text for HDAG
    extraction; excluded from `index_id` content-addressing (digest already addresses bytes).
  - `WorkspaceIndex::scan_path_with_content(root, policy_id)` — scans `.rs` source/test
    files and populates `content` for HDAG extraction.
  - `HostCube.hdag_by_void_id: BTreeMap<Digest, CodeHDAG>` — HDAG keyed by void_id;
    `hdag_count` participates in `cube_id` so enriched cubes differ from file-only cubes.
  - `MissingTestFiber` severity scales with HDAG definition count:
    `HALF + HALF × min(N, 8) / 8` (more definitions → higher urgency for test coverage).
  - Pipeline Step 2b: accepted-decision `SourceCube` dimensions now include
    `rho_coherence` and `omega_phase` from the CodeHDAG when content is available.
  - `IntegrationRunReport.source_cubes: Vec<SourceCube>` — SourceCubes are now exposed
    in the report for downstream inspection and testing.
  - `CubeDimensionProfile::from_raw_map(BTreeMap<String, Q16>)` — new constructor for
    raw-key dimension maps (used by the HDAG enrichment path).
  - 4 new `host.rs` tests + 4 `cube.rs` tests; 3 new pipeline tests; 2 new eval scenarios
    (`rx-hyphae-hdag-extracted-from-source-content`, `rx-hyphae-hdag-severity-scales-with-definition-count`).

* **Crystal certification pipeline — `StructuralCrystalRecord` + cross-run CAD library accumulation**

  - `ConstraintProgram::from_candidate(candidate, replay_status)` — evaluates the standard
    5-constraint program from candidate fields alone (no `EvidenceBundle` object required).
  - `StructuralCrystalCandidate::certify(replay_status)` — single call produces
    `(AssimilationCertificate, StructuralCrystalRecord)` for every `Pending` candidate.
  - `CorpusEntityKind::CrystalRecord` — certified crystal records are first-class corpus
    entities; the corpus now accumulates proven patterns across runs.
  - Pipeline Step 5d-cert: `certified_crystals: Vec<StructuralCrystalRecord>` in
    `IntegrationRunReport`; `certified_crystal_count` in `ReportContent`.
  - `IntegrationRunOptions.prior_crystals: Vec<StructuralCrystalRecord>` — seed the corpus
    with certified records from previous runs, closing the CAD library accumulation loop.
  - 4 new `crystal.rs` tests (186 total); 4 new pipeline tests (91 total); 2 new eval
    scenarios `RX:Crystal`/`RX:Pipeline` (134 total, 815 substrate tests).

* **`AssimilationLedger` — sequenced, content-addressed audit log of all decisions per run**
  (INVARIANT-007 strengthened: `run_id` is now sensitive to decision outcomes, not just
  decision count).

  - `AssimilationLedger { ledger_id, run_id, events, policy_id }` added to
    `kosmo-hyphae/assimilation`. Built via two-pass construction: a placeholder pass
    derives `ledger_id` from the ordered event sequence, then the final `run_id` is
    sealed with `ledger_id` in its content hash.
  - `HyphaeRunResult.ledger: AssimilationLedger` — every passive run now carries its
    full decision log.
  - `RunContent.ledger_id` participates in `run_id` content-addressing.
  - `ReportContent.hyphae_ledger_id` propagates the ledger commitment into the pipeline
    `report_id`.
  - 4 new hyphae tests (182 total); 1 new `RX:Hyphae` eval scenario (132 total).

* **Motif feedback loop + `SuggestPattern` yield kind** — closes the cross-run feedback
  loop so motifs observed in one pipeline run propagate as structural proposals
  into the next run's frontier.

  - `yield_for_intent` now selects `StructuralYieldKind::MotifProposal` for
    `SuggestPattern` intents (previously always `DeficiencyFill`).
  - `SourceFrontierGraph::augmented_with_prior_motifs` appends `SuggestPattern`
    intents for motifs meeting a configurable `min_support` threshold.
  - `passive_run_augmented(index, policy, additional_intents)` — backward-compatible
    wrapper; `passive_run` delegates to it with an empty slice.
  - `IntegrationRunOptions.prior_motifs: Vec<MotifCandidate>` and
    `prior_motif_min_support: Q16` — pipeline uses them to inject intents at the
    top of each run.
  - `MotifCandidate` → `PseBridgeCandidate::StructuralObservation` in Step 6b.
  - 2 new hyphae tests (178 total); 2 new pipeline tests (87 total); 3 new eval
    scenarios (131 total, 803 substrate tests).

* **Pipeline Step 5a: `MotifCandidate` from void kind frequency** — closes the gap
  between `MotifCandidate` (fully implemented with `energy_assessment`) and the
  pipeline (which had no step to generate or expose them).

  - `enable_motif_candidates: bool` in `IntegrationRunOptions` (default false;
    included in `all_layers()`).
  - Step 5a counts `HostVoidKind` occurrences, produces one `MotifCandidate` per
    kind with `support_score = kind_count / total_voids` (Q16 ratio, no floats,
    CROSS-007). Evidence = `hyphae.run_id` (CROSS-006: always non-ZERO).
    Results are energy-ranked before inclusion in the report.
  - `motif_candidate_count` participates in `report_id` (INVARIANT-007).
  - `verify_policy_consistency()` and `summary()` updated.
  - 4 new pipeline tests (85 total); 2 new `RX:Pipeline` eval scenarios (128 total,
    789 substrate tests).

* **`ReduceDeficiency` intents in frontier + spec §2.2 yield compliance** — closes
  the gap between the `DeficiencyVector` (already computed in Step 1c) and the
  `SourceFrontierGraph` (previously void-map-only), and ensures every yield
  produced from a `ReduceDeficiency` intent satisfies the spec §2.2 reference
  invariant (a yield must reference a void OR a deficiency).

  - `SourceFrontierGraph::from_void_map` now derives the `DeficiencyVector`
    internally and appends one `ReduceDeficiency` intent per deficiency kind.
    An empty void map still produces an empty frontier.
  - `SourceFrontierGraph::from_void_map_and_deficiencies` exposed for callers
    that already hold a pre-computed vector.
  - `yield_for_intent` extracts `deficiency_kind_ref` from `ReduceDeficiency`
    intents and passes it into `StructuralYield::new`; all other intent kinds
    continue to produce `deficiency_kind_ref = None`.
  - 4 new hyphae tests (176 total); 3 new `RX:Hyphae` eval scenarios (126 total,
    780 substrate tests).

* **`yield_for_intent` taint/authority propagation** — removes the last
  hardcoded trust override in the passive HYPHAE run path, opening the clean
  intent → Accepted decision path end-to-end.

  - `yield_for_intent` now calls `intent.taint.clone()` and
    `intent.authority.clone()` instead of hardcoding `TaintLabel::Synthetic` /
    `AuthorityLabel::Agent`. The `from_void_map` default remains
    `Unverified`/`Agent`, so all existing passive-run outcomes are unchanged.
  - A `TaintLabel::Clean` + `AuthorityLabel::Foundry` intent now naturally
    produces an `Accepted` decision under operator-approved policy — no special
    casing needed anywhere in the gate stack.
  - 2 new hyphae tests (172 total); 1 new `RX:Hyphae` eval scenario (124 total,
    777 substrate tests).

* **Decision taint propagation to BlueprintUnit** — closes the data flow gap
  between `StructuralYield.taint` and `BlueprintUnit`; every trust signal now
  travels through the full pipeline chain.

  - `AssimilationDecision.taint: TaintLabel` added, propagated from the source
    `StructuralYield` in `from_trace()`. The `taint` field participates in
    `decision_id` content-addressing so different taints produce distinct IDs
    (INVARIANT-007).
  - Pipeline Step 5e uses `decision.taint.clone()` instead of the hardcoded
    `TaintLabel::Synthetic`. All current passive-scan decisions remain Synthetic
    (same runtime behaviour), but a future `OperatorAssisted` run with Clean
    yields will automatically produce fully-compatible `Accepted` blueprint units.
  - 1 new hyphae test (170 total); 1 new pipeline test (81 total); 1 new
    `RX:Pipeline` eval scenario (123 total, 775 substrate tests).

* **SystemCube diagnostics surfaced in pipeline** — compatibility and contradiction
  energy are now first-class citizens of `IntegrationRunReport`, with accessors,
  gate contribution, and summary inclusion.

  - `IntegrationRunReport::systemcube_compatibility_score() -> Option<Q16>` and
    `systemcube_contradiction_energy() -> Option<Q16>` — direct accessors that avoid
    drilling through `Option<KcubeExportReport>`.
  - SystemCube gate contribution upgraded: `Warn` when `compatibility.gaps` is
    non-empty (structural advisory signal, not energy — respects CROSS-010); `Pass`
    when all accepted units are clean.
  - `summary()` now includes `compat=<score>` and `contradiction_energy=<total>` in
    the systemcube section.
  - 3 new pipeline tests (80 total); 2 new `RX:Pipeline` eval scenarios (122 total,
    772 substrate tests).

* **CompatibilityProfileReport real gap detection** — replaces the `perfect()` stub
  in `SystemCube::export_dry_run` with unit-aware gap analysis; every
  `KcubeExportReport` now carries real compatibility diagnostics.

  - `CompatibilityProfileReport::from_units(manifest_id, host_snapshot_id, policy, units)`:
    `AcceptedWithTaint` units produce a `TaintedUnit` gap (severity `Q16::HALF`);
    `source_ref == Digest::ZERO` produces a `MissingSourceRef` gap (severity `Q16::ONE`).
    `compatibility_score = Q16::ONE − avg_gap_severity`, clamped to `[0, ONE]`.
    Gaps sorted by `unit_id`; opaque-rejected units excluded (INVARIANT-007).
  - 5 new `kosmo-systemcube` tests (54 total); 2 new `RX:Compatibility` eval
    scenarios (120 total, 769 substrate tests).

* **ContradictionEnergyReport real pairwise detection** — replaces the `zero_energy`
  stub in `SystemCube::export_dry_run` with a deterministic, unit_id-ordered pairwise
  scan of accepted units; surfaces real `RoleConflict` and `Duplicate` signals.

  - `ContradictionEnergyReport::from_units(manifest_id, policy, units)`:
    same `source_ref` + same `kind` → `Duplicate` (weight `Q16::HALF`);
    same `source_ref` + different `kind` → `RoleConflict` (weight `Q16::ONE`).
    Units iterated in `unit_id` order for determinism (INVARIANT-007).
  - `SystemCube::export_dry_run` now calls `from_units` — every `KcubeExportReport`
    carries real contradiction diagnostics rather than a constant zero.
  - 5 new `kosmo-systemcube` tests (49 total); 2 new `RX:ContradictionEnergy` eval
    scenarios (118 total, 764 substrate tests).

* **BlueprintUnit energy assessment — Step 5e** — completes energy integration for
  `kosmo-systemcube`; every artifact type in the production chain now has
  `energy_assessment`, enabling deterministic priority ordering across all layers.

  - `BlueprintUnit::energy_assessment(gate)`: ψ = `Q16::ONE` for accepted units
    (Accepted / AcceptedWithTaint); `Q16::ZERO` for opaque-rejected. The taint factor
    separately reduces energy for tainted units (Synthetic → ½, Quarantined → 0).
    `evidence_bundle_id = self.unit_id` (self-referential, CROSS-006).
  - Pipeline Step 5e: blueprint units are energy-ranked before `SystemCube::new`,
    surfacing the most trusted units at the top of every manifest.
  - 3 new `kosmo-systemcube` tests (44 total); 2 new `RX:BlueprintEnergy` eval
    scenarios (116 total, 759 substrate tests).

* **PseBridgeCandidate pipeline integration — Step 6b** — surfaces all actionable
  pipeline observations as PSE-ready candidates, completing the observation→submission
  funnel without gating any decisions on PSE acceptance (CROSS-010).

  - `enable_pse_candidates: bool` in `IntegrationRunOptions` (default false). When
    enabled, norm candidates become `StructuralObservation` candidates (ψ=`fitness_score`,
    evidence=`evidence_bundle_id`) and ambiguity profiles + void hypotheses become
    `TopologyObservation` candidates (ψ=`confidence_score`). All are sorted by confidence
    descending, with `id` as deterministic tie-break.
  - `IntegrationRunReport.pse_candidates: Vec<PseBridgeCandidate>`; count participates
    in `report_id`. `verify_policy_consistency()` covers all candidate `policy_id`
    fields. `summary()` reports `pse_candidates: N`.
  - 3 new pipeline tests (77 total); 2 new `RX:Pipeline` eval scenarios (114 total,
    756 substrate tests).

* **DeficiencyVector pipeline integration — Step 1c** — always-on diagnostic summary
  of structural deficiencies derived from the host void map (test coverage gaps,
  documentation gaps). Never requires an option flag.

  - `IntegrationRunReport.deficiency_vector: DeficiencyVector` always present.
    `deficiency_vector_id` participates in `report_id`. `verify_policy_consistency()`
    covers `deficiency_vector.policy_id`. `summary()` reports `deficiency: N entries`.
  - 3 new pipeline tests (74 total); 2 new `RX:Pipeline` eval scenarios (112 total,
    753 substrate tests).

* **StructuralCrystalCandidate pipeline integration — Step 5d** — surfaces the
  explicit certification work queue: one candidate per accepted decision, all
  starting with `support_score = Q16::ZERO` (Pending certification status).

  - `enable_crystal_candidates: bool` in `IntegrationRunOptions` (default false).
    `IntegrationRunReport.crystal_candidates`; count participates in `report_id`.
    `verify_policy_consistency()` covers candidate `policy_id` fields. `summary()`
    reports `crystal_candidates: N`.
  - 3 new pipeline tests (71 total); 2 new `RX:Pipeline` eval scenarios (110 total,
    750 substrate tests).

* **TopologyAmbiguityProfile + ComplementVoidHypothesis pipeline integration — Step 3f** —
  surfaces previously discarded metatron M2 diagnostic details as energy-ranked
  top-level collections in the report.

  - Pipeline Step 3f: flatten `.ambiguities` and `.void_hypotheses` from all
    `metatron_diagnostics`, energy-rank each by `confidence_score` (most-confident first).
    Both collections are empty when `enable_metatron` is false.
  - `IntegrationRunReport.ambiguity_profiles` + `.complement_void_hypotheses`; counts
    participate in `report_id`. `verify_policy_consistency()` covers both. `summary()`
    reports `ambiguities: N | void_hyp: M`.
  - 3 new pipeline tests (68 total); 2 new `RX:Pipeline` eval scenarios (108 total,
    747 substrate tests).

* **NormFitnessTrace pipeline integration — Step 5c** — closes the full
  "Wissen zurück ins Substrat" loop: PSE promotion outcomes feed back into the
  substrate as fitness observations, which can re-rank norm gene candidates.

  - `IntegrationRunOptions.prior_feedback: Vec<PromotionFeedback>` (default empty).
    On each run, feedback records with matching `norm_candidate_id` are folded into
    `NormFitnessTrace::observe_from_feedback`. Only traces with ≥1 observation
    are included in the report.
  - `IntegrationRunReport.norm_fitness_traces: Vec<NormFitnessTrace>`;
    `norm_fitness_trace_count` participates in `report_id`. `verify_policy_consistency()`
    covers all trace `policy_id` fields. `summary()` reports `norm_candidates: N (traces: M)`.
  - 3 new pipeline tests (65 total); 2 new `RX:Pipeline` eval scenarios (106 total,
    744 substrate tests).

* **SurgeryWorkbenchTask pipeline integration — Step 3e** — every energy-ranked
  `TopologicalSurgeryOption` now converts into a workbench-compatible task immediately
  after Step 3b, closing the surgery → workbench gap.

  - Pipeline Step 3e: `surgery_options.iter().map(SurgeryWorkbenchTask::from_option).collect()`;
    tasks are in the same energy-ranked order as the source options. Empty when
    `surgery_options` is empty (i.e., `enable_surgery` or `enable_metatron` is false).
  - `IntegrationRunReport.surgery_workbench_tasks: Vec<SurgeryWorkbenchTask>`;
    `surgery_workbench_task_count` participates in `report_id`. `verify_policy_consistency()`
    covers all task `policy_id` fields. `summary()` reports `surgery: N (tasks: M)`.
  - 3 new pipeline tests (62 total); 2 new `RX:Pipeline` eval scenarios (104 total,
    741 substrate tests).

* **MicroTopologyIndex pipeline integration — Step 3d** — closes the last metatron
  integration gap; `MicroTopologyIndex` existed in the spec but was never assembled.

  - Pipeline Step 3d: after the metatron loop, all `(MetatronMicrograph,
    MetatronRegionFingerprint, MicroTopologyDiagnostic)` triples are folded into a
    `MicroTopologyIndex` via `MicroTopologyIndex::add`. Produces an empty-state index
    when `enable_metatron` is false.
  - `IntegrationRunReport.metatron_index: MicroTopologyIndex`; `index_id` participates
    in `report_id` (content-addressed). `verify_policy_consistency()` covers
    `metatron_index.policy_id`. `summary()` reports `index_id` prefix.
  - 4 new pipeline tests (59 total); 2 new `RX:Pipeline` eval scenarios (102 total,
    738 substrate tests).

* **TopologyAmbiguityProfile + ComplementVoidHypothesis energy_assessment** —
  completes energy integration for all Q16-score types in kosmo-hyphae.
  Every substrate type that carries a Q16 score now has `energy_assessment`.

  - `TopologyAmbiguityProfile::energy_assessment(gate)`: ψ = `confidence_score`;
    `evidence_bundle_id = micrograph_id` (the source micrograph, CROSS-006).
  - `ComplementVoidHypothesis::energy_assessment(gate)`: ψ = `confidence_score`;
    `evidence_bundle_id` = first non-ZERO entry in `evidence_ids`, falling back to
    `micrograph_id` (CROSS-006: always non-ZERO). Both forms allow `rank_by_energy`
    over a diagnostic's sub-items.
  - 4 new `metatron.rs` tests (169 hyphae tests total, 734 substrate tests).

* **SemanticLossRecord + MicrographLiftReport energy integration + pipeline Step 3c** —
  closes the last energy_assessment gap in kosmo-hyphae; lift quality signal now
  surfaces in every metatron-enabled pipeline run.

  - `SemanticLossRecord::energy_assessment(gate)`: ψ = `loss_ratio` (high loss =
    high energy = most urgent to review); `evidence_bundle_id = region_id` (CROSS-006).
  - `MicrographLiftReport::energy_assessment(gate)`: ψ = `loss_ratio`;
    `evidence_bundle_id = micrograph_id` (CROSS-006).
  - Pipeline Step 3c: the M1 lift report (`MicrographLiftReport`) is no longer
    discarded. When `enable_metatron` is true, one report per void is collected,
    energy-ranked by `loss_ratio` (most lossy lifts first), and stored in
    `IntegrationRunReport.lift_reports`. `ReportContent.lift_report_count`
    participates in `report_id`.
  - `summary()` now reports `metatron: N (lift_reports: M)`.
  - 4 new `metatron.rs` tests + 3 new pipeline tests + 2 new `RX:Pipeline`
    eval scenarios (100 total, 730 substrate tests).

* **Resonite, CubeMandorla, CompositeSupportCube energy_assessment** —
  completes energy integration for all swarm and crystal structural types.

  - `Resonite::energy_assessment(gate)`: ψ = `resonance_score`; symmetric
    (r(a,b) produces the same assessment as r(b,a)); `evidence_bundle_id =
    resonite_id` (self-referential, CROSS-006).
  - `CubeMandorla::energy_assessment(gate)`: ψ = `overlap_score`;
    `evidence_bundle_id = mandorla_id` (self-referential, CROSS-006).
  - `CompositeSupportCube::energy_assessment(gate)`: ψ = `aggregate_support`;
    `evidence_bundle_id = composite_id` (self-referential, CROSS-006).
  - No new fields on any type — the type's own content address satisfies CROSS-006.
  - 3 new `crystal.rs` tests + 4 new `swarm.rs` tests.

* **NormGeneCandidate pipeline integration — Step 5b** — closes the last
  hyphae-to-pipeline integration gap; norm gene candidates are now generated
  and ranked as part of every full pipeline run.

  - `IntegrationRunOptions.enable_norm_candidates: bool` (default false).
  - Pipeline Step 5b: for each accepted assimilation decision, a
    `NormGeneCandidate` is created with `fitness_score = Q16::ONE` (initial
    fitness; `NormFitnessTrace` evolves this via feedback in later phases).
    `evidence_bundle_id = decision.evidence_bundle_id` (CROSS-006: non-ZERO
    causal ref — traces back to the original evidence that justified acceptance).
    All candidates are energy-ranked via `rank_by_energy` before being stored.
  - `IntegrationRunReport.norm_candidates: Vec<NormGeneCandidate>`; count
    participates in `report_id` (content-addressed).
  - `verify_policy_consistency()` extended to cover `norm_candidates[i].policy_id`.
  - `summary()` reports `norm_candidates: N`.
  - 3 new pipeline unit tests (52 total); 2 new `RX:Pipeline` eval scenarios
    (98 total, 712 substrate tests).

* **Void priority ranking — pipeline Step 1b** — every `IntegrationRunReport`
  now ships a severity-ordered void repair queue at zero extra I/O cost.

  - `HostVoid::energy_assessment(gate, policy_id)`: ψ = `severity`; taint/phase
    fixed at `Q16::ONE` (void detection has no coherence dimension at this level);
    `evidence_bundle_id = void_id` — the void's own content address satisfies
    CROSS-006 (non-ZERO evidence ref).
  - `TopologicalVoidMap::priority_ranking(gate) -> Vec<Digest>`: ranks all voids
    by energy D via `rank_by_energy`; ties broken deterministically by `void_id`.
  - Pipeline Step 1b: `void_priority_ranking` is always computed after the HYPHAE
    passive run and stored in `IntegrationRunReport`. `ReportContent` carries
    `void_priority_count` so the void count participates in `report_id`.
  - `summary()` now reports `voids: N (priority ranked)`.
  - 5 new `void_map.rs` unit tests; 2 new `RX:Pipeline` eval scenarios (96 total,
    709 substrate tests).

* **Surgery energy assessment + pipeline Step 3b** — closes the surgical
  intervention planning chain from Metatron diagnostics.

  - `TopologicalSurgeryOption::energy_assessment(gate)`: ψ = `confidence_score`,
    `evidence_bundle_id = diagnostic_id` (CROSS-006 non-ZERO causal ref).
  - Pipeline Step 3b derives surgery options from all Metatron diagnostics,
    energy-ranks them via `rank_by_energy`, and stores the ranked slice in
    `IntegrationRunReport.surgery_options`. Gated by `enable_surgery: bool`
    (default false); requires `enable_metatron` to produce any output.
  - `verify_policy_consistency()` now covers `surgery_options[i].policy_id`.
  - 4 surgery unit tests, 3 new `RX:Pipeline` eval scenarios (94 total,
    704 substrate tests).

* **`from_host_and_composite` removed; `MorphogenicCorpusUpdate` as Step 4d** —

  - `HostTargetDelta::from_host_and_composite` deleted (only callers were its
    own tests; used raw `max_by_key` violating the energy invariant). Its two
    tests migrated to `from_source_cubes` with real `SourceCube` objects.
  - Pipeline Step 4d: `MorphogenicCorpusUpdate::skeleton(cartography_update_id,
    collapse_plan_id, policy_id)` — planning skeleton of the post-collapse corpus.
    Participates in `report_id`, `verify_policy_consistency()`, and `summary()`.
  - 2 new `RX:Pipeline` eval scenarios.

* **JsonlCartographyStore persistence wired into pipeline** — closes the
  last persistence gap; `CorpusCartographyUpdate` can now be durably stored.

  - `CartographyEntryKind::CartographyUpdate` added to `kosmo-core`.
  - `kosmo-pipeline` gains `kosmo-store` dep and a new `persistence` module.
  - `persist_cartography_update(update, path, scope, policy)`: fail-closed on
    `allow_host_write == false`; CROSS-006 satisfied (evidence = `update_id`);
    commit labels `after_cartography_id` + `added_entity_count`.
  - 3 unit tests, 2 new `RX:Pipeline` eval scenarios (89 total).

* **StructuralCrystalCandidate gains `energy_assessment`** — last hyphae
  candidate type to receive energy integration.

  - ψ = `support_score` (ZERO at creation; gate factor collapses to zero if
    the gate rejects). Taint = `Q16::ONE`: quarantined yields are rejected
    at the gate cascade before candidacy (`IsNotQuarantined` constraint).
  - 3 new `crystal.rs` unit tests, 2 new `RX:EnergyRanking` eval scenarios.

* **Phase 4c: HostTargetCollapsePlan wired into run_dry_pipeline** —
  planning-only collapse plan now ships with every `IntegrationRunReport`.

  - `run_dry_pipeline` Step 4c: `HostTargetCollapsePlan::from_delta(&void_fill_delta, policy.id)`.
    Status is always `PlanningOnly` — no execution authority in Phase 5.
  - `IntegrationRunReport` gains `collapse_plan: HostTargetCollapsePlan`.
  - `ReportContent` gains `collapse_plan_id`; the collapse plan participates
    in the report's content address — any plan change alters `report_id`.
  - `verify_policy_consistency()` now asserts `collapse_plan.policy_id == policy.id`.
  - `summary()` reports `collapse: N steps (PlanningOnly)`.
  - 3 new `RX:Pipeline` eval scenarios; total 85 scenarios, 682 substrate tests.

* **MotifCandidate policy alignment + SeamGraph seam coherence wired into ranking** —
  two architectural gaps closed in one weld.

  - `MotifCandidate` gains `policy_id: Digest` (aligns with all other substrate types);
    content addressing (`motif_id`) now includes `policy_id`. `new()` signature updated;
    `energy_assessment(gate)` added: ψ=`support_score`, taint factor from `self.taint`.
    5 tests (3 updated, 2 new).
  - `SourceCube::energy_assessment` gains a `seam_coherence: Q16` parameter; the
    `EnergyFactors::seam` field is no longer hardcoded to `Q16::ONE`.
  - `HostTargetDelta::from_source_cubes` gains `seam_map: &BTreeMap<Digest, Q16>`
    (void_id → seam coherence). Each void's seam coherence multiplies its candidates'
    energy; missing entries default to `Q16::ONE`. A cube with `support=1` but
    `seam=0` collapses to zero energy.
  - Pipeline Step 4b (CubeSwarm) moved after LPCM so LPCM seam data feeds the
    void-fill ranking. `seam_map` built from `lpcm_reports`: coherence = fraction
    of compatible seam edges per void (empty graph → `Q16::ONE`).
  - **`tools/kosmo-eval` extended to 82 scenarios** (was 80): 2 new `RX:EnergyRanking`
    scenarios (`rx-energy-motif-assessment-content-addressed`,
    `rx-energy-seam-penalty-reduces-ranking`).

* **Phase 4 CubeSwarm + HostTargetDelta wired into the pipeline** — closes the
  integration gap where `CubeSwarm` and `HostTargetDelta` existed but were never
  called from `run_dry_pipeline`.

  Step 2b in `run_dry_pipeline`: accepted assimilation decisions are converted
  to `SourceCube`s (ψ=1, taint from intent), assembled into a `CubeSwarm`,
  and ranked via `HostTargetDelta::from_source_cubes` (energy-correct path).
  `IntegrationRunReport` now carries `swarm_composite: CompositeSupportCube`
  and `void_fill_delta: HostTargetDelta` — both content-addressed and
  policy-tagged. `verify_policy_consistency()` covers the new fields.

  - **`tools/kosmo-eval` extended to 80 scenarios** (was 76): 4 new
    `RX:Pipeline` scenarios (swarm+delta in report, empty-workspace delta is
    Clean, policy consistency includes swarm, deterministic across runs).

* **Energy kernel adoption in selection paths** — closes the gap where
  `SourceCube` and `NormGeneCandidate` ranked by raw Q16 scores instead of
  the unified tripolar energy kernel (as called out in the `kosmo-core::energy`
  module-level doc).

  - `SourceCube::energy_assessment(gate, license, foundry)` — ψ=`support_score`,
    ρ=average dimension-profile coverage (coherence), ω=1; taint factor from
    `self.taint`. Returns a content-addressed [`EnergyAssessment`].
  - `NormGeneCandidate::energy_assessment(gate)` — ψ=`fitness_score`, ρ=ω=1;
    gate-collapsed fail-closed (CROSS-010 analogue). Returns an `EnergyAssessment`.
  - `HostTargetDelta::from_source_cubes` — the energy-correct companion to
    `from_host_and_composite`. Groups `SourceCube`s by `target_void_id`, calls
    `energy_assessment` on each, then uses `rank_by_energy` to pick the top
    candidate per void. A quarantined cube with `support_score=1.0` loses to a
    clean cube with `support_score=0.5` — the kernel overrides raw Q16.
  - **`tools/kosmo-eval` extended to 76 scenarios** (was 72): 4 new
    `RX:EnergyRanking` scenarios (quarantine zeroes energy, ranking picks best,
    taint beats higher raw score, norm candidate content-addressed assessment).

* **PSE feedback loop — "Wissen zurück ins Substrat"** — closes the final
  vision link by routing `PromotionOutcome` back into substrate fitness tracking.

  - `FeedbackOutcome` (Accepted/Rejected/Deferred/Skipped) — substrate-side
    mirror of PSE's `PromotionOutcome`, in `kosmo-core` to avoid circular
    dependency. `fitness_signal(energy)` maps: Accepted→energy, Deferred→¼,
    Rejected/Skipped→0 (CROSS-010 analogue).
  - `PromotionFeedback` — content-addressed record in `kosmo-core` binding a
    `PromotionRequestRecord` outcome, candidate confidence, derived
    `fitness_signal`, policy, and `evidence_bundle_id` (CROSS-006). 14 unit tests.
  - `CartographyEntryKind::PromotionFeedback` — new variant allowing feedback
    records to be stored in `CorpusCartographyStore`.
  - `build_promotion_feedback` in `kosmo-pse-bridge` — converts
    `PromotionOutcome` → `FeedbackOutcome` and constructs a `PromotionFeedback`
    from a `PromotionRequestRecord` + `PseBridgeCandidate`.
  - `NormFitnessTrace::observe_from_feedback` in `kosmo-hyphae` — consumes a
    `PromotionFeedback` to append a fitness observation; uses `feedback.id` as
    the evidence reference, closing the loop end-to-end. 3 new tests.
  - **`tools/kosmo-eval` extended to 72 scenarios** (was 68): 4 new
    `RX:FeedbackLoop` scenarios (accepted maps to full energy, rejected gives
    zero fitness, stored in cartography as `CartographyStoreCommit`, full
    chain `build_promotion_feedback` + `observe_from_feedback`).

* **`SystemCube::export_to_kcube` weld** — closes the "Blueprint raus" vision
  link by connecting the dry-run `KcubeExportReport` to the real
  `KcubeExecutor`. The method runs `export_dry_run` first; if
  `op_policy.allow_systemcube_materialization = false` it returns
  `SkippedByReportOnly` without touching the filesystem; otherwise it
  serializes the manifest, export assessment, and all accepted blueprint units
  into a `.kcube` archive via `KcubeExecutor::write`.

  `to_kcube_artifacts` produces three artifact kinds:
  `CartographyManifest` (`manifest.json`), `ValidationClosureReport`
  (`export_report.json`), and `StructuralCrystal` (one file per accepted
  `BlueprintUnit` keyed by `unit_id` hex). 5 new unit tests in
  `kosmo-systemcube`.

* **`PolicyProfile::operator_approved_with_systemcube`** — new constructor in
  `kosmo-core` that sets `allow_systemcube_materialization = true` alongside
  the existing operator-approved gates (host write allowed, no network, Foundry
  + ParseBack still required).

* **`tools/kosmo-eval` extended to 68 scenarios** (was 65): 3 new
  `RX:SystemCubeKcube` scenarios (blocked by default policy, write creates
  archive, archive parses back with correct entry count). `kosmo-eval` now
  depends on `kosmo-systemcube`.

* **Unified tripolar energy kernel** (`kosmo-core::energy`) — the single,
  float-free, content-addressed selection core `D = ψ · ρ · ω`.

  - `TripolarEnergy { psi, rho, omega }` — the three poles (meaning / coherence
    / phase), each clamped to `[0, 1]`; `d()` computes `ψ·ρ·ω` in `Q16` integer
    arithmetic (CROSS-007: no floats).
  - `EnergyFactors` — six `[0, 1]` modulators (`gate`, `taint`, `license`,
    `foundry`, `seam`, `contradiction`) derived fail-closed from the substrate's
    own `GateResult` / `TaintLabel` / `LicenseStatus` / `FoundrySurvival`. Each
    factor can only *reduce* energy; a single zero collapses it.
  - `EnergyKernel` — tripolar core × factor product → final selection energy.
  - `EnergyAssessment` — content-addressed, evidence-bound, `verify_id()`.
  - `rank_by_energy` — deterministic descending ranking, `subject_id` tie-break,
    never silently drops a zero-energy candidate.
  - **Non-bypass invariant (CROSS-010):** energy ranks but never gates. A
    `Reject` zeroes the `gate` factor, so a rejected candidate can never
    out-rank a passing one and a high `D` can never flip a `Reject` into an
    `Accept`. 20 unit tests.

* **Real code topology extraction** (`kosmo-hyphae::code_hdag`) — replaced the
  one-node `CodeHDAG` skeleton with `extract_from_rust_source`, a dependency-free
  lexical extractor that emits real module/import/fn/type/test nodes and
  `Imports`/`Contains`/`Tests`/`Implements` edges. Content-addressed to the
  source line; deterministic (INVARIANT-007). Bridges into the energy kernel via
  `rho_coherence()`, `omega_phase()`, `energy_kernel()`, and `energy_assessment()`
  (ψ is a caller input; ρ and ω are derived from graph structure). New `Contains`
  `HDAGEdgeKind`; `CodeHDAG` content-address now covers full edge wiring. 12 new
  unit tests.

* **Real `.kcube` archive executor** (`kosmo-kcube`) — the host-capability
  bridge that turns `KcubeExportPolicy`-gated artifact lists into real
  `.kcube` files on disk and reads them back.

  Archive format: deterministic framed binary (`KCUBEPM\n` magic, LE-encoded
  sections, artifact bytes sorted by path for bit-exact reproducibility).
  `package_digest = SHA-256(artifact_section)` — the manifest JSON is appended
  as a trailer so the digest covers only the artifacts.

  Policy enforcement: `allow_write=false` → `DeniedByPolicy` (no disk touch);
  artifact kind allowlist checked before any write; `allow_overwrite=false`
  blocks silent replacement; `require_roundtrip_verification=true` (the
  default) re-reads the file and compares artifact-section SHA-256 after write.

  `KcubeExecutor::read` — parses a `.kcube` file back to `KcubePackage` for
  import/verify workflows (`parse_kcube_file` is also public).

  CROSS-006: `evidence_bundle_id ≠ ZERO` is propagated into every report
  variant including `DeniedByPolicy`. CROSS-007: no floats (`written_bytes`,
  `elapsed_ms` are `u64`). INVARIANT-007: `KcubeWriteReport.verify_id()` and
  `KcubePackage.verify_id()` both pass after roundtrip read.

  25 unit tests. No new external dependencies (reuses `kosmo-core`,
  `serde`/`serde_json`).

* **`tools/kosmo-eval` extended to 65 scenarios** (was 60): 5 new `RX:Kcube`
  scenarios (write denied when `allow_write=false`, write+roundtrip pass, content-
  addressed package, overwrite guard, `read` parses manifest). `kosmo-eval` now
  depends on `kosmo-kcube`.

* **`tools/kosmo-eval` extended to 60 scenarios** (was 52): 5 new `RX:Energy`
  scenarios (tripolar exactness, gate non-bypass, quarantine/proprietary/foundry
  zeroing, content-addressing, deterministic ranking) and 3 new `RX:Topology`
  scenarios (real-graph extraction, deterministic extraction, the full
  topology→energy chain). `kosmo-eval` now depends on `kosmo-hyphae`.

* Architecture decisions **AD-015** (tripolar energy kernel + non-bypass
  invariant) and **AD-016** (lexical topology extraction + topology→energy
  bridge).

* **KOSMO-OPS-01 Operationalization Staircase** — R0–RX full implementation
  of the empirical validation benchmark for KOSMO-OPS-01 invariants R1–R9.

  Four new host-capability crates:

  - `crates/kosmo-foundry` — Real Foundry executor. Runs allowlisted `cargo`
    subcommands (check / test / clippy) via `std::process::Command`. Policy
    contract: `ReportOnly` → `SkippedByReportOnly` (zero spawn); command
    denied before spawn; `FoundryExecutionReport` content-addressed.
    8 unit tests.

  - `crates/kosmo-store` — Persistent JSONL CorpusCartography store.
    Implements `CorpusCartographyStore` trait from `kosmo-core`. Append-only
    durable backend with `verify_integrity()` (digest mismatch + sequence gap
    detection). Emergent invariant: `DryRun` (`allow_host_write=false`)
    cannot persist — only `OperatorApproved`. 9 unit tests.

  - `crates/kosmo-parseback` — Real ParseBack executor. Snapshots workspace
    crate topology via `cargo metadata --format-version 1 --no-deps`.
    `TopologySnapshot` and `CrateFingerprint` are content-addressed (SHA-256);
    INVARIANT-007: identical inputs → identical IDs. `diff_snapshots()`
    classifies: `NodeRemoved`/`EdgeRemoved` → Critical, `NodeAdded`/
    `EdgeAdded` → Warning, `NodeModified` → Info (fail-closed worst-wins).
    17 unit tests including real workspace integration.

  - `crates/kosmo-operator` — Operator orchestrator. Wires the R1→R2→R3
    full pipeline: ParseBack pre-snapshot → Foundry execution → ParseBack
    post-snapshot + diff → `ValidationClosureReport` synthesis → optional
    JSONL store persistence (only `OperatorApproved` + `allow_host_write`).
    `OperationPlan` and `OperationReport` are content-addressed.
    8 unit tests including real targeted `cargo check` + temp store round-trip.

  `tools/kosmo-eval` benchmark extended to 52 scenarios:
  - 42 existing R1–R9 data-model scenarios (unchanged)
  - 6 new RX:ParseBackExec scenarios (report-only skip, baseline mismatch,
    severity classification, deterministic snapshot, real workspace pass)
  - 4 new RX:Operator scenarios (report-only inconclusive, content-addressed
    report, full-cycle dry-run, approved-persists-closure)

  All 52 scenarios pass `EXIT 0`. Workspace: 614 tests, 0 failures.

  Architecture decisions added: AD-010 (host-capability crate isolation),
  AD-011 (`allow_host_write` extended to disk persistence), AD-012
  (`cargo metadata` strategy for ParseBack), AD-013 (fail-closed severity
  mapping), AD-014 (OperationReport content-addressed over sub-IDs).

### Fixed

* **`-D warnings` build of `kosmo-core` and `kosmo-operator`** — removed two
  pre-existing unused-import warnings (`EvidenceBundle`/`EvidenceRef` at module
  scope in `cartography.rs`; `FoundryCommandPolicy`/`FoundryEnvironmentPolicy`
  at module scope in `kosmo-operator`, now scoped to the tests that use them)
  that broke `RUSTFLAGS="-D warnings"` builds of those crates.

* **Two stale test assertions in `pse-eval-matrix`** (`agent_exoskeleton.rs`):
  `ablation_aggregate_base_metrics_present` expected the robustness label
  `"requires_real_agent_validation"` but the production code now emits
  `"live_agent_validation_completed"` (set when live proof was completed);
  `trace_feature_design_report_is_present_and_consistent` asserted
  `!migration_plan.productive_agent_validated` but the field is intentionally
  `true` since the live Cerebras proof (commit `48fed88`). Both assertions now
  reflect the actual system state. Workspace test suite is **1315 / 1315**.

* **CHANGELOG `RulePredicate` variant names** corrected: entries `MaxUncertainty`,
  `RequireAttribution`, and `NotHallucinationAttractor` never existed in the
  implementation. Replaced with the actual variant set: `MinStability`,
  `MinKuramoto`, `MaxFreeEnergy`, `MinEvidenceEntries`, `CoherenceGate`,
  `PathInvariant`, `RequiresAgentAttribution`.

* **Clippy warnings** eliminated across `pse-adapter-il`, `mef-core`, `pse-server`,
  and `pse-llm-demo`: replaced indexed loop with iterator in `hdag.rs`,
  `sort_by` → `sort_by_key` in cluster sort, `field_reassign_with_default` in
  `entry_to_proxy_crystal()`, `filter_map` → `map` in the server IL retrieve
  handler (always-`Some` branches collapsed), doc overindentation in
  `il_bridge.rs`.

### Added

* **Pfad B — Constitutional Interceptor** (`crates/pse-constitutional-interceptor`,
  11 unit tests) — action-level governance gate layered over the PSE server.

  New crate `pse-constitutional-interceptor`:
  - `ActionContext { verb, target, description, metadata }` — describes any proposed
    system action (request, write, delete, execute, …).
  - `Decision::Allow | Block { rule_id, reason } | Warn { rule_id, reason }` — the
    three constitutional outcomes.
  - `ConstitutionalEvaluator` — evaluates an `ActionContext` against loaded `RuleAtom`s.
    Two-pass evaluation: Pass 1 scans all triggered rules for Blocking (blocking always
    wins, regardless of rule list order); Pass 2 scans for Required (strict mode →
    Block; non-strict → Warn). Trigger matching is case-insensitive substring over
    `"verb target description"`.
  - `EvaluationReport` — per-rule trigger / decision audit trail returned with every
    evaluation.
  - **Strict mode** auto-activates when the nxalien `EpistemicSignal` is Drifting or
    Diverging — Required rules escalate from Warn to Block automatically.

  Tower middleware (`tools/pse-server/src/constitutional.rs`):
  - `ConstitutionalLayer` / `ConstitutionalService<S>` — wraps the entire app router.
    Requests without `x-nxalien-*` headers pass through transparently.
    Block decision → 403 JSON response; Warn decision → `x-nxalien-warn` header added
    to the upstream response without interrupting the handler.
  - `POST /constitutional/check` — evaluate an `ActionContext` against the server's
    loaded rules. Accepts `{ action, strict_mode? }`, returns
    `{ report, active_rule_count, server_strict_mode }`. Returns 403 on Block, 200
    on Allow or Warn (with warn details inline). Strict mode defaults to the server's
    current epistemic signal state (Drifting/Diverging → strict).
  - The `nxalien_bundle` handler refreshes the evaluator's loaded rule set and updates
    the strict mode flag after every evolution cycle.

* **Pfad C — Multi-Repo Attractor** (`tools/nxalien-cli`) — `nxalien compile`
  extended with `--remote <url>` and `--remote-only` flags.

  - `--remote <url>`: after building the local bundle, POST it to a running PSE server
    at `<url>/nxalien/bundle` via `reqwest::blocking`. Prints the remote server's
    `EpistemicSignal`, IL health (`MemoryHealthReport`), and QTIC statistics from the
    JSON response.
  - `--remote-only`: after the remote POST, skip IL commit and `GraphState` update on
    the local filesystem. Useful for repos that treat the shared PSE server as the
    single central attractor.
  - `RemoteBundleResponse / RemoteSignal / RemoteILHealth` — typed response structs
    (`Deserialize`) for parsing the server JSON reply.
  - Enables multi-repo governance: any repository can contribute governance rules to a
    shared PSE attractor over HTTP, without filesystem access to the central IL store.
  - `reqwest = { workspace = true }` and `pse-exploratory` added to
    `tools/nxalien-cli/Cargo.toml`.

* **Pfad D — Exploratory Ledger** (`crates/pse-exploratory`, 16 unit tests) —
  negative-ψ hypothesis tracking for the nxalien governance pipeline.

  New crate `pse-exploratory`:
  - `EntryStatus::Pending | Landed { grounded_at_run, grounded_psi } |
    Decayed { decayed_at_run }`.
  - `ExploratoryEntry { rule_id, initial_psi, initial_qtic, block_hash_prefix,
    added_at_run, decay_after_runs, status }`.
  - Constants: `EXPLORATORY_PSI_THRESHOLD: f64 = 0.0`,
    `DEFAULT_DECAY_AFTER_RUNS: u64 = 10`.
  - `ExploratoryLedger` — file-backed at `<nxalien_dir>/exploratory.json`:
    * `ingest(rule_id, psi, qtic, block_hash_prefix, run)` — idempotent; only parks
      entries with ψ < 0. Already-Pending entries for the same rule_id are no-ops.
    * `check_landings(grounded, current_run)` — same rule_id reappears with new ψ ≥ 0
      → `Pending → Landed` transition.
    * `tick_decay(current_run)` — Pending entries older than `decay_after_runs` runs
      → `Pending → Decayed` transition.
    * `to_unknown_slots()` — Pending → `Unknown` slot (confidence =
      `(1+ψ).clamp(0, 0.99)`); Decayed → `Stale` slot. Used to surface hypotheses in
      the `[NXALIEN-CONTEXT]` block.
  - `ExploratoryLedgerSummary { pending_count, landed_count, decayed_count, mean_psi }`.

  Integrated into the nxalien pipeline:
  - `nxalien compile`: rules with ψ < 0 marked with `◈` in IL crystal output; these
    are ingested into the ledger; landing/decay checked each run; `exploratory_summary.json`
    written to `.nxalien/`.
  - `nxalien_bundle` handler: updates the exploratory ledger after every bundle
    (check_landings → tick_decay → ingest new negative-ψ entries → save).
  - `GET /exploratory/status` server route: `{ active, summary, pending_unknowns }`.

* **Pfad E — Epistemic Thunderbolt Vector** (`crates/pse-reasoning`, 10 unit tests) —
  D=ψ·ρ·ω guided multi-hop reasoning over the IL knowledge graph.

  New crate `pse-reasoning`:
  - `ThunderboltConfig { max_steps: 6, min_d_threshold: 0.01, top_k_per_step: 32 }`.
  - `ReasoningStep { step_index, crystal_id_hex, d_score, cumulative_d, qtic_class,
    stability_score, is_exploratory }`. `is_exploratory = qtic_class ≤ 1`.
  - `TerminationReason::MaxSteps | MinThreshold | NoNewMatches | EmptyStore`.
  - `ReasoningChain { query, steps, total_d, mean_d, terminated_by,
    has_exploratory_steps }` + helper methods `peak_d()`, `mean_qtic()`, `is_empty()`.
  - `guide(query, store, config) -> ReasoningChain` — Epistemic Thunderbolt Vector:
    1. `text_to_vector8(query)` → initial 8D semantic vector.
    2. `score_tripolar(vec)` → all IL crystals ranked by D = ψ·ρ·ω.
    3. Select highest-D unvisited crystal (loop prevention via `HashSet<String>`).
    4. `crystal_meta(id)` → `(qtic_class, stability_score)` for step annotation.
    5. Advance: `crystal_vector8(id)` → next query vector.
    6. Repeat until `MaxSteps | MinThreshold | NoNewMatches | EmptyStore`.

  Two new methods added to `ILStore` in `adapters/pse-adapter-il`:
  `crystal_vector8(crystal_id_hex) -> Option<Vec<f64>>` and
  `crystal_meta(crystal_id_hex) -> Option<(u8, f64)>`.

  Server route `POST /reasoning/guide` accepts `{ query, max_steps?, min_d_threshold? }`
  and returns the full `ReasoningChain`. Returns `{ active: false }` when the IL store
  is not loaded. Live example: 4-hop chain on a 5-crystal store, `total_d = 0.835`,
  terminated by `MaxSteps`.

* **nxalien — agent-context exoskeleton** (`crates/pse-nxalien-*`, `tools/nxalien-cli`) —
  six new crates + CLI implementing the nxalien governance layer as a fully interwoven
  PSE subsystem. nxalien is not a standalone product; every subsystem is wired to the
  PSE corpus. 26 unit tests + 2 integration tests (all green).

  **Six crates:**

  - `pse-nxalien-types`: canonical governance types — `RuleAtom` (SHA-256 / JCS
    content-addressed, evidence-sorted for hash stability), `UnknownSlot`,
    `Severity` (Advisory / Required / Blocking), `GateOutcome` (Accept / Hold /
    EvidenceOnly / Reject), `NxAlienBundle`, `NxAlienManifest`, `AgentContextCube`,
    `C8Coord`. RuleAtom hashes use `pse_types::content_address` — same substrate
    as `SemanticCrystal` IDs.

  - `pse-nxalien-core`: `canon` (wraps PSE content-addressing), `gate` (8-gate
    conjunctive evaluation: G_evidence / G_scope / G_replay / G_canon / G_delta /
    G_budget / G_governance / G_bridge), `scanner` (project auto-detection:
    Rust / TypeScript / Python with tool-chain recognition).

  - `pse-nxalien-cube`: `HypercubeHdag` — C⁸ directed acyclic graph (8 axes:
    ψ evidence_potential, ρ rule_density, ω temporal_phase, χ connectivity,
    η causality, γ governance, υ uncertainty, λ utility). Edge admission by
    semantic coherence R_A ≥ τ_A and causal drift ε_η. 5D projection to
    PSE-native `FiveDState` via η' = clip(0,1, 0.50η + 0.25γ + 0.25(1−υ)).

  - `pse-nxalien-agent`: `ContextProjector` — renders `[NXALIEN-CONTEXT]` blocks,
    `CLAUDE.md`, `AGENTS.md`, `.rules` for LLM system-prompt injection.

  - `pse-nxalien-pse`: `NxAlienObservationAdapter` implementing
    `pse_graph::ObservationAdapter` — bundles enter PSE through the same pathway
    as Binance / weather / seismo adapters. Phase hint from gate outcome
    (Accept=π/4, Hold=π/2, EvidenceOnly=3π/4, Reject=π) so Mandorla interference
    reflects governance quality. Invariant **I-BRIDGE-001** enforced by a static
    source guard: nxalien crates must never construct `SemanticCrystal` directly.

  - `pse-nxalien-evolve`: attractor-constrained rule evolution —
    * `GraphState` persists the PSE point cloud across compile runs
      (`.nxalien/graph_state.json`) so the attractor centroid accumulates history.
    * `EpistemicSignal` classifies stability as Initialising / Converging / Stable /
      Drifting / Diverging from distance to attractor centroid + free-energy trend
      + live IL health overlay.
    * `EvolutionGuard` prevents unbounded drift (min attractor alignment threshold,
      max severity downgrade steps, evidence requirement).
    * `propose_rule_evolution` / `apply_validated_proposals` — rejected proposals
      become `UnknownSlot`s to keep drift visible.
    * `il_bridge`: every `RuleAtom` committed to `ILStore` as a QTIC-certified
      `SemanticCrystal`. Severity drives `stability_score` (0.50 / 0.75 / 1.00);
      evidence density drives `kuramoto_coherence`; ψ = kuramoto − (1−stability)
      reflects governance quality. Blocking rules with evidence reach **Q5**
      (path-invariant attractor); Advisory rules without evidence sit at **Q3**.
      `load_il_health_and_agenda` reads `MemoryHealthReport` + `EpistemicAgenda`
      and folds them into `EpistemicSignal`: `at_risk_count > 0` overrides Stable →
      Drifting; `mean_qtic < 2.0` → Diverging; healthy IL + Converging PSE →
      Stable. Agenda items with p ≥ 0.50 surface as `UnknownSlot`s in the
      `[NXALIEN-CONTEXT]` block.

  **`nxalien compile` pipeline (one command, PSE workspace):**
  1. Scan project → default `RuleAtom` set (5 rules for Rust/cargo)
  2. `auto_downgrade_rules` — Required without evidence → Advisory
  3. `HypercubeHdag` — 17 nodes, 23 edges (acyclic ✓)
  4. 8-gate evaluation → `NxAlienGateReport` (Accept)
  5. JCS manifest hash + SHA-256 replay hash chain
  6. Ingest bundle into `PersistentGraph` via `NxAlienObservationAdapter`
  7. Each `RuleAtom` → `SemanticCrystal` → `ILStore` (QTIC certificate per rule)
  8. `EpistemicSignal::extract_with_il` — PSE attractor + IL health overlay
  9. `propose_rule_evolution` → `apply_validated_proposals` (guard-constrained)
  10. IL agenda → `UnknownSlot`s → `.nxalien/il_agenda_unknowns.json`
  11. Outputs: `nxalien.manifest.json`, `nxalien.signal.json`, `.nxalien/il/`
      ledger, `nxalien.rules.md`, `nxalien.evolved-rules.json`

  **Live output (PSE workspace, 5 rules):**
  ```
  IL crystals : 5/5  QTIC̄=3.80  gate=✓
    rust-test, no-direct-crystal  → Q5  (Blocking, path-invariant)
    rust-fmt, rust-clippy, minimal-reversible → Q3  (Required, gate-passed)
  PSE signal  : Stable (dist=0.000)  IL: Q̄=3.8 u=0.42 at_risk=0 ⚠
  ```
  The ⚠ correctly identifies that rules without Evidence references have
  mean uncertainty 0.42 > 0.30 (healthy threshold) — the system requests
  evidence before confirming full health.

* **PSE+IL Intelligence Layer** — `adapters/pse-adapter-il` — 10 new modules
  implementing an active-cognition layer over the IL ledger. 191 unit tests total.
  The layer turns the ledger from a passive record-keeper into an epistemic system
  that monitors its own health, manages knowledge lifecycle, enforces constitutional
  constraints, and generates a prioritised action plan toward the knowledge fixpoint.

  - **Direction 1 — Context compression** (`context.rs`): `ContextBudget`
    (max_tokens / top_k / min_qtic_class), `CrystalSummary` (compact 1-2 line
    representation with Pfauenthron++ D score), `ILStore::context_for_query()` —
    budget-filtered `[PSE-CONTEXT]...[/PSE-CONTEXT]` block for LLM system message
    injection. `IndexEntry` gains `question`, `scale_tag`, `agent_id` fields
    (backward-compatible via `#[serde(default)]`).

  - **Direction 2 — Causal graph** (`causal.rs`): `CausalGraph`, `CausalLink`,
    `CausalCause` (Refinement | Sequential | ResonanceProximity | MetatronIsomorphic |
    UserAsserted), link strength ∈ [0, 1]. Persisted in `il_causal.json` alongside the
    ledger index. `ILStore::causal_graph()` provides the full lineage DAG.

  - **Direction 3 — Agent layer** (`agent.rs`): `AgentCausalGraph`, `AgentLink`
    — multi-agent extension tracking crystal provenance per agent and cross-agent
    causal relationships. Crystals committed with `agent_id` are automatically wired.

  - **Direction 4 — Constitutional AI substrate** (`constitutional.rs`):
    `ConstitutionalRule`, `Severity` (Blocking | Required | Advisory),
    `RulePredicate` (composable tree: All / Any / Not / MinQticClass /
    MinStability / MinKuramoto / MaxFreeEnergy / MinEvidenceEntries /
    CoherenceGate / PathInvariant / RequiresAgentAttribution), `ConstitutionalReport`
    (SHA-256 content-addressed per crystal), `ConstitutionalAuditReport`,
    `ConstitutionalFeedback`, `Constitution`.

    Two preset constitutions: `eu_ai_act_minimal()` (EU AI Act Articles 9/13/17)
    and `pse_core_safety()` (4 rules including S4 hallucination attractor gate —
    `NOT(stability > 0.8 AND kuramoto < 0.2)`).

    `ILStore::commit_constitutional()` — blocking pre-commit check; crystals
    violating a Blocking rule are rejected before writing. `is_constitutionally_closed()`
    — knowledge-base-level Q5 fixpoint: all blocking rules pass for all crystals.
    19 unit tests.

  - **Direction 5 — Epistemic health monitoring** (`health.rs`):
    `crystal_uncertainty(qtic_class, stability, coherence) -> f64`:
    `u = 1 − (qtic_weight · stability · coherence)^(1/3)`.
    `CrystalHealthMetrics`, `MemoryHealthReport` (total, mean QTIC class,
    fraction_q4_plus, mean_stability, mean_coherence, mean_uncertainty,
    healthy_count, at_risk_count, attributed_fraction, oldest/newest block).
    `is_healthy()`: `fraction_q4_plus ≥ 0.80 AND mean_uncertainty ≤ 0.30`.
    `ILStore::memory_health()`, `at_risk_crystals(threshold)`,
    `crystal_health(id_prefix)`. 13 unit tests.

  - **Direction 6 — Crystal lifecycle management** (`lifecycle.rs`):
    `DecayModel` (Linear / Exponential / Step, each with `half_life`),
    `LifecycleStatus` (Vital / Aging / Stale / Redundant),
    `CrystalLifecycle` (age_blocks, decay, uncertainty, refresh_score, status),
    `ConsolidationCandidate` (MetatronIsomorphic | SemanticOverlap, with
    retain/deprecate decision), `LifecycleReport`.
    `refresh_score = uncertainty × (1 − decay)` — urgency of re-asking a question.
    `is_lifecycle_closed()`: no stale crystals and no consolidation candidates.
    `ILStore::lifecycle_report(model, sim_threshold, reference_index)`. 18 unit tests.

  - **LLM prompt grounding** (`prompt.rs`): `GroundedPrompt` and `PromptConfig`
    — compose the full LLM system message from a `[PSE-CONTEXT]` block,
    a `[AGENDA]` block, and the base system prompt, with configurable token budgets.

  - **Causal retrieval** (`retrieval.rs`): `CausalRetrievalConfig` (seed_k,
    max_depth, causal_blend α), `CausalRole` (Seed | Ancestor { depth } |
    Descendant { depth }), `CausallyGroundedEntry` (summary + role + semantic_score
    + causal_score + blended score), `CausalRetrievalResult`.
    Score blending: `final = α · D_semantic + (1−α) · D_causal` where
    `D_causal = seed_semantic · path_strength / (1 + hop_count)`.
    `to_annotated_context_block()` → `[PSE-CONTEXT causal=true]` with
    `[SEED]` / `[ANCESTOR depth=N]` / `[DESCENDANT depth=N]` annotations.
    `ILStore::causal_retrieval(query, config)`. 11 unit tests.

  - **Knowledge clustering** (`cluster.rs`): `ClusterConfig` (sim_threshold,
    min_cluster_size), `KnowledgeCluster` (members, centroid, mean_stability,
    mean_uncertainty, causal_density, mean_qtic_class), `BridgeCrystal`
    (crystal_id, bridges: Vec<cluster_id>, cross_cluster_degree),
    `ClusteringReport` (clusters, singletons, bridge_crystals, total_crystals,
    clustered_fraction). Union-Find connected-component algorithm.
    Causal density = direct causal edges / C(|members|, 2).
    `is_unified()`: singletons empty AND clusters.len() ≤ 1.
    `ILStore::cluster_knowledge(config)`. 13 unit tests.

  - **Epistemic agenda** (`agenda.rs`): `AgendaAction` (Refresh / Reinforce /
    Consolidate / Guard / Explore), `AgendaItem` (priority ∈ [0,1], action,
    rationale, expected_uncertainty_delta), `EpistemicAgenda` (items sorted
    by descending priority, diagnosis, items_to_fixpoint), `AgendaConfig`.
    Priority model: blocking constitutional violation → 1.00; bridge at risk
    → 0.90×u; stale causal root → 0.85×refresh; consolidation metatron → 0.70;
    consolidation semantic → 0.60; at-risk non-root → 0.75×u; stale non-root
    → 0.65×refresh; singleton → 0.30.
    `to_context_block(top_k)` → `[AGENDA]...[/AGENDA]` for LLM system message.
    `is_fixpoint()`: items list is empty. 13 unit tests.

    **The four fixpoint conditions** — the IL store is at epistemic fixpoint when
    `constitutional_audit().is_constitutionally_closed()` AND
    `lifecycle_report().is_lifecycle_closed()` AND
    `cluster_knowledge().is_unified()` AND
    `epistemic_agenda().is_fixpoint()` all hold simultaneously.

* **Infinity Ledger (IL) integration** — `adapters/pse-adapter-il` — full
  PSE+IL fusion layer.

  Bundles the private Infinity Ledger distribution as a zip in
  `vendors/infinityledger/` (single-repo requirement: cloning `lashsesh/pse` is
  sufficient). Exposes an `ILStore` that wraps the IL block-chain ledger and
  wires it to PSE's `SemanticCrystal` pipeline.

  Key components added:

  - **`ILStore`** — append-only ledger of crystal blocks (8D vector, topology
    signature, stability score, Metatron canonical hash). `commit_with_feedback()`
    returns a `ValidationFeedback`; `commit()` wraps it for backward compatibility.

  - **`ValidationFeedback`** — `{ block_hash, converged, coherence_potential,
    gate_passed, hdag_node_id, il_stability }`. When `|il_stability −
    original.stability| > 0.02`, `refine_crystal()` is automatically called.

  - **`refine_crystal()`** — IL→PSE feedback loop. Produces a new crystal with
    blended stability `0.7·PSE + 0.3·IL`, a fresh SHA-256 content address, and the
    original in `parent_crystal_ids`. The refined crystal is also committed to IL,
    creating a `refinement` HDAG edge.

  - **`IndexEntry`** gains: `phase: f64`, `hdag_node_id: Option<String>`,
    `stability_score: f64` (serde `default = 0.5`), `metatron_canonical_hash:
    Option<String>` — all backward-compatible with existing ledger files.

* **HDAG v1.0** (`adapters/pse-adapter-il/src/hdag.rs`) — Hierarchical Directed
  Acyclic Graph over the IL ledger. Implements the spec in
  `specs/HDAG_bySebastianKlemm_v1.0.pdf`.

  - **5D resonance tensor** per crystal:
    `[mean_propagation_time, kuramoto_coherence, cheeger_estimate, spectral_gap,
    1−stability_score]` = `[temporal, morphic, relational, topological, entropic]`.
    When Metatron data is present, `cheeger_estimate` and `spectral_gap` are
    replaced by `algebraic_connectivity/n` and `spectral_radius/n`.

  - **Coherence potential** ψ = `kuramoto_coherence − (1 − stability_score)`.
    S_coh class: ψ > −0.1 or Kairos gate passed.

  - **Emergent acyclicity** — edges only added when ψ(target) ≥ ψ(source); no
    timestamp checks required.

  - **Four edge causes**: `sequential_commit`, `resonance_proximity` (‖T_A−T_B‖ ≤
    0.35, both in S_coh), `refinement` (parent_crystal_ids link), `metatron_isomorphic`
    (shared Metatron canonical hash).

  - **Path invariance** (`∮Φ·dl = 0`) — `verify_path_invariance()` using Kahn's
    topological sort and canonical-condensation comparison.

  - **Semantic predecessor search** — `find_semantic_predecessors()` for resonance
    proximity edge wiring.

  - **HDAG statistics**: `edge_count_by_cause()`, `mean_coherence_potential()`,
    `topological_order()`.

* **Pfauenthron++ Unified Retrieval** (`D = ψ · ρ · ω`) — implements the
  tripolar scoring formula from `specs/TheTimelessMonolith_bySebastianKlemm_v1.0.pdf`.

  - `ILStore::score_tripolar(&[f64]) -> Vec<ILMatch>` — multiplicative D = ψ·ρ·ω
    where ψ = IL cosine similarity, ρ = `stability_score`, ω = normalized HDAG
    coherence potential.

  - `pse-llm-demo` uses `pfauenthron_score_all()` instead of the legacy
    `query_similar()`. Logs: `[Unified retrieval: N record(s), top D=X.XXX]`.
    Context label: `[Unified Retrieval — Pfauenthron++ D=ψ·ρ·ω]`.

  - Gabriel4D Funnel: all three axes must be non-trivial — a near-zero on any
    axis collapses the overall D score.

* **pse-server IL/HDAG HTTP routes** (`tools/pse-server`) — four new routes on
  top of the existing four PSE routes (total: eight routes):
  `GET /il/status`, `POST /il/retrieve` (Pfauenthron++),
  `GET /il/hdag/coherence`, `GET /il/hdag/order`.
  IL routes activate only when `PSE_IL_PATH` is set at startup.
  `IngestResponse` gains `il_commits: Vec<ILCommitInfo>` (skip_serializing_if empty).

* **MetatronTopologySignature in HDAG tensor** — `crystal_to_tensor()` uses
  Metatron scan data (`algebraic_connectivity/n`, `spectral_radius/n`) when
  present, giving graph-theoretic precision over heuristic cheeger/spectral
  estimates from the topology signature.

* **QTIC theoretical foundation** documented — `specs/QTIC.pdf` fully mapped
  onto PSE+IL. The full QTIC↔PSE+IL table and Q0–Q5 conformance class
  mapping are documented in `README.md`. Every Q5-conformant crystal is a
  seam-stable, path-invariant, replayable information attractor.

* **PSE-VALIDATION-RUNNER-DOMAIN-01** — Domain validation layer for the PSE workspace.

  Adds a complete L3 domain validation pipeline that runs embedded
  ground-truth benchmark scenarios (seismo/vitals/binance) and derives a
  formal `ValidationConclusion` from real run artifacts.

  Key components:

  - **`pse-bench-gt` JSON output**: `--scenario <seismo|vitals|binance>`,
    `--format json`, `--out <path>` flags added to the `bench_gt` binary.
    Produces machine-readable `BenchGtJsonOutput` with P/R/F1, PSE vs
    STL-zscore vs IsoForest metrics, and `config_hash` / `data_hash`.

  - **`DomainValidationSummary`**: Built from real bench_gt JSON outputs.
    Includes `BaselineComparisonReport` (PSE F1 vs baselines per scenario),
    leakage check, and test-split completion status.

  - **Scoring gate tightened**: `ScoringInputs` gains `domain_test_completed`
    field. `EmpiricalImprovement` requires `domain_test_completed = true`;
    domain available but test not done → `DiagnosticFinding`.

  - **Domain CLI**: `pse-validate run --profile domain --domain-manifest <path>`
    with fail-closed behavior (error if `--domain-manifest` is missing).

  - **`verdict.json`**: Written to every run output directory, records
    conclusion, domain flags, and replay identity.

  - **Command plan**: Domain phases now invoke real
    `cargo run -p pse-bench-gt --bin bench_gt -- --scenario <name> --format json`
    commands (DomainCalibration→seismo, DomainValidation→vitals, DomainTest→binance).

  - **Embedded fixture**: `validation_domains/embedded_ground_truth/manifest.json`
    with three non-overlapping splits (distinct data hashes) for
    seismo/vitals/binance scenarios.

  - **8 new tests**: domain profile requires manifest, missing manifest fails,
    domain summary from records, baseline comparison wins, no domain→no
    empirical improvement, test not completed→diagnostic finding, leakage
    invalidates, verdict.json written.

* **PSE-NCTCS-CONFORMANCE-01** — Null-Centered Toroidal Control Closure
  Layer. New submodule `crates/pse-validation-runner/src/nctcs/` (14
  modules) inside the existing `pse-validation-runner` crate.

  Implements a C0–C4 conformance ladder and produces a content-addressed
  `NctcsClosureBundle` (byte-identical replay). The pipeline:

  ```text
  NctcsRunDescriptor + NctcsClosureInput
    → NullCenterRef         (C0: exogenous, not_phase_state, not_agent)
    → NullProjectionAudit   (K0 ≠ π0(K0): projection distinction)
    → ToroidalPhaseFlowAudit (phase-flow timing, visibility-only)
    → PhaseVisibilityAudit  (C1: phase-gated visibility, coverage ≥ θ)
    → CandidateFormationAudit (C2: candidate_requires_visibility_passed)
    → MaterializationAudit  (C2: no direct fabric→tensor mutation,
                              Dissolution-Grundsatz preserved)
    → TraceReplayContractReport (ReplayIdentity ≥ threshold,
                              replay_ready_required_for_gate_pass)
    → classify_conformance  (two-pass: pre-macro, then with MacroControlState)
    → MacroControlState?    (C4 only, from null_center + tensor + trace
                              — NEVER from resonance or ephemeral fabric)
    → NctcsClosureBundle    (content-addressed, JCS + SHA-256)
  ```

  **Conformance ladder**: `C0FormalTyped` (exogenous null center) →
  `C1PhaseGatedVisibility` (phase-gated candidate visibility) →
  `C2GateBoundMaterialization` (gate-bound tensor revisions) →
  `C3AuditableTensor` (auditable tensor history + trace) →
  `C4MacroControl` (full macro control state).

  **Gate semantics** (fail-closed): `NctcsGateOutcome::Pass` is the
  only materializing outcome; `Hold / Reject / Quarantine / NoUpdate /
  HandoffReady` all produce a non-materializing decision record.
  `ValidationClosureStatus` never reaches `EmpiricalImprovement`
  without a real domain validation result.

  Eight NCTCS metrics registered in `pse-eval-matrix`
  (`nctcs_conformance_class_score`, `nctcs_visibility_candidate_compliance`,
  `nctcs_no_direct_persistence_rate`, `nctcs_gate_bound_revision_rate`,
  `nctcs_trace_replay_contract_rate`, `nctcs_macro_state_validity`,
  `nctcs_coherence_truth_separation_rate`,
  `nctcs_domain_validation_required_compliance`).

  CLI commands added to `pse-validation-runner-cli`:
  `nctcs-close` (full closure pipeline → `nctcs_closure_bundle.json`),
  `nctcs-replay` (byte-identity verification),
  `nctcs-verify` (declared bundle_id recomputation).

  Tests: 10 unit tests, 2 integration tests, 3 negative tests (25 total
  in `nctcs/tests.rs`).

* **PSE-METATRON-MONOLITH-01** — Holistic Eigenmode Closure Layer. New
  submodule `crates/pse-metatron/src/closure/` (11 modules) inside the
  existing `pse-metatron` crate, plus a new `pse-metatron-cli` binary.

  Evaluates a composite fail-closed gate over the full PSE stack and
  produces a content-addressed `HolisticEigenmodeState` only when every
  sub-gate passes:

  ```text
  MetatronRunDescriptor + MetatronClosureInput
    → LocalMonolithProjection[]  (content-addressed per projection)
    → IsomorphicProjectionReport[]  (operator-path + gate-order +
                                     trace + replay dependency checks)
    → SpectralGapStitchReport    (prior_gap, post_gap, delta_gap,
                                  improved_or_preserved)
    → MetatronGateReport         (G_meta = G_nctcs ∧ G_trace ∧ G_replay
                                  ∧ G_iso ∧ G_gap ∧ G_eval ∧ G_drift)
    → MetatronClosureOutcome:
        Closed(HolisticEigenmodeState)  ← gate passed
        Diagnostic(MetatronGateReport)  ← gate failed (fail-closed)
        Rejected(reason)                ← pre-flight policy violation
  ```

  **Gate semantics** (fail-closed): `G_iso` requires at least one
  `IsomorphicProjectionReport` with `passed = true` (vacuously-empty
  does NOT pass). No `HolisticEigenmodeState` with productive status
  is ever produced when `G_meta = 0`.

  **Metatron conformance classes** `M0–M5` classify how many gates of
  the composite passed. `HolisticEigenmodeState` is content-addressed
  (JCS + SHA-256, self-referential `state_id` computed from the
  zero-initialized form). Replay verification zeroes the ID before
  recomputing, matching `build()` — same fix applied to
  `verify_nctcs_bundle`.

  Self-contained `closure/primitives.rs` re-implements `Hash256`,
  `CanonicalNumber`, and `content_address()` using `serde_jcs` +
  `sha2` directly to avoid the cyclic crate dependency
  `pse-traverse → pse-core → pse-cascade → pse-metatron → pse-traverse`.

  `pse-metatron-cli` binary (new tool `tools/pse-metatron-cli/`):
  `inspect` / `project-local` / `isomorphism` / `spectral-gap` /
  `close` / `replay` / `verify` (7 subcommands).

  Tests: 8 unit tests, 2 integration tests, 3 negative tests (13 total
  in `closure/tests.rs`).

* **PSE-TRAVERSE-TPT-MTL-04** — Topological Panoptic Triangulation and
  Möbius-Tripolar Micro-Lift topology layer (conformance class TPTM-5).
  New module `crates/pse-traverse/src/topology/` (feature `topology`)
  and CLI binary `pse-traverse-topology-cli` (14 subcommands).

  Core pipeline: `PhaseSpaceWindow` → `AxisBridgeReport` (I-03 axis
  separation) → `MeshHolo` (seed + evolve under `TopologyGuard`) →
  `MicroFiber[]` (primary + MTL-D1 dual + seam per point) →
  `CarrierReport` (I-06 stateless null-center) →
  `ReinterpretationReport` (Betti numbers → claim candidates) →
  `TptMtlGateReport` (13 fail-closed gates) →
  `TopologicalCrystalCandidate` (not a SemanticCrystal) →
  `TptMtlBundle` → `ReplayManifest` (5-digest replay anchor).

  Ten invariants enforced (I-01 … I-10). MTL-D1 dualization uses f64
  for intermediate arithmetic to avoid rational overflow; results are
  quantized to Fixed(scale=9) before any hashing, preserving
  audit-pathway determinism. All 24 topology integration tests and 218
  total pse-traverse tests pass.

  Ten TPT-MTL metrics registered in `pse-eval-matrix`:
  `tpt_adapter_totality_rate`, `tpt_axis_bridge_validity`,
  `tpt_mesh_determinism_identity`, `tpt_topology_robustness`,
  `tpt_micro_lift_coverage`, `tpt_seam_consistency_rate`,
  `tpt_carrier_continuity`, `tpt_false_crystal_rate`,
  `tpt_trace_completeness`, `tpt_replay_identity`.

* **PHASEMATRIX-HIVEMIND-03** — morphodynamic resonance cell substrate.
  New crate `phase-matrix` and CLI tool `pse-phase-matrix-cli` (binary:
  `phase-matrix`). Implements the spec's full cell-pool → pulses →
  cluster → funnel-graph → morphology → convergence → intent → trace
  → dissolution pipeline as a deterministic `run_cell_substrate_cycle`
  that the runner / replay / verify subcommands all share.
  * Data model: `PhaseCell` (with deterministic `synthetic` factory and
    `PhaseCellRole` covering Sensor / Resonator / Router / Validator /
    MemoryProbe / BoundaryGuard / CandidateEmitter /
    MorphologyRegulator), `CellPool` with matrix-boundary enforcement
    at insertion (foreign-parent cells are rejected), `TridentVector`
    (semantic_density × structural_coherence × temporal_phase →
    activation_potential), `LocalResonanceProcessor` +
    `ResonanceNonlinearity` (Logistic / TanhApprox / SaturatingLinear
    / PiecewiseFixed), `ResonancePulse` with `PhaseBin` quantisation
    (Continuous / KPolar / Tripolar / Quadrupolar), `ResonanceCluster`
    + `ClusterLifecycle` (Proposed / Forming / Active / Stabilized /
    Splitting / Fusing / Decaying / Compacted / Dissolved / Rejected)
    + `ClusterFormationReport`, `FunnelGraph` with four edge families
    (Spatial / Temporal / Semantic / Resonance) and DFS-based
    WHITE/GRAY/BLACK acyclicity validation, `MorphodynamicField`
    (`H = α · Φ + β · µ`) + `ClusterMorphologyEvent` (Grow / Split /
    Fuse / Decay / Replicate / Stabilize / DissolveWorkingState /
    CompactToTrace) + `MorphologyDecision`, `ConvergenceField`,
    `TensionToIntentOperator` + `IntentCandidate` (sorted claim refs),
    `RecursiveFeedbackReport` (Ouroboros loop with bounded learning
    rate), `ClusterTrace` tying every artefact hash together,
    `DissolutionMode` (DropWorkingState / CompactToTrace /
    PersistEvidenceOnly / PersistClusterSummary / ArchiveFullState) +
    `DissolutionReport.validate_trace_preservation` enforcing the
    Dissolution-Grundsatz (working state may be compacted but trace +
    evidence + lifecycle history MUST be preserved),
    `CellToHandoffCandidate`, `PhaseSubnet` /
    `PhaseMatrixNode` / `NodeTrustState`, `MatrixClaim` /
    `TruthMaintenanceReport` / `MatrixBoundaryReport`,
    `CycleReportSummary`, `PhaseMatrixRunDescriptorV3` with
    `CellSubstrateThresholds::permissive()` /
    `CellSubstratePolicies::strict()` / `MatrixGatePolicy::strict()`,
    `ReplayObservation` / `verify_cycle_replay`.
  * Five fail-closed gates: `G_cluster` (phase ∧ coherence ∧ morpho ∧
    purpose ∧ trace), `G_morph` (endo ∧ exo ∧ boundary-safe),
    `G_intent` (tension ∧ convergence ∧ conflict ∧ trace-ready),
    `G_dissolve` (working-state-eligible ∧ trace-persisted ∧
    evidence-persisted ∧ gate-history-persisted), plus the
    matrix-boundary check at the pool layer.
  * `pipeline::run_cell_substrate_cycle` drives the full deterministic
    cycle and returns a `CellSubstrateOutcome` (Completed / Hold /
    Rejected / Compacted / MatrixBoundaryViolation /
    DeterminismViolation). Two runs over the same `(input, rd)` are
    byte-identical.
  * **No `SemanticCrystal` and no `FinalizedEmission`** are
    constructed in any cell-substrate module — the substrate emits
    handoff candidates only; the PSE-Bridge remains the only commit
    path. The `no_commit_artefacts_appear_in_outcome_bytes` test
    guards this invariant against canonical bytes.
  * Feature flags: `cell-substrate` (default-on), `cell-cli`,
    `cell-funnel-graph`, `cell-morphodynamics`, `cell-convergence`,
    `cell-handoff`.
  * Float-free in every gate / score path: `Fixed` (`CanonicalNumber`)
    rationals normalised by gcd, JCS-canonical reports, sorted lists
    before hashing, `BTreeMap`-keyed structures, no wall-clock in the
    audit pathway, no platform RNG.
  * CLI `phase-matrix`: `cluster-cycle`, `cluster-replay`,
    `cluster-verify`, `cell-pool`. Four CLI smoke tests cover the full
    cycle / replay / verify / pool flow.
  * Tests: 34 unit tests + 6 end-to-end integration tests + 4 CLI
    smoke tests.

* **PHASEMATRIX-HIVEMIND-03.1** — Dual-Fabric Field-Tensor Stitch
  Layer. Additive patch on the PHASEMATRIX-HIVEMIND-03 cell-substrate
  implementation. Plugs cleanly into the existing `phase-matrix` crate
  without duplicating or parallelising any existing architecture.
  * **Data model**: `FieldTensorState` (Fabric-T — persistent;
    content-addressed; carries `tensor_revision`, `coupling_matrix_hash`,
    `previous_tensor_hash` chain, `trace_head`), `CouplingMatrix` +
    `CouplingEntry` (five coupling kinds: Structural / Resonance /
    Temporal / Semantic / Boundary), `ResonanceFabricState` (Fabric-H
    — ephemeral; derived deterministically from each
    `CellSubstrateCycleReport`; carries mandatory `trace_hash` per §5.2
    Invariant), `EphemeralResonanceLink` (source / target / resonance_score
    / phase_alignment / ttl_ticks), `StitchCandidate` (proposed coupling
    change; never touches Fabric-T directly), `CouplingUpdate` (accepted
    change; references exactly one `StitcherGateReport`),
    `MirrorConsistencyReport` (MCI per candidate), `TensorDeltaReport`
    (cumulative L1 norm + per-edge max + hypothetical tensor-after hash),
    `StitcherGateReport` (per candidate; all seven sub-gate booleans),
    `FieldTensorTrace` (append-only audit log; sorted before hashing),
    `StitcherReport` (content-addressed outcome; sorted accepted_updates
    / rejected_candidates / gate_reports before hashing),
    `StitcherOutcome` (Completed / Hold / Rejected — all carry the report),
    `StitchRunDescriptor` (replay anchor), `StitchThresholds` /
    `StitchPolicies`, `StitchCycleBundle` (replay-ready artefact containing
    rd, fabric_h, tensor_before, tensor_after, outcome,
    source_cluster_trace_hash).
  * **Key invariants**:
    * Invariant 1 — Fabric-H isolation: Fabric-H MUST NEVER directly
      mutate Fabric-T; all changes route through the StitcherGate.
    * Invariant 2 — StitcherGate is fail-closed:
      `G_stitch = G_conv ∧ G_mci ∧ G_delta ∧ G_budget ∧ G_trace ∧ G_boundary ∧ G_evidence`.
    * Invariant 3 — tensor_revision increments exactly once per
      accepted batch.
    * Invariant 4 — previous_tensor_hash chain is preserved.
    * Invariant 5 — when no updates accepted, tensor_after is
      byte-identical to tensor_before (no trace_head mutation).
    * Invariant 9 — `CouplingUpdate`s sorted before hashing when
      `sort_updates_before_hash = true`.
  * **Pipeline** (`run_stitch_cycle`): validate descriptor → build
    Fabric-H → derive candidates → mirror consistency → tensor delta →
    gate evaluation → collect accepted updates → apply to Fabric-T →
    write FieldTensorTrace → write StitcherReport. Replay path
    (`verify_stitch_replay`) reuses the stored Fabric-H directly
    (bypassing `build_resonance_fabric`) for byte-identity verification.
  * **New modules** in `crates/phase-matrix/src/cell/`:
    `field_tensor`, `resonance_fabric`, `coupling_update`, `stitcher`,
    `stitcher_gate`, `mirror_consistency`, `tensor_delta`,
    `field_tensor_trace`, `stitch_pipeline`.
  * **New CLI subcommands** in `pse-phase-matrix-cli`:
    `stitch-fabric`, `stitch-candidates`, `stitch-gate`, `stitch-apply`,
    `stitch-cycle`, `stitch-replay`, `tensor-inspect`.
  * **New Cargo feature**: `cell-stitch` (default-on; depends on all
    four prior cell features).
  * Float-free in every gate / score path (all `Fixed`); no wall-clock;
    `BTreeMap` for all keyed collections; sorted-before-hashing for all
    lists; JCS-canonical reports.
  * 9 unit tests in `stitch_pipeline.rs`; 5 integration tests in
    `end_to_end.rs`; 3 CLI smoke tests in `cli_smoke.rs`.

* **PSE-EVAL-MATRIX-01 — PHASEMATRIX-HIVEMIND-03.1 closure.** Extended
  the eval matrix so the system stays empirically closed across the
  new stitch layer (additive on top of the HIVEMIND-03 closure):
  * New `WorkloadFamily::DualFabricStitch` (the matrix now lists eleven
    mandatory families) plus `WorkloadSpec::dual_fabric_stitch`
    constructor with hold-correctness / no-false-commit /
    replay-byte-identical success criteria.
  * Six new `CellSubstrateMetricKind` variants for the stitch layer:
    `StitcherGatePassRate`, `CouplingUpdateTraceCoverage`,
    `TensorRevisionMonotonicity`, `MirrorConsistencyCompliance`,
    `StitchReplayIdentity`, `FabricHIsolationRate`.  Three are primary:
    `StitcherGatePassRate`, `StitchReplayIdentity`,
    `FabricHIsolationRate`. `FabricHIsolationRate` is always 1.0 (hard
    invariant; any deviation would be a critical failure).
  * New `dual_fabric_stitch_metric_specs()` (6 metrics) and
    `b9_metric_specs()` (16 metrics: 10 cell-substrate + 6 stitch).
  * New `LayerMask::DUAL_FABRIC_STITCH` bit (1 << 13) and
    `B9_DualFabricStitch` ladder rung (= `B8_PhaseMatrix |
    DUAL_FABRIC_STITCH`), `SystemVariantSpec::dual_fabric_stitch()`
    constructor, and `VariantLadder::full_with_dual_fabric_stitch()`.
  * New `dual-fabric-stitch` preset (B0 / B8 / B9 over the
    `DualFabricStitch` workload, scored against all 16 B9 metrics).
  * `SyntheticTrialExecutor` now emits stitch-layer metric observations
    for `DualFabricStitch` workloads: the six stitch metrics for
    stitch-active variants (B9); the ten cell-substrate metrics for
    substrate-active variants (B8+); `FabricHIsolationRate = 1.0`
    always.
  * `preset_dual_fabric_stitch` exported from `pse-eval-matrix` crate root.
  * 4 new tests in `presets.rs` and `cell_substrate_metrics.rs`.

* **PSE-EVAL-MATRIX-01 — PHASEMATRIX-HIVEMIND-03 closure.** Extended
  the eval matrix so the system stays empirically closed across the
  new substrate:
  * New `WorkloadFamily::MorphoCellSubstrate` (the matrix now lists
    ten mandatory families) plus `WorkloadSpec::morpho_cell_substrate`
    constructor with the standard hold-correctness / no-false-commit /
    replay-byte-identical success criteria.
  * New `cell_substrate_metrics` module with the canonical
    PHASEMATRIX-HIVEMIND-03 metric set (ten metrics:
    `cluster_formation_rate`, `morphology_gate_compliance`,
    `convergence_stability`, `intent_generation_rate`,
    `dissolution_trace_preservation`, `funnel_acyclicity_rate`,
    `matrix_boundary_violation_rate`,
    `working_state_compaction_efficiency`,
    `handoff_candidate_utility`, `substrate_self_coherence`).
  * New `LayerMask::CELL_SUBSTRATE` bit and `B8_PhaseMatrix` ladder
    rung (= `B7_FullStack | CELL_SUBSTRATE`),
    `SystemVariantSpec::phase_matrix_substrate()` constructor, and
    `VariantLadder::full_with_phase_matrix()` for the extended
    nine-rung ladder.
  * New `phase-matrix-substrate` preset (B0 / B7 / B8 over the new
    workload, scored against the full cell-substrate metric set).
  * `SyntheticTrialExecutor` now emits the cell-substrate metric
    observations whenever the workload is `MorphoCellSubstrate`,
    pinned to the fail-closed floor for variants without the
    `CELL_SUBSTRATE` bit and monotonically uplifted for the B8
    variant. Two regression tests guard the uplift on
    `cluster_formation_rate` and the lower-is-better behaviour of
    `matrix_boundary_violation_rate`.

* **PSE-EVAL-MATRIX-01** — empirical benchmark matrix for
  post-symbolic cognition systems. New crate `pse-eval-matrix` and
  CLI tool `pse-eval-matrix-cli` (binary: `pse-eval-matrix`).
  * Data model: `EvaluationSpec` (content-addressed, validatable),
    `SystemVariantSpec` over the B0 → B7 variant ladder with
    explicit `LayerMask` bitset, `WorkloadSpec` over nine mandatory
    families (`StreamEvent` / `AnomalyRegime` / `TraversalPuzzle` /
    `CodeAgentPatch` / `DocSynthesis` / `MemoryReuse` /
    `HorizonFinalization` / `CognitionPanorama` / `MultiAgent`),
    `DatasetManifest` with `calibration` / `validation` / `test`
    splits, `GroundTruthProfile` (synthetic-exact, semi-synthetic
    injection, historical, unit-test oracle, human-adjudicated),
    `MetricSpec` (family / direction / primary flag / aggregation /
    invalidation rules), `MetricObservation`,
    `EvaluationRunLedger` (append-only, hash-chained) with
    `EvaluationRunEntry` and `RunStatus`, `TrialReport` with
    `TrialOutputs` / `GateObservationSet` / `ReplayObservation` /
    `DiagnosticRecord`, `EvaluationSummaryReport`,
    `CapabilityProfile`, `AblationSummary` + `MetricDelta` +
    `AblationConclusion`, `StatisticalSummary`,
    `ReviewerReport` (qualitative rubric), `FailureRecord` /
    `FailureKind` (replay mismatch, false crystal, missed event,
    false handoff, over-hold, under-hold, memory mislead, wormhole
    abuse, calibration leakage), `CalibrationLedgerEntry` /
    `CalibrationProfile` / `CalibrationReason`.
  * Operators: `plan_runs` (deterministic plan), `run_trial` +
    `TrialExecutor` trait (pluggable, with reference
    `SyntheticTrialExecutor`), `init_ledger` / `append_to_ledger` /
    `verify_ledger_chain` (rolling chain hash),
    `verify_trial_replay` (byte-identity check), `score_ledger`
    (aggregates strictly from declared `MetricObservation`s — never
    recomputes), `score_capability_profile` (`U_task / U_replay /
    U_safety / U_cognition / U_efficiency / U_calibration /
    U_robustness` + Safety-Adjusted Utility),
    `safety_adjusted_utility`, `cognition_uplift`,
    `layer_marginal_utility`, `summarize_ablation`,
    `build_ablation_ladder` (eight ablation rungs per §3.2),
    `bootstrap_mean_ci` (deterministic seeded LCG — no platform
    RNG), `exact_binomial_ci`, `paired_mean_diff`,
    `render_markdown_summary`, `render_json_summary`.
  * Three built-in presets (§18): `agent-cognition`,
    `streaming-event-detection`, `post-symbolic-ablation`. Each
    preset stamps a content-addressed spec; the CLI's `init
    --template <preset>` is the canonical entry point.
  * Feature flags: `eval-matrix` (default-on), `eval-cli`,
    `eval-agent`, `eval-cognition`, `eval-streams`,
    `eval-statistics`, `eval-reports`.
  * Float-free in every score / metric / gate hash:
    `CanonicalNumber` only, gcd-normalised rationals to keep i128
    arithmetic safe under composition, `BTreeMap` keyed,
    sorted lists before hashing, JCS-canonical reports, no
    wall-clock timestamps in the audit pathway, no platform RNG.
  * `Schlussformel` (§23) enforced: a system counts as *empirically
    improved* only when `ΔU_task > 0 ∧ ΔU_safety ≥ 0 ∧
    ReplayIdentity = 1 ∧ InvalidRunRate ≤ ε ∧ LMU_target > 0`,
    surfaced as `ConclusionFlag::EmpiricalImprovement` vs.
    `DiagnosticFinding` / `InvalidatedByReplay` /
    `InvalidatedByLeakage`.
  * CLI: `pse-eval-matrix init|validate|plan|run|replay|score|ablate|compare|report`.
  * 49 unit tests + 6 end-to-end integration tests + 4 CLI smoke
    tests; workspace test count rises to **839 / 839** passing.

* **PSE-TRAVERSE-COGNITION-01** — panoptic phase cognition kernel
  layer in `crates/pse-traverse/src/cognition/`:
  * Layered data model `C0–C10`: `CognitionRunDescriptor`,
    `CanonicalCognitionState`, `CognitiveState5D`
    (`ψ, ρ, ω, χ, τ` + derived potential / energy / entropy /
    stability_index), `SingularityDetectorReport`,
    typed `OperatorDeclaration` / `OperatorFamily` / `OperatorType` /
    `IntegrationMode` / `IntegratorKind` / `CognitionSimulationSpec`,
    `SpiralMemoryAddress` / `SpiralMemoryHitSet` / `SpiralSegment`,
    `ConstraintLatticeCognition` / `Resonite` / `Infogene` /
    `AdmissibleRegion` / `InfogenePolicy`,
    `HypercubePuzzleState` / `CognitiveDimension` /
    `PartialAssignment` / `CandidateSet` / `HiddenSingle` /
    `BoundaryContract` / `NegativeTopologyWitness` /
    `EntropyCollapseCertificate`,
    `PhasePanorama` / `Horizon360` / `PhasePath` /
    `AttractorCandidate` / `RecognitionBoundary` /
    `ChoiceGeometryReport`,
    `ScorpioPhaseScheduler` / `ActivationWindow` / `ResonanceOffset` /
    `TransportPolicy` / `VectorTunnelTransport` / `ReasonCode`,
    `GovernedWormhole`,
    `SelfModelTensor` / `ReflexiveModulation` /
    `DualTriggerFeedbackGate`,
    `FixpointCalibrationShell` / `PerformanceTriplet` /
    `ResonanceImpulse` / `CarrierMigrationPlan`,
    `AttractorMap` / `AttractorEntry`,
    `SingularityTriggerReport`,
    `CognitionHandoffGate` / `ProjectionHandoffPolicy` /
    `CognitionCandidate` / `CognitionCandidateBundle`,
    `CognitionReport` / `CognitionHoldReport` /
    `CognitionDiagnostic` / `CognitionRecoveryAction` /
    `CognitionOutcome`.
  * Operators: `null_center_unfold`-style derivations,
    `detect_singularity`, `spiral_memory_query`,
    `build_lattice_minimal`, `evaluate_perkolation`,
    `build_puzzle_minimal`, `build_panorama_minimal`,
    `build_scheduler_minimal`, `admit_wormhole`, `build_self_model`,
    `evaluate_dual_trigger`, `calibrate`, `evaluate_por_acceptance`,
    `evaluate_migration`, `evaluate_singularity_trigger`,
    `CognitionHandoffGate::evaluate`.
  * `pipeline::run_cognition` — total reference pipeline (per §16:
    canonicalize → 5D state → spiral memory query → constraint
    lattice → hypercube puzzle → perkolation → scheduler → panorama
    → wormholes → self-model → dual-trigger feedback → fixpoint
    calibration → carrier migration → attractor ranking →
    singularity trigger → handoff gate → bundle-or-hold → report →
    replay).
  * `replay_hash_of` / `assert_replay_match` for byte-identity audit.
  * Feature flags: `cognition` (default-on), `cognition-cli`,
    `cognition-simulation`, `cognition-spiral-memory`,
    `cognition-hypercube`, `cognition-scorpio-phase`,
    `cognition-wormholes`, `cognition-calibration`,
    `cognition-projection-handoff`.
  * Float-free everywhere: gate-relevant scalars are `Fixed`
    (`CanonicalNumber`); rationals are normalised by gcd to keep
    i128 arithmetic safe under composition; keyed structures are
    `BTreeMap`; lists are sorted before hashing; reports are
    JCS-canonicalised.
  * **No `SemanticCrystal` and no `FinalizedEmission`** are
    constructed in any cognition module — the kernel hands a
    `CognitionCandidateBundle` to projection-v0.2, which alone may
    finalise; the PSE-Bridge remains the only commit path.
* **`pse-traverse-cognition-cli`** tool binary (binary name
  `pse-traverse-cognition`) with the spec's twelve subcommands
  (§18): `inspect`, `observe`, `state5`, `memory-query`, `lattice`,
  `puzzle`, `panorama`, `calibrate`, `trigger`, `bundle`, `replay`,
  `verify`. Golden fixtures in
  `tools/pse-traverse-cognition-cli/tests/fixtures/` and
  end-to-end CLI smoke tests for every subcommand.
* **PSE-TRAVERSE-HORIZON-03** — null-centered horizon geometry layer
  in `crates/pse-traverse/src/horizon/`:
  * Data model: `HorizonRunDescriptorV3`, `HorizonThresholdsV3`,
    `HorizonPoliciesV3`, `HorizonFailurePolicy`, `CarrierPolicyV3`,
    `HorizonWindowPolicyV3`, `ProjectionConePolicyV3`,
    `CausalPolicyV3`, `DualityPolicyV3`, `CrossingPolicyV3`,
    `RationalFixed`, `EpochRange`, `HorizonEvidenceRef`,
    `HorizonError`.
  * Operators: `NullCenterUnfold`, `PhaseRayLift` (hypertorus T^n,
    default n = 4), `HorizonVisibility`, `ProjectionConeCheck`,
    `CausalOrderCheck`, `CollapseEmissionDualityCheck`,
    `HorizonCrossingGate`, combined gate
    `G_v0.3 = G_projection_v2 ∧ G_cross ∧ ReplayReady`.
  * Reports & artefacts: `HorizonChart` (content-addressed),
    `PhaseRay`, `EventHorizonWindowV3` /
    `HorizonWindowReportV3`, `ProjectionCone` /
    `ProjectionConeReport`, `CausalAdmissibilityReport` (with
    `CausalViolation`), `DualityReport`, `HorizonCrossingReport`,
    `FinalizedEmissionV3`, `HorizonHoldReport`, `HorizonV3Outcome`
    (`Finalized` / `Hold` / `WaitForHorizon` / `RefineCone` /
    `NeedsCarrierMigration` / `Recondense` / `ProjectionOnly` /
    `InvalidInput` / `DeterminismViolation`),
    `HorizonCertificate`, `replay_hash_of`, `assert_replay_match`.
  * `pipeline::run_horizon_v3` — total reference pipeline
    (canonicalize → null-resolve → unfold → ray-lift →
    window-evaluate → cone-check → causal-check → duality-check →
    crossing-gate → projection-v0.2 merge → finalize-or-hold →
    certify → replay).
  * Feature flags: `horizon` (default-on), `horizon-cli`,
    `horizon-projection-v2`, `horizon-pse-bridge`,
    `horizon-adapters`.
  * Float-free in every audit path: gate-relevant scalars are
    `CanonicalNumber` (`Fixed`); rationals are `RationalFixed` with
    decimal-string i128 serialisation; keyed collections are
    `BTreeMap`; ray / window / kind lists are sorted before
    hashing; every report is JCS-canonicalised.
  * No `SemanticCrystal` is constructed in any horizon module — the
    PSE-Bridge remains the only commit path.
* **`pse-traverse-horizon-cli`** tool binary (binary name
  `pse-traverse-horizon`) implementing the spec's seven subcommands:
  `inspect`, `chart`, `rays`, `crossing`, `finalize` (refuses when
  `G_v0.3 = 0`), `replay` (byte-identity check), `verify`
  (certificate-chain audit). Golden fixtures in
  `tools/pse-traverse-horizon-cli/tests/fixtures/`. End-to-end CLI
  smoke tests cover every subcommand.
* GitHub Actions CI workflow (`.github/workflows/ci.yml`):
  fmt, clippy, build (Linux / macOS / Windows), test, doc, and a
  non-blocking `cargo audit` job.
* Dependabot configuration for weekly Cargo and monthly Actions
  updates, grouped by patch / minor.
* `CONTRIBUTING.md` with the determinism / replay ground rules,
  PR checklist, and adapter recipe.
* `SECURITY.md` with vulnerability-reporting flow, in-scope
  components, threat model, and primitive inventory.
* `CHANGELOG.md` (this file).

### Changed

* Workspace builds and tests now run warning-free under
  `RUSTFLAGS="-D warnings"`.
* `Cargo.lock` is now committed. The workspace ships binaries
  (`pse-cli`, `pse-demo`, `pse-traverse-cli`,
  `pse-traverse-horizon`, `pse-traverse-cognition`,
  `pse-eval-matrix`, `pse-bench-bbo`) where a reproducible build is
  a hard requirement.
* `cargo fmt --all` applied across the workspace; CI now enforces it.
* README and CHANGELOG document the eval matrix, cognition and
  horizon layers alongside the existing signature and dynamics
  layers; workspace test suite now reports **839 / 839** passing.

### Fixed

* Various clippy warnings: removed dead `crystal_count` in the
  Binance adapter test, replaced an `if_same_then_else` in the
  Metatron platonic classifier (the `is_iso ⇒ is_sub` collapse was
  redundant), used `std::f64::consts::PI` instead of an inline literal
  in `pse-traverse`, removed a placeholder `assert!(true)` test in
  `pse-cli` and replaced it with a real one, annotated the
  NaN-handling `!(hi > lo)` in the IsoForest baseline.

## [0.1.0] — 2026-05

Initial public iteration. Highlights below — see `README.md`
"What's new since the last README" for the full strand-letter log
(E through P, plus the signature and dynamics layers).

### Added (top-level)

* Engine architecture, strands E through N: real Mandorla / cascade,
  5D state, `CrystalAdapter`, resonance fingerprint query,
  resonance-landscape-aware TRITON.
* Operator algebra: `compose / dual / bridge / query / interpolate`.
* Falsification: `Shuffle`, `BlockBootstrap`, `PhaseRandomize`.
* AdaptiveCalibrator (P.3): rolling-history quantile thresholds,
  opt-in.
* `state.last_gate` diagnostic surface (P.2).
* PSE Traversal Agent v0.1 (`pse-traverse` crate): full
  ProblemSpec → FieldCube → DoFGraph → CollapsePlan → Candidate →
  GateReport → PSE-bridge pipeline, fail-closed.
* PSE-TRAVERSE-SIGNATURE-01: signature layer
  (`StructuralOperator` → `Signature` → `SignatureDiagnostics` →
  `SignatureGate`), `BlueprintSearch`, `NonDominatedFrontier`,
  `SearchLedger`, `SearchAutopilot`.
* PSE-TRAVERSE-DYNAMICS-01: morphodynamic tick engine
  (`CanonicalNumber`, `Hash256`/`StableId`, `BaseState`/`LiftedState`,
  `FieldAbsorber`, `GuidanceField`, `MorphodynamicCompressor`,
  `TransitionProof`, `DynamicGate`, `dynamic_tick`/`dynamic_run`).
* Ten domain adapters (Binance, ENTSO-E, Seismo, Weather, AirQuality,
  IoT, Syslog, Vitals, Tabular, ModelMon).
* Four tool binaries (`pse-bench-gt`, `pse-bench-bbo`, `pse-audit`,
  `pse-demo`).
* `pse-traverse-cli` with `inspect / plan / run / replay / search /
  dynamics` subcommands.
* `docs/POST_SYMBOLIC.md`, `docs/COMPLIANCE.md`.

[Unreleased]: https://github.com/lashsesh/pse/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/lashsesh/pse/releases/tag/v0.1.0

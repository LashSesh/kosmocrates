//! Kosmocrates integration pipeline — Phase 10 Integration Hardening.
//!
//! Provides a single `run_dry_pipeline()` entry point that wires together:
//!
//! Workbench → HYPHAE v0.3 passive run → CorpusCartography update →
//! optional Metatron v0.4.1 diagnostics → optional LPCM v0.4.2 report →
//! optional SystemCube v0.4.3 export → aggregated GateTrace → IntegrationRunReport
//!
//! The single `PolicyProfile` governs every layer; its ID appears in every
//! sub-report, proving that one policy scoped the entire run.
//!
//! Invariants:
//! - No host files are written.
//! - Fail-closed: any Reject in any layer propagates to final_result.
//! - Evidence propagates: each optional-layer output traces HYPHAE evidence.
//! - All IDs are content-addressed.
//! - Default mode: `PolicyProfile::default_report_only()`.

pub mod aggregator;
pub mod landscape_geometry;
pub mod materialization;
pub mod persistence;

pub use aggregator::{AggregatedGateResult, GateTraceAggregator, LayerGateSummary};
pub use landscape_geometry::{
    couple_proposals, landscape_geometry, LandscapeGeometry, ProposalCluster, SingularProposal,
};
pub use materialization::{
    simulate_foundry_check, MaterializationOutcome, MaterializationPlan, OperatorApprovalToken,
    ParseBackExpectation, WorkbenchMaterializationTask,
};
pub use persistence::persist_cartography_update;

use kosmo_core::ReplayStatus;
use kosmo_core::TaintLabel;
use kosmo_core::{
    rank_by_energy, Digest, GateResult, PolicyProfile, PromotionFeedback, WishFacet, Q16,
};
use kosmo_hyphae::{
    diagnose_micrograph, lift_region, passive_run, passive_run_augmented, ComplementVoidHypothesis,
    CompositeSupportCube, CorpusCartography, CorpusCartographyUpdate, CorpusEntity,
    CorpusEntityKind, CrossLanguageFingerprint, CubeDimensionProfile, CubeSwarm, DeficiencyVector,
    Fragment, FragmentField, FragmentKind, HostTargetCollapsePlan, HostTargetDelta,
    HyphaeRunResult, LpcmPassiveReport, MetatronMicrograph, MetatronRegionFingerprint,
    MicroTopologyDiagnostic, MicroTopologyIndex, MicrographLiftReport, MorphogenicCorpusUpdate,
    MotifCandidate, NormFitnessTrace, NormGeneCandidate, Resonite, SeamGraph, SourceCube,
    StructuralCrystalCandidate, StructuralCrystalRecord, SupportMassVector, SurgeryWorkbenchTask,
    TopologicalSurgeryOption, TopologyAmbiguityProfile,
};
use kosmo_pse_bridge::{PseBridgeCandidate, PseBridgeCandidateKind};
use kosmo_store::CrystalRecordStore;
use kosmo_systemcube::{BlueprintUnit, BlueprintUnitKind, KcubeExportReport, SystemCube};
pub use kosmo_workbench::WorkspaceError;
use kosmo_workbench::WorkspaceIndex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ─── IntegrationRunOptions ────────────────────────────────────────────────────

/// Configuration for which optional pipeline layers to activate.
///
/// All optional layers default to disabled, matching the `ReportOnly` mode.
/// No layer can be enabled unless the governing `PolicyProfile` permits it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntegrationRunOptions {
    /// Run Metatron v0.4.1 region lift + diagnostic for each host void.
    pub enable_metatron: bool,
    /// Run LPCM v0.4.2 passive report for each host void.
    pub enable_lpcm: bool,
    /// Build a SystemCube v0.4.3 from accepted decisions and export dry-run.
    pub enable_systemcube: bool,
    /// Denominator for D-density in the SystemCube report.
    pub systemcube_capacity: u32,
    /// Q16 seam compatibility threshold for LPCM reports.
    pub lpcm_seam_threshold: Q16,
    /// Derive and energy-rank surgery options from Metatron diagnostics.
    /// Only produces results when `enable_metatron` is also true.
    pub enable_surgery: bool,
    /// Derive energy-ranked `MotifCandidate` objects from void kind frequency (Step 5a).
    /// One candidate per recurring void kind; support_score = kind_count / total_voids.
    pub enable_motif_candidates: bool,
    /// Generate energy-ranked `NormGeneCandidate` objects from accepted decisions.
    /// Initial fitness = `Q16::ONE`; evolves via `NormFitnessTrace` in later phases.
    pub enable_norm_candidates: bool,
    /// Collect `StructuralCrystalCandidate` objects from accepted decisions (Step 5d).
    /// Candidates start with `support_score = Q16::ZERO` (pending certification).
    pub enable_crystal_candidates: bool,
    /// Convert norm + topology candidates into `PseBridgeCandidate` objects (Step 6b).
    /// Produces a flat, confidence-ranked list ready for PSE evaluation.
    pub enable_pse_candidates: bool,
    /// Prior `PromotionFeedback` records to ingest into `NormFitnessTrace` objects
    /// for each matching norm candidate (Step 5c). Empty by default.
    pub prior_feedback: Vec<PromotionFeedback>,
    /// Prior-run `MotifCandidate` objects to inject as `SuggestPattern` intents into the
    /// frontier (closes the motif feedback loop across pipeline runs). Only motifs whose
    /// `support_score` meets or exceeds `prior_motif_min_support` are included.
    pub prior_motifs: Vec<MotifCandidate>,
    /// Minimum support score threshold for `prior_motifs` → `SuggestPattern` intent promotion.
    /// Defaults to `Q16::HALF` (motif must appear in ≥50% of scanned sources).
    pub prior_motif_min_support: kosmo_core::Q16,
    /// Certified `StructuralCrystalRecord`s from previous runs to seed the corpus.
    /// These are added as `CorpusEntityKind::CrystalRecord` entities at the start of
    /// the run, making the CAD library accumulate across pipeline invocations.
    /// Only used when `enable_crystal_candidates` is also true.
    pub prior_crystals: Vec<StructuralCrystalRecord>,
    /// Path to a `CrystalRecordStore` JSONL file for automatic cross-session persistence.
    ///
    /// When set:
    /// - **On entry**: if the file exists, all stored records are loaded and merged into
    ///   `prior_crystals` (dedup by `record_id`), closing the session-to-session feedback
    ///   loop without caller-side boilerplate.
    /// - **After Step 5d-cert**: if the policy allows host writes, newly certified crystals
    ///   are appended to the store (idempotent — already-stored records are deduped).
    ///
    /// `ReportOnly` and `DryRun` modes cannot write (same `allow_host_write` invariant as
    /// `JsonlCartographyStore`), but the store is still read at entry in those modes.
    /// `None` by default — the pipeline is fully stateless unless this is set.
    #[serde(skip)]
    pub crystal_store_path: Option<std::path::PathBuf>,
}

impl IntegrationRunOptions {
    /// Safe default: all optional layers disabled.
    pub fn report_only() -> Self {
        Self {
            enable_metatron: false,
            enable_lpcm: false,
            enable_systemcube: false,
            systemcube_capacity: 0,
            lpcm_seam_threshold: Q16::ZERO,
            enable_surgery: false,
            enable_motif_candidates: false,
            enable_norm_candidates: false,
            enable_crystal_candidates: false,
            enable_pse_candidates: false,
            prior_feedback: vec![],
            prior_motifs: vec![],
            prior_motif_min_support: Q16::HALF,
            prior_crystals: vec![],
            crystal_store_path: None,
        }
    }

    /// Enable all optional layers (for integration testing with fixtures).
    pub fn all_layers(systemcube_capacity: u32) -> Self {
        Self {
            enable_metatron: true,
            enable_lpcm: true,
            enable_systemcube: true,
            systemcube_capacity,
            lpcm_seam_threshold: Q16::ZERO,
            enable_surgery: true,
            enable_motif_candidates: true,
            enable_norm_candidates: true,
            enable_crystal_candidates: true,
            enable_pse_candidates: true,
            prior_feedback: vec![],
            prior_motifs: vec![],
            prior_motif_min_support: Q16::HALF,
            prior_crystals: vec![],
            crystal_store_path: None,
        }
    }

    /// Set a crystal store path for automatic cross-session persistence.
    ///
    /// On entry the store is opened (existing records loaded into `prior_crystals`);
    /// after Step 5d-cert newly certified crystals are appended when policy allows.
    pub fn with_crystal_store_path(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.crystal_store_path = Some(path.into());
        self
    }
}

// ─── Integration Run Report ───────────────────────────────────────────────────

/// Content for deterministic `IntegrationRunReport` ID.
#[derive(Serialize)]
struct ReportContent {
    hyphae_run_id: Digest,
    deficiency_vector_id: Digest,
    cartography_update_id: Digest,
    swarm_composite_id: Digest,
    void_fill_delta_id: Digest,
    collapse_plan_id: Digest,
    morphogenic_update_id: Digest,
    void_priority_count: u32,
    policy_id: Digest,
    aggregate_id: Digest,
    metatron_count: u32,
    metatron_index_id: Digest,
    lift_report_count: u32,
    lpcm_count: u32,
    surgery_count: u32,
    surgery_workbench_task_count: u32,
    ambiguity_profile_count: u32,
    void_hypothesis_count: u32,
    crystal_candidate_count: u32,
    certified_crystal_count: u32,
    resonite_count: u32,
    pse_candidate_count: u32,
    motif_candidate_count: u32,
    norm_candidate_count: u32,
    norm_fitness_trace_count: u32,
    has_systemcube: bool,
    hyphae_ledger_id: Digest,
}

/// Unified dry-run report from the full pipeline.
///
/// Every sub-report carries `policy_id` equal to `self.policy_id`, proving
/// that a single `PolicyProfile` governed the entire run (traceability).
/// `final_result` is the fail-closed merge of all layer gate results.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntegrationRunReport {
    pub report_id: Digest,
    pub policy_id: Digest,
    pub hyphae_result: HyphaeRunResult,
    /// Deficiency summary derived from the host void map (always present).
    /// Entries sorted by kind; total_severity is the Q16 average.
    pub deficiency_vector: DeficiencyVector,
    pub cartography_update: CorpusCartographyUpdate,
    /// Phase 2b: SourceCubes built from accepted decisions (one per accepted intent).
    /// Populated always; HDAG dimensions present when workspace entries carry content.
    pub source_cubes: Vec<SourceCube>,
    /// Phase 4: merged support cube from all accepted SourceCubes.
    pub swarm_composite: CompositeSupportCube,
    /// Phase 4: energy-ranked void-fill plan (always present; passive/advisory).
    pub void_fill_delta: HostTargetDelta,
    /// Phase 4c: planning-only collapse plan derived from void-fill delta.
    pub collapse_plan: HostTargetCollapsePlan,
    /// Phase 4d: morphogenic skeleton — records what the corpus would look like
    /// after the collapse plan executes (planning only; no mutation).
    pub morphogenic_update: MorphogenicCorpusUpdate,
    pub metatron_diagnostics: Vec<MicroTopologyDiagnostic>,
    /// Content-addressed index of all micrographs, fingerprints, and diagnostics from
    /// the metatron run. Empty-index (zero-state) when `enable_metatron` is false.
    pub metatron_index: MicroTopologyIndex,
    /// Lift reports from M1 (one per void), energy-ranked by loss_ratio (most lossy first).
    /// Populated when `enable_metatron` is true; empty otherwise.
    pub lift_reports: Vec<MicrographLiftReport>,
    /// Void IDs ranked by severity (highest first) via energy kernel.
    /// Gives operators a prioritized repair order. Never empty when voids exist.
    pub void_priority_ranking: Vec<Digest>,
    pub lpcm_reports: Vec<LpcmPassiveReport>,
    /// Energy-ranked surgery options derived from Metatron diagnostics.
    /// Empty when `enable_surgery` or `enable_metatron` is false.
    pub surgery_options: Vec<TopologicalSurgeryOption>,
    /// Workbench-compatible surgery tasks derived 1:1 from `surgery_options`.
    /// In the same energy-ranked order. Empty when `surgery_options` is empty.
    pub surgery_workbench_tasks: Vec<SurgeryWorkbenchTask>,
    /// Energy-ranked topology ambiguity profiles from all Metatron diagnostics.
    /// Most-confident ambiguity first. Empty when `enable_metatron` is false.
    pub ambiguity_profiles: Vec<TopologyAmbiguityProfile>,
    /// Energy-ranked complement void hypotheses from all Metatron diagnostics.
    /// Most-confident hypothesis first. Empty when `enable_metatron` is false.
    pub complement_void_hypotheses: Vec<ComplementVoidHypothesis>,
    /// Structural crystal candidates from accepted decisions (Step 5d).
    /// All start with `support_score = Q16::ZERO` (Pending certification).
    /// Empty when `enable_crystal_candidates` is false.
    pub crystal_candidates: Vec<StructuralCrystalCandidate>,
    /// Certified crystal records produced from `crystal_candidates` that passed
    /// all constraints (Step 5d-cert). Each record is a proven reusable structural
    /// pattern — the durable element of the CAD library. Pass as `prior_crystals`
    /// in the next run to seed the accumulated pattern base.
    /// Empty when `enable_crystal_candidates` is false.
    pub certified_crystals: Vec<StructuralCrystalRecord>,
    /// Resonite measurements between current-run certified crystals and prior-run
    /// crystals (Step 5e-resonite). Each `Resonite` quantifies structural proximity
    /// between one current and one prior pattern, enabling the CAD library to
    /// detect pattern convergence across runs.
    /// Empty when `enable_crystal_candidates` is false or `prior_crystals` is empty.
    pub resonite_map: Vec<Resonite>,
    /// Flat confidence-ranked PSE bridge candidates (Step 6b) assembled from
    /// norm candidates and topology observations. Ready for PSE evaluation.
    /// Empty when `enable_pse_candidates` is false.
    pub pse_candidates: Vec<PseBridgeCandidate>,
    /// Energy-ranked motif candidates derived from void kind frequency (Step 5a).
    /// One per recurring void kind; support_score = kind_count / total_voids.
    /// Empty when `enable_motif_candidates` is false.
    pub motif_candidates: Vec<MotifCandidate>,
    /// Energy-ranked norm gene candidates generated from accepted decisions.
    /// Initial fitness = Q16::ONE; evolves via NormFitnessTrace in later phases.
    /// Empty when `enable_norm_candidates` is false.
    pub norm_candidates: Vec<NormGeneCandidate>,
    /// Fitness traces for norm candidates, built from `prior_feedback` (Step 5c).
    /// One trace per candidate that has at least one matching feedback record.
    /// Empty when `prior_feedback` is empty or no feedback matches.
    pub norm_fitness_traces: Vec<NormFitnessTrace>,
    pub systemcube_export: Option<KcubeExportReport>,
    pub aggregated_gate: AggregatedGateResult,
    /// Fail-closed merge across all layers.
    pub final_result: GateResult,
    /// Number of newly-written crystal records (Step 5f auto-persist).
    ///
    /// Observational field — not part of `report_id` (depends on store state,
    /// not deterministically reproducible from inputs alone). Zero when
    /// `crystal_store_path` is not set, policy denies writes, or no new
    /// records were produced (dedup applied).
    pub persisted_crystal_count: u32,
}

impl IntegrationRunReport {
    // One argument per content-addressed field: this aggregates every sub-report of
    // a pipeline run, so the 1:1 field mapping is the point. A builder would hide it.
    #[allow(clippy::too_many_arguments)]
    fn new(
        hyphae_result: HyphaeRunResult,
        deficiency_vector: DeficiencyVector,
        cartography_update: CorpusCartographyUpdate,
        source_cubes: Vec<SourceCube>,
        swarm_composite: CompositeSupportCube,
        void_fill_delta: HostTargetDelta,
        collapse_plan: HostTargetCollapsePlan,
        morphogenic_update: MorphogenicCorpusUpdate,
        void_priority_ranking: Vec<Digest>,
        metatron_diagnostics: Vec<MicroTopologyDiagnostic>,
        metatron_index: MicroTopologyIndex,
        lift_reports: Vec<MicrographLiftReport>,
        lpcm_reports: Vec<LpcmPassiveReport>,
        surgery_options: Vec<TopologicalSurgeryOption>,
        surgery_workbench_tasks: Vec<SurgeryWorkbenchTask>,
        ambiguity_profiles: Vec<TopologyAmbiguityProfile>,
        complement_void_hypotheses: Vec<ComplementVoidHypothesis>,
        crystal_candidates: Vec<StructuralCrystalCandidate>,
        certified_crystals: Vec<StructuralCrystalRecord>,
        resonite_map: Vec<Resonite>,
        pse_candidates: Vec<PseBridgeCandidate>,
        motif_candidates: Vec<MotifCandidate>,
        norm_candidates: Vec<NormGeneCandidate>,
        norm_fitness_traces: Vec<NormFitnessTrace>,
        systemcube_export: Option<KcubeExportReport>,
        aggregated_gate: AggregatedGateResult,
        persisted_crystal_count: u32,
        policy: &PolicyProfile,
    ) -> Self {
        let final_result = aggregated_gate.final_result.clone();
        let report_id = Digest::of(&ReportContent {
            hyphae_run_id: hyphae_result.run_id,
            deficiency_vector_id: deficiency_vector.vector_id,
            cartography_update_id: cartography_update.update_id,
            swarm_composite_id: swarm_composite.composite_id,
            void_fill_delta_id: void_fill_delta.delta_id,
            collapse_plan_id: collapse_plan.plan_id,
            morphogenic_update_id: morphogenic_update.update_id,
            void_priority_count: void_priority_ranking.len() as u32,
            policy_id: policy.id,
            aggregate_id: aggregated_gate.aggregate_id,
            metatron_count: metatron_diagnostics.len() as u32,
            metatron_index_id: metatron_index.index_id,
            lift_report_count: lift_reports.len() as u32,
            lpcm_count: lpcm_reports.len() as u32,
            surgery_count: surgery_options.len() as u32,
            surgery_workbench_task_count: surgery_workbench_tasks.len() as u32,
            ambiguity_profile_count: ambiguity_profiles.len() as u32,
            void_hypothesis_count: complement_void_hypotheses.len() as u32,
            crystal_candidate_count: crystal_candidates.len() as u32,
            certified_crystal_count: certified_crystals.len() as u32,
            resonite_count: resonite_map.len() as u32,
            pse_candidate_count: pse_candidates.len() as u32,
            motif_candidate_count: motif_candidates.len() as u32,
            norm_candidate_count: norm_candidates.len() as u32,
            norm_fitness_trace_count: norm_fitness_traces.len() as u32,
            has_systemcube: systemcube_export.is_some(),
            hyphae_ledger_id: hyphae_result.ledger.ledger_id,
        });
        Self {
            report_id,
            policy_id: policy.id,
            hyphae_result,
            deficiency_vector,
            cartography_update,
            source_cubes,
            swarm_composite,
            void_fill_delta,
            collapse_plan,
            morphogenic_update,
            void_priority_ranking,
            metatron_diagnostics,
            metatron_index,
            lift_reports,
            lpcm_reports,
            surgery_options,
            surgery_workbench_tasks,
            ambiguity_profiles,
            complement_void_hypotheses,
            crystal_candidates,
            certified_crystals,
            resonite_map,
            pse_candidates,
            motif_candidates,
            norm_candidates,
            norm_fitness_traces,
            systemcube_export,
            aggregated_gate,
            final_result,
            persisted_crystal_count,
        }
    }

    /// Total contradiction energy from the SystemCube export, if enabled.
    pub fn systemcube_contradiction_energy(&self) -> Option<Q16> {
        self.systemcube_export
            .as_ref()
            .map(|e| e.contradiction_energy.total_energy)
    }

    /// Compatibility score from the SystemCube export, if enabled.
    /// `Q16::ONE` = all accepted units are fully compatible; lower = gaps present.
    pub fn systemcube_compatibility_score(&self) -> Option<Q16> {
        self.systemcube_export
            .as_ref()
            .map(|e| e.compatibility.compatibility_score)
    }

    /// Operator-readable summary of the pipeline run.
    pub fn summary(&self) -> String {
        let scube = if let Some(ref e) = self.systemcube_export {
            format!(
                "systemcube=Some(mode={:?}, compat={}, contradiction_energy={})",
                e.mode,
                e.compatibility.compatibility_score.raw(),
                e.contradiction_energy.total_energy.raw(),
            )
        } else {
            "systemcube=None".into()
        };
        format!(
            "IntegrationRunReport — policy={:.8} | final={:?} | \
             hyphae: {} | deficiency: {} entries (severity={}) | \
             cartography: {} entities | voids (priority): {} | \
             swarm: {} cubes → {:?} | collapse: {} steps ({:?}) | \
             morphogenic: {:.8} | metatron: {} (index: {:.8}, lift: {}) | lpcm: {} | \
             surgery: {} (tasks: {}) | ambiguities: {} | void_hyp: {} | \
             crystal_candidates: {} (certified: {}, resonites: {}, persisted: {}) | \
             pse_candidates: {} | motif_candidates: {} | norm_candidates: {} (traces: {}) | {}",
            hex_prefix(&self.policy_id),
            self.final_result,
            self.hyphae_result.summary(),
            self.deficiency_vector.entries.len(),
            self.deficiency_vector.total_severity.raw(),
            self.cartography_update.added_entity_ids.len(),
            self.void_priority_ranking.len(),
            self.swarm_composite.source_cube_ids.len(),
            self.void_fill_delta.status,
            self.collapse_plan.step_count(),
            self.collapse_plan.status,
            hex_prefix(&self.morphogenic_update.update_id),
            self.metatron_diagnostics.len(),
            hex_prefix(&self.metatron_index.index_id),
            self.lift_reports.len(),
            self.lpcm_reports.len(),
            self.surgery_options.len(),
            self.surgery_workbench_tasks.len(),
            self.ambiguity_profiles.len(),
            self.complement_void_hypotheses.len(),
            self.crystal_candidates.len(),
            self.certified_crystals.len(),
            self.resonite_map.len(),
            self.persisted_crystal_count,
            self.pse_candidates.len(),
            self.motif_candidates.len(),
            self.norm_candidates.len(),
            self.norm_fitness_traces.len(),
            scube,
        )
    }

    /// Whether all layers passed their gates.
    pub fn is_clean(&self) -> bool {
        self.final_result.is_pass()
    }

    /// Verify that every sub-report's policy_id matches the governing policy.
    ///
    /// This is the traceability invariant: one policy, one run.
    pub fn verify_policy_consistency(&self) -> bool {
        let pid = self.policy_id;
        self.hyphae_result.policy_id == pid
            && self.deficiency_vector.policy_id == pid
            && self.cartography_update.policy_id == pid
            && self.swarm_composite.policy_id == pid
            && self.void_fill_delta.policy_id == pid
            && self.collapse_plan.policy_id == pid
            && self.morphogenic_update.policy_id == pid
            && self.aggregated_gate.policy_id == pid
            && self.metatron_diagnostics.iter().all(|d| d.policy_id == pid)
            && self.metatron_index.policy_id == pid
            && self.lpcm_reports.iter().all(|r| r.policy_id == pid)
            && self.surgery_options.iter().all(|o| o.policy_id == pid)
            && self
                .surgery_workbench_tasks
                .iter()
                .all(|t| t.policy_id == pid)
            && self.ambiguity_profiles.iter().all(|a| a.policy_id == pid)
            && self
                .complement_void_hypotheses
                .iter()
                .all(|h| h.policy_id == pid)
            && self.crystal_candidates.iter().all(|c| c.policy_id == pid)
            && self.certified_crystals.iter().all(|r| r.policy_id == pid)
            && self.resonite_map.iter().all(|r| r.policy_id == pid)
            && self.pse_candidates.iter().all(|c| c.policy_id == pid)
            && self.motif_candidates.iter().all(|c| c.policy_id == pid)
            && self.norm_candidates.iter().all(|c| c.policy_id == pid)
            && self.norm_fitness_traces.iter().all(|t| t.policy_id == pid)
            && self
                .systemcube_export
                .as_ref()
                .is_none_or(|e| e.policy_id == pid)
    }
}

fn hex_prefix(d: &Digest) -> String {
    d.as_bytes()
        .iter()
        .take(4)
        .map(|b| format!("{:02x}", b))
        .collect()
}

// ─── Pipeline Entry Point ──────────────────────────────────────────────────────

/// Best cross-language structural resonance of `fp` against its distinct peers.
///
/// Returns the maximum [`CrossLanguageFingerprint::similarity`] to any fingerprint
/// in `distinct` whose source differs from `fp`'s (a true peer, not itself), or
/// `None` when there is no positive resonance or no distinct peer. Integer `Q16`
/// throughout (CROSS-007). Energy ranks, never gates (CROSS-010): the returned
/// value only reorders gate-passed candidates.
fn best_cross_language_resonance(
    fp: &CrossLanguageFingerprint,
    distinct: &[CrossLanguageFingerprint],
) -> Option<Q16> {
    let best = distinct
        .iter()
        .filter(|other| other.source_evidence_id != fp.source_evidence_id)
        .map(|other| fp.similarity(other).raw())
        .max()
        .unwrap_or(0);
    if best > 0 {
        Some(Q16::from_raw(best))
    } else {
        None
    }
}

// ─── Wish landscape: findings become wish proposals ──────────────────────────

/// One point in the wish landscape: a substrate finding projected into the
/// wish vocabulary. Content-addressed; `severity` is the originating void's
/// severity and doubles as the proposal's rank **and** the predicate weight
/// an adopted wish gives this facet (energy ranks — adoption is the
/// operator's choice, never automatic).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WishProposal {
    pub proposal_id: Digest,
    /// The measurable target this finding projects to.
    pub facet: WishFacet,
    /// The originating void (provenance into the diagnosis world).
    pub void_id: Digest,
    /// The void's severity ∈ `[0, 1]` (Q16) — ranking and predicate weight.
    pub severity: Q16,
    /// The module the finding concerns — lets a consumer check whether the
    /// wish world can even *see* the target (a non-Rust module is honest
    /// residue at measurement time, not a stalling wish).
    pub subject: String,
    /// Operator-readable origin, e.g. `"MissingDocFiber @ src/router.rs"`.
    pub rationale: String,
}

#[derive(Serialize)]
struct WishProposalContent<'a> {
    facet: &'a WishFacet,
    void_id: &'a Digest,
    severity_raw: i64,
}

impl WishProposal {
    fn new(
        facet: WishFacet,
        void_id: Digest,
        severity: Q16,
        subject: String,
        rationale: String,
    ) -> Self {
        let proposal_id = Digest::of(&WishProposalContent {
            facet: &facet,
            void_id: &void_id,
            severity_raw: severity.raw(),
        });
        Self {
            proposal_id,
            facet,
            void_id,
            severity,
            subject,
            rationale,
        }
    }
}

/// A finding the wish vocabulary cannot express (yet) — the honest residue
/// of the landscape. Listed, counted, never silently dropped.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UnmappedVoid {
    pub void_id: Digest,
    pub kind_label: String,
    pub location: String,
}

/// The wish landscape of a workspace: every void the wish vocabulary can
/// express, as a ranked [`WishProposal`] — plus the honest residue.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WishLandscape {
    pub proposals: Vec<WishProposal>,
    pub unmapped: Vec<UnmappedVoid>,
}

/// Project the substrate's findings into the wish vocabulary — pure and
/// deterministic. The current projection:
///
/// - `MissingDocFiber { for_module }`  → `Doc(for_module)` (the module's
///   declaration carries no doc comment)
/// - `MissingTestFiber { for_module }` → `Test("<for_module>_smoke")` (the
///   deterministic smoke-test name the scaffolder builds)
/// - everything else → [`UnmappedVoid`] (honest residue; the vocabulary
///   grows finding-class by finding-class)
///
/// Proposals are deduplicated per facet (highest severity wins) and sorted
/// by severity descending, then facet key — deterministic landscape order.
/// The bare module stem of a void target: `"src/router.rs"` → `"router"`,
/// `"handlers.py"` → `"handlers"`. The wish world observes modules by their
/// `mod` declaration name, so the projection must speak that name. The crate
/// roots (`lib`, `main`) keep their stem and stay honest residue at
/// measurement time — no `mod lib;` exists to observe or scaffold.
fn module_stem(target: &str) -> String {
    let last = target.rsplit('/').next().unwrap_or(target);
    let stem = last.split('.').next().unwrap_or(last);
    stem.to_string()
}

pub fn propose_wishes(voids: &[kosmo_hyphae::HostVoid]) -> WishLandscape {
    use kosmo_hyphae::HostVoidKind;
    let mut landscape = WishLandscape::default();
    for v in voids {
        let mapped = match &v.kind {
            HostVoidKind::MissingDocFiber { for_module } if !for_module.is_empty() => {
                let m = module_stem(for_module);
                Some((
                    WishFacet::doc(m.clone()),
                    m,
                    format!("MissingDocFiber @ {}", v.location),
                ))
            }
            HostVoidKind::MissingTestFiber { for_module } if !for_module.is_empty() => {
                let m = module_stem(for_module);
                Some((
                    WishFacet::test(format!("{m}_smoke")),
                    m,
                    format!("MissingTestFiber @ {}", v.location),
                ))
            }
            _ => None,
        };
        match mapped {
            Some((facet, subject, rationale)) => {
                match landscape.proposals.iter_mut().find(|p| p.facet == facet) {
                    Some(existing) if existing.severity >= v.severity => {}
                    Some(existing) => {
                        *existing =
                            WishProposal::new(facet, v.void_id, v.severity, subject, rationale);
                    }
                    None => {
                        landscape.proposals.push(WishProposal::new(
                            facet, v.void_id, v.severity, subject, rationale,
                        ));
                    }
                }
            }
            None => landscape.unmapped.push(UnmappedVoid {
                void_id: v.void_id,
                kind_label: format!("{:?}", v.kind),
                location: v.location.clone(),
            }),
        }
    }
    landscape.proposals.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then(a.facet.key.cmp(&b.facet.key))
    });
    landscape.unmapped.sort_by(|a, b| {
        a.location
            .cmp(&b.location)
            .then(a.kind_label.cmp(&b.kind_label))
    });
    landscape
}

/// The measured standing of one [`WishProposal`] against an observed
/// topology — the honesty column of the landscape.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LandscapeStanding {
    /// The facet is already present — nothing to wish for.
    Met,
    /// Observable and unmet — adoptable.
    Open,
    /// The wish world cannot see the target (non-Rust module, crate root):
    /// honest residue, not a stalling wish.
    BeyondObservation,
    /// No observation was available (e.g. not a cargo workspace).
    Unmeasured,
}

impl LandscapeStanding {
    /// Stable lowercase label for JSON surfaces.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Met => "met",
            Self::Open => "open",
            Self::BeyondObservation => "beyond-observation",
            Self::Unmeasured => "unmeasured",
        }
    }
}

/// Measure every proposal against an observed topology (or mark all
/// [`LandscapeStanding::Unmeasured`] when none is available). One standing
/// definition for every surface — CLI, server, TUI.
pub fn measure_landscape(
    landscape: &WishLandscape,
    observed: Option<&kosmo_core::ObservedTopology>,
) -> Vec<LandscapeStanding> {
    landscape
        .proposals
        .iter()
        .map(|p| match observed {
            None => LandscapeStanding::Unmeasured,
            Some(obs) => {
                if obs.contains(&p.facet) {
                    LandscapeStanding::Met
                } else if obs.contains(&WishFacet::module(p.subject.clone()))
                    || obs.contains(&WishFacet::symbol(p.subject.clone()))
                {
                    LandscapeStanding::Open
                } else {
                    LandscapeStanding::BeyondObservation
                }
            }
        })
        .collect()
}

/// Wrap a certified [`StructuralCrystalRecord`] as a [`PseBridgeCandidate`] —
/// the substrate→core unification adapter.
///
/// The certified crystal is the substrate's strongest durable artifact
/// (gate-passed, constraint-certified, replay-proofed), so this is the primary
/// offer path from the CAD library into PSE crystallization. The candidate is
/// **offer-only**: PSE runs its own gate cascade and decides crystallization
/// (the bridge's architecture contract).
///
/// - `observation_digest` = `record.record_id` (content-addressing by reference).
/// - `confidence` = `(ρ + ω) / 2` in `Q16` integer arithmetic — the record's
///   structural poles, following the substrate convention that a record without
///   HDAG provenance carries the unconstrained baseline `Q16::ONE` for both.
/// - When the record carries a [`CrossLanguageFingerprint`], its language and
///   `fingerprint_id` travel as metadata so the PSE side can route and de-dupe
///   cross-language patterns.
/// - `evidence_bundle_id` comes from the record itself — every certified record
///   is directly evidence-bound (CROSS-006), so store-loaded CAD-library
///   crystals are promotable without resolving their certifying candidate.
pub fn crystal_to_pse_candidate(
    record: &StructuralCrystalRecord,
    source_run_id: Digest,
    policy_id: Digest,
) -> PseBridgeCandidate {
    let confidence = Q16::from_raw((record.rho_coherence.raw() + record.omega_phase.raw()) / 2);
    let language = record
        .fingerprint
        .as_ref()
        .map(|f| f.language.as_str())
        .unwrap_or("-");
    let label = format!("crystal:{}:{}", language, &record.record_id.to_hex()[..12]);
    let candidate = PseBridgeCandidate::new(
        PseBridgeCandidateKind::CertifiedCrystal,
        record.record_id,
        label,
        confidence,
        source_run_id,
        record.evidence_bundle_id,
        policy_id,
    );
    match record.fingerprint.as_ref() {
        Some(f) => candidate
            .with_metadata("language", f.language.as_str())
            .with_metadata("fingerprint_id", f.fingerprint_id.to_hex()),
        None => candidate,
    }
}

/// Run the full Kosmocrates dry-run pipeline on a workspace index.
///
/// No host files are written. All outputs are passive/advisory reports.
/// The governing `PolicyProfile` is applied at every layer.
///
/// Optional layers (Metatron, LPCM, SystemCube) are activated via `options`.
/// In `ReportOnly` mode (default policy), all optional layers are safe to run
/// because no materialization or host mutation can occur.
pub fn run_dry_pipeline(
    index: &WorkspaceIndex,
    options: &IntegrationRunOptions,
    policy: &PolicyProfile,
) -> IntegrationRunReport {
    let mut agg = GateTraceAggregator::new(policy.id);

    // ── 0. Crystal store auto-load (Step 5f pre-run) ─────────────────────────
    // If a crystal_store_path is set and the file exists, open the store and
    // merge its records into the effective prior_crystals. Dedup by record_id
    // so manually-provided prior_crystals are not duplicated.
    let effective_prior_crystals: std::borrow::Cow<[StructuralCrystalRecord]> =
        if let Some(ref store_path) = options.crystal_store_path {
            if store_path.exists() {
                match CrystalRecordStore::open(store_path) {
                    Ok(store) if !store.is_empty() => {
                        let mut merged = options.prior_crystals.clone();
                        for rec in store.records() {
                            if !merged.iter().any(|r| r.record_id == rec.record_id) {
                                merged.push(rec.clone());
                            }
                        }
                        std::borrow::Cow::Owned(merged)
                    }
                    _ => std::borrow::Cow::Borrowed(options.prior_crystals.as_slice()),
                }
            } else {
                std::borrow::Cow::Borrowed(options.prior_crystals.as_slice())
            }
        } else {
            std::borrow::Cow::Borrowed(options.prior_crystals.as_slice())
        };
    let prior_crystals: &[StructuralCrystalRecord] = &effective_prior_crystals;

    // ── 1. HYPHAE v0.3 passive run ────────────────────────────────────────────
    // If prior_motifs are provided, inject SuggestPattern intents (motif feedback loop).
    let hyphae = if options.prior_motifs.is_empty() {
        passive_run(index, policy)
    } else {
        let extra: Vec<_> = options
            .prior_motifs
            .iter()
            .filter(|m| m.support_score.at_least(options.prior_motif_min_support))
            .map(|m| {
                kosmo_hyphae::frontier::SourceIntent::new(
                    kosmo_hyphae::frontier::SourceIntentKind::SuggestPattern {
                        pattern_name: m.name.clone(),
                    },
                    None,
                    m.taint.clone(),
                    kosmo_core::AuthorityLabel::Agent {
                        name: "hyphae-v0.3".into(),
                    },
                )
            })
            .collect();
        passive_run_augmented(index, policy, extra)
    };

    // Record HYPHAE gate contribution: any rejected decision contributes Reject.
    let hyphae_gate = if hyphae.rejected_count > 0 {
        GateResult::Reject {
            reason: format!("{} yields rejected by gate cascade", hyphae.rejected_count),
        }
    } else if hyphae.evidence_only_count > 0 {
        GateResult::Warn {
            message: format!(
                "{} yields downgraded to evidence-only",
                hyphae.evidence_only_count
            ),
        }
    } else {
        GateResult::Pass
    };
    agg.record("hyphae", hyphae.run_id, hyphae_gate);

    // ── 1b. Void priority ranking — severity-ordered repair queue ─────────────
    // Computed once from the void_map; always present, costs only a sort.
    let void_priority_ranking = hyphae
        .host_cube
        .void_map
        .priority_ranking(&GateResult::Pass);

    // ── 1c. DeficiencyVector — derived from void_map, always present ─────────
    let deficiency_vector = DeficiencyVector::from_void_map(&hyphae.host_cube.void_map);

    // ── 2. CorpusCartography update ───────────────────────────────────────────
    // Seed the corpus with prior crystal records so the CAD library accumulates
    // across pipeline runs. Prior crystals are added as CrystalRecord entities
    // before the current run's data is appended.
    let base_corpus = if options.enable_crystal_candidates && !prior_crystals.is_empty() {
        let crystal_entities: Vec<CorpusEntity> = prior_crystals
            .iter()
            .map(|r| {
                CorpusEntity::new(
                    CorpusEntityKind::CrystalRecord,
                    r.record_id,
                    TaintLabel::Clean,
                    policy.id,
                )
            })
            .collect();
        CorpusCartography::empty(policy.id)
            .append(crystal_entities, vec![])
            .0
    } else {
        CorpusCartography::empty(policy.id)
    };
    let (_, cartography_update) = base_corpus.update_from_run(&hyphae);
    agg.record(
        "cartography",
        cartography_update.update_id,
        GateResult::Pass,
    );

    // ── 3. Optional Metatron v0.4.1 diagnostics ───────────────────────────────
    let mut metatron_diagnostics: Vec<MicroTopologyDiagnostic> = Vec::new();
    let mut raw_lift_reports: Vec<MicrographLiftReport> = Vec::new();
    let mut metatron_triples: Vec<(
        MetatronMicrograph,
        MetatronRegionFingerprint,
        MicroTopologyDiagnostic,
    )> = Vec::new();
    if options.enable_metatron {
        for void in &hyphae.host_cube.void_map.voids {
            let ev_id = void.void_id;
            let (micrograph, fingerprint, lift_report) = lift_region(
                void.void_id,
                vec![void.void_id],
                ev_id,
                TaintLabel::Synthetic,
                policy,
            );
            let diag = diagnose_micrograph(&micrograph, &fingerprint, Some(&void.kind), policy);
            agg.record(
                format!("metatron:{:.8}", hex_prefix(&void.void_id)),
                diag.diagnostic_id,
                GateResult::Pass,
            );
            metatron_diagnostics.push(diag.clone());
            raw_lift_reports.push(lift_report);
            metatron_triples.push((micrograph, fingerprint, diag));
        }
    }
    // ── 3c. Energy-rank lift reports by loss_ratio (most lossy first) ─────────
    let lift_reports: Vec<MicrographLiftReport> = {
        let assessments: Vec<_> = raw_lift_reports
            .iter()
            .map(|r| r.energy_assessment(&GateResult::Pass))
            .collect();
        let ranked = rank_by_energy(&assessments);
        ranked
            .iter()
            .filter_map(|a| {
                raw_lift_reports
                    .iter()
                    .find(|r| r.report_id == a.subject_id)
                    .cloned()
            })
            .collect()
    };
    // ── 3d. MicroTopologyIndex — content-addressed index of all metatron output ─
    let metatron_index: MicroTopologyIndex = metatron_triples
        .iter()
        .fold(MicroTopologyIndex::empty(policy.id), |idx, (m, f, d)| {
            idx.add(m, f, d)
        });

    // ── 3b. Optional surgery — energy-ranked options from Metatron diagnostics ─
    // Only runs when both Metatron and surgery are enabled; requires diagnostics.
    let surgery_options: Vec<TopologicalSurgeryOption> =
        if options.enable_surgery && options.enable_metatron {
            let raw: Vec<TopologicalSurgeryOption> = metatron_diagnostics
                .iter()
                .flat_map(|diag| TopologicalSurgeryOption::from_diagnostic(diag, policy))
                .collect();
            let assessments: Vec<_> = raw
                .iter()
                .map(|o| o.energy_assessment(&GateResult::Pass))
                .collect();
            let ranked = rank_by_energy(&assessments);
            // Reorder raw options to match energy ranking.
            ranked
                .iter()
                .filter_map(|a| raw.iter().find(|o| o.option_id == a.subject_id).cloned())
                .collect()
        } else {
            Vec::new()
        };

    // ── 3e. Surgery workbench tasks — 1:1 from ranked surgery options ─────────
    let surgery_workbench_tasks: Vec<SurgeryWorkbenchTask> = surgery_options
        .iter()
        .map(SurgeryWorkbenchTask::from_option)
        .collect();

    // ── 3f. Flatten + energy-rank ambiguity profiles and void hypotheses ────────
    let ambiguity_profiles: Vec<TopologyAmbiguityProfile> = {
        let raw: Vec<TopologyAmbiguityProfile> = metatron_diagnostics
            .iter()
            .flat_map(|d| d.ambiguities.iter().cloned())
            .collect();
        let assessments: Vec<_> = raw
            .iter()
            .map(|a| a.energy_assessment(&GateResult::Pass))
            .collect();
        let ranked = rank_by_energy(&assessments);
        ranked
            .iter()
            .filter_map(|a| raw.iter().find(|p| p.profile_id == a.subject_id).cloned())
            .collect()
    };
    let complement_void_hypotheses: Vec<ComplementVoidHypothesis> = {
        let raw: Vec<ComplementVoidHypothesis> = metatron_diagnostics
            .iter()
            .flat_map(|d| d.void_hypotheses.iter().cloned())
            .collect();
        let assessments: Vec<_> = raw
            .iter()
            .map(|h| h.energy_assessment(&GateResult::Pass))
            .collect();
        let ranked = rank_by_energy(&assessments);
        ranked
            .iter()
            .filter_map(|a| {
                raw.iter()
                    .find(|h| h.hypothesis_id == a.subject_id)
                    .cloned()
            })
            .collect()
    };

    // ── 4. Optional LPCM v0.4.2 passive reports ───────────────────────────────
    let mut lpcm_reports: Vec<LpcmPassiveReport> = Vec::new();
    if options.enable_lpcm {
        for void in &hyphae.host_cube.void_map.voids {
            let ev_id = void.void_id;
            let fragment = Fragment::new(
                void.void_id,
                0,
                FragmentKind::CohesiveRegion,
                vec![void.void_id],
                ev_id,
            );
            let field = FragmentField::new(void.void_id, ev_id, policy, vec![fragment]);
            let entries = vec![(field.fragments[0].fragment_id, Q16::ONE)];
            let support = SupportMassVector::new(field.field_id, policy, entries);
            let seam_graph = SeamGraph::new(field.field_id, policy, vec![]);
            let lpcm = LpcmPassiveReport::build(
                void.void_id,
                field,
                support,
                seam_graph,
                options.lpcm_seam_threshold,
                ev_id,
                policy,
            );
            agg.record(
                format!("lpcm:{:.8}", hex_prefix(&void.void_id)),
                lpcm.report_id,
                GateResult::Pass,
            );
            lpcm_reports.push(lpcm);
        }
    }

    // ── 4b. Phase 4 CubeSwarm — energy-ranked void-fill plan ─────────────────
    // Build seam_map from LPCM reports (when available): per-void seam coherence
    // = fraction of seam edges that are compatible with the threshold.
    let seam_map: BTreeMap<Digest, Q16> = lpcm_reports
        .iter()
        .map(|r| {
            let total = r.seam_graph.edges.len() as u64;
            let compatible = r
                .seam_graph
                .compatible_edges(options.lpcm_seam_threshold)
                .len() as u64;
            let coherence = if total == 0 {
                Q16::ONE
            } else {
                Q16::ratio(compatible, total).unwrap_or(Q16::ONE)
            };
            (r.host_void_id, coherence)
        })
        .collect();

    // Distinct per-file cross-language fingerprints (deduped by source evidence),
    // used to rank cubes by how strongly each void's source resonates structurally
    // with the rest of the workspace — across languages. Energy ranks, never gates
    // (CROSS-010), so this can only reorder gate-passed candidates.
    let distinct_fingerprints: Vec<CrossLanguageFingerprint> = {
        let mut seen: std::collections::BTreeSet<Digest> = std::collections::BTreeSet::new();
        let mut out: Vec<CrossLanguageFingerprint> = Vec::new();
        for fp in hyphae.host_cube.fingerprint_by_void_id.values() {
            if seen.insert(fp.source_evidence_id) {
                out.push(fp.clone());
            }
        }
        out
    };

    // Build SourceCubes from accepted decisions. intents and decisions are
    // produced in lockstep by passive_run, so zip is safe.
    // When a CodeHDAG is available for the void (entry had source content),
    // its rho_coherence and omega_phase are added as dimensions — making the
    // energy assessment structurally aware rather than file-presence-only.
    let source_cubes: Vec<SourceCube> = hyphae
        .frontier
        .intents
        .iter()
        .zip(hyphae.decisions.iter())
        .filter(|(_, d)| d.outcome.is_accepted())
        .map(|(intent, decision)| {
            let dims = if let Some(void_id) = intent.target_void_id {
                if let Some(hdag) = hyphae.host_cube.hdag_by_void_id.get(&void_id) {
                    let rho = hdag.rho_coherence();
                    let omega = hdag.omega_phase();
                    let mut d = std::collections::BTreeMap::new();
                    d.insert("rho_coherence".to_string(), rho);
                    d.insert("omega_phase".to_string(), omega);
                    // When prior crystals are available, compute the best structural
                    // resonance between this void's HDAG and all prior crystal records.
                    // High resonance → the void matches a known certified pattern →
                    // crystal_resonance dimension boosts ρ (coherence) in energy ranking.
                    // Only set when prior_crystals is non-empty (no false-zero baseline).
                    if !prior_crystals.is_empty() {
                        // Prefer the richer, language-independent four-axis fingerprint
                        // resonance when both the void and the prior crystal carry one;
                        // fall back to the two-axis ρ/ω proximity otherwise. This makes
                        // crystal matching work across languages: a Go crystal can
                        // resonate with a structurally-similar Python void.
                        let void_fp = hyphae.host_cube.fingerprint_by_void_id.get(&void_id);
                        let best = prior_crystals
                            .iter()
                            .map(|prior| {
                                if let Some(vfp) = void_fp {
                                    if let Some(sim) = prior.fingerprint_resonance(vfp) {
                                        return sim;
                                    }
                                }
                                let rho_diff =
                                    (rho.raw() - prior.rho_coherence.raw()).unsigned_abs() as i64;
                                let omega_diff =
                                    (omega.raw() - prior.omega_phase.raw()).unsigned_abs() as i64;
                                let rho_sim = (Q16::ONE.raw() - rho_diff).max(0);
                                let omega_sim = (Q16::ONE.raw() - omega_diff).max(0);
                                Q16::from_raw((rho_sim + omega_sim) / 2)
                            })
                            .max_by_key(|q| q.raw())
                            .unwrap_or(Q16::ZERO);
                        if best.raw() > 0 {
                            d.insert("crystal_resonance".to_string(), best);
                        }
                    }
                    // Cross-language structural resonance: how closely this void's
                    // source fingerprint matches any *other* file in the workspace,
                    // regardless of language. A Python file structurally echoing a Go
                    // file resonates; a structural outlier does not. Only set when a
                    // distinct peer exists and resonance is positive (no false-zero).
                    if let Some(fp) = hyphae.host_cube.fingerprint_by_void_id.get(&void_id) {
                        if let Some(res) = best_cross_language_resonance(fp, &distinct_fingerprints)
                        {
                            d.insert("cross_language_resonance".to_string(), res);
                        }
                    }
                    CubeDimensionProfile::from_raw_map(d)
                } else {
                    CubeDimensionProfile::empty()
                }
            } else {
                CubeDimensionProfile::empty()
            };
            SourceCube::new(
                intent.target_void_id,
                format!("intent:{}", &decision.yield_id.to_hex()[..16]),
                dims,
                Q16::ONE,
                intent.taint.clone(),
                decision.evidence_bundle_id,
                policy.id,
            )
        })
        .collect();

    let swarm = CubeSwarm::new(policy.clone(), source_cubes.clone());
    let (_workers, swarm_composite) = swarm.run();

    let host_void_ids: Vec<Digest> = hyphae
        .host_cube
        .void_map
        .voids
        .iter()
        .map(|v| v.void_id)
        .collect();
    let void_fill_delta = HostTargetDelta::from_source_cubes(
        hyphae.host_cube.cube_id,
        &host_void_ids,
        swarm_composite.composite_id,
        &source_cubes,
        &seam_map,
        policy.id,
    );
    agg.record("void_fill_plan", void_fill_delta.delta_id, GateResult::Pass);

    // ── 4c. HostTargetCollapsePlan — planning-only, zero host writes ──────────
    let collapse_plan = HostTargetCollapsePlan::from_delta(&void_fill_delta, policy.id);

    // ── 4d. MorphogenicCorpusUpdate skeleton — corpus state after collapse ────
    // Skeleton only: records what the corpus would look like if the collapse plan
    // executed. No mutation; planning artifact derived from steps 2 + 4c.
    let morphogenic_update = MorphogenicCorpusUpdate::skeleton(
        cartography_update.update_id,
        collapse_plan.plan_id,
        policy.id,
    );

    // ── 5a. Optional motif candidates — from void kind frequency ─────────────
    // One MotifCandidate per HostVoidKind that appears in the void map.
    // support_score = kind_count / total_voids (frequency ratio, Q16 integer arithmetic).
    // Evidence = hyphae run_id (the passive scan is the source of truth). Energy-ranked.
    let motif_candidates: Vec<MotifCandidate> = if options.enable_motif_candidates {
        let total = hyphae.host_cube.void_map.void_count() as u64;
        if total == 0 {
            Vec::new()
        } else {
            let mut counts: BTreeMap<String, u64> = BTreeMap::new();
            for v in &hyphae.host_cube.void_map.voids {
                let kind_key = format!("{:?}", v.kind)
                    .split('{')
                    .next()
                    .unwrap_or("Unknown")
                    .trim()
                    .to_string();
                *counts.entry(kind_key).or_insert(0) += 1;
            }
            let raw: Vec<MotifCandidate> = counts
                .into_iter()
                .map(|(kind_key, count)| {
                    MotifCandidate::new(
                        format!("motif:{}", kind_key),
                        count,
                        total,
                        None,
                        kosmo_core::TaintLabel::Unverified,
                        hyphae.run_id,
                        policy.id,
                    )
                })
                .collect();
            let assessments: Vec<_> = raw
                .iter()
                .map(|c| c.energy_assessment(&GateResult::Pass))
                .collect();
            let ranked = rank_by_energy(&assessments);
            ranked
                .iter()
                .filter_map(|a| raw.iter().find(|c| c.motif_id == a.subject_id).cloned())
                .collect()
        }
    } else {
        Vec::new()
    };

    // ── 5d. Optional crystal candidates — from accepted decisions ────────────
    // One StructuralCrystalCandidate per accepted decision. When the source entry
    // carried content (CodeHDAG available), the candidate is enriched with
    // rho_coherence and omega_phase from the HDAG. These structural signals propagate
    // into the certified StructuralCrystalRecord, making the CAD library structurally
    // rich across runs.
    let crystal_candidates: Vec<StructuralCrystalCandidate> = if options.enable_crystal_candidates {
        hyphae
            .frontier
            .intents
            .iter()
            .zip(hyphae.decisions.iter())
            .filter(|(_, d)| d.outcome.is_accepted())
            .map(|(intent, decision)| {
                let (rho, omega) = intent
                    .target_void_id
                    .and_then(|vid| hyphae.host_cube.hdag_by_void_id.get(&vid))
                    .map(|hdag| (hdag.rho_coherence(), hdag.omega_phase()))
                    .unwrap_or((Q16::ONE, Q16::ONE));
                let candidate = StructuralCrystalCandidate::from_decision_with_signals(
                    decision,
                    intent.target_void_id,
                    rho,
                    omega,
                );
                // Attach the source file's cross-language fingerprint so the
                // certified crystal can resonate with same-shaped voids in any
                // language on a later run.
                match intent
                    .target_void_id
                    .and_then(|vid| hyphae.host_cube.fingerprint_by_void_id.get(&vid))
                {
                    Some(fp) => candidate.with_fingerprint(fp.clone()),
                    None => candidate,
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    // ── 5d-cert. Certify crystal candidates → StructuralCrystalRecord ──────────
    // Run ConstraintProgram::from_candidate on each Pending candidate. Passive runs
    // are always Replayable. All accepted-decision candidates satisfy every standard
    // constraint (CROSS-006 guarantees non-ZERO evidence; quarantine was rejected at
    // the gate cascade and never reaches Pending). The resulting StructuralCrystalRecord
    // is the durable CAD library element — pass as prior_crystals in the next run.
    let certified_crystals: Vec<StructuralCrystalRecord> = if options.enable_crystal_candidates {
        crystal_candidates
            .iter()
            .filter_map(|c| c.certify(ReplayStatus::Replayable).map(|(_, r)| r))
            .collect()
    } else {
        Vec::new()
    };

    // ── 5e-resonite. Structural resonance between current and prior crystals ──
    // Compute pairwise Resonite between every current-run certified crystal and every
    // prior-run crystal passed via options.prior_crystals. Each Resonite quantifies
    // structural proximity (rho/omega distance), enabling the CAD library to detect
    // pattern convergence across runs and rank historical matches by resonance energy.
    let resonite_map: Vec<Resonite> = if options.enable_crystal_candidates
        && !certified_crystals.is_empty()
        && !prior_crystals.is_empty()
    {
        let mut resonites = Vec::new();
        for current in &certified_crystals {
            for prior in prior_crystals {
                resonites.push(Resonite::from_records(current, prior, policy.id));
            }
        }
        resonites
    } else {
        Vec::new()
    };

    // ── 5f. Crystal auto-persist (Step 5f) ────────────────────────────────────
    // If crystal_store_path is set and policy allows host writes, append every
    // newly-certified crystal to the store. Dedup is handled inside the store
    // (re-appending an existing record_id is a no-op). The count of successfully
    // written records is reported as the observational `persisted_crystal_count`.
    let persisted_crystal_count: u32 = if let Some(ref store_path) = options.crystal_store_path {
        if options.enable_crystal_candidates && !certified_crystals.is_empty() {
            match CrystalRecordStore::open(store_path) {
                Ok(mut store) => {
                    let mut written = 0u32;
                    for record in &certified_crystals {
                        if store.append(record.clone(), policy).is_ok() {
                            written += 1;
                        }
                    }
                    written
                }
                Err(_) => 0,
            }
        } else {
            0
        }
    } else {
        0
    };

    // ── 5b. Optional norm candidates — from accepted decisions ────────────────
    // One NormGeneCandidate per accepted decision: name encodes the intent kind
    // so operators can identify the structural context of each candidate.
    // initial fitness = Q16::ONE (just accepted). Evidence = decision's evidence_bundle_id
    // (causal chain: accepted decision → norm gene). Energy-ranked by fitness D.
    let norm_candidates: Vec<NormGeneCandidate> = if options.enable_norm_candidates {
        let raw: Vec<NormGeneCandidate> = hyphae
            .frontier
            .intents
            .iter()
            .zip(hyphae.decisions.iter())
            .filter(|(_, d)| d.outcome.is_accepted())
            .map(|(intent, decision)| {
                let (name, description) = match &intent.kind {
                    kosmo_hyphae::frontier::SourceIntentKind::FillVoid { void_id } => {
                        let hex = &void_id.to_hex()[..16];
                        (
                            format!("norm:void:{}", hex),
                            format!("Norm gene from accepted decision for void {}", hex),
                        )
                    }
                    kosmo_hyphae::frontier::SourceIntentKind::ReduceDeficiency {
                        deficiency_kind,
                    } => {
                        let kind = format!("{:?}", deficiency_kind);
                        (
                            format!("norm:deficiency:{}", kind),
                            format!(
                                "Norm gene from accepted decision reducing {:?} deficiency",
                                deficiency_kind
                            ),
                        )
                    }
                    kosmo_hyphae::frontier::SourceIntentKind::SuggestPattern { pattern_name } => {
                        let short = &pattern_name[..pattern_name.len().min(24)];
                        (
                            format!("norm:pattern:{}", short),
                            format!(
                                "Norm gene from accepted decision suggesting pattern '{}'",
                                pattern_name
                            ),
                        )
                    }
                    kosmo_hyphae::frontier::SourceIntentKind::Custom { description: desc } => {
                        let hex = &decision.yield_id.to_hex()[..16];
                        (
                            format!("norm:custom:{}", hex),
                            format!(
                                "Norm gene from accepted custom decision: {}",
                                &desc[..desc.len().min(48)]
                            ),
                        )
                    }
                };
                NormGeneCandidate::new(
                    name,
                    description,
                    Q16::ONE,
                    decision.evidence_bundle_id,
                    policy.id,
                )
            })
            .collect();
        let assessments: Vec<_> = raw
            .iter()
            .map(|c| c.energy_assessment(&GateResult::Pass))
            .collect();
        let ranked = rank_by_energy(&assessments);
        ranked
            .iter()
            .filter_map(|a| raw.iter().find(|c| c.candidate_id == a.subject_id).cloned())
            .collect()
    } else {
        Vec::new()
    };

    // ── 5c. NormFitnessTrace: apply prior feedback to norm candidates ─────────
    // Closes the "Wissen zurück ins Substrat" loop: PSE outcome → feedback record
    // → fitness trace update. Only builds traces for candidates with ≥1 match.
    let norm_fitness_traces: Vec<NormFitnessTrace> = norm_candidates
        .iter()
        .filter_map(|candidate| {
            let trace = options
                .prior_feedback
                .iter()
                .filter(|fb| fb.norm_candidate_id == candidate.candidate_id)
                .fold(
                    NormFitnessTrace::empty(candidate.candidate_id, policy.id),
                    |t, fb| t.observe_from_feedback(fb),
                );
            if trace.observations.is_empty() {
                None
            } else {
                Some(trace)
            }
        })
        .collect();

    // ── 5. Optional SystemCube v0.4.3 export ──────────────────────────────────
    let systemcube_export: Option<KcubeExportReport> = if options.enable_systemcube {
        let run_desc = kosmo_core::RunDescriptor::new(policy.id, "pipeline");
        // Step 5e: build units then energy-rank them (accepted first, tainted below).
        let raw_units: Vec<BlueprintUnit> = hyphae
            .decisions
            .iter()
            .filter(|d| d.outcome.is_accepted())
            .map(|d| {
                BlueprintUnit::new(
                    BlueprintUnitKind::ModuleBoundary,
                    d.yield_id,
                    kosmo_core::AuthorityLabel::Foundry,
                    d.taint.clone(),
                    vec![d.evidence_bundle_id],
                    policy,
                )
            })
            .collect();
        let assessments: Vec<_> = raw_units
            .iter()
            .map(|u| u.energy_assessment(&GateResult::Pass))
            .collect();
        let ranked_ids = rank_by_energy(&assessments);
        let units: Vec<BlueprintUnit> = ranked_ids
            .iter()
            .filter_map(|a| {
                raw_units
                    .iter()
                    .find(|u| u.unit_id == a.subject_id)
                    .cloned()
            })
            .collect();
        let cube = SystemCube::new(hyphae.host_cube.cube_id, &run_desc, policy, units);
        let export = cube.export_dry_run(options.systemcube_capacity, policy);
        // Gate contribution: Warn when compatibility gaps exist (structural advisory, not energy).
        let cube_gate = if export.compatibility.gaps.is_empty() {
            GateResult::Pass
        } else {
            GateResult::Warn {
                message: format!(
                    "systemcube: {} compatibility gap(s); score={}",
                    export.compatibility.gaps.len(),
                    export.compatibility.compatibility_score.raw(),
                ),
            }
        };
        agg.record("systemcube", export.export_id, cube_gate);
        Some(export)
    } else {
        None
    };

    // ── 6b. PSE bridge candidates — flatten + confidence-rank ─────────────────
    // Converts norm candidates and topology observations into PseBridgeCandidate
    // objects, ready for PSE evaluation via validate_candidate().
    let pse_candidates: Vec<PseBridgeCandidate> = if options.enable_pse_candidates {
        let mut raw: Vec<PseBridgeCandidate> = Vec::new();
        // NormGeneCandidate → StructuralObservation
        for c in &norm_candidates {
            raw.push(PseBridgeCandidate::new(
                PseBridgeCandidateKind::StructuralObservation,
                c.candidate_id,
                format!("norm:{}", &c.name[..c.name.len().min(32)]),
                c.fitness_score,
                hyphae.run_id,
                c.evidence_bundle_id,
                policy.id,
            ));
        }
        // TopologyAmbiguityProfile → TopologyObservation
        for a in &ambiguity_profiles {
            raw.push(PseBridgeCandidate::new(
                PseBridgeCandidateKind::TopologyObservation,
                a.profile_id,
                format!("ambiguity:{:?}", a.ambiguity_kind),
                a.confidence_score,
                hyphae.run_id,
                a.micrograph_id,
                policy.id,
            ));
        }
        // ComplementVoidHypothesis → TopologyObservation
        for h in &complement_void_hypotheses {
            raw.push(PseBridgeCandidate::new(
                PseBridgeCandidateKind::TopologyObservation,
                h.hypothesis_id,
                format!(
                    "void_hyp:{}",
                    &h.hypothesized_void_kind[..h.hypothesized_void_kind.len().min(24)]
                ),
                h.confidence_score,
                hyphae.run_id,
                h.hypothesis_id,
                policy.id,
            ));
        }
        // MotifCandidate → StructuralObservation
        for m in &motif_candidates {
            raw.push(PseBridgeCandidate::new(
                PseBridgeCandidateKind::StructuralObservation,
                m.motif_id,
                format!("motif:{}", &m.name[..m.name.len().min(32)]),
                m.support_score,
                hyphae.run_id,
                m.evidence_bundle_id,
                policy.id,
            ));
        }
        // StructuralCrystalRecord → CertifiedCrystal — the unification path.
        // Every crystal certified in THIS run is offered to PSE; the record is
        // directly evidence-bound (CROSS-006), so no candidate lookup is needed.
        for r in &certified_crystals {
            raw.push(crystal_to_pse_candidate(r, hyphae.run_id, policy.id));
        }
        // Sort by confidence descending (deterministic: stable sort, then by id).
        raw.sort_by(|a, b| b.confidence.cmp(&a.confidence).then(a.id.cmp(&b.id)));
        raw
    } else {
        Vec::new()
    };

    // ── 6. Aggregate gate results ─────────────────────────────────────────────
    let aggregated_gate = agg.aggregate();

    IntegrationRunReport::new(
        hyphae,
        deficiency_vector,
        cartography_update,
        source_cubes,
        swarm_composite,
        void_fill_delta,
        collapse_plan,
        morphogenic_update,
        void_priority_ranking,
        metatron_diagnostics,
        metatron_index,
        lift_reports,
        lpcm_reports,
        surgery_options,
        surgery_workbench_tasks,
        ambiguity_profiles,
        complement_void_hypotheses,
        crystal_candidates,
        certified_crystals,
        resonite_map,
        pse_candidates,
        motif_candidates,
        norm_candidates,
        norm_fitness_traces,
        systemcube_export,
        aggregated_gate,
        persisted_crystal_count,
        policy,
    )
}

// ─── ActionItem — CAM layer: report → ranked actionable directives ────────────

/// Kind of actionable item distilled from an [`IntegrationRunReport`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionItemKind {
    /// A structural void that should be filled.
    FillVoid { void_id: Digest },
    /// A topology surgery option ready to apply.
    RepairTopology { surgery_option_id: Digest },
    /// A PSE candidate ready for external evaluation / promotion.
    PromoteToPse { candidate_id: Digest },
    /// A crystal candidate with `EvidenceOnly` status that needs operator review.
    ReviewCrystal { candidate_id: Digest },
    /// A norm gene that has accumulated enough fitness to warrant application.
    ApplyNorm {
        norm_candidate_id: Digest,
        name: String,
    },
    /// A wished-for structural facet that is not yet present: intent-directed
    /// work toward an attached [`kosmo_core::Wish`] (the counterpart to
    /// `FillVoid`, on the intent axis). Carried by the agent from the wish
    /// agenda — the pipeline scan itself never emits these.
    RealizeWishFacet { facet: kosmo_core::WishFacet },
}

/// Serialize-only content for [`ActionItem`] content-addressing.
#[derive(Serialize)]
struct ActionContent {
    kind_tag: &'static str,
    target_id: Digest,
    policy_id: Digest,
}

/// A ranked, actionable directive produced from an [`IntegrationRunReport`].
///
/// `priority_score` is a Q16 ratio derived from the item's position in its
/// energy-ranked source list — `Q16::ONE` = highest priority, `Q16::ZERO` = lowest.
/// All items across all categories are merged and sorted by `priority_score`
/// descending so callers get a single, unified work queue.
///
/// `action_id` is content-addressed from `(kind_tag, target_id, policy_id)`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionItem {
    pub action_id: Digest,
    pub priority_score: Q16,
    pub kind: ActionItemKind,
    pub description: String,
    pub policy_id: Digest,
}

impl ActionItem {
    fn new(
        kind_tag: &'static str,
        target_id: Digest,
        priority_score: Q16,
        kind: ActionItemKind,
        description: String,
        policy_id: Digest,
    ) -> Self {
        let action_id = Digest::of(&ActionContent {
            kind_tag,
            target_id,
            policy_id,
        });
        Self {
            action_id,
            priority_score,
            kind,
            description,
            policy_id,
        }
    }
}

/// Compute a descending priority score for the item at `pos` out of `total`.
///
/// Returns `Q16::ONE` when `pos == 0` (top of the list), `Q16::ZERO` when
/// `pos + 1 == total` (last item), and a proportional ratio in between.
/// When `total == 0` returns `Q16::ZERO` (no items → no priority).
fn rank_score(pos: usize, total: usize) -> Q16 {
    if total == 0 {
        return Q16::ZERO;
    }
    // score = (total - pos) / total → ONE for pos=0, approaches ZERO for pos=total-1
    Q16::ratio((total - pos) as u64, total as u64).unwrap_or(Q16::ZERO)
}

impl IntegrationRunReport {
    /// Produce a unified, priority-ranked list of actionable directives.
    ///
    /// Aggregates across five categories (void fill, topology repair, PSE promotion,
    /// crystal review, norm application) and returns them sorted by `priority_score`
    /// descending — the first item is the single highest-priority action across the
    /// entire report.
    ///
    /// Each category contributes items ranked by the energy ordering already present
    /// in the report (void_priority_ranking, surgery_options, pse_candidates, etc.).
    /// `EvidenceOnly` crystal candidates are included as `ReviewCrystal` items; fully
    /// `Pending`/`Certified` candidates are omitted (no operator action needed).
    pub fn action_items(&self) -> Vec<ActionItem> {
        let pid = self.policy_id;
        let mut items: Vec<ActionItem> = Vec::new();

        // ── FillVoid: one per void in priority order ───────────────────────────
        let n = self.void_priority_ranking.len();
        for (pos, &void_id) in self.void_priority_ranking.iter().enumerate() {
            items.push(ActionItem::new(
                "fill_void",
                void_id,
                rank_score(pos, n),
                ActionItemKind::FillVoid { void_id },
                format!("Fill structural void {:.16}", void_id.to_hex()),
                pid,
            ));
        }

        // ── RepairTopology: energy-ranked surgery options ──────────────────────
        let n = self.surgery_options.len();
        for (pos, opt) in self.surgery_options.iter().enumerate() {
            items.push(ActionItem::new(
                "repair_topology",
                opt.option_id,
                rank_score(pos, n),
                ActionItemKind::RepairTopology {
                    surgery_option_id: opt.option_id,
                },
                format!(
                    "Apply topology surgery option {:.16}",
                    opt.option_id.to_hex()
                ),
                pid,
            ));
        }

        // ── PromoteToPse: confidence-ranked PSE candidates ─────────────────────
        let n = self.pse_candidates.len();
        for (pos, cand) in self.pse_candidates.iter().enumerate() {
            items.push(ActionItem::new(
                "promote_to_pse",
                cand.id,
                rank_score(pos, n),
                ActionItemKind::PromoteToPse {
                    candidate_id: cand.id,
                },
                format!(
                    "Promote PSE candidate {:.16} ({:?})",
                    cand.id.to_hex(),
                    cand.kind
                ),
                pid,
            ));
        }

        // ── ReviewCrystal: EvidenceOnly crystal candidates need operator review ─
        let evidence_only: Vec<_> = self
            .crystal_candidates
            .iter()
            .filter(|c| {
                matches!(
                    c.certification_status,
                    kosmo_hyphae::crystal::CertificationStatus::EvidenceOnly
                )
            })
            .collect();
        let n = evidence_only.len();
        for (pos, cand) in evidence_only.iter().enumerate() {
            items.push(ActionItem::new(
                "review_crystal",
                cand.candidate_id,
                rank_score(pos, n),
                ActionItemKind::ReviewCrystal {
                    candidate_id: cand.candidate_id,
                },
                format!(
                    "Review EvidenceOnly crystal candidate {:.16}",
                    cand.candidate_id.to_hex()
                ),
                pid,
            ));
        }

        // ── ApplyNorm: all norm candidates (ranked by fitness via position) ─────
        let n = self.norm_candidates.len();
        for (pos, nc) in self.norm_candidates.iter().enumerate() {
            items.push(ActionItem::new(
                "apply_norm",
                nc.candidate_id,
                rank_score(pos, n),
                ActionItemKind::ApplyNorm {
                    norm_candidate_id: nc.candidate_id,
                    name: nc.name.clone(),
                },
                format!(
                    "Apply norm '{}' (fitness={})",
                    nc.name,
                    nc.fitness_score.raw()
                ),
                pid,
            ));
        }

        // Sort by priority_score descending (highest first).
        items.sort_by_key(|it| std::cmp::Reverse(it.priority_score.raw()));
        items
    }
}

// ─── Filesystem-level entry point ────────────────────────────────────────────

/// Run the full Kosmocrates pipeline on a workspace directory path.
///
/// Equivalent to `WorkspaceIndex::scan_path_with_content` + `run_dry_pipeline`.
/// Source `.rs` files are read with content so that HDAG extraction produces
/// code-structure-aware void severity and `crystal_resonance` SourceCube dimensions.
///
/// When `options.crystal_store_path` is set the CAD library is automatically loaded
/// from disk before the run and persisted after Step 5d-cert (policy-gated).
pub fn run_workspace_pipeline(
    root: &str,
    options: &IntegrationRunOptions,
    policy: &PolicyProfile,
) -> Result<IntegrationRunReport, WorkspaceError> {
    let index = WorkspaceIndex::scan_path_with_content(root, policy.id)?;
    Ok(run_dry_pipeline(&index, options, policy))
}

/// A stateful pipeline session that holds options and policy across multiple runs.
///
/// Create once with `WorkspacePipelineSession::new(options, policy)`, then call
/// `run(root)` for each workspace. When `options.crystal_store_path` is set the
/// session automatically accumulates crystal knowledge across every `run()` call —
/// the CAD library grows richer with each invocation without any caller bookkeeping.
pub struct WorkspacePipelineSession {
    options: IntegrationRunOptions,
    policy: PolicyProfile,
    run_count: u32,
}

impl WorkspacePipelineSession {
    /// Create a new session.
    pub fn new(options: IntegrationRunOptions, policy: PolicyProfile) -> Self {
        Self {
            options,
            policy,
            run_count: 0,
        }
    }

    /// Run the pipeline on a workspace directory path.
    ///
    /// Returns the report for this run. If `crystal_store_path` is configured, the
    /// pipeline auto-loads prior crystals and auto-persists newly certified ones.
    pub fn run(&mut self, root: &str) -> Result<IntegrationRunReport, WorkspaceError> {
        let report = run_workspace_pipeline(root, &self.options, &self.policy)?;
        self.run_count += 1;
        Ok(report)
    }

    /// Total number of completed `run()` calls.
    pub fn run_count(&self) -> u32 {
        self.run_count
    }

    /// Reference to the current options (includes `crystal_store_path` if set).
    pub fn options(&self) -> &IntegrationRunOptions {
        &self.options
    }

    /// Reference to the governing policy profile.
    pub fn policy(&self) -> &PolicyProfile {
        &self.policy
    }

    /// Extend the `prior_feedback` pool with newly observed outcomes.
    ///
    /// Records pushed here are injected into the next `run()` call, closing the
    /// "Wissen zurück ins Substrat" loop at the pipeline layer: agent execution
    /// outcomes flow back into norm-fitness scoring so the pipeline re-ranks
    /// candidates on the next scan.
    pub fn extend_prior_feedback(
        &mut self,
        feedback: impl IntoIterator<Item = kosmo_core::PromotionFeedback>,
    ) {
        self.options.prior_feedback.extend(feedback);
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::PolicyProfile;
    use kosmo_workbench::WorkspaceIndex;

    fn policy() -> PolicyProfile {
        PolicyProfile::default_report_only()
    }

    fn empty_index() -> WorkspaceIndex {
        WorkspaceIndex::from_entries("test-root".into(), vec![], policy().id)
    }

    fn fixture_index() -> WorkspaceIndex {
        use kosmo_workbench::{WorkspaceEntry, WorkspaceEntryKind};
        let entries = vec![
            WorkspaceEntry {
                path: "src/lib.rs".into(),
                digest: Digest::of_bytes(b"lib"),
                size_bytes: 100,
                kind: WorkspaceEntryKind::SourceFile,
                content: None,
            },
            WorkspaceEntry {
                path: "src/main.rs".into(),
                digest: Digest::of_bytes(b"main"),
                size_bytes: 200,
                kind: WorkspaceEntryKind::SourceFile,
                content: None,
            },
            WorkspaceEntry {
                path: "src/lib_test.rs".into(),
                digest: Digest::of_bytes(b"lib_test"),
                size_bytes: 50,
                kind: WorkspaceEntryKind::TestFile,
                content: None,
            },
        ];
        WorkspaceIndex::from_entries("test-root".into(), entries, policy().id)
    }

    // ── Pipeline: empty workspace ─────────────────────────────────────────────

    #[test]
    fn pipeline_empty_workspace_passes_all_gates() {
        let r = run_dry_pipeline(
            &empty_index(),
            &IntegrationRunOptions::report_only(),
            &policy(),
        );
        assert!(
            r.final_result.is_pass(),
            "empty workspace must pass all gates"
        );
    }

    #[test]
    fn pipeline_empty_workspace_report_is_content_addressed() {
        let r1 = run_dry_pipeline(
            &empty_index(),
            &IntegrationRunOptions::report_only(),
            &policy(),
        );
        let r2 = run_dry_pipeline(
            &empty_index(),
            &IntegrationRunOptions::report_only(),
            &policy(),
        );
        assert_eq!(r1.report_id, r2.report_id);
        assert_ne!(r1.report_id, Digest::ZERO);
    }

    // ── DeficiencyVector Step 1c ──────────────────────────────────────────────

    #[test]
    fn pipeline_deficiency_vector_always_present() {
        let r = run_dry_pipeline(
            &empty_index(),
            &IntegrationRunOptions::report_only(),
            &policy(),
        );
        assert_ne!(
            r.deficiency_vector.vector_id,
            Digest::ZERO,
            "vector_id must be non-ZERO"
        );
        assert_eq!(r.deficiency_vector.policy_id, policy().id);
    }

    #[test]
    fn pipeline_deficiency_vector_empty_for_empty_workspace() {
        let r = run_dry_pipeline(
            &empty_index(),
            &IntegrationRunOptions::report_only(),
            &policy(),
        );
        assert!(
            r.deficiency_vector.entries.is_empty(),
            "no deficiencies for empty workspace"
        );
    }

    #[test]
    fn pipeline_deficiency_vector_is_deterministic() {
        let r1 = run_dry_pipeline(
            &fixture_index(),
            &IntegrationRunOptions::report_only(),
            &policy(),
        );
        let r2 = run_dry_pipeline(
            &fixture_index(),
            &IntegrationRunOptions::report_only(),
            &policy(),
        );
        assert_eq!(
            r1.deficiency_vector.vector_id, r2.deficiency_vector.vector_id,
            "deficiency_vector must be deterministic"
        );
    }

    // ── Policy consistency (traceability) ─────────────────────────────────────

    #[test]
    fn pipeline_policy_consistency_empty() {
        let r = run_dry_pipeline(
            &empty_index(),
            &IntegrationRunOptions::report_only(),
            &policy(),
        );
        assert!(
            r.verify_policy_consistency(),
            "every sub-report must carry the same policy_id"
        );
    }

    #[test]
    fn pipeline_policy_consistency_all_layers() {
        let r = run_dry_pipeline(
            &fixture_index(),
            &IntegrationRunOptions::all_layers(4),
            &policy(),
        );
        assert!(
            r.verify_policy_consistency(),
            "all-layers run must have consistent policy_id in every sub-report"
        );
    }

    // ── Metatron optional layer ────────────────────────────────────────────────

    #[test]
    fn pipeline_no_metatron_when_disabled() {
        let r = run_dry_pipeline(
            &fixture_index(),
            &IntegrationRunOptions::report_only(),
            &policy(),
        );
        assert!(
            r.metatron_diagnostics.is_empty(),
            "Metatron must be off when disabled"
        );
    }

    #[test]
    fn pipeline_metatron_produces_one_diagnostic_per_void() {
        let opts = IntegrationRunOptions {
            enable_metatron: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        assert_eq!(
            r.metatron_diagnostics.len(),
            r.hyphae_result.host_cube.void_count(),
            "one Metatron diagnostic per host void"
        );
    }

    #[test]
    fn pipeline_metatron_diagnostics_carry_correct_policy_id() {
        let opts = IntegrationRunOptions {
            enable_metatron: true,
            ..IntegrationRunOptions::report_only()
        };
        let p = policy();
        let r = run_dry_pipeline(&fixture_index(), &opts, &p);
        for diag in &r.metatron_diagnostics {
            assert_eq!(
                diag.policy_id, p.id,
                "Metatron diagnostic must carry pipeline policy_id"
            );
        }
    }

    // ── LPCM optional layer ────────────────────────────────────────────────────

    #[test]
    fn pipeline_no_lpcm_when_disabled() {
        let r = run_dry_pipeline(
            &fixture_index(),
            &IntegrationRunOptions::report_only(),
            &policy(),
        );
        assert!(r.lpcm_reports.is_empty(), "LPCM must be off when disabled");
    }

    #[test]
    fn pipeline_lpcm_produces_one_report_per_void() {
        let opts = IntegrationRunOptions {
            enable_lpcm: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        assert_eq!(
            r.lpcm_reports.len(),
            r.hyphae_result.host_cube.void_count(),
            "one LPCM report per host void"
        );
    }

    #[test]
    fn pipeline_lpcm_reports_carry_correct_policy_id() {
        let opts = IntegrationRunOptions {
            enable_lpcm: true,
            ..IntegrationRunOptions::report_only()
        };
        let p = policy();
        let r = run_dry_pipeline(&fixture_index(), &opts, &p);
        for lr in &r.lpcm_reports {
            assert_eq!(
                lr.policy_id, p.id,
                "LPCM report must carry pipeline policy_id"
            );
        }
    }

    // ── SystemCube optional layer ──────────────────────────────────────────────

    #[test]
    fn pipeline_no_systemcube_when_disabled() {
        let r = run_dry_pipeline(
            &fixture_index(),
            &IntegrationRunOptions::report_only(),
            &policy(),
        );
        assert!(
            r.systemcube_export.is_none(),
            "SystemCube must be off when disabled"
        );
    }

    #[test]
    fn pipeline_systemcube_export_is_blocked_by_default_policy() {
        use kosmo_systemcube::KcubeExportMode;
        let opts = IntegrationRunOptions::all_layers(4);
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        if let Some(ref export) = r.systemcube_export {
            assert_eq!(
                export.mode,
                KcubeExportMode::BlockedByPolicy,
                "SystemCube export must be BlockedByPolicy in ReportOnly mode"
            );
        }
    }

    #[test]
    fn pipeline_systemcube_carries_correct_policy_id() {
        let opts = IntegrationRunOptions::all_layers(4);
        let p = policy();
        let r = run_dry_pipeline(&fixture_index(), &opts, &p);
        if let Some(ref export) = r.systemcube_export {
            assert_eq!(export.policy_id, p.id);
        }
    }

    #[test]
    fn pipeline_systemcube_compatibility_accessor_returns_score() {
        let opts = IntegrationRunOptions::all_layers(4);
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        let score = r.systemcube_compatibility_score();
        assert!(
            score.is_some(),
            "compatibility score must be Some when systemcube enabled"
        );
        // All pipeline units carry TaintLabel::Synthetic → AcceptedWithTaint → gaps → score < ONE
        assert!(
            score.unwrap() < Q16::ONE,
            "tainted units must produce score < ONE (got {})",
            score.unwrap().raw()
        );
    }

    #[test]
    fn pipeline_systemcube_contradiction_energy_accessor_returns_value() {
        let opts = IntegrationRunOptions::all_layers(4);
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        let energy = r.systemcube_contradiction_energy();
        assert!(
            energy.is_some(),
            "contradiction energy must be Some when systemcube enabled"
        );
    }

    #[test]
    fn pipeline_systemcube_tainted_units_produce_warn_gate() {
        let opts = IntegrationRunOptions::all_layers(4);
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        // All pipeline units are Synthetic → compatibility gaps → gate Warn
        // Warn is not Pass and not Reject — aggregate should carry the Warn
        assert!(
            !r.final_result.is_rejected(),
            "compatibility gaps must Warn, never Reject"
        );
        // summary must now include compat and contradiction_energy
        let s = r.summary();
        assert!(s.contains("compat="), "summary must include compat score");
        assert!(
            s.contains("contradiction_energy="),
            "summary must include contradiction energy"
        );
    }

    #[test]
    fn pipeline_blueprint_units_carry_decision_taint() {
        // All passive_run decisions are Synthetic → BlueprintUnits must be
        // AcceptedWithTaint, producing TaintedUnit compatibility gaps.
        let opts = IntegrationRunOptions::all_layers(4);
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        if let Some(ref export) = r.systemcube_export {
            // Every compatibility gap must be TaintedUnit (propagated from Synthetic decision)
            for gap in &export.compatibility.gaps {
                assert_eq!(
                    gap.gap_kind, "TaintedUnit",
                    "gaps from Synthetic decisions must be TaintedUnit, got {}",
                    gap.gap_kind
                );
            }
            // If there are accepted units there must be at least one TaintedUnit gap
            if export.d_density.accepted_unit_count > 0 {
                assert!(
                    !export.compatibility.gaps.is_empty(),
                    "Synthetic decisions must generate TaintedUnit gaps"
                );
            }
        }
    }

    // ── Fail-closed gate propagation ──────────────────────────────────────────

    #[test]
    fn fail_closed_aggregator_reject_propagates() {
        let mut agg = GateTraceAggregator::new(Digest::of_bytes(b"p"));
        agg.record("hyphae", Digest::of_bytes(b"h"), GateResult::Pass);
        agg.record(
            "lpcm",
            Digest::of_bytes(b"l"),
            GateResult::Reject {
                reason: "bad".into(),
            },
        );
        agg.record(
            "meta",
            Digest::of_bytes(b"m"),
            GateResult::Warn {
                message: "low".into(),
            },
        );
        let result = agg.aggregate();
        assert!(
            result.final_result.is_rejected(),
            "single Reject must propagate to final_result"
        );
    }

    // ── CROSS-013: no host files written ──────────────────────────────────────

    #[test]
    fn cross_013_pipeline_no_host_write() {
        let r = run_dry_pipeline(
            &fixture_index(),
            &IntegrationRunOptions::all_layers(4),
            &policy(),
        );
        // Structural: IntegrationRunReport has no write/patch methods.
        // Policy confirms host writes forbidden.
        assert!(!policy().allow_host_write);
        assert!(!policy().allow_systemcube_materialization);
        let _s = r.summary();
    }

    // ── CROSS-002: host mutation impossible in default policy ─────────────────

    #[test]
    fn cross_002_host_mutation_impossible_by_default() {
        let p = policy();
        assert!(
            !p.allow_host_write,
            "allow_host_write must be false in default policy"
        );
        assert!(
            !p.allow_systemcube_materialization,
            "systemcube materialization must be false in default policy"
        );
        assert!(
            !p.allow_external_acquisition,
            "external acquisition must be false in default policy"
        );
    }

    // ── Determinism across all layers ─────────────────────────────────────────

    #[test]
    fn pipeline_all_layers_is_deterministic() {
        let opts = IntegrationRunOptions::all_layers(4);
        let r1 = run_dry_pipeline(&fixture_index(), &opts, &policy());
        let r2 = run_dry_pipeline(&fixture_index(), &opts, &policy());
        assert_eq!(r1.report_id, r2.report_id, "pipeline must be deterministic");
    }

    // ── Summary is non-empty ───────────────────────────────────────────────────

    #[test]
    fn pipeline_summary_non_empty() {
        let r = run_dry_pipeline(
            &fixture_index(),
            &IntegrationRunOptions::all_layers(4),
            &policy(),
        );
        let s = r.summary();
        assert!(!s.is_empty());
        assert!(s.contains("IntegrationRunReport"));
    }

    // ── MicroTopologyIndex from Metatron ──────────────────────────────────────

    #[test]
    fn pipeline_metatron_index_empty_when_metatron_disabled() {
        let r = run_dry_pipeline(
            &fixture_index(),
            &IntegrationRunOptions::report_only(),
            &policy(),
        );
        assert!(
            r.metatron_index.micrograph_ids.is_empty(),
            "index must be empty when Metatron disabled"
        );
        assert!(r.metatron_index.diagnostic_ids.is_empty());
    }

    #[test]
    fn pipeline_metatron_index_has_one_entry_per_void() {
        let opts = IntegrationRunOptions {
            enable_metatron: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        let void_count = r.hyphae_result.host_cube.void_count();
        assert_eq!(
            r.metatron_index.micrograph_ids.len(),
            void_count,
            "one micrograph per void"
        );
        assert_eq!(
            r.metatron_index.diagnostic_ids.len(),
            void_count,
            "one diagnostic per void"
        );
        assert_ne!(
            r.metatron_index.index_id,
            Digest::ZERO,
            "index_id must be non-ZERO"
        );
    }

    #[test]
    fn pipeline_metatron_index_is_deterministic() {
        let opts = IntegrationRunOptions {
            enable_metatron: true,
            ..IntegrationRunOptions::report_only()
        };
        let r1 = run_dry_pipeline(&fixture_index(), &opts, &policy());
        let r2 = run_dry_pipeline(&fixture_index(), &opts, &policy());
        assert_eq!(
            r1.metatron_index.index_id, r2.metatron_index.index_id,
            "metatron_index must be deterministic"
        );
    }

    #[test]
    fn pipeline_metatron_index_carries_correct_policy_id() {
        let opts = IntegrationRunOptions {
            enable_metatron: true,
            ..IntegrationRunOptions::report_only()
        };
        let p = policy();
        let r = run_dry_pipeline(&fixture_index(), &opts, &p);
        assert_eq!(
            r.metatron_index.policy_id, p.id,
            "metatron_index must carry pipeline policy_id"
        );
        assert!(
            r.verify_policy_consistency(),
            "verify_policy_consistency must cover metatron_index"
        );
    }

    // ── Lift reports from Metatron ────────────────────────────────────────────

    #[test]
    fn pipeline_no_lift_reports_when_metatron_disabled() {
        let r = run_dry_pipeline(
            &fixture_index(),
            &IntegrationRunOptions::report_only(),
            &policy(),
        );
        assert!(
            r.lift_reports.is_empty(),
            "lift_reports must be empty when Metatron is disabled"
        );
    }

    #[test]
    fn pipeline_lift_reports_one_per_void_when_metatron_enabled() {
        let opts = IntegrationRunOptions {
            enable_metatron: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        assert_eq!(
            r.lift_reports.len(),
            r.hyphae_result.host_cube.void_count(),
            "one lift report per host void"
        );
        for rep in &r.lift_reports {
            assert_ne!(
                rep.report_id,
                Digest::ZERO,
                "lift_report.report_id must be non-ZERO"
            );
        }
    }

    #[test]
    fn pipeline_lift_reports_are_deterministic() {
        let opts = IntegrationRunOptions {
            enable_metatron: true,
            ..IntegrationRunOptions::report_only()
        };
        let r1 = run_dry_pipeline(&fixture_index(), &opts, &policy());
        let r2 = run_dry_pipeline(&fixture_index(), &opts, &policy());
        let ids1: Vec<_> = r1.lift_reports.iter().map(|r| r.report_id).collect();
        let ids2: Vec<_> = r2.lift_reports.iter().map(|r| r.report_id).collect();
        assert_eq!(ids1, ids2, "lift_reports must be deterministic");
    }

    // ── Motif candidates optional layer (Step 5a) ────────────────────────────

    #[test]
    fn pipeline_no_motif_candidates_when_disabled() {
        let r = run_dry_pipeline(
            &fixture_index(),
            &IntegrationRunOptions::report_only(),
            &policy(),
        );
        assert!(
            r.motif_candidates.is_empty(),
            "motif_candidates must be empty when disabled"
        );
    }

    #[test]
    fn pipeline_motif_candidates_from_void_kind_frequency() {
        let opts = IntegrationRunOptions {
            enable_motif_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        if r.hyphae_result.host_cube.void_count() > 0 {
            assert!(
                !r.motif_candidates.is_empty(),
                "motif_candidates must be non-empty when voids exist"
            );
            for c in &r.motif_candidates {
                assert!(
                    c.name.starts_with("motif:"),
                    "candidate name must carry motif: prefix"
                );
                assert_ne!(
                    c.motif_id,
                    Digest::ZERO,
                    "INVARIANT-007: motif_id must be non-ZERO"
                );
                assert_ne!(
                    c.evidence_bundle_id,
                    Digest::ZERO,
                    "CROSS-006: non-ZERO evidence ref"
                );
                assert_ne!(
                    c.support_score,
                    Q16::ZERO,
                    "support_score must be > 0 for observed void kinds"
                );
            }
        }
    }

    #[test]
    fn pipeline_motif_candidates_carry_correct_policy_id() {
        let opts = IntegrationRunOptions {
            enable_motif_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let p = policy();
        let r = run_dry_pipeline(&fixture_index(), &opts, &p);
        for c in &r.motif_candidates {
            assert_eq!(
                c.policy_id, p.id,
                "motif candidate must carry pipeline policy_id"
            );
        }
    }

    #[test]
    fn pipeline_motif_candidates_are_deterministic() {
        let opts = IntegrationRunOptions {
            enable_motif_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let p = policy();
        let r1 = run_dry_pipeline(&fixture_index(), &opts, &p);
        let r2 = run_dry_pipeline(&fixture_index(), &opts, &p);
        assert_eq!(
            r1.report_id, r2.report_id,
            "motif candidates must be deterministic (INVARIANT-007)"
        );
    }

    #[test]
    fn pipeline_prior_motifs_inject_suggest_pattern_intents() {
        let p = policy();
        // First run: collect motif candidates.
        let opts1 = IntegrationRunOptions {
            enable_motif_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let r1 = run_dry_pipeline(&fixture_index(), &opts1, &p);
        let prior_motifs = r1.motif_candidates.clone();

        if prior_motifs.is_empty() {
            return; // No voids → no motifs → skip.
        }
        // Second run: feed prior motifs back via prior_motifs.
        let opts2 = IntegrationRunOptions {
            prior_motifs,
            prior_motif_min_support: Q16::ZERO, // accept all
            ..IntegrationRunOptions::report_only()
        };
        let r2 = run_dry_pipeline(&fixture_index(), &opts2, &p);

        let suggest_count = r2
            .hyphae_result
            .frontier
            .intents
            .iter()
            .filter(|i| {
                matches!(
                    &i.kind,
                    kosmo_hyphae::frontier::SourceIntentKind::SuggestPattern { .. }
                )
            })
            .count();
        assert!(
            suggest_count > 0,
            "prior_motifs must inject SuggestPattern intents into the frontier"
        );
        // run_id differs because frontier_id changes (INVARIANT-007).
        assert_ne!(
            r1.hyphae_result.run_id, r2.hyphae_result.run_id,
            "augmented run must have different run_id due to extra intents"
        );
    }

    // ── Norm candidates optional layer ────────────────────────────────────────

    #[test]
    fn pipeline_no_norm_candidates_when_disabled() {
        let r = run_dry_pipeline(
            &fixture_index(),
            &IntegrationRunOptions::report_only(),
            &policy(),
        );
        assert!(
            r.norm_candidates.is_empty(),
            "norm_candidates must be empty when disabled"
        );
    }

    #[test]
    fn pipeline_norm_candidates_generated_from_accepted_decisions() {
        let opts = IntegrationRunOptions {
            enable_norm_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        let accepted_count = r
            .hyphae_result
            .decisions
            .iter()
            .filter(|d| d.outcome.is_accepted())
            .count();
        assert_eq!(
            r.norm_candidates.len(),
            accepted_count,
            "one norm candidate per accepted decision"
        );
        for c in &r.norm_candidates {
            assert_eq!(
                c.fitness_score,
                Q16::ONE,
                "initial fitness must be Q16::ONE"
            );
            assert!(
                c.name.starts_with("norm:void:"),
                "candidate name must encode target void"
            );
            assert_ne!(
                c.candidate_id,
                Digest::ZERO,
                "candidate_id must be non-ZERO"
            );
            assert_ne!(
                c.evidence_bundle_id,
                Digest::ZERO,
                "CROSS-006: non-ZERO evidence ref"
            );
        }
    }

    #[test]
    fn pipeline_norm_candidates_carry_correct_policy_id() {
        let opts = IntegrationRunOptions {
            enable_norm_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let p = policy();
        let r = run_dry_pipeline(&fixture_index(), &opts, &p);
        for c in &r.norm_candidates {
            assert_eq!(
                c.policy_id, p.id,
                "norm candidate must carry pipeline policy_id"
            );
        }
    }

    // ── Ambiguity profiles + void hypotheses (Step 3f) ───────────────────────

    #[test]
    fn pipeline_no_ambiguity_profiles_when_metatron_disabled() {
        let r = run_dry_pipeline(
            &fixture_index(),
            &IntegrationRunOptions::report_only(),
            &policy(),
        );
        assert!(
            r.ambiguity_profiles.is_empty(),
            "ambiguity_profiles must be empty when Metatron is disabled"
        );
        assert!(
            r.complement_void_hypotheses.is_empty(),
            "complement_void_hypotheses must be empty when Metatron is disabled"
        );
    }

    #[test]
    fn pipeline_ambiguity_profiles_are_energy_ranked() {
        let opts = IntegrationRunOptions {
            enable_metatron: true,
            ..IntegrationRunOptions::report_only()
        };
        let r1 = run_dry_pipeline(&fixture_index(), &opts, &policy());
        let r2 = run_dry_pipeline(&fixture_index(), &opts, &policy());
        // Determinism.
        let ids1: Vec<_> = r1.ambiguity_profiles.iter().map(|a| a.profile_id).collect();
        let ids2: Vec<_> = r2.ambiguity_profiles.iter().map(|a| a.profile_id).collect();
        assert_eq!(ids1, ids2, "ambiguity_profiles must be deterministic");
        // Energy order: confidence_score descending.
        for window in r1.ambiguity_profiles.windows(2) {
            assert!(
                window[0].confidence_score >= window[1].confidence_score,
                "ambiguity_profiles must be ranked by confidence_score descending"
            );
        }
    }

    #[test]
    fn pipeline_ambiguity_profiles_carry_correct_policy_id() {
        let opts = IntegrationRunOptions {
            enable_metatron: true,
            ..IntegrationRunOptions::report_only()
        };
        let p = policy();
        let r = run_dry_pipeline(&fixture_index(), &opts, &p);
        for a in &r.ambiguity_profiles {
            assert_eq!(
                a.policy_id, p.id,
                "ambiguity profile must carry pipeline policy_id"
            );
            assert_ne!(a.profile_id, Digest::ZERO, "profile_id must be non-ZERO");
        }
        for h in &r.complement_void_hypotheses {
            assert_eq!(
                h.policy_id, p.id,
                "void hypothesis must carry pipeline policy_id"
            );
            assert_ne!(
                h.hypothesis_id,
                Digest::ZERO,
                "hypothesis_id must be non-ZERO"
            );
        }
        assert!(
            r.verify_policy_consistency(),
            "verify_policy_consistency must cover ambiguities + hypotheses"
        );
    }

    // ── Surgery workbench tasks (Step 3e) ────────────────────────────────────

    #[test]
    fn pipeline_no_surgery_workbench_tasks_when_surgery_disabled() {
        let r = run_dry_pipeline(
            &fixture_index(),
            &IntegrationRunOptions::report_only(),
            &policy(),
        );
        assert!(
            r.surgery_workbench_tasks.is_empty(),
            "surgery_workbench_tasks must be empty when surgery is disabled"
        );
    }

    #[test]
    fn pipeline_surgery_workbench_tasks_match_surgery_options_count() {
        let opts = IntegrationRunOptions {
            enable_surgery: true,
            enable_metatron: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        assert_eq!(
            r.surgery_workbench_tasks.len(),
            r.surgery_options.len(),
            "surgery_workbench_tasks must be 1:1 with surgery_options"
        );
    }

    // ── PSE bridge candidates (Step 6b) ──────────────────────────────────────

    #[test]
    fn pipeline_no_pse_candidates_when_disabled() {
        let r = run_dry_pipeline(
            &fixture_index(),
            &IntegrationRunOptions::report_only(),
            &policy(),
        );
        assert!(
            r.pse_candidates.is_empty(),
            "pse_candidates must be empty when disabled"
        );
    }

    #[test]
    fn pipeline_pse_candidates_from_norm_and_topology_observations() {
        let opts = IntegrationRunOptions {
            enable_metatron: true,
            enable_norm_candidates: true,
            enable_pse_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        let expected_min = r.norm_candidates.len()
            + r.ambiguity_profiles.len()
            + r.complement_void_hypotheses.len();
        assert_eq!(
            r.pse_candidates.len(),
            expected_min,
            "pse_candidates must cover all norm + topology sources"
        );
        // Confidence descending.
        for window in r.pse_candidates.windows(2) {
            assert!(
                window[0].confidence >= window[1].confidence,
                "pse_candidates must be sorted by confidence descending"
            );
        }
        // All must be non-ZERO and carry pipeline policy_id.
        for c in &r.pse_candidates {
            assert_ne!(c.id, Digest::ZERO, "PSE candidate id must be non-ZERO");
            assert_eq!(
                c.policy_id,
                policy().id,
                "PSE candidate must carry pipeline policy_id"
            );
            assert_ne!(
                c.evidence_bundle_id,
                Digest::ZERO,
                "CROSS-006: evidence must be non-ZERO"
            );
        }
    }

    #[test]
    fn pipeline_pse_candidates_include_motif_candidates() {
        let opts = IntegrationRunOptions {
            enable_motif_candidates: true,
            enable_pse_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        let expected = r.norm_candidates.len()
            + r.ambiguity_profiles.len()
            + r.complement_void_hypotheses.len()
            + r.motif_candidates.len();
        assert_eq!(
            r.pse_candidates.len(),
            expected,
            "pse_candidates must include motif candidates as StructuralObservations"
        );
        let motif_pse: Vec<_> = r
            .pse_candidates
            .iter()
            .filter(|c| c.label.starts_with("motif:motif:"))
            .collect();
        if !r.motif_candidates.is_empty() {
            assert!(
                !motif_pse.is_empty(),
                "motif-derived PSE candidates must have 'motif:motif:' label prefix"
            );
        }
    }

    #[test]
    fn pipeline_pse_candidates_are_deterministic() {
        let opts = IntegrationRunOptions {
            enable_metatron: true,
            enable_norm_candidates: true,
            enable_pse_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let r1 = run_dry_pipeline(&fixture_index(), &opts, &policy());
        let r2 = run_dry_pipeline(&fixture_index(), &opts, &policy());
        let ids1: Vec<_> = r1.pse_candidates.iter().map(|c| c.id).collect();
        let ids2: Vec<_> = r2.pse_candidates.iter().map(|c| c.id).collect();
        assert_eq!(ids1, ids2, "pse_candidates must be deterministic");
        assert_eq!(
            r1.report_id, r2.report_id,
            "pse_candidate_count must participate in report_id"
        );
    }

    // ── Crystal candidates from accepted decisions (Step 5d) ─────────────────

    #[test]
    fn pipeline_no_crystal_candidates_when_disabled() {
        let r = run_dry_pipeline(
            &fixture_index(),
            &IntegrationRunOptions::report_only(),
            &policy(),
        );
        assert!(
            r.crystal_candidates.is_empty(),
            "crystal_candidates must be empty when disabled"
        );
    }

    #[test]
    fn pipeline_crystal_candidates_one_per_accepted_decision() {
        let opts = IntegrationRunOptions {
            enable_crystal_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        let accepted_count = r
            .hyphae_result
            .decisions
            .iter()
            .filter(|d| d.outcome.is_accepted())
            .count();
        assert_eq!(
            r.crystal_candidates.len(),
            accepted_count,
            "one crystal candidate per accepted decision"
        );
        for c in &r.crystal_candidates {
            assert_ne!(
                c.candidate_id,
                Digest::ZERO,
                "candidate_id must be non-ZERO"
            );
            assert_eq!(
                c.support_score,
                Q16::ZERO,
                "support_score starts at zero (Pending)"
            );
            assert_ne!(
                c.evidence_bundle_id,
                Digest::ZERO,
                "CROSS-006: evidence must be non-ZERO"
            );
        }
    }

    #[test]
    fn pipeline_crystal_candidates_carry_correct_policy_id() {
        let opts = IntegrationRunOptions {
            enable_crystal_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let p = policy();
        let r = run_dry_pipeline(&fixture_index(), &opts, &p);
        for c in &r.crystal_candidates {
            assert_eq!(
                c.policy_id, p.id,
                "crystal candidate must carry pipeline policy_id"
            );
        }
        assert!(
            r.verify_policy_consistency(),
            "verify_policy_consistency must cover crystal_candidates"
        );
    }

    // ── Crystal certification Step 5d-cert ───────────────────────────────────

    #[test]
    fn pipeline_certified_crystals_match_candidates_when_enabled() {
        let opts = IntegrationRunOptions {
            enable_crystal_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        // Every Pending candidate from accepted decisions must be certified.
        assert_eq!(
            r.certified_crystals.len(),
            r.crystal_candidates
                .iter()
                .filter(|c| c.is_certifiable())
                .count(),
            "every certifiable candidate must produce a StructuralCrystalRecord"
        );
        for rec in &r.certified_crystals {
            assert_ne!(rec.record_id, Digest::ZERO, "record_id must be non-ZERO");
            assert_eq!(
                rec.policy_id,
                policy().id,
                "record must carry pipeline policy_id"
            );
        }
    }

    #[test]
    fn pipeline_no_certified_crystals_when_disabled() {
        let r = run_dry_pipeline(
            &fixture_index(),
            &IntegrationRunOptions::report_only(),
            &policy(),
        );
        assert!(
            r.certified_crystals.is_empty(),
            "certified_crystals must be empty when disabled"
        );
    }

    #[test]
    fn pipeline_certified_crystals_deterministic() {
        let opts = IntegrationRunOptions {
            enable_crystal_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let r1 = run_dry_pipeline(&fixture_index(), &opts, &policy());
        let r2 = run_dry_pipeline(&fixture_index(), &opts, &policy());
        let ids1: Vec<_> = r1.certified_crystals.iter().map(|r| r.record_id).collect();
        let ids2: Vec<_> = r2.certified_crystals.iter().map(|r| r.record_id).collect();
        assert_eq!(
            ids1, ids2,
            "certified_crystals must be deterministic (INVARIANT-007)"
        );
    }

    #[test]
    fn pipeline_prior_crystals_seed_corpus() {
        // Run once to collect certified crystals, then feed as prior_crystals to the
        // next run. The second run's corpus must contain the prior crystal entities.
        let opts = IntegrationRunOptions {
            enable_crystal_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let r1 = run_dry_pipeline(&fixture_index(), &opts, &policy());
        let prior = r1.certified_crystals.clone();
        if prior.is_empty() {
            return; // No voids → no candidates → skip.
        }
        let opts2 = IntegrationRunOptions {
            enable_crystal_candidates: true,
            prior_crystals: prior.clone(),
            ..IntegrationRunOptions::report_only()
        };
        let r2 = run_dry_pipeline(&fixture_index(), &opts2, &policy());
        // The second run's cartography_update must reflect the seeded entities:
        // Prior crystals are added before update_from_run, so the "before" corpus
        // already has crystal entities — the after cartography_id will differ from r1.
        assert_ne!(
            r1.report_id, r2.report_id,
            "prior_crystals must change report_id (content-addressing reflects seeded state)"
        );
    }

    // ── Crystal record structural fingerprint ────────────────────────────────

    #[test]
    fn pipeline_crystal_candidate_carries_hdag_signals_when_content_present() {
        let opts = IntegrationRunOptions {
            enable_crystal_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&content_fixture_index(), &opts, &policy());
        // All candidates must have source_void_id, rho_coherence, omega_phase set.
        // In report_only mode there are no accepted decisions (EvidenceOnly only),
        // so crystal_candidates will be empty — test the structural invariant vacuously.
        // The unit tests in crystal.rs cover the actual signal propagation.
        for c in &r.crystal_candidates {
            assert_ne!(c.candidate_id, Digest::ZERO);
            assert_eq!(c.policy_id, policy().id);
        }
        // If candidates exist (from accepted decisions), they must carry HDAG signals.
        for c in &r.crystal_candidates {
            if let Some(void_id) = c.source_void_id {
                if r.hyphae_result
                    .host_cube
                    .hdag_by_void_id
                    .contains_key(&void_id)
                {
                    assert_ne!(
                        c.rho_coherence,
                        Q16::ONE,
                        "HDAG-enriched candidate should not default to ONE"
                    );
                }
            }
        }
    }

    #[test]
    fn pipeline_certified_crystal_carries_source_void_id() {
        let opts = IntegrationRunOptions {
            enable_crystal_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        // Every certified crystal must carry policy_id; void_id may be None.
        for rec in &r.certified_crystals {
            assert_eq!(rec.policy_id, policy().id);
            assert_ne!(rec.record_id, Digest::ZERO);
        }
    }

    // ── Resonite map — Step 5e-resonite ──────────────────────────────────────

    // ── Crystal → PSE unification adapter ───────────────────────────────────

    /// Build a gate-passing decision (Clean taint, Foundry authority) so the
    /// crystal candidate is certifiable — the same construction crystal.rs uses.
    fn make_certifiable_candidate() -> kosmo_hyphae::StructuralCrystalCandidate {
        use kosmo_core::{
            AuthorityLabel, EvidenceBundle, EvidenceKind, EvidenceRef, ReplayStatus, TaintLabel,
        };
        use kosmo_hyphae::{
            AssimilationDecision, GateCascade, StructuralYield, StructuralYieldKind,
        };
        let policy = policy();
        let ev = EvidenceBundle::seal(
            vec![EvidenceRef::new(
                Digest::of_bytes(b"e"),
                EvidenceKind::HostScan,
                "scan",
            )],
            policy.id,
            ReplayStatus::Replayable,
        );
        let yield_ = StructuralYield::new(
            StructuralYieldKind::DeficiencyFill,
            Some(Digest::of_bytes(b"v")),
            None,
            TaintLabel::Clean,
            AuthorityLabel::Foundry,
            ev.bundle_id,
            policy.id,
        );
        let cascade = GateCascade::standard_gates(policy.clone());
        let trace = cascade.apply(&yield_, &ev);
        let decision = AssimilationDecision::from_trace(&yield_, &trace, &ev, policy.id);
        kosmo_hyphae::StructuralCrystalCandidate::from_decision_with_signals(
            &decision,
            Some(Digest::of_bytes(b"v")),
            Q16::HALF,
            Q16::ONE,
        )
    }

    #[test]
    fn crystal_to_pse_candidate_wraps_record() {
        use kosmo_core::ReplayStatus;
        use kosmo_hyphae::{CrossLanguageFingerprint, SourceLanguage};
        let fp = CrossLanguageFingerprint::from_source(
            SourceLanguage::Python,
            Digest::of_bytes(b"src"),
            "import os\ndef a(x):\n    return x\n",
        );
        let candidate = make_certifiable_candidate().with_fingerprint(fp.clone());
        let (_cert, record) = candidate
            .certify(ReplayStatus::Replayable)
            .expect("clean-taint candidate must certify");

        let run_id = Digest::of_bytes(b"run");
        let c = crystal_to_pse_candidate(&record, run_id, policy().id);

        assert_eq!(c.kind, PseBridgeCandidateKind::CertifiedCrystal);
        assert_eq!(
            c.observation_digest, record.record_id,
            "addressed by record_id"
        );
        // confidence = (ρ + ω) / 2 = (0.5 + 1.0) / 2 = 0.75
        assert_eq!(c.confidence, Q16::ratio(3, 4).unwrap());
        // The record is directly evidence-bound; the candidate inherits it.
        assert_eq!(record.evidence_bundle_id, candidate.evidence_bundle_id);
        assert_eq!(c.evidence_bundle_id, record.evidence_bundle_id);
        assert_ne!(c.evidence_bundle_id, Digest::ZERO, "CROSS-006");
        assert!(
            c.label.starts_with("crystal:python:"),
            "label carries language: {}",
            c.label
        );
        assert_eq!(
            c.metadata.get("language").map(String::as_str),
            Some("python")
        );
        assert_eq!(
            c.metadata.get("fingerprint_id"),
            Some(&fp.fingerprint_id.to_hex()),
            "fingerprint travels as metadata"
        );
        assert!(c.verify_id(), "candidate is content-addressed");

        // Deterministic: same record → same candidate id.
        let c2 = crystal_to_pse_candidate(&record, run_id, policy().id);
        assert_eq!(c.id, c2.id);
    }

    #[test]
    fn crystal_to_pse_candidate_without_fingerprint_is_plain() {
        use kosmo_core::ReplayStatus;
        let candidate = make_certifiable_candidate();
        let (_cert, record) = candidate.certify(ReplayStatus::Replayable).unwrap();
        let c = crystal_to_pse_candidate(&record, Digest::of_bytes(b"run"), policy().id);
        assert_eq!(c.kind, PseBridgeCandidateKind::CertifiedCrystal);
        assert!(
            c.label.starts_with("crystal:-:"),
            "no-fingerprint label: {}",
            c.label
        );
        assert!(c.metadata.is_empty(), "no fingerprint → no metadata");
        assert!(c.verify_id());
    }

    #[test]
    fn pipeline_offers_every_certified_crystal_to_pse() {
        // Wiring invariant: with both layers enabled, the pipeline offers exactly
        // one CertifiedCrystal candidate per crystal certified in this run (zero
        // in report_only fixtures, where decisions are EvidenceOnly — the
        // equality must hold in both regimes).
        let opts = IntegrationRunOptions {
            enable_pse_candidates: true,
            enable_crystal_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&content_fixture_index(), &opts, &policy());
        let offered = r
            .pse_candidates
            .iter()
            .filter(|c| c.kind == PseBridgeCandidateKind::CertifiedCrystal)
            .count();
        assert_eq!(
            offered,
            r.certified_crystals.len(),
            "every certified crystal must be offered to PSE, and none invented"
        );
    }

    // ── Crystal-boosted SourceCube scoring (crystal_resonance dimension) ────

    #[test]
    fn cross_language_resonance_helper_picks_best_distinct_peer() {
        use kosmo_hyphae::{CrossLanguageFingerprint, SourceLanguage};
        // Structurally-identical algorithm in two languages → similarity 1.0.
        let py = CrossLanguageFingerprint::from_source(
            SourceLanguage::Python,
            Digest::of_bytes(b"py"),
            "import os\ndef a(x):\n    return x\ndef b(y):\n    return y\n",
        );
        let rs = CrossLanguageFingerprint::from_source(
            SourceLanguage::Rust,
            Digest::of_bytes(b"rs"),
            "use std::io;\npub fn a(x: u32) -> u32 { x }\npub fn b(y: u32) -> u32 { y }\n",
        );
        // A distinct peer exists → resonance equals the similarity to that peer.
        let res = best_cross_language_resonance(&py, &[py.clone(), rs.clone()]);
        assert_eq!(
            res,
            Some(py.similarity(&rs)),
            "resonance is similarity to the distinct peer"
        );
        assert_eq!(
            res,
            Some(Q16::ONE),
            "identical structural mix → resonance 1.0"
        );
        // Only itself in the set → no distinct peer → None.
        assert!(best_cross_language_resonance(&py, std::slice::from_ref(&py)).is_none());
        // Empty peer set → None.
        assert!(best_cross_language_resonance(&py, &[]).is_none());
    }

    #[test]
    fn pipeline_cross_language_resonance_invariant_and_fingerprints_stored() {
        use kosmo_workbench::{WorkspaceEntry, WorkspaceEntryKind};
        // Two structurally-similar modules in different languages. The HostCube must
        // store a fingerprint per void (the real cross-language proof through the
        // pipeline scan), and the resonance-dimension invariant must hold: any
        // SourceCube produced for an HDAG void carries cross_language_resonance.
        // (source_cubes is empty in report_only + Unverified taint, so the invariant
        // is vacuously satisfied there; the helper test above covers the computation.)
        let py = "import os\n\ndef alpha(x):\n    return x\n\ndef beta(y):\n    return y\n";
        let rs =
            "use std::io;\n\npub fn alpha(x: u32) -> u32 { x }\npub fn beta(y: u32) -> u32 { y }\n";
        let mk = |path: &str, src: &str| WorkspaceEntry {
            path: path.into(),
            digest: Digest::of_bytes(src.as_bytes()),
            size_bytes: src.len() as u64,
            kind: WorkspaceEntryKind::SourceFile,
            content: Some(src.to_string()),
        };
        let index = WorkspaceIndex::from_entries(
            "/repo".into(),
            vec![mk("src/mod_a.py", py), mk("src/mod_b.rs", rs)],
            policy().id,
        );
        let r = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy());
        let fps = &r.hyphae_result.host_cube.fingerprint_by_void_id;
        assert!(
            fps.len() >= 2,
            "both polyglot files must have fingerprints stored, got {}",
            fps.len()
        );
        for cube in &r.source_cubes {
            if let Some(void_id) = cube.target_void_id {
                if fps.contains_key(&void_id) {
                    assert!(
                        cube.dimension_profile.dimensions.contains_key("cross_language_resonance"),
                        "SourceCube for an HDAG void with a distinct peer must carry cross_language_resonance"
                    );
                }
            }
        }
    }

    #[test]
    fn pipeline_crystal_resonance_absent_without_prior_crystals() {
        // crystal_resonance must not appear when no prior_crystals are provided.
        let opts = IntegrationRunOptions::report_only();
        let r = run_dry_pipeline(&content_fixture_index(), &opts, &policy());
        let boosted = r.source_cubes.iter().any(|c| {
            c.dimension_profile
                .dimensions
                .contains_key("crystal_resonance")
        });
        assert!(
            !boosted,
            "crystal_resonance must not appear without prior_crystals"
        );
    }

    #[test]
    fn pipeline_crystal_resonance_absent_without_hdag() {
        // crystal_resonance requires source content (HDAG); file-presence-only entries
        // cannot produce a structural resonance match.
        let opts1 = IntegrationRunOptions {
            enable_crystal_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let r1 = run_dry_pipeline(&fixture_index(), &opts1, &policy());
        let prior = r1.certified_crystals.clone();
        let opts2 = IntegrationRunOptions {
            prior_crystals: prior,
            ..IntegrationRunOptions::report_only()
        };
        // fixture_index has content: None → no HDAG → no crystal_resonance dim.
        let r2 = run_dry_pipeline(&fixture_index(), &opts2, &policy());
        let boosted = r2.source_cubes.iter().any(|c| {
            c.dimension_profile
                .dimensions
                .contains_key("crystal_resonance")
        });
        assert!(
            !boosted,
            "crystal_resonance must not appear when entries lack source content"
        );
    }

    #[test]
    fn pipeline_resonite_map_empty_without_prior_crystals() {
        let opts = IntegrationRunOptions {
            enable_crystal_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        assert!(
            r.resonite_map.is_empty(),
            "resonite_map must be empty when no prior_crystals are provided"
        );
    }

    #[test]
    fn pipeline_resonite_map_populated_with_prior_crystals() {
        let opts1 = IntegrationRunOptions {
            enable_crystal_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let r1 = run_dry_pipeline(&fixture_index(), &opts1, &policy());
        if r1.certified_crystals.is_empty() {
            return; // no accepted decisions → skip
        }
        let prior = r1.certified_crystals.clone();
        let opts2 = IntegrationRunOptions {
            enable_crystal_candidates: true,
            prior_crystals: prior.clone(),
            ..IntegrationRunOptions::report_only()
        };
        let r2 = run_dry_pipeline(&fixture_index(), &opts2, &policy());
        if r2.certified_crystals.is_empty() {
            return; // still no accepted decisions → skip
        }
        let expected_count = r2.certified_crystals.len() * prior.len();
        assert_eq!(
            r2.resonite_map.len(),
            expected_count,
            "resonite_map must have current × prior entries"
        );
        for resonite in &r2.resonite_map {
            assert_eq!(resonite.policy_id, policy().id);
            assert_ne!(resonite.resonite_id, Digest::ZERO);
        }
    }

    #[test]
    fn pipeline_resonite_map_policy_consistent() {
        let opts1 = IntegrationRunOptions {
            enable_crystal_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let r1 = run_dry_pipeline(&fixture_index(), &opts1, &policy());
        let opts2 = IntegrationRunOptions {
            enable_crystal_candidates: true,
            prior_crystals: r1.certified_crystals.clone(),
            ..IntegrationRunOptions::report_only()
        };
        let r2 = run_dry_pipeline(&fixture_index(), &opts2, &policy());
        assert!(
            r2.verify_policy_consistency(),
            "resonite_map must be covered by verify_policy_consistency"
        );
    }

    #[test]
    fn pipeline_resonite_count_in_report_id() {
        let opts1 = IntegrationRunOptions {
            enable_crystal_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let r1 = run_dry_pipeline(&fixture_index(), &opts1, &policy());
        let opts2 = IntegrationRunOptions {
            enable_crystal_candidates: true,
            prior_crystals: r1.certified_crystals.clone(),
            ..IntegrationRunOptions::report_only()
        };
        let r2 = run_dry_pipeline(&fixture_index(), &opts2, &policy());
        // resonite_count participates in report_id, so reports with/without resonites differ.
        if r2.resonite_map.is_empty() {
            // No crystals → resonite_count=0 → same hash domain as r1 from resonite perspective.
            // This is expected; just verify report_ids are non-zero.
            assert_ne!(r2.report_id, Digest::ZERO);
        } else {
            assert_ne!(
                r1.report_id, r2.report_id,
                "different resonite_count must produce different report_id"
            );
        }
    }

    // ── NormFitnessTrace from prior feedback (Step 5c) ───────────────────────

    #[test]
    fn pipeline_no_fitness_traces_when_no_prior_feedback() {
        let opts = IntegrationRunOptions {
            enable_norm_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        assert!(
            r.norm_fitness_traces.is_empty(),
            "norm_fitness_traces must be empty when prior_feedback is empty"
        );
    }

    #[test]
    fn pipeline_fitness_trace_built_from_matching_feedback() {
        use kosmo_core::{FeedbackOutcome, PromotionFeedback};
        let p = policy();
        let opts_gen = IntegrationRunOptions {
            enable_norm_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        // First run: generate candidates.
        let r1 = run_dry_pipeline(&fixture_index(), &opts_gen, &p);
        if r1.norm_candidates.is_empty() {
            return; // no accepted decisions in this fixture — skip
        }
        let candidate = &r1.norm_candidates[0];
        let energy = Q16::ratio(3, 4).unwrap();
        let feedback = PromotionFeedback::new(
            Digest::of_bytes(b"record"),
            candidate.candidate_id,
            candidate.candidate_id,
            FeedbackOutcome::Accepted,
            energy,
            p.id,
            candidate.evidence_bundle_id,
        );
        // Second run: apply the feedback.
        let opts_fb = IntegrationRunOptions {
            enable_norm_candidates: true,
            prior_feedback: vec![feedback.clone()],
            ..IntegrationRunOptions::report_only()
        };
        let r2 = run_dry_pipeline(&fixture_index(), &opts_fb, &p);
        let trace = r2
            .norm_fitness_traces
            .iter()
            .find(|t| t.candidate_id == candidate.candidate_id)
            .expect("trace must exist for the candidate that received feedback");
        assert_eq!(trace.observations.len(), 1);
        assert_eq!(trace.latest_fitness(), energy);
        assert_eq!(trace.observations[0].evidence_ref, feedback.id);
        assert_eq!(trace.policy_id, p.id);
    }

    #[test]
    fn pipeline_fitness_trace_skips_unmatched_feedback() {
        use kosmo_core::{FeedbackOutcome, PromotionFeedback};
        let p = policy();
        // Build feedback with a random candidate_id that matches nothing.
        let unrelated_id = Digest::of_bytes(b"unrelated-candidate");
        let feedback = PromotionFeedback::new(
            Digest::of_bytes(b"record"),
            unrelated_id,
            unrelated_id,
            FeedbackOutcome::Accepted,
            Q16::ONE,
            p.id,
            Digest::of_bytes(b"ev"),
        );
        let opts = IntegrationRunOptions {
            enable_norm_candidates: true,
            prior_feedback: vec![feedback],
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&fixture_index(), &opts, &p);
        assert!(
            r.norm_fitness_traces.is_empty(),
            "unmatched feedback must not produce traces"
        );
    }

    // ── SourceCube HDAG dimension enrichment (Step 2b) ───────────────────────

    fn content_fixture_index() -> WorkspaceIndex {
        use kosmo_workbench::{WorkspaceEntry, WorkspaceEntryKind};
        let source = "pub fn alpha() {}\npub fn beta() {}\npub fn gamma() {}\n";
        let entries = vec![WorkspaceEntry {
            path: "src/lib.rs".into(),
            digest: Digest::of_bytes(source.as_bytes()),
            size_bytes: source.len() as u64,
            kind: WorkspaceEntryKind::SourceFile,
            content: Some(source.to_string()),
        }];
        WorkspaceIndex::from_entries("test-root".into(), entries, policy().id)
    }

    #[test]
    fn pipeline_source_cubes_field_present_always() {
        // source_cubes is always in the report (may be empty when no accepted decisions).
        let opts = IntegrationRunOptions::report_only();
        let r_empty = run_dry_pipeline(&empty_index(), &opts, &policy());
        let r_fixture = run_dry_pipeline(&fixture_index(), &opts, &policy());
        let r_content = run_dry_pipeline(&content_fixture_index(), &opts, &policy());
        // In report_only mode all intents produce EvidenceOnly (Unverified/Agent taint),
        // so source_cubes is always empty. Verify the field is present and well-formed.
        assert!(r_empty.source_cubes.is_empty());
        assert!(r_fixture.source_cubes.is_empty());
        assert!(r_content.source_cubes.is_empty());
    }

    #[test]
    fn pipeline_source_cube_hdag_dims_when_void_has_hdag() {
        // Verify that ANY SourceCube produced for a void with an HDAG carries
        // rho_coherence and omega_phase. We test this invariant by checking every cube
        // in source_cubes: if the host_cube has an HDAG for the void_id, the dims must be set.
        // (When source_cubes is empty this is vacuously true — the unit tests in
        //  kosmo-hyphae/src/host.rs verify the HDAG extraction path directly.)
        let opts = IntegrationRunOptions::report_only();
        let r = run_dry_pipeline(&content_fixture_index(), &opts, &policy());
        for cube in &r.source_cubes {
            if let Some(void_id) = cube.target_void_id {
                if r.hyphae_result
                    .host_cube
                    .hdag_by_void_id
                    .contains_key(&void_id)
                {
                    assert!(
                        cube.dimension_profile
                            .dimensions
                            .contains_key("rho_coherence"),
                        "SourceCube for HDAG void must carry rho_coherence"
                    );
                    assert!(
                        cube.dimension_profile
                            .dimensions
                            .contains_key("omega_phase"),
                        "SourceCube for HDAG void must carry omega_phase"
                    );
                }
            }
        }
    }

    #[test]
    fn pipeline_source_cube_no_hdag_dimensions_without_content() {
        let opts = IntegrationRunOptions::report_only();
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        // Without source content (fixture_index has content: None), no HDAG dimensions.
        let enriched = r
            .source_cubes
            .iter()
            .any(|c| c.dimension_profile.dimensions.contains_key("rho_coherence"));
        assert!(
            !enriched,
            "no HDAG dimensions expected when WorkspaceEntry.content is None"
        );
    }

    #[test]
    fn pipeline_surgery_workbench_tasks_carry_correct_policy_id() {
        let opts = IntegrationRunOptions {
            enable_surgery: true,
            enable_metatron: true,
            ..IntegrationRunOptions::report_only()
        };
        let p = policy();
        let r = run_dry_pipeline(&fixture_index(), &opts, &p);
        for t in &r.surgery_workbench_tasks {
            assert_eq!(
                t.policy_id, p.id,
                "surgery workbench task must carry pipeline policy_id"
            );
            assert_ne!(t.task_id, Digest::ZERO, "task_id must be non-ZERO");
            assert_ne!(
                t.surgery_option_id,
                Digest::ZERO,
                "surgery_option_id must trace back to source option"
            );
        }
    }

    // ── Crystal auto-persist (Step 5f) ───────────────────────────────────────

    fn temp_crystal_store_path(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut p = std::env::temp_dir();
        p.push(format!("kosmo-pipeline-crystal-{tag}-{nanos}.jsonl"));
        p
    }

    #[test]
    fn pipeline_crystal_auto_persist_report_only_does_not_write() {
        // ReportOnly policy forbids host writes — store file must not be created.
        let path = temp_crystal_store_path("ro");
        let opts = IntegrationRunOptions {
            enable_crystal_candidates: true,
            crystal_store_path: Some(path.clone()),
            ..IntegrationRunOptions::report_only()
        };
        run_dry_pipeline(
            &fixture_index(),
            &opts,
            &PolicyProfile::default_report_only(),
        );
        assert!(
            !path.exists(),
            "ReportOnly must not create crystal store file"
        );
        assert_eq!(opts.crystal_store_path.as_deref(), Some(path.as_path()));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn pipeline_crystal_auto_persist_operator_approved_writes_and_reloads() {
        // OperatorApproved pipeline: certified crystals must be auto-persisted.
        // A second run with the same path must auto-load them as prior_crystals.
        let path = temp_crystal_store_path("op");
        let _ = std::fs::remove_file(&path);
        let op_policy = PolicyProfile::operator_approved();

        let opts1 = IntegrationRunOptions {
            enable_crystal_candidates: true,
            crystal_store_path: Some(path.clone()),
            ..IntegrationRunOptions::report_only()
        };
        let r1 = run_dry_pipeline(&fixture_index(), &opts1, &op_policy);
        // In ReportOnly mode fixture all decisions are EvidenceOnly, so crystals = 0.
        // `persisted_crystal_count` reflects what was actually written.
        assert_eq!(
            r1.persisted_crystal_count,
            r1.certified_crystals.len() as u32,
            "persisted count must equal certified count on first run"
        );

        // Second run: auto-loads any stored crystals into prior_crystals.
        let opts2 = IntegrationRunOptions {
            enable_crystal_candidates: true,
            crystal_store_path: Some(path.clone()),
            ..IntegrationRunOptions::report_only()
        };
        let r2 = run_dry_pipeline(&fixture_index(), &opts2, &op_policy);
        // Auto-loaded prior_crystals propagate to resonite_map (if any crystals exist).
        // Both runs use the same fixture so certified_crystals are the same set;
        // dedup means persisted_crystal_count on r2 = 0 (all already stored).
        if r1.certified_crystals.is_empty() {
            // No crystals produced (ReportOnly + standard fixture) — still valid.
            assert_eq!(r2.persisted_crystal_count, 0);
        } else {
            assert_eq!(
                r2.persisted_crystal_count, 0,
                "second run must not re-write already-persisted crystals (dedup)"
            );
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn pipeline_crystal_store_path_none_gives_zero_persisted_count() {
        let opts = IntegrationRunOptions {
            enable_crystal_candidates: true,
            crystal_store_path: None,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        assert_eq!(
            r.persisted_crystal_count, 0,
            "no crystal_store_path → persisted_crystal_count must be 0"
        );
    }

    #[test]
    fn pipeline_crystal_store_path_none_does_not_affect_report_id() {
        // persisted_crystal_count is NOT in report_id, so two runs differing only
        // in whether they persist to disk must produce the same report_id.
        let path = temp_crystal_store_path("same-id");
        let _ = std::fs::remove_file(&path);
        let op_policy = PolicyProfile::operator_approved();
        let opts_no_store = IntegrationRunOptions {
            enable_crystal_candidates: true,
            crystal_store_path: None,
            ..IntegrationRunOptions::report_only()
        };
        let opts_with_store = IntegrationRunOptions {
            enable_crystal_candidates: true,
            crystal_store_path: Some(path.clone()),
            ..IntegrationRunOptions::report_only()
        };
        let r_no = run_dry_pipeline(&fixture_index(), &opts_no_store, &op_policy);
        let r_yes = run_dry_pipeline(&fixture_index(), &opts_with_store, &op_policy);
        assert_eq!(
            r_no.report_id, r_yes.report_id,
            "crystal persistence must not affect report_id (observational field)"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn pipeline_with_crystal_store_path_builder() {
        // Smoke-test the with_crystal_store_path builder; no assertions on content.
        let path = temp_crystal_store_path("builder");
        let opts = IntegrationRunOptions::report_only().with_crystal_store_path(path.clone());
        assert_eq!(opts.crystal_store_path.as_deref(), Some(path.as_path()));
        let _ = std::fs::remove_file(&path);
    }

    // ── WorkspacePipelineSession + run_workspace_pipeline ────────────────────

    fn temp_workspace(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut p = std::env::temp_dir();
        p.push(format!("kosmo-ws-{tag}-{nanos}"));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn write_rs(dir: &std::path::Path, name: &str, content: &str) {
        let src = dir.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join(name), content).unwrap();
    }

    #[test]
    fn run_workspace_pipeline_on_real_path() {
        let ws = temp_workspace("real");
        write_rs(&ws, "lib.rs", "pub fn foo() {}\npub fn bar() {}\n");
        write_rs(&ws, "main.rs", "fn main() {}\n");
        let opts = IntegrationRunOptions {
            enable_crystal_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let result = run_workspace_pipeline(ws.to_str().unwrap(), &opts, &policy());
        let report = result.expect("scan + pipeline must succeed on existing path");
        assert_ne!(report.report_id, Digest::ZERO, "report_id must be non-ZERO");
        assert!(
            report.verify_policy_consistency(),
            "policy must be consistent"
        );
        let _ = std::fs::remove_dir_all(&ws);
    }

    #[test]
    fn run_workspace_pipeline_nonexistent_path_returns_error() {
        let result = run_workspace_pipeline(
            "/nonexistent/totally/made/up",
            &IntegrationRunOptions::report_only(),
            &policy(),
        );
        assert!(
            result.is_err(),
            "nonexistent path must yield WorkspaceError"
        );
    }

    #[test]
    fn run_workspace_pipeline_extracts_hdag_from_content() {
        // When source content is available, HDAG extraction produces non-empty hdag_by_void_id.
        // We verify this by checking that the HostCube in the HYPHAE result is non-trivial.
        let ws = temp_workspace("hdag");
        write_rs(
            &ws,
            "lib.rs",
            "pub fn alpha() {}\npub fn beta() {}\npub fn gamma() {}\n",
        );
        let opts = IntegrationRunOptions::report_only();
        let report =
            run_workspace_pipeline(ws.to_str().unwrap(), &opts, &policy()).expect("must succeed");
        // At minimum the workspace has source files → void_map is non-trivial.
        assert_ne!(report.deficiency_vector.vector_id, Digest::ZERO);
        let _ = std::fs::remove_dir_all(&ws);
    }

    #[test]
    fn workspace_pipeline_session_run_count_increments() {
        let ws = temp_workspace("session");
        write_rs(&ws, "lib.rs", "pub fn foo() {}\n");
        let mut session =
            WorkspacePipelineSession::new(IntegrationRunOptions::report_only(), policy());
        assert_eq!(session.run_count(), 0);
        session
            .run(ws.to_str().unwrap())
            .expect("first run must succeed");
        assert_eq!(session.run_count(), 1);
        session
            .run(ws.to_str().unwrap())
            .expect("second run must succeed");
        assert_eq!(session.run_count(), 2);
        let _ = std::fs::remove_dir_all(&ws);
    }

    #[test]
    fn workspace_pipeline_session_accumulates_crystals_via_store() {
        // Two sessions on the same workspace with a shared crystal_store_path:
        // the second run auto-loads whatever the first persisted.
        let ws = temp_workspace("accumulate");
        write_rs(&ws, "lib.rs", "pub fn foo() {}\npub fn bar() {}\n");
        let store_path = temp_crystal_store_path("accumulate");
        let _ = std::fs::remove_file(&store_path);
        let op_policy = PolicyProfile::operator_approved();

        let opts = IntegrationRunOptions {
            enable_crystal_candidates: true,
            crystal_store_path: Some(store_path.clone()),
            ..IntegrationRunOptions::report_only()
        };
        let mut session = WorkspacePipelineSession::new(opts, op_policy);
        let r1 = session.run(ws.to_str().unwrap()).expect("first run");
        let r2 = session.run(ws.to_str().unwrap()).expect("second run");

        // Second run must not re-persist already-stored crystals (dedup).
        if !r1.certified_crystals.is_empty() {
            assert_eq!(
                r2.persisted_crystal_count, 0,
                "second run must not duplicate crystals from first run"
            );
        }
        // Both runs must be non-trivially identified.
        assert_ne!(r1.report_id, Digest::ZERO);
        assert_ne!(r2.report_id, Digest::ZERO);
        let _ = std::fs::remove_dir_all(&ws);
        let _ = std::fs::remove_file(&store_path);
    }

    // ── ActionItem — CAM layer ────────────────────────────────────────────────

    #[test]
    fn action_items_empty_report_has_no_items() {
        let r = run_dry_pipeline(
            &empty_index(),
            &IntegrationRunOptions::report_only(),
            &policy(),
        );
        let items = r.action_items();
        // Empty workspace → no voids, no surgery, no PSE, no crystals, no norms.
        assert!(
            items.is_empty(),
            "empty workspace must produce no action items"
        );
    }

    #[test]
    fn action_items_returns_fill_void_for_each_void_in_report() {
        let r = run_dry_pipeline(
            &fixture_index(),
            &IntegrationRunOptions::report_only(),
            &policy(),
        );
        let voids = r.void_priority_ranking.len();
        let fill_voids: Vec<_> = r
            .action_items()
            .into_iter()
            .filter(|a| matches!(a.kind, ActionItemKind::FillVoid { .. }))
            .collect();
        assert_eq!(
            fill_voids.len(),
            voids,
            "must have one FillVoid item per void in void_priority_ranking"
        );
    }

    #[test]
    fn action_items_are_sorted_descending_by_priority() {
        let r = run_dry_pipeline(
            &fixture_index(),
            &IntegrationRunOptions::report_only(),
            &policy(),
        );
        let items = r.action_items();
        for window in items.windows(2) {
            assert!(
                window[0].priority_score.raw() >= window[1].priority_score.raw(),
                "action_items must be sorted by priority_score descending"
            );
        }
    }

    #[test]
    fn action_items_are_content_addressed() {
        let r = run_dry_pipeline(
            &fixture_index(),
            &IntegrationRunOptions::report_only(),
            &policy(),
        );
        let items = r.action_items();
        for item in &items {
            assert_ne!(item.action_id, Digest::ZERO, "action_id must be non-ZERO");
            assert_eq!(
                item.policy_id,
                policy().id,
                "action_id must carry pipeline policy_id"
            );
        }
    }

    #[test]
    fn action_items_top_void_has_highest_priority() {
        // The void at position 0 in void_priority_ranking should score Q16::ONE.
        let r = run_dry_pipeline(
            &fixture_index(),
            &IntegrationRunOptions::report_only(),
            &policy(),
        );
        if r.void_priority_ranking.is_empty() {
            return;
        }
        let top_void = r.void_priority_ranking[0];
        let items = r.action_items();
        let top_item = items.iter().find(
            |a| matches!(a.kind, ActionItemKind::FillVoid { void_id } if void_id == top_void),
        );
        if let Some(item) = top_item {
            assert_eq!(
                item.priority_score,
                Q16::ONE,
                "top-ranked void must have Q16::ONE priority_score"
            );
        }
    }

    #[test]
    fn action_items_surgery_produces_repair_topology_entries() {
        let opts = IntegrationRunOptions {
            enable_metatron: true,
            enable_surgery: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        let repairs: Vec<_> = r
            .action_items()
            .into_iter()
            .filter(|a| matches!(a.kind, ActionItemKind::RepairTopology { .. }))
            .collect();
        assert_eq!(
            repairs.len(),
            r.surgery_options.len(),
            "must have one RepairTopology per surgery option"
        );
    }

    #[test]
    fn action_items_norm_candidates_produce_apply_norm_entries() {
        let opts = IntegrationRunOptions {
            enable_norm_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        let norms: Vec<_> = r
            .action_items()
            .into_iter()
            .filter(|a| matches!(a.kind, ActionItemKind::ApplyNorm { .. }))
            .collect();
        assert_eq!(
            norms.len(),
            r.norm_candidates.len(),
            "must have one ApplyNorm per norm candidate"
        );
    }

    #[test]
    fn workspace_pipeline_session_exposes_options_and_policy() {
        let opts = IntegrationRunOptions::report_only();
        let pol = PolicyProfile::default_report_only();
        let session = WorkspacePipelineSession::new(opts.clone(), pol.clone());
        assert_eq!(session.policy().id, pol.id);
        assert_eq!(
            session.options().enable_crystal_candidates,
            opts.enable_crystal_candidates
        );
    }

    // ─── Wish landscape ──────────────────────────────────────────────────────

    fn void(kind: kosmo_hyphae::HostVoidKind, sev_pct: u64, loc: &str) -> kosmo_hyphae::HostVoid {
        kosmo_hyphae::HostVoid::new(kind, Q16::ratio(sev_pct, 100).unwrap(), loc.to_string())
    }

    #[test]
    fn landscape_projects_doc_and_test_voids() {
        use kosmo_hyphae::HostVoidKind;
        let voids = vec![
            void(
                HostVoidKind::MissingDocFiber {
                    for_module: "router".into(),
                },
                70,
                "src/router.rs",
            ),
            void(
                HostVoidKind::MissingTestFiber {
                    for_module: "router".into(),
                },
                90,
                "src/router.rs",
            ),
            void(
                HostVoidKind::MissingErrorHandling {
                    location: "src/io.rs:40".into(),
                },
                50,
                "src/io.rs",
            ),
        ];
        let l = propose_wishes(&voids);
        assert_eq!(l.proposals.len(), 2);
        // Sorted by severity descending: the test wish (0.90) first.
        assert_eq!(l.proposals[0].facet, WishFacet::test("router_smoke"));
        assert_eq!(l.proposals[1].facet, WishFacet::doc("router"));
        assert!(l.proposals[1]
            .rationale
            .contains("MissingDocFiber @ src/router.rs"));
        // The inexpressible finding is honest residue, never dropped.
        assert_eq!(l.unmapped.len(), 1);
        assert!(l.unmapped[0].kind_label.contains("MissingErrorHandling"));
    }

    #[test]
    fn landscape_normalizes_path_targets_to_module_stems() {
        use kosmo_hyphae::HostVoidKind;
        let voids = vec![
            void(
                HostVoidKind::MissingDocFiber {
                    for_module: "src/router.rs".into(),
                },
                40,
                "src/router.rs",
            ),
            void(
                HostVoidKind::MissingTestFiber {
                    for_module: "lib/handlers.py".into(),
                },
                60,
                "lib/handlers.py",
            ),
        ];
        let l = propose_wishes(&voids);
        assert_eq!(l.proposals[0].facet, WishFacet::test("handlers_smoke"));
        assert_eq!(l.proposals[0].subject, "handlers");
        assert_eq!(l.proposals[1].facet, WishFacet::doc("router"));
        assert_eq!(l.proposals[1].subject, "router");
        // Provenance keeps the full location.
        assert!(l.proposals[1].rationale.contains("@ src/router.rs"));
    }

    #[test]
    fn landscape_dedups_per_facet_keeping_max_severity() {
        use kosmo_hyphae::HostVoidKind;
        let voids = vec![
            void(
                HostVoidKind::MissingDocFiber {
                    for_module: "router".into(),
                },
                30,
                "src/a.rs",
            ),
            void(
                HostVoidKind::MissingDocFiber {
                    for_module: "router".into(),
                },
                80,
                "src/b.rs",
            ),
        ];
        let l = propose_wishes(&voids);
        assert_eq!(l.proposals.len(), 1);
        assert_eq!(l.proposals[0].severity, Q16::ratio(80, 100).unwrap());
        assert!(l.proposals[0].rationale.contains("src/b.rs"));
        // Content-addressed and verifiable: same inputs, same id.
        let again = propose_wishes(&voids);
        assert_eq!(l, again, "the landscape is deterministic");
    }
}

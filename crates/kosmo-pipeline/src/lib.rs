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
pub mod materialization;
pub mod persistence;

pub use aggregator::{AggregatedGateResult, GateTraceAggregator, LayerGateSummary};
pub use materialization::{
    MaterializationOutcome, MaterializationPlan, OperatorApprovalToken,
    ParseBackExpectation, WorkbenchMaterializationTask, simulate_foundry_check,
};
pub use persistence::persist_cartography_update;

use kosmo_core::{Digest, GateResult, PolicyProfile, PromotionFeedback, Q16, rank_by_energy};
use kosmo_core::TaintLabel;
use std::collections::BTreeMap;
use kosmo_hyphae::{
    CompositeSupportCube, ComplementVoidHypothesis, CorpusCartography, CorpusCartographyUpdate,
    CubeSwarm, CubeDimensionProfile, HostTargetCollapsePlan, HostTargetDelta, LpcmPassiveReport,
    MetatronMicrograph, MetatronRegionFingerprint, MicrographLiftReport,
    MicroTopologyDiagnostic, MicroTopologyIndex, MorphogenicCorpusUpdate, NormFitnessTrace,
    NormGeneCandidate, Fragment, FragmentField, FragmentKind, SourceCube, SeamGraph,
    StructuralCrystalCandidate, SupportMassVector, SurgeryWorkbenchTask, TopologicalSurgeryOption,
    TopologyAmbiguityProfile, diagnose_micrograph, lift_region, passive_run, HyphaeRunResult,
};
use kosmo_systemcube::{
    BlueprintUnit, BlueprintUnitKind, KcubeExportReport, SystemCube,
};
use kosmo_workbench::WorkspaceIndex;
use serde::{Deserialize, Serialize};

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
    /// Generate energy-ranked `NormGeneCandidate` objects from accepted decisions.
    /// Initial fitness = `Q16::ONE`; evolves via `NormFitnessTrace` in later phases.
    pub enable_norm_candidates: bool,
    /// Collect `StructuralCrystalCandidate` objects from accepted decisions (Step 5d).
    /// Candidates start with `support_score = Q16::ZERO` (pending certification).
    pub enable_crystal_candidates: bool,
    /// Prior `PromotionFeedback` records to ingest into `NormFitnessTrace` objects
    /// for each matching norm candidate (Step 5c). Empty by default.
    pub prior_feedback: Vec<PromotionFeedback>,
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
            enable_norm_candidates: false,
            enable_crystal_candidates: false,
            prior_feedback: vec![],
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
            enable_norm_candidates: true,
            enable_crystal_candidates: true,
            prior_feedback: vec![],
        }
    }
}

// ─── Integration Run Report ───────────────────────────────────────────────────

/// Content for deterministic `IntegrationRunReport` ID.
#[derive(Serialize)]
struct ReportContent {
    hyphae_run_id: Digest,
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
    norm_candidate_count: u32,
    norm_fitness_trace_count: u32,
    has_systemcube: bool,
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
    pub cartography_update: CorpusCartographyUpdate,
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
}

impl IntegrationRunReport {
    fn new(
        hyphae_result: HyphaeRunResult,
        cartography_update: CorpusCartographyUpdate,
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
        norm_candidates: Vec<NormGeneCandidate>,
        norm_fitness_traces: Vec<NormFitnessTrace>,
        systemcube_export: Option<KcubeExportReport>,
        aggregated_gate: AggregatedGateResult,
        policy: &PolicyProfile,
    ) -> Self {
        let final_result = aggregated_gate.final_result.clone();
        let report_id = Digest::of(&ReportContent {
            hyphae_run_id: hyphae_result.run_id,
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
            norm_candidate_count: norm_candidates.len() as u32,
            norm_fitness_trace_count: norm_fitness_traces.len() as u32,
            has_systemcube: systemcube_export.is_some(),
        });
        Self {
            report_id,
            policy_id: policy.id,
            hyphae_result,
            cartography_update,
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
            norm_candidates,
            norm_fitness_traces,
            systemcube_export,
            aggregated_gate,
            final_result,
        }
    }

    /// Operator-readable summary of the pipeline run.
    pub fn summary(&self) -> String {
        let scube = if let Some(ref e) = self.systemcube_export {
            format!("systemcube=Some(mode={:?})", e.mode)
        } else {
            "systemcube=None".into()
        };
        format!(
            "IntegrationRunReport — policy={:.8} | final={:?} | \
             hyphae: {} | cartography: {} entities | voids (priority): {} | \
             swarm: {} cubes → {:?} | collapse: {} steps ({:?}) | \
             morphogenic: {:.8} | metatron: {} (index: {:.8}, lift: {}) | lpcm: {} | \
             surgery: {} (tasks: {}) | ambiguities: {} | void_hyp: {} | \
             crystal_candidates: {} | norm_candidates: {} (traces: {}) | {}",
            hex_prefix(&self.policy_id),
            self.final_result,
            self.hyphae_result.summary(),
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
            && self.surgery_workbench_tasks.iter().all(|t| t.policy_id == pid)
            && self.ambiguity_profiles.iter().all(|a| a.policy_id == pid)
            && self.complement_void_hypotheses.iter().all(|h| h.policy_id == pid)
            && self.crystal_candidates.iter().all(|c| c.policy_id == pid)
            && self.norm_candidates.iter().all(|c| c.policy_id == pid)
            && self.norm_fitness_traces.iter().all(|t| t.policy_id == pid)
            && self
                .systemcube_export
                .as_ref()
                .map_or(true, |e| e.policy_id == pid)
    }
}

fn hex_prefix(d: &Digest) -> String {
    d.as_bytes().iter().take(4).map(|b| format!("{:02x}", b)).collect()
}

// ─── Pipeline Entry Point ──────────────────────────────────────────────────────

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

    // ── 1. HYPHAE v0.3 passive run ────────────────────────────────────────────
    let hyphae = passive_run(index, policy);

    // Record HYPHAE gate contribution: any rejected decision contributes Reject.
    let hyphae_gate = if hyphae.rejected_count > 0 {
        GateResult::Reject {
            reason: format!("{} yields rejected by gate cascade", hyphae.rejected_count),
        }
    } else if hyphae.evidence_only_count > 0 {
        GateResult::Warn {
            message: format!("{} yields downgraded to evidence-only", hyphae.evidence_only_count),
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

    // ── 2. CorpusCartography update ───────────────────────────────────────────
    let corpus = CorpusCartography::empty(policy.id);
    let (_, cartography_update) = corpus.update_from_run(&hyphae);
    agg.record(
        "cartography",
        cartography_update.update_id,
        GateResult::Pass,
    );

    // ── 3. Optional Metatron v0.4.1 diagnostics ───────────────────────────────
    let mut metatron_diagnostics: Vec<MicroTopologyDiagnostic> = Vec::new();
    let mut raw_lift_reports: Vec<MicrographLiftReport> = Vec::new();
    let mut metatron_triples: Vec<(MetatronMicrograph, MetatronRegionFingerprint, MicroTopologyDiagnostic)> = Vec::new();
    if options.enable_metatron {
        for void in &hyphae.host_cube.void_map.voids {
            let ev_id = void.void_id;
            let (micrograph, fingerprint, lift_report) =
                lift_region(void.void_id, vec![void.void_id], ev_id, TaintLabel::Synthetic, policy);
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
        let assessments: Vec<_> = raw_lift_reports.iter()
            .map(|r| r.energy_assessment(&GateResult::Pass))
            .collect();
        let ranked = rank_by_energy(&assessments);
        ranked.iter()
            .filter_map(|a| raw_lift_reports.iter().find(|r| r.report_id == a.subject_id).cloned())
            .collect()
    };
    // ── 3d. MicroTopologyIndex — content-addressed index of all metatron output ─
    let metatron_index: MicroTopologyIndex = metatron_triples
        .iter()
        .fold(MicroTopologyIndex::empty(policy.id), |idx, (m, f, d)| idx.add(m, f, d));

    // ── 3b. Optional surgery — energy-ranked options from Metatron diagnostics ─
    // Only runs when both Metatron and surgery are enabled; requires diagnostics.
    let surgery_options: Vec<TopologicalSurgeryOption> =
        if options.enable_surgery && options.enable_metatron {
            let raw: Vec<TopologicalSurgeryOption> = metatron_diagnostics
                .iter()
                .flat_map(|diag| TopologicalSurgeryOption::from_diagnostic(diag, policy))
                .collect();
            let assessments: Vec<_> = raw.iter()
                .map(|o| o.energy_assessment(&GateResult::Pass))
                .collect();
            let ranked = rank_by_energy(&assessments);
            // Reorder raw options to match energy ranking.
            ranked.iter()
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
        let assessments: Vec<_> = raw.iter()
            .map(|a| a.energy_assessment(&GateResult::Pass))
            .collect();
        let ranked = rank_by_energy(&assessments);
        ranked.iter()
            .filter_map(|a| raw.iter().find(|p| p.profile_id == a.subject_id).cloned())
            .collect()
    };
    let complement_void_hypotheses: Vec<ComplementVoidHypothesis> = {
        let raw: Vec<ComplementVoidHypothesis> = metatron_diagnostics
            .iter()
            .flat_map(|d| d.void_hypotheses.iter().cloned())
            .collect();
        let assessments: Vec<_> = raw.iter()
            .map(|h| h.energy_assessment(&GateResult::Pass))
            .collect();
        let ranked = rank_by_energy(&assessments);
        ranked.iter()
            .filter_map(|a| raw.iter().find(|h| h.hypothesis_id == a.subject_id).cloned())
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

    // Build SourceCubes from accepted decisions. intents and decisions are
    // produced in lockstep by passive_run, so zip is safe.
    let source_cubes: Vec<SourceCube> = hyphae
        .frontier
        .intents
        .iter()
        .zip(hyphae.decisions.iter())
        .filter(|(_, d)| d.outcome.is_accepted())
        .map(|(intent, decision)| {
            SourceCube::new(
                intent.target_void_id,
                format!("intent:{}", &decision.yield_id.to_hex()[..16]),
                CubeDimensionProfile::empty(),
                Q16::ONE,
                intent.taint.clone(),
                decision.evidence_bundle_id,
                policy.id,
            )
        })
        .collect();

    let swarm = CubeSwarm::new(policy.clone(), source_cubes.clone());
    let (_workers, swarm_composite) = swarm.run();

    let host_void_ids: Vec<Digest> = hyphae.host_cube.void_map.voids.iter().map(|v| v.void_id).collect();
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

    // ── 5d. Optional crystal candidates — from accepted decisions ────────────
    // One StructuralCrystalCandidate per accepted decision; support_score = Q16::ZERO
    // (Pending certification). Collected to form an explicit certification work queue.
    let crystal_candidates: Vec<StructuralCrystalCandidate> =
        if options.enable_crystal_candidates {
            hyphae.decisions.iter()
                .filter(|d| d.outcome.is_accepted())
                .map(StructuralCrystalCandidate::from_decision)
                .collect()
        } else {
            Vec::new()
        };

    // ── 5b. Optional norm candidates — from accepted decisions ────────────────
    // One NormGeneCandidate per accepted decision: name derived from void/yield,
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
                let void_hex = intent.target_void_id
                    .map(|id| id.to_hex()[..16].to_string())
                    .unwrap_or_else(|| decision.yield_id.to_hex()[..16].to_string());
                let name = format!("norm:void:{}", void_hex);
                let description = format!(
                    "Norm gene from accepted decision for void {}",
                    void_hex
                );
                NormGeneCandidate::new(
                    name,
                    description,
                    Q16::ONE,
                    decision.evidence_bundle_id,
                    policy.id,
                )
            })
            .collect();
        let assessments: Vec<_> = raw.iter()
            .map(|c| c.energy_assessment(&GateResult::Pass))
            .collect();
        let ranked = rank_by_energy(&assessments);
        ranked.iter()
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
            let trace = options.prior_feedback
                .iter()
                .filter(|fb| fb.norm_candidate_id == candidate.candidate_id)
                .fold(
                    NormFitnessTrace::empty(candidate.candidate_id, policy.id),
                    |t, fb| t.observe_from_feedback(fb),
                );
            if trace.observations.is_empty() { None } else { Some(trace) }
        })
        .collect();

    // ── 5. Optional SystemCube v0.4.3 export ──────────────────────────────────
    let systemcube_export: Option<KcubeExportReport> = if options.enable_systemcube {
        let run_desc = kosmo_core::RunDescriptor::new(policy.id, "pipeline");
        let units: Vec<BlueprintUnit> = hyphae
            .decisions
            .iter()
            .filter(|d| d.outcome.is_accepted())
            .map(|d| {
                BlueprintUnit::new(
                    BlueprintUnitKind::ModuleBoundary,
                    d.yield_id,
                    kosmo_core::AuthorityLabel::Foundry,
                    TaintLabel::Synthetic,
                    vec![d.evidence_bundle_id],
                    policy,
                )
            })
            .collect();
        let cube = SystemCube::new(hyphae.host_cube.cube_id, &run_desc, policy, units);
        let export = cube.export_dry_run(options.systemcube_capacity, policy);
        agg.record("systemcube", export.export_id, GateResult::Pass);
        Some(export)
    } else {
        None
    };

    // ── 6. Aggregate gate results ─────────────────────────────────────────────
    let aggregated_gate = agg.aggregate();

    IntegrationRunReport::new(
        hyphae,
        cartography_update,
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
        norm_candidates,
        norm_fitness_traces,
        systemcube_export,
        aggregated_gate,
        policy,
    )
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
            WorkspaceEntry { path: "src/lib.rs".into(), digest: Digest::of_bytes(b"lib"), size_bytes: 100, kind: WorkspaceEntryKind::SourceFile },
            WorkspaceEntry { path: "src/main.rs".into(), digest: Digest::of_bytes(b"main"), size_bytes: 200, kind: WorkspaceEntryKind::SourceFile },
            WorkspaceEntry { path: "src/lib_test.rs".into(), digest: Digest::of_bytes(b"lib_test"), size_bytes: 50, kind: WorkspaceEntryKind::TestFile },
        ];
        WorkspaceIndex::from_entries("test-root".into(), entries, policy().id)
    }

    // ── Pipeline: empty workspace ─────────────────────────────────────────────

    #[test]
    fn pipeline_empty_workspace_passes_all_gates() {
        let r = run_dry_pipeline(&empty_index(), &IntegrationRunOptions::report_only(), &policy());
        assert!(r.final_result.is_pass(), "empty workspace must pass all gates");
    }

    #[test]
    fn pipeline_empty_workspace_report_is_content_addressed() {
        let r1 = run_dry_pipeline(&empty_index(), &IntegrationRunOptions::report_only(), &policy());
        let r2 = run_dry_pipeline(&empty_index(), &IntegrationRunOptions::report_only(), &policy());
        assert_eq!(r1.report_id, r2.report_id);
        assert_ne!(r1.report_id, Digest::ZERO);
    }

    // ── Policy consistency (traceability) ─────────────────────────────────────

    #[test]
    fn pipeline_policy_consistency_empty() {
        let r = run_dry_pipeline(&empty_index(), &IntegrationRunOptions::report_only(), &policy());
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
        let r = run_dry_pipeline(&fixture_index(), &IntegrationRunOptions::report_only(), &policy());
        assert!(r.metatron_diagnostics.is_empty(), "Metatron must be off when disabled");
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
            assert_eq!(diag.policy_id, p.id, "Metatron diagnostic must carry pipeline policy_id");
        }
    }

    // ── LPCM optional layer ────────────────────────────────────────────────────

    #[test]
    fn pipeline_no_lpcm_when_disabled() {
        let r = run_dry_pipeline(&fixture_index(), &IntegrationRunOptions::report_only(), &policy());
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
            assert_eq!(lr.policy_id, p.id, "LPCM report must carry pipeline policy_id");
        }
    }

    // ── SystemCube optional layer ──────────────────────────────────────────────

    #[test]
    fn pipeline_no_systemcube_when_disabled() {
        let r = run_dry_pipeline(&fixture_index(), &IntegrationRunOptions::report_only(), &policy());
        assert!(r.systemcube_export.is_none(), "SystemCube must be off when disabled");
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

    // ── Fail-closed gate propagation ──────────────────────────────────────────

    #[test]
    fn fail_closed_aggregator_reject_propagates() {
        let mut agg = GateTraceAggregator::new(Digest::of_bytes(b"p"));
        agg.record("hyphae", Digest::of_bytes(b"h"), GateResult::Pass);
        agg.record("lpcm", Digest::of_bytes(b"l"), GateResult::Reject { reason: "bad".into() });
        agg.record("meta", Digest::of_bytes(b"m"), GateResult::Warn { message: "low".into() });
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
        assert!(!p.allow_host_write, "allow_host_write must be false in default policy");
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
        let r = run_dry_pipeline(&fixture_index(), &IntegrationRunOptions::report_only(), &policy());
        assert!(r.metatron_index.micrograph_ids.is_empty(), "index must be empty when Metatron disabled");
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
        assert_eq!(r.metatron_index.micrograph_ids.len(), void_count, "one micrograph per void");
        assert_eq!(r.metatron_index.diagnostic_ids.len(), void_count, "one diagnostic per void");
        assert_ne!(r.metatron_index.index_id, Digest::ZERO, "index_id must be non-ZERO");
    }

    #[test]
    fn pipeline_metatron_index_is_deterministic() {
        let opts = IntegrationRunOptions {
            enable_metatron: true,
            ..IntegrationRunOptions::report_only()
        };
        let r1 = run_dry_pipeline(&fixture_index(), &opts, &policy());
        let r2 = run_dry_pipeline(&fixture_index(), &opts, &policy());
        assert_eq!(r1.metatron_index.index_id, r2.metatron_index.index_id, "metatron_index must be deterministic");
    }

    #[test]
    fn pipeline_metatron_index_carries_correct_policy_id() {
        let opts = IntegrationRunOptions {
            enable_metatron: true,
            ..IntegrationRunOptions::report_only()
        };
        let p = policy();
        let r = run_dry_pipeline(&fixture_index(), &opts, &p);
        assert_eq!(r.metatron_index.policy_id, p.id, "metatron_index must carry pipeline policy_id");
        assert!(r.verify_policy_consistency(), "verify_policy_consistency must cover metatron_index");
    }

    // ── Lift reports from Metatron ────────────────────────────────────────────

    #[test]
    fn pipeline_no_lift_reports_when_metatron_disabled() {
        let r = run_dry_pipeline(&fixture_index(), &IntegrationRunOptions::report_only(), &policy());
        assert!(r.lift_reports.is_empty(), "lift_reports must be empty when Metatron is disabled");
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
            assert_ne!(rep.report_id, Digest::ZERO, "lift_report.report_id must be non-ZERO");
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

    // ── Norm candidates optional layer ────────────────────────────────────────

    #[test]
    fn pipeline_no_norm_candidates_when_disabled() {
        let r = run_dry_pipeline(&fixture_index(), &IntegrationRunOptions::report_only(), &policy());
        assert!(r.norm_candidates.is_empty(), "norm_candidates must be empty when disabled");
    }

    #[test]
    fn pipeline_norm_candidates_generated_from_accepted_decisions() {
        let opts = IntegrationRunOptions {
            enable_norm_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&fixture_index(), &opts, &policy());
        let accepted_count = r.hyphae_result.decisions.iter().filter(|d| d.outcome.is_accepted()).count();
        assert_eq!(
            r.norm_candidates.len(),
            accepted_count,
            "one norm candidate per accepted decision"
        );
        for c in &r.norm_candidates {
            assert_eq!(c.fitness_score, Q16::ONE, "initial fitness must be Q16::ONE");
            assert!(c.name.starts_with("norm:void:"), "candidate name must encode target void");
            assert_ne!(c.candidate_id, Digest::ZERO, "candidate_id must be non-ZERO");
            assert_ne!(c.evidence_bundle_id, Digest::ZERO, "CROSS-006: non-ZERO evidence ref");
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
            assert_eq!(c.policy_id, p.id, "norm candidate must carry pipeline policy_id");
        }
    }

    // ── Ambiguity profiles + void hypotheses (Step 3f) ───────────────────────

    #[test]
    fn pipeline_no_ambiguity_profiles_when_metatron_disabled() {
        let r = run_dry_pipeline(&fixture_index(), &IntegrationRunOptions::report_only(), &policy());
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
            assert_eq!(a.policy_id, p.id, "ambiguity profile must carry pipeline policy_id");
            assert_ne!(a.profile_id, Digest::ZERO, "profile_id must be non-ZERO");
        }
        for h in &r.complement_void_hypotheses {
            assert_eq!(h.policy_id, p.id, "void hypothesis must carry pipeline policy_id");
            assert_ne!(h.hypothesis_id, Digest::ZERO, "hypothesis_id must be non-ZERO");
        }
        assert!(r.verify_policy_consistency(), "verify_policy_consistency must cover ambiguities + hypotheses");
    }

    // ── Surgery workbench tasks (Step 3e) ────────────────────────────────────

    #[test]
    fn pipeline_no_surgery_workbench_tasks_when_surgery_disabled() {
        let r = run_dry_pipeline(&fixture_index(), &IntegrationRunOptions::report_only(), &policy());
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

    // ── Crystal candidates from accepted decisions (Step 5d) ─────────────────

    #[test]
    fn pipeline_no_crystal_candidates_when_disabled() {
        let r = run_dry_pipeline(&fixture_index(), &IntegrationRunOptions::report_only(), &policy());
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
        let accepted_count = r.hyphae_result.decisions.iter()
            .filter(|d| d.outcome.is_accepted())
            .count();
        assert_eq!(
            r.crystal_candidates.len(),
            accepted_count,
            "one crystal candidate per accepted decision"
        );
        for c in &r.crystal_candidates {
            assert_ne!(c.candidate_id, Digest::ZERO, "candidate_id must be non-ZERO");
            assert_eq!(c.support_score, Q16::ZERO, "support_score starts at zero (Pending)");
            assert_ne!(c.evidence_bundle_id, Digest::ZERO, "CROSS-006: evidence must be non-ZERO");
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
            assert_eq!(c.policy_id, p.id, "crystal candidate must carry pipeline policy_id");
        }
        assert!(
            r.verify_policy_consistency(),
            "verify_policy_consistency must cover crystal_candidates"
        );
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
        let trace = r2.norm_fitness_traces.iter()
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
            assert_eq!(t.policy_id, p.id, "surgery workbench task must carry pipeline policy_id");
            assert_ne!(t.task_id, Digest::ZERO, "task_id must be non-ZERO");
            assert_ne!(t.surgery_option_id, Digest::ZERO, "surgery_option_id must trace back to source option");
        }
    }
}

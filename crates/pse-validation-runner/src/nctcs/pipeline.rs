//! NCTCS closure pipeline — §7 Referenzfluss.
//!
//! Orchestrates the canonical 12-step closure chain without introducing
//! new engines or commit paths. All inputs are pre-validated artifacts.

use pse_traverse::dynamic_state::CanonicalNumber;

use super::candidate_audit::CandidateFormationAudit;
use super::conformance::{classify_conformance, NctcsConformanceClass};
use super::macro_control_state::{AggregateGateStatus, MacroControlState};
use super::materialization_audit::MaterializationAudit;
use super::null_center::{NullCenterRef, NullProjectionAudit, ToroidalPhaseFlowAudit};
use super::phase_visibility::PhaseVisibilityAudit;
use super::primitives::{content_address, Hash256, NctcsError};
use super::report::NctcsClosureBundle;
use super::run_descriptor::NctcsRunDescriptor;
use super::trace_replay_contract::TraceReplayContractReport;

/// All artifact hashes that drive the closure.
#[derive(Clone, Debug)]
pub struct NctcsClosureInput {
    /// Hash of the ValidationRunBundle.
    pub validation_bundle_hash: Hash256,
    /// Hash of the EvaluationSummaryReport (optional).
    pub eval_summary_hash: Option<Hash256>,
    /// Hash of the StitchCycleBundle (optional).
    pub stitch_bundle_hash: Option<Hash256>,
    /// Hash of the TptMtlBundle (optional).
    pub tpt_bundle_hash: Option<Hash256>,
    /// Pre-computed phase flow hash.
    pub phase_flow_hash: Hash256,
    /// Current tick.
    pub tick: u64,
    /// Hash of the current θt phase state.
    pub theta_t_hash: Hash256,
    /// Hashes of cells visible in the current phase window.
    pub visible_cell_refs: Vec<Hash256>,
    /// Hashes of cells outside the current phase window.
    pub invisible_cell_refs: Vec<Hash256>,
    /// Hashes of candidates that passed formation.
    pub candidate_refs: Vec<Hash256>,
    /// Hashes of candidates that were rejected.
    pub rejected_refs: Vec<Hash256>,
    /// Coverage values (0–100 fixed-point scale 2).
    pub activation_gate_coverage: CanonicalNumber,
    pub boundary_ready_coverage: CanonicalNumber,
    pub evidence_ready_coverage: CanonicalNumber,
    pub trace_replay_ready_coverage: CanonicalNumber,
    /// Materialization hashes.
    pub ephemeral_fabric_hash: Hash256,
    pub tensor_before_hash: Hash256,
    pub stitch_candidate_hashes: Vec<Hash256>,
    pub accepted_update_hashes: Vec<Hash256>,
    pub rejected_candidate_hashes: Vec<Hash256>,
    pub gate_report_hashes: Vec<Hash256>,
    pub tensor_after_hash: Hash256,
    /// Trace/replay contract inputs.
    pub trace_head: Hash256,
    pub evidence_root: Hash256,
    pub gate_history_root: Hash256,
    pub replay_manifest_hash: Hash256,
    /// Whether real domain validation was executed.
    pub has_domain_validation: bool,
    /// Hash of the domain validation result (if present).
    pub domain_validation_hash: Option<Hash256>,
}

/// Execute the NCTCS closure chain (§7.1 steps 1–12).
pub fn close_nctcs(
    rd: NctcsRunDescriptor,
    input: NctcsClosureInput,
) -> Result<NctcsClosureBundle, NctcsError> {
    let rd_hash = rd.content_hash()?;

    // Step 3: Resolve NullCenter.
    let vb_hash = rd.source_validation_bundle_hash.clone();
    let null_center = NullCenterRef::new(vb_hash.clone(), vb_hash);

    // Step 4: NullProjectionAudit.
    let theta_t = input.theta_t_hash.clone();
    let phase_manifold_hash = content_address(&theta_t)?;
    let projected_phase_hash = content_address(&(&theta_t, input.tick))?;
    let projection = NullProjectionAudit::build(
        null_center.null_center_id.clone(),
        projected_phase_hash,
        5,
        phase_manifold_hash,
    )?;

    // Step 5: ToroidalPhaseFlowAudit.
    let phase_flow = ToroidalPhaseFlowAudit::build(
        input.phase_flow_hash.clone(),
        input.tick,
        theta_t.clone(),
        true, // visibility_timing_only
    )?;

    // Step 6: PhaseVisibilityAudit.
    let visibility = PhaseVisibilityAudit::build(
        theta_t,
        input.visible_cell_refs.clone(),
        input.invisible_cell_refs.clone(),
        &rd.thresholds,
    )?;

    // Step 7: CandidateFormationAudit.
    let candidates = CandidateFormationAudit::build(
        input.candidate_refs,
        input.rejected_refs,
        visibility.passed,
        input.activation_gate_coverage,
        input.boundary_ready_coverage,
        input.evidence_ready_coverage,
        input.trace_replay_ready_coverage,
    )?;

    // Step 8: MaterializationAudit.
    let materialization = MaterializationAudit::build(
        input.ephemeral_fabric_hash,
        input.tensor_before_hash,
        input.stitch_candidate_hashes,
        input.accepted_update_hashes,
        input.rejected_candidate_hashes,
        input.gate_report_hashes,
        input.tensor_after_hash.clone(),
        true,
    )?;

    // Step 9: TraceReplayContractReport.
    let canonicalization_hash = content_address(&rd.canonicalization_version)?;
    let replay_identity = CanonicalNumber::FixedI64 {
        raw: 10000,
        scale: 2,
    };
    let trace_replay = TraceReplayContractReport::build(
        input.trace_head,
        input.evidence_root,
        input.gate_history_root,
        input.replay_manifest_hash,
        canonicalization_hash,
        replay_identity,
        rd.policies.require_tensor_history_for_c4,
    )?;

    // Step 10–11: Classify conformance (first pass, macro_state not yet built).
    let conformance_pre = classify_conformance(
        &null_center,
        &projection,
        &visibility,
        &candidates,
        &materialization,
        &trace_replay,
        false,
        input.has_domain_validation,
        rd_hash.clone(),
    )?;

    // Step 10: Build MacroControlState if C3+ is reached.
    let macro_state = if conformance_pre.reached_class >= NctcsConformanceClass::C3AuditableTensor {
        let gate_coverage = CanonicalNumber::FixedI64 {
            raw: 10000,
            scale: 2,
        };
        let agg = AggregateGateStatus {
            total_gates: 1,
            passed_gates: 1,
            failed_gates: 0,
            held_gates: 0,
            quarantined_candidates: 0,
            no_update_count: 0,
            gate_coverage,
        };
        Some(MacroControlState::build(
            rd.run_id.clone(),
            null_center.null_center_id.clone(),
            Some(input.tensor_after_hash),
            trace_replay.trace_head.clone(),
            None,
            vec![],
            vec![],
            None,
            agg,
            conformance_pre.reached_class,
            conformance_pre.validation_status,
            input.eval_summary_hash,
            input.domain_validation_hash,
            vec![],
        )?)
    } else {
        None
    };

    // Re-classify with definitive macro_state knowledge.
    let has_macro = macro_state.is_some();
    let conformance = classify_conformance(
        &null_center,
        &projection,
        &visibility,
        &candidates,
        &materialization,
        &trace_replay,
        has_macro,
        input.has_domain_validation,
        rd_hash,
    )?;

    // Step 12: Build NctcsClosureBundle.
    NctcsClosureBundle::build(
        rd,
        null_center,
        projection,
        phase_flow,
        visibility,
        candidates,
        materialization,
        trace_replay,
        macro_state,
        conformance,
    )
}

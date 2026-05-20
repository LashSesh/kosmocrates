//! Metatron closure pipeline — §5 Canonical Closure Chain.
//!
//! ValidationRun → NCTCSReport → MacroControlState
//!   → LocalMonolithProjection* → IsomorphicProjectionReport*
//!   → SpectralGapStitchReport → MetatronGate → HolisticEigenmodeState
//!
//! The operator MUST NOT mutate any input artifact.

use super::gates::MetatronGateReport;
use super::holistic_state::{
    AggregateGateStatus, HolisticEigenmodeState, MetatronConformanceClass, MetatronValidationStatus,
};
use super::isomorphic::IsomorphicProjectionReport;
use super::local_projection::LocalMonolithProjection;
use super::primitives::{EvidenceRef, Fixed, Hash256, MetatronError, NctcsClass};
use super::run_descriptor::MetatronRunDescriptor;
use super::spectral_gap::SpectralGapStitchReport;

/// All artifact references needed for one closure run.
pub struct MetatronClosureInput {
    pub nctcs_report_ref: Hash256,
    pub validation_report_ref: Hash256,
    pub null_center_ref: Hash256,
    pub field_tensor_ref: Hash256,
    pub macro_control_state_ref: Hash256,
    pub trace_head: Hash256,
    pub nctcs_class: NctcsClass,
    pub local_projections: Vec<LocalMonolithProjection>,
    pub isomorphic_reports: Vec<IsomorphicProjectionReport>,
    pub spectral_gap_report: SpectralGapStitchReport,
    pub replay_identity: Fixed,
    pub eval_status_ok: bool,
    pub unexplained_drift: Fixed,
    pub global_coherence: Fixed,
    pub evidence_refs: Vec<EvidenceRef>,
}

/// Outcome of a Metatron closure run.
pub enum MetatronClosureOutcome {
    /// Gate passed — HolisticEigenmodeState produced.
    Closed(HolisticEigenmodeState),
    /// Gate failed — only diagnostic report produced.
    Diagnostic(MetatronDiagnosticReport),
    /// Pre-flight failed — run was rejected before gate evaluation.
    Rejected(MetatronRejectionReport),
}

/// Diagnostic report (fail-closed: gate failed, no productive state).
pub struct MetatronDiagnosticReport {
    pub gate_report: MetatronGateReport,
    pub local_projections: Vec<LocalMonolithProjection>,
    pub isomorphism_reports: Vec<IsomorphicProjectionReport>,
    pub spectral_gap_report: SpectralGapStitchReport,
}

/// Pre-flight rejection report.
pub struct MetatronRejectionReport {
    pub reason: String,
}

/// Execute the Metatron closure chain.
///
/// MUST NOT mutate any input artifact. Only reads validated artifacts and
/// produces a content-addressed HolisticEigenmodeState on gate pass.
pub fn run_metatron_closure(
    rd: MetatronRunDescriptor,
    input: MetatronClosureInput,
) -> Result<MetatronClosureOutcome, MetatronError> {
    // Pre-flight: verify required inputs are present.
    if rd.policies.reject_on_missing_trace && input.trace_head == Hash256::zero() {
        return Ok(MetatronClosureOutcome::Rejected(MetatronRejectionReport {
            reason: "trace_head is zero (missing trace)".into(),
        }));
    }

    // Collect local projections (already validated externally).
    let local = input.local_projections;
    let iso = input.isomorphic_reports;
    let gap = input.spectral_gap_report;

    // Evaluate MetatronGate (G_meta = G_nctcs ∧ G_trace ∧ ... ∧ G_drift).
    let has_trace_refs = local.iter().all(|p| p.trace_ref != Hash256::zero());
    let gate = MetatronGateReport::evaluate(
        input.nctcs_class,
        rd.thresholds.min_nctcs_class,
        has_trace_refs,
        &input.replay_identity,
        &rd.thresholds.min_replay_identity,
        &iso,
        &gap,
        input.eval_status_ok,
        &input.unexplained_drift,
        &rd.thresholds.max_unexplained_drift,
        input.evidence_refs.clone(),
    )?;

    // Fail-closed: no productive state without gate pass.
    if !gate.passed {
        return Ok(MetatronClosureOutcome::Diagnostic(
            MetatronDiagnosticReport {
                gate_report: gate,
                local_projections: local,
                isomorphism_reports: iso,
                spectral_gap_report: gap,
            },
        ));
    }

    // Gate passed — build HolisticEigenmodeState.
    let local_refs: Vec<Hash256> = local.iter().map(|p| p.projection_id.clone()).collect();
    let iso_refs: Vec<Hash256> = iso.iter().map(|r| r.report_id.clone()).collect();

    let agg = AggregateGateStatus {
        total_gates: 7,
        passed_gates: 7,
        failed_gates: 0,
    };

    let conformance_class = derive_conformance_class(&local, &iso, &gap, &gate);

    let state = HolisticEigenmodeState::build(
        rd.parent_validation_run_id.clone(),
        input.null_center_ref,
        input.field_tensor_ref,
        input.macro_control_state_ref,
        input.trace_head,
        local_refs,
        iso_refs,
        gap.report_id.clone(),
        agg,
        input.global_coherence,
        conformance_class,
        MetatronValidationStatus::Closed,
        input.evidence_refs,
    )?;

    Ok(MetatronClosureOutcome::Closed(state))
}

fn derive_conformance_class(
    local: &[LocalMonolithProjection],
    iso: &[IsomorphicProjectionReport],
    gap: &SpectralGapStitchReport,
    gate: &MetatronGateReport,
) -> MetatronConformanceClass {
    // M4+: gate passed, all iso reports passed, gap preserved
    if gate.passed && iso.iter().all(|r| r.passed) && gap.improved_or_preserved {
        MetatronConformanceClass::M4
    } else if !local.is_empty() && iso.iter().any(|r| r.passed) {
        MetatronConformanceClass::M3
    } else if !iso.is_empty() {
        MetatronConformanceClass::M2
    } else if !local.is_empty() {
        MetatronConformanceClass::M1
    } else {
        MetatronConformanceClass::M0
    }
}

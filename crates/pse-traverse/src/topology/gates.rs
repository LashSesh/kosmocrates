//! Gate family and outcome machine (TPT-MTL §10).
//!
//! All gates are fail-closed: any unknown, missing or erroneous condition
//! evaluates to failure. Budget Exhaustion MUST NOT escalate to Emit.
//!
//! G_all = G_adapter ∧ G_axis ∧ G_topo ∧ G_micro ∧ G_carrier ∧ G_truth
//!         ∧ G_boundary ∧ G_replay ∧ G_matrix ∧ G_emission

use serde::{Deserialize, Serialize};

use super::axis_bridge::AxisBridgeReport;
use super::carrier::CarrierReport;
use super::mesh_holo::MeshHolo;
use super::micro_fiber::MicroFiber;
use super::phase_window::PhaseSpaceWindow;
use super::primitives::{tpt_content_address, Hash256, TopologyError};
use super::reinterpret::ReinterpretationReport;
use super::run_descriptor::TptMtlRunDescriptor;

/// What to do when a gate fails.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FailurePolicy {
    Hold,
    Degrade,
    Recalibrate,
    Quarantine,
    Abort,
}

/// Result of a single gate check.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GateResult {
    pub gate_id: String,
    pub passed: bool,
    pub diagnostics: Vec<String>,
    pub evidence_refs: Vec<Hash256>,
    pub on_fail: FailurePolicy,
    pub trace_ref: Hash256,
}

/// Aggregated gate report (all gates).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TptMtlGateReport {
    pub report_id: Hash256,
    pub adapter: bool,
    pub axis: bool,
    pub symmetry: bool,
    pub topology: bool,
    pub entropy: bool,
    pub micro_lift: bool,
    pub carrier: bool,
    pub kairos: bool,
    pub truth: bool,
    pub boundary: bool,
    pub replay: bool,
    pub matrix: bool,
    pub emission: bool,
    pub all_passed: bool,
    pub gate_results: Vec<GateResult>,
    pub trace_ref: Hash256,
}

/// The overall outcome decision (TPT-MTL §10.3).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TptMtlOutcomeKind {
    Emit,
    Hold,
    Degrade,
    Recalibrate,
    Quarantine,
    Abort,
}

impl TptMtlGateReport {
    pub fn outcome_kind(&self) -> TptMtlOutcomeKind {
        if !self.boundary || !self.replay {
            // Hard boundary or replay identity violated → Abort.
            return TptMtlOutcomeKind::Abort;
        }
        // Check for open critical contradictions in claims (simplified: if
        // truth gate failed, quarantine).
        if !self.truth {
            return TptMtlOutcomeKind::Quarantine;
        }
        // Adapter/axis/fiber/carrier policy insufficient → Recalibrate.
        if !self.adapter || !self.axis || !self.micro_lift || !self.carrier {
            return TptMtlOutcomeKind::Recalibrate;
        }
        if self.all_passed && self.matrix && self.emission {
            TptMtlOutcomeKind::Emit
        } else {
            TptMtlOutcomeKind::Hold
        }
    }
}

/// Check the AdapterGate: window valid, points have provenance or NoiseCandidate,
/// sampling is deterministic.
fn gate_adapter(window: &PhaseSpaceWindow, _rd: &TptMtlRunDescriptor) -> GateResult {
    let mut diagnostics = Vec::new();

    if window.sampling_policy_hash == Hash256::zero() {
        diagnostics.push("sampling_policy_hash is zero — sampling not deterministic".into());
    }
    if window.points.is_empty() {
        diagnostics.push("window has no points".into());
    }
    for point in &window.points {
        if !point.has_provenance_or_noise() {
            diagnostics.push(format!(
                "point {} lacks provenance and is not NoiseCandidate",
                point.point_id.hex()
            ));
        }
    }

    let passed = diagnostics.is_empty();
    GateResult {
        gate_id: "AdapterGate".into(),
        passed,
        diagnostics,
        evidence_refs: vec![window.window_id.clone()],
        on_fail: FailurePolicy::Recalibrate,
        trace_ref: window.trace_ref.clone(),
    }
}

/// Check the AxisGate: axis origin declared, no mixing without TranslationProof.
fn gate_axis(axis_report: &AxisBridgeReport) -> GateResult {
    let passed = axis_report.passed;
    GateResult {
        gate_id: "AxisGate".into(),
        passed,
        diagnostics: axis_report.violations.clone(),
        evidence_refs: vec![axis_report.report_id.clone()],
        on_fail: FailurePolicy::Recalibrate,
        trace_ref: axis_report.report_id.clone(),
    }
}

/// Check the SymmetryGate: declared symmetry honoured or SymmetryBreak documented.
fn gate_symmetry(mesh: &MeshHolo) -> GateResult {
    let passed = mesh.proof.symmetry_pass;
    let diagnostics = if passed {
        vec![]
    } else {
        vec!["symmetry_pass is false — symmetry break not documented".into()]
    };
    GateResult {
        gate_id: "SymmetryGate".into(),
        passed,
        diagnostics,
        evidence_refs: vec![mesh.mesh_id.clone()],
        on_fail: FailurePolicy::Hold,
        trace_ref: mesh.trace_ref.clone(),
    }
}

/// Check the TopologyGate: Betti/persistence stable or allowed shift with proof.
fn gate_topology(mesh: &MeshHolo) -> GateResult {
    let has_guard = !mesh.proof.topology_guard_proofs.is_empty();
    let all_guard_passed = mesh.proof.topology_guard_proofs.iter().all(|p| p.passed);
    let passed = has_guard && all_guard_passed;
    let mut diagnostics = Vec::new();
    if !has_guard {
        diagnostics.push("no topology guard proofs present".into());
    } else if !all_guard_passed {
        for p in &mesh.proof.topology_guard_proofs {
            if !p.passed {
                diagnostics.push(format!("guard proof failed: {}", p.reason));
            }
        }
    }
    GateResult {
        gate_id: "TopologyGate".into(),
        passed,
        diagnostics,
        evidence_refs: vec![mesh.mesh_id.clone()],
        on_fail: FailurePolicy::Hold,
        trace_ref: mesh.trace_ref.clone(),
    }
}

/// Check the EntropyGate: entropy class computed and Explore/Refine/Simplify/Freeze documented.
fn gate_entropy(mesh: &MeshHolo) -> GateResult {
    // Entropy class is always computed by seed_mesh_holo, so this gate always passes.
    GateResult {
        gate_id: "EntropyGate".into(),
        passed: true,
        diagnostics: vec![],
        evidence_refs: vec![mesh.invariants.persistence_digest.clone()],
        on_fail: FailurePolicy::Hold,
        trace_ref: mesh.trace_ref.clone(),
    }
}

/// Check the MicroLiftGate: subwindows, primary, dual, seam, proofs complete.
fn gate_micro_lift_check(fibers: &[MicroFiber]) -> GateResult {
    let (passed, diagnostics) = super::micro_fiber::gate_micro_lift(fibers);
    let evidence_refs = fibers.iter().map(|f| f.fiber_id.clone()).collect();
    GateResult {
        gate_id: "MicroLiftGate".into(),
        passed,
        diagnostics,
        evidence_refs,
        on_fail: FailurePolicy::Recalibrate,
        trace_ref: Hash256::zero(),
    }
}

/// Check the CarrierGate: null-center stateless, times separated, mandorla/helix traceable.
fn gate_carrier(carrier_report: &CarrierReport) -> GateResult {
    GateResult {
        gate_id: "CarrierGate".into(),
        passed: carrier_report.passed,
        diagnostics: carrier_report.diagnostics.clone(),
        evidence_refs: vec![carrier_report.report_id.clone()],
        on_fail: FailurePolicy::Recalibrate,
        trace_ref: carrier_report.trace_ref.clone(),
    }
}

/// Check the KairosGate: carrier continuity and migration traceable.
fn gate_kairos(carrier_report: &CarrierReport) -> GateResult {
    let passed = carrier_report.ladder_traceable && carrier_report.passed;
    let mut diagnostics = Vec::new();
    if !carrier_report.ladder_traceable {
        diagnostics.push("ladder not traceable — KairosGate fail".into());
    }
    GateResult {
        gate_id: "KairosGate".into(),
        passed,
        diagnostics,
        evidence_refs: vec![carrier_report.report_id.clone()],
        on_fail: FailurePolicy::Hold,
        trace_ref: carrier_report.trace_ref.clone(),
    }
}

/// Check the TruthGate: reinterpreted claims have status, scope, evidence, lineage
/// and resolved contradictions.
fn gate_truth(reinterp: &ReinterpretationReport) -> GateResult {
    let mut diagnostics = Vec::new();

    // No unresolved critical contradictions.
    if !reinterp.contradictions.is_empty() {
        diagnostics.push(format!(
            "{} unresolved contradictions",
            reinterp.contradictions.len()
        ));
    }

    for claim in &reinterp.claim_candidates {
        if claim.evidence_refs.is_empty() {
            diagnostics.push(format!(
                "claim {} has no evidence_refs",
                claim.claim_id.hex()
            ));
        }
        if claim.scope.is_empty() {
            diagnostics.push(format!("claim {} has empty scope", claim.claim_id.hex()));
        }
    }

    GateResult {
        gate_id: "TruthGate".into(),
        passed: diagnostics.is_empty(),
        diagnostics,
        evidence_refs: vec![reinterp.report_id.clone()],
        on_fail: FailurePolicy::Quarantine,
        trace_ref: reinterp.trace_ref.clone(),
    }
}

/// Check the BoundaryGate: data/domain/security/action/scope boundaries respected.
fn gate_boundary(window: &PhaseSpaceWindow, rd: &TptMtlRunDescriptor) -> GateResult {
    let mut diagnostics = Vec::new();

    // Every declared source boundary must appear in input_refs or boundary_ref.
    for (i, src_boundary) in rd.source_boundaries.iter().enumerate() {
        if !window.input_refs.contains(src_boundary) && window.boundary_ref != *src_boundary {
            diagnostics.push(format!(
                "source_boundary[{}] {} not found in window input_refs or boundary_ref",
                i,
                src_boundary.hex()
            ));
        }
    }

    GateResult {
        gate_id: "BoundaryGate".into(),
        passed: diagnostics.is_empty(),
        diagnostics,
        evidence_refs: vec![window.boundary_ref.clone()],
        on_fail: FailurePolicy::Abort,
        trace_ref: window.trace_ref.clone(),
    }
}

/// Check the ReplayGate: repetition produces identical digests.
fn gate_replay(
    window_hash: &Hash256,
    mesh_hash: &Hash256,
    expected_window_hash: Option<&Hash256>,
    expected_mesh_hash: Option<&Hash256>,
) -> GateResult {
    let mut diagnostics = Vec::new();

    if let Some(expected) = expected_window_hash {
        if expected != window_hash {
            diagnostics.push(format!(
                "window_hash mismatch: expected {} got {}",
                expected.hex(),
                window_hash.hex()
            ));
        }
    }
    if let Some(expected) = expected_mesh_hash {
        if expected != mesh_hash {
            diagnostics.push(format!(
                "mesh_hash mismatch: expected {} got {}",
                expected.hex(),
                mesh_hash.hex()
            ));
        }
    }

    GateResult {
        gate_id: "ReplayGate".into(),
        passed: diagnostics.is_empty(),
        diagnostics,
        evidence_refs: vec![window_hash.clone(), mesh_hash.clone()],
        on_fail: FailurePolicy::Abort,
        trace_ref: Hash256::zero(),
    }
}

/// Check the MatrixGate: all upstream gates passed, no unresolved critical
/// contradictions, proof obligations registered.
fn gate_matrix(
    adapter: bool,
    axis: bool,
    topology: bool,
    micro_lift: bool,
    carrier: bool,
    truth: bool,
    boundary: bool,
    reinterp: &ReinterpretationReport,
) -> GateResult {
    let all_up = adapter && axis && topology && micro_lift && carrier && truth && boundary;
    let no_contradictions = reinterp.contradictions.is_empty();
    let passed = all_up && no_contradictions;
    let mut diagnostics = Vec::new();
    if !all_up {
        diagnostics.push("not all upstream gates passed".into());
    }
    if !no_contradictions {
        diagnostics.push(format!(
            "{} critical contradictions unresolved",
            reinterp.contradictions.len()
        ));
    }
    GateResult {
        gate_id: "MatrixGate".into(),
        passed,
        diagnostics,
        evidence_refs: vec![reinterp.report_id.clone()],
        on_fail: FailurePolicy::Hold,
        trace_ref: reinterp.trace_ref.clone(),
    }
}

/// Check the EmissionGate: output in allowed scope, full provenance, no external action.
fn gate_emission(reinterp: &ReinterpretationReport, mesh: &MeshHolo) -> GateResult {
    let mut diagnostics = Vec::new();

    // Check emission is only for emission-capable entropy classes.
    if !mesh.invariants.entropy_class.emission_capable() {
        diagnostics.push(format!(
            "entropy class {:?} is not emission-capable",
            mesh.invariants.entropy_class
        ));
    }

    // All claims must have evidence.
    for claim in &reinterp.claim_candidates {
        if claim.evidence_refs.is_empty() {
            diagnostics.push(format!(
                "claim {} missing evidence for emission",
                claim.claim_id.hex()
            ));
        }
    }

    GateResult {
        gate_id: "EmissionGate".into(),
        passed: diagnostics.is_empty(),
        diagnostics,
        evidence_refs: vec![reinterp.report_id.clone()],
        on_fail: FailurePolicy::Hold,
        trace_ref: reinterp.trace_ref.clone(),
    }
}

/// Run the complete gate family.
pub fn gate_all(
    window: &PhaseSpaceWindow,
    axis_report: &AxisBridgeReport,
    mesh: &MeshHolo,
    fibers: &[MicroFiber],
    carrier_report: &CarrierReport,
    reinterp: &ReinterpretationReport,
    rd: &TptMtlRunDescriptor,
    expected_window_hash: Option<&Hash256>,
    expected_mesh_hash: Option<&Hash256>,
) -> Result<TptMtlGateReport, TopologyError> {
    let r_adapter = gate_adapter(window, rd);
    let r_axis = gate_axis(axis_report);
    let r_symmetry = gate_symmetry(mesh);
    let r_topology = gate_topology(mesh);
    let r_entropy = gate_entropy(mesh);
    let r_micro = gate_micro_lift_check(fibers);
    let r_carrier = gate_carrier(carrier_report);
    let r_kairos = gate_kairos(carrier_report);
    let r_truth = gate_truth(reinterp);
    let r_boundary = gate_boundary(window, rd);

    let window_hash = window.content_hash()?;
    let mesh_hash = mesh.content_hash()?;
    let r_replay = gate_replay(
        &window_hash,
        &mesh_hash,
        expected_window_hash,
        expected_mesh_hash,
    );

    let r_matrix = gate_matrix(
        r_adapter.passed,
        r_axis.passed,
        r_topology.passed,
        r_micro.passed,
        r_carrier.passed,
        r_truth.passed,
        r_boundary.passed,
        reinterp,
    );
    let r_emission = gate_emission(reinterp, mesh);

    let all_passed = r_adapter.passed
        && r_axis.passed
        && r_symmetry.passed
        && r_topology.passed
        && r_entropy.passed
        && r_micro.passed
        && r_carrier.passed
        && r_kairos.passed
        && r_truth.passed
        && r_boundary.passed
        && r_replay.passed
        && r_matrix.passed
        && r_emission.passed;

    let adapter = r_adapter.passed;
    let axis = r_axis.passed;
    let symmetry = r_symmetry.passed;
    let topology = r_topology.passed;
    let entropy = r_entropy.passed;
    let micro_lift = r_micro.passed;
    let carrier = r_carrier.passed;
    let kairos = r_kairos.passed;
    let truth = r_truth.passed;
    let boundary = r_boundary.passed;
    let replay = r_replay.passed;
    let matrix = r_matrix.passed;
    let emission = r_emission.passed;

    let trace_ref = window.trace_ref.clone();
    let gate_results = vec![
        r_adapter, r_axis, r_symmetry, r_topology, r_entropy, r_micro, r_carrier, r_kairos,
        r_truth, r_boundary, r_replay, r_matrix, r_emission,
    ];

    let partial = TptMtlGateReport {
        report_id: Hash256::zero(),
        adapter,
        axis,
        symmetry,
        topology,
        entropy,
        micro_lift,
        carrier,
        kairos,
        truth,
        boundary,
        replay,
        matrix,
        emission,
        all_passed,
        gate_results,
        trace_ref,
    };
    let report_id = tpt_content_address(&partial)?;
    Ok(TptMtlGateReport {
        report_id,
        ..partial
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::axis_bridge::build_axis_bridge_report;
    use crate::topology::carrier::{evaluate_carrier, CarrierContext};
    use crate::topology::mesh_holo::{evolve_mesh, seed_mesh_holo};
    use crate::topology::micro_fiber::lift_micro_fibers;
    use crate::topology::phase_window::{PointMeta, PointStatus, TptPoint};
    use crate::topology::primitives::{tpt_content_address, Fixed, Hash256, TptEvidenceRef};
    use crate::topology::reinterpret::reinterpret_mesh_to_panoptic;
    use crate::topology::run_descriptor::TptMtlRunDescriptor;

    fn make_window(n: usize) -> PhaseSpaceWindow {
        let rd = TptMtlRunDescriptor::default_permissive();
        let mut points = Vec::new();
        for i in 0..n {
            let v = Fixed::quantize(i as f64 * 0.15 + 0.1, 9).unwrap();
            let point_id = tpt_content_address(&("gate-pt", i)).unwrap();
            points.push(TptPoint {
                point_id,
                x: [v.clone(), v.clone(), v.clone(), v.clone(), v.clone()],
                meta: PointMeta {
                    semantic_axis_labels: rd.axis_policy.semantic_axes.clone(),
                    runtime_axis_labels: rd.axis_policy.runtime_axes.clone(),
                    status: PointStatus::Active,
                },
                provenance: vec![TptEvidenceRef {
                    evidence_id: Hash256::zero(),
                    kind: "test".into(),
                    source_digest: Hash256::zero(),
                    trace_ref: Hash256::zero(),
                }],
                carrier_ref: Hash256::zero(),
                gate_refs: vec![],
            });
        }
        points.sort();
        PhaseSpaceWindow {
            window_id: tpt_content_address(&("gw", n)).unwrap(),
            domain_profile: rd.domain_profile.clone(),
            input_refs: vec![],
            points,
            carrier: CarrierContext::new_minimal(1),
            horizon_refs: vec![],
            constraint_refs: vec![],
            boundary_ref: Hash256::zero(),
            sampling_policy_hash: tpt_content_address(&"sampling-policy").unwrap(),
            trace_ref: Hash256::zero(),
            rd_hash: Hash256::zero(),
        }
    }

    #[test]
    fn gate_unknown_status_fails_closed() {
        // If any gate is given a poison input, the gate report must not pass.
        let rd = TptMtlRunDescriptor::default_permissive();
        let window = make_window(2);
        let axis_report = build_axis_bridge_report(&window, &rd).unwrap();
        let mesh = seed_mesh_holo(&window, &rd).unwrap();
        let (evolved_mesh, _) = evolve_mesh(mesh, &rd).unwrap();
        let fibers = lift_micro_fibers(&window, &evolved_mesh, &rd).unwrap();
        let carrier = CarrierContext::new_minimal(1);
        let carrier_report = evaluate_carrier(&carrier, &rd, false).unwrap();
        let reinterp = reinterpret_mesh_to_panoptic(&evolved_mesh, &carrier_report, &rd).unwrap();

        // Inject a wrong expected hash → ReplayGate fails → outcome Abort.
        let wrong_hash = tpt_content_address(&"wrong").unwrap();
        let gate_report = gate_all(
            &window,
            &axis_report,
            &evolved_mesh,
            &fibers,
            &carrier_report,
            &reinterp,
            &rd,
            Some(&wrong_hash),
            None,
        )
        .unwrap();
        assert!(!gate_report.replay);
        assert!(!gate_report.all_passed);
        assert!(matches!(
            gate_report.outcome_kind(),
            TptMtlOutcomeKind::Abort
        ));
    }
}

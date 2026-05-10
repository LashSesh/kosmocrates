//! TopologicalCrystalCandidate and TptMtlBundle (TPT-MTL §12).
//!
//! Only gate-passed candidates may be materialized. The Bundle is the
//! replayable artefact set.

use serde::{Deserialize, Serialize};

use super::axis_bridge::AxisBridgeReport;
use super::carrier::CarrierReport;
use super::gates::{TptMtlGateReport, TptMtlOutcomeKind};
use super::mesh_holo::MeshHolo;
use super::micro_fiber::MicroFiber;
use super::phase_window::PhaseSpaceWindow;
use super::primitives::{tpt_content_address, Hash256, TptEvidenceRef, TopologyError};
use super::reinterpret::ReinterpretationReport;
use super::run_descriptor::TptMtlRunDescriptor;

/// Status of a TopologicalCrystalCandidate.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CandidateStatus {
    Draft,
    Hold,
    Quarantined,
    GatePassed,
    Emitted,
}

/// A topology-layer crystal candidate (TPT-MTL §12.1).
///
/// MUST NOT be a SemanticCrystal. Does not trigger external commits.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TopologicalCrystalCandidate {
    pub candidate_id: Hash256,
    pub window_ref: Hash256,
    pub mesh_ref: Hash256,
    pub micro_fiber_set_ref: Hash256,
    pub carrier_report_ref: Hash256,
    pub reinterpretation_report_ref: Hash256,
    pub gate_report_ref: Hash256,
    pub evidence_refs: Vec<TptEvidenceRef>,
    pub status: CandidateStatus,
    pub trace_ref: Hash256,
}

/// Form a TopologicalCrystalCandidate from the pipeline outputs.
pub fn form_topological_crystal_candidate(
    window: &PhaseSpaceWindow,
    mesh: &MeshHolo,
    fibers: &[MicroFiber],
    carrier_report: &CarrierReport,
    reinterp: &ReinterpretationReport,
    gate_report: &TptMtlGateReport,
) -> Result<TopologicalCrystalCandidate, TopologyError> {
    let micro_fiber_set_ref = tpt_content_address(&fibers.to_vec())?;

    let status = match gate_report.outcome_kind() {
        TptMtlOutcomeKind::Emit => CandidateStatus::GatePassed,
        TptMtlOutcomeKind::Quarantine => CandidateStatus::Quarantined,
        _ => CandidateStatus::Hold,
    };

    let evidence_refs = reinterp
        .claim_candidates
        .iter()
        .flat_map(|c| &c.evidence_refs)
        .map(|h| TptEvidenceRef {
            evidence_id: h.clone(),
            kind: "claim-evidence".into(),
            source_digest: reinterp.report_id.clone(),
            trace_ref: window.trace_ref.clone(),
        })
        .collect();

    let partial = TopologicalCrystalCandidate {
        candidate_id: Hash256::zero(),
        window_ref: window.window_id.clone(),
        mesh_ref: mesh.mesh_id.clone(),
        micro_fiber_set_ref,
        carrier_report_ref: carrier_report.report_id.clone(),
        reinterpretation_report_ref: reinterp.report_id.clone(),
        gate_report_ref: gate_report.report_id.clone(),
        evidence_refs,
        status,
        trace_ref: window.trace_ref.clone(),
    };
    let candidate_id = tpt_content_address(&partial)?;
    Ok(TopologicalCrystalCandidate {
        candidate_id,
        ..partial
    })
}

/// The replayable bundle of all topology-layer artefacts (TPT-MTL §12.2).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TptMtlBundle {
    pub bundle_id: Hash256,
    pub rd_hash: Hash256,
    pub source_manifest_hash: Hash256,
    pub phase_space_window_hash: Hash256,
    pub axis_bridge_report_hash: Hash256,
    pub mesh_holo_hash: Hash256,
    pub micro_fiber_set_hash: Hash256,
    pub carrier_report_hash: Hash256,
    pub reinterpretation_report_hash: Hash256,
    pub gate_report_hash: Hash256,
    pub crystal_candidate_hash: Hash256,
    pub trace_hash: Hash256,
    pub replay_manifest_hash: Hash256,
    pub outcome: TptMtlOutcomeKind,
}

/// Materialize a TptMtlBundle from all pipeline artefacts.
pub fn materialize_bundle(
    rd: &TptMtlRunDescriptor,
    window: &PhaseSpaceWindow,
    axis_report: &AxisBridgeReport,
    mesh: &MeshHolo,
    fibers: &[MicroFiber],
    carrier_report: &CarrierReport,
    reinterp: &ReinterpretationReport,
    gate_report: &TptMtlGateReport,
    candidate: &TopologicalCrystalCandidate,
    replay_manifest_hash: &Hash256,
) -> Result<TptMtlBundle, TopologyError> {
    let rd_hash = rd.content_hash()?;
    let source_manifest_hash = tpt_content_address(&rd.source_boundaries)?;
    let phase_space_window_hash = window.content_hash()?;
    let axis_bridge_report_hash = tpt_content_address(axis_report)?;
    let mesh_holo_hash = mesh.content_hash()?;
    let micro_fiber_set_hash = tpt_content_address(&fibers.to_vec())?;
    let carrier_report_hash = tpt_content_address(carrier_report)?;
    let reinterpretation_report_hash = tpt_content_address(reinterp)?;
    let gate_report_hash = tpt_content_address(gate_report)?;
    let crystal_candidate_hash = tpt_content_address(candidate)?;
    let trace_hash = tpt_content_address(&window.trace_ref)?;
    let outcome = gate_report.outcome_kind();

    let partial = TptMtlBundle {
        bundle_id: Hash256::zero(),
        rd_hash,
        source_manifest_hash,
        phase_space_window_hash,
        axis_bridge_report_hash,
        mesh_holo_hash,
        micro_fiber_set_hash,
        carrier_report_hash,
        reinterpretation_report_hash,
        gate_report_hash,
        crystal_candidate_hash,
        trace_hash,
        replay_manifest_hash: replay_manifest_hash.clone(),
        outcome,
    };
    let bundle_id = tpt_content_address(&partial)?;
    Ok(TptMtlBundle {
        bundle_id,
        ..partial
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::axis_bridge::build_axis_bridge_report;
    use crate::topology::carrier::{evaluate_carrier, CarrierContext};
    use crate::topology::gates::gate_all;
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
            let v = Fixed::quantize(i as f64 * 0.2 + 0.1, 9).unwrap();
            let point_id = tpt_content_address(&("art-pt", i)).unwrap();
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
            window_id: tpt_content_address(&("aw", n)).unwrap(),
            domain_profile: rd.domain_profile.clone(),
            input_refs: vec![],
            points,
            carrier: CarrierContext::new_minimal(1),
            horizon_refs: vec![],
            constraint_refs: vec![],
            boundary_ref: Hash256::zero(),
            sampling_policy_hash: tpt_content_address(&"sp").unwrap(),
            trace_ref: Hash256::zero(),
            rd_hash: Hash256::zero(),
        }
    }

    #[test]
    fn bundle_is_deterministic() {
        let rd = TptMtlRunDescriptor::default_permissive();
        let window = make_window(3);
        let axis_report = build_axis_bridge_report(&window, &rd).unwrap();
        let mesh = seed_mesh_holo(&window, &rd).unwrap();
        let (evolved, _) = evolve_mesh(mesh, &rd).unwrap();
        let fibers = lift_micro_fibers(&window, &evolved, &rd).unwrap();
        let carrier = evaluate_carrier(&window.carrier, &rd, false).unwrap();
        let reinterp = reinterpret_mesh_to_panoptic(&evolved, &carrier, &rd).unwrap();
        let gate_report = gate_all(
            &window, &axis_report, &evolved, &fibers, &carrier, &reinterp, &rd, None, None,
        )
        .unwrap();
        let candidate =
            form_topological_crystal_candidate(&window, &evolved, &fibers, &carrier, &reinterp, &gate_report)
                .unwrap();
        let replay_hash = Hash256::zero();
        let b1 =
            materialize_bundle(&rd, &window, &axis_report, &evolved, &fibers, &carrier, &reinterp, &gate_report, &candidate, &replay_hash)
                .unwrap();
        let b2 =
            materialize_bundle(&rd, &window, &axis_report, &evolved, &fibers, &carrier, &reinterp, &gate_report, &candidate, &replay_hash)
                .unwrap();
        assert_eq!(b1.bundle_id, b2.bundle_id);
    }
}

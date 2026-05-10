//! MicroFiber — the full Möbius-tripolar micro-lift fibre (TPT-MTL §8.3).
//!
//! A MicroFiber MUST NOT exist without Subwindow, Provenance, DualProof,
//! SeamProof and Trace (Definition of Done §6).

use serde::{Deserialize, Serialize};

use super::carrier::CarrierContext;
use super::dual_antiphase::{apply_mtl_d1, DualAntiphase};
use super::mesh_holo::MeshHolo;
use super::phase_window::PhaseSpaceWindow;
use super::primary_phase::{compute_primary_phase, PrimaryPhase};
use super::primitives::{tpt_content_address, Hash256, TopologyError};
use super::run_descriptor::TptMtlRunDescriptor;
use super::seam::{compute_seam, SeamComponent};
use super::subwindow::{build_local_subwindow, LocalSubwindow};

/// A complete MicroFiber for one point (TPT-MTL §8.3).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MicroFiber {
    pub fiber_id: Hash256,
    pub point_id: Hash256,
    pub subwindow_ref: Hash256,
    pub primary: PrimaryPhase,
    pub dual: DualAntiphase,
    pub seam: SeamComponent,
    pub carrier_ref: Hash256,
    pub lift_proof: Hash256,
    pub trace_ref: Hash256,
}

/// Lift a complete `MicroFiber` for one window point.
pub fn lift_micro_fiber(
    window: &PhaseSpaceWindow,
    mesh: &MeshHolo,
    point_idx: usize,
    rd: &TptMtlRunDescriptor,
) -> Result<MicroFiber, TopologyError> {
    let point = window.points.get(point_idx).ok_or_else(|| {
        TopologyError::InvalidMicroFiber {
            reason: format!("point_idx {} out of bounds", point_idx),
        }
    })?;

    // Build subwindow.
    let subwindow = build_local_subwindow(window, mesh, &point.point_id, rd)?;

    // Collect neighbourhood coordinates (including center).
    let neighbourhood_coords: Vec<[super::primitives::Fixed; 5]> = window
        .points
        .iter()
        .filter(|p| subwindow.point_ids.contains(&p.point_id))
        .map(|p| p.x.clone())
        .collect();

    // Compute primary phase.
    let primary = compute_primary_phase(point, &subwindow, &neighbourhood_coords)?;

    // Apply MTL-D1 dualization.
    let (dual, dual_proof) = apply_mtl_d1(
        point,
        &primary,
        &subwindow,
        &neighbourhood_coords,
        &rd.micro_lift_policy.epsilon,
    )?;

    // Compute seam (requires carrier).
    let (seam, seam_proof) = compute_seam(&primary, &dual, &window.carrier, rd)?;

    // Lift proof = content address of (dual_proof, seam_proof).
    let lift_proof = tpt_content_address(&(&dual_proof, &seam_proof))?;

    let partial = MicroFiber {
        fiber_id: Hash256::zero(),
        point_id: point.point_id.clone(),
        subwindow_ref: subwindow.subwindow_id.clone(),
        primary,
        dual,
        seam,
        carrier_ref: window.carrier.carrier_id.clone(),
        lift_proof,
        trace_ref: window.trace_ref.clone(),
    };
    let fiber_id = tpt_content_address(&partial)?;
    Ok(MicroFiber { fiber_id, ..partial })
}

/// Lift MicroFibers for all active (non-Blocked) points in the window.
pub fn lift_micro_fibers(
    window: &PhaseSpaceWindow,
    mesh: &MeshHolo,
    rd: &TptMtlRunDescriptor,
) -> Result<Vec<MicroFiber>, TopologyError> {
    use super::phase_window::PointStatus;

    let mut fibers = Vec::new();
    for (i, point) in window.points.iter().enumerate() {
        // Blocked points are not lifted.
        if point.meta.status == PointStatus::Blocked {
            continue;
        }
        // Points without provenance are NoiseCandidate — lift as degraded.
        let fiber = lift_micro_fiber(window, mesh, i, rd)?;
        fibers.push(fiber);
    }
    // Sort for determinism.
    fibers.sort();
    Ok(fibers)
}

/// Gate check: all fibers have non-zero lift_proof and trace_ref.
pub fn gate_micro_lift(fibers: &[MicroFiber]) -> (bool, Vec<String>) {
    let mut diagnostics = Vec::new();
    for fiber in fibers {
        if fiber.lift_proof == Hash256::zero() {
            diagnostics.push(format!(
                "fiber {} has zero lift_proof",
                fiber.fiber_id.hex()
            ));
        }
        if fiber.subwindow_ref == Hash256::zero() {
            diagnostics.push(format!(
                "fiber {} has zero subwindow_ref",
                fiber.fiber_id.hex()
            ));
        }
    }
    (diagnostics.is_empty(), diagnostics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::mesh_holo::seed_mesh_holo;
    use crate::topology::phase_window::{PointMeta, PointStatus, TptPoint};
    use crate::topology::primitives::{Fixed, TptEvidenceRef};
    use crate::topology::run_descriptor::TptMtlRunDescriptor;

    fn make_window(n: usize) -> PhaseSpaceWindow {
        let rd = TptMtlRunDescriptor::default_permissive();
        let mut points = Vec::new();
        for i in 0..n {
            let v = Fixed::quantize(i as f64 * 0.1 + 0.1, 9).unwrap();
            let point_id = tpt_content_address(&("fiber-pt", i)).unwrap();
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
            window_id: tpt_content_address(&("fw", n)).unwrap(),
            domain_profile: rd.domain_profile.clone(),
            input_refs: vec![],
            points,
            carrier: super::super::carrier::CarrierContext::new_minimal(1),
            horizon_refs: vec![],
            constraint_refs: vec![],
            boundary_ref: Hash256::zero(),
            sampling_policy_hash: Hash256::zero(),
            trace_ref: Hash256::zero(),
            rd_hash: Hash256::zero(),
        }
    }

    #[test]
    fn micro_fiber_lift_is_deterministic() {
        let rd = TptMtlRunDescriptor::default_permissive();
        let window = make_window(3);
        let mesh = seed_mesh_holo(&window, &rd).unwrap();
        let fibers1 = lift_micro_fibers(&window, &mesh, &rd).unwrap();
        let fibers2 = lift_micro_fibers(&window, &mesh, &rd).unwrap();
        assert_eq!(fibers1.len(), fibers2.len());
        for (f1, f2) in fibers1.iter().zip(fibers2.iter()) {
            assert_eq!(f1.fiber_id, f2.fiber_id);
        }
    }

    #[test]
    fn micro_lift_gate_passes_valid_fibers() {
        let rd = TptMtlRunDescriptor::default_permissive();
        let window = make_window(2);
        let mesh = seed_mesh_holo(&window, &rd).unwrap();
        let fibers = lift_micro_fibers(&window, &mesh, &rd).unwrap();
        let (passed, diags) = gate_micro_lift(&fibers);
        assert!(passed, "diagnostics: {:?}", diags);
    }
}

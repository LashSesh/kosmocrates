//! Replay manifest and verification for the topology layer (TPT-MTL §16.3).
//!
//! Invariant I-10: Identical inputs, RD, policies and operator versions
//! MUST produce identical digests for Window, Mesh, MicroFibers,
//! GateReport and Bundle.

use serde::{Deserialize, Serialize};

use super::artifact::TptMtlBundle;
use super::primitives::{tpt_content_address, Hash256, TopologyError};

/// Replay manifest: records the expected digests for a run.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ReplayManifest {
    pub manifest_id: Hash256,
    pub rd_hash: Hash256,
    pub expected_window_hash: Hash256,
    pub expected_mesh_hash: Hash256,
    pub expected_fiber_set_hash: Hash256,
    pub expected_gate_report_hash: Hash256,
    pub expected_bundle_hash: Hash256,
}

impl ReplayManifest {
    /// Build a ReplayManifest from a completed bundle.
    pub fn from_bundle(bundle: &TptMtlBundle) -> Result<Self, TopologyError> {
        let partial = ReplayManifest {
            manifest_id: Hash256::zero(),
            rd_hash: bundle.rd_hash.clone(),
            expected_window_hash: bundle.phase_space_window_hash.clone(),
            expected_mesh_hash: bundle.mesh_holo_hash.clone(),
            expected_fiber_set_hash: bundle.micro_fiber_set_hash.clone(),
            expected_gate_report_hash: bundle.gate_report_hash.clone(),
            expected_bundle_hash: bundle.bundle_id.clone(),
        };
        let manifest_id = tpt_content_address(&partial)?;
        Ok(ReplayManifest {
            manifest_id,
            ..partial
        })
    }

    /// Verify that a replayed bundle matches this manifest.
    pub fn verify(&self, replayed: &TptMtlBundle) -> Result<(), TopologyError> {
        let mut mismatches = Vec::new();

        if replayed.phase_space_window_hash != self.expected_window_hash {
            mismatches.push(format!(
                "window_hash: expected={} got={}",
                self.expected_window_hash.hex(),
                replayed.phase_space_window_hash.hex()
            ));
        }
        if replayed.mesh_holo_hash != self.expected_mesh_hash {
            mismatches.push(format!(
                "mesh_hash: expected={} got={}",
                self.expected_mesh_hash.hex(),
                replayed.mesh_holo_hash.hex()
            ));
        }
        if replayed.micro_fiber_set_hash != self.expected_fiber_set_hash {
            mismatches.push(format!(
                "fiber_set_hash: expected={} got={}",
                self.expected_fiber_set_hash.hex(),
                replayed.micro_fiber_set_hash.hex()
            ));
        }
        if replayed.gate_report_hash != self.expected_gate_report_hash {
            mismatches.push(format!(
                "gate_report_hash: expected={} got={}",
                self.expected_gate_report_hash.hex(),
                replayed.gate_report_hash.hex()
            ));
        }
        if replayed.bundle_id != self.expected_bundle_hash {
            mismatches.push(format!(
                "bundle_id: expected={} got={}",
                self.expected_bundle_hash.hex(),
                replayed.bundle_id.hex()
            ));
        }

        if mismatches.is_empty() {
            Ok(())
        } else {
            Err(TopologyError::ReplayMismatch {
                reason: mismatches.join("; "),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::artifact::{
        form_topological_crystal_candidate, materialize_bundle, TptMtlBundle,
    };
    use crate::topology::axis_bridge::build_axis_bridge_report;
    use crate::topology::carrier::{evaluate_carrier, CarrierContext};
    use crate::topology::gates::gate_all;
    use crate::topology::mesh_holo::{evolve_mesh, seed_mesh_holo};
    use crate::topology::micro_fiber::lift_micro_fibers;
    use crate::topology::phase_window::{PointMeta, PointStatus, TptPoint};
    use crate::topology::primitives::{tpt_content_address, Fixed, Hash256, TptEvidenceRef};
    use crate::topology::reinterpret::reinterpret_mesh_to_panoptic;
    use crate::topology::run_descriptor::TptMtlRunDescriptor;

    fn make_bundle(n: usize) -> TptMtlBundle {
        let rd = TptMtlRunDescriptor::default_permissive();
        let mut points = Vec::new();
        for i in 0..n {
            let v = Fixed::quantize(i as f64 * 0.25, 9).unwrap();
            let point_id = tpt_content_address(&("rpl-pt", i)).unwrap();
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

        use crate::topology::phase_window::PhaseSpaceWindow;
        let window = PhaseSpaceWindow {
            window_id: tpt_content_address(&("rw3", n)).unwrap(),
            domain_profile: rd.domain_profile.clone(),
            input_refs: vec![],
            points,
            carrier: CarrierContext::new_minimal(1),
            horizon_refs: vec![],
            constraint_refs: vec![],
            boundary_ref: Hash256::zero(),
            sampling_policy_hash: tpt_content_address(&"sph3").unwrap(),
            trace_ref: Hash256::zero(),
            rd_hash: Hash256::zero(),
        };
        let axis = build_axis_bridge_report(&window, &rd).unwrap();
        let mesh = seed_mesh_holo(&window, &rd).unwrap();
        let (evolved, _) = evolve_mesh(mesh, &rd).unwrap();
        let fibers = lift_micro_fibers(&window, &evolved, &rd).unwrap();
        let carrier = evaluate_carrier(&window.carrier, &rd, false).unwrap();
        let reinterp = reinterpret_mesh_to_panoptic(&evolved, &carrier, &rd).unwrap();
        let gate_report = gate_all(
            &window, &axis, &evolved, &fibers, &carrier, &reinterp, &rd, None, None,
        )
        .unwrap();
        let candidate = form_topological_crystal_candidate(
            &window,
            &evolved,
            &fibers,
            &carrier,
            &reinterp,
            &gate_report,
        )
        .unwrap();
        materialize_bundle(
            &rd,
            &window,
            &axis,
            &evolved,
            &fibers,
            &carrier,
            &reinterp,
            &gate_report,
            &candidate,
            &Hash256::zero(),
        )
        .unwrap()
    }

    #[test]
    fn replay_manifest_verifies_identical_bundle() {
        let bundle = make_bundle(3);
        let manifest = ReplayManifest::from_bundle(&bundle).unwrap();
        // Re-run produces identical bundle.
        let bundle2 = make_bundle(3);
        assert!(manifest.verify(&bundle2).is_ok());
    }

    #[test]
    fn replay_mismatch_detected() {
        let bundle = make_bundle(3);
        let manifest = ReplayManifest::from_bundle(&bundle).unwrap();
        let mut tampered = bundle.clone();
        tampered.bundle_id = tpt_content_address(&"tampered").unwrap();
        assert!(manifest.verify(&tampered).is_err());
    }
}

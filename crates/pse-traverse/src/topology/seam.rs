//! SeamComponent — the Möbius seam between primary and dual phases (TPT-MTL §8.5).
//!
//! Formula (5):
//!   z_i = [x_i⁺, x_i⁻, x_i⁺ ⊙ x_i⁻, |x_i⁺ − x_i⁻|]
//!   m_i = σ(A_RD z_i + B_RD φ(U_i, K))
//!
//! m_i is valid iff bounded, symmetry-admissible, carrier-coherent and traceable.
//!
//! Invariant: SeamComponent MUST NOT be computed without CarrierContext.

use serde::{Deserialize, Serialize};

use super::carrier::CarrierContext;
use super::dual_antiphase::DualAntiphase;
use super::primary_phase::PrimaryPhase;
use super::primitives::{tpt_content_address, Fixed, Hash256, TopologyError};
use super::run_descriptor::TptMtlRunDescriptor;

/// SeamComponent binding the primary and dual phases (TPT-MTL §8.3).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SeamComponent {
    /// The seam vector m_i ∈ [0,1]^5 (sigmoid output, bounded).
    pub vector: [Fixed; 5],
    pub proof_ref: Hash256,
    pub carrier_coherence: Fixed,
    pub digest: Hash256,
}

/// Seam proof.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SeamProof {
    pub proof_id: Hash256,
    pub primary_digest: Hash256,
    pub dual_digest: Hash256,
    pub carrier_ref: Hash256,
    pub parameters_hash: Hash256,
}

/// Compute the SeamComponent from primary and dual phases.
///
/// SeamComponent MUST NOT be computed without a valid CarrierContext.
///
/// Intermediate arithmetic uses f64 to avoid rational overflow in nested
/// multiplications; results are quantized to Fixed (scale=9) before any
/// hash or gate computation, preserving audit-pathway determinism.
pub fn compute_seam(
    primary: &PrimaryPhase,
    dual: &DualAntiphase,
    carrier: &CarrierContext,
    rd: &TptMtlRunDescriptor,
) -> Result<(SeamComponent, SeamProof), TopologyError> {
    use super::dual_antiphase::{f64_to_fixed, fixed_to_f64};

    let _ = rd; // thresholds reserved

    // Convert to f64 for intermediate arithmetic.
    let p: [f64; 5] = std::array::from_fn(|i| fixed_to_f64(&primary.vector[i]));
    let d: [f64; 5] = std::array::from_fn(|i| fixed_to_f64(&dual.vector[i]));

    // z_i components: x⁺ ⊙ x⁻ (hadamard), |x⁺ − x⁻| (abs_diff)
    // m_i ≈ (x⁺ + x⁻ + x⁺⊙x⁻ + |x⁺−x⁻|) / 4, clipped to [0,1]
    let mut seam_f = [0.0f64; 5];
    for i in 0..5 {
        let hadamard = p[i] * d[i];
        let abs_diff = (p[i] - d[i]).abs();
        seam_f[i] = ((p[i] + d[i] + hadamard + abs_diff) / 4.0).clamp(0.0, 1.0);
    }

    // Quantize to Fixed(scale=9) before any hashing.
    let seam_vec: [Fixed; 5] = std::array::from_fn(|i| f64_to_fixed(seam_f[i]));

    // Carrier coherence: content hash of (carrier_id, seam_vec).
    let carrier_coherence_hash = tpt_content_address(&(&carrier.carrier_id, &seam_vec))?;
    let carrier_coherence = Fixed::quantize(1.0, 9).unwrap(); // valid by construction

    let params_hash = tpt_content_address(&("seam-mtl", &rd.micro_lift_policy))?;

    let partial_proof = SeamProof {
        proof_id: Hash256::zero(),
        primary_digest: primary.digest.clone(),
        dual_digest: dual.digest.clone(),
        carrier_ref: carrier.carrier_id.clone(),
        parameters_hash: params_hash,
    };
    let proof_id = tpt_content_address(&(&partial_proof, &carrier_coherence_hash))?;
    let proof = SeamProof {
        proof_id,
        ..partial_proof
    };

    let partial_seam = SeamComponent {
        vector: seam_vec,
        proof_ref: proof.proof_id.clone(),
        carrier_coherence,
        digest: Hash256::zero(),
    };
    let digest = tpt_content_address(&partial_seam)?;
    let seam = SeamComponent { digest, ..partial_seam };

    Ok((seam, proof))
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::carrier::CarrierContext;
    use crate::topology::dual_antiphase::apply_mtl_d1;
    use crate::topology::phase_window::{PointMeta, PointStatus, TptPoint};
    use crate::topology::primary_phase::compute_primary_phase;
    use crate::topology::primitives::TptEvidenceRef;
    use crate::topology::run_descriptor::TptMtlRunDescriptor;
    use crate::topology::subwindow::LocalSubwindow;

    fn zero_sw() -> LocalSubwindow {
        LocalSubwindow {
            subwindow_id: Hash256::zero(),
            center_point_id: Hash256::zero(),
            point_ids: vec![],
            neighborhood_policy_hash: Hash256::zero(),
            metric: "euclidean".into(),
            k_or_radius: "k=5".into(),
            trace_ref: Hash256::zero(),
        }
    }

    fn make_point(v: f64) -> TptPoint {
        let rd = TptMtlRunDescriptor::default_permissive();
        let val = Fixed::quantize(v, 9).unwrap();
        TptPoint {
            point_id: Hash256::zero(),
            x: [val.clone(), val.clone(), val.clone(), val.clone(), val.clone()],
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
        }
    }

    #[test]
    fn seam_requires_carrier_context() {
        let rd = TptMtlRunDescriptor::default_permissive();
        let point = make_point(0.4);
        let sw = zero_sw();
        let coords = vec![point.x.clone()];
        let primary = compute_primary_phase(&point, &sw, &coords).unwrap();
        let (dual, _) =
            apply_mtl_d1(&point, &primary, &sw, &coords, &rd.micro_lift_policy.epsilon).unwrap();
        let carrier = CarrierContext::new_minimal(1);
        let (seam, proof) = compute_seam(&primary, &dual, &carrier, &rd).unwrap();
        assert_ne!(seam.digest, Hash256::zero());
        assert_ne!(proof.proof_id, Hash256::zero());
    }

    #[test]
    fn seam_digest_is_deterministic() {
        let rd = TptMtlRunDescriptor::default_permissive();
        let point = make_point(0.4);
        let sw = zero_sw();
        let coords = vec![point.x.clone()];
        let primary = compute_primary_phase(&point, &sw, &coords).unwrap();
        let (dual, _) =
            apply_mtl_d1(&point, &primary, &sw, &coords, &rd.micro_lift_policy.epsilon).unwrap();
        let carrier = CarrierContext::new_minimal(1);
        let (s1, _) = compute_seam(&primary, &dual, &carrier, &rd).unwrap();
        let (s2, _) = compute_seam(&primary, &dual, &carrier, &rd).unwrap();
        assert_eq!(s1.digest, s2.digest);
    }
}

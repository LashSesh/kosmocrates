//! DualAntiphase — Möbius-tripolar dualization (TPT-MTL §8.3–§8.4).
//!
//! Invariant I-07: An Antiphase MUST arise from a declared dualization rule
//! with Proof, Parameters and Digest.
//! The minimal standard rule is MTL-D1 (reflection about subwindow centre).
//!
//! Formula (4):
//!   D_i = I − 2 v_i v_i^T / (||v_i||² + ε)
//!   v_i = x_i − x̄_U_i
//!   x_i⁻ = clip`[0,1]`^5(x̄_U_i + D_i(x_i − x̄_U_i))
//!
//! Intermediate arithmetic uses f64 to avoid rational overflow in nested
//! multiplications; the result is quantized back to Fixed (scale=9) before
//! any hash or gate computation, preserving audit-pathway determinism.

use serde::{Deserialize, Serialize};

use super::phase_window::TptPoint;
use super::primary_phase::PrimaryPhase;
use super::primitives::{tpt_content_address, Fixed, Hash256, TopologyError};
use super::subwindow::{compute_centroid, LocalSubwindow};

/// A declared dualization proof.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DualizationProof {
    pub proof_id: Hash256,
    pub rule_id: String,
    pub parameters_hash: Hash256,
    pub input_digest: Hash256,
    pub output_digest: Hash256,
}

/// The antiphase of a point (mirror via declared dualization rule).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DualAntiphase {
    pub vector: [Fixed; 5],
    pub rule_id: String,
    pub proof_ref: Hash256,
    pub digest: Hash256,
}

/// Failure type for dualization errors.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DualizationFailure {
    pub reason: String,
    pub rule_id: String,
    pub input_digest: Hash256,
}

/// Public re-export of fixed-point multiply for seam.rs.
pub fn mul_fixed_pub(a: &Fixed, b: &Fixed) -> Fixed {
    let af = fixed_to_f64(a);
    let bf = fixed_to_f64(b);
    f64_to_fixed(af * bf)
}

/// Apply the MTL-D1 standard dualization rule.
///
/// Uses f64 arithmetic for intermediate computation to avoid rational
/// overflow; converts back to Fixed (scale=9) before any hashing.
pub fn apply_mtl_d1(
    point: &TptPoint,
    primary: &PrimaryPhase,
    subwindow: &LocalSubwindow,
    neighborhood_coords: &[[Fixed; 5]],
    epsilon: &Fixed,
) -> Result<(DualAntiphase, DualizationProof), TopologyError> {
    let _ = subwindow;

    let centroid = compute_centroid(neighborhood_coords);
    let centroid_f: [f64; 5] = std::array::from_fn(|i| fixed_to_f64(&centroid[i]));
    let vi_f: [f64; 5] = std::array::from_fn(|i| fixed_to_f64(&primary.vector[i]));
    let eps = fixed_to_f64(epsilon);

    // ||v_i||²
    let norm_sq: f64 = vi_f.iter().map(|x| x * x).sum();
    let denom = norm_sq + eps;

    // scale = 1 - 2 * norm_sq / denom
    let scale = 1.0 - 2.0 * norm_sq / denom;

    // reflected = centroid + v_i * scale, clipped to [0,1]^5
    let mut reflected_f = [0.0f64; 5];
    for i in 0..5 {
        reflected_f[i] = (centroid_f[i] + vi_f[i] * scale).clamp(0.0, 1.0);
    }

    // Convert back to Fixed (scale=9 for 1e-9 precision).
    let reflected: [Fixed; 5] = std::array::from_fn(|i| f64_to_fixed(reflected_f[i]));

    // Build proof.
    let params_hash = tpt_content_address(&("MTL-D1", "epsilon", eps.to_bits(), &centroid_f))?;
    let input_digest = tpt_content_address(&point.x)?;

    let partial_antiphase = DualAntiphase {
        vector: reflected,
        rule_id: "MTL-D1".into(),
        proof_ref: Hash256::zero(),
        digest: Hash256::zero(),
    };
    let output_digest = tpt_content_address(&partial_antiphase.vector)?;

    let partial_proof = DualizationProof {
        proof_id: Hash256::zero(),
        rule_id: "MTL-D1".into(),
        parameters_hash: params_hash,
        input_digest,
        output_digest,
    };
    let proof_id = tpt_content_address(&partial_proof)?;
    let proof = DualizationProof {
        proof_id,
        ..partial_proof
    };

    let antiphase_with_proof = DualAntiphase {
        proof_ref: proof.proof_id.clone(),
        ..partial_antiphase
    };
    let digest = tpt_content_address(&antiphase_with_proof)?;
    let antiphase = DualAntiphase {
        digest,
        ..antiphase_with_proof
    };

    Ok((antiphase, proof))
}

// ── helpers ──────────────────────────────────────────────────────────────────

/// Extract f64 from a Fixed value (only used for intermediate computation).
pub(crate) fn fixed_to_f64(x: &Fixed) -> f64 {
    use crate::dynamic_state::CanonicalNumber;
    match x {
        CanonicalNumber::FixedI64 { raw, scale } => *raw as f64 / 10f64.powi(*scale as i32),
        CanonicalNumber::Rational { num, den } => *num as f64 / *den as f64,
    }
}

/// Convert f64 to Fixed (FixedI64 with scale=9).
pub(crate) fn f64_to_fixed(v: f64) -> Fixed {
    Fixed::quantize(v, 9).unwrap_or_else(|_| {
        // Fallback for non-finite values: clamp to 0.
        Fixed::quantize(0.0, 9).unwrap()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
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
            x: [
                val.clone(),
                val.clone(),
                val.clone(),
                val.clone(),
                val.clone(),
            ],
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
    fn mtl_d1_dualization_is_deterministic() {
        let rd = TptMtlRunDescriptor::default_permissive();
        let point = make_point(0.5);
        let sw = zero_sw();
        let coords = vec![point.x.clone()];
        let primary = compute_primary_phase(&point, &sw, &coords).unwrap();
        let epsilon = rd.micro_lift_policy.epsilon.clone();

        let (a1, _) = apply_mtl_d1(&point, &primary, &sw, &coords, &epsilon).unwrap();
        let (a2, _) = apply_mtl_d1(&point, &primary, &sw, &coords, &epsilon).unwrap();
        assert_eq!(a1.digest, a2.digest);
    }

    #[test]
    fn mtl_d1_has_rule_id() {
        let rd = TptMtlRunDescriptor::default_permissive();
        let point = make_point(0.3);
        let sw = zero_sw();
        let coords = vec![point.x.clone()];
        let primary = compute_primary_phase(&point, &sw, &coords).unwrap();
        let (antiphase, proof) = apply_mtl_d1(
            &point,
            &primary,
            &sw,
            &coords,
            &rd.micro_lift_policy.epsilon,
        )
        .unwrap();
        assert_eq!(antiphase.rule_id, "MTL-D1");
        assert_eq!(proof.rule_id, "MTL-D1");
        assert_ne!(proof.proof_id, Hash256::zero());
    }

    #[test]
    fn reflected_vector_is_in_unit_hypercube() {
        let rd = TptMtlRunDescriptor::default_permissive();
        let point = make_point(0.7);
        let sw = zero_sw();
        let coords = vec![point.x.clone()];
        let primary = compute_primary_phase(&point, &sw, &coords).unwrap();
        let (antiphase, _) = apply_mtl_d1(
            &point,
            &primary,
            &sw,
            &coords,
            &rd.micro_lift_policy.epsilon,
        )
        .unwrap();
        for v in &antiphase.vector {
            let vf = fixed_to_f64(v);
            assert!(vf >= 0.0 && vf <= 1.0, "reflected value {vf} out of [0,1]");
        }
    }
}

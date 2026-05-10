//! PrimaryPhase — local feature vector for a point in its subwindow.

use serde::{Deserialize, Serialize};

use super::phase_window::TptPoint;
use super::primitives::{tpt_content_address, Fixed, Hash256, TopologyError};
use super::subwindow::{compute_centroid, LocalSubwindow};

/// The primary phase of a point relative to its local subwindow.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PrimaryPhase {
    /// Local feature vector (centred, 5D).
    pub vector: [Fixed; 5],
    /// Content hash of local features used.
    pub local_features_hash: Hash256,
    pub digest: Hash256,
}

/// Compute the primary phase of a point in its subwindow.
pub fn compute_primary_phase(
    point: &TptPoint,
    subwindow: &LocalSubwindow,
    neighborhood_coords: &[[Fixed; 5]],
) -> Result<PrimaryPhase, TopologyError> {
    let _ = subwindow;
    // Center: mean of neighbourhood coordinates.
    let centroid = compute_centroid(neighborhood_coords);

    // Primary vector: point - centroid (5D).
    let vector = subtract5d(&point.x, &centroid);

    let local_features_hash = tpt_content_address(&neighborhood_coords)?;

    let partial = PrimaryPhase {
        vector,
        local_features_hash,
        digest: Hash256::zero(),
    };
    let digest = tpt_content_address(&partial)?;
    Ok(PrimaryPhase { digest, ..partial })
}

fn subtract5d(a: &[Fixed; 5], b: &[Fixed; 5]) -> [Fixed; 5] {
    use super::dual_antiphase::{f64_to_fixed, fixed_to_f64};
    std::array::from_fn(|i| f64_to_fixed(fixed_to_f64(&a[i]) - fixed_to_f64(&b[i])))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::phase_window::{PointMeta, PointStatus, TptPoint};
    use crate::topology::primitives::TptEvidenceRef;
    use crate::topology::run_descriptor::TptMtlRunDescriptor;
    use crate::topology::subwindow::LocalSubwindow;

    fn zero_subwindow() -> LocalSubwindow {
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
    fn primary_phase_digest_is_deterministic() {
        let point = make_point(0.5);
        let sw = zero_subwindow();
        let coords = vec![point.x.clone()];
        let p1 = compute_primary_phase(&point, &sw, &coords).unwrap();
        let p2 = compute_primary_phase(&point, &sw, &coords).unwrap();
        assert_eq!(p1.digest, p2.digest);
    }
}

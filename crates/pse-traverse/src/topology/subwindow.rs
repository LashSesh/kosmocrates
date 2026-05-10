//! LocalSubwindow — neighbourhood context for MicroFiber lift (TPT-MTL §8.2).
//!
//! LocalSubwindow MUST be deterministically derived from window, point, RD
//! and neighbourhood policy. Ties are broken via TieBreakPolicy.

use serde::{Deserialize, Serialize};

use super::dual_antiphase::{f64_to_fixed, fixed_to_f64};
use super::mesh_holo::MeshHolo;
use super::phase_window::PhaseSpaceWindow;
use super::primitives::{tpt_content_address, Fixed, Hash256, TopologyError};
use super::run_descriptor::TptMtlRunDescriptor;

/// A local subwindow around a center point.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LocalSubwindow {
    pub subwindow_id: Hash256,
    pub center_point_id: Hash256,
    pub point_ids: Vec<Hash256>,
    pub neighborhood_policy_hash: Hash256,
    pub metric: String,
    pub k_or_radius: String,
    pub trace_ref: Hash256,
}

/// Build a local subwindow for a specific center point.
///
/// Uses k-nearest neighbours (by L1 distance in 5D) with deterministic
/// tie-breaking via TieBreakPolicy.
pub fn build_local_subwindow(
    window: &PhaseSpaceWindow,
    mesh: &MeshHolo,
    center_point_id: &Hash256,
    rd: &TptMtlRunDescriptor,
) -> Result<LocalSubwindow, TopologyError> {
    let _ = mesh; // reserved for future distance-weighted lookup

    let neighborhood_policy_hash = tpt_content_address(&rd.micro_lift_policy)?;

    // Find center point.
    let center_point = window
        .points
        .iter()
        .find(|p| &p.point_id == center_point_id)
        .ok_or_else(|| TopologyError::InvalidWindow {
            reason: format!(
                "center_point_id {} not found in window",
                center_point_id.hex()
            ),
        })?;

    // Parse k from the policy k_or_radius string (e.g. "k=5").
    let k = parse_k(&rd.micro_lift_policy.k_or_radius).unwrap_or(5);

    // Compute distances from center to all other points.
    let mut distances: Vec<(Hash256, Fixed)> = window
        .points
        .iter()
        .filter(|p| p.point_id != *center_point_id)
        .map(|p| {
            let dist = l1_distance5d(&center_point.x, &p.x);
            (p.point_id.clone(), dist)
        })
        .collect();

    // Sort deterministically: by distance, then by point_id hex for tie-breaking.
    distances.sort_by(|a, b| {
        let d = a.1.cmp(&b.1);
        if d == std::cmp::Ordering::Equal {
            a.0.cmp(&b.0)
        } else {
            d
        }
    });

    let mut point_ids: Vec<Hash256> = distances
        .into_iter()
        .take(k)
        .map(|(id, _)| id)
        .collect();
    // Always include center.
    point_ids.push(center_point_id.clone());
    point_ids.sort();

    let partial = LocalSubwindow {
        subwindow_id: Hash256::zero(),
        center_point_id: center_point_id.clone(),
        point_ids,
        neighborhood_policy_hash,
        metric: rd.micro_lift_policy.neighborhood_metric.clone(),
        k_or_radius: rd.micro_lift_policy.k_or_radius.clone(),
        trace_ref: window.trace_ref.clone(),
    };
    let subwindow_id = tpt_content_address(&partial)?;
    Ok(LocalSubwindow {
        subwindow_id,
        ..partial
    })
}

fn parse_k(s: &str) -> Option<usize> {
    s.strip_prefix("k=")
        .and_then(|n| n.parse::<usize>().ok())
}

fn l1_distance5d(a: &[Fixed; 5], b: &[Fixed; 5]) -> Fixed {
    let sum: f64 = (0..5).map(|i| (fixed_to_f64(&a[i]) - fixed_to_f64(&b[i])).abs()).sum();
    f64_to_fixed(sum)
}

/// Compute the centroid (mean) of a set of points in 5D.
///
/// Uses f64 internally to avoid rational overflow; result is quantized to
/// Fixed(scale=9) before any hash computation.
pub fn compute_centroid(coordinates: &[[Fixed; 5]]) -> [Fixed; 5] {
    if coordinates.is_empty() {
        return std::array::from_fn(|_| Fixed::quantize(0.0, 9).unwrap());
    }
    let n = coordinates.len() as f64;
    let mut sums = [0.0f64; 5];
    for coords in coordinates {
        for i in 0..5 {
            sums[i] += fixed_to_f64(&coords[i]);
        }
    }
    std::array::from_fn(|i| f64_to_fixed(sums[i] / n))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::mesh_holo::seed_mesh_holo;
    use crate::topology::phase_window::{PointMeta, PointStatus, TptPoint};
    use crate::topology::primitives::TptEvidenceRef;
    use crate::topology::run_descriptor::TptMtlRunDescriptor;

    fn make_window(n: usize) -> PhaseSpaceWindow {
        let rd = TptMtlRunDescriptor::default_permissive();
        let mut points = Vec::new();
        for i in 0..n {
            let v = Fixed::quantize(i as f64 * 0.1, 9).unwrap();
            let point_id = tpt_content_address(&("pt", i)).unwrap();
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
            window_id: tpt_content_address(&n).unwrap(),
            domain_profile: rd.domain_profile.clone(),
            input_refs: vec![],
            points,
            carrier: Default::default(),
            horizon_refs: vec![],
            constraint_refs: vec![],
            boundary_ref: Hash256::zero(),
            sampling_policy_hash: Hash256::zero(),
            trace_ref: Hash256::zero(),
            rd_hash: Hash256::zero(),
        }
    }

    #[test]
    fn subwindow_is_deterministic() {
        let rd = TptMtlRunDescriptor::default_permissive();
        let window = make_window(4);
        let mesh = seed_mesh_holo(&window, &rd).unwrap();
        let center = window.points[0].point_id.clone();
        let sw1 = build_local_subwindow(&window, &mesh, &center, &rd).unwrap();
        let sw2 = build_local_subwindow(&window, &mesh, &center, &rd).unwrap();
        assert_eq!(sw1.subwindow_id, sw2.subwindow_id);
    }
}

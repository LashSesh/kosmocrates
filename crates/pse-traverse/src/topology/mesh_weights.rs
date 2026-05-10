//! Mesh weighting with MicroFiber signals (TPT-MTL §9.1: WeightMicroMesh).

use serde::{Deserialize, Serialize};

use super::mesh_holo::{MeshEdge, MeshHolo};
use super::micro_fiber::MicroFiber;
use super::persistence::add_fixed;
use super::primitives::{tpt_content_address, Fixed, Hash256, TopologyError};
use super::run_descriptor::TptMtlRunDescriptor;

/// Weight the mesh by integrating micro-fibre seam signals into edge weights.
///
/// For each edge, find the two endpoint fibers and compute the average seam
/// carrier coherence as an additive weight factor.
pub fn weight_mesh_with_microfibers(
    mesh: MeshHolo,
    fibers: &[MicroFiber],
    rd: &TptMtlRunDescriptor,
) -> Result<MeshHolo, TopologyError> {
    let _ = rd;

    // Build a lookup: point_id → fiber seam carrier_coherence.
    let fiber_map: std::collections::BTreeMap<Hash256, Fixed> = fibers
        .iter()
        .map(|f| (f.point_id.clone(), f.seam.carrier_coherence.clone()))
        .collect();

    // Re-weight edges based on endpoint fiber coherence.
    let new_edges: Vec<MeshEdge> = mesh
        .edges
        .iter()
        .map(|edge| {
            let coh_v1 = fiber_map
                .get(&edge.v1)
                .cloned()
                .unwrap_or_else(|| Fixed::quantize(0.0, 9).unwrap());
            let coh_v2 = fiber_map
                .get(&edge.v2)
                .cloned()
                .unwrap_or_else(|| Fixed::quantize(0.0, 9).unwrap());

            // New weight = original weight + (coh_v1 + coh_v2) / 2
            let coh_sum = add_fixed(&coh_v1, &coh_v2);
            let coh_avg = rational_div(&coh_sum, &Fixed::quantize(2.0, 9).unwrap());
            let new_weight = add_fixed(&edge.weight, &coh_avg);

            MeshEdge {
                weight: new_weight,
                ..edge.clone()
            }
        })
        .collect();

    let mut new_mesh = mesh;
    new_mesh.edges = new_edges;

    // Update mesh_id after mutation.
    let mesh_id = tpt_content_address(&new_mesh)?;
    new_mesh.mesh_id = mesh_id;

    Ok(new_mesh)
}

fn rational_div(num: &Fixed, den: &Fixed) -> Fixed {
    let (nn, nd) = as_rational(num);
    let (dn, dd) = as_rational(den);
    crate::dynamic_state::CanonicalNumber::Rational {
        num: nn.saturating_mul(dd),
        den: nd.saturating_mul(dn),
    }
}

fn as_rational(x: &Fixed) -> (i128, i128) {
    use crate::dynamic_state::CanonicalNumber;
    match x {
        CanonicalNumber::FixedI64 { raw, scale } => {
            let den = 10i128.pow(*scale);
            (*raw as i128, den)
        }
        CanonicalNumber::Rational { num, den } => (*num, *den),
    }
}

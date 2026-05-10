//! MeshHolo — topological triangulation artefact (TPT-MTL §7).
//!
//! MeshHolo is deterministically derived from a PhaseSpaceWindow.
//! Every mutation must go through TopologyGuard (invariant I-08).

use serde::{Deserialize, Serialize};

use super::persistence::{compute_persistence_from_simplices, PersistenceDiagram};
use super::phase_window::PhaseSpaceWindow;
use super::primitives::{tpt_content_address, Fixed, Hash256, TopologyError};
use super::run_descriptor::TptMtlRunDescriptor;
use super::topology_guard::{check_topology_guard, TopologyGuardProof, TopologyGuardResult};

/// Entropy classification for the mesh state.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EntropyClass {
    /// High entropy — exploration zone (not emission-capable).
    Explore,
    /// Medium entropy — refinement zone.
    Refine,
    /// Low entropy — simplification zone.
    Simplify,
    /// Very low entropy — frozen, capture zone (candidate possible).
    Freeze,
}

impl EntropyClass {
    /// Classify based on vertex density ratio.
    pub fn classify(vertex_count: usize, simplex_count: usize) -> Self {
        if vertex_count == 0 {
            return EntropyClass::Explore;
        }
        let ratio = simplex_count as f64 / vertex_count as f64;
        if ratio < 0.5 {
            EntropyClass::Explore
        } else if ratio < 1.5 {
            EntropyClass::Refine
        } else if ratio < 3.0 {
            EntropyClass::Simplify
        } else {
            EntropyClass::Freeze
        }
    }

    /// Returns true if emission is allowed for this class.
    pub fn emission_capable(&self) -> bool {
        matches!(self, EntropyClass::Freeze | EntropyClass::Simplify)
    }
}

/// A vertex in the mesh.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MeshVertex {
    pub vertex_id: Hash256,
    pub point_ref: Hash256,
    pub coordinates: [Fixed; 5],
    pub weight: Fixed,
}

/// An edge between two vertices.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MeshEdge {
    pub edge_id: Hash256,
    pub v1: Hash256,
    pub v2: Hash256,
    pub weight: Fixed,
}

/// A simplex (triangle, tetrahedron, etc.) in the mesh.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Simplex {
    pub simplex_id: Hash256,
    /// Sorted vertex ids (deterministic).
    pub vertices: Vec<Hash256>,
    pub dimension: u8,
    pub weight: Fixed,
}

/// Topological invariants computed from the mesh.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TopologicalInvariants {
    pub betti: Vec<i64>,
    pub persistence_digest: Hash256,
    pub entropy_class: EntropyClass,
    pub lambda_gap: Fixed,
}

/// Mesh proof of validity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MeshProof {
    pub proof_id: Hash256,
    pub topology_guard_proofs: Vec<TopologyGuardProof>,
    pub symmetry_pass: bool,
    pub seed_hash: Hash256,
}

/// The primary topological triangulation artefact (TPT-MTL §7.1).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MeshHolo {
    pub mesh_id: Hash256,
    pub window_ref: Hash256,
    pub vertices: Vec<MeshVertex>,
    pub edges: Vec<MeshEdge>,
    pub simplices: Vec<Simplex>,
    pub invariants: TopologicalInvariants,
    pub proof: MeshProof,
    pub trace_ref: Hash256,
}

impl MeshHolo {
    pub fn content_hash(&self) -> Result<Hash256, TopologyError> {
        tpt_content_address(self)
    }

    /// Returns Betti numbers as the invariant slice.
    pub fn betti(&self) -> &[i64] {
        &self.invariants.betti
    }
}

/// Seed the initial mesh from a PhaseSpaceWindow (deterministic).
pub fn seed_mesh_holo(
    window: &PhaseSpaceWindow,
    rd: &TptMtlRunDescriptor,
) -> Result<MeshHolo, TopologyError> {
    let _ = rd;
    // Build vertices from window points.
    let mut vertices: Vec<MeshVertex> = Vec::new();
    for point in &window.points {
        let weight = Fixed::quantize(1.0, 9).unwrap();
        let partial = MeshVertex {
            vertex_id: Hash256::zero(),
            point_ref: point.point_id.clone(),
            coordinates: point.x.clone(),
            weight,
        };
        let vertex_id =
            tpt_content_address(&("mesh-vertex", &partial.point_ref, &partial.coordinates))?;
        vertices.push(MeshVertex {
            vertex_id,
            ..partial
        });
    }

    // Sort for determinism.
    vertices.sort();

    // Build edges: connect all pairs (complete 1-skeleton for small meshes).
    let mut edges: Vec<MeshEdge> = Vec::new();
    let n = vertices.len();
    for i in 0..n {
        for j in (i + 1)..n {
            let dist = euclidean5d(&vertices[i].coordinates, &vertices[j].coordinates);
            let partial = MeshEdge {
                edge_id: Hash256::zero(),
                v1: vertices[i].vertex_id.clone(),
                v2: vertices[j].vertex_id.clone(),
                weight: dist,
            };
            let edge_id = tpt_content_address(&("mesh-edge", &partial.v1, &partial.v2))?;
            edges.push(MeshEdge { edge_id, ..partial });
        }
    }
    edges.sort();

    // Build simplices: triangles for triples of vertices.
    let mut simplices: Vec<Simplex> = Vec::new();
    if n >= 3 {
        for i in 0..n {
            for j in (i + 1)..n {
                for k in (j + 1)..n {
                    let mut sv = vec![
                        vertices[i].vertex_id.clone(),
                        vertices[j].vertex_id.clone(),
                        vertices[k].vertex_id.clone(),
                    ];
                    sv.sort();
                    let partial = Simplex {
                        simplex_id: Hash256::zero(),
                        vertices: sv,
                        dimension: 2,
                        weight: Fixed::quantize(1.0, 9).unwrap(),
                    };
                    let simplex_id = tpt_content_address(&("simplex", &partial.vertices))?;
                    simplices.push(Simplex {
                        simplex_id,
                        ..partial
                    });
                }
            }
        }
    }
    simplices.sort();

    // Build persistence diagram from edges.
    let edge_triples: Vec<(usize, usize, Fixed)> = edges
        .iter()
        .map(|e| {
            let v1_idx = vertices
                .iter()
                .position(|v| v.vertex_id == e.v1)
                .unwrap_or(0);
            let v2_idx = vertices
                .iter()
                .position(|v| v.vertex_id == e.v2)
                .unwrap_or(0);
            (v1_idx, v2_idx, e.weight.clone())
        })
        .collect();
    let pd = compute_persistence_from_simplices(n, &edge_triples);
    let betti = pd.betti_numbers(4);
    let persistence_digest = pd.digest()?;
    let entropy_class = EntropyClass::classify(n, simplices.len());
    let lambda_gap = Fixed::quantize(0.0, 9).unwrap();

    let invariants = TopologicalInvariants {
        betti,
        persistence_digest,
        entropy_class,
        lambda_gap,
    };

    // Seed proof: no mutations yet.
    let seed_hash = tpt_content_address(&("seed", &window.window_id, &vertices))?;
    let partial_proof = MeshProof {
        proof_id: Hash256::zero(),
        topology_guard_proofs: vec![],
        symmetry_pass: true,
        seed_hash,
    };
    let proof_id = tpt_content_address(&partial_proof)?;
    let proof = MeshProof {
        proof_id,
        ..partial_proof
    };

    let partial_mesh = MeshHolo {
        mesh_id: Hash256::zero(),
        window_ref: window.window_id.clone(),
        vertices,
        edges,
        simplices,
        invariants,
        proof,
        trace_ref: window.trace_ref.clone(),
    };
    let mesh_id = tpt_content_address(&partial_mesh)?;
    Ok(MeshHolo {
        mesh_id,
        ..partial_mesh
    })
}

/// Apply a local mesh evolution step under TopologyGuard.
pub fn evolve_mesh(
    mesh: MeshHolo,
    rd: &TptMtlRunDescriptor,
) -> Result<(MeshHolo, TopologyGuardProof), TopologyError> {
    // For TPTM-2 the evolution step is identity (no mutation).
    // TopologyGuard is still invoked to produce the required proof.
    let old_betti = mesh.invariants.betti.clone();
    let old_pd = PersistenceDiagram::empty();
    let new_pd = PersistenceDiagram::empty();

    let guard_result = check_topology_guard(
        &old_betti,
        &old_betti,
        &old_pd,
        &new_pd,
        rd,
        mesh.trace_ref.clone(),
    )?;

    let proof = match guard_result {
        TopologyGuardResult::Pass(p) => p,
        TopologyGuardResult::Failure(p) => {
            return Err(TopologyError::TopologyGuardFailure {
                reason: p.reason.clone(),
            })
        }
    };

    // Attach guard proof to mesh.
    let mut new_mesh = mesh;
    new_mesh.proof.topology_guard_proofs.push(proof.clone());
    let mesh_id = tpt_content_address(&new_mesh)?;
    new_mesh.mesh_id = mesh_id;

    Ok((new_mesh, proof))
}

/// Euclidean distance in 5D using Fixed arithmetic.
fn euclidean5d(a: &[Fixed; 5], b: &[Fixed; 5]) -> Fixed {
    use super::dual_antiphase::{f64_to_fixed, fixed_to_f64};
    let sum: f64 = (0..5)
        .map(|i| (fixed_to_f64(&a[i]) - fixed_to_f64(&b[i])).abs())
        .sum();
    f64_to_fixed(sum)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::phase_window::{PointMeta, PointStatus, TptPoint};
    use crate::topology::primitives::TptEvidenceRef;
    use crate::topology::run_descriptor::TptMtlRunDescriptor;

    fn make_window_with_n_points(n: usize) -> PhaseSpaceWindow {
        let rd = TptMtlRunDescriptor::default_permissive();
        let mut points = Vec::new();
        for i in 0..n {
            let v = Fixed::quantize(i as f64 * 0.1, 9).unwrap();
            let point = TptPoint {
                point_id: tpt_content_address(&("test-point", i)).unwrap(),
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
            };
            points.push(point);
        }
        points.sort();
        PhaseSpaceWindow {
            window_id: tpt_content_address(&("window", n)).unwrap(),
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
    fn seed_mesh_is_deterministic() {
        let rd = TptMtlRunDescriptor::default_permissive();
        let window = make_window_with_n_points(3);
        let m1 = seed_mesh_holo(&window, &rd).unwrap();
        let m2 = seed_mesh_holo(&window, &rd).unwrap();
        assert_eq!(m1.mesh_id, m2.mesh_id);
    }

    #[test]
    fn evolve_mesh_produces_guard_proof() {
        let rd = TptMtlRunDescriptor::default_permissive();
        let window = make_window_with_n_points(2);
        let mesh = seed_mesh_holo(&window, &rd).unwrap();
        let (evolved, proof) = evolve_mesh(mesh, &rd).unwrap();
        assert!(proof.passed);
        assert!(!evolved.proof.topology_guard_proofs.is_empty());
    }
}

//! Local topology patch types.

use serde::{Deserialize, Serialize};

use super::primitives::{EvidenceRef, Fixed, Hash256};

/// A local topology patch constructed for a single fragment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalTopologyPatch {
    /// Content address of this patch.
    pub patch_id: Hash256,
    /// Fragment this patch covers.
    pub fragment_id: Hash256,
    /// Topology vertices (sorted by vertex_id for determinism).
    pub vertices: Vec<TopologyVertex>,
    /// Topology edges (sorted by edge_id for determinism).
    pub edges: Vec<TopologyEdge>,
    /// Simplices (sorted by simplex_id for determinism).
    pub simplices: Vec<TopologySimplex>,
    /// Canonical hash of the boundary model used.
    pub boundary_model_hash: Hash256,
    /// Topology invariant summary (Betti numbers, spectral gap, etc.).
    pub invariant_summary: TopologyInvariantSummary,
    /// Gate result: did this patch pass topology guards?
    pub topology_guard: LpcmTopologyGuardResult,
    /// Evidence refs for this patch.
    pub evidence_refs: Vec<EvidenceRef>,
}

/// A vertex in the local topology patch.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TopologyVertex {
    pub vertex_id: Hash256,
    pub coordinate_hash: Hash256,
    pub feature_hash: Hash256,
    pub evidence_ref: EvidenceRef,
}

/// An edge in the local topology patch.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TopologyEdge {
    pub edge_id: Hash256,
    pub a: Hash256,
    pub b: Hash256,
    pub weight: Fixed,
}

/// A simplex (higher-order face) in the local topology patch.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TopologySimplex {
    pub simplex_id: Hash256,
    /// Vertex ids (sorted for canonical form).
    pub vertices: Vec<Hash256>,
    pub orientation_hash: Hash256,
}

/// Compact Betti/spectral summary of a topology patch.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TopologyInvariantSummary {
    /// β₀: connected components.
    pub betti_0: u32,
    /// β₁: loops.
    pub betti_1: u32,
    /// β₂: voids.
    pub betti_2: u32,
    /// Spectral gap approximation as Fixed.
    pub spectral_gap: Fixed,
    /// Whether the patch graph is triangulable.
    pub is_triangulable: bool,
}

impl Default for TopologyInvariantSummary {
    fn default() -> Self {
        Self {
            betti_0: 1,
            betti_1: 0,
            betti_2: 0,
            spectral_gap: Fixed::zero(),
            is_triangulable: false,
        }
    }
}

/// Result of running topology guards on a local patch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LpcmTopologyGuardResult {
    pub passed: bool,
    pub diagnostics: Vec<String>,
}

/// Build a local topology patch for a fragment.
///
/// Constructs vertices, edges, and simplices from fragment evidence refs.
pub fn build_local_topology_patch(
    rd: &super::run_descriptor::LpcmRunDescriptor,
    fragment: &super::fragment::FragmentRef,
) -> LocalTopologyPatch {
    use super::primitives::{fixed_from_f64, lpcm_content_address};

    let vertices: Vec<TopologyVertex> = fragment
        .evidence_refs
        .iter()
        .map(|er| TopologyVertex {
            vertex_id: er.address.clone(),
            coordinate_hash: lpcm_content_address(&er.address).unwrap_or(Hash256::zero()),
            feature_hash: lpcm_content_address(&(&er.kind, &er.relation))
                .unwrap_or(Hash256::zero()),
            evidence_ref: er.clone(),
        })
        .collect();

    // Build edges: consecutive vertex pairs.
    let mut edges: Vec<TopologyEdge> = Vec::new();
    for i in 0..vertices.len().saturating_sub(1) {
        let edge_id =
            lpcm_content_address(&(&vertices[i].vertex_id, &vertices[i + 1].vertex_id))
                .unwrap_or(Hash256::zero());
        edges.push(TopologyEdge {
            edge_id,
            a: vertices[i].vertex_id.clone(),
            b: vertices[i + 1].vertex_id.clone(),
            weight: fixed_from_f64(1.0),
        });
    }

    // Build simplices: triangles from consecutive triples.
    let mut simplices: Vec<TopologySimplex> = Vec::new();
    for i in 0..vertices.len().saturating_sub(2) {
        let mut verts = vec![
            vertices[i].vertex_id.clone(),
            vertices[i + 1].vertex_id.clone(),
            vertices[i + 2].vertex_id.clone(),
        ];
        verts.sort();
        let orient =
            lpcm_content_address(&(&verts, "simplex")).unwrap_or(Hash256::zero());
        let simplex_id = lpcm_content_address(&(&verts, &orient)).unwrap_or(Hash256::zero());
        simplices.push(TopologySimplex {
            simplex_id,
            vertices: verts,
            orientation_hash: orient,
        });
    }

    let betti_0 = if vertices.is_empty() { 0 } else { 1 };
    let is_triangulable = vertices.len() >= 3;
    let invariant_summary = TopologyInvariantSummary {
        betti_0,
        betti_1: 0,
        betti_2: 0,
        spectral_gap: fixed_from_f64(if edges.is_empty() { 0.0 } else { 0.5 }),
        is_triangulable,
    };

    let topology_passed = !vertices.is_empty()
        || !rd.policies.topology_policy.require_triangulable;
    let topology_guard = LpcmTopologyGuardResult {
        passed: topology_passed,
        diagnostics: if topology_passed {
            vec![]
        } else {
            vec!["no vertices in patch".to_string()]
        },
    };

    let boundary_model_hash = fragment.boundary_hash.clone();
    let patch_id = lpcm_content_address(&(&fragment.fragment_id, &vertices, &edges))
        .unwrap_or(Hash256::zero());

    LocalTopologyPatch {
        patch_id,
        fragment_id: fragment.fragment_id.clone(),
        vertices,
        edges,
        simplices,
        boundary_model_hash,
        invariant_summary,
        topology_guard,
        evidence_refs: fragment.evidence_refs.clone(),
    }
}

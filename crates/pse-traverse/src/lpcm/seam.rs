//! Seam graph construction and seam scoring (§13).
//!
//! Seam score: Γ(KU, KV) = α_b·B + α_e·C + α_t·T + α_r·R + α_p·P
//! where B=boundary, C=carrier, T=topology, R=replay, P=provenance.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::local_gate::LocalCondensationCandidate;
use super::primitives::{
    fixed_add, fixed_from_f64, fixed_ge, fixed_mul, lpcm_content_address, Fixed, Hash256,
    LpcmResult,
};
use super::run_descriptor::{LpcmRunDescriptor, SupportPolicy};

/// A seam edge between two local condensation candidates.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SeamEdge {
    /// Content address of this edge.
    pub edge_id: Hash256,
    pub from_local_candidate: Hash256,
    pub to_local_candidate: Hash256,
    /// Γ(KU, KV): composite seam score.
    pub seam_score: Fixed,
    /// B: boundary compatibility score.
    pub boundary_score: Fixed,
    /// C: carrier compatibility score.
    pub carrier_score: Fixed,
    /// Whether replay is compatible between the two candidates.
    pub replay_compatible: bool,
    /// Whether this edge was accepted (seam_score >= seam_min).
    pub accepted: bool,
    /// Failure kind if not accepted.
    pub failure: Option<super::LpcmFailureKind>,
}

/// Connected component in the seam percolation graph.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PercolationComponent {
    pub component_id: Hash256,
    /// Node ids (sorted for determinism).
    pub nodes: Vec<Hash256>,
    pub is_percolating: bool,
}

/// Seam percolation graph over all accepted local condensation candidates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeamPercolationGraph {
    pub graph_id: Hash256,
    /// Sorted list of node ids (local_candidate_ids).
    pub nodes: Vec<Hash256>,
    pub edges: Vec<SeamEdge>,
    pub connected_components: Vec<PercolationComponent>,
}

/// Build the seam percolation graph from accepted local candidates.
pub fn build_seam_percolation_graph(
    rd: &LpcmRunDescriptor,
    local_candidates: &[LocalCondensationCandidate],
) -> LpcmResult<SeamPercolationGraph> {
    let policy = &rd.policies.support_policy;

    // Sort candidates by id for determinism.
    let mut sorted_candidates = local_candidates.to_vec();
    sorted_candidates.sort_by_key(|c| c.local_candidate_id.clone());

    let nodes: Vec<Hash256> = sorted_candidates
        .iter()
        .map(|c| c.local_candidate_id.clone())
        .collect();

    let mut edges: Vec<SeamEdge> = Vec::new();

    // Build edges between all pairs in different fragments.
    for i in 0..sorted_candidates.len() {
        for j in (i + 1)..sorted_candidates.len() {
            let a = &sorted_candidates[i];
            let b = &sorted_candidates[j];
            if a.fragment_id == b.fragment_id {
                continue;
            }
            let edge = compute_seam_edge(a, b, policy, &rd.thresholds.seam_min)?;
            edges.push(edge);
        }
    }

    // Sort edges by edge_id for determinism.
    edges.sort_by_key(|e| e.edge_id.clone());

    // Compute connected components (union-find style).
    let components = compute_connected_components(&nodes, &edges);

    let graph_id = lpcm_content_address(&(&nodes, &edges))?;

    Ok(SeamPercolationGraph {
        graph_id,
        nodes,
        edges,
        connected_components: components,
    })
}

fn compute_seam_edge(
    a: &LocalCondensationCandidate,
    b: &LocalCondensationCandidate,
    policy: &SupportPolicy,
    seam_min: &Fixed,
) -> LpcmResult<SeamEdge> {
    // Simple boundary/carrier scoring based on evidence overlap heuristic.
    // In a full implementation, these would compare topology boundary hashes.
    let boundary_score = fixed_from_f64(0.5); // placeholder: no boundary data at this layer
    let carrier_score = fixed_from_f64(0.5); // placeholder: no carrier data at this layer
    let topology_score = fixed_from_f64(0.5); // placeholder
    let replay_score = fixed_from_f64(1.0); // conservative: assume compatible
    let provenance_score = if !a.evidence_refs.is_empty() && !b.evidence_refs.is_empty() {
        fixed_from_f64(1.0)
    } else {
        fixed_from_f64(0.0)
    };

    let seam_score = seam_score_weighted(
        &boundary_score,
        &carrier_score,
        &topology_score,
        &replay_score,
        &provenance_score,
        policy,
    );

    let replay_compatible = true;
    let accepted = fixed_ge(&seam_score, seam_min);

    let edge_id =
        lpcm_content_address(&(&a.local_candidate_id, &b.local_candidate_id, &seam_score))?;

    Ok(SeamEdge {
        edge_id,
        from_local_candidate: a.local_candidate_id.clone(),
        to_local_candidate: b.local_candidate_id.clone(),
        seam_score,
        boundary_score,
        carrier_score,
        replay_compatible,
        accepted,
        failure: if accepted {
            None
        } else {
            Some(super::LpcmFailureKind::SeamBreak)
        },
    })
}

/// Γ(KU, KV) = α_b·B + α_e·C + α_t·T + α_r·R + α_p·P
fn seam_score_weighted(
    boundary: &Fixed,
    carrier: &Fixed,
    topology: &Fixed,
    replay: &Fixed,
    provenance: &Fixed,
    policy: &SupportPolicy,
) -> Fixed {
    let t1 = fixed_mul(&policy.alpha_boundary, boundary);
    let t2 = fixed_mul(&policy.alpha_carrier, carrier);
    let t3 = fixed_mul(&policy.alpha_topology, topology);
    let t4 = fixed_mul(&policy.alpha_replay, replay);
    let t5 = fixed_mul(&policy.alpha_provenance, provenance);
    fixed_add(&fixed_add(&fixed_add(&fixed_add(&t1, &t2), &t3), &t4), &t5)
}

fn compute_connected_components(
    nodes: &[Hash256],
    edges: &[SeamEdge],
) -> Vec<PercolationComponent> {
    let mut parent: BTreeMap<Hash256, Hash256> =
        nodes.iter().map(|n| (n.clone(), n.clone())).collect();

    let accepted_edges: Vec<&SeamEdge> = edges.iter().filter(|e| e.accepted).collect();

    for edge in accepted_edges {
        let ra = find_root(&mut parent, &edge.from_local_candidate);
        let rb = find_root(&mut parent, &edge.to_local_candidate);
        if ra != rb {
            parent.insert(ra, rb);
        }
    }

    // Collect components.
    let mut comp_map: BTreeMap<Hash256, Vec<Hash256>> = BTreeMap::new();
    for node in nodes {
        let root = find_root(&mut parent, node);
        comp_map.entry(root).or_default().push(node.clone());
    }

    let mut components: Vec<PercolationComponent> = comp_map
        .into_iter()
        .map(|(_root, mut members)| {
            members.sort();
            let is_percolating = members.len() > 1;
            let component_id = lpcm_content_address(&members).unwrap_or(Hash256::zero());
            PercolationComponent {
                component_id,
                nodes: members,
                is_percolating,
            }
        })
        .collect();

    components.sort_by_key(|c| c.component_id.clone());
    components
}

fn find_root(parent: &mut BTreeMap<Hash256, Hash256>, node: &Hash256) -> Hash256 {
    let mut current = node.clone();
    loop {
        let p = parent.get(&current).cloned().unwrap_or(current.clone());
        if p == current {
            break;
        }
        // Path compression.
        let gp = parent.get(&p).cloned().unwrap_or(p.clone());
        parent.insert(current.clone(), gp.clone());
        current = p;
    }
    current
}

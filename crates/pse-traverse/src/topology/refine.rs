//! Recursive mesh refinement under budget (TPT-MTL §9.1: RefineRecursive).
//!
//! Invariant I-09: Recursive refinement MUST be bounded by Depth-, Step-,
//! Vertex-, Simplex-, Time- and Memory-Budgets.
//! Budget Exhaustion MUST NOT escalate to Emit.

use serde::{Deserialize, Serialize};

use super::mesh_holo::MeshHolo;
use super::persistence::PersistenceDiagram;
use super::primitives::{tpt_content_address, TopologyError};
use super::run_descriptor::TptMtlRunDescriptor;
use super::topology_guard::{check_topology_guard, TopologyGuardResult};

/// Budget tracking state for recursive refinement.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BudgetState {
    pub depth: u32,
    pub steps: u64,
    pub vertices: u64,
    pub simplices: u64,
}

impl BudgetState {
    pub fn from_mesh(mesh: &MeshHolo) -> Self {
        BudgetState {
            depth: 0,
            steps: 0,
            vertices: mesh.vertices.len() as u64,
            simplices: mesh.simplices.len() as u64,
        }
    }

    pub fn check_budget(&self, rd: &TptMtlRunDescriptor) -> Result<(), TopologyError> {
        let b = &rd.budgets;
        if self.depth > b.max_depth {
            return Err(TopologyError::BudgetExhausted {
                reason: format!("depth {} > max_depth {}", self.depth, b.max_depth),
            });
        }
        if self.steps > b.max_steps {
            return Err(TopologyError::BudgetExhausted {
                reason: format!("steps {} > max_steps {}", self.steps, b.max_steps),
            });
        }
        if self.vertices > b.max_vertices {
            return Err(TopologyError::BudgetExhausted {
                reason: format!(
                    "vertices {} > max_vertices {}",
                    self.vertices, b.max_vertices
                ),
            });
        }
        if self.simplices > b.max_simplices {
            return Err(TopologyError::BudgetExhausted {
                reason: format!(
                    "simplices {} > max_simplices {}",
                    self.simplices, b.max_simplices
                ),
            });
        }
        Ok(())
    }
}

/// Apply bounded recursive refinement to a mesh.
///
/// For TPTM-3/4, this is an identity pass (no structural mutations beyond
/// the already-applied evolve_mesh step), but budget tracking and guard
/// proofs are always produced.
///
/// Returns (refined_mesh, steps_consumed).
pub fn refine_recursive(
    mesh: MeshHolo,
    rd: &TptMtlRunDescriptor,
    target_depth: u32,
) -> Result<(MeshHolo, u32), TopologyError> {
    let mut budget = BudgetState::from_mesh(&mesh);
    budget.check_budget(rd)?;

    let mut current = mesh;
    let max_depth = target_depth.min(rd.budgets.max_depth);

    for _depth in 0..max_depth {
        budget.depth += 1;
        budget.steps += 1;
        budget.check_budget(rd)?;

        // Identity refinement step: re-compute mesh_id after guard proof.
        let old_betti = current.invariants.betti.clone();
        let pd = PersistenceDiagram::empty();

        let guard_result = check_topology_guard(
            &old_betti,
            &old_betti,
            &pd,
            &pd,
            rd,
            current.trace_ref.clone(),
        )?;

        let proof = match guard_result {
            TopologyGuardResult::Pass(p) => p,
            TopologyGuardResult::Failure(p) => {
                // Guard failure during refinement → stop, return current mesh.
                // Budget is not exhausted, but topology is unstable.
                return Err(TopologyError::TopologyGuardFailure { reason: p.reason });
            }
        };

        current.proof.topology_guard_proofs.push(proof);
        let mesh_id = tpt_content_address(&current)?;
        current.mesh_id = mesh_id;

        budget.vertices = current.vertices.len() as u64;
        budget.simplices = current.simplices.len() as u64;
    }

    Ok((current, budget.depth))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::mesh_holo::seed_mesh_holo;
    use crate::topology::phase_window::{PointMeta, PointStatus, TptPoint};
    use crate::topology::primitives::{tpt_content_address, Fixed, Hash256, TptEvidenceRef};
    use crate::topology::run_descriptor::TptMtlRunDescriptor;

    fn make_window(n: usize) -> super::super::phase_window::PhaseSpaceWindow {
        let rd = TptMtlRunDescriptor::default_permissive();
        let mut points = Vec::new();
        for i in 0..n {
            let v = Fixed::quantize(i as f64 * 0.1 + 0.05, 9).unwrap();
            let point_id = tpt_content_address(&("refine-pt", i)).unwrap();
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
        super::super::phase_window::PhaseSpaceWindow {
            window_id: tpt_content_address(&("rw", n)).unwrap(),
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
    fn recursive_refinement_respects_budget() {
        let mut rd = TptMtlRunDescriptor::default_permissive();
        rd.budgets.max_depth = 2;
        let window = make_window(2);
        let mesh = seed_mesh_holo(&window, &rd).unwrap();
        let (refined, depth) = refine_recursive(mesh, &rd, 10).unwrap();
        // Should not exceed max_depth.
        assert!(depth <= rd.budgets.max_depth, "depth={}", depth);
        assert!(!refined.proof.topology_guard_proofs.is_empty());
    }

    #[test]
    fn budget_exhaustion_does_not_emit() {
        let mut rd = TptMtlRunDescriptor::default_permissive();
        rd.budgets.max_steps = 0; // exhausted immediately
        let window = make_window(2);
        let mesh = seed_mesh_holo(&window, &rd).unwrap();
        // BudgetExhausted error — MUST NOT emit.
        let result = refine_recursive(mesh, &rd, 1);
        assert!(result.is_err());
        match result.unwrap_err() {
            TopologyError::BudgetExhausted { .. } => {}
            other => panic!("expected BudgetExhausted, got {:?}", other),
        }
    }
}

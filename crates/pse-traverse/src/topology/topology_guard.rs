//! TopologyGuard — guards every mesh mutation (TPT-MTL §7.3).
//!
//! Invariant I-08: every mesh mutation MUST produce a TopologyGuardProof.
//! Invariant I-05: Trace, Evidence, Gate-History and Proofs MUST NOT disappear.

use serde::{Deserialize, Serialize};

use super::persistence::PersistenceDiagram;
use super::primitives::{tpt_content_address, Fixed, Hash256, TopologyError};
use super::run_descriptor::TptMtlRunDescriptor;

/// Proof produced when a mesh mutation passes the TopologyGuard.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TopologyGuardProof {
    pub proof_id: Hash256,
    pub old_betti: Vec<i64>,
    pub new_betti: Vec<i64>,
    pub betti_shift: Vec<i64>,
    pub allowed_shift: Vec<i64>,
    pub pd_distance: Fixed,
    pub pd_threshold: Fixed,
    pub passed: bool,
    pub reason: String,
    pub trace_ref: Hash256,
}

/// Result of checking the topology guard.
pub enum TopologyGuardResult {
    Pass(TopologyGuardProof),
    Failure(TopologyGuardProof),
}

/// Check topology guard: Δβ ∈ AllowedShift ∧ W_p(PD_old, PD_new) ≤ θ_PD.
///
/// A non-justified change in Betti vector, persistence diagram or incidence
/// boundary MUST lead to TopologyFailure.
pub fn check_topology_guard(
    old_betti: &[i64],
    new_betti: &[i64],
    old_pd: &PersistenceDiagram,
    new_pd: &PersistenceDiagram,
    rd: &TptMtlRunDescriptor,
    trace_ref: Hash256,
) -> Result<TopologyGuardResult, TopologyError> {
    let allowed_shift = &rd.mesh_policy.max_betti_shift;
    let pd_threshold = &rd.mesh_policy.max_pd_wasserstein;

    // Compute Betti shift.
    let max_dim = old_betti.len().max(new_betti.len());
    let mut betti_shift = vec![0i64; max_dim];
    for i in 0..max_dim {
        let ob = old_betti.get(i).copied().unwrap_or(0);
        let nb = new_betti.get(i).copied().unwrap_or(0);
        betti_shift[i] = nb - ob;
    }

    // Check Betti shift is within allowed bounds.
    let mut betti_ok = true;
    for (i, &shift) in betti_shift.iter().enumerate() {
        let max_shift = allowed_shift.get(i).copied().unwrap_or(i64::MAX);
        if shift.abs() > max_shift {
            betti_ok = false;
            break;
        }
    }

    // Check persistence diagram distance.
    let pd_dist = PersistenceDiagram::wasserstein_approx(old_pd, new_pd);
    let pd_ok = crate::horizon::primitives::fixed_le(&pd_dist, pd_threshold);

    let passed = betti_ok && pd_ok;
    let reason = if passed {
        "topology guard passed".into()
    } else if !betti_ok {
        format!(
            "betti shift {:?} exceeds allowed {:?}",
            betti_shift, allowed_shift
        )
    } else {
        format!(
            "pd_distance {:?} exceeds threshold {:?}",
            pd_dist, pd_threshold
        )
    };

    let partial = TopologyGuardProof {
        proof_id: Hash256::zero(),
        old_betti: old_betti.to_vec(),
        new_betti: new_betti.to_vec(),
        betti_shift: betti_shift.clone(),
        allowed_shift: allowed_shift.clone(),
        pd_distance: pd_dist,
        pd_threshold: pd_threshold.clone(),
        passed,
        reason,
        trace_ref,
    };
    let proof_id = tpt_content_address(&partial)?;
    let proof = TopologyGuardProof {
        proof_id,
        ..partial
    };

    if passed {
        Ok(TopologyGuardResult::Pass(proof))
    } else {
        Ok(TopologyGuardResult::Failure(proof))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::run_descriptor::TptMtlRunDescriptor;

    #[test]
    fn topology_guard_passes_no_change() {
        let rd = TptMtlRunDescriptor::default_permissive();
        let betti = vec![1i64, 0, 0, 0, 0];
        let pd = PersistenceDiagram::empty();
        let result = check_topology_guard(&betti, &betti, &pd, &pd, &rd, Hash256::zero()).unwrap();
        assert!(matches!(result, TopologyGuardResult::Pass(_)));
    }

    #[test]
    fn topology_guard_rejects_unallowed_betti_shift() {
        let mut rd = TptMtlRunDescriptor::default_permissive();
        rd.mesh_policy.max_betti_shift = vec![0, 0, 0, 0, 0]; // no shift allowed
        let old_betti = vec![1i64, 0, 0, 0, 0];
        let new_betti = vec![3i64, 0, 0, 0, 0]; // shift of 2, exceeds 0
        let pd = PersistenceDiagram::empty();
        let result =
            check_topology_guard(&old_betti, &new_betti, &pd, &pd, &rd, Hash256::zero()).unwrap();
        assert!(matches!(result, TopologyGuardResult::Failure(_)));
    }
}

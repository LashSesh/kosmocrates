//! Layer C7 — Wormhole Hypernet and Governed Shortcuts (§12).
//!
//! Wormholes are non-local edges admitted *only* when they declare a
//! `ReasonCode`, stay within their TTL and budget, and carry an audit
//! trace. No free non-locality, no hidden channels — every shortcut is
//! replay-traceable.

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::primitives::{fixed_le, CognitionError, Fixed};
use super::run_descriptor::CognitionRunDescriptor;
use super::scorpio_phase_scheduler::ReasonCode;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GovernedWormhole {
    pub wormhole_id: Hash256,
    pub source_node: Hash256,
    pub target_node: Hash256,
    pub reason_code: ReasonCode,
    pub ttl: u64,
    pub budget_cost: Fixed,
    pub admitted: bool,
    pub audit_trace: Hash256,
}

impl GovernedWormhole {
    pub fn with_id(mut self) -> Result<Self, CognitionError> {
        let mut probe = self.clone();
        probe.wormhole_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| CognitionError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.wormhole_id = Hash256(id);
        Ok(self)
    }
}

/// Per §12.2 — admission gate. A wormhole is admitted iff:
///
/// `Budget(w) ≤ B_max ∧ TTL(w) ≤ TTL_max ∧ Reason(w) ∈ R ∧ ReplayTrace(w) = 1`.
pub fn admit_wormhole(
    proposal: GovernedWormhole,
    rd: &CognitionRunDescriptor,
) -> Result<GovernedWormhole, CognitionError> {
    let budget_ok = fixed_le(&proposal.budget_cost, &rd.policies.wormhole_budget);
    let ttl_ok = proposal.ttl <= rd.policies.wormhole_max_ttl;
    let trace_ok = proposal.audit_trace != Hash256::zero();
    // Every ReasonCode value is in the admitted set R by construction.
    // (Allow-list semantics: we'd narrow R via descriptor extension if
    // a use-case demanded.)
    let reason_ok = matches!(
        proposal.reason_code,
        ReasonCode::ConstraintBridge
            | ReasonCode::SpiralMemoryRecall
            | ReasonCode::AttractorShortcut
            | ReasonCode::HorizonAlignment
            | ReasonCode::CarrierMigration
            | ReasonCode::ProjectionPreparation
    );
    let admitted = budget_ok && ttl_ok && trace_ok && reason_ok;
    let mut w = proposal;
    w.admitted = admitted;
    w.with_id()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognition::run_descriptor::{CognitionPolicies, CognitionThresholds};
    use std::collections::BTreeMap;

    fn rd() -> CognitionRunDescriptor {
        CognitionRunDescriptor {
            run_id: "cognition.wh".into(),
            problem_spec_hash: Hash256::zero(),
            traversal_report_hash: None,
            projection_v2_hash: None,
            seed: 0,
            operator_versions: BTreeMap::new(),
            thresholds: CognitionThresholds::permissive(),
            policies: CognitionPolicies::default_policies(),
            canonicalization_version: "cognition-v0.1".into(),
        }
    }

    fn proposal(budget: f64, ttl: u64, trace_byte: u8) -> GovernedWormhole {
        let mut tr = [0u8; 32];
        tr[0] = trace_byte;
        GovernedWormhole {
            wormhole_id: Hash256::zero(),
            source_node: Hash256::zero(),
            target_node: Hash256::zero(),
            reason_code: ReasonCode::ConstraintBridge,
            ttl,
            budget_cost: Fixed::quantize(budget, 9).unwrap(),
            admitted: false,
            audit_trace: Hash256(tr),
        }
    }

    #[test]
    fn admission_requires_reason_budget_ttl() {
        let rd = rd();
        let w = admit_wormhole(proposal(0.5, 4, 0xab), &rd).unwrap();
        assert!(w.admitted);
        // Over budget.
        let w_bad = admit_wormhole(proposal(10.0, 4, 0xab), &rd).unwrap();
        assert!(!w_bad.admitted);
        // Over TTL.
        let w_ttl = admit_wormhole(proposal(0.5, 1024, 0xab), &rd).unwrap();
        assert!(!w_ttl.admitted);
        // Empty trace.
        let w_trace = admit_wormhole(proposal(0.5, 4, 0), &rd).unwrap();
        assert!(!w_trace.admitted);
    }
}

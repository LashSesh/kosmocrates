//! KBL — the Kosmocrates Binding Layer (spec ch. 19): bind concrete Kosmocrates
//! artifacts (WishCube, SystemCube, StructuralCrystalRecord, IntegrationRunReport,
//! MaterializationReport, …) to the formal CDK calculus.

use kosmo_cdk_core::Status;
use kosmo_core::Digest;
use serde::{Deserialize, Serialize};

/// A bound Kosmocrates artifact. Binding invariant 19.1: every bound artifact
/// requires the tuple `(id, type, evidence, policy, trace, replay, boundary,
/// status)`. Without these fields it MAY be observed but MUST NOT be pulled into a
/// DiamondCube as a support unit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundUnit {
    pub id: Digest,
    pub artifact_type: String,
    pub evidence: Digest,
    pub policy: Digest,
    pub trace: Digest,
    pub replay: bool,
    pub boundary: Digest,
    pub status: Status,
}

impl BoundUnit {
    /// KBL-1 / invariant 19.1: a unit is pullable into a DiamondCube only if it is
    /// fully bound — evidence and trace present (CROSS-006), replay-aware (I4), and
    /// in an accepting status. A score never makes an unbound unit pullable (I2).
    pub fn is_pullable(&self) -> bool {
        self.evidence != Digest::ZERO
            && self.trace != Digest::ZERO
            && self.replay
            && matches!(self.status, Status::Pass | Status::Warn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bound(evidence: Digest, trace: Digest, replay: bool, status: Status) -> BoundUnit {
        BoundUnit {
            id: Digest::of_bytes(b"art"),
            artifact_type: "SystemCube".into(),
            evidence,
            policy: Digest::of_bytes(b"pol"),
            trace,
            replay,
            boundary: Digest::ZERO,
            status,
        }
    }

    #[test]
    fn only_a_fully_bound_artifact_is_pullable() {
        let ev = Digest::of_bytes(b"ev");
        let tr = Digest::of_bytes(b"tr");
        assert!(bound(ev, tr, true, Status::Pass).is_pullable());
        // Missing evidence / trace, or no replay, or a non-accepting status → not pullable.
        assert!(!bound(Digest::ZERO, tr, true, Status::Pass).is_pullable(), "no evidence");
        assert!(!bound(ev, Digest::ZERO, true, Status::Pass).is_pullable(), "no trace");
        assert!(!bound(ev, tr, false, Status::Pass).is_pullable(), "not replay-aware");
        assert!(!bound(ev, tr, true, Status::Reject).is_pullable(), "rejected status");
    }
}

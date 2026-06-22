//! KBL — the Kosmocrates Binding Layer (spec ch. 19): bind concrete Kosmocrates
//! artifacts (WishCube, SystemCube, StructuralCrystalRecord, IntegrationRunReport,
//! MaterializationReport, …) to the formal CDK calculus.

use kosmo_cdk_core::{Stage, StageMetrics, Status};
use kosmo_core::{Digest, Q16};
use kosmo_systemcube::{BlueprintUnit, SystemCubeManifest};
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

/// Bind a **real** Kosmocrates `SystemCube` into a CDK [`Stage`] (KBL, ch. 19) —
/// "the DiamondCube is the stricter, QSR-certified condensation form of a
/// SystemCube-related artifact" (§16.3). The accepted blueprint units become the
/// stage's support; the opaque (rejected) units are exkalibrated to the residue
/// report (I3 — boundary over absorption); the real pairwise
/// `ContradictionEnergyReport` is the stage's contradiction energy `K`; purity is
/// the accepted fraction, and a contradiction-free cube is the more irreducible
/// (`Irr = 1 − K`). The stage is evidence-bound by the manifest id (Inv. 19.1).
pub fn bind_systemcube(
    manifest: &SystemCubeManifest,
    units: &[BlueprintUnit],
    contradiction_energy: Q16,
) -> Stage {
    let support = manifest.accepted_unit_ids.clone();
    let rejected: Vec<Digest> = units
        .iter()
        .filter(|u| !u.is_accepted())
        .map(|u| u.unit_id)
        .collect();
    let total = support.len() + rejected.len();
    let purity = Q16::ratio(support.len() as u64, total.max(1) as u64).unwrap_or(Q16::ZERO);
    Stage {
        attractor: manifest.manifest_id,
        units: support,
        evidence: manifest.manifest_id, // evidence-bound (Inv. 19.1)
        gate_status: if manifest.accepted_count() > 0 {
            Status::Pass
        } else {
            Status::Reject
        },
        trace_root: manifest.run_id,
        residue: Digest::of(&rejected),
        metrics: StageMetrics {
            density: purity,
            purity,
            irreducibility: Q16::ONE.saturating_sub(contradiction_energy),
            curvature: Q16::ZERO,
            contradiction_energy,
        },
        certificates: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::{AuthorityLabel, PolicyProfile, RunDescriptor, TaintLabel};
    use kosmo_systemcube::{BlueprintUnitKind, ContradictionEnergyReport};

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

    #[test]
    fn binding_a_real_systemcube_yields_an_evidence_bound_stage() {
        let policy = PolicyProfile::default_report_only();
        let run = RunDescriptor::new(policy.id, "kosmocrates");
        let mk = |name: &str, evidenced: bool| {
            BlueprintUnit::new(
                BlueprintUnitKind::ModuleBoundary,
                Digest::of_bytes(name.as_bytes()),
                AuthorityLabel::Operator,
                TaintLabel::Clean,
                if evidenced {
                    vec![Digest::of_bytes(name.as_bytes())]
                } else {
                    vec![] // no evidence → opaque, rejected
                },
                &policy,
            )
        };
        let units = vec![mk("a", true), mk("b", true), mk("opaque", false)];
        let manifest =
            SystemCubeManifest::new(Digest::of_bytes(b"host"), &run, &policy, &units);
        let k = ContradictionEnergyReport::from_units(manifest.manifest_id, &policy, &units)
            .total_energy;

        let stage = bind_systemcube(&manifest, &units, k);
        assert_eq!(stage.units.len(), 2, "two accepted units become the support");
        assert_ne!(stage.evidence, Digest::ZERO, "evidence-bound (Inv. 19.1)");
        assert_eq!(stage.gate_status, Status::Pass);
        // The opaque unit is exkalibrated → purity = accepted/total = 2/3 (I3).
        assert_eq!(stage.metrics.purity, Q16::ratio(2, 3).unwrap());
    }
}

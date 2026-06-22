//! DiamondCube condensation (spec ch. 17, Listing 20.3): the QSR-certified,
//! irreducible, materializable system core — `Diamondize(W) = Core(Fix(Ω(W)))`.

use crate::stack::FoldBundle;
use kosmo_cdk_core::{GateError, Status};
use kosmo_core::Digest;
use serde::{Deserialize, Serialize};

/// The materialization profile (Listing 21.6) — the L8 safety gate. Default
/// **report-only**; never operator-approved without explicit policy (negative
/// test 13: materialization writes without operator-approved policy).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterializationProfile {
    #[default]
    ReportOnly,
    DryRun,
    OperatorApproved,
}

/// A DiamondCubeCandidate (Def. 17.1 / Listing 21.6): a QSR-certified, irreducible,
/// replayable, materializable system core derived from a closed stack.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiamondCubeCandidate {
    pub diamond_id: Digest,
    pub source_stack_id: Digest,
    pub systemcube_id: Option<Digest>,
    pub qsr_certificate_id: Digest,
    pub support_units: Vec<Digest>,
    pub residue_report_id: Digest,
    pub materialization_profile: MaterializationProfile,
    pub pse_bridge_candidate_id: Option<Digest>,
}

/// Strong irreducibility (Def. 17.2 / §18.3): `Irr(B_N) = 1` iff there are no
/// removable units — any proper subset `K' ⊊ K` would fail QSR.
pub fn is_irreducible(removable_units: &[Digest]) -> bool {
    removable_units.is_empty()
}

/// Press a fold bundle into a DiamondCubeCandidate (Listing 20.3). Def. 17.1
/// requires **all** of `QSR ∧ Gate ∧ Replay ∧ Irreducible ∧ Materializable`, with
/// a non-null QSR certificate — fail-closed (negative test 11: no DiamondCube
/// without a QSR pass). The materialization profile defaults to report-only.
#[allow(clippy::too_many_arguments)]
pub fn press_diamond(
    bundle: &FoldBundle,
    qsr: Status,
    gate: Status,
    replay: bool,
    irreducible: bool,
    materializable: bool,
    qsr_certificate: Digest,
) -> Result<DiamondCubeCandidate, GateError> {
    let accepting = |s: Status| matches!(s, Status::Pass | Status::Warn);
    if !accepting(qsr) {
        return Err(GateError::Rejected("QSR not satisfied".into()));
    }
    if qsr_certificate == Digest::ZERO {
        return Err(GateError::Rejected("missing QSR certificate".into()));
    }
    if !accepting(gate) {
        return Err(GateError::Rejected("gate not satisfied".into()));
    }
    if !replay {
        return Err(GateError::ReplayIncomplete("diamond requires replay".into()));
    }
    if !irreducible {
        return Err(GateError::Rejected("not strongly irreducible".into()));
    }
    if !materializable {
        return Err(GateError::Rejected("not materializable".into()));
    }
    let source_stack_id = bundle.bundle_id;
    let residue_report_id = Digest::of(&bundle.residue);
    let support_units = bundle.support.clone();
    let diamond_id = Digest::of(&(source_stack_id, qsr_certificate, &support_units));
    Ok(DiamondCubeCandidate {
        diamond_id,
        source_stack_id,
        systemcube_id: None,
        qsr_certificate_id: qsr_certificate,
        support_units,
        residue_report_id,
        materialization_profile: MaterializationProfile::ReportOnly,
        pse_bridge_candidate_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_cdk_core::Stage;

    fn bundle() -> FoldBundle {
        FoldBundle::fold(&[] as &[Stage], vec![Digest::of_bytes(b"u")], vec![Digest::of_bytes(b"r")])
    }
    fn cert() -> Digest {
        Digest::of_bytes(b"qsr-cert")
    }

    #[test]
    fn a_fully_certified_irreducible_core_presses_a_diamond() {
        let cube = press_diamond(&bundle(), Status::Pass, Status::Pass, true, true, true, cert())
            .expect("diamond");
        // L8 safety: defaults to report-only (negative test 13).
        assert_eq!(cube.materialization_profile, MaterializationProfile::ReportOnly);
        assert_ne!(cube.diamond_id, Digest::ZERO);
        assert_eq!(cube.qsr_certificate_id, cert());
    }

    #[test]
    fn a_diamond_without_a_qsr_certificate_is_refused() {
        // Negative test 11 — no DiamondCube without a QSR certificate.
        assert!(press_diamond(&bundle(), Status::Pass, Status::Pass, true, true, true, Digest::ZERO).is_err());
        assert!(press_diamond(&bundle(), Status::Reject, Status::Pass, true, true, true, cert()).is_err());
    }

    #[test]
    fn a_reducible_or_non_replayable_core_is_refused() {
        assert!(press_diamond(&bundle(), Status::Pass, Status::Pass, true, false, true, cert()).is_err(), "reducible");
        assert!(press_diamond(&bundle(), Status::Pass, Status::Pass, false, true, true, cert()).is_err(), "no replay");
        assert!(press_diamond(&bundle(), Status::Pass, Status::Pass, true, true, false, cert()).is_err(), "not materializable");
        // is_irreducible reflects §18.3.
        assert!(is_irreducible(&[]));
        assert!(!is_irreducible(&[Digest::of_bytes(b"x")]));
    }
}

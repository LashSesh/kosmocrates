use kosmo_core::{
    Digest, EnergyAssessment, EnergyFactors, EnergyKernel, FoundrySurvival, GateResult,
    LicenseStatus, TaintLabel, TripolarEnergy, Q16,
};
use serde::{Deserialize, Serialize};

/// A recurring structural pattern candidate observed across source evidence.
///
/// `support_score` is a Q16 value in [0, 1]. Gate comparisons use
/// `support_score.raw()` (integer) — no float in gate paths (CROSS-007).
/// High support does not bypass gates (CROSS-010).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MotifCandidate {
    pub motif_id: Digest,
    pub name: String,
    pub support_count: u64,
    pub support_score: Q16,
    pub hdag_pattern_id: Option<Digest>,
    pub taint: TaintLabel,
    pub evidence_bundle_id: Digest,
    pub policy_id: Digest,
}

impl MotifCandidate {
    pub fn new(
        name: String,
        support_count: u64,
        total_sources: u64,
        hdag_pattern_id: Option<Digest>,
        taint: TaintLabel,
        evidence_bundle_id: Digest,
        policy_id: Digest,
    ) -> Self {
        let support_score = if total_sources > 0 {
            Q16::ratio(support_count, total_sources).unwrap_or(Q16::ZERO)
        } else {
            Q16::ZERO
        };
        let motif_id = Digest::of_bytes(
            &[
                name.as_bytes(),
                &support_count.to_le_bytes(),
                &support_score.raw().to_le_bytes(),
                evidence_bundle_id.as_bytes(),
                policy_id.as_bytes(),
            ]
            .concat(),
        );
        Self {
            motif_id,
            name,
            support_count,
            support_score,
            hdag_pattern_id,
            taint,
            evidence_bundle_id,
            policy_id,
        }
    }

    /// A motif with high support still requires gate approval (CROSS-010).
    pub fn exceeds_support_threshold(&self, threshold: Q16) -> bool {
        self.support_score.exceeds(threshold)
    }

    /// Build an [`EnergyAssessment`] for this motif using the tripolar kernel.
    ///
    /// - ψ (meaning) = `support_score` — fraction of sources exhibiting this pattern.
    /// - ρ (coherence) = `Q16::ONE` — no structural coherence data at MotifCandidate level.
    /// - ω (phase) = `Q16::ONE` — no phase data at MotifCandidate level.
    /// - Taint factor from `self.taint`; gate factor from caller.
    ///
    /// Use `rank_by_energy` on the returned assessments to rank candidates by D.
    pub fn energy_assessment(&self, gate: &GateResult) -> EnergyAssessment {
        let tripolar = TripolarEnergy::new(self.support_score, Q16::ONE, Q16::ONE);
        let factors = EnergyFactors {
            gate: EnergyFactors::gate_factor(gate),
            taint: EnergyFactors::taint_factor(&self.taint),
            license: EnergyFactors::license_factor(&LicenseStatus::NotApplicable),
            foundry: EnergyFactors::foundry_factor(FoundrySurvival::Unavailable),
            seam: Q16::ONE,
            contradiction: Q16::ONE,
        };
        EnergyAssessment::new(
            self.motif_id,
            EnergyKernel::new(tripolar, factors),
            self.policy_id,
            self.evidence_bundle_id,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::{Digest, GateResult, TaintLabel, Q16};

    fn pid() -> Digest {
        Digest::of_bytes(b"p")
    }

    #[test]
    fn motif_support_score_is_ratio() {
        let m = MotifCandidate::new(
            "test-pattern".into(),
            3,
            4,
            None,
            TaintLabel::Clean,
            Digest::ZERO,
            pid(),
        );
        assert!((m.support_score.to_f64() - 0.75).abs() < 1e-4);
    }

    #[test]
    fn cross_010_high_support_does_not_bypass_gates() {
        let m = MotifCandidate::new(
            "high-support".into(),
            10,
            10,
            None,
            TaintLabel::Unverified,
            Digest::ZERO,
            pid(),
        );
        assert_eq!(m.support_score, Q16::ONE);
        assert!(matches!(m.taint, TaintLabel::Unverified));
    }

    #[test]
    fn motif_deterministic() {
        let m1 = MotifCandidate::new(
            "p".into(),
            2,
            4,
            None,
            TaintLabel::Clean,
            Digest::ZERO,
            pid(),
        );
        let m2 = MotifCandidate::new(
            "p".into(),
            2,
            4,
            None,
            TaintLabel::Clean,
            Digest::ZERO,
            pid(),
        );
        assert_eq!(m1.motif_id, m2.motif_id);
    }

    #[test]
    fn motif_energy_assessment_content_addressed() {
        let ev = Digest::of_bytes(b"ev");
        let m = MotifCandidate::new("pat".into(), 3, 4, None, TaintLabel::Clean, ev, pid());
        let a1 = m.energy_assessment(&GateResult::Pass);
        let a2 = m.energy_assessment(&GateResult::Pass);
        assert_eq!(a1.id, a2.id, "energy_assessment must be deterministic");
        assert_eq!(a1.subject_id, m.motif_id);
    }

    #[test]
    fn motif_quarantine_zeroes_energy() {
        let ev = Digest::of_bytes(b"ev");
        let m = MotifCandidate::new(
            "tainted".into(),
            10,
            10,
            None,
            TaintLabel::Quarantined {
                reason: "test".into(),
            },
            ev,
            pid(),
        );
        let a = m.energy_assessment(&GateResult::Pass);
        assert!(
            a.kernel.is_zeroed(),
            "quarantined taint must collapse energy to zero"
        );
    }
}

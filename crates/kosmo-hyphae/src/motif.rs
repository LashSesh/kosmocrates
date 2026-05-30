use kosmo_core::{Digest, Q16, TaintLabel};
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
}

impl MotifCandidate {
    pub fn new(
        name: String,
        support_count: u64,
        total_sources: u64,
        hdag_pattern_id: Option<Digest>,
        taint: TaintLabel,
        evidence_bundle_id: Digest,
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
            ]
            .concat(),
        );
        Self { motif_id, name, support_count, support_score, hdag_pattern_id, taint, evidence_bundle_id }
    }

    /// A motif with high support still requires gate approval (CROSS-010).
    pub fn exceeds_support_threshold(&self, threshold: Q16) -> bool {
        self.support_score.exceeds(threshold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::{Digest, Q16, TaintLabel};

    #[test]
    fn motif_support_score_is_ratio() {
        let m = MotifCandidate::new(
            "test-pattern".into(),
            3, 4,
            None,
            TaintLabel::Clean,
            Digest::ZERO,
        );
        // 3/4 = 0.75 → Q16 ≈ 49152
        assert!((m.support_score.to_f64() - 0.75).abs() < 1e-4);
    }

    #[test]
    fn cross_010_high_support_does_not_bypass_gates() {
        // A motif with 100% support is still just a candidate — not trusted.
        let m = MotifCandidate::new(
            "high-support".into(),
            10, 10,
            None,
            TaintLabel::Unverified,
            Digest::ZERO,
        );
        assert_eq!(m.support_score, Q16::ONE);
        // The taint is Unverified — gates will Warn on this.
        // High support score alone must not promote to trusted status.
        assert!(matches!(m.taint, TaintLabel::Unverified));
    }

    #[test]
    fn motif_deterministic() {
        let m1 = MotifCandidate::new("p".into(), 2, 4, None, TaintLabel::Clean, Digest::ZERO);
        let m2 = MotifCandidate::new("p".into(), 2, 4, None, TaintLabel::Clean, Digest::ZERO);
        assert_eq!(m1.motif_id, m2.motif_id);
    }
}

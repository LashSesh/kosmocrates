use crate::digest::Digest;
use crate::fixed_point::Q16;
use serde::{Deserialize, Serialize};

/// The outcome of a PSE promotion as seen by the feedback path.
///
/// This mirrors the essential states of `PromotionOutcome` from
/// `kosmo-pse-bridge` without creating a cross-crate dependency.
/// `kosmo-pse-bridge` converts its `PromotionOutcome` into this type
/// when building a `PromotionFeedback` record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeedbackOutcome {
    /// PSE accepted the candidate for crystallization consideration.
    Accepted,
    /// PSE rejected the candidate (did not meet threshold).
    Rejected,
    /// PSE deferred the decision (insufficient evidence or context).
    Deferred,
    /// Submission was skipped by policy (`ReportOnly` or `allow_submission=false`).
    Skipped,
}

impl FeedbackOutcome {
    /// Returns the fitness signal this outcome contributes to a `NormFitnessTrace`.
    ///
    /// - `Accepted` → `energy` (full energy reinforces the norm)
    /// - `Deferred` → ¼ (weak positive: keep the norm alive but don't reinforce)
    /// - `Rejected` → 0 (zero: PSE rejected — no fitness credit)
    /// - `Skipped`  → 0 (no information — do not update fitness)
    pub fn fitness_signal(&self, energy: Q16) -> Q16 {
        match self {
            Self::Accepted => energy,
            Self::Deferred => Q16::ratio(1, 4).unwrap_or(Q16::ZERO),
            Self::Rejected | Self::Skipped => Q16::ZERO,
        }
    }

    pub fn is_accepted(&self) -> bool {
        matches!(self, Self::Accepted)
    }

    pub fn is_rejected(&self) -> bool {
        matches!(self, Self::Rejected)
    }

    pub fn is_skipped(&self) -> bool {
        matches!(self, Self::Skipped)
    }
}

/// Content for deterministic `PromotionFeedback` ID.
#[derive(Serialize)]
struct FeedbackContent<'a> {
    record_id: &'a Digest,
    candidate_id: &'a Digest,
    norm_candidate_id: &'a Digest,
    outcome: &'a FeedbackOutcome,
    energy_raw: i64,
    fitness_signal_raw: i64,
    policy_id: &'a Digest,
    evidence_bundle_id: &'a Digest,
}

/// Content-addressed record that carries a PSE promotion outcome back into the
/// `kosmo-*` substrate (CorpusCartography + NormFitnessTrace).
///
/// Designed as the "Wissen zurück ins Substrat" record: it binds the PSE
/// decision to the norm candidate that was evaluated and provides a
/// `fitness_signal` ready to ingest into `NormFitnessTrace`.
///
/// Invariants:
/// - `id = SHA-256(JCS(content))` — INVARIANT-007
/// - `evidence_bundle_id ≠ ZERO` — CROSS-006
/// - `fitness_signal = ZERO` when `outcome == Rejected` — PSE rejection
///   must never produce a positive fitness contribution (CROSS-010 analogue)
/// - `norm_candidate_id = ZERO` when the candidate has no norm association
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionFeedback {
    pub id: Digest,
    /// `PromotionRequestRecord.id` from `kosmo-pse-bridge`.
    pub record_id: Digest,
    /// `PseBridgeCandidate.id` — the candidate that was evaluated.
    pub candidate_id: Digest,
    /// `NormGeneCandidate.candidate_id` linked to this feedback.
    /// `ZERO` when the candidate has no norm gene association.
    pub norm_candidate_id: Digest,
    /// PSE's decision in a form the substrate can reason over.
    pub outcome: FeedbackOutcome,
    /// Energy value (`D`) of the candidate at submission time.
    pub energy_at_submission: Q16,
    /// Fitness signal derived from `outcome` and `energy_at_submission`.
    /// Ready to ingest into `NormFitnessTrace::observe`.
    pub fitness_signal: Q16,
    pub policy_id: Digest,
    /// Mandatory evidence binding (CROSS-006).
    pub evidence_bundle_id: Digest,
}

impl PromotionFeedback {
    /// Build a content-addressed `PromotionFeedback`.
    ///
    /// `fitness_signal` is derived from `outcome.fitness_signal(energy_at_submission)`.
    pub fn new(
        record_id: Digest,
        candidate_id: Digest,
        norm_candidate_id: Digest,
        outcome: FeedbackOutcome,
        energy_at_submission: Q16,
        policy_id: Digest,
        evidence_bundle_id: Digest,
    ) -> Self {
        let fitness_signal = outcome.fitness_signal(energy_at_submission);
        let mut f = Self {
            id: Digest::ZERO,
            record_id,
            candidate_id,
            norm_candidate_id,
            outcome,
            energy_at_submission,
            fitness_signal,
            policy_id,
            evidence_bundle_id,
        };
        f.id = f.compute_id();
        f
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&FeedbackContent {
            record_id: &self.record_id,
            candidate_id: &self.candidate_id,
            norm_candidate_id: &self.norm_candidate_id,
            outcome: &self.outcome,
            energy_raw: self.energy_at_submission.raw(),
            fitness_signal_raw: self.fitness_signal.raw(),
            policy_id: &self.policy_id,
            evidence_bundle_id: &self.evidence_bundle_id,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }

    pub fn has_norm_association(&self) -> bool {
        self.norm_candidate_id != Digest::ZERO
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    fn bundle_id() -> Digest {
        d(b"evidence-bundle")
    }

    fn policy_id() -> Digest {
        d(b"policy")
    }

    fn energy_half() -> Q16 {
        Q16::ratio(1, 2).unwrap()
    }

    fn feedback(outcome: FeedbackOutcome) -> PromotionFeedback {
        PromotionFeedback::new(
            d(b"record"),
            d(b"candidate"),
            d(b"norm-candidate"),
            outcome,
            energy_half(),
            policy_id(),
            bundle_id(),
        )
    }

    // ── FeedbackOutcome::fitness_signal ───────────────────────────────────

    #[test]
    fn accepted_fitness_equals_energy() {
        let energy = Q16::ratio(3, 4).unwrap();
        assert_eq!(FeedbackOutcome::Accepted.fitness_signal(energy), energy);
    }

    #[test]
    fn deferred_fitness_is_quarter() {
        let sig = FeedbackOutcome::Deferred.fitness_signal(Q16::ONE);
        assert_eq!(sig, Q16::ratio(1, 4).unwrap());
    }

    #[test]
    fn rejected_fitness_is_zero() {
        // CROSS-010 analogue: PSE rejection must never produce positive fitness.
        assert!(FeedbackOutcome::Rejected.fitness_signal(Q16::ONE).is_zero());
    }

    #[test]
    fn skipped_fitness_is_zero() {
        assert!(FeedbackOutcome::Skipped.fitness_signal(Q16::ONE).is_zero());
    }

    #[test]
    fn deferred_does_not_depend_on_energy() {
        let sig1 = FeedbackOutcome::Deferred.fitness_signal(Q16::ONE);
        let sig2 = FeedbackOutcome::Deferred.fitness_signal(Q16::ZERO);
        // Both are Q16::ratio(1,4) — deferred is always the same fixed partial credit
        assert_eq!(sig1, sig2);
    }

    // ── PromotionFeedback content addressing ──────────────────────────────

    #[test]
    fn feedback_id_deterministic() {
        let f1 = feedback(FeedbackOutcome::Accepted);
        let f2 = feedback(FeedbackOutcome::Accepted);
        assert_eq!(f1.id, f2.id);
    }

    #[test]
    fn feedback_verify_id() {
        assert!(feedback(FeedbackOutcome::Accepted).verify_id());
        assert!(feedback(FeedbackOutcome::Rejected).verify_id());
        assert!(feedback(FeedbackOutcome::Deferred).verify_id());
        assert!(feedback(FeedbackOutcome::Skipped).verify_id());
    }

    #[test]
    fn feedback_different_outcomes_differ() {
        let fa = feedback(FeedbackOutcome::Accepted);
        let fr = feedback(FeedbackOutcome::Rejected);
        assert_ne!(fa.id, fr.id);
    }

    #[test]
    fn feedback_fitness_signal_correct_for_accepted() {
        let energy = energy_half();
        let f = PromotionFeedback::new(
            d(b"r"), d(b"c"), d(b"n"),
            FeedbackOutcome::Accepted,
            energy,
            policy_id(),
            bundle_id(),
        );
        assert_eq!(f.fitness_signal, energy);
    }

    #[test]
    fn feedback_fitness_signal_zero_for_rejected() {
        let f = PromotionFeedback::new(
            d(b"r"), d(b"c"), d(b"n"),
            FeedbackOutcome::Rejected,
            Q16::ONE,
            policy_id(),
            bundle_id(),
        );
        assert!(f.fitness_signal.is_zero());
    }

    #[test]
    fn feedback_evidence_mandatory() {
        let f = feedback(FeedbackOutcome::Accepted);
        assert_ne!(f.evidence_bundle_id, Digest::ZERO);
    }

    #[test]
    fn feedback_norm_association_zero_when_none() {
        let f = PromotionFeedback::new(
            d(b"r"), d(b"c"),
            Digest::ZERO,  // no norm association
            FeedbackOutcome::Deferred,
            Q16::ONE,
            policy_id(),
            bundle_id(),
        );
        assert!(!f.has_norm_association());
        assert!(f.verify_id());
    }

    #[test]
    fn feedback_has_norm_association_when_set() {
        let f = feedback(FeedbackOutcome::Accepted);
        assert!(f.has_norm_association());
    }

    #[test]
    fn feedback_different_energy_differs() {
        let f1 = PromotionFeedback::new(
            d(b"r"), d(b"c"), d(b"n"),
            FeedbackOutcome::Accepted,
            Q16::ratio(1, 4).unwrap(),
            policy_id(),
            bundle_id(),
        );
        let f2 = PromotionFeedback::new(
            d(b"r"), d(b"c"), d(b"n"),
            FeedbackOutcome::Accepted,
            Q16::ratio(3, 4).unwrap(),
            policy_id(),
            bundle_id(),
        );
        assert_ne!(f1.id, f2.id);
    }
}

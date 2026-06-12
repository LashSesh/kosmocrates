use kosmo_core::{
    Digest, EnergyAssessment, EnergyFactors, EnergyKernel, FoundrySurvival, GateResult,
    LicenseStatus, PromotionFeedback, TripolarEnergy, Q16,
};
use serde::{Deserialize, Serialize};

/// Serialize-only for NormGeneCandidate content-addressing.
#[derive(Serialize)]
struct CandidateContent {
    name: String,
    fitness_score: i64,
    evidence_bundle_id: Digest,
    policy_id: Digest,
}

/// A candidate structural norm derived from observed patterns.
///
/// Norm genes are NEVER trusted by default. Promoting a candidate to a
/// trusted norm requires an explicit governance path (Phase 11+).
/// The spec forbids treating `NormGeneCandidate` as a trusted norm.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NormGeneCandidate {
    pub candidate_id: Digest,
    pub name: String,
    pub description: String,
    pub fitness_score: Q16,
    pub evidence_bundle_id: Digest,
    pub policy_id: Digest,
}

impl NormGeneCandidate {
    pub fn new(
        name: String,
        description: String,
        fitness_score: Q16,
        evidence_bundle_id: Digest,
        policy_id: Digest,
    ) -> Self {
        let candidate_id = Digest::of(&CandidateContent {
            name: name.clone(),
            fitness_score: fitness_score.raw(),
            evidence_bundle_id,
            policy_id,
        });
        Self {
            candidate_id,
            name,
            description,
            fitness_score,
            evidence_bundle_id,
            policy_id,
        }
    }

    /// Fitness above threshold is a metric — it does NOT confer trust.
    pub fn exceeds_fitness_threshold(&self, threshold: Q16) -> bool {
        self.fitness_score.exceeds(threshold)
    }

    /// Build an [`EnergyAssessment`] for this candidate using the tripolar kernel.
    ///
    /// - ψ (meaning) = `fitness_score` — accumulated fitness from PSE feedback.
    /// - ρ (coherence) = `Q16::ONE` — no coherence data at NormGeneCandidate level.
    /// - ω (phase) = `Q16::ONE` — no phase data at NormGeneCandidate level.
    /// - Gate factor from caller; taint = Clean; license = NotApplicable.
    ///
    /// Use `rank_by_energy` on the returned assessments to rank candidates by D.
    pub fn energy_assessment(&self, gate: &GateResult) -> EnergyAssessment {
        let tripolar = TripolarEnergy::new(self.fitness_score, Q16::ONE, Q16::ONE);
        let factors = EnergyFactors {
            gate: EnergyFactors::gate_factor(gate),
            taint: Q16::ONE,
            license: EnergyFactors::license_factor(&LicenseStatus::NotApplicable),
            foundry: EnergyFactors::foundry_factor(FoundrySurvival::Unavailable),
            seam: Q16::ONE,
            contradiction: Q16::ONE,
        };
        EnergyAssessment::new(
            self.candidate_id,
            EnergyKernel::new(tripolar, factors),
            self.policy_id,
            self.evidence_bundle_id,
        )
    }
}

/// One fitness observation in a NormFitnessTrace.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FitnessObservation {
    pub sequence: u64,
    pub fitness: Q16,
    pub evidence_ref: Digest,
}

/// Serialize-only for NormFitnessTrace content-addressing.
#[derive(Serialize)]
struct TraceContent {
    candidate_id: Digest,
    observation_count: u64,
    policy_id: Digest,
}

/// Tracks how a NormGeneCandidate's fitness evolves across observations.
///
/// Observations are ordered by sequence number (monotonically increasing).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NormFitnessTrace {
    pub trace_id: Digest,
    pub candidate_id: Digest,
    pub observations: Vec<FitnessObservation>,
    pub policy_id: Digest,
}

impl NormFitnessTrace {
    pub fn empty(candidate_id: Digest, policy_id: Digest) -> Self {
        let trace_id = Digest::of(&TraceContent {
            candidate_id,
            observation_count: 0,
            policy_id,
        });
        Self {
            trace_id,
            candidate_id,
            observations: vec![],
            policy_id,
        }
    }

    /// Append an observation. Returns a new trace (non-mutating).
    pub fn observe(mut self, fitness: Q16, evidence_ref: Digest) -> Self {
        let sequence = self.observations.len() as u64;
        self.observations.push(FitnessObservation {
            sequence,
            fitness,
            evidence_ref,
        });
        self.trace_id = Digest::of(&TraceContent {
            candidate_id: self.candidate_id,
            observation_count: self.observations.len() as u64,
            policy_id: self.policy_id,
        });
        self
    }

    /// Latest fitness observation, or `Q16::ZERO` if no observations yet.
    pub fn latest_fitness(&self) -> Q16 {
        self.observations
            .last()
            .map(|o| o.fitness)
            .unwrap_or(Q16::ZERO)
    }

    /// Append a fitness observation derived from a `PromotionFeedback` record.
    ///
    /// Uses `feedback.fitness_signal` as the observation value and `feedback.id`
    /// as the evidence reference — closing the "Wissen zurück ins Substrat" loop.
    pub fn observe_from_feedback(self, feedback: &PromotionFeedback) -> Self {
        self.observe(feedback.fitness_signal, feedback.id)
    }

    /// Integer-averaged fitness across all observations (no floats — CROSS-007).
    pub fn average_fitness(&self) -> Q16 {
        if self.observations.is_empty() {
            return Q16::ZERO;
        }
        let sum_raw: i64 = self.observations.iter().map(|o| o.fitness.raw()).sum();
        Q16::from_raw(sum_raw / self.observations.len() as i64)
    }

    /// Exponentially-smoothed fitness in pure `Q16` integer arithmetic
    /// (CROSS-007). `alpha` is the retention weight of the running value:
    /// `ema₀ = first observation`, `emaₜ = alpha·emaₜ₋₁ + (1−alpha)·obsₜ`.
    /// Wonderlamp's `φ ← 0.9·φ + 0.1·r` update becomes
    /// `smoothed_fitness(Q16::ratio(9, 10).unwrap())`. Out-of-range `alpha`
    /// is clamped to `[ZERO, ONE]`; an empty trace yields `ZERO`.
    pub fn smoothed_fitness(&self, alpha: Q16) -> Q16 {
        let alpha = if alpha.is_negative() {
            Q16::ZERO
        } else if alpha.exceeds(Q16::ONE) {
            Q16::ONE
        } else {
            alpha
        };
        let inv = Q16::ONE.saturating_sub(alpha);
        let mut iter = self.observations.iter();
        let Some(first) = iter.next() else {
            return Q16::ZERO;
        };
        let mut ema = first.fitness;
        for obs in iter {
            let kept = alpha.checked_mul(ema).unwrap_or(Q16::ZERO);
            let fresh = inv.checked_mul(obs.fitness).unwrap_or(Q16::ZERO);
            ema = kept.saturating_add(fresh);
        }
        ema
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::{Digest, Q16};

    #[test]
    fn norm_gene_candidate_is_content_addressed() {
        let pid = Digest::of_bytes(b"p");
        let ev_id = Digest::of_bytes(b"e");
        let c1 = NormGeneCandidate::new("test-norm".into(), "desc".into(), Q16::HALF, ev_id, pid);
        let c2 = NormGeneCandidate::new("test-norm".into(), "desc".into(), Q16::HALF, ev_id, pid);
        assert_eq!(c1.candidate_id, c2.candidate_id);
        assert_ne!(c1.candidate_id, Digest::ZERO);
    }

    #[test]
    fn norm_gene_high_fitness_does_not_confer_trust() {
        // SAFETY INVARIANT: exceeds_fitness_threshold is a metric only.
        // Spec: do not treat NormGeneCandidate as a trusted norm.
        let pid = Digest::of_bytes(b"p");
        let cand =
            NormGeneCandidate::new("high-fit".into(), "".into(), Q16::ONE, Digest::ZERO, pid);
        // Fitness = 1.0 (maximum)
        assert!(cand.exceeds_fitness_threshold(Q16::HALF));
        // But the candidate struct has NO is_trusted field.
        // High fitness alone cannot produce a trusted norm.
        assert_eq!(cand.fitness_score, Q16::ONE);
    }

    #[test]
    fn norm_fitness_trace_appends_in_sequence() {
        let pid = Digest::of_bytes(b"p");
        let cid = Digest::of_bytes(b"c");
        let ev_id = Digest::of_bytes(b"e");
        let trace = NormFitnessTrace::empty(cid, pid)
            .observe(Q16::ratio(1, 4).unwrap(), ev_id)
            .observe(Q16::HALF, ev_id)
            .observe(Q16::ratio(3, 4).unwrap(), ev_id);
        assert_eq!(trace.observations.len(), 3);
        assert_eq!(trace.observations[0].sequence, 0);
        assert_eq!(trace.observations[2].sequence, 2);
        assert_eq!(trace.latest_fitness(), Q16::ratio(3, 4).unwrap());
    }

    #[test]
    fn norm_fitness_trace_average_is_integer() {
        let pid = Digest::of_bytes(b"p");
        let cid = Digest::of_bytes(b"c");
        let ev_id = Digest::of_bytes(b"e");
        let trace = NormFitnessTrace::empty(cid, pid)
            .observe(Q16::HALF, ev_id)
            .observe(Q16::HALF, ev_id);
        // avg(HALF, HALF) = HALF — no float division
        assert_eq!(trace.average_fitness(), Q16::HALF);
    }

    #[test]
    fn observe_from_feedback_accepted_raises_fitness() {
        use kosmo_core::{FeedbackOutcome, PromotionFeedback};
        let pid = Digest::of_bytes(b"p");
        let cid = Digest::of_bytes(b"c");
        let energy = Q16::ratio(3, 4).unwrap();
        let feedback = PromotionFeedback::new(
            Digest::of_bytes(b"record"),
            Digest::of_bytes(b"candidate"),
            cid,
            FeedbackOutcome::Accepted,
            energy,
            pid,
            Digest::of_bytes(b"ev"),
        );
        let trace = NormFitnessTrace::empty(cid, pid).observe_from_feedback(&feedback);
        assert_eq!(trace.observations.len(), 1);
        assert_eq!(trace.latest_fitness(), energy);
    }

    #[test]
    fn observe_from_feedback_rejected_sets_zero_fitness() {
        use kosmo_core::{FeedbackOutcome, PromotionFeedback};
        let pid = Digest::of_bytes(b"p");
        let cid = Digest::of_bytes(b"c");
        let feedback = PromotionFeedback::new(
            Digest::of_bytes(b"rec"),
            Digest::of_bytes(b"cand"),
            cid,
            FeedbackOutcome::Rejected,
            Q16::ONE,
            pid,
            Digest::of_bytes(b"ev"),
        );
        let trace = NormFitnessTrace::empty(cid, pid).observe_from_feedback(&feedback);
        assert!(
            trace.latest_fitness().is_zero(),
            "rejected feedback must produce zero fitness (CROSS-010 analogue)"
        );
    }

    #[test]
    fn observe_from_feedback_uses_feedback_id_as_evidence_ref() {
        use kosmo_core::{FeedbackOutcome, PromotionFeedback};
        let pid = Digest::of_bytes(b"p");
        let cid = Digest::of_bytes(b"c");
        let feedback = PromotionFeedback::new(
            Digest::of_bytes(b"rec"),
            Digest::of_bytes(b"cand"),
            cid,
            FeedbackOutcome::Deferred,
            Q16::ONE,
            pid,
            Digest::of_bytes(b"ev"),
        );
        let trace = NormFitnessTrace::empty(cid, pid).observe_from_feedback(&feedback);
        assert_eq!(trace.observations[0].evidence_ref, feedback.id);
    }

    #[test]
    fn smoothed_fitness_is_zero_on_empty_and_exact_on_constant() {
        let pid = Digest::of_bytes(b"p");
        let cid = Digest::of_bytes(b"c");
        let ev = Digest::of_bytes(b"e");
        let empty = NormFitnessTrace::empty(cid, pid);
        assert_eq!(empty.smoothed_fitness(Q16::HALF), Q16::ZERO);

        // HALF·HALF + HALF·HALF is exact in Q16: a constant signal stays put.
        let constant = NormFitnessTrace::empty(cid, pid)
            .observe(Q16::HALF, ev)
            .observe(Q16::HALF, ev)
            .observe(Q16::HALF, ev);
        assert_eq!(constant.smoothed_fitness(Q16::HALF), Q16::HALF);
    }

    #[test]
    fn smoothed_fitness_weights_recency_by_alpha() {
        let pid = Digest::of_bytes(b"p");
        let cid = Digest::of_bytes(b"c");
        let ev = Digest::of_bytes(b"e");
        let trace = NormFitnessTrace::empty(cid, pid)
            .observe(Q16::ZERO, ev)
            .observe(Q16::ONE, ev);
        // Low retention: the fresh observation dominates (0.1·0 + 0.9·1).
        let fast = trace.smoothed_fitness(Q16::ratio(1, 10).unwrap());
        assert!(fast.exceeds(trace.average_fitness()));
        // High retention: the past dominates (0.9·0 + 0.1·1).
        let slow = trace.smoothed_fitness(Q16::ratio(9, 10).unwrap());
        assert!(trace.average_fitness().exceeds(slow));
        // Degenerate alphas clamp instead of misbehaving.
        assert_eq!(trace.smoothed_fitness(Q16::from_i64(7)), Q16::ZERO);
    }

    #[test]
    fn norm_fitness_trace_is_content_addressed() {
        let pid = Digest::of_bytes(b"p");
        let cid = Digest::of_bytes(b"c");
        let t1 = NormFitnessTrace::empty(cid, pid);
        let t2 = NormFitnessTrace::empty(cid, pid);
        assert_eq!(t1.trace_id, t2.trace_id);
        assert_ne!(t1.trace_id, Digest::ZERO);
    }
}

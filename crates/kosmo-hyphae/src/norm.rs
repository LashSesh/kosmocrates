use kosmo_core::{Digest, Q16};
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
        Self { candidate_id, name, description, fitness_score, evidence_bundle_id, policy_id }
    }

    /// Fitness above threshold is a metric — it does NOT confer trust.
    pub fn exceeds_fitness_threshold(&self, threshold: Q16) -> bool {
        self.fitness_score.exceeds(threshold)
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
        Self { trace_id, candidate_id, observations: vec![], policy_id }
    }

    /// Append an observation. Returns a new trace (non-mutating).
    pub fn observe(mut self, fitness: Q16, evidence_ref: Digest) -> Self {
        let sequence = self.observations.len() as u64;
        self.observations.push(FitnessObservation { sequence, fitness, evidence_ref });
        self.trace_id = Digest::of(&TraceContent {
            candidate_id: self.candidate_id,
            observation_count: self.observations.len() as u64,
            policy_id: self.policy_id,
        });
        self
    }

    /// Latest fitness observation, or `Q16::ZERO` if no observations yet.
    pub fn latest_fitness(&self) -> Q16 {
        self.observations.last().map(|o| o.fitness).unwrap_or(Q16::ZERO)
    }

    /// Integer-averaged fitness across all observations (no floats — CROSS-007).
    pub fn average_fitness(&self) -> Q16 {
        if self.observations.is_empty() {
            return Q16::ZERO;
        }
        let sum_raw: i64 = self.observations.iter().map(|o| o.fitness.raw()).sum();
        Q16::from_raw(sum_raw / self.observations.len() as i64)
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
        let c1 = NormGeneCandidate::new(
            "test-norm".into(), "desc".into(), Q16::HALF, ev_id, pid,
        );
        let c2 = NormGeneCandidate::new(
            "test-norm".into(), "desc".into(), Q16::HALF, ev_id, pid,
        );
        assert_eq!(c1.candidate_id, c2.candidate_id);
        assert_ne!(c1.candidate_id, Digest::ZERO);
    }

    #[test]
    fn norm_gene_high_fitness_does_not_confer_trust() {
        // SAFETY INVARIANT: exceeds_fitness_threshold is a metric only.
        // Spec: do not treat NormGeneCandidate as a trusted norm.
        let pid = Digest::of_bytes(b"p");
        let cand = NormGeneCandidate::new(
            "high-fit".into(), "".into(), Q16::ONE, Digest::ZERO, pid,
        );
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
    fn norm_fitness_trace_is_content_addressed() {
        let pid = Digest::of_bytes(b"p");
        let cid = Digest::of_bytes(b"c");
        let t1 = NormFitnessTrace::empty(cid, pid);
        let t2 = NormFitnessTrace::empty(cid, pid);
        assert_eq!(t1.trace_id, t2.trace_id);
        assert_ne!(t1.trace_id, Digest::ZERO);
    }
}

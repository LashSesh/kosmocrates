//! Norm lifecycle — Phase 4-full completion.
//!
//! A *norm* is a structural invariant a collapse run must satisfy to be
//! considered well-formed. In v0.1 norms are opaque identifiers with a weight;
//! a future phase will materialise them as composable predicates evaluated
//! against the candidate and evidence chain.
//!
//! Types exported here:
//! - [`NormSpec`]            — declares a norm (id, kind, weight, predicate).
//! - [`NormFitness`]         — measures how well a candidate satisfies one norm.
//! - [`ConstraintLattice`]   — partial order over constraints by implication.
//! - [`CollapseCertificate`] — content-addressed proof of a well-formed collapse.

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::field_cube::EvidenceRef;
use crate::report::CommitOutcome;
use crate::Result;

// ─────────────────────────────────────────────────────────────────────────────
// NormSpec
// ─────────────────────────────────────────────────────────────────────────────

/// Declares a structural norm. The `predicate` string is an opaque identifier
/// resolved by a future norm registry; the v0.1 MVP treats it as metadata.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NormSpec {
    pub id: String,
    pub name: String,
    pub description: String,
    pub kind: NormKind,
    /// Relative importance weight in [0.0, ∞). Weight 0 always fails.
    pub weight: f64,
    pub predicate: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NormKind {
    /// About the shape of the collapse plan (ordering invariants, step counts).
    Structural,
    /// About the content of candidates (completeness, value validity).
    Content,
    /// About evidence coverage attached to the commit.
    Evidence,
}

// ─────────────────────────────────────────────────────────────────────────────
// NormFitness
// ─────────────────────────────────────────────────────────────────────────────

/// Fitness of a single candidate-against-norm evaluation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NormFitness {
    pub norm_id: String,
    /// Satisfaction score in [0.0, 1.0]. 1.0 = fully satisfied.
    pub score: f64,
    pub passed: bool,
    pub message: String,
}

impl NormFitness {
    /// Evaluate a norm against an opaque `payload` (bytes to be refined in
    /// Phase 5+). MVP rule: a norm passes iff its weight is positive.
    pub fn evaluate(norm: &NormSpec, _payload: &[u8]) -> Self {
        let passed = norm.weight > 0.0;
        NormFitness {
            norm_id: norm.id.clone(),
            score: if passed { 1.0 } else { 0.0 },
            passed,
            message: if passed {
                "ok".into()
            } else {
                "norm skipped: zero weight".into()
            },
        }
    }

    /// Evaluate a batch of norms and return one `NormFitness` per norm.
    /// Output is sorted by `norm_id` for determinism.
    pub fn evaluate_all(norms: &[NormSpec], payload: &[u8]) -> Vec<Self> {
        let mut out: Vec<Self> = norms.iter().map(|n| Self::evaluate(n, payload)).collect();
        out.sort_by(|a, b| a.norm_id.cmp(&b.norm_id));
        out
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ConstraintLattice
// ─────────────────────────────────────────────────────────────────────────────

/// Partial order over constraints by implication strength.
///
/// v0.1 derives order from the constraint `weight` field: constraint A implies
/// B when A.weight ≥ B.weight. A future phase will derive this from a proper
/// lattice over the predicate language.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct ConstraintLattice {
    /// Directed implication edges: `(source_id, implied_id)`.
    /// Source has strictly higher weight than implied.
    pub implications: Vec<(String, String)>,
    /// Constraint IDs at the top — highest implication strength.
    pub maximal: Vec<String>,
    /// Constraint IDs at the bottom — lowest implication strength.
    pub minimal: Vec<String>,
}

impl ConstraintLattice {
    /// Build a weight-based lattice from `(id, weight)` pairs.
    ///
    /// All output vectors are sorted for determinism.
    pub fn from_weights(constraints: &[(String, f64)]) -> Self {
        if constraints.is_empty() {
            return Self::default();
        }
        // Sort by weight descending, then by id ascending for tie-breaks.
        let mut sorted = constraints.to_vec();
        sorted.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.0.cmp(&b.0))
        });

        let max_w = sorted.first().map(|(_, w)| *w).unwrap_or(0.0);
        let min_w = sorted.last().map(|(_, w)| *w).unwrap_or(0.0);

        let mut maximal: Vec<String> = sorted
            .iter()
            .filter(|(_, w)| (w - max_w).abs() < f64::EPSILON)
            .map(|(id, _)| id.clone())
            .collect();
        maximal.sort();

        let mut minimal: Vec<String> = sorted
            .iter()
            .filter(|(_, w)| (w - min_w).abs() < f64::EPSILON)
            .map(|(id, _)| id.clone())
            .collect();
        minimal.sort();

        // Implication edges: pairs where the left has strictly higher weight.
        let mut implications: Vec<(String, String)> = Vec::new();
        for i in 0..sorted.len() {
            for j in (i + 1)..sorted.len() {
                if (sorted[i].1 - sorted[j].1).abs() > f64::EPSILON {
                    implications.push((sorted[i].0.clone(), sorted[j].0.clone()));
                }
            }
        }
        implications.sort();
        implications.dedup();

        ConstraintLattice {
            implications,
            maximal,
            minimal,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CollapseCertificate
// ─────────────────────────────────────────────────────────────────────────────

/// Content-addressed proof that a specific collapse run was well-formed:
/// all required hard constraints were covered, the gate passed, and the commit
/// outcome is recorded.
///
/// The `id` field is the SHA-256 / JCS content address of all other fields;
/// use [`CollapseCertificate::with_id`] to finalise it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CollapseCertificate {
    /// SHA-256 hex of this certificate (assigned by [`Self::with_id`]).
    pub id: String,
    pub field_cube_id: String,
    pub collapse_plan_id: String,
    pub candidate_id: String,
    /// Hex address of the `GateReport` that gated this candidate.
    pub gate_report_address: String,
    pub norm_scores: Vec<NormFitness>,
    pub all_hard_constraints_covered: bool,
    pub commit_outcome: CommitOutcome,
    pub evidence: Vec<EvidenceRef>,
    /// Logical clock from `TraversalRunDescriptor.started_at_logical`.
    pub issued_at_logical: u64,
}

impl CollapseCertificate {
    /// Compute and assign the certificate's SHA-256 content address.
    ///
    /// The `id` field is excluded from the hash input (set to `""` before
    /// hashing) so that the address is a pure function of the certificate's
    /// content.
    pub fn with_id(mut self) -> Result<Self> {
        let mut probe = self.clone();
        probe.id = String::new();
        let h = content_address(&probe)?;
        self.id = h.iter().map(|b| format!("{:02x}", b)).collect();
        Ok(self)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CollapseCertificateSignatureEvidence
// ─────────────────────────────────────────────────────────────────────────────

/// Signature-layer evidence that can be attached to a `CollapseCertificate`
/// to prove that the collapse was evaluated against a structural operator
/// and signature gate (spec §15.2).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CollapseCertificateSignatureEvidence {
    pub operator_id: String,
    pub signature_id: String,
    pub diagnostics_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_outcome_id: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::CommitOutcome;

    fn norm(id: &str, weight: f64) -> NormSpec {
        NormSpec {
            id: id.into(),
            name: id.into(),
            description: "test norm".into(),
            kind: NormKind::Structural,
            weight,
            predicate: "always_true".into(),
        }
    }

    #[test]
    fn norm_fitness_score_in_unit_interval() {
        let n = norm("n.1", 1.0);
        let f = NormFitness::evaluate(&n, b"payload");
        assert!((0.0..=1.0).contains(&f.score), "score={}", f.score);
        assert!(f.passed);
    }

    #[test]
    fn norm_fitness_zero_weight_fails() {
        let n = norm("n.0", 0.0);
        let f = NormFitness::evaluate(&n, b"payload");
        assert!(!f.passed);
        assert_eq!(f.score, 0.0);
    }

    #[test]
    fn norm_fitness_evaluate_all_sorted() {
        let norms = vec![norm("n.b", 1.0), norm("n.a", 0.5)];
        let results = NormFitness::evaluate_all(&norms, b"");
        // Must be sorted by norm_id
        assert_eq!(results[0].norm_id, "n.a");
        assert_eq!(results[1].norm_id, "n.b");
    }

    #[test]
    fn constraint_lattice_maximal_and_minimal() {
        let constraints = vec![
            ("c.a".to_string(), 1.0f64),
            ("c.b".to_string(), 0.5),
            ("c.c".to_string(), 0.5),
        ];
        let lattice = ConstraintLattice::from_weights(&constraints);
        assert_eq!(lattice.maximal, vec!["c.a"]);
        // Both c.b and c.c share the minimum weight
        assert!(lattice.minimal.contains(&"c.b".to_string()));
        assert!(lattice.minimal.contains(&"c.c".to_string()));
        // c.a implies both c.b and c.c (it has higher weight)
        assert!(lattice
            .implications
            .contains(&("c.a".to_string(), "c.b".to_string())));
        assert!(lattice
            .implications
            .contains(&("c.a".to_string(), "c.c".to_string())));
    }

    #[test]
    fn constraint_lattice_is_deterministic() {
        let constraints = vec![("c.a".to_string(), 1.0f64), ("c.b".to_string(), 0.5)];
        let l1 = ConstraintLattice::from_weights(&constraints);
        let l2 = ConstraintLattice::from_weights(&constraints);
        assert_eq!(l1, l2);
    }

    #[test]
    fn constraint_lattice_empty_input() {
        let lattice = ConstraintLattice::from_weights(&[]);
        assert!(lattice.implications.is_empty());
        assert!(lattice.maximal.is_empty());
        assert!(lattice.minimal.is_empty());
    }

    #[test]
    fn collapse_certificate_has_stable_address() {
        let cert = CollapseCertificate {
            id: String::new(),
            field_cube_id: "fc.1".into(),
            collapse_plan_id: "plan.fc.1".into(),
            candidate_id: "cand.abc12345".into(),
            gate_report_address: "0".repeat(64),
            norm_scores: vec![],
            all_hard_constraints_covered: true,
            commit_outcome: CommitOutcome::EvidenceOnly {
                candidate_id: "cand.abc12345".into(),
                evidence: vec![],
            },
            evidence: vec![],
            issued_at_logical: 0,
        }
        .with_id()
        .unwrap();
        assert_eq!(cert.id.len(), 64, "expected 64-char hex address");
        // Re-derive from identical data → same address.
        let cert2 = CollapseCertificate {
            id: String::new(),
            ..cert.clone()
        }
        .with_id()
        .unwrap();
        assert_eq!(cert.id, cert2.id, "certificate address is not stable");
    }

    #[test]
    fn collapse_certificate_different_content_different_address() {
        let base = CollapseCertificate {
            id: String::new(),
            field_cube_id: "fc.1".into(),
            collapse_plan_id: "plan.fc.1".into(),
            candidate_id: "cand.aaa".into(),
            gate_report_address: "0".repeat(64),
            norm_scores: vec![],
            all_hard_constraints_covered: true,
            commit_outcome: CommitOutcome::EvidenceOnly {
                candidate_id: "cand.aaa".into(),
                evidence: vec![],
            },
            evidence: vec![],
            issued_at_logical: 0,
        };
        let c1 = base.clone().with_id().unwrap();
        let c2 = CollapseCertificate {
            candidate_id: "cand.bbb".into(),
            commit_outcome: CommitOutcome::EvidenceOnly {
                candidate_id: "cand.bbb".into(),
                evidence: vec![],
            },
            ..base
        }
        .with_id()
        .unwrap();
        assert_ne!(
            c1.id, c2.id,
            "distinct content must yield distinct addresses"
        );
    }
}

//! `ConvergenceField` (§13).

use serde::{Deserialize, Serialize};

use super::primitives::{content_address, EvidenceRef, Fixed, Hash256, PhaseError};

/// `ConvergenceField` (§13.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConvergenceField {
    /// Content-addressed field id.
    pub field_id: Hash256,
    /// Owning cluster id.
    pub cluster_id: Hash256,
    /// Sorted input pulse ids.
    pub input_pulses: Vec<Hash256>,
    /// Optional memory projection hash.
    pub memory_projection_hash: Option<Hash256>,
    /// Convergence score.
    pub convergence_score: Fixed,
    /// Conflict score.
    pub conflict_score: Fixed,
    /// Optional stabilised intent hash.
    pub stabilized_intent_hash: Option<Hash256>,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl ConvergenceField {
    /// Recompute `field_id` after sorting the pulse list.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        self.input_pulses.sort();
        let mut probe = self.clone();
        probe.field_id = Hash256::zero();
        self.field_id = content_address(&probe)?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convergence_field_sorts_pulses_before_hashing() {
        let mut a = [0u8; 32];
        a[0] = 1;
        let mut b = [0u8; 32];
        b[0] = 2;
        let f1 = ConvergenceField {
            field_id: Hash256::zero(),
            cluster_id: Hash256::zero(),
            input_pulses: vec![Hash256(a), Hash256(b)],
            memory_projection_hash: None,
            convergence_score: Fixed::Rational { num: 7, den: 10 },
            conflict_score: Fixed::Rational { num: 1, den: 10 },
            stabilized_intent_hash: None,
            evidence_refs: Vec::new(),
        }
        .with_id()
        .unwrap();
        let f2 = ConvergenceField {
            field_id: Hash256::zero(),
            cluster_id: Hash256::zero(),
            input_pulses: vec![Hash256(b), Hash256(a)],
            memory_projection_hash: None,
            convergence_score: Fixed::Rational { num: 7, den: 10 },
            conflict_score: Fixed::Rational { num: 1, den: 10 },
            stabilized_intent_hash: None,
            evidence_refs: Vec::new(),
        }
        .with_id()
        .unwrap();
        assert_eq!(f1.field_id, f2.field_id);
    }
}

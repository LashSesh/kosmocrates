//! `MorphodynamicField`, `ClusterMorphologyEvent` and
//! `MorphologyDecision` (§10).

use serde::{Deserialize, Serialize};

use super::primitives::{
    content_address, fixed_add, fixed_mul, EvidenceRef, Fixed, Hash256, PhaseError,
};

/// `MorphodynamicField` (§10.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MorphodynamicField {
    /// Content-addressed field id.
    pub field_id: Hash256,
    /// Optional parent cluster id.
    pub parent_cluster_id: Option<Hash256>,
    /// `Φ(x, t)` — phase resonance field.
    pub phase_resonance_field: Fixed,
    /// `µ(x, t)` — growth field.
    pub growth_field: Fixed,
    /// `α` weight.
    pub alpha: Fixed,
    /// `β` weight.
    pub beta: Fixed,
    /// `H = α·Φ + β·µ`.
    pub morphodynamic_potential: Fixed,
    /// Endogeny score.
    pub endogeny_score: Fixed,
    /// Exogeny score.
    pub exogeny_score: Fixed,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl MorphodynamicField {
    /// Recompute `field_id` and `morphodynamic_potential` from the
    /// stored components. The potential formula is the spec's
    /// `H(x, t) = α·Φ(x, t) + β·µ(x, t)` (§4.2).
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        let term_phi = fixed_mul(&self.alpha, &self.phase_resonance_field);
        let term_mu = fixed_mul(&self.beta, &self.growth_field);
        self.morphodynamic_potential = fixed_add(&term_phi, &term_mu);
        let mut probe = self.clone();
        probe.field_id = Hash256::zero();
        self.field_id = content_address(&probe)?;
        Ok(self)
    }

    /// Construct a minimal field with sensible weights and zero scores.
    pub fn minimal(
        parent_cluster_id: Option<Hash256>,
        phase_resonance_field: Fixed,
        growth_field: Fixed,
    ) -> Result<Self, PhaseError> {
        let half = Fixed::Rational { num: 1, den: 2 };
        MorphodynamicField {
            field_id: Hash256::zero(),
            parent_cluster_id,
            phase_resonance_field,
            growth_field,
            alpha: half.clone(),
            beta: half,
            morphodynamic_potential: Fixed::Rational { num: 0, den: 1 },
            endogeny_score: Fixed::Rational { num: 1, den: 2 },
            exogeny_score: Fixed::Rational { num: 1, den: 2 },
            evidence_refs: Vec::new(),
        }
        .with_id()
    }
}

/// Morphology event kinds (§10.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ClusterMorphologyEvent {
    /// Grow the cluster.
    Grow,
    /// Split into sub-clusters.
    Split,
    /// Fuse with a sibling cluster.
    Fuse,
    /// Decay (members lose coherence).
    Decay,
    /// Replicate (copy with shared trace).
    Replicate,
    /// Stabilise.
    Stabilize,
    /// Dissolve working state.
    DissolveWorkingState,
    /// Compact to trace.
    CompactToTrace,
}

/// `MorphologyDecision` (§10.2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MorphologyDecision {
    /// Content-addressed decision id.
    pub decision_id: Hash256,
    /// Cluster id.
    pub cluster_id: Hash256,
    /// Event the gate evaluated.
    pub event: ClusterMorphologyEvent,
    /// Morphodynamic potential at decision time.
    pub morphodynamic_potential: Fixed,
    /// Hash of the gate report that produced this decision.
    pub gate_report_hash: Hash256,
    /// Whether the morphology gate allowed the event.
    pub allowed: bool,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl MorphologyDecision {
    /// Recompute `decision_id`.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        let mut probe = self.clone();
        probe.decision_id = Hash256::zero();
        self.decision_id = content_address(&probe)?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn morphodynamic_potential_matches_formula() {
        let field = MorphodynamicField::minimal(
            None,
            Fixed::quantize(0.4, 9).unwrap(),
            Fixed::quantize(0.6, 9).unwrap(),
        )
        .unwrap();
        // H = 0.5 * 0.4 + 0.5 * 0.6 = 0.5
        assert_eq!(
            field.morphodynamic_potential,
            Fixed::quantize(0.5, 9).unwrap()
        );
    }

    #[test]
    fn morphology_decision_is_content_addressed() {
        let d = MorphologyDecision {
            decision_id: Hash256::zero(),
            cluster_id: Hash256::zero(),
            event: ClusterMorphologyEvent::Grow,
            morphodynamic_potential: Fixed::quantize(0.5, 9).unwrap(),
            gate_report_hash: Hash256::zero(),
            allowed: true,
            evidence_refs: Vec::new(),
        }
        .with_id()
        .unwrap();
        let again = d.clone().with_id().unwrap();
        assert_eq!(d.decision_id, again.decision_id);
    }
}

//! Layer C8 — Self Model Tensor and Dual Trigger Feedback (§13).
//!
//! `SelfModelTensor` is a deterministic snapshot of self-coherence:
//! environment alignment, entropy delta, coupling matrix hash and a
//! reflexive modulation describing the predicted post-update drift.
//! `DualTriggerFeedbackGate` enforces the `(t1 ∧ t̄2 ∧ Δϕ)` ordering
//! per §13.2.

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::primitives::{fixed_ge, fixed_le, CognitionError, Fixed};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReflexiveModulation {
    pub modulation_id: Hash256,
    pub before_state_hash: Hash256,
    pub after_state_hash: Hash256,
    pub predicted_drift: Fixed,
    pub stabilizing: bool,
}

impl ReflexiveModulation {
    pub fn with_id(mut self) -> Result<Self, CognitionError> {
        let mut probe = self.clone();
        probe.modulation_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| CognitionError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.modulation_id = Hash256(id);
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfModelTensor {
    pub tensor_id: Hash256,
    pub resonance_state_hash: Hash256,
    pub environment_alignment: Fixed,
    pub entropy_delta: Fixed,
    pub coupling_matrix_hash: Hash256,
    pub reflexive_modulation: ReflexiveModulation,
    pub self_coherence_score: Fixed,
}

impl SelfModelTensor {
    pub fn with_id(mut self) -> Result<Self, CognitionError> {
        let mut probe = self.clone();
        probe.tensor_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| CognitionError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.tensor_id = Hash256(id);
        Ok(self)
    }

    /// `G_self`: self-coherence ≥ θ_self ∧ predicted drift ≤ max_drift.
    pub fn passes_gate(&self, min_coherence: &Fixed, max_drift: &Fixed) -> bool {
        fixed_ge(&self.self_coherence_score, min_coherence)
            && fixed_le(&self.reflexive_modulation.predicted_drift, max_drift)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DualTriggerFeedbackGate {
    pub gate_id: Hash256,
    pub trigger1_threshold: Fixed,
    pub trigger2_threshold: Fixed,
    pub phase_shift: Fixed,
    pub aggregation_window: u64,
    pub aggregated_energy: Fixed,
    pub trigger1_fired: bool,
    pub trigger2_fired: bool,
    pub fired: bool,
}

impl DualTriggerFeedbackGate {
    pub fn with_id(mut self) -> Result<Self, CognitionError> {
        let mut probe = self.clone();
        probe.gate_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| CognitionError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.gate_id = Hash256(id);
        Ok(self)
    }
}

/// `G_dual = 1 ⇔ E(t) > θ1 ∧ Ē[t, t+w] > θ2 ∧ Δϕ ≥ ϕ_min`.
///
/// In v0.1 we use `instantaneous_energy` as `E(t)`,
/// `aggregated_energy` as `Ē[t, t+w]` and `phase_shift` as `Δϕ`. The
/// trigger fires *only* when both energy thresholds are crossed AND
/// the phase ordering invariant holds — the spec calls this an
/// "ordered trigger sequence" (§21.1.15).
pub fn evaluate_dual_trigger(
    instantaneous_energy: &Fixed,
    aggregated_energy: &Fixed,
    phase_shift: &Fixed,
    trigger1_threshold: Fixed,
    trigger2_threshold: Fixed,
    phase_min: &Fixed,
    aggregation_window: u64,
) -> Result<DualTriggerFeedbackGate, CognitionError> {
    let trigger1_fired = super::primitives::fixed_ge(instantaneous_energy, &trigger1_threshold);
    let trigger2_fired = super::primitives::fixed_ge(aggregated_energy, &trigger2_threshold);
    let phase_ok = super::primitives::fixed_ge(phase_shift, phase_min);
    // Ordered trigger sequence: t1 must fire before (or with) t2; in
    // a single-shot evaluation we just require both to be true.
    let fired = trigger1_fired && trigger2_fired && phase_ok;
    DualTriggerFeedbackGate {
        gate_id: Hash256::zero(),
        trigger1_threshold,
        trigger2_threshold,
        phase_shift: phase_shift.clone(),
        aggregation_window,
        aggregated_energy: aggregated_energy.clone(),
        trigger1_fired,
        trigger2_fired,
        fired,
    }
    .with_id()
}

/// Build a SelfModelTensor stub from the resonance state hash and a
/// before/after pair of state hashes.
pub fn build_self_model(
    resonance_state_hash: Hash256,
    before: Hash256,
    after: Hash256,
    alignment: Fixed,
    entropy_delta: Fixed,
    coherence: Fixed,
) -> Result<SelfModelTensor, CognitionError> {
    let coupling_matrix_hash = Hash256(
        content_address(&("self.coupling.matrix", &before, &after)).map_err(|e| {
            CognitionError::Canonicalization {
                reason: e.to_string(),
            }
        })?,
    );
    let modulation = ReflexiveModulation {
        modulation_id: Hash256::zero(),
        before_state_hash: before,
        after_state_hash: after,
        predicted_drift: super::primitives::fixed_abs_diff(&entropy_delta, &Fixed::zero()),
        stabilizing: alignment.cmp(&Fixed::zero()) != std::cmp::Ordering::Less,
    }
    .with_id()?;
    SelfModelTensor {
        tensor_id: Hash256::zero(),
        resonance_state_hash,
        environment_alignment: alignment,
        entropy_delta,
        coupling_matrix_hash,
        reflexive_modulation: modulation,
        self_coherence_score: coherence,
    }
    .with_id()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_excessive_drift() {
        let m = build_self_model(
            Hash256::zero(),
            Hash256::zero(),
            Hash256::zero(),
            Fixed::quantize(1.0, 9).unwrap(),
            Fixed::quantize(5.0, 9).unwrap(),
            Fixed::quantize(0.9, 9).unwrap(),
        )
        .unwrap();
        // Tight max_drift ⇒ gate fails despite high coherence.
        assert!(!m.passes_gate(
            &Fixed::quantize(0.5, 9).unwrap(),
            &Fixed::quantize(1.0, 9).unwrap()
        ));
    }

    #[test]
    fn dual_trigger_requires_ordered_trigger_sequence() {
        let g = evaluate_dual_trigger(
            &Fixed::quantize(2.0, 9).unwrap(),
            &Fixed::quantize(0.0, 9).unwrap(),
            &Fixed::quantize(1.0, 9).unwrap(),
            Fixed::quantize(1.0, 9).unwrap(),
            Fixed::quantize(1.0, 9).unwrap(),
            &Fixed::quantize(0.5, 9).unwrap(),
            8,
        )
        .unwrap();
        // First trigger fires but aggregated energy is below θ2 ⇒ no fire.
        assert!(g.trigger1_fired);
        assert!(!g.trigger2_fired);
        assert!(!g.fired);
    }
}

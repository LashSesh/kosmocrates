//! Field signal computation for the traversal dynamics layer.
//! FIELD-01: empty state set MUST produce neutral signal.

use serde::{Deserialize, Serialize};

use crate::dynamic_state::{CanonicalNumber, DynamicError, Hash256, LiftedState, stable_id_of};
use crate::dynamic_policy::DynamicPolicy;

// ───────────────────────────────────────────────────────────────────────────────
// FieldSignal
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldSignal {
    pub signal_id: Hash256,
    pub alignment: CanonicalNumber,
    pub dispersion: CanonicalNumber,
    pub pressure: CanonicalNumber,
    pub state_count: usize,
}

impl FieldSignal {
    pub fn with_id(mut self) -> Result<Self, DynamicError> {
        let mut probe = self.clone();
        probe.signal_id = Hash256::zero();
        let id = stable_id_of(&probe)?;
        self.signal_id = id;
        Ok(self)
    }
}

// ───────────────────────────────────────────────────────────────────────────────
// FieldAbsorber trait
// ───────────────────────────────────────────────────────────────────────────────

pub trait FieldAbsorber {
    fn absorb(&self, states: &[LiftedState], policy: &DynamicPolicy) -> Result<FieldSignal, DynamicError>;
}

// ───────────────────────────────────────────────────────────────────────────────
// DefaultFieldAbsorber
// ───────────────────────────────────────────────────────────────────────────────

pub struct DefaultFieldAbsorber {
    pub alpha: CanonicalNumber,
    pub beta: CanonicalNumber,
}

impl Default for DefaultFieldAbsorber {
    fn default() -> Self {
        DefaultFieldAbsorber {
            alpha: CanonicalNumber::quantize_default(1.0).unwrap(),
            beta: CanonicalNumber::quantize_default(1.0).unwrap(),
        }
    }
}

impl FieldAbsorber for DefaultFieldAbsorber {
    fn absorb(&self, states: &[LiftedState], _policy: &DynamicPolicy) -> Result<FieldSignal, DynamicError> {
        // FIELD-01: empty state set → neutral signal
        if states.is_empty() {
            let signal = FieldSignal {
                signal_id: Hash256::zero(),
                alignment: CanonicalNumber::zero(),
                dispersion: CanonicalNumber::zero(),
                pressure: CanonicalNumber::zero(),
                state_count: 0,
            };
            return signal.with_id();
        }

        // 1. Sort states by state_id (deterministic order)
        let mut sorted: Vec<&LiftedState> = states.iter().collect();
        sorted.sort_by_key(|s| &s.state_id);

        let n = sorted.len() as f64;

        // 2. alignment = weighted coherence of auxiliary coordinate
        //    (mean of auxiliary.to_f64() values)
        let alignment_raw: f64 = sorted.iter().map(|s| s.auxiliary.to_f64()).sum::<f64>() / n;

        // 3. dispersion = mean component-wise variance of base dimension coords
        let base_dim = sorted[0].lift_profile.base_dimension;
        let dispersion_raw = if base_dim == 0 || n == 0.0 {
            0.0
        } else {
            let mut total_variance = 0.0;
            for dim in 0..base_dim {
                let vals: Vec<f64> = sorted.iter().map(|s| {
                    if dim < s.coords.len().saturating_sub(1) {
                        s.coords[dim].to_f64()
                    } else {
                        0.0
                    }
                }).collect();
                let mean = vals.iter().sum::<f64>() / n;
                let var = vals.iter().map(|&v| (v - mean) * (v - mean)).sum::<f64>() / n;
                total_variance += var;
            }
            total_variance / base_dim as f64
        };

        // 4. pressure = alpha * alignment + beta * dispersion
        let alpha = self.alpha.to_f64();
        let beta = self.beta.to_f64();
        let pressure_raw = alpha * alignment_raw + beta * dispersion_raw;

        // 5. Quantize all to CanonicalNumber::quantize_default
        let alignment = CanonicalNumber::quantize_default(alignment_raw)?;
        let dispersion = CanonicalNumber::quantize_default(dispersion_raw)?;
        let pressure = CanonicalNumber::quantize_default(pressure_raw)?;

        // 6. signal_id = stable_id_of(&signal_without_id)
        let signal = FieldSignal {
            signal_id: Hash256::zero(),
            alignment,
            dispersion,
            pressure,
            state_count: states.len(),
        };
        signal.with_id()
    }
}

// ───────────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamic_state::{AuxiliaryKind, BaseState, DefaultStateLift, LiftProfile, StateLift};

    fn make_lifted_state(tick: u64, coords: Vec<f64>) -> LiftedState {
        let base = BaseState {
            state_id: Hash256::zero(),
            coords: coords.iter().map(|&v| CanonicalNumber::quantize_default(v).unwrap()).collect(),
            weight: CanonicalNumber::quantize_default(1.0).unwrap(),
            evidence_refs: vec![],
        };
        let base = base.with_id().unwrap();
        DefaultStateLift.lift(&base, tick).unwrap()
    }

    #[test]
    fn field_empty_is_neutral() {
        let absorber = DefaultFieldAbsorber::default();
        let policy = crate::dynamic_policy::DynamicPolicy::default();
        let signal = absorber.absorb(&[], &policy).unwrap();
        assert_eq!(signal.alignment.to_f64(), 0.0);
        assert_eq!(signal.dispersion.to_f64(), 0.0);
        assert_eq!(signal.pressure.to_f64(), 0.0);
        assert_eq!(signal.state_count, 0);
    }

    #[test]
    fn field_absorb_is_deterministic() {
        let absorber = DefaultFieldAbsorber::default();
        let policy = crate::dynamic_policy::DynamicPolicy::default();
        let states = vec![
            make_lifted_state(1, vec![1.0, 2.0]),
            make_lifted_state(2, vec![3.0, 4.0]),
        ];
        let s1 = absorber.absorb(&states, &policy).unwrap();
        let s2 = absorber.absorb(&states, &policy).unwrap();
        assert_eq!(s1.signal_id, s2.signal_id);
        assert_eq!(s1.alignment, s2.alignment);
        assert_eq!(s1.dispersion, s2.dispersion);
        assert_eq!(s1.pressure, s2.pressure);
    }
}

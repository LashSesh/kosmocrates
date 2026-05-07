//! Dynamic policy types for the traversal dynamics layer.
//! POLICY-01: all adaptations are deterministic and replayable.

use serde::{Deserialize, Serialize};

use crate::dynamic_state::{CanonicalNumber, DynamicError, Hash256, stable_id_of};

// ───────────────────────────────────────────────────────────────────────────────
// DynamicPolicyKind
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DynamicPolicyKind {
    Explore,
    Exploit,
    Homeostasis,
}

// ───────────────────────────────────────────────────────────────────────────────
// DynamicPolicy
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicPolicy {
    pub policy_id: Hash256,
    pub kind: DynamicPolicyKind,
    pub learning_rate: CanonicalNumber,
    pub decay: CanonicalNumber,
    pub split_threshold: CanonicalNumber,
    pub merge_threshold: CanonicalNumber,
    pub prune_threshold: CanonicalNumber,
    pub variance_threshold: CanonicalNumber,
    pub strict_coherence: bool,
    pub target_density: Option<CanonicalNumber>,
}

impl DynamicPolicy {
    pub fn with_id(mut self) -> Result<Self, DynamicError> {
        let mut probe = self.clone();
        probe.policy_id = Hash256::zero();
        let id = stable_id_of(&probe)?;
        self.policy_id = id;
        Ok(self)
    }

    /// POLICY-01: deterministic homeostasis adaptation.
    pub fn adapt_homeostasis(
        &self,
        current_density: f64,
        target_density: f64,
    ) -> Result<Self, DynamicError> {
        if !current_density.is_finite() {
            return Err(DynamicError::NonFiniteNumber { field: "current_density".into() });
        }
        if !target_density.is_finite() || target_density == 0.0 {
            return Err(DynamicError::NonFiniteNumber { field: "target_density".into() });
        }
        let rho_ratio = current_density / target_density;

        if rho_ratio > 1.2 {
            // High density: increase decay and prune_threshold by 10%, decrease learning_rate by 10%
            let lr = CanonicalNumber::quantize_default(self.learning_rate.to_f64() * 0.9)?;
            let decay = CanonicalNumber::quantize_default(self.decay.to_f64() * 1.1)?;
            let prune = CanonicalNumber::quantize_default(self.prune_threshold.to_f64() * 1.1)?;
            let mut new_policy = self.clone();
            new_policy.policy_id = Hash256::zero();
            new_policy.learning_rate = lr;
            new_policy.decay = decay;
            new_policy.prune_threshold = prune;
            new_policy.with_id()
        } else if rho_ratio < 0.8 {
            // Low density: decrease decay and prune_threshold by 10%, increase learning_rate by 10%
            let lr = CanonicalNumber::quantize_default(self.learning_rate.to_f64() * 1.1)?;
            let decay = CanonicalNumber::quantize_default(self.decay.to_f64() * 0.9)?;
            let prune = CanonicalNumber::quantize_default(self.prune_threshold.to_f64() * 0.9)?;
            let mut new_policy = self.clone();
            new_policy.policy_id = Hash256::zero();
            new_policy.learning_rate = lr;
            new_policy.decay = decay;
            new_policy.prune_threshold = prune;
            new_policy.with_id()
        } else {
            // In range: return self unchanged
            Ok(self.clone())
        }
    }
}

impl Default for DynamicPolicy {
    fn default() -> Self {
        let mut p = DynamicPolicy {
            policy_id: Hash256::zero(),
            kind: DynamicPolicyKind::Explore,
            learning_rate: CanonicalNumber::quantize_default(0.01).unwrap(),
            decay: CanonicalNumber::quantize_default(0.001).unwrap(),
            split_threshold: CanonicalNumber::quantize_default(10.0).unwrap(),
            merge_threshold: CanonicalNumber::quantize_default(0.1).unwrap(),
            prune_threshold: CanonicalNumber::quantize_default(0.001).unwrap(),
            variance_threshold: CanonicalNumber::quantize_default(0.5).unwrap(),
            strict_coherence: false,
            target_density: None,
        };
        // Compute policy_id
        let mut probe = p.clone();
        probe.policy_id = Hash256::zero();
        if let Ok(id) = stable_id_of(&probe) {
            p.policy_id = id;
        }
        p
    }
}

// ───────────────────────────────────────────────────────────────────────────────
// QuantizationPolicy
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuantizationPolicy {
    pub scale: u32,
    pub rounding: String,
}

impl Default for QuantizationPolicy {
    fn default() -> Self {
        QuantizationPolicy {
            scale: 9,
            rounding: "banker".to_string(),
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────────
// PolicyScheduleEntry
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyScheduleEntry {
    pub at_tick: u64,
    pub policy: DynamicPolicy,
}

// ───────────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_homeostasis_adapts_high_density() {
        let policy = DynamicPolicy::default();
        let orig_lr = policy.learning_rate.to_f64();
        let orig_decay = policy.decay.to_f64();
        let orig_prune = policy.prune_threshold.to_f64();
        // rho_ratio = 2.0 / 1.0 = 2.0 > 1.2
        let adapted = policy.adapt_homeostasis(2.0, 1.0).unwrap();
        // learning_rate decreases by 10%
        let diff_lr = (adapted.learning_rate.to_f64() - orig_lr * 0.9).abs();
        assert!(diff_lr < 1e-6, "learning_rate should decrease: {} vs {}", adapted.learning_rate.to_f64(), orig_lr * 0.9);
        // decay increases by 10%
        let diff_decay = (adapted.decay.to_f64() - orig_decay * 1.1).abs();
        assert!(diff_decay < 1e-6, "decay should increase: {} vs {}", adapted.decay.to_f64(), orig_decay * 1.1);
        // prune_threshold increases by 10%
        let diff_prune = (adapted.prune_threshold.to_f64() - orig_prune * 1.1).abs();
        assert!(diff_prune < 1e-6, "prune_threshold should increase: {} vs {}", adapted.prune_threshold.to_f64(), orig_prune * 1.1);
        // policy_id should be recomputed (different from original)
        assert_ne!(adapted.policy_id, policy.policy_id);
    }

    #[test]
    fn policy_homeostasis_adapts_low_density() {
        let policy = DynamicPolicy::default();
        let orig_lr = policy.learning_rate.to_f64();
        let orig_decay = policy.decay.to_f64();
        let orig_prune = policy.prune_threshold.to_f64();
        // rho_ratio = 0.5 / 1.0 = 0.5 < 0.8
        let adapted = policy.adapt_homeostasis(0.5, 1.0).unwrap();
        // learning_rate increases by 10%
        let diff_lr = (adapted.learning_rate.to_f64() - orig_lr * 1.1).abs();
        assert!(diff_lr < 1e-6, "learning_rate should increase: {} vs {}", adapted.learning_rate.to_f64(), orig_lr * 1.1);
        // decay decreases by 10%
        let diff_decay = (adapted.decay.to_f64() - orig_decay * 0.9).abs();
        assert!(diff_decay < 1e-6, "decay should decrease: {} vs {}", adapted.decay.to_f64(), orig_decay * 0.9);
        // prune_threshold decreases by 10%
        let diff_prune = (adapted.prune_threshold.to_f64() - orig_prune * 0.9).abs();
        assert!(diff_prune < 1e-6, "prune_threshold should decrease: {} vs {}", adapted.prune_threshold.to_f64(), orig_prune * 0.9);
    }

    #[test]
    fn policy_homeostasis_stable_in_range() {
        let policy = DynamicPolicy::default();
        // rho_ratio = 1.0 — in range, no change
        let adapted = policy.adapt_homeostasis(1.0, 1.0).unwrap();
        assert_eq!(adapted.learning_rate, policy.learning_rate);
        assert_eq!(adapted.decay, policy.decay);
        assert_eq!(adapted.prune_threshold, policy.prune_threshold);
    }
}

//! `LocalResonanceProcessor` and `ResonancePulse` (§9).
//!
//! The processor projects an arbitrary input descriptor (already
//! canonicalised by upstream layers) onto a `TridentVector` plus a
//! scalar resonance pulse. The implementation is deterministic and
//! float-free: nonlinearities are fixed-point look-up tables.

use serde::{Deserialize, Serialize};

use super::primitives::{content_address, EvidenceRef, Fixed, Hash256, PhaseError};
use super::trident_vector::TridentVector;

/// Nonlinearity used by the local resonance processor (§9.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ResonanceNonlinearity {
    /// Logistic-like piecewise approximation in fixed-point.
    Logistic,
    /// Tanh-like piecewise approximation in fixed-point.
    TanhApprox,
    /// Saturating-linear (clamped to `[0, 1]`).
    SaturatingLinear,
    /// Piecewise-fixed table.
    PiecewiseFixed,
}

/// `LocalResonanceProcessor` (§9.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalResonanceProcessor {
    /// Content-addressed processor id.
    pub processor_id: Hash256,
    /// Hash of the input projection used to derive the trident.
    pub input_projection_hash: Hash256,
    /// Hash of the weight profile.
    pub weight_profile_hash: Hash256,
    /// Nonlinearity policy.
    pub nonlinearity: ResonanceNonlinearity,
    /// Output trident vector.
    pub output_trident: TridentVector,
    /// Output resonance pulse.
    pub resonance_pulse: Fixed,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl LocalResonanceProcessor {
    /// Recompute the content-addressed `processor_id`.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        let mut probe = self.clone();
        probe.processor_id = Hash256::zero();
        self.processor_id = content_address(&probe)?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn processor_is_content_addressed() {
        let p = LocalResonanceProcessor {
            processor_id: Hash256::zero(),
            input_projection_hash: Hash256::zero(),
            weight_profile_hash: Hash256::zero(),
            nonlinearity: ResonanceNonlinearity::SaturatingLinear,
            output_trident: TridentVector::zero(),
            resonance_pulse: Fixed::Rational { num: 1, den: 2 },
            evidence_refs: Vec::new(),
        }
        .with_id()
        .unwrap();
        let again = p.clone().with_id().unwrap();
        assert_eq!(p.processor_id, again.processor_id);
    }
}

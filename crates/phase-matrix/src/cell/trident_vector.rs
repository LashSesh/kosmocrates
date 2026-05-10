//! `TridentVector` (§4.1, §8.1) — semantic density × structural
//! coherence × temporal phase. Activation potential is the product of
//! the three components.

use serde::{Deserialize, Serialize};

use super::primitives::{fixed_mul, Fixed};

/// `T(t) = (s(t), c(t), p(t))` per §4.1.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TridentVector {
    /// `s(t)` — semantic density / significance.
    pub semantic_density: Fixed,
    /// `c(t)` — structural coherence / dominance.
    pub structural_coherence: Fixed,
    /// `p(t)` — temporal phase readiness.
    pub temporal_phase: Fixed,
}

impl TridentVector {
    /// Build a zero trident.
    pub fn zero() -> Self {
        let z = Fixed::Rational { num: 0, den: 1 };
        TridentVector {
            semantic_density: z.clone(),
            structural_coherence: z.clone(),
            temporal_phase: z,
        }
    }

    /// Activation potential `D(t) = s · c · p`.
    pub fn activation_potential(&self) -> Fixed {
        fixed_mul(
            &fixed_mul(&self.semantic_density, &self.structural_coherence),
            &self.temporal_phase,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activation_potential_is_deterministic() {
        let t = TridentVector {
            semantic_density: Fixed::quantize(0.5, 9).unwrap(),
            structural_coherence: Fixed::quantize(0.5, 9).unwrap(),
            temporal_phase: Fixed::quantize(0.5, 9).unwrap(),
        };
        assert_eq!(t.activation_potential(), t.activation_potential());
    }

    #[test]
    fn zero_trident_has_zero_potential() {
        assert_eq!(
            TridentVector::zero().activation_potential(),
            Fixed::Rational { num: 0, den: 1 }
        );
    }
}

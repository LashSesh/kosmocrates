//! `ResonancePulse` and `PhaseBin` (§9.2).

use serde::{Deserialize, Serialize};

use super::primitives::{content_address, EvidenceRef, Fixed, Hash256, PhaseError};
use super::trident_vector::TridentVector;

/// Phase quantisation used to bin pulses (§9.2).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PhaseBin {
    /// Continuous phase value in `[0, 1)`.
    Continuous(Fixed),
    /// `k`-polar bin index.
    KPolar(u32),
    /// Tripolar bin: `-1`, `0`, `+1`.
    Tripolar(i8),
    /// Quadrupolar bin pair.
    Quadrupolar(i8, i8),
}

/// `ResonancePulse` (§9.2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResonancePulse {
    /// Content-addressed pulse id.
    pub pulse_id: Hash256,
    /// Source cell id.
    pub source_cell_id: Hash256,
    /// Trident snapshot at pulse time.
    pub trident: TridentVector,
    /// Activation potential.
    pub activation_potential: Fixed,
    /// Phase bin.
    pub phase_bin: PhaseBin,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl ResonancePulse {
    /// Recompute `pulse_id`.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        let mut probe = self.clone();
        probe.pulse_id = Hash256::zero();
        self.pulse_id = content_address(&probe)?;
        Ok(self)
    }

    /// Build a deterministic pulse from a cell + trident.
    pub fn from_cell(
        source_cell_id: Hash256,
        trident: TridentVector,
        activation_potential: Fixed,
        bin_index: u32,
    ) -> Result<Self, PhaseError> {
        ResonancePulse {
            pulse_id: Hash256::zero(),
            source_cell_id,
            trident,
            activation_potential,
            phase_bin: PhaseBin::KPolar(bin_index),
            evidence_refs: Vec::new(),
        }
        .with_id()
    }
}

impl PartialOrd for ResonancePulse {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ResonancePulse {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.pulse_id.cmp(&other.pulse_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulse_id_is_stable() {
        let a = ResonancePulse::from_cell(
            Hash256::zero(),
            TridentVector::zero(),
            Fixed::Rational { num: 1, den: 2 },
            3,
        )
        .unwrap();
        let b = ResonancePulse::from_cell(
            Hash256::zero(),
            TridentVector::zero(),
            Fixed::Rational { num: 1, den: 2 },
            3,
        )
        .unwrap();
        assert_eq!(a.pulse_id, b.pulse_id);
    }
}

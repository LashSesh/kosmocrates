//! Phase carrier specification for v0.3.
//!
//! A `PhaseCarrierSpecV3` declares one cyclic axis of the hypertoroidal
//! phase space: its rational frequency, its phase offset, the role it
//! plays in the schedule, and the quantisation policy used to discretise
//! its phase before any gate decision is taken.

use serde::{Deserialize, Serialize};

use super::primitives::{Fixed, RationalFixed, StableId};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PhaseCarrierSpecV3 {
    pub carrier_id: StableId,
    pub frequency: RationalFixed,
    pub phase_offset: RationalFixed,
    pub role: PhaseCarrierRole,
    pub quantization: PhaseQuantizationV3,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PhaseCarrierRole {
    ExtrinsicProgress,
    IntrinsicResonance,
    HorizonVisibility,
    FinalizationReplay,
    Auxiliary(String),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PhaseQuantizationV3 {
    Continuous,
    KPolar { k: u32 },
    Tripolar { tau_low: Fixed, tau_high: Fixed },
    Quadrupolar { tau: Fixed },
}

impl PhaseCarrierSpecV3 {
    /// Build a minimal carrier with sensible defaults.
    pub fn minimal(carrier_id: impl Into<String>, frequency: RationalFixed) -> Self {
        PhaseCarrierSpecV3 {
            carrier_id: carrier_id.into(),
            frequency,
            phase_offset: RationalFixed::zero(),
            role: PhaseCarrierRole::ExtrinsicProgress,
            quantization: PhaseQuantizationV3::Continuous,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_carrier_has_zero_offset() {
        let c = PhaseCarrierSpecV3::minimal("c.test", RationalFixed::new(1, 2).unwrap());
        assert_eq!(c.phase_offset, RationalFixed::zero());
    }
}

//! Ophanim — closed wing cycles with roundtrip self-calibration (spec ch. 12.3).

use kosmo_cdk_core::Status;
use kosmo_core::Digest;
use serde::{Deserialize, Serialize};

/// An Ophanim is a closed wing cycle `Ω = (w1, …, wk, w1)` that produces not mere
/// output but repeatable self-calibration: `d(Ω^t(x), A⋆) → 0` (Def. 12.1). The
/// `trajectory` records the distance proxy (a `dim`) at each iteration; the cycle
/// is admissible only if **replayable** and **roundtrip-stable** (the
/// OphanimRoundtripGate, §25).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OphanimCycle {
    pub wings: Vec<Digest>,
    /// `dim` at each iteration of the cycle — the self-calibration record.
    pub trajectory: Vec<u32>,
    pub attractor_dim: u32,
    pub replayable: bool,
}

impl OphanimCycle {
    pub fn id(&self) -> Digest {
        Digest::of(self)
    }

    /// `d(Ω^t(x), A⋆) → 0`: the distance-to-attractor is monotone non-increasing
    /// and the cycle actually approaches the attractor (reaches it, or strictly
    /// nears it). A diverging cycle does not converge.
    pub fn converges(&self) -> bool {
        let dist: Vec<u32> = self
            .trajectory
            .iter()
            .map(|d| d.abs_diff(self.attractor_dim))
            .collect();
        match dist.as_slice() {
            [] => false,
            [only] => *only == 0,
            _ => {
                if dist.windows(2).any(|w| w[1] > w[0]) {
                    return false; // distance increased → diverging
                }
                *dist.last().unwrap() == 0 || dist.last().unwrap() < dist.first().unwrap()
            }
        }
    }

    /// OphanimRoundtripGate (§25): a cycle is admissible only if it is replayable
    /// (negative test 10) **and** convergent — fail-closed.
    pub fn roundtrip(&self) -> Status {
        if !self.replayable || !self.converges() {
            return Status::Reject;
        }
        Status::Pass
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cycle(trajectory: Vec<u32>, attractor_dim: u32, replayable: bool) -> OphanimCycle {
        OphanimCycle {
            wings: vec![Digest::of_bytes(b"w0"), Digest::of_bytes(b"w1")],
            trajectory,
            attractor_dim,
            replayable,
        }
    }

    #[test]
    fn a_converging_replayable_cycle_passes_roundtrip() {
        // 8 → 4 → 2 → 1, attractor 1: monotone approach, reaches the attractor.
        let c = cycle(vec![8, 4, 2, 1], 1, true);
        assert!(c.converges());
        assert_eq!(c.roundtrip(), Status::Pass);
    }

    #[test]
    fn a_diverging_cycle_does_not_converge() {
        // distance grows (2 → 5 from attractor 1) → not self-calibrating.
        let c = cycle(vec![3, 6, 9], 1, true);
        assert!(!c.converges());
        assert_eq!(c.roundtrip(), Status::Reject);
    }

    #[test]
    fn a_non_replayable_cycle_is_rejected() {
        // Negative test 10 — an Ophanim cycle that is not replayable fails the gate
        // even if it would converge.
        let c = cycle(vec![4, 2, 1], 1, false);
        assert!(c.converges());
        assert_eq!(c.roundtrip(), Status::Reject, "not replayable → fail-closed");
    }
}

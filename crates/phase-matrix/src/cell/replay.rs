//! Replay verification (§22.2 cluster-replay rule).

use serde::{Deserialize, Serialize};

use super::primitives::{content_address, Hash256, PhaseError};

/// Per-cycle replay observation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ReplayObservation {
    /// Hash recorded the first time the cycle ran.
    pub expected_hash: Hash256,
    /// Hash observed when re-running with the same inputs.
    pub observed_hash: Hash256,
    /// Whether the observation passed.
    pub passed: bool,
}

/// Verify that the canonical bytes of `actual` match `expected`.
pub fn verify_cycle_replay<T: serde::Serialize>(
    expected: &Hash256,
    actual: &T,
) -> Result<ReplayObservation, PhaseError> {
    let observed = content_address(actual)?;
    Ok(ReplayObservation {
        expected_hash: expected.clone(),
        observed_hash: observed.clone(),
        passed: *expected == observed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_passes_for_byte_identical_input() {
        let v = ("trace", 17u32);
        let expected = content_address(&v).unwrap();
        let obs = verify_cycle_replay(&expected, &v).unwrap();
        assert!(obs.passed);
    }
}

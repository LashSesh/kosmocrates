//! Replay verification (§7.2 rule 5, §10.6, §13).
//!
//! A run is byte-identical iff re-deriving the trial report from the
//! same `(spec, variant, workload, dataset, run_descriptor)` produces
//! the same `report_hash`. Anything else invalidates the run.

use serde::{Deserialize, Serialize};

use crate::primitives::{canonical_bytes, content_address, EvalError, Hash256};

/// Per-trial replay observation (§10.6).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ReplayObservation {
    /// Hash recorded the first time the trial was executed.
    pub expected_hash: Hash256,
    /// Hash observed when re-running with the same inputs.
    pub observed_hash: Hash256,
    /// Whether the observation passed.
    pub passed: bool,
}

/// `ReplayVerificationReport` summarising every trial's replay
/// observation. Required artefact per §17.2.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayVerificationReport {
    /// Content-addressed report id.
    pub report_id: Hash256,
    /// Anchoring spec hash.
    pub evaluation_spec_hash: Hash256,
    /// Per-trial replay observations.
    pub observations: Vec<ReplayObservation>,
    /// Number of trials whose replay failed.
    pub failed_count: u64,
}

impl ReplayVerificationReport {
    /// Recompute `report_id`.
    pub fn with_id(mut self) -> Result<Self, EvalError> {
        self.observations.sort();
        let mut probe = self.clone();
        probe.report_id = Hash256::zero();
        self.report_id = content_address(&probe)?;
        Ok(self)
    }
}

/// Verify that the canonical bytes of `actual` match `expected`. Used
/// by both the in-process replay path and the CLI `replay` subcommand.
pub fn verify_trial_replay<T: serde::Serialize>(
    expected_hash: &Hash256,
    actual: &T,
) -> Result<ReplayObservation, EvalError> {
    let observed_hash = content_address(actual)?;
    let _ = canonical_bytes(actual)?; // ensure JCS-safe
    Ok(ReplayObservation {
        expected_hash: expected_hash.clone(),
        observed_hash: observed_hash.clone(),
        passed: *expected_hash == observed_hash,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_passes_for_byte_identical_input() {
        let value = ("trial", 42u32);
        let expected = content_address(&value).unwrap();
        let obs = verify_trial_replay(&expected, &value).unwrap();
        assert!(obs.passed);
    }

    #[test]
    fn replay_fails_for_modified_input() {
        let v1 = ("trial", 42u32);
        let v2 = ("trial", 43u32);
        let expected = content_address(&v1).unwrap();
        let obs = verify_trial_replay(&expected, &v2).unwrap();
        assert!(!obs.passed);
    }
}

//! Failure taxonomy (§13).
//!
//! Each failure kind has a deterministic mandatory reaction that the
//! scorer applies when summarising the run.

use serde::{Deserialize, Serialize};

use crate::primitives::{content_address, EvalError, Hash256};

/// Per §13 — the canonical failure kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FailureKind {
    /// Hash or bytes diverge on replay → invalidate the run.
    ReplayMismatch,
    /// Crystal/event without ground-truth correspondence → safety
    /// metric increases.
    FalseCrystal,
    /// Ground-truth event not detected → recall drops.
    MissedEvent,
    /// CognitionCandidateBundle leads to incorrect finalisation →
    /// handoff precision drops.
    FalseHandoff,
    /// System blocks a correct candidate → hold calibration must be
    /// reviewed.
    OverHold,
    /// System lets an unstable candidate pass → fail-closed violation.
    UnderHold,
    /// SpiralMemory hit hurts the outcome.
    MemoryMislead,
    /// Wormhole admitted without reason / budget / TTL or with
    /// negative net utility → governance failure.
    WormholeAbuse,
    /// Test split influences calibration → entire evaluation invalid.
    CalibrationLeakage,
}

impl FailureKind {
    /// Whether the failure invalidates the run entirely (per §13's
    /// "Pflichtreaktion" column).
    pub fn invalidates_run(self) -> bool {
        matches!(
            self,
            FailureKind::ReplayMismatch | FailureKind::CalibrationLeakage
        )
    }

    /// Whether the failure should be counted as a safety event.
    pub fn is_safety_event(self) -> bool {
        matches!(
            self,
            FailureKind::FalseCrystal
                | FailureKind::FalseHandoff
                | FailureKind::UnderHold
                | FailureKind::WormholeAbuse
        )
    }
}

/// Recorded failure on a single trial.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FailureRecord {
    /// Content-addressed record id.
    pub record_id: Hash256,
    /// Failure kind.
    pub kind: FailureKind,
    /// Free-form message (no PII / wall-clock).
    pub message: String,
}

impl FailureRecord {
    /// Build a failure record with a content-addressed id.
    pub fn new(kind: FailureKind, message: impl Into<String>) -> Result<Self, EvalError> {
        let message = message.into();
        let mut r = FailureRecord {
            record_id: Hash256::zero(),
            kind,
            message: message.clone(),
        };
        r.record_id = content_address(&(kind, &message))?;
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_mismatch_invalidates_run() {
        assert!(FailureKind::ReplayMismatch.invalidates_run());
        assert!(FailureKind::CalibrationLeakage.invalidates_run());
        assert!(!FailureKind::FalseCrystal.invalidates_run());
    }

    #[test]
    fn safety_events_are_classified() {
        assert!(FailureKind::FalseCrystal.is_safety_event());
        assert!(FailureKind::UnderHold.is_safety_event());
        assert!(!FailureKind::OverHold.is_safety_event());
    }

    #[test]
    fn record_is_content_addressed() {
        let a = FailureRecord::new(FailureKind::FalseCrystal, "x").unwrap();
        let b = FailureRecord::new(FailureKind::FalseCrystal, "x").unwrap();
        assert_eq!(a.record_id, b.record_id);
    }
}

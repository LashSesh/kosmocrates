//! `CalibrationProfile`, `CalibrationLedgerEntry`, calibration policy
//! (§9, §10.2).

use serde::{Deserialize, Serialize};

use crate::primitives::{content_address, EvalError, Hash256};

/// Calibration policy attached to a variant.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CalibrationProfile {
    /// Logical name (`"frozen"`, `"adaptive_quantile"`, …).
    pub name: String,
    /// Whether thresholds may move during a run (must be false in the
    /// test split per §9.3).
    pub adaptive: bool,
    /// Optional initial threshold bundle hash.
    pub initial_threshold_hash: Option<Hash256>,
}

impl CalibrationProfile {
    /// Frozen profile — no threshold movement, valid in test split.
    pub fn frozen() -> Self {
        CalibrationProfile {
            name: "frozen".into(),
            adaptive: false,
            initial_threshold_hash: None,
        }
    }

    /// Adaptive quantile-style profile (rolling-history thresholds).
    pub fn adaptive_quantile() -> Self {
        CalibrationProfile {
            name: "adaptive_quantile".into(),
            adaptive: true,
            initial_threshold_hash: None,
        }
    }
}

/// Why a calibration step happened (§9.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CalibrationReason {
    /// Initial calibration on the calibration split.
    InitialCalibration,
    /// Drift detected during the validation split.
    DriftDetected,
    /// Manual override by an operator.
    ManualOverride,
    /// Scheduled re-calibration.
    Scheduled,
}

/// Single entry in the calibration ledger.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalibrationLedgerEntry {
    /// Content-addressed entry id.
    pub entry_id: Hash256,
    /// Threshold bundle that was active before the change.
    pub previous_threshold_hash: Hash256,
    /// Threshold bundle that became active after the change.
    pub new_threshold_hash: Hash256,
    /// Hash of the evidence window that justified the change.
    pub evidence_window_hash: Hash256,
    /// Why the calibration happened.
    pub reason: CalibrationReason,
    /// Whether the calibration was accepted (`false` ⇒ rejected, kept
    /// previous thresholds).
    pub accepted: bool,
}

impl CalibrationLedgerEntry {
    /// Recompute the content-addressed `entry_id`.
    pub fn with_id(mut self) -> Result<Self, EvalError> {
        let mut probe = self.clone();
        probe.entry_id = Hash256::zero();
        let id = content_address(&probe)?;
        self.entry_id = id;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_profile_disallows_adaptive() {
        assert!(!CalibrationProfile::frozen().adaptive);
    }

    #[test]
    fn ledger_entry_is_content_addressed() {
        let mut e = CalibrationLedgerEntry {
            entry_id: Hash256::zero(),
            previous_threshold_hash: Hash256::zero(),
            new_threshold_hash: Hash256::zero(),
            evidence_window_hash: Hash256::zero(),
            reason: CalibrationReason::InitialCalibration,
            accepted: true,
        };
        e = e.with_id().unwrap();
        let id = e.entry_id.clone();
        let recomputed = e.with_id().unwrap();
        assert_eq!(recomputed.entry_id, id);
    }
}

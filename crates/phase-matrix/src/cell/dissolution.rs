//! `DissolutionMode` and `DissolutionReport` (§16).
//!
//! The Dissolution-Grundsatz (§16.1) is enforced by [`DissolutionReport::validate_trace_preservation`]:
//! a report MUST preserve `retained_trace_hash`, `retained_evidence_hashes`
//! and `lifecycle_event_hashes`. Spurenlose Auflösung is rejected.

use serde::{Deserialize, Serialize};

use super::primitives::{content_address, Hash256, PhaseError};

/// `DissolutionMode` (§16.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DissolutionMode {
    /// Drop only the volatile working state.
    DropWorkingState,
    /// Compact the cluster into a stable trace.
    CompactToTrace,
    /// Persist evidence references only (no full trace body).
    PersistEvidenceOnly,
    /// Persist a cluster summary (lighter than the full trace).
    PersistClusterSummary,
    /// Archive the entire state.
    ArchiveFullState,
}

/// `DissolutionReport` (§16.2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DissolutionReport {
    /// Content-addressed report id.
    pub report_id: Hash256,
    /// Cluster id.
    pub cluster_id: Hash256,
    /// Dissolution mode.
    pub mode: DissolutionMode,
    /// Hash of the working state before dissolution.
    pub working_state_before_hash: Hash256,
    /// Hash of the retained trace artefact.
    pub retained_trace_hash: Hash256,
    /// Sorted evidence hashes that survived dissolution.
    pub retained_evidence_hashes: Vec<Hash256>,
    /// Sorted lifecycle-event hashes that survived dissolution.
    pub lifecycle_event_hashes: Vec<Hash256>,
    /// Whether the dissolution gate passed.
    pub passed: bool,
}

impl DissolutionReport {
    /// Recompute `report_id` after sorting evidence + lifecycle lists.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        self.retained_evidence_hashes.sort();
        self.retained_evidence_hashes.dedup();
        self.lifecycle_event_hashes.sort();
        self.lifecycle_event_hashes.dedup();
        let mut probe = self.clone();
        probe.report_id = Hash256::zero();
        self.report_id = content_address(&probe)?;
        Ok(self)
    }

    /// Enforce the Dissolution-Grundsatz: trace + at least one evidence
    /// reference + at least one lifecycle event MUST be preserved.
    pub fn validate_trace_preservation(&self) -> Result<(), PhaseError> {
        if self.retained_trace_hash == Hash256::zero() {
            return Err(PhaseError::Schema {
                reason: "dissolution: retained_trace_hash is zero".into(),
            });
        }
        if self.retained_evidence_hashes.is_empty() {
            return Err(PhaseError::Schema {
                reason: "dissolution: retained_evidence_hashes is empty".into(),
            });
        }
        if self.lifecycle_event_hashes.is_empty() {
            return Err(PhaseError::Schema {
                reason: "dissolution: lifecycle_event_hashes is empty".into(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dissolution_preserves_trace() {
        let r = DissolutionReport {
            report_id: Hash256::zero(),
            cluster_id: Hash256::zero(),
            mode: DissolutionMode::CompactToTrace,
            working_state_before_hash: Hash256::zero(),
            retained_trace_hash: {
                let mut x = [0u8; 32];
                x[0] = 9;
                Hash256(x)
            },
            retained_evidence_hashes: vec![Hash256::zero()],
            lifecycle_event_hashes: vec![Hash256::zero()],
            passed: true,
        };
        assert!(r.validate_trace_preservation().is_ok());
    }

    #[test]
    fn dissolution_without_trace_is_rejected() {
        let r = DissolutionReport {
            report_id: Hash256::zero(),
            cluster_id: Hash256::zero(),
            mode: DissolutionMode::DropWorkingState,
            working_state_before_hash: Hash256::zero(),
            retained_trace_hash: Hash256::zero(),
            retained_evidence_hashes: vec![],
            lifecycle_event_hashes: vec![],
            passed: false,
        };
        assert!(r.validate_trace_preservation().is_err());
    }
}

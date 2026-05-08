//! `CognitionReport` and `CognitionHoldReport` (§17). Every audit
//! reference is content-addressed and sorted before hashing.

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::primitives::{CognitionError, EvidenceRef};
use super::run_descriptor::{CognitionFailurePolicy, CognitionGateKind};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitionReport {
    pub report_id: Hash256,
    pub rd_hash: Hash256,
    pub cognition_state_id: Hash256,
    pub cognitive_state_5d_hash: Hash256,
    pub spiral_hitset_hash: Hash256,
    pub lattice_hash: Hash256,
    pub hypercube_puzzle_hash: Hash256,
    pub phase_panorama_hash: Hash256,
    pub self_model_tensor_hash: Hash256,
    pub attractor_map_hash: Hash256,
    pub handoff_gate_hash: Hash256,
    pub outcome_hash: Hash256,
    pub evidence_refs: Vec<EvidenceRef>,
}

impl CognitionReport {
    pub fn with_id(mut self) -> Result<Self, CognitionError> {
        let mut probe = self.clone();
        probe.report_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| CognitionError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.report_id = Hash256(id);
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitionDiagnostic {
    pub diagnostic_id: Hash256,
    pub message: String,
    pub gate: CognitionGateKind,
}

impl CognitionDiagnostic {
    pub fn new(
        gate: CognitionGateKind,
        message: impl Into<String>,
    ) -> Result<Self, CognitionError> {
        let mut d = CognitionDiagnostic {
            diagnostic_id: Hash256::zero(),
            message: message.into(),
            gate,
        };
        let id = content_address(&(d.gate, &d.message)).map_err(|e| {
            CognitionError::Canonicalization {
                reason: e.to_string(),
            }
        })?;
        d.diagnostic_id = Hash256(id);
        Ok(d)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CognitionRecoveryAction {
    QuerySpiralMemory,
    RefineConstraints,
    SplitHypercube,
    ExpandPanorama,
    AdmitWormhole,
    WaitForPhaseWindow,
    MigrateCarrier,
    CalibrateOperators,
    RequireHumanReview,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitionHoldReport {
    pub hold_id: Hash256,
    pub rd_hash: Hash256,
    pub cognition_state_id: Hash256,
    pub failed_gate: CognitionGateKind,
    pub failure_policy: CognitionFailurePolicy,
    pub diagnostics: Vec<CognitionDiagnostic>,
    pub recommended_next_step: Option<CognitionRecoveryAction>,
}

impl CognitionHoldReport {
    pub fn with_id(mut self) -> Result<Self, CognitionError> {
        let mut probe = self.clone();
        probe.hold_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| CognitionError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.hold_id = Hash256(id);
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum CognitionOutcome {
    CandidateBundle { bundle_hash: Hash256 },
    Hold { hold: CognitionHoldReport },
}

impl CognitionOutcome {
    pub fn outcome_hash(&self) -> Result<Hash256, CognitionError> {
        Ok(Hash256(content_address(self).map_err(|e| {
            CognitionError::Canonicalization {
                reason: e.to_string(),
            }
        })?))
    }
}

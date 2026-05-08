//! Layer C0 — Canonical Cognition State.
//!
//! `CanonicalCognitionState` ties the entire kernel together: every
//! sub-layer (5D state, lattice, puzzle, panorama, spiral memory,
//! self-model, scheduler) is referenced by content hash, so the
//! state's `state_id` is a function of every gate-relevant input.

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::cognitive_state_5d::CognitiveState5D;
use super::primitives::{CognitionError, EvidenceRef};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalCognitionState {
    pub state_id: Hash256,
    pub run_descriptor_hash: Hash256,
    pub null_center_id: Hash256,
    pub source_traversal_report_hash: Option<Hash256>,
    pub source_projection_report_hash: Option<Hash256>,
    pub cognitive_vector: CognitiveState5D,
    pub constraint_lattice_id: Hash256,
    pub hypercube_puzzle_id: Hash256,
    pub phase_panorama_id: Hash256,
    pub spiral_memory_context_id: Hash256,
    pub self_model_tensor_id: Hash256,
    pub scheduler_state_id: Hash256,
    pub evidence_refs: Vec<EvidenceRef>,
}

impl CanonicalCognitionState {
    /// Recompute the content-addressed `state_id` so two runs with the
    /// same field bytes yield the same id.
    pub fn with_id(mut self) -> Result<Self, CognitionError> {
        let mut probe = self.clone();
        probe.state_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| CognitionError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.state_id = Hash256(id);
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_state() -> CanonicalCognitionState {
        CanonicalCognitionState {
            state_id: Hash256::zero(),
            run_descriptor_hash: Hash256::zero(),
            null_center_id: Hash256::zero(),
            source_traversal_report_hash: None,
            source_projection_report_hash: None,
            cognitive_vector: CognitiveState5D::zero(),
            constraint_lattice_id: Hash256::zero(),
            hypercube_puzzle_id: Hash256::zero(),
            phase_panorama_id: Hash256::zero(),
            spiral_memory_context_id: Hash256::zero(),
            self_model_tensor_id: Hash256::zero(),
            scheduler_state_id: Hash256::zero(),
            evidence_refs: Vec::new(),
        }
    }

    #[test]
    fn canonical_cognition_state_is_content_addressed() {
        let a = empty_state().with_id().unwrap();
        let b = empty_state().with_id().unwrap();
        assert_eq!(a.state_id, b.state_id);
        let mut tampered = empty_state();
        tampered.run_descriptor_hash = {
            let mut alt = [0u8; 32];
            alt[0] = 0xff;
            Hash256(alt)
        };
        let tampered = tampered.with_id().unwrap();
        assert_ne!(a.state_id, tampered.state_id);
    }
}

//! Layer C10 (part 2) — Singularity Trigger and Projection Handoff
//! (§15).
//!
//! Per §15.3 the handoff gate is conjunctive over every sub-gate plus
//! `ReplayReady = 1`. Only when it fires may a `CognitionCandidateBundle`
//! be emitted; otherwise the cognition pipeline produces a
//! `CognitionHoldReport`.

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::primitives::{CognitionError, EvidenceRef, Fixed};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ProjectionHandoffPolicy {
    Hold,
    ProjectBestCandidate,
    ProjectTopK { k: u32 },
    WaitForHorizon,
    RequestCarrierMigration,
    RequireHumanReview,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitionCandidate {
    pub candidate_id: Hash256,
    pub source_path_id: Hash256,
    pub target_state_hash: Hash256,
    pub attractor_score: Fixed,
    pub constraint_support_score: Fixed,
    pub memory_support_score: Fixed,
    pub projection_readiness: Fixed,
}

impl PartialOrd for CognitionCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CognitionCandidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher projection_readiness first; ties broken by candidate_id.
        other
            .projection_readiness
            .cmp(&self.projection_readiness)
            .then(self.candidate_id.cmp(&other.candidate_id))
    }
}

impl CognitionCandidate {
    pub fn with_id(mut self) -> Result<Self, CognitionError> {
        let mut probe = self.clone();
        probe.candidate_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| CognitionError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.candidate_id = Hash256(id);
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitionCandidateBundle {
    pub bundle_id: Hash256,
    pub cognition_state_id: Hash256,
    pub panorama_id: Hash256,
    pub attractor_map_id: Hash256,
    pub candidate_states: Vec<CognitionCandidate>,
    pub recommended_projection_policy: ProjectionHandoffPolicy,
    pub evidence_refs: Vec<EvidenceRef>,
}

impl CognitionCandidateBundle {
    pub fn with_id(mut self) -> Result<Self, CognitionError> {
        // Highest readiness first, but the canonical bytes use a stable
        // (lexicographic-by-candidate_id) order so two bundles that
        // differ only in caller-side iteration land on the same id.
        self.candidate_states
            .sort_by(|a, b| a.candidate_id.cmp(&b.candidate_id));
        let mut probe = self.clone();
        probe.bundle_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| CognitionError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.bundle_id = Hash256(id);
        Ok(self)
    }
}

/// Per §15.3 — the handoff gate. Pass-only when every sub-gate fires.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitionHandoffGate {
    pub gate_id: Hash256,
    pub g_perc: bool,
    pub g_resonance: bool,
    pub g_panorama: bool,
    pub g_self: bool,
    pub g_trigger: bool,
    pub replay_ready: bool,
    pub passed: bool,
}

impl CognitionHandoffGate {
    pub fn evaluate(
        g_perc: bool,
        g_resonance: bool,
        g_panorama: bool,
        g_self: bool,
        g_trigger: bool,
        replay_ready: bool,
    ) -> Result<Self, CognitionError> {
        let passed = g_perc && g_resonance && g_panorama && g_self && g_trigger && replay_ready;
        let mut g = CognitionHandoffGate {
            gate_id: Hash256::zero(),
            g_perc,
            g_resonance,
            g_panorama,
            g_self,
            g_trigger,
            replay_ready,
            passed,
        };
        let id = content_address(&(
            g.g_perc,
            g.g_resonance,
            g.g_panorama,
            g.g_self,
            g.g_trigger,
            g.replay_ready,
            g.passed,
        ))
        .map_err(|e| CognitionError::Canonicalization {
            reason: e.to_string(),
        })?;
        g.gate_id = Hash256(id);
        Ok(g)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handoff_gate_fails_closed() {
        let g = CognitionHandoffGate::evaluate(true, true, true, true, false, true).unwrap();
        assert!(!g.passed);
        let g_pass = CognitionHandoffGate::evaluate(true, true, true, true, true, true).unwrap();
        assert!(g_pass.passed);
    }

    #[test]
    fn resonance_gate_blocks_independently() {
        let g = CognitionHandoffGate::evaluate(true, false, true, true, true, true).unwrap();
        assert!(!g.passed);
        assert!(!g.g_resonance);
        assert!(g.g_perc);
    }
}

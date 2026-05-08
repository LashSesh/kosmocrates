//! Layer C5 — Phase Panorama and Choice Geometry (§10).
//!
//! The panorama holds the 720-degree two-horizon model: the current
//! horizon (active states, open gates, local attractors) and the
//! counterfactual horizon (alternative attractors, blocked paths,
//! later phase windows). `coverage_score` measures how much of the
//! reachable solvability landscape is observable, anchoring the
//! `G_panorama` gate.

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::primitives::{CognitionError, EvidenceRef, Fixed};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PhasePathSummary {
    pub path_id: Hash256,
    pub phase_in: Fixed,
    pub phase_out: Fixed,
    pub weight: Fixed,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PhasePath {
    pub path_id: Hash256,
    pub from_state: Hash256,
    pub to_state: Hash256,
    pub status: PhasePathStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PhasePathStatus {
    Visible,
    Latent,
    Blocked,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AttractorCandidate {
    pub attractor_id: Hash256,
    pub state_hash: Hash256,
    pub depth: Fixed,
    pub stability: Fixed,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RecognitionBoundary {
    pub boundary_id: Hash256,
    pub from_horizon: Hash256,
    pub to_horizon: Hash256,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Horizon360 {
    pub horizon_id: Hash256,
    pub anchor_state: Hash256,
    pub phase_window_hashes: Vec<Hash256>,
    pub path_summaries: Vec<PhasePathSummary>,
    pub local_attractors: Vec<Hash256>,
    pub gate_status_hash: Hash256,
}

impl Horizon360 {
    pub fn with_id(mut self) -> Result<Self, CognitionError> {
        self.phase_window_hashes.sort();
        self.path_summaries.sort();
        self.local_attractors.sort();
        let mut probe = self.clone();
        probe.horizon_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| CognitionError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.horizon_id = Hash256(id);
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhasePanorama {
    pub panorama_id: Hash256,
    pub null_center_id: Hash256,
    pub current_horizon: Horizon360,
    pub counterfactual_horizon: Horizon360,
    pub visible_paths: Vec<PhasePath>,
    pub latent_paths: Vec<PhasePath>,
    pub blocked_paths: Vec<PhasePath>,
    pub attractor_candidates: Vec<AttractorCandidate>,
    pub recognition_boundaries: Vec<RecognitionBoundary>,
    pub coverage_score: Fixed,
}

impl PhasePanorama {
    pub fn with_id(mut self) -> Result<Self, CognitionError> {
        self.visible_paths.sort();
        self.latent_paths.sort();
        self.blocked_paths.sort();
        self.attractor_candidates.sort();
        self.recognition_boundaries.sort();
        let mut probe = self.clone();
        probe.panorama_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| CognitionError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.panorama_id = Hash256(id);
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChoiceGeometryReport {
    pub report_id: Hash256,
    pub panorama_id: Hash256,
    pub option_count: u64,
    pub visible_option_count: u64,
    pub blocked_option_count: u64,
    pub asymmetry_score: Fixed,
    pub convergence_score: Fixed,
    pub recognition_boundary_count: u64,
    pub evidence_refs: Vec<EvidenceRef>,
}

impl ChoiceGeometryReport {
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

/// Build a minimal panorama from a null center and a logical step.
/// The pipeline can extend this with caller-supplied paths/attractors;
/// the spec only mandates that current AND counterfactual horizons are
/// present and that ids are content-addressed.
pub fn build_panorama_minimal(
    null_center_id: &Hash256,
    logical_step: u64,
) -> Result<PhasePanorama, CognitionError> {
    let anchor = null_center_id.clone();
    let current = Horizon360 {
        horizon_id: Hash256::zero(),
        anchor_state: anchor.clone(),
        phase_window_hashes: Vec::new(),
        path_summaries: Vec::new(),
        local_attractors: Vec::new(),
        gate_status_hash: Hash256::zero(),
    }
    .with_id()?;
    let counterfactual = Horizon360 {
        horizon_id: Hash256::zero(),
        anchor_state: anchor.clone(),
        phase_window_hashes: Vec::new(),
        path_summaries: Vec::new(),
        local_attractors: Vec::new(),
        gate_status_hash: Hash256::zero(),
    }
    .with_id()?;
    let coverage_score = Fixed::Rational {
        num: 1,
        den: (logical_step as i128).max(1) + 1,
    };
    PhasePanorama {
        panorama_id: Hash256::zero(),
        null_center_id: null_center_id.clone(),
        current_horizon: current,
        counterfactual_horizon: counterfactual,
        visible_paths: Vec::new(),
        latent_paths: Vec::new(),
        blocked_paths: Vec::new(),
        attractor_candidates: Vec::new(),
        recognition_boundaries: Vec::new(),
        coverage_score,
    }
    .with_id()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_current_and_counterfactual_horizons() {
        let p = build_panorama_minimal(&Hash256::zero(), 1).unwrap();
        assert_ne!(p.current_horizon.horizon_id, Hash256::zero());
        assert_ne!(p.counterfactual_horizon.horizon_id, Hash256::zero());
    }
}

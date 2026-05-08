//! Layer C10 (part 1) — Singularity Trigger.
//!
//! Re-uses the 5D detector's `singular_window_open` plus an attractor
//! score gate to produce a deterministic trigger verdict for the
//! handoff layer.

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::attractor_map::AttractorMap;
use super::cognitive_state_5d::{detect_singularity, CognitiveState5D, SingularityDetectorReport};
use super::primitives::{fixed_ge, CognitionError, Fixed};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SingularityTriggerReport {
    pub report_id: Hash256,
    pub detector_report_id: Hash256,
    pub attractor_map_id: Hash256,
    pub trigger_score: Fixed,
    pub passed: bool,
}

impl SingularityTriggerReport {
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

pub fn evaluate_singularity_trigger(
    state: &CognitiveState5D,
    attractor_map: &AttractorMap,
    epsilon_det: &Fixed,
    epsilon_eigen: &Fixed,
    theta_trigger: &Fixed,
    min_trigger_score: &Fixed,
) -> Result<(SingularityDetectorReport, SingularityTriggerReport), CognitionError> {
    let detector = detect_singularity(state, epsilon_det, epsilon_eigen, theta_trigger)?;
    let trigger_score = detector.trigger_score.clone();
    let passed = detector.singular_window_open && fixed_ge(&trigger_score, min_trigger_score);
    let trigger = SingularityTriggerReport {
        report_id: Hash256::zero(),
        detector_report_id: detector.report_id.clone(),
        attractor_map_id: attractor_map.map_id.clone(),
        trigger_score,
        passed,
    }
    .with_id()?;
    Ok((detector, trigger))
}

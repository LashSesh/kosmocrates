//! Horizon gate combinators.
//!
//! `G_v0.3 = G_projection_v2 ∧ G_cross ∧ ReplayReady`.
//!
//! When a Projection-v0.2 outcome is not attached the combined gate
//! falls back to `G_cross ∧ ReplayReady` — but `FinalizedEmissionV3`
//! still requires the Projection-v0.2 layer to certify, since the spec
//! makes that gate conjunctive.

use serde::{Deserialize, Serialize};

use crate::dynamic_state::Hash256;

use super::horizon_crossing::HorizonCrossingReport;
use super::primitives::HorizonError;

/// Outcome of the v0.2 ↔ v0.3 gate merge.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombinedGateOutcome {
    pub projection_v2_passed: bool,
    pub horizon_v3_passed: bool,
    pub replay_ready: bool,
    pub passed: bool,
    pub projection_v2_certificate_hash: Option<Hash256>,
}

/// External handle to a Projection-v0.2 outcome. The horizon layer does
/// not own the v0.2 implementation; callers pass in a tiny snapshot
/// describing whether v0.2 admitted the projection and the address of
/// its certificate (for the certificate chain).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProjectionV2OutcomeRef {
    pub passed: bool,
    pub certificate_hash: Option<Hash256>,
}

pub fn merge_projection_and_horizon_gates(
    projection_v2: Option<&ProjectionV2OutcomeRef>,
    crossing: &HorizonCrossingReport,
    replay_ready: bool,
) -> Result<CombinedGateOutcome, HorizonError> {
    let projection_v2_passed = projection_v2.map(|p| p.passed).unwrap_or(false);
    let horizon_v3_passed = crossing.crossing_allowed;
    let passed = projection_v2_passed && horizon_v3_passed && replay_ready;
    Ok(CombinedGateOutcome {
        projection_v2_passed,
        horizon_v3_passed,
        replay_ready,
        passed,
        projection_v2_certificate_hash: projection_v2.and_then(|p| p.certificate_hash.clone()),
    })
}

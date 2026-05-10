//! Replay and verify for Metatron closure reports.
//!
//! Replay MUST be byte-identical: re-running run_metatron_closure on the
//! same inputs must produce a state with the same state_id.

use super::holistic_state::HolisticEigenmodeState;
use super::primitives::{content_address, Hash256, MetatronError};

/// Result of a closure replay check.
pub struct MetatronReplayResult {
    pub original_state_id: Hash256,
    pub replayed_state_id: Hash256,
    pub passed: bool,
}

/// Verify byte-identity between original and replayed state.
pub fn verify_metatron_replay(
    original: &HolisticEigenmodeState,
    replayed: &HolisticEigenmodeState,
) -> Result<MetatronReplayResult, MetatronError> {
    let orig_id = content_address(original)?;
    let replay_id = content_address(replayed)?;
    let passed = orig_id == replay_id;
    Ok(MetatronReplayResult {
        original_state_id: Hash256(orig_id.0),
        replayed_state_id: Hash256(replay_id.0),
        passed,
    })
}

/// Verify that a state's declared state_id is its correct content address.
pub fn verify_holistic_eigenmode_state(
    state: &HolisticEigenmodeState,
) -> Result<bool, MetatronError> {
    // Recompute from the zero-ID form (same as build() does).
    let mut partial = state.clone();
    partial.state_id = Hash256::zero();
    let computed = content_address(&partial)?;
    Ok(computed == state.state_id)
}

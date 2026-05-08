//! Replay helpers — byte-identity check across two cognition runs.

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::primitives::CognitionError;

pub fn replay_hash_of(
    rd_hash: &Hash256,
    cognition_state_id: &Hash256,
    outcome_hash: &Hash256,
) -> Result<Hash256, CognitionError> {
    let h = content_address(&(
        "cognition-v0.1-replay",
        rd_hash,
        cognition_state_id,
        outcome_hash,
    ))
    .map_err(|e| CognitionError::Canonicalization {
        reason: e.to_string(),
    })?;
    Ok(Hash256(h))
}

pub fn assert_replay_match(expected: &Hash256, observed: &Hash256) -> Result<(), CognitionError> {
    if expected == observed {
        Ok(())
    } else {
        Err(CognitionError::ReplayMismatch {
            reason: format!("expected={} observed={}", expected.hex(), observed.hex()),
        })
    }
}

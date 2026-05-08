//! Replay invariants for the horizon layer.
//!
//! The pipeline's `replay_hash` is the canonical hash of
//! `(rd_hash, chart_id, crossing_report_hash, finalized_emission_hash)`.
//! Two runs with byte-identical descriptors and inputs MUST produce the
//! same replay hash.

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::primitives::HorizonError;

pub fn replay_hash_of(
    rd_hash: &Hash256,
    chart_id: &Hash256,
    crossing_report_hash: &Hash256,
    finalized_emission_hash: Option<&Hash256>,
) -> Result<Hash256, HorizonError> {
    let h = content_address(&(
        "horizon-v0.3-replay",
        rd_hash,
        chart_id,
        crossing_report_hash,
        finalized_emission_hash,
    ))
    .map_err(|e| HorizonError::Canonicalization {
        reason: e.to_string(),
    })?;
    Ok(Hash256(h))
}

pub fn assert_replay_match(expected: &Hash256, observed: &Hash256) -> Result<(), HorizonError> {
    if expected == observed {
        Ok(())
    } else {
        Err(HorizonError::ReplayMismatch {
            reason: format!("expected={} observed={}", expected.hex(), observed.hex()),
        })
    }
}

//! Replay verification for NCTCS closure bundles.
//!
//! Replay MUST be byte-identical: re-running close_nctcs on the same
//! inputs must produce a bundle with the same bundle_id.

use super::primitives::{content_address, Hash256, NctcsError};
use super::report::NctcsClosureBundle;

/// Result of a bundle replay check.
#[derive(Debug)]
pub struct NctcsReplayResult {
    pub original_bundle_id: Hash256,
    pub replayed_bundle_id: Hash256,
    pub passed: bool,
}

/// Verify that `replayed` is byte-identical to `original` by comparing
/// their content-addressed bundle_ids.
pub fn verify_nctcs_replay(
    original: &NctcsClosureBundle,
    replayed: &NctcsClosureBundle,
) -> Result<NctcsReplayResult, NctcsError> {
    let orig_id = content_address(original)?;
    let replay_id = content_address(replayed)?;
    let passed = orig_id == replay_id;
    Ok(NctcsReplayResult {
        original_bundle_id: Hash256(orig_id.0),
        replayed_bundle_id: Hash256(replay_id.0),
        passed,
    })
}

/// Verify that a bundle's declared bundle_id matches its computed content address.
pub fn verify_nctcs_bundle(bundle: &NctcsClosureBundle) -> Result<bool, NctcsError> {
    // Recompute from the zero-ID form (same as build() does).
    let mut partial = bundle.clone();
    partial.bundle_id = Hash256::zero();
    let computed = content_address(&partial)?;
    Ok(computed == bundle.bundle_id)
}

//! Replay verification (§5.3 replay / §16 pt. 6).
//!
//! Verifies that the ledger chain is intact and that all hashed artifacts
//! match their recorded hashes. A replay pass is required for
//! EmpiricalImprovement.

use serde::{Deserialize, Serialize};

use crate::bundle::BundleManifest;
use crate::ledger::ValidationRunLedger;
use crate::primitives::{content_address, Hash256, ValidationError};

/// Result of a replay verification pass.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayResult {
    pub replay_id: Hash256,
    pub run_id: String,
    pub ledger_chain_ok: bool,
    pub entry_links_ok: bool,
    pub bundle_hashes_ok: bool,
    pub replay_identity: bool,
    pub failure_reasons: Vec<String>,
}

impl ReplayResult {
    pub fn content_hash(&self) -> Result<Hash256, ValidationError> {
        content_address(self)
    }
}

/// Verify the ledger chain and bundle hashes.
pub fn verify_replay(
    ledger: &ValidationRunLedger,
    bundle: Option<&BundleManifest>,
) -> Result<ReplayResult, ValidationError> {
    let mut reasons = Vec::new();

    let ledger_chain_ok = match ledger.verify_chain() {
        Ok(()) => true,
        Err(e) => {
            reasons.push(format!("ledger chain: {e}"));
            false
        }
    };

    let entry_links_ok = match ledger.verify_entry_links() {
        Ok(()) => true,
        Err(e) => {
            reasons.push(format!("entry links: {e}"));
            false
        }
    };

    let bundle_hashes_ok = match bundle {
        None => true, // no bundle to check
        Some(b) => {
            // Re-zero manifest_id before hashing — matching how create_bundle computed it.
            let check = crate::bundle::BundleManifest {
                manifest_id: Hash256::zero(),
                ..b.clone()
            };
            match content_address(&check) {
                Ok(h) if h == b.manifest_id => true,
                Ok(h) => {
                    reasons.push(format!(
                        "bundle manifest_id mismatch: stored {} computed {}",
                        b.manifest_id.hex(),
                        h.hex()
                    ));
                    false
                }
                Err(e) => {
                    reasons.push(format!("bundle hash: {e}"));
                    false
                }
            }
        }
    };

    let replay_identity = ledger_chain_ok && entry_links_ok && bundle_hashes_ok;

    let partial = ReplayResult {
        replay_id: Hash256::zero(),
        run_id: ledger.run_id.clone(),
        ledger_chain_ok,
        entry_links_ok,
        bundle_hashes_ok,
        replay_identity,
        failure_reasons: reasons,
    };
    let replay_id = content_address(&partial)?;
    Ok(ReplayResult { replay_id, ..partial })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_plan::ValidationPhase;
    use crate::executor::CommandResult;
    use crate::ledger::ValidationRunLedger;

    #[test]
    fn replay_passes_for_intact_ledger() {
        let mut ledger =
            ValidationRunLedger::new("run_001".into(), Hash256::zero(), Hash256::zero()).unwrap();
        let result = CommandResult {
            command_id: Hash256::zero(),
            exit_code: 0,
            stdout_hash: Hash256::zero(),
            stderr_hash: Hash256::zero(),
            stdout_len: 0,
            stderr_len: 0,
            stdout_path: None,
        };
        ledger.append(ValidationPhase::Quality, &result, vec![]).unwrap();
        let replay = verify_replay(&ledger, None).unwrap();
        assert!(replay.replay_identity);
    }

    #[test]
    fn replay_fails_for_broken_ledger() {
        let mut ledger =
            ValidationRunLedger::new("run_002".into(), Hash256::zero(), Hash256::zero()).unwrap();
        let result = CommandResult {
            command_id: Hash256::zero(),
            exit_code: 0,
            stdout_hash: Hash256::zero(),
            stderr_hash: Hash256::zero(),
            stdout_len: 0,
            stderr_len: 0,
            stdout_path: None,
        };
        ledger.append(ValidationPhase::Quality, &result, vec![]).unwrap();
        // Break the chain hash.
        ledger.chain_hash = Hash256::zero();
        // chain_hash is now wrong (unless the single entry happened to produce zero).
        // Only fails if chain_hash diverges; let's force it:
        ledger.entries[0].entry_id =
            Hash256::from_hex(&"ff".repeat(32)).unwrap_or(Hash256::zero());
        let replay = verify_replay(&ledger, None).unwrap();
        assert!(!replay.replay_identity);
    }
}

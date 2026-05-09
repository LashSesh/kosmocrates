//! `EvaluationRunLedger` — append-only, hash-chained per §10.5.
//!
//! Every entry is content-addressed, the chain hash binds entries in
//! order, and `append_to_ledger` refuses any insertion that would break
//! the chain. This is the spec's audit anchor: from the ledger you can
//! deterministically re-derive every TrialReport id and the final
//! summary report id.

use serde::{Deserialize, Serialize};

use crate::primitives::{content_address, EvalError, Hash256};

/// Status of a single run after execution + replay verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RunStatus {
    /// Run completed and replay verified byte-identical.
    Completed,
    /// Run completed but replay produced different bytes; the run MUST
    /// be excluded from scoring (§7.2 rule 5).
    InvalidReplayMismatch,
    /// Run failed during execution (e.g. budget exhausted).
    InvalidExecution,
    /// Schema or split leakage was detected.
    InvalidLeakage,
}

/// Single ledger entry (§10.5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationRunEntry {
    /// Content-addressed entry id.
    pub entry_id: Hash256,
    /// Variant id (matches the spec).
    pub variant_id: String,
    /// Workload id.
    pub workload_id: String,
    /// Dataset id.
    pub dataset_id: Hash256,
    /// Run descriptor hash.
    pub run_descriptor_hash: Hash256,
    /// Trial report content hash.
    pub report_hash: Hash256,
    /// Replay verification hash.
    pub replay_hash: Hash256,
    /// Run status.
    pub status: RunStatus,
}

impl EvaluationRunEntry {
    /// Recompute `entry_id` from the rest of the fields.
    pub fn with_id(mut self) -> Result<Self, EvalError> {
        let mut probe = self.clone();
        probe.entry_id = Hash256::zero();
        self.entry_id = content_address(&probe)?;
        Ok(self)
    }
}

/// `EvaluationRunLedger` — append-only, hash-chained.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationRunLedger {
    /// Content-addressed ledger id (re-computed when entries change).
    pub ledger_id: Hash256,
    /// Anchoring evaluation spec.
    pub evaluation_spec_hash: Hash256,
    /// Append-only entries.
    pub entries: Vec<EvaluationRunEntry>,
    /// Rolling chain hash: `chain[0] = H(spec)`, `chain[i+1] =
    /// H(chain[i], entries[i].entry_id)`.
    pub chain_hash: Hash256,
}

/// Initialise an empty ledger anchored on `spec_hash`. The chain hash
/// is the content address of `("eval-matrix-ledger-v0.1", spec_hash)`.
pub fn init_ledger(spec_hash: Hash256) -> Result<EvaluationRunLedger, EvalError> {
    let chain_hash = content_address(&("eval-matrix-ledger-v0.1", &spec_hash))?;
    let mut l = EvaluationRunLedger {
        ledger_id: Hash256::zero(),
        evaluation_spec_hash: spec_hash,
        entries: Vec::new(),
        chain_hash,
    };
    l.ledger_id = compute_ledger_id(&l)?;
    Ok(l)
}

/// Append an entry to the ledger. Recomputes `chain_hash` and `ledger_id`.
/// Refuses to append if the entry's `entry_id` is `zero` (caller must
/// have called `with_id` first).
pub fn append_to_ledger(
    mut ledger: EvaluationRunLedger,
    entry: EvaluationRunEntry,
) -> Result<EvaluationRunLedger, EvalError> {
    if entry.entry_id == Hash256::zero() {
        return Err(EvalError::LedgerChainBroken {
            reason: "attempted to append entry with zero entry_id (call with_id first)".into(),
        });
    }
    // Verify the entry's stored id matches its canonical content.
    let recomputed = entry.clone().with_id()?;
    if recomputed.entry_id != entry.entry_id {
        return Err(EvalError::LedgerChainBroken {
            reason: format!(
                "entry id mismatch: stored {} vs recomputed {}",
                entry.entry_id.hex(),
                recomputed.entry_id.hex()
            ),
        });
    }
    ledger.chain_hash = content_address(&(&ledger.chain_hash, &entry.entry_id))?;
    ledger.entries.push(entry);
    ledger.ledger_id = compute_ledger_id(&ledger)?;
    Ok(ledger)
}

/// Verify that the ledger's `chain_hash` matches the canonical re-fold
/// over `entries`. Used by `replay` and `score`.
pub fn verify_ledger_chain(ledger: &EvaluationRunLedger) -> Result<(), EvalError> {
    let mut chain = content_address(&("eval-matrix-ledger-v0.1", &ledger.evaluation_spec_hash))?;
    for entry in &ledger.entries {
        let recomputed = entry.clone().with_id()?;
        if recomputed.entry_id != entry.entry_id {
            return Err(EvalError::LedgerChainBroken {
                reason: format!("entry {} bytes diverged", entry.entry_id.hex()),
            });
        }
        chain = content_address(&(&chain, &entry.entry_id))?;
    }
    if chain != ledger.chain_hash {
        return Err(EvalError::LedgerChainBroken {
            reason: "rolling chain hash diverged".into(),
        });
    }
    Ok(())
}

fn compute_ledger_id(ledger: &EvaluationRunLedger) -> Result<Hash256, EvalError> {
    let mut probe = ledger.clone();
    probe.ledger_id = Hash256::zero();
    content_address(&probe)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(variant: &str) -> EvaluationRunEntry {
        EvaluationRunEntry {
            entry_id: Hash256::zero(),
            variant_id: variant.into(),
            workload_id: "w.0".into(),
            dataset_id: Hash256::zero(),
            run_descriptor_hash: Hash256::zero(),
            report_hash: Hash256::zero(),
            replay_hash: Hash256::zero(),
            status: RunStatus::Completed,
        }
        .with_id()
        .unwrap()
    }

    #[test]
    fn empty_ledger_chain_hashes_anchor() {
        let l = init_ledger(Hash256::zero()).unwrap();
        assert!(verify_ledger_chain(&l).is_ok());
    }

    #[test]
    fn append_extends_chain_deterministically() {
        let l1 = init_ledger(Hash256::zero()).unwrap();
        let l2 = append_to_ledger(l1.clone(), entry("B0")).unwrap();
        let l3 = append_to_ledger(l1, entry("B0")).unwrap();
        assert_eq!(l2.chain_hash, l3.chain_hash);
        assert_eq!(l2.ledger_id, l3.ledger_id);
        assert!(verify_ledger_chain(&l2).is_ok());
    }

    #[test]
    fn tampered_entry_breaks_chain_verification() {
        let l1 = init_ledger(Hash256::zero()).unwrap();
        let l2 = append_to_ledger(l1, entry("B0")).unwrap();
        let mut tampered = l2.clone();
        tampered.entries[0].variant_id = "B1".into();
        assert!(verify_ledger_chain(&tampered).is_err());
    }

    #[test]
    fn append_rejects_unhashed_entry() {
        let l = init_ledger(Hash256::zero()).unwrap();
        let raw = EvaluationRunEntry {
            entry_id: Hash256::zero(),
            variant_id: "B0".into(),
            workload_id: "w.0".into(),
            dataset_id: Hash256::zero(),
            run_descriptor_hash: Hash256::zero(),
            report_hash: Hash256::zero(),
            replay_hash: Hash256::zero(),
            status: RunStatus::Completed,
        };
        assert!(append_to_ledger(l, raw).is_err());
    }
}

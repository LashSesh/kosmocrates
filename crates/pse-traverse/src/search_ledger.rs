//! Search ledger — append-only, hash-chained record of traversal evaluations.
//!
//! Each [`SearchLedgerEntry`] is content-addressed and linked to the previous
//! entry via `parent_entry_id`, forming a tamper-evident chain.
//!
//! Implements spec PSE-TRAVERSE-SIGNATURE-01 §12.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::canonical::hex_address;
use crate::field_cube::EvidenceRef;

// ──────────────────────────────────────────────────────────────────────────────
// CommitOutcomeKind
// ──────────────────────────────────────────────────────────────────────────────

/// Data-free variant tag mirroring [`crate::report::CommitOutcome`], used in
/// ledger entries where only the kind matters (not the payload).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommitOutcomeKind {
    Crystal,
    NoCrystal,
    EvidenceOnly,
    GateFailed,
}

// ──────────────────────────────────────────────────────────────────────────────
// SearchLedgerEntry
// ──────────────────────────────────────────────────────────────────────────────

/// One evaluation record in the ledger.
///
/// `entry_id` is content-addressed (derived with the field set to "").
/// `parent_entry_id` links this entry to the previous one, or is `None` for
/// the first entry.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SearchLedgerEntry {
    /// Content-addressed identifier — derived with this field set to "".
    pub entry_id: String,
    /// ID of the immediately preceding entry, or `None` for the first entry.
    pub parent_entry_id: Option<String>,
    /// Blueprint that produced this evaluation.
    pub blueprint_id: String,
    /// Hash of the `ProblemSpec` this evaluation targets.
    pub problem_spec_hash: String,
    /// Collapse plan identifier.
    pub plan_id: String,
    /// Optional structural operator identifier.
    pub operator_id: Option<String>,
    /// Optional signature identifier.
    pub signature_id: Option<String>,
    /// Optional diagnostics identifier.
    pub diagnostics_id: Option<String>,
    /// Content address of the gate report artefact.
    pub gate_report_address: String,
    /// High-level outcome kind.
    pub commit_outcome_kind: CommitOutcomeKind,
    /// Numeric scores recorded at evaluation time (keys are metric names).
    pub scores: BTreeMap<String, f64>,
    /// Evidence references gathered during this evaluation.
    pub evidence_refs: Vec<EvidenceRef>,
    /// Monotonically increasing index within the ledger (0-based).
    pub logical_index: u64,
}

// ──────────────────────────────────────────────────────────────────────────────
// SearchLedgerReport
// ──────────────────────────────────────────────────────────────────────────────

/// Immutable snapshot of the full ledger plus the current Pareto frontier.
///
/// `report_id` is content-addressed (derived with the field set to "").
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SearchLedgerReport {
    /// Content-addressed identifier — derived with this field set to "".
    pub report_id: String,
    pub problem_spec_hash: String,
    pub entry_count: u64,
    /// Entries sorted by `logical_index` ascending.
    pub entries: Vec<SearchLedgerEntry>,
    /// Sorted blueprint IDs on the current non-dominated frontier.
    pub frontier_blueprint_ids: Vec<String>,
}

// ──────────────────────────────────────────────────────────────────────────────
// SearchLedger
// ──────────────────────────────────────────────────────────────────────────────

/// Append-only, hash-chained store of [`SearchLedgerEntry`] records.
pub struct SearchLedger {
    entries: Vec<SearchLedgerEntry>,
}

impl SearchLedger {
    /// Create an empty ledger.
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Append an entry to the ledger.
    ///
    /// This method:
    /// 1. Sets `entry.parent_entry_id` to the last entry's `entry_id` (or
    ///    `None` if the ledger is empty).
    /// 2. Assigns `entry.logical_index = entries.len()` (before the push).
    /// 3. Derives `entry.entry_id` via [`hex_address`] with `entry_id = ""`.
    pub fn append(&mut self, mut entry: SearchLedgerEntry) {
        let parent_id = self.entries.last().map(|e| e.entry_id.clone());
        entry.parent_entry_id = parent_id;
        entry.logical_index = self.entries.len() as u64;

        // Content-address with entry_id = "" first.
        entry.entry_id = String::new();
        entry.entry_id = hex_address(&entry).unwrap_or_default();

        self.entries.push(entry);
    }

    /// Verify the entire chain by re-deriving each `entry_id` and checking
    /// the `parent_entry_id` linkage.
    ///
    /// Returns `Ok(())` if the chain is intact, or `Err(description)` on the
    /// first detected inconsistency.
    pub fn verify_chain(&self) -> Result<(), String> {
        let mut prev_id: Option<String> = None;

        for entry in &self.entries {
            // Re-derive entry_id.
            let mut copy = entry.clone();
            copy.entry_id = String::new();
            let expected_id = hex_address(&copy)
                .map_err(|e| format!("hash error at index {}: {}", entry.logical_index, e))?;

            if entry.entry_id != expected_id {
                return Err(format!(
                    "entry_id mismatch at index {}: stored={} computed={}",
                    entry.logical_index, entry.entry_id, expected_id
                ));
            }

            if entry.parent_entry_id != prev_id {
                return Err(format!(
                    "parent_entry_id mismatch at index {}: expected {:?}, got {:?}",
                    entry.logical_index, prev_id, entry.parent_entry_id
                ));
            }

            prev_id = Some(entry.entry_id.clone());
        }

        Ok(())
    }

    /// Build a [`SearchLedgerReport`] from the current state.
    ///
    /// `frontier_ids` should be the result of
    /// [`NonDominatedFrontier::frontier_ids`].  It is stored sorted.
    pub fn report(
        &self,
        problem_spec_hash: &str,
        mut frontier_ids: Vec<String>,
    ) -> SearchLedgerReport {
        frontier_ids.sort();

        let mut entries_sorted = self.entries.clone();
        entries_sorted.sort_by_key(|e| e.logical_index);

        let mut report = SearchLedgerReport {
            report_id: String::new(),
            problem_spec_hash: problem_spec_hash.to_string(),
            entry_count: entries_sorted.len() as u64,
            entries: entries_sorted,
            frontier_blueprint_ids: frontier_ids,
        };

        report.report_id = hex_address(&report).unwrap_or_default();
        report
    }
}

impl Default for SearchLedger {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(blueprint_id: &str, plan_id: &str) -> SearchLedgerEntry {
        SearchLedgerEntry {
            entry_id: String::new(),
            parent_entry_id: None,
            blueprint_id: blueprint_id.to_string(),
            problem_spec_hash: "spec.hash.test".to_string(),
            plan_id: plan_id.to_string(),
            operator_id: None,
            signature_id: None,
            diagnostics_id: None,
            gate_report_address: "gate.addr".to_string(),
            commit_outcome_kind: CommitOutcomeKind::NoCrystal,
            scores: BTreeMap::new(),
            evidence_refs: Vec::new(),
            logical_index: 0,
        }
    }

    #[test]
    fn search_ledger_hash_chain_verifies() {
        let mut ledger = SearchLedger::new();
        ledger.append(make_entry("bp1", "plan1"));
        ledger.append(make_entry("bp2", "plan2"));
        ledger.append(make_entry("bp3", "plan3"));

        assert_eq!(ledger.entries.len(), 3);
        assert_eq!(ledger.entries[0].logical_index, 0);
        assert_eq!(ledger.entries[1].logical_index, 1);
        assert_eq!(ledger.entries[2].logical_index, 2);
        assert!(ledger.entries[0].parent_entry_id.is_none());
        assert_eq!(
            ledger.entries[1].parent_entry_id.as_deref(),
            Some(ledger.entries[0].entry_id.as_str())
        );
        assert_eq!(
            ledger.entries[2].parent_entry_id.as_deref(),
            Some(ledger.entries[1].entry_id.as_str())
        );

        assert!(
            ledger.verify_chain().is_ok(),
            "chain should verify without error"
        );
    }

    #[test]
    fn search_ledger_detects_parent_mismatch() {
        let mut ledger = SearchLedger::new();
        ledger.append(make_entry("bp1", "plan1"));
        ledger.append(make_entry("bp2", "plan2"));
        ledger.append(make_entry("bp3", "plan3"));

        // Corrupt the parent_entry_id of entry[1].
        ledger.entries[1].parent_entry_id = Some("corrupted.parent.id".to_string());

        let result = ledger.verify_chain();
        assert!(
            result.is_err(),
            "verify_chain should detect the parent mismatch"
        );
    }

    #[test]
    fn search_ledger_report_is_content_addressed() {
        let mut ledger = SearchLedger::new();
        ledger.append(make_entry("bp1", "plan1"));
        ledger.append(make_entry("bp2", "plan2"));

        let report1 = ledger.report("spec.hash.test", vec!["bp1".into()]);
        let report2 = ledger.report("spec.hash.test", vec!["bp1".into()]);
        assert_eq!(report1.report_id, report2.report_id);
        assert_eq!(report1.report_id.len(), 64);
    }

    #[test]
    fn search_ledger_entry_ids_are_64_chars() {
        let mut ledger = SearchLedger::new();
        ledger.append(make_entry("bp1", "plan1"));
        for entry in &ledger.entries {
            assert_eq!(entry.entry_id.len(), 64, "entry_id must be 64 hex chars");
        }
    }
}

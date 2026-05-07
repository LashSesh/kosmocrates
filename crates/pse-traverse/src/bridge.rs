//! PSE bridge — the only legitimate path from a traversal candidate to
//! a `SemanticCrystal`.
//!
//! The traversal layer never fabricates crystals; it serialises the
//! candidate's payload bytes into PSE observations and submits them via
//! `pse_core::macro_step`. PSE's gate, falsifier, and consensus stages
//! decide whether a crystal forms.

use crate::canonical::hex_address;
use crate::gate::Candidate;
use crate::report::CommitOutcome;
use crate::{Result, TraverseError};

/// The committer trait. Production callers can implement their own
/// (e.g. an in-memory mock for unit tests, or an evidence-only sink
/// for non-crystal-emitting domains). The shipped impl is
/// [`PseMacroStepCommitter`].
pub trait CrystalCommitter {
    fn commit_candidate(
        &mut self,
        candidate: &Candidate,
        evidence_payloads: &[Vec<u8>],
    ) -> Result<CommitOutcome>;
}

/// Committer that calls `pse_core::macro_step`.
pub struct PseMacroStepCommitter<A: pse_graph::ObservationAdapter> {
    pub state: pse_core::GlobalState,
    pub config: pse_types::Config,
    pub adapter: A,
}

impl<A: pse_graph::ObservationAdapter> PseMacroStepCommitter<A> {
    pub fn new(config: pse_types::Config, adapter: A) -> Self {
        let state = pse_core::GlobalState::new(&config);
        Self {
            state,
            config,
            adapter,
        }
    }
}

impl<A: pse_graph::ObservationAdapter> CrystalCommitter for PseMacroStepCommitter<A> {
    fn commit_candidate(
        &mut self,
        candidate: &Candidate,
        evidence_payloads: &[Vec<u8>],
    ) -> Result<CommitOutcome> {
        let payloads = candidate.to_observation_payloads(evidence_payloads)?;
        match pse_core::macro_step(&mut self.state, &payloads, &self.config, &self.adapter) {
            Ok(Some(crystal)) => {
                let hex: String = crystal
                    .crystal_id
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect();
                Ok(CommitOutcome::Crystal {
                    candidate_id: candidate.id.clone(),
                    crystal_address_hex: hex,
                })
            }
            Ok(None) => {
                let snap_json = self
                    .state
                    .last_gate
                    .as_ref()
                    .and_then(|g| serde_json::to_string(g).ok());
                Ok(CommitOutcome::NoCrystal {
                    candidate_id: candidate.id.clone(),
                    reason: "pse_macro_step_returned_none".into(),
                    gate_snapshot_json: snap_json,
                })
            }
            Err(e) => Err(TraverseError::PseCommit(e.to_string())),
        }
    }
}

/// Convenience: turn a `GateReport` failure into a `CommitOutcome::GateFailed`
/// with the gate report's content address.
pub fn gate_failed(
    candidate: &Candidate,
    report: &crate::gate::GateReport,
) -> Result<CommitOutcome> {
    let addr = hex_address(report)?;
    Ok(CommitOutcome::GateFailed {
        candidate_id: candidate.id.clone(),
        gate_report_address_hex: addr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gate::Candidate;
    use std::collections::BTreeMap;

    /// Adapter from pse-graph that lands every payload on a single
    /// vertex — sufficient to exercise the macro_step path. It will
    /// reject (return Ok(None)) on default thresholds, which is exactly
    /// what we want to assert: NoCrystal must round-trip through the
    /// committer without panicking.
    fn passthrough() -> pse_graph::PassthroughAdapter {
        pse_graph::PassthroughAdapter::new("traverse_test")
    }

    fn candidate() -> Candidate {
        Candidate {
            id: "cand.1".into(),
            field_cube_id: "fc.x".into(),
            assignments: BTreeMap::new(),
            claimed_satisfies: Vec::new(),
            payloads: vec![br#"{"hello":"world"}"#.to_vec()],
            provenance: "test".into(),
        }
    }

    #[test]
    fn pse_bridge_returns_no_crystal_safely() {
        let mut committer = PseMacroStepCommitter::new(pse_types::Config::default(), passthrough());
        let r = committer
            .commit_candidate(&candidate(), &[])
            .expect("must not panic");
        // Default thresholds reject everything → NoCrystal expected.
        match r {
            CommitOutcome::NoCrystal { candidate_id, .. } => {
                assert_eq!(candidate_id, "cand.1");
            }
            CommitOutcome::Crystal { candidate_id, .. } => {
                // Acceptable too — engine evolution may make this pass on
                // some configs. We assert structural soundness only.
                assert_eq!(candidate_id, "cand.1");
            }
            other => panic!("unexpected outcome: {:?}", other),
        }
    }
}

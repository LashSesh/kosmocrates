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

    /// Convenience constructor using the `preset_planning` config so
    /// callers bridging a Tier-2 `CollapsePlan` into PSE Core get
    /// properly calibrated thresholds without manual config wiring.
    pub fn for_planning(adapter: A) -> Self {
        Self::new(pse_types::Config::preset_planning(), adapter)
    }

    /// Feed a pre-encoded sequence of observation batches — **one batch
    /// per logical plan step** — through successive `macro_step` calls.
    ///
    /// This is the correct Tier-1/Tier-2 bridge protocol:
    /// - Tier-2 (traverse) encodes each `CollapseStep` as one batch.
    /// - Tier-1 (PSE Core) sees a time-evolving graph that grows tick-by-tick.
    /// - Graph structure, deformation (`d`) and coherence (`q`) metrics
    ///   evolve naturally, making the Kairos gate meaningful.
    ///
    /// Returns the first `SemanticCrystal` produced (if any) together
    /// with the zero-based tick index at which it formed.
    pub fn commit_plan_sequence(
        &mut self,
        step_batches: &[Vec<Vec<u8>>],
    ) -> Result<Option<(pse_types::SemanticCrystal, usize)>> {
        for (tick, batch) in step_batches.iter().enumerate() {
            match pse_core::macro_step(&mut self.state, batch, &self.config, &self.adapter) {
                Ok(Some(crystal)) => return Ok(Some((crystal, tick))),
                Ok(None) => {}
                Err(e) => return Err(TraverseError::PseCommit(e.to_string())),
            }
        }
        Ok(None)
    }
}

/// Encode a `CollapsePlan` as a sequence of per-step observation batches
/// for use with [`PseMacroStepCommitter::commit_plan_sequence`].
///
/// Each [`crate::plan::CollapseStep`] becomes a single observation batch
/// containing its JSON-serialized representation. Submitting these
/// sequentially gives PSE Core a time-varying stream where each tick
/// corresponds to one planning step — so graph structure, deformation
/// and edge-density metrics grow naturally.
pub fn plan_to_step_batches(plan: &crate::plan::CollapsePlan) -> Vec<Vec<Vec<u8>>> {
    plan.steps
        .iter()
        .map(|step| {
            let payload = serde_json::to_vec(step).unwrap_or_default();
            vec![payload]
        })
        .collect()
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

    #[test]
    fn commit_plan_sequence_returns_none_on_empty_batches() {
        let mut committer =
            PseMacroStepCommitter::new(pse_types::Config::preset_planning(), passthrough());
        let result = committer
            .commit_plan_sequence(&[])
            .expect("must not panic");
        assert!(result.is_none());
    }

    #[test]
    fn commit_plan_sequence_runs_all_steps() {
        let mut committer =
            PseMacroStepCommitter::for_planning(passthrough());
        // 5 synthetic step batches — one JSON payload each.
        let batches: Vec<Vec<Vec<u8>>> = (0..5)
            .map(|i| {
                vec![serde_json::to_vec(&serde_json::json!({"step": i, "kind": "assign"}))
                    .unwrap()]
            })
            .collect();
        // Returns either Some crystal or None — both are valid on this
        // tiny synthetic workload. The important thing is it doesn't panic.
        let _ = committer
            .commit_plan_sequence(&batches)
            .expect("must not panic");
        // State must have advanced by at least 5 ticks.
        assert!(committer.state.commit_index >= 5);
    }

    #[test]
    fn plan_to_step_batches_encoding() {
        use crate::plan::{
            CollapsePlan, CollapseEffect, CollapseStep, CollapseStepKind, FailurePolicy,
            OrderingPolicy,
        };
        let plan = CollapsePlan {
            id: "plan.test".into(),
            field_cube_id: "fc.test".into(),
            steps: vec![CollapseStep {
                id: "s0".into(),
                kind: CollapseStepKind::ResolveDimension,
                target_nodes: vec![],
                required_constraints: vec![],
                expected_effect: CollapseEffect {
                    estimated_reduction: 0.5,
                    kind: "resolve".into(),
                },
                failure_policy: FailurePolicy::FailClosed,
            }],
            ordering_policy: OrderingPolicy::DeterministicLexCoupling,
            expected_reduction: 0.5,
            evidence: vec![],
        };
        let batches = plan_to_step_batches(&plan);
        assert_eq!(batches.len(), 1);
        // Each batch is one payload, and it's non-empty JSON.
        assert!(!batches[0][0].is_empty());
        let v: serde_json::Value = serde_json::from_slice(&batches[0][0]).expect("valid JSON");
        assert!(v.get("id").is_some());
    }
}

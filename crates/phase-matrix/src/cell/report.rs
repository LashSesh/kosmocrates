//! Cycle-report summary helpers and `StitcherReport`
//! (PHASEMATRIX-HIVEMIND-03.1 §10).

use serde::{Deserialize, Serialize};

use super::pipeline::CellSubstrateOutcome;
use super::primitives::{content_address, EvidenceRef, Fixed, Hash256, PhaseError};

/// Compact summary of a single cell-substrate cycle.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CycleReportSummary {
    /// Run descriptor hash.
    pub rd_hash: Hash256,
    /// Cycle outcome kind tag (`"completed"` / `"hold"` / …).
    pub outcome_kind: String,
    /// Resulting cluster id (if any).
    pub cluster_id: Option<Hash256>,
    /// Cluster coherence score (if available).
    pub cluster_coherence: Option<Fixed>,
    /// Cluster conflict score (if available).
    pub cluster_conflict: Option<Fixed>,
    /// Whether the cycle produced an intent candidate.
    pub produced_intent: bool,
    /// Whether a dissolution report was attached.
    pub produced_dissolution: bool,
}

impl CycleReportSummary {
    /// Build a summary from any [`CellSubstrateOutcome`].
    pub fn from_outcome(outcome: &CellSubstrateOutcome) -> Self {
        match outcome {
            CellSubstrateOutcome::Completed(r) => CycleReportSummary {
                rd_hash: r.rd_hash.clone(),
                outcome_kind: "completed".into(),
                cluster_id: Some(r.cluster.cluster_id.clone()),
                cluster_coherence: Some(r.cluster.coherence_score.clone()),
                cluster_conflict: Some(r.cluster.conflict_score.clone()),
                produced_intent: r.intent.is_some(),
                produced_dissolution: r.dissolution.is_some(),
            },
            CellSubstrateOutcome::Hold(h) => CycleReportSummary {
                rd_hash: h.rd_hash.clone(),
                outcome_kind: "hold".into(),
                cluster_id: None,
                cluster_coherence: None,
                cluster_conflict: None,
                produced_intent: false,
                produced_dissolution: false,
            },
            CellSubstrateOutcome::Rejected(r) => CycleReportSummary {
                rd_hash: r.rd_hash.clone(),
                outcome_kind: "rejected".into(),
                cluster_id: None,
                cluster_coherence: None,
                cluster_conflict: None,
                produced_intent: false,
                produced_dissolution: false,
            },
            CellSubstrateOutcome::Compacted(d) => CycleReportSummary {
                rd_hash: Hash256::zero(),
                outcome_kind: "compacted".into(),
                cluster_id: Some(d.cluster_id.clone()),
                cluster_coherence: None,
                cluster_conflict: None,
                produced_intent: false,
                produced_dissolution: true,
            },
            CellSubstrateOutcome::MatrixBoundaryViolation(b) => CycleReportSummary {
                rd_hash: Hash256::zero(),
                outcome_kind: "matrix_boundary_violation".into(),
                cluster_id: Some(b.subject_id.clone()),
                cluster_coherence: None,
                cluster_conflict: None,
                produced_intent: false,
                produced_dissolution: false,
            },
            CellSubstrateOutcome::DeterminismViolation(d) => CycleReportSummary {
                rd_hash: d.rd_hash.clone(),
                outcome_kind: "determinism_violation".into(),
                cluster_id: None,
                cluster_coherence: None,
                cluster_conflict: None,
                produced_intent: false,
                produced_dissolution: false,
            },
        }
    }
}

/// Top-level report produced by one stitch cycle (§10).
///
/// `StitcherReport` is content-addressed and captures the full before/after
/// tensor state, accepted updates, rejected candidates, gate reports and
/// the `FieldTensorTrace`.  It is the replay anchor for the Stitch layer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StitcherReport {
    /// Content address of this report.
    pub report_id: Hash256,
    /// Content address of the `StitchRunDescriptor`.
    pub rd_hash: Hash256,
    /// Content address of `ResonanceFabricState` (Fabric-H snapshot).
    pub fabric_h_hash: Hash256,
    /// Content address of `FieldTensorState` before this cycle.
    pub tensor_before_hash: Hash256,
    /// Content address of `FieldTensorState` after this cycle.
    pub tensor_after_hash: Hash256,
    /// Content address of the sorted set of `StitchCandidate`s.
    pub stitch_candidates_hash: Hash256,
    /// Content addresses of accepted `CouplingUpdate`s (sorted).
    pub accepted_updates: Vec<Hash256>,
    /// Content addresses of rejected `StitchCandidate`s (sorted).
    pub rejected_candidates: Vec<Hash256>,
    /// Content addresses of `StitcherGateReport`s (sorted).
    pub gate_reports: Vec<Hash256>,
    /// Content address of the `TensorDeltaReport`.
    pub tensor_delta_report_hash: Hash256,
    /// Content address of the `FieldTensorTrace`.
    pub field_tensor_trace_hash: Hash256,
    /// True iff at least one update was accepted.
    pub passed: bool,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl StitcherReport {
    /// Recompute `report_id`.
    ///
    /// Sorts `accepted_updates`, `rejected_candidates` and `gate_reports`
    /// before hashing (Invariant 9).
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        self.accepted_updates.sort();
        self.rejected_candidates.sort();
        self.gate_reports.sort();
        let mut probe = self.clone();
        probe.report_id = Hash256::zero();
        self.report_id = content_address(&probe)?;
        Ok(self)
    }
}

//! Cycle-report summary helpers used by the CLI report subcommand.

use serde::{Deserialize, Serialize};

use super::pipeline::CellSubstrateOutcome;
use super::primitives::{Fixed, Hash256};

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

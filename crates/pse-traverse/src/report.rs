//! Top-level report types — what a CLI run emits.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::dof::DoFGraph;
use crate::excision::PathExcision;
use crate::field_cube::{EvidenceRef, FieldCube};
use crate::gate::GateReport;
use crate::plan::{CollapsePlan, OrderingPolicy};
use crate::spec::ProblemSpec;

/// Replay-faithful descriptor of a traversal run.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TraversalRunDescriptor {
    pub run_id: String,
    pub problem_spec_hash: String,
    pub operator_versions: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    pub ordering_policy: OrderingPolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pse_config_hash: Option<String>,
    pub started_at_logical: u64,
}

/// Aggregated per-run report. Carries everything needed for replay
/// verification: the descriptor, the field cube, the DoF graph, the
/// plan, the excisions, the gate reports, and the commit outcomes.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TraversalRunReport {
    pub descriptor: TraversalRunDescriptor,
    /// The original `ProblemSpec` is stored in the report so replay is
    /// self-contained: a verifier only needs the report file to
    /// reconstruct cube, graph, plan, and excisions deterministically.
    pub problem_spec: ProblemSpec,
    pub field_cube: FieldCube,
    pub dof_graph: DoFGraph,
    pub collapse_plan: CollapsePlan,
    pub path_excisions: Vec<PathExcision>,
    pub gate_reports: Vec<GateReport>,
    pub commit_outcomes: Vec<CommitOutcome>,
    /// Optional signature-layer artefact references. Absent when the
    /// signature extension was not activated for this run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature_extension: Option<TraversalRunReportSignatureExtension>,
    /// Optional dynamics run report. Absent unless the `dynamics run` subcommand was used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dynamic_run: Option<crate::dynamic_report::DynamicRunReport>,
}

/// Result of attempting to commit a candidate.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum CommitOutcome {
    /// PSE returned a `SemanticCrystal`. Hex-encoded address is the only
    /// piece we keep in the report (the full crystal is in the PSE archive).
    Crystal {
        candidate_id: String,
        crystal_address_hex: String,
    },
    /// PSE rejected — gate failed inside `macro_step`. The snapshot is
    /// recorded for diagnostics.
    NoCrystal {
        candidate_id: String,
        reason: String,
        /// JSON string of the GateSnapshot from `state.last_gate`.
        gate_snapshot_json: Option<String>,
    },
    /// Candidate did not warrant a PSE commit (e.g. solver said
    /// "evidence only"); a content-addressed evidence reference is
    /// kept instead.
    EvidenceOnly {
        candidate_id: String,
        evidence: Vec<EvidenceRef>,
    },
    /// Gate failed before PSE was invoked.
    GateFailed {
        candidate_id: String,
        gate_report_address_hex: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CarrierMigrationReport {
    pub source_carrier: String,
    pub target_carrier: String,
    pub trigger: MigrationTrigger,
    pub load_delta: f64,
    pub pre_coherence: f64,
    pub post_coherence_estimate: f64,
    pub accepted: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum MigrationTrigger {
    Saturation,
    Friction,
    Shock,
    Drift,
    Manual,
}

/// Optional signature-layer extension for `TraversalRunReport`.
/// All fields are content-addressed IDs; the full artefacts are in
/// the pse-traverse artefact store or the SearchLedger.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct TraversalRunReportSignatureExtension {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blueprint_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostics_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature_gate_outcome_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search_ledger_entry_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RefinementRequest {
    pub candidate_id: String,
    pub reason: String,
    pub suggestions: Vec<String>,
}

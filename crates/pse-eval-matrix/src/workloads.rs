//! `WorkloadSpec` + the mandatory workload families (§4).

use serde::{Deserialize, Serialize};

use crate::primitives::{content_address, EvalError, Hash256};

/// Mandatory workload families per §4.1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum WorkloadFamily {
    /// Detect structural events in observation streams.
    StreamEvent,
    /// Detect regime shifts / drift / state breaks.
    AnomalyRegime,
    /// Collapse structured problem spaces.
    TraversalPuzzle,
    /// Coding-agent tasks with tests + repo context.
    CodeAgentPatch,
    /// Document → spec extraction.
    DocSynthesis,
    /// Find prior solution paths in memory.
    MemoryReuse,
    /// Finalise candidates correctly or hold them.
    HorizonFinalization,
    /// Map visible / latent / blocked paths.
    CognitionPanorama,
    /// Multi-agent coordination with wormholes + budgets.
    MultiAgent,
    /// PHASEMATRIX-HIVEMIND-03 morphodynamic resonance cell substrate
    /// — cluster formation rate, morphology gate compliance,
    /// dissolution trace preservation, intent generation.
    MorphoCellSubstrate,
    /// PHASEMATRIX-HIVEMIND-03.1 Dual-Fabric Field-Tensor Stitch Layer
    /// — StitcherGate compliance, coupling-update trace preservation,
    /// tensor-revision monotonicity, replay byte-identity.
    DualFabricStitch,
    /// PSE-LPCM-IMPLEMENTATION-01 Fragmented 51% Condensation Layer
    /// — local majority gate, seam compatibility, percolation path
    /// selection, coarse-grain condensation, replay byte-identity.
    LpcmFragmentCollapse,
}

/// Canonical list of mandatory families. The matrix MUST cover several
/// families per §4.1 — a single demo is not sufficient. The eval matrix
/// extends to ten families to cover the PHASEMATRIX-HIVEMIND-03
/// substrate alongside the original nine.
pub const MANDATORY_WORKLOAD_FAMILIES: &[WorkloadFamily] = &[
    WorkloadFamily::StreamEvent,
    WorkloadFamily::AnomalyRegime,
    WorkloadFamily::TraversalPuzzle,
    WorkloadFamily::CodeAgentPatch,
    WorkloadFamily::DocSynthesis,
    WorkloadFamily::MemoryReuse,
    WorkloadFamily::HorizonFinalization,
    WorkloadFamily::CognitionPanorama,
    WorkloadFamily::MultiAgent,
    WorkloadFamily::MorphoCellSubstrate,
    WorkloadFamily::DualFabricStitch,
    WorkloadFamily::LpcmFragmentCollapse,
];

/// Per-task budget (limits how many iterations / how much wallclock-equivalent
/// each variant may spend on a workload).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TaskBudget {
    /// Maximum iterations per task.
    pub max_iterations: u32,
    /// Maximum logical step count per trial (no wall-clock).
    pub max_logical_steps: u64,
    /// Maximum input tokens / observations per trial.
    pub max_input_units: u64,
}

impl TaskBudget {
    /// Permissive default — used by golden fixtures.
    pub fn permissive() -> Self {
        TaskBudget {
            max_iterations: 4,
            max_logical_steps: 1024,
            max_input_units: 1024,
        }
    }
}

/// Single declarative success criterion. Multiple criteria are
/// AND-combined.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SuccessCriterion {
    /// All declared tests must pass (`tests_passed_ratio == 1.0`).
    AllTestsPass,
    /// Detected events must match ground truth events.
    DetectionMatchesGroundTruth,
    /// Replay byte-identity must hold.
    ReplayByteIdentical,
    /// No `false_commit` events permitted.
    NoFalseCommit,
    /// Hold/Wait/Abort decisions must be correct.
    HoldsAreCorrect,
}

/// `WorkloadSpec` (§10.3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkloadSpec {
    /// Stable workload id (e.g. `"w.stream_minimal"`).
    pub workload_id: String,
    /// Workload family.
    pub family: WorkloadFamily,
    /// Hash of the input dataset manifest.
    pub input_manifest_hash: Hash256,
    /// Optional ground-truth profile id (mandatory if `success_criteria`
    /// is non-empty).
    pub ground_truth_profile: Option<Hash256>,
    /// Per-task budget.
    pub task_budget: TaskBudget,
    /// Success criteria.
    pub success_criteria: Vec<SuccessCriterion>,
}

impl WorkloadSpec {
    /// Build a stream-event workload with a default budget and
    /// detection-matching + replay criteria.
    pub fn stream_event(
        id: impl Into<String>,
        dataset_id: Hash256,
        ground_truth_profile: Option<Hash256>,
    ) -> Self {
        WorkloadSpec {
            workload_id: id.into(),
            family: WorkloadFamily::StreamEvent,
            input_manifest_hash: dataset_id,
            ground_truth_profile,
            task_budget: TaskBudget::permissive(),
            success_criteria: vec![
                SuccessCriterion::DetectionMatchesGroundTruth,
                SuccessCriterion::NoFalseCommit,
                SuccessCriterion::ReplayByteIdentical,
            ],
        }
    }

    /// Build a code-agent workload.
    pub fn code_agent_patch(
        id: impl Into<String>,
        dataset_id: Hash256,
        ground_truth_profile: Option<Hash256>,
    ) -> Self {
        WorkloadSpec {
            workload_id: id.into(),
            family: WorkloadFamily::CodeAgentPatch,
            input_manifest_hash: dataset_id,
            ground_truth_profile,
            task_budget: TaskBudget::permissive(),
            success_criteria: vec![
                SuccessCriterion::AllTestsPass,
                SuccessCriterion::ReplayByteIdentical,
            ],
        }
    }

    /// Build a horizon-finalisation workload.
    pub fn horizon_finalization(
        id: impl Into<String>,
        dataset_id: Hash256,
        ground_truth_profile: Option<Hash256>,
    ) -> Self {
        WorkloadSpec {
            workload_id: id.into(),
            family: WorkloadFamily::HorizonFinalization,
            input_manifest_hash: dataset_id,
            ground_truth_profile,
            task_budget: TaskBudget::permissive(),
            success_criteria: vec![
                SuccessCriterion::HoldsAreCorrect,
                SuccessCriterion::NoFalseCommit,
                SuccessCriterion::ReplayByteIdentical,
            ],
        }
    }

    /// Build a cognition-panorama workload.
    pub fn cognition_panorama(
        id: impl Into<String>,
        dataset_id: Hash256,
        ground_truth_profile: Option<Hash256>,
    ) -> Self {
        WorkloadSpec {
            workload_id: id.into(),
            family: WorkloadFamily::CognitionPanorama,
            input_manifest_hash: dataset_id,
            ground_truth_profile,
            task_budget: TaskBudget::permissive(),
            success_criteria: vec![
                SuccessCriterion::DetectionMatchesGroundTruth,
                SuccessCriterion::ReplayByteIdentical,
            ],
        }
    }

    /// Build a `MorphoCellSubstrate` workload (PHASEMATRIX-HIVEMIND-03
    /// `cluster-cycle`). Success criteria: holds correctness (the
    /// formation gate fires only when warranted), no false commit,
    /// replay byte-identity.
    pub fn morpho_cell_substrate(
        id: impl Into<String>,
        dataset_id: Hash256,
        ground_truth_profile: Option<Hash256>,
    ) -> Self {
        WorkloadSpec {
            workload_id: id.into(),
            family: WorkloadFamily::MorphoCellSubstrate,
            input_manifest_hash: dataset_id,
            ground_truth_profile,
            task_budget: TaskBudget::permissive(),
            success_criteria: vec![
                SuccessCriterion::HoldsAreCorrect,
                SuccessCriterion::NoFalseCommit,
                SuccessCriterion::ReplayByteIdentical,
            ],
        }
    }

    /// Build a `DualFabricStitch` workload (PHASEMATRIX-HIVEMIND-03.1).
    /// Success criteria: StitcherGate compliance (`HoldsAreCorrect`),
    /// no direct Fabric-H→Fabric-T mutation (`NoFalseCommit`), replay
    /// byte-identity.
    pub fn dual_fabric_stitch(
        id: impl Into<String>,
        dataset_id: Hash256,
        ground_truth_profile: Option<Hash256>,
    ) -> Self {
        WorkloadSpec {
            workload_id: id.into(),
            family: WorkloadFamily::DualFabricStitch,
            input_manifest_hash: dataset_id,
            ground_truth_profile,
            task_budget: TaskBudget::permissive(),
            success_criteria: vec![
                SuccessCriterion::HoldsAreCorrect,
                SuccessCriterion::NoFalseCommit,
                SuccessCriterion::ReplayByteIdentical,
            ],
        }
    }

    /// Build an `LpcmFragmentCollapse` workload
    /// (PSE-LPCM-IMPLEMENTATION-01). Success criteria: local gate must
    /// fire only when warranted (`HoldsAreCorrect`), no candidate may be
    /// treated as truth (`NoFalseCommit`), replay byte-identity.
    pub fn lpcm_fragment_collapse(
        id: impl Into<String>,
        dataset_id: Hash256,
        ground_truth_profile: Option<Hash256>,
    ) -> Self {
        WorkloadSpec {
            workload_id: id.into(),
            family: WorkloadFamily::LpcmFragmentCollapse,
            input_manifest_hash: dataset_id,
            ground_truth_profile,
            task_budget: TaskBudget::permissive(),
            success_criteria: vec![
                SuccessCriterion::HoldsAreCorrect,
                SuccessCriterion::NoFalseCommit,
                SuccessCriterion::ReplayByteIdentical,
            ],
        }
    }

    /// Stable content hash of the workload spec (not stored on the
    /// type itself — used by the runner to anchor `RunDescriptor`s).
    pub fn content_hash(&self) -> Result<Hash256, EvalError> {
        content_address(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mandatory_families_cover_spec_table() {
        // §4.1 lists nine families; extended to twelve with MorphoCellSubstrate,
        // DualFabricStitch, and LpcmFragmentCollapse.
        assert_eq!(MANDATORY_WORKLOAD_FAMILIES.len(), 12);
        assert!(MANDATORY_WORKLOAD_FAMILIES.contains(&WorkloadFamily::MorphoCellSubstrate));
        assert!(MANDATORY_WORKLOAD_FAMILIES.contains(&WorkloadFamily::DualFabricStitch));
        assert!(MANDATORY_WORKLOAD_FAMILIES.contains(&WorkloadFamily::LpcmFragmentCollapse));
    }

    #[test]
    fn stream_event_workload_has_replay_criterion() {
        let w = WorkloadSpec::stream_event("w.s", Hash256::zero(), None);
        assert!(w
            .success_criteria
            .contains(&SuccessCriterion::ReplayByteIdentical));
    }
}

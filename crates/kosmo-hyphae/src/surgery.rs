//! Metatron v0.4.1 M3 — planning-only topological surgery.
//!
//! Surgery options are derived from `MicroTopologyDiagnostic` (M2 output).
//! All outputs are planning artifacts only — no host mutations in Phase 7.
//! `SurgeryBackedCollapseStep` extends `CollapseStep` with surgery provenance.
//! `SurgeryWorkbenchTask` expresses a surgery option as a workbench task.

use crate::collapse::{CollapseStep};
use crate::metatron::MicroTopologyDiagnostic;
use kosmo_core::{Digest, PolicyProfile, Q16};
use serde::{Deserialize, Serialize};

// ─── Surgery Kind ─────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TopologicalSurgeryKind {
    InsertEdge,
    RemoveEdge,
    SplitNode,
    MergeNodes,
    ReanchorBoundary,
    FillComplementVoid { hypothesized_void_kind: String },
    Custom(String),
}

// ─── Surgery Precondition ─────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurgeryPrecondition {
    DiagnosticPresent { diagnostic_id: Digest },
    NoActiveCollapseForRegion { region_id: Digest },
    PolicyApproved,
}

// ─── Surgery Effect ───────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurgeryEffect {
    FillsVoid { void_kind: String },
    ReducesAmbiguity { profile_id: Digest },
    RecordsEvidence,
    ImprovesFingerprintQuality,
}

// ─── Surgery Risk ─────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurgeryRisk {
    TopologyRegression,
    SemanticDrift,
    EvidenceGap,
    InsufficientPreconditions,
    Custom(String),
}

// ─── Surgery Option ───────────────────────────────────────────────────────────

/// Serialize-only for TopologicalSurgeryOption content-addressing.
#[derive(Serialize)]
struct SurgeryOptionContent {
    diagnostic_id: Digest,
    surgery_kind: String,
    /// Unique id of the source element (ambiguity.profile_id or hypothesis.hypothesis_id).
    source_id: Digest,
    confidence_score: i64,
    policy_id: Digest,
}

/// A planning-only topological surgery option derived from a `MicroTopologyDiagnostic`.
///
/// Never executes host mutations. Status is always planning-only in Phase 7.
/// `source_id` disambiguates multiple options from the same diagnostic.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TopologicalSurgeryOption {
    pub option_id: Digest,
    pub diagnostic_id: Digest,
    pub source_id: Digest,
    pub surgery_kind: TopologicalSurgeryKind,
    pub preconditions: Vec<SurgeryPrecondition>,
    pub effects: Vec<SurgeryEffect>,
    pub risks: Vec<SurgeryRisk>,
    pub confidence_score: Q16,
    pub policy_id: Digest,
}

impl TopologicalSurgeryOption {
    pub fn new(
        diagnostic_id: Digest,
        source_id: Digest,
        surgery_kind: TopologicalSurgeryKind,
        preconditions: Vec<SurgeryPrecondition>,
        effects: Vec<SurgeryEffect>,
        risks: Vec<SurgeryRisk>,
        confidence_score: Q16,
        policy_id: Digest,
    ) -> Self {
        let option_id = Digest::of(&SurgeryOptionContent {
            diagnostic_id,
            surgery_kind: format!("{:?}", surgery_kind),
            source_id,
            confidence_score: confidence_score.raw(),
            policy_id,
        });
        Self {
            option_id,
            diagnostic_id,
            source_id,
            surgery_kind,
            preconditions,
            effects,
            risks,
            confidence_score,
            policy_id,
        }
    }

    /// Derive surgery options from a `MicroTopologyDiagnostic`.
    ///
    /// One `ReanchorBoundary` option is produced per `Boundary` ambiguity.
    /// One `FillComplementVoid` option is produced per void hypothesis.
    /// The result is sorted by `option_id` for deterministic replay.
    pub fn from_diagnostic(
        diagnostic: &MicroTopologyDiagnostic,
        policy: &PolicyProfile,
    ) -> Vec<Self> {
        let mut options = Vec::new();

        for ambiguity in &diagnostic.ambiguities {
            let preconditions = vec![
                SurgeryPrecondition::DiagnosticPresent {
                    diagnostic_id: diagnostic.diagnostic_id,
                },
                SurgeryPrecondition::PolicyApproved,
            ];
            let effects = vec![
                SurgeryEffect::ReducesAmbiguity { profile_id: ambiguity.profile_id },
                SurgeryEffect::ImprovesFingerprintQuality,
            ];
            let risks = vec![SurgeryRisk::TopologyRegression, SurgeryRisk::SemanticDrift];
            options.push(Self::new(
                diagnostic.diagnostic_id,
                ambiguity.profile_id,
                TopologicalSurgeryKind::ReanchorBoundary,
                preconditions,
                effects,
                risks,
                ambiguity.confidence_score,
                policy.id,
            ));
        }

        for hypothesis in &diagnostic.void_hypotheses {
            let preconditions = vec![
                SurgeryPrecondition::DiagnosticPresent {
                    diagnostic_id: diagnostic.diagnostic_id,
                },
                SurgeryPrecondition::PolicyApproved,
            ];
            let effects = vec![
                SurgeryEffect::FillsVoid {
                    void_kind: hypothesis.hypothesized_void_kind.clone(),
                },
                SurgeryEffect::RecordsEvidence,
            ];
            let risks = vec![SurgeryRisk::EvidenceGap, SurgeryRisk::InsufficientPreconditions];
            options.push(Self::new(
                diagnostic.diagnostic_id,
                hypothesis.hypothesis_id,
                TopologicalSurgeryKind::FillComplementVoid {
                    hypothesized_void_kind: hypothesis.hypothesized_void_kind.clone(),
                },
                preconditions,
                effects,
                risks,
                hypothesis.confidence_score,
                policy.id,
            ));
        }

        options.sort_by_key(|o| o.option_id);
        options
    }
}

// ─── Surgery-Backed Collapse Step ─────────────────────────────────────────────

/// Serialize-only for SurgeryBackedCollapseStep content-addressing.
#[derive(Serialize)]
struct SurgeryStepContent {
    collapse_step_id: Digest,
    surgery_option_id: Digest,
    diagnostic_id: Digest,
    policy_id: Digest,
}

/// A `CollapseStep` backed by a `TopologicalSurgeryOption`.
///
/// Wraps an existing `CollapseStep` with a surgery provenance backref.
/// Planning-only in Phase 7; no host mutation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SurgeryBackedCollapseStep {
    pub surgery_step_id: Digest,
    pub collapse_step: CollapseStep,
    pub surgery_option_id: Digest,
    pub diagnostic_id: Digest,
    pub policy_id: Digest,
}

impl SurgeryBackedCollapseStep {
    pub fn new(
        collapse_step: CollapseStep,
        surgery_option: &TopologicalSurgeryOption,
    ) -> Self {
        let surgery_step_id = Digest::of(&SurgeryStepContent {
            collapse_step_id: collapse_step.step_id,
            surgery_option_id: surgery_option.option_id,
            diagnostic_id: surgery_option.diagnostic_id,
            policy_id: surgery_option.policy_id,
        });
        Self {
            surgery_step_id,
            collapse_step,
            surgery_option_id: surgery_option.option_id,
            diagnostic_id: surgery_option.diagnostic_id,
            policy_id: surgery_option.policy_id,
        }
    }
}

// ─── Surgery Workbench Task ───────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurgeryTaskStatus {
    /// Phase 7 maximum — task declared but no execution authorized.
    PlanningOnly,
    /// Awaiting operator approval for execution (Phase 10+).
    AwaitingApproval,
}

/// Serialize-only for SurgeryWorkbenchTask content-addressing.
#[derive(Serialize)]
struct SurgeryTaskContent {
    surgery_option_id: Digest,
    diagnostic_id: Digest,
    surgery_kind: String,
    policy_id: Digest,
}

/// A workbench-compatible task derived from a `TopologicalSurgeryOption`.
///
/// Expresses the surgery as a discrete workbench task that can be handed to
/// a `FoundryRunner` once operator approval is granted (Phase 10+).
/// Status is always `PlanningOnly` in Phase 7.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SurgeryWorkbenchTask {
    pub task_id: Digest,
    pub surgery_option_id: Digest,
    pub diagnostic_id: Digest,
    pub surgery_kind_label: String,
    pub status: SurgeryTaskStatus,
    pub policy_id: Digest,
}

impl SurgeryWorkbenchTask {
    pub fn from_option(option: &TopologicalSurgeryOption) -> Self {
        let surgery_kind_label = format!("{:?}", option.surgery_kind);
        let task_id = Digest::of(&SurgeryTaskContent {
            surgery_option_id: option.option_id,
            diagnostic_id: option.diagnostic_id,
            surgery_kind: surgery_kind_label.clone(),
            policy_id: option.policy_id,
        });
        Self {
            task_id,
            surgery_option_id: option.option_id,
            diagnostic_id: option.diagnostic_id,
            surgery_kind_label,
            status: SurgeryTaskStatus::PlanningOnly,
            policy_id: option.policy_id,
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "SurgeryWorkbenchTask — kind: {} | status: {:?}",
            self.surgery_kind_label,
            self.status,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collapse::{CollapseAction, CollapseStep};
    use crate::metatron::{diagnose_micrograph, lift_region};
    use crate::void_map::HostVoidKind;
    use kosmo_core::{Digest, PolicyProfile, TaintLabel};

    fn pid() -> Digest {
        Digest::of_bytes(b"p")
    }
    fn void_id() -> Digest {
        Digest::of_bytes(b"void")
    }
    fn ev_id() -> Digest {
        Digest::of_bytes(b"ev")
    }
    fn policy() -> PolicyProfile {
        PolicyProfile::default_report_only()
    }
    fn node_ids(n: usize) -> Vec<Digest> {
        (0..n).map(|i| Digest::of_bytes(format!("node{}", i).as_bytes())).collect()
    }

    fn make_diagnostic_single_node() -> (
        crate::metatron::MetatronMicrograph,
        crate::metatron::MicroTopologyDiagnostic,
    ) {
        let p = policy();
        let (micrograph, fingerprint, _) =
            lift_region(void_id(), node_ids(1), ev_id(), TaintLabel::Synthetic, &p);
        let diag = diagnose_micrograph(&micrograph, &fingerprint, None, &p);
        (micrograph, diag)
    }

    fn make_diagnostic_with_hypothesis() -> (
        crate::metatron::MetatronMicrograph,
        crate::metatron::MicroTopologyDiagnostic,
    ) {
        let p = policy();
        let kind = HostVoidKind::MissingTestFiber { for_module: "lib.rs".into() };
        let (micrograph, fingerprint, _) =
            lift_region(void_id(), node_ids(2), ev_id(), TaintLabel::Synthetic, &p);
        let diag = diagnose_micrograph(&micrograph, &fingerprint, Some(&kind), &p);
        (micrograph, diag)
    }

    #[test]
    fn surgery_option_from_boundary_ambiguity_is_reanchor() {
        let (_, diag) = make_diagnostic_single_node();
        let p = policy();
        let options = TopologicalSurgeryOption::from_diagnostic(&diag, &p);
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].surgery_kind, TopologicalSurgeryKind::ReanchorBoundary);
    }

    #[test]
    fn surgery_option_from_hypothesis_is_fill_complement_void() {
        let (_, diag) = make_diagnostic_with_hypothesis();
        let p = policy();
        let options = TopologicalSurgeryOption::from_diagnostic(&diag, &p);
        assert_eq!(options.len(), 1);
        assert!(
            matches!(
                options[0].surgery_kind,
                TopologicalSurgeryKind::FillComplementVoid { .. }
            ),
            "expected FillComplementVoid, got {:?}",
            options[0].surgery_kind,
        );
    }

    #[test]
    fn surgery_option_is_content_addressed() {
        let (_, diag) = make_diagnostic_single_node();
        let p = policy();
        let opts1 = TopologicalSurgeryOption::from_diagnostic(&diag, &p);
        let opts2 = TopologicalSurgeryOption::from_diagnostic(&diag, &p);
        assert_eq!(opts1[0].option_id, opts2[0].option_id);
        assert_ne!(opts1[0].option_id, Digest::ZERO);
    }

    #[test]
    fn surgery_option_has_policy_approved_precondition() {
        let (_, diag) = make_diagnostic_single_node();
        let options = TopologicalSurgeryOption::from_diagnostic(&diag, &policy());
        let has_approved = options[0]
            .preconditions
            .iter()
            .any(|c| *c == SurgeryPrecondition::PolicyApproved);
        assert!(has_approved, "surgery option must require PolicyApproved");
    }

    #[test]
    fn surgery_option_has_diagnostic_present_precondition() {
        let (_, diag) = make_diagnostic_single_node();
        let options = TopologicalSurgeryOption::from_diagnostic(&diag, &policy());
        let has_diag_present = options[0].preconditions.iter().any(|c| {
            matches!(c, SurgeryPrecondition::DiagnosticPresent { diagnostic_id }
                if *diagnostic_id == diag.diagnostic_id)
        });
        assert!(has_diag_present, "surgery option must require DiagnosticPresent");
    }

    #[test]
    fn empty_diagnostic_produces_no_surgery_options() {
        let p = policy();
        let (micrograph, fingerprint, _) =
            lift_region(void_id(), node_ids(3), ev_id(), TaintLabel::Synthetic, &p);
        let diag = diagnose_micrograph(&micrograph, &fingerprint, None, &p);
        let options = TopologicalSurgeryOption::from_diagnostic(&diag, &p);
        assert!(options.is_empty(), "clean multi-node diagnostic must produce no surgery options");
    }

    #[test]
    fn surgery_backed_collapse_step_is_content_addressed() {
        let (_, diag) = make_diagnostic_single_node();
        let options = TopologicalSurgeryOption::from_diagnostic(&diag, &policy());
        let step = CollapseStep::new(
            0,
            CollapseAction::RecordNegativeEvidence { record_id: Digest::ZERO },
            vec![diag.diagnostic_id],
            pid(),
        );
        let s1 = SurgeryBackedCollapseStep::new(step.clone(), &options[0]);
        let s2 = SurgeryBackedCollapseStep::new(step, &options[0]);
        assert_eq!(s1.surgery_step_id, s2.surgery_step_id);
        assert_ne!(s1.surgery_step_id, Digest::ZERO);
    }

    #[test]
    fn surgery_backed_collapse_step_links_to_diagnostic() {
        let (_, diag) = make_diagnostic_single_node();
        let options = TopologicalSurgeryOption::from_diagnostic(&diag, &policy());
        let step = CollapseStep::new(
            0,
            CollapseAction::RecordNegativeEvidence { record_id: Digest::ZERO },
            vec![],
            pid(),
        );
        let backed = SurgeryBackedCollapseStep::new(step, &options[0]);
        assert_eq!(backed.diagnostic_id, diag.diagnostic_id);
        assert_eq!(backed.surgery_option_id, options[0].option_id);
    }

    #[test]
    fn surgery_workbench_task_is_planning_only() {
        let (_, diag) = make_diagnostic_single_node();
        let options = TopologicalSurgeryOption::from_diagnostic(&diag, &policy());
        let task = SurgeryWorkbenchTask::from_option(&options[0]);
        assert_eq!(task.status, SurgeryTaskStatus::PlanningOnly);
        assert_ne!(task.task_id, Digest::ZERO);
    }

    #[test]
    fn surgery_workbench_task_is_content_addressed() {
        let (_, diag) = make_diagnostic_single_node();
        let options = TopologicalSurgeryOption::from_diagnostic(&diag, &policy());
        let t1 = SurgeryWorkbenchTask::from_option(&options[0]);
        let t2 = SurgeryWorkbenchTask::from_option(&options[0]);
        assert_eq!(t1.task_id, t2.task_id);
    }

    #[test]
    fn surgery_workbench_task_summary_non_empty() {
        let (_, diag) = make_diagnostic_single_node();
        let options = TopologicalSurgeryOption::from_diagnostic(&diag, &policy());
        let task = SurgeryWorkbenchTask::from_option(&options[0]);
        let s = task.summary();
        assert!(s.contains("PlanningOnly"));
        assert!(s.contains("SurgeryWorkbenchTask"));
    }
}

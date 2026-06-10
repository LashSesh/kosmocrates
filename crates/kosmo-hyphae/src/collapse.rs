use crate::delta::{DeltaAction, HostTargetDelta};
use kosmo_core::Digest;
use serde::{Deserialize, Serialize};

/// The planned action for one step in a HostTargetCollapsePlan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CollapseAction {
    /// Fill a host void with a candidate source cube (planning only).
    FillVoid {
        void_id: Digest,
        candidate_source_cube_id: Digest,
    },
    /// Reduce a deficiency by a target amount.
    ReduceDeficiency {
        deficiency_kind: String,
        target_score_raw: i64,
    },
    /// Certify a crystal candidate.
    CertifyCrystal { candidate_id: Digest },
    /// Record a negative evidence entry.
    RecordNegativeEvidence { record_id: Digest },
}

/// Serialize-only for CollapseStep content-addressing.
#[derive(Serialize)]
struct StepContent {
    sequence: u64,
    action: String,
    precondition_count: u64,
    policy_id: Digest,
}

/// One step in a HostTargetCollapsePlan.
///
/// Steps are ordered by sequence number. Preconditions are sorted digests
/// of policies or evidence that must be present before this step executes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CollapseStep {
    pub step_id: Digest,
    pub sequence: u64,
    pub action: CollapseAction,
    pub precondition_ids: Vec<Digest>,
    pub policy_id: Digest,
}

impl CollapseStep {
    pub fn new(
        sequence: u64,
        action: CollapseAction,
        mut precondition_ids: Vec<Digest>,
        policy_id: Digest,
    ) -> Self {
        precondition_ids.sort();
        let step_id = Digest::of(&StepContent {
            sequence,
            action: format!("{:?}", action),
            precondition_count: precondition_ids.len() as u64,
            policy_id,
        });
        Self {
            step_id,
            sequence,
            action,
            precondition_ids,
            policy_id,
        }
    }
}

/// Planning status of a HostTargetCollapsePlan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CollapsePlanStatus {
    /// Phase 5 maximum — plan exists but no mutation is authorized.
    PlanningOnly,
    /// Awaiting explicit operator approval (Phase 10+).
    OperatorApprovalPending,
    /// Operator-approved; execution is permitted (Phase 11 only).
    Approved,
}

/// Serialize-only for HostTargetCollapsePlan content-addressing.
#[derive(Serialize)]
struct PlanContent {
    host_cube_id: Digest,
    delta_id: Digest,
    step_count: u64,
    policy_id: Digest,
}

/// A planning-only artifact describing how host voids could be collapsed.
///
/// Phase 5 status is always `PlanningOnly`. Execution requires Phase 11
/// with an `OperatorApproved` PolicyProfile and explicit user authorization.
/// No host files are written by this type.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HostTargetCollapsePlan {
    pub plan_id: Digest,
    pub host_cube_id: Digest,
    pub delta_id: Digest,
    pub steps: Vec<CollapseStep>,
    pub status: CollapsePlanStatus,
    pub policy_id: Digest,
}

impl HostTargetCollapsePlan {
    /// Build a planning-only collapse plan from a `HostTargetDelta`.
    ///
    /// One `FillVoid` step is created per void fill entry in the delta.
    /// Status is always `PlanningOnly` in Phase 5.
    pub fn from_delta(delta: &HostTargetDelta, policy_id: Digest) -> Self {
        let steps: Vec<CollapseStep> = delta
            .void_fills
            .iter()
            .enumerate()
            .map(|(seq, vf)| {
                let candidate = match &vf.action {
                    DeltaAction::FillVoid {
                        top_candidate_cube_id,
                    } => *top_candidate_cube_id,
                    _ => Digest::ZERO,
                };
                CollapseStep::new(
                    seq as u64,
                    CollapseAction::FillVoid {
                        void_id: vf.void_id,
                        candidate_source_cube_id: candidate,
                    },
                    vec![delta.delta_id],
                    policy_id,
                )
            })
            .collect();

        let plan_id = Digest::of(&PlanContent {
            host_cube_id: delta.host_cube_id,
            delta_id: delta.delta_id,
            step_count: steps.len() as u64,
            policy_id,
        });

        Self {
            plan_id,
            host_cube_id: delta.host_cube_id,
            delta_id: delta.delta_id,
            steps,
            status: CollapsePlanStatus::PlanningOnly,
            policy_id,
        }
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    pub fn summary(&self) -> String {
        format!(
            "HostTargetCollapsePlan (planning-only) — steps: {} | status: {:?}",
            self.step_count(),
            self.status,
        )
    }
}

/// Serialize-only for MorphogenicCorpusUpdate content-addressing.
#[derive(Serialize)]
struct MorphogenicContent {
    cartography_update_id: Digest,
    collapse_plan_id: Digest,
    policy_id: Digest,
}

/// Skeleton of a morphogenic corpus update — records what *would* happen
/// when a CollapsePlan is executed.
///
/// In Phase 5 this is a planning artifact only. Execution is Phase 11.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MorphogenicCorpusUpdate {
    pub update_id: Digest,
    pub cartography_update_id: Digest,
    pub collapse_plan_id: Digest,
    pub policy_id: Digest,
}

impl MorphogenicCorpusUpdate {
    pub fn skeleton(
        cartography_update_id: Digest,
        collapse_plan_id: Digest,
        policy_id: Digest,
    ) -> Self {
        let update_id = Digest::of(&MorphogenicContent {
            cartography_update_id,
            collapse_plan_id,
            policy_id,
        });
        Self {
            update_id,
            cartography_update_id,
            collapse_plan_id,
            policy_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::delta::{DeltaAction, HostTargetDelta, VoidFillDelta};
    use kosmo_core::{Digest, Q16};

    fn make_delta_with_fills(policy_id: Digest) -> HostTargetDelta {
        let void_id = Digest::of_bytes(b"v");
        let cube_id = Digest::of_bytes(b"c");
        let fills = vec![VoidFillDelta {
            void_id,
            candidate_cube_ids: vec![cube_id],
            best_support_score: Q16::HALF,
            action: DeltaAction::FillVoid {
                top_candidate_cube_id: cube_id,
            },
        }];
        HostTargetDelta::new(
            Digest::of_bytes(b"h"),
            Digest::of_bytes(b"comp"),
            fills,
            vec![],
            policy_id,
        )
    }

    #[test]
    fn collapse_plan_from_delta_is_planning_only() {
        let pid = Digest::of_bytes(b"p");
        let delta = make_delta_with_fills(pid);
        let plan = HostTargetCollapsePlan::from_delta(&delta, pid);
        assert_eq!(plan.status, CollapsePlanStatus::PlanningOnly);
        assert_eq!(plan.step_count(), 1);
        assert_ne!(plan.plan_id, Digest::ZERO);
    }

    #[test]
    fn collapse_plan_is_content_addressed() {
        let pid = Digest::of_bytes(b"p");
        let delta = make_delta_with_fills(pid);
        let p1 = HostTargetCollapsePlan::from_delta(&delta, pid);
        let p2 = HostTargetCollapsePlan::from_delta(&delta, pid);
        assert_eq!(p1.plan_id, p2.plan_id);
    }

    #[test]
    fn collapse_plan_empty_delta_has_no_steps() {
        let pid = Digest::of_bytes(b"p");
        let delta = HostTargetDelta::new(Digest::ZERO, Digest::ZERO, vec![], vec![], pid);
        let plan = HostTargetCollapsePlan::from_delta(&delta, pid);
        assert_eq!(plan.step_count(), 0);
        assert_eq!(plan.status, CollapsePlanStatus::PlanningOnly);
    }

    #[test]
    fn collapse_step_preconditions_sorted() {
        let pid = Digest::of_bytes(b"p");
        let a = Digest::of_bytes(b"aaa");
        let b = Digest::of_bytes(b"bbb");
        let step = CollapseStep::new(
            0,
            CollapseAction::RecordNegativeEvidence {
                record_id: Digest::ZERO,
            },
            vec![b, a], // reversed
            pid,
        );
        // sorted → first element should be a (smaller byte value "aaa" < "bbb")
        assert_eq!(
            step.precondition_ids[0].min(step.precondition_ids[1]),
            step.precondition_ids[0]
        );
    }

    #[test]
    fn morphogenic_update_skeleton_is_content_addressed() {
        let pid = Digest::of_bytes(b"p");
        let cu_id = Digest::of_bytes(b"cu");
        let cp_id = Digest::of_bytes(b"cp");
        let u1 = MorphogenicCorpusUpdate::skeleton(cu_id, cp_id, pid);
        let u2 = MorphogenicCorpusUpdate::skeleton(cu_id, cp_id, pid);
        assert_eq!(u1.update_id, u2.update_id);
        assert_ne!(u1.update_id, Digest::ZERO);
    }

    #[test]
    fn collapse_plan_summary_non_empty() {
        let pid = Digest::of_bytes(b"p");
        let delta = make_delta_with_fills(pid);
        let plan = HostTargetCollapsePlan::from_delta(&delta, pid);
        let s = plan.summary();
        assert!(s.contains("planning-only"));
        assert!(s.contains("PlanningOnly"));
    }
}

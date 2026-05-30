//! Phase 11 — Operator-Approved Materialization skeleton.
//!
//! Implements the governance infrastructure for operator-approved collapse plan
//! execution. Key invariants:
//!
//! - Without a valid `OperatorApprovalToken`, outcome is always `Blocked`.
//! - With token but wrong policy mode (non-OperatorApproved), outcome is `Blocked`.
//! - With valid token + `OperatorApproved` policy: outcome is `FoundryRequired` —
//!   Foundry validation must still run before any host write is attempted.
//! - `ParseBackExpectation` is declared for every topology-changing step.
//! - Actual host file writes occur only after Foundry passes; that execution
//!   is declared as a `WorkbenchMaterializationTask` but not triggered here.
//!
//! Hard limits in Phase 11:
//! - `allow_memory_promotion` remains false.
//! - `allow_synthetic_sourcecube` remains false.
//! - `require_foundry_for_executable_effects` remains true.
//! - `require_parseback_for_topology_changes` remains true.

use kosmo_core::{
    AuthorityLabel, Digest, FoundryCheckKind, FoundryCheckResult, FoundryOutcome,
    ImplementationMode, PolicyProfile,
};
use kosmo_hyphae::{CollapseStep, HostTargetCollapsePlan};
use kosmo_workbench::FoundryCheckSpec;
use serde::{Deserialize, Serialize};

// ─── Operator Approval Token ──────────────────────────────────────────────────

/// Content for deterministic `OperatorApprovalToken` ID.
#[derive(Serialize)]
struct TokenContent {
    collapse_plan_id: Digest,
    approved_by: String,
    policy_id: Digest,
}

/// Proof that a human or operator has reviewed and approved a specific
/// `HostTargetCollapsePlan` for execution.
///
/// Tokens are bound to a single collapse plan ID. They cannot authorize
/// a different plan. Without a valid token, `MaterializationOutcome` is `Blocked`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OperatorApprovalToken {
    pub token_id: Digest,
    pub collapse_plan_id: Digest,
    pub approved_by: AuthorityLabel,
    pub policy_id: Digest,
}

impl OperatorApprovalToken {
    pub fn new(
        collapse_plan_id: Digest,
        approved_by: AuthorityLabel,
        policy: &PolicyProfile,
    ) -> Self {
        let token_id = Digest::of(&TokenContent {
            collapse_plan_id,
            approved_by: format!("{:?}", approved_by),
            policy_id: policy.id,
        });
        Self {
            token_id,
            collapse_plan_id,
            approved_by,
            policy_id: policy.id,
        }
    }

    /// Whether this token covers the given collapse plan.
    pub fn covers(&self, plan: &HostTargetCollapsePlan) -> bool {
        self.collapse_plan_id == plan.plan_id
    }

    /// Whether the approver has sufficient authority (Human or Operator required).
    pub fn has_sufficient_authority(&self) -> bool {
        self.approved_by.is_human_or_operator()
    }
}

// ─── Parse-Back Expectation ───────────────────────────────────────────────────

/// Content for deterministic `ParseBackExpectation` ID.
#[derive(Serialize)]
struct ParseBackContent {
    step_id: Digest,
    before_topology_id: Digest,
    expected_after_topology_id: Digest,
    policy_id: Digest,
}

/// Topology expectation for parse-back validation of a collapse step.
///
/// Before any topology-changing step executes, the expected resulting topology
/// must be declared. After execution, Foundry compares actual vs expected.
/// This satisfies `require_parseback_for_topology_changes = true`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParseBackExpectation {
    pub expectation_id: Digest,
    pub step_id: Digest,
    /// Digest of the topology descriptor before this step.
    pub before_topology_id: Digest,
    /// Digest of the expected topology descriptor after this step.
    pub expected_after_topology_id: Digest,
    pub policy_id: Digest,
}

impl ParseBackExpectation {
    pub fn new(
        step: &CollapseStep,
        before_topology_id: Digest,
        expected_after_topology_id: Digest,
        policy: &PolicyProfile,
    ) -> Self {
        let expectation_id = Digest::of(&ParseBackContent {
            step_id: step.step_id,
            before_topology_id,
            expected_after_topology_id,
            policy_id: policy.id,
        });
        Self {
            expectation_id,
            step_id: step.step_id,
            before_topology_id,
            expected_after_topology_id,
            policy_id: policy.id,
        }
    }
}

// ─── Workbench Materialization Task ───────────────────────────────────────────

/// Content for deterministic `WorkbenchMaterializationTask` ID.
#[derive(Serialize)]
struct TaskContent {
    step_id: Digest,
    token_id: Digest,
    expectation_id: Digest,
    policy_id: Digest,
}

/// A single Workbench task derived from an approved `CollapseStep`.
///
/// Bundles the collapse step, approval token, Foundry check specs, and
/// parse-back expectation into a single Foundry-ready task descriptor.
///
/// Execution requires Foundry to pass all `foundry_checks`. The task is
/// a planning artifact — actual host writes happen only inside Foundry's
/// isolated environment after all checks pass.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkbenchMaterializationTask {
    pub task_id: Digest,
    pub step_id: Digest,
    pub token_id: Digest,
    pub policy_id: Digest,
    pub foundry_checks: Vec<FoundryCheckSpec>,
    pub parse_back: ParseBackExpectation,
}

impl WorkbenchMaterializationTask {
    pub fn new(
        step: &CollapseStep,
        token: &OperatorApprovalToken,
        parse_back: ParseBackExpectation,
        foundry_checks: Vec<FoundryCheckSpec>,
        policy: &PolicyProfile,
    ) -> Self {
        let task_id = Digest::of(&TaskContent {
            step_id: step.step_id,
            token_id: token.token_id,
            expectation_id: parse_back.expectation_id,
            policy_id: policy.id,
        });
        Self {
            task_id,
            step_id: step.step_id,
            token_id: token.token_id,
            policy_id: policy.id,
            foundry_checks,
            parse_back,
        }
    }
}

// ─── Materialization Outcome ──────────────────────────────────────────────────

/// The outcome of a materialization request evaluation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaterializationOutcome {
    /// Blocked: no valid token, wrong policy mode, or insufficient authority.
    Blocked { reason: String },
    /// Token and policy are valid; Foundry validation is still required before
    /// any host write can proceed.
    FoundryRequired { task_count: u32 },
}

impl MaterializationOutcome {
    pub fn is_blocked(&self) -> bool {
        matches!(self, MaterializationOutcome::Blocked { .. })
    }

    pub fn is_foundry_required(&self) -> bool {
        matches!(self, MaterializationOutcome::FoundryRequired { .. })
    }
}

// ─── Materialization Plan ─────────────────────────────────────────────────────

/// Content for deterministic `MaterializationPlan` ID.
#[derive(Serialize)]
struct PlanContent {
    collapse_plan_id: Digest,
    token_id: Digest,
    policy_id: Digest,
    outcome: String,
    task_count: u32,
}

/// Planning artifact for an operator-approved materialization run.
///
/// Contains the outcome evaluation and, if not blocked, the full list of
/// `WorkbenchMaterializationTask`s ready for Foundry submission.
/// Tasks are sorted by `task_id` for deterministic replay.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaterializationPlan {
    pub plan_artifact_id: Digest,
    pub collapse_plan_id: Digest,
    pub token_id: Digest,
    pub policy_id: Digest,
    pub outcome: MaterializationOutcome,
    /// Populated when `outcome = FoundryRequired`. Empty when Blocked.
    pub tasks: Vec<WorkbenchMaterializationTask>,
}

impl MaterializationPlan {
    fn new(
        collapse_plan: &HostTargetCollapsePlan,
        token_id: Digest,
        policy: &PolicyProfile,
        outcome: MaterializationOutcome,
        mut tasks: Vec<WorkbenchMaterializationTask>,
    ) -> Self {
        tasks.sort_by_key(|t| t.task_id);
        let task_count = tasks.len() as u32;
        let plan_artifact_id = Digest::of(&PlanContent {
            collapse_plan_id: collapse_plan.plan_id,
            token_id,
            policy_id: policy.id,
            outcome: format!("{:?}", outcome),
            task_count,
        });
        Self {
            plan_artifact_id,
            collapse_plan_id: collapse_plan.plan_id,
            token_id,
            policy_id: policy.id,
            outcome,
            tasks,
        }
    }

    /// Evaluate a materialization request.
    ///
    /// Returns `Blocked` if:
    /// - No token is supplied.
    /// - Token does not cover this plan.
    /// - Approver lacks sufficient authority (not Human or Operator).
    /// - Policy mode is not `OperatorApproved`.
    /// - `allow_host_write` is false in the policy.
    ///
    /// Returns `FoundryRequired` if the above checks pass. Actual execution
    /// requires Foundry checks to pass in an isolated environment.
    pub fn evaluate(
        collapse_plan: &HostTargetCollapsePlan,
        token: Option<&OperatorApprovalToken>,
        policy: &PolicyProfile,
    ) -> Self {
        let blocked = |reason: &str| -> Self {
            Self::new(
                collapse_plan,
                Digest::ZERO,
                policy,
                MaterializationOutcome::Blocked { reason: reason.into() },
                vec![],
            )
        };

        let token = match token {
            None => return blocked("no operator approval token supplied"),
            Some(t) => t,
        };

        if !token.covers(collapse_plan) {
            return blocked("approval token does not cover this collapse plan");
        }
        if !token.has_sufficient_authority() {
            return blocked("approver lacks Human or Operator authority");
        }
        if !matches!(policy.mode, ImplementationMode::OperatorApproved) {
            return blocked("policy mode is not OperatorApproved");
        }
        if !policy.allow_host_write {
            return blocked("policy does not allow host writes");
        }

        // Build a WorkbenchMaterializationTask per collapse step.
        // Parse-back expectation uses the step_id as a stand-in for topology IDs
        // (in production, these would come from the host topology snapshot).
        let foundry_check = FoundryCheckSpec::new(
            FoundryCheckKind::Build,
            "cargo",
            vec!["build".into(), "--workspace".into()],
        );

        let tasks: Vec<WorkbenchMaterializationTask> = collapse_plan
            .steps
            .iter()
            .map(|step| {
                let parse_back = ParseBackExpectation::new(
                    step,
                    step.step_id,
                    Digest::of_bytes(b"expected-after"),
                    policy,
                );
                WorkbenchMaterializationTask::new(
                    step,
                    token,
                    parse_back,
                    vec![foundry_check.clone()],
                    policy,
                )
            })
            .collect();

        let task_count = tasks.len() as u32;
        Self::new(
            collapse_plan,
            token.token_id,
            policy,
            MaterializationOutcome::FoundryRequired { task_count },
            tasks,
        )
    }

    pub fn summary(&self) -> String {
        format!(
            "MaterializationPlan — outcome={:?} tasks={}",
            self.outcome,
            self.tasks.len(),
        )
    }
}

// ─── Simulated Foundry Result ─────────────────────────────────────────────────

/// Simulate a Foundry check result for a `WorkbenchMaterializationTask`.
///
/// In Phase 11, Foundry execution is declared but not wired to live processes.
/// This returns a content-addressed `FoundryCheckResult` for the task's first
/// check spec, reporting `Skipped` (ReportOnly mode) or `Passed` (OperatorApproved).
///
/// Phase 11 ceiling: actual process execution requires a future integration
/// that is not part of the current skeleton.
pub fn simulate_foundry_check(
    task: &WorkbenchMaterializationTask,
    policy: &PolicyProfile,
) -> FoundryCheckResult {
    let kind = task
        .foundry_checks
        .first()
        .map(|s| s.kind.clone())
        .unwrap_or(FoundryCheckKind::Custom("none".into()));
    if matches!(policy.mode, ImplementationMode::OperatorApproved) && policy.allow_host_write {
        FoundryCheckResult::new(kind, FoundryOutcome::Passed, Digest::of_bytes(b"sim-pass"), vec![])
    } else {
        FoundryCheckResult::new(
            kind,
            FoundryOutcome::Skipped { reason: "policy-blocks-execution".into() },
            Digest::of_bytes(b"sim-skip"),
            vec![],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::{AuthorityLabel, PolicyProfile};
    use kosmo_hyphae::{CollapseAction, CollapseStep, CollapsePlanStatus, HostTargetCollapsePlan};

    fn report_only() -> PolicyProfile {
        PolicyProfile::default_report_only()
    }

    fn op_approved() -> PolicyProfile {
        PolicyProfile::operator_approved()
    }

    fn make_plan(policy: &PolicyProfile) -> HostTargetCollapsePlan {
        let step = CollapseStep::new(
            0,
            CollapseAction::RecordNegativeEvidence { record_id: Digest::of_bytes(b"neg") },
            vec![],
            policy.id,
        );
        HostTargetCollapsePlan {
            plan_id: Digest::of_bytes(b"plan"),
            host_cube_id: Digest::of_bytes(b"host"),
            delta_id: Digest::of_bytes(b"delta"),
            steps: vec![step],
            status: CollapsePlanStatus::OperatorApprovalPending,
            policy_id: policy.id,
        }
    }

    fn make_token(plan: &HostTargetCollapsePlan, policy: &PolicyProfile) -> OperatorApprovalToken {
        OperatorApprovalToken::new(plan.plan_id, AuthorityLabel::Operator, policy)
    }

    // ── OperatorApprovalToken ──────────────────────────────────────────────────

    #[test]
    fn token_is_content_addressed() {
        let p = op_approved();
        let plan = make_plan(&p);
        let t1 = make_token(&plan, &p);
        let t2 = make_token(&plan, &p);
        assert_eq!(t1.token_id, t2.token_id);
        assert_ne!(t1.token_id, Digest::ZERO);
    }

    #[test]
    fn token_covers_correct_plan() {
        let p = op_approved();
        let plan = make_plan(&p);
        let token = make_token(&plan, &p);
        assert!(token.covers(&plan));
    }

    #[test]
    fn token_does_not_cover_different_plan() {
        let p = op_approved();
        let plan = make_plan(&p);
        let other_plan = HostTargetCollapsePlan {
            plan_id: Digest::of_bytes(b"other_plan"),
            ..make_plan(&p)
        };
        let token = make_token(&plan, &p);
        assert!(!token.covers(&other_plan));
    }

    #[test]
    fn token_operator_authority_is_sufficient() {
        let p = op_approved();
        let plan = make_plan(&p);
        let token = OperatorApprovalToken::new(plan.plan_id, AuthorityLabel::Operator, &p);
        assert!(token.has_sufficient_authority());
    }

    #[test]
    fn token_agent_authority_is_insufficient() {
        let p = op_approved();
        let plan = make_plan(&p);
        let token = OperatorApprovalToken::new(
            plan.plan_id,
            AuthorityLabel::Agent { name: "bot".into() },
            &p,
        );
        assert!(!token.has_sufficient_authority());
    }

    // ── ParseBackExpectation ───────────────────────────────────────────────────

    #[test]
    fn parse_back_expectation_is_content_addressed() {
        let p = op_approved();
        let step = CollapseStep::new(
            0,
            CollapseAction::RecordNegativeEvidence { record_id: Digest::ZERO },
            vec![],
            p.id,
        );
        let e1 = ParseBackExpectation::new(&step, Digest::of_bytes(b"b"), Digest::of_bytes(b"a"), &p);
        let e2 = ParseBackExpectation::new(&step, Digest::of_bytes(b"b"), Digest::of_bytes(b"a"), &p);
        assert_eq!(e1.expectation_id, e2.expectation_id);
        assert_ne!(e1.expectation_id, Digest::ZERO);
    }

    // ── WorkbenchMaterializationTask ───────────────────────────────────────────

    #[test]
    fn task_is_content_addressed() {
        let p = op_approved();
        let plan = make_plan(&p);
        let token = make_token(&plan, &p);
        let step = &plan.steps[0];
        let pb = ParseBackExpectation::new(step, step.step_id, Digest::of_bytes(b"after"), &p);
        let checks = vec![FoundryCheckSpec::new(FoundryCheckKind::Build, "cargo", vec!["build".into()])];
        let t1 = WorkbenchMaterializationTask::new(step, &token, pb.clone(), checks.clone(), &p);
        let t2 = WorkbenchMaterializationTask::new(step, &token, pb, checks, &p);
        assert_eq!(t1.task_id, t2.task_id);
        assert_ne!(t1.task_id, Digest::ZERO);
    }

    // ── MaterializationPlan::evaluate — Blocked cases ─────────────────────────

    #[test]
    fn evaluate_blocked_when_no_token() {
        let p = op_approved();
        let plan = make_plan(&p);
        let mp = MaterializationPlan::evaluate(&plan, None, &p);
        assert!(mp.outcome.is_blocked(), "no token must yield Blocked");
        assert!(mp.tasks.is_empty());
    }

    #[test]
    fn evaluate_blocked_when_token_wrong_plan() {
        let p = op_approved();
        let plan = make_plan(&p);
        let other = HostTargetCollapsePlan {
            plan_id: Digest::of_bytes(b"other"),
            ..make_plan(&p)
        };
        let token = make_token(&other, &p);
        let mp = MaterializationPlan::evaluate(&plan, Some(&token), &p);
        assert!(mp.outcome.is_blocked(), "wrong plan token must yield Blocked");
    }

    #[test]
    fn evaluate_blocked_when_policy_is_report_only() {
        let ro = report_only();
        let oa = op_approved();
        let plan = make_plan(&oa);
        let token = make_token(&plan, &oa);
        // Token is valid but policy is ReportOnly.
        let mp = MaterializationPlan::evaluate(&plan, Some(&token), &ro);
        assert!(mp.outcome.is_blocked(), "ReportOnly policy must yield Blocked");
    }

    #[test]
    fn evaluate_blocked_when_agent_authority() {
        let p = op_approved();
        let plan = make_plan(&p);
        let weak_token = OperatorApprovalToken::new(
            plan.plan_id,
            AuthorityLabel::Agent { name: "bot".into() },
            &p,
        );
        let mp = MaterializationPlan::evaluate(&plan, Some(&weak_token), &p);
        assert!(mp.outcome.is_blocked(), "agent authority must yield Blocked");
    }

    // ── MaterializationPlan::evaluate — FoundryRequired case ─────────────────

    #[test]
    fn evaluate_foundry_required_with_valid_token_and_policy() {
        let p = op_approved();
        let plan = make_plan(&p);
        let token = make_token(&plan, &p);
        let mp = MaterializationPlan::evaluate(&plan, Some(&token), &p);
        assert!(
            mp.outcome.is_foundry_required(),
            "valid token + OperatorApproved policy must yield FoundryRequired, not {:?}",
            mp.outcome
        );
        assert_eq!(mp.tasks.len(), plan.steps.len(), "one task per step");
    }

    #[test]
    fn evaluate_foundry_required_declares_parse_back_per_step() {
        let p = op_approved();
        let plan = make_plan(&p);
        let token = make_token(&plan, &p);
        let mp = MaterializationPlan::evaluate(&plan, Some(&token), &p);
        for task in &mp.tasks {
            // Parse-back expectation is declared for every task.
            assert_ne!(task.parse_back.expectation_id, Digest::ZERO);
            assert_ne!(task.parse_back.before_topology_id, Digest::ZERO);
        }
    }

    #[test]
    fn evaluate_foundry_required_has_foundry_checks_per_task() {
        let p = op_approved();
        let plan = make_plan(&p);
        let token = make_token(&plan, &p);
        let mp = MaterializationPlan::evaluate(&plan, Some(&token), &p);
        for task in &mp.tasks {
            assert!(
                !task.foundry_checks.is_empty(),
                "every materialization task must declare at least one Foundry check"
            );
        }
    }

    #[test]
    fn materialization_plan_is_content_addressed() {
        let p = op_approved();
        let plan = make_plan(&p);
        let token = make_token(&plan, &p);
        let mp1 = MaterializationPlan::evaluate(&plan, Some(&token), &p);
        let mp2 = MaterializationPlan::evaluate(&plan, Some(&token), &p);
        assert_eq!(mp1.plan_artifact_id, mp2.plan_artifact_id);
        assert_ne!(mp1.plan_artifact_id, Digest::ZERO);
    }

    #[test]
    fn tasks_sorted_by_task_id() {
        let p = op_approved();
        let plan = make_plan(&p);
        let token = make_token(&plan, &p);
        let mp = MaterializationPlan::evaluate(&plan, Some(&token), &p);
        let ids: Vec<Digest> = mp.tasks.iter().map(|t| t.task_id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "tasks must be sorted by task_id");
    }

    // ── PolicyProfile::operator_approved() invariants ─────────────────────────

    #[test]
    fn operator_approved_policy_allows_host_write() {
        let p = op_approved();
        assert!(p.allow_host_write, "OperatorApproved policy must allow host writes");
        assert!(matches!(p.mode, ImplementationMode::OperatorApproved));
    }

    #[test]
    fn operator_approved_policy_keeps_foundry_requirement() {
        let p = op_approved();
        assert!(
            p.require_foundry_for_executable_effects,
            "OperatorApproved policy must still require Foundry"
        );
        assert!(
            p.require_parseback_for_topology_changes,
            "OperatorApproved policy must still require parse-back"
        );
        assert!(
            p.require_operator_approval_for_materialization,
            "OperatorApproved policy must still require operator approval gate"
        );
    }

    #[test]
    fn operator_approved_policy_disallows_memory_promotion() {
        let p = op_approved();
        assert!(!p.allow_memory_promotion, "memory promotion must stay false");
        assert!(!p.allow_synthetic_sourcecube, "synthetic source cube must stay false");
        assert!(!p.allow_network, "network access must stay false");
    }

    // ── simulate_foundry_check ─────────────────────────────────────────────────

    #[test]
    fn simulate_foundry_check_passes_in_operator_approved_mode() {
        let p = op_approved();
        let plan = make_plan(&p);
        let token = make_token(&plan, &p);
        let mp = MaterializationPlan::evaluate(&plan, Some(&token), &p);
        if let Some(task) = mp.tasks.first() {
            let result = simulate_foundry_check(task, &p);
            assert!(result.outcome.is_passed(), "simulated Foundry check must pass in OperatorApproved mode");
        }
    }

    #[test]
    fn simulate_foundry_check_skipped_in_report_only_mode() {
        let p = op_approved();
        let ro = report_only();
        let plan = make_plan(&p);
        let token = make_token(&plan, &p);
        let mp = MaterializationPlan::evaluate(&plan, Some(&token), &p);
        if let Some(task) = mp.tasks.first() {
            let result = simulate_foundry_check(task, &ro);
            assert!(!result.outcome.is_passed(), "Foundry check must be Skipped in ReportOnly mode");
        }
    }

    // ── Summary ───────────────────────────────────────────────────────────────

    #[test]
    fn materialization_plan_summary_non_empty() {
        let p = op_approved();
        let plan = make_plan(&p);
        let token = make_token(&plan, &p);
        let mp = MaterializationPlan::evaluate(&plan, Some(&token), &p);
        let s = mp.summary();
        assert!(!s.is_empty());
        assert!(s.contains("MaterializationPlan"));
    }
}

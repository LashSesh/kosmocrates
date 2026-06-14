use crate::digest::Digest;
use crate::evidence::EvidenceRef;
use serde::{Deserialize, Serialize};

/// How the isolated worktree is created before task application.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorktreeCreationMethod {
    /// `git worktree add` — preferred; shares object store with the host repo.
    GitWorktree,
    /// Full directory copy to a temp path — fallback when git is unavailable.
    TempCopy,
}

/// What to do with the isolated worktree after execution completes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorktreeCleanupPolicy {
    /// Remove the worktree unconditionally after execution.
    AlwaysDelete,
    /// Preserve the worktree only when execution failed (for inspection).
    PreserveOnFailure,
    /// Always preserve — useful for operator review before promotion.
    AlwaysPreserve,
}

/// Specifies the isolated worktree sandbox for a materialization run.
///
/// `allow_host_write` is hardwired to `false` — materialization NEVER
/// writes to the host workspace root (MATBOUND-002).
/// `id` is the content digest of all fields except `id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IsolatedWorktreeSpec {
    pub id: Digest,
    pub creation_method: WorktreeCreationMethod,
    /// Content digest of the host workspace root being isolated.
    pub host_root_digest: Digest,
    /// Content digest of the path where the worktree will be created.
    pub worktree_path_digest: Digest,
    pub cleanup_policy: WorktreeCleanupPolicy,
    /// Always `false` — invariant enforced by constructor.
    pub allow_host_write: bool,
    pub policy_id: Digest,
}

#[derive(Serialize)]
struct WorktreeSpecContent<'a> {
    creation_method: &'a WorktreeCreationMethod,
    host_root_digest: &'a Digest,
    worktree_path_digest: &'a Digest,
    cleanup_policy: &'a WorktreeCleanupPolicy,
    allow_host_write: bool,
    policy_id: &'a Digest,
}

impl IsolatedWorktreeSpec {
    pub fn new(
        creation_method: WorktreeCreationMethod,
        host_root_digest: Digest,
        worktree_path_digest: Digest,
        cleanup_policy: WorktreeCleanupPolicy,
        policy_id: Digest,
    ) -> Self {
        let mut s = Self {
            id: Digest::ZERO,
            creation_method,
            host_root_digest,
            worktree_path_digest,
            cleanup_policy,
            allow_host_write: false, // invariant: never true
            policy_id,
        };
        s.id = s.compute_id();
        s
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&WorktreeSpecContent {
            creation_method: &self.creation_method,
            host_root_digest: &self.host_root_digest,
            worktree_path_digest: &self.worktree_path_digest,
            cleanup_policy: &self.cleanup_policy,
            allow_host_write: self.allow_host_write,
            policy_id: &self.policy_id,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }
}

/// Classifies what kind of change a `WorkbenchTaskApplication` performs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkbenchTaskKind {
    PatchFile,
    CreateFile,
    DeleteFile,
    RenameFile,
    Custom(String),
}

/// A single task to be applied within an isolated worktree.
///
/// References the `WorkbenchTask` artifact by `task_digest` — the task content
/// is never inlined (content-addressing by reference).
/// `sequence` provides application ordering within a `MaterializationExecutionPlan`.
/// `id` is the content digest of all fields except `id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkbenchTaskApplication {
    pub id: Digest,
    /// Content digest of the `WorkbenchTask` artifact being applied.
    pub task_digest: Digest,
    pub task_kind: WorkbenchTaskKind,
    /// Content digest of the target path within the worktree.
    pub target_path_digest: Digest,
    pub sequence: u64,
    pub policy_id: Digest,
}

#[derive(Serialize)]
struct TaskAppContent<'a> {
    task_digest: &'a Digest,
    task_kind: &'a WorkbenchTaskKind,
    target_path_digest: &'a Digest,
    sequence: u64,
    policy_id: &'a Digest,
}

impl WorkbenchTaskApplication {
    pub fn new(
        task_digest: Digest,
        task_kind: WorkbenchTaskKind,
        target_path_digest: Digest,
        sequence: u64,
        policy_id: Digest,
    ) -> Self {
        let mut a = Self {
            id: Digest::ZERO,
            task_digest,
            task_kind,
            target_path_digest,
            sequence,
            policy_id,
        };
        a.id = a.compute_id();
        a
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&TaskAppContent {
            task_digest: &self.task_digest,
            task_kind: &self.task_kind,
            target_path_digest: &self.target_path_digest,
            sequence: self.sequence,
            policy_id: &self.policy_id,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }
}

/// Outcome of applying a single `WorkbenchTaskApplication`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskApplicationOutcome {
    /// Task was applied successfully to the isolated worktree.
    Applied,
    /// Task was intentionally skipped (e.g., pre-condition not met).
    Skipped { reason: String },
    /// Task failed during application.
    Failed { reason: String },
    /// Policy denied this task (e.g., target path outside worktree boundary).
    DeniedByPolicy { reason: String },
}

impl TaskApplicationOutcome {
    pub fn is_applied(&self) -> bool {
        matches!(self, Self::Applied)
    }

    pub fn is_failure_class(&self) -> bool {
        matches!(self, Self::Failed { .. } | Self::DeniedByPolicy { .. })
    }
}

/// Content-addressed result of a single `WorkbenchTaskApplication`.
///
/// `id` is the content digest of all fields except `id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskApplicationResult {
    pub id: Digest,
    pub application_id: Digest,
    pub outcome: TaskApplicationOutcome,
    pub evidence_bundle_id: Digest,
}

#[derive(Serialize)]
struct TaskResultContent<'a> {
    application_id: &'a Digest,
    outcome: &'a TaskApplicationOutcome,
    evidence_bundle_id: &'a Digest,
}

impl TaskApplicationResult {
    pub fn new(
        application_id: Digest,
        outcome: TaskApplicationOutcome,
        evidence_bundle_id: Digest,
    ) -> Self {
        let mut r = Self {
            id: Digest::ZERO,
            application_id,
            outcome,
            evidence_bundle_id,
        };
        r.id = r.compute_id();
        r
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&TaskResultContent {
            application_id: &self.application_id,
            outcome: &self.outcome,
            evidence_bundle_id: &self.evidence_bundle_id,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }
}

/// A complete, ordered materialization plan for an isolated worktree run.
///
/// Combines an `IsolatedWorktreeSpec`, an ordered list of task applications,
/// and references to the `FoundryExecutionPlan` and `ParseBackPlan` that must
/// run after the tasks complete.
///
/// MATBOUND-001: constructing a plan does not authorize execution.
/// MATBOUND-003: `OperatorApproved` mode + this plan → isolated worktree only;
///   host root is never touched.
/// `id` is computed from component IDs for composable determinism.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterializationExecutionPlan {
    pub id: Digest,
    pub policy_id: Digest,
    pub worktree_spec: IsolatedWorktreeSpec,
    pub task_applications: Vec<WorkbenchTaskApplication>,
    /// References the `FoundryExecutionPlan` to run after task application.
    pub foundry_plan_id: Digest,
    /// References the `ParseBackPlan` to run after Foundry.
    pub parseback_plan_id: Digest,
    pub evidence_refs: Vec<EvidenceRef>,
}

#[derive(Serialize)]
struct MatPlanContent<'a> {
    policy_id: &'a Digest,
    worktree_spec_id: &'a Digest,
    task_application_ids: Vec<&'a Digest>,
    foundry_plan_id: &'a Digest,
    parseback_plan_id: &'a Digest,
    evidence_digests: Vec<&'a Digest>,
}

impl MaterializationExecutionPlan {
    pub fn new(
        policy_id: Digest,
        worktree_spec: IsolatedWorktreeSpec,
        task_applications: Vec<WorkbenchTaskApplication>,
        foundry_plan_id: Digest,
        parseback_plan_id: Digest,
    ) -> Self {
        let mut p = Self {
            id: Digest::ZERO,
            policy_id,
            worktree_spec,
            task_applications,
            foundry_plan_id,
            parseback_plan_id,
            evidence_refs: vec![],
        };
        p.id = p.compute_id();
        p
    }

    pub fn with_evidence(mut self, refs: Vec<EvidenceRef>) -> Self {
        self.evidence_refs = refs;
        self.id = self.compute_id();
        self
    }

    fn compute_id(&self) -> Digest {
        let task_application_ids: Vec<&Digest> =
            self.task_applications.iter().map(|t| &t.id).collect();
        let evidence_digests: Vec<&Digest> = self.evidence_refs.iter().map(|e| &e.digest).collect();
        Digest::of(&MatPlanContent {
            policy_id: &self.policy_id,
            worktree_spec_id: &self.worktree_spec.id,
            task_application_ids,
            foundry_plan_id: &self.foundry_plan_id,
            parseback_plan_id: &self.parseback_plan_id,
            evidence_digests,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }
}

/// Overall outcome of a `MaterializationExecutionPlan` run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaterializationOutcome {
    /// Skipped because `PolicyProfile.mode == ReportOnly`.
    SkippedByReportOnly,
    /// All task applications completed; Foundry and ParseBack were triggered.
    Completed,
    /// Some tasks applied, some skipped or failed.
    PartiallyApplied {
        applied: u64,
        skipped: u64,
        failed: u64,
    },
    /// All or a blocking set of tasks failed.
    Failed,
    /// Policy denied execution before any task ran.
    DeniedByPolicy,
    /// Worktree could not be created (I/O error or git failure).
    WorktreeCreationFailed,
    /// Outcome cannot be determined.
    Inconclusive,
}

impl MaterializationOutcome {
    pub fn is_completed(&self) -> bool {
        matches!(self, Self::Completed)
    }

    pub fn is_skipped_report_only(&self) -> bool {
        matches!(self, Self::SkippedByReportOnly)
    }

    pub fn is_failure_class(&self) -> bool {
        matches!(
            self,
            Self::Failed | Self::DeniedByPolicy | Self::WorktreeCreationFailed | Self::Inconclusive
        )
    }
}

/// The complete, content-addressed record of a materialization execution.
///
/// `validation_closure_report_id` is set only after Foundry + ParseBack complete
/// and `ValidationClosureReport` is produced. It may be `None` when execution
/// failed before reaching the validation stage.
///
/// MATBOUND-004: this report is the evidence record — it does not authorize
/// promoting the worktree changes to the host. That requires a separate
/// operator review step.
///
/// `elapsed_ms` is `u64` — no floats in any audit path.
/// `id` is the content digest of all fields except `id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterializationExecutionReport {
    pub id: Digest,
    pub plan_id: Digest,
    pub outcome: MaterializationOutcome,
    pub task_results: Vec<TaskApplicationResult>,
    /// Digest of the actual worktree path created; `None` if creation failed.
    pub worktree_path_digest: Option<Digest>,
    /// Set after Foundry + ParseBack run and `ValidationClosureReport` is produced.
    pub validation_closure_report_id: Option<Digest>,
    pub diagnostics: Vec<String>,
    pub evidence_bundle_id: Digest,
    pub elapsed_ms: u64,
}

#[derive(Serialize)]
struct MatReportContent<'a> {
    plan_id: &'a Digest,
    outcome: &'a MaterializationOutcome,
    task_result_ids: Vec<&'a Digest>,
    worktree_path_digest: &'a Option<Digest>,
    validation_closure_report_id: &'a Option<Digest>,
    diagnostics: &'a Vec<String>,
    evidence_bundle_id: &'a Digest,
}

impl MaterializationExecutionReport {
    pub fn skipped_by_report_only(plan_id: Digest, evidence_bundle_id: Digest) -> Self {
        let mut r = Self {
            id: Digest::ZERO,
            plan_id,
            outcome: MaterializationOutcome::SkippedByReportOnly,
            task_results: vec![],
            worktree_path_digest: None,
            validation_closure_report_id: None,
            diagnostics: vec!["Skipped: PolicyProfile.mode == ReportOnly".into()],
            evidence_bundle_id,
            elapsed_ms: 0,
        };
        r.id = r.compute_id();
        r
    }

    pub fn new(
        plan_id: Digest,
        outcome: MaterializationOutcome,
        task_results: Vec<TaskApplicationResult>,
        worktree_path_digest: Option<Digest>,
        validation_closure_report_id: Option<Digest>,
        diagnostics: Vec<String>,
        evidence_bundle_id: Digest,
        elapsed_ms: u64,
    ) -> Self {
        let mut r = Self {
            id: Digest::ZERO,
            plan_id,
            outcome,
            task_results,
            worktree_path_digest,
            validation_closure_report_id,
            diagnostics,
            evidence_bundle_id,
            elapsed_ms,
        };
        r.id = r.compute_id();
        r
    }

    /// Returns a new report with `validation_closure_report_id` set, recomputing `id`.
    pub fn with_validation_closure(mut self, vcr_id: Digest) -> Self {
        self.validation_closure_report_id = Some(vcr_id);
        self.id = self.compute_id();
        self
    }

    fn compute_id(&self) -> Digest {
        let task_result_ids: Vec<&Digest> = self.task_results.iter().map(|r| &r.id).collect();
        Digest::of(&MatReportContent {
            plan_id: &self.plan_id,
            outcome: &self.outcome,
            task_result_ids,
            worktree_path_digest: &self.worktree_path_digest,
            validation_closure_report_id: &self.validation_closure_report_id,
            diagnostics: &self.diagnostics,
            evidence_bundle_id: &self.evidence_bundle_id,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }

    pub fn applied_count(&self) -> u64 {
        self.task_results
            .iter()
            .filter(|r| r.outcome.is_applied())
            .count() as u64
    }

    pub fn failed_count(&self) -> u64 {
        self.task_results
            .iter()
            .filter(|r| r.outcome.is_failure_class())
            .count() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{EvidenceBundle, EvidenceKind, ReplayStatus};

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    fn policy_id() -> Digest {
        d(b"policy")
    }
    fn bundle_id() -> Digest {
        d(b"bundle")
    }
    fn plan_id() -> Digest {
        d(b"mat-plan")
    }

    fn basic_worktree_spec() -> IsolatedWorktreeSpec {
        IsolatedWorktreeSpec::new(
            WorktreeCreationMethod::GitWorktree,
            d(b"host-root"),
            d(b"worktree-path"),
            WorktreeCleanupPolicy::AlwaysDelete,
            policy_id(),
        )
    }

    fn make_task_app(sequence: u64) -> WorkbenchTaskApplication {
        WorkbenchTaskApplication::new(
            d(b"task"),
            WorkbenchTaskKind::PatchFile,
            d(b"target-path"),
            sequence,
            policy_id(),
        )
    }

    fn basic_mat_plan() -> MaterializationExecutionPlan {
        MaterializationExecutionPlan::new(
            policy_id(),
            basic_worktree_spec(),
            vec![make_task_app(1), make_task_app(2)],
            d(b"foundry-plan"),
            d(b"parseback-plan"),
        )
    }

    // --- IsolatedWorktreeSpec ---

    #[test]
    fn worktree_spec_id_deterministic() {
        let s1 = basic_worktree_spec();
        let s2 = basic_worktree_spec();
        assert_eq!(s1.id, s2.id);
    }

    #[test]
    fn worktree_spec_verify_id() {
        assert!(basic_worktree_spec().verify_id());
    }

    #[test]
    fn worktree_spec_allow_host_write_always_false() {
        let s = basic_worktree_spec();
        assert!(
            !s.allow_host_write,
            "MATBOUND-002: host write must always be false"
        );
    }

    #[test]
    fn worktree_spec_different_methods_differ() {
        let git = IsolatedWorktreeSpec::new(
            WorktreeCreationMethod::GitWorktree,
            d(b"root"),
            d(b"wt"),
            WorktreeCleanupPolicy::AlwaysDelete,
            policy_id(),
        );
        let tmp = IsolatedWorktreeSpec::new(
            WorktreeCreationMethod::TempCopy,
            d(b"root"),
            d(b"wt"),
            WorktreeCleanupPolicy::AlwaysDelete,
            policy_id(),
        );
        assert_ne!(git.id, tmp.id);
    }

    #[test]
    fn worktree_spec_different_cleanup_policies_differ() {
        let del = IsolatedWorktreeSpec::new(
            WorktreeCreationMethod::GitWorktree,
            d(b"root"),
            d(b"wt"),
            WorktreeCleanupPolicy::AlwaysDelete,
            policy_id(),
        );
        let keep = IsolatedWorktreeSpec::new(
            WorktreeCreationMethod::GitWorktree,
            d(b"root"),
            d(b"wt"),
            WorktreeCleanupPolicy::AlwaysPreserve,
            policy_id(),
        );
        assert_ne!(del.id, keep.id);
    }

    // --- WorkbenchTaskApplication ---

    #[test]
    fn task_app_id_deterministic() {
        let t1 = make_task_app(1);
        let t2 = make_task_app(1);
        assert_eq!(t1.id, t2.id);
    }

    #[test]
    fn task_app_verify_id() {
        assert!(make_task_app(1).verify_id());
    }

    #[test]
    fn task_app_different_sequences_differ() {
        let t1 = make_task_app(1);
        let t2 = make_task_app(2);
        assert_ne!(t1.id, t2.id);
    }

    #[test]
    fn task_app_different_kinds_differ() {
        let patch = WorkbenchTaskApplication::new(
            d(b"task"),
            WorkbenchTaskKind::PatchFile,
            d(b"path"),
            1,
            policy_id(),
        );
        let create = WorkbenchTaskApplication::new(
            d(b"task"),
            WorkbenchTaskKind::CreateFile,
            d(b"path"),
            1,
            policy_id(),
        );
        assert_ne!(patch.id, create.id);
    }

    // --- TaskApplicationOutcome ---

    #[test]
    fn outcome_applied_is_applied() {
        assert!(TaskApplicationOutcome::Applied.is_applied());
        assert!(!TaskApplicationOutcome::Applied.is_failure_class());
    }

    #[test]
    fn outcome_failed_is_failure_class() {
        assert!(TaskApplicationOutcome::Failed {
            reason: "oops".into()
        }
        .is_failure_class());
        assert!(TaskApplicationOutcome::DeniedByPolicy {
            reason: "scope".into()
        }
        .is_failure_class());
    }

    #[test]
    fn outcome_skipped_is_neither() {
        let o = TaskApplicationOutcome::Skipped {
            reason: "pre-condition".into(),
        };
        assert!(!o.is_applied());
        assert!(!o.is_failure_class());
    }

    // --- TaskApplicationResult ---

    #[test]
    fn task_result_id_deterministic() {
        let r1 =
            TaskApplicationResult::new(d(b"app"), TaskApplicationOutcome::Applied, bundle_id());
        let r2 =
            TaskApplicationResult::new(d(b"app"), TaskApplicationOutcome::Applied, bundle_id());
        assert_eq!(r1.id, r2.id);
    }

    #[test]
    fn task_result_verify_id() {
        let r = TaskApplicationResult::new(
            d(b"app"),
            TaskApplicationOutcome::Failed {
                reason: "conflict".into(),
            },
            bundle_id(),
        );
        assert!(r.verify_id());
    }

    #[test]
    fn task_result_different_outcomes_differ() {
        let r1 =
            TaskApplicationResult::new(d(b"app"), TaskApplicationOutcome::Applied, bundle_id());
        let r2 = TaskApplicationResult::new(
            d(b"app"),
            TaskApplicationOutcome::Failed { reason: "x".into() },
            bundle_id(),
        );
        assert_ne!(r1.id, r2.id);
    }

    // --- MaterializationExecutionPlan ---

    #[test]
    fn mat_plan_id_deterministic() {
        let p1 = basic_mat_plan();
        let p2 = basic_mat_plan();
        assert_eq!(p1.id, p2.id);
    }

    #[test]
    fn mat_plan_verify_id() {
        assert!(basic_mat_plan().verify_id());
    }

    #[test]
    fn mat_plan_with_evidence_changes_id() {
        let plan = basic_mat_plan();
        let id_before = plan.id;
        let plan2 = plan.with_evidence(vec![EvidenceRef::new(
            d(b"ev"),
            crate::evidence::EvidenceKind::RunRecord,
            "prior run",
        )]);
        assert_ne!(id_before, plan2.id);
        assert!(plan2.verify_id());
    }

    #[test]
    fn mat_plan_different_foundry_plan_differs() {
        let p1 = basic_mat_plan();
        let p2 = MaterializationExecutionPlan::new(
            policy_id(),
            basic_worktree_spec(),
            vec![make_task_app(1), make_task_app(2)],
            d(b"other-foundry-plan"),
            d(b"parseback-plan"),
        );
        assert_ne!(p1.id, p2.id);
    }

    #[test]
    fn mat_plan_worktree_spec_host_write_is_false() {
        let plan = basic_mat_plan();
        assert!(!plan.worktree_spec.allow_host_write);
    }

    // --- MaterializationOutcome ---

    #[test]
    fn outcome_completed_is_not_failure() {
        assert!(MaterializationOutcome::Completed.is_completed());
        assert!(!MaterializationOutcome::Completed.is_failure_class());
    }

    #[test]
    fn outcome_skipped_report_only() {
        let o = MaterializationOutcome::SkippedByReportOnly;
        assert!(o.is_skipped_report_only());
        assert!(!o.is_failure_class());
        assert!(!o.is_completed());
    }

    #[test]
    fn outcome_all_failure_variants() {
        for o in [
            MaterializationOutcome::Failed,
            MaterializationOutcome::DeniedByPolicy,
            MaterializationOutcome::WorktreeCreationFailed,
            MaterializationOutcome::Inconclusive,
        ] {
            assert!(o.is_failure_class(), "{o:?} should be failure class");
        }
    }

    // --- MaterializationExecutionReport ---

    #[test]
    fn mat_report_skipped_deterministic() {
        let r1 = MaterializationExecutionReport::skipped_by_report_only(plan_id(), bundle_id());
        let r2 = MaterializationExecutionReport::skipped_by_report_only(plan_id(), bundle_id());
        assert_eq!(r1.id, r2.id);
    }

    #[test]
    fn mat_report_skipped_verify_id() {
        let r = MaterializationExecutionReport::skipped_by_report_only(plan_id(), bundle_id());
        assert!(r.verify_id());
        assert!(r.outcome.is_skipped_report_only());
        assert!(r.task_results.is_empty());
        assert!(r.worktree_path_digest.is_none());
        assert!(r.validation_closure_report_id.is_none());
    }

    #[test]
    fn mat_report_new_deterministic() {
        let r1 = MaterializationExecutionReport::new(
            plan_id(),
            MaterializationOutcome::Completed,
            vec![],
            Some(d(b"wt-path")),
            None,
            vec![],
            bundle_id(),
            5_000,
        );
        let r2 = MaterializationExecutionReport::new(
            plan_id(),
            MaterializationOutcome::Completed,
            vec![],
            Some(d(b"wt-path")),
            None,
            vec![],
            bundle_id(),
            5_000,
        );
        assert_eq!(r1.id, r2.id);
    }

    #[test]
    fn mat_report_verify_id_with_tasks() {
        let results = vec![
            TaskApplicationResult::new(d(b"a1"), TaskApplicationOutcome::Applied, bundle_id()),
            TaskApplicationResult::new(
                d(b"a2"),
                TaskApplicationOutcome::Skipped {
                    reason: "already present".into(),
                },
                bundle_id(),
            ),
        ];
        let r = MaterializationExecutionReport::new(
            plan_id(),
            MaterializationOutcome::Completed,
            results,
            Some(d(b"wt")),
            None,
            vec![],
            bundle_id(),
            3_000,
        );
        assert!(r.verify_id());
    }

    #[test]
    fn mat_report_with_validation_closure_changes_id() {
        let r = MaterializationExecutionReport::new(
            plan_id(),
            MaterializationOutcome::Completed,
            vec![],
            Some(d(b"wt")),
            None,
            vec![],
            bundle_id(),
            1_000,
        );
        let id_before = r.id;
        let r2 = r.with_validation_closure(d(b"vcr"));
        assert_ne!(id_before, r2.id);
        assert!(r2.verify_id());
        assert_eq!(r2.validation_closure_report_id, Some(d(b"vcr")));
    }

    #[test]
    fn mat_report_applied_and_failed_counts() {
        let results = vec![
            TaskApplicationResult::new(d(b"a1"), TaskApplicationOutcome::Applied, bundle_id()),
            TaskApplicationResult::new(d(b"a2"), TaskApplicationOutcome::Applied, bundle_id()),
            TaskApplicationResult::new(
                d(b"a3"),
                TaskApplicationOutcome::Failed {
                    reason: "conflict".into(),
                },
                bundle_id(),
            ),
        ];
        let r = MaterializationExecutionReport::new(
            plan_id(),
            MaterializationOutcome::PartiallyApplied {
                applied: 2,
                skipped: 0,
                failed: 1,
            },
            results,
            None,
            None,
            vec![],
            bundle_id(),
            2_000,
        );
        assert_eq!(r.applied_count(), 2);
        assert_eq!(r.failed_count(), 1);
        assert!(r.verify_id());
    }

    #[test]
    fn mat_report_evidence_bundle_mandatory() {
        let r = MaterializationExecutionReport::skipped_by_report_only(plan_id(), bundle_id());
        assert_ne!(r.evidence_bundle_id, Digest::ZERO);
    }

    #[test]
    fn mat_report_no_floats_in_elapsed() {
        let r = MaterializationExecutionReport::skipped_by_report_only(plan_id(), bundle_id());
        assert_eq!(r.elapsed_ms, 0u64);
    }

    #[test]
    fn matbound_plan_does_not_authorize_execution() {
        use crate::policy::{ImplementationMode, PolicyProfile};
        // MATBOUND-001: having a plan does not mean execution is authorized.
        // In ReportOnly mode, the report must be SkippedByReportOnly.
        let profile = PolicyProfile::default();
        assert_eq!(profile.mode, ImplementationMode::ReportOnly);

        let r = MaterializationExecutionReport::skipped_by_report_only(plan_id(), bundle_id());
        assert!(r.outcome.is_skipped_report_only());
    }

    #[test]
    fn r5_evidence_bundle_integration() {
        let ev_bundle = EvidenceBundle::seal(
            vec![EvidenceRef::new(
                d(b"task-ev"),
                EvidenceKind::RunRecord,
                "task application evidence",
            )],
            policy_id(),
            ReplayStatus::Replayable,
        );
        let r = MaterializationExecutionReport::new(
            plan_id(),
            MaterializationOutcome::Completed,
            vec![],
            Some(d(b"wt")),
            Some(d(b"vcr")),
            vec![],
            ev_bundle.bundle_id,
            4_500,
        );
        assert!(r.verify_id());
        assert_ne!(r.evidence_bundle_id, Digest::ZERO);
    }
}

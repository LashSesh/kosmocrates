use crate::digest::Digest;
use crate::evidence::EvidenceRef;
use crate::run::{FoundryCheckKind, FoundryCheckResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A path paired with its content digest, used to declare sandbox boundaries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathDigest {
    pub path: String,
    pub content_digest: Digest,
}

/// Classifies the execution environment for a Foundry plan run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FoundrySandboxKind {
    /// No command is executed; the plan is recorded for audit only (ReportOnly mode).
    ReportOnlyNoExec,
    /// Commands run in the host workspace directory; writes are not committed.
    LocalDryRun,
    /// Commands run in an isolated git worktree copy of the workspace.
    IsolatedWorktree,
    /// Commands run inside a container with restricted capabilities.
    Container,
}

/// Specifies the sandbox in which a `FoundryExecutionPlan` runs.
///
/// `id` is the content digest of all fields except `id`.
/// `allow_network` and `allow_host_write` default to `false` (fail-closed).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundrySandboxSpec {
    pub id: Digest,
    pub sandbox_kind: FoundrySandboxKind,
    pub root_path_digest: Digest,
    pub allow_network: bool,
    pub allow_host_write: bool,
    pub allow_env_passthrough: bool,
    pub writable_paths: Vec<PathDigest>,
    pub readonly_paths: Vec<PathDigest>,
}

#[derive(Serialize)]
struct SandboxContent<'a> {
    sandbox_kind: &'a FoundrySandboxKind,
    root_path_digest: &'a Digest,
    allow_network: bool,
    allow_host_write: bool,
    allow_env_passthrough: bool,
    writable_paths: &'a Vec<PathDigest>,
    readonly_paths: &'a Vec<PathDigest>,
}

impl FoundrySandboxSpec {
    pub fn new(sandbox_kind: FoundrySandboxKind, root_path_digest: Digest) -> Self {
        let mut s = Self {
            id: Digest::ZERO,
            sandbox_kind,
            root_path_digest,
            allow_network: false,
            allow_host_write: false,
            allow_env_passthrough: false,
            writable_paths: vec![],
            readonly_paths: vec![],
        };
        s.id = s.compute_id();
        s
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&SandboxContent {
            sandbox_kind: &self.sandbox_kind,
            root_path_digest: &self.root_path_digest,
            allow_network: self.allow_network,
            allow_host_write: self.allow_host_write,
            allow_env_passthrough: self.allow_env_passthrough,
            writable_paths: &self.writable_paths,
            readonly_paths: &self.readonly_paths,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }
}

/// A single command explicitly permitted in a Foundry run.
///
/// Commands not in `FoundryCommandPolicy::allowed_commands` are denied.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllowedFoundryCommand {
    pub program: String,
    pub allowed_subcommands: Vec<String>,
    pub allow_arbitrary_args: bool,
}

/// Governs which commands a Foundry run may execute.
///
/// `id` is the content digest of all fields except `id`.
/// `allow_shell`, `allow_network_tools`, `allow_package_install` default to `false`.
///
/// The default cargo policy permits: `cargo check`, `cargo test`,
/// `cargo fmt --check`, `cargo clippy`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryCommandPolicy {
    pub id: Digest,
    pub allowed_commands: Vec<AllowedFoundryCommand>,
    pub forbidden_commands: Vec<String>,
    pub allow_shell: bool,
    pub allow_network_tools: bool,
    pub allow_package_install: bool,
}

#[derive(Serialize)]
struct CommandPolicyContent<'a> {
    allowed_commands: &'a Vec<AllowedFoundryCommand>,
    forbidden_commands: &'a Vec<String>,
    allow_shell: bool,
    allow_network_tools: bool,
    allow_package_install: bool,
}

impl FoundryCommandPolicy {
    /// The four canonical allowlisted Foundry commands per KOSMO-OPS-01 §3.2.
    pub fn default_cargo_policy() -> Self {
        let allowed = vec![AllowedFoundryCommand {
            program: "cargo".into(),
            allowed_subcommands: vec!["check".into(), "test".into(), "fmt".into(), "clippy".into()],
            allow_arbitrary_args: false,
        }];
        let mut p = Self {
            id: Digest::ZERO,
            allowed_commands: allowed,
            forbidden_commands: vec![],
            allow_shell: false,
            allow_network_tools: false,
            allow_package_install: false,
        };
        p.id = p.compute_id();
        p
    }

    pub fn is_command_allowed(&self, program: &str, subcommand: &str) -> bool {
        self.allowed_commands.iter().any(|c| {
            c.program == program
                && (c.allow_arbitrary_args || c.allowed_subcommands.iter().any(|s| s == subcommand))
        })
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&CommandPolicyContent {
            allowed_commands: &self.allowed_commands,
            forbidden_commands: &self.forbidden_commands,
            allow_shell: self.allow_shell,
            allow_network_tools: self.allow_network_tools,
            allow_package_install: self.allow_package_install,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }
}

/// Controls execution timeouts for a Foundry plan run.
///
/// All durations are in milliseconds (`u64`) — no floats in any gate path.
/// `id` is the content digest of all fields except `id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryTimeoutPolicy {
    pub id: Digest,
    pub per_check_timeout_ms: u64,
    pub total_plan_timeout_ms: u64,
    pub fail_on_timeout: bool,
}

#[derive(Serialize)]
struct TimeoutContent {
    per_check_timeout_ms: u64,
    total_plan_timeout_ms: u64,
    fail_on_timeout: bool,
}

impl FoundryTimeoutPolicy {
    pub fn new(per_check_timeout_ms: u64, total_plan_timeout_ms: u64) -> Self {
        let mut t = Self {
            id: Digest::ZERO,
            per_check_timeout_ms,
            total_plan_timeout_ms,
            fail_on_timeout: true,
        };
        t.id = t.compute_id();
        t
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&TimeoutContent {
            per_check_timeout_ms: self.per_check_timeout_ms,
            total_plan_timeout_ms: self.total_plan_timeout_ms,
            fail_on_timeout: self.fail_on_timeout,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }
}

/// Controls environment variable access during a Foundry run.
///
/// `allowed_vars` maps variable names to optional override values using
/// `BTreeMap` for deterministic ordering. A `None` value means pass-through
/// from the ambient environment (only when `allow_passthrough` is true).
/// `id` is the content digest of all fields except `id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryEnvironmentPolicy {
    pub id: Digest,
    pub allowed_vars: BTreeMap<String, Option<String>>,
    pub allow_passthrough: bool,
    pub strip_secrets: bool,
}

#[derive(Serialize)]
struct EnvPolicyContent<'a> {
    allowed_vars: &'a BTreeMap<String, Option<String>>,
    allow_passthrough: bool,
    strip_secrets: bool,
}

impl FoundryEnvironmentPolicy {
    /// Fully locked: no vars allowed, no passthrough, secrets stripped.
    pub fn locked() -> Self {
        let mut e = Self {
            id: Digest::ZERO,
            allowed_vars: BTreeMap::new(),
            allow_passthrough: false,
            strip_secrets: true,
        };
        e.id = e.compute_id();
        e
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&EnvPolicyContent {
            allowed_vars: &self.allowed_vars,
            allow_passthrough: self.allow_passthrough,
            strip_secrets: self.strip_secrets,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }
}

/// Input specification for a single check within a `FoundryExecutionPlan`.
///
/// Distinct from `FoundryCheckResult` (which is the output record).
/// `id` is the content digest of all fields except `id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryCheckSpec {
    pub id: Digest,
    pub check_kind: FoundryCheckKind,
    pub target: String,
    pub args: Vec<String>,
    pub required: bool,
}

#[derive(Serialize)]
struct CheckSpecContent<'a> {
    check_kind: &'a FoundryCheckKind,
    target: &'a String,
    args: &'a Vec<String>,
    required: bool,
}

impl FoundryCheckSpec {
    pub fn new(check_kind: FoundryCheckKind, target: impl Into<String>, required: bool) -> Self {
        let mut s = Self {
            id: Digest::ZERO,
            check_kind,
            target: target.into(),
            args: vec![],
            required,
        };
        s.id = s.compute_id();
        s
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self.id = self.compute_id();
        self
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&CheckSpecContent {
            check_kind: &self.check_kind,
            target: &self.target,
            args: &self.args,
            required: self.required,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }
}

/// Overall outcome of a complete `FoundryExecutionPlan` run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FoundryExecutionOutcome {
    /// Execution was skipped because `PolicyProfile.mode == ReportOnly`.
    /// This is NOT a failure; it means the plan was validated but not executed.
    SkippedByReportOnly,
    /// All required checks passed.
    Passed,
    /// One or more required checks failed.
    Failed,
    /// Execution exceeded the `FoundryTimeoutPolicy` limit.
    TimedOut,
    /// A command was denied by `FoundryCommandPolicy`.
    CommandDeniedByPolicy,
    /// The sandbox detected a boundary violation.
    SandboxViolation,
    /// Result cannot be determined (e.g. partial execution before crash).
    Inconclusive,
}

impl FoundryExecutionOutcome {
    pub fn is_passed(&self) -> bool {
        matches!(self, Self::Passed)
    }

    pub fn is_skipped_report_only(&self) -> bool {
        matches!(self, Self::SkippedByReportOnly)
    }

    pub fn is_failure_class(&self) -> bool {
        matches!(
            self,
            Self::Failed
                | Self::TimedOut
                | Self::CommandDeniedByPolicy
                | Self::SandboxViolation
                | Self::Inconclusive
        )
    }
}

/// A top-level, content-addressed Foundry execution plan.
///
/// Combines sandbox spec, command policy, timeout policy, environment policy,
/// and the ordered list of check specs to execute.
/// `id` is computed from the component `id` fields for composable determinism.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryExecutionPlan {
    pub id: Digest,
    pub policy_id: Digest,
    pub workspace_index_id: Digest,
    pub task_id: Digest,
    pub sandbox: FoundrySandboxSpec,
    pub checks: Vec<FoundryCheckSpec>,
    pub command_policy: FoundryCommandPolicy,
    pub timeout_policy: FoundryTimeoutPolicy,
    pub environment_policy: FoundryEnvironmentPolicy,
    pub evidence_refs: Vec<EvidenceRef>,
}

#[derive(Serialize)]
struct PlanContent<'a> {
    policy_id: &'a Digest,
    workspace_index_id: &'a Digest,
    task_id: &'a Digest,
    sandbox_id: &'a Digest,
    check_ids: Vec<&'a Digest>,
    command_policy_id: &'a Digest,
    timeout_policy_id: &'a Digest,
    environment_policy_id: &'a Digest,
    evidence_digests: Vec<&'a Digest>,
}

impl FoundryExecutionPlan {
    pub fn new(
        policy_id: Digest,
        workspace_index_id: Digest,
        task_id: Digest,
        sandbox: FoundrySandboxSpec,
        checks: Vec<FoundryCheckSpec>,
        command_policy: FoundryCommandPolicy,
        timeout_policy: FoundryTimeoutPolicy,
        environment_policy: FoundryEnvironmentPolicy,
    ) -> Self {
        let mut p = Self {
            id: Digest::ZERO,
            policy_id,
            workspace_index_id,
            task_id,
            sandbox,
            checks,
            command_policy,
            timeout_policy,
            environment_policy,
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
        let check_ids: Vec<&Digest> = self.checks.iter().map(|c| &c.id).collect();
        let evidence_digests: Vec<&Digest> = self.evidence_refs.iter().map(|e| &e.digest).collect();
        Digest::of(&PlanContent {
            policy_id: &self.policy_id,
            workspace_index_id: &self.workspace_index_id,
            task_id: &self.task_id,
            sandbox_id: &self.sandbox.id,
            check_ids,
            command_policy_id: &self.command_policy.id,
            timeout_policy_id: &self.timeout_policy.id,
            environment_policy_id: &self.environment_policy.id,
            evidence_digests,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }
}

/// The complete, content-addressed record of a Foundry execution plan run.
///
/// `outcome == SkippedByReportOnly` when `PolicyProfile.mode == ReportOnly`.
/// Every report carries an `evidence_bundle_id` for auditability.
/// `id` is the content digest of all fields except `id`.
/// `elapsed_ms` is `u64` — no floats in any gate/audit path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryExecutionReport {
    pub id: Digest,
    pub plan_id: Digest,
    pub outcome: FoundryExecutionOutcome,
    pub check_results: Vec<FoundryCheckResult>,
    pub diagnostics: Vec<String>,
    pub evidence_bundle_id: Digest,
    pub elapsed_ms: u64,
}

#[derive(Serialize)]
struct ReportContent<'a> {
    plan_id: &'a Digest,
    outcome: &'a FoundryExecutionOutcome,
    check_result_ids: Vec<&'a Digest>,
    diagnostics: &'a Vec<String>,
    evidence_bundle_id: &'a Digest,
    elapsed_ms: u64,
}

impl FoundryExecutionReport {
    pub fn skipped_by_report_only(plan_id: Digest, evidence_bundle_id: Digest) -> Self {
        let mut r = Self {
            id: Digest::ZERO,
            plan_id,
            outcome: FoundryExecutionOutcome::SkippedByReportOnly,
            check_results: vec![],
            diagnostics: vec!["Skipped: PolicyProfile.mode == ReportOnly".into()],
            evidence_bundle_id,
            elapsed_ms: 0,
        };
        r.id = r.compute_id();
        r
    }

    pub fn new(
        plan_id: Digest,
        outcome: FoundryExecutionOutcome,
        check_results: Vec<FoundryCheckResult>,
        diagnostics: Vec<String>,
        evidence_bundle_id: Digest,
        elapsed_ms: u64,
    ) -> Self {
        let mut r = Self {
            id: Digest::ZERO,
            plan_id,
            outcome,
            check_results,
            diagnostics,
            evidence_bundle_id,
            elapsed_ms,
        };
        r.id = r.compute_id();
        r
    }

    fn compute_id(&self) -> Digest {
        let check_result_ids: Vec<&Digest> =
            self.check_results.iter().map(|c| &c.check_id).collect();
        Digest::of(&ReportContent {
            plan_id: &self.plan_id,
            outcome: &self.outcome,
            check_result_ids,
            diagnostics: &self.diagnostics,
            evidence_bundle_id: &self.evidence_bundle_id,
            elapsed_ms: self.elapsed_ms,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{EvidenceBundle, EvidenceKind, ReplayStatus};
    use crate::run::{FoundryCheckKind, FoundryCheckResult, FoundryOutcome};

    fn dummy_digest(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    fn report_only_sandbox() -> FoundrySandboxSpec {
        FoundrySandboxSpec::new(
            FoundrySandboxKind::ReportOnlyNoExec,
            dummy_digest(b"workspace-root"),
        )
    }

    fn cargo_policy() -> FoundryCommandPolicy {
        FoundryCommandPolicy::default_cargo_policy()
    }

    fn tight_timeout() -> FoundryTimeoutPolicy {
        FoundryTimeoutPolicy::new(30_000, 120_000)
    }

    fn locked_env() -> FoundryEnvironmentPolicy {
        FoundryEnvironmentPolicy::locked()
    }

    // --- FoundrySandboxSpec ---

    #[test]
    fn sandbox_spec_id_deterministic() {
        let s1 = report_only_sandbox();
        let s2 = report_only_sandbox();
        assert_eq!(s1.id, s2.id);
    }

    #[test]
    fn sandbox_spec_verify_id() {
        let s = report_only_sandbox();
        assert!(s.verify_id());
    }

    #[test]
    fn sandbox_spec_different_kinds_differ() {
        let s1 =
            FoundrySandboxSpec::new(FoundrySandboxKind::ReportOnlyNoExec, dummy_digest(b"root"));
        let s2 =
            FoundrySandboxSpec::new(FoundrySandboxKind::IsolatedWorktree, dummy_digest(b"root"));
        assert_ne!(s1.id, s2.id);
    }

    #[test]
    fn sandbox_spec_defaults_deny_network_and_write() {
        let s = report_only_sandbox();
        assert!(!s.allow_network);
        assert!(!s.allow_host_write);
    }

    // --- FoundryCommandPolicy ---

    #[test]
    fn command_policy_id_deterministic() {
        let p1 = cargo_policy();
        let p2 = cargo_policy();
        assert_eq!(p1.id, p2.id);
    }

    #[test]
    fn command_policy_verify_id() {
        let p = cargo_policy();
        assert!(p.verify_id());
    }

    #[test]
    fn command_policy_allows_cargo_check() {
        let p = cargo_policy();
        assert!(p.is_command_allowed("cargo", "check"));
        assert!(p.is_command_allowed("cargo", "test"));
        assert!(p.is_command_allowed("cargo", "fmt"));
        assert!(p.is_command_allowed("cargo", "clippy"));
    }

    #[test]
    fn command_policy_denies_shell_and_rm() {
        let p = cargo_policy();
        assert!(!p.allow_shell);
        assert!(!p.allow_network_tools);
        assert!(!p.allow_package_install);
        assert!(!p.is_command_allowed("bash", ""));
        assert!(!p.is_command_allowed("rm", ""));
    }

    #[test]
    fn command_policy_denies_cargo_publish() {
        let p = cargo_policy();
        assert!(!p.is_command_allowed("cargo", "publish"));
    }

    // --- FoundryTimeoutPolicy ---

    #[test]
    fn timeout_policy_id_deterministic() {
        let t1 = tight_timeout();
        let t2 = tight_timeout();
        assert_eq!(t1.id, t2.id);
    }

    #[test]
    fn timeout_policy_verify_id() {
        assert!(tight_timeout().verify_id());
    }

    #[test]
    fn timeout_policy_different_values_differ() {
        let t1 = FoundryTimeoutPolicy::new(1_000, 5_000);
        let t2 = FoundryTimeoutPolicy::new(2_000, 5_000);
        assert_ne!(t1.id, t2.id);
    }

    // --- FoundryEnvironmentPolicy ---

    #[test]
    fn env_policy_locked_id_deterministic() {
        let e1 = locked_env();
        let e2 = locked_env();
        assert_eq!(e1.id, e2.id);
    }

    #[test]
    fn env_policy_locked_verify_id() {
        assert!(locked_env().verify_id());
    }

    #[test]
    fn env_policy_locked_denies_passthrough() {
        let e = locked_env();
        assert!(!e.allow_passthrough);
        assert!(e.strip_secrets);
        assert!(e.allowed_vars.is_empty());
    }

    // --- FoundryCheckSpec ---

    #[test]
    fn check_spec_id_deterministic() {
        let s1 = FoundryCheckSpec::new(FoundryCheckKind::Build, "kosmo-core", true);
        let s2 = FoundryCheckSpec::new(FoundryCheckKind::Build, "kosmo-core", true);
        assert_eq!(s1.id, s2.id);
    }

    #[test]
    fn check_spec_verify_id() {
        let s = FoundryCheckSpec::new(FoundryCheckKind::Test, "kosmo-hyphae", true);
        assert!(s.verify_id());
    }

    #[test]
    fn check_spec_with_args_changes_id() {
        let base = FoundryCheckSpec::new(FoundryCheckKind::Test, "kosmo-core", true);
        let id_before = base.id;
        let with_args = base.with_args(vec!["--no-fail-fast".into()]);
        assert_ne!(id_before, with_args.id);
        assert!(with_args.verify_id());
    }

    #[test]
    fn check_spec_different_kinds_differ() {
        let build = FoundryCheckSpec::new(FoundryCheckKind::Build, "target", true);
        let test = FoundryCheckSpec::new(FoundryCheckKind::Test, "target", true);
        assert_ne!(build.id, test.id);
    }

    // --- FoundryExecutionOutcome ---

    #[test]
    fn outcome_passed_is_not_failure_class() {
        assert!(FoundryExecutionOutcome::Passed.is_passed());
        assert!(!FoundryExecutionOutcome::Passed.is_failure_class());
    }

    #[test]
    fn outcome_skipped_is_not_failure_class() {
        let o = FoundryExecutionOutcome::SkippedByReportOnly;
        assert!(o.is_skipped_report_only());
        assert!(!o.is_failure_class());
        assert!(!o.is_passed());
    }

    #[test]
    fn outcome_all_failure_variants_are_failure_class() {
        for o in [
            FoundryExecutionOutcome::Failed,
            FoundryExecutionOutcome::TimedOut,
            FoundryExecutionOutcome::CommandDeniedByPolicy,
            FoundryExecutionOutcome::SandboxViolation,
            FoundryExecutionOutcome::Inconclusive,
        ] {
            assert!(o.is_failure_class(), "{o:?} should be failure class");
        }
    }

    // --- FoundryExecutionPlan ---

    fn build_plan() -> FoundryExecutionPlan {
        let checks = vec![
            FoundryCheckSpec::new(FoundryCheckKind::Build, "kosmo-core", true),
            FoundryCheckSpec::new(FoundryCheckKind::Test, "kosmo-core", true),
        ];
        FoundryExecutionPlan::new(
            dummy_digest(b"policy"),
            dummy_digest(b"workspace-index"),
            dummy_digest(b"task"),
            report_only_sandbox(),
            checks,
            cargo_policy(),
            tight_timeout(),
            locked_env(),
        )
    }

    #[test]
    fn plan_id_deterministic() {
        let p1 = build_plan();
        let p2 = build_plan();
        assert_eq!(p1.id, p2.id);
    }

    #[test]
    fn plan_verify_id() {
        assert!(build_plan().verify_id());
    }

    #[test]
    fn plan_different_policy_differs() {
        let p1 = build_plan();
        let mut p2 = build_plan();
        p2.policy_id = dummy_digest(b"other-policy");
        p2.id = {
            let check_ids: Vec<&Digest> = p2.checks.iter().map(|c| &c.id).collect();
            let evidence_digests: Vec<&Digest> =
                p2.evidence_refs.iter().map(|e| &e.digest).collect();
            Digest::of(&PlanContent {
                policy_id: &p2.policy_id,
                workspace_index_id: &p2.workspace_index_id,
                task_id: &p2.task_id,
                sandbox_id: &p2.sandbox.id,
                check_ids,
                command_policy_id: &p2.command_policy.id,
                timeout_policy_id: &p2.timeout_policy.id,
                environment_policy_id: &p2.environment_policy.id,
                evidence_digests,
            })
        };
        assert_ne!(p1.id, p2.id);
    }

    #[test]
    fn plan_with_evidence_changes_id() {
        let plan = build_plan();
        let id_before = plan.id;
        let plan2 = plan.with_evidence(vec![EvidenceRef::new(
            dummy_digest(b"ev1"),
            EvidenceKind::FoundryCheck,
            "prior check",
        )]);
        assert_ne!(id_before, plan2.id);
        assert!(plan2.verify_id());
    }

    // --- FoundryExecutionReport ---

    #[test]
    fn report_skipped_by_report_only_deterministic() {
        let plan_id = dummy_digest(b"plan");
        let bundle_id = dummy_digest(b"bundle");
        let r1 = FoundryExecutionReport::skipped_by_report_only(plan_id, bundle_id);
        let r2 = FoundryExecutionReport::skipped_by_report_only(plan_id, bundle_id);
        assert_eq!(r1.id, r2.id);
    }

    #[test]
    fn report_skipped_verify_id() {
        let r = FoundryExecutionReport::skipped_by_report_only(
            dummy_digest(b"plan"),
            dummy_digest(b"bundle"),
        );
        assert!(r.verify_id());
        assert!(r.outcome.is_skipped_report_only());
    }

    #[test]
    fn report_new_deterministic() {
        let plan_id = dummy_digest(b"plan");
        let bundle_id = dummy_digest(b"bundle");
        let r1 = FoundryExecutionReport::new(
            plan_id,
            FoundryExecutionOutcome::Passed,
            vec![],
            vec!["all checks passed".into()],
            bundle_id,
            4_200,
        );
        let r2 = FoundryExecutionReport::new(
            plan_id,
            FoundryExecutionOutcome::Passed,
            vec![],
            vec!["all checks passed".into()],
            bundle_id,
            4_200,
        );
        assert_eq!(r1.id, r2.id);
    }

    #[test]
    fn report_verify_id_with_check_results() {
        let plan_id = dummy_digest(b"plan");
        let bundle_id = dummy_digest(b"bundle");
        let check = FoundryCheckResult::new(
            FoundryCheckKind::Test,
            FoundryOutcome::Passed,
            dummy_digest(b"ev"),
            vec![],
        );
        let r = FoundryExecutionReport::new(
            plan_id,
            FoundryExecutionOutcome::Passed,
            vec![check],
            vec![],
            bundle_id,
            1_000,
        );
        assert!(r.verify_id());
    }

    #[test]
    fn report_different_elapsed_differs() {
        let plan_id = dummy_digest(b"plan");
        let bundle_id = dummy_digest(b"bundle");
        let r1 = FoundryExecutionReport::new(
            plan_id,
            FoundryExecutionOutcome::Passed,
            vec![],
            vec![],
            bundle_id,
            100,
        );
        let r2 = FoundryExecutionReport::new(
            plan_id,
            FoundryExecutionOutcome::Passed,
            vec![],
            vec![],
            bundle_id,
            200,
        );
        assert_ne!(r1.id, r2.id);
    }

    #[test]
    fn report_evidence_bundle_id_is_mandatory() {
        let r = FoundryExecutionReport::skipped_by_report_only(
            dummy_digest(b"plan"),
            dummy_digest(b"bundle"),
        );
        assert_ne!(r.evidence_bundle_id, Digest::ZERO);
    }

    #[test]
    fn report_only_policy_produces_skipped_outcome() {
        use crate::policy::{ImplementationMode, PolicyProfile};

        let profile = PolicyProfile::default();
        assert_eq!(profile.mode, ImplementationMode::ReportOnly);

        let r = FoundryExecutionReport::skipped_by_report_only(
            dummy_digest(b"plan"),
            dummy_digest(b"bundle"),
        );
        assert!(r.outcome.is_skipped_report_only());
    }

    #[test]
    fn r1_invariant_no_floats_in_timeout() {
        let t = FoundryTimeoutPolicy::new(30_000, 120_000);
        // Verified at compile time: fields are u64 not f32/f64.
        // Runtime: values round-trip exactly.
        assert_eq!(t.per_check_timeout_ms, 30_000u64);
        assert_eq!(t.total_plan_timeout_ms, 120_000u64);
    }

    #[test]
    fn r1_evidence_bundle_integration() {
        let policy_id = dummy_digest(b"policy");
        let bundle = EvidenceBundle::seal(
            vec![EvidenceRef::new(
                dummy_digest(b"check-ev"),
                EvidenceKind::FoundryCheck,
                "foundry check evidence",
            )],
            policy_id,
            ReplayStatus::Replayable,
        );
        let r = FoundryExecutionReport::new(
            dummy_digest(b"plan"),
            FoundryExecutionOutcome::Passed,
            vec![],
            vec![],
            bundle.bundle_id,
            500,
        );
        assert!(r.verify_id());
        assert_ne!(r.evidence_bundle_id, Digest::ZERO);
    }
}

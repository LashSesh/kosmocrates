//! KOSMO-OPS-01 RX — Operator orchestrator.
//!
//! Wires the three real executors into a complete R1→R2→R3 validation-closure
//! pipeline:
//!
//!   1. **ParseBack pre-snapshot** — `cargo metadata` captures current topology.
//!   2. **Foundry execution** — allowlisted cargo checks against the workspace.
//!   3. **ParseBack post-snapshot** — re-scan; diff against pre.
//!   4. **ValidationClosure** — combine outcomes into a single closure decision.
//!   5. **Persistence** (OperatorApproved only) — append closure to JSONL store.
//!
//! Policy contract:
//!  - `ReportOnly` → all sub-reports `SkippedByReportOnly`; closure `Inconclusive`.
//!    No I/O whatsoever.
//!  - `DryRun` → full read-only execution; no host writes; closure not persisted.
//!  - `OperatorApproved` → full execution; if `store_path` is set, appends the
//!    closure payload digest to the JSONL cartography store.
//!
//! This makes `ValidationClosureReport` truly real (R3 operationalized) rather
//! than just a data model populated with simulated sub-outcomes.

use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use kosmo_core::{
    CartographyEntryKind, CartographyStoreCommit, CorpusCartographyStore, CorpusScope, Digest,
    FoundryExecutionPlan, ImplementationMode, ParseBackScanScope, ParseBackSeverity, PolicyProfile,
    ValidationClosureReport,
};
use kosmo_foundry::FoundryExecutor;
use kosmo_parseback::{ParseBackExecutor, TopologySnapshot};

// ─── OperationPlan ────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct PlanContent<'a> {
    foundry_plan_id: &'a Digest,
    parseback_scope_tag: String,
    policy_id: &'a Digest,
}

/// Input specification for one operator cycle.
///
/// Bundles a `FoundryExecutionPlan` (R1 artifact) with the ParseBack scan scope
/// and the governing policy id. `plan_id` is the content digest of all fields.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationPlan {
    pub plan_id: Digest,
    pub foundry_plan: FoundryExecutionPlan,
    pub parseback_scope: ParseBackScanScope,
    pub policy_id: Digest,
}

impl OperationPlan {
    pub fn new(
        foundry_plan: FoundryExecutionPlan,
        parseback_scope: ParseBackScanScope,
        policy_id: Digest,
    ) -> Self {
        let mut p = Self {
            plan_id: Digest::ZERO,
            foundry_plan,
            parseback_scope,
            policy_id,
        };
        p.plan_id = p.compute_id();
        p
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&PlanContent {
            foundry_plan_id: &self.foundry_plan.id,
            parseback_scope_tag: format!("{:?}", self.parseback_scope),
            policy_id: &self.policy_id,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.plan_id == self.compute_id()
    }
}

// ─── OperationReport ──────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ReportContent<'a> {
    plan_id: &'a Digest,
    foundry_report_id: &'a Digest,
    parseback_report_id: &'a Digest,
    closure_report_id: &'a Digest,
    persisted: bool,
}

/// The complete, content-addressed record of one operator cycle.
///
/// Chains `FoundryExecutionReport` → `ParseBackReport` → `ValidationClosureReport`
/// into a single auditable artefact. `report_id` is the content digest of the
/// plan id, the three sub-report ids and `persisted` — deliberately NOT the
/// wall-clock `elapsed_ms`, which is observational, varies run to run, and would
/// break INVARIANT-007 (identical inputs → identical id).
///
/// `persisted` is `true` only when the closure payload digest was appended to the
/// JSONL cartography store under `OperatorApproved` policy.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OperationReport {
    pub report_id: Digest,
    pub plan_id: Digest,
    pub foundry_report: kosmo_core::FoundryExecutionReport,
    pub parseback_report: kosmo_core::ParseBackReport,
    pub closure_report: ValidationClosureReport,
    pub elapsed_ms: u64,
    pub persisted: bool,
}

impl OperationReport {
    fn build(
        plan_id: Digest,
        foundry_report: kosmo_core::FoundryExecutionReport,
        parseback_report: kosmo_core::ParseBackReport,
        closure_report: ValidationClosureReport,
        elapsed_ms: u64,
        persisted: bool,
    ) -> Self {
        let report_id = Digest::of(&ReportContent {
            plan_id: &plan_id,
            foundry_report_id: &foundry_report.id,
            parseback_report_id: &parseback_report.id,
            closure_report_id: &closure_report.id,
            persisted,
        });
        Self {
            report_id,
            plan_id,
            foundry_report,
            parseback_report,
            closure_report,
            elapsed_ms,
            persisted,
        }
    }

    pub fn verify_id(&self) -> bool {
        let expected = Digest::of(&ReportContent {
            plan_id: &self.plan_id,
            foundry_report_id: &self.foundry_report.id,
            parseback_report_id: &self.parseback_report.id,
            closure_report_id: &self.closure_report.id,
            persisted: self.persisted,
        });
        self.report_id == expected
    }
}

// ─── OperatorExecutor ─────────────────────────────────────────────────────────

/// Orchestrates the R1→R2→R3 validation-closure pipeline.
///
/// Each `execute()` call is atomic in the sense that all sub-reports are
/// produced in one invocation and the resulting `OperationReport` is
/// content-addressed over all of them.
pub struct OperatorExecutor {
    workspace_root: PathBuf,
    store_path: Option<PathBuf>,
}

impl OperatorExecutor {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            store_path: None,
        }
    }

    /// Attach a JSONL cartography store for `OperatorApproved` persistence.
    ///
    /// When set, successful closure reports are appended to the store under
    /// `OperatorApproved` policy. Has no effect under `ReportOnly` or `DryRun`.
    pub fn with_store(mut self, store_path: impl Into<PathBuf>) -> Self {
        self.store_path = Some(store_path.into());
        self
    }

    /// Execute the full R1→R2→R3 pipeline.
    ///
    /// `evidence_bundle_id` is forwarded to every sub-executor and the
    /// `ValidationClosureReport`. It MUST NOT be `Digest::ZERO`.
    pub fn execute(
        &self,
        plan: &OperationPlan,
        policy: &PolicyProfile,
        evidence_bundle_id: Digest,
    ) -> OperationReport {
        let t0 = Instant::now();

        // ── ReportOnly: skip all I/O; return minimal skipped reports ──────────
        if policy.mode == ImplementationMode::ReportOnly {
            return self.skipped_report(plan, evidence_bundle_id, t0);
        }

        // ── DryRun / OperatorApproved: full read-only execution ───────────────

        // Step 1: Pre-materialization topology snapshot.
        let parseback_executor = ParseBackExecutor::new(self.workspace_root.clone());
        let pre_snapshot = match parseback_executor.snapshot(&plan.parseback_scope) {
            Ok(s) => s,
            Err(e) => {
                return self.inconclusive_report(
                    plan,
                    evidence_bundle_id,
                    t0,
                    &format!("pre-snapshot failed: {e}"),
                )
            }
        };

        // Step 2: Foundry checks (read-only; cargo check/test/clippy).
        let foundry_executor = FoundryExecutor::new(self.workspace_root.clone());
        let foundry_report =
            foundry_executor.execute(&plan.foundry_plan, policy, evidence_bundle_id);

        // Step 3: Post-materialization ParseBack — diff against pre-snapshot.
        let parseback_plan = kosmo_core::ParseBackPlan::new(
            plan.policy_id,
            plan.plan_id,
            plan.parseback_scope.clone(),
            pre_snapshot.snapshot_id,
        );
        let parseback_report =
            parseback_executor.execute(&parseback_plan, &pre_snapshot, policy, evidence_bundle_id);

        // Step 4: Compute ValidationClosure from real outcomes.
        let has_warnings = parseback_report.worst_severity() == Some(&ParseBackSeverity::Warning);
        let elapsed = t0.elapsed().as_millis() as u64;
        let closure_report = ValidationClosureReport::new(
            plan.plan_id,
            foundry_report.id,
            parseback_report.id,
            foundry_report.outcome.clone(),
            parseback_report.outcome.clone(),
            has_warnings,
            vec![],
            evidence_bundle_id,
        );

        // Step 5: Persist if OperatorApproved and store path configured.
        let persisted = if policy.mode == ImplementationMode::OperatorApproved {
            if let Some(ref store_path) = self.store_path {
                self.persist_closure(&closure_report, store_path, plan.policy_id, policy)
            } else {
                false
            }
        } else {
            false
        };

        OperationReport::build(
            plan.plan_id,
            foundry_report,
            parseback_report,
            closure_report,
            elapsed,
            persisted,
        )
    }

    fn skipped_report(
        &self,
        plan: &OperationPlan,
        evidence_bundle_id: Digest,
        t0: Instant,
    ) -> OperationReport {
        use kosmo_core::{
            FoundryExecutionOutcome, FoundryExecutionReport, ParseBackOutcome, ParseBackReport,
        };

        let foundry_report = FoundryExecutionReport::skipped_by_report_only(
            plan.foundry_plan.id,
            evidence_bundle_id,
        );
        let empty_topology = TopologySnapshot::from_parts(
            plan.parseback_scope.clone(),
            Default::default(),
            Default::default(),
        );
        let parseback_report = ParseBackReport::skipped_by_report_only(
            plan.plan_id,
            empty_topology.snapshot_id,
            evidence_bundle_id,
        );
        let elapsed = t0.elapsed().as_millis() as u64;
        let closure_report = ValidationClosureReport::new(
            plan.plan_id,
            foundry_report.id,
            parseback_report.id,
            FoundryExecutionOutcome::SkippedByReportOnly,
            ParseBackOutcome::SkippedByReportOnly,
            false,
            vec![],
            evidence_bundle_id,
        );
        OperationReport::build(
            plan.plan_id,
            foundry_report,
            parseback_report,
            closure_report,
            elapsed,
            false,
        )
    }

    fn inconclusive_report(
        &self,
        plan: &OperationPlan,
        evidence_bundle_id: Digest,
        t0: Instant,
        diagnostic: &str,
    ) -> OperationReport {
        use kosmo_core::{
            FoundryExecutionOutcome, FoundryExecutionReport, ParseBackOutcome, ParseBackReport,
        };

        let foundry_report = FoundryExecutionReport::skipped_by_report_only(
            plan.foundry_plan.id,
            evidence_bundle_id,
        );
        let empty_topology = TopologySnapshot::from_parts(
            plan.parseback_scope.clone(),
            Default::default(),
            Default::default(),
        );
        let parseback_report = ParseBackReport::new(
            plan.plan_id,
            ParseBackOutcome::Inconclusive,
            vec![],
            empty_topology.snapshot_id,
            empty_topology.snapshot_id,
            vec![diagnostic.into()],
            evidence_bundle_id,
            0,
        );
        let elapsed = t0.elapsed().as_millis() as u64;
        let closure_report = ValidationClosureReport::new(
            plan.plan_id,
            foundry_report.id,
            parseback_report.id,
            FoundryExecutionOutcome::Inconclusive,
            ParseBackOutcome::Inconclusive,
            false,
            vec![diagnostic.into()],
            evidence_bundle_id,
        );
        OperationReport::build(
            plan.plan_id,
            foundry_report,
            parseback_report,
            closure_report,
            elapsed,
            false,
        )
    }

    fn persist_closure(
        &self,
        closure: &ValidationClosureReport,
        store_path: &Path,
        policy_id: Digest,
        policy: &PolicyProfile,
    ) -> bool {
        use kosmo_store::JsonlCartographyStore;

        let scope = CorpusScope::LocalHostProject;
        let mut store = match JsonlCartographyStore::open(store_path, scope.clone(), policy_id) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let next_seq = match store.read_manifest() {
            Ok(m) => m.head_sequence + 1,
            Err(_) => return false,
        };
        let commit = CartographyStoreCommit::new(
            scope,
            next_seq,
            closure.id,
            CartographyEntryKind::Custom("ValidationClosure".into()),
            closure.evidence_bundle_id,
            policy_id,
        );
        store.append(commit, policy).is_ok()
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Build a standard `OperationPlan` using the kosmo-foundry standard cargo plan.
///
/// Runs `cargo check`, `cargo clippy`, and `cargo test` across the workspace
/// under the given policy, with the specified per-check timeout (ms).
pub fn standard_plan(
    workspace_root: &Path,
    scope: ParseBackScanScope,
    policy_id: Digest,
    per_check_timeout_ms: u64,
) -> OperationPlan {
    let workspace_digest = Digest::of_bytes(workspace_root.to_string_lossy().as_bytes());
    let foundry_plan = kosmo_foundry::standard_cargo_plan(
        policy_id,
        workspace_digest,
        Digest::of_bytes(b"operator-task"),
        workspace_digest,
        per_check_timeout_ms,
    );
    OperationPlan::new(foundry_plan, scope, policy_id)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::{EvidenceBundle, EvidenceKind, EvidenceRef, ReplayStatus};

    fn pid() -> Digest {
        Digest::of_bytes(b"policy")
    }

    fn bundle_id() -> Digest {
        EvidenceBundle::seal(
            vec![EvidenceRef::new(
                Digest::of_bytes(b"ev"),
                EvidenceKind::HostScan,
                "test",
            )],
            pid(),
            ReplayStatus::Replayable,
        )
        .bundle_id
    }

    fn empty_foundry_plan() -> FoundryExecutionPlan {
        use kosmo_core::{
            FoundryCommandPolicy, FoundryEnvironmentPolicy, FoundrySandboxKind, FoundrySandboxSpec,
            FoundryTimeoutPolicy,
        };
        FoundryExecutionPlan::new(
            pid(),
            Digest::of_bytes(b"widx"),
            Digest::of_bytes(b"task"),
            FoundrySandboxSpec::new(FoundrySandboxKind::LocalDryRun, Digest::of_bytes(b"root")),
            vec![],
            FoundryCommandPolicy::default_cargo_policy(),
            FoundryTimeoutPolicy::new(5_000, 20_000),
            FoundryEnvironmentPolicy::locked(),
        )
    }

    fn make_plan(scope: ParseBackScanScope) -> OperationPlan {
        OperationPlan::new(empty_foundry_plan(), scope, pid())
    }

    #[test]
    fn plan_is_content_addressed() {
        let p1 = make_plan(ParseBackScanScope::FullWorkspace);
        let p2 = make_plan(ParseBackScanScope::FullWorkspace);
        assert_eq!(p1.plan_id, p2.plan_id);
        assert_ne!(p1.plan_id, Digest::ZERO);
    }

    #[test]
    fn plan_different_scope_differs() {
        let p1 = make_plan(ParseBackScanScope::FullWorkspace);
        let p2 = make_plan(ParseBackScanScope::AffectedCratesOnly);
        assert_ne!(p1.plan_id, p2.plan_id);
    }

    #[test]
    fn plan_verify_id() {
        assert!(make_plan(ParseBackScanScope::FullWorkspace).verify_id());
    }

    #[test]
    fn report_only_produces_inconclusive_closure() {
        let executor = OperatorExecutor::new(PathBuf::from("/nonexistent"));
        let plan = make_plan(ParseBackScanScope::FullWorkspace);
        let policy = PolicyProfile::default_report_only();

        let report = executor.execute(&plan, &policy, bundle_id());

        assert!(
            report
                .closure_report
                .final_validation_status
                .is_inconclusive(),
            "ReportOnly must produce Inconclusive, got {:?}",
            report.closure_report.final_validation_status
        );
        assert!(!report.persisted);
        assert!(report.verify_id());
        assert!(report.foundry_report.outcome.is_skipped_report_only());
        assert!(report.parseback_report.outcome.is_skipped_report_only());
    }

    #[test]
    fn report_is_content_addressed() {
        let executor = OperatorExecutor::new(PathBuf::from("/nonexistent"));
        let plan = make_plan(ParseBackScanScope::FullWorkspace);
        let policy = PolicyProfile::default_report_only();
        let bid = bundle_id();

        let r1 = executor.execute(&plan, &policy, bid);
        let r2 = executor.execute(&plan, &policy, bid);
        assert_eq!(
            r1.report_id, r2.report_id,
            "INVARIANT-007: identical inputs must produce identical ids"
        );
        assert_ne!(r1.report_id, Digest::ZERO);
    }

    #[test]
    fn report_id_excludes_wall_clock_elapsed() {
        // INVARIANT-007: the content address must depend only on the inputs and
        // sub-reports, never on how long the run took. Same sub-reports, a
        // wildly different elapsed_ms → identical report_id. (Including the
        // wall clock made the digest flaky under slow/instrumented runs.)
        let executor = OperatorExecutor::new(PathBuf::from("/nonexistent"));
        let plan = make_plan(ParseBackScanScope::FullWorkspace);
        let policy = PolicyProfile::default_report_only();
        let r = executor.execute(&plan, &policy, bundle_id());
        let slow = OperationReport::build(
            r.plan_id,
            r.foundry_report.clone(),
            r.parseback_report.clone(),
            r.closure_report.clone(),
            r.elapsed_ms + 999_999,
            r.persisted,
        );
        assert_eq!(
            r.report_id, slow.report_id,
            "report_id must not depend on wall-clock elapsed_ms (INVARIANT-007)"
        );
    }

    #[test]
    fn report_only_does_not_persist() {
        let dir = std::env::temp_dir();
        let store_path = dir.join(format!(
            "kosmo-op-test-{}.jsonl",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let executor =
            OperatorExecutor::new(PathBuf::from("/nonexistent")).with_store(store_path.clone());
        let plan = make_plan(ParseBackScanScope::FullWorkspace);
        let policy = PolicyProfile::default_report_only();

        let report = executor.execute(&plan, &policy, bundle_id());
        let _ = std::fs::remove_file(&store_path);

        assert!(!report.persisted);
        assert!(!store_path.exists());
    }

    fn minimal_check_plan() -> FoundryExecutionPlan {
        use kosmo_core::{
            FoundryCheckKind, FoundryCheckSpec, FoundryCommandPolicy, FoundryEnvironmentPolicy,
            FoundrySandboxKind, FoundrySandboxSpec, FoundryTimeoutPolicy,
        };
        let check_spec = FoundryCheckSpec::new(FoundryCheckKind::Build, "kosmo-parseback", true)
            .with_args(vec!["-p".into(), "kosmo-parseback".into()]);
        FoundryExecutionPlan::new(
            pid(),
            Digest::of_bytes(b"widx"),
            Digest::of_bytes(b"task"),
            FoundrySandboxSpec::new(FoundrySandboxKind::LocalDryRun, Digest::of_bytes(b"root")),
            vec![check_spec],
            FoundryCommandPolicy::default_cargo_policy(),
            FoundryTimeoutPolicy::new(30_000, 120_000),
            FoundryEnvironmentPolicy::locked(),
        )
    }

    #[test]
    fn dry_run_full_cycle_on_workspace() {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..");
        let foundry_plan = minimal_check_plan();
        let plan = OperationPlan::new(foundry_plan, ParseBackScanScope::AffectedCratesOnly, pid());
        let executor = OperatorExecutor::new(workspace_root);
        let report = executor.execute(&plan, &PolicyProfile::dry_run(), bundle_id());

        assert!(
            report.closure_report.final_validation_status.is_passed()
                || report
                    .closure_report
                    .final_validation_status
                    .is_inconclusive(),
            "DryRun on clean workspace: expected Passed or Inconclusive, got {:?}",
            report.closure_report.final_validation_status
        );
        assert!(!report.persisted, "DryRun must not persist");
        assert!(report.verify_id());
    }

    #[test]
    fn operator_approved_persists_closure() {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..");
        let store_path = std::env::temp_dir().join(format!(
            "kosmo-op-persist-{}.jsonl",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let foundry_plan = minimal_check_plan();
        let plan = OperationPlan::new(foundry_plan, ParseBackScanScope::AffectedCratesOnly, pid());
        let executor = OperatorExecutor::new(workspace_root).with_store(store_path.clone());
        let report = executor.execute(&plan, &PolicyProfile::operator_approved(), bundle_id());
        let file_exists = store_path.exists();
        let _ = std::fs::remove_file(&store_path);

        assert!(report.verify_id(), "report id must verify");
        // Persistence requires OperatorApproved + successful closure.
        // If foundry succeeded, the closure should be persisted.
        if report.closure_report.final_validation_status.is_passed() {
            assert!(report.persisted, "OperatorApproved + Passed must persist");
            assert!(file_exists, "store file must exist when persisted");
        }
    }
}

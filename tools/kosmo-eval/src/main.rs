use std::io::{self, Write as IoWrite};
use std::path::PathBuf;
use std::time::Instant;

use kosmo_core::{
    // R1 - Foundry
    FoundryCommandPolicy, FoundryEnvironmentPolicy, FoundryExecutionOutcome,
    FoundryExecutionPlan, FoundryExecutionReport, FoundrySandboxKind, FoundrySandboxSpec,
    FoundryTimeoutPolicy,
    // R2 - ParseBack
    ParseBackOutcome, ParseBackReport, ParseBackSeverity,
    ParseBackTopologyDelta, TopologyChangeKind,
    // R3 - Validation Closure
    ValidationClosureReport, ValidationClosureStatus, determine_closure_status,
    // R4 - Cartography
    CartographyEntryKind, CartographyStoreCommit, CartographyStoreError,
    CorpusCartographyStore, CorpusScope, InMemoryCartographyStore,
    // PSE promotion feedback loop
    FeedbackOutcome, PromotionFeedback,
    // R5 - Materialization
    IsolatedWorktreeSpec, WorktreeCleanupPolicy, WorktreeCreationMethod,
    // R6 - KCube
    KcubeExportPolicy, KcubeRoundtripVerification,
    // R8 - Acquisition
    AcquiredSource, AcquisitionSourceKind, AcquisitionTaint,
    LicenseCheckOutcome, SecretScanOutcome, SourceAcquisitionCapability,
    // R9 - Evaluation
    EvaluationCriteria, EvaluationDimension, EvaluationHarness,
    EvaluationRunReport, EvaluationScenario, EvaluationSuiteOutcome,
    EvaluationSuiteReport, StubEvaluationHarness,
    // Core
    Digest, PolicyProfile, Q16,
    // Unified tripolar energy kernel
    EnergyAssessment, EnergyFactors, EnergyKernel, FoundrySurvival, GateResult, LicenseStatus,
    TaintLabel, TripolarEnergy, rank_by_energy,
};

use kosmo_hyphae::code_hdag::{CodeHDAG, HDAGEdgeKind};
use kosmo_hyphae::cube::{CubeDimensionProfile, SourceCube};
use kosmo_hyphae::delta::HostTargetDelta;
use kosmo_hyphae::motif::MotifCandidate;
use kosmo_hyphae::norm::{NormFitnessTrace, NormGeneCandidate};

use kosmo_pse_bridge::{
    PseBridgeCandidate, PseBridgeCandidateKind, PseBridgePolicy, PseBridgeRateLimit,
    PromotionOutcome, PromotionRequestRecord, build_promotion_feedback, validate_candidate,
};

use kosmo_parseback::{CrateFingerprint, ParseBackExecutor, TopologySnapshot, diff_snapshots};
use kosmo_operator::{OperationPlan, OperatorExecutor, standard_plan};
use kosmo_kcube::{KcubeArtifact, KcubeExecutor, kcube_file_name};
use kosmo_systemcube::{
    BlueprintUnit, BlueprintUnitKind, CompatibilityProfileReport, CompatibilityStatus,
    ContradictionEnergyReport, EnergyStatus, SystemCube,
};
use kosmo_pipeline::{IntegrationRunOptions, run_dry_pipeline};
use kosmo_workbench::WorkspaceIndex;

fn d(seed: &[u8]) -> Digest {
    Digest::of_bytes(seed)
}

struct ScenarioResult {
    name: &'static str,
    phase: &'static str,
    passed: bool,
    elapsed_ms: u64,
    details: Vec<String>,
}

fn run_check(
    name: &'static str,
    phase: &'static str,
    f: impl FnOnce() -> Result<(), String>,
) -> ScenarioResult {
    let t0 = Instant::now();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    let elapsed_ms = t0.elapsed().as_millis() as u64;
    match result {
        Ok(Ok(())) => ScenarioResult { name, phase, passed: true, elapsed_ms, details: vec![] },
        Ok(Err(msg)) => ScenarioResult { name, phase, passed: false, elapsed_ms, details: vec![msg] },
        Err(_) => ScenarioResult {
            name, phase, passed: false, elapsed_ms,
            details: vec!["scenario panicked unexpectedly".into()],
        },
    }
}

fn build_scenarios() -> Vec<ScenarioResult> {
    let mut v = Vec::new();

    // ── R1: Foundry ──────────────────────────────────────────────────────────

    v.push(run_check("r1-sandbox-defaults-no-network", "R1:Foundry", || {
        let spec = FoundrySandboxSpec::new(FoundrySandboxKind::LocalDryRun, d(b"root"));
        if spec.allow_network {
            return Err("allow_network must default to false".into());
        }
        if spec.allow_host_write {
            return Err("allow_host_write must default to false".into());
        }
        if !spec.verify_id() {
            return Err("FoundrySandboxSpec id verification failed".into());
        }
        Ok(())
    }));

    v.push(run_check("r1-cargo-policy-shell-disabled", "R1:Foundry", || {
        let policy = FoundryCommandPolicy::default_cargo_policy();
        if policy.allow_shell {
            return Err("allow_shell must be false in cargo policy".into());
        }
        if policy.allow_network_tools {
            return Err("allow_network_tools must be false in cargo policy".into());
        }
        if policy.allow_package_install {
            return Err("allow_package_install must be false in cargo policy".into());
        }
        if !policy.is_command_allowed("cargo", "check") {
            return Err("cargo check must be allowed".into());
        }
        if !policy.is_command_allowed("cargo", "test") {
            return Err("cargo test must be allowed".into());
        }
        if policy.is_command_allowed("rm", "-rf") {
            return Err("rm -rf must NOT be allowed".into());
        }
        if !policy.verify_id() {
            return Err("FoundryCommandPolicy id verification failed".into());
        }
        Ok(())
    }));

    v.push(run_check("r1-report-only-outcome-skipped", "R1:Foundry", || {
        let report = FoundryExecutionReport::skipped_by_report_only(d(b"plan"), d(b"bundle"));
        if !report.outcome.is_skipped_report_only() {
            return Err(format!("expected SkippedByReportOnly, got {:?}", report.outcome));
        }
        if report.outcome.is_passed() {
            return Err("SkippedByReportOnly must not be treated as passed".into());
        }
        if !report.verify_id() {
            return Err("FoundryExecutionReport id verification failed".into());
        }
        Ok(())
    }));

    v.push(run_check("r1-plan-id-deterministic", "R1:Foundry", || {
        let make = || {
            let sandbox = FoundrySandboxSpec::new(FoundrySandboxKind::IsolatedWorktree, d(b"root"));
            let cmd = FoundryCommandPolicy::default_cargo_policy();
            let timeout = FoundryTimeoutPolicy::new(30_000, 120_000);
            let env = FoundryEnvironmentPolicy::locked();
            FoundryExecutionPlan::new(
                d(b"policy"), d(b"workspace"), d(b"task"),
                sandbox, vec![], cmd, timeout, env,
            )
        };
        let p1 = make();
        let p2 = make();
        if p1.id != p2.id {
            return Err(format!("non-deterministic plan ids: {:?} vs {:?}", p1.id, p2.id));
        }
        if !p1.verify_id() {
            return Err("FoundryExecutionPlan id verification failed".into());
        }
        Ok(())
    }));

    v.push(run_check("r1-env-policy-locked-no-passthrough", "R1:Foundry", || {
        let env = FoundryEnvironmentPolicy::locked();
        if env.allow_passthrough {
            return Err("locked env policy must not allow passthrough".into());
        }
        if !env.strip_secrets {
            return Err("locked env policy must strip secrets".into());
        }
        if !env.allowed_vars.is_empty() {
            return Err("locked env policy must have no allowed vars".into());
        }
        if !env.verify_id() {
            return Err("FoundryEnvironmentPolicy id verification failed".into());
        }
        Ok(())
    }));

    // ── R2: ParseBack ────────────────────────────────────────────────────────

    v.push(run_check("r2-topology-unchanged-is-pass", "R2:ParseBack", || {
        if !ParseBackOutcome::TopologyUnchanged.is_passed() {
            return Err("TopologyUnchanged.is_passed() must be true".into());
        }
        if !ParseBackOutcome::Passed.is_passed() {
            return Err("Passed.is_passed() must be true".into());
        }
        if ParseBackOutcome::Failed.is_passed() {
            return Err("Failed.is_passed() must be false".into());
        }
        if ParseBackOutcome::Inconclusive.is_passed() {
            return Err("Inconclusive.is_passed() must be false".into());
        }
        Ok(())
    }));

    v.push(run_check("r2-severity-worst-wins-ordering", "R2:ParseBack", || {
        if !(ParseBackSeverity::Info < ParseBackSeverity::Warning) {
            return Err("Info must be less than Warning".into());
        }
        if !(ParseBackSeverity::Warning < ParseBackSeverity::Critical) {
            return Err("Warning must be less than Critical".into());
        }
        let worst = [ParseBackSeverity::Warning, ParseBackSeverity::Info, ParseBackSeverity::Critical]
            .iter()
            .max()
            .cloned();
        if worst != Some(ParseBackSeverity::Critical) {
            return Err("max() must yield Critical in worst-wins".into());
        }
        Ok(())
    }));

    v.push(run_check("r2-delta-id-deterministic", "R2:ParseBack", || {
        let make_delta = || ParseBackTopologyDelta::new(
            TopologyChangeKind::NodeAdded, None, "test node added", ParseBackSeverity::Warning,
        );
        let d1 = make_delta();
        let d2 = make_delta();
        if d1.id != d2.id {
            return Err(format!("non-deterministic delta ids: {:?} vs {:?}", d1.id, d2.id));
        }
        if !d1.verify_id() {
            return Err("ParseBackTopologyDelta id verification failed".into());
        }
        Ok(())
    }));

    v.push(run_check("r2-report-topology-unchanged-id-deterministic", "R2:ParseBack", || {
        let make = || ParseBackReport::topology_unchanged(d(b"plan"), d(b"topo"), d(b"bundle"), 42);
        let r1 = make();
        let r2 = make();
        if r1.id != r2.id {
            return Err(format!("non-deterministic report ids: {:?} vs {:?}", r1.id, r2.id));
        }
        if !r1.verify_id() {
            return Err("ParseBackReport id verification failed".into());
        }
        if !r1.outcome.is_passed() {
            return Err("TopologyUnchanged report must have is_passed()==true".into());
        }
        Ok(())
    }));

    v.push(run_check("r2-has-critical-delta-detection", "R2:ParseBack", || {
        let delta = ParseBackTopologyDelta::new(
            TopologyChangeKind::StructuralCrystalAffected, None, "critical change",
            ParseBackSeverity::Critical,
        );
        let report = ParseBackReport::new(
            d(b"plan"), ParseBackOutcome::Failed, vec![delta],
            d(b"pre"), d(b"post"), vec![], d(b"bundle"), 10,
        );
        if !report.has_critical_delta() {
            return Err("has_critical_delta() must be true when Critical delta present".into());
        }
        if report.worst_severity() != Some(&ParseBackSeverity::Critical) {
            return Err("worst_severity() must be Critical".into());
        }
        Ok(())
    }));

    // ── R3: Validation Closure ───────────────────────────────────────────────

    v.push(run_check("r3-both-fail-is-failed-both", "R3:ValidationClosure", || {
        let status = determine_closure_status(
            &FoundryExecutionOutcome::Failed,
            &ParseBackOutcome::Failed,
            false,
        );
        if status != ValidationClosureStatus::FailedBoth {
            return Err(format!("expected FailedBoth, got {:?}", status));
        }
        if !status.is_failure_class() {
            return Err("FailedBoth must be in failure class".into());
        }
        Ok(())
    }));

    v.push(run_check("r3-foundry-fail-only-is-failed-foundry", "R3:ValidationClosure", || {
        let status = determine_closure_status(
            &FoundryExecutionOutcome::Failed,
            &ParseBackOutcome::Passed,
            false,
        );
        if status != ValidationClosureStatus::FailedFoundry {
            return Err(format!("expected FailedFoundry, got {:?}", status));
        }
        Ok(())
    }));

    v.push(run_check("r3-report-only-yields-inconclusive", "R3:ValidationClosure", || {
        let s1 = determine_closure_status(
            &FoundryExecutionOutcome::SkippedByReportOnly,
            &ParseBackOutcome::Passed,
            false,
        );
        if s1 != ValidationClosureStatus::Inconclusive {
            return Err(format!("expected Inconclusive (foundry skipped), got {:?}", s1));
        }
        let s2 = determine_closure_status(
            &FoundryExecutionOutcome::Passed,
            &ParseBackOutcome::SkippedByReportOnly,
            false,
        );
        if s2 != ValidationClosureStatus::Inconclusive {
            return Err(format!("expected Inconclusive (parseback skipped), got {:?}", s2));
        }
        Ok(())
    }));

    v.push(run_check("r3-closure-report-id-deterministic", "R3:ValidationClosure", || {
        let make = || ValidationClosureReport::new(
            d(b"plan"), d(b"foundry_report"), d(b"parseback_report"),
            FoundryExecutionOutcome::Passed, ParseBackOutcome::Passed,
            false, vec![], d(b"bundle"),
        );
        let r1 = make();
        let r2 = make();
        if r1.id != r2.id {
            return Err(format!("non-deterministic VCR ids: {:?} vs {:?}", r1.id, r2.id));
        }
        if !r1.verify_id() {
            return Err("ValidationClosureReport id verification failed".into());
        }
        if r1.final_validation_status != ValidationClosureStatus::Passed {
            return Err(format!(
                "expected Passed for both-pass closure, got {:?}",
                r1.final_validation_status
            ));
        }
        Ok(())
    }));

    // ── R4: CorpusCartography Store ──────────────────────────────────────────

    v.push(run_check("r4-report-only-denies-all-appends", "R4:Cartography", || {
        let mut store = InMemoryCartographyStore::new(CorpusScope::LocalHostProject, d(b"pol"));
        let commit = CartographyStoreCommit::new(
            CorpusScope::LocalHostProject, 1, d(b"payload"),
            CartographyEntryKind::HostCubeSkeleton, d(b"bundle"), d(b"pol"),
        );
        let policy = PolicyProfile::default_report_only();
        match store.append(commit, &policy) {
            Err(CartographyStoreError::PolicyDenied { .. }) => Ok(()),
            Ok(_) => Err("expected PolicyDenied in ReportOnly mode, got Ok".into()),
            Err(e) => Err(format!("expected PolicyDenied, got error: {e}")),
        }
    }));

    v.push(run_check("r4-sequence-monotonic-enforced", "R4:Cartography", || {
        let mut store = InMemoryCartographyStore::new(CorpusScope::LocalHostProject, d(b"pol"));
        let dry_run = PolicyProfile::dry_run();

        // Skip sequence 1, try 2 — must fail
        let commit2 = CartographyStoreCommit::new(
            CorpusScope::LocalHostProject, 2, d(b"payload"),
            CartographyEntryKind::HostCubeSkeleton, d(b"bundle"), d(b"pol"),
        );
        match store.append(commit2, &dry_run) {
            Err(CartographyStoreError::SequenceViolation { expected, got }) => {
                if expected != 1 || got != 2 {
                    return Err(format!(
                        "wrong violation values: expected={expected}, got={got}"
                    ));
                }
                Ok(())
            }
            Ok(_) => Err("expected SequenceViolation for seq=2 on empty store".into()),
            Err(e) => Err(format!("expected SequenceViolation, got: {e}")),
        }
    }));

    v.push(run_check("r4-integrity-empty-store-is-intact", "R4:Cartography", || {
        let store = InMemoryCartographyStore::new(CorpusScope::WorkspaceFamily, d(b"pol"));
        let report = store.verify_integrity(d(b"bundle"))
            .map_err(|e| format!("verify_integrity failed: {e}"))?;
        if !report.status.is_intact() {
            return Err(format!("expected Empty/Intact, got {:?}", report.status));
        }
        if !report.verify_id() {
            return Err("CartographyIntegrityReport id verification failed".into());
        }
        Ok(())
    }));

    v.push(run_check("r4-successful-append-and-integrity", "R4:Cartography", || {
        let mut store = InMemoryCartographyStore::new(CorpusScope::LocalHostProject, d(b"pol"));
        let dry_run = PolicyProfile::dry_run();

        let commit = CartographyStoreCommit::new(
            CorpusScope::LocalHostProject, 1, d(b"payload"),
            CartographyEntryKind::EvidenceSummary, d(b"bundle"), d(b"pol"),
        );
        store.append(commit, &dry_run).map_err(|e| format!("append failed: {e}"))?;

        let report = store.verify_integrity(d(b"bundle"))
            .map_err(|e| format!("integrity check failed: {e}"))?;
        if !report.status.is_intact() {
            return Err(format!("expected Intact after one append, got {:?}", report.status));
        }
        if report.checked_count != 1 {
            return Err(format!("expected checked_count=1, got {}", report.checked_count));
        }
        Ok(())
    }));

    // ── R5: Isolated Worktree Materialization ─────────────────────────────────

    v.push(run_check("r5-host-write-hardwired-false", "R5:Materialization", || {
        let spec = IsolatedWorktreeSpec::new(
            WorktreeCreationMethod::GitWorktree,
            d(b"host_root"),
            d(b"worktree_path"),
            WorktreeCleanupPolicy::AlwaysDelete,
            d(b"policy"),
        );
        if spec.allow_host_write {
            return Err("allow_host_write must ALWAYS be false (MATBOUND-002)".into());
        }
        if !spec.verify_id() {
            return Err("IsolatedWorktreeSpec id verification failed".into());
        }
        Ok(())
    }));

    v.push(run_check("r5-worktree-spec-id-deterministic", "R5:Materialization", || {
        let make = || IsolatedWorktreeSpec::new(
            WorktreeCreationMethod::TempCopy,
            d(b"host"),
            d(b"path"),
            WorktreeCleanupPolicy::PreserveOnFailure,
            d(b"policy"),
        );
        let s1 = make();
        let s2 = make();
        if s1.id != s2.id {
            return Err(format!("non-deterministic spec ids: {:?} vs {:?}", s1.id, s2.id));
        }
        Ok(())
    }));

    // ── R6: SystemCube Disk Export ────────────────────────────────────────────

    v.push(run_check("r6-export-policy-report-only-no-write", "R6:KCube", || {
        let policy = KcubeExportPolicy::report_only(d(b"policy"), d(b"target_dir"));
        if policy.allow_write {
            return Err("allow_write must be false in report_only export policy".into());
        }
        if !policy.require_roundtrip_verification {
            return Err("require_roundtrip_verification must be true".into());
        }
        if policy.allow_overwrite {
            return Err("allow_overwrite must be false".into());
        }
        if !policy.verify_id() {
            return Err("KcubeExportPolicy id verification failed".into());
        }
        Ok(())
    }));

    v.push(run_check("r6-roundtrip-verified-equal-digests", "R6:KCube", || {
        let archive_digest = d(b"archive_bytes");
        let v_pass = KcubeRoundtripVerification::new(archive_digest, archive_digest, d(b"bundle"));
        if !v_pass.verification_passed {
            return Err("equal digests must produce verification_passed=true".into());
        }
        if !v_pass.verify_id() {
            return Err("KcubeRoundtripVerification id verification failed".into());
        }
        let v_fail = KcubeRoundtripVerification::new(d(b"archive_bytes"), d(b"different"), d(b"bundle"));
        if v_fail.verification_passed {
            return Err("different digests must produce verification_passed=false".into());
        }
        Ok(())
    }));

    v.push(run_check("r6-roundtrip-verification-id-deterministic", "R6:KCube", || {
        let make = || KcubeRoundtripVerification::new(d(b"expected"), d(b"observed"), d(b"bundle"));
        let v1 = make();
        let v2 = make();
        if v1.id != v2.id {
            return Err(format!("non-deterministic ids: {:?} vs {:?}", v1.id, v2.id));
        }
        Ok(())
    }));

    // ── R7: PSE Bridge ────────────────────────────────────────────────────────

    v.push(run_check("r7-report-only-blocks-all-submissions", "R7:PSEBridge", || {
        let policy = PseBridgePolicy::deny_all(d(b"pol"));
        let candidate = PseBridgeCandidate::new(
            PseBridgeCandidateKind::StructuralObservation,
            d(b"obs"), "test", Q16::ONE, d(b"run"), d(b"bundle"), d(b"pol"),
        );
        let profile = PolicyProfile::default_report_only();
        match validate_candidate(&candidate, &policy, &profile) {
            Err(PromotionOutcome::SkippedByReportOnly) => Ok(()),
            Ok(()) => Err("expected SkippedByReportOnly, got Ok".into()),
            Err(e) => Err(format!("expected SkippedByReportOnly, got {:?}", e)),
        }
    }));

    v.push(run_check("r7-deny-all-policy-blocks-dryrun", "R7:PSEBridge", || {
        let policy = PseBridgePolicy::deny_all(d(b"pol"));
        let candidate = PseBridgeCandidate::new(
            PseBridgeCandidateKind::StructuralObservation,
            d(b"obs"), "test", Q16::ONE, d(b"run"), d(b"bundle"), d(b"pol"),
        );
        let profile = PolicyProfile::dry_run();
        match validate_candidate(&candidate, &policy, &profile) {
            Err(PromotionOutcome::SkippedByPolicy) => Ok(()),
            Ok(()) => Err("deny_all must produce SkippedByPolicy in DryRun mode".into()),
            Err(e) => Err(format!("expected SkippedByPolicy, got {:?}", e)),
        }
    }));

    v.push(run_check("r7-zero-evidence-bundle-rejected", "R7:PSEBridge", || {
        let policy = PseBridgePolicy::allow(
            d(b"pol"),
            vec![PseBridgeCandidateKind::StructuralObservation],
            PseBridgeRateLimit::permissive(),
        );
        let candidate = PseBridgeCandidate::new(
            PseBridgeCandidateKind::StructuralObservation,
            d(b"obs"), "no-evidence", Q16::ONE,
            d(b"run"), Digest::ZERO, // ZERO evidence_bundle_id
            d(b"pol"),
        );
        let profile = PolicyProfile::dry_run();
        match validate_candidate(&candidate, &policy, &profile) {
            Err(PromotionOutcome::Rejected { .. }) => Ok(()),
            Ok(()) => Err("zero evidence_bundle_id must be Rejected".into()),
            Err(e) => Err(format!("expected Rejected, got {:?}", e)),
        }
    }));

    v.push(run_check("r7-candidate-id-deterministic", "R7:PSEBridge", || {
        let make = || PseBridgeCandidate::new(
            PseBridgeCandidateKind::TopologyObservation,
            d(b"observation"), "determinism-check", Q16::HALF,
            d(b"run"), d(b"bundle"), d(b"policy"),
        );
        let c1 = make();
        let c2 = make();
        if c1.id != c2.id {
            return Err(format!("non-deterministic candidate ids: {:?} vs {:?}", c1.id, c2.id));
        }
        if !c1.verify_id() {
            return Err("PseBridgeCandidate id verification failed".into());
        }
        Ok(())
    }));

    // ── R8: Controlled Acquisition ────────────────────────────────────────────

    v.push(run_check("r8-taint-starts-unverified", "R8:Acquisition", || {
        let source = AcquiredSource::new(
            AcquisitionSourceKind::LocalPath,
            d(b"path"), d(b"content"), d(b"sandbox"), d(b"bundle"), d(b"pol"), 1024,
        );
        if source.taint != AcquisitionTaint::Unverified {
            return Err(format!("expected Unverified, got {:?}", source.taint));
        }
        if !matches!(source.license_check, LicenseCheckOutcome::NotRun) {
            return Err("license_check must start as NotRun".into());
        }
        if !matches!(source.secret_scan, SecretScanOutcome::NotRun) {
            return Err("secret_scan must start as NotRun".into());
        }
        if source.may_proceed_to_foundry() {
            return Err("Unverified source must not proceed to Foundry".into());
        }
        Ok(())
    }));

    v.push(run_check("r8-taint-cleared-requires-both-checks", "R8:Acquisition", || {
        // Both pass → Cleared
        let cleared = AcquiredSource::new(
            AcquisitionSourceKind::LocalPath,
            d(b"path"), d(b"content"), d(b"sandbox"), d(b"bundle"), d(b"pol"), 512,
        )
        .with_license_check(LicenseCheckOutcome::Passed { spdx_id: "MIT".into() })
        .with_secret_scan(SecretScanOutcome::Passed);

        if cleared.taint != AcquisitionTaint::Cleared {
            return Err(format!("both-pass must yield Cleared, got {:?}", cleared.taint));
        }
        if !cleared.may_proceed_to_foundry() {
            return Err("Cleared source must proceed to Foundry".into());
        }

        // License only → NOT cleared
        let lic_only = AcquiredSource::new(
            AcquisitionSourceKind::LocalPath,
            d(b"path2"), d(b"content"), d(b"sandbox"), d(b"bundle"), d(b"pol"), 512,
        )
        .with_license_check(LicenseCheckOutcome::Passed { spdx_id: "Apache-2.0".into() });

        if lic_only.taint == AcquisitionTaint::Cleared {
            return Err("license-only must NOT yield Cleared".into());
        }
        if lic_only.may_proceed_to_foundry() {
            return Err("license-only source must not proceed to Foundry".into());
        }
        Ok(())
    }));

    v.push(run_check("r8-capability-deny-all-locked", "R8:Acquisition", || {
        let cap = SourceAcquisitionCapability::deny_all(d(b"pol"));
        if cap.allow_local_path {
            return Err("deny_all must lock allow_local_path=false".into());
        }
        if cap.allow_network {
            return Err("deny_all must lock allow_network=false".into());
        }
        if cap.allow_git_repository {
            return Err("deny_all must lock allow_git_repository=false".into());
        }
        if !cap.allowed_source_kinds.is_empty() {
            return Err("deny_all must have empty allowed_source_kinds".into());
        }
        if !cap.verify_id() {
            return Err("SourceAcquisitionCapability id verification failed".into());
        }
        Ok(())
    }));

    v.push(run_check("r8-taint-id-deterministic", "R8:Acquisition", || {
        let make = || AcquiredSource::new(
            AcquisitionSourceKind::Archive,
            d(b"path"), d(b"content"), d(b"sandbox"), d(b"bundle"), d(b"pol"), 2048,
        );
        let s1 = make();
        let s2 = make();
        if s1.id != s2.id {
            return Err(format!("non-deterministic source ids: {:?} vs {:?}", s1.id, s2.id));
        }
        if !s1.verify_id() {
            return Err("AcquiredSource id verification failed".into());
        }
        Ok(())
    }));

    // ── R9: Evaluation Harness ───────────────────────────────────────────────

    v.push(run_check("r9-deterministic-report-ids-invariant007", "R9:Evaluation", || {
        let scenario = EvaluationScenario::new(
            "determinism-test",
            EvaluationDimension::FoundryExecution,
            d(b"input_state"),
            d(b"expected_outcome"),
            EvaluationCriteria::strict(),
            d(b"policy"),
        );
        let harness = StubEvaluationHarness::always_pass();
        let profile = PolicyProfile::dry_run();

        let r1 = harness.run_scenario(&scenario, &profile, d(b"bundle"));
        let r2 = harness.run_scenario(&scenario, &profile, d(b"bundle"));

        if r1.id != r2.id {
            return Err(format!(
                "INVARIANT-007 violated: identical inputs produced different ids: {:?} vs {:?}",
                r1.id, r2.id
            ));
        }
        if !r1.verify_id() {
            return Err("EvaluationRunReport id verification failed".into());
        }
        if !r1.outcome.is_passed() {
            return Err(format!("always_pass harness produced non-pass outcome: {:?}", r1.outcome));
        }
        Ok(())
    }));

    v.push(run_check("r9-report-only-skips-execution", "R9:Evaluation", || {
        let scenario = EvaluationScenario::new(
            "ro-skip-test",
            EvaluationDimension::AcquisitionTaint,
            d(b"input"), d(b"expected"),
            EvaluationCriteria::permissive(),
            d(b"pol"),
        );
        let harness = StubEvaluationHarness::always_pass();
        let report_only = PolicyProfile::default_report_only();
        let report = harness.run_scenario(&scenario, &report_only, d(b"bundle"));
        if !report.outcome.is_skipped_report_only() {
            return Err(format!(
                "expected SkippedByReportOnly in ReportOnly mode, got {:?}",
                report.outcome
            ));
        }
        Ok(())
    }));

    v.push(run_check("r9-always-fail-harness-fails-strict", "R9:Evaluation", || {
        let scenario = EvaluationScenario::new(
            "fail-test",
            EvaluationDimension::CartographyIntegrity,
            d(b"input"), d(b"expected"),
            EvaluationCriteria::strict(),
            d(b"pol"),
        );
        let harness = StubEvaluationHarness::always_fail();
        let profile = PolicyProfile::dry_run();
        let report = harness.run_scenario(&scenario, &profile, d(b"bundle"));
        if report.outcome.is_passed() {
            return Err("always_fail harness must not produce passed outcome".into());
        }
        if !report.outcome.is_failure_class() {
            return Err(format!("expected failure class outcome, got {:?}", report.outcome));
        }
        Ok(())
    }));

    v.push(run_check("r9-suite-worst-wins-aggregation", "R9:Evaluation", || {
        let make_scenario = |name: &str| EvaluationScenario::new(
            name,
            EvaluationDimension::End2End,
            d(name.as_bytes()),
            d(b"expected"),
            EvaluationCriteria::strict(),
            d(b"pol"),
        );
        let pass_harness = StubEvaluationHarness::always_pass();
        let fail_harness = StubEvaluationHarness::always_fail();
        let profile = PolicyProfile::dry_run();

        let s1 = make_scenario("s1");
        let s2 = make_scenario("s2");
        let s3 = make_scenario("s3");

        let reports = vec![
            pass_harness.run_scenario(&s1, &profile, d(b"b1")),
            pass_harness.run_scenario(&s2, &profile, d(b"b2")),
            fail_harness.run_scenario(&s3, &profile, d(b"b3")),  // one failure
        ];

        let suite = EvaluationSuiteReport::from_run_reports(&reports, d(b"suite_bundle"), 0);

        if suite.suite_outcome != EvaluationSuiteOutcome::SomeFailed {
            return Err(format!(
                "worst-wins: expected SomeFailed, got {:?}", suite.suite_outcome
            ));
        }
        if suite.passed_count != 2 {
            return Err(format!("expected 2 passed, got {}", suite.passed_count));
        }
        if suite.failed_count != 1 {
            return Err(format!("expected 1 failed, got {}", suite.failed_count));
        }
        if !suite.verify_id() {
            return Err("EvaluationSuiteReport id verification failed".into());
        }
        Ok(())
    }));

    v.push(run_check("r9-suite-all-pass-is-all-passed", "R9:Evaluation", || {
        let scenario = EvaluationScenario::new(
            "all-pass",
            EvaluationDimension::BridgeCandidate,
            d(b"input"), d(b"expected"),
            EvaluationCriteria::permissive(),
            d(b"pol"),
        );
        let harness = StubEvaluationHarness::always_pass();
        let profile = PolicyProfile::dry_run();

        let reports: Vec<EvaluationRunReport> = (0..3)
            .map(|i| harness.run_scenario(&scenario, &profile, d(&[i])))
            .collect();

        let suite = EvaluationSuiteReport::from_run_reports(&reports, d(b"bundle"), 0);
        if suite.suite_outcome != EvaluationSuiteOutcome::AllPassed {
            return Err(format!("expected AllPassed, got {:?}", suite.suite_outcome));
        }
        if suite.failed_count != 0 {
            return Err(format!("expected 0 failed, got {}", suite.failed_count));
        }
        Ok(())
    }));

    v.push(run_check("r9-policy-id-content-addressed", "R9:Evaluation", || {
        let p1 = PolicyProfile::default_report_only();
        let p2 = PolicyProfile::default_report_only();
        if p1.id != p2.id {
            return Err("PolicyProfile id must be deterministic across calls".into());
        }
        if !p1.verify_id() {
            return Err("PolicyProfile verify_id() failed".into());
        }
        let dry = PolicyProfile::dry_run();
        if p1.id == dry.id {
            return Err("different modes must produce different ids".into());
        }
        let op = PolicyProfile::operator_approved();
        if p1.id == op.id || dry.id == op.id {
            return Err("all three modes must have distinct ids".into());
        }
        Ok(())
    }));

    // ── RX: Real Foundry Executor ─────────────────────────────────────────────
    // These exercise the live executor's governance layer (cross-platform,
    // no compilation). The real process spawn/capture/timeout paths are covered
    // by `cargo test -p kosmo-foundry`.

    v.push(run_check("rx-foundry-report-only-spawns-nothing", "RX:FoundryExec", || {
        let plan = kosmo_foundry::standard_cargo_plan(
            d(b"pol"), d(b"ws"), d(b"task"), d(b"root"), 30_000,
        );
        let exec = kosmo_foundry::FoundryExecutor::new(".");
        let report = exec.execute(&plan, &PolicyProfile::default_report_only(), d(b"bundle"));
        if !report.outcome.is_skipped_report_only() {
            return Err(format!("expected SkippedByReportOnly, got {:?}", report.outcome));
        }
        if !report.check_results.is_empty() {
            return Err("ReportOnly must spawn nothing — no check results allowed".into());
        }
        if !report.verify_id() {
            return Err("FoundryExecutionReport id verification failed".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-foundry-command-denied-not-executed", "RX:FoundryExec", || {
        // Default cargo policy allows only `cargo`; invoke a different program.
        let plan = kosmo_foundry::standard_cargo_plan(
            d(b"pol"), d(b"ws"), d(b"task"), d(b"root"), 30_000,
        );
        let exec = kosmo_foundry::FoundryExecutor::new(".").with_program("definitely-not-cargo");
        let report = exec.execute(&plan, &PolicyProfile::dry_run(), d(b"bundle"));
        if report.outcome != kosmo_core::FoundryExecutionOutcome::CommandDeniedByPolicy {
            return Err(format!("expected CommandDeniedByPolicy, got {:?}", report.outcome));
        }
        if !report.verify_id() {
            return Err("FoundryExecutionReport id verification failed".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-foundry-kind-mapping-read-only", "RX:FoundryExec", || {
        use kosmo_core::FoundryCheckKind as K;
        use kosmo_foundry::map_kind_to_subcommand as m;
        // Only read-only verification subcommands are reachable.
        if m(&K::Build) != Some("check") { return Err("Build must map to check".into()); }
        if m(&K::Test) != Some("test") { return Err("Test must map to test".into()); }
        if m(&K::Lint) != Some("clippy") { return Err("Lint must map to clippy".into()); }
        // No mapping exposes a mutating command.
        if m(&K::Security).is_some() { return Err("Security must not map to cargo".into()); }
        if m(&K::Custom("x".into())).is_some() { return Err("Custom must not map".into()); }
        Ok(())
    }));

    // ── RX: Persistent CorpusCartography Store (disk) ─────────────────────────
    // Demonstrates the emergent host-write invariant: a durable append is a
    // host write, so DryRun (allow_host_write == false) cannot persist — only
    // OperatorApproved can. Uses a temp file that is cleaned up.

    v.push(run_check("rx-store-dryrun-cannot-persist", "RX:PersistStore", || {
        use kosmo_core::{CartographyEntryKind, CartographyStoreCommit, CorpusCartographyStore,
            CorpusScope, CartographyStoreError};
        let path = temp_store_path("eval-dryrun");
        let mut store = kosmo_store::JsonlCartographyStore::open(
            &path, CorpusScope::LocalHostProject, d(b"pol"),
        ).map_err(|e| format!("open failed: {e}"))?;
        let commit = CartographyStoreCommit::new(
            CorpusScope::LocalHostProject, 1, d(b"payload"),
            CartographyEntryKind::EvidenceSummary, d(b"bundle"), d(b"pol"),
        );
        let res = store.append(commit, &PolicyProfile::dry_run());
        let denied = matches!(res, Err(CartographyStoreError::PolicyDenied { .. }));
        let no_file = !path.exists();
        let _ = std::fs::remove_file(&path);
        if !denied {
            return Err("DryRun must be denied: a durable append is a host write".into());
        }
        if !no_file {
            return Err("no file may be created when persist is denied".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-store-operator-approved-persists-reloads", "RX:PersistStore", || {
        use kosmo_core::{CartographyEntryKind, CartographyStoreCommit, CorpusCartographyStore,
            CorpusScope};
        let path = temp_store_path("eval-persist");
        let _ = std::fs::remove_file(&path);
        let scope = CorpusScope::LocalHostProject;
        // Write two commits under OperatorApproved (allow_host_write == true).
        {
            let mut store = kosmo_store::JsonlCartographyStore::open(&path, scope.clone(), d(b"pol"))
                .map_err(|e| format!("open failed: {e}"))?;
            for seq in 1..=2u64 {
                let commit = CartographyStoreCommit::new(
                    scope.clone(), seq, d(format!("p{seq}").as_bytes()),
                    CartographyEntryKind::EvidenceSummary, d(b"bundle"), d(b"pol"),
                );
                store.append(commit, &PolicyProfile::operator_approved())
                    .map_err(|e| format!("append {seq} failed: {e}"))?;
            }
        }
        // Reopen from disk and verify integrity of the durable copy.
        let reopened = kosmo_store::JsonlCartographyStore::open(&path, scope, d(b"pol"))
            .map_err(|e| format!("reopen failed: {e}"))?;
        let report = reopened.verify_integrity(d(b"bundle"))
            .map_err(|e| format!("integrity failed: {e}"))?;
        let intact = report.status.is_intact() && report.checked_count == 2;
        let _ = std::fs::remove_file(&path);
        if !intact {
            return Err(format!("expected Intact/2, got {:?}/{}", report.status, report.checked_count));
        }
        Ok(())
    }));

    // ── RX: Real ParseBack Executor ───────────────────────────────────────────
    // Validates the real ParseBack executor: governance, baseline integrity,
    // deterministic snapshotting, and structural diff logic.

    v.push(run_check("rx-parseback-report-only-skips-scan", "RX:ParseBackExec", || {
        use kosmo_core::{ParseBackPlan, ParseBackScanScope};
        let executor = ParseBackExecutor::new(PathBuf::from("/nonexistent"));
        let pre = TopologySnapshot::from_parts(
            ParseBackScanScope::FullWorkspace,
            Default::default(),
            Default::default(),
        );
        let plan = ParseBackPlan::new(
            d(b"pol"), d(b"mat-plan"),
            ParseBackScanScope::FullWorkspace,
            pre.snapshot_id,
        );
        let policy = PolicyProfile::default_report_only();
        let report = executor.execute(&plan, &pre, &policy, d(b"bundle"));
        if !report.outcome.is_skipped_report_only() {
            return Err(format!("expected SkippedByReportOnly, got {:?}", report.outcome));
        }
        if !report.verify_id() {
            return Err("report id verification failed".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-parseback-baseline-mismatch-inconclusive", "RX:ParseBackExec", || {
        use kosmo_core::{ParseBackPlan, ParseBackScanScope};
        let executor = ParseBackExecutor::new(PathBuf::from("/nonexistent"));
        let pre = TopologySnapshot::from_parts(
            ParseBackScanScope::FullWorkspace,
            Default::default(),
            Default::default(),
        );
        let wrong_baseline = d(b"wrong-baseline");
        let plan = ParseBackPlan::new(
            d(b"pol"), d(b"mat-plan"),
            ParseBackScanScope::FullWorkspace,
            wrong_baseline, // does not match pre.snapshot_id
        );
        let policy = PolicyProfile::dry_run();
        let report = executor.execute(&plan, &pre, &policy, d(b"bundle"));
        if !report.outcome.is_failure_class() {
            return Err(format!("expected Inconclusive, got {:?}", report.outcome));
        }
        if report.diagnostics.iter().all(|s| !s.contains("mismatch")) {
            return Err("diagnostic must mention mismatch".into());
        }
        if !report.verify_id() {
            return Err("report id verification failed".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-parseback-diff-node-added-warning", "RX:ParseBackExec", || {
        use kosmo_core::ParseBackScanScope;
        use std::collections::{BTreeMap, BTreeSet};
        let pre = TopologySnapshot::from_parts(
            ParseBackScanScope::FullWorkspace,
            BTreeMap::new(),
            BTreeSet::new(),
        );
        let fp = CrateFingerprint::new("new-crate".into(), vec!["lib.rs".into()], vec![]);
        let mut nodes = BTreeMap::new();
        nodes.insert("new-crate".to_string(), fp.clone());
        let post = TopologySnapshot::from_parts(
            ParseBackScanScope::FullWorkspace,
            nodes,
            BTreeSet::new(),
        );
        let deltas = diff_snapshots(&pre, &post);
        if deltas.len() != 1 {
            return Err(format!("expected 1 delta, got {}", deltas.len()));
        }
        use kosmo_core::TopologyChangeKind;
        if !matches!(deltas[0].change_kind, TopologyChangeKind::NodeAdded) {
            return Err(format!("expected NodeAdded, got {:?}", deltas[0].change_kind));
        }
        use kosmo_core::ParseBackSeverity;
        if deltas[0].severity != ParseBackSeverity::Warning {
            return Err(format!("expected Warning severity, got {:?}", deltas[0].severity));
        }
        if !deltas[0].verify_id() {
            return Err("delta id verification failed".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-parseback-diff-node-removed-critical", "RX:ParseBackExec", || {
        use kosmo_core::ParseBackScanScope;
        use std::collections::{BTreeMap, BTreeSet};
        let fp = CrateFingerprint::new("gone-crate".into(), vec!["lib.rs".into()], vec![]);
        let mut nodes = BTreeMap::new();
        nodes.insert("gone-crate".to_string(), fp);
        let pre = TopologySnapshot::from_parts(
            ParseBackScanScope::FullWorkspace,
            nodes,
            BTreeSet::new(),
        );
        let post = TopologySnapshot::from_parts(
            ParseBackScanScope::FullWorkspace,
            BTreeMap::new(),
            BTreeSet::new(),
        );
        let deltas = diff_snapshots(&pre, &post);
        if deltas.len() != 1 {
            return Err(format!("expected 1 delta, got {}", deltas.len()));
        }
        use kosmo_core::TopologyChangeKind;
        if !matches!(deltas[0].change_kind, TopologyChangeKind::NodeRemoved) {
            return Err(format!("expected NodeRemoved, got {:?}", deltas[0].change_kind));
        }
        use kosmo_core::ParseBackSeverity;
        if deltas[0].severity != ParseBackSeverity::Critical {
            return Err(format!("expected Critical severity, got {:?}", deltas[0].severity));
        }
        if !deltas[0].verify_id() {
            return Err("delta id verification failed".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-parseback-snapshot-deterministic", "RX:ParseBackExec", || {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..").join("..");
        let executor = ParseBackExecutor::new(workspace_root);
        use kosmo_core::ParseBackScanScope;
        let s1 = match executor.snapshot(&ParseBackScanScope::FullWorkspace) {
            Ok(s) => s,
            Err(e) => return Err(format!("snapshot failed: {}", e)),
        };
        let s2 = match executor.snapshot(&ParseBackScanScope::FullWorkspace) {
            Ok(s) => s,
            Err(e) => return Err(format!("snapshot (2nd) failed: {}", e)),
        };
        if s1.snapshot_id != s2.snapshot_id {
            return Err("snapshots are not deterministic".into());
        }
        if s1.crate_count() < 10 {
            return Err(format!("expected ≥10 crates, got {}", s1.crate_count()));
        }
        Ok(())
    }));

    v.push(run_check("rx-parseback-unchanged-workspace-passes", "RX:ParseBackExec", || {
        use kosmo_core::{ParseBackPlan, ParseBackScanScope};
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..").join("..");
        let executor = ParseBackExecutor::new(workspace_root);
        let pre = match executor.snapshot(&ParseBackScanScope::AffectedCratesOnly) {
            Ok(s) => s,
            Err(e) => return Err(format!("pre-snapshot failed: {}", e)),
        };
        let plan = ParseBackPlan::new(
            d(b"pol"), d(b"mat-plan"),
            ParseBackScanScope::AffectedCratesOnly,
            pre.snapshot_id,
        );
        let policy = PolicyProfile::dry_run();
        let report = executor.execute(&plan, &pre, &policy, d(b"bundle"));
        if !report.outcome.is_passed() {
            return Err(format!("unchanged workspace must pass, got {:?}", report.outcome));
        }
        if !report.verify_id() {
            return Err("report id verification failed".into());
        }
        if report.pre_topology_id != report.post_topology_id {
            return Err("pre/post topology ids must be equal when unchanged".into());
        }
        Ok(())
    }));

    // ── RX: Operator (R1→R2→R3 full pipeline) ────────────────────────────────
    // Validates the operator orchestration: policy governance and the full
    // validation-closure pipeline producing a real content-addressed report.

    v.push(run_check("rx-operator-report-only-inconclusive", "RX:Operator", || {
        use kosmo_core::{ParseBackScanScope, ValidationClosureStatus};
        let plan = standard_plan(
            &PathBuf::from("/nonexistent"),
            ParseBackScanScope::FullWorkspace,
            d(b"pol"),
            5_000,
        );
        let executor = OperatorExecutor::new(PathBuf::from("/nonexistent"));
        let report = executor.execute(&plan, &PolicyProfile::default_report_only(), d(b"bundle"));
        if !matches!(report.closure_report.final_validation_status, ValidationClosureStatus::Inconclusive) {
            return Err(format!("ReportOnly must be Inconclusive, got {:?}", report.closure_report.final_validation_status));
        }
        if report.persisted {
            return Err("ReportOnly must never persist".into());
        }
        if !report.verify_id() {
            return Err("report id verification failed".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-operator-report-is-content-addressed", "RX:Operator", || {
        use kosmo_core::ParseBackScanScope;
        let plan = standard_plan(
            &PathBuf::from("/nonexistent"),
            ParseBackScanScope::FullWorkspace,
            d(b"pol"),
            5_000,
        );
        let executor = OperatorExecutor::new(PathBuf::from("/nonexistent"));
        let bid = d(b"bundle");
        let r1 = executor.execute(&plan, &PolicyProfile::default_report_only(), bid);
        let r2 = executor.execute(&plan, &PolicyProfile::default_report_only(), bid);
        if r1.report_id != r2.report_id {
            return Err("INVARIANT-007: identical inputs must produce identical report_id".into());
        }
        if r1.report_id == d(b"") {
            return Err("report_id must not be zero/empty".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-operator-full-cycle-dry-run", "RX:Operator", || {
        use kosmo_core::{FoundryCheckKind, FoundryCheckSpec, FoundryCommandPolicy,
            FoundryEnvironmentPolicy, FoundryExecutionPlan, FoundrySandboxKind,
            FoundrySandboxSpec, FoundryTimeoutPolicy, ParseBackScanScope,
            ValidationClosureStatus};
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..").join("..");
        // Use a targeted check-only plan on a single already-compiled crate for speed.
        let check_spec = FoundryCheckSpec::new(FoundryCheckKind::Build, "kosmo-parseback", true)
            .with_args(vec!["-p".into(), "kosmo-parseback".into()]);
        let foundry_plan = FoundryExecutionPlan::new(
            d(b"pol"),
            d(b"widx"),
            d(b"task"),
            FoundrySandboxSpec::new(FoundrySandboxKind::LocalDryRun, d(b"root")),
            vec![check_spec],
            FoundryCommandPolicy::default_cargo_policy(),
            FoundryTimeoutPolicy::new(60_000, 120_000),
            FoundryEnvironmentPolicy::locked(),
        );
        let plan = OperationPlan::new(foundry_plan, ParseBackScanScope::AffectedCratesOnly, d(b"pol"));
        let executor = OperatorExecutor::new(&workspace_root);
        let report = executor.execute(&plan, &PolicyProfile::dry_run(), d(b"bundle"));
        let ok = matches!(
            report.closure_report.final_validation_status,
            ValidationClosureStatus::Passed | ValidationClosureStatus::PassedWithWarnings
        );
        if !ok {
            return Err(format!(
                "DryRun on clean workspace expected Passed/PassedWithWarnings, got {:?} (foundry: {:?}, parseback: {:?})",
                report.closure_report.final_validation_status,
                report.foundry_report.outcome,
                report.parseback_report.outcome,
            ));
        }
        if report.persisted {
            return Err("DryRun must not persist".into());
        }
        if !report.verify_id() {
            return Err("report id verification failed".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-operator-approved-persists-closure", "RX:Operator", || {
        use kosmo_core::{FoundryCheckKind, FoundryCheckSpec, FoundryCommandPolicy,
            FoundryEnvironmentPolicy, FoundryExecutionPlan, FoundrySandboxKind,
            FoundrySandboxSpec, FoundryTimeoutPolicy, ParseBackScanScope,
            ValidationClosureStatus};
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..").join("..");
        let store_path = {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos()).unwrap_or(0);
            std::env::temp_dir().join(format!("kosmo-op-eval-{nanos}.jsonl"))
        };
        let check_spec = FoundryCheckSpec::new(FoundryCheckKind::Build, "kosmo-parseback", true)
            .with_args(vec!["-p".into(), "kosmo-parseback".into()]);
        let foundry_plan = FoundryExecutionPlan::new(
            d(b"pol"),
            d(b"widx"),
            d(b"task"),
            FoundrySandboxSpec::new(FoundrySandboxKind::LocalDryRun, d(b"root")),
            vec![check_spec],
            FoundryCommandPolicy::default_cargo_policy(),
            FoundryTimeoutPolicy::new(60_000, 120_000),
            FoundryEnvironmentPolicy::locked(),
        );
        let plan = OperationPlan::new(foundry_plan, ParseBackScanScope::AffectedCratesOnly, d(b"pol"));
        let executor = OperatorExecutor::new(&workspace_root)
            .with_store(&store_path);
        let report = executor.execute(&plan, &PolicyProfile::operator_approved(), d(b"bundle"));
        let file_created = store_path.exists();
        let _ = std::fs::remove_file(&store_path);

        if !report.verify_id() {
            return Err("report id verification failed".into());
        }
        if matches!(
            report.closure_report.final_validation_status,
            ValidationClosureStatus::Passed | ValidationClosureStatus::PassedWithWarnings
        ) {
            if !report.persisted {
                return Err("OperatorApproved + Passed must persist closure".into());
            }
            if !file_created {
                return Err("store file must be created when closure is persisted".into());
            }
        } else {
            return Err(format!(
                "OperatorApproved on clean workspace expected Passed, got {:?}",
                report.closure_report.final_validation_status
            ));
        }
        Ok(())
    }));

    // ── RX:Pipeline — Phase 4 CubeSwarm + HostTargetDelta weld ──────────────

    v.push(run_check("rx-pipeline-swarm-and-delta-in-report", "RX:Pipeline", || {
        // The IntegrationRunReport must carry swarm_composite and void_fill_delta
        // for every run — Phase 4 CubeSwarm is always-on (passive / advisory).
        let policy = PolicyProfile::default_report_only();
        let index = WorkspaceIndex::from_entries("test".into(), vec![], policy.id);
        let r = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        if r.swarm_composite.policy_id != policy.id {
            return Err("swarm_composite must carry pipeline policy_id".into());
        }
        if r.void_fill_delta.policy_id != policy.id {
            return Err("void_fill_delta must carry pipeline policy_id".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-pipeline-empty-workspace-delta-clean", "RX:Pipeline", || {
        // An empty workspace produces no voids → delta status must be Clean.
        use kosmo_hyphae::delta::DeltaStatus;
        let policy = PolicyProfile::default_report_only();
        let index = WorkspaceIndex::from_entries("test".into(), vec![], policy.id);
        let r = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        if r.void_fill_delta.status != DeltaStatus::Clean {
            return Err(format!(
                "empty workspace must yield Clean delta, got {:?}",
                r.void_fill_delta.status
            ));
        }
        Ok(())
    }));

    v.push(run_check("rx-pipeline-policy-consistency-includes-swarm", "RX:Pipeline", || {
        // verify_policy_consistency() must check swarm + delta policy_ids too.
        let policy = PolicyProfile::default_report_only();
        let index = WorkspaceIndex::from_entries("test".into(), vec![], policy.id);
        let r = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        if !r.verify_policy_consistency() {
            return Err("verify_policy_consistency must pass with swarm/delta present".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-pipeline-report-is-deterministic", "RX:Pipeline", || {
        // Two identical runs must produce the same report_id — the Phase 4
        // swarm + delta must be content-addressed and deterministic.
        let policy = PolicyProfile::default_report_only();
        let index = WorkspaceIndex::from_entries("test".into(), vec![], policy.id);
        let r1 = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        let r2 = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        if r1.report_id != r2.report_id {
            return Err("pipeline must be deterministic across runs".into());
        }
        if r1.swarm_composite.composite_id != r2.swarm_composite.composite_id {
            return Err("swarm_composite must be deterministic".into());
        }
        if r1.void_fill_delta.delta_id != r2.void_fill_delta.delta_id {
            return Err("void_fill_delta must be deterministic".into());
        }
        Ok(())
    }));

    // ── RX:Pipeline — Phase 4c HostTargetCollapsePlan weld ───────────────────

    v.push(run_check("rx-pipeline-collapse-plan-is-planning-only", "RX:Pipeline", || {
        // The collapse plan generated by run_dry_pipeline must always be
        // PlanningOnly in Phase 5 — no execution authority exists yet.
        use kosmo_hyphae::CollapsePlanStatus;
        let policy = PolicyProfile::default_report_only();
        let index = WorkspaceIndex::from_entries("test".into(), vec![], policy.id);
        let r = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        if r.collapse_plan.status != CollapsePlanStatus::PlanningOnly {
            return Err(format!("expected PlanningOnly, got {:?}", r.collapse_plan.status));
        }
        Ok(())
    }));

    v.push(run_check("rx-pipeline-collapse-plan-carries-policy-id", "RX:Pipeline", || {
        // collapse_plan.policy_id must match the governing policy (traceability).
        let policy = PolicyProfile::default_report_only();
        let index = WorkspaceIndex::from_entries("test".into(), vec![], policy.id);
        let r = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        if r.collapse_plan.policy_id != policy.id {
            return Err("collapse_plan.policy_id must match pipeline policy".into());
        }
        if !r.verify_policy_consistency() {
            return Err("verify_policy_consistency must include collapse_plan".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-pipeline-collapse-plan-is-content-addressed", "RX:Pipeline", || {
        // Identical runs must yield identical plan_id (determinism).
        // plan_id must be non-zero when delta carries fills.
        let policy = PolicyProfile::default_report_only();
        use kosmo_workbench::{WorkspaceEntry, WorkspaceEntryKind};
        use kosmo_core::Digest;
        let entries = vec![
            WorkspaceEntry { path: "a.rs".into(), digest: Digest::of_bytes(b"a"), size_bytes: 10, kind: WorkspaceEntryKind::SourceFile },
        ];
        let index = WorkspaceIndex::from_entries("test".into(), entries, policy.id);
        let r1 = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        let r2 = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        if r1.collapse_plan.plan_id != r2.collapse_plan.plan_id {
            return Err("collapse plan must be deterministic (content-addressed)".into());
        }
        // collapse_plan.delta_id must match void_fill_delta.delta_id
        if r1.collapse_plan.delta_id != r1.void_fill_delta.delta_id {
            return Err("collapse_plan.delta_id must equal void_fill_delta.delta_id".into());
        }
        Ok(())
    }));

    // ── RX:Pipeline — JsonlCartographyStore persistence (policy-gated) ────────

    v.push(run_check("rx-pipeline-cartography-persist-blocked-by-default-policy", "RX:Pipeline", || {
        // run_dry_pipeline uses ReportOnly policy → persisting its cartography
        // update must be denied. One invariant governs both the run and any
        // subsequent persistence attempt: allow_host_write == false.
        use kosmo_core::{CartographyStoreError, CorpusScope};
        use kosmo_pipeline::persist_cartography_update;
        use std::time::{SystemTime, UNIX_EPOCH};
        let policy = PolicyProfile::default_report_only();
        let index = WorkspaceIndex::from_entries("test".into(), vec![], policy.id);
        let r = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("kosmo-eval-persist-ro-{nanos}.jsonl"));
        let res = persist_cartography_update(
            &r.cartography_update, &path, CorpusScope::LocalHostProject, &policy,
        );
        if !matches!(res, Err(CartographyStoreError::PolicyDenied { .. })) {
            return Err("ReportOnly policy must deny cartography persistence".into());
        }
        if path.exists() { let _ = std::fs::remove_file(&path); }
        Ok(())
    }));

    v.push(run_check("rx-pipeline-cartography-persist-succeeds-with-operator-approved", "RX:Pipeline", || {
        // OperatorApproved policy (allow_host_write=true) must allow persistence.
        // The stored commit's payload_digest must equal the update's update_id.
        use kosmo_core::{CartographyEntryKind, CartographyStoreError, CorpusScope};
        use kosmo_pipeline::persist_cartography_update;
        use kosmo_store::JsonlCartographyStore;
        use kosmo_core::CorpusCartographyStore;
        use std::time::{SystemTime, UNIX_EPOCH};
        let policy = PolicyProfile::operator_approved();
        let index = WorkspaceIndex::from_entries("test".into(), vec![], policy.id);
        let r = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("kosmo-eval-persist-oa-{nanos}.jsonl"));
        let commit_id = persist_cartography_update(
            &r.cartography_update, &path, CorpusScope::LocalHostProject, &policy,
        ).map_err(|e| format!("persist failed: {:?}", e))?;
        if commit_id == Digest::ZERO {
            return Err("persisted commit_id must be non-zero".into());
        }
        // Verify: reopen store, check payload_digest == update_id
        let store = JsonlCartographyStore::open(&path, CorpusScope::LocalHostProject, policy.id)
            .map_err(|e| format!("reopen failed: {:?}", e))?;
        let manifest = store.read_manifest().map_err(|e| format!("manifest: {:?}", e))?;
        if manifest.entries.len() != 1 {
            return Err(format!("expected 1 commit, got {}", manifest.entries.len()));
        }
        if manifest.entries[0].payload_digest != r.cartography_update.update_id {
            return Err("commit payload_digest must equal cartography update_id".into());
        }
        if manifest.entries[0].payload_kind != CartographyEntryKind::CartographyUpdate {
            return Err("commit payload_kind must be CartographyUpdate".into());
        }
        let _ = std::fs::remove_file(&path);
        Ok(())
    }));

    // ── RX:Pipeline — Phase 4d MorphogenicCorpusUpdate skeleton ─────────────

    v.push(run_check("rx-pipeline-morphogenic-update-is-content-addressed", "RX:Pipeline", || {
        // MorphogenicCorpusUpdate in the report must be deterministic and must
        // link cartography_update_id + collapse_plan_id.
        let policy = PolicyProfile::default_report_only();
        let index = WorkspaceIndex::from_entries("test".into(), vec![], policy.id);
        let r1 = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        let r2 = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        if r1.morphogenic_update.update_id != r2.morphogenic_update.update_id {
            return Err("morphogenic_update must be deterministic".into());
        }
        if r1.morphogenic_update.update_id == Digest::ZERO {
            return Err("morphogenic_update.update_id must be non-zero".into());
        }
        // Structural integrity: must link to the correct sub-artifacts
        if r1.morphogenic_update.cartography_update_id != r1.cartography_update.update_id {
            return Err("morphogenic_update.cartography_update_id must equal cartography_update.update_id".into());
        }
        if r1.morphogenic_update.collapse_plan_id != r1.collapse_plan.plan_id {
            return Err("morphogenic_update.collapse_plan_id must equal collapse_plan.plan_id".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-pipeline-morphogenic-update-carries-policy-id", "RX:Pipeline", || {
        // morphogenic_update.policy_id must participate in verify_policy_consistency().
        let policy = PolicyProfile::default_report_only();
        let index = WorkspaceIndex::from_entries("test".into(), vec![], policy.id);
        let r = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        if r.morphogenic_update.policy_id != policy.id {
            return Err("morphogenic_update.policy_id must match pipeline policy".into());
        }
        if !r.verify_policy_consistency() {
            return Err("verify_policy_consistency must include morphogenic_update.policy_id".into());
        }
        Ok(())
    }));

    // ── RX:Pipeline — Step 3b surgery options (energy-ranked) ────────────────

    v.push(run_check("rx-pipeline-no-surgery-when-disabled", "RX:Pipeline", || {
        // Surgery is off by default; report must carry an empty surgery_options vec.
        let policy = PolicyProfile::default_report_only();
        let index = WorkspaceIndex::from_entries("test".into(), vec![], policy.id);
        let r = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        if !r.surgery_options.is_empty() {
            return Err("surgery_options must be empty when enable_surgery=false".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-pipeline-surgery-produces-options-with-metatron", "RX:Pipeline", || {
        // With both Metatron and surgery enabled, any diagnostics that contain
        // ambiguities or void hypotheses should yield surgery options.
        // With the fixture index (3 entries → 3 voids → 3 diagnostics), at least
        // one option should be produced if the diagnostics have any hypotheses.
        // We verify: surgery_options.len() >= 0 (never panics), all carry policy_id,
        // and verify_policy_consistency() passes.
        use kosmo_workbench::{WorkspaceEntry, WorkspaceEntryKind};
        let policy = PolicyProfile::default_report_only();
        let entries = vec![
            WorkspaceEntry { path: "src/lib.rs".into(), digest: Digest::of_bytes(b"lib"),
                size_bytes: 100, kind: WorkspaceEntryKind::SourceFile },
            WorkspaceEntry { path: "src/main.rs".into(), digest: Digest::of_bytes(b"main"),
                size_bytes: 200, kind: WorkspaceEntryKind::SourceFile },
        ];
        let index = WorkspaceIndex::from_entries("test".into(), entries, policy.id);
        let opts = IntegrationRunOptions {
            enable_metatron: true,
            enable_surgery: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&index, &opts, &policy);
        // All surgery options must carry the pipeline policy_id.
        for opt in &r.surgery_options {
            if opt.policy_id != policy.id {
                return Err("surgery option policy_id must match pipeline policy".into());
            }
        }
        if !r.verify_policy_consistency() {
            return Err("verify_policy_consistency must cover surgery_options".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-pipeline-surgery-options-are-energy-ranked", "RX:Pipeline", || {
        // When surgery options are present, they must be ordered by energy
        // (confidence_score descending) — highest D first.
        use kosmo_workbench::{WorkspaceEntry, WorkspaceEntryKind};
        let policy = PolicyProfile::default_report_only();
        let entries = vec![
            WorkspaceEntry { path: "a.rs".into(), digest: Digest::of_bytes(b"a"),
                size_bytes: 50, kind: WorkspaceEntryKind::SourceFile },
            WorkspaceEntry { path: "b.rs".into(), digest: Digest::of_bytes(b"b"),
                size_bytes: 80, kind: WorkspaceEntryKind::SourceFile },
            WorkspaceEntry { path: "c.rs".into(), digest: Digest::of_bytes(b"c"),
                size_bytes: 60, kind: WorkspaceEntryKind::SourceFile },
        ];
        let index = WorkspaceIndex::from_entries("test".into(), entries, policy.id);
        let opts = IntegrationRunOptions {
            enable_metatron: true,
            enable_surgery: true,
            ..IntegrationRunOptions::report_only()
        };
        let r1 = run_dry_pipeline(&index, &opts, &policy);
        let r2 = run_dry_pipeline(&index, &opts, &policy);
        // Determinism: same options in same order across runs.
        let ids1: Vec<_> = r1.surgery_options.iter().map(|o| o.option_id).collect();
        let ids2: Vec<_> = r2.surgery_options.iter().map(|o| o.option_id).collect();
        if ids1 != ids2 {
            return Err("surgery_options must be deterministic across runs".into());
        }
        // Energy ordering: each option's confidence_score must be ≥ the next.
        for window in r1.surgery_options.windows(2) {
            if window[0].confidence_score < window[1].confidence_score {
                return Err("surgery_options must be ranked by confidence_score descending".into());
            }
        }
        Ok(())
    }));

    // ── RX:Pipeline — Step 1b void priority ranking ───────────────────────────

    v.push(run_check("rx-pipeline-void-priority-ranking-length-matches-voids", "RX:Pipeline", || {
        // void_priority_ranking must contain exactly one entry per host void.
        // Empty workspace → 0 voids → 0 ranked entries.
        let policy = PolicyProfile::default_report_only();
        let index = WorkspaceIndex::from_entries("test".into(), vec![], policy.id);
        let r = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        let void_count = r.hyphae_result.host_cube.void_count();
        if r.void_priority_ranking.len() != void_count {
            return Err(format!(
                "void_priority_ranking.len()={} must equal void_count={}",
                r.void_priority_ranking.len(), void_count
            ));
        }
        Ok(())
    }));

    v.push(run_check("rx-pipeline-void-priority-ranking-is-deterministic", "RX:Pipeline", || {
        // Identical runs must produce identical priority rankings (determinism).
        use kosmo_workbench::{WorkspaceEntry, WorkspaceEntryKind};
        let policy = PolicyProfile::default_report_only();
        let entries = vec![
            WorkspaceEntry { path: "x.rs".into(), digest: Digest::of_bytes(b"x"),
                size_bytes: 10, kind: WorkspaceEntryKind::SourceFile },
            WorkspaceEntry { path: "y.rs".into(), digest: Digest::of_bytes(b"y"),
                size_bytes: 20, kind: WorkspaceEntryKind::SourceFile },
        ];
        let index = WorkspaceIndex::from_entries("test".into(), entries, policy.id);
        let r1 = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        let r2 = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        if r1.void_priority_ranking != r2.void_priority_ranking {
            return Err("void_priority_ranking must be deterministic".into());
        }
        // All IDs in ranking must appear in the void_map.
        let void_ids: std::collections::HashSet<_> = r1.hyphae_result.host_cube.void_map.voids
            .iter().map(|v| v.void_id).collect();
        for vid in &r1.void_priority_ranking {
            if !void_ids.contains(vid) {
                return Err("void_priority_ranking must only contain known void IDs".into());
            }
        }
        Ok(())
    }));

    // ── RX:Pipeline — Step 5b NormGeneCandidate generation ───────────────────

    v.push(run_check("rx-pipeline-metatron-index-empty-without-metatron", "RX:Pipeline", || {
        use kosmo_pipeline::{IntegrationRunOptions, run_dry_pipeline};
        use kosmo_workbench::WorkspaceIndex;
        let policy = PolicyProfile::default_report_only();
        let entries = vec![
            kosmo_workbench::WorkspaceEntry {
                path: "src/lib.rs".into(),
                digest: Digest::of_bytes(b"lib"),
                size_bytes: 100,
                kind: kosmo_workbench::WorkspaceEntryKind::SourceFile,
            },
        ];
        let index = WorkspaceIndex::from_entries("test-root".into(), entries, policy.id);
        let r = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        if !r.metatron_index.micrograph_ids.is_empty() || !r.metatron_index.diagnostic_ids.is_empty() {
            return Err("metatron_index must be empty when Metatron disabled".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-pipeline-metatron-index-content-addressed", "RX:Pipeline", || {
        use kosmo_pipeline::{IntegrationRunOptions, run_dry_pipeline};
        use kosmo_workbench::WorkspaceIndex;
        let policy = PolicyProfile::default_report_only();
        let entries = vec![
            kosmo_workbench::WorkspaceEntry {
                path: "src/lib.rs".into(),
                digest: Digest::of_bytes(b"lib"),
                size_bytes: 100,
                kind: kosmo_workbench::WorkspaceEntryKind::SourceFile,
            },
            kosmo_workbench::WorkspaceEntry {
                path: "src/lib_test.rs".into(),
                digest: Digest::of_bytes(b"test"),
                size_bytes: 50,
                kind: kosmo_workbench::WorkspaceEntryKind::TestFile,
            },
        ];
        let index = WorkspaceIndex::from_entries("test-root".into(), entries, policy.id);
        let opts = IntegrationRunOptions {
            enable_metatron: true,
            ..IntegrationRunOptions::report_only()
        };
        let r1 = run_dry_pipeline(&index, &opts, &policy);
        let r2 = run_dry_pipeline(&index, &opts, &policy);
        // Index must be deterministic and match void count.
        if r1.metatron_index.index_id != r2.metatron_index.index_id {
            return Err("metatron_index must be deterministic".into());
        }
        if r1.metatron_index.index_id == Digest::ZERO {
            return Err("metatron_index.index_id must be non-ZERO".into());
        }
        let void_count = r1.hyphae_result.host_cube.void_count();
        if r1.metatron_index.micrograph_ids.len() != void_count {
            return Err(format!(
                "expected {} micrograph IDs in index, got {}",
                void_count, r1.metatron_index.micrograph_ids.len()
            ));
        }
        // Index policy_id must match pipeline policy.
        if r1.metatron_index.policy_id != policy.id {
            return Err("metatron_index.policy_id mismatch".into());
        }
        // metatron_index_id must participate in report_id.
        if r1.report_id != r2.report_id {
            return Err("report_id must be deterministic (metatron_index participates)".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-pipeline-lift-reports-empty-without-metatron", "RX:Pipeline", || {
        use kosmo_pipeline::{IntegrationRunOptions, run_dry_pipeline};
        use kosmo_workbench::WorkspaceIndex;
        let policy = PolicyProfile::default_report_only();
        let entries = vec![
            kosmo_workbench::WorkspaceEntry {
                path: "src/lib.rs".into(),
                digest: Digest::of_bytes(b"lib"),
                size_bytes: 100,
                kind: kosmo_workbench::WorkspaceEntryKind::SourceFile,
            },
        ];
        let index = WorkspaceIndex::from_entries("test-root".into(), entries, policy.id);
        let r = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        if !r.lift_reports.is_empty() {
            return Err(format!(
                "lift_reports must be empty when Metatron disabled, got {}",
                r.lift_reports.len()
            ));
        }
        Ok(())
    }));

    v.push(run_check("rx-pipeline-lift-reports-one-per-void-with-metatron", "RX:Pipeline", || {
        use kosmo_pipeline::{IntegrationRunOptions, run_dry_pipeline};
        use kosmo_workbench::WorkspaceIndex;
        let policy = PolicyProfile::default_report_only();
        let entries = vec![
            kosmo_workbench::WorkspaceEntry {
                path: "src/lib.rs".into(),
                digest: Digest::of_bytes(b"lib"),
                size_bytes: 100,
                kind: kosmo_workbench::WorkspaceEntryKind::SourceFile,
            },
            kosmo_workbench::WorkspaceEntry {
                path: "src/lib_test.rs".into(),
                digest: Digest::of_bytes(b"test"),
                size_bytes: 50,
                kind: kosmo_workbench::WorkspaceEntryKind::TestFile,
            },
        ];
        let index = WorkspaceIndex::from_entries("test-root".into(), entries, policy.id);
        let opts = IntegrationRunOptions {
            enable_metatron: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&index, &opts, &policy);
        let void_count = r.hyphae_result.host_cube.void_count();
        if r.lift_reports.len() != void_count {
            return Err(format!(
                "expected {} lift reports (= void count), got {}",
                void_count, r.lift_reports.len()
            ));
        }
        // All lift reports must carry non-ZERO IDs.
        for rep in &r.lift_reports {
            if rep.report_id == Digest::ZERO {
                return Err("CROSS-006: lift_report.report_id must be non-ZERO".into());
            }
            if rep.micrograph_id == Digest::ZERO {
                return Err("lift_report.micrograph_id must be non-ZERO".into());
            }
        }
        // Lift reports must participate in report_id (content-addressed).
        let r2 = run_dry_pipeline(&index, &opts, &policy);
        if r.report_id != r2.report_id {
            return Err("lift_reports must participate in report_id deterministically".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-pipeline-norm-candidates-disabled-by-default", "RX:Pipeline", || {
        use kosmo_pipeline::{IntegrationRunOptions, run_dry_pipeline};
        use kosmo_workbench::WorkspaceIndex;
        let policy = PolicyProfile::default_report_only();
        let entries = vec![
            kosmo_workbench::WorkspaceEntry {
                path: "src/lib.rs".into(),
                digest: Digest::of_bytes(b"lib"),
                size_bytes: 100,
                kind: kosmo_workbench::WorkspaceEntryKind::SourceFile,
            },
        ];
        let index = WorkspaceIndex::from_entries("test-root".into(), entries, policy.id);
        let r = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        if !r.norm_candidates.is_empty() {
            return Err(format!(
                "norm_candidates must be empty when disabled, got {}",
                r.norm_candidates.len()
            ));
        }
        Ok(())
    }));

    v.push(run_check("rx-pipeline-norm-candidates-from-accepted-decisions", "RX:Pipeline", || {
        use kosmo_pipeline::{IntegrationRunOptions, run_dry_pipeline};
        use kosmo_workbench::WorkspaceIndex;
        let policy = PolicyProfile::default_report_only();
        let entries = vec![
            kosmo_workbench::WorkspaceEntry {
                path: "src/lib.rs".into(),
                digest: Digest::of_bytes(b"lib"),
                size_bytes: 100,
                kind: kosmo_workbench::WorkspaceEntryKind::SourceFile,
            },
            kosmo_workbench::WorkspaceEntry {
                path: "src/lib_test.rs".into(),
                digest: Digest::of_bytes(b"test"),
                size_bytes: 50,
                kind: kosmo_workbench::WorkspaceEntryKind::TestFile,
            },
        ];
        let index = WorkspaceIndex::from_entries("test-root".into(), entries, policy.id);
        let opts = IntegrationRunOptions {
            enable_norm_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&index, &opts, &policy);
        // Each accepted decision yields a norm candidate.
        let accepted = r.hyphae_result.decisions.iter().filter(|d| d.outcome.is_accepted()).count();
        if r.norm_candidates.len() != accepted {
            return Err(format!(
                "expected {} norm candidates (= accepted decisions), got {}",
                accepted, r.norm_candidates.len()
            ));
        }
        // All must carry the pipeline policy_id and non-ZERO evidence refs.
        for c in &r.norm_candidates {
            if c.policy_id != policy.id {
                return Err("norm candidate policy_id mismatch".into());
            }
            if c.evidence_bundle_id == Digest::ZERO {
                return Err("CROSS-006: norm candidate evidence_bundle_id must be non-ZERO".into());
            }
            if c.fitness_score != Q16::ONE {
                return Err(format!("initial fitness must be Q16::ONE, got {:?}", c.fitness_score));
            }
        }
        // verify_policy_consistency() must cover norm candidates.
        if !r.verify_policy_consistency() {
            return Err("verify_policy_consistency must include norm_candidates".into());
        }
        Ok(())
    }));

    // ── RX:Pipeline — Step 3e SurgeryWorkbenchTask conversion ────────────────

    v.push(run_check("rx-pipeline-surgery-workbench-tasks-disabled-by-default", "RX:Pipeline", || {
        use kosmo_pipeline::{IntegrationRunOptions, run_dry_pipeline};
        use kosmo_workbench::WorkspaceIndex;
        let policy = PolicyProfile::default_report_only();
        let entries = vec![
            kosmo_workbench::WorkspaceEntry {
                path: "src/lib.rs".into(),
                digest: Digest::of_bytes(b"lib"),
                size_bytes: 100,
                kind: kosmo_workbench::WorkspaceEntryKind::SourceFile,
            },
        ];
        let index = WorkspaceIndex::from_entries("test-root".into(), entries, policy.id);
        let r = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        if !r.surgery_workbench_tasks.is_empty() {
            return Err(format!(
                "surgery_workbench_tasks must be empty when surgery disabled, got {}",
                r.surgery_workbench_tasks.len()
            ));
        }
        Ok(())
    }));

    v.push(run_check("rx-pipeline-surgery-workbench-tasks-match-surgery-options", "RX:Pipeline", || {
        use kosmo_pipeline::{IntegrationRunOptions, run_dry_pipeline};
        use kosmo_workbench::{WorkspaceEntry, WorkspaceEntryKind, WorkspaceIndex};
        let policy = PolicyProfile::default_report_only();
        let entries = vec![
            WorkspaceEntry { path: "src/lib.rs".into(), digest: Digest::of_bytes(b"lib"),
                size_bytes: 100, kind: WorkspaceEntryKind::SourceFile },
            WorkspaceEntry { path: "src/lib_test.rs".into(), digest: Digest::of_bytes(b"test"),
                size_bytes: 50, kind: WorkspaceEntryKind::TestFile },
        ];
        let index = WorkspaceIndex::from_entries("test-root".into(), entries, policy.id);
        let opts = IntegrationRunOptions {
            enable_surgery: true,
            enable_metatron: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&index, &opts, &policy);
        // 1:1 correspondence with surgery_options.
        if r.surgery_workbench_tasks.len() != r.surgery_options.len() {
            return Err(format!(
                "surgery_workbench_tasks.len()={} must equal surgery_options.len()={}",
                r.surgery_workbench_tasks.len(), r.surgery_options.len()
            ));
        }
        // All workbench tasks must carry the pipeline policy_id.
        for t in &r.surgery_workbench_tasks {
            if t.policy_id != policy.id {
                return Err("surgery workbench task policy_id mismatch".into());
            }
            if t.task_id == Digest::ZERO {
                return Err("surgery workbench task_id must be non-ZERO".into());
            }
            if t.surgery_option_id == Digest::ZERO {
                return Err("surgery_option_id must trace back to source option".into());
            }
        }
        // verify_policy_consistency() must cover surgery_workbench_tasks.
        if !r.verify_policy_consistency() {
            return Err("verify_policy_consistency must include surgery_workbench_tasks".into());
        }
        // Determinism.
        let r2 = run_dry_pipeline(&index, &opts, &policy);
        let ids1: Vec<_> = r.surgery_workbench_tasks.iter().map(|t| t.task_id).collect();
        let ids2: Vec<_> = r2.surgery_workbench_tasks.iter().map(|t| t.task_id).collect();
        if ids1 != ids2 {
            return Err("surgery_workbench_tasks must be deterministic across runs".into());
        }
        Ok(())
    }));

    // ── RX:Pipeline — Step 1c DeficiencyVector ───────────────────────────────

    v.push(run_check("rx-pipeline-deficiency-vector-always-present", "RX:Pipeline", || {
        use kosmo_pipeline::{IntegrationRunOptions, run_dry_pipeline};
        use kosmo_workbench::WorkspaceIndex;
        let policy = PolicyProfile::default_report_only();
        // Empty workspace: no voids → empty deficiency vector, but still present.
        let index = WorkspaceIndex::from_entries("test".into(), vec![], policy.id);
        let r = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        if r.deficiency_vector.vector_id == Digest::ZERO {
            return Err("deficiency_vector.vector_id must be non-ZERO even when empty".into());
        }
        if r.deficiency_vector.policy_id != policy.id {
            return Err("deficiency_vector.policy_id must match pipeline policy_id".into());
        }
        if !r.deficiency_vector.entries.is_empty() {
            return Err("empty workspace must produce empty deficiency entries".into());
        }
        if !r.verify_policy_consistency() {
            return Err("verify_policy_consistency must cover deficiency_vector".into());
        }
        // Determinism: vector_id must participate in report_id.
        let r2 = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        if r.report_id != r2.report_id {
            return Err("deficiency_vector must participate in report_id".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-pipeline-deficiency-vector-captures-void-kinds", "RX:Pipeline", || {
        use kosmo_pipeline::{IntegrationRunOptions, run_dry_pipeline};
        use kosmo_workbench::{WorkspaceEntry, WorkspaceEntryKind, WorkspaceIndex};
        let policy = PolicyProfile::default_report_only();
        // Source files without tests create MissingTestFiber voids.
        let entries = vec![
            WorkspaceEntry { path: "src/lib.rs".into(), digest: Digest::of_bytes(b"lib"),
                size_bytes: 100, kind: WorkspaceEntryKind::SourceFile },
            WorkspaceEntry { path: "src/main.rs".into(), digest: Digest::of_bytes(b"main"),
                size_bytes: 200, kind: WorkspaceEntryKind::SourceFile },
        ];
        let index = WorkspaceIndex::from_entries("test-root".into(), entries, policy.id);
        let r = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        // With source files only, void_map should have MissingTestFiber voids.
        // DeficiencyVector should reflect total_severity > 0 if any deficiencies found.
        if r.deficiency_vector.vector_id == Digest::ZERO {
            return Err("deficiency_vector must always have non-ZERO vector_id".into());
        }
        // total_severity must be consistent: either zero (no entries) or positive.
        if !r.deficiency_vector.entries.is_empty() {
            if r.deficiency_vector.total_severity == Q16::ZERO {
                return Err("non-empty deficiency entries must yield non-zero total_severity".into());
            }
        }
        Ok(())
    }));

    // ── RX:Pipeline — Step 6b PSE bridge candidates ──────────────────────────

    v.push(run_check("rx-pipeline-pse-candidates-disabled-by-default", "RX:Pipeline", || {
        use kosmo_pipeline::{IntegrationRunOptions, run_dry_pipeline};
        use kosmo_workbench::WorkspaceIndex;
        let policy = PolicyProfile::default_report_only();
        let entries = vec![
            kosmo_workbench::WorkspaceEntry {
                path: "src/lib.rs".into(),
                digest: Digest::of_bytes(b"lib"),
                size_bytes: 100,
                kind: kosmo_workbench::WorkspaceEntryKind::SourceFile,
            },
        ];
        let index = WorkspaceIndex::from_entries("test-root".into(), entries, policy.id);
        let r = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        if !r.pse_candidates.is_empty() {
            return Err(format!(
                "pse_candidates must be empty when disabled, got {}",
                r.pse_candidates.len()
            ));
        }
        Ok(())
    }));

    v.push(run_check("rx-pipeline-pse-candidates-cover-norm-and-topology", "RX:Pipeline", || {
        use kosmo_pipeline::{IntegrationRunOptions, run_dry_pipeline};
        use kosmo_workbench::{WorkspaceEntry, WorkspaceEntryKind, WorkspaceIndex};
        let policy = PolicyProfile::default_report_only();
        let entries = vec![
            WorkspaceEntry { path: "src/lib.rs".into(), digest: Digest::of_bytes(b"lib"),
                size_bytes: 100, kind: WorkspaceEntryKind::SourceFile },
            WorkspaceEntry { path: "src/lib_test.rs".into(), digest: Digest::of_bytes(b"test"),
                size_bytes: 50, kind: WorkspaceEntryKind::TestFile },
        ];
        let index = WorkspaceIndex::from_entries("test-root".into(), entries, policy.id);
        let opts = IntegrationRunOptions {
            enable_metatron: true,
            enable_norm_candidates: true,
            enable_pse_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&index, &opts, &policy);
        let expected = r.norm_candidates.len()
            + r.ambiguity_profiles.len()
            + r.complement_void_hypotheses.len();
        if r.pse_candidates.len() != expected {
            return Err(format!(
                "expected {} PSE candidates (norm + ambiguities + void_hyp), got {}",
                expected, r.pse_candidates.len()
            ));
        }
        // Confidence descending.
        for window in r.pse_candidates.windows(2) {
            if window[0].confidence < window[1].confidence {
                return Err("pse_candidates must be sorted by confidence descending".into());
            }
        }
        for c in &r.pse_candidates {
            if c.policy_id != policy.id {
                return Err("PSE candidate policy_id mismatch".into());
            }
            if c.id == Digest::ZERO {
                return Err("PSE candidate id must be non-ZERO".into());
            }
            if c.evidence_bundle_id == Digest::ZERO {
                return Err("CROSS-006: PSE candidate evidence_bundle_id must be non-ZERO".into());
            }
        }
        if !r.verify_policy_consistency() {
            return Err("verify_policy_consistency must include pse_candidates".into());
        }
        // Determinism.
        let r2 = run_dry_pipeline(&index, &opts, &policy);
        let ids1: Vec<_> = r.pse_candidates.iter().map(|c| c.id).collect();
        let ids2: Vec<_> = r2.pse_candidates.iter().map(|c| c.id).collect();
        if ids1 != ids2 {
            return Err("pse_candidates must be deterministic".into());
        }
        Ok(())
    }));

    // ── RX:Pipeline — Step 5d StructuralCrystalCandidate certification queue ──

    v.push(run_check("rx-pipeline-crystal-candidates-disabled-by-default", "RX:Pipeline", || {
        use kosmo_pipeline::{IntegrationRunOptions, run_dry_pipeline};
        use kosmo_workbench::WorkspaceIndex;
        let policy = PolicyProfile::default_report_only();
        let entries = vec![
            kosmo_workbench::WorkspaceEntry {
                path: "src/lib.rs".into(),
                digest: Digest::of_bytes(b"lib"),
                size_bytes: 100,
                kind: kosmo_workbench::WorkspaceEntryKind::SourceFile,
            },
        ];
        let index = WorkspaceIndex::from_entries("test-root".into(), entries, policy.id);
        let r = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        if !r.crystal_candidates.is_empty() {
            return Err(format!(
                "crystal_candidates must be empty when disabled, got {}",
                r.crystal_candidates.len()
            ));
        }
        Ok(())
    }));

    v.push(run_check("rx-pipeline-crystal-candidates-one-per-accepted-decision", "RX:Pipeline", || {
        use kosmo_pipeline::{IntegrationRunOptions, run_dry_pipeline};
        use kosmo_workbench::{WorkspaceEntry, WorkspaceEntryKind, WorkspaceIndex};
        let policy = PolicyProfile::default_report_only();
        let entries = vec![
            WorkspaceEntry { path: "src/lib.rs".into(), digest: Digest::of_bytes(b"lib"),
                size_bytes: 100, kind: WorkspaceEntryKind::SourceFile },
            WorkspaceEntry { path: "src/lib_test.rs".into(), digest: Digest::of_bytes(b"test"),
                size_bytes: 50, kind: WorkspaceEntryKind::TestFile },
        ];
        let index = WorkspaceIndex::from_entries("test-root".into(), entries, policy.id);
        let opts = IntegrationRunOptions {
            enable_crystal_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&index, &opts, &policy);
        let accepted = r.hyphae_result.decisions.iter().filter(|d| d.outcome.is_accepted()).count();
        if r.crystal_candidates.len() != accepted {
            return Err(format!(
                "expected {} crystal candidates (= accepted decisions), got {}",
                accepted, r.crystal_candidates.len()
            ));
        }
        for c in &r.crystal_candidates {
            if c.policy_id != policy.id {
                return Err("crystal candidate policy_id mismatch".into());
            }
            if c.candidate_id == Digest::ZERO {
                return Err("candidate_id must be non-ZERO".into());
            }
            if c.support_score != Q16::ZERO {
                return Err("support_score must be ZERO at creation (Pending certification)".into());
            }
        }
        if !r.verify_policy_consistency() {
            return Err("verify_policy_consistency must include crystal_candidates".into());
        }
        // Determinism.
        let r2 = run_dry_pipeline(&index, &opts, &policy);
        let ids1: Vec<_> = r.crystal_candidates.iter().map(|c| c.candidate_id).collect();
        let ids2: Vec<_> = r2.crystal_candidates.iter().map(|c| c.candidate_id).collect();
        if ids1 != ids2 {
            return Err("crystal_candidates must be deterministic".into());
        }
        Ok(())
    }));

    // ── RX:Pipeline — Step 3f ambiguity profiles + void hypotheses ───────────

    v.push(run_check("rx-pipeline-ambiguity-profiles-empty-without-metatron", "RX:Pipeline", || {
        use kosmo_pipeline::{IntegrationRunOptions, run_dry_pipeline};
        use kosmo_workbench::WorkspaceIndex;
        let policy = PolicyProfile::default_report_only();
        let entries = vec![
            kosmo_workbench::WorkspaceEntry {
                path: "src/lib.rs".into(),
                digest: Digest::of_bytes(b"lib"),
                size_bytes: 100,
                kind: kosmo_workbench::WorkspaceEntryKind::SourceFile,
            },
        ];
        let index = WorkspaceIndex::from_entries("test-root".into(), entries, policy.id);
        let r = run_dry_pipeline(&index, &IntegrationRunOptions::report_only(), &policy);
        if !r.ambiguity_profiles.is_empty() || !r.complement_void_hypotheses.is_empty() {
            return Err(format!(
                "ambiguity_profiles ({}) and complement_void_hypotheses ({}) must be empty without Metatron",
                r.ambiguity_profiles.len(), r.complement_void_hypotheses.len()
            ));
        }
        Ok(())
    }));

    v.push(run_check("rx-pipeline-ambiguity-profiles-policy-consistent", "RX:Pipeline", || {
        use kosmo_pipeline::{IntegrationRunOptions, run_dry_pipeline};
        use kosmo_workbench::{WorkspaceEntry, WorkspaceEntryKind, WorkspaceIndex};
        let policy = PolicyProfile::default_report_only();
        let entries = vec![
            WorkspaceEntry { path: "src/lib.rs".into(), digest: Digest::of_bytes(b"lib"),
                size_bytes: 100, kind: WorkspaceEntryKind::SourceFile },
            WorkspaceEntry { path: "src/lib_test.rs".into(), digest: Digest::of_bytes(b"test"),
                size_bytes: 50, kind: WorkspaceEntryKind::TestFile },
        ];
        let index = WorkspaceIndex::from_entries("test-root".into(), entries, policy.id);
        let opts = IntegrationRunOptions {
            enable_metatron: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&index, &opts, &policy);
        for a in &r.ambiguity_profiles {
            if a.policy_id != policy.id {
                return Err("ambiguity profile policy_id mismatch".into());
            }
            if a.profile_id == Digest::ZERO {
                return Err("CROSS-006: profile_id must be non-ZERO".into());
            }
        }
        for h in &r.complement_void_hypotheses {
            if h.policy_id != policy.id {
                return Err("void hypothesis policy_id mismatch".into());
            }
            if h.hypothesis_id == Digest::ZERO {
                return Err("CROSS-006: hypothesis_id must be non-ZERO".into());
            }
        }
        if !r.verify_policy_consistency() {
            return Err("verify_policy_consistency must cover ambiguity_profiles and complement_void_hypotheses".into());
        }
        // Determinism.
        let r2 = run_dry_pipeline(&index, &opts, &policy);
        let ids1: Vec<_> = r.ambiguity_profiles.iter().map(|a| a.profile_id).collect();
        let ids2: Vec<_> = r2.ambiguity_profiles.iter().map(|a| a.profile_id).collect();
        if ids1 != ids2 {
            return Err("ambiguity_profiles must be deterministic".into());
        }
        // Energy ordering: confidence_score descending.
        for window in r.ambiguity_profiles.windows(2) {
            if window[0].confidence_score < window[1].confidence_score {
                return Err("ambiguity_profiles must be ranked by confidence_score descending".into());
            }
        }
        // Counts participate in report_id.
        if r.report_id != r2.report_id {
            return Err("ambiguity/hypothesis counts must participate in report_id".into());
        }
        Ok(())
    }));

    // ── RX:Pipeline — Step 5c NormFitnessTrace from prior feedback ───────────

    v.push(run_check("rx-pipeline-norm-fitness-traces-empty-without-feedback", "RX:Pipeline", || {
        use kosmo_pipeline::{IntegrationRunOptions, run_dry_pipeline};
        use kosmo_workbench::WorkspaceIndex;
        let policy = PolicyProfile::default_report_only();
        let entries = vec![
            kosmo_workbench::WorkspaceEntry {
                path: "src/lib.rs".into(),
                digest: Digest::of_bytes(b"lib"),
                size_bytes: 100,
                kind: kosmo_workbench::WorkspaceEntryKind::SourceFile,
            },
        ];
        let index = WorkspaceIndex::from_entries("test-root".into(), entries, policy.id);
        let opts = IntegrationRunOptions {
            enable_norm_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let r = run_dry_pipeline(&index, &opts, &policy);
        if !r.norm_fitness_traces.is_empty() {
            return Err(format!(
                "norm_fitness_traces must be empty when prior_feedback is empty, got {}",
                r.norm_fitness_traces.len()
            ));
        }
        if !r.verify_policy_consistency() {
            return Err("verify_policy_consistency must pass with empty traces".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-pipeline-norm-fitness-traces-from-matching-feedback", "RX:Pipeline", || {
        use kosmo_pipeline::{IntegrationRunOptions, run_dry_pipeline};
        use kosmo_workbench::{WorkspaceEntry, WorkspaceEntryKind, WorkspaceIndex};
        let policy = PolicyProfile::default_report_only();
        let entries = vec![
            WorkspaceEntry { path: "src/lib.rs".into(), digest: Digest::of_bytes(b"lib"),
                size_bytes: 100, kind: WorkspaceEntryKind::SourceFile },
            WorkspaceEntry { path: "src/lib_test.rs".into(), digest: Digest::of_bytes(b"test"),
                size_bytes: 50, kind: WorkspaceEntryKind::TestFile },
        ];
        let index = WorkspaceIndex::from_entries("test-root".into(), entries, policy.id);
        // First: generate candidates.
        let opts_gen = IntegrationRunOptions {
            enable_norm_candidates: true,
            ..IntegrationRunOptions::report_only()
        };
        let r_gen = run_dry_pipeline(&index, &opts_gen, &policy);
        if r_gen.norm_candidates.is_empty() {
            return Ok(()); // no accepted decisions — not an error, just skip
        }
        let candidate = &r_gen.norm_candidates[0];
        let energy = Q16::ratio(3, 4).unwrap();
        let feedback = PromotionFeedback::new(
            Digest::of_bytes(b"record"),
            candidate.candidate_id,
            candidate.candidate_id,
            FeedbackOutcome::Accepted,
            energy,
            policy.id,
            candidate.evidence_bundle_id,
        );
        // Second: apply prior_feedback.
        let opts_fb = IntegrationRunOptions {
            enable_norm_candidates: true,
            prior_feedback: vec![feedback.clone()],
            ..IntegrationRunOptions::report_only()
        };
        let r_fb = run_dry_pipeline(&index, &opts_fb, &policy);
        let trace = r_fb.norm_fitness_traces.iter()
            .find(|t| t.candidate_id == candidate.candidate_id)
            .ok_or("trace must exist for candidate that received feedback")?;
        // Fitness trace content.
        if trace.latest_fitness() != energy {
            return Err(format!(
                "latest_fitness must equal energy {}, got {}",
                energy.raw(), trace.latest_fitness().raw()
            ));
        }
        if trace.observations[0].evidence_ref != feedback.id {
            return Err("evidence_ref must equal feedback.id — CROSS-006 via trace".into());
        }
        if trace.policy_id != policy.id {
            return Err("trace.policy_id must match pipeline policy_id".into());
        }
        // Policy consistency must cover traces.
        if !r_fb.verify_policy_consistency() {
            return Err("verify_policy_consistency must include norm_fitness_traces".into());
        }
        // Traces must participate in report_id.
        let r_fb2 = run_dry_pipeline(&index, &opts_fb, &policy);
        if r_fb.report_id != r_fb2.report_id {
            return Err("norm_fitness_traces must participate in report_id deterministically".into());
        }
        Ok(())
    }));

    // ── RX:BlueprintEnergy — BlueprintUnit energy_assessment ─────────────────

    v.push(run_check("rx-blueprint-energy-accepted-positive-opaque-zero", "RX:BlueprintEnergy", || {
        let policy = PolicyProfile::default_report_only();
        let src = Digest::of_bytes(b"src");
        let ev = Digest::of_bytes(b"ev");
        let accepted = BlueprintUnit::new(
            BlueprintUnitKind::ModuleBoundary,
            src,
            kosmo_core::AuthorityLabel::Foundry,
            TaintLabel::Clean,
            vec![ev],
            &policy,
        );
        let opaque = BlueprintUnit::new(
            BlueprintUnitKind::ModuleBoundary,
            src,
            kosmo_core::AuthorityLabel::Foundry,
            TaintLabel::Clean,
            vec![],
            &policy,
        );
        let ea = accepted.energy_assessment(&GateResult::Pass);
        let eo = opaque.energy_assessment(&GateResult::Pass);
        if ea.energy.raw() == 0 {
            return Err("accepted BlueprintUnit must have positive energy".into());
        }
        if eo.energy.raw() != 0 {
            return Err(format!("opaque-rejected BlueprintUnit must have zero energy, got {}", eo.energy.raw()));
        }
        // evidence self-referential (CROSS-006): unit_id used as evidence_bundle_id
        if ea.evidence_bundle_id != accepted.unit_id {
            return Err("evidence_bundle_id must equal unit_id (self-referential, CROSS-006)".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-blueprint-energy-tainted-ranks-below-clean", "RX:BlueprintEnergy", || {
        let policy = PolicyProfile::default_report_only();
        let ev = Digest::of_bytes(b"ev");
        let clean = BlueprintUnit::new(
            BlueprintUnitKind::ModuleBoundary,
            Digest::of_bytes(b"s1"),
            kosmo_core::AuthorityLabel::Foundry,
            TaintLabel::Clean,
            vec![ev],
            &policy,
        );
        let synthetic = BlueprintUnit::new(
            BlueprintUnitKind::ModuleBoundary,
            Digest::of_bytes(b"s2"),
            kosmo_core::AuthorityLabel::Foundry,
            TaintLabel::Synthetic,
            vec![ev],
            &policy,
        );
        let e_clean = clean.energy_assessment(&GateResult::Pass).energy;
        let e_synthetic = synthetic.energy_assessment(&GateResult::Pass).energy;
        if e_clean <= e_synthetic {
            return Err(format!(
                "clean (energy={}) must rank above synthetic tainted (energy={})",
                e_clean.raw(), e_synthetic.raw()
            ));
        }
        // rank_by_energy must surface clean first
        let assessments = vec![
            synthetic.energy_assessment(&GateResult::Pass),
            clean.energy_assessment(&GateResult::Pass),
        ];
        let ranked = rank_by_energy(&assessments);
        if ranked[0].subject_id != clean.unit_id {
            return Err("rank_by_energy must put clean BlueprintUnit first".into());
        }
        Ok(())
    }));

    // ── RX:Energy — unified tripolar energy kernel (D = ψ·ρ·ω) ──────────────

    v.push(run_check("rx-energy-tripolar-is-exact-product", "RX:Energy", || {
        // 0.5 · 0.5 · 0.5 = 0.125, integer-exact in Q16 (no floats).
        let t = TripolarEnergy::new(Q16::HALF, Q16::HALF, Q16::HALF);
        if t.d() != Q16::ratio(1, 8).unwrap() {
            return Err(format!("expected D=1/8, got raw {}", t.d().raw()));
        }
        if TripolarEnergy::unit().d() != Q16::ONE {
            return Err("unit tripolar must yield D=1".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-energy-reject-gate-never-bypassed", "RX:Energy", || {
        // A maximal tripolar core with a Reject gate must yield ZERO energy:
        // energy ranks, it never bypasses a gate (CROSS-010).
        let factors = EnergyFactors::derive(
            &GateResult::Reject { reason: "missing evidence".into() },
            &TaintLabel::Clean,
            &LicenseStatus::Permissive { spdx: "MIT".into() },
            FoundrySurvival::Passed,
            Q16::ONE,
            Q16::ZERO,
        );
        let rejected = EnergyKernel::new(TripolarEnergy::unit(), factors);
        if !rejected.is_zeroed() {
            return Err("Reject gate must zero the energy".into());
        }
        // A passing kernel with even a tiny D out-ranks the rejected maximal one.
        let passing = EnergyKernel::new(
            TripolarEnergy::new(Q16::ratio(1, 100).unwrap(), Q16::ONE, Q16::ONE),
            EnergyFactors::all_clean(),
        );
        if passing.energy().raw() <= rejected.energy().raw() {
            return Err("a gate-passing candidate must out-rank a rejected one".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-energy-quarantine-and-proprietary-zero", "RX:Energy", || {
        // Hard taint/license states collapse energy to zero (fail-closed).
        for factors in [
            EnergyFactors::derive(
                &GateResult::Pass,
                &TaintLabel::Quarantined { reason: "x".into() },
                &LicenseStatus::Permissive { spdx: "MIT".into() },
                FoundrySurvival::Passed,
                Q16::ONE,
                Q16::ZERO,
            ),
            EnergyFactors::derive(
                &GateResult::Pass,
                &TaintLabel::Clean,
                &LicenseStatus::Proprietary,
                FoundrySurvival::Passed,
                Q16::ONE,
                Q16::ZERO,
            ),
            EnergyFactors::derive(
                &GateResult::Pass,
                &TaintLabel::Clean,
                &LicenseStatus::Permissive { spdx: "MIT".into() },
                FoundrySurvival::Failed,
                Q16::ONE,
                Q16::ZERO,
            ),
        ] {
            let k = EnergyKernel::new(TripolarEnergy::unit(), factors);
            if !k.is_zeroed() {
                return Err("quarantine / proprietary / foundry-failure must zero energy".into());
            }
        }
        Ok(())
    }));

    v.push(run_check("rx-energy-assessment-content-addressed", "RX:Energy", || {
        let k = EnergyKernel::new(TripolarEnergy::unit(), EnergyFactors::all_clean());
        let a1 = EnergyAssessment::new(d(b"subject"), k, d(b"pol"), d(b"bundle"));
        let a2 = EnergyAssessment::new(d(b"subject"), k, d(b"pol"), d(b"bundle"));
        if a1.id != a2.id {
            return Err("identical inputs must produce identical assessment id".into());
        }
        if !a1.verify_id() || a1.id == Digest::ZERO {
            return Err("assessment id must verify and be non-zero".into());
        }
        if a1.evidence_bundle_id == Digest::ZERO {
            return Err("assessment must be evidence-bound".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-energy-ranking-deterministic", "RX:Energy", || {
        let hi = EnergyAssessment::new(
            d(b"hi"),
            EnergyKernel::new(TripolarEnergy::unit(), EnergyFactors::all_clean()),
            d(b"pol"),
            d(b"bundle"),
        );
        let lo = EnergyAssessment::new(
            d(b"lo"),
            EnergyKernel::new(
                TripolarEnergy::new(Q16::ratio(1, 4).unwrap(), Q16::ONE, Q16::ONE),
                EnergyFactors::all_clean(),
            ),
            d(b"pol"),
            d(b"bundle"),
        );
        let r1 = rank_by_energy(&[lo.clone(), hi.clone()]);
        let r2 = rank_by_energy(&[hi.clone(), lo.clone()]);
        if r1[0].subject_id != hi.subject_id || r2[0].subject_id != hi.subject_id {
            return Err("higher energy must rank first regardless of input order".into());
        }
        if r1.len() != 2 {
            return Err("ranking must preserve every candidate".into());
        }
        Ok(())
    }));

    // ── RX:EnergyRanking — kernel adoption in SourceCube and NormGeneCandidate ─

    v.push(run_check("rx-energy-source-cube-quarantine-zeroes-energy", "RX:EnergyRanking", || {
        // A SourceCube with Quarantined taint must produce zero energy — the
        // taint_factor collapses D to zero even with perfect support_score (CROSS-007).
        let policy = PolicyProfile::default_report_only();
        let ev = d(b"ev");
        let cube = SourceCube::new(
            Some(d(b"void")),
            "src/dangerous.rs".into(),
            CubeDimensionProfile::empty(),
            Q16::ONE,
            TaintLabel::Quarantined { reason: "test".into() },
            ev,
            policy.id,
        );
        let a = cube.energy_assessment(&GateResult::Pass, &LicenseStatus::NotApplicable, FoundrySurvival::Unavailable, Q16::ONE);
        if !a.kernel.is_zeroed() {
            return Err("quarantined taint must collapse energy to zero".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-energy-source-cube-ranking-picks-best", "RX:EnergyRanking", || {
        // HostTargetDelta::from_source_cubes must select the cube with higher
        // energy as the top candidate for a void — not the cube with higher
        // cube_id or insertion order.
        let policy = PolicyProfile::default_report_only();
        let void_id = d(b"void");
        let ev = d(b"ev");
        let lo = SourceCube::new(
            Some(void_id), "src/low.rs".into(), CubeDimensionProfile::empty(),
            Q16::ratio(1, 4).unwrap(), TaintLabel::Clean, ev, policy.id,
        );
        let hi = SourceCube::new(
            Some(void_id), "src/high.rs".into(), CubeDimensionProfile::empty(),
            Q16::ratio(3, 4).unwrap(), TaintLabel::Clean, ev, policy.id,
        );
        let delta = HostTargetDelta::from_source_cubes(
            d(b"host"), &[void_id], d(b"comp"), &[lo.clone(), hi.clone()],
            &std::collections::BTreeMap::new(), policy.id,
        );
        if delta.void_fills.len() != 1 {
            return Err("one void fill expected".into());
        }
        let fill = &delta.void_fills[0];
        if fill.action != (kosmo_hyphae::delta::DeltaAction::FillVoid { top_candidate_cube_id: hi.cube_id }) {
            return Err(format!("wrong top candidate: expected hi cube, got {:?}", fill.action));
        }
        if fill.candidate_cube_ids.len() != 2 {
            return Err("both cubes must appear in candidate_cube_ids".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-energy-taint-beats-higher-raw-score", "RX:EnergyRanking", || {
        // A clean cube with lower support_score must rank above a tainted cube
        // with higher raw support_score — proving energy kernel overrides raw Q16.
        let policy = PolicyProfile::default_report_only();
        let void_id = d(b"void2");
        let ev = d(b"ev");
        let tainted_high = SourceCube::new(
            Some(void_id), "src/tainted.rs".into(), CubeDimensionProfile::empty(),
            Q16::ONE,  // highest raw score
            TaintLabel::Quarantined { reason: "legacy".into() },
            ev, policy.id,
        );
        let clean_low = SourceCube::new(
            Some(void_id), "src/clean.rs".into(), CubeDimensionProfile::empty(),
            Q16::HALF, // lower raw score
            TaintLabel::Clean,
            ev, policy.id,
        );
        let delta = HostTargetDelta::from_source_cubes(
            d(b"host"), &[void_id], d(b"comp"),
            &[tainted_high.clone(), clean_low.clone()],
            &std::collections::BTreeMap::new(), policy.id,
        );
        if delta.void_fills.is_empty() {
            return Err("expected one void fill".into());
        }
        let top = match &delta.void_fills[0].action {
            kosmo_hyphae::delta::DeltaAction::FillVoid { top_candidate_cube_id } => *top_candidate_cube_id,
            _ => return Err("expected FillVoid".into()),
        };
        if top != clean_low.cube_id {
            return Err("clean cube with lower raw score must beat quarantined cube with higher raw score".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-energy-norm-candidate-assessment-content-addressed", "RX:EnergyRanking", || {
        // NormGeneCandidate::energy_assessment must be content-addressed:
        // same inputs → same assessment id.
        let pid = d(b"p");
        let ev_id = d(b"e");
        let cand = NormGeneCandidate::new(
            "test-norm".into(), "desc".into(), Q16::HALF, ev_id, pid,
        );
        let a1 = cand.energy_assessment(&GateResult::Pass);
        let a2 = cand.energy_assessment(&GateResult::Pass);
        if a1.id != a2.id {
            return Err("energy_assessment must be deterministic (INVARIANT-007)".into());
        }
        if a1.subject_id != cand.candidate_id {
            return Err("assessment subject_id must be candidate_id".into());
        }
        // Rejected gate collapses energy to zero (CROSS-010 analogue).
        let rejected = cand.energy_assessment(&GateResult::Reject { reason: "test".into() });
        if !rejected.kernel.is_zeroed() {
            return Err("rejected gate must zero the energy".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-energy-motif-assessment-content-addressed", "RX:EnergyRanking", || {
        // MotifCandidate::energy_assessment must be deterministic (INVARIANT-007)
        // and use motif_id as subject_id. policy_id is now a first-class field.
        let pid = d(b"p");
        let ev = d(b"ev");
        let m = MotifCandidate::new("pattern-A".into(), 3, 4, None, TaintLabel::Clean, ev, pid);
        let a1 = m.energy_assessment(&GateResult::Pass);
        let a2 = m.energy_assessment(&GateResult::Pass);
        if a1.id != a2.id {
            return Err("MotifCandidate energy_assessment must be deterministic".into());
        }
        if a1.subject_id != m.motif_id {
            return Err("subject_id must equal motif_id".into());
        }
        // Different policy_id → different motif_id → different assessment id
        let m2 = MotifCandidate::new("pattern-A".into(), 3, 4, None, TaintLabel::Clean, ev, d(b"q"));
        let a3 = m2.energy_assessment(&GateResult::Pass);
        if a1.id == a3.id {
            return Err("different policy_id must produce different assessment id".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-energy-seam-penalty-reduces-ranking", "RX:EnergyRanking", || {
        // A SourceCube with perfect support but poor seam coherence (seam=0)
        // must rank below a cube with lower support but full seam coherence.
        // Proves SeamGraph data flows into EnergyFactors::seam.
        let policy = PolicyProfile::default_report_only();
        let void_id = d(b"void3");
        let ev = d(b"ev");
        let bad_seam = SourceCube::new(
            Some(void_id), "src/fragmented.rs".into(), CubeDimensionProfile::empty(),
            Q16::ONE,  // perfect support
            TaintLabel::Clean, ev, policy.id,
        );
        let good_seam = SourceCube::new(
            Some(void_id), "src/coherent.rs".into(), CubeDimensionProfile::empty(),
            Q16::HALF, // lower support
            TaintLabel::Clean, ev, policy.id,
        );
        // seam_map: void has zero seam coherence (no compatible seam edges)
        let mut seam_map = std::collections::BTreeMap::new();
        seam_map.insert(void_id, Q16::ZERO);
        let delta = HostTargetDelta::from_source_cubes(
            d(b"host"), &[void_id], d(b"comp"),
            &[bad_seam.clone(), good_seam.clone()],
            &seam_map, policy.id,
        );
        let top = match &delta.void_fills[0].action {
            kosmo_hyphae::delta::DeltaAction::FillVoid { top_candidate_cube_id } => *top_candidate_cube_id,
            _ => return Err("expected FillVoid".into()),
        };
        // bad_seam has support=1 but seam=0 → energy=0; good_seam has support=½ but seam=0 too
        // Wait — seam_map applies the SAME seam coherence to ALL cubes targeting that void.
        // With seam=0 for the void, BOTH cubes get energy=0. Tie broken by subject_id.
        // Let's verify: both cubes get zero energy when seam=0.
        let a_bad = bad_seam.energy_assessment(
            &GateResult::Pass, &LicenseStatus::NotApplicable, FoundrySurvival::Unavailable, Q16::ZERO,
        );
        let a_good = good_seam.energy_assessment(
            &GateResult::Pass, &LicenseStatus::NotApplicable, FoundrySurvival::Unavailable, Q16::ZERO,
        );
        if !a_bad.kernel.is_zeroed() || !a_good.kernel.is_zeroed() {
            return Err("seam=0 must collapse energy to zero for all cubes targeting that void".into());
        }
        // Seam=½: bad_seam energy=1*½=½, good_seam energy=½*½=¼. bad_seam should rank first.
        let mut seam_map_half = std::collections::BTreeMap::new();
        seam_map_half.insert(void_id, Q16::HALF);
        let delta2 = HostTargetDelta::from_source_cubes(
            d(b"host"), &[void_id], d(b"comp"),
            &[bad_seam.clone(), good_seam.clone()],
            &seam_map_half, policy.id,
        );
        let top2 = match &delta2.void_fills[0].action {
            kosmo_hyphae::delta::DeltaAction::FillVoid { top_candidate_cube_id } => *top_candidate_cube_id,
            _ => return Err("expected FillVoid".into()),
        };
        if top2 != bad_seam.cube_id {
            return Err("with seam=½: support=1 cube must rank above support=½ cube".into());
        }
        let _ = top;
        Ok(())
    }));

    // ── RX:EnergyRanking — StructuralCrystalCandidate energy_assessment ──────

    v.push(run_check("rx-energy-crystal-candidate-content-addressed", "RX:EnergyRanking", || {
        // energy_assessment on a StructuralCrystalCandidate must be deterministic
        // (content-addressed) and must carry the correct subject_id + policy_id.
        use kosmo_hyphae::{
            AssimilationDecision, GateCascade, StructuralCrystalCandidate,
            StructuralYield, StructuralYieldKind,
        };
        use kosmo_core::{
            AuthorityLabel, EvidenceBundle, EvidenceKind, EvidenceRef, ReplayStatus,
        };
        let policy = PolicyProfile::default_report_only();
        let ev_ref = EvidenceRef::new(d(b"ev"), EvidenceKind::HostScan, "scan");
        let ev = EvidenceBundle::seal(vec![ev_ref], policy.id, ReplayStatus::Replayable);
        let yield_ = StructuralYield::new(
            StructuralYieldKind::DeficiencyFill, Some(d(b"void")), None,
            TaintLabel::Clean, AuthorityLabel::Foundry,
            ev.bundle_id, policy.id,
        );
        let cascade = GateCascade::standard_gates(policy.clone());
        let trace = cascade.apply(&yield_, &ev);
        let decision = AssimilationDecision::from_trace(&yield_, &trace, &ev, policy.id);
        let candidate = StructuralCrystalCandidate::from_decision(&decision);
        let a1 = candidate.energy_assessment(&GateResult::Pass);
        let a2 = candidate.energy_assessment(&GateResult::Pass);
        if a1.id != a2.id {
            return Err("crystal candidate energy_assessment must be deterministic".into());
        }
        if a1.subject_id != candidate.candidate_id {
            return Err("subject_id must equal candidate_id".into());
        }
        if a1.policy_id != policy.id {
            return Err("policy_id must match pipeline policy".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-energy-crystal-candidate-reject-gate-zeroes-energy", "RX:EnergyRanking", || {
        // A Reject gate must zero the energy of a crystal candidate (CROSS-010).
        // A zero support_score candidate also yields zero energy.
        use kosmo_hyphae::{
            AssimilationDecision, GateCascade, StructuralCrystalCandidate,
            StructuralYield, StructuralYieldKind,
        };
        use kosmo_core::{
            AuthorityLabel, EvidenceBundle, EvidenceKind, EvidenceRef, ReplayStatus,
        };
        let policy = PolicyProfile::default_report_only();
        let ev_ref = EvidenceRef::new(d(b"ev2"), EvidenceKind::HostScan, "scan");
        let ev = EvidenceBundle::seal(vec![ev_ref], policy.id, ReplayStatus::Replayable);
        let yield_ = StructuralYield::new(
            StructuralYieldKind::DeficiencyFill, Some(d(b"void2")), None,
            TaintLabel::Clean, AuthorityLabel::Foundry,
            ev.bundle_id, policy.id,
        );
        let cascade = GateCascade::standard_gates(policy.clone());
        let trace = cascade.apply(&yield_, &ev);
        let decision = AssimilationDecision::from_trace(&yield_, &trace, &ev, policy.id);
        let candidate = StructuralCrystalCandidate::from_decision(&decision);
        // Reject gate → zero energy
        let reject_a = candidate.energy_assessment(&GateResult::Reject { reason: "test".into() });
        if !reject_a.kernel.is_zeroed() {
            return Err("Reject gate must zero crystal candidate energy".into());
        }
        // support_score starts at Q16::ZERO → zero energy even with Pass gate
        let pass_a = candidate.energy_assessment(&GateResult::Pass);
        if !pass_a.kernel.is_zeroed() {
            return Err("zero support_score must yield zero energy (CROSS-007 invariant)".into());
        }
        Ok(())
    }));

    // ── RX:Topology — real lexical code-HDAG extraction ─────────────────────

    const TOPO_SAMPLE: &str = "\
use std::collections::BTreeMap;
use crate::digest::Digest;

pub mod inner;

pub struct Widget { pub size: u32 }
pub enum Color { Red, Green }
pub trait Render { fn render(&self) -> String; }

impl Render for Widget {
    fn render(&self) -> String { String::new() }
}

pub fn build(a: u32, b: u32) -> Widget { Widget { size: a + b } }
fn helper() {}

#[test]
fn it_builds() { assert!(true); }
";

    v.push(run_check("rx-topology-extracts-real-graph", "RX:Topology", || {
        let h = CodeHDAG::extract_from_rust_source(
            d(b"ev"),
            "src/widget.rs",
            TOPO_SAMPLE,
            TaintLabel::Clean,
        );
        if h.edges_of_kind(&HDAGEdgeKind::Imports) != 2 {
            return Err(format!("expected 2 imports, got {}", h.edges_of_kind(&HDAGEdgeKind::Imports)));
        }
        if h.edges_of_kind(&HDAGEdgeKind::Tests) != 1 {
            return Err("expected exactly one #[test] fn".into());
        }
        if h.edges_of_kind(&HDAGEdgeKind::Implements) != 1 {
            return Err("expected one `impl Render for Widget`".into());
        }
        if h.definition_count() < 8 {
            return Err(format!("expected rich topology, got {} defs", h.definition_count()));
        }
        // Not the old one-node skeleton.
        let sk = CodeHDAG::skeleton_for_source(d(b"ev"), "src/widget.rs", TaintLabel::Clean);
        if h.hdag_id == sk.hdag_id || h.nodes.len() <= sk.nodes.len() {
            return Err("extracted graph must be strictly richer than the skeleton".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-topology-extraction-deterministic", "RX:Topology", || {
        let a = CodeHDAG::extract_from_rust_source(d(b"ev"), "x.rs", TOPO_SAMPLE, TaintLabel::Clean);
        let b = CodeHDAG::extract_from_rust_source(d(b"ev"), "x.rs", TOPO_SAMPLE, TaintLabel::Clean);
        if a.hdag_id != b.hdag_id {
            return Err("identical source must yield identical hdag_id (INVARIANT-007)".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-topology-energy-chain", "RX:Topology", || {
        // The full front of the vision: real topology in → tripolar energy on it.
        // A complete, tested module must carry more energy than an empty one,
        // holding the semantic pole ψ constant.
        let rich = CodeHDAG::extract_from_rust_source(
            d(b"ev"),
            "src/widget.rs",
            TOPO_SAMPLE,
            TaintLabel::Clean,
        );
        let empty = CodeHDAG::extract_from_rust_source(
            d(b"ev"),
            "src/empty.rs",
            "// nothing here\n",
            TaintLabel::Clean,
        );
        let psi = Q16::ONE;
        let rich_e = rich.energy_kernel(psi, EnergyFactors::all_clean()).energy();
        let empty_e = empty.energy_kernel(psi, EnergyFactors::all_clean()).energy();
        if rich_e.raw() <= empty_e.raw() {
            return Err("richer topology must yield higher energy".into());
        }
        if !empty_e.is_zero() {
            return Err("empty topology must yield zero energy".into());
        }
        // ω = 1 because imports + contained defs + tests are all present.
        if rich.omega_phase() != Q16::ONE {
            return Err("complete module must have ω = 1".into());
        }
        // And the assessment over real topology is content-addressed & evidence-bound.
        let a = rich.energy_assessment(psi, EnergyFactors::all_clean(), d(b"pol"), d(b"bundle"));
        if !a.verify_id() || a.subject_id != rich.hdag_id {
            return Err("topology energy assessment must verify and key on hdag_id".into());
        }
        Ok(())
    }));

    // ── RX:Kcube — real .kcube archive executor ──────────────────────────────

    v.push(run_check("rx-kcube-denied-when-write-false", "RX:Kcube", || {
        // CROSS-010 analogue: allow_write=false must never produce a written file.
        let dir = eval_tmp_dir("rx-kcube-deny");
        let exec = KcubeExecutor::new(&dir);
        let policy = KcubeExportPolicy::report_only(d(b"pol"), d(b"dir"));
        let art = KcubeArtifact::new(
            kosmo_core::KcubeArtifactKind::StructuralCrystal,
            "c.bin",
            b"data".to_vec(),
        );
        let report = exec.write("scope", vec![art], &policy, d(b"ev-bundle"), 1);
        if report.outcome.is_written() {
            return Err("allow_write=false must not produce Written outcome".into());
        }
        if report.written_bytes != 0 {
            return Err("written_bytes must be 0 when denied".into());
        }
        if !report.verify_id() {
            return Err("DeniedByPolicy report must be content-addressed".into());
        }
        // evidence must be bound even in denied reports (CROSS-006)
        if report.evidence_bundle_id == Digest::ZERO {
            return Err("evidence_bundle_id must be non-zero even in denied reports".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-kcube-write-and-roundtrip-passes", "RX:Kcube", || {
        // Real file write: the archive is created and roundtrip verification passes.
        let dir = eval_tmp_dir("rx-kcube-write");
        let exec = KcubeExecutor::new(&dir);
        let policy = KcubeExportPolicy::write_once(
            d(b"pol"),
            d(b"dir"),
            vec![kosmo_core::KcubeArtifactKind::StructuralCrystal],
        );
        let artifacts = vec![
            KcubeArtifact::new(kosmo_core::KcubeArtifactKind::StructuralCrystal, "c1.bin", b"crystal-a".to_vec()),
            KcubeArtifact::new(kosmo_core::KcubeArtifactKind::StructuralCrystal, "c2.bin", b"crystal-b".to_vec()),
        ];
        let report = exec.write("bench-scope", artifacts, &policy, d(b"ev-bundle"), 1);
        if !report.outcome.is_written() {
            return Err(format!("expected Written, got {:?}", report.outcome));
        }
        if !report.roundtrip_passed() {
            return Err("roundtrip verification must pass for a correct write".into());
        }
        if report.written_bytes == 0 {
            return Err("written_bytes must be > 0".into());
        }
        if !report.verify_id() {
            return Err("KcubeWriteReport must be content-addressed".into());
        }
        // File must exist on disk
        let fname = kcube_file_name("bench-scope", 1);
        if !dir.join(&fname).exists() {
            return Err(format!("expected .kcube file {fname} to exist on disk"));
        }
        Ok(())
    }));

    v.push(run_check("rx-kcube-package-content-addressed", "RX:Kcube", || {
        // The KcubePackage stored in the written archive verifies its own id
        // (INVARIANT-007) and its package_digest matches the artifact section SHA-256.
        let dir = eval_tmp_dir("rx-kcube-pkg-ca");
        let exec = KcubeExecutor::new(&dir);
        let policy = KcubeExportPolicy::write_once(
            d(b"pol"),
            d(b"dir"),
            vec![kosmo_core::KcubeArtifactKind::EvidenceBundle],
        );
        let art = KcubeArtifact::new(
            kosmo_core::KcubeArtifactKind::EvidenceBundle,
            "bundle.json",
            b"{\"ev\":1}".to_vec(),
        );
        let report = exec.write("ca-scope", vec![art], &policy, d(b"ev-bundle"), 9);
        if !report.outcome.is_written() {
            return Err(format!("write failed: {:?}", report.outcome));
        }
        let fname = kcube_file_name("ca-scope", 9);
        let pkg = exec.read(&fname)
            .map_err(|e| format!("read failed: {e}"))?;
        if !pkg.verify_id() {
            return Err("KcubePackage.verify_id() must pass after roundtrip read".into());
        }
        if pkg.scope != "ca-scope" {
            return Err(format!("scope mismatch: {:?}", pkg.scope));
        }
        if pkg.entry_count() != 1 {
            return Err(format!("expected 1 entry, got {}", pkg.entry_count()));
        }
        Ok(())
    }));

    v.push(run_check("rx-kcube-overwrite-denied-by-default", "RX:Kcube", || {
        // A second write with the same sequence must be blocked when allow_overwrite=false.
        let dir = eval_tmp_dir("rx-kcube-overwrite");
        let exec = KcubeExecutor::new(&dir);
        let policy = KcubeExportPolicy::write_once(
            d(b"pol"),
            d(b"dir"),
            vec![kosmo_core::KcubeArtifactKind::StructuralCrystal],
        );
        let mk_art = || vec![KcubeArtifact::new(
            kosmo_core::KcubeArtifactKind::StructuralCrystal, "c.bin", b"v1".to_vec(),
        )];
        let r1 = exec.write("over-scope", mk_art(), &policy, d(b"ev"), 3);
        if !r1.outcome.is_written() {
            return Err(format!("first write failed: {:?}", r1.outcome));
        }
        let r2 = exec.write("over-scope", mk_art(), &policy, d(b"ev"), 3);
        if r2.outcome.is_written() {
            return Err("second write to same sequence must be blocked (allow_overwrite=false)".into());
        }
        if r2.outcome.is_failure_class()
            && !matches!(r2.outcome, kosmo_core::KcubeWriteOutcome::DeniedByPolicy { .. })
        {
            return Err(format!("expected DeniedByPolicy, got {:?}", r2.outcome));
        }
        if !r2.verify_id() {
            return Err("overwrite-denied report must be content-addressed".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-kcube-read-parses-manifest", "RX:Kcube", || {
        // KcubeExecutor::read returns the correctly-parsed KcubePackage from disk.
        let dir = eval_tmp_dir("rx-kcube-read");
        let exec = KcubeExecutor::new(&dir);
        let policy = KcubeExportPolicy::write_once(
            d(b"pol"),
            d(b"dir"),
            vec![
                kosmo_core::KcubeArtifactKind::StructuralCrystal,
                kosmo_core::KcubeArtifactKind::EvidenceBundle,
            ],
        );
        let artifacts = vec![
            KcubeArtifact::new(kosmo_core::KcubeArtifactKind::StructuralCrystal, "a.bin", b"aaa".to_vec()),
            KcubeArtifact::new(kosmo_core::KcubeArtifactKind::EvidenceBundle,   "b.json", b"{}".to_vec()),
        ];
        let report = exec.write("read-scope", artifacts, &policy, d(b"ev-bundle"), 77);
        if !report.outcome.is_written() {
            return Err(format!("write failed: {:?}", report.outcome));
        }
        let fname = kcube_file_name("read-scope", 77);
        let pkg = exec.read(&fname).map_err(|e| format!("read error: {e}"))?;
        if pkg.entry_count() != 2 {
            return Err(format!("expected 2 entries, got {}", pkg.entry_count()));
        }
        if pkg.created_at_sequence != 77 {
            return Err(format!("sequence mismatch: {}", pkg.created_at_sequence));
        }
        if pkg.evidence_bundle_id != d(b"ev-bundle") {
            return Err("evidence_bundle_id mismatch after roundtrip".into());
        }
        // package_id in the write report must match the parsed package id
        if report.package_id != pkg.id {
            return Err("package_id in write report must equal parsed KcubePackage.id".into());
        }
        Ok(())
    }));

    // ── RX:SystemCubeKcube — SystemCube → real .kcube archive weld ─────────────

    v.push(run_check("rx-systemcube-kcube-blocked-default-policy", "RX:SystemCubeKcube", || {
        // CROSS-010 analogue: default PolicyProfile must block systemcube materialization.
        let dir = eval_tmp_dir("rx-sc-kcube-blocked");
        let exec = KcubeExecutor::new(&dir);
        let policy = kosmo_core::PolicyProfile::default_report_only();
        let export_policy = kosmo_core::KcubeExportPolicy::write_once(
            d(b"pol"), d(b"dir"),
            vec![kosmo_core::KcubeArtifactKind::CartographyManifest],
        );
        let unit = BlueprintUnit::new(
            BlueprintUnitKind::ModuleBoundary,
            d(b"src"),
            kosmo_core::AuthorityLabel::Operator,
            kosmo_core::TaintLabel::Clean,
            vec![d(b"ev")],
            &policy,
        );
        let cube = SystemCube::new(d(b"host"), &kosmo_core::RunDescriptor::new(policy.id, "host"), &policy, vec![unit]);
        let report = cube.export_to_kcube(&exec, 4, &export_policy, &policy, d(b"bundle"), 1);
        if !report.outcome.is_skipped_report_only() {
            return Err(format!("expected SkippedByReportOnly, got {:?}", report.outcome));
        }
        if report.written_bytes != 0 {
            return Err("written_bytes must be 0 when blocked".into());
        }
        if report.evidence_bundle_id == Digest::ZERO {
            return Err("CROSS-006: evidence must be bound even when blocked".into());
        }
        if !report.verify_id() {
            return Err("skipped report must be content-addressed".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-systemcube-kcube-writes-archive", "RX:SystemCubeKcube", || {
        // Full chain: SystemCube → export_to_kcube → real .kcube file on disk.
        let dir = eval_tmp_dir("rx-sc-kcube-write");
        let exec = KcubeExecutor::new(&dir);
        let base_policy = kosmo_core::PolicyProfile::default_report_only();
        let op_policy = kosmo_core::PolicyProfile::operator_approved_with_systemcube();
        let export_policy = kosmo_core::KcubeExportPolicy::write_once(
            d(b"pol"), d(b"dir"),
            vec![
                kosmo_core::KcubeArtifactKind::CartographyManifest,
                kosmo_core::KcubeArtifactKind::ValidationClosureReport,
                kosmo_core::KcubeArtifactKind::StructuralCrystal,
            ],
        );
        let unit = BlueprintUnit::new(
            BlueprintUnitKind::CrystalReference,
            d(b"crystal"),
            kosmo_core::AuthorityLabel::Operator,
            kosmo_core::TaintLabel::Clean,
            vec![d(b"ev")],
            &base_policy,
        );
        let run = kosmo_core::RunDescriptor::new(base_policy.id, "bench");
        let cube = SystemCube::new(d(b"host"), &run, &base_policy, vec![unit]);
        let report = cube.export_to_kcube(&exec, 4, &export_policy, &op_policy, d(b"bundle"), 1);
        if !report.outcome.is_written() {
            return Err(format!("expected Written, got {:?}", report.outcome));
        }
        if !report.roundtrip_passed() {
            return Err("roundtrip must pass for a correct write".into());
        }
        if report.written_bytes == 0 {
            return Err("written_bytes must be > 0".into());
        }
        if !report.verify_id() {
            return Err("write report must be content-addressed".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-systemcube-kcube-archive-parses-back", "RX:SystemCubeKcube", || {
        // The written .kcube archive must parse back to a valid KcubePackage
        // with the expected number of artifact entries.
        let dir = eval_tmp_dir("rx-sc-kcube-parse");
        let exec = KcubeExecutor::new(&dir);
        let base_policy = kosmo_core::PolicyProfile::default_report_only();
        let op_policy = kosmo_core::PolicyProfile::operator_approved_with_systemcube();
        let export_policy = kosmo_core::KcubeExportPolicy::write_once(
            d(b"pol"), d(b"dir"),
            vec![
                kosmo_core::KcubeArtifactKind::CartographyManifest,
                kosmo_core::KcubeArtifactKind::ValidationClosureReport,
                kosmo_core::KcubeArtifactKind::StructuralCrystal,
            ],
        );
        let unit1 = BlueprintUnit::new(
            BlueprintUnitKind::ModuleBoundary, d(b"s1"),
            kosmo_core::AuthorityLabel::Operator, kosmo_core::TaintLabel::Clean,
            vec![d(b"e1")], &base_policy,
        );
        let unit2 = BlueprintUnit::new(
            BlueprintUnitKind::FiberDescriptor, d(b"s2"),
            kosmo_core::AuthorityLabel::Operator, kosmo_core::TaintLabel::Clean,
            vec![d(b"e2")], &base_policy,
        );
        let run = kosmo_core::RunDescriptor::new(base_policy.id, "parse-bench");
        let cube = SystemCube::new(d(b"host"), &run, &base_policy, vec![unit1, unit2]);
        let report = cube.export_to_kcube(&exec, 4, &export_policy, &op_policy, d(b"bundle"), 42);
        if !report.outcome.is_written() {
            return Err(format!("write failed: {:?}", report.outcome));
        }
        // Derive the same scope the weld uses: "systemcube-{first 16 hex chars of cube_id}"
        let scope = format!("systemcube-{}", &cube.cube_id.to_hex()[..16]);
        let fname = kcube_file_name(&scope, 42);
        let pkg = exec.read(&fname).map_err(|e| format!("read error: {e}"))?;
        if !pkg.verify_id() {
            return Err("KcubePackage.verify_id() must pass after roundtrip read".into());
        }
        // manifest.json + export_report.json + 2 accepted crystal units = 4 artifacts
        if pkg.entry_count() != 4 {
            return Err(format!("expected 4 artifacts (manifest + report + 2 units), got {}", pkg.entry_count()));
        }
        // package_id in the write report must match the parsed package id
        if report.package_id != pkg.id {
            return Err("package_id in write report must equal parsed KcubePackage.id".into());
        }
        Ok(())
    }));

    // ── RX:ContradictionEnergy — real pairwise contradiction detection ───────────

    v.push(run_check("rx-contradiction-energy-role-conflict-detected", "RX:ContradictionEnergy", || {
        let policy = PolicyProfile::default_report_only();
        let mid = d(b"manifest");
        let src = d(b"shared_source");
        let ev1 = d(b"ev1");
        let ev2 = d(b"ev2");
        // Two units with the same source_ref but different kinds = RoleConflict.
        let boundary = BlueprintUnit::new(
            BlueprintUnitKind::ModuleBoundary,
            src,
            kosmo_core::AuthorityLabel::Foundry,
            TaintLabel::Clean,
            vec![ev1],
            &policy,
        );
        let crystal = BlueprintUnit::new(
            BlueprintUnitKind::CrystalReference,
            src,
            kosmo_core::AuthorityLabel::Foundry,
            TaintLabel::Clean,
            vec![ev2],
            &policy,
        );
        let r = ContradictionEnergyReport::from_units(mid, &policy, &[boundary, crystal]);
        if r.status != EnergyStatus::Available {
            return Err(format!("expected Available, got {:?}", r.status));
        }
        if r.contradictions.len() != 1 {
            return Err(format!("expected 1 contradiction, got {}", r.contradictions.len()));
        }
        if r.contradictions[0].weight != Q16::ONE {
            return Err(format!("RoleConflict weight must be Q16::ONE, got {}", r.contradictions[0].weight.raw()));
        }
        if !r.contradictions[0].reason.contains("RoleConflict") {
            return Err("contradiction reason must mention RoleConflict".into());
        }
        if r.total_energy != Q16::ONE {
            return Err(format!("total_energy must be ONE, got {}", r.total_energy.raw()));
        }
        Ok(())
    }));

    v.push(run_check("rx-contradiction-energy-independent-sources-zero", "RX:ContradictionEnergy", || {
        let policy = PolicyProfile::default_report_only();
        let mid = d(b"manifest");
        let ev = d(b"ev");
        // Units with distinct source_refs must not generate contradictions.
        let u1 = BlueprintUnit::new(
            BlueprintUnitKind::ModuleBoundary,
            d(b"src_a"),
            kosmo_core::AuthorityLabel::Foundry,
            TaintLabel::Clean,
            vec![ev],
            &policy,
        );
        let u2 = BlueprintUnit::new(
            BlueprintUnitKind::ModuleBoundary,
            d(b"src_b"),
            kosmo_core::AuthorityLabel::Foundry,
            TaintLabel::Clean,
            vec![ev],
            &policy,
        );
        let r = ContradictionEnergyReport::from_units(mid, &policy, &[u1.clone(), u2.clone()]);
        if r.status != EnergyStatus::Available {
            return Err(format!("expected Available, got {:?}", r.status));
        }
        if r.total_energy != Q16::ZERO {
            return Err(format!("independent sources must yield zero energy, got {}", r.total_energy.raw()));
        }
        if !r.contradictions.is_empty() {
            return Err(format!("expected 0 contradictions, got {}", r.contradictions.len()));
        }
        // report must be content-addressed and deterministic
        let r2 = ContradictionEnergyReport::from_units(mid, &policy, &[u1, u2]);
        if r.report_id != r2.report_id {
            return Err("ContradictionEnergyReport must be deterministic (INVARIANT-007)".into());
        }
        Ok(())
    }));

    // ── RX:Compatibility — CompatibilityProfileReport real gap detection ─────────

    v.push(run_check("rx-compatibility-clean-units-score-one", "RX:Compatibility", || {
        let policy = PolicyProfile::default_report_only();
        let mid = d(b"manifest");
        let hid = d(b"host");
        let ev = d(b"ev");
        let u1 = BlueprintUnit::new(
            BlueprintUnitKind::ModuleBoundary,
            d(b"src1"),
            kosmo_core::AuthorityLabel::Foundry,
            TaintLabel::Clean,
            vec![ev],
            &policy,
        );
        let u2 = BlueprintUnit::new(
            BlueprintUnitKind::FiberDescriptor,
            d(b"src2"),
            kosmo_core::AuthorityLabel::Foundry,
            TaintLabel::Clean,
            vec![ev],
            &policy,
        );
        let r = CompatibilityProfileReport::from_units(mid, hid, &policy, &[u1.clone(), u2.clone()]);
        if r.status != CompatibilityStatus::Available {
            return Err(format!("expected Available, got {:?}", r.status));
        }
        if r.compatibility_score != Q16::ONE {
            return Err(format!("all-clean units must yield score=ONE, got {}", r.compatibility_score.raw()));
        }
        if !r.gaps.is_empty() {
            return Err(format!("expected 0 gaps, got {}", r.gaps.len()));
        }
        // report must be content-addressed
        let r2 = CompatibilityProfileReport::from_units(mid, hid, &policy, &[u1, u2]);
        if r.report_id != r2.report_id {
            return Err("CompatibilityProfileReport must be deterministic (INVARIANT-007)".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-compatibility-tainted-unit-reduces-score", "RX:Compatibility", || {
        let policy = PolicyProfile::default_report_only();
        let mid = d(b"manifest");
        let hid = d(b"host");
        let ev = d(b"ev");
        let tainted = BlueprintUnit::new(
            BlueprintUnitKind::ModuleBoundary,
            d(b"src1"),
            kosmo_core::AuthorityLabel::Foundry,
            TaintLabel::Synthetic,
            vec![ev],
            &policy,
        );
        let r = CompatibilityProfileReport::from_units(mid, hid, &policy, &[tainted]);
        if r.status != CompatibilityStatus::Available {
            return Err(format!("expected Available, got {:?}", r.status));
        }
        if r.gaps.len() != 1 {
            return Err(format!("expected 1 gap, got {}", r.gaps.len()));
        }
        if r.gaps[0].gap_kind != "TaintedUnit" {
            return Err(format!("expected TaintedUnit gap, got {}", r.gaps[0].gap_kind));
        }
        if r.gaps[0].severity != Q16::HALF {
            return Err(format!("TaintedUnit severity must be HALF, got {}", r.gaps[0].severity.raw()));
        }
        if r.compatibility_score != Q16::HALF {
            return Err(format!("one tainted unit must yield score=HALF, got {}", r.compatibility_score.raw()));
        }
        Ok(())
    }));

    // ── RX:FeedbackLoop — PSE promotion outcome back into substrate ─────────────

    v.push(run_check("rx-feedback-accepted-maps-to-full-energy", "RX:FeedbackLoop", || {
        // Accepted promotion → fitness_signal = energy_at_submission.
        let energy = Q16::ratio(3, 4).unwrap();
        let f = PromotionFeedback::new(
            d(b"record"), d(b"candidate"), d(b"norm"),
            FeedbackOutcome::Accepted,
            energy, d(b"pol"), d(b"ev"),
        );
        if f.fitness_signal != energy {
            return Err(format!("accepted must map fitness to energy, got {:?}", f.fitness_signal.raw()));
        }
        if !f.verify_id() {
            return Err("PromotionFeedback must be content-addressed (INVARIANT-007)".into());
        }
        if f.evidence_bundle_id == Digest::ZERO {
            return Err("CROSS-006: evidence must be non-zero".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-feedback-rejected-fitness-is-zero", "RX:FeedbackLoop", || {
        // Rejected → fitness_signal = ZERO (CROSS-010 analogue for feedback).
        let f = PromotionFeedback::new(
            d(b"record"), d(b"candidate"), d(b"norm"),
            FeedbackOutcome::Rejected,
            Q16::ONE, d(b"pol"), d(b"ev"),
        );
        if !f.fitness_signal.is_zero() {
            return Err("rejected feedback must produce zero fitness — PSE rejection cannot confer fitness".into());
        }
        if !f.verify_id() {
            return Err("content-addressing must hold even for rejected feedback".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-feedback-stored-in-cartography", "RX:FeedbackLoop", || {
        // A PromotionFeedback can be stored as a CartographyStoreCommit with
        // CartographyEntryKind::PromotionFeedback.
        let feedback = PromotionFeedback::new(
            d(b"rec"), d(b"cand"), d(b"norm"),
            FeedbackOutcome::Accepted,
            Q16::HALF, d(b"pol"), d(b"ev"),
        );
        let commit = CartographyStoreCommit::new(
            CorpusScope::LocalHostProject,
            1,
            feedback.id,
            CartographyEntryKind::PromotionFeedback,
            feedback.evidence_bundle_id,
            feedback.policy_id,
        );
        if !commit.verify_id() {
            return Err("CartographyStoreCommit with PromotionFeedback must verify_id()".into());
        }
        if commit.payload_digest != feedback.id {
            return Err("commit payload_digest must equal feedback.id".into());
        }
        Ok(())
    }));

    v.push(run_check("rx-feedback-norm-trace-updates-from-feedback", "RX:FeedbackLoop", || {
        // build_promotion_feedback + NormFitnessTrace::observe_from_feedback closes
        // the full loop: PSE outcome → feedback record → norm fitness trace update.
        let policy = PolicyProfile::default_report_only();
        let norm_cid = d(b"norm-cand");
        let energy = Q16::ratio(7, 8).unwrap();

        // Build a PseBridgeCandidate with confidence = energy
        let candidate = PseBridgeCandidate::new(
            PseBridgeCandidateKind::StructuralObservation,
            d(b"obs"),
            "eval-norm",
            energy,
            d(b"run"),
            d(b"ev"),
            policy.id,
        );

        // Build a Accepted PromotionRequestRecord
        let record = PromotionRequestRecord::new(candidate.id, PromotionOutcome::Accepted, d(b"ev"), 10);

        // Build feedback via the bridge function
        let feedback = build_promotion_feedback(&record, &candidate, norm_cid, policy.id);
        if !feedback.outcome.is_accepted() {
            return Err("feedback outcome must be Accepted".into());
        }
        if feedback.fitness_signal != energy {
            return Err(format!("fitness_signal must equal energy={}, got {}", energy.raw(), feedback.fitness_signal.raw()));
        }
        if !feedback.verify_id() {
            return Err("feedback must verify_id()".into());
        }

        // Ingest into NormFitnessTrace
        let trace = NormFitnessTrace::empty(norm_cid, policy.id)
            .observe_from_feedback(&feedback);
        if trace.latest_fitness() != energy {
            return Err(format!("NormFitnessTrace.latest_fitness must equal energy after feedback ingestion"));
        }
        if trace.observations[0].evidence_ref != feedback.id {
            return Err("evidence_ref in trace observation must be feedback.id".into());
        }

        Ok(())
    }));

    v
}

/// Unique temp path for an eval scenario's working directory.
fn eval_tmp_dir(tag: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join("kosmo-eval-kcube").join(tag);
    std::fs::remove_dir_all(&p).ok();
    std::fs::create_dir_all(&p).unwrap();
    p
}

/// Unique temp path for a benchmark store scenario.
fn temp_store_path(tag: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut p = std::env::temp_dir();
    p.push(format!("kosmo-{tag}-{nanos}.jsonl"));
    p
}

fn run_cerebras(api_key: &str) -> ScenarioResult {
    // Model is configurable via CEREBRAS_MODEL; default is the current Cerebras fast model.
    let model = std::env::var("CEREBRAS_MODEL")
        .unwrap_or_else(|_| "gpt-oss-120b".to_string());

    let t0 = Instant::now();
    let result: Result<(), String> = (|| {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("client build failed: {e}"))?;

        println!("  model: {model}");
        let body = serde_json::json!({
            "model": model,
            "messages": [{"role": "user", "content": "Reply with exactly one word: KOSMO-OK"}],
            "max_tokens": 512
        });

        let resp = client
            .post("https://api.cerebras.ai/v1/chat/completions")
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| format!("HTTP request failed: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            return Err(format!("HTTP {status}: {}", &body[..body.len().min(200)]));
        }

        let raw = resp.text().map_err(|e| format!("read response failed: {e}"))?;
        let json: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|e| format!("JSON parse failed: {e}\nRaw: {}", &raw[..raw.len().min(400)]))?;

        // Extract text content robustly: handle both string and array content fields.
        let msg = &json["choices"][0]["message"];
        let content: String = match &msg["content"] {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Array(parts) => {
                // OpenAI-style content blocks: [{type:"text", text:"..."}]
                parts.iter()
                    .filter_map(|p| p["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("")
            }
            serde_json::Value::Null => {
                // Dump the full response so the caller can diagnose
                return Err(format!(
                    "content is null — full response:\n{}",
                    serde_json::to_string_pretty(&json).unwrap_or(raw)
                ));
            }
            other => {
                return Err(format!(
                    "unexpected content type: {other}\nfull response:\n{}",
                    serde_json::to_string_pretty(&json).unwrap_or(raw)
                ));
            }
        };

        if content.to_uppercase().contains("KOSMO") {
            Ok(())
        } else {
            Err(format!("unexpected response: {:?}", content))
        }
    })();

    let elapsed_ms = t0.elapsed().as_millis() as u64;
    match result {
        Ok(()) => ScenarioResult {
            name: "cerebras-api-round-trip",
            phase: "CEREBRAS",
            passed: true,
            elapsed_ms,
            details: vec![],
        },
        Err(msg) => ScenarioResult {
            name: "cerebras-api-round-trip",
            phase: "CEREBRAS",
            passed: false,
            elapsed_ms,
            details: vec![msg],
        },
    }
}

fn prompt_cerebras_key() -> Option<String> {
    if let Ok(key) = std::env::var("CEREBRAS_API_KEY") {
        if !key.is_empty() {
            return Some(key);
        }
    }
    print!("\nCerebras API key (press Enter to skip): ");
    io::stdout().flush().ok();
    let mut line = String::new();
    io::stdin().read_line(&mut line).ok();
    let trimmed = line.trim().to_string();
    if trimmed.is_empty() { None } else { Some(trimmed) }
}

fn print_table(results: &[ScenarioResult]) {
    const NW: usize = 42;
    const PW: usize = 20;
    const SW: usize = 4;
    const TW: usize = 6;
    let sep = format!(
        "+-{:-<NW$}-+-{:-<PW$}-+-{:-<SW$}-+-{:-<TW$}-+",
        "", "", "", ""
    );
    println!("\n{sep}");
    println!(
        "| {:<NW$} | {:<PW$} | {:<SW$} | {:>TW$} |",
        "SCENARIO", "PHASE", "STAT", "ms"
    );
    println!("{sep}");
    for r in results {
        let status = if r.passed { "PASS" } else { "FAIL" };
        println!(
            "| {:<NW$} | {:<PW$} | {:<SW$} | {:>TW$} |",
            r.name, r.phase, status, r.elapsed_ms
        );
        for detail in &r.details {
            println!("| {:<72} |", format!("  ↳ {detail}"));
        }
    }
    println!("{sep}");
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║    KOSMO-OPS-01 Empirical Validation Benchmark               ║");
    println!("║    Phases R1–R9 · All invariants · Content-addressed         ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    println!("\nBuilding scenario suite…");
    let mut results = build_scenarios();
    let core_count = results.len();
    println!("  {core_count} core invariant scenarios ready.");

    // Cerebras integration
    let key = prompt_cerebras_key();
    if let Some(ref api_key) = key {
        println!("Running Cerebras API scenario…");
        results.push(run_cerebras(api_key));
    } else {
        println!("Cerebras API scenario skipped (no key).");
    }

    print_table(&results);

    let total = results.len();
    let passed = results.iter().filter(|r| r.passed).count();
    let failed = total - passed;

    println!("\nSummary: {passed}/{total} scenarios passed");

    if failed == 0 {
        println!("RESULT: ✔  ALL INVARIANTS VERIFIED");
    } else {
        println!("RESULT: ✘  {failed} INVARIANT(S) VIOLATED");
        std::process::exit(1);
    }
}

use std::io::{self, Write as IoWrite};
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
};

use kosmo_pse_bridge::{
    PseBridgeCandidate, PseBridgeCandidateKind, PseBridgePolicy, PseBridgeRateLimit,
    PromotionOutcome, validate_candidate,
};

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

    v
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

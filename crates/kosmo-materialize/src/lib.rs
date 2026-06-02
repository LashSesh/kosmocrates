//! Kosmocrates materializer — the write/validate layer that arms the agent's
//! `dry_run = false` path.
//!
//! A [`Patch`] (from `kosmo-synthesizer`) describes file changes. This crate
//! decides *whether* and *how* to put them on disk, governed by the
//! [`PolicyProfile`], and validates the result by compiling (and optionally
//! testing) the workspace via `kosmo-foundry`.
//!
//! # Policy gating (fail-closed)
//!
//! | Mode | Behaviour |
//! |------|-----------|
//! | `ReportOnly` | [`MaterializeOutcome::SkippedByPolicy`] — no I/O at all. |
//! | `DryRun` | **Sandbox**: copy the workspace to a temp dir, apply there, validate; the host is never touched. |
//! | `OperatorApproved` + `allow_host_write` | **In-place**: back up touched files, apply to the host, validate, and roll back on failure. |
//! | `OperatorApproved` without `allow_host_write` | blocked ([`SkippedByPolicy`](MaterializeOutcome::SkippedByPolicy)). |
//!
//! The host workspace is mutated **only** in the in-place path, and even then a
//! failed validation triggers a full restore of every touched file
//! (CROSS-002 / CROSS-013: no net host mutation unless validation passes).
//!
//! # Validation is pluggable
//!
//! [`PatchValidator`] abstracts the compile/test step. [`CargoFoundryValidator`]
//! drives real `cargo check` / `cargo test` through `kosmo-foundry`;
//! [`AlwaysPass`] / [`AlwaysFail`] are deterministic stubs for tests.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use kosmo_core::{
    Digest, FoundryCheckKind, FoundryCheckSpec, FoundryCommandPolicy, FoundryEnvironmentPolicy,
    FoundryExecutionOutcome, FoundryExecutionPlan, FoundrySandboxKind, FoundrySandboxSpec,
    FoundryTimeoutPolicy, GateResult, ImplementationMode, PolicyProfile,
};
use kosmo_foundry::FoundryExecutor;
use kosmo_synthesizer::{FileChange, FileChangeKind, Patch};

// ─── Options ────────────────────────────────────────────────────────────────

/// Knobs for one materialization.
#[derive(Clone, Debug)]
pub struct MaterializeOptions {
    /// Run `cargo test` in addition to `cargo check`.
    pub run_tests: bool,
}

impl Default for MaterializeOptions {
    fn default() -> Self {
        Self { run_tests: true }
    }
}

// ─── Validation ─────────────────────────────────────────────────────────────

/// Result of validating a workspace after a patch was applied to it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationReport {
    /// `Some(true/false)` once `cargo check` ran; `None` if it did not.
    pub compile_passed: Option<bool>,
    /// `Some(true/false)` once `cargo test` ran; `None` if tests were skipped.
    pub tests_passed: Option<bool>,
    pub gate_result: GateResult,
    /// Non-ZERO evidence reference (CROSS-006).
    pub evidence_bundle_id: Digest,
    pub diagnostics: Vec<String>,
}

impl ValidationReport {
    /// A patch is acceptable when nothing failed: the gate did not reject and
    /// neither compile nor tests came back `Some(false)`.
    pub fn is_acceptable(&self) -> bool {
        !matches!(self.gate_result, GateResult::Reject { .. })
            && self.compile_passed != Some(false)
            && self.tests_passed != Some(false)
    }

    /// Deterministic "everything passed" report (used by stubs / tests).
    pub fn passed(run_tests: bool) -> Self {
        Self {
            compile_passed: Some(true),
            tests_passed: if run_tests { Some(true) } else { None },
            gate_result: GateResult::Pass,
            evidence_bundle_id: Digest::of_bytes(b"kosmo-materialize-validation-pass"),
            diagnostics: vec![],
        }
    }

    /// Deterministic "compile failed" report (used by stubs / tests).
    pub fn failed(reason: impl Into<String>, run_tests: bool) -> Self {
        let reason = reason.into();
        Self {
            compile_passed: Some(false),
            tests_passed: if run_tests { Some(false) } else { None },
            gate_result: GateResult::Reject { reason: reason.clone() },
            evidence_bundle_id: Digest::of_bytes(b"kosmo-materialize-validation-fail"),
            diagnostics: vec![reason],
        }
    }
}

/// Pluggable compile/test backend.
pub trait PatchValidator {
    /// Validate the workspace rooted at `root` (the patch is already applied).
    fn validate(&self, root: &Path, run_tests: bool) -> ValidationReport;
    fn name(&self) -> &str;
}

/// Real validator: drives `cargo check` (+ optional `cargo test`) via
/// `kosmo-foundry`, with its hardened sandbox/timeout/env-scrubbing.
pub struct CargoFoundryValidator {
    timeout_ms: u64,
    policy_id: Digest,
}

impl Default for CargoFoundryValidator {
    fn default() -> Self {
        Self {
            timeout_ms: 600_000,
            policy_id: Digest::of_bytes(b"kosmo-materialize-cargo-validator"),
        }
    }
}

impl CargoFoundryValidator {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }
}

impl PatchValidator for CargoFoundryValidator {
    fn validate(&self, root: &Path, run_tests: bool) -> ValidationReport {
        let root_digest = Digest::of(&root.to_string_lossy().to_string());
        let sandbox = FoundrySandboxSpec::new(FoundrySandboxKind::LocalDryRun, root_digest);
        let mut checks = vec![FoundryCheckSpec::new(FoundryCheckKind::Build, "workspace", true)];
        if run_tests {
            checks.push(FoundryCheckSpec::new(FoundryCheckKind::Test, "workspace", true));
        }
        let plan = FoundryExecutionPlan::new(
            self.policy_id,
            root_digest,
            Digest::of_bytes(b"kosmo-materialize-task"),
            sandbox,
            checks,
            FoundryCommandPolicy::default_cargo_policy(),
            FoundryTimeoutPolicy::new(self.timeout_ms, self.timeout_ms.saturating_mul(4)),
            FoundryEnvironmentPolicy::locked(),
        );

        // Foundry only spawns when the policy is at least DryRun; use an
        // OperatorApproved policy here so the checks actually run. (The *host*
        // policy gating already happened in `Materializer::materialize`.)
        let exec = FoundryExecutor::new(root);
        let policy = PolicyProfile::operator_approved();
        let report = exec.execute(
            &plan,
            &policy,
            Digest::of_bytes(b"kosmo-materialize-foundry-evidence"),
        );

        let compile = report
            .check_results
            .iter()
            .find(|r| r.check_kind == FoundryCheckKind::Build)
            .map(|r| r.outcome.is_passed());
        let tests = if run_tests {
            report
                .check_results
                .iter()
                .find(|r| r.check_kind == FoundryCheckKind::Test)
                .map(|r| r.outcome.is_passed())
        } else {
            None
        };

        let gate_result = match &report.outcome {
            FoundryExecutionOutcome::Passed => GateResult::Pass,
            FoundryExecutionOutcome::SkippedByReportOnly => GateResult::Warn {
                message: "validation skipped by policy".into(),
            },
            other => GateResult::Reject {
                reason: format!("foundry outcome {other:?}: {}", report.diagnostics.join("; ")),
            },
        };

        ValidationReport {
            compile_passed: compile,
            tests_passed: tests,
            gate_result,
            evidence_bundle_id: report.evidence_bundle_id,
            diagnostics: report.diagnostics,
        }
    }

    fn name(&self) -> &str {
        "cargo-foundry"
    }
}

/// Stub validator that always reports success — for tests and dry assembly.
pub struct AlwaysPass;
impl PatchValidator for AlwaysPass {
    fn validate(&self, _root: &Path, run_tests: bool) -> ValidationReport {
        ValidationReport::passed(run_tests)
    }
    fn name(&self) -> &str {
        "always-pass"
    }
}

/// Stub validator that always reports failure — for testing the rollback path.
pub struct AlwaysFail;
impl PatchValidator for AlwaysFail {
    fn validate(&self, _root: &Path, run_tests: bool) -> ValidationReport {
        ValidationReport::failed("stub: forced failure", run_tests)
    }
    fn name(&self) -> &str {
        "always-fail"
    }
}

// ─── Outcome + report ───────────────────────────────────────────────────────

/// What the materializer ended up doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaterializeOutcome {
    /// Policy forbade any materialization; nothing was written.
    SkippedByPolicy,
    /// Validated in a sandbox copy; the host was never touched.
    SandboxValidated,
    /// Validated in a sandbox copy and it failed; the host was never touched.
    SandboxRejected,
    /// Applied to the host workspace and validation passed.
    AppliedToHost,
    /// Applied to the host, validation failed, host restored to prior state.
    RolledBack,
}

impl MaterializeOutcome {
    fn tag(self) -> u8 {
        match self {
            MaterializeOutcome::SkippedByPolicy => 0,
            MaterializeOutcome::SandboxValidated => 1,
            MaterializeOutcome::SandboxRejected => 2,
            MaterializeOutcome::AppliedToHost => 3,
            MaterializeOutcome::RolledBack => 4,
        }
    }
}

#[derive(Serialize)]
struct ReportContent {
    patch_id: Digest,
    outcome_tag: u8,
    applied_to_host: bool,
    compile_passed: Option<bool>,
    tests_passed: Option<bool>,
}

/// Content-addressed record of one materialization.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaterializeReport {
    pub report_id: Digest,
    pub patch_id: Digest,
    pub outcome: MaterializeOutcome,
    /// `true` only when changes persist on the host after this call.
    pub applied_to_host: bool,
    pub compile_passed: Option<bool>,
    pub tests_passed: Option<bool>,
    pub gate_result: GateResult,
    /// Number of file changes applied (to host or sandbox); not a net count.
    pub files_written: u32,
    pub blocking_reason: Option<String>,
    /// Non-ZERO evidence reference (CROSS-006).
    pub evidence_bundle_id: Digest,
    pub diagnostics: Vec<String>,
}

impl MaterializeReport {
    fn build(
        patch_id: Digest,
        outcome: MaterializeOutcome,
        applied_to_host: bool,
        compile_passed: Option<bool>,
        tests_passed: Option<bool>,
        gate_result: GateResult,
        files_written: u32,
        blocking_reason: Option<String>,
        evidence_bundle_id: Digest,
        diagnostics: Vec<String>,
    ) -> Self {
        let report_id = Digest::of(&ReportContent {
            patch_id,
            outcome_tag: outcome.tag(),
            applied_to_host,
            compile_passed,
            tests_passed,
        });
        Self {
            report_id,
            patch_id,
            outcome,
            applied_to_host,
            compile_passed,
            tests_passed,
            gate_result,
            files_written,
            blocking_reason,
            evidence_bundle_id,
            diagnostics,
        }
    }

    fn skipped(patch_id: Digest, reason: String, evidence: Digest) -> Self {
        Self::build(
            patch_id,
            MaterializeOutcome::SkippedByPolicy,
            false,
            None,
            None,
            GateResult::Warn { message: reason.clone() },
            0,
            Some(reason),
            evidence,
            vec![],
        )
    }

    fn from_validation(
        patch_id: Digest,
        outcome: MaterializeOutcome,
        applied_to_host: bool,
        validation: ValidationReport,
        files_written: u32,
        blocking_reason: Option<String>,
    ) -> Self {
        Self::build(
            patch_id,
            outcome,
            applied_to_host,
            validation.compile_passed,
            validation.tests_passed,
            validation.gate_result,
            files_written,
            blocking_reason,
            validation.evidence_bundle_id,
            validation.diagnostics,
        )
    }
}

// ─── Materializer ───────────────────────────────────────────────────────────

enum Strategy {
    Skip(String),
    Sandbox,
    InPlace,
}

fn strategy_for(policy: &PolicyProfile) -> Strategy {
    match policy.mode {
        ImplementationMode::ReportOnly => Strategy::Skip("report-only: patch not materialized".into()),
        ImplementationMode::DryRun => Strategy::Sandbox,
        // OperatorApproved and AutonomousBounded may write the host — but only
        // when allow_host_write is set (fail-closed otherwise).
        ImplementationMode::OperatorApproved | ImplementationMode::AutonomousBounded
            if policy.allow_host_write =>
        {
            Strategy::InPlace
        }
        ImplementationMode::OperatorApproved | ImplementationMode::AutonomousBounded => {
            Strategy::Skip("host-write mode but allow_host_write=false".into())
        }
    }
}

/// Applies patches to a workspace under policy control.
pub struct Materializer {
    workspace_root: PathBuf,
}

impl Materializer {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self { workspace_root: workspace_root.into() }
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    /// Materialize `patch` under `policy`, validating with `validator`.
    pub fn materialize(
        &self,
        patch: &Patch,
        policy: &PolicyProfile,
        validator: &dyn PatchValidator,
        options: &MaterializeOptions,
    ) -> io::Result<MaterializeReport> {
        // CROSS-006: evidence must never be ZERO.
        let evidence = if patch.patch_id != Digest::ZERO {
            patch.patch_id
        } else {
            Digest::of_bytes(b"kosmo-materialize-evidence")
        };
        let files = patch.file_changes.len() as u32;

        match strategy_for(policy) {
            Strategy::Skip(reason) => Ok(MaterializeReport::skipped(patch.patch_id, reason, evidence)),

            Strategy::Sandbox => {
                let sandbox = make_sandbox(&patch.patch_id)?;
                let copy_result = (|| {
                    copy_tree(&self.workspace_root, &sandbox, &["target", ".git"])?;
                    apply_changes(&sandbox, &patch.file_changes)?;
                    Ok::<(), io::Error>(())
                })();
                if let Err(e) = copy_result {
                    let _ = fs::remove_dir_all(&sandbox);
                    return Err(e);
                }
                let validation = validator.validate(&sandbox, options.run_tests);
                let _ = fs::remove_dir_all(&sandbox); // best-effort cleanup
                let outcome = if validation.is_acceptable() {
                    MaterializeOutcome::SandboxValidated
                } else {
                    MaterializeOutcome::SandboxRejected
                };
                Ok(MaterializeReport::from_validation(
                    patch.patch_id,
                    outcome,
                    false,
                    validation,
                    files,
                    None,
                ))
            }

            Strategy::InPlace => {
                let backups = apply_changes(&self.workspace_root, &patch.file_changes)?;
                let validation = validator.validate(&self.workspace_root, options.run_tests);
                if validation.is_acceptable() {
                    Ok(MaterializeReport::from_validation(
                        patch.patch_id,
                        MaterializeOutcome::AppliedToHost,
                        true,
                        validation,
                        files,
                        None,
                    ))
                } else {
                    restore(&backups)?;
                    Ok(MaterializeReport::from_validation(
                        patch.patch_id,
                        MaterializeOutcome::RolledBack,
                        false,
                        validation,
                        files,
                        Some("validation failed; host restored to prior state".into()),
                    ))
                }
            }
        }
    }
}

// ─── Filesystem helpers ─────────────────────────────────────────────────────

/// A record of what one file looked like before a change, for rollback.
struct Backup {
    path: PathBuf,
    /// Prior bytes, or `None` if the file did not exist before.
    prior: Option<Vec<u8>>,
}

/// Apply `changes` to `root`, returning backups in application order.
fn apply_changes(root: &Path, changes: &[FileChange]) -> io::Result<Vec<Backup>> {
    let mut backups = Vec::with_capacity(changes.len());
    for ch in changes {
        let target = root.join(&ch.path);
        let prior = fs::read(&target).ok();
        match ch.kind {
            FileChangeKind::Delete => {
                if target.exists() {
                    fs::remove_file(&target)?;
                }
            }
            FileChangeKind::Create | FileChangeKind::Modify => {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&target, ch.content.as_bytes())?;
            }
        }
        backups.push(Backup { path: target, prior });
    }
    Ok(backups)
}

/// Restore every backed-up file to its prior state, in reverse order.
fn restore(backups: &[Backup]) -> io::Result<()> {
    for b in backups.iter().rev() {
        match &b.prior {
            Some(bytes) => {
                if let Some(parent) = b.path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&b.path, bytes)?;
            }
            None => {
                if b.path.exists() {
                    fs::remove_file(&b.path)?;
                }
            }
        }
    }
    Ok(())
}

/// Recursively copy `src` into `dst`, skipping any top-level/nested directory
/// whose name matches an entry in `skip` (e.g. `target`, `.git`). Symlinks are
/// ignored for safety.
fn copy_tree(src: &Path, dst: &Path, skip: &[&str]) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        if skip.iter().any(|s| *s == name.to_string_lossy()) {
            continue;
        }
        let ft = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(&name);
        if ft.is_dir() {
            copy_tree(&from, &to, skip)?;
        } else if ft.is_file() {
            fs::copy(&from, &to)?;
        }
        // symlinks and other node types are intentionally skipped
    }
    Ok(())
}

fn make_sandbox(patch_id: &Digest) -> io::Result<PathBuf> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let hex = patch_id.to_hex();
    let short = &hex[..hex.len().min(12)];
    let dir = std::env::temp_dir().join(format!("kosmo-materialize-{short}-{nanos}"));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_synthesizer::{FileChange, Patch};

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    /// A throwaway workspace directory under the temp dir.
    struct TempWorkspace {
        root: PathBuf,
    }
    impl TempWorkspace {
        fn new(tag: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!("kosmo-mat-test-{tag}-{nanos}"));
            fs::create_dir_all(&root).unwrap();
            Self { root }
        }
        fn write(&self, rel: &str, content: &str) {
            let p = self.root.join(rel);
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(p, content).unwrap();
        }
        fn read(&self, rel: &str) -> Option<String> {
            fs::read_to_string(self.root.join(rel)).ok()
        }
        fn exists(&self, rel: &str) -> bool {
            self.root.join(rel).exists()
        }
    }
    impl Drop for TempWorkspace {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn patch_create(path: &str, content: &str) -> Patch {
        Patch::new(d(b"req"), vec![FileChange::create(path, content)], "test")
    }

    #[test]
    fn report_only_skips_with_no_io() {
        let ws = TempWorkspace::new("ro");
        let patch = patch_create("src/new.rs", "pub fn n() {}");
        let m = Materializer::new(&ws.root);
        let rep = m
            .materialize(&patch, &PolicyProfile::default_report_only(), &AlwaysPass, &MaterializeOptions::default())
            .unwrap();
        assert_eq!(rep.outcome, MaterializeOutcome::SkippedByPolicy);
        assert!(!rep.applied_to_host);
        assert_eq!(rep.files_written, 0);
        assert!(!ws.exists("src/new.rs"));
    }

    #[test]
    fn dry_run_validates_in_sandbox_without_touching_host() {
        let ws = TempWorkspace::new("dry");
        ws.write("Cargo.toml", "[package]\n");
        let patch = patch_create("src/added.rs", "pub fn a() {}");
        let m = Materializer::new(&ws.root);
        let rep = m
            .materialize(&patch, &PolicyProfile::dry_run(), &AlwaysPass, &MaterializeOptions::default())
            .unwrap();
        assert_eq!(rep.outcome, MaterializeOutcome::SandboxValidated);
        assert!(!rep.applied_to_host);
        assert_eq!(rep.compile_passed, Some(true));
        // Host must NOT have the new file — only the sandbox copy did.
        assert!(!ws.exists("src/added.rs"));
    }

    #[test]
    fn dry_run_rejects_when_validation_fails() {
        let ws = TempWorkspace::new("dryfail");
        ws.write("Cargo.toml", "[package]\n");
        let patch = patch_create("src/added.rs", "broken");
        let m = Materializer::new(&ws.root);
        let rep = m
            .materialize(&patch, &PolicyProfile::dry_run(), &AlwaysFail, &MaterializeOptions::default())
            .unwrap();
        assert_eq!(rep.outcome, MaterializeOutcome::SandboxRejected);
        assert!(!rep.applied_to_host);
        assert!(!ws.exists("src/added.rs"));
    }

    #[test]
    fn operator_approved_applies_to_host_on_success() {
        let ws = TempWorkspace::new("ok");
        let patch = patch_create("src/added.rs", "pub fn a() {}");
        let m = Materializer::new(&ws.root);
        let rep = m
            .materialize(&patch, &PolicyProfile::operator_approved(), &AlwaysPass, &MaterializeOptions::default())
            .unwrap();
        assert_eq!(rep.outcome, MaterializeOutcome::AppliedToHost);
        assert!(rep.applied_to_host);
        assert_eq!(ws.read("src/added.rs").as_deref(), Some("pub fn a() {}"));
    }

    #[test]
    fn operator_approved_rolls_back_modified_file_on_failure() {
        let ws = TempWorkspace::new("rollback-mod");
        ws.write("src/lib.rs", "ORIGINAL");
        let patch = Patch::new(d(b"r"), vec![FileChange::modify("src/lib.rs", "MUTATED")], "test");
        let m = Materializer::new(&ws.root);
        let rep = m
            .materialize(&patch, &PolicyProfile::operator_approved(), &AlwaysFail, &MaterializeOptions::default())
            .unwrap();
        assert_eq!(rep.outcome, MaterializeOutcome::RolledBack);
        assert!(!rep.applied_to_host);
        // File content restored to the original.
        assert_eq!(ws.read("src/lib.rs").as_deref(), Some("ORIGINAL"));
        assert!(rep.blocking_reason.is_some());
    }

    #[test]
    fn operator_approved_rolls_back_created_file_on_failure() {
        let ws = TempWorkspace::new("rollback-new");
        let patch = patch_create("src/brandnew.rs", "pub fn x() {}");
        let m = Materializer::new(&ws.root);
        let rep = m
            .materialize(&patch, &PolicyProfile::operator_approved(), &AlwaysFail, &MaterializeOptions::default())
            .unwrap();
        assert_eq!(rep.outcome, MaterializeOutcome::RolledBack);
        // The file we created must be gone again after rollback.
        assert!(!ws.exists("src/brandnew.rs"));
    }

    #[test]
    fn operator_approved_restores_deleted_file_on_failure() {
        let ws = TempWorkspace::new("rollback-del");
        ws.write("src/keep.rs", "KEEP ME");
        let patch = Patch::new(d(b"r"), vec![FileChange::delete("src/keep.rs")], "test");
        let m = Materializer::new(&ws.root);
        let rep = m
            .materialize(&patch, &PolicyProfile::operator_approved(), &AlwaysFail, &MaterializeOptions::default())
            .unwrap();
        assert_eq!(rep.outcome, MaterializeOutcome::RolledBack);
        assert_eq!(ws.read("src/keep.rs").as_deref(), Some("KEEP ME"));
    }

    #[test]
    fn operator_approved_without_host_write_is_blocked() {
        let ws = TempWorkspace::new("nohostwrite");
        let mut policy = PolicyProfile::operator_approved();
        policy.allow_host_write = false;
        let patch = patch_create("src/x.rs", "pub fn x() {}");
        let m = Materializer::new(&ws.root);
        let rep = m
            .materialize(&patch, &policy, &AlwaysPass, &MaterializeOptions::default())
            .unwrap();
        assert_eq!(rep.outcome, MaterializeOutcome::SkippedByPolicy);
        assert!(!ws.exists("src/x.rs"));
    }

    #[test]
    fn report_id_is_deterministic() {
        let ws = TempWorkspace::new("det");
        let patch = patch_create("src/a.rs", "pub fn a() {}");
        let m = Materializer::new(&ws.root);
        let r1 = m
            .materialize(&patch, &PolicyProfile::operator_approved(), &AlwaysPass, &MaterializeOptions::default())
            .unwrap();
        let r2 = m
            .materialize(&patch, &PolicyProfile::operator_approved(), &AlwaysPass, &MaterializeOptions::default())
            .unwrap();
        assert_eq!(r1.report_id, r2.report_id);
    }

    #[test]
    fn evidence_bundle_id_is_never_zero() {
        let ws = TempWorkspace::new("ev");
        let patch = patch_create("src/a.rs", "pub fn a() {}");
        let m = Materializer::new(&ws.root);
        for policy in [
            PolicyProfile::default_report_only(),
            PolicyProfile::dry_run(),
            PolicyProfile::operator_approved(),
        ] {
            let rep = m
                .materialize(&patch, &policy, &AlwaysPass, &MaterializeOptions::default())
                .unwrap();
            assert_ne!(rep.evidence_bundle_id, Digest::ZERO);
        }
    }

    #[test]
    fn apply_then_restore_round_trips() {
        let ws = TempWorkspace::new("apply");
        ws.write("a.rs", "A0");
        let changes = vec![
            FileChange::modify("a.rs", "A1"),
            FileChange::create("b.rs", "B1"),
        ];
        let backups = apply_changes(&ws.root, &changes).unwrap();
        assert_eq!(ws.read("a.rs").as_deref(), Some("A1"));
        assert_eq!(ws.read("b.rs").as_deref(), Some("B1"));
        restore(&backups).unwrap();
        assert_eq!(ws.read("a.rs").as_deref(), Some("A0"));
        assert!(!ws.exists("b.rs"));
    }
}

use kosmo_core::{
    Digest, EvidenceBundle, EvidenceKind, EvidenceRef, FoundryCheckKind, FoundryCheckResult,
    FoundryOutcome, LedgerEvent, LedgerEventKind, PolicyProfile, ReplayStatus,
};
use serde::{Deserialize, Serialize};

/// Specification for a single Foundry check (build, test, lint, etc.).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FoundryCheckSpec {
    pub kind: FoundryCheckKind,
    /// Executable to invoke (e.g. "cargo"). Empty means unavailable.
    pub command: String,
    pub args: Vec<String>,
}

impl FoundryCheckSpec {
    pub fn new(kind: FoundryCheckKind, command: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            kind,
            command: command.into(),
            args,
        }
    }
}

/// Runs configured Foundry checks and emits `FoundryCheckResult`s and
/// a corresponding `EvidenceBundle`.
///
/// Behavior by policy mode:
/// - `ReportOnly`: all checks return `Skipped` (no processes launched).
/// - `DryRun` and above: checks are executed via `std::process::Command`.
///
/// The runner never modifies host files. It only reads (cargo check/test output).
pub struct FoundryRunner {
    pub policy: PolicyProfile,
    pub checks: Vec<FoundryCheckSpec>,
}

impl FoundryRunner {
    pub fn new(policy: PolicyProfile) -> Self {
        Self {
            policy,
            checks: vec![],
        }
    }

    /// Construct with the standard check suite (build + test).
    pub fn standard_checks(policy: PolicyProfile) -> Self {
        Self {
            checks: vec![
                FoundryCheckSpec::new(
                    FoundryCheckKind::Build,
                    "cargo",
                    vec!["check".into(), "--workspace".into(), "--quiet".into()],
                ),
                FoundryCheckSpec::new(
                    FoundryCheckKind::Test,
                    "cargo",
                    vec!["test".into(), "--workspace".into(), "--quiet".into()],
                ),
            ],
            policy,
        }
    }

    pub fn with_check(mut self, spec: FoundryCheckSpec) -> Self {
        self.checks.push(spec);
        self
    }

    /// Run a single check. In `ReportOnly` mode the command is never executed.
    pub fn run_check(&self, spec: &FoundryCheckSpec, evidence_id: Digest) -> FoundryCheckResult {
        if self.policy.is_report_only() {
            return FoundryCheckResult::new(
                spec.kind.clone(),
                FoundryOutcome::Skipped {
                    reason: "ReportOnly mode — no processes launched".into(),
                },
                evidence_id,
                vec![],
            );
        }

        if spec.command.is_empty() {
            return FoundryCheckResult::new(
                spec.kind.clone(),
                FoundryOutcome::Unavailable {
                    reason: "no command configured".into(),
                },
                evidence_id,
                vec![],
            );
        }

        match std::process::Command::new(&spec.command)
            .args(&spec.args)
            .output()
        {
            Ok(out) => {
                if out.status.success() {
                    FoundryCheckResult::new(
                        spec.kind.clone(),
                        FoundryOutcome::Passed,
                        evidence_id,
                        vec![String::from_utf8_lossy(&out.stdout).trim().to_string()]
                            .into_iter()
                            .filter(|s| !s.is_empty())
                            .collect(),
                    )
                } else {
                    FoundryCheckResult::new(
                        spec.kind.clone(),
                        FoundryOutcome::Failed {
                            exit_code: out.status.code().unwrap_or(-1),
                            stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
                        },
                        evidence_id,
                        vec![],
                    )
                }
            }
            Err(e) => FoundryCheckResult::new(
                spec.kind.clone(),
                FoundryOutcome::Unavailable {
                    reason: format!("command not found or failed to launch: {e}"),
                },
                evidence_id,
                vec![],
            ),
        }
    }

    /// Run all configured checks and return results with an evidence bundle.
    pub fn run_all(&self, workspace_evidence_id: Digest) -> FoundryRunOutput {
        let mut results = Vec::with_capacity(self.checks.len());
        let mut events = Vec::with_capacity(self.checks.len());

        for (seq, spec) in self.checks.iter().enumerate() {
            let result = self.run_check(spec, workspace_evidence_id);
            let event = LedgerEvent::new(LedgerEventKind::GateCascadeDecision, seq as u64)
                .with_run_id(workspace_evidence_id)
                .with_gate_result(if result.outcome.is_passed() {
                    kosmo_core::GateResult::Pass
                } else {
                    kosmo_core::GateResult::Warn {
                        message: format!("{:?} not passed", result.check_kind),
                    }
                });
            events.push(event);
            results.push(result);
        }

        // Collect evidence refs from all check results
        let refs: Vec<EvidenceRef> = results
            .iter()
            .map(|r| {
                EvidenceRef::new(
                    r.check_id,
                    EvidenceKind::FoundryCheck,
                    format!("{:?}", r.check_kind),
                )
            })
            .collect();

        let bundle = EvidenceBundle::seal(refs, self.policy.id, ReplayStatus::Replayable);

        FoundryRunOutput {
            results,
            evidence_bundle: bundle,
            ledger_events: events,
        }
    }
}

/// Output of a full Foundry run.
pub struct FoundryRunOutput {
    pub results: Vec<FoundryCheckResult>,
    pub evidence_bundle: EvidenceBundle,
    pub ledger_events: Vec<LedgerEvent>,
}

impl FoundryRunOutput {
    pub fn all_passed(&self) -> bool {
        self.results.iter().all(|r| r.outcome.is_passed())
    }

    pub fn any_failed(&self) -> bool {
        self.results.iter().any(|r| r.outcome.is_failed())
    }

    pub fn skipped_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| matches!(r.outcome, FoundryOutcome::Skipped { .. }))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::{Digest, FoundryOutcome, PolicyProfile};

    #[test]
    fn foundry_runner_report_only_skips_all() {
        let policy = PolicyProfile::default_report_only();
        let runner = FoundryRunner::standard_checks(policy);
        let output = runner.run_all(Digest::ZERO);
        assert_eq!(output.skipped_count(), 2);
        assert!(!output.any_failed());
        assert!(output.evidence_bundle.verify_id());
    }

    #[test]
    fn foundry_runner_report_only_no_processes() {
        let policy = PolicyProfile::default_report_only();
        let runner = FoundryRunner::new(policy).with_check(FoundryCheckSpec::new(
            FoundryCheckKind::Custom("impossible".into()),
            "/nonexistent/binary",
            vec![],
        ));
        let output = runner.run_all(Digest::ZERO);
        // In ReportOnly mode the binary is NOT invoked — result is Skipped, not Unavailable
        assert!(
            matches!(output.results[0].outcome, FoundryOutcome::Skipped { .. }),
            "expected Skipped in ReportOnly mode, got {:?}",
            output.results[0].outcome
        );
    }

    #[test]
    fn foundry_runner_empty_command_is_unavailable_in_dry_run() {
        let policy = PolicyProfile::dry_run();
        let runner = FoundryRunner::new(policy).with_check(FoundryCheckSpec::new(
            FoundryCheckKind::Build,
            "", // empty command
            vec![],
        ));
        let output = runner.run_all(Digest::ZERO);
        assert!(matches!(
            output.results[0].outcome,
            FoundryOutcome::Unavailable { .. }
        ));
    }

    #[test]
    fn foundry_run_output_evidence_bundle_is_sealed() {
        let policy = PolicyProfile::default_report_only();
        let runner = FoundryRunner::standard_checks(policy);
        let output = runner.run_all(Digest::of_bytes(b"workspace"));
        assert!(output.evidence_bundle.verify_id());
        // Two checks → two evidence refs
        assert_eq!(output.evidence_bundle.refs.len(), 2);
    }

    #[test]
    #[ignore = "integration: requires cargo on PATH"]
    fn foundry_dry_run_executes_cargo_version() {
        let policy = PolicyProfile::dry_run();
        let runner = FoundryRunner::new(policy).with_check(FoundryCheckSpec::new(
            FoundryCheckKind::Custom("cargo-version".into()),
            "cargo",
            vec!["--version".into()],
        ));
        let output = runner.run_all(Digest::ZERO);
        assert!(output.results[0].outcome.is_passed());
    }
}

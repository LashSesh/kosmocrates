use crate::digest::Digest;
use crate::foundry::FoundryExecutionOutcome;
use crate::parseback::ParseBackOutcome;
use serde::{Deserialize, Serialize};

/// Combined decision from a Foundry execution and a parse-back rescan.
///
/// Both sub-systems must pass independently for `Passed`.
/// Any failure is surfaced as a specific `Failed*` variant (worst-wins).
/// `PassedWithWarnings` is emitted when both pass but warnings were present.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationClosureStatus {
    /// Both Foundry and ParseBack passed with no warnings.
    Passed,
    /// Both passed, but at least one produced a warning-class signal.
    PassedWithWarnings,
    /// Foundry failed; ParseBack passed or was not reached.
    FailedFoundry,
    /// Foundry passed; ParseBack failed.
    FailedParseBack,
    /// Both Foundry and ParseBack failed.
    FailedBoth,
    /// Status cannot be determined (e.g., either sub-system was SkippedByReportOnly,
    /// or both produced Inconclusive outcomes).
    Inconclusive,
}

impl ValidationClosureStatus {
    pub fn is_passed(&self) -> bool {
        matches!(self, Self::Passed | Self::PassedWithWarnings)
    }

    pub fn is_failure_class(&self) -> bool {
        matches!(
            self,
            Self::FailedFoundry | Self::FailedParseBack | Self::FailedBoth
        )
    }

    pub fn is_inconclusive(&self) -> bool {
        matches!(self, Self::Inconclusive)
    }
}

/// Computes the `ValidationClosureStatus` from the two sub-system outcomes.
///
/// Rules (in priority order):
/// 1. Either outcome is `SkippedByReportOnly` → `Inconclusive`
///    (cannot validate without execution)
/// 2. Both are failure-class → `FailedBoth`
/// 3. Foundry is failure-class only → `FailedFoundry`
/// 4. ParseBack is failure-class only → `FailedParseBack`
/// 5. Both pass + `has_warnings` → `PassedWithWarnings`
/// 6. Both pass, no warnings → `Passed`
/// 7. Fallthrough → `Inconclusive`
pub fn determine_closure_status(
    foundry: &FoundryExecutionOutcome,
    parseback: &ParseBackOutcome,
    has_warnings: bool,
) -> ValidationClosureStatus {
    if foundry.is_skipped_report_only() || parseback.is_skipped_report_only() {
        return ValidationClosureStatus::Inconclusive;
    }

    let foundry_failed = foundry.is_failure_class();
    let parseback_failed = parseback.is_failure_class();

    match (foundry_failed, parseback_failed) {
        (true, true) => ValidationClosureStatus::FailedBoth,
        (true, false) => ValidationClosureStatus::FailedFoundry,
        (false, true) => ValidationClosureStatus::FailedParseBack,
        (false, false) => {
            if foundry.is_passed() && parseback.is_passed() {
                if has_warnings {
                    ValidationClosureStatus::PassedWithWarnings
                } else {
                    ValidationClosureStatus::Passed
                }
            } else {
                ValidationClosureStatus::Inconclusive
            }
        }
    }
}

/// The complete, content-addressed validation closure record.
///
/// Combines the outcomes of a `FoundryExecutionReport` and a `ParseBackReport`
/// into a single authoritative decision for a materialization attempt.
///
/// `id` is the content digest of all fields except `id`.
/// `evidence_bundle_id` is mandatory — every durable decision must be evidence-bound.
/// `has_warnings` is set by the caller after inspecting sub-report diagnostics
/// (e.g., `ParseBackReport::has_critical_delta()` or non-empty check diagnostics).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationClosureReport {
    pub id: Digest,
    pub materialization_plan_id: Digest,
    pub foundry_report_id: Digest,
    pub parseback_report_id: Digest,
    pub foundry_outcome: FoundryExecutionOutcome,
    pub parseback_outcome: ParseBackOutcome,
    pub final_validation_status: ValidationClosureStatus,
    pub has_warnings: bool,
    pub diagnostics: Vec<String>,
    pub evidence_bundle_id: Digest,
}

#[derive(Serialize)]
struct ClosureContent<'a> {
    materialization_plan_id: &'a Digest,
    foundry_report_id: &'a Digest,
    parseback_report_id: &'a Digest,
    foundry_outcome: &'a FoundryExecutionOutcome,
    parseback_outcome: &'a ParseBackOutcome,
    final_validation_status: &'a ValidationClosureStatus,
    has_warnings: bool,
    diagnostics: &'a Vec<String>,
    evidence_bundle_id: &'a Digest,
}

impl ValidationClosureReport {
    /// Constructs the report and computes `final_validation_status` automatically
    /// via `determine_closure_status`.
    pub fn new(
        materialization_plan_id: Digest,
        foundry_report_id: Digest,
        parseback_report_id: Digest,
        foundry_outcome: FoundryExecutionOutcome,
        parseback_outcome: ParseBackOutcome,
        has_warnings: bool,
        diagnostics: Vec<String>,
        evidence_bundle_id: Digest,
    ) -> Self {
        let final_validation_status =
            determine_closure_status(&foundry_outcome, &parseback_outcome, has_warnings);
        let mut r = Self {
            id: Digest::ZERO,
            materialization_plan_id,
            foundry_report_id,
            parseback_report_id,
            foundry_outcome,
            parseback_outcome,
            final_validation_status,
            has_warnings,
            diagnostics,
            evidence_bundle_id,
        };
        r.id = r.compute_id();
        r
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&ClosureContent {
            materialization_plan_id: &self.materialization_plan_id,
            foundry_report_id: &self.foundry_report_id,
            parseback_report_id: &self.parseback_report_id,
            foundry_outcome: &self.foundry_outcome,
            parseback_outcome: &self.parseback_outcome,
            final_validation_status: &self.final_validation_status,
            has_warnings: self.has_warnings,
            diagnostics: &self.diagnostics,
            evidence_bundle_id: &self.evidence_bundle_id,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{EvidenceBundle, EvidenceKind, EvidenceRef, ReplayStatus};
    use crate::foundry::FoundryExecutionOutcome;
    use crate::parseback::ParseBackOutcome;

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    fn mat_plan_id() -> Digest {
        d(b"mat-plan")
    }
    fn foundry_report_id() -> Digest {
        d(b"foundry-report")
    }
    fn parseback_report_id() -> Digest {
        d(b"parseback-report")
    }
    fn bundle_id() -> Digest {
        d(b"bundle")
    }

    fn make_report(
        fo: FoundryExecutionOutcome,
        po: ParseBackOutcome,
        has_warnings: bool,
    ) -> ValidationClosureReport {
        ValidationClosureReport::new(
            mat_plan_id(),
            foundry_report_id(),
            parseback_report_id(),
            fo,
            po,
            has_warnings,
            vec![],
            bundle_id(),
        )
    }

    // --- ValidationClosureStatus helpers ---

    #[test]
    fn status_passed_is_passed() {
        assert!(ValidationClosureStatus::Passed.is_passed());
        assert!(!ValidationClosureStatus::Passed.is_failure_class());
    }

    #[test]
    fn status_passed_with_warnings_is_passed() {
        assert!(ValidationClosureStatus::PassedWithWarnings.is_passed());
    }

    #[test]
    fn status_failure_variants() {
        for s in [
            ValidationClosureStatus::FailedFoundry,
            ValidationClosureStatus::FailedParseBack,
            ValidationClosureStatus::FailedBoth,
        ] {
            assert!(s.is_failure_class(), "{s:?} should be failure class");
            assert!(!s.is_passed());
        }
    }

    #[test]
    fn status_inconclusive_is_neither() {
        let s = ValidationClosureStatus::Inconclusive;
        assert!(!s.is_passed());
        assert!(!s.is_failure_class());
        assert!(s.is_inconclusive());
    }

    // --- determine_closure_status ---

    #[test]
    fn closure_both_pass_no_warnings() {
        let s = determine_closure_status(
            &FoundryExecutionOutcome::Passed,
            &ParseBackOutcome::Passed,
            false,
        );
        assert_eq!(s, ValidationClosureStatus::Passed);
    }

    #[test]
    fn closure_both_pass_with_warnings() {
        let s = determine_closure_status(
            &FoundryExecutionOutcome::Passed,
            &ParseBackOutcome::TopologyUnchanged,
            true,
        );
        assert_eq!(s, ValidationClosureStatus::PassedWithWarnings);
    }

    #[test]
    fn closure_topology_unchanged_is_passed() {
        let s = determine_closure_status(
            &FoundryExecutionOutcome::Passed,
            &ParseBackOutcome::TopologyUnchanged,
            false,
        );
        assert_eq!(s, ValidationClosureStatus::Passed);
    }

    #[test]
    fn closure_foundry_failed_parseback_passed() {
        let s = determine_closure_status(
            &FoundryExecutionOutcome::Failed,
            &ParseBackOutcome::Passed,
            false,
        );
        assert_eq!(s, ValidationClosureStatus::FailedFoundry);
    }

    #[test]
    fn closure_foundry_passed_parseback_failed() {
        let s = determine_closure_status(
            &FoundryExecutionOutcome::Passed,
            &ParseBackOutcome::Failed,
            false,
        );
        assert_eq!(s, ValidationClosureStatus::FailedParseBack);
    }

    #[test]
    fn closure_both_failed() {
        let s = determine_closure_status(
            &FoundryExecutionOutcome::Failed,
            &ParseBackOutcome::Failed,
            false,
        );
        assert_eq!(s, ValidationClosureStatus::FailedBoth);
    }

    #[test]
    fn closure_foundry_timed_out_parseback_passed() {
        let s = determine_closure_status(
            &FoundryExecutionOutcome::TimedOut,
            &ParseBackOutcome::Passed,
            false,
        );
        assert_eq!(s, ValidationClosureStatus::FailedFoundry);
    }

    #[test]
    fn closure_sandbox_violation_is_foundry_failed() {
        let s = determine_closure_status(
            &FoundryExecutionOutcome::SandboxViolation,
            &ParseBackOutcome::Passed,
            false,
        );
        assert_eq!(s, ValidationClosureStatus::FailedFoundry);
    }

    #[test]
    fn closure_command_denied_is_foundry_failed() {
        let s = determine_closure_status(
            &FoundryExecutionOutcome::CommandDeniedByPolicy,
            &ParseBackOutcome::Inconclusive,
            false,
        );
        assert_eq!(s, ValidationClosureStatus::FailedBoth);
    }

    #[test]
    fn closure_foundry_skipped_is_inconclusive() {
        let s = determine_closure_status(
            &FoundryExecutionOutcome::SkippedByReportOnly,
            &ParseBackOutcome::Passed,
            false,
        );
        assert_eq!(s, ValidationClosureStatus::Inconclusive);
    }

    #[test]
    fn closure_parseback_skipped_is_inconclusive() {
        let s = determine_closure_status(
            &FoundryExecutionOutcome::Passed,
            &ParseBackOutcome::SkippedByReportOnly,
            false,
        );
        assert_eq!(s, ValidationClosureStatus::Inconclusive);
    }

    #[test]
    fn closure_both_skipped_is_inconclusive() {
        let s = determine_closure_status(
            &FoundryExecutionOutcome::SkippedByReportOnly,
            &ParseBackOutcome::SkippedByReportOnly,
            false,
        );
        assert_eq!(s, ValidationClosureStatus::Inconclusive);
    }

    #[test]
    fn closure_foundry_inconclusive_parseback_passed() {
        let s = determine_closure_status(
            &FoundryExecutionOutcome::Inconclusive,
            &ParseBackOutcome::Passed,
            false,
        );
        assert_eq!(s, ValidationClosureStatus::FailedFoundry);
    }

    // --- ValidationClosureReport ---

    #[test]
    fn report_id_deterministic() {
        let r1 = make_report(
            FoundryExecutionOutcome::Passed,
            ParseBackOutcome::Passed,
            false,
        );
        let r2 = make_report(
            FoundryExecutionOutcome::Passed,
            ParseBackOutcome::Passed,
            false,
        );
        assert_eq!(r1.id, r2.id);
    }

    #[test]
    fn report_verify_id() {
        let r = make_report(
            FoundryExecutionOutcome::Passed,
            ParseBackOutcome::Passed,
            false,
        );
        assert!(r.verify_id());
    }

    #[test]
    fn report_status_derived_automatically() {
        let r = make_report(
            FoundryExecutionOutcome::Failed,
            ParseBackOutcome::Failed,
            false,
        );
        assert_eq!(
            r.final_validation_status,
            ValidationClosureStatus::FailedBoth
        );
        assert!(r.verify_id());
    }

    #[test]
    fn report_passed_with_warnings_status() {
        let r = make_report(
            FoundryExecutionOutcome::Passed,
            ParseBackOutcome::Passed,
            true,
        );
        assert_eq!(
            r.final_validation_status,
            ValidationClosureStatus::PassedWithWarnings
        );
        assert!(r.verify_id());
    }

    #[test]
    fn report_different_outcomes_differ() {
        let r1 = make_report(
            FoundryExecutionOutcome::Passed,
            ParseBackOutcome::Passed,
            false,
        );
        let r2 = make_report(
            FoundryExecutionOutcome::Failed,
            ParseBackOutcome::Passed,
            false,
        );
        assert_ne!(r1.id, r2.id);
    }

    #[test]
    fn report_evidence_bundle_id_mandatory() {
        let r = make_report(
            FoundryExecutionOutcome::Passed,
            ParseBackOutcome::Passed,
            false,
        );
        assert_ne!(r.evidence_bundle_id, Digest::ZERO);
    }

    #[test]
    fn report_inconclusive_when_report_only() {
        use crate::policy::{ImplementationMode, PolicyProfile};
        let profile = PolicyProfile::default();
        assert_eq!(profile.mode, ImplementationMode::ReportOnly);

        let r = make_report(
            FoundryExecutionOutcome::SkippedByReportOnly,
            ParseBackOutcome::SkippedByReportOnly,
            false,
        );
        assert_eq!(
            r.final_validation_status,
            ValidationClosureStatus::Inconclusive
        );
        assert!(r.verify_id());
    }

    #[test]
    fn report_failed_foundry_only() {
        let r = make_report(
            FoundryExecutionOutcome::TimedOut,
            ParseBackOutcome::TopologyUnchanged,
            false,
        );
        assert_eq!(
            r.final_validation_status,
            ValidationClosureStatus::FailedFoundry
        );
        assert!(r.final_validation_status.is_failure_class());
        assert!(r.verify_id());
    }

    #[test]
    fn report_failed_parseback_only() {
        let r = make_report(
            FoundryExecutionOutcome::Passed,
            ParseBackOutcome::Failed,
            false,
        );
        assert_eq!(
            r.final_validation_status,
            ValidationClosureStatus::FailedParseBack
        );
        assert!(r.verify_id());
    }

    #[test]
    fn report_with_diagnostics_verify_id() {
        let r = ValidationClosureReport::new(
            mat_plan_id(),
            foundry_report_id(),
            parseback_report_id(),
            FoundryExecutionOutcome::Passed,
            ParseBackOutcome::Passed,
            true,
            vec![
                "warning: slow compile time".into(),
                "warning: large delta count".into(),
            ],
            bundle_id(),
        );
        assert!(r.verify_id());
        assert_eq!(
            r.final_validation_status,
            ValidationClosureStatus::PassedWithWarnings
        );
    }

    #[test]
    fn report_different_materialization_plan_differs() {
        let r1 = ValidationClosureReport::new(
            d(b"plan-a"),
            foundry_report_id(),
            parseback_report_id(),
            FoundryExecutionOutcome::Passed,
            ParseBackOutcome::Passed,
            false,
            vec![],
            bundle_id(),
        );
        let r2 = ValidationClosureReport::new(
            d(b"plan-b"),
            foundry_report_id(),
            parseback_report_id(),
            FoundryExecutionOutcome::Passed,
            ParseBackOutcome::Passed,
            false,
            vec![],
            bundle_id(),
        );
        assert_ne!(r1.id, r2.id);
    }

    #[test]
    fn r3_evidence_bundle_integration() {
        let ev_bundle = EvidenceBundle::seal(
            vec![
                EvidenceRef::new(
                    d(b"foundry-ev"),
                    EvidenceKind::FoundryCheck,
                    "foundry check",
                ),
                EvidenceRef::new(d(b"scan-ev"), EvidenceKind::HostScan, "parseback scan"),
            ],
            d(b"policy"),
            ReplayStatus::Replayable,
        );
        let r = ValidationClosureReport::new(
            mat_plan_id(),
            foundry_report_id(),
            parseback_report_id(),
            FoundryExecutionOutcome::Passed,
            ParseBackOutcome::Passed,
            false,
            vec![],
            ev_bundle.bundle_id,
        );
        assert!(r.verify_id());
        assert_ne!(r.evidence_bundle_id, Digest::ZERO);
        assert_eq!(r.final_validation_status, ValidationClosureStatus::Passed);
    }

    #[test]
    fn r3_worst_wins_both_failed_takes_precedence() {
        // FailedBoth must dominate over either individual failure variant.
        let both = determine_closure_status(
            &FoundryExecutionOutcome::SandboxViolation,
            &ParseBackOutcome::Inconclusive,
            false,
        );
        assert_eq!(both, ValidationClosureStatus::FailedBoth);
    }
}

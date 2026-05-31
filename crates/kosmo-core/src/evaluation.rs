use crate::digest::Digest;
use crate::evidence::EvidenceRef;
use crate::fixed_point::Q16;
use crate::policy::{ImplementationMode, PolicyProfile};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Classifies what aspect of the system an `EvaluationScenario` exercises.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvaluationDimension {
    FoundryExecution,
    ParseBackAccuracy,
    ValidationClosure,
    CartographyIntegrity,
    BridgeCandidate,
    AcquisitionTaint,
    End2End,
    Custom(String),
}

/// Acceptance thresholds for an `EvaluationScenario`.
///
/// All thresholds are `Q16` — no floats in any gate or metric comparison path.
/// `require_deterministic`: identical inputs must produce identical report IDs
/// (INVARIANT-007).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationCriteria {
    pub min_confidence: Q16,
    pub max_failure_rate: Q16,
    pub min_evidence_coverage: Q16,
    pub require_deterministic: bool,
}

#[derive(Serialize)]
struct CriteriaContent {
    min_confidence_raw: i64,
    max_failure_rate_raw: i64,
    min_evidence_coverage_raw: i64,
    require_deterministic: bool,
}

impl EvaluationCriteria {
    pub fn strict() -> Self {
        Self {
            min_confidence: Q16::ratio(9, 10).unwrap(),
            max_failure_rate: Q16::ratio(1, 10).unwrap(),
            min_evidence_coverage: Q16::ratio(4, 5).unwrap(),
            require_deterministic: true,
        }
    }

    pub fn permissive() -> Self {
        Self {
            min_confidence: Q16::HALF,
            max_failure_rate: Q16::HALF,
            min_evidence_coverage: Q16::ratio(1, 4).unwrap(),
            require_deterministic: true,
        }
    }

    fn content_digest(&self) -> Digest {
        Digest::of(&CriteriaContent {
            min_confidence_raw: self.min_confidence.raw(),
            max_failure_rate_raw: self.max_failure_rate.raw(),
            min_evidence_coverage_raw: self.min_evidence_coverage.raw(),
            require_deterministic: self.require_deterministic,
        })
    }
}

/// A fixture-driven evaluation scenario.
///
/// `input_state_digest` and `expected_outcome_digest` reference fixture
/// artifacts by digest — they are never inlined.
/// `id` is the content digest of all fields except `id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationScenario {
    pub id: Digest,
    pub name: String,
    pub dimension: EvaluationDimension,
    /// Digest of the input fixture artifact.
    pub input_state_digest: Digest,
    /// Digest of the expected output artifact.
    pub expected_outcome_digest: Digest,
    pub criteria: EvaluationCriteria,
    pub policy_id: Digest,
    pub evidence_refs: Vec<EvidenceRef>,
}

#[derive(Serialize)]
struct ScenarioContent<'a> {
    name: &'a String,
    dimension: &'a EvaluationDimension,
    input_state_digest: &'a Digest,
    expected_outcome_digest: &'a Digest,
    criteria_digest: Digest,
    policy_id: &'a Digest,
    evidence_digests: Vec<&'a Digest>,
}

impl EvaluationScenario {
    pub fn new(
        name: impl Into<String>,
        dimension: EvaluationDimension,
        input_state_digest: Digest,
        expected_outcome_digest: Digest,
        criteria: EvaluationCriteria,
        policy_id: Digest,
    ) -> Self {
        let mut s = Self {
            id: Digest::ZERO,
            name: name.into(),
            dimension,
            input_state_digest,
            expected_outcome_digest,
            criteria,
            policy_id,
            evidence_refs: vec![],
        };
        s.id = s.compute_id();
        s
    }

    pub fn with_evidence(mut self, refs: Vec<EvidenceRef>) -> Self {
        self.evidence_refs = refs;
        self.id = self.compute_id();
        self
    }

    fn compute_id(&self) -> Digest {
        let evidence_digests: Vec<&Digest> = self.evidence_refs.iter().map(|e| &e.digest).collect();
        Digest::of(&ScenarioContent {
            name: &self.name,
            dimension: &self.dimension,
            input_state_digest: &self.input_state_digest,
            expected_outcome_digest: &self.expected_outcome_digest,
            criteria_digest: self.criteria.content_digest(),
            policy_id: &self.policy_id,
            evidence_digests,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }
}

/// Q16-only metric record produced by a scenario run.
///
/// `additional` maps metric names to Q16 raw `i64` values — serialized
/// as integers, never floats. BTreeMap for deterministic ordering.
/// `id` is the content digest of all fields except `id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationMetrics {
    pub id: Digest,
    pub scenario_id: Digest,
    pub confidence_score: Q16,
    pub failure_rate: Q16,
    pub evidence_coverage: Q16,
    pub determinism_verified: bool,
    /// Additional Q16 metrics keyed by name; values are raw `i64` (Q16 × 2^16).
    pub additional: BTreeMap<String, i64>,
}

#[derive(Serialize)]
struct MetricsContent<'a> {
    scenario_id: &'a Digest,
    confidence_score_raw: i64,
    failure_rate_raw: i64,
    evidence_coverage_raw: i64,
    determinism_verified: bool,
    additional: &'a BTreeMap<String, i64>,
}

impl EvaluationMetrics {
    pub fn new(
        scenario_id: Digest,
        confidence_score: Q16,
        failure_rate: Q16,
        evidence_coverage: Q16,
        determinism_verified: bool,
    ) -> Self {
        let mut m = Self {
            id: Digest::ZERO,
            scenario_id,
            confidence_score,
            failure_rate,
            evidence_coverage,
            determinism_verified,
            additional: BTreeMap::new(),
        };
        m.id = m.compute_id();
        m
    }

    /// Record an additional Q16 metric by name (raw i64 value).
    pub fn with_metric(mut self, name: impl Into<String>, value_raw: i64) -> Self {
        self.additional.insert(name.into(), value_raw);
        self.id = self.compute_id();
        self
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&MetricsContent {
            scenario_id: &self.scenario_id,
            confidence_score_raw: self.confidence_score.raw(),
            failure_rate_raw: self.failure_rate.raw(),
            evidence_coverage_raw: self.evidence_coverage.raw(),
            determinism_verified: self.determinism_verified,
            additional: &self.additional,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }

    /// Returns `true` iff all metrics satisfy the given criteria.
    pub fn satisfies(&self, criteria: &EvaluationCriteria) -> bool {
        self.confidence_score >= criteria.min_confidence
            && self.failure_rate <= criteria.max_failure_rate
            && self.evidence_coverage >= criteria.min_evidence_coverage
            && (!criteria.require_deterministic || self.determinism_verified)
    }
}

/// Overall outcome of a single scenario run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvaluationOutcome {
    /// Skipped because `PolicyProfile.mode == ReportOnly`.
    SkippedByReportOnly,
    /// All acceptance criteria satisfied.
    Passed,
    /// Criteria satisfied but with warnings (e.g., confidence near threshold).
    PassedWithWarnings,
    /// One or more acceptance criteria violated.
    Failed { criteria_violations: Vec<String> },
    /// Outcome cannot be determined.
    Inconclusive,
}

impl EvaluationOutcome {
    pub fn is_passed(&self) -> bool {
        matches!(self, Self::Passed | Self::PassedWithWarnings)
    }

    pub fn is_skipped_report_only(&self) -> bool {
        matches!(self, Self::SkippedByReportOnly)
    }

    pub fn is_failure_class(&self) -> bool {
        matches!(self, Self::Failed { .. } | Self::Inconclusive)
    }
}

/// Content-addressed record of running one `EvaluationScenario`.
///
/// `id` is the content digest of all fields except `id`.
/// `elapsed_ms` is `u64` — no floats in any audit path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationRunReport {
    pub id: Digest,
    pub scenario_id: Digest,
    pub outcome: EvaluationOutcome,
    pub metrics: EvaluationMetrics,
    pub diagnostics: Vec<String>,
    pub evidence_bundle_id: Digest,
    pub elapsed_ms: u64,
}

#[derive(Serialize)]
struct RunReportContent<'a> {
    scenario_id: &'a Digest,
    outcome: &'a EvaluationOutcome,
    metrics_id: &'a Digest,
    diagnostics: &'a Vec<String>,
    evidence_bundle_id: &'a Digest,
    elapsed_ms: u64,
}

impl EvaluationRunReport {
    pub fn skipped_by_report_only(
        scenario_id: Digest,
        evidence_bundle_id: Digest,
    ) -> Self {
        let metrics = EvaluationMetrics::new(
            scenario_id,
            Q16::ZERO,
            Q16::ZERO,
            Q16::ZERO,
            false,
        );
        let mut r = Self {
            id: Digest::ZERO,
            scenario_id,
            outcome: EvaluationOutcome::SkippedByReportOnly,
            metrics,
            diagnostics: vec!["Skipped: PolicyProfile.mode == ReportOnly".into()],
            evidence_bundle_id,
            elapsed_ms: 0,
        };
        r.id = r.compute_id();
        r
    }

    pub fn new(
        scenario_id: Digest,
        outcome: EvaluationOutcome,
        metrics: EvaluationMetrics,
        diagnostics: Vec<String>,
        evidence_bundle_id: Digest,
        elapsed_ms: u64,
    ) -> Self {
        let mut r = Self {
            id: Digest::ZERO,
            scenario_id,
            outcome,
            metrics,
            diagnostics,
            evidence_bundle_id,
            elapsed_ms,
        };
        r.id = r.compute_id();
        r
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&RunReportContent {
            scenario_id: &self.scenario_id,
            outcome: &self.outcome,
            metrics_id: &self.metrics.id,
            diagnostics: &self.diagnostics,
            evidence_bundle_id: &self.evidence_bundle_id,
            elapsed_ms: self.elapsed_ms,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }
}

/// Overall outcome of an evaluation suite (aggregated across all scenarios).
///
/// Worst-wins: `AllFailed` dominates `SomeFailed` dominates
/// `SomePassedWithWarnings` dominates `AllPassed`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvaluationSuiteOutcome {
    AllPassed,
    SomePassedWithWarnings,
    SomeFailed,
    AllFailed,
    Inconclusive,
}

impl EvaluationSuiteOutcome {
    pub fn is_all_passed(&self) -> bool {
        matches!(self, Self::AllPassed)
    }

    pub fn is_failure_class(&self) -> bool {
        matches!(self, Self::SomeFailed | Self::AllFailed | Self::Inconclusive)
    }
}

/// Aggregates multiple `EvaluationRunReport`s into a suite-level decision.
///
/// Counts use `u64` — no floats. Worst-wins aggregation applied to outcome.
/// `evidence_bundle_id` is mandatory.
/// `id` is the content digest of all fields except `id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationSuiteReport {
    pub id: Digest,
    pub run_report_ids: Vec<Digest>,
    pub suite_outcome: EvaluationSuiteOutcome,
    pub passed_count: u64,
    pub warned_count: u64,
    pub failed_count: u64,
    pub skipped_count: u64,
    pub evidence_bundle_id: Digest,
    pub elapsed_ms: u64,
}

#[derive(Serialize)]
struct SuiteReportContent<'a> {
    run_report_ids: &'a Vec<Digest>,
    suite_outcome: &'a EvaluationSuiteOutcome,
    passed_count: u64,
    warned_count: u64,
    failed_count: u64,
    skipped_count: u64,
    evidence_bundle_id: &'a Digest,
    elapsed_ms: u64,
}

impl EvaluationSuiteReport {
    /// Build a suite report from a slice of run reports — applies worst-wins.
    pub fn from_run_reports(
        reports: &[EvaluationRunReport],
        evidence_bundle_id: Digest,
        elapsed_ms: u64,
    ) -> Self {
        let mut passed = 0u64;
        let mut warned = 0u64;
        let mut failed = 0u64;
        let mut skipped = 0u64;

        for r in reports {
            match &r.outcome {
                EvaluationOutcome::Passed => passed += 1,
                EvaluationOutcome::PassedWithWarnings => warned += 1,
                EvaluationOutcome::Failed { .. } | EvaluationOutcome::Inconclusive => failed += 1,
                EvaluationOutcome::SkippedByReportOnly => skipped += 1,
            }
        }

        let total = passed + warned + failed + skipped;
        let suite_outcome = if total == 0 || reports.is_empty() {
            EvaluationSuiteOutcome::Inconclusive
        } else if failed == total {
            EvaluationSuiteOutcome::AllFailed
        } else if failed > 0 {
            EvaluationSuiteOutcome::SomeFailed
        } else if warned > 0 {
            EvaluationSuiteOutcome::SomePassedWithWarnings
        } else if passed == total {
            EvaluationSuiteOutcome::AllPassed
        } else {
            EvaluationSuiteOutcome::Inconclusive
        };

        let run_report_ids: Vec<Digest> = reports.iter().map(|r| r.id).collect();
        let mut s = Self {
            id: Digest::ZERO,
            run_report_ids,
            suite_outcome,
            passed_count: passed,
            warned_count: warned,
            failed_count: failed,
            skipped_count: skipped,
            evidence_bundle_id,
            elapsed_ms,
        };
        s.id = s.compute_id();
        s
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&SuiteReportContent {
            run_report_ids: &self.run_report_ids,
            suite_outcome: &self.suite_outcome,
            passed_count: self.passed_count,
            warned_count: self.warned_count,
            failed_count: self.failed_count,
            skipped_count: self.skipped_count,
            evidence_bundle_id: &self.evidence_bundle_id,
            elapsed_ms: self.elapsed_ms,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }

    pub fn total_count(&self) -> u64 {
        self.passed_count + self.warned_count + self.failed_count + self.skipped_count
    }
}

/// Drives scenario execution and returns a content-addressed `EvaluationRunReport`.
///
/// In `ReportOnly` mode, implementations MUST return
/// `EvaluationRunReport::skipped_by_report_only(...)` without executing anything.
pub trait EvaluationHarness {
    fn run_scenario(
        &self,
        scenario: &EvaluationScenario,
        policy: &PolicyProfile,
        evidence_bundle_id: Digest,
    ) -> EvaluationRunReport;
}

/// Deterministic stub harness for testing.
///
/// Applies the scenario's criteria against pre-supplied metrics to determine
/// the outcome. Returns `SkippedByReportOnly` in `ReportOnly` mode.
pub struct StubEvaluationHarness {
    pub metrics_fn: fn(&EvaluationScenario) -> EvaluationMetrics,
}

impl StubEvaluationHarness {
    pub fn always_pass() -> Self {
        Self {
            metrics_fn: |s| {
                EvaluationMetrics::new(
                    s.id,
                    Q16::ratio(95, 100).unwrap(),
                    Q16::ratio(5, 100).unwrap(),
                    Q16::ratio(90, 100).unwrap(),
                    true,
                )
            },
        }
    }

    pub fn always_fail() -> Self {
        Self {
            metrics_fn: |s| {
                EvaluationMetrics::new(s.id, Q16::ZERO, Q16::ONE, Q16::ZERO, false)
            },
        }
    }
}

impl EvaluationHarness for StubEvaluationHarness {
    fn run_scenario(
        &self,
        scenario: &EvaluationScenario,
        policy: &PolicyProfile,
        evidence_bundle_id: Digest,
    ) -> EvaluationRunReport {
        if policy.mode == ImplementationMode::ReportOnly {
            return EvaluationRunReport::skipped_by_report_only(scenario.id, evidence_bundle_id);
        }

        let metrics = (self.metrics_fn)(scenario);
        let (outcome, diagnostics) = if metrics.satisfies(&scenario.criteria) {
            (EvaluationOutcome::Passed, vec![])
        } else {
            let mut violations = vec![];
            if metrics.confidence_score < scenario.criteria.min_confidence {
                violations.push(format!(
                    "confidence {:.4} < min {:.4}",
                    metrics.confidence_score.to_f64(),
                    scenario.criteria.min_confidence.to_f64(),
                ));
            }
            if metrics.failure_rate > scenario.criteria.max_failure_rate {
                violations.push(format!(
                    "failure_rate {:.4} > max {:.4}",
                    metrics.failure_rate.to_f64(),
                    scenario.criteria.max_failure_rate.to_f64(),
                ));
            }
            if metrics.evidence_coverage < scenario.criteria.min_evidence_coverage {
                violations.push(format!(
                    "evidence_coverage {:.4} < min {:.4}",
                    metrics.evidence_coverage.to_f64(),
                    scenario.criteria.min_evidence_coverage.to_f64(),
                ));
            }
            if scenario.criteria.require_deterministic && !metrics.determinism_verified {
                violations.push("determinism not verified".into());
            }
            (EvaluationOutcome::Failed { criteria_violations: violations.clone() }, violations)
        };

        EvaluationRunReport::new(scenario.id, outcome, metrics, diagnostics, evidence_bundle_id, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{EvidenceBundle, EvidenceKind, EvidenceRef, ReplayStatus};
    use crate::policy::PolicyProfile;

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    fn policy_id() -> Digest { d(b"policy") }
    fn bundle_id() -> Digest { d(b"bundle") }

    fn basic_scenario() -> EvaluationScenario {
        EvaluationScenario::new(
            "foundry-smoke",
            EvaluationDimension::FoundryExecution,
            d(b"input"),
            d(b"expected"),
            EvaluationCriteria::strict(),
            policy_id(),
        )
    }

    fn passing_metrics(scenario_id: Digest) -> EvaluationMetrics {
        EvaluationMetrics::new(
            scenario_id,
            Q16::ratio(95, 100).unwrap(),
            Q16::ratio(5, 100).unwrap(),
            Q16::ratio(90, 100).unwrap(),
            true,
        )
    }

    fn failing_metrics(scenario_id: Digest) -> EvaluationMetrics {
        EvaluationMetrics::new(scenario_id, Q16::ZERO, Q16::ONE, Q16::ZERO, false)
    }

    // --- EvaluationCriteria ---

    #[test]
    fn criteria_strict_no_floats() {
        let c = EvaluationCriteria::strict();
        // All thresholds are Q16 — raw() returns i64.
        assert!(c.min_confidence.raw() > 0);
        assert!(c.max_failure_rate.raw() > 0);
        assert!(c.min_evidence_coverage.raw() > 0);
        assert!(c.require_deterministic);
    }

    #[test]
    fn criteria_content_digest_deterministic() {
        let d1 = EvaluationCriteria::strict().content_digest();
        let d2 = EvaluationCriteria::strict().content_digest();
        assert_eq!(d1, d2);
    }

    #[test]
    fn criteria_strict_differs_from_permissive() {
        let ds = EvaluationCriteria::strict().content_digest();
        let dp = EvaluationCriteria::permissive().content_digest();
        assert_ne!(ds, dp);
    }

    // --- EvaluationScenario ---

    #[test]
    fn scenario_id_deterministic() {
        let s1 = basic_scenario();
        let s2 = basic_scenario();
        assert_eq!(s1.id, s2.id);
    }

    #[test]
    fn scenario_verify_id() {
        assert!(basic_scenario().verify_id());
    }

    #[test]
    fn scenario_with_evidence_changes_id() {
        let s = basic_scenario();
        let id_before = s.id;
        let s2 = s.with_evidence(vec![EvidenceRef::new(
            d(b"ev"), EvidenceKind::RunRecord, "run",
        )]);
        assert_ne!(id_before, s2.id);
        assert!(s2.verify_id());
    }

    #[test]
    fn scenario_different_dimensions_differ() {
        let s1 = EvaluationScenario::new(
            "x", EvaluationDimension::FoundryExecution,
            d(b"in"), d(b"out"), EvaluationCriteria::strict(), policy_id(),
        );
        let s2 = EvaluationScenario::new(
            "x", EvaluationDimension::ParseBackAccuracy,
            d(b"in"), d(b"out"), EvaluationCriteria::strict(), policy_id(),
        );
        assert_ne!(s1.id, s2.id);
    }

    // --- EvaluationMetrics ---

    #[test]
    fn metrics_id_deterministic() {
        let m1 = passing_metrics(d(b"s"));
        let m2 = passing_metrics(d(b"s"));
        assert_eq!(m1.id, m2.id);
    }

    #[test]
    fn metrics_verify_id() {
        assert!(passing_metrics(d(b"s")).verify_id());
    }

    #[test]
    fn metrics_with_additional_changes_id() {
        let m = passing_metrics(d(b"s"));
        let id_before = m.id;
        let m2 = m.with_metric("latency_p99", Q16::ratio(3, 100).unwrap().raw());
        assert_ne!(id_before, m2.id);
        assert!(m2.verify_id());
    }

    #[test]
    fn metrics_satisfies_strict_criteria() {
        let m = passing_metrics(d(b"s"));
        assert!(m.satisfies(&EvaluationCriteria::strict()));
    }

    #[test]
    fn metrics_fails_strict_criteria() {
        let m = failing_metrics(d(b"s"));
        assert!(!m.satisfies(&EvaluationCriteria::strict()));
    }

    #[test]
    fn metrics_no_floats_in_additional() {
        let m = EvaluationMetrics::new(d(b"s"), Q16::ONE, Q16::ZERO, Q16::ONE, true)
            .with_metric("score", Q16::ratio(7, 10).unwrap().raw());
        // additional values are i64 raw Q16, not f64
        let raw: i64 = *m.additional.get("score").unwrap();
        assert_eq!(raw, Q16::ratio(7, 10).unwrap().raw());
    }

    #[test]
    fn metrics_determinism_flag_affects_satisfies() {
        let m = EvaluationMetrics::new(
            d(b"s"), Q16::ratio(95, 100).unwrap(), Q16::ratio(5, 100).unwrap(),
            Q16::ratio(90, 100).unwrap(), false, // determinism not verified
        );
        let c = EvaluationCriteria::strict(); // require_deterministic = true
        assert!(!m.satisfies(&c));
    }

    // --- EvaluationOutcome ---

    #[test]
    fn outcome_passed_variants() {
        assert!(EvaluationOutcome::Passed.is_passed());
        assert!(EvaluationOutcome::PassedWithWarnings.is_passed());
    }

    #[test]
    fn outcome_skipped_not_failure() {
        let o = EvaluationOutcome::SkippedByReportOnly;
        assert!(o.is_skipped_report_only());
        assert!(!o.is_failure_class());
        assert!(!o.is_passed());
    }

    #[test]
    fn outcome_failed_and_inconclusive_are_failure_class() {
        assert!(EvaluationOutcome::Failed { criteria_violations: vec![] }.is_failure_class());
        assert!(EvaluationOutcome::Inconclusive.is_failure_class());
    }

    // --- EvaluationRunReport ---

    #[test]
    fn run_report_skipped_deterministic() {
        let r1 = EvaluationRunReport::skipped_by_report_only(d(b"s"), bundle_id());
        let r2 = EvaluationRunReport::skipped_by_report_only(d(b"s"), bundle_id());
        assert_eq!(r1.id, r2.id);
    }

    #[test]
    fn run_report_skipped_verify_id() {
        let r = EvaluationRunReport::skipped_by_report_only(d(b"s"), bundle_id());
        assert!(r.verify_id());
        assert!(r.outcome.is_skipped_report_only());
        assert_eq!(r.elapsed_ms, 0u64);
    }

    #[test]
    fn run_report_new_deterministic() {
        let s = basic_scenario();
        let m = passing_metrics(s.id);
        let r1 = EvaluationRunReport::new(
            s.id, EvaluationOutcome::Passed, m.clone(), vec![], bundle_id(), 100,
        );
        let r2 = EvaluationRunReport::new(
            s.id, EvaluationOutcome::Passed, m, vec![], bundle_id(), 100,
        );
        assert_eq!(r1.id, r2.id);
    }

    #[test]
    fn run_report_verify_id() {
        let s = basic_scenario();
        let r = EvaluationRunReport::new(
            s.id,
            EvaluationOutcome::Failed { criteria_violations: vec!["confidence".into()] },
            failing_metrics(s.id),
            vec!["low confidence".into()],
            bundle_id(),
            50,
        );
        assert!(r.verify_id());
    }

    #[test]
    fn run_report_evidence_mandatory() {
        let r = EvaluationRunReport::skipped_by_report_only(d(b"s"), bundle_id());
        assert_ne!(r.evidence_bundle_id, Digest::ZERO);
    }

    // --- EvaluationSuiteReport ---

    #[test]
    fn suite_report_empty_is_inconclusive() {
        let s = EvaluationSuiteReport::from_run_reports(&[], bundle_id(), 0);
        assert_eq!(s.suite_outcome, EvaluationSuiteOutcome::Inconclusive);
        assert!(s.verify_id());
    }

    #[test]
    fn suite_report_all_passed() {
        let s = basic_scenario();
        let reports = vec![
            EvaluationRunReport::new(
                s.id, EvaluationOutcome::Passed, passing_metrics(s.id), vec![], bundle_id(), 10,
            ),
            EvaluationRunReport::new(
                s.id, EvaluationOutcome::Passed, passing_metrics(s.id), vec![], bundle_id(), 10,
            ),
        ];
        let suite = EvaluationSuiteReport::from_run_reports(&reports, bundle_id(), 20);
        assert_eq!(suite.suite_outcome, EvaluationSuiteOutcome::AllPassed);
        assert_eq!(suite.passed_count, 2);
        assert_eq!(suite.total_count(), 2);
        assert!(suite.verify_id());
    }

    #[test]
    fn suite_report_some_failed_dominates_passed() {
        let s = basic_scenario();
        let reports = vec![
            EvaluationRunReport::new(
                s.id, EvaluationOutcome::Passed, passing_metrics(s.id), vec![], bundle_id(), 0,
            ),
            EvaluationRunReport::new(
                s.id,
                EvaluationOutcome::Failed { criteria_violations: vec!["x".into()] },
                failing_metrics(s.id),
                vec![],
                bundle_id(),
                0,
            ),
        ];
        let suite = EvaluationSuiteReport::from_run_reports(&reports, bundle_id(), 0);
        assert_eq!(suite.suite_outcome, EvaluationSuiteOutcome::SomeFailed);
        assert!(suite.suite_outcome.is_failure_class());
        assert!(suite.verify_id());
    }

    #[test]
    fn suite_report_all_failed() {
        let s = basic_scenario();
        let reports = vec![
            EvaluationRunReport::new(
                s.id,
                EvaluationOutcome::Failed { criteria_violations: vec![] },
                failing_metrics(s.id), vec![], bundle_id(), 0,
            ),
            EvaluationRunReport::new(
                s.id,
                EvaluationOutcome::Failed { criteria_violations: vec![] },
                failing_metrics(s.id), vec![], bundle_id(), 0,
            ),
        ];
        let suite = EvaluationSuiteReport::from_run_reports(&reports, bundle_id(), 0);
        assert_eq!(suite.suite_outcome, EvaluationSuiteOutcome::AllFailed);
        assert_eq!(suite.failed_count, 2);
        assert!(suite.verify_id());
    }

    #[test]
    fn suite_report_warned_between_passed_and_failed() {
        let s = basic_scenario();
        let reports = vec![
            EvaluationRunReport::new(
                s.id, EvaluationOutcome::Passed, passing_metrics(s.id), vec![], bundle_id(), 0,
            ),
            EvaluationRunReport::new(
                s.id, EvaluationOutcome::PassedWithWarnings, passing_metrics(s.id),
                vec!["near threshold".into()], bundle_id(), 0,
            ),
        ];
        let suite = EvaluationSuiteReport::from_run_reports(&reports, bundle_id(), 0);
        assert_eq!(suite.suite_outcome, EvaluationSuiteOutcome::SomePassedWithWarnings);
        assert!(!suite.suite_outcome.is_failure_class());
        assert!(suite.verify_id());
    }

    #[test]
    fn suite_report_deterministic() {
        let s = basic_scenario();
        let reports = vec![EvaluationRunReport::new(
            s.id, EvaluationOutcome::Passed, passing_metrics(s.id), vec![], bundle_id(), 5,
        )];
        let s1 = EvaluationSuiteReport::from_run_reports(&reports, bundle_id(), 100);
        let s2 = EvaluationSuiteReport::from_run_reports(&reports, bundle_id(), 100);
        assert_eq!(s1.id, s2.id);
    }

    // --- StubEvaluationHarness ---

    #[test]
    fn harness_report_only_skips() {
        let h = StubEvaluationHarness::always_pass();
        let s = basic_scenario();
        let profile = PolicyProfile::default();
        let r = h.run_scenario(&s, &profile, bundle_id());
        assert!(r.outcome.is_skipped_report_only());
        assert!(r.verify_id());
    }

    #[test]
    fn harness_always_pass_produces_passed() {
        let h = StubEvaluationHarness::always_pass();
        let s = basic_scenario();
        let profile = PolicyProfile { mode: ImplementationMode::DryRun, ..PolicyProfile::default() };
        let r = h.run_scenario(&s, &profile, bundle_id());
        assert_eq!(r.outcome, EvaluationOutcome::Passed);
        assert!(r.verify_id());
    }

    #[test]
    fn harness_always_fail_produces_failed() {
        let h = StubEvaluationHarness::always_fail();
        let s = basic_scenario();
        let profile = PolicyProfile { mode: ImplementationMode::DryRun, ..PolicyProfile::default() };
        let r = h.run_scenario(&s, &profile, bundle_id());
        assert!(r.outcome.is_failure_class());
        assert!(r.verify_id());
    }

    #[test]
    fn harness_run_deterministic() {
        let h = StubEvaluationHarness::always_pass();
        let s = basic_scenario();
        let profile = PolicyProfile { mode: ImplementationMode::DryRun, ..PolicyProfile::default() };
        let r1 = h.run_scenario(&s, &profile, bundle_id());
        let r2 = h.run_scenario(&s, &profile, bundle_id());
        assert_eq!(r1.id, r2.id, "INVARIANT-007: identical inputs → identical report IDs");
    }

    #[test]
    fn r9_evidence_bundle_integration() {
        let ev_bundle = EvidenceBundle::seal(
            vec![EvidenceRef::new(
                d(b"eval-ev"), EvidenceKind::RunRecord, "evaluation run evidence",
            )],
            policy_id(),
            ReplayStatus::Replayable,
        );
        let s = basic_scenario();
        let r = EvaluationRunReport::skipped_by_report_only(s.id, ev_bundle.bundle_id);
        assert!(r.verify_id());
        assert_ne!(r.evidence_bundle_id, Digest::ZERO);
    }
}

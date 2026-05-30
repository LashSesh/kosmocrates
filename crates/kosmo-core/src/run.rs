use crate::digest::Digest;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Result of a single gate check in a GateCascade.
///
/// Gate comparisons always use fixed-point (`Q16`) values, never raw floats.
/// A `Reject` or `Downgrade` result must propagate through the cascade
/// and cannot be silently ignored.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateResult {
    Pass,
    Warn { message: String },
    Reject { reason: String },
    Downgrade { from: String, to: String, reason: String },
}

impl GateResult {
    pub fn is_pass(&self) -> bool {
        matches!(self, GateResult::Pass)
    }

    pub fn is_warn(&self) -> bool {
        matches!(self, GateResult::Warn { .. })
    }

    pub fn is_rejected(&self) -> bool {
        matches!(self, GateResult::Reject { .. })
    }

    pub fn is_downgrade(&self) -> bool {
        matches!(self, GateResult::Downgrade { .. })
    }

    /// Returns the most restrictive of two results.
    pub fn merge(self, other: Self) -> Self {
        match (&self, &other) {
            (GateResult::Reject { .. }, _) | (_, GateResult::Reject { .. }) => {
                if self.is_rejected() { self } else { other }
            }
            (GateResult::Downgrade { .. }, _) | (_, GateResult::Downgrade { .. }) => {
                if self.is_downgrade() { self } else { other }
            }
            (GateResult::Warn { .. }, _) | (_, GateResult::Warn { .. }) => {
                if self.is_warn() { self } else { other }
            }
            _ => GateResult::Pass,
        }
    }
}

/// Classifies what kind of event is recorded in the ledger.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LedgerEventKind {
    RunStarted,
    RunCompleted,
    HostScanCompleted,
    VoidMapProduced,
    StructuralYieldEmitted,
    GateCascadeDecision,
    AssimilationDecision,
    PolicyViolation,
    EvidenceBundleSealed,
    CorpusCartographyUpdated,
    ReportGenerated,
    Custom(String),
}

/// An immutable, content-addressed ledger entry.
///
/// `event_id` is the canonical digest of all fields except `event_id` itself.
/// The `sequence` field provides total ordering within a run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerEvent {
    pub event_id: Digest,
    pub kind: LedgerEventKind,
    pub run_id: Option<Digest>,
    pub evidence_bundle_id: Option<Digest>,
    pub gate_result: Option<GateResult>,
    pub payload: BTreeMap<String, String>,
    pub sequence: u64,
}

/// Internal for content-addressing LedgerEvent (excludes event_id).
#[derive(Serialize)]
struct LedgerEventContent<'a> {
    kind: &'a LedgerEventKind,
    run_id: &'a Option<Digest>,
    evidence_bundle_id: &'a Option<Digest>,
    gate_result: &'a Option<GateResult>,
    payload: &'a BTreeMap<String, String>,
    sequence: u64,
}

impl LedgerEvent {
    pub fn new(kind: LedgerEventKind, sequence: u64) -> Self {
        let mut ev = Self {
            event_id: Digest::ZERO,
            kind,
            run_id: None,
            evidence_bundle_id: None,
            gate_result: None,
            payload: BTreeMap::new(),
            sequence,
        };
        ev.event_id = ev.compute_id();
        ev
    }

    pub fn with_run_id(mut self, run_id: Digest) -> Self {
        self.run_id = Some(run_id);
        self.event_id = self.compute_id();
        self
    }

    pub fn with_gate_result(mut self, result: GateResult) -> Self {
        self.gate_result = Some(result);
        self.event_id = self.compute_id();
        self
    }

    pub fn with_payload(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.payload.insert(key.into(), value.into());
        self.event_id = self.compute_id();
        self
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&LedgerEventContent {
            kind: &self.kind,
            run_id: &self.run_id,
            evidence_bundle_id: &self.evidence_bundle_id,
            gate_result: &self.gate_result,
            payload: &self.payload,
            sequence: self.sequence,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.event_id == self.compute_id()
    }
}

/// The type of Foundry check performed.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FoundryCheckKind {
    Build,
    Test,
    Lint,
    TypeCheck,
    Security,
    ParseBack,
    Custom(String),
}

/// The outcome of a Foundry check invocation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FoundryOutcome {
    Passed,
    Failed { exit_code: i32, stderr: String },
    Skipped { reason: String },
    /// Foundry infrastructure is not available in the current environment.
    /// Must be recorded explicitly — failure to check is not the same as passing.
    Unavailable { reason: String },
}

impl FoundryOutcome {
    pub fn is_passed(&self) -> bool {
        matches!(self, FoundryOutcome::Passed)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, FoundryOutcome::Failed { .. })
    }

    pub fn is_unavailable(&self) -> bool {
        matches!(self, FoundryOutcome::Unavailable { .. })
    }
}

/// Internal for content-addressing FoundryCheckResult.
#[derive(Serialize)]
struct CheckContent<'a> {
    check_kind: &'a FoundryCheckKind,
    outcome: &'a FoundryOutcome,
    evidence_id: &'a Digest,
    diagnostics: &'a Vec<String>,
}

/// A content-addressed record of a single Foundry check.
///
/// `check_id` is the canonical digest of `(check_kind, outcome, evidence_id, diagnostics)`.
/// `Unavailable` outcome must still be recorded — it is not the same as `Passed`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryCheckResult {
    pub check_id: Digest,
    pub check_kind: FoundryCheckKind,
    pub outcome: FoundryOutcome,
    pub evidence_id: Digest,
    pub diagnostics: Vec<String>,
}

impl FoundryCheckResult {
    pub fn new(
        check_kind: FoundryCheckKind,
        outcome: FoundryOutcome,
        evidence_id: Digest,
        diagnostics: Vec<String>,
    ) -> Self {
        let mut r = Self {
            check_id: Digest::ZERO,
            check_kind,
            outcome,
            evidence_id,
            diagnostics,
        };
        r.check_id = r.compute_id();
        r
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&CheckContent {
            check_kind: &self.check_kind,
            outcome: &self.outcome,
            evidence_id: &self.evidence_id,
            diagnostics: &self.diagnostics,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.check_id == self.compute_id()
    }
}

/// HYPHAE run descriptor.
///
/// Distinct from `pse_types::RunDescriptor` (which is PSE-engine specific).
/// This type identifies and scopes a single HYPHAE / Workbench execution.
/// `run_id` is the canonical digest of the run's identity fields.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunDescriptor {
    pub run_id: Digest,
    pub policy_id: Digest,
    pub host_path: String,
    pub sequence: u64,
    pub labels: BTreeMap<String, String>,
}

/// Internal for content-addressing RunDescriptor.
#[derive(Serialize)]
struct RunContent<'a> {
    policy_id: &'a Digest,
    host_path: &'a String,
    sequence: u64,
    labels: &'a BTreeMap<String, String>,
}

impl RunDescriptor {
    pub fn new(policy_id: Digest, host_path: impl Into<String>) -> Self {
        let mut r = Self {
            run_id: Digest::ZERO,
            policy_id,
            host_path: host_path.into(),
            sequence: 0,
            labels: BTreeMap::new(),
        };
        r.run_id = r.compute_id();
        r
    }

    pub fn with_sequence(mut self, seq: u64) -> Self {
        self.sequence = seq;
        self.run_id = self.compute_id();
        self
    }

    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self.run_id = self.compute_id();
        self
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&RunContent {
            policy_id: &self.policy_id,
            host_path: &self.host_path,
            sequence: self.sequence,
            labels: &self.labels,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.run_id == self.compute_id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_result_pass_is_pass() {
        assert!(GateResult::Pass.is_pass());
        assert!(!GateResult::Pass.is_rejected());
    }

    #[test]
    fn gate_result_reject_is_rejected() {
        let r = GateResult::Reject { reason: "tainted".into() };
        assert!(r.is_rejected());
        assert!(!r.is_pass());
    }

    #[test]
    fn gate_result_merge_reject_dominates() {
        let reject = GateResult::Reject { reason: "bad".into() };
        let merged = reject.merge(GateResult::Pass);
        assert!(merged.is_rejected());
    }

    #[test]
    fn gate_result_merge_warn_over_pass() {
        let warn = GateResult::Warn { message: "low score".into() };
        let merged = GateResult::Pass.merge(warn);
        assert!(merged.is_warn());
    }

    #[test]
    fn ledger_event_id_deterministic() {
        let e1 = LedgerEvent::new(LedgerEventKind::RunStarted, 0);
        let e2 = LedgerEvent::new(LedgerEventKind::RunStarted, 0);
        assert_eq!(e1.event_id, e2.event_id);
    }

    #[test]
    fn ledger_event_verify_id() {
        let e = LedgerEvent::new(LedgerEventKind::HostScanCompleted, 1)
            .with_payload("path", "/home/test");
        assert!(e.verify_id());
    }

    #[test]
    fn foundry_check_unavailable_is_not_passed() {
        let r = FoundryCheckResult::new(
            FoundryCheckKind::Build,
            FoundryOutcome::Unavailable { reason: "no cargo in env".into() },
            Digest::ZERO,
            vec![],
        );
        assert!(!r.outcome.is_passed());
        assert!(r.outcome.is_unavailable());
        assert!(r.verify_id());
    }

    #[test]
    fn foundry_check_id_deterministic() {
        let r1 = FoundryCheckResult::new(
            FoundryCheckKind::Test,
            FoundryOutcome::Passed,
            Digest::of_bytes(b"evidence"),
            vec!["all good".into()],
        );
        let r2 = FoundryCheckResult::new(
            FoundryCheckKind::Test,
            FoundryOutcome::Passed,
            Digest::of_bytes(b"evidence"),
            vec!["all good".into()],
        );
        assert_eq!(r1.check_id, r2.check_id);
    }

    #[test]
    fn run_descriptor_id_deterministic() {
        let policy = Digest::of_bytes(b"policy");
        let r1 = RunDescriptor::new(policy, "/workspace");
        let r2 = RunDescriptor::new(policy, "/workspace");
        assert_eq!(r1.run_id, r2.run_id);
    }

    #[test]
    fn run_descriptor_verify_id() {
        let r = RunDescriptor::new(Digest::ZERO, "/workspace")
            .with_label("phase", "1");
        assert!(r.verify_id());
    }
}

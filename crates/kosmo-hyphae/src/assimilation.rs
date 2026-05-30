use crate::gates::GateTrace;
use crate::structural_yield::StructuralYield;
use kosmo_core::{Digest, EvidenceBundle, GateResult, LedgerEvent, LedgerEventKind};
use serde::{Deserialize, Serialize};

/// The outcome of the assimilation decision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssimilationOutcome {
    /// Yield passed all gates and is accepted for planning.
    Accepted { reason: String },
    /// A gate rejected the yield.
    RejectedByGate { gate: String, reason: String },
    /// Yield was downgraded (e.g. from Workbench to EvidenceOnly).
    Downgraded { from_kind: String, to_kind: String, reason: String },
    /// Yield is kept as evidence only — not plannable yet.
    EvidenceOnly { reason: String },
    /// Decision deferred pending more evidence or operator review.
    Deferred { reason: String },
}

impl AssimilationOutcome {
    pub fn is_accepted(&self) -> bool {
        matches!(self, AssimilationOutcome::Accepted { .. })
    }

    pub fn is_rejected(&self) -> bool {
        matches!(self, AssimilationOutcome::RejectedByGate { .. })
    }
}

/// Serialize-only for content-addressing.
#[derive(Serialize)]
struct DecisionContent {
    yield_id: Digest,
    gate_trace_id: Digest,
    outcome: String,
    evidence_bundle_id: Digest,
    policy_id: Digest,
}

/// An immutable, content-addressed record of the assimilation decision
/// for a single `StructuralYield`.
///
/// Every decision is evidence-bound (CROSS-006) and records the full gate
/// trace so it can be re-examined or replayed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssimilationDecision {
    pub decision_id: Digest,
    pub yield_id: Digest,
    pub outcome: AssimilationOutcome,
    pub gate_trace_id: Digest,
    pub evidence_bundle_id: Digest,
    pub policy_id: Digest,
}

impl AssimilationDecision {
    pub fn from_trace(
        yield_: &StructuralYield,
        trace: &GateTrace,
        evidence: &EvidenceBundle,
        policy_id: Digest,
    ) -> Self {
        let outcome = match &trace.final_result {
            GateResult::Pass => AssimilationOutcome::Accepted {
                reason: "all gates passed".into(),
            },
            GateResult::Reject { reason } => {
                let gate = trace
                    .checks
                    .iter()
                    .find(|c| c.result.is_rejected())
                    .map(|c| format!("{:?}", c.gate))
                    .unwrap_or_else(|| "unknown gate".into());
                AssimilationOutcome::RejectedByGate {
                    gate,
                    reason: reason.clone(),
                }
            }
            GateResult::Warn { message } => AssimilationOutcome::EvidenceOnly {
                reason: format!("gate warning: {}", message),
            },
            GateResult::Downgrade { from, to, reason } => AssimilationOutcome::Downgraded {
                from_kind: from.clone(),
                to_kind: to.clone(),
                reason: reason.clone(),
            },
        };

        let decision_id = Digest::of(&DecisionContent {
            yield_id: yield_.yield_id,
            gate_trace_id: trace.trace_id,
            outcome: format!("{:?}", outcome),
            evidence_bundle_id: evidence.bundle_id,
            policy_id,
        });

        Self {
            decision_id,
            yield_id: yield_.yield_id,
            outcome,
            gate_trace_id: trace.trace_id,
            evidence_bundle_id: evidence.bundle_id,
            policy_id,
        }
    }

    pub fn to_ledger_event(&self, sequence: u64) -> LedgerEvent {
        LedgerEvent::new(LedgerEventKind::AssimilationDecision, sequence)
            .with_gate_result(match &self.outcome {
                AssimilationOutcome::Accepted { .. } => GateResult::Pass,
                AssimilationOutcome::RejectedByGate { reason, .. } => {
                    GateResult::Reject { reason: reason.clone() }
                }
                AssimilationOutcome::EvidenceOnly { reason } => {
                    GateResult::Warn { message: reason.clone() }
                }
                AssimilationOutcome::Downgraded { from_kind, to_kind, reason } => {
                    GateResult::Downgrade {
                        from: from_kind.clone(),
                        to: to_kind.clone(),
                        reason: reason.clone(),
                    }
                }
                AssimilationOutcome::Deferred { .. } => GateResult::Pass,
            })
    }
}

/// A record of negative evidence — a yield that was rejected.
///
/// Stored so future runs can avoid re-proposing the same failed yield
/// (CROSS-012: negative evidence persisted and affects future ranking).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NegativeEvidenceRecord {
    pub record_id: Digest,
    pub yield_id: Digest,
    pub gate_trace_id: Digest,
    pub reason: String,
    pub policy_id: Digest,
}

impl NegativeEvidenceRecord {
    pub fn from_decision(decision: &AssimilationDecision) -> Option<Self> {
        match &decision.outcome {
            AssimilationOutcome::RejectedByGate { reason, .. } => {
                let record_id = Digest::of_bytes(
                    &[
                        decision.yield_id.as_bytes(),
                        decision.gate_trace_id.as_bytes(),
                        reason.as_bytes(),
                    ]
                    .concat(),
                );
                Some(Self {
                    record_id,
                    yield_id: decision.yield_id,
                    gate_trace_id: decision.gate_trace_id,
                    reason: reason.clone(),
                    policy_id: decision.policy_id,
                })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gates::GateCascade;
    use crate::structural_yield::{StructuralYield, StructuralYieldKind};
    use kosmo_core::{
        AuthorityLabel, Digest, EvidenceBundle, EvidenceKind, EvidenceRef,
        PolicyProfile, ReplayStatus, TaintLabel,
    };

    fn make_evidence(policy_id: Digest) -> EvidenceBundle {
        EvidenceBundle::seal(
            vec![EvidenceRef::new(Digest::of_bytes(b"ev"), EvidenceKind::HostScan, "scan")],
            policy_id,
            ReplayStatus::Replayable,
        )
    }

    #[test]
    fn assimilation_accepted_for_clean_yield() {
        let policy = PolicyProfile::default_report_only();
        let void_id = Digest::of_bytes(b"void");
        let ev = make_evidence(policy.id);
        let yield_ = StructuralYield::new(
            StructuralYieldKind::DeficiencyFill,
            Some(void_id),
            None,
            TaintLabel::Clean,
            AuthorityLabel::Foundry,
            ev.bundle_id,
            policy.id,
        );
        let cascade = GateCascade::standard_gates(policy.clone());
        let trace = cascade.apply(&yield_, &ev);
        let decision = AssimilationDecision::from_trace(&yield_, &trace, &ev, policy.id);
        assert!(decision.outcome.is_accepted());
        assert_ne!(decision.decision_id, Digest::ZERO);
    }

    #[test]
    fn assimilation_rejected_for_quarantined() {
        let policy = PolicyProfile::default_report_only();
        let void_id = Digest::of_bytes(b"void");
        let ev = make_evidence(policy.id);
        let yield_ = StructuralYield::new(
            StructuralYieldKind::DeficiencyFill,
            Some(void_id),
            None,
            TaintLabel::Quarantined { reason: "malicious".into() },
            AuthorityLabel::Unknown,
            ev.bundle_id,
            policy.id,
        );
        let cascade = GateCascade::standard_gates(policy.clone());
        let trace = cascade.apply(&yield_, &ev);
        let decision = AssimilationDecision::from_trace(&yield_, &trace, &ev, policy.id);
        assert!(decision.outcome.is_rejected());
    }

    #[test]
    fn cross_012_negative_evidence_representable() {
        // CROSS-012: Negative evidence is persisted and can affect future ranking.
        let policy = PolicyProfile::default_report_only();
        let void_id = Digest::of_bytes(b"void");
        let ev = make_evidence(policy.id);
        let yield_ = StructuralYield::new(
            StructuralYieldKind::DeficiencyFill,
            Some(void_id),
            None,
            TaintLabel::Quarantined { reason: "known-bad".into() },
            AuthorityLabel::Unknown,
            ev.bundle_id,
            policy.id,
        );
        let cascade = GateCascade::standard_gates(policy.clone());
        let trace = cascade.apply(&yield_, &ev);
        let decision = AssimilationDecision::from_trace(&yield_, &trace, &ev, policy.id);
        let neg = NegativeEvidenceRecord::from_decision(&decision)
            .expect("rejected yield must produce NegativeEvidenceRecord");
        assert_eq!(neg.yield_id, yield_.yield_id);
        assert_ne!(neg.record_id, Digest::ZERO);
    }

    #[test]
    fn assimilation_decision_is_content_addressed() {
        let policy = PolicyProfile::default_report_only();
        let void_id = Digest::of_bytes(b"v");
        let ev = make_evidence(policy.id);
        let yield_ = StructuralYield::new(
            StructuralYieldKind::DeficiencyFill,
            Some(void_id), None,
            TaintLabel::Clean, AuthorityLabel::Foundry,
            ev.bundle_id, policy.id,
        );
        let cascade = GateCascade::standard_gates(policy.clone());
        let trace = cascade.apply(&yield_, &ev);
        let d1 = AssimilationDecision::from_trace(&yield_, &trace, &ev, policy.id);
        let d2 = AssimilationDecision::from_trace(&yield_, &trace, &ev, policy.id);
        assert_eq!(d1.decision_id, d2.decision_id);
    }
}

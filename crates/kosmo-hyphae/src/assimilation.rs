use crate::gates::GateTrace;
use crate::structural_yield::StructuralYield;
use kosmo_core::{Digest, EvidenceBundle, GateResult, LedgerEvent, LedgerEventKind, TaintLabel};
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
    taint: String,
    evidence_bundle_id: Digest,
    policy_id: Digest,
}

/// An immutable, content-addressed record of the assimilation decision
/// for a single `StructuralYield`.
///
/// Every decision is evidence-bound (CROSS-006) and records the full gate
/// trace so it can be re-examined or replayed.
/// `taint` is propagated from the source `StructuralYield` so consumers
/// (e.g. `BlueprintUnit` assembly) can reflect the actual trust level.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssimilationDecision {
    pub decision_id: Digest,
    pub yield_id: Digest,
    pub outcome: AssimilationOutcome,
    pub gate_trace_id: Digest,
    /// Taint propagated from the originating `StructuralYield`.
    pub taint: TaintLabel,
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

        let taint = yield_.taint.clone();
        let decision_id = Digest::of(&DecisionContent {
            yield_id: yield_.yield_id,
            gate_trace_id: trace.trace_id,
            outcome: format!("{:?}", outcome),
            taint: format!("{:?}", taint),
            evidence_bundle_id: evidence.bundle_id,
            policy_id,
        });

        Self {
            decision_id,
            yield_id: yield_.yield_id,
            outcome,
            gate_trace_id: trace.trace_id,
            taint,
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

/// Serialize-only for `AssimilationLedger` content-addressing.
#[derive(Serialize)]
struct LedgerContent {
    run_id: Digest,
    event_count: u32,
    event_ids: Vec<Digest>,
    policy_id: Digest,
}

/// A sequenced, content-addressed log of all assimilation decisions in a run.
///
/// One `LedgerEvent` per decision, numbered 0..N-1. The `ledger_id` is the
/// content hash of all event IDs in order — changing any decision changes the
/// entire ledger ID (INVARIANT-007).
///
/// Used as an audit trail: every gate outcome is traceable to its sequence
/// position and the run that produced it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssimilationLedger {
    pub ledger_id: Digest,
    pub run_id: Digest,
    pub events: Vec<LedgerEvent>,
    pub policy_id: Digest,
}

impl AssimilationLedger {
    /// Build a ledger from an ordered slice of decisions.
    ///
    /// Sequence numbers are assigned 0..decisions.len()-1. The ledger_id
    /// is computed over all resulting event IDs.
    pub fn from_decisions(decisions: &[AssimilationDecision], run_id: Digest, policy_id: Digest) -> Self {
        let events: Vec<LedgerEvent> = decisions
            .iter()
            .enumerate()
            .map(|(seq, d)| {
                d.to_ledger_event(seq as u64).with_run_id(run_id)
            })
            .collect();
        let event_ids: Vec<Digest> = events.iter().map(|e| e.event_id).collect();
        let ledger_id = Digest::of(&LedgerContent {
            run_id,
            event_count: events.len() as u32,
            event_ids,
            policy_id,
        });
        Self { ledger_id, run_id, events, policy_id }
    }

    /// Total number of decisions recorded.
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Number of events with a `Pass` gate result.
    pub fn accepted_count(&self) -> usize {
        self.events.iter().filter(|e| e.gate_result.as_ref().is_some_and(|r| r.is_pass())).count()
    }

    /// Number of events with a `Reject` gate result.
    pub fn rejected_count(&self) -> usize {
        self.events.iter().filter(|e| e.gate_result.as_ref().is_some_and(|r| r.is_rejected())).count()
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

    #[test]
    fn decision_propagates_taint_from_yield() {
        let policy = PolicyProfile::default_report_only();
        let ev = make_evidence(policy.id);

        // Clean yield → decision.taint == Clean
        let clean_yield = StructuralYield::new(
            StructuralYieldKind::DeficiencyFill,
            Some(Digest::of_bytes(b"v")), None,
            TaintLabel::Clean, AuthorityLabel::Foundry,
            ev.bundle_id, policy.id,
        );
        let cascade = GateCascade::standard_gates(policy.clone());
        let trace = cascade.apply(&clean_yield, &ev);
        let d = AssimilationDecision::from_trace(&clean_yield, &trace, &ev, policy.id);
        assert_eq!(d.taint, TaintLabel::Clean, "decision must carry yield taint");

        // Synthetic yield → decision.taint == Synthetic
        let synthetic_yield = StructuralYield::new(
            StructuralYieldKind::DeficiencyFill,
            Some(Digest::of_bytes(b"v2")), None,
            TaintLabel::Synthetic, AuthorityLabel::Agent { name: "hyphae".into() },
            ev.bundle_id, policy.id,
        );
        let trace2 = cascade.apply(&synthetic_yield, &ev);
        let d2 = AssimilationDecision::from_trace(&synthetic_yield, &trace2, &ev, policy.id);
        assert_eq!(d2.taint, TaintLabel::Synthetic, "synthetic yield must propagate Synthetic taint");

        // Different taints → different decision_ids (taint is in content hash)
        assert_ne!(d.decision_id, d2.decision_id, "taint must participate in content-address");
    }

    fn make_decision(policy: &PolicyProfile, ev: &EvidenceBundle, seed: &[u8], clean: bool) -> AssimilationDecision {
        let taint = if clean {
            TaintLabel::Clean
        } else {
            TaintLabel::Quarantined { reason: "bad".into() }
        };
        let authority = if clean { AuthorityLabel::Foundry } else { AuthorityLabel::Unknown };
        let yield_ = StructuralYield::new(
            StructuralYieldKind::DeficiencyFill,
            Some(Digest::of_bytes(seed)), None,
            taint, authority, ev.bundle_id, policy.id,
        );
        let cascade = GateCascade::standard_gates(policy.clone());
        let trace = cascade.apply(&yield_, ev);
        AssimilationDecision::from_trace(&yield_, &trace, ev, policy.id)
    }

    #[test]
    fn ledger_empty_decisions_has_non_zero_id() {
        let policy = PolicyProfile::default_report_only();
        let run_id = Digest::of_bytes(b"run");
        let ledger = AssimilationLedger::from_decisions(&[], run_id, policy.id);
        assert_ne!(ledger.ledger_id, Digest::ZERO);
        assert_eq!(ledger.event_count(), 0);
        assert_eq!(ledger.accepted_count(), 0);
        assert_eq!(ledger.rejected_count(), 0);
    }

    #[test]
    fn ledger_event_count_matches_decisions() {
        let policy = PolicyProfile::default_report_only();
        let ev = make_evidence(policy.id);
        let run_id = Digest::of_bytes(b"run");
        let d1 = make_decision(&policy, &ev, b"a", true);
        let d2 = make_decision(&policy, &ev, b"b", false);
        let d3 = make_decision(&policy, &ev, b"c", true);
        let ledger = AssimilationLedger::from_decisions(&[d1, d2, d3], run_id, policy.id);
        assert_eq!(ledger.event_count(), 3);
        assert_eq!(ledger.accepted_count(), 2);
        assert_eq!(ledger.rejected_count(), 1);
    }

    #[test]
    fn ledger_different_outcomes_produce_different_ids() {
        let policy = PolicyProfile::default_report_only();
        let ev = make_evidence(policy.id);
        let run_id = Digest::of_bytes(b"run");
        let d_accept = make_decision(&policy, &ev, b"x", true);
        let d_reject = make_decision(&policy, &ev, b"y", false);
        let ledger_accept = AssimilationLedger::from_decisions(&[d_accept], run_id, policy.id);
        let ledger_reject = AssimilationLedger::from_decisions(&[d_reject], run_id, policy.id);
        assert_ne!(ledger_accept.ledger_id, ledger_reject.ledger_id,
            "different decisions must produce different ledger_ids");
    }

    #[test]
    fn ledger_is_content_addressed() {
        let policy = PolicyProfile::default_report_only();
        let ev = make_evidence(policy.id);
        let run_id = Digest::of_bytes(b"run");
        let d = make_decision(&policy, &ev, b"z", true);
        let l1 = AssimilationLedger::from_decisions(std::slice::from_ref(&d), run_id, policy.id);
        let l2 = AssimilationLedger::from_decisions(&[d], run_id, policy.id);
        assert_eq!(l1.ledger_id, l2.ledger_id, "same decisions → same ledger_id (INVARIANT-007)");
    }
}

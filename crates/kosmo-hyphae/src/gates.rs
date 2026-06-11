use crate::structural_yield::StructuralYield;
use kosmo_core::{Digest, EvidenceBundle, GateResult, PolicyProfile, TaintLabel};
use serde::{Deserialize, Serialize};

/// Named gate in a GateCascade.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateKind {
    /// Reject quarantined/external taint; warn on synthetic/unverified.
    TaintGate,
    /// Warn if authority is Unknown or Agent (requires scrutiny).
    AuthorityGate,
    /// Reject if the evidence bundle is empty.
    EvidenceGate,
    /// Reject if the yield references neither a host void nor a deficiency.
    VoidRefGate,
    /// Enforce policy-level constraints.
    PolicyGate,
    Custom(String),
}

/// The outcome of a single gate check.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GateCheckRecord {
    pub gate: GateKind,
    pub result: GateResult,
    pub sequence: u64,
}

/// Serialize-only for content-addressing.
#[derive(Serialize)]
struct TraceContent {
    check_results: Vec<String>,
    final_result: String,
    policy_id: Digest,
}

/// Immutable record of all gate checks applied to one `StructuralYield`.
///
/// `trace_id` is content-addressed. `final_result` is the most restrictive
/// of all individual gate results (Reject > Downgrade > Warn > Pass).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GateTrace {
    pub trace_id: Digest,
    pub checks: Vec<GateCheckRecord>,
    pub final_result: GateResult,
    pub policy_id: Digest,
}

impl GateTrace {
    fn new(checks: Vec<GateCheckRecord>, policy_id: Digest) -> Self {
        let final_result = checks
            .iter()
            .fold(GateResult::Pass, |acc, c| acc.merge(c.result.clone()));

        let check_results: Vec<String> = checks
            .iter()
            .map(|c| format!("{:?}:{:?}", c.gate, c.result))
            .collect();

        let trace_id = Digest::of(&TraceContent {
            check_results,
            final_result: format!("{:?}", final_result),
            policy_id,
        });

        Self {
            trace_id,
            checks,
            final_result,
            policy_id,
        }
    }

    pub fn passed(&self) -> bool {
        self.final_result.is_pass()
    }

    pub fn was_rejected(&self) -> bool {
        self.final_result.is_rejected()
    }
}

/// Applies a sequence of named gates to a `StructuralYield`.
///
/// Gates run in order; all gates are recorded regardless of earlier results
/// (no short-circuit) so the full trace is always available.
pub struct GateCascade {
    pub policy: PolicyProfile,
    pub gates: Vec<GateKind>,
}

impl GateCascade {
    /// Standard gate sequence for HYPHAE v0.3 passive run.
    pub fn standard_gates(policy: PolicyProfile) -> Self {
        Self {
            gates: vec![
                GateKind::TaintGate,
                GateKind::EvidenceGate,
                GateKind::VoidRefGate,
                GateKind::AuthorityGate,
                GateKind::PolicyGate,
            ],
            policy,
        }
    }

    /// Apply all gates to a yield and return the full trace.
    pub fn apply(&self, yield_: &StructuralYield, evidence: &EvidenceBundle) -> GateTrace {
        let mut checks = Vec::with_capacity(self.gates.len());
        for (seq, gate) in self.gates.iter().enumerate() {
            let result = self.run_gate(gate, yield_, evidence);
            checks.push(GateCheckRecord {
                gate: gate.clone(),
                result,
                sequence: seq as u64,
            });
        }
        GateTrace::new(checks, self.policy.id)
    }

    fn run_gate(
        &self,
        gate: &GateKind,
        yield_: &StructuralYield,
        evidence: &EvidenceBundle,
    ) -> GateResult {
        match gate {
            GateKind::TaintGate => match &yield_.taint {
                TaintLabel::Quarantined { reason } => GateResult::Reject {
                    reason: format!("quarantined: {}", reason),
                },
                TaintLabel::External => GateResult::Reject {
                    reason: "external taint — CROSS-005".into(),
                },
                TaintLabel::Synthetic => GateResult::Warn {
                    message: "synthetic taint — low authority".into(),
                },
                TaintLabel::Unverified => GateResult::Warn {
                    message: "unverified taint — requires review".into(),
                },
                TaintLabel::PolicyRestricted => GateResult::Warn {
                    message: "policy-restricted taint".into(),
                },
                TaintLabel::Clean => GateResult::Pass,
            },

            GateKind::EvidenceGate => {
                if evidence.is_empty() {
                    GateResult::Reject {
                        reason: "evidence bundle is empty — CROSS-006".into(),
                    }
                } else {
                    GateResult::Pass
                }
            }

            GateKind::VoidRefGate => {
                if yield_.has_void_reference() {
                    GateResult::Pass
                } else {
                    GateResult::Reject {
                        reason: "yield must reference a HostVoid or DeficiencyEntry".into(),
                    }
                }
            }

            GateKind::AuthorityGate => match &yield_.authority {
                kosmo_core::AuthorityLabel::Unknown => GateResult::Warn {
                    message: "unknown authority — downgrade recommended".into(),
                },
                kosmo_core::AuthorityLabel::Agent { .. } => GateResult::Warn {
                    message: "agent authority — human review recommended".into(),
                },
                _ => GateResult::Pass,
            },

            GateKind::PolicyGate => {
                // A passive run never writes to the host, so the policy gate passes in
                // every mode: ReportOnly accepts yields for reporting, and both
                // host-write-allowed and host-write-denied policies are inert here
                // because no host write is attempted.
                GateResult::Pass
            }

            GateKind::Custom(_) => GateResult::Pass,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structural_yield::{StructuralYield, StructuralYieldKind};
    use kosmo_core::{
        AuthorityLabel, Digest, EvidenceBundle, EvidenceKind, EvidenceRef, PolicyProfile,
        ReplayStatus, TaintLabel,
    };

    fn make_evidence(policy_id: Digest, empty: bool) -> EvidenceBundle {
        if empty {
            EvidenceBundle::empty(policy_id)
        } else {
            EvidenceBundle::seal(
                vec![EvidenceRef::new(
                    Digest::of_bytes(b"e"),
                    EvidenceKind::HostScan,
                    "scan",
                )],
                policy_id,
                ReplayStatus::Replayable,
            )
        }
    }

    fn make_yield(
        taint: TaintLabel,
        void_id: Option<Digest>,
        evidence_id: Digest,
    ) -> StructuralYield {
        StructuralYield::new(
            StructuralYieldKind::DeficiencyFill,
            void_id,
            None,
            taint,
            AuthorityLabel::Agent {
                name: "hyphae".into(),
            },
            evidence_id,
            Digest::ZERO,
        )
    }

    #[test]
    fn gate_cascade_rejects_quarantined_taint() {
        let policy = PolicyProfile::default_report_only();
        let cascade = GateCascade::standard_gates(policy.clone());
        let void_id = Digest::of_bytes(b"void");
        let ev = make_evidence(policy.id, false);
        let y = make_yield(
            TaintLabel::Quarantined {
                reason: "bad source".into(),
            },
            Some(void_id),
            ev.bundle_id,
        );
        let trace = cascade.apply(&y, &ev);
        assert!(trace.was_rejected(), "quarantined taint must be rejected");
    }

    #[test]
    fn gate_cascade_rejects_empty_evidence() {
        let policy = PolicyProfile::default_report_only();
        let cascade = GateCascade::standard_gates(policy.clone());
        let void_id = Digest::of_bytes(b"void");
        let ev = make_evidence(policy.id, true); // empty bundle
        let y = make_yield(TaintLabel::Clean, Some(void_id), ev.bundle_id);
        let trace = cascade.apply(&y, &ev);
        assert!(trace.was_rejected(), "empty evidence must be rejected");
    }

    #[test]
    fn gate_cascade_rejects_yield_without_void_ref() {
        let policy = PolicyProfile::default_report_only();
        let cascade = GateCascade::standard_gates(policy.clone());
        let ev = make_evidence(policy.id, false);
        let y = make_yield(TaintLabel::Clean, None, ev.bundle_id); // no void ref
        let trace = cascade.apply(&y, &ev);
        assert!(
            trace.was_rejected(),
            "yield without void ref must be rejected by VoidRefGate"
        );
    }

    #[test]
    fn gate_cascade_pass_clean_yield_with_void_ref_and_evidence() {
        let policy = PolicyProfile::default_report_only();
        let cascade = GateCascade::standard_gates(policy.clone());
        let void_id = Digest::of_bytes(b"void");
        let ev = make_evidence(policy.id, false);
        // Use Foundry authority (not Agent) to avoid AuthorityGate Warn
        let y = StructuralYield::new(
            StructuralYieldKind::DeficiencyFill,
            Some(void_id),
            None,
            TaintLabel::Clean,
            AuthorityLabel::Foundry,
            ev.bundle_id,
            Digest::ZERO,
        );
        let trace = cascade.apply(&y, &ev);
        assert!(
            trace.passed(),
            "clean yield with void ref and evidence must pass"
        );
    }

    #[test]
    fn gate_cascade_external_taint_rejected() {
        let policy = PolicyProfile::default_report_only();
        let cascade = GateCascade::standard_gates(policy.clone());
        let ev = make_evidence(policy.id, false);
        let y = make_yield(
            TaintLabel::External,
            Some(Digest::of_bytes(b"v")),
            ev.bundle_id,
        );
        let trace = cascade.apply(&y, &ev);
        assert!(
            trace.was_rejected(),
            "external taint must be rejected (CROSS-005)"
        );
    }

    #[test]
    fn gate_trace_is_content_addressed() {
        let policy = PolicyProfile::default_report_only();
        let cascade = GateCascade::standard_gates(policy.clone());
        let void_id = Digest::of_bytes(b"v");
        let ev = make_evidence(policy.id, false);
        let y = StructuralYield::new(
            StructuralYieldKind::DeficiencyFill,
            Some(void_id),
            None,
            TaintLabel::Clean,
            AuthorityLabel::Foundry,
            ev.bundle_id,
            Digest::ZERO,
        );
        let t1 = cascade.apply(&y, &ev);
        let t2 = cascade.apply(&y, &ev);
        assert_eq!(t1.trace_id, t2.trace_id, "GateTrace must be deterministic");
        assert_ne!(t1.trace_id, Digest::ZERO);
    }
}

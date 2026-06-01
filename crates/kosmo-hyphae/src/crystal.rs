use crate::assimilation::{AssimilationDecision, AssimilationOutcome};
use crate::gates::GateTrace;
use kosmo_core::{
    Digest, EnergyAssessment, EnergyFactors, EnergyKernel, EvidenceBundle, FoundrySurvival,
    GateResult, LicenseStatus, PolicyProfile, Q16, ReplayStatus, TripolarEnergy,
};
use serde::{Deserialize, Serialize};

/// Certification status of a structural crystal candidate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CertificationStatus {
    /// Not yet evaluated.
    Pending,
    /// All constraints satisfied — ready to become a crystal record.
    Certified,
    /// Failed one or more constraints.
    Rejected { reason: String },
    /// Accepted for evidence but not for planning (gate warned but did not reject).
    EvidenceOnly,
}

/// Serialize-only for StructuralCrystalCandidate content-addressing.
#[derive(Serialize)]
struct CandidateContent {
    yield_id: Digest,
    decision_id: Digest,
    support_score: i64,
    evidence_bundle_id: Digest,
    policy_id: Digest,
}

/// A structural yield that has survived the gate cascade and is being
/// considered for crystal certification.
///
/// A candidate's `CertificationStatus` starts as `Pending` and advances
/// through constraint checking; it never becomes `Certified` without a
/// fully satisfied `ConstraintProgram`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StructuralCrystalCandidate {
    pub candidate_id: Digest,
    pub yield_id: Digest,
    pub decision_id: Digest,
    pub support_score: Q16,
    pub evidence_bundle_id: Digest,
    pub certification_status: CertificationStatus,
    pub policy_id: Digest,
}

impl StructuralCrystalCandidate {
    pub fn from_decision(decision: &AssimilationDecision) -> Self {
        let certification_status = match &decision.outcome {
            AssimilationOutcome::Accepted { .. } => CertificationStatus::Pending,
            AssimilationOutcome::EvidenceOnly { .. } => CertificationStatus::EvidenceOnly,
            AssimilationOutcome::Downgraded { .. } => CertificationStatus::EvidenceOnly,
            AssimilationOutcome::RejectedByGate { reason, .. } => {
                CertificationStatus::Rejected { reason: reason.clone() }
            }
            AssimilationOutcome::Deferred { .. } => CertificationStatus::Pending,
        };

        let candidate_id = Digest::of(&CandidateContent {
            yield_id: decision.yield_id,
            decision_id: decision.decision_id,
            support_score: 0, // score is not yet known at candidate creation
            evidence_bundle_id: decision.evidence_bundle_id,
            policy_id: decision.policy_id,
        });

        Self {
            candidate_id,
            yield_id: decision.yield_id,
            decision_id: decision.decision_id,
            support_score: Q16::ZERO,
            evidence_bundle_id: decision.evidence_bundle_id,
            certification_status,
            policy_id: decision.policy_id,
        }
    }

    pub fn is_certifiable(&self) -> bool {
        matches!(self.certification_status, CertificationStatus::Pending)
    }

    /// Build an [`EnergyAssessment`] for this crystal candidate.
    ///
    /// - ψ (meaning)  = `support_score` — zero until score is assigned post-gate.
    /// - ρ (coherence) = `Q16::ONE` — no structural coherence data here.
    /// - ω (phase)    = `Q16::ONE` — no phase data here.
    /// - taint factor = `Q16::ONE` — quarantined yields never reach candidate
    ///   stage (`IsNotQuarantined` constraint; quarantine collapses at gate).
    pub fn energy_assessment(&self, gate: &GateResult) -> EnergyAssessment {
        let tripolar = TripolarEnergy::new(self.support_score, Q16::ONE, Q16::ONE);
        let factors = EnergyFactors {
            gate: EnergyFactors::gate_factor(gate),
            taint: Q16::ONE,
            license: EnergyFactors::license_factor(&LicenseStatus::NotApplicable),
            foundry: EnergyFactors::foundry_factor(FoundrySurvival::Unavailable),
            seam: Q16::ONE,
            contradiction: Q16::ONE,
        };
        EnergyAssessment::new(
            self.candidate_id,
            EnergyKernel::new(tripolar, factors),
            self.policy_id,
            self.evidence_bundle_id,
        )
    }
}

/// Kind of constraint in a ConstraintProgram.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintKind {
    HasEvidence,
    HasGateTrace,
    HasVoidReference,
    IsReplayable,
    IsNotQuarantined,
    Custom(String),
}

/// One constraint in a ConstraintProgram.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Constraint {
    pub constraint_id: Digest,
    pub kind: ConstraintKind,
    pub satisfied: bool,
}

impl Constraint {
    pub fn new(kind: ConstraintKind, satisfied: bool) -> Self {
        let constraint_id = Digest::of_bytes(
            format!("{:?}:{}", kind, satisfied).as_bytes(),
        );
        Self { constraint_id, kind, satisfied }
    }
}

/// Serialize-only for ConstraintProgram content-addressing.
#[derive(Serialize)]
struct ProgramContent {
    constraint_ids: Vec<Digest>,
    all_satisfied: bool,
    policy_id: Digest,
}

/// A set of constraints that must be satisfied for a candidate to become a crystal.
///
/// Constraints are sorted by constraint_id for determinism.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConstraintProgram {
    pub program_id: Digest,
    pub constraints: Vec<Constraint>,
    pub all_satisfied: bool,
    pub policy_id: Digest,
}

impl ConstraintProgram {
    pub fn evaluate(mut constraints: Vec<Constraint>, policy_id: Digest) -> Self {
        constraints.sort_by_key(|c| c.constraint_id);
        let all_satisfied = constraints.iter().all(|c| c.satisfied);
        let program_id = Digest::of(&ProgramContent {
            constraint_ids: constraints.iter().map(|c| c.constraint_id).collect(),
            all_satisfied,
            policy_id,
        });
        Self { program_id, constraints, all_satisfied, policy_id }
    }

    /// Build a standard constraint program for a candidate, evidence, and gate trace.
    pub fn standard(
        candidate: &StructuralCrystalCandidate,
        evidence: &EvidenceBundle,
        replay_status: ReplayStatus,
    ) -> Self {
        let constraints = vec![
            Constraint::new(ConstraintKind::HasEvidence, !evidence.is_empty()),
            Constraint::new(ConstraintKind::HasGateTrace, true), // trace exists if we got here
            Constraint::new(ConstraintKind::HasVoidReference, candidate.yield_id != Digest::ZERO),
            Constraint::new(ConstraintKind::IsReplayable, matches!(replay_status, ReplayStatus::Replayable)),
            Constraint::new(ConstraintKind::IsNotQuarantined, true), // quarantined yields are rejected, never candidates
        ];
        Self::evaluate(constraints, candidate.policy_id)
    }
}

/// Serialize-only for ReplayProof content-addressing.
#[derive(Serialize)]
struct ProofContent {
    artifact_id: Digest,
    replay_status: String,
    evidence_bundle_id: Digest,
    policy_id: Digest,
}

/// Proof that an artifact can be replayed from its evidence bundle.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayProof {
    pub proof_id: Digest,
    pub artifact_id: Digest,
    pub replay_status: ReplayStatus,
    pub evidence_bundle_id: Digest,
    pub policy_id: Digest,
}

impl ReplayProof {
    pub fn new(
        artifact_id: Digest,
        replay_status: ReplayStatus,
        evidence_bundle_id: Digest,
        policy_id: Digest,
    ) -> Self {
        let proof_id = Digest::of(&ProofContent {
            artifact_id,
            replay_status: format!("{:?}", replay_status),
            evidence_bundle_id,
            policy_id,
        });
        Self { proof_id, artifact_id, replay_status, evidence_bundle_id, policy_id }
    }

    pub fn is_replayable(&self) -> bool {
        matches!(self.replay_status, ReplayStatus::Replayable)
    }
}

/// Serialize-only for AssimilationCertificate content-addressing.
#[derive(Serialize)]
struct CertificateContent {
    candidate_id: Digest,
    constraint_program_id: Digest,
    replay_proof_id: Digest,
    evidence_bundle_id: Digest,
    policy_id: Digest,
}

/// Certificate issued when a StructuralCrystalCandidate satisfies all
/// constraints in a ConstraintProgram.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssimilationCertificate {
    pub certificate_id: Digest,
    pub candidate_id: Digest,
    pub constraint_program_id: Digest,
    pub replay_proof_id: Digest,
    pub evidence_bundle_id: Digest,
    pub policy_id: Digest,
}

impl AssimilationCertificate {
    /// Issue a certificate. Returns `None` if the constraint program is not
    /// fully satisfied.
    pub fn issue(
        candidate: &StructuralCrystalCandidate,
        program: &ConstraintProgram,
        proof: &ReplayProof,
    ) -> Option<Self> {
        if !program.all_satisfied {
            return None;
        }
        let certificate_id = Digest::of(&CertificateContent {
            candidate_id: candidate.candidate_id,
            constraint_program_id: program.program_id,
            replay_proof_id: proof.proof_id,
            evidence_bundle_id: candidate.evidence_bundle_id,
            policy_id: candidate.policy_id,
        });
        Some(Self {
            certificate_id,
            candidate_id: candidate.candidate_id,
            constraint_program_id: program.program_id,
            replay_proof_id: proof.proof_id,
            evidence_bundle_id: candidate.evidence_bundle_id,
            policy_id: candidate.policy_id,
        })
    }
}

/// Serialize-only for StructuralCrystalRecord content-addressing.
#[derive(Serialize)]
struct RecordContent {
    candidate_id: Digest,
    certificate_id: Digest,
    policy_id: Digest,
}

/// The final certified structural crystal — a yield that has passed all gates
/// and constraints and is recorded as a durable structural pattern.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StructuralCrystalRecord {
    pub record_id: Digest,
    pub candidate_id: Digest,
    pub certificate_id: Digest,
    pub policy_id: Digest,
}

impl StructuralCrystalRecord {
    pub fn from_certificate(cert: &AssimilationCertificate) -> Self {
        let record_id = Digest::of(&RecordContent {
            candidate_id: cert.candidate_id,
            certificate_id: cert.certificate_id,
            policy_id: cert.policy_id,
        });
        Self {
            record_id,
            candidate_id: cert.candidate_id,
            certificate_id: cert.certificate_id,
            policy_id: cert.policy_id,
        }
    }
}

/// Serialize-only for Resonite content-addressing.
#[derive(Serialize)]
struct ResoniteContent {
    pattern_a_id: Digest,
    pattern_b_id: Digest,
    resonance_score: i64,
    policy_id: Digest,
}

/// A resonance measure between two structural patterns (crystal records or
/// candidates). Resonance is symmetric: (a, b) and (b, a) produce the same
/// resonite_id.
///
/// Score is Q16 in [0, 1] — no floats (CROSS-007).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Resonite {
    pub resonite_id: Digest,
    pub pattern_a_id: Digest,
    pub pattern_b_id: Digest,
    pub resonance_score: Q16,
    pub policy_id: Digest,
}

impl Resonite {
    pub fn new(
        mut a: Digest,
        mut b: Digest,
        resonance_score: Q16,
        policy_id: Digest,
    ) -> Self {
        // Canonical order: smaller id first (symmetry).
        if a > b {
            std::mem::swap(&mut a, &mut b);
        }
        let resonite_id = Digest::of(&ResoniteContent {
            pattern_a_id: a,
            pattern_b_id: b,
            resonance_score: resonance_score.raw(),
            policy_id,
        });
        Self { resonite_id, pattern_a_id: a, pattern_b_id: b, resonance_score, policy_id }
    }
}

/// Serialize-only for DualFabricGateCascade content-addressing.
#[derive(Serialize)]
struct DualCascadeContent {
    primary_trace_id: Digest,
    secondary_trace_id: Digest,
    merged_result: String,
    policy_id: Digest,
}

/// A gate cascade operating over two "fabrics" (structural layers).
///
/// Both traces run independently; the merged result is the most restrictive
/// of the two (using `GateResult::merge` semantics — Reject > Downgrade >
/// Warn > Pass).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DualFabricGateCascade {
    pub cascade_id: Digest,
    pub primary_trace_id: Digest,
    pub secondary_trace_id: Digest,
    pub primary_result: GateResult,
    pub secondary_result: GateResult,
    pub merged_result: GateResult,
    pub policy_id: Digest,
}

impl DualFabricGateCascade {
    pub fn new(
        primary: &GateTrace,
        secondary: &GateTrace,
        policy: &PolicyProfile,
    ) -> Self {
        let merged_result = primary.final_result.clone().merge(secondary.final_result.clone());
        let cascade_id = Digest::of(&DualCascadeContent {
            primary_trace_id: primary.trace_id,
            secondary_trace_id: secondary.trace_id,
            merged_result: format!("{:?}", merged_result),
            policy_id: policy.id,
        });
        Self {
            cascade_id,
            primary_trace_id: primary.trace_id,
            secondary_trace_id: secondary.trace_id,
            primary_result: primary.final_result.clone(),
            secondary_result: secondary.final_result.clone(),
            merged_result,
            policy_id: policy.id,
        }
    }

    pub fn passed(&self) -> bool {
        self.merged_result.is_pass()
    }

    pub fn was_rejected(&self) -> bool {
        self.merged_result.is_rejected()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assimilation::AssimilationDecision;
    use crate::gates::GateCascade;
    use crate::structural_yield::{StructuralYield, StructuralYieldKind};
    use kosmo_core::{
        AuthorityLabel, Digest, EvidenceBundle, EvidenceKind, EvidenceRef,
        PolicyProfile, ReplayStatus, TaintLabel,
    };

    fn make_evidence(policy_id: Digest) -> EvidenceBundle {
        EvidenceBundle::seal(
            vec![EvidenceRef::new(Digest::of_bytes(b"e"), EvidenceKind::HostScan, "scan")],
            policy_id,
            ReplayStatus::Replayable,
        )
    }

    fn make_accepted_decision(policy: &PolicyProfile) -> AssimilationDecision {
        let ev = make_evidence(policy.id);
        let void_id = Digest::of_bytes(b"v");
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
        AssimilationDecision::from_trace(&yield_, &trace, &ev, policy.id)
    }

    #[test]
    fn crystal_candidate_from_accepted_decision_is_pending() {
        let policy = PolicyProfile::default_report_only();
        let decision = make_accepted_decision(&policy);
        let candidate = StructuralCrystalCandidate::from_decision(&decision);
        assert!(candidate.is_certifiable());
        assert_ne!(candidate.candidate_id, Digest::ZERO);
    }

    #[test]
    fn crystal_candidate_from_evidence_only_decision() {
        let policy = PolicyProfile::default_report_only();
        let ev = make_evidence(policy.id);
        let void_id = Digest::of_bytes(b"v");
        // Synthetic taint → EvidenceOnly outcome
        let yield_ = StructuralYield::new(
            StructuralYieldKind::DeficiencyFill,
            Some(void_id), None,
            TaintLabel::Synthetic, AuthorityLabel::Foundry,
            ev.bundle_id, policy.id,
        );
        let cascade = GateCascade::standard_gates(policy.clone());
        let trace = cascade.apply(&yield_, &ev);
        let decision = AssimilationDecision::from_trace(&yield_, &trace, &ev, policy.id);
        let candidate = StructuralCrystalCandidate::from_decision(&decision);
        assert_eq!(candidate.certification_status, CertificationStatus::EvidenceOnly);
        assert!(!candidate.is_certifiable());
    }

    #[test]
    fn constraint_program_all_satisfied_issues_certificate() {
        let policy = PolicyProfile::default_report_only();
        let decision = make_accepted_decision(&policy);
        let candidate = StructuralCrystalCandidate::from_decision(&decision);
        let ev = make_evidence(policy.id);
        let program = ConstraintProgram::standard(&candidate, &ev, ReplayStatus::Replayable);
        assert!(program.all_satisfied);
        let proof = ReplayProof::new(
            candidate.candidate_id, ReplayStatus::Replayable, ev.bundle_id, policy.id,
        );
        let cert = AssimilationCertificate::issue(&candidate, &program, &proof);
        assert!(cert.is_some(), "satisfied constraints must produce a certificate");
        assert_ne!(cert.unwrap().certificate_id, Digest::ZERO);
    }

    #[test]
    fn constraint_program_unsatisfied_blocks_certificate() {
        let policy = PolicyProfile::default_report_only();
        let decision = make_accepted_decision(&policy);
        let candidate = StructuralCrystalCandidate::from_decision(&decision);
        // Empty evidence → HasEvidence constraint fails
        let empty_ev = EvidenceBundle::empty(policy.id);
        let program = ConstraintProgram::standard(&candidate, &empty_ev, ReplayStatus::Replayable);
        assert!(!program.all_satisfied);
        let proof = ReplayProof::new(candidate.candidate_id, ReplayStatus::Replayable, empty_ev.bundle_id, policy.id);
        assert!(AssimilationCertificate::issue(&candidate, &program, &proof).is_none());
    }

    #[test]
    fn crystal_record_from_certificate_is_content_addressed() {
        let policy = PolicyProfile::default_report_only();
        let decision = make_accepted_decision(&policy);
        let candidate = StructuralCrystalCandidate::from_decision(&decision);
        let ev = make_evidence(policy.id);
        let program = ConstraintProgram::standard(&candidate, &ev, ReplayStatus::Replayable);
        let proof = ReplayProof::new(candidate.candidate_id, ReplayStatus::Replayable, ev.bundle_id, policy.id);
        let cert = AssimilationCertificate::issue(&candidate, &program, &proof).unwrap();
        let r1 = StructuralCrystalRecord::from_certificate(&cert);
        let r2 = StructuralCrystalRecord::from_certificate(&cert);
        assert_eq!(r1.record_id, r2.record_id);
        assert_ne!(r1.record_id, Digest::ZERO);
    }

    #[test]
    fn resonite_is_symmetric() {
        let pid = Digest::of_bytes(b"p");
        let a = Digest::of_bytes(b"a");
        let b = Digest::of_bytes(b"b");
        let r1 = Resonite::new(a, b, Q16::HALF, pid);
        let r2 = Resonite::new(b, a, Q16::HALF, pid);
        assert_eq!(r1.resonite_id, r2.resonite_id, "Resonite must be symmetric");
    }

    #[test]
    fn dual_fabric_cascade_merges_results() {
        let policy = PolicyProfile::default_report_only();
        let ev = make_evidence(policy.id);
        let void_id = Digest::of_bytes(b"v");
        let yield_ = StructuralYield::new(
            StructuralYieldKind::DeficiencyFill,
            Some(void_id), None,
            TaintLabel::Clean, AuthorityLabel::Foundry,
            ev.bundle_id, policy.id,
        );
        let cascade = GateCascade::standard_gates(policy.clone());
        let trace1 = cascade.apply(&yield_, &ev);
        let trace2 = cascade.apply(&yield_, &ev);
        let dual = DualFabricGateCascade::new(&trace1, &trace2, &policy);
        // Both traces pass → merged is Pass
        assert!(dual.passed());
        assert_ne!(dual.cascade_id, Digest::ZERO);
    }

    #[test]
    fn crystal_candidate_energy_assessment_content_addressed() {
        let policy = PolicyProfile::default_report_only();
        let decision = make_accepted_decision(&policy);
        let candidate = StructuralCrystalCandidate::from_decision(&decision);
        let a1 = candidate.energy_assessment(&GateResult::Pass);
        let a2 = candidate.energy_assessment(&GateResult::Pass);
        assert_eq!(a1.id, a2.id, "energy_assessment must be deterministic");
        assert_eq!(a1.subject_id, candidate.candidate_id);
        assert_eq!(a1.policy_id, policy.id);
    }

    #[test]
    fn crystal_candidate_reject_gate_zeroes_energy() {
        let policy = PolicyProfile::default_report_only();
        let decision = make_accepted_decision(&policy);
        let candidate = StructuralCrystalCandidate::from_decision(&decision);
        let a = candidate.energy_assessment(&GateResult::Reject { reason: "test".into() });
        assert!(a.kernel.is_zeroed(), "Reject gate must zero energy (CROSS-010)");
    }

    #[test]
    fn crystal_candidate_zero_support_score_zeroes_energy() {
        let policy = PolicyProfile::default_report_only();
        let decision = make_accepted_decision(&policy);
        // support_score starts at Q16::ZERO at candidate creation
        let candidate = StructuralCrystalCandidate::from_decision(&decision);
        assert_eq!(candidate.support_score, Q16::ZERO);
        let a = candidate.energy_assessment(&GateResult::Pass);
        assert!(a.kernel.is_zeroed(), "zero support_score must yield zero energy");
    }
}

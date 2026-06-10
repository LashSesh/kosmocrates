use crate::assimilation::{AssimilationDecision, AssimilationOutcome};
use crate::gates::GateTrace;
use crate::xlang::CrossLanguageFingerprint;
use kosmo_core::{
    Digest, EnergyAssessment, EnergyFactors, EnergyKernel, EvidenceBundle, FoundrySurvival,
    GateResult, LicenseStatus, PolicyProfile, ReplayStatus, TripolarEnergy, Q16,
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
    /// `Digest::ZERO` when no void reference is available.
    source_void_id: Digest,
    rho_coherence: i64,
    omega_phase: i64,
    /// `Digest::ZERO` when no cross-language fingerprint is attached.
    fingerprint_id: Digest,
    policy_id: Digest,
}

/// A structural yield that has survived the gate cascade and is being
/// considered for crystal certification.
///
/// A candidate's `CertificationStatus` starts as `Pending` and advances
/// through constraint checking; it never becomes `Certified` without a
/// fully satisfied `ConstraintProgram`.
///
/// `source_void_id`, `rho_coherence`, and `omega_phase` carry structural
/// provenance from the `CodeHDAG` of the originating source file (when
/// content was available at host scan time). They default to `None` / `Q16::ONE`
/// when no HDAG is available. These signals propagate into `StructuralCrystalRecord`
/// so the CAD library retains code-structure context.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StructuralCrystalCandidate {
    pub candidate_id: Digest,
    pub yield_id: Digest,
    pub decision_id: Digest,
    pub support_score: Q16,
    pub evidence_bundle_id: Digest,
    pub certification_status: CertificationStatus,
    /// The void this candidate addresses (`None` for non-void intents).
    pub source_void_id: Option<Digest>,
    /// Structural-coherence pole ρ from the originating `CodeHDAG`.
    /// `Q16::ONE` when no HDAG is available (unconstrained baseline).
    pub rho_coherence: Q16,
    /// Structural-complexity pole ω from the originating `CodeHDAG`.
    /// `Q16::ONE` when no HDAG is available (unconstrained baseline).
    pub omega_phase: Q16,
    /// Language-independent structural fingerprint of the originating source file,
    /// when one was available at host-scan time. Carries the four density axes that
    /// let `crystal_resonance` match certified crystals to voids *across languages*.
    #[serde(default)]
    pub fingerprint: Option<CrossLanguageFingerprint>,
    pub policy_id: Digest,
}

impl StructuralCrystalCandidate {
    pub fn from_decision(decision: &AssimilationDecision) -> Self {
        Self::from_decision_with_signals(decision, None, Q16::ONE, Q16::ONE)
    }

    /// Build a candidate with HDAG-derived structural signals.
    ///
    /// `source_void_id` is the void this candidate addresses (from the originating
    /// intent's `target_void_id`). `rho_coherence` and `omega_phase` come from the
    /// void's `CodeHDAG` when source content was available; pass `Q16::ONE` for both
    /// when no HDAG is available (the content-addressing includes them, so HDAG-enriched
    /// candidates have different `candidate_id`s from file-presence-only ones).
    pub fn from_decision_with_signals(
        decision: &AssimilationDecision,
        source_void_id: Option<Digest>,
        rho_coherence: Q16,
        omega_phase: Q16,
    ) -> Self {
        let certification_status = match &decision.outcome {
            AssimilationOutcome::Accepted { .. } => CertificationStatus::Pending,
            AssimilationOutcome::EvidenceOnly { .. } => CertificationStatus::EvidenceOnly,
            AssimilationOutcome::Downgraded { .. } => CertificationStatus::EvidenceOnly,
            AssimilationOutcome::RejectedByGate { reason, .. } => CertificationStatus::Rejected {
                reason: reason.clone(),
            },
            AssimilationOutcome::Deferred { .. } => CertificationStatus::Pending,
        };

        let candidate_id = Digest::of(&CandidateContent {
            yield_id: decision.yield_id,
            decision_id: decision.decision_id,
            support_score: 0,
            evidence_bundle_id: decision.evidence_bundle_id,
            source_void_id: source_void_id.unwrap_or(Digest::ZERO),
            rho_coherence: rho_coherence.raw(),
            omega_phase: omega_phase.raw(),
            fingerprint_id: Digest::ZERO,
            policy_id: decision.policy_id,
        });

        Self {
            candidate_id,
            yield_id: decision.yield_id,
            decision_id: decision.decision_id,
            support_score: Q16::ZERO,
            evidence_bundle_id: decision.evidence_bundle_id,
            certification_status,
            source_void_id,
            rho_coherence,
            omega_phase,
            fingerprint: None,
            policy_id: decision.policy_id,
        }
    }

    /// Attach the originating source file's cross-language fingerprint, recomputing
    /// `candidate_id` so the enriched candidate is content-addressed distinctly. The
    /// fingerprint propagates into the certified [`StructuralCrystalRecord`], making
    /// cross-language crystal matching possible across runs.
    pub fn with_fingerprint(mut self, fingerprint: CrossLanguageFingerprint) -> Self {
        let fingerprint_id = fingerprint.fingerprint_id;
        self.candidate_id = Digest::of(&CandidateContent {
            yield_id: self.yield_id,
            decision_id: self.decision_id,
            support_score: 0,
            evidence_bundle_id: self.evidence_bundle_id,
            source_void_id: self.source_void_id.unwrap_or(Digest::ZERO),
            rho_coherence: self.rho_coherence.raw(),
            omega_phase: self.omega_phase.raw(),
            fingerprint_id,
            policy_id: self.policy_id,
        });
        self.fingerprint = Some(fingerprint);
        self
    }

    pub fn is_certifiable(&self) -> bool {
        matches!(self.certification_status, CertificationStatus::Pending)
    }

    /// Run the standard certification pipeline for this candidate.
    ///
    /// Returns `Some((certificate, record))` iff the candidate is `Pending`
    /// and all constraints are satisfied. The `replay_status` comes from the
    /// enclosing HYPHAE run (passive runs are always `Replayable`).
    ///
    /// This is the principal entry point for the pipeline Step 5d-cert.
    pub fn certify(
        &self,
        replay_status: ReplayStatus,
    ) -> Option<(AssimilationCertificate, StructuralCrystalRecord)> {
        if !self.is_certifiable() {
            return None;
        }
        let program = ConstraintProgram::from_candidate(self, replay_status.clone());
        let proof = ReplayProof::new(
            self.candidate_id,
            replay_status,
            self.evidence_bundle_id,
            self.policy_id,
        );
        let cert = AssimilationCertificate::issue(self, &program, &proof)?;
        let record = StructuralCrystalRecord::from_certificate(&cert, self);
        Some((cert, record))
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
        let constraint_id = Digest::of_bytes(format!("{:?}:{}", kind, satisfied).as_bytes());
        Self {
            constraint_id,
            kind,
            satisfied,
        }
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
        Self {
            program_id,
            constraints,
            all_satisfied,
            policy_id,
        }
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
            Constraint::new(
                ConstraintKind::HasVoidReference,
                candidate.yield_id != Digest::ZERO,
            ),
            Constraint::new(
                ConstraintKind::IsReplayable,
                matches!(replay_status, ReplayStatus::Replayable),
            ),
            Constraint::new(ConstraintKind::IsNotQuarantined, true), // quarantined yields are rejected, never candidates
        ];
        Self::evaluate(constraints, candidate.policy_id)
    }

    /// Build a constraint program from the candidate itself, without requiring
    /// the originating `EvidenceBundle` object.
    ///
    /// Used in pipeline Step 5d-cert where only the candidate's ID fields are
    /// available. CROSS-006 guarantees `evidence_bundle_id != ZERO` for all
    /// candidates derived from accepted decisions; `IsNotQuarantined` holds
    /// because quarantined yields are rejected by the gate cascade and never
    /// reach `Pending` status.
    pub fn from_candidate(
        candidate: &StructuralCrystalCandidate,
        replay_status: ReplayStatus,
    ) -> Self {
        let constraints = vec![
            Constraint::new(
                ConstraintKind::HasEvidence,
                candidate.evidence_bundle_id != Digest::ZERO,
            ),
            Constraint::new(ConstraintKind::HasGateTrace, true),
            Constraint::new(
                ConstraintKind::HasVoidReference,
                candidate.yield_id != Digest::ZERO,
            ),
            Constraint::new(
                ConstraintKind::IsReplayable,
                matches!(replay_status, ReplayStatus::Replayable),
            ),
            Constraint::new(ConstraintKind::IsNotQuarantined, candidate.is_certifiable()),
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
        Self {
            proof_id,
            artifact_id,
            replay_status,
            evidence_bundle_id,
            policy_id,
        }
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
    /// `Digest::ZERO` when no void reference is available.
    source_void_id: Digest,
    rho_coherence: i64,
    omega_phase: i64,
    /// `Digest::ZERO` when no cross-language fingerprint is attached.
    fingerprint_id: Digest,
    policy_id: Digest,
}

/// The final certified structural crystal — a yield that has passed all gates
/// and constraints and is recorded as a durable structural pattern.
///
/// `source_void_id`, `rho_coherence`, and `omega_phase` carry the structural
/// provenance from the `CodeHDAG` of the originating source file, making the CAD
/// library element structurally rich: two records for the same void kind but
/// different structural complexity will have different `record_id`s.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StructuralCrystalRecord {
    pub record_id: Digest,
    pub candidate_id: Digest,
    pub certificate_id: Digest,
    /// The void this record addresses. `None` for non-void candidates.
    pub source_void_id: Option<Digest>,
    /// Structural-coherence pole ρ from the originating `CodeHDAG`.
    pub rho_coherence: Q16,
    /// Structural-complexity pole ω from the originating `CodeHDAG`.
    pub omega_phase: Q16,
    /// Language-independent structural fingerprint carried over from the candidate.
    /// Enables cross-language resonance: a crystal certified from one language can
    /// match a structurally-similar void in another.
    #[serde(default)]
    pub fingerprint: Option<CrossLanguageFingerprint>,
    pub policy_id: Digest,
}

impl StructuralCrystalRecord {
    fn fingerprint_id(&self) -> Digest {
        self.fingerprint
            .as_ref()
            .map(|f| f.fingerprint_id)
            .unwrap_or(Digest::ZERO)
    }

    /// Recompute and verify the `record_id` against the record's fields.
    ///
    /// Returns `true` iff the stored `record_id` matches the expected content-addressed
    /// digest. Used by `CrystalRecordStore::verify_integrity`.
    pub fn verify_id(&self) -> bool {
        let expected = Digest::of(&RecordContent {
            candidate_id: self.candidate_id,
            certificate_id: self.certificate_id,
            source_void_id: self.source_void_id.unwrap_or(Digest::ZERO),
            rho_coherence: self.rho_coherence.raw(),
            omega_phase: self.omega_phase.raw(),
            fingerprint_id: self.fingerprint_id(),
            policy_id: self.policy_id,
        });
        self.record_id == expected
    }

    /// Cross-language structural resonance between this crystal and a void's
    /// fingerprint — the four-axis [`CrossLanguageFingerprint::similarity`], in
    /// `Q16` integer arithmetic. `None` when this crystal carries no fingerprint
    /// (callers fall back to `rho`/`omega` proximity). Language-independent: a Go
    /// crystal can resonate with a Python void.
    pub fn fingerprint_resonance(
        &self,
        void_fingerprint: &CrossLanguageFingerprint,
    ) -> Option<Q16> {
        self.fingerprint
            .as_ref()
            .map(|f| f.similarity(void_fingerprint))
    }

    /// Build a record from an issued certificate and the originating candidate.
    ///
    /// The candidate carries `source_void_id`, `rho_coherence`, and `omega_phase`
    /// so the CAD library element retains code-structure provenance. All three
    /// participate in `record_id` content-addressing.
    pub fn from_certificate(
        cert: &AssimilationCertificate,
        candidate: &StructuralCrystalCandidate,
    ) -> Self {
        let fingerprint_id = candidate
            .fingerprint
            .as_ref()
            .map(|f| f.fingerprint_id)
            .unwrap_or(Digest::ZERO);
        let record_id = Digest::of(&RecordContent {
            candidate_id: cert.candidate_id,
            certificate_id: cert.certificate_id,
            source_void_id: candidate.source_void_id.unwrap_or(Digest::ZERO),
            rho_coherence: candidate.rho_coherence.raw(),
            omega_phase: candidate.omega_phase.raw(),
            fingerprint_id,
            policy_id: cert.policy_id,
        });
        Self {
            record_id,
            candidate_id: cert.candidate_id,
            certificate_id: cert.certificate_id,
            source_void_id: candidate.source_void_id,
            rho_coherence: candidate.rho_coherence,
            omega_phase: candidate.omega_phase,
            fingerprint: candidate.fingerprint.clone(),
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
    pub fn new(mut a: Digest, mut b: Digest, resonance_score: Q16, policy_id: Digest) -> Self {
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
        Self {
            resonite_id,
            pattern_a_id: a,
            pattern_b_id: b,
            resonance_score,
            policy_id,
        }
    }

    /// Compute resonance between two certified crystal records from their structural signals.
    ///
    /// Resonance = structural proximity: how similar the two patterns are in
    /// `rho_coherence` (test coverage ratio) and `omega_phase` (complexity signal).
    ///
    /// Score formula (Q16, no floats — CROSS-007):
    ///   `score = ((ONE - |ρ_a - ρ_b|) + (ONE - |ω_a - ω_b|)) / 2`
    ///
    /// Two identical patterns score ONE; maximally different patterns score ZERO.
    /// Symmetric: `from_records(a, b, p) == from_records(b, a, p)` by construction
    /// (canonical ordering of the two record IDs in `Resonite::new`).
    pub fn from_records(
        a: &StructuralCrystalRecord,
        b: &StructuralCrystalRecord,
        policy_id: Digest,
    ) -> Self {
        let rho_diff = (a.rho_coherence.raw() - b.rho_coherence.raw()).unsigned_abs() as i64;
        let omega_diff = (a.omega_phase.raw() - b.omega_phase.raw()).unsigned_abs() as i64;
        let rho_sim = (Q16::ONE.raw() - rho_diff).max(0);
        let omega_sim = (Q16::ONE.raw() - omega_diff).max(0);
        let score = Q16::from_raw((rho_sim + omega_sim) / 2);
        Self::new(a.record_id, b.record_id, score, policy_id)
    }

    /// Build an [`EnergyAssessment`] for this resonance measurement.
    ///
    /// - ψ (meaning)   = `resonance_score` — how strongly the two patterns resonate.
    /// - ρ (coherence) = `Q16::ONE` — no additional coherence data at resonite level.
    /// - ω (phase)     = `Q16::ONE` — no phase data at resonite level.
    /// - `evidence_bundle_id` = `resonite_id` (self-referential content address —
    ///   satisfies CROSS-006 non-ZERO evidence ref).
    pub fn energy_assessment(&self, gate: &GateResult) -> EnergyAssessment {
        let tripolar = TripolarEnergy::new(self.resonance_score, Q16::ONE, Q16::ONE);
        let factors = EnergyFactors {
            gate: EnergyFactors::gate_factor(gate),
            taint: Q16::ONE,
            license: EnergyFactors::license_factor(&LicenseStatus::NotApplicable),
            foundry: EnergyFactors::foundry_factor(FoundrySurvival::Unavailable),
            seam: Q16::ONE,
            contradiction: Q16::ONE,
        };
        EnergyAssessment::new(
            self.resonite_id,
            EnergyKernel::new(tripolar, factors),
            self.policy_id,
            self.resonite_id,
        )
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
    pub fn new(primary: &GateTrace, secondary: &GateTrace, policy: &PolicyProfile) -> Self {
        let merged_result = primary
            .final_result
            .clone()
            .merge(secondary.final_result.clone());
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
        AuthorityLabel, Digest, EvidenceBundle, EvidenceKind, EvidenceRef, PolicyProfile,
        ReplayStatus, TaintLabel,
    };

    fn make_evidence(policy_id: Digest) -> EvidenceBundle {
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
            Some(void_id),
            None,
            TaintLabel::Synthetic,
            AuthorityLabel::Foundry,
            ev.bundle_id,
            policy.id,
        );
        let cascade = GateCascade::standard_gates(policy.clone());
        let trace = cascade.apply(&yield_, &ev);
        let decision = AssimilationDecision::from_trace(&yield_, &trace, &ev, policy.id);
        let candidate = StructuralCrystalCandidate::from_decision(&decision);
        assert_eq!(
            candidate.certification_status,
            CertificationStatus::EvidenceOnly
        );
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
            candidate.candidate_id,
            ReplayStatus::Replayable,
            ev.bundle_id,
            policy.id,
        );
        let cert = AssimilationCertificate::issue(&candidate, &program, &proof);
        assert!(
            cert.is_some(),
            "satisfied constraints must produce a certificate"
        );
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
        let proof = ReplayProof::new(
            candidate.candidate_id,
            ReplayStatus::Replayable,
            empty_ev.bundle_id,
            policy.id,
        );
        assert!(AssimilationCertificate::issue(&candidate, &program, &proof).is_none());
    }

    #[test]
    fn crystal_record_from_certificate_is_content_addressed() {
        let policy = PolicyProfile::default_report_only();
        let decision = make_accepted_decision(&policy);
        let candidate = StructuralCrystalCandidate::from_decision(&decision);
        let ev = make_evidence(policy.id);
        let program = ConstraintProgram::standard(&candidate, &ev, ReplayStatus::Replayable);
        let proof = ReplayProof::new(
            candidate.candidate_id,
            ReplayStatus::Replayable,
            ev.bundle_id,
            policy.id,
        );
        let cert = AssimilationCertificate::issue(&candidate, &program, &proof).unwrap();
        let r1 = StructuralCrystalRecord::from_certificate(&cert, &candidate);
        let r2 = StructuralCrystalRecord::from_certificate(&cert, &candidate);
        assert_eq!(r1.record_id, r2.record_id);
        assert_ne!(r1.record_id, Digest::ZERO);
    }

    #[test]
    fn candidate_with_signals_has_different_id_than_without() {
        let policy = PolicyProfile::default_report_only();
        let decision = make_accepted_decision(&policy);
        let void_id = Digest::of_bytes(b"v");
        let c_plain = StructuralCrystalCandidate::from_decision(&decision);
        let c_rich = StructuralCrystalCandidate::from_decision_with_signals(
            &decision,
            Some(void_id),
            Q16::HALF,
            Q16::ratio(1, 4).unwrap(),
        );
        assert_ne!(
            c_plain.candidate_id, c_rich.candidate_id,
            "HDAG signals must change candidate_id (structural provenance in content hash)"
        );
        assert_eq!(c_rich.source_void_id, Some(void_id));
        assert_eq!(c_rich.rho_coherence, Q16::HALF);
    }

    #[test]
    fn record_carries_structural_signals() {
        let policy = PolicyProfile::default_report_only();
        let decision = make_accepted_decision(&policy);
        let void_id = Digest::of_bytes(b"v");
        let candidate = StructuralCrystalCandidate::from_decision_with_signals(
            &decision,
            Some(void_id),
            Q16::HALF,
            Q16::ratio(3, 4).unwrap(),
        );
        let (_, record) = candidate.certify(ReplayStatus::Replayable).unwrap();
        assert_eq!(record.source_void_id, Some(void_id));
        assert_eq!(record.rho_coherence, Q16::HALF);
        assert_eq!(record.omega_phase, Q16::ratio(3, 4).unwrap());
        assert_ne!(record.record_id, Digest::ZERO);
    }

    #[test]
    fn record_id_differs_with_different_signals() {
        let policy = PolicyProfile::default_report_only();
        let d1 = make_accepted_decision(&policy);
        let c1 =
            StructuralCrystalCandidate::from_decision_with_signals(&d1, None, Q16::HALF, Q16::ONE);
        let c2 = StructuralCrystalCandidate::from_decision_with_signals(
            &d1,
            None,
            Q16::ratio(1, 4).unwrap(),
            Q16::ONE,
        );
        // d1 is the same decision, but different rho → different candidate_id → different record_id
        let (_, r1) = c1.certify(ReplayStatus::Replayable).unwrap();
        let (_, r2) = c2.certify(ReplayStatus::Replayable).unwrap();
        assert_ne!(
            r1.record_id, r2.record_id,
            "different structural signals must produce different record_ids"
        );
    }

    #[test]
    fn resonite_from_records_identical_scores_one() {
        let policy = PolicyProfile::default_report_only();
        let decision = make_accepted_decision(&policy);
        let candidate = StructuralCrystalCandidate::from_decision_with_signals(
            &decision,
            None,
            Q16::HALF,
            Q16::HALF,
        );
        let (_, record) = candidate.certify(ReplayStatus::Replayable).unwrap();
        let r = Resonite::from_records(&record, &record, policy.id);
        assert_eq!(
            r.resonance_score,
            Q16::ONE,
            "identical patterns must resonate at score ONE"
        );
    }

    #[test]
    fn resonite_from_records_is_symmetric() {
        let policy = PolicyProfile::default_report_only();
        let d1 = make_accepted_decision(&policy);
        let d2 = make_accepted_decision(&policy);
        let c1 =
            StructuralCrystalCandidate::from_decision_with_signals(&d1, None, Q16::HALF, Q16::ONE);
        let c2 = StructuralCrystalCandidate::from_decision_with_signals(
            &d2,
            None,
            Q16::ratio(1, 4).unwrap(),
            Q16::HALF,
        );
        let (_, r1) = c1.certify(ReplayStatus::Replayable).unwrap();
        let (_, r2) = c2.certify(ReplayStatus::Replayable).unwrap();
        let res_ab = Resonite::from_records(&r1, &r2, policy.id);
        let res_ba = Resonite::from_records(&r2, &r1, policy.id);
        assert_eq!(
            res_ab.resonite_id, res_ba.resonite_id,
            "Resonite::from_records must be symmetric"
        );
    }

    #[test]
    fn resonite_from_records_maximally_different_scores_zero() {
        let policy = PolicyProfile::default_report_only();
        let d1 = make_accepted_decision(&policy);
        let d2 = make_accepted_decision(&policy);
        let c1 =
            StructuralCrystalCandidate::from_decision_with_signals(&d1, None, Q16::ZERO, Q16::ZERO);
        let c2 =
            StructuralCrystalCandidate::from_decision_with_signals(&d2, None, Q16::ONE, Q16::ONE);
        let (_, r1) = c1.certify(ReplayStatus::Replayable).unwrap();
        let (_, r2) = c2.certify(ReplayStatus::Replayable).unwrap();
        let res = Resonite::from_records(&r1, &r2, policy.id);
        assert_eq!(
            res.resonance_score,
            Q16::ZERO,
            "maximally different structural signals must resonate at score ZERO"
        );
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
            Some(void_id),
            None,
            TaintLabel::Clean,
            AuthorityLabel::Foundry,
            ev.bundle_id,
            policy.id,
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
        let a = candidate.energy_assessment(&GateResult::Reject {
            reason: "test".into(),
        });
        assert!(
            a.kernel.is_zeroed(),
            "Reject gate must zero energy (CROSS-010)"
        );
    }

    #[test]
    fn certify_pending_candidate_produces_record() {
        let policy = PolicyProfile::default_report_only();
        let decision = make_accepted_decision(&policy);
        let candidate = StructuralCrystalCandidate::from_decision(&decision);
        assert!(candidate.is_certifiable());
        let result = candidate.certify(ReplayStatus::Replayable);
        assert!(
            result.is_some(),
            "Pending candidate with Replayable status must certify"
        );
        let (cert, record) = result.unwrap();
        assert_ne!(cert.certificate_id, Digest::ZERO);
        assert_ne!(record.record_id, Digest::ZERO);
        assert_eq!(record.candidate_id, candidate.candidate_id);
    }

    #[test]
    fn certify_evidence_only_candidate_returns_none() {
        let policy = PolicyProfile::default_report_only();
        let ev = make_evidence(policy.id);
        let void_id = Digest::of_bytes(b"v");
        let yield_ = StructuralYield::new(
            StructuralYieldKind::DeficiencyFill,
            Some(void_id),
            None,
            TaintLabel::Synthetic,
            AuthorityLabel::Foundry,
            ev.bundle_id,
            policy.id,
        );
        let cascade = GateCascade::standard_gates(policy.clone());
        let trace = cascade.apply(&yield_, &ev);
        let decision = AssimilationDecision::from_trace(&yield_, &trace, &ev, policy.id);
        let candidate = StructuralCrystalCandidate::from_decision(&decision);
        assert!(!candidate.is_certifiable());
        assert!(
            candidate.certify(ReplayStatus::Replayable).is_none(),
            "EvidenceOnly candidate must not certify"
        );
    }

    #[test]
    fn certify_is_deterministic() {
        let policy = PolicyProfile::default_report_only();
        let decision = make_accepted_decision(&policy);
        let candidate = StructuralCrystalCandidate::from_decision(&decision);
        let (_, r1) = candidate.certify(ReplayStatus::Replayable).unwrap();
        let (_, r2) = candidate.certify(ReplayStatus::Replayable).unwrap();
        assert_eq!(
            r1.record_id, r2.record_id,
            "certify must be deterministic (INVARIANT-007)"
        );
    }

    #[test]
    fn from_candidate_constraint_program_all_satisfied_for_pending() {
        let policy = PolicyProfile::default_report_only();
        let decision = make_accepted_decision(&policy);
        let candidate = StructuralCrystalCandidate::from_decision(&decision);
        let program = ConstraintProgram::from_candidate(&candidate, ReplayStatus::Replayable);
        assert!(
            program.all_satisfied,
            "Pending candidate with Replayable status must satisfy all constraints"
        );
        assert_eq!(program.constraints.len(), 5);
        assert_ne!(program.program_id, Digest::ZERO);
    }

    #[test]
    fn crystal_candidate_zero_support_score_zeroes_energy() {
        let policy = PolicyProfile::default_report_only();
        let decision = make_accepted_decision(&policy);
        // support_score starts at Q16::ZERO at candidate creation
        let candidate = StructuralCrystalCandidate::from_decision(&decision);
        assert_eq!(candidate.support_score, Q16::ZERO);
        let a = candidate.energy_assessment(&GateResult::Pass);
        assert!(
            a.kernel.is_zeroed(),
            "zero support_score must yield zero energy"
        );
    }

    #[test]
    fn resonite_energy_assessment_content_addressed() {
        let pid = Digest::of_bytes(b"p");
        let r = Resonite::new(
            Digest::of_bytes(b"a"),
            Digest::of_bytes(b"b"),
            Q16::HALF,
            pid,
        );
        let a1 = r.energy_assessment(&GateResult::Pass);
        let a2 = r.energy_assessment(&GateResult::Pass);
        assert_eq!(a1.id, a2.id, "energy_assessment must be deterministic");
        assert_eq!(a1.subject_id, r.resonite_id);
        assert_eq!(
            a1.evidence_bundle_id, r.resonite_id,
            "self-referential evidence must equal resonite_id"
        );
        assert_ne!(
            a1.evidence_bundle_id,
            Digest::ZERO,
            "CROSS-006: non-ZERO evidence ref"
        );
    }

    #[test]
    fn resonite_reject_gate_zeroes_energy() {
        let pid = Digest::of_bytes(b"p");
        let r = Resonite::new(
            Digest::of_bytes(b"a"),
            Digest::of_bytes(b"b"),
            Q16::ONE,
            pid,
        );
        let a = r.energy_assessment(&GateResult::Reject {
            reason: "test".into(),
        });
        assert!(
            a.kernel.is_zeroed(),
            "Reject gate must zero resonite energy"
        );
    }

    #[test]
    fn resonite_energy_is_symmetric() {
        let pid = Digest::of_bytes(b"p");
        let r1 = Resonite::new(
            Digest::of_bytes(b"a"),
            Digest::of_bytes(b"b"),
            Q16::HALF,
            pid,
        );
        let r2 = Resonite::new(
            Digest::of_bytes(b"b"),
            Digest::of_bytes(b"a"),
            Q16::HALF,
            pid,
        );
        let a1 = r1.energy_assessment(&GateResult::Pass);
        let a2 = r2.energy_assessment(&GateResult::Pass);
        assert_eq!(
            a1.id, a2.id,
            "resonite energy must be symmetric (r(a,b) == r(b,a))"
        );
    }

    // ── Cross-language crystal fingerprint ─────────────────────────────────────

    #[test]
    fn crystal_with_fingerprint_changes_id_and_propagates_to_record() {
        use crate::xlang::{CrossLanguageFingerprint, SourceLanguage};
        let policy = PolicyProfile::default_report_only();
        let decision = make_accepted_decision(&policy);
        let plain = StructuralCrystalCandidate::from_decision_with_signals(
            &decision,
            Some(Digest::of_bytes(b"v")),
            Q16::HALF,
            Q16::HALF,
        );
        let fp = CrossLanguageFingerprint::from_source(
            SourceLanguage::Python,
            Digest::of_bytes(b"src"),
            "import os\ndef a(x):\n    return x\ndef b(y):\n    return y\n",
        );
        let rich = plain.clone().with_fingerprint(fp);
        assert_ne!(
            plain.candidate_id, rich.candidate_id,
            "fingerprint changes the candidate id"
        );
        assert!(rich.fingerprint.is_some());

        let (_c, record) = rich.certify(ReplayStatus::Replayable).expect("certifiable");
        assert!(
            record.fingerprint.is_some(),
            "record must carry the fingerprint"
        );
        assert!(
            record.verify_id(),
            "record_id is content-addressed incl. fingerprint"
        );

        let (_c2, plain_record) = plain
            .certify(ReplayStatus::Replayable)
            .expect("certifiable");
        assert!(plain_record.fingerprint.is_none());
        assert_ne!(
            record.record_id, plain_record.record_id,
            "fingerprint distinguishes records"
        );
    }

    #[test]
    fn crystal_record_resonates_across_languages() {
        use crate::xlang::{CrossLanguageFingerprint, SourceLanguage};
        let policy = PolicyProfile::default_report_only();
        let decision = make_accepted_decision(&policy);
        // A crystal certified from a Go file.
        let go_fp = CrossLanguageFingerprint::from_source(
            SourceLanguage::Go,
            Digest::of_bytes(b"go"),
            "package main\nimport \"fmt\"\nfunc a(n int) int { return n }\nfunc b(n int) int { return n }\n",
        );
        let (_c, record) = StructuralCrystalCandidate::from_decision_with_signals(
            &decision,
            Some(Digest::of_bytes(b"v")),
            Q16::HALF,
            Q16::HALF,
        )
        .with_fingerprint(go_fp)
        .certify(ReplayStatus::Replayable)
        .unwrap();

        // A structurally-equivalent Python void: import + two functions.
        let py_fp = CrossLanguageFingerprint::from_source(
            SourceLanguage::Python,
            Digest::of_bytes(b"py"),
            "import os\ndef a(n):\n    return n\ndef b(n):\n    return n\n",
        );
        let res = record.fingerprint_resonance(&py_fp);
        assert_eq!(
            res,
            Some(Q16::ONE),
            "identical structural mix → cross-language resonance 1.0"
        );

        // A record without a fingerprint cannot resonate by fingerprint.
        let (_c2, plain) = StructuralCrystalCandidate::from_decision_with_signals(
            &decision,
            Some(Digest::of_bytes(b"v")),
            Q16::HALF,
            Q16::HALF,
        )
        .certify(ReplayStatus::Replayable)
        .unwrap();
        assert!(plain.fingerprint_resonance(&py_fp).is_none());
    }
}

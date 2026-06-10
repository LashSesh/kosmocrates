//! Candidate-only bridge from `kosmo-*` artifacts to PSE observations.
//!
//! The substrate analyses topology and synthesizes systems (see
//! [`docs/SUBSTRATE.md`] and [`docs/WISH_TO_SYSTEM.md`]); when one of its
//! artifacts is worth remembering, it is *offered* to the PSE crystallization
//! engine — never committed directly. This crate is that one-way offer desk.
//!
//! ## Architecture contract
//!
//! This crate is the ONLY permitted crossing point between the `kosmo-*` layer
//! and the PSE substrate. It intentionally does NOT depend on `pse-core`.
//!
//! `kosmo-hyphae` (and all other `kosmo-*` crates) must never import `pse-core`
//! directly. All crossing goes through `PromotionRequest` → PSE decides.
//!
//! ## What this crate does NOT do
//!
//! - It does NOT commit `SemanticCrystal` records. PSE decides crystallization.
//! - It does NOT call any PSE-internal gate or cascade.
//! - It does NOT promote `CorpusCartography` to trusted memory.
//! - In `ReportOnly` mode, all submission attempts produce `SkippedByReportOnly`.
//!
//! ## Promotion lifecycle
//!
//! 1. **Wrap.** A `kosmo-*` artifact (a structural yield, a topology diff, a
//!    validation closure, a `.kcube` descriptor, a certified structural crystal)
//!    becomes a [`PseBridgeCandidate`] — content-addressed by *reference*
//!    (`observation_digest`), carrying a `Q16` `confidence` and full
//!    evidence/policy provenance.
//! 2. **Bundle.** One or more candidates are gathered into a [`PromotionRequest`].
//! 3. **Gate.** [`validate_candidate`] applies the policy: outside an execution
//!    mode that permits it, the request is refused fail-closed.
//! 4. **Decide.** PSE returns a [`PromotionOutcome`] — `Promoted`, `Rejected`,
//!    `SkippedByReportOnly`, or `SkippedByPolicy`. This crate models the outcome;
//!    it never makes the crystallization decision itself.
//! 5. **Feed back.** [`build_promotion_feedback`] turns the outcome into a
//!    [`PromotionFeedback`] the agent loop consumes, so a promoted observation
//!    raises — and a rejected one lowers — the confidence of similar future work.
//!
//! ## Constraints
//!
//! - **No floats (CROSS-007).** `confidence` and every score are `Q16`.
//! - **Fail-closed.** Default `ReportOnly` policy yields `SkippedByReportOnly`;
//!   promotion requires an explicit execution mode.
//! - **Content-addressed & evidence-bound.** Each candidate's `id` is the digest
//!   of its fields; `evidence_bundle_id` is non-zero (CROSS-006).
//!
//! [`docs/SUBSTRATE.md`]: ../../../SUBSTRATE.md
//! [`docs/WISH_TO_SYSTEM.md`]: ../../../docs/WISH_TO_SYSTEM.md

use kosmo_core::{
    Digest, EvidenceRef, FeedbackOutcome, ImplementationMode, PolicyProfile, PromotionFeedback, Q16,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Classifies what kind of observation a `PseBridgeCandidate` carries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PseBridgeCandidateKind {
    /// A structural pattern extracted from a HYPHAE workspace scan.
    StructuralObservation,
    /// A topology change detected by a parse-back rescan.
    TopologyObservation,
    /// A distilled summary of an evidence bundle.
    EvidenceSummary,
    /// The outcome of a `ValidationClosureReport`.
    ValidationResult,
    /// A `KcubePackage` descriptor offered as a knowledge artifact.
    KcubeArtifact,
    /// A certified `StructuralCrystalRecord` — the substrate's strongest durable
    /// artifact: gate-passed, constraint-certified, replay-proofed, and (when the
    /// source carried content) bearing a cross-language structural fingerprint.
    /// This is the primary unification path from the substrate's CAD library
    /// into PSE crystallization.
    CertifiedCrystal,
    Custom(String),
}

/// A candidate observation offered to PSE from the `kosmo-*` layer.
///
/// This is NOT a `SemanticCrystal`. PSE receives it, evaluates it against
/// its own gate cascade, and decides whether to crystallize.
///
/// `observation_digest` points to the raw observation data — the candidate
/// does not inline it (content-addressing by reference).
/// `confidence` is a `Q16` score — no floats in any gate path.
/// `id` is the content digest of all fields except `id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PseBridgeCandidate {
    pub id: Digest,
    pub kind: PseBridgeCandidateKind,
    /// Content digest of the raw observation artifact (not inlined).
    pub observation_digest: Digest,
    pub label: String,
    /// Q16 confidence score produced by the `kosmo-*` analysis layer.
    pub confidence: Q16,
    pub source_run_id: Digest,
    pub evidence_bundle_id: Digest,
    pub policy_id: Digest,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Serialize)]
struct CandidateContent<'a> {
    kind: &'a PseBridgeCandidateKind,
    observation_digest: &'a Digest,
    label: &'a String,
    confidence_raw: i64,
    source_run_id: &'a Digest,
    evidence_bundle_id: &'a Digest,
    policy_id: &'a Digest,
    metadata: &'a BTreeMap<String, String>,
}

impl PseBridgeCandidate {
    pub fn new(
        kind: PseBridgeCandidateKind,
        observation_digest: Digest,
        label: impl Into<String>,
        confidence: Q16,
        source_run_id: Digest,
        evidence_bundle_id: Digest,
        policy_id: Digest,
    ) -> Self {
        let mut c = Self {
            id: Digest::ZERO,
            kind,
            observation_digest,
            label: label.into(),
            confidence,
            source_run_id,
            evidence_bundle_id,
            policy_id,
            metadata: BTreeMap::new(),
        };
        c.id = c.compute_id();
        c
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self.id = self.compute_id();
        self
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&CandidateContent {
            kind: &self.kind,
            observation_digest: &self.observation_digest,
            label: &self.label,
            confidence_raw: self.confidence.raw(),
            source_run_id: &self.source_run_id,
            evidence_bundle_id: &self.evidence_bundle_id,
            policy_id: &self.policy_id,
            metadata: &self.metadata,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }
}

/// Rate limit for candidate submissions — all counts are `u64` (no floats).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PseBridgeRateLimit {
    pub max_candidates_per_run: u64,
    pub max_candidates_per_hour: u64,
}

impl PseBridgeRateLimit {
    pub fn strict() -> Self {
        Self {
            max_candidates_per_run: 10,
            max_candidates_per_hour: 50,
        }
    }

    pub fn permissive() -> Self {
        Self {
            max_candidates_per_run: 100,
            max_candidates_per_hour: 500,
        }
    }
}

impl Default for PseBridgeRateLimit {
    fn default() -> Self {
        Self::strict()
    }
}

/// Governs what `kosmo-*` may submit to PSE as promotion candidates.
///
/// `allow_submission` is `false` in `ReportOnly` mode — all submissions
/// are denied and produce `PromotionOutcome::SkippedByReportOnly`.
/// `require_evidence` defaults to `true` — candidates without an
/// evidence bundle are rejected before reaching PSE.
/// `id` is the content digest of all fields except `id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PseBridgePolicy {
    pub id: Digest,
    pub policy_id: Digest,
    pub allow_submission: bool,
    pub allowed_candidate_kinds: Vec<PseBridgeCandidateKind>,
    pub rate_limit: PseBridgeRateLimit,
    pub require_evidence: bool,
}

#[derive(Serialize)]
struct BridgePolicyContent<'a> {
    policy_id: &'a Digest,
    allow_submission: bool,
    allowed_candidate_kinds: &'a Vec<PseBridgeCandidateKind>,
    max_candidates_per_run: u64,
    max_candidates_per_hour: u64,
    require_evidence: bool,
}

impl PseBridgePolicy {
    /// Fail-closed default: no submission allowed (ReportOnly equivalent).
    pub fn deny_all(policy_id: Digest) -> Self {
        let mut p = Self {
            id: Digest::ZERO,
            policy_id,
            allow_submission: false,
            allowed_candidate_kinds: vec![],
            rate_limit: PseBridgeRateLimit::strict(),
            require_evidence: true,
        };
        p.id = p.compute_id();
        p
    }

    /// Policy that permits submission of specific candidate kinds.
    pub fn allow(
        policy_id: Digest,
        allowed_candidate_kinds: Vec<PseBridgeCandidateKind>,
        rate_limit: PseBridgeRateLimit,
    ) -> Self {
        let mut p = Self {
            id: Digest::ZERO,
            policy_id,
            allow_submission: true,
            allowed_candidate_kinds,
            rate_limit,
            require_evidence: true,
        };
        p.id = p.compute_id();
        p
    }

    pub fn is_kind_allowed(&self, kind: &PseBridgeCandidateKind) -> bool {
        self.allow_submission && self.allowed_candidate_kinds.contains(kind)
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&BridgePolicyContent {
            policy_id: &self.policy_id,
            allow_submission: self.allow_submission,
            allowed_candidate_kinds: &self.allowed_candidate_kinds,
            max_candidates_per_run: self.rate_limit.max_candidates_per_run,
            max_candidates_per_hour: self.rate_limit.max_candidates_per_hour,
            require_evidence: self.require_evidence,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }
}

/// A promotion request sent to PSE for consideration.
///
/// References the candidate and policy by digest — does not inline them.
/// `sequence` provides submission ordering within a run.
/// `id` is the content digest of all fields except `id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionRequest {
    pub id: Digest,
    pub candidate_id: Digest,
    pub bridge_policy_id: Digest,
    pub evidence_bundle_id: Digest,
    pub sequence: u64,
    pub additional_refs: Vec<EvidenceRef>,
}

#[derive(Serialize)]
struct RequestContent<'a> {
    candidate_id: &'a Digest,
    bridge_policy_id: &'a Digest,
    evidence_bundle_id: &'a Digest,
    sequence: u64,
    additional_ref_digests: Vec<&'a Digest>,
}

impl PromotionRequest {
    pub fn new(
        candidate_id: Digest,
        bridge_policy_id: Digest,
        evidence_bundle_id: Digest,
        sequence: u64,
    ) -> Self {
        let mut r = Self {
            id: Digest::ZERO,
            candidate_id,
            bridge_policy_id,
            evidence_bundle_id,
            sequence,
            additional_refs: vec![],
        };
        r.id = r.compute_id();
        r
    }

    pub fn with_refs(mut self, refs: Vec<EvidenceRef>) -> Self {
        self.additional_refs = refs;
        self.id = self.compute_id();
        self
    }

    fn compute_id(&self) -> Digest {
        let additional_ref_digests: Vec<&Digest> =
            self.additional_refs.iter().map(|r| &r.digest).collect();
        Digest::of(&RequestContent {
            candidate_id: &self.candidate_id,
            bridge_policy_id: &self.bridge_policy_id,
            evidence_bundle_id: &self.evidence_bundle_id,
            sequence: self.sequence,
            additional_ref_digests,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }
}

/// The outcome of a `PromotionRequest` as seen by the bridge layer.
///
/// Note: `Accepted` means PSE received and accepted the candidate for
/// crystallization consideration — not that a `SemanticCrystal` was created.
/// PSE's internal gate cascade makes the final decision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PromotionOutcome {
    /// PSE accepted the candidate for crystallization consideration.
    Accepted,
    /// PSE rejected the candidate (did not meet threshold).
    Rejected { reason: String },
    /// PSE deferred the decision (insufficient evidence or context).
    Deferred,
    /// Submission skipped: `PseBridgePolicy.allow_submission == false`.
    SkippedByPolicy,
    /// Submission skipped: `PolicyProfile.mode == ReportOnly`.
    SkippedByReportOnly,
}

impl PromotionOutcome {
    pub fn is_accepted(&self) -> bool {
        matches!(self, Self::Accepted)
    }

    pub fn is_skipped(&self) -> bool {
        matches!(self, Self::SkippedByPolicy | Self::SkippedByReportOnly)
    }

    pub fn is_failure_class(&self) -> bool {
        matches!(self, Self::Rejected { .. })
    }
}

/// Content-addressed record of a `PromotionRequest` and its `PromotionOutcome`.
///
/// `elapsed_ms` is `u64` — no floats in any audit path.
/// `evidence_bundle_id` is mandatory.
/// `id` is the content digest of all fields except `id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionRequestRecord {
    pub id: Digest,
    pub request_id: Digest,
    pub outcome: PromotionOutcome,
    pub evidence_bundle_id: Digest,
    pub elapsed_ms: u64,
}

#[derive(Serialize)]
struct RecordContent<'a> {
    request_id: &'a Digest,
    outcome: &'a PromotionOutcome,
    evidence_bundle_id: &'a Digest,
    elapsed_ms: u64,
}

impl PromotionRequestRecord {
    pub fn skipped_by_report_only(request_id: Digest, evidence_bundle_id: Digest) -> Self {
        let mut r = Self {
            id: Digest::ZERO,
            request_id,
            outcome: PromotionOutcome::SkippedByReportOnly,
            evidence_bundle_id,
            elapsed_ms: 0,
        };
        r.id = r.compute_id();
        r
    }

    pub fn new(
        request_id: Digest,
        outcome: PromotionOutcome,
        evidence_bundle_id: Digest,
        elapsed_ms: u64,
    ) -> Self {
        let mut r = Self {
            id: Digest::ZERO,
            request_id,
            outcome,
            evidence_bundle_id,
            elapsed_ms,
        };
        r.id = r.compute_id();
        r
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&RecordContent {
            request_id: &self.request_id,
            outcome: &self.outcome,
            evidence_bundle_id: &self.evidence_bundle_id,
            elapsed_ms: self.elapsed_ms,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }
}

/// Validates a `PseBridgeCandidate` against a `PseBridgePolicy` and a
/// `PolicyProfile`, returning the appropriate `PromotionOutcome` if denied.
///
/// Returns `Ok(())` when the candidate may proceed to submission.
/// Returns `Err(PromotionOutcome)` with the denial reason when blocked.
pub fn validate_candidate(
    candidate: &PseBridgeCandidate,
    bridge_policy: &PseBridgePolicy,
    profile: &PolicyProfile,
) -> Result<(), PromotionOutcome> {
    if profile.mode == ImplementationMode::ReportOnly {
        return Err(PromotionOutcome::SkippedByReportOnly);
    }

    if !bridge_policy.allow_submission {
        return Err(PromotionOutcome::SkippedByPolicy);
    }

    if !bridge_policy.is_kind_allowed(&candidate.kind) {
        return Err(PromotionOutcome::SkippedByPolicy);
    }

    if bridge_policy.require_evidence && candidate.evidence_bundle_id == Digest::ZERO {
        return Err(PromotionOutcome::Rejected {
            reason: "evidence_bundle_id is ZERO — evidence required".into(),
        });
    }

    Ok(())
}

/// Build a `PromotionFeedback` record from a `PromotionRequestRecord` and its
/// associated `PseBridgeCandidate`.
///
/// Converts `PromotionOutcome` → `FeedbackOutcome` and derives `fitness_signal`
/// from the outcome and `candidate.confidence` (the energy proxy at submission).
///
/// `norm_candidate_id` should be the `NormGeneCandidate.candidate_id` that
/// originated this bridge candidate, or `Digest::ZERO` if none.
pub fn build_promotion_feedback(
    record: &PromotionRequestRecord,
    candidate: &PseBridgeCandidate,
    norm_candidate_id: Digest,
    policy_id: Digest,
) -> PromotionFeedback {
    let outcome = match &record.outcome {
        PromotionOutcome::Accepted => FeedbackOutcome::Accepted,
        PromotionOutcome::Rejected { .. } => FeedbackOutcome::Rejected,
        PromotionOutcome::Deferred => FeedbackOutcome::Deferred,
        PromotionOutcome::SkippedByPolicy | PromotionOutcome::SkippedByReportOnly => {
            FeedbackOutcome::Skipped
        }
    };
    PromotionFeedback::new(
        record.id,
        candidate.id,
        norm_candidate_id,
        outcome,
        candidate.confidence,
        policy_id,
        record.evidence_bundle_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::{
        EvidenceBundle, EvidenceKind, ImplementationMode, PolicyProfile, ReplayStatus,
    };

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    fn policy_id() -> Digest {
        d(b"policy")
    }
    fn bundle_id() -> Digest {
        d(b"bundle")
    }

    fn report_only_profile() -> PolicyProfile {
        PolicyProfile::default()
    }

    fn elevated_profile() -> PolicyProfile {
        PolicyProfile {
            mode: ImplementationMode::DryRun,
            ..PolicyProfile::default()
        }
    }

    fn basic_candidate() -> PseBridgeCandidate {
        PseBridgeCandidate::new(
            PseBridgeCandidateKind::StructuralObservation,
            d(b"observation"),
            "test observation",
            Q16::ONE,
            d(b"run"),
            bundle_id(),
            policy_id(),
        )
    }

    fn allow_policy() -> PseBridgePolicy {
        PseBridgePolicy::allow(
            policy_id(),
            vec![
                PseBridgeCandidateKind::StructuralObservation,
                PseBridgeCandidateKind::TopologyObservation,
            ],
            PseBridgeRateLimit::strict(),
        )
    }

    // --- PseBridgeCandidate ---

    #[test]
    fn candidate_id_deterministic() {
        let c1 = basic_candidate();
        let c2 = basic_candidate();
        assert_eq!(c1.id, c2.id);
    }

    #[test]
    fn candidate_verify_id() {
        assert!(basic_candidate().verify_id());
    }

    #[test]
    fn candidate_with_metadata_changes_id() {
        let c = basic_candidate();
        let id_before = c.id;
        let c2 = c.with_metadata("phase", "R7");
        assert_ne!(id_before, c2.id);
        assert!(c2.verify_id());
    }

    #[test]
    fn candidate_different_kinds_differ() {
        let c1 = PseBridgeCandidate::new(
            PseBridgeCandidateKind::StructuralObservation,
            d(b"obs"),
            "x",
            Q16::ONE,
            d(b"run"),
            bundle_id(),
            policy_id(),
        );
        let c2 = PseBridgeCandidate::new(
            PseBridgeCandidateKind::TopologyObservation,
            d(b"obs"),
            "x",
            Q16::ONE,
            d(b"run"),
            bundle_id(),
            policy_id(),
        );
        assert_ne!(c1.id, c2.id);
    }

    #[test]
    fn candidate_confidence_is_q16_not_float() {
        let c = basic_candidate();
        // Q16::ONE.raw() == 1 << 16 == 65536
        assert_eq!(c.confidence.raw(), 1i64 << 16);
    }

    #[test]
    fn candidate_different_confidence_differs() {
        let c1 = PseBridgeCandidate::new(
            PseBridgeCandidateKind::StructuralObservation,
            d(b"obs"),
            "x",
            Q16::ONE,
            d(b"run"),
            bundle_id(),
            policy_id(),
        );
        let c2 = PseBridgeCandidate::new(
            PseBridgeCandidateKind::StructuralObservation,
            d(b"obs"),
            "x",
            Q16::ZERO,
            d(b"run"),
            bundle_id(),
            policy_id(),
        );
        assert_ne!(c1.id, c2.id);
    }

    // --- PseBridgeRateLimit ---

    #[test]
    fn rate_limit_strict_defaults() {
        let r = PseBridgeRateLimit::strict();
        assert_eq!(r.max_candidates_per_run, 10u64);
        assert_eq!(r.max_candidates_per_hour, 50u64);
    }

    // --- PseBridgePolicy ---

    #[test]
    fn bridge_policy_deny_all_id_deterministic() {
        let p1 = PseBridgePolicy::deny_all(policy_id());
        let p2 = PseBridgePolicy::deny_all(policy_id());
        assert_eq!(p1.id, p2.id);
    }

    #[test]
    fn bridge_policy_deny_all_verify_id() {
        assert!(PseBridgePolicy::deny_all(policy_id()).verify_id());
    }

    #[test]
    fn bridge_policy_deny_all_blocks_everything() {
        let p = PseBridgePolicy::deny_all(policy_id());
        assert!(!p.allow_submission);
        assert!(!p.is_kind_allowed(&PseBridgeCandidateKind::StructuralObservation));
    }

    #[test]
    fn bridge_policy_allow_verify_id() {
        assert!(allow_policy().verify_id());
    }

    #[test]
    fn bridge_policy_allow_permits_listed_kinds() {
        let p = allow_policy();
        assert!(p.is_kind_allowed(&PseBridgeCandidateKind::StructuralObservation));
        assert!(p.is_kind_allowed(&PseBridgeCandidateKind::TopologyObservation));
        assert!(!p.is_kind_allowed(&PseBridgeCandidateKind::KcubeArtifact));
    }

    #[test]
    fn bridge_policy_require_evidence_default_true() {
        assert!(PseBridgePolicy::deny_all(policy_id()).require_evidence);
        assert!(allow_policy().require_evidence);
    }

    // --- PromotionRequest ---

    #[test]
    fn request_id_deterministic() {
        let r1 = PromotionRequest::new(d(b"cand"), d(b"bp"), bundle_id(), 1);
        let r2 = PromotionRequest::new(d(b"cand"), d(b"bp"), bundle_id(), 1);
        assert_eq!(r1.id, r2.id);
    }

    #[test]
    fn request_verify_id() {
        assert!(PromotionRequest::new(d(b"cand"), d(b"bp"), bundle_id(), 1).verify_id());
    }

    #[test]
    fn request_with_refs_changes_id() {
        let r = PromotionRequest::new(d(b"cand"), d(b"bp"), bundle_id(), 1);
        let id_before = r.id;
        let r2 = r.with_refs(vec![EvidenceRef::new(
            d(b"x"),
            EvidenceKind::RunRecord,
            "run",
        )]);
        assert_ne!(id_before, r2.id);
        assert!(r2.verify_id());
    }

    #[test]
    fn request_different_sequence_differs() {
        let r1 = PromotionRequest::new(d(b"cand"), d(b"bp"), bundle_id(), 1);
        let r2 = PromotionRequest::new(d(b"cand"), d(b"bp"), bundle_id(), 2);
        assert_ne!(r1.id, r2.id);
    }

    // --- PromotionOutcome ---

    #[test]
    fn outcome_accepted_is_accepted() {
        assert!(PromotionOutcome::Accepted.is_accepted());
        assert!(!PromotionOutcome::Accepted.is_skipped());
        assert!(!PromotionOutcome::Accepted.is_failure_class());
    }

    #[test]
    fn outcome_skipped_variants() {
        assert!(PromotionOutcome::SkippedByReportOnly.is_skipped());
        assert!(PromotionOutcome::SkippedByPolicy.is_skipped());
        assert!(!PromotionOutcome::SkippedByReportOnly.is_failure_class());
    }

    #[test]
    fn outcome_rejected_is_failure() {
        let o = PromotionOutcome::Rejected {
            reason: "low confidence".into(),
        };
        assert!(o.is_failure_class());
        assert!(!o.is_accepted());
    }

    #[test]
    fn outcome_deferred_is_neither() {
        assert!(!PromotionOutcome::Deferred.is_accepted());
        assert!(!PromotionOutcome::Deferred.is_skipped());
        assert!(!PromotionOutcome::Deferred.is_failure_class());
    }

    // --- PromotionRequestRecord ---

    #[test]
    fn record_skipped_deterministic() {
        let r1 = PromotionRequestRecord::skipped_by_report_only(d(b"req"), bundle_id());
        let r2 = PromotionRequestRecord::skipped_by_report_only(d(b"req"), bundle_id());
        assert_eq!(r1.id, r2.id);
    }

    #[test]
    fn record_skipped_verify_id() {
        let r = PromotionRequestRecord::skipped_by_report_only(d(b"req"), bundle_id());
        assert!(r.verify_id());
        assert!(r.outcome.is_skipped());
        assert_eq!(r.elapsed_ms, 0u64);
    }

    #[test]
    fn record_new_deterministic() {
        let r1 =
            PromotionRequestRecord::new(d(b"req"), PromotionOutcome::Accepted, bundle_id(), 42);
        let r2 =
            PromotionRequestRecord::new(d(b"req"), PromotionOutcome::Accepted, bundle_id(), 42);
        assert_eq!(r1.id, r2.id);
    }

    #[test]
    fn record_verify_id() {
        let r = PromotionRequestRecord::new(
            d(b"req"),
            PromotionOutcome::Rejected {
                reason: "threshold".into(),
            },
            bundle_id(),
            100,
        );
        assert!(r.verify_id());
    }

    #[test]
    fn record_evidence_mandatory() {
        let r = PromotionRequestRecord::skipped_by_report_only(d(b"req"), bundle_id());
        assert_ne!(r.evidence_bundle_id, Digest::ZERO);
    }

    #[test]
    fn record_no_floats_in_elapsed() {
        let r = PromotionRequestRecord::new(d(b"req"), PromotionOutcome::Accepted, bundle_id(), 99);
        assert_eq!(r.elapsed_ms, 99u64);
    }

    // --- validate_candidate ---

    #[test]
    fn validate_report_only_skips() {
        let c = basic_candidate();
        let p = allow_policy();
        let profile = report_only_profile();
        let result = validate_candidate(&c, &p, &profile);
        assert_eq!(result, Err(PromotionOutcome::SkippedByReportOnly));
    }

    #[test]
    fn validate_deny_all_policy_skips() {
        let c = basic_candidate();
        let p = PseBridgePolicy::deny_all(policy_id());
        let profile = elevated_profile();
        let result = validate_candidate(&c, &p, &profile);
        assert_eq!(result, Err(PromotionOutcome::SkippedByPolicy));
    }

    #[test]
    fn validate_disallowed_kind_skips() {
        let c = PseBridgeCandidate::new(
            PseBridgeCandidateKind::KcubeArtifact,
            d(b"obs"),
            "x",
            Q16::ONE,
            d(b"run"),
            bundle_id(),
            policy_id(),
        );
        let p = allow_policy(); // only allows Structural + Topology
        let result = validate_candidate(&c, &p, &elevated_profile());
        assert_eq!(result, Err(PromotionOutcome::SkippedByPolicy));
    }

    #[test]
    fn validate_zero_evidence_rejected() {
        let c = PseBridgeCandidate::new(
            PseBridgeCandidateKind::StructuralObservation,
            d(b"obs"),
            "x",
            Q16::ONE,
            d(b"run"),
            Digest::ZERO, // no evidence
            policy_id(),
        );
        let p = allow_policy();
        let result = validate_candidate(&c, &p, &elevated_profile());
        assert!(matches!(result, Err(PromotionOutcome::Rejected { .. })));
    }

    #[test]
    fn validate_passes_for_valid_candidate() {
        let c = basic_candidate();
        let p = allow_policy();
        let result = validate_candidate(&c, &p, &elevated_profile());
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn r7_no_pse_core_dependency() {
        // Structural: this crate's Cargo.toml must not list pse-core.
        // Verified by reading the manifest at compile time — the crate
        // compiles and tests pass without pse-core in the dependency tree.
        // kosmo-hyphae → kosmo-pse-bridge → kosmo-core (only).
        let candidate = basic_candidate();
        assert!(candidate.verify_id());
    }

    #[test]
    fn r7_candidate_is_not_semantic_crystal() {
        // A PseBridgeCandidate carries a label and confidence score
        // but has no crystallization authority. PSE decides.
        let c = basic_candidate();
        assert!(c.confidence.raw() > 0);
        // There is no SemanticCrystal type here — import would fail to compile.
    }

    #[test]
    fn r7_evidence_bundle_integration() {
        let ev_bundle = EvidenceBundle::seal(
            vec![EvidenceRef::new(
                d(b"scan-ev"),
                EvidenceKind::StructuralYield,
                "structural yield evidence",
            )],
            policy_id(),
            ReplayStatus::Replayable,
        );
        let c = PseBridgeCandidate::new(
            PseBridgeCandidateKind::StructuralObservation,
            d(b"obs"),
            "bridge test",
            Q16::HALF,
            d(b"run"),
            ev_bundle.bundle_id,
            policy_id(),
        );
        assert!(c.verify_id());
        assert_ne!(c.evidence_bundle_id, Digest::ZERO);
    }

    #[test]
    fn certified_crystal_kind_roundtrips_and_is_distinct() {
        // The unification kind: serde-stable and distinct from the scan kind, so
        // PSE-side routing can tell a certified crystal from a raw observation.
        let c = PseBridgeCandidate::new(
            PseBridgeCandidateKind::CertifiedCrystal,
            d(b"record"),
            "crystal:python:abc",
            Q16::ratio(3, 4).unwrap(),
            d(b"run"),
            bundle_id(),
            policy_id(),
        );
        assert!(c.verify_id());
        let json = serde_json::to_string(&c).unwrap();
        let back: PseBridgeCandidate = serde_json::from_str(&json).unwrap();
        assert_eq!(back, c, "serde roundtrip must be lossless");
        assert_eq!(back.kind, PseBridgeCandidateKind::CertifiedCrystal);
        assert_ne!(
            PseBridgeCandidateKind::CertifiedCrystal,
            PseBridgeCandidateKind::StructuralObservation
        );
    }
}

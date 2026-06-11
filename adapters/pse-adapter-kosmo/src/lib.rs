//! PSE domain adapter for the Kosmocrates substrate bridge — the **consumption
//! half of the substrate→core unification**.
//!
//! The substrate offers its strongest durable artifacts — certified structural
//! crystals, wrapped as [`PseBridgeCandidate`]s of kind `CertifiedCrystal` by
//! `kosmo-pipeline::crystal_to_pse_candidate` — through `kosmo-pse-bridge`, the
//! single sanctioned crossing point between the layers. This adapter is the PSE
//! side of that crossing: it canonicalizes a candidate into a PSE
//! [`Observation`] and feeds it through [`pse_core::macro_step`], where the
//! engine's own fail-closed gate decides crystallization.
//!
//! ```text
//! kosmo substrate                      │ bridge │            PSE core
//! StructuralCrystalRecord ─► PseBridgeCandidate ─► [this adapter] ─► Observation
//!                                                                     │ macro_step
//!                                                  Option<SemanticCrystal> ◄┘
//! ```
//!
//! ## Contract
//!
//! - **PSE decides.** The adapter never constructs or commits a
//!   `SemanticCrystal`; it submits observations and reports the engine's
//!   verdict as a [`PromotionOutcome`] for the substrate's feedback loop.
//! - **Fail-closed canonicalization.** Unlike sensor adapters that pass raw
//!   bytes through, this is an inter-layer crossing: payloads that do not parse
//!   as a [`PseBridgeCandidate`], fail `verify_id()` (tampered content
//!   addressing), carry zero evidence (CROSS-006), or have a kind outside the
//!   adapter's allowlist are rejected, never ingested.
//! - **Policy-gated submission.** [`offer_candidate`] runs the bridge's
//!   [`validate_candidate`] first: under a `ReportOnly` profile or a
//!   `deny_all` bridge policy nothing reaches the engine
//!   (`SkippedByReportOnly` / `SkippedByPolicy`).
//! - **The float boundary.** The substrate's `Q16` confidence becomes the
//!   observation's semantic `phase_hint` here — this crate is PSE's `f64`
//!   world, and the conversion happens exactly at this documented seam.
//!   Structurally similar crystals (similar `(ρ+ω)/2`) receive similar
//!   phases, so coherent substrate output can actually resonate.

use kosmo_core::{Digest, PolicyProfile};
use kosmo_pse_bridge::{
    validate_candidate, PromotionOutcome, PseBridgeCandidate, PseBridgeCandidateKind,
    PseBridgePolicy,
};
use pse_adapter_il::qtic::{classify, QticCertificate, QticInput, MCI_THRESHOLD};
use pse_core::GlobalState;
use pse_graph::{ObservationAdapter, ObserveError};
use pse_types::{
    content_address_raw, Config, Hash256, MeasurementContext, Observation, ProvenanceEnvelope,
    SemanticCrystal,
};

/// PSE observation adapter for substrate bridge candidates.
///
/// `allowed_kinds` is the adapter-level allowlist (fail-closed): the default
/// accepts only `CertifiedCrystal` — the substrate's gate-passed,
/// constraint-certified, replay-proofed artifact class.
pub struct KosmoBridgeAdapter {
    source: String,
    allowed_kinds: Vec<PseBridgeCandidateKind>,
}

impl KosmoBridgeAdapter {
    /// Create an adapter for the given scope (e.g. a workspace name).
    /// Accepts only `CertifiedCrystal` candidates.
    pub fn new(scope: &str) -> Self {
        Self {
            source: format!("kosmo-bridge:{}", scope),
            allowed_kinds: vec![PseBridgeCandidateKind::CertifiedCrystal],
        }
    }

    /// Widen (or narrow) the accepted candidate kinds. An empty list rejects
    /// everything — there is no "allow all" escape hatch.
    pub fn with_allowed_kinds(mut self, kinds: Vec<PseBridgeCandidateKind>) -> Self {
        self.allowed_kinds = kinds;
        self
    }

    /// Parse and gate a raw payload into a verified candidate (fail-closed).
    fn parse_candidate(&self, raw: &[u8]) -> Result<PseBridgeCandidate, ObserveError> {
        let candidate: PseBridgeCandidate = serde_json::from_slice(raw)
            .map_err(|e| ObserveError::Canonicalize(format!("not a PseBridgeCandidate: {e}")))?;
        if !candidate.verify_id() {
            return Err(ObserveError::Canonicalize(
                "candidate id does not match content — tampered or corrupted".into(),
            ));
        }
        if candidate.evidence_bundle_id == Digest::ZERO {
            return Err(ObserveError::Canonicalize(
                "evidence_bundle_id is ZERO — evidence required (CROSS-006)".into(),
            ));
        }
        if !self.allowed_kinds.contains(&candidate.kind) {
            return Err(ObserveError::Canonicalize(format!(
                "candidate kind {:?} not in adapter allowlist",
                candidate.kind
            )));
        }
        Ok(candidate)
    }
}

/// Semantic phase hint in `[0, 2π)` from the candidate's `Q16` confidence.
///
/// Confidence is the crystal's structural strength `(ρ+ω)/2`, so similar
/// crystals receive similar phases (semantic locality), letting coherent
/// substrate output produce coherent data-helix phase distributions.
fn phase_from_confidence(candidate: &PseBridgeCandidate) -> f64 {
    let unit = candidate.confidence.to_f64().clamp(0.0, 1.0);
    (unit * std::f64::consts::TAU) % std::f64::consts::TAU
}

impl ObservationAdapter for KosmoBridgeAdapter {
    fn source_id(&self) -> &str {
        &self.source
    }

    fn canonicalize(
        &self,
        raw: &[u8],
        context: &MeasurementContext,
    ) -> Result<Observation, ObserveError> {
        let candidate = self.parse_candidate(raw)?;

        // Carry the candidate's metadata (language, fingerprint_id, …) into the
        // measurement context so downstream layers can route by it.
        let mut context = context.clone();
        for (k, v) in &candidate.metadata {
            context.tags.insert(format!("kosmo.{}", k), v.clone());
        }
        context
            .tags
            .insert("kosmo.label".into(), candidate.label.clone());

        let payload = raw.to_vec();
        let digest: Hash256 = content_address_raw(&payload);
        Ok(Observation {
            timestamp: 0.0,
            source_id: self.source.clone(),
            provenance: ProvenanceEnvelope {
                origin: self.source.clone(),
                chain: Vec::new(),
                sig: None,
            },
            payload,
            context,
            digest,
            schema_version: "1.0.0".into(),
            phase_hint: Some(phase_from_confidence(&candidate)),
        })
    }
}

impl pse_core::DomainAdapter for KosmoBridgeAdapter {
    fn domain_name(&self) -> &str {
        "kosmo"
    }
}

/// The engine's verdict on one offered candidate.
#[derive(Clone, Debug)]
pub struct CrystalOffer {
    /// The offered candidate's content-addressed id.
    pub candidate_id: Digest,
    /// The promotion outcome, in the bridge's vocabulary, for the substrate's
    /// feedback loop (`build_promotion_feedback`).
    pub outcome: PromotionOutcome,
    /// The crystal PSE committed, when the engine's fail-closed gate fired.
    pub crystal: Option<SemanticCrystal>,
    /// QTIC conformance certificate for the committed crystal (Q0–Q5).
    /// `Some` exactly when `crystal` is `Some`. See [`qtic_for_promoted`] for
    /// the honest ceiling in this path.
    pub qtic: Option<QticCertificate>,
}

/// Classify a crystal committed through the promotion path into the QTIC
/// conformance hierarchy (Q0–Q5).
///
/// Classes are **earned, never granted** — the input states only what this
/// path can truthfully attest:
///
/// - `gate_passed = true`: `macro_step` returned the crystal, which means the
///   engine's Kairos gate fired — that *is* gate-passed condensation (Q3).
/// - `psi` is derived from the crystal itself
///   (`kuramoto_coherence − (1 − stability_score)`).
/// - `block_hash` and `hdag_node_id` are empty and `path_inv` is false: the
///   promotion path commits to the in-memory engine, not to the Infinity
///   Ledger, so there is no trace anchor and no HDAG path invariance.
///   `il_stability` is 0.0 for the same reason (no IL feedback signal).
///
/// Consequently a promoted crystal **caps at Q3** today; the certificate's
/// `trace_ready`/`path_inv` fields make visible exactly what an IL commit
/// would add (Q4: auditable, Q5: path-invariant). When the promotion path
/// gains an IL anchor, the same classification lifts further without changing
/// vocabulary.
pub fn qtic_for_promoted(crystal: &SemanticCrystal, extrinsic_t: u64) -> QticCertificate {
    let psi = crystal.topology_signature.kuramoto_coherence - (1.0 - crystal.stability_score);
    classify(&QticInput {
        crystal,
        block_hash: "",
        hdag_node_id: "",
        extrinsic_t,
        gate_passed: true,
        psi,
        il_stability: 0.0,
        path_inv: false,
        mci_threshold: MCI_THRESHOLD,
    })
}

/// Offer one candidate to the PSE engine under full policy gating.
///
/// Gate order (fail-closed):
/// 1. [`validate_candidate`] — `ReportOnly` profiles and denying bridge
///    policies short-circuit without touching the engine.
/// 2. The adapter's canonicalization (parse, `verify_id`, evidence, kind).
/// 3. [`pse_core::macro_step`] — the engine's own crystallization gate.
///
/// Outcome mapping: a committed crystal is `Accepted`; clean ingestion without
/// crystallization is `Deferred` (the engine's stability gate has not fired —
/// fail-closed, no memory was created); any engine or canonicalization error is
/// `Rejected` with the reason.
pub fn offer_candidate(
    state: &mut GlobalState,
    config: &Config,
    profile: &PolicyProfile,
    bridge_policy: &PseBridgePolicy,
    candidate: &PseBridgeCandidate,
    adapter: &KosmoBridgeAdapter,
) -> CrystalOffer {
    if let Err(outcome) = validate_candidate(candidate, bridge_policy, profile) {
        return CrystalOffer {
            candidate_id: candidate.id,
            outcome,
            crystal: None,
            qtic: None,
        };
    }

    let payload = match serde_json::to_vec(candidate) {
        Ok(p) => p,
        Err(e) => {
            return CrystalOffer {
                candidate_id: candidate.id,
                outcome: PromotionOutcome::Rejected {
                    reason: format!("serialization failed: {e}"),
                },
                crystal: None,
                qtic: None,
            }
        }
    };

    match pse_core::macro_step(state, &[payload], config, adapter) {
        Ok(Some(crystal)) => {
            let qtic = qtic_for_promoted(&crystal, state.commit_index);
            CrystalOffer {
                candidate_id: candidate.id,
                outcome: PromotionOutcome::Accepted,
                crystal: Some(crystal),
                qtic: Some(qtic),
            }
        }
        Ok(None) => CrystalOffer {
            candidate_id: candidate.id,
            outcome: PromotionOutcome::Deferred,
            crystal: None,
            qtic: None,
        },
        Err(e) => CrystalOffer {
            candidate_id: candidate.id,
            outcome: PromotionOutcome::Rejected {
                reason: e.to_string(),
            },
            crystal: None,
            qtic: None,
        },
    }
}

/// Offer a batch of candidates one at a time (per-candidate attribution).
/// Order is preserved; each candidate gets its own engine step and verdict.
pub fn offer_candidates(
    state: &mut GlobalState,
    config: &Config,
    profile: &PolicyProfile,
    bridge_policy: &PseBridgePolicy,
    candidates: &[PseBridgeCandidate],
    adapter: &KosmoBridgeAdapter,
) -> Vec<CrystalOffer> {
    candidates
        .iter()
        .map(|c| offer_candidate(state, config, profile, bridge_policy, c, adapter))
        .collect()
}

/// Describe a committed crystal in substrate context.
pub fn describe_crystal(crystal: &SemanticCrystal, candidate: &PseBridgeCandidate) -> String {
    format!(
        "Kosmo: {} crystallized, stability={:.4}, region={} vertices",
        candidate.label,
        crystal.stability_score,
        crystal.region.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::{ImplementationMode, Q16};
    use kosmo_pse_bridge::PseBridgeRateLimit;

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    fn certified_candidate() -> PseBridgeCandidate {
        PseBridgeCandidate::new(
            PseBridgeCandidateKind::CertifiedCrystal,
            d(b"record"),
            "crystal:python:abc123def456",
            Q16::ratio(3, 4).unwrap(),
            d(b"run"),
            d(b"evidence"),
            d(b"policy"),
        )
        .with_metadata("language", "python")
        .with_metadata("fingerprint_id", d(b"fp").to_hex())
    }

    fn allowing_bridge_policy() -> PseBridgePolicy {
        PseBridgePolicy::allow(
            d(b"policy"),
            vec![PseBridgeCandidateKind::CertifiedCrystal],
            PseBridgeRateLimit::strict(),
        )
    }

    fn execution_profile() -> PolicyProfile {
        PolicyProfile {
            mode: ImplementationMode::DryRun,
            ..PolicyProfile::default()
        }
    }

    // ── Canonicalization: fail-closed gates ─────────────────────────────────

    #[test]
    fn canonicalize_accepts_valid_certified_crystal() {
        let adapter = KosmoBridgeAdapter::new("test");
        let candidate = certified_candidate();
        let raw = serde_json::to_vec(&candidate).unwrap();
        let obs = adapter
            .canonicalize(&raw, &MeasurementContext::default())
            .expect("valid candidate must canonicalize");
        assert_eq!(obs.source_id, "kosmo-bridge:test");
        assert_eq!(
            obs.digest,
            content_address_raw(&raw),
            "payload content-addressed"
        );
        let phase = obs.phase_hint.expect("phase hint must be set");
        assert!(
            (0.0..std::f64::consts::TAU).contains(&phase),
            "phase in [0, 2π), got {phase}"
        );
        assert_eq!(
            obs.context.tags.get("kosmo.language").map(String::as_str),
            Some("python"),
            "metadata travels as context tags"
        );
        assert!(obs.context.tags.contains_key("kosmo.label"));
    }

    #[test]
    fn canonicalize_rejects_garbage_bytes() {
        let adapter = KosmoBridgeAdapter::new("test");
        let err = adapter
            .canonicalize(b"not json at all", &MeasurementContext::default())
            .unwrap_err();
        assert!(matches!(err, ObserveError::Canonicalize(_)));
    }

    #[test]
    fn canonicalize_rejects_tampered_candidate() {
        let adapter = KosmoBridgeAdapter::new("test");
        let mut candidate = certified_candidate();
        candidate.confidence = Q16::ONE; // mutate content without recomputing id
        let raw = serde_json::to_vec(&candidate).unwrap();
        let err = adapter
            .canonicalize(&raw, &MeasurementContext::default())
            .unwrap_err();
        assert!(
            matches!(err, ObserveError::Canonicalize(ref m) if m.contains("tampered")),
            "tampered candidate must be rejected: {err}"
        );
    }

    #[test]
    fn canonicalize_rejects_zero_evidence() {
        let adapter = KosmoBridgeAdapter::new("test");
        let candidate = PseBridgeCandidate::new(
            PseBridgeCandidateKind::CertifiedCrystal,
            d(b"r"),
            "crystal:-:x",
            Q16::HALF,
            d(b"run"),
            Digest::ZERO,
            d(b"policy"),
        );
        let raw = serde_json::to_vec(&candidate).unwrap();
        let err = adapter
            .canonicalize(&raw, &MeasurementContext::default())
            .unwrap_err();
        assert!(
            matches!(err, ObserveError::Canonicalize(ref m) if m.contains("CROSS-006")),
            "zero evidence must be rejected: {err}"
        );
    }

    #[test]
    fn canonicalize_rejects_disallowed_kind() {
        let adapter = KosmoBridgeAdapter::new("test"); // CertifiedCrystal only
        let candidate = PseBridgeCandidate::new(
            PseBridgeCandidateKind::StructuralObservation,
            d(b"obs"),
            "raw-scan",
            Q16::HALF,
            d(b"run"),
            d(b"ev"),
            d(b"policy"),
        );
        let raw = serde_json::to_vec(&candidate).unwrap();
        let err = adapter
            .canonicalize(&raw, &MeasurementContext::default())
            .unwrap_err();
        assert!(
            matches!(err, ObserveError::Canonicalize(ref m) if m.contains("allowlist")),
            "disallowed kind must be rejected: {err}"
        );
    }

    // ── Policy gating: nothing reaches the engine when gated ────────────────

    #[test]
    fn offer_skipped_under_report_only_profile() {
        let config = Config::default();
        let mut state = GlobalState::new(&config);
        let adapter = KosmoBridgeAdapter::new("test");
        let offer = offer_candidate(
            &mut state,
            &config,
            &PolicyProfile::default(), // ReportOnly
            &allowing_bridge_policy(),
            &certified_candidate(),
            &adapter,
        );
        assert_eq!(offer.outcome, PromotionOutcome::SkippedByReportOnly);
        assert!(offer.crystal.is_none());
        assert_eq!(state.commit_index, 0, "engine must not be touched");
    }

    #[test]
    fn offer_skipped_under_deny_all_bridge_policy() {
        let config = Config::default();
        let mut state = GlobalState::new(&config);
        let adapter = KosmoBridgeAdapter::new("test");
        let offer = offer_candidate(
            &mut state,
            &config,
            &execution_profile(),
            &PseBridgePolicy::deny_all(d(b"policy")),
            &certified_candidate(),
            &adapter,
        );
        assert_eq!(offer.outcome, PromotionOutcome::SkippedByPolicy);
        assert_eq!(state.commit_index, 0, "engine must not be touched");
    }

    // ── End-to-end: candidates flow through the real engine ─────────────────

    #[test]
    fn offer_reaches_engine_and_reports_verdict() {
        let config = Config::default();
        let mut state = GlobalState::new(&config);
        let adapter = KosmoBridgeAdapter::new("e2e");
        let bridge_policy = allowing_bridge_policy();
        let profile = execution_profile();

        // Offer a stream of distinct certified candidates; the engine ingests
        // each and crystallizes if and only if its own stability gate fires.
        let candidates: Vec<PseBridgeCandidate> = (0..50u32)
            .map(|i| {
                PseBridgeCandidate::new(
                    PseBridgeCandidateKind::CertifiedCrystal,
                    d(format!("record-{i}").as_bytes()),
                    format!("crystal:rust:{i:012}"),
                    Q16::ratio(u64::from(i % 10) + 40, 100).unwrap(),
                    d(b"run"),
                    d(b"evidence"),
                    d(b"policy"),
                )
            })
            .collect();

        let offers = offer_candidates(
            &mut state,
            &config,
            &profile,
            &bridge_policy,
            &candidates,
            &adapter,
        );

        assert_eq!(offers.len(), 50, "one verdict per candidate");
        assert!(
            state.commit_index > 0,
            "valid candidates must be ingested by the engine"
        );
        // Every outcome is an engine verdict, never a skip (policy allowed all):
        for offer in &offers {
            assert!(
                matches!(
                    offer.outcome,
                    PromotionOutcome::Accepted
                        | PromotionOutcome::Deferred
                        | PromotionOutcome::Rejected { .. }
                ),
                "policy-passed candidates get engine verdicts, got {:?}",
                offer.outcome
            );
        }
        // PSE decides: Accepted iff the engine committed a crystal.
        for offer in &offers {
            assert_eq!(
                offer.outcome.is_accepted(),
                offer.crystal.is_some(),
                "Accepted must carry the committed crystal"
            );
        }
    }

    #[test]
    fn phase_hint_has_semantic_locality() {
        // Similar confidence → similar phase; the engine can resonate on it.
        let mk = |pct: u64| {
            PseBridgeCandidate::new(
                PseBridgeCandidateKind::CertifiedCrystal,
                d(format!("r{pct}").as_bytes()),
                "crystal:-:x",
                Q16::ratio(pct, 100).unwrap(),
                d(b"run"),
                d(b"ev"),
                d(b"policy"),
            )
        };
        let close_a = phase_from_confidence(&mk(75));
        let close_b = phase_from_confidence(&mk(76));
        let far = phase_from_confidence(&mk(25));
        assert!(
            (close_a - close_b).abs() < (close_a - far).abs(),
            "nearby confidence must map to nearby phase"
        );
    }

    // ── QTIC classification of promoted crystals ─────────────────────────────

    fn make_crystal(stability: f64, kuramoto: f64) -> SemanticCrystal {
        let mut c = SemanticCrystal {
            crystal_id: [7u8; 32],
            region: vec![],
            constraint_program: Default::default(),
            stability_score: stability,
            topology_signature: Default::default(),
            betti_numbers: vec![],
            evidence_chain: Default::default(),
            commit_proof: Default::default(),
            operator_versions: Default::default(),
            created_at: 1,
            free_energy: 0.0,
            carrier_instance_idx: 0,
            scale_tag: "test".to_string(),
            universe_id: String::new(),
            sub_crystal_ids: vec![],
            parent_crystal_ids: vec![],
            genesis_metadata: None,
            metatron_signature: None,
        };
        c.topology_signature.kuramoto_coherence = kuramoto;
        c
    }

    #[test]
    fn promoted_crystal_classifies_q3_and_never_higher() {
        use pse_adapter_il::qtic::QticClass;
        // A strong committed crystal: stable, coherent — the engine's gate
        // fired (that's what commitment means), so Q3 is earned. But the
        // promotion path has no IL trace anchor and no HDAG path invariance,
        // so Q4/Q5 must NOT be granted, and the certificate shows why.
        let crystal = make_crystal(0.9, 0.8);
        let cert = qtic_for_promoted(&crystal, 42);
        assert_eq!(
            cert.conformance_class,
            QticClass::Q3,
            "gate-passed condensation"
        );
        assert!(!cert.trace_ready, "no IL block hash → not trace-ready");
        assert!(!cert.path_inv, "no HDAG → no path invariance");
        assert!(cert.replay_ready, "non-zero crystal id is replay-ready");
        assert!(cert.gate_passed);
        assert_eq!(cert.extrinsic_t, 42);
    }

    #[test]
    fn weak_promoted_crystal_classifies_below_q3() {
        use pse_adapter_il::qtic::QticClass;
        // ψ = kuramoto − (1 − stability) = 0.0 − 1.0 = −1.0 < −0.1: the
        // intrinsic phase is not stable, so Q2 is not earned and the class
        // honestly stays at Q1 despite the gate having fired.
        let crystal = make_crystal(0.0, 0.0);
        let cert = qtic_for_promoted(&crystal, 1);
        assert_eq!(
            cert.conformance_class,
            QticClass::Q1,
            "unstable phase caps at Q1"
        );
    }

    #[test]
    fn offer_carries_qtic_iff_crystal() {
        // The structural invariant on the verdict: a certificate exists exactly
        // when a crystal was committed — never for Deferred/Rejected/Skipped.
        let config = Config::default();
        let mut state = GlobalState::new(&config);
        let adapter = KosmoBridgeAdapter::new("qtic");
        let offers = offer_candidates(
            &mut state,
            &config,
            &execution_profile(),
            &allowing_bridge_policy(),
            &[certified_candidate()],
            &adapter,
        );
        for offer in &offers {
            assert_eq!(
                offer.qtic.is_some(),
                offer.crystal.is_some(),
                "QTIC certificate exactly when a crystal was committed"
            );
        }
        // And the skip path never classifies:
        let skipped = offer_candidate(
            &mut state,
            &config,
            &PolicyProfile::default(), // ReportOnly
            &allowing_bridge_policy(),
            &certified_candidate(),
            &adapter,
        );
        assert!(skipped.qtic.is_none());
    }
}

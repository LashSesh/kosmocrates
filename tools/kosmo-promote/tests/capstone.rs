//! Capstone: the complete substrate→core path in one test.
//!
//! Crosses every layer of the unification in-process, with no host writes and
//! no network: a gate-passed structural yield is certified into a crystal
//! (kosmo-hyphae), carries a cross-language fingerprint (xlang), is wrapped as
//! a bridge candidate (kosmo-pipeline), and is offered to the real PSE engine
//! (pse-adapter-kosmo → pse-core::macro_step), which alone decides
//! crystallization.

use kosmo_core::{
    AuthorityLabel, Digest, EvidenceBundle, EvidenceKind, EvidenceRef, ImplementationMode,
    PolicyProfile, ReplayStatus, TaintLabel, Q16,
};
use kosmo_hyphae::{
    AssimilationDecision, CrossLanguageFingerprint, GateCascade, SourceLanguage,
    StructuralCrystalCandidate, StructuralYield, StructuralYieldKind,
};
use kosmo_pipeline::crystal_to_pse_candidate;
use kosmo_pse_bridge::{
    PromotionOutcome, PseBridgeCandidateKind, PseBridgePolicy, PseBridgeRateLimit,
};
use pse_adapter_kosmo::{offer_candidates, KosmoBridgeAdapter};
use pse_core::GlobalState;
use pse_types::Config;

/// One certified crystal from a clean-taint yield, fingerprinted in `language`.
fn certified_crystal_candidate(
    seed: &str,
    language: SourceLanguage,
    source: &str,
) -> kosmo_pse_bridge::PseBridgeCandidate {
    let policy = PolicyProfile::default_report_only();

    // Layer 1 — evidence-bound structural yield, gate cascade, decision.
    let ev = EvidenceBundle::seal(
        vec![EvidenceRef::new(
            Digest::of_bytes(seed.as_bytes()),
            EvidenceKind::HostScan,
            "capstone scan",
        )],
        policy.id,
        ReplayStatus::Replayable,
    );
    let yield_ = StructuralYield::new(
        StructuralYieldKind::DeficiencyFill,
        Some(Digest::of_bytes(format!("void-{seed}").as_bytes())),
        None,
        TaintLabel::Clean,
        AuthorityLabel::Foundry,
        ev.bundle_id,
        policy.id,
    );
    let trace = GateCascade::standard_gates(policy.clone()).apply(&yield_, &ev);
    let decision = AssimilationDecision::from_trace(&yield_, &trace, &ev, policy.id);
    assert!(
        decision.outcome.is_accepted(),
        "clean yield must pass all gates"
    );

    // Layer 2 — crystal candidate with a cross-language fingerprint, certified.
    let fingerprint = CrossLanguageFingerprint::from_source(
        language,
        Digest::of_bytes(format!("src-{seed}").as_bytes()),
        source,
    );
    let candidate = StructuralCrystalCandidate::from_decision_with_signals(
        &decision,
        Some(Digest::of_bytes(format!("void-{seed}").as_bytes())),
        Q16::ratio(3, 4).unwrap(),
        Q16::ONE,
    )
    .with_fingerprint(fingerprint);
    let (_cert, record) = candidate
        .certify(ReplayStatus::Replayable)
        .expect("pending candidate with full constraints must certify");
    assert!(record.verify_id(), "record is content-addressed");
    assert!(
        record.fingerprint.is_some(),
        "fingerprint travels into the record"
    );
    assert_ne!(
        record.evidence_bundle_id,
        Digest::ZERO,
        "record is directly evidence-bound (CROSS-006)"
    );

    // Layer 3 — wrap for the bridge (the offer side of the unification).
    crystal_to_pse_candidate(&record, Digest::of_bytes(b"capstone-run"), policy.id)
}

#[test]
fn substrate_crystals_flow_into_the_pse_engine() {
    // Crystals from three languages — same structural shape, distinct sources.
    let candidates = vec![
        certified_crystal_candidate(
            "py",
            SourceLanguage::Python,
            "import os\ndef a(x):\n    return x\ndef b(y):\n    return y\n",
        ),
        certified_crystal_candidate(
            "go",
            SourceLanguage::Go,
            "package main\nimport \"fmt\"\nfunc a(n int) int { return n }\nfunc b(n int) int { return n }\n",
        ),
        certified_crystal_candidate(
            "rs",
            SourceLanguage::Rust,
            "use std::io;\npub fn a(x: u32) -> u32 { x }\npub fn b(y: u32) -> u32 { y }\n",
        ),
    ];
    for c in &candidates {
        assert_eq!(c.kind, PseBridgeCandidateKind::CertifiedCrystal);
        assert!(c.verify_id());
        assert!(
            c.metadata.contains_key("language"),
            "fingerprint metadata present"
        );
    }

    // Layer 4 — offer to the real engine under an execution profile.
    let profile = PolicyProfile {
        mode: ImplementationMode::DryRun,
        ..PolicyProfile::default()
    };
    let bridge_policy = PseBridgePolicy::allow(
        Digest::of_bytes(b"capstone-policy"),
        vec![PseBridgeCandidateKind::CertifiedCrystal],
        PseBridgeRateLimit::strict(),
    );
    let adapter = KosmoBridgeAdapter::new("capstone");
    let config = Config::default();
    let mut state = GlobalState::new(&config);

    let offers = offer_candidates(
        &mut state,
        &config,
        &profile,
        &bridge_policy,
        &candidates,
        &adapter,
    );

    // The engine was really fed: every candidate got an engine verdict and the
    // observations were committed. PSE alone decides whether anything
    // crystallizes; both Accepted and Deferred are honest verdicts here, a skip
    // is not.
    assert_eq!(offers.len(), 3, "one verdict per crystal");
    assert!(state.commit_index > 0, "engine ingested the candidates");
    for offer in &offers {
        assert!(
            matches!(
                offer.outcome,
                PromotionOutcome::Accepted
                    | PromotionOutcome::Deferred
                    | PromotionOutcome::Rejected { .. }
            ),
            "policy-passed crystal must reach the engine, got {:?}",
            offer.outcome
        );
        assert_eq!(offer.outcome.is_accepted(), offer.crystal.is_some());
    }
}

#[test]
fn capstone_is_fail_closed_under_report_only() {
    // The same crystals under the default ReportOnly profile: the engine is
    // never touched. Production default = no promotion without an operator.
    let candidates = vec![certified_crystal_candidate(
        "ro",
        SourceLanguage::Python,
        "import os\ndef a(x):\n    return x\n",
    )];
    let bridge_policy = PseBridgePolicy::allow(
        Digest::of_bytes(b"capstone-policy"),
        vec![PseBridgeCandidateKind::CertifiedCrystal],
        PseBridgeRateLimit::strict(),
    );
    let adapter = KosmoBridgeAdapter::new("capstone");
    let config = Config::default();
    let mut state = GlobalState::new(&config);

    let offers = offer_candidates(
        &mut state,
        &config,
        &PolicyProfile::default(), // ReportOnly
        &bridge_policy,
        &candidates,
        &adapter,
    );

    assert_eq!(offers.len(), 1);
    assert_eq!(offers[0].outcome, PromotionOutcome::SkippedByReportOnly);
    assert_eq!(state.commit_index, 0, "engine must stay untouched");
}

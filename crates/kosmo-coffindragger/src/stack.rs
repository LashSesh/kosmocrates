//! ASCC — the Attractor Stack Closure Calculus (spec ch. 8, Listing 20.1):
//! canonical embedding, support-preserving accretion, contractive consolidation,
//! the fold, and the closure run. Cleaning is never silent forgetting.

use kosmo_cdk_core::{qsr_stack, Canonical, Stage, StageEmbeddingCertificate, Status};
use kosmo_core::Digest;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Certify a canonical stage embedding (Def. 8.1, ASCC-1). Valid iff it preserves
/// canonical structure (`canonical_eq`) and semantic load (`semantic_eq`) and is
/// evidence-bound (CROSS-006). Without this certificate the successor is merely a
/// new version, not a valid assimilation stage (negative test 2 — an embedding
/// that changes the predecessor's canonical structure yields no certificate).
pub fn certify_embedding(
    from: &Stage,
    to: &Stage,
    canonical_eq: bool,
    semantic_eq: bool,
    residue_report: Digest,
    evidence: Digest,
) -> Option<StageEmbeddingCertificate> {
    if !canonical_eq || !semantic_eq || evidence == Digest::ZERO {
        return None;
    }
    let mut cert = StageEmbeddingCertificate {
        certificate_id: Digest::ZERO,
        from_stage: from.canon(),
        to_stage: to.canon(),
        canonical_equivalence: canonical_eq,
        semantic_equivalence: semantic_eq,
        residue_report_id: residue_report,
        evidence_bundle_id: evidence,
    };
    cert.certificate_id = Digest::of(&(
        cert.from_stage,
        cert.to_stage,
        cert.residue_report_id,
        cert.evidence_bundle_id,
    ));
    Some(cert)
}

/// Verify support-preserving accretion (Def. 8.3 + Contract 8.4, ASCC-2): every
/// unit that left the support MUST appear in the exclusion set `Xn` or the residue
/// `Rn` — `Support(Sn) \ Support(Sn+1) ⊆ Xn ∪ Rn`. No covert removal (negative
/// test 1: a successor deletes bearing units without a residue entry).
pub fn verify_accretion(
    prev_support: &[Digest],
    new_support: &[Digest],
    exclusions: &[Digest],
    residue: &[Digest],
) -> Status {
    let kept: HashSet<&Digest> = new_support.iter().collect();
    let accounted: HashSet<&Digest> = exclusions.iter().chain(residue.iter()).collect();
    for u in prev_support {
        if !kept.contains(u) && !accounted.contains(u) {
            return Status::Reject; // silent forgetting → boundary violation (I3)
        }
    }
    Status::Pass
}

/// Verify contractive consolidation (Def. 8.5, ASCC-3): cost (here `|support|`) is
/// non-increasing, and any dropped unit is accounted as residue (semantic load
/// preserved or explicitly recorded). Negative test 5 — contraction that removes
/// bearing units without recording them fails.
pub fn verify_contraction(before: &[Digest], after: &[Digest], residue: &[Digest]) -> Status {
    if after.len() > before.len() {
        return Status::Reject; // not a contraction — cost rose
    }
    verify_accretion(before, after, &[], residue)
}

/// A fold bundle `B_N` (Def. 8.7): a content-addressed, evidence-bound structural
/// bundle — *not* a text summary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoldBundle {
    pub bundle_id: Digest,
    pub stage_ids: Vec<Digest>,
    pub support: Vec<Digest>,
    pub residue: Vec<Digest>,
}

impl FoldBundle {
    /// `FoldN: StackN → B_N` (Def. 8.7) — content-addressed over the stages'
    /// canonical ids, the support, and the residue.
    pub fn fold(stages: &[Stage], support: Vec<Digest>, residue: Vec<Digest>) -> Self {
        let stage_ids: Vec<Digest> = stages.iter().map(|s| s.canon()).collect();
        let bundle_id = Digest::of(&(&stage_ids, &support, &residue));
        Self {
            bundle_id,
            stage_ids,
            support,
            residue,
        }
    }
}

/// Stack closure condition (Def. 8.8, ASCC-4): closed iff the gate passes, replay
/// holds, and `Canon(B_N) = Canon(S_N)` — the fold preserves the canonical
/// identity of the last stage (StackClosureGate; negative test 6 — folded though
/// gate or replay is missing).
pub fn stack_closed(bundle: &FoldBundle, last_stage: &Stage, gate: Status, replay: bool) -> Status {
    if !matches!(gate, Status::Pass | Status::Warn) || !replay {
        return Status::Reject;
    }
    if !bundle.stage_ids.contains(&last_stage.canon()) {
        return Status::Reject; // Canon(B_N) ≠ Canon(S_N)
    }
    Status::Pass
}

/// The outcome of an Attractor Stack Closure run (Listing 20.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClosureOutcome {
    /// QSR passed and the stack is closed — a diamond candidate.
    DiamondCandidate(FoldBundle),
    /// QSR failed or the stack is not closed — deferred, residue made visible.
    DeferredStackReport { bundle: FoldBundle, residue: Vec<Digest> },
}

/// The Attractor Stack Closure run (Listing 20.1, ASCC-4/5): fold the stages,
/// check stack-QSR and the closure condition, and emit a `DiamondCandidate` or a
/// `DeferredStackReport`. Fail-closed — a missing gate/replay or a QSR failure
/// defers, it never forces (negative test 11: a DiamondCube without a QSR pass).
pub fn close_stack(
    stages: &[Stage],
    support: Vec<Digest>,
    residue: Vec<Digest>,
    replay: bool,
) -> ClosureOutcome {
    let bundle = FoldBundle::fold(stages, support, residue.clone());
    let qsr = qsr_stack(stages);
    let closed = stages
        .last()
        .map(|last| stack_closed(&bundle, last, qsr, replay))
        .unwrap_or(Status::Reject);
    if matches!(qsr, Status::Pass | Status::Warn) && matches!(closed, Status::Pass | Status::Warn) {
        ClosureOutcome::DiamondCandidate(bundle)
    } else {
        ClosureOutcome::DeferredStackReport { bundle, residue }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_cdk_core::StageMetrics;
    use kosmo_core::Q16;

    fn stage(units: &[&[u8]], density: Q16, contr: Q16) -> Stage {
        Stage {
            attractor: Digest::ZERO,
            units: units.iter().map(|u| Digest::of_bytes(u)).collect(),
            evidence: Digest::of_bytes(b"ev"),
            gate_status: Status::Pass,
            trace_root: Digest::of_bytes(b"tr"),
            residue: Digest::ZERO,
            metrics: StageMetrics {
                density,
                purity: Q16::HALF,
                irreducibility: Q16::HALF,
                curvature: Q16::ZERO,
                contradiction_energy: contr,
            },
            certificates: vec![],
        }
    }

    fn d(s: &[u8]) -> Digest {
        Digest::of_bytes(s)
    }

    #[test]
    fn an_embedding_that_changes_canonical_structure_yields_no_certificate() {
        let a = stage(&[b"u1"], Q16::HALF, Q16::HALF);
        let b = stage(&[b"u1", b"u2"], Q16::HALF, Q16::HALF);
        let ev = Digest::of_bytes(b"ev");
        assert!(certify_embedding(&a, &b, true, true, Digest::ZERO, ev).is_some());
        // canonical structure changed → not a valid assimilation stage.
        assert!(certify_embedding(&a, &b, false, true, Digest::ZERO, ev).is_none());
        // evidence-free → no certificate (CROSS-006).
        assert!(certify_embedding(&a, &b, true, true, Digest::ZERO, Digest::ZERO).is_none());
    }

    #[test]
    fn covert_removal_fails_accretion_but_accounted_removal_passes() {
        let prev = [d(b"a"), d(b"b"), d(b"c")];
        // c dropped silently → reject (negative test 1).
        assert_eq!(verify_accretion(&prev, &[d(b"a"), d(b"b")], &[], &[]), Status::Reject);
        // c dropped but recorded as residue → pass (cleaning is visible).
        assert_eq!(verify_accretion(&prev, &[d(b"a"), d(b"b")], &[], &[d(b"c")]), Status::Pass);
        // c moved to exclusions → pass.
        assert_eq!(verify_accretion(&prev, &[d(b"a"), d(b"b")], &[d(b"c")], &[]), Status::Pass);
    }

    #[test]
    fn contraction_must_not_grow_and_must_account_for_loss() {
        let before = [d(b"a"), d(b"b"), d(b"c")];
        assert_eq!(verify_contraction(&before, &[d(b"a"), d(b"b")], &[d(b"c")]), Status::Pass);
        // grew → not a contraction.
        assert_eq!(verify_contraction(&before, &[d(b"a"), d(b"b"), d(b"c"), d(b"x")], &[]), Status::Reject);
        // shrank but dropped unit unaccounted → reject (negative test 5).
        assert_eq!(verify_contraction(&before, &[d(b"a")], &[d(b"b")]), Status::Reject);
    }

    #[test]
    fn a_qsr_monotone_closed_stack_emits_a_diamond_candidate() {
        // Density rising, contradiction falling, all gated → stack-QSR passes.
        let s0 = stage(&[b"a"], Q16::ratio(1, 4).unwrap(), Q16::HALF);
        let s1 = stage(&[b"a", b"b"], Q16::HALF, Q16::ratio(1, 4).unwrap());
        let stages = [s0, s1];
        match close_stack(&stages, vec![d(b"a"), d(b"b")], vec![], true) {
            ClosureOutcome::DiamondCandidate(b) => {
                assert!(b.stage_ids.contains(&stages[1].canon()), "Canon(B_N) = Canon(S_N)");
            }
            other => panic!("expected a diamond candidate, got {other:?}"),
        }
    }

    #[test]
    fn a_qsr_regressing_stack_is_deferred_not_forced() {
        // Density falls across stages → stack-QSR rejects → deferred (fail-closed,
        // negative test 11: no DiamondCube without a QSR pass).
        let s0 = stage(&[b"a"], Q16::HALF, Q16::ZERO);
        let s1 = stage(&[b"a"], Q16::ratio(1, 4).unwrap(), Q16::HALF); // density down, contr up
        match close_stack(&[s0, s1], vec![d(b"a")], vec![], true) {
            ClosureOutcome::DeferredStackReport { .. } => {}
            other => panic!("expected a deferred report, got {other:?}"),
        }
    }

    #[test]
    fn a_stack_folded_without_replay_is_not_closed() {
        // Negative test 6 — folded though replay is missing.
        let s0 = stage(&[b"a"], Q16::HALF, Q16::ZERO);
        let bundle = FoldBundle::fold(std::slice::from_ref(&s0), vec![d(b"a")], vec![]);
        assert_eq!(stack_closed(&bundle, &s0, Status::Pass, false), Status::Reject);
    }
}

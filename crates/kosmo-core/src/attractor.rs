//! Attractor — the wish as a fixed point the system converges onto.
//!
//! Run 1 ([`crate::wish`]) gave intent a *measurable distance*. This module
//! treats that distance as the dynamics of a system descending toward a fixed
//! point: the **wish is the attractor `x*`**, and the wish distance `V` is a
//! **Lyapunov function** with `V ≥ 0` and `V = 0` only at `x*` (the wish
//! realized).
//!
//! A trajectory of distances across iterations **converges** iff `V` is
//! monotone non-increasing and reaches `ZERO`. This is the intent-axis analogue
//! of LPCM's [`monotone_contractive_filter`](crate) doctrine: a step that
//! *increases* the distance is a contraction violation (a regression away from
//! the attractor) and is flagged fail-closed — the agent loop must reject the
//! patch that caused it rather than accept a move in the wrong direction.
//!
//! Finite convergence: `V` lives in `Q16`, a *discrete* lattice. Over the unit
//! interval there are only [`MAX_STRICT_CONTRACTION_STEPS`] distinct values, so
//! any *strictly* contracting trajectory reaches the attractor in a finite,
//! bounded number of steps — the convergence claim is not asymptotic hand-waving
//! but a counting argument.
//!
//! Like every selection quantity in the substrate, this **ranks, it never
//! gates**: the trace describes how a workspace is moving, but grants no
//! capability and bypasses no policy.

use crate::digest::Digest;
use crate::fixed_point::{Q16, Q16_SCALE};
use crate::wish::WishAssessment;
use serde::{Deserialize, Serialize};

/// The maximum length of a *strictly* contracting trajectory over the unit
/// interval `[ZERO, ONE]`.
///
/// `Q16` has `Q16_SCALE` (65536) fractional steps; including both endpoints
/// there are `65537` distinct representable values in `[0, 1]`. A strictly
/// decreasing sequence of distances can therefore visit at most this many
/// values before it must reach `ZERO` — a finite, machine-checkable bound on
/// convergence time.
pub const MAX_STRICT_CONTRACTION_STEPS: u32 = Q16_SCALE as u32 + 1;

/// One transition between two consecutive distances in a trajectory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConvergenceStep {
    /// `V` strictly decreased: the workspace moved toward the attractor.
    Contracting,
    /// `V` unchanged: no progress this step (plateau).
    Stalled,
    /// `V` increased: a regression away from the attractor — a contraction
    /// violation that the loop must reject (fail-closed).
    Diverging,
}

/// Overall convergence state of a trajectory toward a wish-attractor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttractorStatus {
    /// Reached `V == ZERO` without ever diverging: the wish is realized and the
    /// system sits at its fixed point.
    Converged,
    /// Never diverged, not yet at `ZERO`, and the latest step decreased `V`:
    /// descending toward the attractor.
    Converging,
    /// Never diverged, not at `ZERO`, and the latest step made no progress:
    /// stuck on a plateau.
    Stalled,
    /// At least one step increased `V`: the monotone-contraction invariant was
    /// violated. `first_divergence` on the trace carries the offending index.
    Diverging,
    /// Fewer than one distance recorded — no trajectory to classify.
    Indeterminate,
}

impl AttractorStatus {
    pub fn is_converged(&self) -> bool {
        matches!(self, Self::Converged)
    }
    /// True unless the trajectory diverged: the contraction invariant holds.
    pub fn is_contractive(&self) -> bool {
        !matches!(self, Self::Diverging)
    }
}

/// Derive the overall status and the first divergence index from a distance
/// sequence (oldest first).
fn derive_status(distances: &[Q16]) -> (AttractorStatus, Option<u32>) {
    if distances.is_empty() {
        return (AttractorStatus::Indeterminate, None);
    }
    // First step that increased V (a contraction violation).
    for i in 1..distances.len() {
        if distances[i] > distances[i - 1] {
            return (AttractorStatus::Diverging, Some(i as u32));
        }
    }
    let last = distances[distances.len() - 1];
    if last.is_zero() {
        return (AttractorStatus::Converged, None);
    }
    if distances.len() == 1 {
        // A single positive sample: in progress, no regression seen.
        return (AttractorStatus::Converging, None);
    }
    let prev = distances[distances.len() - 2];
    if last < prev {
        (AttractorStatus::Converging, None)
    } else {
        // last == prev (cannot be greater: no divergence above)
        (AttractorStatus::Stalled, None)
    }
}

/// Content for the deterministic `WishConvergenceTrace` id.
#[derive(Serialize)]
struct TraceContent<'a> {
    wish_id: &'a Digest,
    distances_raw: &'a Vec<i64>,
    status: &'a AttractorStatus,
    first_divergence: Option<u32>,
    evidence_bundle_id: &'a Digest,
}

/// A content-addressed record of a workspace's trajectory toward a wish-attractor.
///
/// `distances` is the Lyapunov sequence `V_0, V_1, …` (oldest first), one entry
/// per iteration of the agent loop. `status` and `first_divergence` are derived
/// from it.
///
/// Invariants:
/// - `id = SHA-256(JCS(content))` — INVARIANT-007
/// - `evidence_bundle_id ≠ ZERO` — CROSS-006
/// - the contraction invariant (`V` non-increasing) holds iff
///   `status.is_contractive()`; a violating step is recorded in
///   `first_divergence` (the LPCM `SpuriousExpansion` analogue)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WishConvergenceTrace {
    pub id: Digest,
    /// Identity of the attractor `x*` (the wish this trajectory descends toward).
    pub wish_id: Digest,
    /// `V` at each iteration, oldest first.
    pub distances: Vec<Q16>,
    pub status: AttractorStatus,
    /// Index into `distances` of the first step that increased `V`, if any.
    pub first_divergence: Option<u32>,
    pub evidence_bundle_id: Digest,
}

impl WishConvergenceTrace {
    /// Build a trace from a raw distance sequence (oldest first).
    pub fn new(wish_id: Digest, distances: Vec<Q16>, evidence_bundle_id: Digest) -> Self {
        let (status, first_divergence) = derive_status(&distances);
        let mut t = Self {
            id: Digest::ZERO,
            wish_id,
            distances,
            status,
            first_divergence,
            evidence_bundle_id,
        };
        t.id = t.compute_id();
        t
    }

    /// Build a trace from a slice of [`WishAssessment`]s (oldest first).
    ///
    /// All assessments must concern the same wish. A mixed-wish slice describes
    /// no single attractor and yields an empty, `Indeterminate` trace
    /// (fail-closed) rather than a misleading trajectory.
    pub fn from_assessments(assessments: &[WishAssessment], evidence_bundle_id: Digest) -> Self {
        let Some(first) = assessments.first() else {
            return Self::new(Digest::ZERO, vec![], evidence_bundle_id);
        };
        let wish_id = first.wish_id;
        if assessments.iter().any(|a| a.wish_id != wish_id) {
            return Self::new(Digest::ZERO, vec![], evidence_bundle_id);
        }
        let distances = assessments.iter().map(|a| a.distance).collect();
        Self::new(wish_id, distances, evidence_bundle_id)
    }

    /// Return a new trace with `next_distance` appended — one more iteration of
    /// the dynamics. Immutable: the id is recomputed.
    pub fn extended(&self, next_distance: Q16, evidence_bundle_id: Digest) -> Self {
        let mut distances = self.distances.clone();
        distances.push(next_distance);
        Self::new(self.wish_id, distances, evidence_bundle_id)
    }

    fn compute_id(&self) -> Digest {
        let distances_raw: Vec<i64> = self.distances.iter().map(|q| q.raw()).collect();
        Digest::of(&TraceContent {
            wish_id: &self.wish_id,
            distances_raw: &distances_raw,
            status: &self.status,
            first_divergence: self.first_divergence,
            evidence_bundle_id: &self.evidence_bundle_id,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }

    /// CROSS-006.
    pub fn is_evidence_bound(&self) -> bool {
        self.evidence_bundle_id != Digest::ZERO
    }

    /// The system is *at* the attractor: the most recent distance is `ZERO`.
    pub fn at_attractor(&self) -> bool {
        self.distances.last().is_some_and(|d| d.is_zero())
    }

    /// The contraction invariant holds (no step increased `V`).
    pub fn is_contractive(&self) -> bool {
        self.status.is_contractive()
    }

    pub fn is_converged(&self) -> bool {
        self.status.is_converged()
    }

    /// The most recent Lyapunov value, if any.
    pub fn latest_distance(&self) -> Option<Q16> {
        self.distances.last().copied()
    }

    /// Per-transition classification (one entry per consecutive pair).
    pub fn step_classifications(&self) -> Vec<ConvergenceStep> {
        self.distances
            .windows(2)
            .map(|w| {
                if w[1] > w[0] {
                    ConvergenceStep::Diverging
                } else if w[1] < w[0] {
                    ConvergenceStep::Contracting
                } else {
                    ConvergenceStep::Stalled
                }
            })
            .collect()
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wish::{assess_wish, ObservedTopology, Wish, WishFacet, WishPredicate};

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    fn wish_id() -> Digest {
        d(b"wish")
    }

    fn ev() -> Digest {
        d(b"trace-evidence")
    }

    fn q(n: u64, den: u64) -> Q16 {
        Q16::ratio(n, den).unwrap()
    }

    // ── content addressing ────────────────────────────────────────────────

    #[test]
    fn trace_id_deterministic() {
        let a = WishConvergenceTrace::new(wish_id(), vec![Q16::ONE, Q16::HALF], ev());
        let b = WishConvergenceTrace::new(wish_id(), vec![Q16::ONE, Q16::HALF], ev());
        assert_eq!(a.id, b.id);
    }

    #[test]
    fn trace_verify_id() {
        let t = WishConvergenceTrace::new(wish_id(), vec![Q16::ONE, Q16::ZERO], ev());
        assert!(t.verify_id());
    }

    #[test]
    fn trace_evidence_mandatory() {
        let t = WishConvergenceTrace::new(wish_id(), vec![Q16::ONE], ev());
        assert!(t.is_evidence_bound());
        assert_ne!(t.evidence_bundle_id, Digest::ZERO);
    }

    // ── status derivation ─────────────────────────────────────────────────

    #[test]
    fn empty_is_indeterminate() {
        let t = WishConvergenceTrace::new(wish_id(), vec![], ev());
        assert_eq!(t.status, AttractorStatus::Indeterminate);
        assert!(!t.at_attractor());
    }

    #[test]
    fn single_zero_is_converged() {
        let t = WishConvergenceTrace::new(wish_id(), vec![Q16::ZERO], ev());
        assert_eq!(t.status, AttractorStatus::Converged);
        assert!(t.at_attractor());
    }

    #[test]
    fn single_positive_is_converging() {
        let t = WishConvergenceTrace::new(wish_id(), vec![Q16::HALF], ev());
        assert_eq!(t.status, AttractorStatus::Converging);
        assert!(!t.at_attractor());
    }

    #[test]
    fn strictly_decreasing_to_zero_is_converged() {
        let t = WishConvergenceTrace::new(
            wish_id(),
            vec![Q16::ONE, Q16::HALF, q(1, 4), Q16::ZERO],
            ev(),
        );
        assert_eq!(t.status, AttractorStatus::Converged);
        assert!(t.is_contractive());
        assert!(t.at_attractor());
        assert!(t.first_divergence.is_none());
    }

    #[test]
    fn decreasing_not_zero_is_converging() {
        let t = WishConvergenceTrace::new(wish_id(), vec![Q16::ONE, Q16::HALF], ev());
        assert_eq!(t.status, AttractorStatus::Converging);
        assert!(t.is_contractive());
    }

    #[test]
    fn plateau_is_stalled() {
        let t = WishConvergenceTrace::new(wish_id(), vec![Q16::HALF, Q16::HALF], ev());
        assert_eq!(t.status, AttractorStatus::Stalled);
        assert!(t.is_contractive(), "a plateau does not violate contraction");
    }

    #[test]
    fn increase_is_diverging_with_first_index() {
        let t = WishConvergenceTrace::new(wish_id(), vec![Q16::HALF, q(3, 4)], ev());
        assert_eq!(t.status, AttractorStatus::Diverging);
        assert_eq!(t.first_divergence, Some(1));
        assert!(!t.is_contractive());
    }

    #[test]
    fn divergence_anywhere_flags_first_even_if_later_reaches_zero() {
        // Down, then UP (violation at index 2), then to zero. A regression at
        // any point is fail-closed regardless of the eventual endpoint.
        let t = WishConvergenceTrace::new(
            wish_id(),
            vec![Q16::ONE, Q16::HALF, q(3, 4), Q16::ZERO],
            ev(),
        );
        assert_eq!(t.status, AttractorStatus::Diverging);
        assert_eq!(t.first_divergence, Some(2));
    }

    // ── step classifications ──────────────────────────────────────────────

    #[test]
    fn step_classifications_label_each_transition() {
        let t = WishConvergenceTrace::new(
            wish_id(),
            vec![Q16::ONE, Q16::HALF, Q16::HALF, q(3, 4)],
            ev(),
        );
        assert_eq!(
            t.step_classifications(),
            vec![
                ConvergenceStep::Contracting,
                ConvergenceStep::Stalled,
                ConvergenceStep::Diverging,
            ]
        );
    }

    // ── extension (one more iteration) ────────────────────────────────────

    #[test]
    fn extended_appends_and_recomputes_id() {
        let t0 = WishConvergenceTrace::new(wish_id(), vec![Q16::ONE], ev());
        let t1 = t0.extended(Q16::HALF, ev());
        assert_eq!(t1.distances, vec![Q16::ONE, Q16::HALF]);
        assert_ne!(t0.id, t1.id);
        assert!(t1.verify_id());
        assert_eq!(t1.status, AttractorStatus::Converging);
    }

    #[test]
    fn fixed_point_is_stable_under_extension() {
        // Once at the attractor (V == ZERO), a further iteration that stays at
        // ZERO keeps the trace Converged — f(x*) = x*.
        let converged = WishConvergenceTrace::new(wish_id(), vec![Q16::HALF, Q16::ZERO], ev());
        assert!(converged.is_converged());
        let still = converged.extended(Q16::ZERO, ev());
        assert!(still.is_converged());
        assert!(still.at_attractor());
        assert!(still.is_contractive());
    }

    // ── from_assessments integration with Run 1 ───────────────────────────

    fn sample_wish() -> Wish {
        Wish::new(
            "two things",
            [
                WishPredicate::require(WishFacet::crate_("a")),
                WishPredicate::require(WishFacet::crate_("b")),
            ],
            d(b"policy"),
            d(b"bundle"),
        )
    }

    #[test]
    fn from_assessments_builds_converging_trajectory() {
        let w = sample_wish();
        let a0 = assess_wish(&w, &ObservedTopology::empty(), ev());
        let a1 = assess_wish(&w, &ObservedTopology::from_facets([WishFacet::crate_("a")]), ev());
        let a2 = assess_wish(
            &w,
            &ObservedTopology::from_facets([WishFacet::crate_("a"), WishFacet::crate_("b")]),
            ev(),
        );
        let trace = WishConvergenceTrace::from_assessments(&[a0, a1, a2], ev());
        assert_eq!(trace.wish_id, w.id);
        assert_eq!(trace.distances, vec![Q16::ONE, Q16::HALF, Q16::ZERO]);
        assert_eq!(trace.status, AttractorStatus::Converged);
        assert!(trace.is_contractive());
    }

    #[test]
    fn from_assessments_mixed_wish_is_indeterminate() {
        let wa = Wish::new("a", [WishPredicate::require(WishFacet::crate_("a"))], d(b"p"), d(b"e"));
        let wb = Wish::new("b", [WishPredicate::require(WishFacet::crate_("b"))], d(b"p"), d(b"e"));
        let aa = assess_wish(&wa, &ObservedTopology::empty(), ev());
        let ab = assess_wish(&wb, &ObservedTopology::empty(), ev());
        let trace = WishConvergenceTrace::from_assessments(&[aa, ab], ev());
        assert_eq!(trace.status, AttractorStatus::Indeterminate);
        assert!(trace.distances.is_empty());
        assert_eq!(trace.wish_id, Digest::ZERO);
    }

    #[test]
    fn from_assessments_empty_is_indeterminate() {
        let trace = WishConvergenceTrace::from_assessments(&[], ev());
        assert_eq!(trace.status, AttractorStatus::Indeterminate);
    }

    // ── finite-convergence bound ──────────────────────────────────────────

    #[test]
    fn max_strict_contraction_steps_is_q16_resolution_plus_one() {
        assert_eq!(MAX_STRICT_CONTRACTION_STEPS, 65537);
    }
}

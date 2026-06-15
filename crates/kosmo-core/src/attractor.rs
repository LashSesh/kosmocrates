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
use crate::wish::{WishAssessment, WishCube, WishLayer};
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

// ─── Run 3: the per-layer (tensor) descent ──────────────────────────────────────

/// One stratum's Lyapunov sequence within a layered descent (oldest first), with
/// its own derived status — the per-layer analogue of [`WishConvergenceTrace`].
///
/// `distances[i] = ONE − (layer opacity at iteration i)`, so the same
/// contraction doctrine applies one stratum at a time: a *rising* layer distance
/// is a fail-closed regression of that layer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerTrajectory {
    pub layer: WishLayer,
    /// Per-iteration layer distance `V_layer`, oldest first.
    pub distances: Vec<Q16>,
    pub status: AttractorStatus,
    /// Index into `distances` of the first step that increased `V_layer`, if any.
    pub first_divergence: Option<u32>,
}

/// A render-order violation a *flat* scalar trajectory cannot see. Every variant
/// is a ranking / warning signal (CROSS-010), never a gate: it informs the loop,
/// it never un-meets a facet or bypasses a policy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RenderAnomaly {
    /// A deeper stratum is solid while a shallower non-empty stratum is not —
    /// sintered before debind. The wish "set" out of order (suspect an over-fit
    /// shell). Detected on the most recent cube.
    SetOutOfOrder {
        deeper: WishLayer,
        ungrounded_below: WishLayer,
    },
    /// On a step where the *flat* opacity rose or held (the scalar trace looks
    /// healthy), a deeper stratum's opacity fell — a deep regression the
    /// headline number masked. Carries the stratum and the step index.
    MaskedDeepRegression { layer: WishLayer, step: u32 },
}

/// Content for the deterministic per-layer trajectory id fragment.
#[derive(Serialize)]
struct LayerTrajectoryContent<'a> {
    rank: u8,
    distances_raw: Vec<i64>,
    status: &'a AttractorStatus,
    first_divergence: Option<u32>,
}

/// Content for the deterministic `LayeredConvergenceTrace` id.
#[derive(Serialize)]
struct LayeredTraceContent<'a> {
    wish_id: &'a Digest,
    layers: Vec<LayerTrajectoryContent<'a>>,
    anomalies: &'a Vec<RenderAnomaly>,
    evidence_bundle_id: &'a Digest,
}

/// A content-addressed record of a workspace's **per-layer** trajectory toward a
/// wish-attractor — the precision instrument the scalar [`WishConvergenceTrace`]
/// cannot be. Built from a sequence of [`WishCube`]s (oldest first), it runs the
/// same contraction doctrine ([`derive_status`]) one stratum at a time and flags
/// the regressions a single scalar masks.
///
/// Invariants:
/// - `id = SHA-256(JCS(content))` — INVARIANT-007
/// - `evidence_bundle_id ≠ ZERO` — CROSS-006
/// - the flat trace remains the headline; this refines it, and it gates nothing
///   (CROSS-010)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayeredConvergenceTrace {
    pub id: Digest,
    /// Identity of the attractor `x*` (the wish this trajectory descends toward).
    pub wish_id: Digest,
    /// One trajectory per **non-empty** stratum, ascending by rank.
    pub layers: Vec<LayerTrajectory>,
    /// Render-order anomalies found across the descent (deterministically ordered).
    pub anomalies: Vec<RenderAnomaly>,
    pub evidence_bundle_id: Digest,
}

impl LayeredConvergenceTrace {
    /// Build the per-layer trace from a descent film of cubes (oldest first).
    ///
    /// All cubes must concern the same wish; a mixed-wish slice describes no
    /// single attractor and yields an empty, anomaly-free trace (fail-closed),
    /// mirroring [`WishConvergenceTrace::from_assessments`].
    pub fn from_cubes(cubes: &[WishCube], evidence_bundle_id: Digest) -> Self {
        let Some(first) = cubes.first() else {
            return Self::seal(Digest::ZERO, vec![], vec![], evidence_bundle_id);
        };
        let wish_id = first.wish_id;
        if cubes.iter().any(|c| c.wish_id != wish_id) {
            return Self::seal(Digest::ZERO, vec![], vec![], evidence_bundle_id);
        }

        // One trajectory per non-empty stratum (the wish is fixed across the
        // descent, so emptiness is invariant — read it from the first cube).
        let mut layers: Vec<LayerTrajectory> = Vec::new();
        for view in &first.layers {
            if view.is_empty_layer() {
                continue;
            }
            let layer = view.layer;
            let distances: Vec<Q16> = cubes
                .iter()
                .map(|c| {
                    let op = c
                        .layer(layer)
                        .map(|l| l.opacity)
                        .unwrap_or(Q16::ZERO);
                    Q16::ONE.saturating_sub(op)
                })
                .collect();
            let (status, first_divergence) = derive_status(&distances);
            layers.push(LayerTrajectory {
                layer,
                distances,
                status,
                first_divergence,
            });
        }

        let anomalies = Self::detect_anomalies(cubes);
        Self::seal(wish_id, layers, anomalies, evidence_bundle_id)
    }

    /// Masked deep regressions (across the descent) + set-out-of-order (on the
    /// latest cube). Deterministically ordered: regressions by step then rank,
    /// then structural anomalies by deeper rank.
    fn detect_anomalies(cubes: &[WishCube]) -> Vec<RenderAnomaly> {
        let mut out: Vec<RenderAnomaly> = Vec::new();

        // Masked deep regression: the flat opacity did not worsen, yet a layer's
        // opacity strictly fell.
        for i in 1..cubes.len() {
            let prev = &cubes[i - 1];
            let cur = &cubes[i];
            if cur.overall_opacity < prev.overall_opacity {
                continue; // the scalar already shows a regression — not masked
            }
            for view in &cur.layers {
                if view.is_empty_layer() {
                    continue;
                }
                let prev_op = prev.layer(view.layer).map(|l| l.opacity).unwrap_or(Q16::ZERO);
                if view.opacity < prev_op {
                    out.push(RenderAnomaly::MaskedDeepRegression {
                        layer: view.layer,
                        step: i as u32,
                    });
                }
            }
        }

        // Set-out-of-order on the latest cube: a solid stratum floating above the
        // shallowest still-hollow non-empty stratum.
        if let Some(last) = cubes.last() {
            let first_gap = last
                .layers
                .iter()
                .find(|l| !l.is_empty_layer() && !l.is_solid())
                .map(|l| l.layer);
            if let Some(gap) = first_gap {
                for view in &last.layers {
                    if !view.is_empty_layer() && view.is_solid() && view.layer.rank() > gap.rank() {
                        out.push(RenderAnomaly::SetOutOfOrder {
                            deeper: view.layer,
                            ungrounded_below: gap,
                        });
                    }
                }
            }
        }

        out
    }

    fn seal(
        wish_id: Digest,
        layers: Vec<LayerTrajectory>,
        anomalies: Vec<RenderAnomaly>,
        evidence_bundle_id: Digest,
    ) -> Self {
        let mut t = Self {
            id: Digest::ZERO,
            wish_id,
            layers,
            anomalies,
            evidence_bundle_id,
        };
        t.id = t.compute_id();
        t
    }

    fn compute_id(&self) -> Digest {
        let layers: Vec<LayerTrajectoryContent> = self
            .layers
            .iter()
            .map(|l| LayerTrajectoryContent {
                rank: l.layer.rank(),
                distances_raw: l.distances.iter().map(|q| q.raw()).collect(),
                status: &l.status,
                first_divergence: l.first_divergence,
            })
            .collect();
        Digest::of(&LayeredTraceContent {
            wish_id: &self.wish_id,
            layers,
            anomalies: &self.anomalies,
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

    /// A strictly stronger contraction claim than the scalar trace alone: every
    /// stratum is contractive **and** no render anomaly was found. A masked deep
    /// regression makes its stratum `Diverging`, so this is `false` exactly when
    /// the layered view caught something the headline number hid.
    pub fn is_strictly_contractive(&self) -> bool {
        !self.layers.is_empty()
            && self.layers.iter().all(|l| l.status.is_contractive())
            && self.anomalies.is_empty()
    }

    /// The deepest stratum of the *contiguous converged base*: the highest layer
    /// such that it and every (non-empty) stratum below it has `Converged`
    /// (fully solid and never regressed). The frontier the staged pipeline climbs.
    pub fn solid_frontier(&self) -> Option<WishLayer> {
        let mut frontier = None;
        for l in &self.layers {
            if l.status.is_converged() {
                frontier = Some(l.layer);
            } else {
                break;
            }
        }
        frontier
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
        let a1 = assess_wish(
            &w,
            &ObservedTopology::from_facets([WishFacet::crate_("a")]),
            ev(),
        );
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
        let wa = Wish::new(
            "a",
            [WishPredicate::require(WishFacet::crate_("a"))],
            d(b"p"),
            d(b"e"),
        );
        let wb = Wish::new(
            "b",
            [WishPredicate::require(WishFacet::crate_("b"))],
            d(b"p"),
            d(b"e"),
        );
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

    // ── Run 3: LayeredConvergenceTrace ─────────────────────────────────────

    use crate::wish::{assess_wish_layered, WishFacetKind};

    /// A wish spanning Existence (crate `a`) + Wiring (a contract).
    fn two_layer_wish() -> Wish {
        Wish::new(
            "two strata",
            [
                WishPredicate::require(WishFacet::crate_("a")),
                WishPredicate::require(WishFacet::new(WishFacetKind::Contract, "f(A)->B")),
            ],
            d(b"p"),
            ev(),
        )
    }

    /// Two existence crates + one wiring contract — for masked-regression tests.
    fn masked_wish() -> Wish {
        Wish::new(
            "masked",
            [
                WishPredicate::require(WishFacet::crate_("a")),
                WishPredicate::require(WishFacet::crate_("b")),
                WishPredicate::require(WishFacet::new(WishFacetKind::Contract, "f(A)->B")),
            ],
            d(b"p"),
            ev(),
        )
    }

    fn cube_at(w: &Wish, facets: &[WishFacet]) -> WishCube {
        assess_wish_layered(
            w,
            &ObservedTopology::from_facets(facets.iter().cloned()),
            ev(),
        )
    }

    #[test]
    fn layered_trace_from_cubes_tracks_each_layer_independently() {
        let w = two_layer_wish();
        let contract = WishFacet::new(WishFacetKind::Contract, "f(A)->B");
        let c0 = cube_at(&w, &[]);
        let c1 = cube_at(&w, &[WishFacet::crate_("a")]);
        let c2 = cube_at(&w, &[WishFacet::crate_("a"), contract]);
        let trace = LayeredConvergenceTrace::from_cubes(&[c0, c1, c2], ev());

        assert_eq!(trace.layers.len(), 2, "only the two non-empty strata");
        let existence = trace
            .layers
            .iter()
            .find(|l| l.layer == WishLayer::Existence)
            .unwrap();
        assert_eq!(existence.distances, vec![Q16::ONE, Q16::ZERO, Q16::ZERO]);
        assert_eq!(existence.status, AttractorStatus::Converged);
        let wiring = trace
            .layers
            .iter()
            .find(|l| l.layer == WishLayer::Wiring)
            .unwrap();
        assert_eq!(wiring.distances, vec![Q16::ONE, Q16::ONE, Q16::ZERO]);
        assert_eq!(wiring.status, AttractorStatus::Converged);

        assert!(trace.is_strictly_contractive());
        assert!(trace.anomalies.is_empty());
        assert_eq!(trace.solid_frontier(), Some(WishLayer::Wiring));
    }

    #[test]
    fn masked_deep_regression_is_detected_when_scalar_improves() {
        let w = masked_wish();
        let contract = WishFacet::new(WishFacetKind::Contract, "f(A)->B");
        // c0: wiring solid, existence hollow. c1: existence solid, wiring LOST.
        let c0 = cube_at(&w, &[contract]);
        let c1 = cube_at(&w, &[WishFacet::crate_("a"), WishFacet::crate_("b")]);
        assert!(
            c1.overall_opacity > c0.overall_opacity,
            "the flat opacity rises 1/3 → 2/3"
        );
        let scalar_distances = vec![
            Q16::ONE.saturating_sub(c0.overall_opacity),
            Q16::ONE.saturating_sub(c1.overall_opacity),
        ];
        let cubes = vec![c0, c1];
        let trace = LayeredConvergenceTrace::from_cubes(&cubes, ev());

        assert!(
            trace.anomalies.contains(&RenderAnomaly::MaskedDeepRegression {
                layer: WishLayer::Wiring,
                step: 1,
            }),
            "the layered view catches the deep regression"
        );
        assert!(!trace.is_strictly_contractive());

        // The SCALAR trace over the very same run is fooled into 'Converging'.
        let scalar = WishConvergenceTrace::new(cubes[0].wish_id, scalar_distances, ev());
        assert_eq!(
            scalar.status,
            AttractorStatus::Converging,
            "a single number cannot see the masked regression"
        );
    }

    #[test]
    fn set_out_of_order_when_deep_solid_over_hollow_shallow() {
        let w = two_layer_wish();
        let contract = WishFacet::new(WishFacetKind::Contract, "f(A)->B");
        let c = cube_at(&w, &[contract]); // wiring solid, existence hollow
        let trace = LayeredConvergenceTrace::from_cubes(&[c], ev());
        assert!(trace.anomalies.contains(&RenderAnomaly::SetOutOfOrder {
            deeper: WishLayer::Wiring,
            ungrounded_below: WishLayer::Existence,
        }));
    }

    #[test]
    fn solid_frontier_stops_at_first_unconverged_layer() {
        let w = two_layer_wish();
        let c0 = cube_at(&w, &[]);
        let c1 = cube_at(&w, &[WishFacet::crate_("a")]); // existence solid, wiring never met
        let trace = LayeredConvergenceTrace::from_cubes(&[c0, c1], ev());
        assert_eq!(trace.solid_frontier(), Some(WishLayer::Existence));
    }

    #[test]
    fn empty_layered_trace_is_not_strictly_contractive() {
        let trace = LayeredConvergenceTrace::from_cubes(&[], ev());
        assert!(trace.layers.is_empty());
        assert!(!trace.is_strictly_contractive());
        assert_eq!(trace.wish_id, Digest::ZERO);
    }

    #[test]
    fn layered_trace_mixed_wish_is_empty() {
        let wa = two_layer_wish();
        let wb = masked_wish();
        let trace = LayeredConvergenceTrace::from_cubes(&[cube_at(&wa, &[]), cube_at(&wb, &[])], ev());
        assert!(trace.layers.is_empty());
        assert_eq!(trace.wish_id, Digest::ZERO);
        assert!(trace.anomalies.is_empty());
    }

    #[test]
    fn layered_trace_id_deterministic_and_verifies() {
        let w = two_layer_wish();
        let cubes = vec![cube_at(&w, &[]), cube_at(&w, &[WishFacet::crate_("a")])];
        let t1 = LayeredConvergenceTrace::from_cubes(&cubes, ev());
        let t2 = LayeredConvergenceTrace::from_cubes(&cubes, ev());
        assert_eq!(t1.id, t2.id);
        assert!(t1.verify_id());
    }

    #[test]
    fn layered_trace_evidence_mandatory() {
        let w = two_layer_wish();
        let trace = LayeredConvergenceTrace::from_cubes(&[cube_at(&w, &[])], ev());
        assert!(trace.is_evidence_bound());
        assert_ne!(trace.evidence_bundle_id, Digest::ZERO);
    }
}

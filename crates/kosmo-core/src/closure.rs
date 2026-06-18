//! Run 4 — the staged closure pipeline.
//!
//! Run 1 ([`crate::wish`]) gave intent a measurable distance; Run 2
//! ([`crate::attractor`]) made the descent a convergence contract; Run 3
//! ([`crate::wish::WishCube`], [`crate::attractor::LayeredConvergenceTrace`])
//! turned the flat scalar into a *layered* render whose opacity rises stratum by
//! stratum. This module drives that render as the PSP **Staged Closure
//! Pipeline** — `Solve → Gate → Coagula` — applied one stratum at a time,
//! bottom-up: the wish is the *same workpiece* passed through successive
//! machines (print → debind → sinter), and each stratum *coagulates*
//! (solidifies into the diamond) only once it is rendered, contractive, and
//! evidence-bound.
//!
//! Doctrine (CROSS-010): the staging **orders effort** and **refuses
//! regressions**; it never *prevents* a deeper facet from counting if synthesis
//! produces it, and it grants no capability. A facet met out of order is still
//! met — it is only *flagged* (a [`RenderAnomaly`]). The pipeline ranks and
//! rejects-regressions; it authorises nothing.

use crate::attractor::{LayeredConvergenceTrace, RenderAnomaly};
use crate::digest::Digest;
use crate::fixed_point::Q16;
use crate::wish::{WishCube, WishLayer};
use serde::{Deserialize, Serialize};

/// One stage of the staged closure pipeline (PSP `Solve → Gate → Coagula`),
/// applied per render stratum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClosureStage {
    /// Synthesize toward this stratum's unmet facets (the curriculum step).
    Solve,
    /// Measure: did the stratum hold (contractive, evidence-bound) without a
    /// render-order violation? A failing gate rejects the patch (fail-closed).
    Gate,
    /// The stratum reached opacity `ONE` and is frozen — the solid diamond layer.
    Coagula,
}

impl ClosureStage {
    pub fn label(self) -> &'static str {
        match self {
            ClosureStage::Solve => "solve",
            ClosureStage::Gate => "gate",
            ClosureStage::Coagula => "coagula",
        }
    }
}

/// The closure state of one render stratum at the end of a (partial) descent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StratumClosure {
    /// The print head has not reached this stratum: nothing rendered yet.
    Pending,
    /// Rendering: opacity is rising but below `ONE`, and the gate has not held.
    Solving { opacity: Q16 },
    /// The gate holds — contractive and evidence-bound — but opacity `< ONE`.
    Gated { opacity: Q16 },
    /// Opacity `== ONE`, evidence-bound, frozen. The massive, solid layer.
    Coagulated,
    /// The gate failed: this stratum regressed or set out of order (fail-closed).
    Fractured { anomaly: RenderAnomaly },
}

impl StratumClosure {
    pub fn is_coagulated(&self) -> bool {
        matches!(self, StratumClosure::Coagulated)
    }
    pub fn is_fractured(&self) -> bool {
        matches!(self, StratumClosure::Fractured { .. })
    }
    /// The pipeline stage this state sits in.
    pub fn stage(&self) -> ClosureStage {
        match self {
            StratumClosure::Pending | StratumClosure::Solving { .. } => ClosureStage::Solve,
            StratumClosure::Gated { .. } | StratumClosure::Fractured { .. } => ClosureStage::Gate,
            StratumClosure::Coagulated => ClosureStage::Coagula,
        }
    }
    /// A stable lowercase tag for rendering and audit.
    pub fn label(&self) -> &'static str {
        match self {
            StratumClosure::Pending => "pending",
            StratumClosure::Solving { .. } => "solving",
            StratumClosure::Gated { .. } => "gated",
            StratumClosure::Coagulated => "coagulated",
            StratumClosure::Fractured { .. } => "fractured",
        }
    }
}

/// Content for the deterministic `StagedClosureReport` id.
#[derive(Serialize)]
struct StagedReportContent<'a> {
    wish_id: &'a Digest,
    strata: &'a Vec<(WishLayer, StratumClosure)>,
    frontier: &'a Option<WishLayer>,
    fully_coagulated: bool,
    evidence_bundle_id: &'a Digest,
}

/// A content-addressed record of driving one wish through the staged closure
/// pipeline, stratum by stratum — the Run-4 artifact.
///
/// Folded purely from a descent film of [`WishCube`]s and its
/// [`LayeredConvergenceTrace`]: it reads, for every non-empty stratum, whether
/// that layer is still solving, has gated, has coagulated, or fractured. Only
/// non-empty strata appear — an empty stratum asks nothing and never blocks
/// (`empty_stratum_is_skipped_not_blocking`).
///
/// Invariants:
/// - `id = SHA-256(JCS(content))` — INVARIANT-007
/// - `evidence_bundle_id ≠ ZERO` — CROSS-006
/// - it ranks and refuses-regressions; it gates no capability — CROSS-010
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StagedClosureReport {
    pub id: Digest,
    pub wish_id: Digest,
    /// One entry per non-empty stratum, ascending by [`WishLayer::rank`].
    pub strata: Vec<(WishLayer, StratumClosure)>,
    /// The deepest stratum of the contiguous coagulated base (the print height).
    pub frontier: Option<WishLayer>,
    /// Every non-empty stratum has coagulated: the diamond is fully cut.
    pub fully_coagulated: bool,
    pub evidence_bundle_id: Digest,
}

impl StagedClosureReport {
    /// Fold a descent film (`cubes`, oldest → newest) and its layered `trace`
    /// into the per-stratum closure state. Pure and deterministic; testable
    /// without any workspace.
    pub fn from_descent(
        cubes: &[WishCube],
        trace: &LayeredConvergenceTrace,
        evidence_bundle_id: Digest,
    ) -> Self {
        let Some(latest) = cubes.last() else {
            return Self::seal(Digest::ZERO, vec![], None, false, evidence_bundle_id);
        };
        let evidence_bound = evidence_bundle_id != Digest::ZERO;

        let mut strata: Vec<(WishLayer, StratumClosure)> = Vec::new();
        for view in &latest.layers {
            if view.is_empty_layer() {
                continue; // nothing asked here — skip, never block
            }
            let layer = view.layer;

            // A render anomaly naming this stratum fractures it (gate refused it).
            let anomaly = trace
                .anomalies
                .iter()
                .find(|a| match a {
                    RenderAnomaly::MaskedDeepRegression { layer: l, .. } => *l == layer,
                    RenderAnomaly::SetOutOfOrder { deeper, .. } => *deeper == layer,
                })
                .cloned();

            // Whether this layer's trajectory never regressed (contractive).
            let contractive = trace
                .layers
                .iter()
                .find(|t| t.layer == layer)
                .map(|t| t.status.is_contractive())
                .unwrap_or(true);

            let state = if let Some(a) = anomaly {
                StratumClosure::Fractured { anomaly: a }
            } else if view.is_solid() && evidence_bound {
                StratumClosure::Coagulated
            } else if view.met_count == 0 {
                StratumClosure::Pending
            } else if contractive && evidence_bound {
                StratumClosure::Gated {
                    opacity: view.opacity,
                }
            } else {
                StratumClosure::Solving {
                    opacity: view.opacity,
                }
            };
            strata.push((layer, state));
        }

        // The contiguous coagulated base, bottom-up.
        let mut frontier = None;
        for (layer, state) in &strata {
            if state.is_coagulated() {
                frontier = Some(*layer);
            } else {
                break;
            }
        }
        let fully_coagulated =
            !strata.is_empty() && strata.iter().all(|(_, s)| s.is_coagulated());

        Self::seal(
            latest.wish_id,
            strata,
            frontier,
            fully_coagulated,
            evidence_bundle_id,
        )
    }

    fn seal(
        wish_id: Digest,
        strata: Vec<(WishLayer, StratumClosure)>,
        frontier: Option<WishLayer>,
        fully_coagulated: bool,
        evidence_bundle_id: Digest,
    ) -> Self {
        let mut r = Self {
            id: Digest::ZERO,
            wish_id,
            strata,
            frontier,
            fully_coagulated,
            evidence_bundle_id,
        };
        r.id = r.compute_id();
        r
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&StagedReportContent {
            wish_id: &self.wish_id,
            strata: &self.strata,
            frontier: &self.frontier,
            fully_coagulated: self.fully_coagulated,
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

    /// The closure state of a given stratum, if it carries predicates.
    pub fn stratum(&self, layer: WishLayer) -> Option<&StratumClosure> {
        self.strata
            .iter()
            .find(|(l, _)| *l == layer)
            .map(|(_, s)| s)
    }

    /// Any stratum fractured — a gate refused a patch this descent?
    pub fn has_fracture(&self) -> bool {
        self.strata.iter().any(|(_, s)| s.is_fractured())
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wish::{
        assess_wish_layered, ObservedTopology, Wish, WishFacet, WishFacetKind, WishPredicate,
    };

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }
    fn ev() -> Digest {
        d(b"closure-evidence")
    }

    /// Existence (crate `a`) + Wiring (a contract); Shape/Verified/Live empty.
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

    fn cube_at(w: &Wish, facets: &[WishFacet]) -> WishCube {
        assess_wish_layered(
            w,
            &ObservedTopology::from_facets(facets.iter().cloned()),
            ev(),
        )
    }

    fn contract() -> WishFacet {
        WishFacet::new(WishFacetKind::Contract, "f(A)->B")
    }

    #[test]
    fn staged_report_coagulates_bottom_up_under_clean_descent() {
        let w = two_layer_wish();
        let c0 = cube_at(&w, &[]);
        let c1 = cube_at(&w, &[WishFacet::crate_("a")]);
        let c2 = cube_at(&w, &[WishFacet::crate_("a"), contract()]);
        let cubes = vec![c0, c1, c2];
        let trace = LayeredConvergenceTrace::from_cubes(&cubes, ev());
        let report = StagedClosureReport::from_descent(&cubes, &trace, ev());

        assert_eq!(report.strata.len(), 2, "only the two non-empty strata");
        assert!(report.stratum(WishLayer::Existence).unwrap().is_coagulated());
        assert!(report.stratum(WishLayer::Wiring).unwrap().is_coagulated());
        assert!(report.fully_coagulated);
        assert_eq!(report.frontier, Some(WishLayer::Wiring));
        assert!(!report.has_fracture());
    }

    #[test]
    fn frontier_is_partial_when_only_base_is_solid() {
        let w = two_layer_wish();
        let cubes = vec![cube_at(&w, &[]), cube_at(&w, &[WishFacet::crate_("a")])];
        let trace = LayeredConvergenceTrace::from_cubes(&cubes, ev());
        let report = StagedClosureReport::from_descent(&cubes, &trace, ev());

        assert!(report.stratum(WishLayer::Existence).unwrap().is_coagulated());
        assert_eq!(
            report.stratum(WishLayer::Wiring),
            Some(&StratumClosure::Pending)
        );
        assert_eq!(report.frontier, Some(WishLayer::Existence));
        assert!(!report.fully_coagulated);
    }

    #[test]
    fn gate_rejects_a_masked_deep_regression() {
        let w = Wish::new(
            "masked",
            [
                WishPredicate::require(WishFacet::crate_("a")),
                WishPredicate::require(WishFacet::crate_("b")),
                WishPredicate::require(contract()),
            ],
            d(b"p"),
            ev(),
        );
        // c0: wiring solid, existence hollow. c1: existence solid, wiring LOST,
        // while the flat opacity rises — a masked deep regression.
        let cubes = vec![
            cube_at(&w, &[contract()]),
            cube_at(&w, &[WishFacet::crate_("a"), WishFacet::crate_("b")]),
        ];
        let trace = LayeredConvergenceTrace::from_cubes(&cubes, ev());
        let report = StagedClosureReport::from_descent(&cubes, &trace, ev());

        assert!(report.has_fracture());
        assert!(report.stratum(WishLayer::Wiring).unwrap().is_fractured());
        assert!(!report.fully_coagulated);
    }

    #[test]
    fn fractured_stratum_on_set_out_of_order() {
        let w = two_layer_wish();
        let cubes = vec![cube_at(&w, &[contract()])]; // wiring solid over hollow existence
        let trace = LayeredConvergenceTrace::from_cubes(&cubes, ev());
        let report = StagedClosureReport::from_descent(&cubes, &trace, ev());

        assert!(report.stratum(WishLayer::Wiring).unwrap().is_fractured());
        assert_eq!(
            report.stratum(WishLayer::Existence),
            Some(&StratumClosure::Pending)
        );
        assert!(!report.fully_coagulated);
        assert_eq!(report.frontier, None);
    }

    #[test]
    fn empty_stratum_is_skipped_not_blocking() {
        // Shape/Verified/Live are empty in two_layer_wish; they must not appear
        // and must not prevent full coagulation.
        let w = two_layer_wish();
        let cubes = vec![cube_at(&w, &[WishFacet::crate_("a"), contract()])];
        let trace = LayeredConvergenceTrace::from_cubes(&cubes, ev());
        let report = StagedClosureReport::from_descent(&cubes, &trace, ev());

        assert_eq!(report.strata.len(), 2, "only non-empty strata");
        assert!(report.stratum(WishLayer::Shape).is_none());
        assert!(report.fully_coagulated);
    }

    #[test]
    fn fully_coagulated_only_when_every_nonempty_stratum_solid() {
        let w = two_layer_wish();
        let cubes = vec![cube_at(&w, &[WishFacet::crate_("a")])]; // wiring missing
        let trace = LayeredConvergenceTrace::from_cubes(&cubes, ev());
        let report = StagedClosureReport::from_descent(&cubes, &trace, ev());
        assert!(!report.fully_coagulated);
    }

    #[test]
    fn empty_descent_is_failclosed() {
        let trace = LayeredConvergenceTrace::from_cubes(&[], ev());
        let report = StagedClosureReport::from_descent(&[], &trace, ev());
        assert!(report.strata.is_empty());
        assert!(!report.fully_coagulated);
        assert_eq!(report.wish_id, Digest::ZERO);
        assert_eq!(report.frontier, None);
    }

    #[test]
    fn staged_report_id_deterministic_and_verifies() {
        let w = two_layer_wish();
        let cubes = vec![cube_at(&w, &[WishFacet::crate_("a")])];
        let trace = LayeredConvergenceTrace::from_cubes(&cubes, ev());
        let r1 = StagedClosureReport::from_descent(&cubes, &trace, ev());
        let r2 = StagedClosureReport::from_descent(&cubes, &trace, ev());
        assert_eq!(r1.id, r2.id);
        assert!(r1.verify_id());
    }

    #[test]
    fn staged_report_evidence_mandatory() {
        let w = two_layer_wish();
        let cubes = vec![cube_at(&w, &[])];
        let trace = LayeredConvergenceTrace::from_cubes(&cubes, ev());
        let report = StagedClosureReport::from_descent(&cubes, &trace, ev());
        assert!(report.is_evidence_bound());
        assert_ne!(report.evidence_bundle_id, Digest::ZERO);
    }

    #[test]
    fn stage_progression_labels() {
        assert_eq!(StratumClosure::Pending.stage(), ClosureStage::Solve);
        assert_eq!(StratumClosure::Coagulated.stage(), ClosureStage::Coagula);
        assert_eq!(
            StratumClosure::Gated { opacity: Q16::HALF }.stage(),
            ClosureStage::Gate
        );
    }
}

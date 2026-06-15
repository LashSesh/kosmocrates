//! Run 5 — the convergence lens in facet space (the descent-side Konus).
//!
//! Run 4's staged pipeline ([`crate::closure`]) picks *which stratum* to
//! solidify next. This picks *which facet within it* to realize first: the one
//! the most other unmet facets depend on — the load-bearing seed before its
//! branches (a crate before its modules; both endpoints before a dependency
//! edge; a symbol before the contract that names it). It focuses the Solve
//! stage onto the foundation, so the descent stops trying to hang a symbol in a
//! crate that does not yet exist.
//!
//! The *candidate-space* Konus — focusing `n` oracle perspectives into one via a
//! geomean of tripolar scores — already lives in `kosmo-synthesizer::consensus`
//! (the Ophanim/Konus/Monolith ensemble). This is its *facet-space* twin: the
//! same convergence principle, applied to which target to realize rather than
//! which candidate to keep. Both are CROSS-010 lenses — they **order and focus**
//! effort, they never forbid a facet or un-meet one.

use crate::digest::Digest;
use crate::fixed_point::Q16;
use crate::wish::WishFacet;
use serde::{Deserialize, Serialize};

/// One facet's leverage: how many other unmet facets depend on it (so realizing
/// it first unblocks the most), plus a `Q16` score (the depended-on fraction).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FacetLeverage {
    pub facet: WishFacet,
    pub leverage: u32,
    /// `leverage / |unmet|` in `Q16` — the fraction of the set that depends on it.
    pub score: Q16,
}

/// True iff `g` depends on `f` — `f` is a *prerequisite* of `g`, so `f` should
/// be realized first. Two deterministic, string-structural relations:
///
/// - **path containment**: `g.key` is inside `f`'s namespace (`foo` ◁ `foo::bar`
///   ◁ `foo::bar::baz`).
/// - **structural endpoint**: `f.key` is a whole token of an *edge/signature*
///   key — the `from`/`to` of a `from->to` dependency, the symbols of a
///   `from>>via>>to` composition, the name of a `name(..)->R` contract. Only
///   keys that actually encode an edge are tokenized, so a bare path like
///   `foo::bar` never spuriously "depends on" a bare symbol `bar`.
pub fn depends_on(g: &WishFacet, f: &WishFacet) -> bool {
    if g == f || f.key.is_empty() {
        return false;
    }
    // Path containment: g lives inside f's namespace.
    if g.key.starts_with(&format!("{}::", f.key)) {
        return true;
    }
    // Structural-edge endpoints: tokenize only keys that encode an edge.
    const EDGE_MARKERS: [&str; 4] = ["->", ">>", "=>", "("];
    if EDGE_MARKERS.iter().any(|m| g.key.contains(m)) {
        const SEPS: &[char] = &['-', '>', '<', '(', ')', ',', ' ', '/', ':', '=', '~'];
        return g
            .key
            .split(|c| SEPS.contains(&c))
            .any(|t| !t.is_empty() && t == f.key);
    }
    false
}

/// Content for the deterministic `PrecedenceOrder` id.
#[derive(Serialize)]
struct LevContent<'a> {
    facet: &'a WishFacet,
    leverage: u32,
    score_raw: i64,
}

#[derive(Serialize)]
struct PrecedenceContent<'a> {
    wish_id: &'a Digest,
    ranking: Vec<LevContent<'a>>,
    evidence_bundle_id: &'a Digest,
}

/// A content-addressed precedence ranking of an unmet facet set — foundations
/// first. The descent-side convergence lens (Run 5).
///
/// Invariants:
/// - `id = SHA-256(JCS(content))` — INVARIANT-007
/// - `evidence_bundle_id ≠ ZERO` — CROSS-006
/// - it ranks/orders effort; it gates nothing — CROSS-010
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrecedenceOrder {
    pub id: Digest,
    pub wish_id: Digest,
    /// Facets ranked by descending leverage (ties: shallower layer, then facet).
    pub ranking: Vec<FacetLeverage>,
    pub evidence_bundle_id: Digest,
}

impl PrecedenceOrder {
    /// Focus an unmet facet set: rank each facet by how many others depend on
    /// it, foundations first. Pure and deterministic.
    pub fn focus(unmet: &[WishFacet], wish_id: Digest, evidence_bundle_id: Digest) -> Self {
        let n = unmet.len() as u64;
        let mut ranking: Vec<FacetLeverage> = unmet
            .iter()
            .map(|f| {
                let leverage = unmet.iter().filter(|g| depends_on(g, f)).count() as u32;
                let score = Q16::ratio(leverage as u64, n).unwrap_or(Q16::ZERO);
                FacetLeverage {
                    facet: f.clone(),
                    leverage,
                    score,
                }
            })
            .collect();
        // Foundations first: most-depended-on, then shallower layer, then the
        // facet's own total order — fully deterministic.
        ranking.sort_by(|a, b| {
            b.leverage
                .cmp(&a.leverage)
                .then(
                    a.facet
                        .kind
                        .layer()
                        .rank()
                        .cmp(&b.facet.kind.layer().rank()),
                )
                .then(a.facet.cmp(&b.facet))
        });
        let mut o = Self {
            id: Digest::ZERO,
            wish_id,
            ranking,
            evidence_bundle_id,
        };
        o.id = o.compute_id();
        o
    }

    fn compute_id(&self) -> Digest {
        let ranking: Vec<LevContent> = self
            .ranking
            .iter()
            .map(|r| LevContent {
                facet: &r.facet,
                leverage: r.leverage,
                score_raw: r.score.raw(),
            })
            .collect();
        Digest::of(&PrecedenceContent {
            wish_id: &self.wish_id,
            ranking,
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

    /// The focal facet — the highest-leverage foundation to realize next.
    pub fn focal(&self) -> Option<&WishFacet> {
        self.ranking.first().map(|r| &r.facet)
    }

    /// The facets reordered foundations-first — the Solve target order.
    pub fn ordered_facets(&self) -> Vec<WishFacet> {
        self.ranking.iter().map(|r| r.facet.clone()).collect()
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wish::WishFacetKind;

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }
    fn ev() -> Digest {
        d(b"precedence-evidence")
    }

    #[test]
    fn depends_on_path_containment() {
        let crate_ = WishFacet::crate_("foo");
        let module = WishFacet::module("foo::bar");
        let symbol = WishFacet::symbol("foo::bar::baz");
        assert!(depends_on(&module, &crate_), "module depends on its crate");
        assert!(depends_on(&symbol, &module), "symbol depends on its module");
        assert!(depends_on(&symbol, &crate_), "symbol depends on its crate");
        assert!(!depends_on(&crate_, &module), "the crate is the prerequisite");
    }

    #[test]
    fn depends_on_dependency_edge_endpoints() {
        let dep = WishFacet::new(WishFacetKind::Dependency, "foo->bar");
        assert!(depends_on(&dep, &WishFacet::crate_("foo")));
        assert!(depends_on(&dep, &WishFacet::crate_("bar")));
        assert!(!depends_on(&dep, &WishFacet::crate_("baz")));
    }

    #[test]
    fn depends_on_composition_and_contract_symbols() {
        let comp = WishFacet::new(WishFacetKind::Composition, "make>>Config>>load");
        assert!(depends_on(&comp, &WishFacet::symbol("make")));
        assert!(depends_on(&comp, &WishFacet::symbol("load")));
        let contract = WishFacet::new(WishFacetKind::Contract, "handle(Req)->Resp");
        assert!(depends_on(&contract, &WishFacet::symbol("handle")));
    }

    #[test]
    fn bare_path_does_not_spuriously_depend_on_a_bare_symbol() {
        // "foo::bar" must NOT be read as depending on a bare symbol "bar".
        let module = WishFacet::module("foo::bar");
        let bare = WishFacet::symbol("bar");
        assert!(!depends_on(&module, &bare));
    }

    #[test]
    fn focus_ranks_foundation_first() {
        // A crate that hosts two modules outranks the modules.
        let unmet = vec![
            WishFacet::module("foo::a"),
            WishFacet::crate_("foo"),
            WishFacet::module("foo::b"),
        ];
        let order = PrecedenceOrder::focus(&unmet, d(b"w"), ev());
        assert_eq!(order.focal(), Some(&WishFacet::crate_("foo")));
        assert_eq!(order.ranking[0].leverage, 2, "two modules depend on the crate");
        // The reordering is a permutation of the input.
        let mut got = order.ordered_facets();
        got.sort();
        let mut want = unmet.clone();
        want.sort();
        assert_eq!(got, want);
    }

    #[test]
    fn ties_break_by_shallower_layer_then_facet() {
        // No dependencies among these → all leverage 0 → existence before wiring.
        let unmet = vec![
            WishFacet::new(WishFacetKind::Contract, "a(X)->Y"),
            WishFacet::crate_("z-crate"),
        ];
        let order = PrecedenceOrder::focus(&unmet, d(b"w"), ev());
        assert_eq!(
            order.focal(),
            Some(&WishFacet::crate_("z-crate")),
            "existence (the crate) is foundational even with zero dependents"
        );
    }

    #[test]
    fn precedence_id_deterministic_and_verifies() {
        let unmet = vec![WishFacet::crate_("foo"), WishFacet::module("foo::bar")];
        let a = PrecedenceOrder::focus(&unmet, d(b"w"), ev());
        let b = PrecedenceOrder::focus(&unmet, d(b"w"), ev());
        assert_eq!(a.id, b.id);
        assert!(a.verify_id());
    }

    #[test]
    fn precedence_is_order_independent_of_input() {
        // Same set in any input order → same ranking → same id (it is a focus,
        // not a record of how the caller happened to list the facets).
        let a = PrecedenceOrder::focus(
            &[WishFacet::crate_("foo"), WishFacet::module("foo::bar")],
            d(b"w"),
            ev(),
        );
        let b = PrecedenceOrder::focus(
            &[WishFacet::module("foo::bar"), WishFacet::crate_("foo")],
            d(b"w"),
            ev(),
        );
        assert_eq!(a.id, b.id);
        assert_eq!(a.ordered_facets(), b.ordered_facets());
    }

    #[test]
    fn precedence_evidence_mandatory() {
        let order = PrecedenceOrder::focus(&[WishFacet::crate_("foo")], d(b"w"), ev());
        assert!(order.is_evidence_bound());
        assert_ne!(order.evidence_bundle_id, Digest::ZERO);
    }
}

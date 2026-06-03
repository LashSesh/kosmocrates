//! Wish — a content-addressed declaration of a *desired* workspace topology.
//!
//! This is the seed of the "Wunsch-zu-System-Maschine". The existing substrate
//! measures voids against an *implicit* target: structural completeness. A
//! [`Wish`] makes the target *explicit* — a set of positive structural facets
//! the workspace is supposed to exhibit. [`assess_wish`] then measures the
//! **distance** between an observed topology and that wish, in `Q16`
//! fixed-point arithmetic.
//!
//! The distance is a *gradient*, never a gate (the CROSS-010 doctrine applied to
//! intent): it ranks how far a workspace sits from the wish, but it grants no
//! capability and bypasses no policy. Driving the distance to zero is the job of
//! the agent loop; this module only defines the *target* and the *ruler*.
//!
//! Fail-closed by construction: a required facet is satisfied **only** when it
//! is positively present in the observed topology. Anything unknown or
//! unobserved counts as unmet — a wish is never satisfied by absence of
//! evidence.

use crate::digest::Digest;
use crate::fixed_point::Q16;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The kind of structural facet a wish can require.
///
/// Every facet is expressed *positively* (something that must be present), even
/// "the bad thing must be gone": that is modelled as a [`WishFacetKind::Resolution`]
/// facet which the pipeline emits once the deficiency is resolved. Positive-only
/// targets keep the assessment strictly fail-closed — a wish can never be
/// satisfied by the mere *absence* of an observation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum WishFacetKind {
    /// A crate / package is present in the workspace.
    Crate,
    /// A module path is present (e.g. `kosmo_core::wish`).
    Module,
    /// A public symbol is present (fn / type / trait / const).
    Symbol,
    /// A named capability or behaviour is present (free-form tag).
    Capability,
    /// A named deficiency / void has been resolved.
    Resolution,
    /// A directed crate dependency edge `"from->to"` is present.
    Dependency,
    /// A public function signature `"name/arity"` is present (name + arg count).
    Signature,
    /// A test is present (`#[test] fn name`), keyed by the test function name.
    Test,
}

/// A single normalized structural facet: a `(kind, key)` pair.
///
/// Two facets are equal iff they share both kind and key. The `key` is the
/// caller's stable identifier for the thing (crate name, module path, symbol
/// path, capability tag, or void id) and is trimmed on construction.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct WishFacet {
    pub kind: WishFacetKind,
    pub key: String,
}

impl WishFacet {
    pub fn new(kind: WishFacetKind, key: impl Into<String>) -> Self {
        Self {
            kind,
            key: key.into().trim().to_string(),
        }
    }
    /// A crate / package facet. (`crate` is a keyword, hence the trailing `_`.)
    pub fn crate_(name: impl Into<String>) -> Self {
        Self::new(WishFacetKind::Crate, name)
    }
    pub fn module(path: impl Into<String>) -> Self {
        Self::new(WishFacetKind::Module, path)
    }
    pub fn symbol(path: impl Into<String>) -> Self {
        Self::new(WishFacetKind::Symbol, path)
    }
    pub fn capability(tag: impl Into<String>) -> Self {
        Self::new(WishFacetKind::Capability, tag)
    }
    pub fn resolution(key: impl Into<String>) -> Self {
        Self::new(WishFacetKind::Resolution, key)
    }
    /// A dependency-edge facet: crate `from` depends on crate `to`
    /// (keyed `"from->to"`).
    pub fn dependency(from: impl AsRef<str>, to: impl AsRef<str>) -> Self {
        Self::new(
            WishFacetKind::Dependency,
            format!("{}->{}", from.as_ref(), to.as_ref()),
        )
    }
    /// A function-signature facet, keyed `"name/arity"` (e.g. `"handle/2"`).
    pub fn signature(key: impl Into<String>) -> Self {
        Self::new(WishFacetKind::Signature, key)
    }
    /// A test facet, keyed by the test function name.
    pub fn test(name: impl Into<String>) -> Self {
        Self::new(WishFacetKind::Test, name)
    }
}

/// One requirement of a wish: a facet that must be present, with a `Q16` weight.
///
/// The weight scales the facet's contribution to the wish distance, so a wish
/// can say "this part matters twice as much as that part". Default weight is
/// `Q16::ONE` via [`WishPredicate::require`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WishPredicate {
    pub facet: WishFacet,
    pub weight: Q16,
}

impl WishPredicate {
    /// A unit-weight requirement.
    pub fn require(facet: WishFacet) -> Self {
        Self {
            facet,
            weight: Q16::ONE,
        }
    }
    /// A requirement with an explicit weight. A negative weight is clamped to
    /// `Q16::ZERO`: a predicate can never *subtract* from the distance and so
    /// can never make a wish look more realized than the facts allow.
    pub fn weighted(facet: WishFacet, weight: Q16) -> Self {
        let weight = if weight.is_negative() {
            Q16::ZERO
        } else {
            weight
        };
        Self { facet, weight }
    }
}

/// The set of facets observed as *present* in a workspace.
///
/// Built by a caller from a real scan (pipeline topology, parse-back snapshot,
/// resolved-void set). In the first iteration it is supplied directly; later
/// runs populate it from the live pipeline. It is treated as authoritative for
/// the facets it contains: a facet not in the set is considered **not present**
/// (fail-closed).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedTopology {
    facets: BTreeSet<WishFacet>,
}

impl ObservedTopology {
    pub fn empty() -> Self {
        Self {
            facets: BTreeSet::new(),
        }
    }
    pub fn from_facets(facets: impl IntoIterator<Item = WishFacet>) -> Self {
        Self {
            facets: facets.into_iter().collect(),
        }
    }
    /// Record a facet as present. Returns `true` if newly inserted.
    pub fn insert(&mut self, facet: WishFacet) -> bool {
        self.facets.insert(facet)
    }
    pub fn contains(&self, facet: &WishFacet) -> bool {
        self.facets.contains(facet)
    }
    pub fn len(&self) -> usize {
        self.facets.len()
    }
    pub fn is_empty(&self) -> bool {
        self.facets.is_empty()
    }
    pub fn facets(&self) -> impl Iterator<Item = &WishFacet> {
        self.facets.iter()
    }
}

/// Content for the deterministic `Wish` id.
#[derive(Serialize)]
struct WishContent<'a> {
    label: &'a str,
    predicates: &'a Vec<WishPredicate>,
    policy_id: &'a Digest,
    evidence_bundle_id: &'a Digest,
}

/// A content-addressed declaration of a desired workspace topology.
///
/// Invariants:
/// - `id = SHA-256(JCS(content))` — INVARIANT-007
/// - `predicates` are sorted by facet and de-duplicated, so two wishes that
///   request the same facets in any order share an `id` (deterministic replay)
/// - `evidence_bundle_id ≠ ZERO` for a durable wish — CROSS-006
///   (enforced by callers; surfaced via [`Wish::is_evidence_bound`])
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wish {
    pub id: Digest,
    pub label: String,
    pub predicates: Vec<WishPredicate>,
    pub policy_id: Digest,
    pub evidence_bundle_id: Digest,
}

impl Wish {
    pub fn new(
        label: impl Into<String>,
        predicates: impl IntoIterator<Item = WishPredicate>,
        policy_id: Digest,
        evidence_bundle_id: Digest,
    ) -> Self {
        let mut predicates: Vec<WishPredicate> = predicates.into_iter().collect();
        // Deterministic order, and a duplicated facet must not double-count
        // toward the distance: sort by facet, then keep the first of each.
        predicates.sort_by(|a, b| a.facet.cmp(&b.facet));
        predicates.dedup_by(|a, b| a.facet == b.facet);
        let mut w = Self {
            id: Digest::ZERO,
            label: label.into(),
            predicates,
            policy_id,
            evidence_bundle_id,
        };
        w.id = w.compute_id();
        w
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&WishContent {
            label: &self.label,
            predicates: &self.predicates,
            policy_id: &self.policy_id,
            evidence_bundle_id: &self.evidence_bundle_id,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }

    /// CROSS-006: a durable wish must be evidence-bound.
    pub fn is_evidence_bound(&self) -> bool {
        self.evidence_bundle_id != Digest::ZERO
    }

    /// Total `Q16` weight across all predicates (saturating).
    pub fn total_weight(&self) -> Q16 {
        self.predicates
            .iter()
            .fold(Q16::ZERO, |acc, p| acc.saturating_add(p.weight))
    }

    pub fn predicate_count(&self) -> usize {
        self.predicates.len()
    }
}

/// Categorical closure state of a wish against an observed topology.
///
/// Mirrors the substrate's existing "closure" vocabulary (`ValidationClosureStatus`)
/// but on the *intent* axis: how close a workspace is to a desired shape.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WishClosureStatus {
    /// The wish has at least one predicate and every one is satisfied.
    Realized,
    /// Some — but not all — predicates are satisfied.
    Approaching,
    /// The wish has predicates but none are satisfied yet.
    Unstarted,
    /// The wish carries no predicates: trivially (and meaninglessly) satisfied.
    Vacuous,
}

impl WishClosureStatus {
    pub fn is_realized(&self) -> bool {
        matches!(self, Self::Realized)
    }
    pub fn is_vacuous(&self) -> bool {
        matches!(self, Self::Vacuous)
    }
}

/// Content for the deterministic `WishAssessment` id.
#[derive(Serialize)]
struct AssessmentContent<'a> {
    wish_id: &'a Digest,
    distance_raw: i64,
    met_count: u32,
    total_count: u32,
    status: &'a WishClosureStatus,
    unmet_facets: &'a Vec<WishFacet>,
    evidence_bundle_id: &'a Digest,
}

/// A content-addressed measurement of how far an observed topology is from a wish.
///
/// `distance` is the weighted fraction of unmet predicates, in `Q16`: `ZERO` ⇒
/// fully realized, `ONE` ⇒ nothing met. `unmet_facets` is the remaining
/// gradient — exactly the facet set the agent loop must close to realize the
/// wish (the hand-off point for the next iteration).
///
/// Invariants:
/// - `id = SHA-256(JCS(content))` — INVARIANT-007
/// - `evidence_bundle_id ≠ ZERO` — CROSS-006
/// - `distance` is a *ranking* quantity only; it gates nothing (CROSS-010 doctrine)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WishAssessment {
    pub id: Digest,
    pub wish_id: Digest,
    pub distance: Q16,
    pub met_count: u32,
    pub total_count: u32,
    pub status: WishClosureStatus,
    pub unmet_facets: Vec<WishFacet>,
    pub evidence_bundle_id: Digest,
}

impl WishAssessment {
    fn compute_id(&self) -> Digest {
        Digest::of(&AssessmentContent {
            wish_id: &self.wish_id,
            distance_raw: self.distance.raw(),
            met_count: self.met_count,
            total_count: self.total_count,
            status: &self.status,
            unmet_facets: &self.unmet_facets,
            evidence_bundle_id: &self.evidence_bundle_id,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }

    pub fn is_realized(&self) -> bool {
        self.status.is_realized()
    }

    /// CROSS-006.
    pub fn is_evidence_bound(&self) -> bool {
        self.evidence_bundle_id != Digest::ZERO
    }
}

/// Measure the distance between an observed topology and a wish.
///
/// Pure and deterministic: identical `(wish, observed, evidence_bundle_id)`
/// always produce a byte-identical [`WishAssessment`]. Fail-closed: a predicate
/// is *met* only when its facet is positively present in `observed`.
///
/// The `distance` is the weighted unmet fraction. When all weights are zero
/// (a degenerate wish) it falls back to the unmet fraction by *count*, so the
/// gradient stays consistent with the categorical `status`.
pub fn assess_wish(
    wish: &Wish,
    observed: &ObservedTopology,
    evidence_bundle_id: Digest,
) -> WishAssessment {
    let total_count = wish.predicates.len() as u32;

    let mut met_weight = Q16::ZERO;
    let mut total_weight = Q16::ZERO;
    let mut met_count: u32 = 0;
    let mut unmet_facets: Vec<WishFacet> = Vec::new();

    for p in &wish.predicates {
        total_weight = total_weight.saturating_add(p.weight);
        if observed.contains(&p.facet) {
            met_weight = met_weight.saturating_add(p.weight);
            met_count += 1;
        } else {
            unmet_facets.push(p.facet.clone());
        }
    }
    // `predicates` is sorted by facet in `Wish::new`, so `unmet_facets` is too.

    let distance = if total_weight.is_zero() {
        if total_count == 0 {
            Q16::ZERO
        } else {
            // Weight-less wish: fall back to the unmet fraction by count.
            Q16::ratio((total_count - met_count) as u64, total_count as u64).unwrap_or(Q16::ONE)
        }
    } else {
        let unmet_weight = total_weight.saturating_sub(met_weight);
        unmet_weight.checked_div(total_weight).unwrap_or(Q16::ONE)
    };

    let status = if total_count == 0 {
        WishClosureStatus::Vacuous
    } else if met_count == total_count {
        WishClosureStatus::Realized
    } else if met_count == 0 {
        WishClosureStatus::Unstarted
    } else {
        WishClosureStatus::Approaching
    };

    let mut a = WishAssessment {
        id: Digest::ZERO,
        wish_id: wish.id,
        distance,
        met_count,
        total_count,
        status,
        unmet_facets,
        evidence_bundle_id,
    };
    a.id = a.compute_id();
    a
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    fn policy_id() -> Digest {
        d(b"policy")
    }

    fn bundle_id() -> Digest {
        d(b"evidence-bundle")
    }

    fn assess_bundle() -> Digest {
        d(b"assessment-evidence")
    }

    /// A wish requiring a crate, a module and a capability — all unit weight.
    fn sample_wish() -> Wish {
        Wish::new(
            "expose an http server",
            [
                WishPredicate::require(WishFacet::crate_("kosmo-server")),
                WishPredicate::require(WishFacet::module("kosmo_server::routes")),
                WishPredicate::require(WishFacet::capability("http-server")),
            ],
            policy_id(),
            bundle_id(),
        )
    }

    // ── WishFacet ─────────────────────────────────────────────────────────

    #[test]
    fn facet_constructors_normalize_key() {
        let f = WishFacet::crate_("  kosmo-server  ");
        assert_eq!(f.key, "kosmo-server");
        assert_eq!(f.kind, WishFacetKind::Crate);
    }

    #[test]
    fn facet_equality_by_kind_and_key() {
        assert_eq!(WishFacet::symbol("a::b"), WishFacet::symbol("a::b"));
        assert_ne!(WishFacet::symbol("a::b"), WishFacet::module("a::b"));
        assert_ne!(WishFacet::symbol("a::b"), WishFacet::symbol("a::c"));
    }

    // ── Wish content addressing ───────────────────────────────────────────

    #[test]
    fn wish_id_deterministic() {
        assert_eq!(sample_wish().id, sample_wish().id);
    }

    #[test]
    fn wish_verify_id() {
        assert!(sample_wish().verify_id());
    }

    #[test]
    fn wish_predicate_order_does_not_change_id() {
        let a = Wish::new(
            "w",
            [
                WishPredicate::require(WishFacet::crate_("a")),
                WishPredicate::require(WishFacet::crate_("b")),
            ],
            policy_id(),
            bundle_id(),
        );
        let b = Wish::new(
            "w",
            [
                WishPredicate::require(WishFacet::crate_("b")),
                WishPredicate::require(WishFacet::crate_("a")),
            ],
            policy_id(),
            bundle_id(),
        );
        assert_eq!(a.id, b.id, "predicate order must not affect identity");
    }

    #[test]
    fn wish_dedups_repeated_facets() {
        let w = Wish::new(
            "w",
            [
                WishPredicate::require(WishFacet::crate_("a")),
                WishPredicate::require(WishFacet::crate_("b")),
                WishPredicate::require(WishFacet::crate_("a")),
            ],
            policy_id(),
            bundle_id(),
        );
        assert_eq!(w.predicate_count(), 2, "duplicate facet must collapse");
    }

    #[test]
    fn wish_different_label_differs() {
        let a = Wish::new("one", [], policy_id(), bundle_id());
        let b = Wish::new("two", [], policy_id(), bundle_id());
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn wish_is_evidence_bound() {
        assert!(sample_wish().is_evidence_bound());
        let unbound = Wish::new("w", [], policy_id(), Digest::ZERO);
        assert!(!unbound.is_evidence_bound());
    }

    // ── assess_wish ───────────────────────────────────────────────────────

    #[test]
    fn assess_empty_observed_is_unstarted_distance_one() {
        let w = sample_wish();
        let a = assess_wish(&w, &ObservedTopology::empty(), assess_bundle());
        assert_eq!(a.status, WishClosureStatus::Unstarted);
        assert_eq!(a.distance, Q16::ONE);
        assert_eq!(a.met_count, 0);
        assert_eq!(a.total_count, 3);
        assert_eq!(a.unmet_facets.len(), 3);
    }

    #[test]
    fn assess_full_observed_is_realized_distance_zero() {
        let w = sample_wish();
        let observed = ObservedTopology::from_facets([
            WishFacet::crate_("kosmo-server"),
            WishFacet::module("kosmo_server::routes"),
            WishFacet::capability("http-server"),
        ]);
        let a = assess_wish(&w, &observed, assess_bundle());
        assert_eq!(a.status, WishClosureStatus::Realized);
        assert!(a.is_realized());
        assert_eq!(a.distance, Q16::ZERO);
        assert_eq!(a.met_count, 3);
        assert!(a.unmet_facets.is_empty());
    }

    #[test]
    fn assess_partial_is_approaching_half_distance() {
        let w = Wish::new(
            "two things",
            [
                WishPredicate::require(WishFacet::crate_("a")),
                WishPredicate::require(WishFacet::crate_("b")),
            ],
            policy_id(),
            bundle_id(),
        );
        let observed = ObservedTopology::from_facets([WishFacet::crate_("a")]);
        let a = assess_wish(&w, &observed, assess_bundle());
        assert_eq!(a.status, WishClosureStatus::Approaching);
        assert_eq!(a.distance, Q16::HALF);
        assert_eq!(a.met_count, 1);
        assert_eq!(a.unmet_facets, vec![WishFacet::crate_("b")]);
    }

    #[test]
    fn assess_weighted_distance_reflects_weights() {
        // a (weight 1) met, b (weight 3) unmet → unmet fraction = 3/4.
        let w = Wish::new(
            "weighted",
            [
                WishPredicate::weighted(WishFacet::crate_("a"), Q16::ONE),
                WishPredicate::weighted(WishFacet::crate_("b"), Q16::from_i64(3)),
            ],
            policy_id(),
            bundle_id(),
        );
        let observed = ObservedTopology::from_facets([WishFacet::crate_("a")]);
        let a = assess_wish(&w, &observed, assess_bundle());
        assert_eq!(a.distance, Q16::ratio(3, 4).unwrap());
        assert_eq!(a.status, WishClosureStatus::Approaching);
    }

    #[test]
    fn assess_vacuous_when_no_predicates() {
        let w = Wish::new("empty", [], policy_id(), bundle_id());
        let a = assess_wish(&w, &ObservedTopology::empty(), assess_bundle());
        assert_eq!(a.status, WishClosureStatus::Vacuous);
        assert!(a.status.is_vacuous());
        assert_eq!(a.distance, Q16::ZERO);
        assert!(!a.is_realized());
    }

    #[test]
    fn assess_is_failclosed_unrelated_facet_counts_unmet() {
        // Observing something the wish did not ask for must not satisfy it.
        let w = Wish::new(
            "w",
            [WishPredicate::require(WishFacet::crate_("wanted"))],
            policy_id(),
            bundle_id(),
        );
        let observed = ObservedTopology::from_facets([WishFacet::crate_("unrelated")]);
        let a = assess_wish(&w, &observed, assess_bundle());
        assert_eq!(a.status, WishClosureStatus::Unstarted);
        assert_eq!(a.distance, Q16::ONE);
    }

    #[test]
    fn assess_distance_is_bounded_unit_interval() {
        // Across the spectrum, distance never leaves [ZERO, ONE].
        let w = sample_wish();
        for observed in [
            ObservedTopology::empty(),
            ObservedTopology::from_facets([WishFacet::crate_("kosmo-server")]),
            ObservedTopology::from_facets([
                WishFacet::crate_("kosmo-server"),
                WishFacet::module("kosmo_server::routes"),
                WishFacet::capability("http-server"),
            ]),
        ] {
            let a = assess_wish(&w, &observed, assess_bundle());
            assert!(a.distance.at_least(Q16::ZERO));
            assert!(Q16::ONE.at_least(a.distance));
        }
    }

    // ── WishAssessment content addressing ─────────────────────────────────

    #[test]
    fn assessment_id_deterministic() {
        let w = sample_wish();
        let o = ObservedTopology::from_facets([WishFacet::crate_("kosmo-server")]);
        let a1 = assess_wish(&w, &o, assess_bundle());
        let a2 = assess_wish(&w, &o, assess_bundle());
        assert_eq!(a1.id, a2.id);
    }

    #[test]
    fn assessment_verify_id() {
        let a = assess_wish(&sample_wish(), &ObservedTopology::empty(), assess_bundle());
        assert!(a.verify_id());
    }

    #[test]
    fn assessment_different_observation_differs() {
        let w = sample_wish();
        let a1 = assess_wish(&w, &ObservedTopology::empty(), assess_bundle());
        let a2 = assess_wish(
            &w,
            &ObservedTopology::from_facets([WishFacet::crate_("kosmo-server")]),
            assess_bundle(),
        );
        assert_ne!(a1.id, a2.id);
    }

    #[test]
    fn assessment_unmet_facets_are_sorted() {
        let w = sample_wish();
        let a = assess_wish(&w, &ObservedTopology::empty(), assess_bundle());
        let mut sorted = a.unmet_facets.clone();
        sorted.sort();
        assert_eq!(a.unmet_facets, sorted, "unmet facets must be deterministically ordered");
    }

    #[test]
    fn assessment_evidence_mandatory() {
        let a = assess_wish(&sample_wish(), &ObservedTopology::empty(), assess_bundle());
        assert!(a.is_evidence_bound());
        assert_ne!(a.evidence_bundle_id, Digest::ZERO);
    }

    #[test]
    fn assessment_links_back_to_wish() {
        let w = sample_wish();
        let a = assess_wish(&w, &ObservedTopology::empty(), assess_bundle());
        assert_eq!(a.wish_id, w.id);
    }
}

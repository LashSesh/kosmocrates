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
    /// A typed function contract `"name(T0,T1)->R"` is present — the typed
    /// promotion of [`WishFacetKind::Signature`] (parameter + return *types*,
    /// not merely the count). Shallow types only: single tokens and simple
    /// generics round-trip; deep generics are stored verbatim.
    Contract,
    /// A test is present (`#[test] fn name`), keyed by the test function name.
    Test,
    /// A **documented** public item is present: the item named by the key
    /// carries a doc comment (`///` or `#[doc = …]`) immediately above its
    /// definition. The wish-language mirror of the substrate's most common
    /// finding (`MissingDocFiber`): "docs of helper" is now a measurable
    /// target, not just a diagnosis. Keyed like [`WishFacetKind::Symbol`]
    /// (bare name, optionally `name@crate`).
    Doc,
    /// A *validated behaviour* is present, keyed by a spec `"name(args)=>expected"`.
    /// The keystone of the behaviour axis: this facet is satisfied **only** when
    /// a scaffolded spec-test pinning that input→output pair actually *passes*
    /// (observed by running the suite). Acceptance over generation — the loop
    /// declares it realized only when behaviour is observed correct.
    Behavior,
    /// A typed data-flow composition `"from>>via>>to"`: `from` returns type
    /// `via` and `to` takes `via` as its first parameter, so the components
    /// *wire together* (`to(from(x))` typechecks). The behavioural cousin of
    /// [`WishFacetKind::Dependency`]: a dependency says A *can see* B; a
    /// composition says A's output *fits* B's input.
    Composition,
    /// A *runtime* probe, keyed `"args=>expect"` (e.g. `"add,2,3=>out~5"`):
    /// the built artifact, **run** with `args`, exhibits `expect`
    /// (`exit:<code>` and/or `out~<substr>`, optionally capped by a
    /// tail-anchored wall-clock budget `ms<N` — speed as a measurable
    /// facet). The level-5 keystone — observed not by reading source or
    /// running a unit test, but by *executing the program* under the
    /// sandbox. Fail-closed: met only on a clean exit that matches, within
    /// budget.
    Run,
    /// A *service* probe, keyed `"method:path=>expect"`
    /// (e.g. `"GET:/health=>200"`): the built artifact, **started as a server**,
    /// answers the request with `expect` (an HTTP status and/or `body~<substr>`).
    /// The deepest observation — start, await readiness, probe, tear down —
    /// fail-closed: met only if the server actually answers and matches.
    Service,
}

/// The render stratum a facet kind occupies — its height in the wish-hypercube,
/// from the first-printed transparent scaffold ([`WishLayer::Existence`]) to the
/// last-printed massive layer ([`WishLayer::Live`], a program that executes or
/// serves). A *graded depth*, never a gate (CROSS-010): the stratum colours how
/// solid a layer is and orders the rendering, but it grants no capability and it
/// never un-meets a facet.
///
/// The ordering is the substrate's existing maturation ladder lifted onto the
/// intent axis: existence-strata are the `Pending`-equivalent scaffold, the
/// upper strata the `Certified`-equivalent solid core (cf. the crystal
/// `CertificationStatus` lifecycle in `kosmo-hyphae`). The map is *explicit*
/// (not the enum's derived `Ord`) because declaration order does not match
/// semantic depth — `Capability`/`Resolution` are free-form tags with no
/// structural depth, and `Doc` is a shape attribute, not a post-test concern.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum WishLayer {
    /// The crate/module/symbol scaffold: the print exists but is transparent.
    Existence,
    /// Declared shape — signatures, dependency edges, docs — still a hologram.
    Shape,
    /// Typed interlock — contracts and compositions that wire together.
    Wiring,
    /// Observed-correct logic — tests pass, behaviours hold.
    Verified,
    /// The program executes / serves: the massive, solid diamond layer.
    Live,
}

/// Number of render strata in a wish-hypercube (existence … live).
pub const WISH_LAYER_COUNT: usize = 5;

impl WishLayer {
    /// All strata, shallowest (first-printed) first.
    pub fn all() -> [WishLayer; WISH_LAYER_COUNT] {
        [
            WishLayer::Existence,
            WishLayer::Shape,
            WishLayer::Wiring,
            WishLayer::Verified,
            WishLayer::Live,
        ]
    }
    /// The stratum's height — `0` is the first-printed scaffold. Used as the
    /// stable key in content-addressed cube/trace ids (no float, no `Ord`-order
    /// dependence).
    pub fn rank(self) -> u8 {
        match self {
            WishLayer::Existence => 0,
            WishLayer::Shape => 1,
            WishLayer::Wiring => 2,
            WishLayer::Verified => 3,
            WishLayer::Live => 4,
        }
    }
    /// A stable lowercase label for rendering and audit.
    pub fn label(self) -> &'static str {
        match self {
            WishLayer::Existence => "existence",
            WishLayer::Shape => "shape",
            WishLayer::Wiring => "wiring",
            WishLayer::Verified => "verified",
            WishLayer::Live => "live",
        }
    }
}

impl WishFacetKind {
    /// The render stratum of this facet kind — its height in the 3-D print, from
    /// the transparent existence scaffold up to the massive `Live` layer.
    ///
    /// Explicit by design (see [`WishLayer`]): the enum's declaration order does
    /// **not** match semantic depth, so this is never `self as usize`.
    pub fn layer(&self) -> WishLayer {
        use WishFacetKind::*;
        match self {
            Crate | Module | Symbol | Capability | Resolution => WishLayer::Existence,
            Signature | Dependency | Doc => WishLayer::Shape,
            Contract | Composition => WishLayer::Wiring,
            Test | Behavior => WishLayer::Verified,
            Run | Service => WishLayer::Live,
        }
    }
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
    /// A documented-item facet: the named public item carries a doc comment.
    pub fn doc(path: impl Into<String>) -> Self {
        Self::new(WishFacetKind::Doc, path)
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
    /// A typed function-contract facet, keyed `"name(T0,T1)->R"` — the typed
    /// promotion of [`WishFacet::signature`]. `params` are the parameter
    /// *types* in declaration order; `ret` is the return type (`"()"` for
    /// none). Types are stored verbatim, so callers pass normalized type
    /// strings (the observer in `kosmo-intent` collapses whitespace).
    pub fn contract(name: impl AsRef<str>, params: &[&str], ret: impl AsRef<str>) -> Self {
        Self::new(
            WishFacetKind::Contract,
            format!("{}({})->{}", name.as_ref(), params.join(","), ret.as_ref()),
        )
    }
    /// A typed function-contract facet from a pre-formed `"name(T)->R"` key.
    pub fn contract_key(key: impl Into<String>) -> Self {
        Self::new(WishFacetKind::Contract, key)
    }
    /// A test facet, keyed by the test function name.
    pub fn test(name: impl Into<String>) -> Self {
        Self::new(WishFacetKind::Test, name)
    }
    /// A validated-behaviour facet, keyed by a spec `"name(args)=>expected"`
    /// (e.g. `"add(2,3)=>5"`). Met only when the scaffolded spec-test passes.
    pub fn behavior(spec: impl Into<String>) -> Self {
        Self::new(WishFacetKind::Behavior, spec)
    }
    /// A typed data-flow composition facet `"from>>via>>to"`: `from` returns
    /// `via` and `to` consumes `via` as its first parameter.
    pub fn composition(from: impl AsRef<str>, via: impl AsRef<str>, to: impl AsRef<str>) -> Self {
        Self::new(
            WishFacetKind::Composition,
            format!("{}>>{}>>{}", from.as_ref(), via.as_ref(), to.as_ref()),
        )
    }
    /// A runtime probe facet, keyed `"args=>expect"` (e.g. `"add,2,3=>out~5"`
    /// or budgeted `"add,2,3=>out~5,ms<50"`): the built artifact run with
    /// comma-separated `args` exhibits `expect` within the optional budget.
    pub fn run(probe: impl Into<String>) -> Self {
        Self::new(WishFacetKind::Run, probe)
    }
    /// A service probe facet, keyed `"method:path=>expect"` (e.g.
    /// `"GET:/health=>200"`): the built artifact, started as a server, answers
    /// the request with the expected status and/or `body~<substr>`.
    pub fn service(probe: impl Into<String>) -> Self {
        Self::new(WishFacetKind::Service, probe)
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
    /// Keep only the facets for which `keep` returns `true`. Used to swap
    /// lexically-observed facets for validated ones (e.g. green tests).
    pub fn retain(&mut self, keep: impl Fn(&WishFacet) -> bool) {
        self.facets.retain(|f| keep(f));
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

/// One rendered stratum of a wish-hypercube: how solid this layer currently is.
///
/// `opacity` is `ONE − (per-layer distance)` — `ZERO` ⇒ transparent (nothing in
/// this stratum is met), `ONE` ⇒ solid (every facet in this stratum is met).
/// `unmet_facets` is the per-layer gradient: exactly the gaps the loop must close
/// to solidify this layer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WishLayerView {
    pub layer: WishLayer,
    pub met_count: u32,
    pub total_count: u32,
    /// `ONE − per-layer distance`. `ZERO` = transparent, `ONE` = solid.
    pub opacity: Q16,
    /// Fail-closed remaining gaps in this stratum (sorted, like the wish).
    pub unmet_facets: Vec<WishFacet>,
}

impl WishLayerView {
    /// Solid iff this stratum carries predicates and every one is met — the
    /// wish-layer analogue of crystal `Certified`.
    pub fn is_solid(&self) -> bool {
        self.total_count > 0 && self.met_count == self.total_count
    }
    /// No predicates ask anything at this height: an empty (not hollow) stratum.
    pub fn is_empty_layer(&self) -> bool {
        self.total_count == 0
    }
}

/// Content for the deterministic `WishCube` id.
#[derive(Serialize)]
struct WishCubeContent<'a> {
    wish_id: &'a Digest,
    assessment_id: &'a Digest,
    /// Per stratum `(rank, opacity_raw, met, total)` — sorted by rank, JCS-friendly.
    layers: &'a Vec<(u8, i64, u32, u32)>,
    overall_opacity_raw: i64,
    structural_solidity_raw: i64,
    evidence_bundle_id: &'a Digest,
}

/// A wish projected as a layered hypercube: the flat predicate set partitioned by
/// render-stratum, each stratum carrying its own opacity (`ONE − distance`).
///
/// Strictly a richer *view* over the same facts [`assess_wish`] measures — the
/// flat [`WishAssessment`] is preserved and linked by `assessment_id`. Two
/// solidity scalars are exposed, for two honest questions:
/// - `overall_opacity` — the weighted-mean opacity (`== ONE − the flat distance`),
///   the same number the scalar trajectory tracks; kept for continuity.
/// - `structural_solidity` — the **geometric** mean of the non-empty layers'
///   opacities, so one hollow layer silences the ensemble ([`Q16::geomean`]'s
///   soft-unanimity): the "you cannot be a solid diamond above a hollow layer"
///   measure — what a flat scalar cannot see.
///
/// Invariants:
/// - `id = SHA-256(JCS(content))` — INVARIANT-007
/// - `evidence_bundle_id ≠ ZERO` — CROSS-006
/// - every quantity ranks, nothing gates — CROSS-010
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WishCube {
    pub id: Digest,
    pub wish_id: Digest,
    /// Links the embedded flat [`WishAssessment`] this cube is a view over.
    pub assessment_id: Digest,
    /// One view per stratum, ascending by [`WishLayer::rank`].
    pub layers: Vec<WishLayerView>,
    /// `ONE − flat distance` — continuity with the scalar Run-1 measurement.
    pub overall_opacity: Q16,
    /// Geometric mean of the non-empty layers' opacities (soft-unanimity).
    pub structural_solidity: Q16,
    pub evidence_bundle_id: Digest,
}

impl WishCube {
    fn compute_id(&self) -> Digest {
        let layers: Vec<(u8, i64, u32, u32)> = self
            .layers
            .iter()
            .map(|l| (l.layer.rank(), l.opacity.raw(), l.met_count, l.total_count))
            .collect();
        Digest::of(&WishCubeContent {
            wish_id: &self.wish_id,
            assessment_id: &self.assessment_id,
            layers: &layers,
            overall_opacity_raw: self.overall_opacity.raw(),
            structural_solidity_raw: self.structural_solidity.raw(),
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

    /// The view for a given stratum (always present — empty strata included).
    pub fn layer(&self, layer: WishLayer) -> Option<&WishLayerView> {
        self.layers.iter().find(|l| l.layer == layer)
    }

    /// The deepest stratum of the *contiguous solid base*, skipping empty
    /// strata (nothing asked there does not block the print). `None` until the
    /// lowest non-empty stratum is solid. This is "how high the print has set" —
    /// the curriculum frontier the staged pipeline climbs.
    pub fn solid_frontier(&self) -> Option<WishLayer> {
        let mut frontier = None;
        for l in &self.layers {
            if l.is_empty_layer() {
                continue;
            }
            if l.is_solid() {
                frontier = Some(l.layer);
            } else {
                break;
            }
        }
        frontier
    }

    /// A deeper non-empty stratum has started rendering (`met_count > 0`) while a
    /// shallower non-empty stratum is not yet solid — a layer "floating" in
    /// mid-air above an unset base (suspect an over-fit shell). Advisory only
    /// (CROSS-010); it never un-meets the floating facet.
    pub fn has_floating_layer(&self) -> bool {
        let mut unsolid_below = false;
        for l in &self.layers {
            if l.is_empty_layer() {
                continue;
            }
            if l.met_count > 0 && unsolid_below {
                return true;
            }
            if !l.is_solid() {
                unsolid_below = true;
            }
        }
        false
    }
}

/// Render a wish into its layered hypercube against an observed topology.
///
/// Reuses [`assess_wish`] for the flat measurement (so `overall_opacity` is
/// *exactly* `ONE − distance`) and partitions the predicates by
/// [`WishFacetKind::layer`] for the per-stratum opacities. Pure and
/// deterministic: identical `(wish, observed, evidence_bundle_id)` produce a
/// byte-identical [`WishCube`].
pub fn assess_wish_layered(
    wish: &Wish,
    observed: &ObservedTopology,
    evidence_bundle_id: Digest,
) -> WishCube {
    let assessment = assess_wish(wish, observed, evidence_bundle_id);

    let layers: Vec<WishLayerView> = WishLayer::all()
        .into_iter()
        .map(|layer| {
            let mut met_count = 0u32;
            let mut total_count = 0u32;
            let mut unmet_facets: Vec<WishFacet> = Vec::new();
            // `wish.predicates` is sorted by facet, so `unmet_facets` stays sorted.
            for p in &wish.predicates {
                if p.facet.kind.layer() != layer {
                    continue;
                }
                total_count += 1;
                if observed.contains(&p.facet) {
                    met_count += 1;
                } else {
                    unmet_facets.push(p.facet.clone());
                }
            }
            let opacity = if total_count == 0 {
                Q16::ZERO
            } else {
                Q16::ratio(met_count as u64, total_count as u64).unwrap_or(Q16::ZERO)
            };
            WishLayerView {
                layer,
                met_count,
                total_count,
                opacity,
                unmet_facets,
            }
        })
        .collect();

    let overall_opacity = Q16::ONE.saturating_sub(assessment.distance);

    // Geomean over the non-empty strata only: an empty stratum is "nothing
    // asked," not a hollow zero, so it must not drag solidity to ZERO.
    let solidities: Vec<Q16> = layers
        .iter()
        .filter(|l| !l.is_empty_layer())
        .map(|l| l.opacity)
        .collect();
    let structural_solidity = if solidities.is_empty() {
        Q16::ZERO
    } else {
        Q16::geomean(&solidities)
    };

    let mut cube = WishCube {
        id: Digest::ZERO,
        wish_id: wish.id,
        assessment_id: assessment.id,
        layers,
        overall_opacity,
        structural_solidity,
        evidence_bundle_id,
    };
    cube.id = cube.compute_id();
    cube
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

    #[test]
    fn contract_facet_key_format() {
        let f = WishFacet::contract("handle", &["Request"], "Response");
        assert_eq!(f.kind, WishFacetKind::Contract);
        assert_eq!(f.key, "handle(Request)->Response");
        // The `contract_key` constructor accepts the same pre-formed key.
        assert_eq!(f, WishFacet::contract_key("handle(Request)->Response"));
    }

    #[test]
    fn contract_facet_no_params_unit_return() {
        assert_eq!(WishFacet::contract("tick", &[], "()").key, "tick()->()");
        assert_eq!(
            WishFacet::contract("add", &["i32", "i32"], "i32").key,
            "add(i32,i32)->i32"
        );
    }

    #[test]
    fn behavior_facet_key_is_the_spec() {
        let f = WishFacet::behavior("add(2,3)=>5");
        assert_eq!(f.kind, WishFacetKind::Behavior);
        assert_eq!(f.key, "add(2,3)=>5");
        assert_ne!(f, WishFacet::behavior("add(2,3)=>6"));
    }

    #[test]
    fn composition_facet_key_is_the_pipe() {
        let f = WishFacet::composition("parse", "Ast", "eval");
        assert_eq!(f.kind, WishFacetKind::Composition);
        assert_eq!(f.key, "parse>>Ast>>eval");
    }

    #[test]
    fn run_facet_key_is_the_probe() {
        let f = WishFacet::run("add,2,3=>out~5");
        assert_eq!(f.kind, WishFacetKind::Run);
        assert_eq!(f.key, "add,2,3=>out~5");
        assert_ne!(f, WishFacet::run("add,2,3=>out~6"));
    }

    #[test]
    fn service_facet_key_is_the_probe() {
        let f = WishFacet::service("GET:/health=>200");
        assert_eq!(f.kind, WishFacetKind::Service);
        assert_eq!(f.key, "GET:/health=>200");
        // Distinct from a Run facet with a similar key shape.
        assert_ne!(f.kind, WishFacet::run("GET:/health=>200").kind);
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
        assert_eq!(
            a.unmet_facets, sorted,
            "unmet facets must be deterministically ordered"
        );
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

    // ── Run 3: the layer ladder ───────────────────────────────────────────

    #[test]
    fn every_facet_kind_has_a_layer() {
        use WishFacetKind::*;
        let expect = [
            (Crate, WishLayer::Existence),
            (Module, WishLayer::Existence),
            (Symbol, WishLayer::Existence),
            (Capability, WishLayer::Existence),
            (Resolution, WishLayer::Existence),
            (Signature, WishLayer::Shape),
            (Dependency, WishLayer::Shape),
            (Doc, WishLayer::Shape),
            (Contract, WishLayer::Wiring),
            (Composition, WishLayer::Wiring),
            (Test, WishLayer::Verified),
            (Behavior, WishLayer::Verified),
            (Run, WishLayer::Live),
            (Service, WishLayer::Live),
        ];
        for (kind, layer) in expect {
            assert_eq!(kind.layer(), layer, "{kind:?} should map to {layer:?}");
        }
    }

    #[test]
    fn layers_are_strictly_graded_existence_lowest_live_highest() {
        assert!(WishFacetKind::Crate.layer() < WishFacetKind::Service.layer());
        assert_eq!(WishLayer::all().len(), WISH_LAYER_COUNT);
        let ranks: Vec<u8> = WishLayer::all().iter().map(|l| l.rank()).collect();
        assert_eq!(ranks, vec![0, 1, 2, 3, 4], "ranks ascend with semantic depth");
    }

    #[test]
    fn layer_does_not_follow_enum_declaration_order() {
        // `Doc` is declared AFTER `Test` in WishFacetKind, yet is shallower.
        assert!(WishFacetKind::Doc.layer() < WishFacetKind::Test.layer());
        // Free-form tags declared early are still existence-level.
        assert_eq!(WishFacetKind::Resolution.layer(), WishLayer::Existence);
    }

    // ── Run 3: WishCube (per-layer opacity) ───────────────────────────────

    /// A wish spanning three strata: crate (Existence), contract (Wiring),
    /// run probe (Live). Shape and Verified are left empty on purpose.
    fn layered_wish() -> Wish {
        Wish::new(
            "a spanning wish",
            [
                WishPredicate::require(WishFacet::crate_("kosmo-api")),
                WishPredicate::require(WishFacet::new(
                    WishFacetKind::Contract,
                    "handle(Request)->Response",
                )),
                WishPredicate::require(WishFacet::new(WishFacetKind::Run, "ping=>out~pong")),
            ],
            policy_id(),
            bundle_id(),
        )
    }

    #[test]
    fn cube_partitions_predicates_by_layer() {
        let w = layered_wish();
        let cube = assess_wish_layered(&w, &ObservedTopology::empty(), assess_bundle());
        assert_eq!(cube.layers.len(), WISH_LAYER_COUNT);
        assert_eq!(cube.layer(WishLayer::Existence).unwrap().total_count, 1);
        assert_eq!(cube.layer(WishLayer::Wiring).unwrap().total_count, 1);
        assert_eq!(cube.layer(WishLayer::Live).unwrap().total_count, 1);
        assert!(cube.layer(WishLayer::Shape).unwrap().is_empty_layer());
        assert!(cube.layer(WishLayer::Verified).unwrap().is_empty_layer());
    }

    #[test]
    fn empty_observed_cube_is_fully_transparent() {
        let cube = assess_wish_layered(&layered_wish(), &ObservedTopology::empty(), assess_bundle());
        for l in &cube.layers {
            assert_eq!(l.opacity, Q16::ZERO);
        }
        assert_eq!(cube.overall_opacity, Q16::ZERO);
        assert_eq!(cube.structural_solidity, Q16::ZERO);
        assert_eq!(cube.solid_frontier(), None);
    }

    #[test]
    fn full_observed_cube_is_fully_solid() {
        let w = layered_wish();
        let observed =
            ObservedTopology::from_facets(w.predicates.iter().map(|p| p.facet.clone()));
        let cube = assess_wish_layered(&w, &observed, assess_bundle());
        for l in &cube.layers {
            if !l.is_empty_layer() {
                assert!(l.is_solid());
                assert_eq!(l.opacity, Q16::ONE);
            }
        }
        assert_eq!(cube.overall_opacity, Q16::ONE);
        assert_eq!(cube.structural_solidity, Q16::ONE);
        assert_eq!(cube.solid_frontier(), Some(WishLayer::Live));
        assert!(!cube.has_floating_layer());
    }

    #[test]
    fn overall_opacity_equals_one_minus_flat_distance() {
        let w = layered_wish();
        let observed = ObservedTopology::from_facets([WishFacet::crate_("kosmo-api")]);
        let a = assess_wish(&w, &observed, assess_bundle());
        let cube = assess_wish_layered(&w, &observed, assess_bundle());
        assert_eq!(cube.overall_opacity, Q16::ONE.saturating_sub(a.distance));
        assert_eq!(cube.assessment_id, a.id, "cube links the flat assessment");
    }

    #[test]
    fn structural_solidity_is_silenced_by_one_hollow_layer() {
        // Existence solid, Wiring + Live hollow → geomean ZERO, though the
        // weighted-mean opacity is a healthy fraction.
        let w = layered_wish();
        let observed = ObservedTopology::from_facets([WishFacet::crate_("kosmo-api")]);
        let cube = assess_wish_layered(&w, &observed, assess_bundle());
        assert_eq!(cube.layer(WishLayer::Existence).unwrap().opacity, Q16::ONE);
        assert_eq!(
            cube.structural_solidity,
            Q16::ZERO,
            "one hollow layer silences solidity"
        );
        assert!(
            cube.overall_opacity.is_positive(),
            "but the mean opacity is non-zero"
        );
    }

    #[test]
    fn floating_layer_detected_when_deep_renders_over_hollow_shallow() {
        // Live met while Existence is hollow → a layer floating in mid-air.
        let w = layered_wish();
        let observed =
            ObservedTopology::from_facets([WishFacet::new(WishFacetKind::Run, "ping=>out~pong")]);
        let cube = assess_wish_layered(&w, &observed, assess_bundle());
        assert!(cube.layer(WishLayer::Live).unwrap().met_count > 0);
        assert!(!cube.layer(WishLayer::Existence).unwrap().is_solid());
        assert!(cube.has_floating_layer());
        assert_eq!(cube.solid_frontier(), None, "nothing solid at the base");
    }

    #[test]
    fn cube_id_deterministic_and_verifies() {
        let w = layered_wish();
        let o = ObservedTopology::from_facets([WishFacet::crate_("kosmo-api")]);
        let c1 = assess_wish_layered(&w, &o, assess_bundle());
        let c2 = assess_wish_layered(&w, &o, assess_bundle());
        assert_eq!(c1.id, c2.id);
        assert!(c1.verify_id());
    }

    #[test]
    fn cube_evidence_mandatory() {
        let cube =
            assess_wish_layered(&layered_wish(), &ObservedTopology::empty(), assess_bundle());
        assert!(cube.is_evidence_bound());
        assert_ne!(cube.evidence_bundle_id, Digest::ZERO);
    }

    #[test]
    fn cube_is_failclosed_on_unrelated_facet() {
        // An observed facet not in the wish never raises any opacity.
        let w = layered_wish();
        let observed = ObservedTopology::from_facets([WishFacet::crate_("totally-unrelated")]);
        let cube = assess_wish_layered(&w, &observed, assess_bundle());
        assert_eq!(cube.overall_opacity, Q16::ZERO);
        assert_eq!(cube.structural_solidity, Q16::ZERO);
    }
}

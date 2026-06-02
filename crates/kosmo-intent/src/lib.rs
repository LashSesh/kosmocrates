//! Kosmocrates intent layer — connect the wish ruler to the real workspace.
//!
//! Runs 1 and 2 built the *target* ([`kosmo_core::Wish`]), the *ruler*
//! ([`kosmo_core::assess_wish`]), and the *convergence contract*
//! ([`kosmo_core::WishConvergenceTrace`]) — all pure, all measured against a
//! hand-supplied [`ObservedTopology`]. This crate fills the last gap: it reads a
//! **real** workspace and turns it into that observation, and it ties the three
//! pieces together in a stateful [`WishSession`] that tracks convergence — and,
//! fail-closed, divergence — across successive iterations.
//!
//! Observation granularity: this layer observes the workspace's **crate
//! topology** via [`kosmo_parseback`] (crate presence is content-addressed and
//! name-preserving). Finer-grained `Module` / `Symbol` facets need a
//! name-preserving source extractor and are a later run; the observation API is
//! a facet *set*, so those sources merge in without an interface change.
//!
//! Like everything on the selection path, this **ranks, it never gates**: it
//! measures how far a workspace is from a wish and whether it is moving toward
//! the attractor, but grants no capability and bypasses no policy.

use std::collections::BTreeSet;
use std::path::PathBuf;

use kosmo_core::{
    assess_wish, Digest, ObservedTopology, ParseBackScanScope, Wish, WishAssessment,
    WishConvergenceTrace, WishFacet,
};
use kosmo_parseback::{ParseBackError, ParseBackExecutor, TopologySnapshot};
use serde::{Deserialize, Serialize};

// ─── Observation adapter ────────────────────────────────────────────────────

/// Build the present-facet set from a crate topology snapshot.
///
/// Emits one [`WishFacet::crate_`] per crate node. (Dependency edges and
/// per-crate file fingerprints are not surfaced as facets here — a wish targets
/// *presence*, and `Module` / `Symbol` facets need a name-preserving source
/// extractor, which is a later run.)
pub fn facets_from_snapshot(snapshot: &TopologySnapshot) -> BTreeSet<WishFacet> {
    snapshot
        .crate_nodes
        .keys()
        .map(|name| WishFacet::crate_(name.clone()))
        .collect()
}

/// Build an [`ObservedTopology`] from a crate topology snapshot.
pub fn observe_snapshot(snapshot: &TopologySnapshot) -> ObservedTopology {
    ObservedTopology::from_facets(facets_from_snapshot(snapshot))
}

/// Scan a real workspace and observe its crate topology.
///
/// Read-only: runs `cargo metadata` once via [`ParseBackExecutor`], no host
/// mutations. Uses [`ParseBackScanScope::AffectedFilesOnly`] because the
/// observed facet set is the crate *names*, which are scope-independent — this
/// just avoids walking every source file.
pub fn observe_workspace(root: impl Into<PathBuf>) -> Result<ObservedTopology, ParseBackError> {
    observe_workspace_scoped(root, ParseBackScanScope::AffectedFilesOnly)
}

/// As [`observe_workspace`], with an explicit scan scope.
pub fn observe_workspace_scoped(
    root: impl Into<PathBuf>,
    scope: ParseBackScanScope,
) -> Result<ObservedTopology, ParseBackError> {
    let snapshot = ParseBackExecutor::new(root.into()).snapshot(&scope)?;
    Ok(observe_snapshot(&snapshot))
}

// ─── WishSession ─────────────────────────────────────────────────────────────

/// A stateful descent toward a wish-attractor across successive observations.
///
/// Ties the three layers together: each [`WishSession::observe`] measures the
/// workspace against the wish (Run 1), appends the resulting distance to the
/// trajectory, and exposes a [`WishConvergenceTrace`] (Run 2) so the caller can
/// ask whether the system is converging — or has **diverged** from the
/// attractor, which the agent loop must treat fail-closed.
///
/// Serializable so a descent can be persisted and resumed across sessions,
/// consistent with the substrate's content-addressed, replayable contract.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WishSession {
    wish: Wish,
    evidence_bundle_id: Digest,
    assessments: Vec<WishAssessment>,
}

impl WishSession {
    /// Start a descent toward `wish`. `evidence_bundle_id` binds every
    /// assessment and trace this session produces (CROSS-006).
    pub fn new(wish: Wish, evidence_bundle_id: Digest) -> Self {
        Self {
            wish,
            evidence_bundle_id,
            assessments: vec![],
        }
    }

    pub fn wish(&self) -> &Wish {
        &self.wish
    }

    /// Assess the wish against an observation, append it to the trajectory, and
    /// return the new assessment.
    pub fn observe(&mut self, observed: &ObservedTopology) -> &WishAssessment {
        let assessment = assess_wish(&self.wish, observed, self.evidence_bundle_id);
        self.assessments.push(assessment);
        self.assessments
            .last()
            .expect("just pushed an assessment")
    }

    /// All assessments recorded so far, oldest first.
    pub fn assessments(&self) -> &[WishAssessment] {
        &self.assessments
    }

    /// The most recent assessment, if any.
    pub fn latest(&self) -> Option<&WishAssessment> {
        self.assessments.last()
    }

    /// Number of observations recorded.
    pub fn iterations(&self) -> usize {
        self.assessments.len()
    }

    /// The convergence trace accumulated so far — the attractor contract.
    pub fn trace(&self) -> WishConvergenceTrace {
        WishConvergenceTrace::from_assessments(&self.assessments, self.evidence_bundle_id)
    }

    /// The system is at the wish-attractor: the latest observation realized the wish.
    pub fn at_attractor(&self) -> bool {
        self.trace().at_attractor()
    }

    /// No observed step has increased the distance: the contraction invariant holds.
    pub fn is_contractive(&self) -> bool {
        self.trace().is_contractive()
    }

    /// The wish is realized and the descent never diverged.
    pub fn is_converged(&self) -> bool {
        self.trace().is_converged()
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::{AttractorStatus, Q16, Wish, WishClosureStatus, WishFacet, WishPredicate};
    use kosmo_parseback::CrateFingerprint;
    use std::collections::{BTreeMap, BTreeSet};

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    /// A synthetic crate-topology snapshot containing the given crate names.
    fn snap(names: &[&str]) -> TopologySnapshot {
        let mut nodes = BTreeMap::new();
        for n in names {
            nodes.insert(
                n.to_string(),
                CrateFingerprint::new(n.to_string(), vec!["lib.rs".into()], vec![]),
            );
        }
        TopologySnapshot::from_parts(ParseBackScanScope::FullWorkspace, nodes, BTreeSet::new())
    }

    fn wish_two() -> Wish {
        Wish::new(
            "two crates",
            [
                WishPredicate::require(WishFacet::crate_("a")),
                WishPredicate::require(WishFacet::crate_("b")),
            ],
            d(b"policy"),
            d(b"bundle"),
        )
    }

    // ── observation adapter ───────────────────────────────────────────────

    #[test]
    fn facets_from_snapshot_emits_one_crate_facet_per_node() {
        let facets = facets_from_snapshot(&snap(&["a", "b"]));
        assert_eq!(facets.len(), 2);
        assert!(facets.contains(&WishFacet::crate_("a")));
        assert!(facets.contains(&WishFacet::crate_("b")));
    }

    #[test]
    fn observe_snapshot_contains_crate_facets() {
        let observed = observe_snapshot(&snap(&["alpha"]));
        assert!(observed.contains(&WishFacet::crate_("alpha")));
        assert!(!observed.contains(&WishFacet::crate_("missing")));
    }

    // ── WishSession descent ───────────────────────────────────────────────

    #[test]
    fn session_observe_appends_assessments() {
        let mut s = WishSession::new(wish_two(), d(b"ev"));
        assert_eq!(s.iterations(), 0);
        let a = s.observe(&observe_snapshot(&snap(&[])));
        assert_eq!(a.distance, Q16::ONE);
        assert_eq!(s.iterations(), 1);
        s.observe(&observe_snapshot(&snap(&["a", "b"])));
        assert_eq!(s.iterations(), 2);
        assert_eq!(s.latest().unwrap().distance, Q16::ZERO);
    }

    #[test]
    fn session_converges_over_observations() {
        let mut s = WishSession::new(wish_two(), d(b"ev"));
        s.observe(&observe_snapshot(&snap(&[]))); // ONE
        s.observe(&observe_snapshot(&snap(&["a"]))); // HALF
        s.observe(&observe_snapshot(&snap(&["a", "b"]))); // ZERO
        let trace = s.trace();
        assert_eq!(trace.distances, vec![Q16::ONE, Q16::HALF, Q16::ZERO]);
        assert_eq!(trace.status, AttractorStatus::Converged);
        assert!(s.is_contractive());
        assert!(s.is_converged());
        assert!(s.at_attractor());
    }

    #[test]
    fn session_detects_divergence_fail_closed() {
        // Realize the wish, then regress: the distance rises → divergence.
        let mut s = WishSession::new(wish_two(), d(b"ev"));
        s.observe(&observe_snapshot(&snap(&["a", "b"]))); // ZERO
        s.observe(&observe_snapshot(&snap(&["a"]))); // HALF — moved away
        let trace = s.trace();
        assert_eq!(trace.status, AttractorStatus::Diverging);
        assert_eq!(trace.first_divergence, Some(1));
        assert!(!s.is_contractive());
    }

    #[test]
    fn session_partial_observation_is_approaching() {
        let mut s = WishSession::new(wish_two(), d(b"ev"));
        let a = s.observe(&observe_snapshot(&snap(&["a"])));
        assert_eq!(a.status, WishClosureStatus::Approaching);
        assert_eq!(a.distance, Q16::HALF);
        assert_eq!(a.unmet_facets, vec![WishFacet::crate_("b")]);
    }

    #[test]
    fn session_is_serde_round_trippable() {
        let mut s = WishSession::new(wish_two(), d(b"ev"));
        s.observe(&observe_snapshot(&snap(&["a"])));
        let json = serde_json::to_string(&s).unwrap();
        let back: WishSession = serde_json::from_str(&json).unwrap();
        assert_eq!(back.iterations(), 1);
        assert_eq!(back.wish().id, s.wish().id);
        assert_eq!(back.latest().unwrap().distance, Q16::HALF);
    }

    // ── real-workspace observation (graceful skip if cargo unavailable) ────

    #[test]
    fn observe_workspace_reads_real_crate_topology() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let observed = match observe_workspace(&root) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("cargo metadata unavailable, skipping: {e}");
                return;
            }
        };
        // The workspace contains these crates (kosmo-intent is this crate).
        assert!(observed.contains(&WishFacet::crate_("kosmo-core")));
        assert!(observed.contains(&WishFacet::crate_("kosmo-intent")));

        // A wish over real + imaginary crates lands Approaching: 2 met, 1 unmet.
        let wish = Wish::new(
            "exists in the real workspace",
            [
                WishPredicate::require(WishFacet::crate_("kosmo-core")),
                WishPredicate::require(WishFacet::crate_("kosmo-intent")),
                WishPredicate::require(WishFacet::crate_("kosmo-does-not-exist-xyz")),
            ],
            d(b"policy"),
            d(b"bundle"),
        );
        let mut s = WishSession::new(wish, d(b"ev"));
        let a = s.observe(&observed);
        assert_eq!(a.status, WishClosureStatus::Approaching);
        assert_eq!(a.met_count, 2);
        assert_eq!(a.total_count, 3);
        assert_eq!(
            a.unmet_facets,
            vec![WishFacet::crate_("kosmo-does-not-exist-xyz")]
        );
    }
}

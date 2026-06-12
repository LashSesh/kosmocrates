//! Landscape geometry — spectral structure over the wish landscape.
//!
//! Assimilated from Wonderlamp's hypercube *consumer* side, with its domain
//! registries and rigid dimension-category tables left behind: here the
//! coupled objects are [`WishProposal`]s, and **every coupling feature is
//! computed from `WishProposal` fields alone** (the structural guard against
//! a new domain registry growing back — pinned by test).
//!
//! Coupling weights (integer percent components, exact at the boundaries,
//! same arithmetic discipline as the swarm consensus):
//!
//! - **subject affinity, 45** — the proposals concern the same organ
//!   (equal subjects, or subjects sharing an identifier subword ≥ 3 chars,
//!   boundary-aware);
//! - **kind affinity, 30** — the same facet kind (the projection of the
//!   originating void class);
//! - **severity proximity, 25 × (1 − |Δseverity|)** — but proximity alone
//!   never couples: without subject or kind affinity there is no edge
//!   (two unrelated findings of similar badness are not a cluster).
//!
//! On that graph, `kosmo-spectral` finds the *shape* of the work:
//! [`landscape_geometry`] returns conductance-bounded spectral clusters
//! (coherent sub-wishes — adopt ONE cluster as ONE wish instead of a blind
//! top-k) and the articulation singularities (proposals whose removal
//! disconnects the landscape: the most consequential decisions first).
//! Everything here is advisory structure over the already-measured
//! landscape (CROSS-010): it ranks and groups, it gates nothing, and the
//! landscape itself is byte-identical whether or not geometry is computed.

use kosmo_core::Q16;
use kosmo_spectral::{singularities, spectral_clusters, CouplingGraph};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::WishProposal;

/// Integer percent weights of the coupling components
/// (subject, kind, severity proximity) — they sum to 100.
pub const COUPLING_WEIGHTS: (i64, i64, i64) = (45, 30, 25);

/// Lowercase identifier subwords (≥ 3 chars, not purely numeric) of a
/// subject string — the same boundary discipline as norm learning.
fn subject_subwords(subject: &str) -> BTreeSet<String> {
    subject
        .to_ascii_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 3)
        .filter(|w| !w.chars().all(|c| c.is_ascii_digit()))
        .map(String::from)
        .collect()
}

/// Two subjects name the same organ: equal, or sharing a subword.
fn subjects_affine(a: &str, b: &str) -> bool {
    if a.eq_ignore_ascii_case(b) {
        return true;
    }
    !subject_subwords(a).is_disjoint(&subject_subwords(b))
}

/// The coupling weight between two proposals — computed from
/// `WishProposal` fields ONLY. `ZERO` (no edge) when neither subject nor
/// kind bind them; severity proximity only ever modulates an existing bond.
pub fn coupling(a: &WishProposal, b: &WishProposal) -> Q16 {
    let subject = subjects_affine(&a.subject, &b.subject);
    let kind = a.facet.kind == b.facet.kind;
    if !subject && !kind {
        return Q16::ZERO;
    }
    let (w_subject, w_kind, w_proximity) = COUPLING_WEIGHTS;
    let proximity_raw = Q16::ONE
        .saturating_sub(a.severity.saturating_sub(b.severity).abs())
        .raw()
        .max(0);
    let subject_raw = if subject { Q16::ONE.raw() } else { 0 };
    let kind_raw = if kind { Q16::ONE.raw() } else { 0 };
    // One division at the end — exact at the boundaries (consensus pattern).
    Q16::from_raw((w_subject * subject_raw + w_kind * kind_raw + w_proximity * proximity_raw) / 100)
}

/// Build the coupling graph over a slice of proposals (the caller chooses
/// the population — kosmo-run couples the *open* proposals). Node `i` is
/// `proposals[i]`.
pub fn couple_proposals(proposals: &[WishProposal]) -> CouplingGraph {
    let mut graph = CouplingGraph::new(proposals.len());
    for i in 0..proposals.len() {
        for j in (i + 1)..proposals.len() {
            let w = coupling(&proposals[i], &proposals[j]);
            if w.is_positive() {
                graph.add_edge(i, j, w);
            }
        }
    }
    graph
}

/// One coherent cluster of the landscape: a sub-wish candidate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProposalCluster {
    /// Indices into the proposal slice the geometry was computed over.
    pub members: Vec<usize>,
    /// The distinct subjects inside the cluster, sorted — the human label.
    pub subjects: Vec<String>,
    /// Saturating sum of member severities — the cluster's standing mass
    /// (ordering key: the heaviest coherent work first).
    pub severity_mass: Q16,
}

/// A proposal whose removal disconnects the landscape's coupling structure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SingularProposal {
    /// Index into the proposal slice the geometry was computed over.
    pub index: usize,
    /// Coupling mass hinging on this proposal (`Q16`, saturating).
    pub coupling_mass: Q16,
}

/// The spectral shape of (a slice of) the landscape.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LandscapeGeometry {
    /// Conductance-bounded spectral clusters, heaviest severity mass first
    /// (ties: smallest first member).
    pub clusters: Vec<ProposalCluster>,
    /// Articulation proposals, heaviest coupling mass first.
    pub singular: Vec<SingularProposal>,
}

/// Compute the landscape's geometry: couple the proposals, cluster with
/// conductance-bounded recursive bisection (at most `max_clusters`, but the
/// count *emerges* — tight clusters refuse to shatter), and surface the
/// articulation singularities. Pure and deterministic.
pub fn landscape_geometry(proposals: &[WishProposal], max_clusters: usize) -> LandscapeGeometry {
    if proposals.is_empty() {
        return LandscapeGeometry::default();
    }
    let graph = couple_proposals(proposals);
    let mut clusters: Vec<ProposalCluster> = spectral_clusters(&graph, max_clusters)
        .into_iter()
        .map(|members| {
            let subjects: Vec<String> = members
                .iter()
                .map(|&i| proposals[i].subject.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let severity_mass = members.iter().fold(Q16::ZERO, |acc, &i| {
                acc.saturating_add(proposals[i].severity)
            });
            ProposalCluster {
                members,
                subjects,
                severity_mass,
            }
        })
        .collect();
    clusters.sort_by(|a, b| {
        b.severity_mass
            .cmp(&a.severity_mass)
            .then(a.members[0].cmp(&b.members[0]))
    });

    let singular = singularities(&graph)
        .into_iter()
        .map(|s| SingularProposal {
            index: s.node,
            coupling_mass: s.coupling_mass,
        })
        .collect();

    LandscapeGeometry { clusters, singular }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::{Digest, WishFacet};

    fn proposal(facet: WishFacet, subject: &str, severity: Q16) -> WishProposal {
        // The test constructor mirrors propose_wishes' projection shape.
        WishProposal {
            proposal_id: Digest::of_bytes(format!("{subject}-{:?}", facet).as_bytes()),
            facet,
            void_id: Digest::of_bytes(subject.as_bytes()),
            severity,
            subject: subject.to_string(),
            rationale: format!("test @ {subject}"),
        }
    }

    fn q(n: u64, d: u64) -> Q16 {
        Q16::ratio(n, d).unwrap()
    }

    #[test]
    fn coupling_components_are_exact_and_proximity_never_couples_alone() {
        let doc_router = proposal(WishFacet::doc("router"), "router", q(9, 10));
        let test_router = proposal(WishFacet::test("router_smoke"), "router", q(9, 10));
        let doc_parser = proposal(WishFacet::doc("parser"), "parser", q(9, 10));
        let test_parser = proposal(WishFacet::test("parser_smoke"), "parser", q(9, 10));

        // Same subject, different kind, equal severity: 45 + 25 = 70%.
        assert_eq!(coupling(&doc_router, &test_router), q(70, 100));
        // Different subject, same kind, equal severity: 30 + 25 = 55%.
        assert_eq!(coupling(&doc_router, &doc_parser), q(55, 100));
        // Nothing in common but severity: NO edge.
        assert_eq!(coupling(&doc_router, &test_parser), Q16::ZERO);
        // Severity distance modulates: same subject+kind at distance 1 ⇒ 75%.
        let hot = proposal(WishFacet::doc("router"), "router", Q16::ONE);
        let cold = proposal(WishFacet::doc("router"), "router", Q16::ZERO);
        assert_eq!(coupling(&hot, &cold), q(75, 100));
        // Symmetry.
        assert_eq!(coupling(&cold, &hot), q(75, 100));
    }

    #[test]
    fn subject_affinity_is_boundary_aware() {
        let a = proposal(WishFacet::doc("user_router"), "user_router", Q16::HALF);
        let b = proposal(WishFacet::test("router_smoke"), "router", Q16::HALF);
        let c = proposal(WishFacet::test("username_smoke"), "username", Q16::HALF);
        assert!(
            coupling(&a, &b) > q(45, 100),
            "shared subword 'router' binds"
        );
        // "username" does not contain the subword "user" at a boundary…
        // but they still share NO subword, and kinds differ for a vs c.
        assert_eq!(coupling(&a, &c), Q16::ZERO);
    }

    #[test]
    fn geometry_clusters_per_module_not_per_kind() {
        // Two modules, each with a doc and a test proposal. The per-module
        // bond (subject) must beat the per-kind bond (doc–doc / test–test).
        let proposals = vec![
            proposal(WishFacet::doc("router"), "router", q(9, 10)),
            proposal(WishFacet::test("router_smoke"), "router", q(9, 10)),
            proposal(WishFacet::doc("parser"), "parser", q(5, 10)),
            proposal(WishFacet::test("parser_smoke"), "parser", q(5, 10)),
        ];
        let geometry = landscape_geometry(&proposals, 6);
        let memberships: Vec<Vec<usize>> = geometry
            .clusters
            .iter()
            .map(|c| c.members.clone())
            .collect();
        assert_eq!(
            memberships,
            vec![vec![0, 1], vec![2, 3]],
            "modules cluster together; the heavier (router) cluster leads"
        );
        assert_eq!(geometry.clusters[0].subjects, vec!["router".to_string()]);
        assert_eq!(
            geometry.clusters[0].severity_mass,
            q(9, 10).saturating_add(q(9, 10))
        );
    }

    #[test]
    fn singular_proposals_are_articulation_points() {
        // A couples to B (subject), B couples to C (kind), A–C share nothing:
        // a path — B is the hinge of the landscape.
        let proposals = vec![
            proposal(WishFacet::doc("alpha"), "alpha", Q16::HALF),
            proposal(WishFacet::test("alpha_smoke"), "alpha", Q16::HALF),
            proposal(WishFacet::test("beta_smoke"), "beta", Q16::HALF),
        ];
        assert_eq!(coupling(&proposals[0], &proposals[2]), Q16::ZERO);
        let geometry = landscape_geometry(&proposals, 6);
        assert_eq!(geometry.singular.len(), 1);
        assert_eq!(
            geometry.singular[0].index, 1,
            "the test of alpha is the hinge"
        );
        assert!(geometry.singular[0].coupling_mass.is_positive());
    }

    #[test]
    fn geometry_is_deterministic_and_total_on_empty() {
        let proposals = vec![
            proposal(WishFacet::doc("router"), "router", q(9, 10)),
            proposal(WishFacet::test("router_smoke"), "router", q(8, 10)),
        ];
        assert_eq!(
            landscape_geometry(&proposals, 4),
            landscape_geometry(&proposals, 4)
        );
        assert_eq!(landscape_geometry(&[], 4), LandscapeGeometry::default());
    }

    /// CRUD-relapse guard #5: the coupling layer must never grow a domain
    /// registry — features come from WishProposal fields only, and the
    /// source carries no domain/stack vocabulary.
    #[test]
    fn geometry_source_carries_no_domain_registry() {
        let body = include_str!("landscape_geometry.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap()
            .to_ascii_lowercase();
        for forbidden in [
            "database",
            "backend",
            "frontend",
            "ecommerce",
            "warehouse",
            "actix",
            "sqlx",
            "domain_registry",
            "dimcategory",
            "appspec",
        ] {
            assert!(
                !body.contains(forbidden),
                "forbidden domain vocabulary '{forbidden}' in landscape_geometry.rs"
            );
        }
    }
}

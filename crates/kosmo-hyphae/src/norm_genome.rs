//! Norm genome — co-activation genes, relations, composition.
//!
//! Assimilated from Wonderlamp's `isls-norms/genome.rs` + `relations.rs` +
//! `compose_norms`, in pure `Q16` integer arithmetic (CROSS-007 — the
//! predecessor used `f64` Jaccard). The genome never *does* anything by
//! itself: genes and relations are advisory structure over the catalog
//! (CROSS-010 — they rank and inform, they gate nothing), and composition
//! emits facet templates only, like everything else in the norm organ.
//!
//! - [`cluster_genes`]: norms that fire together across runs (pairwise
//!   Jaccard over run sets ≥ threshold, single-link) form a [`NormGene`].
//! - [`relate`]: `Dependent` (one requires the other) > `Conflicting`
//!   (same trigger or same shape under different identities) > `Compatible`
//!   (shared facet kind) > `Independent`.
//! - [`compose`]: BFS over `requires`, conflict detection, merged template
//!   list — a transient bundle; making it a durable norm is the caller's
//!   governed step.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use kosmo_core::{Digest, Q16};
use serde::{Deserialize, Serialize};

use crate::norm_schema::{Norm, NormFacetTemplate};

/// One run's worth of norm activations (e.g. all norms whose facets a single
/// descent realized). `run_tag` is the run's digest identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormActivation {
    pub run_tag: Digest,
    pub norm_ids: Vec<Digest>,
}

impl NormActivation {
    pub fn new(run_tag: Digest, norm_ids: impl IntoIterator<Item = Digest>) -> Self {
        let mut norm_ids: Vec<Digest> = norm_ids.into_iter().collect();
        norm_ids.sort();
        norm_ids.dedup();
        Self { run_tag, norm_ids }
    }
}

/// Serialize-only content for the gene id.
#[derive(Serialize)]
struct GeneContent<'a> {
    member_norm_ids: &'a Vec<Digest>,
    cohesion_raw: i64,
    evidence_bundle_id: &'a Digest,
    policy_id: &'a Digest,
}

/// A cluster of norms that co-activate: the genome's unit of heredity.
/// Advisory only — membership confers no trust and no trigger.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormGene {
    pub gene_id: Digest,
    pub member_norm_ids: Vec<Digest>,
    /// Mean pairwise co-activation Jaccard inside the gene (`ONE` for a
    /// singleton), integer-averaged.
    pub cohesion: Q16,
    pub evidence_bundle_id: Digest,
    pub policy_id: Digest,
}

impl NormGene {
    fn new(
        member_norm_ids: Vec<Digest>,
        cohesion: Q16,
        evidence_bundle_id: Digest,
        policy_id: Digest,
    ) -> Self {
        let gene_id = Digest::of(&GeneContent {
            member_norm_ids: &member_norm_ids,
            cohesion_raw: cohesion.raw(),
            evidence_bundle_id: &evidence_bundle_id,
            policy_id: &policy_id,
        });
        Self {
            gene_id,
            member_norm_ids,
            cohesion,
            evidence_bundle_id,
            policy_id,
        }
    }
}

/// Wonderlamp's gene threshold (0.8), as an integer ratio.
pub fn default_gene_threshold() -> Q16 {
    Q16::ratio(4, 5).expect("4/5 is representable")
}

/// Co-activation Jaccard of two norms over the run corpus:
/// `|runs with both| / |runs with either|` (`ZERO` when neither ever fired).
fn coactivation(runs_a: &BTreeSet<Digest>, runs_b: &BTreeSet<Digest>) -> Q16 {
    let both = runs_a.intersection(runs_b).count() as u64;
    let either = runs_a.union(runs_b).count() as u64;
    if either == 0 {
        return Q16::ZERO;
    }
    Q16::ratio(both, either).unwrap_or(Q16::ZERO)
}

/// Single-link clustering of norms by co-activation: an edge exists where the
/// pairwise Jaccard ≥ `threshold`; connected components become genes, sorted
/// by their (sorted) member lists. Deterministic and replayable.
pub fn cluster_genes(
    activations: &[NormActivation],
    threshold: Q16,
    evidence_bundle_id: Digest,
    policy_id: Digest,
) -> Vec<NormGene> {
    // Which runs did each norm fire in?
    let mut runs: BTreeMap<Digest, BTreeSet<Digest>> = BTreeMap::new();
    for act in activations {
        for id in &act.norm_ids {
            runs.entry(*id).or_default().insert(act.run_tag);
        }
    }
    let ids: Vec<Digest> = runs.keys().copied().collect();
    let n = ids.len();

    // Pairwise Jaccard + threshold adjacency.
    let mut jaccard: BTreeMap<(usize, usize), Q16> = BTreeMap::new();
    let mut adjacent: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in 0..n {
        for j in (i + 1)..n {
            let q = coactivation(&runs[&ids[i]], &runs[&ids[j]]);
            jaccard.insert((i, j), q);
            if q.at_least(threshold) {
                adjacent[i].push(j);
                adjacent[j].push(i);
            }
        }
    }

    // Connected components (BFS).
    let mut seen = vec![false; n];
    let mut genes = Vec::new();
    for start in 0..n {
        if seen[start] {
            continue;
        }
        let mut component = Vec::new();
        let mut queue = VecDeque::from([start]);
        seen[start] = true;
        while let Some(i) = queue.pop_front() {
            component.push(i);
            for &j in &adjacent[i] {
                if !seen[j] {
                    seen[j] = true;
                    queue.push_back(j);
                }
            }
        }
        component.sort_unstable();
        let members: Vec<Digest> = component.iter().map(|&i| ids[i]).collect();
        // Integer mean of all pairwise Jaccards inside the component.
        let cohesion = if component.len() < 2 {
            Q16::ONE
        } else {
            let mut sum: i64 = 0;
            let mut pairs: i64 = 0;
            for (a, &i) in component.iter().enumerate() {
                for &j in component.iter().skip(a + 1) {
                    sum += jaccard[&(i.min(j), i.max(j))].raw();
                    pairs += 1;
                }
            }
            Q16::from_raw(sum / pairs)
        };
        genes.push(NormGene::new(
            members,
            cohesion,
            evidence_bundle_id,
            policy_id,
        ));
    }
    genes.sort_by(|a, b| a.member_norm_ids.cmp(&b.member_norm_ids));
    genes
}

/// How two norms in a catalog stand to each other. Advisory taxonomy
/// (CROSS-010): relations inform composition and review, they gate nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NormRelation {
    /// One requires the other.
    Dependent,
    /// Both claim the same trigger word, or the same template shape lives
    /// under two different identities.
    Conflicting,
    /// They share at least one facet kind — instantiations overlap in nature.
    Compatible,
    Independent,
}

/// Classify the relation between two norms (a norm relates to itself as
/// `Compatible`). Precedence: Dependent > Conflicting > Compatible.
pub fn relate(a: &Norm, b: &Norm) -> NormRelation {
    if a.norm_id != b.norm_id {
        if a.requires.contains(&b.norm_id) || b.requires.contains(&a.norm_id) {
            return NormRelation::Dependent;
        }
        let same_trigger = matches!((&a.trigger, &b.trigger), (Some(x), Some(y)) if x == y);
        if same_trigger || a.template == b.template {
            return NormRelation::Conflicting;
        }
    }
    let kinds_a: BTreeSet<_> = a.template.iter().map(|t| &t.kind).collect();
    let kinds_b: BTreeSet<_> = b.template.iter().map(|t| &t.kind).collect();
    if kinds_a.intersection(&kinds_b).next().is_some() {
        NormRelation::Compatible
    } else {
        NormRelation::Independent
    }
}

/// Why a composition failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComposeError {
    /// A required norm id is not in the catalog — fail closed, never skip.
    MissingRequirement { norm_id: Digest },
    /// Two gathered norms conflict.
    ConflictingMembers { a: Digest, b: Digest },
}

impl std::fmt::Display for ComposeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingRequirement { norm_id } => {
                write!(f, "composition requires unknown norm {norm_id:?}")
            }
            Self::ConflictingMembers { a, b } => {
                write!(f, "composition members conflict: {a:?} vs {b:?}")
            }
        }
    }
}

impl std::error::Error for ComposeError {}

/// Compose the named root norms with everything they (transitively) require:
/// BFS over `requires` resolving against `catalog`, pairwise conflict check,
/// merged sorted template list. The result is a transient bundle — turning it
/// into a durable norm is a separate, governed act.
pub fn compose(
    roots: &[Digest],
    catalog: &BTreeMap<Digest, Norm>,
) -> Result<Vec<NormFacetTemplate>, ComposeError> {
    let mut gathered: BTreeMap<Digest, &Norm> = BTreeMap::new();
    let mut queue: VecDeque<Digest> = roots.iter().copied().collect();
    while let Some(id) = queue.pop_front() {
        if gathered.contains_key(&id) {
            continue;
        }
        let norm = catalog
            .get(&id)
            .ok_or(ComposeError::MissingRequirement { norm_id: id })?;
        gathered.insert(id, norm);
        queue.extend(norm.requires.iter().copied());
    }

    let members: Vec<&Norm> = gathered.values().copied().collect();
    for (i, a) in members.iter().enumerate() {
        for b in members.iter().skip(i + 1) {
            if relate(a, b) == NormRelation::Conflicting {
                return Err(ComposeError::ConflictingMembers {
                    a: a.norm_id,
                    b: b.norm_id,
                });
            }
        }
    }

    let templates: BTreeSet<NormFacetTemplate> = members
        .iter()
        .flat_map(|n| n.template.iter().cloned())
        .collect();
    Ok(templates.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::norm_schema::{NormLevel, NormOrigin};
    use kosmo_core::WishFacetKind;

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    fn norm_with(name: &str, templates: Vec<NormFacetTemplate>, requires: Vec<Digest>) -> Norm {
        Norm::new(
            name,
            "",
            NormLevel::Molecule,
            "test",
            templates,
            requires,
            [],
            d(b"cand"),
            d(b"ev"),
            d(b"pol"),
            NormOrigin::OperatorInjected {
                source: "test".into(),
            },
        )
    }

    fn act(run: &[u8], ids: &[Digest]) -> NormActivation {
        NormActivation::new(d(run), ids.iter().copied())
    }

    #[test]
    fn coactivated_norms_cluster_into_one_gene() {
        let (a, b, c) = (d(b"norm-a"), d(b"norm-b"), d(b"norm-c"));
        // a+b fire together in every run; c fires alone.
        let activations = [
            act(b"r1", &[a, b]),
            act(b"r2", &[a, b]),
            act(b"r3", &[a, b]),
            act(b"r4", &[c]),
        ];
        let genes = cluster_genes(&activations, default_gene_threshold(), d(b"ev"), d(b"pol"));
        assert_eq!(genes.len(), 2);
        let pair = genes.iter().find(|g| g.member_norm_ids.len() == 2).unwrap();
        let mut expected = vec![a, b];
        expected.sort();
        assert_eq!(pair.member_norm_ids, expected);
        assert_eq!(pair.cohesion, Q16::ONE);
        let solo = genes.iter().find(|g| g.member_norm_ids.len() == 1).unwrap();
        assert_eq!(solo.member_norm_ids, vec![c]);
        assert_eq!(solo.cohesion, Q16::ONE);
        assert_ne!(pair.gene_id, solo.gene_id);
    }

    #[test]
    fn weak_coactivation_stays_below_the_threshold() {
        let (a, b) = (d(b"norm-a"), d(b"norm-b"));
        // Together in 1 of 3 runs: Jaccard 1/3 < 4/5.
        let activations = [act(b"r1", &[a, b]), act(b"r2", &[a]), act(b"r3", &[b])];
        let genes = cluster_genes(&activations, default_gene_threshold(), d(b"ev"), d(b"pol"));
        assert_eq!(genes.len(), 2, "1/3 co-activation must not fuse a gene");
        // A permissive threshold fuses them — the knob is real.
        let fused = cluster_genes(&activations, Q16::ratio(1, 4).unwrap(), d(b"ev"), d(b"pol"));
        assert_eq!(fused.len(), 1);
        assert_eq!(fused[0].member_norm_ids.len(), 2);
        assert_eq!(fused[0].cohesion, Q16::ratio(1, 3).unwrap());
    }

    #[test]
    fn clustering_is_deterministic() {
        let (a, b, c) = (d(b"x"), d(b"y"), d(b"z"));
        let activations = [act(b"r1", &[a, b, c]), act(b"r2", &[a, b])];
        let g1 = cluster_genes(&activations, Q16::HALF, d(b"ev"), d(b"pol"));
        let g2 = cluster_genes(&activations, Q16::HALF, d(b"ev"), d(b"pol"));
        assert_eq!(g1, g2);
    }

    #[test]
    fn relations_cover_the_four_cases() {
        let module = |p: &str| NormFacetTemplate::new(WishFacetKind::Module, p);
        let test_t = |p: &str| NormFacetTemplate::new(WishFacetKind::Test, p);

        let base = norm_with("base", vec![module("{name}")], vec![]);
        let dependent = norm_with("dep", vec![test_t("{name}_works")], vec![base.norm_id]);
        assert_eq!(relate(&base, &dependent), NormRelation::Dependent);
        assert_eq!(relate(&dependent, &base), NormRelation::Dependent);

        // Same shape under a different identity (name differs → id differs).
        let twin = norm_with("twin", vec![module("{name}")], vec![]);
        assert_eq!(relate(&base, &twin), NormRelation::Conflicting);

        // Same trigger word, disjoint shapes.
        let armed_a = norm_with("a", vec![module("{name}_a")], vec![]).with_trigger("organ");
        let armed_b = norm_with("b", vec![test_t("{name}_b")], vec![]).with_trigger("organ");
        assert_eq!(relate(&armed_a, &armed_b), NormRelation::Conflicting);

        // Shared facet kind, different shapes.
        let sibling = norm_with("sib", vec![module("{name}_core")], vec![]);
        assert_eq!(relate(&base, &sibling), NormRelation::Compatible);

        // Nothing in common.
        let stranger = norm_with("str", vec![test_t("{name}_works")], vec![]);
        assert_eq!(relate(&base, &stranger), NormRelation::Independent);
    }

    #[test]
    fn compose_merges_requirements_transitively() {
        let module = |p: &str| NormFacetTemplate::new(WishFacetKind::Module, p);
        let test_t = |p: &str| NormFacetTemplate::new(WishFacetKind::Test, p);
        let cap = |p: &str| NormFacetTemplate::new(WishFacetKind::Capability, p);

        let leaf = norm_with("leaf", vec![module("{name}")], vec![]);
        let mid = norm_with("mid", vec![test_t("{name}_works")], vec![leaf.norm_id]);
        let root = norm_with("root", vec![cap("organ:{name}")], vec![mid.norm_id]);
        let catalog: BTreeMap<Digest, Norm> = [&leaf, &mid, &root]
            .into_iter()
            .map(|n| (n.norm_id, n.clone()))
            .collect();

        let templates = compose(&[root.norm_id], &catalog).unwrap();
        assert_eq!(templates.len(), 3, "root + mid + leaf templates merged");
        assert!(templates.contains(&module("{name}")));
        assert!(templates.contains(&test_t("{name}_works")));
    }

    #[test]
    fn compose_fails_closed_on_missing_and_conflicting() {
        let module = |p: &str| NormFacetTemplate::new(WishFacetKind::Module, p);
        let ghost = d(b"not-in-catalog");
        let needy = norm_with("needy", vec![module("{name}")], vec![ghost]);
        let catalog: BTreeMap<Digest, Norm> = [(needy.norm_id, needy.clone())].into();
        assert_eq!(
            compose(&[needy.norm_id], &catalog),
            Err(ComposeError::MissingRequirement { norm_id: ghost })
        );

        let a = norm_with("a", vec![module("{name}")], vec![]);
        let twin = norm_with("twin", vec![module("{name}")], vec![]);
        let catalog: BTreeMap<Digest, Norm> = [&a, &twin]
            .into_iter()
            .map(|n| (n.norm_id, n.clone()))
            .collect();
        assert!(matches!(
            compose(&[a.norm_id, twin.norm_id], &catalog),
            Err(ComposeError::ConflictingMembers { .. })
        ));
    }
}

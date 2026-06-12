//! Norm learning — distilling archetypes from *realized* wishes.
//!
//! Assimilated from Wonderlamp's `isls-norms/learning.rs`
//! (`CrossLayerPattern` → `NormCandidate` → promotion thresholds), re-grounded
//! on the epistemic substrate: the observed unit is not a generated file tree
//! but a **realized facet bundle** — the predicates of a wish the descent
//! actually closed. Learning therefore can only ever produce facet-emitting
//! norms; there is no path from observation to a file template.
//!
//! The pipeline:
//! 1. Each realized descent records a content-addressed
//!    [`FacetBundleObservation`] (facets, workspace tag, languages).
//! 2. [`abstract_bundle`] finds the bundle's *subject* — the lowercase
//!    identifier subword shared by the most facet keys — and rewrites every
//!    key with the subject replaced by `{name}` (whole-subword, boundary
//!    aware). Bundles without a shared subject abstract to nothing: one
//!    observation of one thing is an event, not a pattern.
//! 3. [`promotable`] groups observations by abstracted shape and promotes a
//!    shape to a [`NormProposal`] once it has been **realized at least
//!    `min_observations` times across at least `min_workspaces` distinct
//!    workspaces with consistency ≥ `min_consistency`** (realized / total
//!    sightings of that shape).
//!
//! A proposal carries the [`Norm`] (origin [`NormOrigin::Learned`], trigger
//! `None` — arming is the operator's step) and its linked
//! [`NormGeneCandidate`], so the existing promotion-feedback fitness loop
//! applies unchanged. Proposals whose shape fails the anti-disease
//! [`Norm::validate`] gate are dropped, not emitted.

use std::collections::{BTreeMap, BTreeSet};

use kosmo_core::{Digest, WishFacet, Q16};
use serde::{Deserialize, Serialize};

use crate::norm::NormGeneCandidate;
use crate::norm_schema::{Norm, NormFacetTemplate, NormLevel, NormOrigin, NAME_PLACEHOLDER};

/// Serialize-only content for the observation id.
#[derive(Serialize)]
struct ObservationContent<'a> {
    workspace_tag: &'a Digest,
    facets: &'a Vec<WishFacet>,
    languages: &'a Vec<String>,
    realized: bool,
    evidence_bundle_id: &'a Digest,
    policy_id: &'a Digest,
}

/// One sighting of a facet bundle: the predicates of a wish, at the moment a
/// descent ended, in one workspace. `workspace_tag` is a digest of the
/// workspace identity (never a raw path — no host paths in durable artifacts).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FacetBundleObservation {
    pub observation_id: Digest,
    pub workspace_tag: Digest,
    pub facets: Vec<WishFacet>,
    /// Source languages seen in the workspace (via `SourceLanguage::as_str`),
    /// sorted — provenance for cross-language pattern tracking.
    pub languages: Vec<String>,
    /// Whether the wish was realized when recorded. Unrealized sightings count
    /// against a shape's consistency.
    pub realized: bool,
    pub evidence_bundle_id: Digest,
    pub policy_id: Digest,
}

impl FacetBundleObservation {
    pub fn new(
        workspace_tag: Digest,
        facets: impl IntoIterator<Item = WishFacet>,
        languages: impl IntoIterator<Item = String>,
        realized: bool,
        evidence_bundle_id: Digest,
        policy_id: Digest,
    ) -> Self {
        let mut facets: Vec<WishFacet> = facets.into_iter().collect();
        facets.sort();
        facets.dedup();
        let languages: Vec<String> = languages
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let mut o = Self {
            observation_id: Digest::ZERO,
            workspace_tag,
            facets,
            languages,
            realized,
            evidence_bundle_id,
            policy_id,
        };
        o.observation_id = o.compute_id();
        o
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&ObservationContent {
            workspace_tag: &self.workspace_tag,
            facets: &self.facets,
            languages: &self.languages,
            realized: self.realized,
            evidence_bundle_id: &self.evidence_bundle_id,
            policy_id: &self.policy_id,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.observation_id == self.compute_id()
    }
}

/// Promotion thresholds. Defaults mirror the plan: a pattern must recur
/// (≥3 realized sightings), generalize (≥2 workspaces) and hold (consistency
/// ≥ 3/4) before it may even become a *proposal* — and a proposal is still
/// trigger-less until an operator promotes it.
#[derive(Clone, Debug)]
pub struct NormLearningConfig {
    pub min_observations: u32,
    pub min_workspaces: u32,
    pub min_consistency: Q16,
}

impl Default for NormLearningConfig {
    fn default() -> Self {
        Self {
            min_observations: 3,
            min_workspaces: 2,
            min_consistency: Q16::ratio(3, 4).expect("3/4 is representable"),
        }
    }
}

/// A shape that crossed the learning thresholds: the trigger-less [`Norm`]
/// plus its linked fitness candidate and the provenance counts.
#[derive(Clone, Debug)]
pub struct NormProposal {
    pub norm: Norm,
    pub candidate: NormGeneCandidate,
    /// Digest of the abstracted template list — the grouping key.
    pub shape_id: Digest,
    pub observation_count: u32,
    pub workspace_count: u32,
    pub consistency: Q16,
}

/// Lowercase identifier subwords of a facet key: split on every
/// non-alphanumeric character (which includes `_`, so snake_case decomposes),
/// keep words of length ≥ 3 that are all-lowercase-or-digit and not purely
/// numeric. Capitalized type names (`String` in a contract key) never qualify
/// as subjects.
fn subwords(key: &str) -> BTreeSet<String> {
    key.split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 3)
        .filter(|w| {
            w.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
        .filter(|w| !w.chars().all(|c| c.is_ascii_digit()))
        .map(String::from)
        .collect()
}

/// The bundle's subject: the subword present in the most facets (counted once
/// per facet). Requires presence in ≥ 2 facets — a bundle whose keys share
/// nothing has no subject and abstracts to nothing. Ties break toward the
/// longer, then lexicographically smaller word (deterministic).
pub fn abstract_subject(facets: &[WishFacet]) -> Option<String> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for f in facets {
        for w in subwords(&f.key) {
            *counts.entry(w).or_default() += 1;
        }
    }
    counts
        .into_iter()
        .filter(|(_, count)| *count >= 2)
        .max_by(|(wa, ca), (wb, cb)| {
            ca.cmp(cb).then(wa.len().cmp(&wb.len())).then(wb.cmp(wa)) // smaller word wins the final tie
        })
        .map(|(w, _)| w)
}

/// Replace whole-subword occurrences of `subject` in `key` with `{name}`.
/// Boundary-aware: `user` rewrites in `create_user` and `user_test`, never
/// inside `username`.
fn abstract_key(key: &str, subject: &str) -> String {
    let mut out = String::with_capacity(key.len());
    let mut last = 0;
    for (i, m) in key.match_indices(subject) {
        if i < last {
            continue;
        }
        let before_ok = key[..i]
            .chars()
            .next_back()
            .map(|c| !c.is_alphanumeric())
            .unwrap_or(true);
        let after = i + m.len();
        let after_ok = key[after..]
            .chars()
            .next()
            .map(|c| !c.is_alphanumeric())
            .unwrap_or(true);
        if before_ok && after_ok {
            out.push_str(&key[last..i]);
            out.push_str(NAME_PLACEHOLDER);
            last = after;
        }
    }
    out.push_str(&key[last..]);
    out
}

/// Abstract a facet bundle into `(subject, templates)`, or `None` when no
/// shared subject exists. Templates are sorted and deduplicated — the shape.
pub fn abstract_bundle(facets: &[WishFacet]) -> Option<(String, Vec<NormFacetTemplate>)> {
    let subject = abstract_subject(facets)?;
    let mut templates: Vec<NormFacetTemplate> = facets
        .iter()
        .map(|f| NormFacetTemplate::new(f.kind.clone(), abstract_key(&f.key, &subject)))
        .collect();
    templates.sort();
    templates.dedup();
    Some((subject, templates))
}

/// Per-shape accumulator.
struct ShapeStats {
    templates: Vec<NormFacetTemplate>,
    total: u32,
    realized: u32,
    realized_workspaces: BTreeSet<Digest>,
    realized_observation_ids: Vec<Digest>,
    languages: BTreeSet<String>,
}

/// Scan the observation corpus and return every shape that crossed the
/// thresholds, as trigger-less [`NormProposal`]s sorted by `shape_id`.
///
/// Evidence for a proposal is the digest of its contributing (realized)
/// observation ids — content-bound provenance, never `ZERO` (CROSS-006).
/// Shapes whose norm fails [`Norm::validate`] (e.g. a frozen structural key
/// survived abstraction) are silently dropped: fail-closed learning.
pub fn promotable(
    observations: &[FacetBundleObservation],
    cfg: &NormLearningConfig,
    policy_id: Digest,
) -> Vec<NormProposal> {
    let mut shapes: BTreeMap<Digest, ShapeStats> = BTreeMap::new();
    for obs in observations {
        let Some((_, templates)) = abstract_bundle(&obs.facets) else {
            continue;
        };
        let shape_id = Digest::of(&templates);
        let stats = shapes.entry(shape_id).or_insert_with(|| ShapeStats {
            templates,
            total: 0,
            realized: 0,
            realized_workspaces: BTreeSet::new(),
            realized_observation_ids: Vec::new(),
            languages: BTreeSet::new(),
        });
        stats.total += 1;
        if obs.realized {
            stats.realized += 1;
            stats.realized_workspaces.insert(obs.workspace_tag);
            stats.realized_observation_ids.push(obs.observation_id);
            stats.languages.extend(obs.languages.iter().cloned());
        }
    }

    let mut proposals = Vec::new();
    for (shape_id, stats) in shapes {
        if stats.realized < cfg.min_observations {
            continue;
        }
        if (stats.realized_workspaces.len() as u32) < cfg.min_workspaces {
            continue;
        }
        let consistency =
            Q16::ratio(stats.realized as u64, stats.total as u64).unwrap_or(Q16::ZERO);
        if !consistency.at_least(cfg.min_consistency) {
            continue;
        }

        let mut observation_ids = stats.realized_observation_ids.clone();
        observation_ids.sort();
        observation_ids.dedup();
        let evidence = Digest::of(&observation_ids);

        let name = format!("learned-{}", &shape_id.to_hex()[..12]);
        let patterns: Vec<String> = stats
            .templates
            .iter()
            .map(|t| format!("{:?} {}", t.kind, t.key_pattern))
            .collect();
        let mut description = format!(
            "{} facet template(s), realized {}x across {} workspace(s): {}",
            stats.templates.len(),
            stats.realized,
            stats.realized_workspaces.len(),
            patterns.join(", ")
        );
        if !stats.languages.is_empty() {
            description.push_str(&format!(
                " [languages: {}]",
                stats
                    .languages
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }

        let candidate = NormGeneCandidate::new(
            name.clone(),
            description.clone(),
            consistency,
            evidence,
            policy_id,
        );
        let kinds: BTreeSet<String> = stats
            .templates
            .iter()
            .map(|t| format!("{:?}", t.kind).to_ascii_lowercase())
            .collect();
        let semantic_class = kinds.into_iter().collect::<Vec<_>>().join("+");
        let level = NormLevel::from_breadth(stats.templates.len());
        let norm = Norm::new(
            name,
            description,
            level,
            semantic_class,
            stats.templates,
            [],
            [],
            candidate.candidate_id,
            evidence,
            policy_id,
            NormOrigin::Learned { observation_ids },
        );
        if norm.validate().is_err() {
            continue; // a diseased shape never becomes a proposal
        }
        proposals.push(NormProposal {
            norm,
            candidate,
            shape_id,
            observation_count: stats.realized,
            workspace_count: stats.realized_workspaces.len() as u32,
            consistency,
        });
    }
    proposals
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::WishFacetKind;

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    fn bundle(subject: &str) -> Vec<WishFacet> {
        vec![
            WishFacet::module(subject),
            WishFacet::symbol(format!("{subject}_load")),
            WishFacet::capability(format!("organ:{subject}")),
        ]
    }

    fn obs(ws: &[u8], subject: &str, realized: bool) -> FacetBundleObservation {
        FacetBundleObservation::new(
            d(ws),
            bundle(subject),
            ["rust".to_string()],
            realized,
            d(subject.as_bytes()),
            d(b"pol"),
        )
    }

    #[test]
    fn observation_is_content_addressed() {
        let a = obs(b"ws1", "alpha", true);
        let b = obs(b"ws1", "alpha", true);
        assert_eq!(a.observation_id, b.observation_id);
        assert!(a.verify_id());
        let c = obs(b"ws1", "alpha", false);
        assert_ne!(a.observation_id, c.observation_id);
    }

    #[test]
    fn subject_is_the_dominant_shared_subword() {
        let facets = bundle("alpha");
        assert_eq!(abstract_subject(&facets), Some("alpha".into()));
        // Capitalized type names never qualify: a contract bundle's subject is
        // the identifier, not "String".
        let typed = vec![
            WishFacet::contract("create_user", &["String"], "String"),
            WishFacet::contract("get_user", &["String"], "String"),
            WishFacet::module("user"),
        ];
        assert_eq!(abstract_subject(&typed), Some("user".into()));
        // No shared subword → no subject.
        let disjoint = vec![WishFacet::module("alpha"), WishFacet::symbol("beta")];
        assert_eq!(abstract_subject(&disjoint), None);
    }

    #[test]
    fn abstraction_is_boundary_aware() {
        assert_eq!(abstract_key("create_user", "user"), "create_{name}");
        assert_eq!(abstract_key("username", "user"), "username");
        assert_eq!(abstract_key("user_test", "user"), "{name}_test");
        assert_eq!(abstract_key("crud:user", "user"), "crud:{name}");
        assert_eq!(
            abstract_key("create_user(String)->String", "user"),
            "create_{name}(String)->String"
        );
    }

    #[test]
    fn same_shape_across_subjects_groups_together() {
        let (_, ta) = abstract_bundle(&bundle("alpha")).unwrap();
        let (_, tb) = abstract_bundle(&bundle("beta")).unwrap();
        assert_eq!(ta, tb, "alpha and beta bundles share one shape");
        assert_eq!(Digest::of(&ta), Digest::of(&tb));
    }

    #[test]
    fn promotion_fires_exactly_at_the_thresholds() {
        let cfg = NormLearningConfig::default();
        // Two realized observations, two workspaces: below min_observations.
        let two = [obs(b"ws1", "alpha", true), obs(b"ws2", "beta", true)];
        assert!(promotable(&two, &cfg, d(b"pol")).is_empty());

        // Three realized, ONE workspace: below min_workspaces.
        let one_ws = [
            obs(b"ws1", "alpha", true),
            obs(b"ws1", "beta", true),
            obs(b"ws1", "gamma", true),
        ];
        assert!(promotable(&one_ws, &cfg, d(b"pol")).is_empty());

        // Three realized across two workspaces: promoted.
        let three = [
            obs(b"ws1", "alpha", true),
            obs(b"ws2", "beta", true),
            obs(b"ws1", "gamma", true),
        ];
        let proposals = promotable(&three, &cfg, d(b"pol"));
        assert_eq!(proposals.len(), 1);
        let p = &proposals[0];
        assert_eq!(p.observation_count, 3);
        assert_eq!(p.workspace_count, 2);
        assert_eq!(p.consistency, Q16::ONE);
        assert_eq!(p.norm.trigger, None, "learning never arms a trigger");
        assert!(p.norm.validate().is_ok());
        assert_eq!(p.norm.template.len(), 3);
        assert_eq!(p.norm.linked_candidate_id, p.candidate.candidate_id);
        assert!(
            matches!(p.norm.origin, NormOrigin::Learned { ref observation_ids } if observation_ids.len() == 3)
        );
        // The learned norm expands like an archetype.
        let facets = p.norm.instantiate("delta");
        assert!(facets.contains(&WishFacet::module("delta")));
        assert!(facets.contains(&WishFacet::symbol("delta_load")));
        assert!(facets.contains(&WishFacet::capability("organ:delta")));
    }

    #[test]
    fn unrealized_sightings_break_consistency() {
        let cfg = NormLearningConfig::default();
        // 3 realized + 2 failed sightings of the same shape: 3/5 < 3/4.
        let mixed = [
            obs(b"ws1", "alpha", true),
            obs(b"ws2", "beta", true),
            obs(b"ws1", "gamma", true),
            obs(b"ws2", "delta", false),
            obs(b"ws1", "epsilon", false),
        ];
        assert!(promotable(&mixed, &cfg, d(b"pol")).is_empty());
    }

    #[test]
    fn promotion_is_deterministic() {
        let cfg = NormLearningConfig::default();
        let corpus = [
            obs(b"ws1", "alpha", true),
            obs(b"ws2", "beta", true),
            obs(b"ws1", "gamma", true),
        ];
        let a = promotable(&corpus, &cfg, d(b"pol"));
        let b = promotable(&corpus, &cfg, d(b"pol"));
        assert_eq!(a.len(), b.len());
        assert_eq!(a[0].norm.norm_id, b[0].norm.norm_id);
        assert_eq!(a[0].candidate.candidate_id, b[0].candidate.candidate_id);
    }

    #[test]
    fn fixed_capability_survives_but_frozen_module_shape_is_dropped() {
        // Bundle where one capability key does not contain the subject: the
        // template stays fixed, which is legitimate for Capability.
        let with_fixed_cap = |s: &str| {
            vec![
                WishFacet::module(s),
                WishFacet::symbol(format!("{s}_run")),
                WishFacet::capability("audit-log"),
            ]
        };
        let mk = |ws: &[u8], s: &str| {
            FacetBundleObservation::new(
                d(ws),
                with_fixed_cap(s),
                [],
                true,
                d(s.as_bytes()),
                d(b"p"),
            )
        };
        let corpus = [mk(b"w1", "alpha"), mk(b"w2", "beta"), mk(b"w1", "gamma")];
        let proposals = promotable(&corpus, &NormLearningConfig::default(), d(b"p"));
        assert_eq!(proposals.len(), 1);
        assert!(proposals[0]
            .norm
            .template
            .iter()
            .any(|t| t.kind == WishFacetKind::Capability && t.key_pattern == "audit-log"));

        // A frozen Module key in every bundle ("helpers" never abstracted)
        // makes the shape fail validate() — dropped, never proposed.
        let frozen = |s: &str| {
            vec![
                WishFacet::module("helpers"),
                WishFacet::symbol(format!("{s}_run")),
                WishFacet::test(format!("{s}_works")),
            ]
        };
        let mkf = |ws: &[u8], s: &str| {
            FacetBundleObservation::new(d(ws), frozen(s), [], true, d(s.as_bytes()), d(b"p"))
        };
        let corpus = [mkf(b"w1", "alpha"), mkf(b"w2", "beta"), mkf(b"w1", "gamma")];
        assert!(
            promotable(&corpus, &NormLearningConfig::default(), d(b"p")).is_empty(),
            "a shape with a frozen structural key is the disease — never proposed"
        );
    }
}

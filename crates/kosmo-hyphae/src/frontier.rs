use crate::deficiency::{DeficiencyKind, DeficiencyVector};
use crate::void_map::TopologicalVoidMap;
use kosmo_core::{AuthorityLabel, Digest, LicenseStatus, TaintLabel};
use serde::{Deserialize, Serialize};

/// What a source candidate is intended to provide.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceIntentKind {
    FillVoid { void_id: Digest },
    ReduceDeficiency { deficiency_kind: DeficiencyKind },
    SuggestPattern { pattern_name: String },
    Custom { description: String },
}

/// Serialize-only for content-addressing.
#[derive(Serialize)]
struct IntentContent {
    kind: String, // stable JSON of SourceIntentKind
    target_void_id: Option<Digest>,
    taint_label: String,
    authority_label: String,
}

/// A declared intent to acquire or propose a source candidate.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceIntent {
    pub intent_id: Digest,
    pub kind: SourceIntentKind,
    pub target_void_id: Option<Digest>,
    pub taint: TaintLabel,
    pub authority: AuthorityLabel,
}

impl SourceIntent {
    pub fn new(
        kind: SourceIntentKind,
        target_void_id: Option<Digest>,
        taint: TaintLabel,
        authority: AuthorityLabel,
    ) -> Self {
        let intent_id = Digest::of(&IntentContent {
            kind: format!("{:?}", kind),
            target_void_id,
            taint_label: format!("{:?}", taint),
            authority_label: format!("{:?}", authority),
        });
        Self {
            intent_id,
            kind,
            target_void_id,
            taint,
            authority,
        }
    }
}

/// Bounded evidence about a local source candidate.
///
/// Source candidates are local only in Phase 3 (no network acquisition).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceEvidence {
    pub evidence_id: Digest,
    pub source_path: String,
    pub content_digest: Digest,
    pub taint: TaintLabel,
    pub license: LicenseStatus,
    pub observation_ids: Vec<Digest>,
}

impl SourceEvidence {
    pub fn new(
        source_path: String,
        content_digest: Digest,
        taint: TaintLabel,
        license: LicenseStatus,
    ) -> Self {
        let evidence_id =
            Digest::of_bytes(&[source_path.as_bytes(), content_digest.as_bytes()].concat());
        Self {
            evidence_id,
            source_path,
            content_digest,
            taint,
            license,
            observation_ids: vec![],
        }
    }
}

/// Serialize-only for content-addressing SourceFrontierGraph.
#[derive(Serialize)]
struct FrontierContent {
    intent_ids: Vec<Digest>,
    evidence_ids: Vec<Digest>,
    policy_id: Digest,
}

/// The graph of candidate source intents for a HYPHAE run.
///
/// All intents are sorted by intent_id for determinism.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceFrontierGraph {
    pub graph_id: Digest,
    pub intents: Vec<SourceIntent>,
    pub evidence: Vec<SourceEvidence>,
    pub policy_id: Digest,
}

impl SourceFrontierGraph {
    pub fn empty(policy_id: Digest) -> Self {
        let graph_id = Digest::of(&FrontierContent {
            intent_ids: vec![],
            evidence_ids: vec![],
            policy_id,
        });
        Self {
            graph_id,
            intents: vec![],
            evidence: vec![],
            policy_id,
        }
    }

    pub fn from_intents(mut intents: Vec<SourceIntent>, policy_id: Digest) -> Self {
        intents.sort_by_key(|i| i.intent_id);
        let graph_id = Digest::of(&FrontierContent {
            intent_ids: intents.iter().map(|i| i.intent_id).collect(),
            evidence_ids: vec![],
            policy_id,
        });
        Self {
            graph_id,
            intents,
            evidence: vec![],
            policy_id,
        }
    }

    /// Build a frontier from a void map and its derived deficiency vector.
    ///
    /// Produces one `FillVoid` intent per void and one `ReduceDeficiency` intent per
    /// deficiency kind. The category-level `ReduceDeficiency` intents satisfy spec §2.2
    /// (a yield must reference either a void or a deficiency) for any yield built from
    /// them, and are used by pipeline stages that target a whole deficiency class rather
    /// than a single void.
    pub fn from_void_map(void_map: &TopologicalVoidMap, policy_id: Digest) -> Self {
        let deficiency_vector = DeficiencyVector::from_void_map(void_map);
        Self::from_void_map_and_deficiencies(void_map, &deficiency_vector, policy_id)
    }

    pub fn from_void_map_and_deficiencies(
        void_map: &TopologicalVoidMap,
        deficiency_vector: &DeficiencyVector,
        policy_id: Digest,
    ) -> Self {
        let mut intents: Vec<SourceIntent> = void_map
            .voids
            .iter()
            .map(|v| {
                SourceIntent::new(
                    SourceIntentKind::FillVoid { void_id: v.void_id },
                    Some(v.void_id),
                    TaintLabel::Unverified,
                    AuthorityLabel::Agent {
                        name: "hyphae-v0.3".into(),
                    },
                )
            })
            .collect();

        // One ReduceDeficiency intent per deficiency kind (category-level intents).
        for entry in &deficiency_vector.entries {
            intents.push(SourceIntent::new(
                SourceIntentKind::ReduceDeficiency {
                    deficiency_kind: entry.kind.clone(),
                },
                None,
                TaintLabel::Unverified,
                AuthorityLabel::Agent {
                    name: "hyphae-v0.3".into(),
                },
            ));
        }

        Self::from_intents(intents, policy_id)
    }

    /// Augment an existing frontier with `SuggestPattern` intents from prior-run motif candidates.
    ///
    /// Each motif candidate whose `support_score` exceeds `min_support` contributes one intent.
    /// Intents carry the motif's taint and `AuthorityLabel::Agent { name: "hyphae-v0.3" }`.
    /// Used to close the motif feedback loop across pipeline runs.
    pub fn augmented_with_prior_motifs(
        mut self,
        motifs: &[crate::motif::MotifCandidate],
        min_support: kosmo_core::Q16,
        policy_id: Digest,
    ) -> Self {
        for motif in motifs {
            if motif.support_score.at_least(min_support) {
                self.intents.push(SourceIntent::new(
                    SourceIntentKind::SuggestPattern {
                        pattern_name: motif.name.clone(),
                    },
                    None,
                    motif.taint.clone(),
                    AuthorityLabel::Agent {
                        name: "hyphae-v0.3".into(),
                    },
                ));
            }
        }
        // Re-sort and re-seal the graph_id after augmentation.
        self.intents.sort_by_key(|i| i.intent_id);
        self.graph_id = Digest::of(&FrontierContent {
            intent_ids: self.intents.iter().map(|i| i.intent_id).collect(),
            evidence_ids: self.evidence.iter().map(|e| e.evidence_id).collect(),
            policy_id,
        });
        self.policy_id = policy_id;
        self
    }

    pub fn verify_id(&self) -> bool {
        let expected = Digest::of(&FrontierContent {
            intent_ids: self.intents.iter().map(|i| i.intent_id).collect(),
            evidence_ids: self.evidence.iter().map(|e| e.evidence_id).collect(),
            policy_id: self.policy_id,
        });
        self.graph_id == expected
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::void_map::{HostVoid, HostVoidKind, TopologicalVoidMap};
    use kosmo_core::{Digest, Q16};

    #[test]
    fn source_intent_id_deterministic() {
        let vid = Digest::of_bytes(b"void");
        let i1 = SourceIntent::new(
            SourceIntentKind::FillVoid { void_id: vid },
            Some(vid),
            TaintLabel::Unverified,
            AuthorityLabel::Agent {
                name: "hyphae".into(),
            },
        );
        let i2 = SourceIntent::new(
            SourceIntentKind::FillVoid { void_id: vid },
            Some(vid),
            TaintLabel::Unverified,
            AuthorityLabel::Agent {
                name: "hyphae".into(),
            },
        );
        assert_eq!(i1.intent_id, i2.intent_id);
    }

    #[test]
    fn frontier_from_void_map() {
        let pid = Digest::of_bytes(b"p");
        let voids = vec![HostVoid::new(
            HostVoidKind::MissingTestFiber {
                for_module: "src/lib.rs".into(),
            },
            Q16::HALF,
            "src/lib.rs".into(),
        )];
        let vm = TopologicalVoidMap::from_voids(voids, pid);
        let frontier = SourceFrontierGraph::from_void_map(&vm, pid);
        // 1 FillVoid intent + 1 ReduceDeficiency(TestCoverage) intent
        assert_eq!(frontier.intents.len(), 2);
        assert!(frontier
            .intents
            .iter()
            .any(|i| matches!(&i.kind, SourceIntentKind::FillVoid { .. })));
        assert!(frontier
            .intents
            .iter()
            .any(|i| matches!(&i.kind, SourceIntentKind::ReduceDeficiency { .. })));
        assert!(frontier.verify_id());
    }

    #[test]
    fn frontier_from_void_map_empty_produces_no_reduce_deficiency_intents() {
        let pid = Digest::of_bytes(b"p");
        let vm = TopologicalVoidMap::from_voids(vec![], pid);
        let frontier = SourceFrontierGraph::from_void_map(&vm, pid);
        assert_eq!(frontier.intents.len(), 0);
        assert!(frontier.verify_id());
    }

    #[test]
    fn frontier_from_void_map_mixed_kinds_produce_two_reduce_deficiency_intents() {
        let pid = Digest::of_bytes(b"p");
        let voids = vec![
            HostVoid::new(
                HostVoidKind::MissingTestFiber {
                    for_module: "src/a.rs".into(),
                },
                Q16::HALF,
                "src/a.rs".into(),
            ),
            HostVoid::new(
                HostVoidKind::MissingDocFiber {
                    for_module: "src/b.rs".into(),
                },
                Q16::HALF,
                "src/b.rs".into(),
            ),
        ];
        let vm = TopologicalVoidMap::from_voids(voids, pid);
        let frontier = SourceFrontierGraph::from_void_map(&vm, pid);
        // 2 FillVoid + 2 ReduceDeficiency (TestCoverage + Documentation)
        assert_eq!(frontier.intents.len(), 4);
        let reduce_count = frontier
            .intents
            .iter()
            .filter(|i| matches!(&i.kind, SourceIntentKind::ReduceDeficiency { .. }))
            .count();
        assert_eq!(
            reduce_count, 2,
            "one ReduceDeficiency intent per deficiency kind"
        );
        assert!(frontier.verify_id());
    }
}

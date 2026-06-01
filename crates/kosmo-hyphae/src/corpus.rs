use crate::run::HyphaeRunResult;
use kosmo_core::{Digest, TaintLabel};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The kind of an entity in the corpus.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CorpusEntityKind {
    Yield,
    Decision,
    NegativeEvidence,
    SourceCube,
    Motif,
    HostVoid,
    /// A certified `StructuralCrystalRecord` — a proven reusable structural pattern.
    /// Crystal records accumulate across runs, seeding the persistent CAD library.
    CrystalRecord,
}

/// Serialize-only for CorpusEntity content-addressing.
#[derive(Serialize)]
struct EntityContent {
    kind: String,
    data_digest: Digest,
    taint: String,
    policy_id: Digest,
}

/// A content-addressed record of one structural entity in the corpus.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CorpusEntity {
    pub entity_id: Digest,
    pub kind: CorpusEntityKind,
    pub data_digest: Digest,
    pub taint: TaintLabel,
    pub policy_id: Digest,
}

impl CorpusEntity {
    pub fn new(
        kind: CorpusEntityKind,
        data_digest: Digest,
        taint: TaintLabel,
        policy_id: Digest,
    ) -> Self {
        let entity_id = Digest::of(&EntityContent {
            kind: format!("{:?}", kind),
            data_digest,
            taint: format!("{:?}", taint),
            policy_id,
        });
        Self { entity_id, kind, data_digest, taint, policy_id }
    }
}

/// Kind of directed relation between two corpus entities.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationKind {
    DecisionForYield,
    NegativeEvidenceForYield,
    MotifFromYield,
    CubeTargetsVoid,
    Custom(String),
}

/// A directed relation between two corpus entities.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CorpusRelation {
    pub from_id: Digest,
    pub to_id: Digest,
    pub kind: RelationKind,
}

impl CorpusRelation {
    pub fn new(from_id: Digest, to_id: Digest, kind: RelationKind) -> Self {
        Self { from_id, to_id, kind }
    }
}

/// Serialize-only for CorpusCartography content-addressing.
#[derive(Serialize)]
struct CartographyContent {
    entity_ids: Vec<Digest>,
    relation_count: u64,
    policy_id: Digest,
}

/// An append-only, content-addressed record of all structural entities and
/// relations observed across HYPHAE runs.
///
/// Entities are sorted by entity_id. Once added, entities are never removed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CorpusCartography {
    pub cartography_id: Digest,
    pub entities: Vec<CorpusEntity>,
    pub relations: Vec<CorpusRelation>,
    pub policy_id: Digest,
}

impl CorpusCartography {
    pub fn empty(policy_id: Digest) -> Self {
        let cartography_id = Digest::of(&CartographyContent {
            entity_ids: vec![],
            relation_count: 0,
            policy_id,
        });
        Self { cartography_id, entities: vec![], relations: vec![], policy_id }
    }

    /// Append new entities and relations. Returns the updated cartography and
    /// a `CorpusCartographyUpdate` recording what changed.
    ///
    /// Entities already present (by entity_id) are deduplicated. The original
    /// `CorpusCartography` is not mutated — a new one is returned.
    pub fn append(
        &self,
        new_entities: Vec<CorpusEntity>,
        new_relations: Vec<CorpusRelation>,
    ) -> (CorpusCartography, CorpusCartographyUpdate) {
        let before_id = self.cartography_id;
        let existing_ids: BTreeSet<Digest> =
            self.entities.iter().map(|e| e.entity_id).collect();

        let novel: Vec<CorpusEntity> = new_entities
            .into_iter()
            .filter(|e| !existing_ids.contains(&e.entity_id))
            .collect();

        let mut added_entity_ids: Vec<Digest> = novel.iter().map(|e| e.entity_id).collect();
        added_entity_ids.sort();

        let mut merged_entities = self.entities.clone();
        merged_entities.extend(novel);
        merged_entities.sort_by_key(|e| e.entity_id);

        // Deduplicate relations by (from_id, to_id) — same pair is idempotent.
        let existing_rel_keys: BTreeSet<(Digest, Digest)> = self
            .relations
            .iter()
            .map(|r| (r.from_id, r.to_id))
            .collect();
        let novel_relations: Vec<CorpusRelation> = new_relations
            .into_iter()
            .filter(|r| !existing_rel_keys.contains(&(r.from_id, r.to_id)))
            .collect();
        let added_relation_count = novel_relations.len() as u64;

        let mut merged_relations = self.relations.clone();
        merged_relations.extend(novel_relations);
        merged_relations.sort_by(|a, b| {
            a.from_id.cmp(&b.from_id).then(a.to_id.cmp(&b.to_id))
        });

        let after_id = Digest::of(&CartographyContent {
            entity_ids: merged_entities.iter().map(|e| e.entity_id).collect(),
            relation_count: merged_relations.len() as u64,
            policy_id: self.policy_id,
        });

        let updated = CorpusCartography {
            cartography_id: after_id,
            entities: merged_entities,
            relations: merged_relations,
            policy_id: self.policy_id,
        };

        let update = CorpusCartographyUpdate::new(
            before_id,
            after_id,
            added_entity_ids,
            added_relation_count,
            self.policy_id,
        );

        (updated, update)
    }

    /// Extract entities from a `HyphaeRunResult` and append them.
    ///
    /// Satisfies the Phase 4 exit criterion: v0.3 run updates CorpusCartography
    /// append-only.
    pub fn update_from_run(
        &self,
        run: &HyphaeRunResult,
    ) -> (CorpusCartography, CorpusCartographyUpdate) {
        let mut new_entities: Vec<CorpusEntity> = Vec::new();
        let mut new_relations: Vec<CorpusRelation> = Vec::new();

        for decision in &run.decisions {
            let yield_entity = CorpusEntity::new(
                CorpusEntityKind::Yield,
                decision.yield_id,
                TaintLabel::Synthetic,
                run.policy_id,
            );
            let decision_entity = CorpusEntity::new(
                CorpusEntityKind::Decision,
                decision.decision_id,
                TaintLabel::Synthetic,
                run.policy_id,
            );
            new_relations.push(CorpusRelation::new(
                decision_entity.entity_id,
                yield_entity.entity_id,
                RelationKind::DecisionForYield,
            ));
            new_entities.push(yield_entity);
            new_entities.push(decision_entity);
        }

        for neg in &run.negative_evidence {
            let neg_entity = CorpusEntity::new(
                CorpusEntityKind::NegativeEvidence,
                neg.record_id,
                TaintLabel::Synthetic,
                run.policy_id,
            );
            // Compute the yield entity_id for the relation (mirrors what was added above).
            let yield_entity_id = CorpusEntity::new(
                CorpusEntityKind::Yield,
                neg.yield_id,
                TaintLabel::Synthetic,
                run.policy_id,
            )
            .entity_id;
            new_relations.push(CorpusRelation::new(
                neg_entity.entity_id,
                yield_entity_id,
                RelationKind::NegativeEvidenceForYield,
            ));
            new_entities.push(neg_entity);
        }

        self.append(new_entities, new_relations)
    }

    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    pub fn contains(&self, entity_id: Digest) -> bool {
        self.entities.iter().any(|e| e.entity_id == entity_id)
    }
}

/// Serialize-only for CorpusCartographyUpdate content-addressing.
#[derive(Serialize)]
struct UpdateContent {
    before_id: Digest,
    after_id: Digest,
    added_entity_count: u64,
    policy_id: Digest,
}

/// A record of what was appended to a CorpusCartography in one update.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CorpusCartographyUpdate {
    pub update_id: Digest,
    pub before_cartography_id: Digest,
    pub after_cartography_id: Digest,
    pub added_entity_ids: Vec<Digest>,
    pub added_relation_count: u64,
    pub policy_id: Digest,
}

impl CorpusCartographyUpdate {
    fn new(
        before_id: Digest,
        after_id: Digest,
        added_entity_ids: Vec<Digest>,
        added_relation_count: u64,
        policy_id: Digest,
    ) -> Self {
        let update_id = Digest::of(&UpdateContent {
            before_id,
            after_id,
            added_entity_count: added_entity_ids.len() as u64,
            policy_id,
        });
        Self {
            update_id,
            before_cartography_id: before_id,
            after_cartography_id: after_id,
            added_entity_ids,
            added_relation_count,
            policy_id,
        }
    }

    pub fn is_noop(&self) -> bool {
        self.before_cartography_id == self.after_cartography_id
    }
}

/// Filtered view of source cube entities in the corpus.
pub struct SourceCubeIndex {
    pub cube_entity_ids: Vec<Digest>,
}

impl SourceCubeIndex {
    pub fn from_corpus(corpus: &CorpusCartography) -> Self {
        let cube_entity_ids = corpus
            .entities
            .iter()
            .filter(|e| e.kind == CorpusEntityKind::SourceCube)
            .map(|e| e.entity_id)
            .collect();
        Self { cube_entity_ids }
    }

    pub fn count(&self) -> usize {
        self.cube_entity_ids.len()
    }
}

/// Filtered view of motif entities in the corpus.
pub struct MotifIndex {
    pub motif_entity_ids: Vec<Digest>,
}

impl MotifIndex {
    pub fn from_corpus(corpus: &CorpusCartography) -> Self {
        let motif_entity_ids = corpus
            .entities
            .iter()
            .filter(|e| e.kind == CorpusEntityKind::Motif)
            .map(|e| e.entity_id)
            .collect();
        Self { motif_entity_ids }
    }

    pub fn count(&self) -> usize {
        self.motif_entity_ids.len()
    }
}

/// Filtered view of negative evidence entities in the corpus.
pub struct NegativeEvidenceIndex {
    pub record_entity_ids: Vec<Digest>,
}

impl NegativeEvidenceIndex {
    pub fn from_corpus(corpus: &CorpusCartography) -> Self {
        let record_entity_ids = corpus
            .entities
            .iter()
            .filter(|e| e.kind == CorpusEntityKind::NegativeEvidence)
            .map(|e| e.entity_id)
            .collect();
        Self { record_entity_ids }
    }

    pub fn count(&self) -> usize {
        self.record_entity_ids.len()
    }
}

/// Serialize-only for CartographyPrecheck content-addressing.
#[derive(Serialize)]
struct PrecheckContent {
    run_id: Digest,
    policy_id: Digest,
}

/// Validates that a run is safe to append to the corpus.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CartographyPrecheck {
    pub precheck_id: Digest,
    pub run_id: Digest,
    pub passed: bool,
    pub reason: Option<String>,
    pub policy_id: Digest,
}

impl CartographyPrecheck {
    pub fn check(run: &HyphaeRunResult) -> Self {
        let passed = run.run_id != Digest::ZERO;
        let reason = if !passed { Some("run_id is zero".into()) } else { None };
        let precheck_id =
            Digest::of(&PrecheckContent { run_id: run.run_id, policy_id: run.policy_id });
        Self { precheck_id, run_id: run.run_id, passed, reason, policy_id: run.policy_id }
    }
}

/// Serialize-only for ReplayManifest content-addressing.
#[derive(Serialize)]
struct ManifestContent {
    run_id: Digest,
    artifact_count: u64,
    policy_id: Digest,
}

/// A manifest of all artifact digests needed to replay a HYPHAE run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayManifest {
    pub manifest_id: Digest,
    pub run_id: Digest,
    pub artifact_digests: Vec<Digest>,
    pub policy_id: Digest,
}

impl ReplayManifest {
    pub fn from_run(run: &HyphaeRunResult) -> Self {
        let mut artifact_digests: Vec<Digest> =
            vec![run.host_cube.cube_id, run.frontier.graph_id];
        for d in &run.decisions {
            artifact_digests.push(d.decision_id);
            artifact_digests.push(d.evidence_bundle_id);
            artifact_digests.push(d.gate_trace_id);
        }
        artifact_digests.sort();
        artifact_digests.dedup();

        let manifest_id = Digest::of(&ManifestContent {
            run_id: run.run_id,
            artifact_count: artifact_digests.len() as u64,
            policy_id: run.policy_id,
        });

        Self { manifest_id, run_id: run.run_id, artifact_digests, policy_id: run.policy_id }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::passive_run;
    use kosmo_core::{Digest, PolicyProfile};
    use kosmo_workbench::workspace::{WorkspaceEntry, WorkspaceEntryKind, WorkspaceIndex};

    fn make_index(paths: &[&str]) -> WorkspaceIndex {
        let entries: Vec<WorkspaceEntry> = paths
            .iter()
            .map(|p| WorkspaceEntry {
                path: p.to_string(),
                digest: Digest::of_bytes(p.as_bytes()),
                size_bytes: 100,
                kind: WorkspaceEntryKind::SourceFile,
            })
            .collect();
        let count = entries.len() as u64;
        WorkspaceIndex {
            index_id: Digest::of_bytes(b"corpus-test"),
            root: "test".into(),
            entries,
            policy_id: Digest::ZERO,
            entry_count: count,
        }
    }

    #[test]
    fn corpus_cartography_empty_is_content_addressed() {
        let pid = Digest::of_bytes(b"p");
        let c1 = CorpusCartography::empty(pid);
        let c2 = CorpusCartography::empty(pid);
        assert_eq!(c1.cartography_id, c2.cartography_id);
        assert_ne!(c1.cartography_id, Digest::ZERO);
    }

    #[test]
    fn corpus_append_deduplicates_entities() {
        let pid = Digest::of_bytes(b"p");
        let corpus = CorpusCartography::empty(pid);
        let entity = CorpusEntity::new(
            CorpusEntityKind::Yield, Digest::of_bytes(b"y"), TaintLabel::Synthetic, pid,
        );
        let (corpus2, _) = corpus.append(vec![entity.clone()], vec![]);
        // Append the same entity again — should be deduplicated
        let (corpus3, update) = corpus2.append(vec![entity], vec![]);
        assert_eq!(corpus3.entity_count(), 1, "duplicate entity must not be added twice");
        assert!(update.is_noop(), "re-appending same entity is a noop");
    }

    #[test]
    fn corpus_append_grows_monotonically() {
        let pid = Digest::of_bytes(b"p");
        let corpus = CorpusCartography::empty(pid);
        let e1 = CorpusEntity::new(CorpusEntityKind::Yield, Digest::of_bytes(b"a"), TaintLabel::Synthetic, pid);
        let e2 = CorpusEntity::new(CorpusEntityKind::Decision, Digest::of_bytes(b"b"), TaintLabel::Synthetic, pid);
        let (corpus2, _) = corpus.append(vec![e1], vec![]);
        let (corpus3, _) = corpus2.append(vec![e2], vec![]);
        assert_eq!(corpus3.entity_count(), 2);
        // corpus2 is unchanged (append is non-mutating)
        assert_eq!(corpus2.entity_count(), 1);
    }

    #[test]
    fn corpus_update_from_run_adds_entities() {
        let policy = PolicyProfile::default_report_only();
        let index = make_index(&["src/foo.rs", "src/bar.rs"]);
        let run = passive_run(&index, &policy);
        let corpus = CorpusCartography::empty(policy.id);
        let (updated, update) = corpus.update_from_run(&run);
        assert!(updated.entity_count() > 0, "run must add entities to corpus");
        assert!(!update.is_noop(), "update from a non-empty run must not be a noop");
        assert_ne!(updated.cartography_id, corpus.cartography_id);
    }

    #[test]
    fn corpus_update_from_run_is_idempotent() {
        let policy = PolicyProfile::default_report_only();
        let index = make_index(&["src/foo.rs"]);
        let run = passive_run(&index, &policy);
        let corpus = CorpusCartography::empty(policy.id);
        let (corpus1, _) = corpus.update_from_run(&run);
        let (corpus2, update2) = corpus1.update_from_run(&run);
        assert_eq!(corpus1.cartography_id, corpus2.cartography_id, "re-appending same run must be idempotent");
        assert!(update2.is_noop());
    }

    #[test]
    fn corpus_precheck_passes_for_valid_run() {
        let policy = PolicyProfile::default_report_only();
        let index = make_index(&["src/lib.rs"]);
        let run = passive_run(&index, &policy);
        let precheck = CartographyPrecheck::check(&run);
        assert!(precheck.passed);
        assert!(precheck.reason.is_none());
        assert_ne!(precheck.precheck_id, Digest::ZERO);
    }

    #[test]
    fn replay_manifest_contains_run_artifacts() {
        let policy = PolicyProfile::default_report_only();
        let index = make_index(&["src/alpha.rs"]);
        let run = passive_run(&index, &policy);
        let manifest = ReplayManifest::from_run(&run);
        assert!(manifest.artifact_digests.len() >= 2, "manifest must contain at least host_cube and frontier");
        assert_eq!(manifest.run_id, run.run_id);
        assert_ne!(manifest.manifest_id, Digest::ZERO);
    }

    #[test]
    fn negative_evidence_index_counts_rejected_decisions() {
        let policy = PolicyProfile::default_report_only();
        // Use source files to get voids — passive run yields are Synthetic (EvidenceOnly, not rejected)
        let index = make_index(&["src/x.rs"]);
        let run = passive_run(&index, &policy);
        let corpus = CorpusCartography::empty(policy.id);
        let (corpus, _) = corpus.update_from_run(&run);
        let neg_idx = NegativeEvidenceIndex::from_corpus(&corpus);
        // Passive run with Synthetic taint → EvidenceOnly (no rejection) → 0 neg evidence records
        assert_eq!(neg_idx.count(), run.negative_evidence.len());
    }
}

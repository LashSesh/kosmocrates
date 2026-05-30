//! Code HDAG (Hypergraph DAG) skeleton.
//!
//! In Phase 3 this is a structural skeleton only — no real Rust parser is wired.
//! Nodes represent code observations; edges represent structural relations.
//! All HDAG artifacts preserve backref to their source evidence (CROSS-006).

use kosmo_core::{Digest, TaintLabel};
use serde::{Deserialize, Serialize};

/// What was observed about a piece of code.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservationKind {
    FunctionDefinition { name: String, arity: u32 },
    TypeDefinition { name: String },
    TestDefinition { name: String },
    ModuleDeclaration { name: String },
    ImportStatement { module: String },
    ErrorPropagation { mechanism: String },
    Custom { description: String },
}

/// A content-addressed observation of a code element.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CodeObservation {
    pub obs_id: Digest,
    pub kind: ObservationKind,
    pub location: String,
    pub fragment_digest: Digest,
    pub taint: TaintLabel,
    /// Backref to the source evidence that produced this observation.
    pub source_evidence_id: Digest,
}

impl CodeObservation {
    pub fn new(
        kind: ObservationKind,
        location: String,
        fragment_digest: Digest,
        taint: TaintLabel,
        source_evidence_id: Digest,
    ) -> Self {
        let obs_id = Digest::of_bytes(
            &[
                format!("{:?}", kind).as_bytes(),
                location.as_bytes(),
                fragment_digest.as_bytes(),
                source_evidence_id.as_bytes(),
            ]
            .concat(),
        );
        Self { obs_id, kind, location, fragment_digest, taint, source_evidence_id }
    }
}

/// Edge kind in the code HDAG.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HDAGEdgeKind {
    Calls,
    Imports,
    Implements,
    Tests,
    Documents,
    Custom(String),
}

/// A directed edge between two HDAG nodes (by observation id).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HDAGEdge {
    pub from_obs: Digest,
    pub to_obs: Digest,
    pub kind: HDAGEdgeKind,
}

/// A node in the code HDAG.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HDAGNode {
    pub node_id: Digest,
    pub observation_id: Digest,
    pub label: String,
}

impl HDAGNode {
    pub fn from_observation(obs: &CodeObservation) -> Self {
        Self {
            node_id: obs.obs_id,
            observation_id: obs.obs_id,
            label: obs.location.clone(),
        }
    }
}

/// Serialize-only for content-addressing CodeHDAG.
#[derive(Serialize)]
struct HDAGContent {
    node_ids: Vec<Digest>,
    edge_count: u64,
    source_evidence_id: Digest,
    taint: String,
}

/// A bounded, content-addressed hypergraph DAG over code observations.
///
/// HDAG node/edge backrefs to source evidence are preserved (Phase 6 requirement,
/// already satisfied here). Taint propagates from source to HDAG.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CodeHDAG {
    pub hdag_id: Digest,
    pub nodes: Vec<HDAGNode>,
    pub edges: Vec<HDAGEdge>,
    /// Backref to the source evidence this HDAG was derived from.
    pub source_evidence_id: Digest,
    pub taint: TaintLabel,
}

impl CodeHDAG {
    pub fn new(
        nodes: Vec<HDAGNode>,
        edges: Vec<HDAGEdge>,
        source_evidence_id: Digest,
        taint: TaintLabel,
    ) -> Self {
        let hdag_id = Digest::of(&HDAGContent {
            node_ids: nodes.iter().map(|n| n.node_id).collect(),
            edge_count: edges.len() as u64,
            source_evidence_id,
            taint: format!("{:?}", taint),
        });
        Self { hdag_id, nodes, edges, source_evidence_id, taint }
    }

    /// Minimal HDAG for one source file (skeleton — one node, no edges).
    pub fn skeleton_for_source(source_evidence_id: Digest, location: &str, taint: TaintLabel) -> Self {
        let obs = CodeObservation::new(
            ObservationKind::ModuleDeclaration { name: location.to_string() },
            location.to_string(),
            source_evidence_id,
            taint.clone(),
            source_evidence_id,
        );
        let node = HDAGNode::from_observation(&obs);
        Self::new(vec![node], vec![], source_evidence_id, taint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::{Digest, TaintLabel};

    #[test]
    fn code_hdag_skeleton_has_backref() {
        let ev_id = Digest::of_bytes(b"source");
        let hdag = CodeHDAG::skeleton_for_source(ev_id, "src/lib.rs", TaintLabel::Clean);
        assert_eq!(hdag.source_evidence_id, ev_id, "backref must be preserved");
        assert_eq!(hdag.nodes.len(), 1);
        assert_eq!(hdag.edges.len(), 0);
    }

    #[test]
    fn code_hdag_deterministic() {
        let ev_id = Digest::of_bytes(b"ev");
        let h1 = CodeHDAG::skeleton_for_source(ev_id, "src/a.rs", TaintLabel::Unverified);
        let h2 = CodeHDAG::skeleton_for_source(ev_id, "src/a.rs", TaintLabel::Unverified);
        assert_eq!(h1.hdag_id, h2.hdag_id);
    }
}

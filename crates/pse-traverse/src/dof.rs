//! `DoFGraph` — the working graph of the agent.
//!
//! Nodes are degrees of freedom, constraint nodes, evidence nodes,
//! candidates, boundaries and merge points. Edges link constraints to
//! the dimensions they couple, candidates to the evidence backing them,
//! etc. The graph is the substrate over which the [`crate::plan`]
//! collapse planner decides ordering.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::field_cube::{EvidenceRef, FieldCube, TopologySummary};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub String);

impl NodeId {
    pub fn new(s: impl Into<String>) -> Self { NodeId(s.into()) }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DoFNode {
    pub id: NodeId,
    pub kind: DoFNodeKind,
    pub label: String,
    pub weight: f64,
    pub status: NodeStatus,
    pub evidence: Vec<EvidenceRef>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DoFNodeKind {
    Dimension,
    Constraint,
    Evidence,
    Candidate,
    Boundary,
    Merge,
    Commit,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum NodeStatus {
    Open,
    Resolved,
    Excised,
    Failed,
    Committed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DoFEdge {
    pub from: NodeId,
    pub to: NodeId,
    pub kind: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DoFGraph {
    pub nodes: BTreeMap<NodeId, DoFNode>,
    pub edges: Vec<DoFEdge>,
    pub topology: TopologySummary,
}

impl DoFGraph {
    /// Build a `DoFGraph` from a `FieldCube`. Deterministic: dimension and
    /// constraint nodes are inserted in `BTreeMap` order; edges are sorted
    /// by `(from, to, kind)`.
    pub fn from_field_cube(cube: &FieldCube) -> Self {
        let mut nodes: BTreeMap<NodeId, DoFNode> = BTreeMap::new();
        for (id, dim) in &cube.dimensions {
            nodes.insert(NodeId::new(id), DoFNode {
                id: NodeId::new(id),
                kind: DoFNodeKind::Dimension,
                label: dim.label.clone(),
                weight: 1.0,
                status: NodeStatus::Open,
                evidence: Vec::new(),
            });
        }
        for (id, c) in &cube.constraints {
            nodes.insert(NodeId::new(id), DoFNode {
                id: NodeId::new(id),
                kind: DoFNodeKind::Constraint,
                label: c.predicate.clone(),
                weight: c.weight,
                status: NodeStatus::Open,
                evidence: Vec::new(),
            });
        }

        let mut edges: Vec<DoFEdge> = cube.couplings.iter().map(|cp| DoFEdge {
            from: NodeId::new(&cp.from),
            to: NodeId::new(&cp.to),
            kind: cp.kind.clone(),
        }).collect();
        edges.sort();

        DoFGraph {
            nodes,
            edges,
            topology: cube.topology.clone(),
        }
    }

    /// Number of edges incident on `node` (treating edges as undirected).
    pub fn degree(&self, node: &NodeId) -> u64 {
        self.edges.iter().filter(|e| &e.from == node || &e.to == node).count() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::content_address;
    use crate::field_cube::DefaultFieldCubeBuilder;
    use crate::field_cube::FieldCubeBuilder;
    use crate::spec::{
        ConstraintSpec, ConstraintKind, DimensionKind, DimensionSource, DimensionSpec,
        OutputSpec, ProblemSpec, ReplayPolicy, RiskPolicy, ValueDomain,
    };

    fn spec() -> ProblemSpec {
        ProblemSpec {
            id: "p.dof".into(),
            title: "T".into(),
            objective: "o".into(),
            domain: "d".into(),
            inputs: vec![],
            constraints: vec![ConstraintSpec {
                id: "c.a".into(),
                kind: ConstraintKind::Hard,
                predicate: "p".into(),
                weight: 1.0,
                dimensions: vec!["d.x".into()],
            }],
            dimensions: vec![DimensionSpec {
                id: "d.x".into(),
                label: "X".into(),
                kind: DimensionKind::Boolean,
                values: ValueDomain::Boolean,
                required: true,
                source: DimensionSource::User,
            }],
            desired_outputs: vec![OutputSpec { id: "o.x".into(), kind: "y".into() }],
            risk_policy: RiskPolicy::default(),
            replay: ReplayPolicy::default(),
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn dof_graph_builds_from_cube() {
        let cube = DefaultFieldCubeBuilder.build(&spec()).unwrap();
        let g = DoFGraph::from_field_cube(&cube);
        assert!(g.nodes.contains_key(&NodeId::new("d.x")));
        assert!(g.nodes.contains_key(&NodeId::new("c.a")));
        assert_eq!(g.edges.len(), 1);
        assert_eq!(g.degree(&NodeId::new("d.x")), 1);
    }

    #[test]
    fn dof_graph_address_stable_across_runs() {
        let cube = DefaultFieldCubeBuilder.build(&spec()).unwrap();
        let g1 = DoFGraph::from_field_cube(&cube);
        let g2 = DoFGraph::from_field_cube(&cube);
        assert_eq!(content_address(&g1).unwrap(), content_address(&g2).unwrap());
    }
}

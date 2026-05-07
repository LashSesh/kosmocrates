//! Operator-Signature layer — PSE-TRAVERSE-SIGNATURE-01
//!
//! Builds a [`StructuralOperator`] from a [`DoFGraph`], capturing the
//! matrix profile of the graph's Laplacian / directed-coupling structure
//! in a canonically serialisable, content-addressed form.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::canonical::hex_address;
use crate::dof::{DoFGraph, DoFNodeKind, NodeId};
use crate::field_cube::EvidenceRef;

// ---------------------------------------------------------------------------
// SignatureValue
// ---------------------------------------------------------------------------

/// Canonical fixed-point scalar: value = mantissa × 10^(−scale).
///
/// Both fields are integers so the type is `Eq`, `Ord`, and `Hash`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignatureValue {
    pub mantissa: i64,
    pub scale: i32,
}

impl SignatureValue {
    /// Round `value` to `scale` decimal places and return the canonical
    /// fixed-point representation.  Returns [`SignatureError::NonFiniteValue`]
    /// if `value` is NaN or ±∞.
    pub fn quantize(value: f64, scale: i32) -> Result<Self, SignatureError> {
        if !value.is_finite() {
            return Err(SignatureError::NonFiniteValue);
        }
        let factor = 10_f64.powi(scale);
        let mantissa = (value * factor).round() as i64;
        Ok(SignatureValue { mantissa, scale })
    }

    /// Convert back to `f64`.
    pub fn to_f64(&self) -> f64 {
        self.mantissa as f64 / 10_f64.powi(self.scale)
    }

    /// Zero at the given scale.
    pub fn zero(scale: i32) -> Self {
        SignatureValue { mantissa: 0, scale }
    }
}

// ---------------------------------------------------------------------------
// Edge / weight types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct WeightedEdge {
    pub u: usize,
    pub v: usize,
    pub weight: SignatureValue,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct WeightedDirectedEdge {
    pub from: usize,
    pub to: usize,
    pub weight: SignatureValue,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeWeight {
    pub node: usize,
    pub weight: SignatureValue,
}

// ---------------------------------------------------------------------------
// MatrixProfile
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MatrixProfile {
    pub n: usize,
    /// Diagonal entries (Laplacian degrees), sorted ascending by value.
    pub diagonal: Vec<SignatureValue>,
    /// Undirected edges, sorted by (min(u,v), max(u,v)).
    pub symmetric_edges: Vec<WeightedEdge>,
    /// Net directed edges (fwd − bwd ≠ 0), sorted by (from, to).
    pub directed_edges: Vec<WeightedDirectedEdge>,
    /// Constraint-node penalty weights, sorted by node index.
    pub constraint_weights: Vec<NodeWeight>,
}

// ---------------------------------------------------------------------------
// OperatorKind
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OperatorKind {
    Laplacian,
    DirectedCoupling,
    ConstraintWeighted,
    Composite,
}

// ---------------------------------------------------------------------------
// OperatorBuildConfig
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OperatorBuildConfig {
    pub kind: OperatorKind,
    /// Penalty multiplier for hard constraints.
    pub lambda_hard: f64,
    /// Penalty multiplier for soft constraints.
    pub lambda_soft: f64,
    /// Number of decimal digits kept after quantization.
    pub quantization_scale: i32,
    /// Semver string for the operator schema.
    pub operator_version: String,
}

impl Default for OperatorBuildConfig {
    fn default() -> Self {
        OperatorBuildConfig {
            kind: OperatorKind::Laplacian,
            lambda_hard: 1.0,
            lambda_soft: 0.5,
            quantization_scale: 6,
            operator_version: "v0.1".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// StructuralOperator
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct StructuralOperator {
    /// 64-char lower-case hex SHA-256, content-addressed over the operator
    /// (with `operator_id` set to the empty string during hashing).
    pub operator_id: String,
    /// 64-char lower-case hex SHA-256 of the source [`DoFGraph`] bytes.
    pub substrate_id: String,
    pub kind: OperatorKind,
    pub matrix_profile: MatrixProfile,
    pub parameters: BTreeMap<String, SignatureValue>,
    pub evidence_refs: Vec<EvidenceRef>,
    pub operator_version: String,
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum SignatureError {
    #[error("operator is empty")]
    EmptyOperator,
    #[error("non-finite value")]
    NonFiniteValue,
    #[error("quantization failed: {0}")]
    QuantizationFailed(String),
    #[error("algorithm did not converge after {0} iterations")]
    AlgorithmDidNotConverge(usize),
    #[error("unsupported operator kind")]
    UnsupportedOperatorKind,
}

#[derive(Debug, thiserror::Error)]
pub enum OperatorBuildError {
    #[error("graph has no nodes")]
    EmptyGraph,
    #[error("invalid node reference: {0}")]
    InvalidNodeReference(String),
    #[error("invalid weight: {0}")]
    InvalidWeight(String),
    #[error("missing constraint reference: {0}")]
    MissingConstraintReference(String),
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Build a [`StructuralOperator`] from a [`DoFGraph`].
///
/// The node index is assigned by [`BTreeMap`] iteration order (alphabetical
/// on [`NodeId`]), which is deterministic across runs.
pub fn build_operator_from_dof_graph(
    graph: &DoFGraph,
    config: &OperatorBuildConfig,
) -> Result<StructuralOperator, OperatorBuildError> {
    // 1. Guard: empty graph.
    if graph.nodes.is_empty() {
        return Err(OperatorBuildError::EmptyGraph);
    }

    // 2. Build deterministic index: NodeId → usize (BTreeMap iteration = alphabetical).
    let node_idx: BTreeMap<&NodeId, usize> = graph
        .nodes
        .keys()
        .enumerate()
        .map(|(i, k)| (k, i))
        .collect();

    // 3. n = total node count.
    let n = node_idx.len();
    let scale = config.quantization_scale;

    // 4. Degree of each node (undirected).
    let mut degree: Vec<usize> = vec![0usize; n];
    for edge in &graph.edges {
        if let (Some(&fi), Some(&ti)) = (node_idx.get(&edge.from), node_idx.get(&edge.to)) {
            degree[fi] += 1;
            degree[ti] += 1;
        }
    }

    // 5. Diagonal: degree quantized, then sorted ascending.
    let mut diagonal: Vec<SignatureValue> = (0..n)
        .map(|i| {
            SignatureValue::quantize(degree[i] as f64, scale)
                .unwrap_or_else(|_| SignatureValue::zero(scale))
        })
        .collect();
    diagonal.sort();

    // 6. Symmetric edges: one entry per (u,v) pair, weight = 1.0 quantized.
    //    Dedup on (u, v) — we keep the first occurrence since weight is always 1.
    let unit = SignatureValue::quantize(1.0, scale).unwrap_or_else(|_| SignatureValue::zero(scale));
    let mut sym_set: BTreeMap<(usize, usize), SignatureValue> = BTreeMap::new();
    for edge in &graph.edges {
        if let (Some(&fi), Some(&ti)) = (node_idx.get(&edge.from), node_idx.get(&edge.to)) {
            let u = fi.min(ti);
            let v = fi.max(ti);
            sym_set.entry((u, v)).or_insert_with(|| unit.clone());
        }
    }
    let symmetric_edges: Vec<WeightedEdge> = sym_set
        .into_iter()
        .map(|((u, v), weight)| WeightedEdge { u, v, weight })
        .collect();
    // BTreeMap iteration is already sorted by (u, v).

    // 7. Directed edges: net flow (fwd − bwd) for each ordered pair (i < j).
    let mut directed_edges: Vec<WeightedDirectedEdge> = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            let fwd = graph
                .edges
                .iter()
                .filter(|e| node_idx.get(&e.from) == Some(&i) && node_idx.get(&e.to) == Some(&j))
                .count();
            let bwd = graph
                .edges
                .iter()
                .filter(|e| node_idx.get(&e.from) == Some(&j) && node_idx.get(&e.to) == Some(&i))
                .count();
            if fwd != bwd {
                let net = fwd as f64 - bwd as f64;
                let weight = SignatureValue::quantize(net, scale)
                    .unwrap_or_else(|_| SignatureValue::zero(scale));
                directed_edges.push(WeightedDirectedEdge {
                    from: i,
                    to: j,
                    weight,
                });
            }
        }
    }
    // Already generated in (from < to) order with i < j outer loop.
    directed_edges.sort_by(|a, b| a.from.cmp(&b.from).then(a.to.cmp(&b.to)));

    // 8. Constraint weights: lambda_hard * node.weight for Constraint nodes.
    let mut constraint_weights: Vec<NodeWeight> = Vec::new();
    for (node_id, node) in &graph.nodes {
        if node.kind == DoFNodeKind::Constraint {
            if let Some(&idx) = node_idx.get(node_id) {
                let v = config.lambda_hard * node.weight;
                let weight = SignatureValue::quantize(v, scale)
                    .unwrap_or_else(|_| SignatureValue::zero(scale));
                constraint_weights.push(NodeWeight { node: idx, weight });
            }
        }
    }
    constraint_weights.sort_by_key(|nw| nw.node);

    // 9. Assemble MatrixProfile.
    let matrix_profile = MatrixProfile {
        n,
        diagonal,
        symmetric_edges,
        directed_edges,
        constraint_weights,
    };

    // 10. substrate_id = canonical address of the DoFGraph.
    let substrate_id = hex_address(graph).unwrap_or_else(|_| "0".repeat(64));

    // 11. Parameters BTreeMap.
    let mut parameters: BTreeMap<String, SignatureValue> = BTreeMap::new();
    parameters.insert(
        "lambda_hard".into(),
        SignatureValue::quantize(config.lambda_hard, scale)
            .unwrap_or_else(|_| SignatureValue::zero(scale)),
    );
    parameters.insert(
        "lambda_soft".into(),
        SignatureValue::quantize(config.lambda_soft, scale)
            .unwrap_or_else(|_| SignatureValue::zero(scale)),
    );
    parameters.insert(
        "quantization_scale".into(),
        SignatureValue {
            mantissa: config.quantization_scale as i64,
            scale: 0,
        },
    );

    // Collect evidence refs from all nodes.
    let evidence_refs: Vec<EvidenceRef> = {
        let mut refs: Vec<EvidenceRef> = graph
            .nodes
            .values()
            .flat_map(|n| n.evidence.iter().cloned())
            .collect();
        refs.sort();
        refs.dedup();
        refs
    };

    // 12–13. Build operator with empty operator_id, then content-address it.
    let mut op = StructuralOperator {
        operator_id: String::new(),
        substrate_id,
        kind: config.kind.clone(),
        matrix_profile,
        parameters,
        evidence_refs,
        operator_version: config.operator_version.clone(),
    };

    let hex = hex_address(&op).unwrap_or_else(|e| e.to_string());
    op.operator_id = hex;

    Ok(op)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use crate::dof::DoFGraph;
    use crate::field_cube::{DefaultFieldCubeBuilder, FieldCubeBuilder};
    use crate::spec::{
        ConstraintKind, ConstraintSpec, DimensionKind, DimensionSource, DimensionSpec, OutputSpec,
        ProblemSpec, ReplayPolicy, RiskPolicy, ValueDomain,
    };

    fn minimal_spec() -> ProblemSpec {
        ProblemSpec {
            id: "p.op-test".into(),
            title: "T".into(),
            objective: "o".into(),
            domain: "d".into(),
            inputs: vec![],
            constraints: vec![ConstraintSpec {
                id: "c.a".into(),
                kind: ConstraintKind::Hard,
                predicate: "pred".into(),
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
            desired_outputs: vec![OutputSpec {
                id: "o.x".into(),
                kind: "y".into(),
            }],
            risk_policy: RiskPolicy::default(),
            replay: ReplayPolicy::default(),
            metadata: BTreeMap::new(),
        }
    }

    fn minimal_graph() -> DoFGraph {
        let cube = DefaultFieldCubeBuilder.build(&minimal_spec()).unwrap();
        DoFGraph::from_field_cube(&cube)
    }

    #[test]
    fn operator_from_empty_graph_fails() {
        use crate::field_cube::TopologySummary;
        let graph = DoFGraph {
            nodes: BTreeMap::new(),
            edges: vec![],
            topology: TopologySummary::default(),
        };
        let config = OperatorBuildConfig::default();
        let result = build_operator_from_dof_graph(&graph, &config);
        assert!(matches!(result, Err(OperatorBuildError::EmptyGraph)));
    }

    #[test]
    fn operator_id_is_stable() {
        let graph = minimal_graph();
        let config = OperatorBuildConfig::default();
        let op1 = build_operator_from_dof_graph(&graph, &config).unwrap();
        let op2 = build_operator_from_dof_graph(&graph, &config).unwrap();
        assert_eq!(op1.operator_id, op2.operator_id);
        assert_eq!(op1.operator_id.len(), 64);
    }

    #[test]
    fn operator_edges_are_sorted() {
        let graph = minimal_graph();
        let config = OperatorBuildConfig::default();
        let op = build_operator_from_dof_graph(&graph, &config).unwrap();

        // diagonal: ascending
        let diag = &op.matrix_profile.diagonal;
        for w in diag.windows(2) {
            assert!(w[0] <= w[1], "diagonal not sorted ascending: {:?}", diag);
        }

        // symmetric_edges: sorted by (u, v)
        let sym = &op.matrix_profile.symmetric_edges;
        for w in sym.windows(2) {
            assert!(
                (w[0].u, w[0].v) <= (w[1].u, w[1].v),
                "symmetric_edges not sorted: {:?}",
                sym
            );
        }

        // directed_edges: sorted by (from, to)
        let dir = &op.matrix_profile.directed_edges;
        for w in dir.windows(2) {
            assert!(
                (w[0].from, w[0].to) <= (w[1].from, w[1].to),
                "directed_edges not sorted: {:?}",
                dir
            );
        }

        // constraint_weights: sorted by node
        let cw = &op.matrix_profile.constraint_weights;
        for w in cw.windows(2) {
            assert!(
                w[0].node <= w[1].node,
                "constraint_weights not sorted: {:?}",
                cw
            );
        }
    }
}

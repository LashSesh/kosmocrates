//! Hypercube-HDAG embedding and compiler for nxalien.
//!
//! Builds a directed acyclic graph over C^8 from a set of rules, unknowns, and
//! project metadata.  Cycle-forming edges are silently dropped (recorded as
//! advisory notes).

use petgraph::algo::is_cyclic_directed;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::Topo;
use pse_nxalien_types::{
    AgentContextCube, C8Coord, EdgeCause, NodeKind, NxAlienEdge, NxAlienNode, NxAlienPolicy,
    ProjectMetadata, RuleAtom, Severity, UnknownSlot, UnknownStatus,
};
use std::collections::BTreeMap;

// ─── HypercubeHdag ───────────────────────────────────────────────────────────

pub struct HypercubeHdag {
    graph: DiGraph<String, ()>,
    nodes: BTreeMap<String, NxAlienNode>,
    edges: Vec<NxAlienEdge>,
    node_indices: BTreeMap<String, NodeIndex>,
    policy: NxAlienPolicy,
    advisory_notes: Vec<String>,
}

impl HypercubeHdag {
    pub fn new(policy: NxAlienPolicy) -> Self {
        Self {
            graph: DiGraph::new(),
            nodes: BTreeMap::new(),
            edges: Vec::new(),
            node_indices: BTreeMap::new(),
            policy,
            advisory_notes: Vec::new(),
        }
    }

    /// Add a node if its id is not already present.  Returns its petgraph index.
    pub fn add_node(&mut self, node: NxAlienNode) -> NodeIndex {
        if let Some(&idx) = self.node_indices.get(&node.id) {
            return idx;
        }
        let id = node.id.clone();
        let idx = self.graph.add_node(id.clone());
        self.node_indices.insert(id.clone(), idx);
        self.nodes.insert(id, node);
        idx
    }

    /// Try to add a directed edge.  If the edge would create a cycle it is
    /// dropped and recorded as an advisory note.  Returns `true` if added.
    pub fn try_add_edge(&mut self, from: &str, to: &str, cause: EdgeCause, weight: f64) -> bool {
        let Some(&fi) = self.node_indices.get(from) else {
            return false;
        };
        let Some(&ti) = self.node_indices.get(to) else {
            return false;
        };
        // Optimistically add to check acyclicity.
        let tmp = self.graph.add_edge(fi, ti, ());
        if is_cyclic_directed(&self.graph) {
            self.graph.remove_edge(tmp);
            self.advisory_notes
                .push(format!("Cycle detected: {from} → {to}; edge dropped"));
            return false;
        }
        // Check semantic coherence (τ_A threshold).
        let vi = &self.nodes[from];
        let vj = &self.nodes[to];
        let r_a = semantic_coherence(vi, vj);
        if r_a < self.policy.tau_a {
            self.graph.remove_edge(tmp);
            self.advisory_notes.push(format!(
                "Low coherence ({r_a:.3} < τ_A={:.3}): {from} → {to}; edge dropped",
                self.policy.tau_a
            ));
            return false;
        }
        // Causal drift invariant: η_j ≥ η_i - ε_η.
        let eta_i = vi.coord[4];
        let eta_j = vj.coord[4];
        if eta_j < eta_i - self.policy.eta_eps {
            self.graph.remove_edge(tmp);
            self.advisory_notes.push(format!(
                "Causal drift ({eta_j:.3} < {eta_i:.3} - ε): {from} → {to}; edge dropped"
            ));
            return false;
        }
        self.edges.push(NxAlienEdge {
            from: from.to_string(),
            to: to.to_string(),
            cause,
            weight,
        });
        true
    }

    /// Whether the underlying graph is acyclic.
    pub fn is_acyclic(&self) -> bool {
        !is_cyclic_directed(&self.graph)
    }

    /// Canonical topological order with tie-breaking by node id (lexicographic).
    pub fn topological_order(&self) -> Vec<String> {
        let mut topo = Topo::new(&self.graph);
        let mut order = Vec::new();
        while let Some(idx) = topo.next(&self.graph) {
            order.push(self.graph[idx].clone());
        }
        order
    }

    /// 5D projection: η' = clip(0, 1, 0.50·η + 0.25·γ + 0.25·(1-υ)).
    pub fn project_5d(coord: &C8Coord) -> [f64; 5] {
        let eta_prime =
            (0.50 * coord[4] + 0.25 * coord[5] + 0.25 * (1.0 - coord[6])).clamp(0.0, 1.0);
        [coord[0], coord[1], coord[2], coord[3], eta_prime]
    }

    /// Export as a serialisable AgentContextCube.
    pub fn to_cube(&self) -> AgentContextCube {
        AgentContextCube {
            dimension: 8,
            nodes: self.nodes.clone(),
            edges: self.edges.clone(),
            policy: self.policy.clone(),
        }
    }

    /// Advisory notes for edges dropped due to cycle / coherence / drift.
    pub fn advisory_notes(&self) -> &[String] {
        &self.advisory_notes
    }

    // ─── Build from a bundle ─────────────────────────────────────────────────

    /// Build an HDAG from project metadata, compiled rules, and unknown slots.
    pub fn build_from_bundle(
        metadata: &ProjectMetadata,
        rules: &[RuleAtom],
        unknowns: &[UnknownSlot],
    ) -> Self {
        Self::build_with_policy(metadata, rules, unknowns, NxAlienPolicy::default())
    }

    pub fn build_with_policy(
        metadata: &ProjectMetadata,
        rules: &[RuleAtom],
        unknowns: &[UnknownSlot],
        policy: NxAlienPolicy,
    ) -> Self {
        let mut hdag = HypercubeHdag::new(policy);

        // 1. Project root node.
        let project_id = "project:root".to_string();
        hdag.add_node(NxAlienNode {
            id: project_id.clone(),
            kind: NodeKind::Project,
            label: metadata.name.clone(),
            coord: [0.9, 0.5, 0.5, 1.0, 0.5, 0.3, 0.1, 0.8],
            payload: serde_json::to_value(metadata).unwrap_or_default(),
        });

        // 2. Tool nodes (one per detected command).
        for (tool_label, cmd) in [
            ("tool:test", &metadata.test_command),
            ("tool:build", &metadata.build_command),
            ("tool:fmt", &metadata.fmt_command),
            ("tool:lint", &metadata.lint_command),
        ] {
            if cmd.is_some() {
                hdag.add_node(NxAlienNode {
                    id: tool_label.to_string(),
                    kind: NodeKind::Tool,
                    label: tool_label.to_string(),
                    coord: [0.7, 0.4, 0.5, 0.6, 0.5, 0.4, 0.1, 0.8],
                    payload: serde_json::json!({ "command": cmd }),
                });
                hdag.try_add_edge(&project_id, tool_label, EdgeCause::DerivesFrom, 1.0);
            }
        }

        // 3. Rule nodes + evidence nodes.
        let mut specialist_triggers: BTreeMap<&str, Vec<String>> = BTreeMap::new();

        for rule in rules {
            let rule_id = format!("rule:{}", rule.id);
            let coord = rule_coord(rule);
            hdag.add_node(NxAlienNode {
                id: rule_id.clone(),
                kind: NodeKind::Rule,
                label: rule.id.clone(),
                coord,
                payload: serde_json::to_value(rule).unwrap_or_default(),
            });
            // Rule ← Project
            hdag.try_add_edge(&project_id, &rule_id, EdgeCause::DerivesFrom, 0.8);

            // Evidence nodes.
            for ev in &rule.evidence {
                let ev_id = format!("evidence:{}", ev.source);
                if !hdag.nodes.contains_key(&ev_id) {
                    hdag.add_node(NxAlienNode {
                        id: ev_id.clone(),
                        kind: NodeKind::Evidence,
                        label: ev.source.clone(),
                        coord: [0.8, 0.2, 0.6, 0.4, 0.4, 0.1, 0.2, 0.5],
                        payload: serde_json::to_value(ev).unwrap_or_default(),
                    });
                }
                hdag.try_add_edge(&rule_id, &ev_id, EdgeCause::ValidatesBy, 0.9);
            }

            // Tool ← Rule (ValidatesBy).
            if rule.scope.commands.contains(&"cargo test".to_string())
                || rule.trigger.contains(&"test".to_string())
            {
                hdag.try_add_edge(&rule_id, "tool:test", EdgeCause::ValidatesBy, 0.7);
            }
            if rule.trigger.contains(&"fmt".to_string())
                || rule.trigger.contains(&"format".to_string())
            {
                hdag.try_add_edge(&rule_id, "tool:fmt", EdgeCause::ValidatesBy, 0.7);
            }
            if rule.trigger.contains(&"lint".to_string())
                || rule.trigger.contains(&"clippy".to_string())
            {
                hdag.try_add_edge(&rule_id, "tool:lint", EdgeCause::ValidatesBy, 0.7);
            }

            // Accumulate specialist triggers.
            for kw in &rule.trigger {
                let specialist = specialist_for_keyword(kw);
                specialist_triggers
                    .entry(specialist)
                    .or_default()
                    .push(kw.clone());
            }
        }

        // 4. Specialist nodes.
        let specialist_ids: Vec<(String, Vec<String>)> = specialist_triggers
            .into_iter()
            .filter(|(k, _)| !k.is_empty())
            .map(|(k, v)| (format!("specialist:{k}"), v))
            .collect();

        for (spec_id, triggers) in &specialist_ids {
            hdag.add_node(NxAlienNode {
                id: spec_id.clone(),
                kind: NodeKind::Specialist,
                label: spec_id.clone(),
                coord: [0.6, 0.5, 0.5, 0.5, 0.5, 0.5, 0.1, 0.7],
                payload: serde_json::json!({ "triggers": triggers }),
            });
        }
        // Wire rules → specialists.
        for rule in rules {
            let rule_id = format!("rule:{}", rule.id);
            for kw in &rule.trigger {
                let spec_key = format!("specialist:{}", specialist_for_keyword(kw));
                if hdag.nodes.contains_key(&spec_key) {
                    hdag.try_add_edge(&rule_id, &spec_key, EdgeCause::Triggers, 0.6);
                }
            }
        }

        // 5. Gate node.
        hdag.add_node(NxAlienNode {
            id: "gate:G_A".to_string(),
            kind: NodeKind::Gate,
            label: "G_A".to_string(),
            coord: [0.5, 0.5, 0.5, 0.5, 0.5, 0.8, 0.0, 0.5],
            payload: serde_json::json!({ "gates": 8 }),
        });
        hdag.try_add_edge(&project_id, "gate:G_A", EdgeCause::ValidatesBy, 1.0);
        for rule in rules {
            hdag.try_add_edge(
                &format!("rule:{}", rule.id),
                "gate:G_A",
                EdgeCause::Constrains,
                0.8,
            );
        }

        // 6. Unknown nodes.
        for unknown in unknowns {
            let unk_id = format!("unknown:{}", unknown.name);
            let coord = unknown_coord(unknown);
            hdag.add_node(NxAlienNode {
                id: unk_id.clone(),
                kind: NodeKind::Unknown,
                label: unknown.name.clone(),
                coord,
                payload: serde_json::to_value(unknown).unwrap_or_default(),
            });
            hdag.try_add_edge(&project_id, &unk_id, EdgeCause::DerivesFrom, 0.4);
        }

        // 7. Output node.
        hdag.add_node(NxAlienNode {
            id: "output:manifest".to_string(),
            kind: NodeKind::Output,
            label: "nxalien.manifest.json".to_string(),
            coord: [0.9, 0.7, 0.8, 0.6, 0.7, 0.5, 0.0, 0.9],
            payload: serde_json::json!({ "file": "nxalien.manifest.json" }),
        });
        hdag.try_add_edge("gate:G_A", "output:manifest", EdgeCause::HandoffToPSE, 1.0);

        hdag
    }
}

// ─── Private helpers ─────────────────────────────────────────────────────────

/// Semantic coherence R_A(vi, vj):
/// ψi·ψj·ρi·ρj·cos(ωi - ωj)·(1 - max(υi, υj))
fn semantic_coherence(vi: &NxAlienNode, vj: &NxAlienNode) -> f64 {
    let psi_i = vi.coord[0];
    let psi_j = vj.coord[0];
    let rho_i = vi.coord[1];
    let rho_j = vj.coord[1];
    let omega_i = vi.coord[2];
    let omega_j = vj.coord[2];
    let upsilon_i = vi.coord[6];
    let upsilon_j = vj.coord[6];
    psi_i * psi_j * rho_i * rho_j * (omega_i - omega_j).cos() * (1.0 - upsilon_i.max(upsilon_j))
}

/// Assign C^8 coordinate to a rule based on severity and evidence count.
fn rule_coord(rule: &RuleAtom) -> C8Coord {
    let evidence_density = (rule.evidence.len() as f64 / 3.0).min(1.0);
    match rule.severity {
        Severity::Blocking => [evidence_density, 0.9, 0.5, 0.7, 0.6, 1.0, 0.0, 0.9],
        Severity::Required => [evidence_density, 0.7, 0.5, 0.6, 0.5, 0.7, 0.1, 0.7],
        Severity::Advisory => [evidence_density, 0.4, 0.5, 0.4, 0.3, 0.3, 0.2, 0.4],
    }
}

/// Assign C^8 coordinate to an unknown slot.
fn unknown_coord(u: &UnknownSlot) -> C8Coord {
    match u.status {
        UnknownStatus::Resolved => [0.8, 0.6, 0.8, 0.5, 0.6, 0.5, 0.1, 0.6],
        UnknownStatus::Proposed => [0.4, 0.4, 0.5, 0.4, 0.4, 0.3, 0.5, 0.4],
        UnknownStatus::Stale | UnknownStatus::Superseded => {
            [0.3, 0.3, 0.3, 0.3, 0.3, 0.2, 0.7, 0.3]
        }
        UnknownStatus::Unknown => [0.1, 0.3, 0.5, 0.3, 0.3, 0.2, 0.9, 0.3],
    }
}

/// Map a trigger keyword to a specialist domain label.
fn specialist_for_keyword(kw: &str) -> &'static str {
    match kw {
        "layer" | "crate" | "api" | "adapter" | "API" => "architecture",
        "gate" | "commit" | "crystal" | "governance" => "safety",
        "test" | "ci" | "coverage" => "quality",
        "security" | "auth" | "token" => "security",
        _ => "",
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use pse_nxalien_types::{EvidenceRef, ScopeRef};

    fn test_metadata() -> ProjectMetadata {
        ProjectMetadata {
            name: "test-project".to_string(),
            language: "rust".to_string(),
            package_manager: "cargo".to_string(),
            test_command: Some("cargo test".to_string()),
            build_command: Some("cargo build".to_string()),
            fmt_command: Some("cargo fmt".to_string()),
            lint_command: Some("cargo clippy".to_string()),
            ..Default::default()
        }
    }

    fn test_rules() -> Vec<RuleAtom> {
        vec![
            RuleAtom {
                id: "rule-a".to_string(),
                scope: ScopeRef {
                    files: vec!["**/*.rs".to_string()],
                    ..Default::default()
                },
                trigger: vec!["test".to_string()],
                prescription: "Run tests".to_string(),
                severity: Severity::Blocking,
                evidence: vec![EvidenceRef {
                    kind: "file".to_string(),
                    source: "Cargo.toml".to_string(),
                }],
                gate: None,
                decay: None,
                hash: String::new(),
            }
            .with_hash(),
            RuleAtom {
                id: "rule-b".to_string(),
                scope: ScopeRef {
                    files: vec!["**/*.rs".to_string()],
                    ..Default::default()
                },
                trigger: vec!["fmt".to_string()],
                prescription: "Run fmt".to_string(),
                severity: Severity::Required,
                evidence: vec![EvidenceRef {
                    kind: "file".to_string(),
                    source: "Cargo.toml".to_string(),
                }],
                gate: None,
                decay: None,
                hash: String::new(),
            }
            .with_hash(),
        ]
    }

    #[test]
    fn graph_acyclic() {
        let hdag = HypercubeHdag::build_from_bundle(&test_metadata(), &test_rules(), &[]);
        assert!(hdag.is_acyclic());
    }

    #[test]
    fn deterministic_topological_order() {
        let h1 = HypercubeHdag::build_from_bundle(&test_metadata(), &test_rules(), &[]);
        let h2 = HypercubeHdag::build_from_bundle(&test_metadata(), &test_rules(), &[]);
        assert_eq!(h1.topological_order(), h2.topological_order());
    }

    #[test]
    fn cube_has_nodes_and_edges() {
        let hdag = HypercubeHdag::build_from_bundle(&test_metadata(), &test_rules(), &[]);
        let cube = hdag.to_cube();
        assert!(!cube.nodes.is_empty(), "cube must have nodes");
        assert!(!cube.edges.is_empty(), "cube must have edges");
        assert_eq!(cube.dimension, 8);
    }

    #[test]
    fn cycle_attempt_rejected() {
        let mut hdag = HypercubeHdag::new(NxAlienPolicy::default());
        hdag.add_node(NxAlienNode {
            id: "n1".to_string(),
            kind: NodeKind::Project,
            label: "n1".to_string(),
            coord: [0.9, 0.5, 0.5, 1.0, 0.5, 0.3, 0.1, 0.8],
            payload: serde_json::Value::Null,
        });
        hdag.add_node(NxAlienNode {
            id: "n2".to_string(),
            kind: NodeKind::Rule,
            label: "n2".to_string(),
            coord: [0.8, 0.7, 0.5, 0.7, 0.5, 0.7, 0.1, 0.7],
            payload: serde_json::Value::Null,
        });
        let added1 = hdag.try_add_edge("n1", "n2", EdgeCause::DerivesFrom, 0.9);
        let added2 = hdag.try_add_edge("n2", "n1", EdgeCause::Constrains, 0.9);
        assert!(added1, "first edge should be added");
        assert!(!added2, "cycle-forming edge must be rejected");
        assert!(hdag.is_acyclic());
    }

    #[test]
    fn project_5d_within_range() {
        let coord: C8Coord = [0.9, 0.5, 0.5, 1.0, 0.5, 0.3, 0.1, 0.8];
        let proj = HypercubeHdag::project_5d(&coord);
        for &v in &proj {
            assert!((0.0..=1.0).contains(&v), "5D projection out of range: {v}");
        }
    }
}

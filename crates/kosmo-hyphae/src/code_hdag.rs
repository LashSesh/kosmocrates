//! Code HDAG (Hypergraph DAG) skeleton.
//!
//! In Phase 3 this is a structural skeleton only — no real Rust parser is wired.
//! Nodes represent code observations; edges represent structural relations.
//! All HDAG artifacts preserve backref to their source evidence (CROSS-006).

use kosmo_core::{
    Digest, EnergyAssessment, EnergyFactors, EnergyKernel, Q16, TaintLabel, TripolarEnergy,
};
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
    /// Structural containment: a module contains a definition.
    Contains,
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
///
/// Edges are hashed in full (`from`, `to`, `kind`) — not merely counted — so two
/// graphs with the same node set but different wiring receive distinct ids.
#[derive(Serialize)]
struct HDAGContent {
    node_ids: Vec<Digest>,
    edges: Vec<(Digest, Digest, String)>,
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
            edges: edges
                .iter()
                .map(|e| (e.from_obs, e.to_obs, format!("{:?}", e.kind)))
                .collect(),
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

    /// Extract a real code HDAG from Rust source text.
    ///
    /// This is a deterministic, dependency-free **lexical** extractor — it scans
    /// the source line by line, not via a full AST parser. It recognises the
    /// structurally meaningful Rust constructs that define a file's topology:
    ///
    /// * `mod` declarations          → [`ObservationKind::ModuleDeclaration`]
    /// * `use` imports               → [`ObservationKind::ImportStatement`]
    /// * `fn` definitions            → [`ObservationKind::FunctionDefinition`]
    /// * `struct`/`enum`/`trait`/`union`/`type` → [`ObservationKind::TypeDefinition`]
    /// * `#[test]`-attributed `fn`s  → [`ObservationKind::TestDefinition`]
    /// * `impl Trait for Type`       → an `Implements` edge
    ///
    /// A single root [`ObservationKind::ModuleDeclaration`] node represents the
    /// file; every definition is linked to it (`Contains`, `Tests`, `Imports`,
    /// or `Implements`). Line comments (`//`) are skipped. Each observation's
    /// `fragment_digest` is bound to its actual `location:line:text`, so the
    /// graph is content-addressed down to the source line and identical input
    /// always yields an identical `hdag_id` (INVARIANT-007).
    ///
    /// This is intentionally not a complete parser: it does not resolve macro
    /// expansions, multi-line signatures beyond the opening line, or call
    /// graphs. It captures the module/import/definition skeleton — real topology,
    /// extracted cheaply and reproducibly.
    pub fn extract_from_rust_source(
        source_evidence_id: Digest,
        location: &str,
        content: &str,
        taint: TaintLabel,
    ) -> Self {
        // Root node for the file.
        let root_obs = CodeObservation::new(
            ObservationKind::ModuleDeclaration { name: location.to_string() },
            location.to_string(),
            Digest::of_bytes(format!("{location}:0:<root>").as_bytes()),
            taint.clone(),
            source_evidence_id,
        );
        let root_id = root_obs.obs_id;
        let mut nodes = vec![HDAGNode::from_observation(&root_obs)];
        let mut edges: Vec<HDAGEdge> = Vec::new();

        let mut pending_test = false;
        for (idx, raw) in content.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }

            // A `#[test]` / `#[tokio::test]` attribute marks the next `fn`.
            if is_test_attribute(line) {
                pending_test = true;
                continue;
            }

            let frag = Digest::of_bytes(format!("{location}:{}:{line}", idx + 1).as_bytes());
            let lineloc = format!("{location}:{}", idx + 1);

            if let Some(target) = parse_use(line) {
                push(
                    &mut nodes,
                    &mut edges,
                    root_id,
                    ObservationKind::ImportStatement { module: target },
                    lineloc,
                    frag,
                    &taint,
                    source_evidence_id,
                    HDAGEdgeKind::Imports,
                );
            } else if let Some((trait_name, type_name)) = parse_impl_for(line) {
                push(
                    &mut nodes,
                    &mut edges,
                    root_id,
                    ObservationKind::Custom {
                        description: format!("impl {trait_name} for {type_name}"),
                    },
                    lineloc,
                    frag,
                    &taint,
                    source_evidence_id,
                    HDAGEdgeKind::Implements,
                );
            } else if let Some(name) = parse_mod(line) {
                push(
                    &mut nodes,
                    &mut edges,
                    root_id,
                    ObservationKind::ModuleDeclaration { name },
                    lineloc,
                    frag,
                    &taint,
                    source_evidence_id,
                    HDAGEdgeKind::Contains,
                );
            } else if let Some(name) = parse_type(line) {
                push(
                    &mut nodes,
                    &mut edges,
                    root_id,
                    ObservationKind::TypeDefinition { name },
                    lineloc,
                    frag,
                    &taint,
                    source_evidence_id,
                    HDAGEdgeKind::Contains,
                );
            } else if let Some((name, arity)) = parse_fn(line) {
                let (kind, edge) = if pending_test {
                    (
                        ObservationKind::TestDefinition { name },
                        HDAGEdgeKind::Tests,
                    )
                } else {
                    (
                        ObservationKind::FunctionDefinition { name, arity },
                        HDAGEdgeKind::Contains,
                    )
                };
                push(
                    &mut nodes, &mut edges, root_id, kind, lineloc, frag, &taint,
                    source_evidence_id, edge,
                );
            }

            // The pending-test marker only applies to the immediately following
            // recognised line; reset it once a non-attribute line is processed.
            pending_test = false;
        }

        Self::new(nodes, edges, source_evidence_id, taint)
    }

    /// Number of definition nodes excluding the file root.
    pub fn definition_count(&self) -> usize {
        self.nodes.len().saturating_sub(1)
    }

    /// Count edges of a given kind.
    pub fn edges_of_kind(&self, kind: &HDAGEdgeKind) -> usize {
        self.edges.iter().filter(|e| &e.kind == kind).count()
    }

    // ─── Topology → tripolar energy bridge ──────────────────────────────────
    //
    // These derive the *structural* poles ρ (coherence) and ω (phase) of the
    // unified tripolar energy `D = ψ · ρ · ω` directly from the extracted graph.
    // The semantic pole ψ (goal-fit) is NOT derived here — meaning cannot come
    // from syntax alone, so the caller supplies it. The derivations are pure
    // structural proxies, deterministic and float-free; they feed selection
    // ranking only and never act as a gate (see `kosmo_core::energy`).

    /// Structural-coherence pole ρ ∈ `[0, 1]`.
    ///
    /// A module whose contained definitions are exercised by tests is more
    /// coherent. `ρ = 1/2 + 1/2 · min(1, tests / contains)`; an empty module
    /// (no definitions) has ρ = 0. This is a structural proxy, not a semantic
    /// judgement.
    pub fn rho_coherence(&self) -> Q16 {
        if self.definition_count() == 0 {
            return Q16::ZERO;
        }
        let contains = self.edges_of_kind(&HDAGEdgeKind::Contains).max(1) as u64;
        let tests = self.edges_of_kind(&HDAGEdgeKind::Tests) as u64;
        let tested_ratio = if tests >= contains {
            Q16::ONE
        } else {
            Q16::ratio(tests, contains).unwrap_or(Q16::ZERO)
        };
        Q16::HALF.saturating_add(
            Q16::HALF
                .checked_mul(tested_ratio)
                .unwrap_or(Q16::ZERO),
        )
    }

    /// Phase / topological-coherence pole ω ∈ `[0, 1]`.
    ///
    /// Structural completeness from the presence of the three load-bearing
    /// signals: imports (connection outward), contained definitions, and tests.
    /// `ω = present_signals / 3`.
    pub fn omega_phase(&self) -> Q16 {
        let signals = u64::from(self.edges_of_kind(&HDAGEdgeKind::Imports) > 0)
            + u64::from(self.edges_of_kind(&HDAGEdgeKind::Contains) > 0)
            + u64::from(self.edges_of_kind(&HDAGEdgeKind::Tests) > 0);
        Q16::ratio(signals, 3).unwrap_or(Q16::ZERO)
    }

    /// Build the unified [`EnergyKernel`] for this topology.
    ///
    /// `psi_goal_fit` is the caller-supplied semantic pole ψ; ρ and ω are derived
    /// from the graph. `factors` carries the fail-closed gate/taint/license/
    /// foundry/seam/contradiction modulation.
    pub fn energy_kernel(&self, psi_goal_fit: Q16, factors: EnergyFactors) -> EnergyKernel {
        EnergyKernel::new(
            TripolarEnergy::new(psi_goal_fit, self.rho_coherence(), self.omega_phase()),
            factors,
        )
    }

    /// Content-addressed, evidence-bound [`EnergyAssessment`] for this topology,
    /// keyed on the graph's `hdag_id`.
    pub fn energy_assessment(
        &self,
        psi_goal_fit: Q16,
        factors: EnergyFactors,
        policy_id: Digest,
        evidence_bundle_id: Digest,
    ) -> EnergyAssessment {
        EnergyAssessment::new(
            self.hdag_id,
            self.energy_kernel(psi_goal_fit, factors),
            policy_id,
            evidence_bundle_id,
        )
    }
}

/// Push one observation node and its edge from `root_id`.
#[allow(clippy::too_many_arguments)]
fn push(
    nodes: &mut Vec<HDAGNode>,
    edges: &mut Vec<HDAGEdge>,
    root_id: Digest,
    kind: ObservationKind,
    location: String,
    fragment_digest: Digest,
    taint: &TaintLabel,
    source_evidence_id: Digest,
    edge_kind: HDAGEdgeKind,
) {
    let obs = CodeObservation::new(
        kind,
        location,
        fragment_digest,
        taint.clone(),
        source_evidence_id,
    );
    let node = HDAGNode::from_observation(&obs);
    edges.push(HDAGEdge {
        from_obs: root_id,
        to_obs: node.node_id,
        kind: edge_kind,
    });
    nodes.push(node);
}

/// Strip a leading visibility qualifier (`pub`, `pub(crate)`, …) and other
/// definition qualifiers, returning the remaining tokens.
fn strip_qualifiers(line: &str) -> String {
    let mut s = line.trim();
    // Drop a leading `pub` or `pub(...)`.
    if let Some(rest) = s.strip_prefix("pub") {
        let rest = rest.trim_start();
        if let Some(after_paren) = rest.strip_prefix('(') {
            if let Some(close) = after_paren.find(')') {
                s = after_paren[close + 1..].trim_start();
            } else {
                s = rest;
            }
        } else {
            s = rest;
        }
    }
    s.to_string()
}

/// Parse a `use <path>;` import target (the path, without trailing `;`).
fn parse_use(line: &str) -> Option<String> {
    let s = strip_qualifiers(line);
    let rest = s.strip_prefix("use ")?;
    let target = rest.trim().trim_end_matches(';').trim();
    if target.is_empty() {
        None
    } else {
        Some(target.to_string())
    }
}

/// Parse a `mod <name>;` or `mod <name> {` declaration name.
fn parse_mod(line: &str) -> Option<String> {
    let s = strip_qualifiers(line);
    let rest = s.strip_prefix("mod ")?;
    let name = first_ident(rest)?;
    Some(name)
}

/// Parse a type definition (`struct`/`enum`/`trait`/`union`/`type`) name.
fn parse_type(line: &str) -> Option<String> {
    let s = strip_qualifiers(line);
    // Strip further qualifiers that can precede the keyword.
    let s = s
        .trim_start_matches("unsafe ")
        .trim_start_matches("async ")
        .trim_start();
    for kw in ["struct ", "enum ", "trait ", "union ", "type "] {
        if let Some(rest) = s.strip_prefix(kw) {
            return first_ident(rest);
        }
    }
    None
}

/// Parse a function definition: returns `(name, arity)`.
///
/// `arity` counts comma-separated parameters on the signature's opening line
/// (0 when the parameter list is empty). `self` receivers are counted as
/// parameters — this is a lexical approximation, not type resolution.
fn parse_fn(line: &str) -> Option<(String, u32)> {
    let s = strip_qualifiers(line);
    // Allow `const`, `async`, `unsafe`, `extern "C"` qualifiers before `fn`.
    let mut s = s.as_str();
    loop {
        let trimmed = s.trim_start();
        if let Some(r) = trimmed.strip_prefix("const ") {
            s = r;
        } else if let Some(r) = trimmed.strip_prefix("async ") {
            s = r;
        } else if let Some(r) = trimmed.strip_prefix("unsafe ") {
            s = r;
        } else if let Some(r) = trimmed.strip_prefix("extern ") {
            // skip an optional ABI string literal
            let r = r.trim_start();
            if let Some(after) = r.strip_prefix('"') {
                if let Some(end) = after.find('"') {
                    s = after[end + 1..].trim_start();
                    continue;
                }
            }
            s = r;
        } else {
            break;
        }
    }
    let rest = s.trim_start().strip_prefix("fn ")?;
    let name = first_ident(rest)?;
    // Arity: count commas inside the first (...) on this line.
    let arity = match (rest.find('('), rest.find(')')) {
        (Some(o), Some(c)) if c > o => {
            let inner = rest[o + 1..c].trim();
            if inner.is_empty() {
                0
            } else {
                (inner.matches(',').count() as u32) + 1
            }
        }
        _ => 0,
    };
    Some((name, arity))
}

/// Parse `impl <Trait> for <Type>` → `(trait_name, type_name)`.
///
/// Inherent `impl Type` blocks (no `for`) are intentionally not returned as an
/// `Implements` edge.
fn parse_impl_for(line: &str) -> Option<(String, String)> {
    let s = line.trim();
    // Accept both `impl Foo for Bar` and `impl<T> Foo for Bar` (no space).
    let after_impl = s.strip_prefix("impl")?;
    if !(after_impl.starts_with(' ') || after_impl.starts_with('<')) {
        return None;
    }
    let rest = after_impl.trim_start();
    // Generic impls like `impl<T> Foo for Bar` — drop a leading `<...>`.
    let rest = if rest.starts_with('<') {
        let close = rest.find('>')?;
        rest[close + 1..].trim_start()
    } else {
        rest
    };
    let for_pos = rest.find(" for ")?;
    let trait_name = rest[..for_pos].trim();
    let after_for = rest[for_pos + 5..].trim();
    let type_name = after_for
        .split(|c: char| c == '{' || c == ' ' || c == '<' || c == '\n')
        .next()
        .unwrap_or("")
        .trim();
    if trait_name.is_empty() || type_name.is_empty() {
        None
    } else {
        Some((trait_name.to_string(), type_name.to_string()))
    }
}

/// Is this line a test attribute (`#[test]`, `#[tokio::test]`, …)?
fn is_test_attribute(line: &str) -> bool {
    let s = line.trim();
    s == "#[test]" || (s.starts_with("#[") && s.ends_with("test]") && s.contains("test"))
}

/// Extract the first Rust identifier from the start of `s`, stopping at any
/// non-identifier character (`<`, `(`, `{`, `;`, whitespace, …).
fn first_ident(s: &str) -> Option<String> {
    let s = s.trim_start();
    let ident: String = s
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if ident.is_empty() {
        None
    } else {
        Some(ident)
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

    const SAMPLE: &str = r#"
// a comment
use std::collections::BTreeMap;
use crate::digest::Digest;

pub mod inner;

pub struct Widget {
    pub size: u32,
}

pub enum Color { Red, Green }

pub trait Render {
    fn render(&self) -> String;
}

impl Render for Widget {
    fn render(&self) -> String { String::new() }
}

pub fn build(a: u32, b: u32) -> Widget {
    Widget { size: a + b }
}

fn helper() {}

#[test]
fn it_builds() {
    assert!(true);
}
"#;

    fn extract() -> CodeHDAG {
        CodeHDAG::extract_from_rust_source(
            Digest::of_bytes(b"ev"),
            "src/widget.rs",
            SAMPLE,
            TaintLabel::Clean,
        )
    }

    #[test]
    fn extract_captures_imports() {
        let h = extract();
        // `use std::collections::BTreeMap;` and `use crate::digest::Digest;`
        assert_eq!(h.edges_of_kind(&HDAGEdgeKind::Imports), 2);
        assert!(h.nodes.iter().any(|n| n.label.contains("widget.rs")));
        let import_obs: Vec<_> = h
            .nodes
            .iter()
            .filter(|n| n.label.starts_with("src/widget.rs:"))
            .collect();
        assert!(!import_obs.is_empty());
    }

    #[test]
    fn extract_captures_types_and_fns_and_tests() {
        let h = extract();
        // struct Widget, enum Color, trait Render  → 3 TypeDefinitions
        // fn build, fn helper                       → 2 FunctionDefinitions (Contains)
        //   (trait method `fn render` and impl `fn render` are also fns)
        // #[test] fn it_builds                       → 1 TestDefinition (Tests)
        assert_eq!(h.edges_of_kind(&HDAGEdgeKind::Tests), 1, "one #[test] fn");
        assert_eq!(
            h.edges_of_kind(&HDAGEdgeKind::Implements),
            1,
            "impl Render for Widget"
        );
        // Real topology: many more than the 1-node skeleton.
        assert!(
            h.definition_count() >= 8,
            "expected rich topology, got {} defs",
            h.definition_count()
        );
    }

    #[test]
    fn extract_records_fn_arity() {
        assert_eq!(
            super::parse_fn("pub fn build(a: u32, b: u32) -> Widget {"),
            Some(("build".to_string(), 2))
        );
        assert_eq!(super::parse_fn("fn helper() {}"), Some(("helper".to_string(), 0)));
    }

    #[test]
    fn extract_is_deterministic() {
        let h1 = extract();
        let h2 = extract();
        assert_eq!(h1.hdag_id, h2.hdag_id, "identical source → identical hdag_id");
    }

    #[test]
    fn extract_differs_from_skeleton() {
        let h = extract();
        let sk = CodeHDAG::skeleton_for_source(
            Digest::of_bytes(b"ev"),
            "src/widget.rs",
            TaintLabel::Clean,
        );
        assert_ne!(h.hdag_id, sk.hdag_id);
        assert!(h.nodes.len() > sk.nodes.len());
        assert!(!h.edges.is_empty());
        assert!(sk.edges.is_empty());
    }

    #[test]
    fn extract_propagates_taint() {
        let h = CodeHDAG::extract_from_rust_source(
            Digest::of_bytes(b"ev"),
            "x.rs",
            "use a::b;\npub fn f() {}",
            TaintLabel::External,
        );
        assert_eq!(h.taint, TaintLabel::External);
    }

    #[test]
    fn extract_skips_comments_and_blank_lines() {
        let h = CodeHDAG::extract_from_rust_source(
            Digest::of_bytes(b"ev"),
            "x.rs",
            "// just a comment\n\n   \n// another",
            TaintLabel::Clean,
        );
        assert_eq!(h.definition_count(), 0, "only the root node, no definitions");
    }

    #[test]
    fn topology_energy_richer_module_outranks_empty() {
        use kosmo_core::{EnergyFactors, Q16};
        let rich = extract();
        let empty = CodeHDAG::extract_from_rust_source(
            Digest::of_bytes(b"ev"),
            "empty.rs",
            "// nothing here\n",
            TaintLabel::Clean,
        );
        let psi = Q16::ONE; // same goal-fit for both → compare topology only
        let rich_k = rich.energy_kernel(psi, EnergyFactors::all_clean());
        let empty_k = empty.energy_kernel(psi, EnergyFactors::all_clean());
        assert!(
            rich_k.energy().raw() > empty_k.energy().raw(),
            "a complete, tested module must out-rank an empty one"
        );
        assert_eq!(empty_k.energy(), Q16::ZERO, "empty topology → zero energy");
    }

    #[test]
    fn topology_energy_assessment_is_content_addressed() {
        use kosmo_core::{EnergyFactors, Q16};
        let h = extract();
        let a1 = h.energy_assessment(
            Q16::HALF,
            EnergyFactors::all_clean(),
            Digest::of_bytes(b"pol"),
            Digest::of_bytes(b"ev-bundle"),
        );
        let a2 = h.energy_assessment(
            Q16::HALF,
            EnergyFactors::all_clean(),
            Digest::of_bytes(b"pol"),
            Digest::of_bytes(b"ev-bundle"),
        );
        assert_eq!(a1.id, a2.id);
        assert!(a1.verify_id());
        assert_eq!(a1.subject_id, h.hdag_id, "assessment keyed on the hdag_id");
    }

    #[test]
    fn topology_energy_omega_full_when_all_signals_present() {
        use kosmo_core::Q16;
        // SAMPLE has imports, contained defs, and a test → ω = 3/3 = 1.
        assert_eq!(extract().omega_phase(), Q16::ONE);
    }

    #[test]
    fn topology_energy_reject_gate_zeroes_energy() {
        use kosmo_core::{EnergyFactors, FoundrySurvival, GateResult, LicenseStatus, Q16, TaintLabel as TL};
        let h = extract();
        let factors = EnergyFactors::derive(
            &GateResult::Reject { reason: "tainted".into() },
            &TL::Clean,
            &LicenseStatus::Permissive { spdx: "MIT".into() },
            FoundrySurvival::Passed,
            Q16::ONE,
            Q16::ZERO,
        );
        let k = h.energy_kernel(Q16::ONE, factors);
        assert!(k.is_zeroed(), "real topology cannot bypass a Reject gate");
    }

    #[test]
    fn parse_helpers_handle_visibility_and_qualifiers() {
        assert_eq!(super::parse_type("pub(crate) struct Foo {"), Some("Foo".to_string()));
        assert_eq!(super::parse_type("pub enum Bar {"), Some("Bar".to_string()));
        assert_eq!(super::parse_mod("pub mod inner;"), Some("inner".to_string()));
        assert_eq!(super::parse_use("pub use crate::x::Y;"), Some("crate::x::Y".to_string()));
        assert_eq!(
            super::parse_impl_for("impl<T> Render for Widget<T> {"),
            Some(("Render".to_string(), "Widget".to_string()))
        );
        assert_eq!(super::parse_fn("pub async fn run(self) {"), Some(("run".to_string(), 1)));
    }
}

//! Canonical types and schemas for the nxalien agent-context exoskeleton.

use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::BTreeMap;

// ─── Enumerations ─────────────────────────────────────────────────────────────

/// Severity level for a rule atom.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Advisory,
    Required,
    Blocking,
}

/// Lifecycle status for an unknown slot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UnknownStatus {
    Unknown,
    Proposed,
    Resolved,
    Stale,
    Superseded,
}

/// Outcome of the 8-gate conjunctive evaluation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum GateOutcome {
    #[default]
    Reject,
    Hold,
    EvidenceOnly,
    Accept,
}

/// Kind of node in the Hypercube-HDAG.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeKind {
    Project,
    Rule,
    Unknown,
    Evidence,
    Specialist,
    Tool,
    Gate,
    Output,
}

/// Causal relationship between two nodes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EdgeCause {
    DerivesFrom,
    Constrains,
    Triggers,
    ValidatesBy,
    Resolves,
    Supersedes,
    HandoffToPSE,
}

// ─── Core Structs ─────────────────────────────────────────────────────────────

/// Scope reference: file globs, module names, and command patterns.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScopeRef {
    /// Glob patterns matching files in scope.
    pub files: Vec<String>,
    /// Module names in scope.
    pub modules: Vec<String>,
    /// Command patterns in scope.
    pub commands: Vec<String>,
}

/// Reference to a piece of evidence.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct EvidenceRef {
    /// Kind: "file", "command", "user_directive", "pse_retrieval".
    pub kind: String,
    /// Path, command, or description.
    pub source: String,
}

/// A single rule atom — the atomic unit of the rule policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleAtom {
    /// Unique rule identifier.
    pub id: String,
    /// Scope of applicability.
    pub scope: ScopeRef,
    /// Keywords that trigger this rule.
    pub trigger: Vec<String>,
    /// The prescription text.
    pub prescription: String,
    /// Severity level.
    pub severity: Severity,
    /// Evidence references.
    pub evidence: Vec<EvidenceRef>,
    /// Gate expression string (optional).
    pub gate: Option<String>,
    /// Decay policy string (optional).
    pub decay: Option<String>,
    /// SHA-256 of JCS(self without hash field).
    pub hash: String,
}

/// An unresolved unknown in the project context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnknownSlot {
    /// Name of the unknown.
    pub name: String,
    /// Domain tags.
    pub domain: Vec<String>,
    /// Current status.
    pub status: UnknownStatus,
    /// Candidate resolutions.
    pub candidates: Vec<String>,
    /// Evidence references.
    pub evidence: Vec<EvidenceRef>,
    /// Resolution text, if resolved.
    pub resolution: Option<String>,
    /// Confidence score (0.0–1.0).
    pub confidence: f64,
    /// Expiration unix timestamp (optional).
    pub expires: Option<u64>,
}

/// Descriptor for an nxalien run — uniquely identifies a pipeline execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NxAlienRunDescriptor {
    /// Schema version.
    pub schema_version: String,
    /// SHA-256 of the project root content digest.
    pub project_root_digest: String,
    /// SHA-256 of the policy.
    pub policy_digest: String,
    /// Deterministic seed.
    pub seed: u64,
    /// ISO-8601 UTC timestamp.
    pub started_at_utc: String,
}

impl Default for NxAlienRunDescriptor {
    fn default() -> Self {
        Self {
            schema_version: "1.0".to_string(),
            project_root_digest: String::new(),
            policy_digest: String::new(),
            seed: 0,
            started_at_utc: String::new(),
        }
    }
}

/// Detected project metadata from project scanner.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectMetadata {
    /// Project name.
    pub name: String,
    /// Primary language: "rust", "typescript", etc.
    pub language: String,
    /// Package manager: "cargo", "npm", "pip", etc.
    pub package_manager: String,
    /// SPDX license identifier.
    pub license: Option<String>,
    /// Command to run tests.
    pub test_command: Option<String>,
    /// Command to build.
    pub build_command: Option<String>,
    /// Command to format.
    pub fmt_command: Option<String>,
    /// Command to lint.
    pub lint_command: Option<String>,
    /// Existing context files found (AGENTS.md, CLAUDE.md, .rules, etc.).
    pub existing_context_files: Vec<String>,
}

/// Policy parameters controlling nxalien pipeline behaviour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NxAlienPolicy {
    /// Maximum characters for context block output.
    pub max_context_chars: usize,
    /// Semantic coherence threshold τ_A.
    pub tau_a: f64,
    /// Causal drift tolerance ε_η.
    pub eta_eps: f64,
    /// Minimum evidence refs for a Required rule.
    pub min_evidence_for_required: usize,
    /// Whether replay verification is required.
    pub require_replay: bool,
}

impl Default for NxAlienPolicy {
    fn default() -> Self {
        Self {
            max_context_chars: 8000,
            tau_a: 0.05,
            eta_eps: 0.1,
            min_evidence_for_required: 1,
            require_replay: true,
        }
    }
}

// ─── C^8 Embedding ────────────────────────────────────────────────────────────

/// C^8 embedding coordinate — one value per axis:
/// [ψ evidence_potential, ρ rule_density, ω temporal_phase,
///  χ connectivity, η causality, γ governance, υ uncertainty, λ utility]
pub type C8Coord = [f64; 8];

/// A node in the Hypercube-HDAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NxAlienNode {
    /// Unique node identifier.
    pub id: String,
    /// Node kind.
    pub kind: NodeKind,
    /// Human-readable label.
    pub label: String,
    /// C^8 embedding coordinate.
    pub coord: C8Coord,
    /// Kind-specific payload as JSON.
    pub payload: serde_json::Value,
}

/// A directed edge in the Hypercube-HDAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NxAlienEdge {
    /// Source node id.
    pub from: String,
    /// Target node id.
    pub to: String,
    /// Causal relationship.
    pub cause: EdgeCause,
    /// Edge weight.
    pub weight: f64,
}

/// The compiled agent context cube — nodes + edges + policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContextCube {
    /// Embedding dimension (always 8).
    pub dimension: u8,
    /// All nodes, keyed by id.
    pub nodes: BTreeMap<String, NxAlienNode>,
    /// All edges.
    pub edges: Vec<NxAlienEdge>,
    /// Active policy.
    pub policy: NxAlienPolicy,
}

/// Result of the 8-gate conjunctive evaluation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NxAlienGateReport {
    /// G_evidence: all Required/Blocking rules have evidence.
    pub evidence_ok: bool,
    /// G_scope: all rules have non-empty scope.
    pub scope_ok: bool,
    /// G_replay: replay hash matches manifest.
    pub replay_ok: bool,
    /// G_canon: canonical hash is valid.
    pub canon_ok: bool,
    /// G_delta: diff budget within limits.
    pub delta_ok: bool,
    /// G_budget: context char count within limit.
    pub budget_ok: bool,
    /// G_governance: governance constraints satisfied.
    pub governance_ok: bool,
    /// G_bridge: PSE bridge admissibility confirmed.
    pub bridge_ok: bool,
    /// Overall gate outcome.
    pub outcome: GateOutcome,
    /// Diagnostic notes.
    pub notes: Vec<String>,
}

/// SHA-256 digest of a build artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactDigest {
    /// Path of the artifact.
    pub path: String,
    /// Hex-encoded SHA-256.
    pub sha256: String,
    /// File size in bytes.
    pub size_bytes: u64,
}

/// PSE bridge admissibility report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeAdmissibilityReport {
    /// Whether the candidate is admissible.
    pub admissible: bool,
    /// Human-readable reason.
    pub reason: String,
}

/// A candidate for handoff to the PSE bridge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NxAlienHandoffCandidate {
    /// Unique candidate identifier.
    pub candidate_id: String,
    /// SHA-256 of sorted observation byte slices.
    pub observation_bytes_digest: String,
    /// Source artifacts.
    pub source_artifacts: Vec<ArtifactDigest>,
    /// Intended bridge identifier.
    pub intended_bridge: String,
    /// Admissibility decision.
    pub admissibility: BridgeAdmissibilityReport,
}

/// The complete nxalien run bundle — all artefacts in one struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NxAlienBundle {
    /// Run descriptor.
    pub descriptor: NxAlienRunDescriptor,
    /// Hash of the manifest (JCS of NxAlienManifest minus manifest_hash field).
    pub manifest_hash: String,
    /// Hash of the context cube JSON.
    pub context_hash: String,
    /// Replay chain hash.
    pub replay_hash: String,
    /// Gate evaluation report.
    pub gate_report: NxAlienGateReport,
    /// Output artifacts.
    pub artifacts: Vec<ArtifactDigest>,
    /// PSE bridge handoff candidates.
    pub handoff_candidates: Vec<NxAlienHandoffCandidate>,
    /// Compiled rule atoms.
    pub rules: Vec<RuleAtom>,
    /// Open unknown slots.
    pub unknowns: Vec<UnknownSlot>,
    /// Project metadata.
    pub metadata: ProjectMetadata,
}

// ─── RuleAtom hash impl ───────────────────────────────────────────────────────

#[derive(Serialize)]
struct RuleAtomHashable<'a> {
    id: &'a str,
    scope: &'a ScopeRef,
    trigger: &'a [String],
    prescription: &'a str,
    severity: &'a Severity,
    evidence: Vec<&'a EvidenceRef>,
    gate: Option<&'a str>,
    decay: Option<&'a str>,
}

impl RuleAtom {
    /// Compute canonical hash over all fields except `hash`.
    ///
    /// Evidence refs are sorted by (kind, source) so reordered evidence
    /// produces an identical hash.
    pub fn compute_hash(&self) -> String {
        let mut sorted_evidence: Vec<&EvidenceRef> = self.evidence.iter().collect();
        sorted_evidence.sort();
        let hashable = RuleAtomHashable {
            id: &self.id,
            scope: &self.scope,
            trigger: &self.trigger,
            prescription: &self.prescription,
            severity: &self.severity,
            evidence: sorted_evidence,
            gate: self.gate.as_deref(),
            decay: self.decay.as_deref(),
        };
        let canonical = serde_jcs::to_vec(&hashable).expect("JCS serialization must not fail");
        let digest = sha2::Sha256::digest(&canonical);
        digest.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Return a clone with the `hash` field populated.
    pub fn with_hash(mut self) -> Self {
        self.hash = self.compute_hash();
        self
    }

    /// Verify that the stored hash matches the computed hash.
    pub fn verify_hash(&self) -> bool {
        self.hash == self.compute_hash()
    }
}

// ─── NxAlienManifest ─────────────────────────────────────────────────────────

/// Serialisable manifest written to nxalien.manifest.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NxAlienManifest {
    /// Schema version.
    pub schema_version: String,
    /// Run descriptor.
    pub descriptor: NxAlienRunDescriptor,
    /// Project metadata.
    pub metadata: ProjectMetadata,
    /// Compiled rule atoms.
    pub rules: Vec<RuleAtom>,
    /// Open unknown slots.
    pub unknowns: Vec<UnknownSlot>,
    /// Number of nodes in the cube.
    pub cube_node_count: usize,
    /// Number of edges in the cube.
    pub cube_edge_count: usize,
    /// Gate evaluation report.
    pub gate_report: NxAlienGateReport,
    /// Hash of the manifest (JCS of self minus this field).
    pub manifest_hash: String,
    /// Replay chain hash.
    pub replay_hash: String,
}

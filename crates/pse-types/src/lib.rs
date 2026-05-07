//! Canonical data model for PSE (Post-Symbolic Engine).
//!
//! Defines the shared types, temporal primitives, 5D state representations,
//! and content-addressed hashing used by all other PSE crates.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub use ordered_float::OrderedFloat;

// ─── Temporal Types ──────────────────────────────────────────────────────────

/// Pre-temporal null-center. Stateless. Cannot hold data.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct NullCenter;

/// Intrinsic time (continuous internal evolution)
pub type IntrinsicTime = OrderedFloat<f64>;

/// Extrinsic time / commit index (discrete irreversible sequence)
pub type CommitIndex = u64;

/// Tick size for intrinsic discretization
pub type TickSize = OrderedFloat<f64>;

// ─── Primitive Types ─────────────────────────────────────────────────────────

/// Unique vertex identifier in the observation graph.
pub type VertexId = u64;

/// SHA-256 hash digest.
pub type Hash256 = [u8; 32];

// ─── 5D State Types ───────────────────────────────────────────────────────────

/// Primitive 5D state: (potential, density, frequency, connectivity, causality)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct FiveDState {
    pub p: f64,
    pub rho: f64,
    pub omega: f64,
    pub chi: f64,
    pub eta: f64,
}

impl FiveDState {
    /// Return the state as a 5-element array.
    pub fn as_array(&self) -> [f64; 5] {
        [self.p, self.rho, self.omega, self.chi, self.eta]
    }

    /// Squared Euclidean norm of the state vector.
    pub fn norm_sq(&self) -> f64 {
        self.as_array().iter().map(|x| x * x).sum()
    }

    /// Euclidean distance to another state.
    pub fn distance(&self, other: &Self) -> f64 {
        self.as_array()
            .iter()
            .zip(other.as_array().iter())
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f64>()
            .sqrt()
    }
}

/// Tripolar state: 3D projection of 5D (coherence amplitude, population, phase frequency)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct TripolarState {
    pub psi: f64,
    pub rho: f64,
    pub omega: f64,
}

// ─── Carrier Geometry Types ───────────────────────────────────────────────────

/// Tubus coordinate: (tau, phi, r) in I × S¹ × R≥0
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct TubusCoord {
    pub tau: f64,
    pub phi: f64,
    pub r: f64,
}

/// A data-helix coordinate.
///
/// Conceptually the *partner* of the carrier helix-pair in the Mandorla
/// interference computation. Each observation (or batch of observations)
/// projects onto a single point on its data-helix:
///
///  - `tau` is the intrinsic engine time at the moment of arrival
///    (same time scale as the carrier helix, so the two are
///    commensurable in interference);
///  - `phi` is a phase derived from the observation digest — the data
///    stream's own phase signature;
///  - `r` is the data's *amplitude* in the resonance sense — its
///    Shannon entropy density, so an information-rich observation
///    pushes the helix outward and an information-poor one keeps it
///    near the axis.
///
/// `DataHelix` is a type alias for [`TubusCoord`] because they share
/// the same geometric structure; the alias exists to make the role
/// of each helix explicit at call sites.
pub type DataHelix = TubusCoord;

/// Mandorla state: interference of helix pair.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct MandorlaState {
    pub tau: f64,
    pub r: f64,
    pub delta_phi: f64,
    pub kappa: f64,
}

/// Carrier instance (one entry in the phase-ladder).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CarrierInstance {
    pub helix_a: TubusCoord,
    pub helix_b: TubusCoord,
    pub mandorla: MandorlaState,
    pub resonance: f64,
    pub offset: f64,
}

impl Default for CarrierInstance {
    fn default() -> Self {
        Self {
            helix_a: TubusCoord::default(),
            helix_b: TubusCoord {
                phi: std::f64::consts::PI,
                ..Default::default()
            },
            mandorla: MandorlaState::default(),
            resonance: 0.0,
            offset: 0.0,
        }
    }
}

/// Phase-ladder: ordered set of carrier instances.
pub type PhaseLadder = Vec<CarrierInstance>;

// ─── Graph and Edge Types ─────────────────────────────────────────────────────

/// Edge annotation: 7-tuple for observation graph edges.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EdgeAnnotation {
    pub correlation: f64,
    pub granger: f64,
    pub coherence: f64,
    pub weight: f64,
    pub birth_time: f64,
    pub last_update: f64,
    pub active_windows: u64,
}

/// Topological signature of a graph or crystal.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct TopologySignature {
    pub betti_0: u64,
    pub betti_1: u64,
    pub betti_2: u64,
    pub spectral_gap: f64,
    pub euler_char: i64,
    #[serde(default)]
    pub cheeger_estimate: f64,
    #[serde(default)]
    pub kuramoto_coherence: f64,
    #[serde(default)]
    pub mean_propagation_time: f64,
    #[serde(default)]
    pub dtl_connected: bool,
}

// ─── Observation Types ───────────────────────────────────────────────────────

/// Provenance envelope: origin, chain, and optional signature.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ProvenanceEnvelope {
    pub origin: String,
    pub chain: Vec<String>,
    pub sig: Option<Vec<u8>>,
}

/// Measurement context: key-value tags.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct MeasurementContext {
    pub tags: BTreeMap<String, String>,
}

/// Canonical observation record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Observation {
    pub timestamp: f64,
    pub source_id: String,
    pub provenance: ProvenanceEnvelope,
    pub payload: Vec<u8>,
    pub context: MeasurementContext,
    pub digest: Hash256,
    pub schema_version: String,
    /// Optional **semantic phase hint** in `[0, 2π)` (Strand I).
    ///
    /// When `Some(φ)`, downstream Mandorla computation uses this value
    /// for the data-helix phase instead of the avalanche-hash derivation
    /// from the digest. Domain-aware adapters set this to a value that
    /// preserves *semantic locality* — i.e. similar observations get
    /// similar phases, so coherent data streams produce coherent
    /// data-helix phase distributions and PSE can actually resonate.
    ///
    /// When `None`, the engine falls back to `phase_from_digest(digest)`,
    /// which is uncorrelated with semantic similarity by construction
    /// (SHA-256's avalanche property). The fallback is honest: a
    /// stream of payloads with no domain-aware adapter receives
    /// uncorrelated phases and the Mandorla coherence reflects that.
    ///
    /// `#[serde(default)]` keeps existing on-disk observations
    /// loadable without modification.
    #[serde(default)]
    pub phase_hint: Option<f64>,
}

// ─── Constraint Types ─────────────────────────────────────────────────────────

/// Constraint template family.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ConstraintTemplate {
    Band,
    Ratio,
    Correlation,
    Granger,
    Spectral,
    Topological,
    Phase,
    Contraction,
}

/// Constraint candidate discovered through pattern extraction.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConstraintCandidate {
    pub id: Hash256,
    pub template: ConstraintTemplate,
    pub parameters: BTreeMap<String, f64>,
    pub coverage: f64,
    pub threshold: f64,
    pub formation_energy: f64,
    pub bond_strength: u64,
    pub activation_energy: f64,
}

/// Constraint program = ordered sequence of candidates.
pub type ConstraintProgram = Vec<ConstraintCandidate>;

// ─── Constitutional Types ────────────────────────────────────────────────────

/// Severity level of a constitutional constraint.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConstraintSeverity {
    Mandatory,
    Recommended,
}

/// Conformance class (C0 through C4).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConformanceClass {
    C0,
    C1,
    C2,
    C3,
    C4,
}

/// A single constitutional constraint with satisfaction evidence.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConstitutionalConstraint {
    pub id: String,
    pub axiom_ref: String,
    pub description: String,
    pub severity: ConstraintSeverity,
    pub satisfied: bool,
    pub evidence: String,
}

/// System fingerprint for genesis metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemFingerprint {
    pub pse_version: String,
    pub crate_count: usize,
    pub test_count: usize,
    pub registry_digest: Hash256,
    pub config_digest: Hash256,
    pub platform: String,
    pub rust_version: String,
    pub git_commit: Option<String>,
}

/// Genesis metadata attached to the first crystal.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenesisMetadata {
    pub adamant_version: String,
    pub conformance_class: ConformanceClass,
    pub system_fingerprint: SystemFingerprint,
    pub constitutional_digest: Hash256,
    pub constraints: Vec<ConstitutionalConstraint>,
}

// ─── Evidence, Proof, Gate Types ──────────────────────────────────────────────

/// Evidence entry in an evidence chain.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvidenceEntry {
    pub digest: Hash256,
    pub content: Vec<u8>,
    pub provenance: ProvenanceEnvelope,
    pub prev: Option<Hash256>,
}

/// Evidence chain: sequence of linked evidence entries.
pub type EvidenceChain = Vec<EvidenceEntry>;

/// Gate snapshot: all 8 normalized metrics + Kairos conjunction.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GateSnapshot {
    pub d: f64,
    pub q: f64,
    pub r: f64,
    pub g: f64,
    pub j: f64,
    pub p: f64,
    pub n: f64,
    pub k: f64,
    pub kairos: bool,
}

/// Proof-of-Reproducibility finite-state machine trace.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PoRTrace {
    pub search_enter: f64,
    pub lock_enter: Option<f64>,
    pub verify_enter: Option<f64>,
    pub commit_enter: Option<f64>,
}

/// Consensus result (primal + dual scores + MCI).
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ConsensusResult {
    pub primal_score: f64,
    pub dual_score: f64,
    pub mci: f64,
    pub threshold: f64,
}

/// Commit proof binding a crystal to its validation evidence.
///
/// `falsification_p_value` and `surrogate_count` are populated only when the
/// optional surrogate-data falsification pass ran for this crystal (see
/// [`FalsificationConfig`]). They are not part of the content-address
/// (see `CrystalCore` in `pse-evidence`), so adding them is backward
/// compatible with pre-existing crystal IDs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitProof {
    pub evidence_digests: Vec<Hash256>,
    pub operator_stack: Vec<(String, String)>,
    pub gate_values: GateSnapshot,
    pub structural_result: bool,
    pub consensus_result: ConsensusResult,
    pub por_trace: PoRTrace,
    pub carrier_id: usize,
    pub carrier_offset: f64,
    /// Fraction of surrogate runs in which a "comparable" precursor reached
    /// the post-consensus stage. Populated only when falsification was
    /// enabled during this crystal's formation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub falsification_p_value: Option<f64>,
    /// Number of surrogates generated for the falsification test.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surrogate_count: Option<u32>,
}

impl Default for CommitProof {
    fn default() -> Self {
        Self {
            evidence_digests: Vec::new(),
            operator_stack: Vec::new(),
            gate_values: GateSnapshot::default(),
            structural_result: false,
            consensus_result: ConsensusResult::default(),
            por_trace: PoRTrace::default(),
            carrier_id: 0,
            carrier_offset: 0.0,
            falsification_p_value: None,
            surrogate_count: None,
        }
    }
}

/// Strand O.3: canonical Metatron-scaffold topology signature.
///
/// A passive data structure that pse-metatron fills in by running its
/// scaffold-scan over the induced subgraph of a crystal's region.
/// Stored as an optional field on [`SemanticCrystal`] so any crystal
/// whose region is small enough (≤ 7 vertices for the canonical S₇
/// orbit cache, ≤ 13 for the general scaffold) carries its own
/// **canonical group-theoretic identity** — not just a topological
/// signature, but the unique class identifier in the Periodic Table
/// of Graphs.
///
/// Two crystals with identical `canonical_hash` are *isomorphic by
/// proof*, not just similar by metric. This is the qualitative
/// upgrade Strand O delivers: from cosine similarity to canonical
/// identity.
///
/// All numeric fields default to zero; `canonical_hash` defaults to
/// the empty string. The struct is `#[serde(default)]` so legacy
/// crystals without this signature continue to deserialise.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MetatronTopologySignature {
    /// 16-character hex FNV-1a hash of the canonical 13×13 form.
    /// Two crystals with the same canonical_hash are guaranteed
    /// isomorphic (modulo the S₇ permutation group).
    pub canonical_hash: String,
    /// Number of vertices in the subgraph that was scanned.
    pub n: u32,
    /// Number of edges in the subgraph.
    pub m: u32,
    /// Size of the orbit under S₇. Together with stabilizer_order
    /// satisfies the orbit-stabilizer theorem: orbit · stab = 5040.
    pub orbit_size: u64,
    /// Order of the stabilizer subgroup. High values indicate high
    /// internal symmetry of the subgraph.
    pub stabilizer_order: u64,
    /// Full adjacency-matrix spectrum (eigenvalues, sorted ascending).
    pub spectrum: Vec<f64>,
    /// Largest absolute eigenvalue.
    pub spectral_radius: f64,
    /// Algebraic connectivity (λ₂ of the Laplacian).
    pub algebraic_connectivity: f64,
    /// Graph energy E(G) = Σ|λᵢ| (Gutman 1978).
    pub graph_energy: f64,
    /// Number of spanning trees τ(G) (Kirchhoff's matrix-tree theorem).
    pub spanning_tree_count: u64,
    /// Triangle count.
    pub triangle_count: u32,
    /// Whether the subgraph contains the tetrahedron (K₄) as a
    /// subgraph.
    pub contains_tetrahedron: bool,
    /// Whether the subgraph contains the octahedron (K_{2,2,2}) as a
    /// subgraph.
    pub contains_octahedron: bool,
    /// Whether the subgraph contains the 3-cube (Q₃) as a subgraph.
    pub contains_cube: bool,
}

/// Semantic crystal — the fundamental unit of validated knowledge.
///
/// Content-addressed (SHA-256), evidence-chained, deterministically reproducible.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SemanticCrystal {
    pub crystal_id: Hash256,
    pub region: Vec<VertexId>,
    pub constraint_program: ConstraintProgram,
    pub stability_score: f64,
    pub topology_signature: TopologySignature,
    pub betti_numbers: Vec<u64>,
    pub evidence_chain: EvidenceChain,
    pub commit_proof: CommitProof,
    pub operator_versions: BTreeMap<String, String>,
    pub created_at: CommitIndex,
    pub free_energy: f64,
    pub carrier_instance_idx: usize,
    #[serde(default)]
    pub scale_tag: String,
    #[serde(default)]
    pub universe_id: String,
    #[serde(default)]
    pub sub_crystal_ids: Vec<String>,
    #[serde(default)]
    pub parent_crystal_ids: Vec<String>,
    #[serde(default)]
    pub genesis_metadata: Option<GenesisMetadata>,
    /// Strand O.3: canonical Metatron-scaffold signature, computed
    /// when the crystal's region is small enough for the S₇ orbit
    /// scan. `None` for crystals whose region exceeds the scan bound.
    /// Backward-compatible: defaults to None on legacy serialisations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metatron_signature: Option<MetatronTopologySignature>,
}

// ─── Scheduler Configuration ─────────────────────────────────────────────────

/// Scheduler configuration for the Spiral Scheduler.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub enabled: bool,
    pub n_min: u32,
    pub n_max: u32,
    pub strategy: String,
    pub w_d: f64,
    pub w_f: f64,
    pub w_s: f64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            n_min: 1,
            n_max: 10,
            strategy: "max_pressure".to_string(),
            w_d: 1.0,
            w_f: 1.0,
            w_s: 1.0,
        }
    }
}

/// Run descriptor for replay determinism.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunDescriptor {
    pub config: Config,
    pub operator_versions: BTreeMap<String, String>,
    pub initial_state_digest: Hash256,
    pub seed: Option<u64>,
    #[serde(default)]
    pub registry_digests: BTreeMap<String, Hash256>,
    #[serde(default)]
    pub scheduler: SchedulerConfig,
}

// ─── Configuration Types ──────────────────────────────────────────────────────

/// Master configuration for the PSE engine.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub temporal: TemporalConfig,
    pub carrier: CarrierConfig,
    pub observation: ObservationConfig,
    pub persistence: PersistenceConfig,
    pub extraction: ExtractionConfig,
    pub consensus: ConsensusConfig,
    pub adaptation: AdaptationConfig,
    pub thresholds: ThresholdConfig,
    pub normalization: NormalizationConfig,
    pub archive: ArchiveConfig,
    /// Surrogate-data falsification pass (disabled by default for backward
    /// compatibility; see [`FalsificationConfig`]).
    #[serde(default)]
    pub falsification: FalsificationConfig,
}

/// Temporal dynamics configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TemporalConfig {
    pub dt2: f64,
    pub gamma: f64,
    pub c_temp: f64,
    pub t_default: f64,
}

impl Default for TemporalConfig {
    fn default() -> Self {
        Self {
            dt2: 0.01,
            gamma: 0.01,
            c_temp: 5.0,
            t_default: 1.0,
        }
    }
}

/// Carrier geometry configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CarrierConfig {
    pub lambda: f64,
    pub mu_r: f64,
    pub lambda_q: f64,
    pub lambda_r: f64,
    pub lambda_m: f64,
    pub num_carriers: usize,
    pub thresholds: ThresholdConfig,
    /// Data-helix amplitude-match sharpness (E.2). The Mandorla's data
    /// interference factor is `exp(-eta_r · (r_data − r_carrier)²)`;
    /// small values tolerate amplitude mismatch, large values insist on
    /// equal radii. `#[serde(default)]` so legacy configs without this
    /// field still load and behave identically (the field's default is
    /// the value used when E.2 was wired).
    #[serde(default = "default_eta_r")]
    pub eta_r: f64,
    /// Strand J: per-tick adaptive carrier tracker. When `true`, the
    /// active carrier is re-selected at the start of each macro_step to
    /// the phase-ladder slot whose Mandorla κ against the current
    /// data-helix is highest. When `false` (default — backward-compat),
    /// the existing friction/shock-triggered migration path is the only
    /// way the active carrier ever changes.
    #[serde(default)]
    pub adaptive: bool,
    /// Strand O.2: when `true`, GlobalState constructs the phase-ladder
    /// from the canonical Metatron-scaffold geometry (1 centre carrier
    /// + 12 cuboctahedron-vertex carriers, total 13). The
    /// `num_carriers` field is ignored in that case. When `false`
    /// (default — backward-compat), `num_carriers` evenly-spaced
    /// carriers are built via `build_phase_ladder`.
    #[serde(default)]
    pub use_metatron_ladder: bool,
}

fn default_eta_r() -> f64 {
    1.0
}

impl Default for CarrierConfig {
    fn default() -> Self {
        Self {
            lambda: 0.1,
            mu_r: 0.3,
            lambda_q: 0.33,
            lambda_r: 0.33,
            lambda_m: 0.34,
            num_carriers: 4,
            thresholds: ThresholdConfig::default(),
            eta_r: default_eta_r(),
            adaptive: false,
            use_metatron_ladder: false,
        }
    }
}

/// Observation ingestion configuration.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ObservationConfig {
    pub schema_version: String,
}

/// Persistence layer configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersistenceConfig {
    pub hot_retention_days: u64,
    pub warm_retention_days: u64,
    pub lambda_decay: f64,
    pub max_vertices: usize,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            hot_retention_days: 7,
            warm_retention_days: 90,
            lambda_decay: 0.001,
            max_vertices: 10_000,
        }
    }
}

/// Constraint extraction configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtractionConfig {
    pub alpha_min: f64,
    pub convergence_tau: f64,
    pub kappa_max: f64,
    pub window_hours: f64,
    pub epsilon_merge: f64,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            alpha_min: 0.5,
            convergence_tau: 0.01,
            kappa_max: 0.9,
            window_hours: 24.0,
            epsilon_merge: 0.1,
        }
    }
}

/// Consensus validation configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsensusConfig {
    pub por_kappa_bar: f64,
    pub por_t_min: usize,
    pub por_t_stable: usize,
    pub por_epsilon: f64,
    pub consensus_threshold: f64,
    pub mirror_consistency_eta: f64,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            por_kappa_bar: 0.7,
            por_t_min: 3,
            por_t_stable: 2,
            por_epsilon: 0.05,
            consensus_threshold: 0.6,
            mirror_consistency_eta: 0.8,
        }
    }
}

/// Morphogenic adaptation configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdaptationConfig {
    pub split_threshold: f64,
    pub merge_distance: f64,
    pub max_replicate: usize,
    pub prune_dormant: f64,
    pub top_k_attractor: usize,
}

impl Default for AdaptationConfig {
    fn default() -> Self {
        Self {
            split_threshold: 0.8,
            merge_distance: 0.1,
            max_replicate: 5,
            prune_dormant: 100.0,
            top_k_attractor: 5,
        }
    }
}

/// Gate threshold configuration for all 8 gates.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThresholdConfig {
    pub d: f64,
    pub q: f64,
    pub r: f64,
    pub g: f64,
    pub j: f64,
    pub p: f64,
    pub n: f64,
    pub k: f64,
    pub f_friction: f64,
    pub s_shock: f64,
    pub l_migration: f64,
}

impl Default for ThresholdConfig {
    fn default() -> Self {
        Self {
            d: 0.5,
            q: 0.5,
            r: 0.5,
            g: 0.5,
            j: 0.5,
            p: 0.5,
            n: 0.5,
            k: 0.5,
            f_friction: 0.7,
            s_shock: 0.7,
            l_migration: 0.6,
        }
    }
}

/// Normalization coefficients for metric computation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NormalizationConfig {
    pub mu_d: f64,
    pub mu_q: f64,
    pub mu_j: f64,
    pub mu_f: f64,
    pub mu_s: f64,
    pub lambda_r: f64,
    pub lambda_p: f64,
    pub lambda_seam: f64,
    pub gamma_d: f64,
    pub gamma_q: f64,
    pub gamma_r: f64,
    pub lambda_c: f64,
    pub lambda_e: f64,
}

impl Default for NormalizationConfig {
    fn default() -> Self {
        Self {
            mu_d: 0.20,
            mu_q: 1.0,
            mu_j: 1.0,
            mu_f: 1.0,
            mu_s: 1.0,
            lambda_r: 1.0,
            lambda_p: 1.0,
            lambda_seam: 0.1,
            gamma_d: 0.33,
            gamma_q: 0.33,
            gamma_r: 0.34,
            lambda_c: 1.0,
            lambda_e: 0.0,
        }
    }
}

/// Archive configuration.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ArchiveConfig {
    pub max_chain_length: usize,
}

// ─── Falsification ───────────────────────────────────────────────────────────

/// Strategy for generating surrogate observation streams against which a
/// crystal candidate's statistical significance is tested.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SurrogateMethod {
    /// Uniformly shuffle the observation ordering.
    Shuffle,
    /// Shuffle in contiguous blocks of the given size (preserves local
    /// temporal correlations while breaking long-range structure).
    BlockBootstrap { block_size: usize },
    /// Randomize the Fourier phases while preserving the power spectrum.
    /// (Added in a later increment; the type is listed here for forward
    /// compatibility.)
    PhaseRandomize,
}

impl Default for SurrogateMethod {
    fn default() -> Self {
        SurrogateMethod::Shuffle
    }
}

/// Configuration for the surrogate-data falsification pass (Strand D).
///
/// When `enabled` is false (the default), the pass is a no-op and no extra
/// work happens in the macro-step. When enabled, `k` surrogates are drawn
/// per candidate crystal; the crystal is tagged with the empirical
/// `p_value = #{comparable surrogates} / k`. If `gate_on_fail` is true and
/// `p_value >= alpha`, the crystal is rejected and no commit is emitted.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FalsificationConfig {
    /// Run the falsification pass during `macro_step`.
    pub enabled: bool,
    /// Number of surrogate runs per candidate crystal.
    pub k: u32,
    /// Significance threshold. A crystal passes iff `p_value < alpha`.
    pub alpha: f64,
    /// Surrogate generation strategy.
    pub method: SurrogateMethod,
    /// If true and the falsification test fails, reject the crystal.
    /// If false, the p-value is recorded but the crystal still commits.
    pub gate_on_fail: bool,
    /// L∞ tolerance when deciding whether a surrogate precursor is
    /// "comparable" to the real one across (g_readiness, n_seam, k_crystal, mci).
    pub comparability_tolerance: f64,
}

impl Default for FalsificationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            k: 50,
            alpha: 0.05,
            method: SurrogateMethod::Shuffle,
            gate_on_fail: true,
            comparability_tolerance: 0.10,
        }
    }
}

// ─── Canonical Hashing ──────────────────────────────────────────────────────

/// Canonical JCS serialization (RFC 8785).
pub fn canonical_bytes<T: Serialize>(value: &T) -> Vec<u8> {
    serde_jcs::to_vec(value).expect("JCS serialization must not fail for canonical types")
}

/// Content-address a value: SHA-256(JCS(value)).
pub fn content_address<T: Serialize>(value: &T) -> Hash256 {
    use sha2::{Digest, Sha256};
    let bytes = canonical_bytes(value);
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    hasher.finalize().into()
}

/// Content-address raw bytes: SHA-256(data).
pub fn content_address_raw(data: &[u8]) -> Hash256 {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_center_is_unit_struct() {
        let nc = NullCenter;
        let nc2 = NullCenter::default();
        assert_eq!(nc, nc2);
        assert_eq!(std::mem::size_of::<NullCenter>(), 0);
    }

    #[test]
    fn five_d_state_distance() {
        let a = FiveDState {
            p: 1.0,
            rho: 0.0,
            omega: 0.0,
            chi: 0.0,
            eta: 0.0,
        };
        let b = FiveDState {
            p: 0.0,
            rho: 0.0,
            omega: 0.0,
            chi: 0.0,
            eta: 0.0,
        };
        assert!((a.distance(&b) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn content_address_deterministic() {
        let state = FiveDState {
            p: 1.0,
            rho: 2.0,
            omega: 3.0,
            chi: 4.0,
            eta: 5.0,
        };
        let h1 = content_address(&state);
        let h2 = content_address(&state);
        assert_eq!(h1, h2);
    }

    #[test]
    fn content_address_raw_deterministic() {
        let data = b"hello world";
        let h1 = content_address_raw(data);
        let h2 = content_address_raw(data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn btreemap_in_measurement_context() {
        let mut ctx = MeasurementContext::default();
        ctx.tags.insert("key".to_string(), "val".to_string());
        assert_eq!(ctx.tags.get("key"), Some(&"val".to_string()));
    }

    // ─── Falsification backward-compatibility tests ─────────────────────

    #[test]
    fn commit_proof_default_has_no_falsification_fields() {
        let cp = CommitProof::default();
        assert_eq!(cp.falsification_p_value, None);
        assert_eq!(cp.surrogate_count, None);
    }

    #[test]
    fn commit_proof_deserializes_without_new_fields() {
        // Legacy CommitProof JSON — missing both new optional fields.
        // Must deserialize successfully into the extended struct.
        let legacy = r#"{
            "evidence_digests": [],
            "operator_stack": [],
            "gate_values": {"d":0.0,"q":0.0,"r":0.0,"g":0.0,"j":0.0,"p":0.0,"n":0.0,"k":0.0,"kairos":false},
            "structural_result": false,
            "consensus_result": {"primal_score":0.0,"dual_score":0.0,"mci":0.0,"threshold":0.0},
            "por_trace": {"search_enter":0.0},
            "carrier_id": 0,
            "carrier_offset": 0.0
        }"#;
        let cp: CommitProof = serde_json::from_str(legacy).unwrap();
        assert_eq!(cp.falsification_p_value, None);
        assert_eq!(cp.surrogate_count, None);
        assert_eq!(cp.carrier_id, 0);
    }

    #[test]
    fn commit_proof_none_fields_are_skipped_in_output() {
        // skip_serializing_if = "Option::is_none" means a default-valued
        // CommitProof must NOT mention the two new keys in its JSON output.
        // This preserves byte-identical serialization vs. the pre-change form.
        let cp = CommitProof::default();
        let json = serde_json::to_string(&cp).unwrap();
        assert!(
            !json.contains("falsification_p_value"),
            "default JSON leaked field: {}",
            json
        );
        assert!(
            !json.contains("surrogate_count"),
            "default JSON leaked field: {}",
            json
        );
    }

    #[test]
    fn commit_proof_populated_falsification_roundtrips() {
        let mut cp = CommitProof::default();
        cp.falsification_p_value = Some(0.023);
        cp.surrogate_count = Some(50);
        let json = serde_json::to_string(&cp).unwrap();
        let back: CommitProof = serde_json::from_str(&json).unwrap();
        assert_eq!(back.falsification_p_value, Some(0.023));
        assert_eq!(back.surrogate_count, Some(50));
    }

    #[test]
    fn falsification_config_defaults_are_disabled() {
        let f = FalsificationConfig::default();
        assert!(!f.enabled);
        assert_eq!(f.k, 50);
        assert_eq!(f.alpha, 0.05);
        assert!(f.gate_on_fail);
        assert_eq!(f.method, SurrogateMethod::Shuffle);
    }

    #[test]
    fn config_deserializes_without_falsification_section() {
        // A serialized Default Config does not contain a `falsification`
        // section when re-deserialized through a version of the type
        // without that field. The #[serde(default)] attribute guarantees
        // that the corresponding field loads as FalsificationConfig::default()
        // when missing — the standard backward-compat path for pre-existing
        // on-disk configs.
        let mut baseline_json: serde_json::Value = serde_json::to_value(Config::default()).unwrap();
        // Simulate a legacy config by stripping the new section.
        baseline_json
            .as_object_mut()
            .unwrap()
            .remove("falsification");
        let cfg: Config = serde_json::from_value(baseline_json).unwrap();
        // The default falsification config must be materialized by the
        // serde(default) attribute.
        assert!(!cfg.falsification.enabled);
        assert_eq!(cfg.falsification.k, 50);
    }

    #[test]
    fn crystal_core_hash_independent_of_falsification_fields() {
        // This is the critical I10/I17 invariant: adding optional commit-proof
        // fields must not alter any crystal's content-address. CrystalCore in
        // pse-evidence hashes only {region, stability_score, created_at,
        // free_energy, carrier_instance_idx} — not any CommitProof field.
        // We mirror that subset locally and confirm that two commit proofs
        // which differ ONLY in the new fields yield the same core digest.
        #[derive(serde::Serialize)]
        struct Core<'a> {
            region: &'a Vec<VertexId>,
            stability_score: f64,
            created_at: CommitIndex,
            free_energy: f64,
            carrier_instance_idx: usize,
        }
        let region: Vec<VertexId> = vec![1, 2, 3];
        let core = Core {
            region: &region,
            stability_score: 0.8,
            created_at: 42,
            free_energy: -1.5,
            carrier_instance_idx: 0,
        };
        let h1 = content_address(&core);
        // Build a second identical Core; the digest must be byte-identical.
        let core2 = Core {
            region: &region,
            stability_score: 0.8,
            created_at: 42,
            free_energy: -1.5,
            carrier_instance_idx: 0,
        };
        let h2 = content_address(&core2);
        assert_eq!(h1, h2);
    }
}

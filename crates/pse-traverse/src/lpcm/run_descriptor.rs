//! Run descriptor, threshold, and policy types for the LPCM layer.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::primitives::{fixed_from_f64, EvidenceRef, Fixed, Hash256, StableId};

/// Canonical run descriptor — recorded in every HierarchicalCollapseReport.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LpcmRunDescriptor {
    /// Stable run identifier.
    pub run_id: StableId,
    /// Implementation version tag. MUST be "PSE-LPCM-IMPLEMENTATION-01".
    pub lpcm_version: String,
    /// SHA-256 hash of the source window (binds this run to a specific input).
    pub source_window_hash: Hash256,
    /// SHA-256 hash of the upstream topology output, if available.
    pub source_topology_hash: Option<Hash256>,
    /// Deterministic seed (recorded, not used for randomness).
    pub seed: u64,
    /// Gate thresholds.
    pub thresholds: LpcmThresholds,
    /// Pipeline policies.
    pub policies: LpcmPolicies,
    /// Operator version map for reproducibility.
    pub operator_versions: BTreeMap<String, String>,
    /// External evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl LpcmRunDescriptor {
    pub fn lpcm_version_tag() -> &'static str {
        "PSE-LPCM-IMPLEMENTATION-01"
    }
}

/// Hysteretic gate thresholds for the 51% local majority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LpcmThresholds {
    /// Activation threshold: Z_U >= theta_activate → condense. Default 0.51.
    pub theta_activate: Fixed,
    /// Deactivation threshold: Z_U < theta_deactivate → dormant. Default 0.50.
    pub theta_deactivate: Fixed,
    /// Optional minimum dominance margin Δ_min. Zero means no margin required.
    pub min_dominance_margin: Fixed,
    /// Minimum seam score for an edge to be accepted.
    pub seam_min: Fixed,
    /// Minimum path strength for a percolative path to be accepted.
    pub percolation_min_path_strength: Fixed,
    /// Minimum aggregate support for a coarse-grain condensate to be accepted.
    pub coarse_grain_min_support: Fixed,
    /// Maximum acceptable false local collapse rate (diagnostic bound).
    pub max_false_local_collapse_rate: Fixed,
}

impl Default for LpcmThresholds {
    fn default() -> Self {
        Self {
            theta_activate: fixed_from_f64(0.51),
            theta_deactivate: fixed_from_f64(0.50),
            min_dominance_margin: Fixed::zero(),
            seam_min: fixed_from_f64(0.10),
            percolation_min_path_strength: fixed_from_f64(0.10),
            coarse_grain_min_support: fixed_from_f64(0.10),
            max_false_local_collapse_rate: fixed_from_f64(0.10),
        }
    }
}

/// Pipeline behavior policies.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LpcmPolicies {
    pub partition_policy: PartitionPolicy,
    pub topology_policy: TopologyPolicy,
    pub candidate_policy: CandidatePolicy,
    pub support_policy: SupportPolicy,
    pub tie_policy: TiePolicy,
    pub percolation_policy: PercolationPolicy,
    pub coarse_grain_policy: CoarseGrainPolicy,
    /// If true, fragments without provenance are rejected.
    pub fail_on_missing_provenance: bool,
    /// If true, replay identity is required for a valid report.
    pub require_replay_identity: bool,
}

impl Default for LpcmPolicies {
    fn default() -> Self {
        Self {
            partition_policy: PartitionPolicy::FixedGrid { cells_per_axis: 4 },
            topology_policy: TopologyPolicy::default(),
            candidate_policy: CandidatePolicy::BoundaryCompletion,
            support_policy: SupportPolicy::default(),
            tie_policy: TiePolicy::TieNoCondense,
            percolation_policy: PercolationPolicy::BreadthFirst,
            coarse_grain_policy: CoarseGrainPolicy::MajorityMonotone,
            fail_on_missing_provenance: false,
            require_replay_identity: true,
        }
    }
}

/// How to partition the input window into fragments.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartitionPolicy {
    /// Divide each axis into N cells. Deterministic grid.
    FixedGrid { cells_per_axis: u32 },
    /// Graph k-hop neighborhood partitions.
    GraphKHop { k: u32 },
    /// Sliding carrier window with stride.
    CarrierWindow { width: u32, stride: u32 },
    /// Topology-adaptive partition bounded by vertex count.
    TopologyAdaptive { max_vertices: u32, max_overlap: u32 },
}

/// Topology patch construction policy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyPolicy {
    /// Maximum number of vertices per patch.
    pub max_vertices_per_patch: u32,
    /// Whether to require triangulable patches.
    pub require_triangulable: bool,
}

impl Default for TopologyPolicy {
    fn default() -> Self {
        Self {
            max_vertices_per_patch: 64,
            require_triangulable: false,
        }
    }
}

/// Candidate direction generation policy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CandidatePolicy {
    BoundaryCompletion,
    MeshRefinement,
    SeamBridge,
    ContradictionRemoval,
    HybridDeclared(Vec<CandidateDirectionKindTag>),
}

/// Tag for candidate direction kind (used in policy, not in the direction itself).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CandidateDirectionKindTag {
    CompleteGap,
    ContinuePath,
    ContractRegion,
    RefineMesh,
    BridgeBoundary,
    RemoveContradiction,
    HoldDiagnostic,
}

/// Support mass computation policy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupportPolicy {
    /// Seam weight α_e in seam scoring.
    pub alpha_carrier: Fixed,
    /// Boundary weight α_b in seam scoring.
    pub alpha_boundary: Fixed,
    /// Topology weight α_t in seam scoring.
    pub alpha_topology: Fixed,
    /// Replay weight α_r in seam scoring.
    pub alpha_replay: Fixed,
    /// Provenance weight α_p in seam scoring.
    pub alpha_provenance: Fixed,
    /// Maximum candidates per patch.
    pub max_candidates_per_patch: u32,
}

impl Default for SupportPolicy {
    fn default() -> Self {
        // Weights sum to 1.0 across the 5 seam scoring factors.
        Self {
            alpha_carrier: fixed_from_f64(0.20),
            alpha_boundary: fixed_from_f64(0.20),
            alpha_topology: fixed_from_f64(0.20),
            alpha_replay: fixed_from_f64(0.20),
            alpha_provenance: fixed_from_f64(0.20),
            max_candidates_per_patch: 16,
        }
    }
}

/// Tie-breaking policy for the local majority gate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TiePolicy {
    /// Default: ties never condense.
    TieNoCondense,
    /// Use lexicographic ordering of candidate_id as tiebreaker.
    LexicographicId,
}

/// Percolation search policy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PercolationPolicy {
    BreadthFirst,
    DepthFirst,
    ScoreWeighted,
}

/// Coarse-graining aggregation policy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoarseGrainPolicy {
    WeightedSupportMean,
    MinEdgeBottleneck,
    /// Default: increasing support at lower scale must not reduce aggregate support.
    MajorityMonotone,
}

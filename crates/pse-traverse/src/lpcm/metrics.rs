//! LPCM pipeline metrics (§16).

use serde::{Deserialize, Serialize};

use super::primitives::Fixed;

/// Diagnostic metrics emitted in every HierarchicalCollapseReport.
///
/// MUST be registered with the eval matrix under the "lpcm-fragment-collapse" preset.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LpcmMetrics {
    pub fragment_count: u64,
    /// Fraction of patches that are triangulable.
    pub triangulable_patch_rate: Fixed,
    /// Mean number of candidate directions per accepted patch.
    pub candidate_direction_count_mean: Fixed,
    /// Fraction of fragments that produced an active condensation bit.
    pub local_condensation_rate: Fixed,
    /// Mean dominance margin across all active condensation bits.
    pub mean_dominance_margin: Fixed,
    /// Fraction of seam edges that were accepted.
    pub seam_consistency_rate: Fixed,
    pub percolation_path_count: u64,
    pub strongest_path_score: Fixed,
    pub coarse_grain_stability: Fixed,
    /// Estimated false local collapse rate (diagnostic bound).
    pub false_local_collapse_rate: Fixed,
    /// Whether the last replay verification succeeded.
    pub replay_identity: bool,
}

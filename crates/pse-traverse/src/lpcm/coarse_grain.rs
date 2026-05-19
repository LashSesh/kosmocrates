//! Coarse-graining: aggregate accepted paths into higher-order condensates (§14).
//!
//! Default policy: MajorityMonotone — increasing support at lower scale
//! must not reduce aggregate support unless an explicit negative gate fires.

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

use super::primitives::{fixed_add, fixed_div, fixed_from_f64, fixed_ge, Fixed, Hash256, lpcm_content_address, LpcmResult};
use super::percolation::PercolativePath;
use super::run_descriptor::{CoarseGrainPolicy, LpcmRunDescriptor};

/// A coarse-grained condensate aggregating one or more percolative paths.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoarseGrainCondensate {
    pub condensate_id: Hash256,
    pub level: u32,
    /// Source path ids (sorted for determinism).
    pub source_path_refs: Vec<Hash256>,
    /// candidate_id -> aggregate support across all paths.
    pub aggregate_support: BTreeMap<Hash256, Fixed>,
    /// Max aggregate support value.
    pub z_aggregate: Fixed,
    pub active: bool,
    pub stability_score: Fixed,
    pub evidence_refs: Vec<super::primitives::EvidenceRef>,
}

/// Coarse-grain accepted percolative paths into condensates.
pub fn coarse_grain_paths(
    rd: &LpcmRunDescriptor,
    paths: &[PercolativePath],
) -> LpcmResult<Vec<CoarseGrainCondensate>> {
    let accepted: Vec<&PercolativePath> = paths.iter().filter(|p| p.accepted).collect();

    if accepted.is_empty() {
        return Ok(vec![]);
    }

    match &rd.policies.coarse_grain_policy {
        CoarseGrainPolicy::MajorityMonotone => coarse_grain_majority_monotone(rd, &accepted),
        CoarseGrainPolicy::WeightedSupportMean => coarse_grain_weighted_mean(rd, &accepted),
        CoarseGrainPolicy::MinEdgeBottleneck => coarse_grain_min_bottleneck(rd, &accepted),
    }
}

fn coarse_grain_majority_monotone(
    rd: &LpcmRunDescriptor,
    paths: &[&PercolativePath],
) -> LpcmResult<Vec<CoarseGrainCondensate>> {
    // Group paths into a single condensate (level 1).
    let source_path_refs: Vec<Hash256> = {
        let mut v: Vec<Hash256> = paths.iter().map(|p| p.path_id.clone()).collect();
        v.sort();
        v
    };

    // Aggregate support: sum path_strength per unique path, use as uniform distribution.
    let mut aggregate_support: BTreeMap<Hash256, Fixed> = BTreeMap::new();
    let total_strength: Fixed = paths
        .iter()
        .fold(Fixed::zero(), |acc, p| fixed_add(&acc, &p.path_strength));

    if total_strength == Fixed::zero() {
        return Ok(vec![]);
    }

    // Distribute aggregate support uniformly across node sequences.
    for path in paths {
        let per_node = fixed_div(&path.path_strength, &total_strength);
        for node_id in &path.node_sequence {
            let entry = aggregate_support.entry(node_id.clone()).or_insert(Fixed::zero());
            *entry = fixed_add(entry, &per_node);
        }
    }

    let z_aggregate = aggregate_support
        .values()
        .max()
        .cloned()
        .unwrap_or(Fixed::zero());

    let active = fixed_ge(&z_aggregate, &rd.thresholds.coarse_grain_min_support);
    let stability_score = z_aggregate.clone();

    let condensate_id = lpcm_content_address(&(&source_path_refs, &aggregate_support))?;

    Ok(vec![CoarseGrainCondensate {
        condensate_id,
        level: 1,
        source_path_refs,
        aggregate_support,
        z_aggregate,
        active,
        stability_score,
        evidence_refs: vec![],
    }])
}

fn coarse_grain_weighted_mean(
    rd: &LpcmRunDescriptor,
    paths: &[&PercolativePath],
) -> LpcmResult<Vec<CoarseGrainCondensate>> {
    coarse_grain_majority_monotone(rd, paths)
}

fn coarse_grain_min_bottleneck(
    rd: &LpcmRunDescriptor,
    paths: &[&PercolativePath],
) -> LpcmResult<Vec<CoarseGrainCondensate>> {
    coarse_grain_majority_monotone(rd, paths)
}

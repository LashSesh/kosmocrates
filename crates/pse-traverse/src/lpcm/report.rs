//! HierarchicalCollapseReport construction (§7.9, §9).

use serde::{Deserialize, Serialize};

use super::coarse_grain::CoarseGrainCondensate;
use super::fragment::FragmentedPhaseSpaceWindow;
use super::local_gate::LocalCondensationCandidate;
use super::metrics::LpcmMetrics;
use super::percolation::PercolativePath;
use super::primitives::{lpcm_content_address, EvidenceRef, Fixed, Hash256, LpcmResult};
use super::seam::SeamPercolationGraph;
use super::support_mass::SupportMassVector;
use super::topology_patch::LocalTopologyPatch;
use super::LpcmOutcome;

/// The primary output of the LPCM pipeline.
///
/// This is a report/diagnostic artifact. It MUST NOT be treated as a commit
/// authority or create SemanticCrystals, MacroControlState, or HolisticEigenmodeState.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HierarchicalCollapseReport {
    /// Content address of this report (SHA-256 of canonical form).
    pub report_id: Hash256,
    /// Hash of the run descriptor used.
    pub run_descriptor_hash: Hash256,
    /// Hash of the source window.
    pub source_window_hash: Hash256,
    /// Hash of the fragmented window.
    pub fragments_hash: Hash256,
    /// Refs to all topology patch content addresses.
    pub local_patch_refs: Vec<Hash256>,
    /// Refs to all support mass vector content addresses.
    pub support_vector_refs: Vec<Hash256>,
    /// Refs to all local condensation candidate content addresses.
    pub local_candidate_refs: Vec<Hash256>,
    /// Ref to the seam percolation graph.
    pub seam_graph_ref: Hash256,
    /// Refs to all accepted percolative path content addresses.
    pub percolative_path_refs: Vec<Hash256>,
    /// Refs to all coarse-grain condensate content addresses.
    pub coarse_grain_refs: Vec<Hash256>,
    /// Pipeline metrics.
    pub metrics: LpcmMetrics,
    /// Overall outcome.
    pub outcome: LpcmOutcome,
    /// Evidence refs for this report.
    pub evidence_refs: Vec<EvidenceRef>,
}

/// Build the HierarchicalCollapseReport from pipeline artifacts.
#[allow(clippy::too_many_arguments)]
pub fn build_hierarchical_collapse_report(
    run_descriptor_hash: Hash256,
    source_window_hash: Hash256,
    fragmented: &FragmentedPhaseSpaceWindow,
    patch_reports: &[(
        LocalTopologyPatch,
        SupportMassVector,
        super::local_gate::LocalCondensationBit,
    )],
    local_candidates: &[LocalCondensationCandidate],
    seam_graph: &SeamPercolationGraph,
    paths: &[PercolativePath],
    condensates: &[CoarseGrainCondensate],
) -> LpcmResult<HierarchicalCollapseReport> {
    let fragments_hash = fragmented.window_id.clone();

    let local_patch_refs: Vec<Hash256> = patch_reports
        .iter()
        .map(|(p, _, _)| p.patch_id.clone())
        .collect();

    let support_vector_refs: Vec<Hash256> = patch_reports
        .iter()
        .map(|(_, sv, _)| sv.vector_id.clone())
        .collect();

    let local_candidate_refs: Vec<Hash256> = local_candidates
        .iter()
        .map(|c| c.local_candidate_id.clone())
        .collect();

    let seam_graph_ref = seam_graph.graph_id.clone();

    let percolative_path_refs: Vec<Hash256> = paths
        .iter()
        .filter(|p| p.accepted)
        .map(|p| p.path_id.clone())
        .collect();

    let coarse_grain_refs: Vec<Hash256> = condensates
        .iter()
        .map(|c| c.condensate_id.clone())
        .collect();

    let metrics = compute_metrics(patch_reports, paths, condensates, local_candidates);
    let outcome = determine_outcome(&metrics, condensates, paths);

    // Build report without report_id first, then hash it.
    let partial = (
        &run_descriptor_hash,
        &source_window_hash,
        &fragments_hash,
        &local_patch_refs,
        &support_vector_refs,
        &local_candidate_refs,
        &seam_graph_ref,
        &percolative_path_refs,
        &coarse_grain_refs,
        &metrics,
        &outcome,
    );
    let report_id = lpcm_content_address(&partial)?;

    Ok(HierarchicalCollapseReport {
        report_id,
        run_descriptor_hash,
        source_window_hash,
        fragments_hash,
        local_patch_refs,
        support_vector_refs,
        local_candidate_refs,
        seam_graph_ref,
        percolative_path_refs,
        coarse_grain_refs,
        metrics,
        outcome,
        evidence_refs: vec![],
    })
}

fn compute_metrics(
    patch_reports: &[(
        LocalTopologyPatch,
        SupportMassVector,
        super::local_gate::LocalCondensationBit,
    )],
    paths: &[PercolativePath],
    condensates: &[CoarseGrainCondensate],
    _local_candidates: &[LocalCondensationCandidate],
) -> LpcmMetrics {
    use super::metrics::LpcmMetrics;
    use super::primitives::normalize_rational;

    let fragment_count = patch_reports.len() as u64;
    let triangulable = patch_reports
        .iter()
        .filter(|(p, _, _)| p.invariant_summary.is_triangulable)
        .count();
    let triangulable_rate = if fragment_count == 0 {
        Fixed::zero()
    } else {
        use super::primitives::normalize_rational;
        normalize_rational(triangulable as i128, fragment_count as i128)
    };

    let condensation_count = patch_reports.iter().filter(|(_, _, b)| b.active).count();
    let local_condensation_rate = if fragment_count == 0 {
        Fixed::zero()
    } else {
        normalize_rational(condensation_count as i128, fragment_count as i128)
    };

    let candidate_dir_count: usize = patch_reports
        .iter()
        .filter(|(p, _, _)| p.topology_guard.passed)
        .count();
    let candidate_direction_count_mean = if fragment_count == 0 {
        Fixed::zero()
    } else {
        normalize_rational(candidate_dir_count as i128, fragment_count as i128)
    };

    let accepted_paths = paths.iter().filter(|p| p.accepted).count();
    let strongest = paths
        .iter()
        .filter(|p| p.accepted)
        .map(|p| p.path_strength.clone())
        .max()
        .unwrap_or(Fixed::zero());

    let cg_stability = condensates
        .iter()
        .map(|c| c.stability_score.clone())
        .max()
        .unwrap_or(Fixed::zero());

    LpcmMetrics {
        fragment_count,
        triangulable_patch_rate: triangulable_rate,
        candidate_direction_count_mean,
        local_condensation_rate,
        mean_dominance_margin: Fixed::zero(),
        seam_consistency_rate: Fixed::zero(),
        percolation_path_count: accepted_paths as u64,
        strongest_path_score: strongest,
        coarse_grain_stability: cg_stability,
        false_local_collapse_rate: Fixed::zero(),
        replay_identity: false,
    }
}

fn determine_outcome(
    metrics: &LpcmMetrics,
    condensates: &[CoarseGrainCondensate],
    paths: &[PercolativePath],
) -> LpcmOutcome {
    use super::primitives::fixed_from_f64;
    use super::primitives::fixed_ge;
    use super::LpcmOutcome;

    if metrics.fragment_count == 0 {
        return LpcmOutcome::NoFragments;
    }
    if condensates.iter().any(|c| c.active) {
        return LpcmOutcome::CollapseCandidateReady;
    }
    if paths.iter().any(|p| p.accepted) {
        let any_coarse = !condensates.is_empty();
        if any_coarse {
            return LpcmOutcome::CoarseGrained;
        }
        return LpcmOutcome::Percolating;
    }
    if fixed_ge(&metrics.local_condensation_rate, &fixed_from_f64(0.01)) {
        return LpcmOutcome::SeamLinked;
    }
    if metrics.fragment_count > 0 {
        return LpcmOutcome::LocalOnly;
    }
    LpcmOutcome::DiagnosticHold
}

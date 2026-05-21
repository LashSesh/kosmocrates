//! LPCM pipeline entry point (§9 reference pseudocode).
//!
//! run_lpcm is the canonical entry point for the Fragmented 51% Condensation Layer.
//! It is additive, ablatable, fail-closed, and non-committing.

use super::adapters::{canonicalize_input, sorted_fragments};
use super::candidate::CandidateDirection;
use super::coarse_grain::coarse_grain_paths;
use super::evidence::LpcmInputWindow;
use super::fragment::FragmentRef;
use super::local_gate::{
    build_local_candidate, evaluate_local_condensation, LocalCondensationBit,
    LocalCondensationCandidate,
};
use super::partition::{partition_window, validate_descriptor};
use super::percolation::find_percolative_paths;
use super::primitives::{lpcm_content_address, Fixed, Hash256, LpcmResult};
use super::report::{build_hierarchical_collapse_report, HierarchicalCollapseReport};
use super::run_descriptor::LpcmRunDescriptor;
use super::seam::build_seam_percolation_graph;
use super::support_mass::{compute_normalized_support, SupportFactorVector, SupportMassVector};
use super::topology_patch::{build_local_topology_patch, LocalTopologyPatch};

/// Run the LPCM pipeline end-to-end and emit a HierarchicalCollapseReport.
///
/// INVARIANT: Local51(U) ⟹ Candidate(U), Candidate(U) ≠ Truth.
/// This function MUST NOT create SemanticCrystals, MacroControlState, or HolisticEigenmodeState.
pub fn run_lpcm(
    rd: LpcmRunDescriptor,
    input: LpcmInputWindow,
) -> LpcmResult<HierarchicalCollapseReport> {
    validate_descriptor(&rd)?;
    let canonical_input = canonicalize_input(input)?;

    let fragmented = partition_window(&rd, &canonical_input)?;
    let mut patch_reports: Vec<(LocalTopologyPatch, SupportMassVector, LocalCondensationBit)> =
        Vec::new();
    let mut local_candidates: Vec<LocalCondensationCandidate> = Vec::new();

    for fragment in sorted_fragments(&fragmented) {
        let patch = build_local_topology_patch(&rd, fragment);
        if !patch.topology_guard.passed {
            // Topology guard failed: record diagnostic hold and skip.
            let dummy_sv = SupportMassVector {
                vector_id: Hash256::zero(),
                fragment_id: fragment.fragment_id.clone(),
                weights: std::collections::BTreeMap::new(),
                raw_factors: vec![],
                denominator: Fixed::zero(),
                valid: false,
                failure: Some(super::LpcmFailureKind::TopologyGuardFailed),
            };
            let bit = LocalCondensationBit {
                bit_id: Hash256::zero(),
                fragment_id: fragment.fragment_id.clone(),
                winning_candidate_id: None,
                z_max: Fixed::zero(),
                runner_up: Fixed::zero(),
                dominance_margin: Fixed::zero(),
                active: false,
                status: super::local_gate::LocalCondensationStatus::TopologyRejected,
                failure: Some(super::LpcmFailureKind::TopologyGuardFailed),
            };
            patch_reports.push((patch, dummy_sv, bit));
            continue;
        }

        let candidates = generate_candidate_directions(&rd, &patch, fragment);
        let raw_factors = compute_raw_factors(&rd, &patch, &candidates);
        let support = compute_normalized_support(fragment.fragment_id.clone(), raw_factors)?;
        let bit = evaluate_local_condensation(
            &fragment.fragment_id,
            &support,
            &rd.thresholds,
            &rd.policies.tie_policy,
            false,
        );

        if bit.active {
            if let Ok(candidate) = build_local_candidate(&rd, &bit, &candidates) {
                local_candidates.push(candidate);
            }
        }

        patch_reports.push((patch, support, bit));
    }

    let seam_graph = build_seam_percolation_graph(&rd, &local_candidates)?;
    let paths = find_percolative_paths(&rd, &seam_graph)?;
    let condensates = coarse_grain_paths(&rd, &paths)?;

    let rd_hash = lpcm_content_address(&rd)?;
    let report = build_hierarchical_collapse_report(
        rd_hash,
        canonical_input.window_hash.clone(),
        &fragmented,
        &patch_reports,
        &local_candidates,
        &seam_graph,
        &paths,
        &condensates,
    )?;

    Ok(report)
}

/// Generate candidate directions for a patch using the configured policy.
fn generate_candidate_directions(
    rd: &LpcmRunDescriptor,
    patch: &LocalTopologyPatch,
    fragment: &FragmentRef,
) -> Vec<CandidateDirection> {
    use super::candidate::CandidateDirectionKind;

    // Generate one candidate per edge + one BoundaryCompletion candidate.
    let mut candidates = Vec::new();
    let max = rd.policies.support_policy.max_candidates_per_patch as usize;

    for (i, edge) in patch.edges.iter().take(max.saturating_sub(1)).enumerate() {
        let sig_hash =
            lpcm_content_address(&(&edge.edge_id, "candidate", i)).unwrap_or(Hash256::zero());
        let candidate_id =
            lpcm_content_address(&(&fragment.fragment_id, &sig_hash)).unwrap_or(Hash256::zero());
        candidates.push(CandidateDirection {
            candidate_id,
            fragment_id: fragment.fragment_id.clone(),
            direction_kind: CandidateDirectionKind::ContinuePath,
            direction_signature_hash: sig_hash,
            proposed_boundary_delta_hash: edge.edge_id.clone(),
            proposed_seam_delta_hash: Hash256::zero(),
            evidence_refs: fragment.evidence_refs.clone(),
        });
    }

    // Always add a boundary completion candidate if no edges.
    if candidates.is_empty() || candidates.len() < max {
        let sig_hash = lpcm_content_address(&(&fragment.fragment_id, "boundary-completion"))
            .unwrap_or(Hash256::zero());
        let candidate_id = lpcm_content_address(&(&fragment.fragment_id, &sig_hash, "bc"))
            .unwrap_or(Hash256::zero());
        candidates.push(CandidateDirection {
            candidate_id,
            fragment_id: fragment.fragment_id.clone(),
            direction_kind: CandidateDirectionKind::CompleteGap,
            direction_signature_hash: sig_hash,
            proposed_boundary_delta_hash: fragment.boundary_hash.clone(),
            proposed_seam_delta_hash: Hash256::zero(),
            evidence_refs: fragment.evidence_refs.clone(),
        });
    }

    candidates
}

/// Compute raw support factor vectors for each candidate.
fn compute_raw_factors(
    rd: &LpcmRunDescriptor,
    patch: &LocalTopologyPatch,
    candidates: &[CandidateDirection],
) -> Vec<SupportFactorVector> {
    use super::primitives::fixed_from_f64;

    candidates
        .iter()
        .enumerate()
        .map(|(i, c)| {
            // Simple heuristic factors — deterministic, based only on patch structure.
            let n_verts = patch.vertices.len().max(1);
            let adj = fixed_from_f64((n_verts.min(8) as f64) / 8.0);
            let topo = if patch.topology_guard.passed {
                fixed_from_f64(0.8)
            } else {
                Fixed::zero()
            };
            let seam = fixed_from_f64(0.6);
            let resonance = fixed_from_f64(0.7);
            let provenance = if !c.evidence_refs.is_empty() {
                fixed_from_f64(1.0)
            } else if rd.policies.fail_on_missing_provenance {
                Fixed::zero()
            } else {
                fixed_from_f64(0.5)
            };
            let gate = fixed_from_f64(0.8);
            // Hysteresis: slight boost for later candidates (deterministic variation).
            let hysteresis = fixed_from_f64(0.5 + (i as f64 * 0.01).min(0.2));

            SupportFactorVector {
                candidate_id: c.candidate_id.clone(),
                adjacency: adj,
                topology: topo,
                seam,
                resonance,
                provenance,
                gate,
                hysteresis,
            }
        })
        .collect()
}

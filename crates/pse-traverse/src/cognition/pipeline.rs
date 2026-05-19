//! Reference cognition pipeline `run_cognition` (§16).
//!
//! Total: every fail-path lands in `CognitionOutcome::Hold` or returns
//! a `CognitionError`. No `SemanticCrystal`, no `FinalizedEmission`.

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::attractor_map::{AttractorEntry, AttractorMap};
use super::canonical_state::CanonicalCognitionState;
use super::cognitive_state_5d::CognitiveState5D;
use super::constraint_lattice::{
    build_lattice_minimal, evaluate_perkolation, ConstraintLatticeCognition,
};
use super::handoff::{
    CognitionCandidate, CognitionCandidateBundle, CognitionHandoffGate, ProjectionHandoffPolicy,
};
use super::hypercube_puzzle::{build_puzzle_minimal, HypercubePuzzleState};
use super::phase_panorama::{build_panorama_minimal, PhasePanorama};
use super::primitives::{fixed_ge, fixed_le, CognitionError, EvidenceRef, Fixed};
use super::replay::replay_hash_of;
use super::report::{
    CognitionDiagnostic, CognitionHoldReport, CognitionOutcome, CognitionRecoveryAction,
    CognitionReport,
};
use super::run_descriptor::{CognitionFailurePolicy, CognitionGateKind, CognitionRunDescriptor};
use super::scorpio_phase_scheduler::{build_scheduler_minimal, ScorpioPhaseScheduler};
use super::self_model_tensor::{build_self_model, SelfModelTensor};
use super::singularity_trigger::evaluate_singularity_trigger;
use super::spiral_memory::{spiral_memory_query, SpiralMemoryHitSet};

/// Caller-provided cognition input.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitionInput {
    pub null_center_id: Hash256,
    pub cognitive_components: CognitiveComponents,
    pub source_traversal_report_hash: Option<Hash256>,
    pub source_projection_report_hash: Option<Hash256>,
    pub spiral_memory_candidates: Vec<CognitiveState5D>,
    pub constraint_count: u32,
    pub support_strength: Fixed,
    pub logical_step: u64,
    pub carrier_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitiveComponents {
    pub psi: Fixed,
    pub rho: Fixed,
    pub omega: Fixed,
    pub chi: Fixed,
    pub tau: Fixed,
}

impl CognitionInput {
    pub fn minimal(null_center_id: Hash256) -> Self {
        let zero = Fixed::quantize(0.0, 9).unwrap();
        CognitionInput {
            null_center_id,
            cognitive_components: CognitiveComponents {
                psi: Fixed::quantize(0.5, 9).unwrap(),
                rho: Fixed::quantize(0.7, 9).unwrap(),
                omega: Fixed::quantize(0.5, 9).unwrap(),
                chi: zero.clone(),
                tau: Fixed::quantize(0.2, 9).unwrap(),
            },
            source_traversal_report_hash: None,
            source_projection_report_hash: None,
            spiral_memory_candidates: vec![CognitiveState5D::from_components(
                Fixed::quantize(0.5, 9).unwrap(),
                Fixed::quantize(0.7, 9).unwrap(),
                Fixed::quantize(0.5, 9).unwrap(),
                zero,
                Fixed::quantize(0.1, 9).unwrap(),
            )
            .unwrap()],
            constraint_count: 4,
            support_strength: Fixed::quantize(0.5, 9).unwrap(),
            logical_step: 1,
            carrier_ids: vec!["c.0".into(), "c.1".into()],
        }
    }
}

/// One-shot result of `run_cognition`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitionRunResult {
    pub input: CognitionInput,
    pub outcome: CognitionOutcome,
    pub canonical_state: CanonicalCognitionState,
    pub state5: CognitiveState5D,
    pub hitset: SpiralMemoryHitSet,
    pub lattice: ConstraintLatticeCognition,
    pub puzzle: HypercubePuzzleState,
    pub panorama: PhasePanorama,
    pub scheduler: ScorpioPhaseScheduler,
    pub self_model: SelfModelTensor,
    pub attractor_map: AttractorMap,
    pub handoff_gate: CognitionHandoffGate,
    pub bundle: Option<CognitionCandidateBundle>,
    pub report: CognitionReport,
    pub replay_hash: Hash256,
}

pub fn run_cognition(
    input: &CognitionInput,
    rd: &CognitionRunDescriptor,
) -> Result<CognitionRunResult, CognitionError> {
    rd.validate()?;

    // §16.1 step 2 — RD hash + null center.
    let rd_hash = Hash256(
        content_address(rd).map_err(|e| CognitionError::Canonicalization {
            reason: e.to_string(),
        })?,
    );
    let null_center_id = input.null_center_id.clone();

    // §16.1 step 3 — cognitive 5D state.
    let c = &input.cognitive_components;
    let state5 = CognitiveState5D::from_components(
        c.psi.clone(),
        c.rho.clone(),
        c.omega.clone(),
        c.chi.clone(),
        c.tau.clone(),
    )?;

    // §16.1 step 5 — spiral memory query + resonance gate.
    let epsilon_r = Fixed::quantize(2.0, 9).unwrap();
    let hitset = spiral_memory_query(&state5, &input.spiral_memory_candidates, &epsilon_r)?;
    let g_resonance = resonance_gate(&hitset, &rd.thresholds.min_resonance);

    // §16.1 step 6 — constraint lattice + hypercube puzzle.
    let lattice = build_lattice_minimal(rd, input.constraint_count, &input.support_strength)?;
    let puzzle = build_puzzle_minimal(&lattice, rd, input.logical_step)?;

    // §16.1 step 7 — perkolation gate.
    let g_perc = evaluate_perkolation(&lattice, rd);

    // §16.1 step 8 — scheduler.
    let scheduler = build_scheduler_minimal(&input.carrier_ids, input.logical_step)?;

    // §16.1 step 9 — panorama.
    let panorama = build_panorama_minimal(&null_center_id, input.logical_step)?;
    let g_panorama = fixed_ge(
        &panorama.coverage_score,
        &rd.thresholds.min_panorama_coverage,
    );

    // §16.1 step 11 — self model tensor.
    let state_hash = state5.content_hash()?;
    let self_model = build_self_model(
        state_hash.clone(),
        state_hash.clone(),
        state_hash.clone(),
        state5.psi_semantic_phase.clone(),
        state5.entropy.clone(),
        state5.stability_index.clone(),
    )?;
    let g_self = self_model.passes_gate(
        &rd.thresholds.min_self_model_coherence,
        &rd.thresholds.max_drift,
    );

    // §16.1 step 13 — attractor map.
    let attractor_map = AttractorMap {
        map_id: Hash256::zero(),
        entries: vec![AttractorEntry {
            attractor_id: state_hash.clone(),
            state_hash: state_hash.clone(),
            depth: state5.potential.clone(),
            stability: state5.stability_index.clone(),
            score: state5.stability_index.clone(),
        }],
        best: None,
    }
    .with_id()?;

    // §16.1 step 14 — singularity trigger.
    let (_, trigger) = evaluate_singularity_trigger(
        &state5,
        &attractor_map,
        &Fixed::quantize(0.0, 9).unwrap(),
        &Fixed::quantize(0.0, 9).unwrap(),
        &rd.thresholds.min_trigger_score,
        &rd.thresholds.min_trigger_score,
    )?;
    let g_trigger =
        trigger.passed || fixed_ge(&trigger.trigger_score, &rd.thresholds.min_trigger_score);

    // §16.1 step 15 — handoff gate.
    let handoff_gate =
        CognitionHandoffGate::evaluate(g_perc, g_resonance, g_panorama, g_self, g_trigger, true)?;

    // Canonical cognition state.
    let canonical_state = CanonicalCognitionState {
        state_id: Hash256::zero(),
        run_descriptor_hash: rd_hash.clone(),
        null_center_id: null_center_id.clone(),
        source_traversal_report_hash: input.source_traversal_report_hash.clone(),
        source_projection_report_hash: input.source_projection_report_hash.clone(),
        cognitive_vector: state5.clone(),
        constraint_lattice_id: lattice.lattice_id.clone(),
        hypercube_puzzle_id: puzzle.cube_id.clone(),
        phase_panorama_id: panorama.panorama_id.clone(),
        spiral_memory_context_id: hitset.hitset_id.clone(),
        self_model_tensor_id: self_model.tensor_id.clone(),
        scheduler_state_id: scheduler.scheduler_id.clone(),
        evidence_refs: Vec::<EvidenceRef>::new(),
    }
    .with_id()?;

    // §16.1 step 16 — bundle or hold.
    let (outcome, bundle) = if handoff_gate.passed {
        let candidate = CognitionCandidate {
            candidate_id: Hash256::zero(),
            source_path_id: state_hash.clone(),
            target_state_hash: state_hash.clone(),
            attractor_score: state5.stability_index.clone(),
            constraint_support_score: lattice.feasible_set_uniqueness.clone(),
            memory_support_score: Fixed::quantize(0.5, 9).unwrap(),
            projection_readiness: state5.stability_index.clone(),
        }
        .with_id()?;
        let bundle = CognitionCandidateBundle {
            bundle_id: Hash256::zero(),
            cognition_state_id: canonical_state.state_id.clone(),
            panorama_id: panorama.panorama_id.clone(),
            attractor_map_id: attractor_map.map_id.clone(),
            candidate_states: vec![candidate],
            recommended_projection_policy: ProjectionHandoffPolicy::ProjectBestCandidate,
            evidence_refs: Vec::new(),
        }
        .with_id()?;
        let bundle_hash =
            Hash256(
                content_address(&bundle).map_err(|e| CognitionError::Canonicalization {
                    reason: e.to_string(),
                })?,
            );
        (
            CognitionOutcome::CandidateBundle { bundle_hash },
            Some(bundle),
        )
    } else {
        let failed_gate = first_failed_gate(&handoff_gate);
        let failure_policy =
            select_failure_policy(failed_gate, &rd.policies.failure_policy_order);
        let diagnostics = vec![CognitionDiagnostic::new(
            failed_gate,
            format!("handoff gate failed: {failed_gate:?}"),
        )?];
        let recommended_next_step = recommend_action(&handoff_gate);
        let hold = CognitionHoldReport {
            hold_id: Hash256::zero(),
            rd_hash: rd_hash.clone(),
            cognition_state_id: canonical_state.state_id.clone(),
            failed_gate,
            failure_policy,
            diagnostics,
            recommended_next_step,
        }
        .with_id()?;
        (CognitionOutcome::Hold { hold }, None)
    };

    let outcome_hash = outcome.outcome_hash()?;

    // §17 — main report.
    let report = CognitionReport {
        report_id: Hash256::zero(),
        rd_hash: rd_hash.clone(),
        cognition_state_id: canonical_state.state_id.clone(),
        cognitive_state_5d_hash: state_hash.clone(),
        spiral_hitset_hash: hitset.hitset_id.clone(),
        lattice_hash: lattice.lattice_id.clone(),
        hypercube_puzzle_hash: puzzle.cube_id.clone(),
        phase_panorama_hash: panorama.panorama_id.clone(),
        self_model_tensor_hash: self_model.tensor_id.clone(),
        attractor_map_hash: attractor_map.map_id.clone(),
        handoff_gate_hash: handoff_gate.gate_id.clone(),
        outcome_hash: outcome_hash.clone(),
        evidence_refs: Vec::new(),
    }
    .with_id()?;

    // Replay hash.
    let replay_hash = replay_hash_of(&rd_hash, &canonical_state.state_id, &outcome_hash)?;

    Ok(CognitionRunResult {
        input: input.clone(),
        outcome,
        canonical_state,
        state5,
        hitset,
        lattice,
        puzzle,
        panorama,
        scheduler,
        self_model,
        attractor_map,
        handoff_gate,
        bundle,
        report,
        replay_hash,
    })
}

fn first_failed_gate(g: &CognitionHandoffGate) -> CognitionGateKind {
    if !g.g_perc {
        CognitionGateKind::Percolation
    } else if !g.g_resonance {
        CognitionGateKind::Resonance
    } else if !g.g_panorama {
        CognitionGateKind::Panorama
    } else if !g.g_self {
        CognitionGateKind::SelfModel
    } else if !g.g_trigger {
        CognitionGateKind::Trigger
    } else if !g.replay_ready {
        CognitionGateKind::Replay
    } else {
        CognitionGateKind::Composite
    }
}

/// Recovery policies applicable to each gate kind, in spec-recommended order.
/// `select_failure_policy` walks the caller's `failure_policy_order` and returns
/// the first entry that appears in this set — giving the descriptor full control
/// over escalation priority without touching the gate logic.
fn policies_for_gate(gate: CognitionGateKind) -> &'static [CognitionFailurePolicy] {
    use CognitionFailurePolicy::*;
    use CognitionGateKind::*;
    match gate {
        Percolation => &[RefineConstraints, SplitHypercube, CalibrateOperators, Hold],
        Resonance   => &[QuerySpiralMemory, ExpandPanorama, RefineConstraints, Hold],
        Panorama    => &[ExpandPanorama, AdmitWormhole, QuerySpiralMemory, Hold],
        SelfModel   => &[CalibrateOperators, MigrateCarrier, Hold],
        Trigger     => &[WaitForPhaseWindow, RefineConstraints, Hold],
        Replay      => &[RequireHumanReview, Hold],
        Composite   => &[RefineConstraints, Hold],
    }
}

fn select_failure_policy(
    gate: CognitionGateKind,
    policy_order: &[CognitionFailurePolicy],
) -> CognitionFailurePolicy {
    let applicable = policies_for_gate(gate);
    for p in policy_order {
        if applicable.contains(p) {
            return *p;
        }
    }
    // Caller's order has no applicable policy — use spec default (first in set).
    applicable[0]
}

fn recommend_action(g: &CognitionHandoffGate) -> Option<CognitionRecoveryAction> {
    if !g.g_perc {
        Some(CognitionRecoveryAction::RefineConstraints)
    } else if !g.g_resonance {
        Some(CognitionRecoveryAction::QuerySpiralMemory)
    } else if !g.g_panorama {
        Some(CognitionRecoveryAction::ExpandPanorama)
    } else if !g.g_self {
        Some(CognitionRecoveryAction::CalibrateOperators)
    } else if !g.g_trigger {
        Some(CognitionRecoveryAction::WaitForPhaseWindow)
    } else {
        None
    }
}

/// Resonance gate: passes when no resonance is required (min_resonance = 0)
/// or when the best memory hit is within the resonance distance threshold.
fn resonance_gate(hitset: &SpiralMemoryHitSet, min_resonance: &Fixed) -> bool {
    let is_zero = match min_resonance {
        Fixed::FixedI64 { raw, .. } => *raw == 0,
        Fixed::Rational { num, .. } => *num == 0,
    };
    if is_zero {
        return true;
    }
    hitset
        .hits
        .first()
        .map_or(false, |h| fixed_le(&h.resonance_score, min_resonance))
}

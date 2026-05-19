//! Full traversal stack orchestration (§Stack-01).
//!
//! Chains PSE-TRAVERSE-COGNITION-01 → PSE-TRAVERSE-HORIZON-03 → TPT-MTL-04
//! with deterministic content-addressed handoffs at each stage boundary.
//!
//! Every intermediate artifact is fully content-addressed so the full run
//! can be replayed from a `FullTraversalInput` alone. No `SemanticCrystal`
//! is emitted here — that remains the sole prerogative of `pse_core::macro_step`
//! via the PSE-Bridge.
//!
//! # Gate semantics
//!
//! `MonolithGate::passed = g_cognition ∧ g_horizon ∧ g_topology`
//!
//! All three stages always run (fail-transparent: every layer contributes
//! diagnostic information regardless of upstream gate results).

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use crate::cognition::pipeline::{run_cognition, CognitionInput, CognitionRunResult};
use crate::cognition::primitives::{fixed_add, CognitionError};
use crate::cognition::run_descriptor::CognitionRunDescriptor;

use crate::horizon::gate::ProjectionV2OutcomeRef;
use crate::horizon::outcome::HorizonV3Outcome;
use crate::horizon::pipeline::{run_horizon_v3, HorizonInput, HorizonRunResult};
use crate::horizon::primitives::HorizonError;
use crate::horizon::run_descriptor::HorizonRunDescriptorV3;

use crate::topology::carrier::CarrierContext;
use crate::topology::gates::TptMtlOutcomeKind;
use crate::topology::phase_window::{SamplePoint, TptMtlInput};
use crate::topology::pipeline::{run_tpt_mtl, TptMtlPipelineResult};
use crate::topology::primitives::{tpt_content_address, TptEvidenceRef, TopologyError};
use crate::topology::run_descriptor::TptMtlRunDescriptor;

// ── Error surface ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StackError {
    Cognition { reason: String },
    Horizon { reason: String },
    Topology { reason: String },
    Handoff { reason: String },
}

impl std::fmt::Display for StackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StackError::Cognition { reason } => write!(f, "cognition: {reason}"),
            StackError::Horizon { reason } => write!(f, "horizon: {reason}"),
            StackError::Topology { reason } => write!(f, "topology: {reason}"),
            StackError::Handoff { reason } => write!(f, "handoff: {reason}"),
        }
    }
}

impl From<CognitionError> for StackError {
    fn from(e: CognitionError) -> Self {
        StackError::Cognition { reason: e.to_string() }
    }
}

impl From<HorizonError> for StackError {
    fn from(e: HorizonError) -> Self {
        StackError::Horizon { reason: e.to_string() }
    }
}

impl From<TopologyError> for StackError {
    fn from(e: TopologyError) -> Self {
        StackError::Topology { reason: e.to_string() }
    }
}

// ── Input/Output types ────────────────────────────────────────────────────

/// Combined input for the full traversal stack. One run descriptor per layer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FullTraversalInput {
    pub cog_input: CognitionInput,
    pub cog_rd: CognitionRunDescriptor,
    pub horizon_rd: HorizonRunDescriptorV3,
    pub tpt_rd: TptMtlRunDescriptor,
}

/// Combined gate: all three pipeline stages must pass for the monolith to
/// emit a positive verdict. The `passed` field is the conjunction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonolithGate {
    pub gate_id: Hash256,
    /// Cognition: `CandidateBundle` was emitted (all 5 handoff sub-gates fired).
    pub g_cognition: bool,
    /// Horizon: `FinalizedEmissionV3` was emitted (`G_v0.2 ∧ G_v0.3 ∧ ReplayReady`).
    pub g_horizon: bool,
    /// Topology: `TptMtlOutcomeKind::Emit` was produced.
    pub g_topology: bool,
    pub passed: bool,
}

impl MonolithGate {
    fn build(
        g_cognition: bool,
        g_horizon: bool,
        g_topology: bool,
    ) -> Result<Self, StackError> {
        let passed = g_cognition && g_horizon && g_topology;
        let id = content_address(&(g_cognition, g_horizon, g_topology, passed))
            .map_err(|e| StackError::Handoff { reason: e.to_string() })?;
        Ok(MonolithGate { gate_id: Hash256(id), g_cognition, g_horizon, g_topology, passed })
    }
}

/// All intermediate results plus the combined gate and a cross-layer
/// replay hash.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FullTraversalResult {
    pub cognition: CognitionRunResult,
    pub horizon: HorizonRunResult,
    pub topology: TptMtlPipelineResult,
    pub gate: MonolithGate,
    /// Cross-layer replay anchor: `H(cog.replay_hash ‖ horizon.cert_id ‖ topo.candidate_id)`.
    pub replay_hash: Hash256,
}

// ── Handoff adapters ──────────────────────────────────────────────────────

/// Cognition → Horizon.
///
/// Maps the canonical cognition state as the Horizon `state_id`.
/// When a `CandidateBundle` was emitted, its bundle_id becomes the
/// `ProjectionV2OutcomeRef.certificate_hash` — signalling to the Horizon
/// gate that the projection layer certified this state.
fn cog_to_horizon_input(cog: &CognitionRunResult) -> HorizonInput {
    let projection_v2 = cog.bundle.as_ref().map(|b| ProjectionV2OutcomeRef {
        passed: true,
        certificate_hash: Some(b.bundle_id.clone()),
    });
    HorizonInput {
        state_id: cog.canonical_state.state_id.clone(),
        null_center_id: Some(cog.input.null_center_id.clone()),
        duality_spec: None,
        projection_v2,
    }
}

/// Cognition + Horizon → TptMtlInput.
///
/// The 5D cognitive state seeds a local phase-space neighborhood: the canonical
/// point plus four axis-aligned neighbours at ε = 0.05. Five points yield
/// C(5,3)=10 simplices → vertex/simplex ratio 2.0 → EntropyClass::Simplify,
/// which is the minimum for topology emission. All neighbours carry the same
/// provenance as the canonical point (they probe local geometry, not new state).
///
/// The Horizon certificate_id and the Cognition lattice_id are wired in as
/// horizon_refs and constraint_refs for full cross-layer provenance.
fn cog_horizon_to_tpt_input(
    cog: &CognitionRunResult,
    horizon: &HorizonRunResult,
) -> Result<TptMtlInput, StackError> {
    // state_digest: deterministic hash of both upstream layers' canonical ids.
    let state_digest_bytes =
        content_address(&(&cog.canonical_state.state_id, &horizon.certificate.certificate_id))
            .map_err(|e| StackError::Handoff { reason: e.to_string() })?;
    let state_digest = Hash256(state_digest_bytes);

    let s = &cog.state5;
    let base_prov = TptEvidenceRef {
        evidence_id: Hash256::zero(),
        kind: "cognition.canonical_state".into(),
        source_digest: cog.canonical_state.state_id.clone(),
        trace_ref: cog.replay_hash.clone(),
    };

    // ε = 0.05 in each principal axis — small enough not to distort geometry,
    // large enough to produce distinct vertices after content-addressing.
    let eps = crate::dynamic_state::CanonicalNumber::quantize(0.05, 9)
        .map_err(|e| StackError::Handoff { reason: e.to_string() })?;

    let coords_base = [
        s.psi_semantic_phase.clone(),
        s.rho_coherence_density.clone(),
        s.omega_phase_readiness.clone(),
        s.chi_topological_curvature.clone(),
        s.tau_entropy_symmetry.clone(),
    ];

    // Generate 5 points: [base, +ε on each of psi/rho/omega/chi].
    let mut sample_points: Vec<SamplePoint> = Vec::with_capacity(5);
    sample_points.push(SamplePoint {
        coordinates: coords_base.clone(),
        provenance: vec![base_prov.clone()],
        status: None,
    });
    for axis in 0..4usize {
        let mut coords = coords_base.clone();
        coords[axis] = fixed_add(&coords[axis], &eps);
        sample_points.push(SamplePoint {
            coordinates: coords,
            provenance: vec![base_prov.clone()],
            status: None,
        });
    }

    // boundary_ref: content-address of the horizon certificate chain.
    let boundary_ref =
        tpt_content_address(&horizon.certificate.certificate_id)
            .map_err(|e| StackError::Topology { reason: e.to_string() })?;

    // trace_ref: content-address of both replay hashes.
    let trace_ref =
        tpt_content_address(&(&cog.replay_hash, &horizon.certificate.certificate_id))
            .map_err(|e| StackError::Topology { reason: e.to_string() })?;

    Ok(TptMtlInput {
        state_digest,
        carrier: Some(CarrierContext::new_minimal(cog.input.logical_step)),
        horizon_refs: vec![horizon.certificate.certificate_id.clone()],
        constraint_refs: vec![cog.lattice.lattice_id.clone()],
        sample_points,
        boundary_ref,
        trace_ref,
    })
}

// ── Main entry point ──────────────────────────────────────────────────────

/// Run the full three-stage traversal stack.
///
/// Stages run unconditionally (fail-transparent): even if Cognition holds,
/// Horizon and Topology still execute so all three gate verdicts are
/// available in the result. The `MonolithGate.passed` conjunction summarises
/// the aggregate verdict.
pub fn run_traverse_stack(
    input: &FullTraversalInput,
) -> Result<FullTraversalResult, StackError> {
    // ── Stage 1: Cognition ────────────────────────────────────────────────
    let cog = run_cognition(&input.cog_input, &input.cog_rd)?;
    let g_cognition = cog.bundle.is_some();

    // ── Stage 2: Horizon ─────────────────────────────────────────────────
    let horizon_input = cog_to_horizon_input(&cog);
    let horizon = run_horizon_v3(&horizon_input, &input.horizon_rd)?;
    let g_horizon = matches!(&horizon.outcome, HorizonV3Outcome::Finalized { .. });

    // ── Stage 3: Topology ─────────────────────────────────────────────────
    let tpt_input = cog_horizon_to_tpt_input(&cog, &horizon)?;
    let topo = run_tpt_mtl(&tpt_input, &input.tpt_rd)?;
    let g_topology = topo.outcome.kind == TptMtlOutcomeKind::Emit;

    // ── Combined gate ─────────────────────────────────────────────────────
    let gate = MonolithGate::build(g_cognition, g_horizon, g_topology)?;

    // ── Cross-layer replay hash ───────────────────────────────────────────
    let replay_bytes = content_address(&(
        &cog.replay_hash,
        &horizon.certificate.certificate_id,
        &topo.candidate.candidate_id,
    ))
    .map_err(|e| StackError::Handoff { reason: e.to_string() })?;
    let replay_hash = Hash256(replay_bytes);

    Ok(FullTraversalResult {
        cognition: cog,
        horizon,
        topology: topo,
        gate,
        replay_hash,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::cognition::run_descriptor::{
        CognitionPolicies, CognitionThresholds,
    };
    use crate::cognition::pipeline::CognitiveComponents;
    use crate::horizon::run_descriptor::HorizonRunDescriptorV3;
    use crate::topology::run_descriptor::TptMtlRunDescriptor;

    use super::*;

    fn make_full_input(step: u64) -> FullTraversalInput {
        let cog_input = CognitionInput {
            null_center_id: Hash256::zero(),
            cognitive_components: CognitiveComponents {
                psi: crate::cognition::primitives::Fixed::quantize(0.5, 9).unwrap(),
                rho: crate::cognition::primitives::Fixed::quantize(0.7, 9).unwrap(),
                omega: crate::cognition::primitives::Fixed::quantize(0.6, 9).unwrap(),
                chi: crate::cognition::primitives::Fixed::quantize(0.2, 9).unwrap(),
                tau: crate::cognition::primitives::Fixed::quantize(0.3, 9).unwrap(),
            },
            source_traversal_report_hash: None,
            source_projection_report_hash: None,
            spiral_memory_candidates: vec![],
            constraint_count: 4,
            support_strength: crate::cognition::primitives::Fixed::quantize(0.5, 9).unwrap(),
            logical_step: step,
            carrier_ids: vec!["stack.test".into()],
        };
        FullTraversalInput {
            cog_input,
            cog_rd: CognitionRunDescriptor {
                run_id: format!("stack.test.{step}"),
                problem_spec_hash: Hash256::zero(),
                traversal_report_hash: None,
                projection_v2_hash: None,
                seed: 0,
                operator_versions: BTreeMap::new(),
                thresholds: CognitionThresholds::permissive(),
                policies: CognitionPolicies::default_policies(),
                canonicalization_version: "cognition-v0.1".into(),
            },
            horizon_rd: HorizonRunDescriptorV3::permissive(),
            tpt_rd: TptMtlRunDescriptor::default_permissive(),
        }
    }

    #[test]
    fn stack_runs_end_to_end() {
        let input = make_full_input(1);
        let result = run_traverse_stack(&input).expect("stack run");
        // Gate fields are structurally present.
        assert!(result.replay_hash != Hash256::zero());
        assert!(result.gate.gate_id != Hash256::zero());
    }

    #[test]
    fn stack_is_byte_deterministic() {
        let a = run_traverse_stack(&make_full_input(7)).unwrap();
        let b = run_traverse_stack(&make_full_input(7)).unwrap();
        assert_eq!(a.replay_hash, b.replay_hash);
        assert_eq!(a.gate.gate_id, b.gate.gate_id);
    }

    #[test]
    fn different_inputs_produce_different_replay_hashes() {
        let a = run_traverse_stack(&make_full_input(1)).unwrap();
        let b = run_traverse_stack(&make_full_input(2)).unwrap();
        assert_ne!(a.replay_hash, b.replay_hash);
    }
}

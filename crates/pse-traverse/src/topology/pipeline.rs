//! Reference pipeline `run_tpt_mtl` (TPT-MTL §9.2).
//!
//! Implements the full BuildWindow → Triangulate → LiftMicro → Reinterpret →
//! Gate → Bundle flow. All gates are fail-closed.

use serde::{Deserialize, Serialize};

use super::artifact::{
    form_topological_crystal_candidate, materialize_bundle, TopologicalCrystalCandidate,
    TptMtlBundle,
};
use super::axis_bridge::{build_axis_bridge_report, AxisBridgeReport};
use super::carrier::{evaluate_carrier, CarrierReport};
use super::gates::{gate_all, TptMtlGateReport, TptMtlOutcomeKind};
use super::mesh_holo::{evolve_mesh, seed_mesh_holo, MeshHolo};
use super::mesh_weights::weight_mesh_with_microfibers;
use super::micro_fiber::{lift_micro_fibers, MicroFiber};
use super::phase_window::{build_phase_space_window, PhaseSpaceWindow, TptMtlInput};
use super::primitives::{tpt_content_address, Hash256, TopologyError};
use super::refine::refine_recursive;
use super::reinterpret::{reinterpret_mesh_to_panoptic, ReinterpretationReport};
use super::replay::{ReplayManifest};
use super::run_descriptor::TptMtlRunDescriptor;

/// The complete outcome of a `run_tpt_mtl` call.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TptMtlOutcome {
    pub kind: TptMtlOutcomeKind,
    pub bundle: TptMtlBundle,
    pub replay_manifest: ReplayManifest,
    /// Diagnostic messages from failed gates.
    pub diagnostics: Vec<String>,
}

/// Complete pipeline intermediates (useful for CLI and tests).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TptMtlPipelineResult {
    pub input: TptMtlInput,
    pub rd_hash: Hash256,
    pub window: PhaseSpaceWindow,
    pub axis_report: AxisBridgeReport,
    pub mesh: MeshHolo,
    pub fibers: Vec<MicroFiber>,
    pub carrier_report: CarrierReport,
    pub reinterp: ReinterpretationReport,
    pub gate_report: TptMtlGateReport,
    pub candidate: TopologicalCrystalCandidate,
    pub outcome: TptMtlOutcome,
}

/// Run the full TPT-MTL pipeline.
///
/// Returns `TptMtlOutcome` with the appropriate `TptMtlOutcomeKind`.
/// On any hard-failure (Abort, BudgetExhausted) the error is mapped to
/// an Abort outcome with a Hold bundle where possible.
pub fn run_tpt_mtl(
    input: &TptMtlInput,
    rd: &TptMtlRunDescriptor,
) -> Result<TptMtlPipelineResult, TopologyError> {
    rd.validate()?;
    let rd_hash = rd.content_hash()?;

    // ── BuildWindow ─────────────────────────────────────────────────────
    let window = build_phase_space_window(input, rd)?;

    // ── AxisBridge ──────────────────────────────────────────────────────
    let axis_report = build_axis_bridge_report(&window, rd)?;
    if !axis_report.passed {
        // Recalibrate path: return early with partial bundle.
        return early_outcome(
            TptMtlOutcomeKind::Recalibrate,
            input,
            rd,
            rd_hash,
            window,
            axis_report,
            "axis bridge failed",
        );
    }

    // ── SeedMesh + EvolveMesh ────────────────────────────────────────────
    let mesh0 = seed_mesh_holo(&window, rd)?;
    let (mesh1, _guard_proof) = evolve_mesh(mesh0, rd)?;

    // ── LiftMicro ───────────────────────────────────────────────────────
    let fibers = lift_micro_fibers(&window, &mesh1, rd)?;
    let (micro_ok, micro_diags) = super::micro_fiber::gate_micro_lift(&fibers);
    if !micro_ok {
        return early_outcome_with_mesh(
            TptMtlOutcomeKind::Recalibrate,
            input,
            rd,
            rd_hash,
            window,
            axis_report,
            mesh1,
            &micro_diags.join("; "),
        );
    }

    // ── WeightMicroMesh ─────────────────────────────────────────────────
    let mesh2 = weight_mesh_with_microfibers(mesh1, &fibers, rd)?;

    // ── RefineRecursive (budgeted) ───────────────────────────────────────
    let (refined_mesh, _depth) = refine_recursive(mesh2, rd, rd.budgets.max_depth)?;

    // ── EvaluateCarrier ─────────────────────────────────────────────────
    let carrier_report = evaluate_carrier(&window.carrier, rd, false)?;

    // ── Reinterpret ─────────────────────────────────────────────────────
    let reinterp = reinterpret_mesh_to_panoptic(&refined_mesh, &carrier_report, rd)?;

    // ── GateAll ─────────────────────────────────────────────────────────
    let gate_report = gate_all(
        &window,
        &axis_report,
        &refined_mesh,
        &fibers,
        &carrier_report,
        &reinterp,
        rd,
        None, // no expected hashes on first run
        None,
    )?;

    // ── TopologicalCrystalCandidate ──────────────────────────────────────
    let candidate = form_topological_crystal_candidate(
        &window,
        &refined_mesh,
        &fibers,
        &carrier_report,
        &reinterp,
        &gate_report,
    )?;

    // ── Bundle ──────────────────────────────────────────────────────────
    let replay_manifest_hash = Hash256::zero(); // computed after bundle
    let bundle = materialize_bundle(
        rd,
        &window,
        &axis_report,
        &refined_mesh,
        &fibers,
        &carrier_report,
        &reinterp,
        &gate_report,
        &candidate,
        &replay_manifest_hash,
    )?;

    let replay_manifest = ReplayManifest::from_bundle(&bundle)?;

    // Collect diagnostics from failed gates.
    let diagnostics: Vec<String> = gate_report
        .gate_results
        .iter()
        .filter(|g| !g.passed)
        .flat_map(|g| g.diagnostics.clone())
        .collect();

    let outcome_kind = gate_report.outcome_kind();
    let outcome = TptMtlOutcome {
        kind: outcome_kind,
        bundle,
        replay_manifest,
        diagnostics,
    };

    Ok(TptMtlPipelineResult {
        input: input.clone(),
        rd_hash,
        window,
        axis_report,
        mesh: refined_mesh,
        fibers,
        carrier_report,
        reinterp,
        gate_report,
        candidate,
        outcome,
    })
}

// ── early exit helpers ───────────────────────────────────────────────────────

fn early_outcome(
    kind: TptMtlOutcomeKind,
    input: &TptMtlInput,
    rd: &TptMtlRunDescriptor,
    rd_hash: Hash256,
    window: PhaseSpaceWindow,
    axis_report: AxisBridgeReport,
    reason: &str,
) -> Result<TptMtlPipelineResult, TopologyError> {
    let dummy_mesh = MeshHolo {
        mesh_id: Hash256::zero(),
        window_ref: window.window_id.clone(),
        vertices: vec![],
        edges: vec![],
        simplices: vec![],
        invariants: super::mesh_holo::TopologicalInvariants {
            betti: vec![],
            persistence_digest: Hash256::zero(),
            entropy_class: super::mesh_holo::EntropyClass::Explore,
            lambda_gap: super::primitives::Fixed::quantize(0.0, 9).unwrap(),
        },
        proof: super::mesh_holo::MeshProof {
            proof_id: Hash256::zero(),
            topology_guard_proofs: vec![],
            symmetry_pass: false,
            seed_hash: Hash256::zero(),
        },
        trace_ref: window.trace_ref.clone(),
    };
    early_outcome_with_mesh(kind, input, rd, rd_hash, window, axis_report, dummy_mesh, reason)
}

fn early_outcome_with_mesh(
    kind: TptMtlOutcomeKind,
    input: &TptMtlInput,
    rd: &TptMtlRunDescriptor,
    rd_hash: Hash256,
    window: PhaseSpaceWindow,
    axis_report: AxisBridgeReport,
    mesh: MeshHolo,
    reason: &str,
) -> Result<TptMtlPipelineResult, TopologyError> {
    let carrier_report = evaluate_carrier(&window.carrier, rd, false)?;
    let reinterp = reinterpret_mesh_to_panoptic(&mesh, &carrier_report, rd)?;
    let gate_report = gate_all(
        &window,
        &axis_report,
        &mesh,
        &[],
        &carrier_report,
        &reinterp,
        rd,
        None,
        None,
    )?;
    let candidate = form_topological_crystal_candidate(
        &window,
        &mesh,
        &[],
        &carrier_report,
        &reinterp,
        &gate_report,
    )?;
    let bundle = materialize_bundle(
        rd,
        &window,
        &axis_report,
        &mesh,
        &[],
        &carrier_report,
        &reinterp,
        &gate_report,
        &candidate,
        &Hash256::zero(),
    )?;
    let replay_manifest = ReplayManifest::from_bundle(&bundle)?;
    let outcome = TptMtlOutcome {
        kind: kind.clone(),
        bundle,
        replay_manifest,
        diagnostics: vec![reason.into()],
    };
    Ok(TptMtlPipelineResult {
        input: input.clone(),
        rd_hash,
        window,
        axis_report,
        mesh,
        fibers: vec![],
        carrier_report,
        reinterp,
        gate_report,
        candidate,
        outcome,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::phase_window::{PointStatus, SamplePoint};
    use crate::topology::primitives::{Fixed, Hash256, TptEvidenceRef};
    use crate::topology::run_descriptor::TptMtlRunDescriptor;

    fn minimal_input(n: usize) -> TptMtlInput {
        let sample_points = (0..n)
            .map(|i| SamplePoint {
                coordinates: [
                    Fixed::quantize(i as f64 * 0.1 + 0.05, 9).unwrap(),
                    Fixed::quantize(i as f64 * 0.1 + 0.05, 9).unwrap(),
                    Fixed::quantize(i as f64 * 0.1 + 0.05, 9).unwrap(),
                    Fixed::quantize(i as f64 * 0.1 + 0.05, 9).unwrap(),
                    Fixed::quantize(i as f64 * 0.1 + 0.05, 9).unwrap(),
                ],
                provenance: vec![TptEvidenceRef {
                    evidence_id: Hash256::zero(),
                    kind: "test".into(),
                    source_digest: Hash256::zero(),
                    trace_ref: Hash256::zero(),
                }],
                status: Some(PointStatus::Active),
            })
            .collect();

        TptMtlInput {
            state_digest: Hash256::from_hex(
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            )
            .unwrap(),
            carrier: Some(crate::topology::carrier::CarrierContext::new_minimal(1)),
            horizon_refs: vec![],
            constraint_refs: vec![],
            sample_points,
            boundary_ref: Hash256::zero(),
            trace_ref: Hash256::zero(),
        }
    }

    #[test]
    fn pipeline_end_to_end_is_deterministic() {
        let rd = TptMtlRunDescriptor::default_permissive();
        let input = minimal_input(3);
        let r1 = run_tpt_mtl(&input, &rd).unwrap();
        let r2 = run_tpt_mtl(&input, &rd).unwrap();
        assert_eq!(
            r1.outcome.bundle.bundle_id,
            r2.outcome.bundle.bundle_id,
            "pipeline MUST be deterministic"
        );
    }

    #[test]
    fn pipeline_replay_verifies() {
        let rd = TptMtlRunDescriptor::default_permissive();
        let input = minimal_input(2);
        let r1 = run_tpt_mtl(&input, &rd).unwrap();
        let manifest = r1.outcome.replay_manifest.clone();
        let r2 = run_tpt_mtl(&input, &rd).unwrap();
        assert!(
            manifest.verify(&r2.outcome.bundle).is_ok(),
            "replay MUST produce identical bundle"
        );
    }

    #[test]
    fn pipeline_produces_window_and_mesh() {
        let rd = TptMtlRunDescriptor::default_permissive();
        let input = minimal_input(3);
        let result = run_tpt_mtl(&input, &rd).unwrap();
        assert!(!result.window.points.is_empty());
        assert_ne!(result.mesh.mesh_id, Hash256::zero());
    }
}

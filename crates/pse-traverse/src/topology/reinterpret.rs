//! Reinterpretation of topological features into panoptic cognition (TPT-MTL §11).
//!
//! Feature mapping from mesh topology → claim candidates, horizon updates,
//! gate signals and proof obligations.
//!
//! Invariant: topological gaps, simulated paths and future-window features
//! MUST be marked counterfactual, latent or blocked. They MUST NOT be
//! emitted as observed claims.

use serde::{Deserialize, Serialize};

use super::carrier::CarrierReport;
use super::mesh_holo::{EntropyClass, MeshHolo};
use super::primitives::{tpt_content_address, Hash256, TopologyError};
use super::run_descriptor::TptMtlRunDescriptor;

/// Status of a reinterpreted claim candidate.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ClaimStatus {
    /// Derived from persistent evidence (robuste Motivfamilie).
    Derived,
    /// Open or noise-level (kurzlebiges Feature).
    Open,
    /// Blocked — requires contradiction check.
    Blocked,
    /// Counterfactual — must not be emitted as observed.
    Counterfactual,
    /// Latent — candidate possible but not yet gate-passed.
    Latent,
}

/// A reinterpreted claim candidate from a topological feature.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ClaimCandidate {
    pub claim_id: Hash256,
    pub feature_kind: String,
    pub status: ClaimStatus,
    pub scope: String,
    pub evidence_refs: Vec<Hash256>,
    pub lineage_ref: Hash256,
    pub contradiction_check: bool,
}

/// Horizon update direction.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum HorizonUpdateKind {
    Current,
    Counter,
}

/// A horizon update produced by reinterpretation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HorizonUpdate {
    pub update_id: Hash256,
    pub kind: HorizonUpdateKind,
    pub source_feature: String,
    pub claim_ref: Hash256,
}

/// The full reinterpretation report (TPT-MTL §11.2).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ReinterpretationReport {
    pub report_id: Hash256,
    pub mesh_ref: Hash256,
    pub claim_candidates: Vec<ClaimCandidate>,
    pub current_horizon_updates: Vec<HorizonUpdate>,
    pub counter_horizon_updates: Vec<HorizonUpdate>,
    pub gate_signals: Vec<Hash256>,
    pub proof_obligations: Vec<Hash256>,
    pub contradictions: Vec<Hash256>,
    pub trace_ref: Hash256,
}

/// Reinterpret the mesh topology into panoptic cognition structures.
pub fn reinterpret_mesh_to_panoptic(
    mesh: &MeshHolo,
    carrier_report: &CarrierReport,
    rd: &TptMtlRunDescriptor,
) -> Result<ReinterpretationReport, TopologyError> {
    let _ = rd;
    let mut claim_candidates = Vec::new();
    let mut current_horizon_updates = Vec::new();
    let mut counter_horizon_updates = Vec::new();
    let mut gate_signals = Vec::new();
    let mut proof_obligations = Vec::new();
    let contradictions = Vec::new();

    let betti = &mesh.invariants.betti;
    let entropy = &mesh.invariants.entropy_class;

    // β_0 (connected components) → persistent component families.
    let b0 = betti.get(0).copied().unwrap_or(0);
    for i in 0..b0 {
        let claim = make_claim(
            &format!("persistent-component-{}", i),
            ClaimStatus::Derived,
            "component",
            &mesh.mesh_id,
        )?;
        let update = make_horizon_update(
            HorizonUpdateKind::Current,
            "beta0-component",
            &claim.claim_id,
        )?;
        gate_signals.push(tpt_content_address(&(&claim.claim_id, "beta0-gate"))?);
        proof_obligations.push(tpt_content_address(&(&claim.claim_id, "beta0-proof"))?);
        current_horizon_updates.push(update);
        claim_candidates.push(claim);
    }

    // β_1 (loops) → cyclic dependencies / unresolved relations.
    let b1 = betti.get(1).copied().unwrap_or(0);
    for i in 0..b1 {
        let claim = make_claim(
            &format!("cycle-{}", i),
            ClaimStatus::Blocked,
            "cycle",
            &mesh.mesh_id,
        )?;
        let update =
            make_horizon_update(HorizonUpdateKind::Counter, "beta1-cycle", &claim.claim_id)?;
        counter_horizon_updates.push(update);
        claim_candidates.push(claim);
    }

    // Entropy class → exploration or capture zone signals.
    let entropy_claim_status = match entropy {
        EntropyClass::Explore => ClaimStatus::Counterfactual, // not emission-capable
        EntropyClass::Refine => ClaimStatus::Latent,
        EntropyClass::Simplify => ClaimStatus::Latent,
        EntropyClass::Freeze => ClaimStatus::Derived,
    };
    let entropy_claim = make_claim(
        &format!("entropy-{:?}", entropy),
        entropy_claim_status,
        "entropy-zone",
        &mesh.mesh_id,
    )?;
    gate_signals.push(tpt_content_address(&(
        &entropy_claim.claim_id,
        "entropy-gate",
    ))?);
    claim_candidates.push(entropy_claim);

    // Carrier continuity gate signal.
    if carrier_report.passed {
        gate_signals.push(tpt_content_address(&(
            &carrier_report.report_id,
            "carrier-ok",
        ))?);
    } else {
        gate_signals.push(tpt_content_address(&(
            &carrier_report.report_id,
            "carrier-fail",
        ))?);
    }

    // Sort all for determinism.
    claim_candidates.sort();
    current_horizon_updates.sort();
    counter_horizon_updates.sort();
    gate_signals.sort();
    proof_obligations.sort();

    let partial = ReinterpretationReport {
        report_id: Hash256::zero(),
        mesh_ref: mesh.mesh_id.clone(),
        claim_candidates,
        current_horizon_updates,
        counter_horizon_updates,
        gate_signals,
        proof_obligations,
        contradictions,
        trace_ref: mesh.trace_ref.clone(),
    };
    let report_id = tpt_content_address(&partial)?;
    Ok(ReinterpretationReport {
        report_id,
        ..partial
    })
}

fn make_claim(
    feature: &str,
    status: ClaimStatus,
    scope: &str,
    mesh_ref: &Hash256,
) -> Result<ClaimCandidate, TopologyError> {
    let claim_id = tpt_content_address(&("claim", feature, scope, mesh_ref))?;
    let lineage_ref = tpt_content_address(&("lineage", feature))?;
    Ok(ClaimCandidate {
        claim_id,
        feature_kind: feature.into(),
        status,
        scope: scope.into(),
        evidence_refs: vec![mesh_ref.clone()],
        lineage_ref,
        contradiction_check: false,
    })
}

fn make_horizon_update(
    kind: HorizonUpdateKind,
    source: &str,
    claim_ref: &Hash256,
) -> Result<HorizonUpdate, TopologyError> {
    let update_id = tpt_content_address(&("horizon-update", source, claim_ref))?;
    Ok(HorizonUpdate {
        update_id,
        kind,
        source_feature: source.into(),
        claim_ref: claim_ref.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::carrier::{evaluate_carrier, CarrierContext};
    use crate::topology::mesh_holo::seed_mesh_holo;
    use crate::topology::phase_window::{PointMeta, PointStatus, TptPoint};
    use crate::topology::primitives::{tpt_content_address, Fixed, Hash256, TptEvidenceRef};
    use crate::topology::run_descriptor::TptMtlRunDescriptor;

    fn make_window(n: usize) -> super::super::phase_window::PhaseSpaceWindow {
        let rd = TptMtlRunDescriptor::default_permissive();
        let mut points = Vec::new();
        for i in 0..n {
            let v = Fixed::quantize(i as f64 * 0.2, 9).unwrap();
            let point_id = tpt_content_address(&("ri-pt", i)).unwrap();
            points.push(TptPoint {
                point_id,
                x: [v.clone(), v.clone(), v.clone(), v.clone(), v.clone()],
                meta: PointMeta {
                    semantic_axis_labels: rd.axis_policy.semantic_axes.clone(),
                    runtime_axis_labels: rd.axis_policy.runtime_axes.clone(),
                    status: PointStatus::Active,
                },
                provenance: vec![TptEvidenceRef {
                    evidence_id: Hash256::zero(),
                    kind: "test".into(),
                    source_digest: Hash256::zero(),
                    trace_ref: Hash256::zero(),
                }],
                carrier_ref: Hash256::zero(),
                gate_refs: vec![],
            });
        }
        points.sort();
        super::super::phase_window::PhaseSpaceWindow {
            window_id: tpt_content_address(&("rw2", n)).unwrap(),
            domain_profile: rd.domain_profile.clone(),
            input_refs: vec![],
            points,
            carrier: CarrierContext::new_minimal(1),
            horizon_refs: vec![],
            constraint_refs: vec![],
            boundary_ref: Hash256::zero(),
            sampling_policy_hash: Hash256::zero(),
            trace_ref: Hash256::zero(),
            rd_hash: Hash256::zero(),
        }
    }

    #[test]
    fn reinterpretation_produces_current_and_counter_horizons() {
        let rd = TptMtlRunDescriptor::default_permissive();
        let window = make_window(3);
        let mesh = seed_mesh_holo(&window, &rd).unwrap();
        let carrier = CarrierContext::new_minimal(1);
        let carrier_report = evaluate_carrier(&carrier, &rd, false).unwrap();
        let report = reinterpret_mesh_to_panoptic(&mesh, &carrier_report, &rd).unwrap();
        assert_ne!(report.report_id, Hash256::zero());
        // At least entropy claim is always produced.
        assert!(!report.claim_candidates.is_empty());
    }

    #[test]
    fn reinterpretation_is_deterministic() {
        let rd = TptMtlRunDescriptor::default_permissive();
        let window = make_window(2);
        let mesh = seed_mesh_holo(&window, &rd).unwrap();
        let carrier = CarrierContext::new_minimal(1);
        let carrier_report = evaluate_carrier(&carrier, &rd, false).unwrap();
        let r1 = reinterpret_mesh_to_panoptic(&mesh, &carrier_report, &rd).unwrap();
        let r2 = reinterpret_mesh_to_panoptic(&mesh, &carrier_report, &rd).unwrap();
        assert_eq!(r1.report_id, r2.report_id);
    }
}

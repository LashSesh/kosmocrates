//! Reference pipeline `run_cell_substrate_cycle` (§19).
//!
//! End-to-end deterministic execution of one cell-substrate cycle:
//! pool → pulses → formation → cluster → funnel → morphology →
//! convergence → intent → trace → dissolution. Fail-closed at every
//! gate.

use serde::{Deserialize, Serialize};

use super::cell_pool::CellPool;
use super::cluster_gate::{
    dissolution_gate, intent_gate, morphology_gate, ClusterFormationGateOutcome,
};
use super::cluster_trace::ClusterTrace;
use super::convergence_field::ConvergenceField;
use super::dissolution::{DissolutionMode, DissolutionReport};
use super::funnel_graph::FunnelGraph;
use super::matrix::{MatrixBoundaryReport, PhaseSubnet};
use super::morphodynamic_field::{ClusterMorphologyEvent, MorphodynamicField, MorphologyDecision};
use super::phase_cell::{PhaseCell, PhaseCellRole};
use super::primitives::{
    content_address, fixed_add, fixed_ge, max_fixed, EvidenceRef, Fixed, Hash256, PhaseError,
};
use super::resonance_cluster::{ClusterFormationReport, ClusterLifecycle, ResonanceCluster};
use super::resonance_pulse::ResonancePulse;
use super::run_descriptor::PhaseMatrixRunDescriptorV3;
use super::tension_to_intent::IntentCandidate;
use super::trident_vector::TridentVector;

/// Caller-provided cycle input.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellSubstrateInput {
    /// Anchoring subnet.
    pub subnet: PhaseSubnet,
    /// Cluster purpose hash (declared by the caller).
    pub purpose_hash: Hash256,
    /// Cells to seed the pool with.
    pub seed_cells: Vec<PhaseCell>,
    /// Logical step counter.
    pub logical_step: u64,
}

impl CellSubstrateInput {
    /// Build a synthetic input over a `member_count`-node subnet with
    /// `cells_per_node` cells per member.
    pub fn synthetic(
        matrix_id: Hash256,
        member_count: u8,
        cells_per_node: u32,
    ) -> Result<Self, PhaseError> {
        let subnet = PhaseSubnet::synthetic(matrix_id.clone(), member_count)?;
        let mut seed_cells = Vec::new();
        for node_id in &subnet.member_nodes {
            for i in 0..cells_per_node {
                let role = match i % 4 {
                    0 => PhaseCellRole::Sensor,
                    1 => PhaseCellRole::Resonator,
                    2 => PhaseCellRole::Validator,
                    _ => PhaseCellRole::CandidateEmitter,
                };
                seed_cells.push(PhaseCell::synthetic(node_id.clone(), i, role)?);
            }
        }
        let purpose_hash = content_address(&("cycle.purpose", &matrix_id))?;
        Ok(CellSubstrateInput {
            subnet,
            purpose_hash,
            seed_cells,
            logical_step: 0,
        })
    }
}

/// Successful cycle report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellSubstrateCycleReport {
    /// Content-addressed report id.
    pub report_id: Hash256,
    /// RD content hash.
    pub rd_hash: Hash256,
    /// Pool hash.
    pub pool_hash: Hash256,
    /// Resulting cluster.
    pub cluster: ResonanceCluster,
    /// Funnel graph for the cluster.
    pub funnel: FunnelGraph,
    /// Convergence field for the cluster.
    pub convergence: ConvergenceField,
    /// Optional intent candidate (only when intent gate fired).
    pub intent: Option<IntentCandidate>,
    /// Cluster trace.
    pub trace: ClusterTrace,
    /// Optional dissolution report.
    pub dissolution: Option<DissolutionReport>,
}

impl CellSubstrateCycleReport {
    fn with_id(mut self) -> Result<Self, PhaseError> {
        let mut probe = self.clone();
        probe.report_id = Hash256::zero();
        self.report_id = content_address(&probe)?;
        Ok(self)
    }
}

/// Cluster-hold report (§21 outcome variant).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterHoldReport {
    /// Content-addressed hold id.
    pub hold_id: Hash256,
    /// RD content hash.
    pub rd_hash: Hash256,
    /// Failed gate name.
    pub failed_gate: String,
    /// Diagnostic messages.
    pub diagnostics: Vec<String>,
}

impl ClusterHoldReport {
    fn with_id(mut self) -> Result<Self, PhaseError> {
        self.diagnostics.sort();
        self.diagnostics.dedup();
        let mut probe = self.clone();
        probe.hold_id = Hash256::zero();
        self.hold_id = content_address(&probe)?;
        Ok(self)
    }
}

/// Cluster-rejection report (§21 outcome variant).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterRejectionReport {
    /// Content-addressed rejection id.
    pub rejection_id: Hash256,
    /// RD content hash.
    pub rd_hash: Hash256,
    /// Reason for rejection.
    pub reason: String,
}

impl ClusterRejectionReport {
    /// Recompute `rejection_id`.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        let mut probe = self.clone();
        probe.rejection_id = Hash256::zero();
        self.rejection_id = content_address(&probe)?;
        Ok(self)
    }
}

/// Determinism-violation report (§21 outcome variant).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeterminismReport {
    /// Content-addressed report id.
    pub report_id: Hash256,
    /// RD content hash.
    pub rd_hash: Hash256,
    /// Description of the determinism violation.
    pub reason: String,
}

/// Cycle outcome (§21).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum CellSubstrateOutcome {
    /// Cycle completed successfully.
    Completed(CellSubstrateCycleReport),
    /// Cycle held at the formation gate.
    Hold(ClusterHoldReport),
    /// Cycle rejected entirely.
    Rejected(ClusterRejectionReport),
    /// Cycle compacted to trace only.
    Compacted(DissolutionReport),
    /// Matrix-boundary violation.
    MatrixBoundaryViolation(MatrixBoundaryReport),
    /// Determinism violation.
    DeterminismViolation(DeterminismReport),
}

/// Reference pipeline. Total: every fail-path lands in a
/// `CellSubstrateOutcome::*` or returns a `PhaseError`.
pub fn run_cell_substrate_cycle(
    rd: &PhaseMatrixRunDescriptorV3,
    input: &CellSubstrateInput,
) -> Result<CellSubstrateOutcome, PhaseError> {
    rd.validate()?;

    // Step 3 — pool.
    let mut pool = CellPool::empty(input.subnet.subnet_id.clone())?;
    let allowed = input.subnet.member_nodes.clone();
    for cell in &input.seed_cells {
        pool.insert_cell(cell.clone(), &allowed)?;
    }
    pool.refresh_aggregates();
    let pool = pool.with_id()?;

    // Step 5 — pulses.
    let mut pulses = Vec::with_capacity(pool.cells.len());
    for (i, cell) in pool.cells.iter().enumerate() {
        let pulse = ResonancePulse::from_cell(
            cell.cell_id.clone(),
            cell.trident.clone(),
            cell.trident.activation_potential(),
            (i as u32) % 8,
        )?;
        pulses.push(pulse);
    }
    pulses.sort();

    // Step 6 — cluster formation.
    let phase_alignment_score = max_fixed(
        &pulses
            .iter()
            .map(|p| p.activation_potential.clone())
            .collect::<Vec<_>>(),
    );
    let coherence_score = pool.pool_coherence.clone();
    let morphodynamic_potential = fixed_add(&pool.pool_coherence, &pool.pool_entropy);
    let proposed: Vec<Hash256> = pool.cells.iter().map(|c| c.cell_id.clone()).collect();
    let formation = ClusterFormationReport {
        report_id: Hash256::zero(),
        pool_id: pool.pool_id.clone(),
        proposed_member_cells: proposed.clone(),
        accepted_member_cells: proposed.clone(),
        rejected_member_cells: vec![],
        phase_alignment_score,
        coherence_score: coherence_score.clone(),
        morphodynamic_potential: morphodynamic_potential.clone(),
        purpose_hash: input.purpose_hash.clone(),
        trace_ready: true,
        passed: false,
        evidence_refs: vec![],
    }
    .with_id()?;
    let gate_outcome = ClusterFormationGateOutcome::evaluate(&formation, rd);
    let rd_hash = content_address(rd)?;
    if !gate_outcome.passed {
        let mut diag = Vec::new();
        if !gate_outcome.g_phase {
            diag.push("phase alignment below threshold".into());
        }
        if !gate_outcome.g_coherence {
            diag.push("cluster coherence below threshold".into());
        }
        if !gate_outcome.g_morpho {
            diag.push("morphodynamic potential below threshold".into());
        }
        if !gate_outcome.g_purpose {
            diag.push("cluster purpose missing".into());
        }
        if !gate_outcome.g_trace {
            diag.push("trace not ready".into());
        }
        let hold = ClusterHoldReport {
            hold_id: Hash256::zero(),
            rd_hash,
            failed_gate: "cluster_formation".into(),
            diagnostics: diag,
        }
        .with_id()?;
        return Ok(CellSubstrateOutcome::Hold(hold));
    }

    // Step 7 — cluster.
    let cluster_trident = TridentVector {
        semantic_density: pool.pool_coherence.clone(),
        structural_coherence: pool.pool_coherence.clone(),
        temporal_phase: Fixed::Rational { num: 1, den: 2 },
    };
    let mut cluster = ResonanceCluster {
        cluster_id: Hash256::zero(),
        parent_subnet_id: input.subnet.subnet_id.clone(),
        member_cells: proposed,
        cluster_trident,
        coherence_score,
        conflict_score: pool.pool_entropy.clone(),
        morphodynamic_potential: morphodynamic_potential.clone(),
        funnel_graph_hash: Hash256::zero(),
        convergence_field_hash: Hash256::zero(),
        lifecycle: ClusterLifecycle::Forming,
        trace_hash: Hash256::zero(),
        evidence_refs: vec![],
    }
    .with_id()?;

    // Step 8 — funnel graph.
    let funnel =
        FunnelGraph::minimal_for_cluster(cluster.cluster_id.clone(), &cluster.member_cells)?;
    cluster.funnel_graph_hash = funnel.graph_id.clone();

    // Step 9 — morphodynamic field.
    let field = MorphodynamicField::minimal(
        Some(cluster.cluster_id.clone()),
        cluster.coherence_score.clone(),
        cluster.morphodynamic_potential.clone(),
    )?;
    let allow_event = morphology_gate(&field, rd, true);
    let event = if allow_event {
        ClusterMorphologyEvent::Stabilize
    } else {
        ClusterMorphologyEvent::Decay
    };
    let morph_decision = MorphologyDecision {
        decision_id: Hash256::zero(),
        cluster_id: cluster.cluster_id.clone(),
        event,
        morphodynamic_potential: field.morphodynamic_potential.clone(),
        gate_report_hash: content_address(&field)?,
        allowed: allow_event,
        evidence_refs: vec![],
    }
    .with_id()?;
    cluster.lifecycle = if allow_event {
        ClusterLifecycle::Stabilized
    } else {
        ClusterLifecycle::Decaying
    };

    // Step 11 — convergence.
    let pulse_ids: Vec<Hash256> = pulses.iter().map(|p| p.pulse_id.clone()).collect();
    let convergence = ConvergenceField {
        field_id: Hash256::zero(),
        cluster_id: cluster.cluster_id.clone(),
        input_pulses: pulse_ids,
        memory_projection_hash: None,
        convergence_score: cluster.coherence_score.clone(),
        conflict_score: cluster.conflict_score.clone(),
        stabilized_intent_hash: None,
        evidence_refs: vec![],
    }
    .with_id()?;
    cluster.convergence_field_hash = convergence.field_id.clone();

    // Step 12-13 — tension-to-intent.
    let field_tension = field.morphodynamic_potential.clone();
    let intent_passed = intent_gate(
        &field_tension,
        &convergence.convergence_score,
        &convergence.conflict_score,
        true,
        rd,
    );
    let intent_candidate = if intent_passed {
        let intent = IntentCandidate {
            candidate_id: Hash256::zero(),
            source_cluster_id: cluster.cluster_id.clone(),
            purpose_hash: input.purpose_hash.clone(),
            intent_vector_hash: cluster.cluster_id.clone(),
            confidence: cluster.coherence_score.clone(),
            claim_refs: vec![],
            evidence_refs: vec![],
        }
        .with_id()?;
        Some(intent)
    } else {
        None
    };

    // Step 14 — cluster trace.
    let mut trace = ClusterTrace {
        trace_id: Hash256::zero(),
        cluster_id: cluster.cluster_id.clone(),
        formation_report_hash: formation.report_id.clone(),
        funnel_graph_hash: funnel.graph_id.clone(),
        convergence_field_hash: convergence.field_id.clone(),
        morphology_decision_hashes: vec![morph_decision.decision_id.clone()],
        intent_candidate_hashes: intent_candidate
            .as_ref()
            .map(|c| vec![c.candidate_id.clone()])
            .unwrap_or_default(),
        dissolution_report_hash: None,
        evidence_refs: vec![EvidenceRef {
            kind: "matrix.boundary".into(),
            address: rd.matrix_id.clone(),
            relation: "anchored_in".into(),
        }],
    }
    .with_id()?;
    cluster.trace_hash = trace.trace_id.clone();

    // Step 15 — dissolution (compact preserve trace).
    let dissolution = if input.logical_step >= rd.thresholds.max_working_state_retention_ticks
        || fixed_ge(
            &Fixed::Rational {
                num: input.logical_step as i128,
                den: 1,
            },
            &Fixed::Rational {
                num: rd.thresholds.max_working_state_retention_ticks as i128,
                den: 1,
            },
        ) {
        let report = DissolutionReport {
            report_id: Hash256::zero(),
            cluster_id: cluster.cluster_id.clone(),
            mode: DissolutionMode::CompactToTrace,
            working_state_before_hash: cluster.cluster_id.clone(),
            retained_trace_hash: trace.trace_id.clone(),
            retained_evidence_hashes: vec![rd.matrix_id.clone()],
            lifecycle_event_hashes: vec![morph_decision.decision_id.clone()],
            passed: dissolution_gate(true, true, true, true),
        }
        .with_id()?;
        report.validate_trace_preservation()?;
        trace.dissolution_report_hash = Some(report.report_id.clone());
        let trace_re = trace.clone().with_id()?;
        cluster.trace_hash = trace_re.trace_id.clone();
        let cluster = cluster.clone().with_id()?;
        let mut report_with = CellSubstrateCycleReport {
            report_id: Hash256::zero(),
            rd_hash,
            pool_hash: pool.pool_id.clone(),
            cluster: cluster.clone(),
            funnel,
            convergence,
            intent: intent_candidate,
            trace: trace_re,
            dissolution: Some(report.clone()),
        };
        report_with = report_with.with_id()?;
        return Ok(CellSubstrateOutcome::Completed(report_with));
    } else {
        None
    };

    // Re-id the cluster after the morphology + funnel + convergence
    // updates so its `cluster_id` reflects the final state.
    let cluster = cluster.with_id()?;
    let report = CellSubstrateCycleReport {
        report_id: Hash256::zero(),
        rd_hash,
        pool_hash: pool.pool_id.clone(),
        cluster,
        funnel,
        convergence,
        intent: intent_candidate,
        trace,
        dissolution,
    }
    .with_id()?;
    Ok(CellSubstrateOutcome::Completed(report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::run_descriptor::{
        CellSubstratePolicies, CellSubstrateThresholds, PhaseMatrixRunDescriptorV3,
    };
    use std::collections::{BTreeMap, BTreeSet};

    fn rd() -> PhaseMatrixRunDescriptorV3 {
        PhaseMatrixRunDescriptorV3 {
            run_id: "phase.cycle.test".into(),
            matrix_id: Hash256::zero(),
            parent_profile_hash: None,
            operator_versions: BTreeMap::new(),
            canonicalization_version: "phase-matrix-v0.3".into(),
            thresholds: CellSubstrateThresholds::permissive(),
            policies: CellSubstratePolicies::strict(),
            feature_flags: BTreeSet::new(),
            evidence_refs: vec![],
        }
    }

    #[test]
    fn cycle_produces_completed_outcome_under_permissive_thresholds() {
        let rd = rd();
        let input = CellSubstrateInput::synthetic(rd.matrix_id.clone(), 3, 3).unwrap();
        let outcome = run_cell_substrate_cycle(&rd, &input).unwrap();
        assert!(matches!(outcome, CellSubstrateOutcome::Completed(_)));
    }

    #[test]
    fn cycle_holds_when_morpho_threshold_too_strict() {
        let mut rd = rd();
        rd.thresholds.min_morphodynamic_potential = Fixed::Rational { num: 100, den: 1 };
        let input = CellSubstrateInput::synthetic(rd.matrix_id.clone(), 3, 3).unwrap();
        let outcome = run_cell_substrate_cycle(&rd, &input).unwrap();
        assert!(matches!(outcome, CellSubstrateOutcome::Hold(_)));
    }

    #[test]
    fn two_identical_cycles_replay_byte_identical() {
        let rd = rd();
        let input = CellSubstrateInput::synthetic(rd.matrix_id.clone(), 3, 3).unwrap();
        let a = run_cell_substrate_cycle(&rd, &input).unwrap();
        let b = run_cell_substrate_cycle(&rd, &input).unwrap();
        let ah = pse_traverse::canonical::canonical_bytes(&a).unwrap();
        let bh = pse_traverse::canonical::canonical_bytes(&b).unwrap();
        assert_eq!(ah, bh);
    }
}

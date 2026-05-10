//! Five fail-closed gates (§18) + the policy enums each gate consults.

use serde::{Deserialize, Serialize};

use super::primitives::{fixed_ge, fixed_le, Fixed};
use super::resonance_cluster::ClusterFormationReport;
use super::run_descriptor::PhaseMatrixRunDescriptorV3;

/// Cluster-formation policy (§7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ClusterFormationPolicy {
    /// Strict formation — all formation sub-conditions required.
    Strict,
    /// Permissive (used by sandbox tests only).
    Permissive,
}

/// Morphology policy (§7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MorphologyPolicy {
    /// Strict morphology gate.
    Strict,
    /// Stabilise-only (no Grow/Split/Fuse allowed).
    StabilizeOnly,
}

/// Dissolution policy (§7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DissolutionPolicy {
    /// Compact while preserving trace + evidence.
    CompactPreserveTrace,
    /// Archive full state (heavier).
    ArchiveFullState,
    /// Disable dissolution (for short-lived experiments).
    NoDissolution,
}

/// Cluster-trace policy (§7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ClusterTracePolicy {
    /// Trace every phase.
    AllPhases,
    /// Trace only gate decisions.
    GateDecisionsOnly,
}

/// Intent policy (§7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum IntentPolicy {
    /// Conservative — intent only on strong tension AND convergence.
    Conservative,
    /// Aggressive — intent on either tension OR convergence (used by
    /// sandbox tests only).
    Aggressive,
}

/// Outcome of the cluster-formation gate (§18.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterFormationGateOutcome {
    /// `G_phase` — phase alignment ≥ threshold.
    pub g_phase: bool,
    /// Cluster coherence ≥ threshold.
    pub g_coherence: bool,
    /// Morphodynamic potential ≥ threshold.
    pub g_morpho: bool,
    /// Cluster purpose declared (purpose_hash != zero).
    pub g_purpose: bool,
    /// Trace is ready.
    pub g_trace: bool,
    /// Conjunctive verdict (`g_phase ∧ g_coherence ∧ g_morpho ∧ g_purpose ∧ g_trace`).
    pub passed: bool,
}

impl ClusterFormationGateOutcome {
    /// Per §18.1: `G_cluster = G_phase ∧ score_coh ≥ θ_coh ∧ H ≥ θ_H ∧
    /// purpose_valid ∧ trace_ready`.
    pub fn evaluate(report: &ClusterFormationReport, rd: &PhaseMatrixRunDescriptorV3) -> Self {
        let g_phase = fixed_ge(
            &report.phase_alignment_score,
            &rd.thresholds.min_phase_alignment,
        );
        let g_coherence = fixed_ge(
            &report.coherence_score,
            &rd.thresholds.min_cluster_coherence,
        );
        let g_morpho = fixed_ge(
            &report.morphodynamic_potential,
            &rd.thresholds.min_morphodynamic_potential,
        );
        let g_purpose = report.purpose_hash != super::primitives::Hash256::zero();
        let g_trace = report.trace_ready;
        let passed = g_phase && g_coherence && g_morpho && g_purpose && g_trace;
        ClusterFormationGateOutcome {
            g_phase,
            g_coherence,
            g_morpho,
            g_purpose,
            g_trace,
            passed,
        }
    }
}

/// `G_morph` (§18.2).
pub fn morphology_gate(
    field: &super::morphodynamic_field::MorphodynamicField,
    rd: &PhaseMatrixRunDescriptorV3,
    boundary_safe: bool,
) -> bool {
    let endo_ok = fixed_ge(
        &field.endogeny_score,
        &rd.thresholds.min_morphodynamic_potential,
    );
    let exo_ok = fixed_ge(
        &field.exogeny_score,
        &rd.thresholds.min_morphodynamic_potential,
    );
    endo_ok && exo_ok && boundary_safe
}

/// `G_intent` (§18.3).
pub fn intent_gate(
    field_tension: &Fixed,
    convergence_score: &Fixed,
    conflict_score: &Fixed,
    trace_ready: bool,
    rd: &PhaseMatrixRunDescriptorV3,
) -> bool {
    let tension_ok = fixed_ge(field_tension, &rd.thresholds.min_intent_tension);
    let conv_ok = fixed_ge(convergence_score, &rd.thresholds.min_convergence_score);
    let conflict_ok = fixed_le(conflict_score, &rd.thresholds.max_cluster_conflict);
    tension_ok && conv_ok && conflict_ok && trace_ready
}

/// `G_dissolve` (§18.4).
pub fn dissolution_gate(
    working_state_eligible: bool,
    trace_persisted: bool,
    evidence_persisted: bool,
    gate_history_persisted: bool,
) -> bool {
    working_state_eligible && trace_persisted && evidence_persisted && gate_history_persisted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::primitives::Hash256;
    use crate::cell::run_descriptor::{
        CellSubstratePolicies, CellSubstrateThresholds, PhaseMatrixRunDescriptorV3,
    };
    use std::collections::{BTreeMap, BTreeSet};

    fn rd() -> PhaseMatrixRunDescriptorV3 {
        PhaseMatrixRunDescriptorV3 {
            run_id: "phase.gate.test".into(),
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
    fn cluster_gate_fails_closed_without_trace() {
        let mut purpose = [0u8; 32];
        purpose[0] = 1;
        let report = ClusterFormationReport {
            report_id: Hash256::zero(),
            pool_id: Hash256::zero(),
            proposed_member_cells: vec![],
            accepted_member_cells: vec![],
            rejected_member_cells: vec![],
            phase_alignment_score: Fixed::Rational { num: 1, den: 1 },
            coherence_score: Fixed::Rational { num: 1, den: 1 },
            morphodynamic_potential: Fixed::Rational { num: 1, den: 1 },
            purpose_hash: Hash256(purpose),
            trace_ready: false,
            passed: false,
            evidence_refs: vec![],
        };
        let outcome = ClusterFormationGateOutcome::evaluate(&report, &rd());
        assert!(!outcome.passed);
    }

    #[test]
    fn dissolution_gate_requires_all_terms() {
        assert!(!dissolution_gate(true, false, true, true));
        assert!(!dissolution_gate(true, true, false, true));
        assert!(!dissolution_gate(true, true, true, false));
        assert!(dissolution_gate(true, true, true, true));
    }

    #[test]
    fn intent_gate_requires_tension_and_convergence() {
        let one = Fixed::Rational { num: 1, den: 1 };
        let half = Fixed::Rational { num: 1, den: 2 };
        let zero = Fixed::Rational { num: 0, den: 1 };
        assert!(intent_gate(&one, &one, &zero, true, &rd()));
        assert!(!intent_gate(&one, &one, &zero, false, &rd())); // trace missing
                                                                // tighten intent threshold and recheck
        let mut tight = rd();
        tight.thresholds.min_intent_tension = Fixed::Rational { num: 2, den: 1 };
        assert!(!intent_gate(&one, &one, &zero, true, &tight));
        let _ = half;
    }
}

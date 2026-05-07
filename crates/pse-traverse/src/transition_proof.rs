//! Transition proofs and dynamic gate for the traversal dynamics layer.
//! GATE-01: fail-closed — if proof is None, always Hold.

use serde::{Deserialize, Serialize};

use crate::compressor::CompressorStats;
use crate::dynamic_state::{stable_id_of, BaseState, CanonicalNumber, DynamicError, Hash256};
use crate::field::FieldSignal;
use crate::field_cube::EvidenceRef;

// ───────────────────────────────────────────────────────────────────────────────
// TransitionProof
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionProof {
    pub proof_id: Hash256,
    pub tick: u64,
    pub prev_state_root: Hash256,
    pub next_state_root: Hash256,
    pub path_delta: CanonicalNumber,
    pub alignment: CanonicalNumber,
    pub energy_delta: CanonicalNumber,
    pub density_delta: CanonicalNumber,
    pub valid: bool,
    pub evidence_refs: Vec<EvidenceRef>,
}

impl TransitionProof {
    pub fn with_id(mut self) -> Result<Self, DynamicError> {
        let mut probe = self.clone();
        probe.proof_id = Hash256::zero();
        let id = stable_id_of(&probe)?;
        self.proof_id = id;
        Ok(self)
    }

    pub fn compute(
        tick: u64,
        prev_states: &[BaseState],
        next_states: &[BaseState],
        field_signal: &FieldSignal,
        stats: &CompressorStats,
    ) -> Result<Self, DynamicError> {
        // Sort states for determinism
        let mut prev_sorted = prev_states.to_vec();
        let mut next_sorted = next_states.to_vec();
        prev_sorted.sort_by_key(|s| s.state_id.clone());
        next_sorted.sort_by_key(|s| s.state_id.clone());

        let prev_state_root = stable_id_of(&prev_sorted)?;
        let next_state_root = stable_id_of(&next_sorted)?;

        // path_delta = mean_i(||s_{i,t+1} - s_{i,t}||) quantized
        let path_delta_raw = if prev_sorted.is_empty() || next_sorted.is_empty() {
            0.0
        } else {
            let n = prev_sorted.len().min(next_sorted.len());
            let sum: f64 = prev_sorted
                .iter()
                .zip(next_sorted.iter())
                .take(n)
                .map(|(p, q)| {
                    let mut sq = 0.0;
                    for (a, b) in p.coords.iter().zip(q.coords.iter()) {
                        let d = a.to_f64() - b.to_f64();
                        sq += d * d;
                    }
                    sq.sqrt()
                })
                .sum();
            sum / n as f64
        };
        let path_delta = CanonicalNumber::quantize_default(path_delta_raw)?;

        // energy_delta = E(next) - E(prev), where E(S) = mean(||s||^2)
        let energy_prev = mean_energy(&prev_sorted);
        let energy_next = mean_energy(&next_sorted);
        let energy_delta_raw = energy_next - energy_prev;
        let energy_delta = CanonicalNumber::quantize_default(energy_delta_raw)?;

        // density_delta = from compressor stats (we don't have prev density here, use current)
        let density_delta = stats.density.clone();

        let valid = path_delta_raw.is_finite()
            && field_signal.alignment.to_f64().is_finite()
            && energy_delta_raw.is_finite();

        let proof = TransitionProof {
            proof_id: Hash256::zero(),
            tick,
            prev_state_root,
            next_state_root,
            path_delta,
            alignment: field_signal.alignment.clone(),
            energy_delta,
            density_delta,
            valid,
            evidence_refs: vec![],
        };
        proof.with_id()
    }
}

fn mean_energy(states: &[BaseState]) -> f64 {
    if states.is_empty() {
        return 0.0;
    }
    let sum: f64 = states
        .iter()
        .map(|s| {
            s.coords
                .iter()
                .map(|c| c.to_f64() * c.to_f64())
                .sum::<f64>()
        })
        .sum();
    sum / states.len() as f64
}

// ───────────────────────────────────────────────────────────────────────────────
// DynamicGateDecision
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DynamicGateDecision {
    Fire,
    Hold,
}

// ───────────────────────────────────────────────────────────────────────────────
// GateReason
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateReason {
    pub code: String,
    pub message: String,
}

// ───────────────────────────────────────────────────────────────────────────────
// DynamicGateConfig
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicGateConfig {
    pub config_id: Hash256,
    pub max_path_delta: CanonicalNumber,
    pub min_alignment: CanonicalNumber,
    pub max_density_delta: Option<CanonicalNumber>,
    pub require_energy_decrease: bool,
}

impl DynamicGateConfig {
    pub fn with_id(mut self) -> Result<Self, DynamicError> {
        let mut probe = self.clone();
        probe.config_id = Hash256::zero();
        let id = stable_id_of(&probe)?;
        self.config_id = id;
        Ok(self)
    }
}

impl Default for DynamicGateConfig {
    fn default() -> Self {
        let mut c = DynamicGateConfig {
            config_id: Hash256::zero(),
            max_path_delta: CanonicalNumber::quantize_default(1.0).unwrap(),
            min_alignment: CanonicalNumber::quantize_default(0.0).unwrap(),
            max_density_delta: None,
            require_energy_decrease: false,
        };
        let mut probe = c.clone();
        probe.config_id = Hash256::zero();
        if let Ok(id) = stable_id_of(&probe) {
            c.config_id = id;
        }
        c
    }
}

// ───────────────────────────────────────────────────────────────────────────────
// DynamicGateReport
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicGateReport {
    pub report_id: Hash256,
    pub proof_id: Hash256,
    pub decision: DynamicGateDecision,
    pub passed: bool,
    pub reasons: Vec<GateReason>,
}

impl DynamicGateReport {
    pub fn with_id(mut self) -> Result<Self, DynamicError> {
        let mut probe = self.clone();
        probe.report_id = Hash256::zero();
        let id = stable_id_of(&probe)?;
        self.report_id = id;
        Ok(self)
    }
}

// ───────────────────────────────────────────────────────────────────────────────
// evaluate_dynamic_gate
// GATE-01: fail-closed — if proof is None, always Hold
// ───────────────────────────────────────────────────────────────────────────────

pub fn evaluate_dynamic_gate(
    proof: Option<&TransitionProof>,
    config: &DynamicGateConfig,
) -> Result<DynamicGateReport, DynamicError> {
    // GATE-01: None proof → always Hold
    let Some(proof) = proof else {
        let report = DynamicGateReport {
            report_id: Hash256::zero(),
            proof_id: Hash256::zero(),
            decision: DynamicGateDecision::Hold,
            passed: false,
            reasons: vec![GateReason {
                code: "no_proof".to_string(),
                message: "No transition proof provided; gate is fail-closed".to_string(),
            }],
        };
        return report.with_id();
    };

    let mut reasons: Vec<GateReason> = Vec::new();
    let mut passed = true;

    // valid check
    if !proof.valid {
        passed = false;
        reasons.push(GateReason {
            code: "proof_invalid".to_string(),
            message: "Transition proof is marked invalid".to_string(),
        });
    }

    // path_delta ≤ max_path_delta
    if proof.path_delta > config.max_path_delta {
        passed = false;
        reasons.push(GateReason {
            code: "path_delta_too_high".to_string(),
            message: format!(
                "path_delta {} exceeds max {}",
                proof.path_delta.to_f64(),
                config.max_path_delta.to_f64()
            ),
        });
    }

    // alignment ≥ min_alignment
    if proof.alignment < config.min_alignment {
        passed = false;
        reasons.push(GateReason {
            code: "alignment_too_low".to_string(),
            message: format!(
                "alignment {} below min {}",
                proof.alignment.to_f64(),
                config.min_alignment.to_f64()
            ),
        });
    }

    // energy_delta < 0 OR !require_energy_decrease
    if config.require_energy_decrease && proof.energy_delta.to_f64() >= 0.0 {
        passed = false;
        reasons.push(GateReason {
            code: "energy_not_decreasing".to_string(),
            message: format!(
                "energy_delta {} is non-negative but require_energy_decrease=true",
                proof.energy_delta.to_f64()
            ),
        });
    }

    // max_density_delta check
    if let Some(ref max_dd) = config.max_density_delta {
        if proof.density_delta.abs() > *max_dd {
            passed = false;
            reasons.push(GateReason {
                code: "density_delta_too_high".to_string(),
                message: format!(
                    "density_delta abs {} exceeds max {}",
                    proof.density_delta.abs().to_f64(),
                    max_dd.to_f64()
                ),
            });
        }
    }

    let decision = if passed {
        DynamicGateDecision::Fire
    } else {
        DynamicGateDecision::Hold
    };

    let report = DynamicGateReport {
        report_id: Hash256::zero(),
        proof_id: proof.proof_id.clone(),
        decision,
        passed,
        reasons,
    };
    report.with_id()
}

// ───────────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compressor::CompressorStats;
    use crate::dynamic_state::BaseState;
    use crate::field::FieldSignal;

    fn dummy_stats() -> CompressorStats {
        CompressorStats {
            stats_id: Hash256::zero(),
            nodes_created: 0,
            nodes_merged: 0,
            nodes_split: 0,
            edges_created: 0,
            edges_pruned: 0,
            density: CanonicalNumber::quantize_default(1.0).unwrap(),
            graph_before: Hash256::zero(),
            graph_after: Hash256::zero(),
        }
        .with_id()
        .unwrap()
    }

    fn dummy_signal() -> FieldSignal {
        FieldSignal {
            signal_id: Hash256::zero(),
            alignment: CanonicalNumber::quantize_default(0.5).unwrap(),
            dispersion: CanonicalNumber::zero(),
            pressure: CanonicalNumber::quantize_default(0.5).unwrap(),
            state_count: 1,
        }
        .with_id()
        .unwrap()
    }

    #[test]
    fn dynamic_gate_hold_on_invalid_proof() {
        // GATE-01: None proof → always Hold
        let config = DynamicGateConfig::default();
        let report = evaluate_dynamic_gate(None, &config).unwrap();
        assert!(!report.passed);
        assert_eq!(report.decision, DynamicGateDecision::Hold);
        assert!(!report.reasons.is_empty());
        assert_eq!(report.reasons[0].code, "no_proof");
    }

    #[test]
    fn dynamic_gate_fires_on_valid_proof() {
        let config = DynamicGateConfig::default();
        let base = BaseState::zero(2).unwrap();
        let stats = dummy_stats();
        let signal = dummy_signal();
        let proof =
            TransitionProof::compute(1, &[base.clone()], &[base.clone()], &signal, &stats).unwrap();
        let report = evaluate_dynamic_gate(Some(&proof), &config).unwrap();
        // With zero path_delta, zero alignment vs 0.0 min, should fire
        assert!(report.passed);
        assert_eq!(report.decision, DynamicGateDecision::Fire);
    }

    #[test]
    fn transition_proof_detects_path_delta() {
        // If path_delta exceeds max, gate should Hold
        let stats = dummy_stats();
        let signal = dummy_signal();
        let prev = BaseState {
            state_id: Hash256::zero(),
            coords: vec![CanonicalNumber::quantize_default(0.0).unwrap()],
            weight: CanonicalNumber::zero(),
            evidence_refs: vec![],
        }
        .with_id()
        .unwrap();
        let next = BaseState {
            state_id: Hash256::zero(),
            coords: vec![CanonicalNumber::quantize_default(100.0).unwrap()],
            weight: CanonicalNumber::zero(),
            evidence_refs: vec![],
        }
        .with_id()
        .unwrap();

        let proof = TransitionProof::compute(1, &[prev], &[next], &signal, &stats).unwrap();
        let config = DynamicGateConfig::default(); // max_path_delta = 1.0
        let report = evaluate_dynamic_gate(Some(&proof), &config).unwrap();
        assert!(!report.passed);
        assert_eq!(report.decision, DynamicGateDecision::Hold);
        assert!(report
            .reasons
            .iter()
            .any(|r| r.code == "path_delta_too_high"));
    }
}

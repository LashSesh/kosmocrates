//! Dynamic tick execution for the traversal dynamics layer.
//! TICK-01: total — must never panic, must produce valid report even for empty input.

use serde::{Deserialize, Serialize};

use crate::dynamic_state::{
    BaseState, CanonicalNumber, DefaultStateLift, DynamicError, Hash256, LiftedState, StateLift,
    stable_id_of,
};
use crate::dynamic_policy::DynamicPolicy;
use crate::field::{DefaultFieldAbsorber, FieldAbsorber};
use crate::guidance_field::{DefaultGuidanceField, GuidanceField, GuidanceFieldState};
use crate::compressor::{
    CompressionGraph, DefaultMorphodynamicCompressor, MorphodynamicCompressor,
};
use crate::transition_proof::{DynamicGateConfig, TransitionProof, evaluate_dynamic_gate};
use crate::dynamic_report::{
    DynamicRunDescriptor, DynamicRunReport, DynamicStopCondition, DynamicTickReport,
};

// ───────────────────────────────────────────────────────────────────────────────
// DynamicTraversalState
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DynamicTraversalState {
    pub state_id: Hash256,
    pub tick: u64,
    pub base_states: Vec<BaseState>,
    pub guidance_field: GuidanceFieldState,
    pub compression_graph: CompressionGraph,
    pub policy: DynamicPolicy,
    pub descriptor: DynamicRunDescriptor,
}

impl DynamicTraversalState {
    pub fn with_id(mut self) -> Result<Self, DynamicError> {
        let mut probe = self.clone();
        probe.state_id = Hash256::zero();
        let id = stable_id_of(&probe)?;
        self.state_id = id;
        Ok(self)
    }

    pub fn init(
        base_states: Vec<BaseState>,
        policy: DynamicPolicy,
        descriptor: DynamicRunDescriptor,
    ) -> Result<Self, DynamicError> {
        let guidance_field = GuidanceFieldState::empty()?;
        let compression_graph = CompressionGraph::empty();
        let s = DynamicTraversalState {
            state_id: Hash256::zero(),
            tick: 0,
            base_states,
            guidance_field,
            compression_graph,
            policy,
            descriptor,
        };
        s.with_id()
    }
}

// ───────────────────────────────────────────────────────────────────────────────
// DynamicTickInput
// ───────────────────────────────────────────────────────────────────────────────

pub struct DynamicTickInput {
    pub tick: u64,
    pub base_states: Vec<BaseState>,
    pub policy: DynamicPolicy,
    pub gate_config: DynamicGateConfig,
    pub compute_proof: bool,
}

// ───────────────────────────────────────────────────────────────────────────────
// dynamic_tick
// TICK-01: total — must never panic
// ───────────────────────────────────────────────────────────────────────────────

pub fn dynamic_tick(
    input: &DynamicTickInput,
    dyn_state: &mut DynamicTraversalState,
) -> Result<DynamicTickReport, DynamicError> {
    // require: input.tick == dyn_state.tick + 1 OR dyn_state.tick == 0
    // (We allow either for robustness/TICK-01 totality)

    let prev_state_id = dyn_state.state_id.clone();

    // 1. Sort input.base_states by state_id (deterministic)
    let mut sorted_base: Vec<BaseState> = input.base_states.clone();
    sorted_base.sort_by_key(|s| s.state_id.clone());

    // 2. lifted_states = map DefaultStateLift.lift(base_state, input.tick)
    let lift = DefaultStateLift;
    let mut lifted_states: Vec<LiftedState> = Vec::new();
    for base in &sorted_base {
        match lift.lift(base, input.tick) {
            Ok(ls) => lifted_states.push(ls),
            Err(_) => {
                // TICK-01: be total, skip bad states
            }
        }
    }

    // 3. field_signal = DefaultFieldAbsorber.absorb(lifted_states, policy)
    let absorber = DefaultFieldAbsorber::default();
    let field_signal = absorber.absorb(&lifted_states, &input.policy)
        .unwrap_or_else(|_| {
            // TICK-01: neutral signal on error
            crate::field::FieldSignal {
                signal_id: Hash256::zero(),
                alignment: CanonicalNumber::zero(),
                dispersion: CanonicalNumber::zero(),
                pressure: CanonicalNumber::zero(),
                state_count: 0,
            }
            .with_id()
            .unwrap_or(crate::field::FieldSignal {
                signal_id: Hash256::zero(),
                alignment: CanonicalNumber::zero(),
                dispersion: CanonicalNumber::zero(),
                pressure: CanonicalNumber::zero(),
                state_count: 0,
            })
        });

    // 4 & 5. guidance field operations
    let mut guidance_field = DefaultGuidanceField::new();
    // Restore nodes/transitions from snapshot (we store snapshot in state)
    // Since DefaultGuidanceField doesn't expose node loading, we create fresh for MVP

    let guidance_relax = guidance_field
        .relax(&field_signal, &input.policy)
        .unwrap_or_else(|_| {
            crate::guidance_field::GuidanceRelaxReport {
                report_id: Hash256::zero(),
                field_before: Hash256::zero(),
                field_after: Hash256::zero(),
                nodes_updated: 0,
                transitions_updated: 0,
                transitions_pruned: 0,
            }
            .with_id()
            .unwrap_or(crate::guidance_field::GuidanceRelaxReport {
                report_id: Hash256::zero(),
                field_before: Hash256::zero(),
                field_after: Hash256::zero(),
                nodes_updated: 0,
                transitions_updated: 0,
                transitions_pruned: 0,
            })
        });

    // 6. gradients = map guidance_field.gradient(lifted_state) for each
    let mut gradient_coords_sum: Vec<f64> = Vec::new();
    let mut gradient_count = 0usize;

    for ls in &lifted_states {
        if let Ok(grad) = guidance_field.gradient(ls) {
            let base_dim = ls.lift_profile.base_dimension;
            let base_coords: Vec<f64> = grad.coords[..base_dim.min(grad.coords.len())].iter()
                .map(|c| c.to_f64())
                .collect();
            if gradient_coords_sum.is_empty() {
                gradient_coords_sum = base_coords;
            } else {
                for (i, v) in base_coords.iter().enumerate() {
                    if i < gradient_coords_sum.len() {
                        gradient_coords_sum[i] += v;
                    }
                }
            }
            gradient_count += 1;
        }
    }

    // guidance_base = aggregate gradients: mean of all gradient coords projected back to BaseState
    let dims = if gradient_coords_sum.is_empty() {
        sorted_base.first().map(|s| s.coords.len()).unwrap_or(0)
    } else {
        gradient_coords_sum.len()
    };

    let guidance_base_coords: Result<Vec<CanonicalNumber>, DynamicError> = (0..dims).map(|i| {
        let v = if gradient_count > 0 && i < gradient_coords_sum.len() {
            gradient_coords_sum[i] / gradient_count as f64
        } else {
            0.0
        };
        CanonicalNumber::quantize_default(v)
    }).collect();
    let guidance_base_coords = guidance_base_coords.unwrap_or_else(|_| vec![CanonicalNumber::zero(); dims]);

    let guidance_base = BaseState {
        state_id: Hash256::zero(),
        coords: guidance_base_coords,
        weight: CanonicalNumber::zero(),
        evidence_refs: vec![],
    };
    let guidance_base = guidance_base.with_id().unwrap_or_else(|_| BaseState {
        state_id: Hash256::zero(),
        coords: vec![],
        weight: CanonicalNumber::zero(),
        evidence_refs: vec![],
    });

    // 7. Add each lifted_state to compressor
    let mut compressor = DefaultMorphodynamicCompressor::new();
    for ls in lifted_states.iter() {
        compressor.add_state(ls.clone()).ok();
    }

    // 8. compressor_stats = compressor.advect(guidance_base, policy, tick)
    let compressor_stats = compressor
        .advect(&guidance_base, &input.policy, input.tick)
        .unwrap_or_else(|_| {
            crate::compressor::CompressorStats {
                stats_id: Hash256::zero(),
                nodes_created: 0,
                nodes_merged: 0,
                nodes_split: 0,
                edges_created: 0,
                edges_pruned: 0,
                density: CanonicalNumber::zero(),
                graph_before: Hash256::zero(),
                graph_after: Hash256::zero(),
            }
            .with_id()
            .unwrap_or(crate::compressor::CompressorStats {
                stats_id: Hash256::zero(),
                nodes_created: 0,
                nodes_merged: 0,
                nodes_split: 0,
                edges_created: 0,
                edges_pruned: 0,
                density: CanonicalNumber::zero(),
                graph_before: Hash256::zero(),
                graph_after: Hash256::zero(),
            })
        });

    // 9. next_states = project compressor nodes by highest mass
    let graph_snap = compressor.snapshot();
    let mut nodes_by_mass: Vec<&crate::compressor::CompressionNode> =
        graph_snap.nodes.values().collect();
    nodes_by_mass.sort_by(|a, b| b.mass.cmp(&a.mass));

    let take_n = if sorted_base.is_empty() {
        graph_snap.nodes.len()
    } else {
        sorted_base.len().max(1)
    };

    let mut next_states: Vec<BaseState> = Vec::new();
    for node in nodes_by_mass.iter().take(take_n) {
        let projected = lift.project(&node.state).unwrap_or_else(|_| BaseState {
            state_id: Hash256::zero(),
            coords: vec![],
            weight: CanonicalNumber::zero(),
            evidence_refs: vec![],
        });
        next_states.push(projected);
    }

    // Sort next_states by state_id for determinism
    next_states.sort_by_key(|s| s.state_id.clone());

    // 10. proof
    let proof = if input.compute_proof {
        TransitionProof::compute(
            input.tick,
            &sorted_base,
            &next_states,
            &field_signal,
            &compressor_stats,
        ).ok()
    } else {
        None
    };

    // 11. gate_report
    let gate_report = evaluate_dynamic_gate(proof.as_ref(), &input.gate_config)
        .unwrap_or_else(|_| {
            crate::transition_proof::DynamicGateReport {
                report_id: Hash256::zero(),
                proof_id: Hash256::zero(),
                decision: crate::transition_proof::DynamicGateDecision::Hold,
                passed: false,
                reasons: vec![crate::transition_proof::GateReason {
                    code: "gate_error".to_string(),
                    message: "Gate evaluation failed".to_string(),
                }],
            }
            .with_id()
            .unwrap_or(crate::transition_proof::DynamicGateReport {
                report_id: Hash256::zero(),
                proof_id: Hash256::zero(),
                decision: crate::transition_proof::DynamicGateDecision::Hold,
                passed: false,
                reasons: vec![],
            })
        });

    // 12. next_state_id
    let next_state_id = stable_id_of(&next_states)
        .unwrap_or(Hash256::zero());

    // 13. Update dyn_state
    dyn_state.tick = input.tick;
    dyn_state.base_states = next_states.clone();
    dyn_state.guidance_field = guidance_field.snapshot();
    dyn_state.compression_graph = compressor.snapshot();
    // Recompute state_id
    let new_state_id = stable_id_of(&(dyn_state.tick, &dyn_state.base_states))
        .unwrap_or(Hash256::zero());
    dyn_state.state_id = new_state_id;

    // 14. Return DynamicTickReport
    let report = DynamicTickReport {
        report_id: Hash256::zero(),
        tick: input.tick,
        prev_state_id,
        next_state_id,
        field_signal,
        guidance_relax,
        compressor_stats,
        transition_proof: proof,
        gate_report,
        next_states,
    };
    report.with_id()
}

// ───────────────────────────────────────────────────────────────────────────────
// dynamic_run
// ───────────────────────────────────────────────────────────────────────────────

pub fn dynamic_run(
    descriptor: &DynamicRunDescriptor,
    initial_states: Vec<BaseState>,
    policy: DynamicPolicy,
) -> Result<DynamicRunReport, DynamicError> {
    let mut dyn_state = DynamicTraversalState::init(
        initial_states,
        policy.clone(),
        descriptor.clone(),
    )?;

    let mut tick_reports: Vec<DynamicTickReport> = Vec::new();
    let max_ticks = descriptor.max_ticks;

    let mut fire_count: u64 = 0;
    let mut hold_count: u64 = 0;
    let mut stable_density_count: u64 = 0;
    let mut prev_density: Option<CanonicalNumber> = None;
    let mut stable_root_count: u64 = 0;
    let mut prev_state_root: Option<Hash256> = None;

    for tick in 1..=max_ticks {
        let input = DynamicTickInput {
            tick,
            base_states: dyn_state.base_states.clone(),
            policy: dyn_state.policy.clone(),
            gate_config: descriptor.gate_config.clone(),
            compute_proof: true,
        };
        let report = dynamic_tick(&input, &mut dyn_state)?;

        // Track gate decisions for stop conditions
        match &report.gate_report.decision {
            crate::transition_proof::DynamicGateDecision::Fire => fire_count += 1,
            crate::transition_proof::DynamicGateDecision::Hold => hold_count += 1,
        }

        // Check stop conditions
        let mut should_stop = false;
        for cond in &descriptor.stop_conditions {
            match cond {
                DynamicStopCondition::MaxTicks(n) => {
                    if tick >= *n { should_stop = true; }
                }
                DynamicStopCondition::GateFireCount { min_count } => {
                    if fire_count >= *min_count { should_stop = true; }
                }
                DynamicStopCondition::StabilizedDensity { tolerance, consecutive_ticks } => {
                    let cur = report.compressor_stats.density.clone();
                    if let Some(ref prev) = prev_density {
                        let diff = (cur.to_f64() - prev.to_f64()).abs();
                        if diff <= tolerance.to_f64() {
                            stable_density_count += 1;
                            if stable_density_count >= *consecutive_ticks { should_stop = true; }
                        } else {
                            stable_density_count = 0;
                        }
                    }
                    prev_density = Some(cur);
                }
                DynamicStopCondition::StabilizedStateRoot { consecutive_ticks } => {
                    let cur = report.next_state_id.clone();
                    if let Some(ref prev) = prev_state_root {
                        if &cur == prev {
                            stable_root_count += 1;
                            if stable_root_count >= *consecutive_ticks { should_stop = true; }
                        } else {
                            stable_root_count = 0;
                        }
                    }
                    prev_state_root = Some(cur);
                }
                DynamicStopCondition::MaxHoldCount { max_count } => {
                    if hold_count >= *max_count { should_stop = true; }
                }
            }
        }

        tick_reports.push(report);

        if should_stop { break; }
    }

    let final_state_id = dyn_state.state_id.clone();
    let final_gate_decision = tick_reports
        .last()
        .map(|r| r.gate_report.decision.clone())
        .unwrap_or(crate::transition_proof::DynamicGateDecision::Hold);

    let run = DynamicRunReport {
        run_id: Hash256::zero(),
        descriptor: descriptor.clone(),
        ticks: tick_reports,
        final_state_id,
        final_gate_decision,
        replay_address: Hash256::zero(),
    };
    run.with_id()
}

// ───────────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamic_state::{BaseState, CanonicalNumber};
    use crate::dynamic_policy::DynamicPolicy;
    use crate::transition_proof::{DynamicGateConfig, DynamicGateDecision};

    #[test]
    fn dynamic_tick_empty_input_total() {
        // TICK-01: empty base_states → valid Hold report without panic
        let policy = DynamicPolicy::default();
        let gate_config = DynamicGateConfig::default();
        let descriptor = DynamicRunDescriptor::default();

        let mut dyn_state = DynamicTraversalState::init(vec![], policy.clone(), descriptor).unwrap();

        let input = DynamicTickInput {
            tick: 1,
            base_states: vec![],
            policy,
            gate_config,
            compute_proof: false,
        };

        let report = dynamic_tick(&input, &mut dyn_state);
        // Must not panic, must produce Ok
        assert!(report.is_ok(), "dynamic_tick should succeed on empty input");
        let r = report.unwrap();
        // Should Hold (no proof, fail-closed)
        assert_eq!(r.gate_report.decision, DynamicGateDecision::Hold);
        assert!(!r.gate_report.passed);
        assert_eq!(r.field_signal.state_count, 0);
    }

    #[test]
    fn dynamic_tick_with_states_produces_report() {
        let policy = DynamicPolicy::default();
        let gate_config = DynamicGateConfig::default();
        let descriptor = DynamicRunDescriptor::default();

        let base = BaseState {
            state_id: Hash256::zero(),
            coords: vec![
                CanonicalNumber::quantize_default(1.0).unwrap(),
                CanonicalNumber::quantize_default(2.0).unwrap(),
            ],
            weight: CanonicalNumber::quantize_default(1.0).unwrap(),
            evidence_refs: vec![],
        };
        let base = base.with_id().unwrap();

        let mut dyn_state = DynamicTraversalState::init(vec![base.clone()], policy.clone(), descriptor).unwrap();

        let input = DynamicTickInput {
            tick: 1,
            base_states: vec![base],
            policy,
            gate_config,
            compute_proof: true,
        };

        let report = dynamic_tick(&input, &mut dyn_state).unwrap();
        assert_eq!(report.tick, 1);
        assert!(report.transition_proof.is_some());
        // state_count should be 1
        assert_eq!(report.field_signal.state_count, 1);
    }

    #[test]
    fn dynamic_run_runs_and_reports() {
        let descriptor = DynamicRunDescriptor {
            max_ticks: 3,
            stop_conditions: vec![DynamicStopCondition::MaxTicks(3)],
            ..DynamicRunDescriptor::default()
        };
        let policy = DynamicPolicy::default();
        let base = BaseState::zero(2).unwrap();
        let report = dynamic_run(&descriptor, vec![base], policy).unwrap();
        assert!(!report.ticks.is_empty());
        assert!(report.ticks.len() <= 3);
    }
}

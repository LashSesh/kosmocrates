//! Guidance field for the traversal dynamics layer.
//! GUIDE-01: for empty fields, return zero-gradient with same dimension as position.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::dynamic_policy::DynamicPolicy;
use crate::dynamic_state::{
    stable_id_of, CanonicalNumber, DynamicError, Hash256, LiftedState, StableId,
};
use crate::field::FieldSignal;

// ───────────────────────────────────────────────────────────────────────────────
// GuidanceNode
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuidanceNode {
    pub id: StableId,
    pub state: LiftedState,
    pub resonance: CanonicalNumber,
    pub mass: CanonicalNumber,
}

// ───────────────────────────────────────────────────────────────────────────────
// GuidanceTransition
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GuidanceTransition {
    pub from: StableId,
    pub to: StableId,
    pub phase_delta: CanonicalNumber,
    pub coherence: CanonicalNumber,
    pub weight: CanonicalNumber,
}

// ───────────────────────────────────────────────────────────────────────────────
// GuidanceFieldState
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuidanceFieldState {
    pub nodes: BTreeMap<StableId, GuidanceNode>,
    pub transitions: Vec<GuidanceTransition>,
    pub field_id: Hash256,
}

impl GuidanceFieldState {
    pub fn with_id(mut self) -> Result<Self, DynamicError> {
        let mut probe = self.clone();
        probe.field_id = Hash256::zero();
        let id = stable_id_of(&probe)?;
        self.field_id = id;
        Ok(self)
    }

    pub fn empty() -> Result<Self, DynamicError> {
        let s = GuidanceFieldState {
            nodes: BTreeMap::new(),
            transitions: vec![],
            field_id: Hash256::zero(),
        };
        s.with_id()
    }
}

// ───────────────────────────────────────────────────────────────────────────────
// GuidanceRelaxReport
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuidanceRelaxReport {
    pub report_id: Hash256,
    pub field_before: Hash256,
    pub field_after: Hash256,
    pub nodes_updated: usize,
    pub transitions_updated: usize,
    pub transitions_pruned: usize,
}

impl GuidanceRelaxReport {
    pub fn with_id(mut self) -> Result<Self, DynamicError> {
        let mut probe = self.clone();
        probe.report_id = Hash256::zero();
        let id = stable_id_of(&probe)?;
        self.report_id = id;
        Ok(self)
    }
}

// ───────────────────────────────────────────────────────────────────────────────
// GuidanceField trait
// ───────────────────────────────────────────────────────────────────────────────

pub trait GuidanceField {
    fn relax(
        &mut self,
        signal: &FieldSignal,
        policy: &DynamicPolicy,
    ) -> Result<GuidanceRelaxReport, DynamicError>;

    fn gradient(&self, position: &LiftedState) -> Result<LiftedState, DynamicError>;

    fn snapshot(&self) -> GuidanceFieldState;
}

// ───────────────────────────────────────────────────────────────────────────────
// DefaultGuidanceField
// ───────────────────────────────────────────────────────────────────────────────

pub struct DefaultGuidanceField {
    nodes: BTreeMap<StableId, GuidanceNode>,
    transitions: Vec<GuidanceTransition>,
}

impl DefaultGuidanceField {
    pub fn new() -> Self {
        DefaultGuidanceField {
            nodes: BTreeMap::new(),
            transitions: vec![],
        }
    }
}

impl Default for DefaultGuidanceField {
    fn default() -> Self {
        Self::new()
    }
}

impl GuidanceField for DefaultGuidanceField {
    fn relax(
        &mut self,
        signal: &FieldSignal,
        policy: &DynamicPolicy,
    ) -> Result<GuidanceRelaxReport, DynamicError> {
        // 1. Record field_before
        let snap_before = self.snapshot();
        let field_before = snap_before.field_id;

        let nodes_count = self.nodes.len();

        // 2. Update each node's resonance from signal.alignment (quantized)
        for node in self.nodes.values_mut() {
            node.resonance = signal.alignment.clone();
        }

        // 3. Update transitions: phase_delta and coherence
        let prune_threshold = policy.prune_threshold.to_f64();
        let dispersion = signal.dispersion.to_f64();
        let mut transitions_updated = 0usize;
        let mut transitions_pruned = 0usize;

        let mut new_transitions: Vec<GuidanceTransition> = Vec::new();
        for t in self.transitions.iter() {
            // phase_delta from |aux_from - aux_to|
            let aux_from = self
                .nodes
                .get(&t.from)
                .map(|n| n.state.auxiliary.to_f64())
                .unwrap_or(0.0);
            let aux_to = self
                .nodes
                .get(&t.to)
                .map(|n| n.state.auxiliary.to_f64())
                .unwrap_or(0.0);
            let phase_delta_raw = (aux_from - aux_to).abs();
            let phase_delta = CanonicalNumber::quantize_default(phase_delta_raw)?;

            // coherence decreases with high phase_delta or high dispersion
            let coherence_raw = (1.0 / (1.0 + phase_delta_raw + dispersion)).max(0.0);
            let coherence = CanonicalNumber::quantize_default(coherence_raw)?;

            let weight = t.weight.clone();
            if weight.to_f64() < prune_threshold {
                transitions_pruned += 1;
                continue;
            }
            transitions_updated += 1;
            new_transitions.push(GuidanceTransition {
                from: t.from.clone(),
                to: t.to.clone(),
                phase_delta,
                coherence,
                weight,
            });
        }

        // 4 & 5. Sort transitions by (from, to, weight)
        new_transitions.sort();
        self.transitions = new_transitions;

        // 6. Recompute field_id
        let snap_after = self.snapshot();
        let field_after = snap_after.field_id;

        // 7. Return report
        let report = GuidanceRelaxReport {
            report_id: Hash256::zero(),
            field_before,
            field_after,
            nodes_updated: nodes_count,
            transitions_updated,
            transitions_pruned,
        };
        report.with_id()
    }

    /// GUIDE-01: for empty fields, return zero-gradient with same dimension as position.
    fn gradient(&self, position: &LiftedState) -> Result<LiftedState, DynamicError> {
        if self.nodes.is_empty() {
            // Return clone of position with all coords = zero
            let zero_coords = vec![CanonicalNumber::zero(); position.coords.len()];
            let grad = LiftedState {
                state_id: Hash256::zero(),
                base_state_id: position.base_state_id.clone(),
                coords: zero_coords,
                auxiliary: CanonicalNumber::zero(),
                lift_profile: position.lift_profile.clone(),
            };
            return grad.with_id();
        }

        // Find nearest node by L2 distance on base coords
        let base_dim = position.lift_profile.base_dimension;
        let pos_base: Vec<f64> = position.coords[..base_dim.min(position.coords.len())]
            .iter()
            .map(|c| c.to_f64())
            .collect();

        let nearest = self.nodes.values().min_by(|a, b| {
            let da = l2_distance_base(&pos_base, &a.state, base_dim);
            let db = l2_distance_base(&pos_base, &b.state, base_dim);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        });

        match nearest {
            Some(node) => Ok(node.state.clone()),
            None => {
                // Empty, return zero
                let zero_coords = vec![CanonicalNumber::zero(); position.coords.len()];
                let grad = LiftedState {
                    state_id: Hash256::zero(),
                    base_state_id: position.base_state_id.clone(),
                    coords: zero_coords,
                    auxiliary: CanonicalNumber::zero(),
                    lift_profile: position.lift_profile.clone(),
                };
                grad.with_id()
            }
        }
    }

    fn snapshot(&self) -> GuidanceFieldState {
        let state = GuidanceFieldState {
            nodes: self.nodes.clone(),
            transitions: self.transitions.clone(),
            field_id: Hash256::zero(),
        };
        state.with_id().unwrap_or(GuidanceFieldState {
            nodes: BTreeMap::new(),
            transitions: vec![],
            field_id: Hash256::zero(),
        })
    }
}

fn l2_distance_base(pos: &[f64], state: &LiftedState, base_dim: usize) -> f64 {
    let mut sum = 0.0;
    for i in 0..base_dim {
        let p = pos.get(i).copied().unwrap_or(0.0);
        let s = if i < state.coords.len() {
            state.coords[i].to_f64()
        } else {
            0.0
        };
        sum += (p - s) * (p - s);
    }
    sum.sqrt()
}

// ───────────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamic_policy::DynamicPolicy;
    use crate::dynamic_state::{BaseState, DefaultStateLift, StateLift};

    fn make_lifted(tick: u64, coords: Vec<f64>) -> LiftedState {
        let base = BaseState {
            state_id: Hash256::zero(),
            coords: coords
                .iter()
                .map(|&v| CanonicalNumber::quantize_default(v).unwrap())
                .collect(),
            weight: CanonicalNumber::quantize_default(1.0).unwrap(),
            evidence_refs: vec![],
        };
        let base = base.with_id().unwrap();
        DefaultStateLift.lift(&base, tick).unwrap()
    }

    #[test]
    fn guidance_empty_gradient_zero() {
        let field = DefaultGuidanceField::new();
        let position = make_lifted(1, vec![1.0, 2.0]);
        let grad = field.gradient(&position).unwrap();
        // All coords should be zero
        for c in &grad.coords {
            assert_eq!(c.to_f64(), 0.0, "expected zero gradient coord");
        }
        // Same dimension
        assert_eq!(grad.coords.len(), position.coords.len());
    }

    #[test]
    fn guidance_relax_empty_field_succeeds() {
        let mut field = DefaultGuidanceField::new();
        let policy = DynamicPolicy::default();
        let signal = crate::field::FieldSignal {
            signal_id: Hash256::zero(),
            alignment: CanonicalNumber::zero(),
            dispersion: CanonicalNumber::zero(),
            pressure: CanonicalNumber::zero(),
            state_count: 0,
        };
        let report = field.relax(&signal, &policy).unwrap();
        assert_eq!(report.nodes_updated, 0);
        assert_eq!(report.transitions_pruned, 0);
    }
}

//! Plan delta types for the traversal dynamics layer.

use serde::{Deserialize, Serialize};

use crate::dynamic_state::{CanonicalNumber, DynamicError, Hash256, stable_id_of};

// ───────────────────────────────────────────────────────────────────────────────
// PlanDeltaOp
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanDeltaOp {
    ReorderStep {
        step_id: Hash256,
        before_index: usize,
        after_index: usize,
    },
    SplitStep {
        step_id: Hash256,
        new_steps: Vec<Hash256>,
    },
    MergeSteps {
        step_ids: Vec<Hash256>,
        merged_step: Hash256,
    },
    TightenGate {
        gate_key: String,
        old_value: CanonicalNumber,
        new_value: CanonicalNumber,
    },
    AddExcisionHint {
        target_id: Hash256,
        reason: String,
    },
}

// ───────────────────────────────────────────────────────────────────────────────
// PlanDelta
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanDelta {
    pub delta_id: Hash256,
    pub source_dynamic_report: Hash256,
    pub before_plan_id: Hash256,
    pub after_plan_id: Hash256,
    pub operations: Vec<PlanDeltaOp>,
    pub reasons: Vec<String>,
}

impl PlanDelta {
    pub fn with_id(mut self) -> Result<Self, DynamicError> {
        let mut probe = self.clone();
        probe.delta_id = Hash256::zero();
        let id = stable_id_of(&probe)?;
        self.delta_id = id;
        Ok(self)
    }
}

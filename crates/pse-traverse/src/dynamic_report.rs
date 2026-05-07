//! Dynamic run report types for the traversal dynamics layer.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::compressor::CompressorStats;
use crate::dynamic_policy::{DynamicPolicyKind, PolicyScheduleEntry, QuantizationPolicy};
use crate::dynamic_state::{stable_id_of, BaseState, CanonicalNumber, DynamicError, Hash256};
use crate::field::FieldSignal;
use crate::guidance_field::GuidanceRelaxReport;
use crate::transition_proof::{
    DynamicGateConfig, DynamicGateDecision, DynamicGateReport, TransitionProof,
};

// ───────────────────────────────────────────────────────────────────────────────
// SignatureDrivenPolicyHint
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignatureDrivenPolicyHint {
    pub source_diagnostics_id: Hash256,
    pub recommended_policy: DynamicPolicyKind,
    pub confidence: CanonicalNumber,
    pub reason: String,
}

// ───────────────────────────────────────────────────────────────────────────────
// DynamicStopCondition
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DynamicStopCondition {
    MaxTicks(u64),
    GateFireCount {
        min_count: u64,
    },
    StabilizedDensity {
        tolerance: CanonicalNumber,
        consecutive_ticks: u64,
    },
    StabilizedStateRoot {
        consecutive_ticks: u64,
    },
    MaxHoldCount {
        max_count: u64,
    },
}

// ───────────────────────────────────────────────────────────────────────────────
// DynamicRunDescriptor
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicRunDescriptor {
    pub descriptor_id: Hash256,
    pub schema_version: String,
    pub operator_versions: BTreeMap<String, String>,
    pub initial_state_root: Hash256,
    pub policy_schedule: Vec<PolicyScheduleEntry>,
    pub gate_config: DynamicGateConfig,
    pub quantization: QuantizationPolicy,
    pub max_ticks: u64,
    pub stop_conditions: Vec<DynamicStopCondition>,
}

impl DynamicRunDescriptor {
    pub fn with_id(mut self) -> Result<Self, DynamicError> {
        let mut probe = self.clone();
        probe.descriptor_id = Hash256::zero();
        let id = stable_id_of(&probe)?;
        self.descriptor_id = id;
        Ok(self)
    }
}

impl Default for DynamicRunDescriptor {
    fn default() -> Self {
        let mut d = DynamicRunDescriptor {
            descriptor_id: Hash256::zero(),
            schema_version: "pse-traverse-dynamics-01".to_string(),
            operator_versions: BTreeMap::new(),
            initial_state_root: Hash256::zero(),
            policy_schedule: vec![],
            gate_config: DynamicGateConfig::default(),
            quantization: QuantizationPolicy::default(),
            max_ticks: 100,
            stop_conditions: vec![DynamicStopCondition::MaxTicks(100)],
        };
        if let Ok(id) = {
            let mut probe = d.clone();
            probe.descriptor_id = Hash256::zero();
            stable_id_of(&probe)
        } {
            d.descriptor_id = id;
        }
        d
    }
}

// ───────────────────────────────────────────────────────────────────────────────
// DynamicTickReport
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicTickReport {
    pub report_id: Hash256,
    pub tick: u64,
    pub prev_state_id: Hash256,
    pub next_state_id: Hash256,
    pub field_signal: FieldSignal,
    pub guidance_relax: GuidanceRelaxReport,
    pub compressor_stats: CompressorStats,
    pub transition_proof: Option<TransitionProof>,
    pub gate_report: DynamicGateReport,
    pub next_states: Vec<BaseState>,
}

impl DynamicTickReport {
    pub fn with_id(mut self) -> Result<Self, DynamicError> {
        let mut probe = self.clone();
        probe.report_id = Hash256::zero();
        let id = stable_id_of(&probe)?;
        self.report_id = id;
        Ok(self)
    }
}

// ───────────────────────────────────────────────────────────────────────────────
// DynamicRunReport
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicRunReport {
    pub run_id: Hash256,
    pub descriptor: DynamicRunDescriptor,
    pub ticks: Vec<DynamicTickReport>,
    pub final_state_id: Hash256,
    pub final_gate_decision: DynamicGateDecision,
    pub replay_address: Hash256,
}

impl DynamicRunReport {
    pub fn with_id(mut self) -> Result<Self, DynamicError> {
        // replay_address = stable_id_of(&(descriptor_id, ticks_sorted_by_tick))
        let mut sorted_ticks = self.ticks.clone();
        sorted_ticks.sort_by_key(|t| t.tick);
        let replay_address = stable_id_of(&(&self.descriptor.descriptor_id, &sorted_ticks))?;
        self.replay_address = replay_address;

        let mut probe = self.clone();
        probe.run_id = Hash256::zero();
        let id = stable_id_of(&probe)?;
        self.run_id = id;
        Ok(self)
    }
}

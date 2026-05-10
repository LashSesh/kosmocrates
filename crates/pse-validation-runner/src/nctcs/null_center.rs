//! NullCenter, NullProjectionAudit and ToroidalPhaseFlowAudit.
//!
//! K0 is the pre-dynamic null-center: it MUST NOT be a phase state and
//! MUST NOT be an agent. The projection audit verifies that K0 ≠ π0(K0).

use serde::{Deserialize, Serialize};

use super::primitives::{content_address, EvidenceRef, Hash256, NctcsError};

/// Canonical reference to the NullCenter (K0).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NullCenterRef {
    pub null_center_id: Hash256,
    pub pre_dynamic_layer_id: Hash256,
    /// K0 is declared exogenous to the dynamic field.
    pub declared_exogenous: bool,
    /// K0 is not a phase state.
    pub not_phase_state: bool,
    /// K0 is not an agent.
    pub not_agent: bool,
    pub evidence_refs: Vec<EvidenceRef>,
}

impl NullCenterRef {
    /// Create a canonical null-center reference from a given id.
    pub fn new(null_center_id: Hash256, pre_dynamic_layer_id: Hash256) -> Self {
        NullCenterRef {
            null_center_id,
            pre_dynamic_layer_id,
            declared_exogenous: true,
            not_phase_state: true,
            not_agent: true,
            evidence_refs: vec![],
        }
    }

    /// C0 invariant: K0 must be exogenous, not a phase state, not an agent.
    pub fn c0_satisfied(&self) -> bool {
        self.declared_exogenous && self.not_phase_state && self.not_agent
    }

    pub fn content_hash(&self) -> Result<Hash256, NctcsError> {
        content_address(self)
    }
}

/// Audit that verifies K0 ≠ π0(K0): the projection of K0 into phase space
/// is strictly distinct from K0 itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NullProjectionAudit {
    pub audit_id: Hash256,
    pub null_center_id: Hash256,
    /// SHA-256 of the projected phase state.
    pub projected_phase_hash: Hash256,
    /// True when null_center_id ≠ projected_phase_hash.
    pub projection_distinction_passed: bool,
    pub phase_manifold_dim: u32,
    pub phase_manifold_hash: Hash256,
}

impl NullProjectionAudit {
    pub fn build(
        null_center_id: Hash256,
        projected_phase_hash: Hash256,
        phase_manifold_dim: u32,
        phase_manifold_hash: Hash256,
    ) -> Result<Self, NctcsError> {
        let projection_distinction_passed = null_center_id != projected_phase_hash;
        let partial = NullProjectionAudit {
            audit_id: Hash256::zero(),
            null_center_id,
            projected_phase_hash,
            projection_distinction_passed,
            phase_manifold_dim,
            phase_manifold_hash,
        };
        let id = content_address(&partial)?;
        Ok(NullProjectionAudit {
            audit_id: id,
            ..partial
        })
    }
}

/// Audit of the toroidal phase flow — verifies that phase flow produces
/// timing information only, never truth claims.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToroidalPhaseFlowAudit {
    pub audit_id: Hash256,
    pub phase_flow_hash: Hash256,
    pub tick: u64,
    pub theta_t_hash: Hash256,
    /// True when phase flow is used only for visibility timing.
    pub visibility_timing_only: bool,
}

impl ToroidalPhaseFlowAudit {
    pub fn build(
        phase_flow_hash: Hash256,
        tick: u64,
        theta_t_hash: Hash256,
        visibility_timing_only: bool,
    ) -> Result<Self, NctcsError> {
        let partial = ToroidalPhaseFlowAudit {
            audit_id: Hash256::zero(),
            phase_flow_hash,
            tick,
            theta_t_hash,
            visibility_timing_only,
        };
        let id = content_address(&partial)?;
        Ok(ToroidalPhaseFlowAudit {
            audit_id: id,
            ..partial
        })
    }
}

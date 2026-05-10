//! NctcsClosureBundle — the canonical output of a NCTCS closure run.

use serde::{Deserialize, Serialize};

use super::candidate_audit::CandidateFormationAudit;
use super::conformance::NctcsConformanceReport;
use super::macro_control_state::MacroControlState;
use super::materialization_audit::MaterializationAudit;
use super::null_center::{NullCenterRef, NullProjectionAudit, ToroidalPhaseFlowAudit};
use super::phase_visibility::PhaseVisibilityAudit;
use super::primitives::{content_address, Hash256, NctcsError};
use super::run_descriptor::NctcsRunDescriptor;
use super::trace_replay_contract::TraceReplayContractReport;

/// The canonical output bundle of one NCTCS closure run.
///
/// All fields are content-addressed. Replay reconstruction must produce
/// a byte-identical bundle_id.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NctcsClosureBundle {
    pub bundle_id: Hash256,
    pub rd: NctcsRunDescriptor,
    pub null_center: NullCenterRef,
    pub projection: NullProjectionAudit,
    pub phase_flow: ToroidalPhaseFlowAudit,
    pub visibility: PhaseVisibilityAudit,
    pub candidate_audit: CandidateFormationAudit,
    pub materialization: MaterializationAudit,
    pub trace_replay: TraceReplayContractReport,
    pub macro_state: Option<MacroControlState>,
    pub conformance: NctcsConformanceReport,
}

impl NctcsClosureBundle {
    pub fn build(
        rd: NctcsRunDescriptor,
        null_center: NullCenterRef,
        projection: NullProjectionAudit,
        phase_flow: ToroidalPhaseFlowAudit,
        visibility: PhaseVisibilityAudit,
        candidate_audit: CandidateFormationAudit,
        materialization: MaterializationAudit,
        trace_replay: TraceReplayContractReport,
        macro_state: Option<MacroControlState>,
        conformance: NctcsConformanceReport,
    ) -> Result<Self, NctcsError> {
        let partial = NctcsClosureBundle {
            bundle_id: Hash256::zero(),
            rd,
            null_center,
            projection,
            phase_flow,
            visibility,
            candidate_audit,
            materialization,
            trace_replay,
            macro_state,
            conformance,
        };
        let id = content_address(&partial)?;
        Ok(NctcsClosureBundle {
            bundle_id: id,
            ..partial
        })
    }

    pub fn content_hash(&self) -> Result<Hash256, NctcsError> {
        content_address(self)
    }
}

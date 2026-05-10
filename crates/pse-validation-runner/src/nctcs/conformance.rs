//! Conformance classes, gate taxonomy, and classification logic.

use serde::{Deserialize, Serialize};

use super::candidate_audit::CandidateFormationAudit;
use super::materialization_audit::MaterializationAudit;
use super::null_center::{NullCenterRef, NullProjectionAudit};
use super::phase_visibility::PhaseVisibilityAudit;
use super::primitives::{content_address, EvidenceRef, Hash256, NctcsError};
use super::trace_replay_contract::TraceReplayContractReport;

/// NCTCS conformance class (C0–C4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum NctcsConformanceClass {
    C0FormalTyped,
    C1PhaseGatedVisibility,
    C2GateBoundMaterialization,
    C3AuditableTensor,
    C4MacroControl,
}

impl NctcsConformanceClass {
    pub fn as_str(self) -> &'static str {
        match self {
            NctcsConformanceClass::C0FormalTyped => "C0_FormalTyped",
            NctcsConformanceClass::C1PhaseGatedVisibility => "C1_PhaseGatedVisibility",
            NctcsConformanceClass::C2GateBoundMaterialization => "C2_GateBoundMaterialization",
            NctcsConformanceClass::C3AuditableTensor => "C3_AuditableTensor",
            NctcsConformanceClass::C4MacroControl => "C4_MacroControl",
        }
    }

    pub fn numeric_score(self) -> f64 {
        match self {
            NctcsConformanceClass::C0FormalTyped => 0.25,
            NctcsConformanceClass::C1PhaseGatedVisibility => 0.40,
            NctcsConformanceClass::C2GateBoundMaterialization => 0.60,
            NctcsConformanceClass::C3AuditableTensor => 0.80,
            NctcsConformanceClass::C4MacroControl => 1.00,
        }
    }
}

/// Validation closure status — orthogonal to conformance class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ValidationClosureStatus {
    Invalid,
    InternalPass,
    StructuralPass,
    DomainReady,
    DomainValidated,
    DiagnosticFinding,
    EmpiricalImprovement,
}

impl ValidationClosureStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ValidationClosureStatus::Invalid => "INVALID",
            ValidationClosureStatus::InternalPass => "INTERNAL_PASS",
            ValidationClosureStatus::StructuralPass => "STRUCTURAL_PASS",
            ValidationClosureStatus::DomainReady => "DOMAIN_READY",
            ValidationClosureStatus::DomainValidated => "DOMAIN_VALIDATED",
            ValidationClosureStatus::DiagnosticFinding => "DIAGNOSTIC_FINDING",
            ValidationClosureStatus::EmpiricalImprovement => "EMPIRICAL_IMPROVEMENT",
        }
    }
}

/// Gate outcome taxonomy per §6.1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NctcsGateOutcome {
    Pass,
    Hold,
    Refine,
    Reject,
    Quarantine,
    NoUpdate,
    HandoffReady,
}

impl NctcsGateOutcome {
    /// Only Pass MAY produce a persistent tensor revision.
    pub fn is_materializing(self) -> bool {
        matches!(self, NctcsGateOutcome::Pass)
    }
}

/// A single conformance check with its outcome and evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConformanceCheck {
    pub class: NctcsConformanceClass,
    pub passed: bool,
    pub reason: String,
    pub evidence_refs: Vec<EvidenceRef>,
}

/// Reference to an open proof obligation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProofObligationRef {
    pub obligation_id: String,
    pub description: String,
    pub recovery: String,
}

/// Full NCTCS conformance report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NctcsConformanceReport {
    pub report_id: Hash256,
    pub rd_hash: Hash256,
    pub c0: ConformanceCheck,
    pub c1: ConformanceCheck,
    pub c2: ConformanceCheck,
    pub c3: ConformanceCheck,
    pub c4: ConformanceCheck,
    pub reached_class: NctcsConformanceClass,
    pub validation_status: ValidationClosureStatus,
    pub unresolved_obligations: Vec<ProofObligationRef>,
}

impl NctcsConformanceReport {
    pub fn content_hash(&self) -> Result<Hash256, NctcsError> {
        content_address(self)
    }
}

/// Classify the conformance class from audit results.
pub fn classify_conformance(
    null_center: &NullCenterRef,
    projection: &NullProjectionAudit,
    visibility: &PhaseVisibilityAudit,
    candidates: &CandidateFormationAudit,
    materialization: &MaterializationAudit,
    trace_replay: &TraceReplayContractReport,
    has_macro_control_state: bool,
    has_domain_validation: bool,
    rd_hash: Hash256,
) -> Result<NctcsConformanceReport, NctcsError> {
    let mut obligations = Vec::new();

    // C0: formal typed structure + K0 exogenous
    let c0_passed = null_center.c0_satisfied() && projection.projection_distinction_passed;
    if !c0_passed {
        obligations.push(ProofObligationRef {
            obligation_id: "C0-NULL-CENTER".into(),
            description: "NullCenter must be exogenous, not a phase state, not an agent".into(),
            recovery: "Declare K0 as exogenous in NctcsRunDescriptor.".into(),
        });
    }

    // C1: phase-gated visibility for candidates
    let c1_passed =
        c0_passed && visibility.passed && candidates.candidate_requires_visibility_passed;
    if c0_passed && !c1_passed {
        obligations.push(ProofObligationRef {
            obligation_id: "C1-PHASE-VISIBILITY".into(),
            description: "Candidate formation must imply phase-gated visibility".into(),
            recovery: "Ensure all candidates have a visible phase cell as prerequisite.".into(),
        });
    }

    // C2: gate-bound materialization
    let c2_passed =
        c1_passed && materialization.passed && materialization.no_direct_fabric_to_tensor_mutation;
    if c1_passed && !c2_passed {
        obligations.push(ProofObligationRef {
            obligation_id: "C2-GATE-BOUND-MATERIALIZATION".into(),
            description: "Ephemeral fabric must not directly mutate persistent tensor".into(),
            recovery: "Route all tensor updates through the gate cascade.".into(),
        });
    }

    // C3: auditable tensor (trace + replay)
    let c3_passed = c2_passed && trace_replay.passed;
    if c2_passed && !c3_passed {
        obligations.push(ProofObligationRef {
            obligation_id: "C3-AUDITABLE-TENSOR".into(),
            description:
                "All persistent artifacts must be trace-ready, evidence-ready and replay-ready"
                    .into(),
            recovery: "Ensure TraceReplayContractReport.passed = true with 100% replay identity."
                .into(),
        });
    }

    // C4: macro control state (requires C3 + MacroControlState derived from tensor history)
    let c4_passed = c3_passed && has_macro_control_state;
    if c3_passed && !c4_passed {
        obligations.push(ProofObligationRef {
            obligation_id: "C4-MACRO-CONTROL".into(),
            description: "MacroControlState must be derived from K0, FT, Trace, Policies and ValidationResult".into(),
            recovery: "Build MacroControlState from tensor history and eval summary.".into(),
        });
    }

    let reached_class = if c4_passed {
        NctcsConformanceClass::C4MacroControl
    } else if c3_passed {
        NctcsConformanceClass::C3AuditableTensor
    } else if c2_passed {
        NctcsConformanceClass::C2GateBoundMaterialization
    } else if c1_passed {
        NctcsConformanceClass::C1PhaseGatedVisibility
    } else {
        NctcsConformanceClass::C0FormalTyped
    };

    let validation_status =
        derive_validation_status(reached_class, has_domain_validation, c4_passed);

    let make_check = |class, passed: bool, reason: &str| ConformanceCheck {
        class,
        passed,
        reason: reason.into(),
        evidence_refs: vec![],
    };

    let partial = NctcsConformanceReport {
        report_id: Hash256::zero(),
        rd_hash,
        c0: make_check(
            NctcsConformanceClass::C0FormalTyped,
            c0_passed,
            "NullCenter exogenous and distinct from projection",
        ),
        c1: make_check(
            NctcsConformanceClass::C1PhaseGatedVisibility,
            c1_passed,
            "Phase-gated visibility for candidates",
        ),
        c2: make_check(
            NctcsConformanceClass::C2GateBoundMaterialization,
            c2_passed,
            "Gate-bound materialization enforced",
        ),
        c3: make_check(
            NctcsConformanceClass::C3AuditableTensor,
            c3_passed,
            "Trace-replay contract satisfied",
        ),
        c4: make_check(
            NctcsConformanceClass::C4MacroControl,
            c4_passed,
            "MacroControlState derived from tensor history",
        ),
        reached_class,
        validation_status,
        unresolved_obligations: obligations,
    };
    let id = content_address(&partial)?;
    Ok(NctcsConformanceReport {
        report_id: id,
        ..partial
    })
}

fn derive_validation_status(
    class: NctcsConformanceClass,
    has_domain_validation: bool,
    c4: bool,
) -> ValidationClosureStatus {
    if !c4 && class < NctcsConformanceClass::C2GateBoundMaterialization {
        return ValidationClosureStatus::Invalid;
    }
    if !has_domain_validation {
        return ValidationClosureStatus::StructuralPass;
    }
    ValidationClosureStatus::DomainValidated
}

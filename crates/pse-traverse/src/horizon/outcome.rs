//! Horizon outcomes: `Finalized`, `Hold`, `WaitForHorizon`, etc.
//!
//! Every fail-path lands here. The spec mandates fail-closed semantics:
//! any uncertainty in NullCenter / chart / carrier / ray / cone /
//! causal / duality / gate / replay forces a `Hold` or harder failure.

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::horizon_chart::HorizonChart;
use super::horizon_crossing::HorizonCrossingReport;
use super::primitives::HorizonError;
use super::run_descriptor::HorizonFailurePolicy;

/// Finalized v0.3 emission. Only emitted when the combined
/// (`G_v0.2 ∧ G_v0.3`) gate is satisfied.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinalizedEmissionV3 {
    pub emission_id: Hash256,
    pub chart_id: Hash256,
    pub crossing_report_hash: Hash256,
    pub projection_v2_certificate_hash: Option<Hash256>,
    pub canonical_payload_hash: Hash256,
}

impl FinalizedEmissionV3 {
    pub fn with_id(mut self) -> Result<Self, HorizonError> {
        let mut probe = self.clone();
        probe.emission_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| HorizonError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.emission_id = Hash256(id);
        Ok(self)
    }
}

/// Hold report: the chart-and-crossing pair that didn't make it past
/// the gate, plus the deterministic failure policy. Carries every hash
/// the operator needs to retry safely.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HorizonHoldReport {
    pub hold_id: Hash256,
    pub chart_id: Hash256,
    pub crossing_report_hash: Hash256,
    pub failure_policy: HorizonFailurePolicy,
    pub reasons: Vec<String>,
}

impl HorizonHoldReport {
    pub fn with_id(mut self) -> Result<Self, HorizonError> {
        let mut probe = self.clone();
        probe.hold_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| HorizonError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.hold_id = Hash256(id);
        Ok(self)
    }

    pub fn from_crossing(
        chart: &HorizonChart,
        crossing: &HorizonCrossingReport,
    ) -> Result<Self, HorizonError> {
        let mut reasons = Vec::new();
        if !crossing.cone_report.passed {
            reasons.push("projection cone failed".into());
        }
        if !crossing.causal_report.passed {
            reasons.push("causal admissibility failed".into());
        }
        if !crossing.duality_report.passed {
            reasons.push("collapse-emission duality failed".into());
        }
        let crossing_report_hash =
            Hash256(
                content_address(crossing).map_err(|e| HorizonError::Canonicalization {
                    reason: e.to_string(),
                })?,
            );
        let r = HorizonHoldReport {
            hold_id: Hash256::zero(),
            chart_id: chart.chart_id.clone(),
            crossing_report_hash,
            failure_policy: crossing.failure_policy,
            reasons,
        };
        r.with_id()
    }
}

/// Diagnostic carrier for invalid inputs / determinism violations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HorizonDiagnostic {
    pub diagnostic_id: Hash256,
    pub message: String,
    pub kind: HorizonDiagnosticKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HorizonDiagnosticKind {
    InvalidInput,
    DeterminismViolation,
    Schema,
}

impl HorizonDiagnostic {
    pub fn new(
        kind: HorizonDiagnosticKind,
        message: impl Into<String>,
    ) -> Result<Self, HorizonError> {
        let message = message.into();
        let mut probe = HorizonDiagnostic {
            diagnostic_id: Hash256::zero(),
            message: message.clone(),
            kind: kind.clone(),
        };
        let id = content_address(&probe).map_err(|e| HorizonError::Canonicalization {
            reason: e.to_string(),
        })?;
        probe.diagnostic_id = Hash256(id);
        Ok(probe)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum HorizonV3Outcome {
    Finalized {
        emission: FinalizedEmissionV3,
        hold: Option<HorizonHoldReport>,
    },
    Hold {
        hold: HorizonHoldReport,
    },
    WaitForHorizon {
        hold: HorizonHoldReport,
    },
    RefineCone {
        hold: HorizonHoldReport,
    },
    NeedsCarrierMigration {
        hold: HorizonHoldReport,
    },
    Recondense {
        hold: HorizonHoldReport,
    },
    ProjectionOnly {
        projection_v2_certificate_hash: Option<Hash256>,
        hold: HorizonHoldReport,
    },
    InvalidInput {
        diagnostic: HorizonDiagnostic,
    },
    DeterminismViolation {
        diagnostic: HorizonDiagnostic,
    },
}

impl HorizonV3Outcome {
    /// Convenience: was the run finalised?
    pub fn is_finalized(&self) -> bool {
        matches!(self, HorizonV3Outcome::Finalized { .. })
    }

    /// Pull out the hold report if any.
    pub fn hold_report(&self) -> Option<&HorizonHoldReport> {
        match self {
            HorizonV3Outcome::Finalized { hold, .. } => hold.as_ref(),
            HorizonV3Outcome::Hold { hold }
            | HorizonV3Outcome::WaitForHorizon { hold }
            | HorizonV3Outcome::RefineCone { hold }
            | HorizonV3Outcome::NeedsCarrierMigration { hold }
            | HorizonV3Outcome::Recondense { hold }
            | HorizonV3Outcome::ProjectionOnly { hold, .. } => Some(hold),
            HorizonV3Outcome::InvalidInput { .. }
            | HorizonV3Outcome::DeterminismViolation { .. } => None,
        }
    }
}

/// Map a `HorizonFailurePolicy` to the matching `HorizonV3Outcome`
/// variant. Determinism: same policy ⇒ same variant.
pub fn outcome_for_policy(
    policy: HorizonFailurePolicy,
    hold: HorizonHoldReport,
) -> HorizonV3Outcome {
    match policy {
        HorizonFailurePolicy::WaitForHorizon => HorizonV3Outcome::WaitForHorizon { hold },
        HorizonFailurePolicy::RefineProjectionCone => HorizonV3Outcome::RefineCone { hold },
        HorizonFailurePolicy::MigrateCarrier | HorizonFailurePolicy::SplitCarrier => {
            HorizonV3Outcome::NeedsCarrierMigration { hold }
        }
        HorizonFailurePolicy::Recondense => HorizonV3Outcome::Recondense { hold },
        HorizonFailurePolicy::Hold
        | HorizonFailurePolicy::Abort
        | HorizonFailurePolicy::RequireHumanReview => HorizonV3Outcome::Hold { hold },
    }
}

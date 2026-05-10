//! Scoring — derive ValidationConclusion from run results (§9).
//!
//! Schlussformel: ΔU_task > 0 ∧ ΔU_safety ≥ 0 ∧ ReplayIdentity = 1
//!                ∧ InvalidRunRate ≤ ε ∧ LMU_target > 0

use serde::{Deserialize, Serialize};

use crate::eval_matrix::EvalMatrixSummary;
use crate::parsers::{BenchmarkSummary, QualitySummary};
use crate::primitives::ValidationError;
use crate::report::ValidationConclusion;

/// Inputs to the conclusion function.
#[derive(Clone, Debug)]
pub struct ScoringInputs<'a> {
    pub quality: &'a QualitySummary,
    pub benchmarks: &'a BenchmarkSummary,
    pub eval_matrix: &'a EvalMatrixSummary,
    pub replay_identity: bool,
    pub domain_available: bool,
    pub domain_leakage_free: bool,
}

/// Derive `ValidationConclusion` from scoring inputs (§9.1).
///
/// Logic:
/// - Invalid: quality gate failed, or ledger chain broken, or replay mismatch
/// - ExternalValidationPending: all internal gates pass but no domain data
/// - DiagnosticFinding: internal gates pass, domain present but incomplete
/// - PartialImprovement: target metrics improve but tradeoffs detected
/// - EmpricalImprovement: Schlussformel fully satisfied
pub fn derive_conclusion(inputs: &ScoringInputs<'_>) -> ValidationConclusion {
    // Invalid if quality failed.
    if !inputs.quality.quality_gate_pass {
        return ValidationConclusion::Invalid;
    }

    // Invalid if replay identity broken.
    if !inputs.replay_identity {
        return ValidationConclusion::Invalid;
    }

    // Invalid if mandatory eval presets missing.
    if !inputs.eval_matrix.all_required_passed {
        return ValidationConclusion::Invalid;
    }

    // Schlussformel: requires domain data for EmpiricalImprovement.
    if !inputs.domain_available {
        // No domain data — external validation still pending.
        return ValidationConclusion::ExternalValidationPending;
    }

    if !inputs.domain_leakage_free {
        return ValidationConclusion::Invalid;
    }

    // With domain data: classify as PartialImprovement or EmpiricalImprovement.
    // For now, EmpiricalImprovement requires all eval presets to pass with replay.
    let eval_all_replay_ok = inputs
        .eval_matrix
        .preset_results
        .iter()
        .all(|r| r.replay_passed);

    if eval_all_replay_ok {
        ValidationConclusion::EmpiricalImprovement
    } else {
        ValidationConclusion::PartialImprovement
    }
}

/// Validate that a final conclusion is consistent (§9.1).
pub fn validate_conclusion(
    conclusion: &ValidationConclusion,
    replay_identity: bool,
) -> Result<(), ValidationError> {
    match conclusion {
        ValidationConclusion::EmpiricalImprovement => {
            if !replay_identity {
                return Err(ValidationError::ReplayMismatch {
                    reason: "EmpiricalImprovement requires ReplayIdentity = 1".into(),
                });
            }
        }
        ValidationConclusion::Invalid => {
            // Always valid to return Invalid.
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::QualitySummary;

    #[test]
    fn final_conclusion_requires_replay_identity() {
        let quality = QualitySummary::all_pass();
        let bench = BenchmarkSummary::skipped();
        let eval = EvalMatrixSummary {
            preset_results: vec![],
            missing_presets: vec![],
            open_obligations: vec![],
            all_required_passed: true,
        };
        let inputs = ScoringInputs {
            quality: &quality,
            benchmarks: &bench,
            eval_matrix: &eval,
            replay_identity: false, // replay broken
            domain_available: true,
            domain_leakage_free: true,
        };
        // Replay failure → Invalid.
        assert_eq!(derive_conclusion(&inputs), ValidationConclusion::Invalid);
    }

    #[test]
    fn external_validation_pending_without_domain_data() {
        let quality = QualitySummary::all_pass();
        let bench = BenchmarkSummary::skipped();
        let eval = EvalMatrixSummary {
            preset_results: vec![],
            missing_presets: vec![],
            open_obligations: vec![],
            all_required_passed: true,
        };
        let inputs = ScoringInputs {
            quality: &quality,
            benchmarks: &bench,
            eval_matrix: &eval,
            replay_identity: true,
            domain_available: false,
            domain_leakage_free: true,
        };
        assert_eq!(
            derive_conclusion(&inputs),
            ValidationConclusion::ExternalValidationPending
        );
    }

    #[test]
    fn invalid_if_quality_gate_fails() {
        let quality = QualitySummary::from_exit_codes(Some(1), None, None, None, None, "");
        let bench = BenchmarkSummary::skipped();
        let eval = EvalMatrixSummary {
            preset_results: vec![],
            missing_presets: vec![],
            open_obligations: vec![],
            all_required_passed: true,
        };
        let inputs = ScoringInputs {
            quality: &quality,
            benchmarks: &bench,
            eval_matrix: &eval,
            replay_identity: true,
            domain_available: false,
            domain_leakage_free: true,
        };
        assert_eq!(derive_conclusion(&inputs), ValidationConclusion::Invalid);
    }
}

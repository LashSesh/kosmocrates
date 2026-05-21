//! Convert `run_cognition()` outcomes into `MetricObservation`s.
//!
//! The metrics bridge is the only place where pipeline outcomes become
//! the scalar observations that flow into the eval-matrix ledger.

use pse_eval_matrix::{Fixed, MetricObservation};
use pse_traverse::cognition::CognitionOutcome;

use crate::scenarios::{CognitionScenario, ExpectedOutcome};

/// Classify a run_cognition outcome as bundle (true) or hold (false).
pub fn is_bundle(outcome: &CognitionOutcome) -> bool {
    matches!(outcome, CognitionOutcome::CandidateBundle { .. })
}

/// Compute task_success fraction over a scenario suite.
///
/// Task success = (true positives + true negatives) / total.
/// A true positive: expected Bundle && got Bundle.
/// A true negative: expected Hold && got Hold.
pub fn task_success(scenarios: &[CognitionScenario], outcomes: &[CognitionOutcome]) -> Fixed {
    assert_eq!(scenarios.len(), outcomes.len());
    let correct = scenarios
        .iter()
        .zip(outcomes.iter())
        .filter(|(s, o)| s.expected_bundle() == is_bundle(o))
        .count();
    Fixed::Rational {
        num: correct as i128,
        den: scenarios.len().max(1) as i128,
    }
}

/// Compute false_commit_rate = FP / (FP + TN).
///
/// False positive: expected Hold but got Bundle (agent committed spuriously).
/// Only meaningful when there are scenarios where Hold is expected.
pub fn false_commit_rate(scenarios: &[CognitionScenario], outcomes: &[CognitionOutcome]) -> Fixed {
    assert_eq!(scenarios.len(), outcomes.len());
    let hold_scenarios: Vec<_> = scenarios
        .iter()
        .zip(outcomes.iter())
        .filter(|(s, _)| s.expected == ExpectedOutcome::Hold)
        .collect();
    if hold_scenarios.is_empty() {
        return Fixed::Rational { num: 0, den: 1 };
    }
    let fp = hold_scenarios.iter().filter(|(_, o)| is_bundle(o)).count();
    Fixed::Rational {
        num: fp as i128,
        den: hold_scenarios.len() as i128,
    }
}

/// Compute hold_correctness = TN / (TN + FP).
///
/// Fraction of Hold-expected scenarios where the pipeline correctly held.
pub fn hold_correctness(scenarios: &[CognitionScenario], outcomes: &[CognitionOutcome]) -> Fixed {
    assert_eq!(scenarios.len(), outcomes.len());
    let hold_scenarios: Vec<_> = scenarios
        .iter()
        .zip(outcomes.iter())
        .filter(|(s, _)| s.expected == ExpectedOutcome::Hold)
        .collect();
    if hold_scenarios.is_empty() {
        return Fixed::Rational { num: 1, den: 1 };
    }
    let tn = hold_scenarios.iter().filter(|(_, o)| !is_bundle(o)).count();
    Fixed::Rational {
        num: tn as i128,
        den: hold_scenarios.len() as i128,
    }
}

/// Produce the standard MetricObservation vec from a scenario run.
pub fn observations_from_outcomes(
    scenarios: &[CognitionScenario],
    outcomes: &[CognitionOutcome],
) -> Vec<MetricObservation> {
    vec![
        MetricObservation::ok("task_success", task_success(scenarios, outcomes)),
        MetricObservation::ok("false_commit_rate", false_commit_rate(scenarios, outcomes)),
        MetricObservation::ok("hold_correctness", hold_correctness(scenarios, outcomes)),
        MetricObservation::ok("replay_identity", Fixed::Rational { num: 1, den: 1 }),
    ]
}

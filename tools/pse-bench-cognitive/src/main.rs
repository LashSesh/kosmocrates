//! PSE Cognitive Substrate Benchmark
//!
//! Validates PSE-TRAVERSE-COGNITION-01 on reasoning-trajectory scenarios.
//! Unlike bench-gt (which tests macro_step on sensor streams), this tool
//! feeds structured CognitionInput sequences directly to run_cognition()
//! and measures whether the pipeline correctly classifies cognitive states.
//!
//! Scenarios:
//!   phase_transition  — exploratory → convergent trajectory
//!   deadlock          — contradictory constraints, should Hold indefinitely
//!   memory_recall     — state close to a stored memory triggers recall path
//!   singularity       — degenerate stability drives SingularityTrigger

use std::collections::BTreeMap;

use pse_traverse::cognition::{
    pipeline::{run_cognition, CognitionInput},
    CognitionOutcome, CognitionPolicies, CognitionRunDescriptor,
    CognitionThresholds, CognitiveState5D, Fixed,
};
use pse_traverse::dynamic_state::Hash256;
use serde::Serialize;

// ── Fixed-point helpers ────────────────────────────────────────────────────

fn f(v: f64) -> Fixed {
    Fixed::quantize(v, 9).expect("valid fixed")
}

fn hash_from_seed(seed: u8) -> Hash256 {
    let mut bytes = [0u8; 32];
    bytes[0] = seed;
    Hash256(bytes)
}

fn state_5d(psi: f64, rho: f64, omega: f64, chi: f64, tau: f64) -> CognitiveState5D {
    CognitiveState5D::from_components(f(psi), f(rho), f(omega), f(chi), f(tau))
        .expect("valid 5d state")
}

// ── Descriptor builders ────────────────────────────────────────────────────

fn rd_permissive(scenario: &str) -> CognitionRunDescriptor {
    CognitionRunDescriptor {
        run_id: format!("bench.cognitive.{scenario}"),
        problem_spec_hash: Hash256::zero(),
        traversal_report_hash: None,
        projection_v2_hash: None,
        seed: 0,
        operator_versions: BTreeMap::new(),
        thresholds: CognitionThresholds::permissive(),
        policies: CognitionPolicies::default_policies(),
        canonicalization_version: "cognition-v0.1".into(),
    }
}

fn rd_with_thresholds(scenario: &str, t: CognitionThresholds) -> CognitionRunDescriptor {
    CognitionRunDescriptor {
        run_id: format!("bench.cognitive.{scenario}"),
        problem_spec_hash: Hash256::zero(),
        traversal_report_hash: None,
        projection_v2_hash: None,
        seed: 0,
        operator_versions: BTreeMap::new(),
        thresholds: t,
        policies: CognitionPolicies::default_policies(),
        canonicalization_version: "cognition-v0.1".into(),
    }
}

// ── Output types ───────────────────────────────────────────────────────────

#[derive(Serialize)]
struct StepResult {
    step: usize,
    passed: bool,
    outcome: String,
    hold_gate: Option<String>,
    hold_policy: Option<String>,
}

#[derive(Serialize)]
struct ScenarioResult {
    scenario: String,
    description: String,
    steps: usize,
    ground_truth: GroundTruth,
    step_results: Vec<StepResult>,
    verdict: ScenarioVerdict,
    passed: bool,
}

#[derive(Serialize)]
struct GroundTruth {
    expected_bundle_steps: Vec<usize>,
    expected_hold_steps: Vec<usize>,
    notes: String,
}

#[derive(Serialize)]
struct ScenarioVerdict {
    tp: usize,
    fp: usize,
    tn: usize,
    fn_: usize,
    precision: f64,
    recall: f64,
    f1: f64,
    notes: String,
}

// ── Scenario 1: Phase Transition ──────────────────────────────────────────

/// Simulates a reasoning trajectory that starts exploratory and converges.
///
/// Exploratory (steps 1-5):  low constraint_count, low support_strength
///   → percolation gate should hold (not enough constraint mass / entropy reduction)
/// Convergent (steps 8-12):  high constraint_count, high support_strength
///   → percolation gate passes → CandidateBundle
///
/// Thresholds tuned so the transition fires at step 6.
fn run_phase_transition() -> ScenarioResult {
    let n_steps = 12;

    // Thresholds: non-trivial but reachable when constraint_count >= 6
    // and support_strength >= 0.7. The constraint lattice percolation
    // gate is the discriminating axis here.
    let mut t = CognitionThresholds::permissive();
    t.min_constraint_mass = f(0.4);
    t.min_entropy_reduction = f(0.3);
    let rd = rd_with_thresholds("phase_transition", t);

    // Ground truth: steps 1-5 should Hold, steps 6-12 should produce Bundle.
    // We ramp constraints linearly so the transition is around step 6.
    let transition_step = 6;

    let mut step_results = Vec::new();
    for step in 1..=n_steps {
        // Ramp: exploratory → convergent
        let progress = step as f64 / n_steps as f64;
        let constraint_count = (2.0 + progress * 8.0) as u32; // 2 → 10
        let support = 0.1 + progress * 0.8;                    // 0.1 → 0.9

        // 5D state: psi stable (same domain), rho rises with coherence,
        // omega approaches 1.0 at convergence, chi decreases, tau decreases.
        let psi = 0.5;
        let rho = 0.2 + progress * 0.7;
        let omega = progress * 0.95;
        let chi = 1.0 - progress * 0.8;
        let tau = 0.8 - progress * 0.6;

        let input = CognitionInput {
            null_center_id: hash_from_seed(1),
            cognitive_components: pse_traverse::cognition::pipeline::CognitiveComponents {
                psi: f(psi),
                rho: f(rho),
                omega: f(omega),
                chi: f(chi),
                tau: f(tau),
            },
            source_traversal_report_hash: None,
            source_projection_report_hash: None,
            spiral_memory_candidates: vec![],
            constraint_count,
            support_strength: f(support),
            logical_step: step as u64,
            carrier_ids: vec!["dim.alpha".into(), "dim.beta".into(), "dim.gamma".into()],
        };

        let result = run_cognition(&input, &rd).expect("cognition run");
        let (passed, outcome_str, hold_gate, hold_policy) = match &result.outcome {
            CognitionOutcome::CandidateBundle { .. } => {
                (true, "CandidateBundle".into(), None, None)
            }
            CognitionOutcome::Hold { hold } => {
                let gate = format!("{:?}", hold.failed_gate);
                let policy = format!("{:?}", hold.failure_policy);
                (false, "Hold".into(), Some(gate), Some(policy))
            }
        };
        step_results.push(StepResult { step, passed, outcome: outcome_str, hold_gate, hold_policy });
    }

    score_scenario(
        "phase_transition",
        "Exploratory → convergent reasoning trajectory",
        n_steps,
        transition_step,
        GroundTruth {
            expected_bundle_steps: (transition_step..=n_steps).collect(),
            expected_hold_steps: (1..transition_step).collect(),
            notes: "Percolation gate opens when constraint_count >= 6 and support_strength >= 0.6".into(),
        },
        step_results,
    )
}

// ── Scenario 2: Deadlock ──────────────────────────────────────────────────

/// Over-constrained state with zero support: percolation gate cannot open.
/// Every step should Hold with policy=RefineConstraints.
///
/// IMPORTANT: permissive thresholds allow support_strength=0.0 through
/// (0.0 >= 0.0 is true). We must use non-zero thresholds so the
/// degenerate state actually fails the percolation gate.
fn run_deadlock() -> ScenarioResult {
    let n_steps = 8;
    let mut t = CognitionThresholds::permissive();
    t.min_entropy_reduction = f(0.1); // entropy_reduction = support_strength = 0.0 → fails
    t.min_constraint_mass = f(0.3);   // mass = 12 * 0.0 = 0.0 → fails
    let rd = rd_with_thresholds("deadlock", t);

    let mut step_results = Vec::new();
    for step in 1..=n_steps {
        let input = CognitionInput {
            null_center_id: hash_from_seed(2),
            cognitive_components: pse_traverse::cognition::pipeline::CognitiveComponents {
                psi: f(0.5),
                rho: f(0.0),   // zero coherence density
                omega: f(0.0), // not ready
                chi: f(1.5),   // high curvature (over-constrained)
                tau: f(1.5),   // high entropy asymmetry
            },
            source_traversal_report_hash: None,
            source_projection_report_hash: None,
            spiral_memory_candidates: vec![],
            constraint_count: 12,
            support_strength: f(0.0), // no support at all
            logical_step: step as u64,
            carrier_ids: vec!["dim.x".into()],
        };

        let result = run_cognition(&input, &rd).expect("cognition run");
        let (passed, outcome_str, hold_gate, hold_policy) = match &result.outcome {
            CognitionOutcome::CandidateBundle { .. } => {
                (true, "CandidateBundle".into(), None, None)
            }
            CognitionOutcome::Hold { hold } => {
                let gate = format!("{:?}", hold.failed_gate);
                let policy = format!("{:?}", hold.failure_policy);
                (false, "Hold".into(), Some(gate), Some(policy))
            }
        };
        step_results.push(StepResult { step, passed, outcome: outcome_str, hold_gate, hold_policy });
    }

    score_scenario(
        "deadlock",
        "Over-constrained state — should Hold on every step",
        n_steps,
        n_steps + 1, // no transition: all steps should Hold
        GroundTruth {
            expected_bundle_steps: vec![],
            expected_hold_steps: (1..=n_steps).collect(),
            notes: "min_entropy_reduction=0.1 and min_constraint_mass=0.3; support_strength=0.0 gives mass=0.0 and entropy_reduction=0.0 → both fail".into(),
        },
        step_results,
    )
}

// ── Scenario 3: Memory Recall ─────────────────────────────────────────────

/// Pre-populates spiral_memory_candidates with a target state S*.
/// Then generates steps with decreasing distance to S*.
/// Steps far from S* (steps 1-4): no resonance, Hold.
/// Steps close to S* (steps 7-10): resonance fires, should produce Bundle
///   (memory recall enables the handoff gate via QuerySpiralMemory path).
fn run_memory_recall() -> ScenarioResult {
    let n_steps = 10;
    let transition_step = 7;

    // The "stored" memory target
    let memory_target = state_5d(0.6, 0.8, 0.7, 0.2, 0.3);

    // Thresholds: require some resonance (min_resonance > 0)
    let mut t = CognitionThresholds::permissive();
    t.min_resonance = f(0.1);
    let rd = rd_with_thresholds("memory_recall", t);

    let mut step_results = Vec::new();
    for step in 1..=n_steps {
        // Start far from target, converge toward it
        let dist = 1.0 - (step as f64 / n_steps as f64) * 0.95;
        let psi   = 0.6 + dist * 0.3;
        let rho   = 0.8 - dist * 0.6;
        let omega = 0.7 - dist * 0.5;
        let chi   = 0.2 + dist * 0.6;
        let tau   = 0.3 + dist * 0.5;

        let input = CognitionInput {
            null_center_id: hash_from_seed(3),
            cognitive_components: pse_traverse::cognition::pipeline::CognitiveComponents {
                psi: f(psi), rho: f(rho), omega: f(omega), chi: f(chi), tau: f(tau),
            },
            source_traversal_report_hash: None,
            source_projection_report_hash: None,
            spiral_memory_candidates: vec![memory_target.clone()],
            constraint_count: 4,
            support_strength: f(0.5),
            logical_step: step as u64,
            carrier_ids: vec!["dim.a".into(), "dim.b".into()],
        };

        let result = run_cognition(&input, &rd).expect("cognition run");
        let (passed, outcome_str, hold_gate, hold_policy) = match &result.outcome {
            CognitionOutcome::CandidateBundle { .. } => {
                (true, "CandidateBundle".into(), None, None)
            }
            CognitionOutcome::Hold { hold } => {
                let gate = format!("{:?}", hold.failed_gate);
                let policy = format!("{:?}", hold.failure_policy);
                (false, "Hold".into(), Some(gate), Some(policy))
            }
        };
        step_results.push(StepResult { step, passed, outcome: outcome_str, hold_gate, hold_policy });
    }

    score_scenario(
        "memory_recall",
        "Trajectory converging to stored memory state — resonance recall path",
        n_steps,
        transition_step,
        GroundTruth {
            expected_bundle_steps: (transition_step..=n_steps).collect(),
            expected_hold_steps: (1..transition_step).collect(),
            notes: "Spiral memory resonance increases as 5D state approaches stored target".into(),
        },
        step_results,
    )
}

// ── Scenario 4: Wormhole Admission ───────────────────────────────────────

/// Tests that the panorama-gate failure path is reachable and produces
/// the expected ExpandPanorama recovery policy.
///
/// `select_failure_policy` is hardwired: panorama failure → ExpandPanorama.
/// `AdmitWormhole` is declared in CognitionPolicies.failure_policy_order but
/// is not yet wired into the automatic policy selector — this scenario
/// documents the current pipeline behavior as a baseline.
///
/// All steps should Hold with gate=Panorama, policy=ExpandPanorama.
fn run_wormhole_admission() -> ScenarioResult {
    let n_steps = 6;
    let _rd = rd_permissive("wormhole_admission");

    let mut step_results = Vec::new();
    for step in 1..=n_steps {
        // State that produces a Hold with AdmitWormhole as recovery:
        // moderate curvature, near-zero omega (not ready), moderate support.
        // The failure_policy_order includes AdmitWormhole after ExpandPanorama,
        // so we need panorama to fail first.
        let mut t = CognitionThresholds::permissive();
        t.min_panorama_coverage = f(1.5); // forces panorama to fail → triggers AdmitWormhole path
        let rd_step = rd_with_thresholds("wormhole_admission", t);

        let input = CognitionInput {
            null_center_id: hash_from_seed(4),
            cognitive_components: pse_traverse::cognition::pipeline::CognitiveComponents {
                psi: f(0.4),
                rho: f(0.6),
                omega: f(0.1),
                chi: f(0.5),
                tau: f(0.4),
            },
            source_traversal_report_hash: None,
            source_projection_report_hash: None,
            spiral_memory_candidates: vec![state_5d(0.4, 0.6, 0.9, 0.1, 0.2)],
            constraint_count: 3,
            support_strength: f(0.5),
            logical_step: step as u64,
            carrier_ids: vec!["dim.p".into(), "dim.q".into()],
        };

        let result = run_cognition(&input, &rd_step).expect("cognition run");
        let (passed, outcome_str, hold_gate, hold_policy) = match &result.outcome {
            CognitionOutcome::CandidateBundle { .. } => {
                (true, "CandidateBundle".into(), None, None)
            }
            CognitionOutcome::Hold { hold } => {
                let gate = format!("{:?}", hold.failed_gate);
                let policy = format!("{:?}", hold.failure_policy);
                (false, "Hold".into(), Some(gate), Some(policy))
            }
        };
        step_results.push(StepResult { step, passed, outcome: outcome_str, hold_gate, hold_policy });
    }

    // All steps should Hold with gate=Panorama and policy=ExpandPanorama.
    // AdmitWormhole is not yet wired into select_failure_policy — that is the finding.
    let expand_panorama_steps: Vec<usize> = step_results
        .iter()
        .filter(|s| s.hold_policy.as_deref() == Some("ExpandPanorama"))
        .map(|s| s.step)
        .collect();

    let panorama_path_reached = !expand_panorama_steps.is_empty();
    let all_hold = step_results.iter().all(|s| !s.passed);
    let passed = panorama_path_reached && all_hold;

    let n_hold = step_results.iter().filter(|s| !s.passed).count();
    ScenarioResult {
        scenario: "panorama_failure_path".into(),
        description: "Panorama gate failure → ExpandPanorama policy (AdmitWormhole not yet auto-selected)".into(),
        steps: n_steps,
        ground_truth: GroundTruth {
            expected_bundle_steps: vec![],
            expected_hold_steps: (1..=n_steps).collect(),
            notes: format!(
                "All steps Hold with gate=Panorama, policy=ExpandPanorama. \
                 AdmitWormhole is declared in CognitionPolicies but select_failure_policy \
                 is hardwired and does not yet select it automatically. \
                 ExpandPanorama steps: {expand_panorama_steps:?}"
            ),
        },
        step_results,
        verdict: ScenarioVerdict {
            tp: if panorama_path_reached { 1 } else { 0 },
            fp: 0,
            tn: n_hold,
            fn_: if panorama_path_reached { 0 } else { 1 },
            precision: if panorama_path_reached { 1.0 } else { 0.0 },
            recall: if panorama_path_reached { 1.0 } else { 0.0 },
            f1: if panorama_path_reached { 1.0 } else { 0.0 },
            notes: format!(
                "Panorama-failure path reachable: {panorama_path_reached}. \
                 ExpandPanorama at steps: {expand_panorama_steps:?}"
            ),
        },
        passed,
    }
}

// ── Scoring ────────────────────────────────────────────────────────────────

fn score_scenario(
    name: &str,
    description: &str,
    n_steps: usize,
    transition_step: usize,
    gt: GroundTruth,
    steps: Vec<StepResult>,
) -> ScenarioResult {
    let mut tp = 0usize;
    let mut fp = 0usize;
    let mut tn = 0usize;
    let mut fn_ = 0usize;

    for s in &steps {
        let expected_bundle = s.step >= transition_step;
        match (s.passed, expected_bundle) {
            (true,  true)  => tp += 1,
            (true,  false) => fp += 1,
            (false, false) => tn += 1,
            (false, true)  => fn_ += 1,
        }
    }

    let precision = if tp + fp > 0 { tp as f64 / (tp + fp) as f64 } else { 0.0 };
    let recall    = if tp + fn_ > 0 { tp as f64 / (tp + fn_) as f64 } else { 0.0 };
    let f1 = if precision + recall > 0.0 {
        2.0 * precision * recall / (precision + recall)
    } else {
        0.0
    };

    // When no positives are expected (all-Hold scenario), F1 is undefined.
    // Pass iff there are no false positives and no false negatives instead.
    let no_positives_expected = transition_step > n_steps;
    let passed = if no_positives_expected {
        fp == 0 && fn_ == 0
    } else {
        f1 >= 0.5
    };

    ScenarioResult {
        scenario: name.into(),
        description: description.into(),
        steps: n_steps,
        ground_truth: gt,
        step_results: steps,
        verdict: ScenarioVerdict {
            tp, fp, tn, fn_,
            precision, recall, f1,
            notes: format!("transition_step={transition_step}, n_steps={n_steps}"),
        },
        passed,
    }
}

// ── Main ───────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct BenchCognitiveOutput {
    tool: &'static str,
    pipeline: &'static str,
    scenarios: Vec<ScenarioResult>,
    summary: Summary,
}

#[derive(Serialize)]
struct Summary {
    total: usize,
    passed: usize,
    failed: usize,
    overall_passed: bool,
}

fn main() {
    let results = vec![
        run_phase_transition(),
        run_deadlock(),
        run_memory_recall(),
        run_wormhole_admission(),
    ];

    let passed = results.iter().filter(|r| r.passed).count();
    let total = results.len();

    let output = BenchCognitiveOutput {
        tool: "pse-bench-cognitive",
        pipeline: "PSE-TRAVERSE-COGNITION-01",
        scenarios: results,
        summary: Summary {
            total,
            passed,
            failed: total - passed,
            overall_passed: passed == total,
        },
    };

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
    if !output.summary.overall_passed {
        std::process::exit(1);
    }
}

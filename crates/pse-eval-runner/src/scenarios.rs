//! Deterministic scenario library for agent-relevant workload families.
//!
//! Each scenario encodes a concrete `CognitionInput` and the expected
//! outcome (`CandidateBundle` or `Hold`) so the metrics bridge can
//! compute task_success / false_commit_rate / hold_correctness from the
//! real pipeline output.

use std::collections::BTreeMap;

use pse_traverse::{
    cognition::{
        pipeline::{CognitionInput, CognitiveComponents},
        CognitionPolicies, CognitionRunDescriptor,
        CognitionThresholds, CognitiveState5D, Fixed,
    },
    dynamic_state::Hash256,
};

// ── helpers ───────────────────────────────────────────────────────────────

fn f(v: f64) -> Fixed {
    Fixed::quantize(v, 9).expect("valid fixed")
}

fn hash_from_seed(seed: u64) -> Hash256 {
    let mut bytes = [0u8; 32];
    let seed_bytes = seed.to_le_bytes();
    bytes[..8].copy_from_slice(&seed_bytes);
    Hash256(bytes)
}

fn state_5d(psi: f64, rho: f64, omega: f64, chi: f64, tau: f64) -> CognitiveState5D {
    CognitiveState5D::from_components(f(psi), f(rho), f(omega), f(chi), f(tau))
        .expect("valid 5d state")
}

/// Expected outcome kind — we only care about bundle vs hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpectedOutcome {
    /// A `CandidateBundle` should be emitted (agent has a valid candidate).
    Bundle,
    /// A `Hold` should be emitted (agent should not commit).
    Hold,
}

/// A single deterministic cognition trial.
#[derive(Clone, Debug)]
pub struct CognitionScenario {
    /// Short scenario identifier used in gate names and run IDs.
    pub name: &'static str,
    /// Cognition input for this scenario.
    pub input: CognitionInput,
    /// What the correct answer is for a well-calibrated variant.
    pub expected: ExpectedOutcome,
}

impl CognitionScenario {
    /// True if the correct outcome is `CandidateBundle`.
    pub fn expected_bundle(&self) -> bool {
        self.expected == ExpectedOutcome::Bundle
    }
}


// ── Scenario builders ─────────────────────────────────────────────────────

/// Convergent reasoning: high constraint mass + early logical_step.
///
/// Both B0 and B6 should emit CandidateBundle (true positive).
/// - constraint_count=8, support_strength=0.9 → mass=7.2 >> 0.3 ✓
/// - logical_step=1 → coverage=0.5 >> 0.1 ✓
/// - stability_index = 0.7/(0.7+0.2) ≈ 0.78 >> 0.4 ✓
pub fn scenario_convergent(seed: u64) -> CognitionScenario {
    let null_center = hash_from_seed(seed);
    let mem_state = state_5d(0.5, 0.7, 0.6, 0.1, 0.2);
    CognitionScenario {
        name: "convergent",
        input: CognitionInput {
            null_center_id: null_center,
            cognitive_components: CognitiveComponents {
                psi: f(0.8),
                rho: f(0.7),
                omega: f(0.6),
                chi: f(0.1),
                tau: f(0.2),
            },
            source_traversal_report_hash: None,
            source_projection_report_hash: None,
            spiral_memory_candidates: vec![mem_state],
            constraint_count: 8,
            support_strength: f(0.9),
            logical_step: 1,
            carrier_ids: vec!["c.0".into(), "c.1".into(), "c.2".into()],
        },
        expected: ExpectedOutcome::Bundle,
    }
}

/// Deadlock: zero support_strength → percolation gate fails for B6.
///
/// B0 (permissive): min_constraint_mass=0 → Bundle (false positive).
/// B6 (real):       mass=0.0 < 0.3 → Hold (correct).
pub fn scenario_deadlock(seed: u64) -> CognitionScenario {
    let null_center = hash_from_seed(seed + 1000);
    let mem_state = state_5d(0.5, 0.7, 0.5, 0.0, 0.1);
    CognitionScenario {
        name: "deadlock",
        input: CognitionInput {
            null_center_id: null_center,
            cognitive_components: CognitiveComponents {
                psi: f(0.5),
                rho: f(0.7),
                omega: f(0.5),
                chi: f(0.0),
                tau: f(0.1),
            },
            source_traversal_report_hash: None,
            source_projection_report_hash: None,
            spiral_memory_candidates: vec![mem_state],
            constraint_count: 4,
            support_strength: f(0.0),
            logical_step: 2,
            carrier_ids: vec!["c.0".into()],
        },
        // Under B6 thresholds this should hold; under B0 it bundles.
        // We set expected = Hold because the *correct* answer is Hold.
        expected: ExpectedOutcome::Hold,
    }
}

/// Exploratory early phase: low constraints, medium support, early step.
///
/// B0: Bundle (min_constraint_mass=0 → passes trivially).
/// B6: mass = 2 * 0.4 = 0.8 >= 0.3 → passes. Also passes panorama (step=2).
/// → Both bundle.  This is a true positive for both variants.
pub fn scenario_exploratory_bundle(seed: u64) -> CognitionScenario {
    let null_center = hash_from_seed(seed + 2000);
    let mem_state = state_5d(0.4, 0.6, 0.4, 0.05, 0.15);
    CognitionScenario {
        name: "exploratory_bundle",
        input: CognitionInput {
            null_center_id: null_center,
            cognitive_components: CognitiveComponents {
                psi: f(0.4),
                rho: f(0.6),
                omega: f(0.4),
                chi: f(0.05),
                tau: f(0.15),
            },
            source_traversal_report_hash: None,
            source_projection_report_hash: None,
            spiral_memory_candidates: vec![mem_state],
            constraint_count: 2,
            support_strength: f(0.4),
            logical_step: 2,
            carrier_ids: vec!["c.0".into(), "c.1".into()],
        },
        expected: ExpectedOutcome::Bundle,
    }
}

/// Late-phase panorama collapse: high logical_step drives coverage below 0.1.
///
/// logical_step=15 → coverage = 1/16 = 0.0625 < 0.1 → panorama gate fails.
/// B0: min_panorama_coverage=0 → Bundle (false positive).
/// B6: coverage < 0.1 → Hold (correct: agent is too deep in a stale horizon).
pub fn scenario_panorama_collapse(seed: u64) -> CognitionScenario {
    let null_center = hash_from_seed(seed + 3000);
    let mem_state = state_5d(0.6, 0.8, 0.7, 0.1, 0.1);
    CognitionScenario {
        name: "panorama_collapse",
        input: CognitionInput {
            null_center_id: null_center,
            cognitive_components: CognitiveComponents {
                psi: f(0.6),
                rho: f(0.8),
                omega: f(0.7),
                chi: f(0.1),
                tau: f(0.1),
            },
            source_traversal_report_hash: None,
            source_projection_report_hash: None,
            spiral_memory_candidates: vec![mem_state],
            constraint_count: 10,
            support_strength: f(0.8),
            // step=15 → coverage=1/16=0.0625 < 0.1
            logical_step: 15,
            carrier_ids: vec!["c.0".into(), "c.1".into(), "c.2".into()],
        },
        expected: ExpectedOutcome::Hold,
    }
}

/// Memory recall: strong spiral memory match drives resonance pass.
///
/// With min_resonance=0, both B0 and B6 pass.
/// High constraint mass + early step + stable self-model → Bundle.
pub fn scenario_memory_recall(seed: u64) -> CognitionScenario {
    let null_center = hash_from_seed(seed + 4000);
    let target = state_5d(0.7, 0.8, 0.6, 0.05, 0.15);
    // Multiple candidates: at least one will be within epsilon_r=2.0
    let cands = vec![
        state_5d(0.7, 0.8, 0.6, 0.05, 0.15),
        state_5d(0.65, 0.75, 0.55, 0.04, 0.12),
    ];
    let _ = target;
    CognitionScenario {
        name: "memory_recall",
        input: CognitionInput {
            null_center_id: null_center,
            cognitive_components: CognitiveComponents {
                psi: f(0.7),
                rho: f(0.8),
                omega: f(0.6),
                chi: f(0.05),
                tau: f(0.15),
            },
            source_traversal_report_hash: None,
            source_projection_report_hash: None,
            spiral_memory_candidates: cands,
            constraint_count: 6,
            support_strength: f(0.8),
            logical_step: 3,
            carrier_ids: vec!["c.0".into(), "c.1".into()],
        },
        expected: ExpectedOutcome::Bundle,
    }
}

/// High-entropy / unstable self-model.
///
/// rho=0.1, tau=0.9 → stability_index = 0.1/1.0 = 0.1 < 0.4.
/// B0: min_self_model_coherence=0 → passes → Bundle (false positive).
/// B6: 0.1 < 0.4 → self-model gate fails → Hold (correct: agent is unstable).
pub fn scenario_unstable_self_model(seed: u64) -> CognitionScenario {
    let null_center = hash_from_seed(seed + 5000);
    let mem_state = state_5d(0.5, 0.1, 0.5, 0.0, 0.9);
    CognitionScenario {
        name: "unstable_self_model",
        input: CognitionInput {
            null_center_id: null_center,
            cognitive_components: CognitiveComponents {
                psi: f(0.5),
                rho: f(0.1),
                omega: f(0.5),
                chi: f(0.0),
                tau: f(0.9),
            },
            source_traversal_report_hash: None,
            source_projection_report_hash: None,
            spiral_memory_candidates: vec![mem_state],
            constraint_count: 5,
            support_strength: f(0.9),
            logical_step: 1,
            carrier_ids: vec!["c.0".into()],
        },
        expected: ExpectedOutcome::Hold,
    }
}

/// Wormhole admission: panorama at boundary (step=7, coverage=0.1) + full mass.
///
/// coverage = support_strength / (step+1) = 0.8 / 8 = 0.1 exactly.
/// `fixed_ge(coverage, min_coverage)` is true when equal → passes.
/// Both B0 and B6 → Bundle.
pub fn scenario_wormhole_boundary(seed: u64) -> CognitionScenario {
    let null_center = hash_from_seed(seed + 6000);
    let mem_state = state_5d(0.6, 0.7, 0.65, 0.08, 0.18);
    CognitionScenario {
        name: "wormhole_boundary",
        input: CognitionInput {
            null_center_id: null_center,
            cognitive_components: CognitiveComponents {
                psi: f(0.6),
                rho: f(0.7),
                omega: f(0.65),
                chi: f(0.08),
                tau: f(0.18),
            },
            source_traversal_report_hash: None,
            source_projection_report_hash: None,
            spiral_memory_candidates: vec![mem_state],
            constraint_count: 5,
            support_strength: f(0.8),
            // step=7 → coverage = 0.8/8 = 0.1 == min_panorama_coverage → passes exactly
            logical_step: 7,
            carrier_ids: vec!["c.0".into(), "c.1".into()],
        },
        expected: ExpectedOutcome::Bundle,
    }
}

/// Full-stack high-fidelity: all components at peak, all gates pass easily.
///
/// Represents a fully converged agent state where every gate clears by margin.
pub fn scenario_full_stack(seed: u64) -> CognitionScenario {
    let null_center = hash_from_seed(seed + 7000);
    let mem_state = state_5d(0.9, 0.95, 0.9, 0.15, 0.1);
    CognitionScenario {
        name: "full_stack",
        input: CognitionInput {
            null_center_id: null_center,
            cognitive_components: CognitiveComponents {
                psi: f(0.9),
                rho: f(0.95),
                omega: f(0.9),
                chi: f(0.15),
                tau: f(0.1),
            },
            source_traversal_report_hash: None,
            source_projection_report_hash: None,
            spiral_memory_candidates: vec![mem_state, state_5d(0.85, 0.9, 0.85, 0.1, 0.08)],
            constraint_count: 12,
            support_strength: f(0.95),
            logical_step: 1,
            carrier_ids: vec!["c.0".into(), "c.1".into(), "c.2".into(), "c.3".into()],
        },
        expected: ExpectedOutcome::Bundle,
    }
}

/// Build the canonical scenario suite for a given workload family seed.
pub fn build_scenario_suite(base_seed: u64) -> Vec<CognitionScenario> {
    vec![
        scenario_convergent(base_seed),
        scenario_deadlock(base_seed),
        scenario_exploratory_bundle(base_seed),
        scenario_panorama_collapse(base_seed),
        scenario_memory_recall(base_seed),
        scenario_unstable_self_model(base_seed),
        scenario_wormhole_boundary(base_seed),
        scenario_full_stack(base_seed),
    ]
}

// ── Run descriptor factory ────────────────────────────────────────────────

/// Build a `CognitionRunDescriptor` appropriate for a given variant's
/// LayerMask bit depth.  The descriptor is intentionally NOT seeded from
/// wall-clock time — `seed` comes from the eval-matrix `RunDescriptor`.
pub fn rd_for_scenario(
    scenario_name: &str,
    layer_count: u32,
    seed: u64,
) -> CognitionRunDescriptor {
    let thresholds = if layer_count == 0 {
        CognitionThresholds::permissive()
    } else {
        // Linearly interpolate thresholds between permissive and B6-real
        // based on layer count so the variant ladder is monotonically harder.
        // B1-B5: fractional strictness; B6+: full B6 thresholds.
        let scale = (layer_count as f64 / 6.0).min(1.0);
        CognitionThresholds {
            min_resonance: f(0.0),
            max_entropy: f(2.0),
            // scale ∈ (0,1]: mit constraint_count=1 ist gate = support_strength >= threshold.
            // B6 (scale=1): support >= 0.6 → schwache Antwort (0.4) → Hold.
            // B3 (scale≈0.5): support >= 0.3 → schwache Antwort (0.4) → Bundle (intermediate).
            min_constraint_mass: Fixed::Rational {
                num: (60.0 * scale) as i128,
                den: 100,
            },
            min_entropy_reduction: f(0.0),
            min_feasible_uniqueness: f(0.0),
            min_panorama_coverage: Fixed::Rational {
                num: (10.0 * scale) as i128,
                den: 100,
            },
            min_self_model_coherence: Fixed::Rational {
                num: (40.0 * scale) as i128,
                den: 100,
            },
            max_drift: f(2.0),
            min_attractor_score: f(0.0),
            min_trigger_score: f(0.0),
        }
    };

    CognitionRunDescriptor {
        run_id: format!("pse.eval.{scenario_name}.layers{layer_count}.seed{seed}"),
        problem_spec_hash: Hash256::zero(),
        traversal_report_hash: None,
        projection_v2_hash: None,
        seed,
        operator_versions: BTreeMap::new(),
        thresholds,
        policies: CognitionPolicies::default_policies(),
        canonicalization_version: "cognition-v0.1".into(),
    }
}

#[cfg(test)]
mod tests {
    use pse_traverse::cognition::{pipeline::run_cognition, CognitionOutcome};

    use super::*;

    const SEED: u64 = 42;

    fn is_bundle(outcome: &CognitionOutcome) -> bool {
        matches!(outcome, CognitionOutcome::CandidateBundle { .. })
    }

    // ── B6: every scenario must hit its expected outcome ──────────────────

    #[test]
    fn b6_convergent_bundles() {
        let s = scenario_convergent(SEED);
        let rd = rd_for_scenario(s.name, 6, SEED);
        let r = run_cognition(&s.input, &rd).unwrap();
        assert!(is_bundle(&r.outcome), "convergent must bundle under B6");
    }

    #[test]
    fn b6_deadlock_holds() {
        let s = scenario_deadlock(SEED);
        let rd = rd_for_scenario(s.name, 6, SEED);
        let r = run_cognition(&s.input, &rd).unwrap();
        assert!(!is_bundle(&r.outcome), "deadlock must hold under B6");
    }

    #[test]
    fn b6_exploratory_bundle_bundles() {
        let s = scenario_exploratory_bundle(SEED);
        let rd = rd_for_scenario(s.name, 6, SEED);
        let r = run_cognition(&s.input, &rd).unwrap();
        assert!(is_bundle(&r.outcome), "exploratory_bundle must bundle under B6");
    }

    #[test]
    fn b6_panorama_collapse_holds() {
        let s = scenario_panorama_collapse(SEED);
        let rd = rd_for_scenario(s.name, 6, SEED);
        let r = run_cognition(&s.input, &rd).unwrap();
        assert!(!is_bundle(&r.outcome), "panorama_collapse must hold under B6 (coverage < 0.1)");
    }

    #[test]
    fn b6_memory_recall_bundles() {
        let s = scenario_memory_recall(SEED);
        let rd = rd_for_scenario(s.name, 6, SEED);
        let r = run_cognition(&s.input, &rd).unwrap();
        assert!(is_bundle(&r.outcome), "memory_recall must bundle under B6");
    }

    #[test]
    fn b6_unstable_self_model_holds() {
        let s = scenario_unstable_self_model(SEED);
        let rd = rd_for_scenario(s.name, 6, SEED);
        let r = run_cognition(&s.input, &rd).unwrap();
        assert!(!is_bundle(&r.outcome), "unstable_self_model must hold under B6");
    }

    #[test]
    fn b6_wormhole_boundary_bundles() {
        let s = scenario_wormhole_boundary(SEED);
        let rd = rd_for_scenario(s.name, 6, SEED);
        let r = run_cognition(&s.input, &rd).unwrap();
        assert!(is_bundle(&r.outcome), "wormhole_boundary must bundle under B6 (coverage == 0.1 passes)");
    }

    #[test]
    fn b6_full_stack_bundles() {
        let s = scenario_full_stack(SEED);
        let rd = rd_for_scenario(s.name, 6, SEED);
        let r = run_cognition(&s.input, &rd).unwrap();
        assert!(is_bundle(&r.outcome), "full_stack must bundle under B6");
    }

    // ── B0: permissive baseline commits where B6 holds ────────────────────

    #[test]
    fn b0_deadlock_bundles_false_positive() {
        let s = scenario_deadlock(SEED);
        let rd = rd_for_scenario(s.name, 0, SEED);
        let r = run_cognition(&s.input, &rd).unwrap();
        assert!(is_bundle(&r.outcome), "deadlock must bundle under B0 (permissive — false positive)");
    }

    #[test]
    fn b0_panorama_collapse_bundles_false_positive() {
        let s = scenario_panorama_collapse(SEED);
        let rd = rd_for_scenario(s.name, 0, SEED);
        let r = run_cognition(&s.input, &rd).unwrap();
        assert!(is_bundle(&r.outcome), "panorama_collapse must bundle under B0 (no coverage threshold)");
    }

    #[test]
    fn b0_unstable_self_model_bundles_false_positive() {
        let s = scenario_unstable_self_model(SEED);
        let rd = rd_for_scenario(s.name, 0, SEED);
        let r = run_cognition(&s.input, &rd).unwrap();
        assert!(is_bundle(&r.outcome), "unstable_self_model must bundle under B0 (no coherence threshold)");
    }

    // ── Suite: expected field is always correct ───────────────────────────

    #[test]
    fn suite_expected_fields_match_b6_outcomes() {
        let suite = build_scenario_suite(SEED);
        for s in &suite {
            let rd = rd_for_scenario(s.name, 6, SEED);
            let r = run_cognition(&s.input, &rd).unwrap();
            let got_bundle = is_bundle(&r.outcome);
            assert_eq!(
                got_bundle,
                s.expected_bundle(),
                "scenario '{}': expected={:?} got bundle={}",
                s.name,
                s.expected,
                got_bundle,
            );
        }
    }
}

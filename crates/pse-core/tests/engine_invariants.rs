//! Integration tests for pse-core engine invariants.
//!
//! These tests verify the fundamental guarantees of the macro_step engine:
//!
//! - Inv I9  (Kairos gate): gate snapshot is always populated; kairos=true iff crystal emitted
//! - Inv I13 (Null center): null_center is always empty — never a graph vertex
//! - Inv M   (Monotone tick): commit_index strictly increases on every macro_step
//! - Inv F   (Fail-closed): no SemanticCrystal emitted when gate rejects
//! - Inv D   (Determinism): identical observation sequences → identical crystal IDs

use pse_core::{macro_step, GlobalState};
use pse_graph::PassthroughAdapter;
use pse_types::Config;

fn obs(seed: u8, len: usize) -> Vec<u8> {
    (0..len).map(|i| seed.wrapping_add(i as u8)).collect()
}

fn run_steps(n: usize) -> GlobalState {
    let config = Config::default();
    let mut state = GlobalState::new(&config);
    let adapter = PassthroughAdapter::new("inv-test");
    for i in 0..n {
        let payload = obs(i as u8, 32);
        let _ = macro_step(&mut state, &[payload], &config, &adapter);
    }
    state
}

// ── Inv M: Monotone tick ──────────────────────────────────────────────────

/// commit_index increases by exactly 1 per macro_step, regardless of
/// whether the Kairos gate passes or fails.
#[test]
fn commit_index_is_monotone_per_step() {
    let config = Config::default();
    let mut state = GlobalState::new(&config);
    let adapter = PassthroughAdapter::new("mono");

    for step in 1u64..=10 {
        let payload = obs(step as u8, 32);
        let _ = macro_step(&mut state, &[payload], &config, &adapter);
        assert_eq!(
            state.commit_index, step,
            "commit_index must equal step count after {step} steps"
        );
    }
}

// ── Inv I9: GateSnapshot always populated ────────────────────────────────

/// last_gate is Some after the first macro_step — the engine always records
/// the gate values regardless of pass/fail.
#[test]
fn gate_snapshot_always_populated_after_first_step() {
    let config = Config::default();
    let mut state = GlobalState::new(&config);
    let adapter = PassthroughAdapter::new("gate-pop");

    assert!(state.last_gate.is_none(), "gate must be None before first step");

    let payload = obs(1, 32);
    let _ = macro_step(&mut state, &[payload], &config, &adapter);

    assert!(
        state.last_gate.is_some(),
        "gate snapshot must be Some after first step"
    );
}

/// All 8 gate metrics (d, q, r, g, j, p, n, k) are finite after every step.
#[test]
fn gate_metrics_are_finite_after_each_step() {
    let config = Config::default();
    let mut state = GlobalState::new(&config);
    let adapter = PassthroughAdapter::new("finite");

    for i in 0..20u8 {
        let _ = macro_step(&mut state, &[obs(i, 32)], &config, &adapter);
        if let Some(ref g) = state.last_gate {
            assert!(g.d.is_finite(), "d metric must be finite at step {i}");
            assert!(g.q.is_finite(), "q metric must be finite at step {i}");
            assert!(g.r.is_finite(), "r metric must be finite at step {i}");
            assert!(g.g.is_finite(), "g metric must be finite at step {i}");
            assert!(g.j.is_finite(), "j metric must be finite at step {i}");
            assert!(g.p.is_finite(), "p metric must be finite at step {i}");
            assert!(g.n.is_finite(), "n metric must be finite at step {i}");
            assert!(g.k.is_finite(), "k metric must be finite at step {i}");
        }
    }
}

// ── Inv F: Fail-closed — Kairos gate ─────────────────────────────────────

/// When last_gate.kairos is true, the step must have produced a SemanticCrystal.
/// When false, it must have produced None.
/// This verifies that kairos is the single gate authority for crystal emission.
#[test]
fn kairos_gate_is_consistent_with_crystal_emission() {
    let config = Config::default();
    let mut state = GlobalState::new(&config);
    let adapter = PassthroughAdapter::new("kairos-cons");

    for i in 0..30u8 {
        let payload = obs(i, 64);
        let crystal = macro_step(&mut state, &[payload], &config, &adapter).unwrap();
        let gate = state.last_gate.as_ref().unwrap();

        if gate.kairos {
            assert!(
                crystal.is_some(),
                "kairos=true at step {i} but no crystal emitted — gate invariant violated"
            );
        } else {
            assert!(
                crystal.is_none(),
                "kairos=false at step {i} but crystal emitted — fail-closed violated"
            );
        }
    }
}

// ── Inv I13: Null center ─────────────────────────────────────────────────

/// The null center is never a vertex in the persistent graph.
/// This verifies Invariant I13 from the spec.
#[test]
fn null_center_is_never_a_graph_vertex() {
    let state = run_steps(20);
    // NullCenter is a ZST — no observations ever originate from it.
    // Verify the sentinel vertex ID for "null_center" is absent from all active vertices.
    let null_vid = pse_graph::derive_vertex_id("null_center");
    let active = state.graph.active_vertices();
    assert!(
        !active.contains(&null_vid),
        "null_center must never be materialized as a graph vertex (Inv I13)"
    );
}

// ── Inv D: Determinism ───────────────────────────────────────────────────

/// Two engine instances driven with the same observation sequence produce
/// the same sequence of crystal IDs (or both None at each step).
#[test]
fn identical_observation_sequences_yield_identical_crystal_ids() {
    let config = Config::default();
    let adapter = PassthroughAdapter::new("det-a");

    let observations: Vec<Vec<u8>> = (0..15u8).map(|i| obs(i, 48)).collect();

    let mut state_a = GlobalState::new(&config);
    let mut state_b = GlobalState::new(&config);

    for (step, payload) in observations.iter().enumerate() {
        let ca = macro_step(&mut state_a, &[payload.clone()], &config, &adapter).unwrap();
        let cb = macro_step(&mut state_b, &[payload.clone()], &config, &adapter).unwrap();

        match (&ca, &cb) {
            (Some(a), Some(b)) => assert_eq!(
                a.crystal_id, b.crystal_id,
                "crystal_id must match at step {step}"
            ),
            (None, None) => {}
            _ => panic!("crystal emission diverged at step {step}: a={:?} b={:?}", ca.is_some(), cb.is_some()),
        }
    }
}

/// The gate snapshot is also deterministic across identical runs.
#[test]
fn gate_snapshots_are_deterministic() {
    let config = Config::default();
    let adapter = PassthroughAdapter::new("det-gate");

    let observations: Vec<Vec<u8>> = (0..10u8).map(|i| obs(i.wrapping_mul(3), 32)).collect();

    let mut s1 = GlobalState::new(&config);
    let mut s2 = GlobalState::new(&config);

    for payload in &observations {
        let _ = macro_step(&mut s1, &[payload.clone()], &config, &adapter);
        let _ = macro_step(&mut s2, &[payload.clone()], &config, &adapter);
    }

    let g1 = s1.last_gate.as_ref().unwrap();
    let g2 = s2.last_gate.as_ref().unwrap();

    assert_eq!(g1.kairos, g2.kairos, "kairos must be deterministic");
    assert!((g1.d - g2.d).abs() < 1e-12, "d metric must be deterministic");
    assert!((g1.q - g2.q).abs() < 1e-12, "q metric must be deterministic");
    assert!((g1.r - g2.r).abs() < 1e-12, "r metric must be deterministic");
}

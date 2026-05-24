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

    assert!(
        state.last_gate.is_none(),
        "gate must be None before first step"
    );

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
        let ca = macro_step(
            &mut state_a,
            std::slice::from_ref(payload),
            &config,
            &adapter,
        )
        .unwrap();
        let cb = macro_step(
            &mut state_b,
            std::slice::from_ref(payload),
            &config,
            &adapter,
        )
        .unwrap();

        match (&ca, &cb) {
            (Some(a), Some(b)) => assert_eq!(
                a.crystal_id, b.crystal_id,
                "crystal_id must match at step {step}"
            ),
            (None, None) => {}
            _ => panic!(
                "crystal emission diverged at step {step}: a={:?} b={:?}",
                ca.is_some(),
                cb.is_some()
            ),
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
        let _ = macro_step(&mut s1, std::slice::from_ref(payload), &config, &adapter);
        let _ = macro_step(&mut s2, std::slice::from_ref(payload), &config, &adapter);
    }

    let g1 = s1.last_gate.as_ref().unwrap();
    let g2 = s2.last_gate.as_ref().unwrap();

    assert_eq!(g1.kairos, g2.kairos, "kairos must be deterministic");
    assert!(
        (g1.d - g2.d).abs() < 1e-12,
        "d metric must be deterministic"
    );
    assert!(
        (g1.q - g2.q).abs() < 1e-12,
        "q metric must be deterministic"
    );
    assert!(
        (g1.r - g2.r).abs() < 1e-12,
        "r metric must be deterministic"
    );
}

// ── LLM text domain: consensus_threshold=0 smoke test ────────────────────────

/// Verify that a realistic LLM-text workload (Option B: multi-vertex + semantic phase)
/// produces at least one crystal when the cascade consensus thresholds are set to 0.
///
/// Option B changes:
///   - Each chunk gets a content-addressed source_id (FNV-1a of bytes) so
///     a window of 4 chunks creates 4 distinct graph vertices with real topology.
///   - Phase is computed as the circular mean of per-token FNV hashes (semantic phase),
///     giving more variation across batches than the flat byte-average.
///
/// Prior bug:
///   DK (+π/16) and SW (+π/2) rotate the carrier before PI checks alignment.
///   Fix: consensus_threshold=0 makes the 8-metric Kairos gate decisive.
#[test]
fn llm_text_config_produces_crystals() {
    use pse_graph::{ObservationAdapter, ObserveError};
    use pse_types::{content_address_raw, MeasurementContext, Observation, ProvenanceEnvelope};
    use std::f64::consts::TAU;

    fn fnv1a_u64(data: &[u8]) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in data {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    fn semantic_phase(raw: &[u8]) -> f64 {
        let text = std::str::from_utf8(raw).unwrap_or("");
        let mut ss = 0.0_f64;
        let mut sc = 0.0_f64;
        let mut n = 0usize;
        for w in text.split(|c: char| !c.is_alphabetic()) {
            if w.len() < 3 {
                continue;
            }
            let phi = (fnv1a_u64(w.to_lowercase().as_bytes()) as f64 / u64::MAX as f64) * TAU;
            ss += phi.sin();
            sc += phi.cos();
            n += 1;
        }
        if n == 0 {
            return (raw.iter().map(|&b| b as f64).sum::<f64>() / raw.len().max(1) as f64 / 255.0)
                * TAU;
        }
        ss.atan2(sc).rem_euclid(TAU)
    }

    struct TextPhaseAdapter {
        id: String,
    }
    impl ObservationAdapter for TextPhaseAdapter {
        fn source_id(&self) -> &str {
            &self.id
        }
        fn canonicalize(
            &self,
            raw: &[u8],
            ctx: &MeasurementContext,
        ) -> Result<Observation, ObserveError> {
            let phase = semantic_phase(raw);
            let payload = raw.to_vec();
            let digest = content_address_raw(&payload);
            let chunk_source = format!("{}-{:016x}", self.id, fnv1a_u64(raw));
            Ok(Observation {
                timestamp: 0.0,
                source_id: chunk_source,
                provenance: ProvenanceEnvelope {
                    origin: self.id.clone(),
                    chain: vec![],
                    sig: None,
                },
                payload,
                context: ctx.clone(),
                digest,
                schema_version: "1.0.0".to_string(),
                phase_hint: Some(phase),
            })
        }
    }

    let mut config = Config::default();
    config.calibration.enabled = true;
    config.calibration.target_pass_rate = 0.30;
    config.calibration.window = 20;
    config.calibration.warmup_ticks = 2;
    config.carrier.adaptive = true;
    config.thresholds.d = 0.05;
    config.thresholds.q = 0.05;
    config.thresholds.r = 0.05;
    config.thresholds.g = 0.05;
    config.thresholds.j = 0.05;
    config.thresholds.p = 0.05;
    config.thresholds.n = 0.05;
    config.thresholds.k = 0.05;
    config.consensus.consensus_threshold = 0.0;
    config.consensus.mirror_consistency_eta = 0.0;
    config.consensus.por_kappa_bar = 0.0;

    let adapter = TextPhaseAdapter {
        id: "llm-test".to_string(),
    };
    let mut state = GlobalState::new(&config);

    // Synthetic "LLM response" — 20 sentences, each a separate ASCII chunk.
    let sentences: Vec<Vec<u8>> = vec![
        b"Entropy is a fundamental concept in both thermodynamics and information theory.".to_vec(),
        b"In thermodynamics it measures the disorder of a physical system.".to_vec(),
        b"The second law states that entropy of an isolated system always increases.".to_vec(),
        b"Information entropy measures the average surprise in a probability distribution.".to_vec(),
        b"Both formulations share the log-probability structure and additivity over states.".to_vec(),
        b"Shannon borrowed the term from Boltzmann precisely because of this structural isomorphism.".to_vec(),
        b"The Gibbs entropy generalises the Boltzmann formula to continuous distributions.".to_vec(),
        b"Mutual information is the KL-divergence between joint and marginal distributions.".to_vec(),
        b"The channel capacity theorem links entropy to reliable communication rates.".to_vec(),
        b"Maxwell demon thought experiment connects information erasure with thermodynamic cost.".to_vec(),
        b"Landauer principle states that erasing one bit dissipates at least kT ln2 of heat.".to_vec(),
        b"Irreversibility in thermodynamics corresponds to information loss about microstates.".to_vec(),
        b"The arrow of time emerges from the asymmetry of the second law.".to_vec(),
        b"Statistical mechanics derives macroscopic entropy from microscopic probability counts.".to_vec(),
        b"Phase space volume is preserved by Hamiltonian dynamics via Liouville theorem.".to_vec(),
        b"Entropy maximisation subject to energy constraints yields the Boltzmann distribution.".to_vec(),
        b"Free energy minimisation drives spontaneous processes toward equilibrium.".to_vec(),
        b"Negentropy or syntropy is a measure of order imported from the environment.".to_vec(),
        b"Living systems maintain low entropy by exporting disorder to their surroundings.".to_vec(),
        b"Both thermodynamic and information entropy obey subadditivity and concavity.".to_vec(),
    ];

    let window = 4usize;
    let mut crystals_formed = 0usize;
    for i in 0..sentences.len().saturating_sub(window - 1) {
        let batch: Vec<Vec<u8>> = sentences[i..i + window].to_vec();
        if let Ok(Some(_)) = macro_step(&mut state, &batch, &config, &adapter) {
            crystals_formed += 1;
        }
    }
    assert!(
        crystals_formed >= 1,
        "expected ≥1 crystal with consensus_threshold=0; got 0. \
         Last gate: {:?}",
        state.last_gate
    );
}

//! E.6: introspective queries — the engine's "what do you see?" interface.
//!
//! Until E.6, PSE was strictly a write-mode system: feed observations,
//! get crystals out, no way to ask the engine about its current
//! internal resonance state. [`resonance_fingerprint`] closes that gap.
//! It returns a structured snapshot of:
//!
//!  - the **topology** of the current observation graph
//!    ([`pse_types::TopologySignature`]);
//!  - the **5D centroid** of the active vertex embedding —
//!    where the engine's geometric attention is pointing right now;
//!  - the **active Mandorla** (κ, Δφ, r) given the live data-helix —
//!    how strongly the carrier and data are currently in resonance;
//!  - the **data-helix** itself, if a batch has arrived;
//!  - the **top-K resonant crystals** from pattern memory, with their
//!    similarity scores — what stored patterns this moment looks like;
//!  - the **deviation field** from the top-1 hit — which axes of
//!    spectral signature are different from the closest stored
//!    pattern (the "what's missing" signal).
//!
//! The query is read-only by design: it does **not** update memory
//! statistics or mutate global state. Calling it during a research
//! session is safe and cheap.

use pse_cascade::{mandorla, mandorla_real};
use pse_memory::CrystalSignature;
use pse_types::{DataHelix, FiveDState, Hash256, MandorlaState, TopologySignature};
use serde::{Deserialize, Serialize};

use crate::GlobalState;
use pse_types::Config;

/// Structured snapshot of the engine's current resonance state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResonanceFingerprint {
    /// Topological signature of the current observation graph.
    pub topology: TopologySignature,
    /// 5D centroid (mean across all vertex embeddings, all axes in [0,1]).
    pub centroid: FiveDState,
    /// Number of vertices contributing to the centroid.
    pub centroid_n: usize,
    /// Active carrier index (into the phase-ladder).
    pub active_carrier: usize,
    /// Mandorla state given the active carrier and current data-helix.
    /// Real interference (E.2) when `data_helix` is `Some`,
    /// carrier-only fallback otherwise.
    pub mandorla: MandorlaState,
    /// Data-helix from the most recent batch (E.1). `None` on cold start.
    pub data_helix: Option<DataHelix>,
    /// Top-K (crystal_id, similarity) pairs from pattern memory,
    /// sorted by descending similarity. Empty if memory is empty.
    pub resonant_crystals: Vec<(Hash256, f64)>,
    /// Per-component deviation in 8-axis spectral-signature space
    /// `[spectral_gap, cheeger, kuramoto, prop_time, β₀, β₁, β₂, χ]`,
    /// computed as `current − top-1`. Empty if `resonant_crystals`
    /// is empty.
    pub deviation: Vec<f64>,
    /// Cross-session statistics from pattern memory at query time.
    /// Read-only snapshot.
    pub memory_size: usize,
}

/// Take a structured snapshot of the engine's current resonance state.
///
/// `k` controls how many resonant crystals to return; pass 0 to skip
/// the memory lookup entirely.
pub fn resonance_fingerprint(
    state: &GlobalState,
    config: &Config,
    k: usize,
) -> ResonanceFingerprint {
    let topology = state.graph.topology_signature();
    let (centroid, centroid_n) = compute_centroid(&state.graph.embedding);

    let active = &state.phase_ladder[state.active_carrier];
    let mandorla = match &state.last_data_helix {
        Some(d) => mandorla_real(
            &active.helix_a,
            &active.helix_b,
            d,
            config.carrier.lambda,
            config.carrier.mu_r,
            config.carrier.eta_r,
        ),
        None => mandorla(
            &active.helix_a,
            &active.helix_b,
            config.carrier.lambda,
            config.carrier.mu_r,
        ),
    };

    let memory_size = state.memory.len();

    // Build a candidate signature from the current state and query the
    // pattern memory's top-K. Read-only — does not perturb memory stats.
    let candidate = current_state_signature(
        &topology,
        mandorla.kappa,
        centroid_n,
    );
    let resonant_crystals = if k > 0 {
        state.memory.top_k(&candidate, k)
    } else {
        Vec::new()
    };

    // Deviation against the top-1 hit. We re-fetch the stored signature
    // (top_k returns ids+scores only). If memory does not retain the
    // top-1 (it cannot, in our index it always does), the deviation is
    // empty.
    let deviation = match resonant_crystals.first() {
        Some((id, _)) => match state.memory.signature_for(id) {
            Some(stored) => spectral_deviation(&candidate, stored),
            None => Vec::new(),
        },
        None => Vec::new(),
    };

    ResonanceFingerprint {
        topology,
        centroid,
        centroid_n,
        active_carrier: state.active_carrier,
        mandorla,
        data_helix: state.last_data_helix.clone(),
        resonant_crystals,
        deviation,
        memory_size,
    }
}

/// Compute the per-axis mean of all vertex embeddings.
///
/// Returns the centroid and the count of contributing vertices.
/// Axes are independent — the centroid of an empty graph is the zero
/// state with count 0.
fn compute_centroid(
    embedding: &std::collections::BTreeMap<pse_types::VertexId, FiveDState>,
) -> (FiveDState, usize) {
    let n = embedding.len();
    if n == 0 {
        return (FiveDState::default(), 0);
    }
    let inv = 1.0 / n as f64;
    let mut p = 0.0;
    let mut rho = 0.0;
    let mut omega = 0.0;
    let mut chi = 0.0;
    let mut eta = 0.0;
    for state in embedding.values() {
        p += state.p;
        rho += state.rho;
        omega += state.omega;
        chi += state.chi;
        eta += state.eta;
    }
    (
        FiveDState {
            p: p * inv,
            rho: rho * inv,
            omega: omega * inv,
            chi: chi * inv,
            eta: eta * inv,
        },
        n,
    )
}

/// Build a CrystalSignature from the current engine state, suitable
/// for comparison against stored crystal signatures.
fn current_state_signature(
    topology: &TopologySignature,
    kappa: f64,
    region_size: usize,
) -> CrystalSignature {
    CrystalSignature {
        crystal_id: [0u8; 32], // placeholder — this is a candidate, not stored
        spectral: vec![
            topology.spectral_gap,
            topology.cheeger_estimate,
            topology.kuramoto_coherence,
            topology.mean_propagation_time,
            topology.betti_0 as f64,
            topology.betti_1 as f64,
            topology.betti_2 as f64,
            topology.euler_char as f64,
        ],
        resonance: kappa,
        confidence: kappa, // best proxy in a query context
        content_hash: [0u8; 32],
        tick_range: (0, 0),
        observation_count: region_size,
    }
}

/// Per-axis difference `candidate.spectral − stored.spectral`, padded
/// or truncated to the shorter of the two vectors.
fn spectral_deviation(candidate: &CrystalSignature, stored: &CrystalSignature) -> Vec<f64> {
    let n = candidate.spectral.len().min(stored.spectral.len());
    (0..n)
        .map(|i| candidate.spectral[i] - stored.spectral[i])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GlobalState;
    use pse_graph::PassthroughAdapter;
    use pse_types::Config;

    #[test]
    fn empty_state_yields_well_formed_fingerprint() {
        let cfg = Config::default();
        let state = GlobalState::new(&cfg);
        let fp = resonance_fingerprint(&state, &cfg, 5);
        // No observations → no vertices.
        assert_eq!(fp.centroid_n, 0);
        // No data helix on cold start.
        assert!(fp.data_helix.is_none());
        // No resonant crystals (memory is empty).
        assert!(fp.resonant_crystals.is_empty());
        assert!(fp.deviation.is_empty());
        assert_eq!(fp.memory_size, 0);
        // Mandorla still defined: carrier-only fallback when data is None.
        assert!((0.0..=1.0).contains(&fp.mandorla.kappa));
    }

    #[test]
    fn fingerprint_after_observations_has_data_helix() {
        let cfg = Config::default();
        let mut state = GlobalState::new(&cfg);
        let adapter = PassthroughAdapter::new("query-test");
        let payload = serde_json::to_vec(&"hello").unwrap();
        let _ = crate::macro_step(&mut state, &[payload], &cfg, &adapter);
        let fp = resonance_fingerprint(&state, &cfg, 5);
        // E.1 should have populated the data-helix on this batch.
        assert!(fp.data_helix.is_some());
        let d = fp.data_helix.as_ref().unwrap();
        assert!(d.r >= 0.0 && d.r <= 1.0);
    }

    #[test]
    fn fingerprint_centroid_axes_in_unit_interval() {
        let cfg = Config::default();
        let mut state = GlobalState::new(&cfg);
        let adapter = PassthroughAdapter::new("centroid-test");
        for i in 0..5 {
            let payload = serde_json::to_vec(&format!("obs-{}", i)).unwrap();
            let _ = crate::macro_step(&mut state, &[payload], &cfg, &adapter);
        }
        let fp = resonance_fingerprint(&state, &cfg, 0);
        for axis in [
            fp.centroid.p,
            fp.centroid.rho,
            fp.centroid.omega,
            fp.centroid.chi,
            fp.centroid.eta,
        ] {
            assert!((0.0..=1.0).contains(&axis), "centroid axis out of range: {}", axis);
        }
    }

    #[test]
    fn fingerprint_query_does_not_alter_memory_stats() {
        // A fingerprint query is introspection, not learning — it must
        // not change hit/miss accounting.
        let cfg = Config::default();
        let mut state = GlobalState::new(&cfg);
        let adapter = PassthroughAdapter::new("stats-test");
        let payload = serde_json::to_vec(&"hello").unwrap();
        let _ = crate::macro_step(&mut state, &[payload], &cfg, &adapter);
        let stats_before = state.memory.stats().clone();
        let _fp = resonance_fingerprint(&state, &cfg, 5);
        let stats_after = state.memory.stats().clone();
        assert_eq!(stats_before.total_lookups, stats_after.total_lookups);
        assert_eq!(stats_before.hits, stats_after.hits);
        assert_eq!(stats_before.misses, stats_after.misses);
    }

    #[test]
    fn fingerprint_mandorla_uses_real_interference_when_data_present() {
        // After at least one observation, last_data_helix is Some, so
        // the Mandorla in the fingerprint must be the real-interference
        // form (E.2): κ ≤ κ_carrier_only, with the data-lock factors
        // applied. We test only the `≤` invariant — the exact value
        // depends on the random alignment of the data phase to the
        // carrier axis.
        let cfg = Config::default();
        let mut state = GlobalState::new(&cfg);
        let adapter = PassthroughAdapter::new("mandorla-real-test");
        let payload = serde_json::to_vec(&"data-arrives").unwrap();
        let _ = crate::macro_step(&mut state, &[payload], &cfg, &adapter);
        let fp = resonance_fingerprint(&state, &cfg, 0);
        let active = &state.phase_ladder[state.active_carrier];
        let carrier_only = mandorla(
            &active.helix_a,
            &active.helix_b,
            cfg.carrier.lambda,
            cfg.carrier.mu_r,
        );
        assert!(
            fp.mandorla.kappa <= carrier_only.kappa + 1e-12,
            "real κ={} must not exceed carrier-only κ={}",
            fp.mandorla.kappa,
            carrier_only.kappa
        );
    }
}

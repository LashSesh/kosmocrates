//! E.7: TRITON exploring the **actual resonance landscape**.
//!
//! Until E.7, the navigator's golden-angle spiral roamed an abstract
//! `[0, 1]^d` cube — its Fiedler-gradient and Betti-guard machinery
//! were physically meaningful, but the space they searched was
//! decoupled from the engine's real configuration.
//!
//! E.7 closes that loop. [`resonance_landscape_explorer`] takes a
//! snapshot of the live engine state (active carrier, phase ladder,
//! data-helix from the most recent batch) and constructs a 2-D
//! resonance landscape:
//!
//!  - **u-axis**: hypothetical carrier `α.phi` swept around `[0, 2π)`,
//!  - **v-axis**: hypothetical data-helix radius swept across `[0, 1]`.
//!
//! The evaluator at each landscape point returns the
//! [`SpectralSignature`] of the hypothetical PSE configuration:
//!
//!  - `psi = κ_real` from `mandorla_real` — the actual resonance
//!    coherence at that (carrier-φ, data-r) pair;
//!  - `rho = phase_lock_factor` — `(1 + cos(2·(φ_d − α.phi))) / 2`,
//!    measuring how cleanly the data sits on the carrier axis;
//!  - `omega = amp_match_factor` — `exp(−η_r · (r_d − α.r)²)`,
//!    measuring how cleanly the data amplitude matches the carrier.
//!
//! The TRITON spiral then explores this landscape with its existing
//! Fiedler-gradient momentum and Betti-guard — but the singularities
//! it detects are now singularities of *real resonance topology*, and
//! the best point it returns is a configuration the engine could
//! actually adopt to maximise its current coherence with the data.

use pse_navigator::{Navigator, NavigatorConfig, SpectralSignature};
use pse_types::{Config, DataHelix, TubusCoord};
use serde::{Deserialize, Serialize};

use crate::GlobalState;

/// One probe of the resonance landscape.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResonanceProbe {
    /// Landscape coordinate in `[0, 1]^2`: `[carrier_phi/2π, data_r]`.
    pub point: Vec<f64>,
    /// Spectral signature at this point — see module docs.
    pub signature: SpectralSignature,
    /// Convenience accessor: the κ at this probe (= signature.psi).
    pub kappa: f64,
}

/// Outcome of an exploration run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResonanceExplorationResult {
    /// Number of probes the spiral evaluated.
    pub probes_evaluated: usize,
    /// Highest κ found across all probes.
    pub best_kappa: f64,
    /// `[carrier_phi/2π, data_r]` of the best probe.
    pub best_point: Vec<f64>,
    /// Full per-step history (point + signature + κ).
    pub history: Vec<ResonanceProbe>,
    /// Betti numbers of the simplex mesh after the run — the topology
    /// of the *resonance landscape itself*, as seen by the navigator.
    pub final_betti: Vec<usize>,
    /// Number of spectral singularities detected during the run.
    pub singularities: usize,
}

/// Run the TRITON navigator over the live resonance landscape derived
/// from `state` and `config`.
///
/// `n_steps` controls how many golden-angle probes the spiral takes.
/// The seed for the spiral is derived deterministically from the
/// current `state.commit_index` so two runs at the same engine state
/// produce identical exploration histories — Inv I4-compatible.
pub fn resonance_landscape_explorer(
    state: &GlobalState,
    config: &Config,
    n_steps: usize,
) -> ResonanceExplorationResult {
    let active_carrier = state.phase_ladder[state.active_carrier].clone();
    let alpha = active_carrier.helix_a.clone();
    let beta = active_carrier.helix_b.clone();
    let cfg = config.carrier.clone();

    // Probe-data template: use the current data-helix if present, else
    // a neutral probe at the carrier radius and zero phase.
    let probe_phi = state.last_data_helix.as_ref().map(|d| d.phi).unwrap_or(0.0);
    let probe_tau = state
        .last_data_helix
        .as_ref()
        .map(|d| d.tau)
        .unwrap_or(state.t2);

    let evaluator = move |point: &[f64]| -> SpectralSignature {
        let u = point.first().copied().unwrap_or(0.5).clamp(0.0, 1.0);
        let v = point.get(1).copied().unwrap_or(0.5).clamp(0.0, 1.0);
        let two_pi = std::f64::consts::TAU;
        let phi_alpha = u * two_pi;
        let r_data = v;

        let alpha_p = TubusCoord {
            tau: alpha.tau,
            phi: phi_alpha,
            r: alpha.r,
        };
        let beta_p = TubusCoord {
            tau: beta.tau,
            phi: (phi_alpha + std::f64::consts::PI).rem_euclid(two_pi),
            r: beta.r,
        };
        let probe = DataHelix {
            tau: probe_tau,
            phi: probe_phi,
            r: r_data,
        };
        let m =
            pse_cascade::mandorla_real(&alpha_p, &beta_p, &probe, cfg.lambda, cfg.mu_r, cfg.eta_r);

        // Decompose into the three E.2 factors so the navigator's
        // resonance metric (psi × rho × omega) reflects the
        // physically-meaningful structure of the landscape.
        let phase_lock = 0.5 * (1.0 + (2.0 * (probe.phi - phi_alpha)).cos());
        let amp_match = (-cfg.eta_r * (probe.r - alpha.r).powi(2)).exp();

        SpectralSignature::new(
            m.kappa.clamp(0.0, 1.0),
            phase_lock.clamp(0.0, 1.0),
            amp_match.clamp(0.0, 1.0),
        )
    };

    let nav_config = NavigatorConfig {
        dim: 2,
        seed: 0xE7E7_E7E7_E7E7_E7E7_u64
            .wrapping_add(state.commit_index)
            .wrapping_add(state.active_carrier as u64),
        ..Default::default()
    };
    let mut navigator = Navigator::new(nav_config, evaluator);

    let mut singularities = 0usize;
    for _ in 0..n_steps {
        let step = navigator.step();
        if step.is_singularity {
            singularities += 1;
        }
    }

    let history: Vec<ResonanceProbe> = navigator
        .history
        .iter()
        .map(|step| ResonanceProbe {
            point: step.point.clone(),
            signature: step.signature.clone(),
            kappa: step.signature.psi,
        })
        .collect();

    let (best_kappa, best_point) = history
        .iter()
        .max_by(|a, b| {
            a.kappa
                .partial_cmp(&b.kappa)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|p| (p.kappa, p.point.clone()))
        .unwrap_or((0.0, vec![0.5, 0.5]));

    let final_betti = navigator.mesh.betti_numbers();

    ResonanceExplorationResult {
        probes_evaluated: history.len(),
        best_kappa,
        best_point,
        history,
        final_betti,
        singularities,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pse_graph::PassthroughAdapter;

    #[test]
    fn explorer_runs_to_completion_on_empty_state() {
        let cfg = Config::default();
        let state = GlobalState::new(&cfg);
        let r = resonance_landscape_explorer(&state, &cfg, 30);
        assert_eq!(r.probes_evaluated, 30);
        assert_eq!(r.history.len(), 30);
        assert_eq!(r.best_point.len(), 2);
        // Even on cold start the landscape is well-defined (carrier-only
        // fallback) and best κ should be positive.
        assert!(r.best_kappa > 0.0);
    }

    #[test]
    fn explorer_finds_higher_kappa_than_a_random_probe() {
        // After observations are ingested, the explorer's best probe
        // should beat a single naive midpoint probe — i.e., the
        // golden-angle spiral + Fiedler gradient is doing real work.
        let cfg = Config::default();
        let mut state = GlobalState::new(&cfg);
        let adapter = PassthroughAdapter::new("explore-test");
        for i in 0..5 {
            let payload = serde_json::to_vec(&format!("obs-{}", i)).unwrap();
            let _ = crate::macro_step(&mut state, &[payload], &cfg, &adapter);
        }
        let r = resonance_landscape_explorer(&state, &cfg, 60);

        // Reference: κ at the centre of the landscape (u=0.5, v=0.5).
        let active = &state.phase_ladder[state.active_carrier];
        let alpha = &active.helix_a;
        let beta = &active.helix_b;
        let probe_phi = state.last_data_helix.as_ref().map(|d| d.phi).unwrap_or(0.0);
        let phi_alpha = 0.5 * std::f64::consts::TAU;
        let alpha_p = TubusCoord {
            tau: alpha.tau,
            phi: phi_alpha,
            r: alpha.r,
        };
        let beta_p = TubusCoord {
            tau: beta.tau,
            phi: (phi_alpha + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU),
            r: beta.r,
        };
        let mid_data = DataHelix {
            tau: 0.0,
            phi: probe_phi,
            r: 0.5,
        };
        let midpoint_kappa = pse_cascade::mandorla_real(
            &alpha_p,
            &beta_p,
            &mid_data,
            cfg.carrier.lambda,
            cfg.carrier.mu_r,
            cfg.carrier.eta_r,
        )
        .kappa;

        // The navigator must do *at least* as well as the midpoint —
        // typically it does much better because the spiral covers the
        // landscape and the Fiedler-momentum biases toward higher κ.
        assert!(
            r.best_kappa >= midpoint_kappa - 1e-9,
            "spiral best κ={} should beat or tie midpoint κ={}",
            r.best_kappa,
            midpoint_kappa
        );
    }

    #[test]
    fn explorer_is_deterministic_at_same_engine_state() {
        // Inv I4 compatibility: same state → same exploration history.
        let cfg = Config::default();
        let mut state = GlobalState::new(&cfg);
        let adapter = PassthroughAdapter::new("det");
        let payload = serde_json::to_vec(&"hi").unwrap();
        let _ = crate::macro_step(&mut state, &[payload], &cfg, &adapter);

        let r1 = resonance_landscape_explorer(&state, &cfg, 25);
        let r2 = resonance_landscape_explorer(&state, &cfg, 25);
        assert_eq!(r1.history.len(), r2.history.len());
        for (a, b) in r1.history.iter().zip(r2.history.iter()) {
            assert_eq!(a.point, b.point);
            assert!((a.kappa - b.kappa).abs() < 1e-12);
        }
    }

    #[test]
    fn explorer_best_point_lies_in_landscape() {
        let cfg = Config::default();
        let state = GlobalState::new(&cfg);
        let r = resonance_landscape_explorer(&state, &cfg, 20);
        assert_eq!(r.best_point.len(), 2);
        for axis in &r.best_point {
            assert!(
                (0.0..=1.0).contains(axis),
                "best point out of [0,1]: {}",
                axis
            );
        }
    }

    #[test]
    fn explorer_history_kappas_match_best() {
        let cfg = Config::default();
        let state = GlobalState::new(&cfg);
        let r = resonance_landscape_explorer(&state, &cfg, 40);
        // The reported best_kappa must equal the maximum κ in the history.
        let max_in_history = r
            .history
            .iter()
            .map(|p| p.kappa)
            .fold(f64::NEG_INFINITY, f64::max);
        assert!((r.best_kappa - max_in_history).abs() < 1e-12);
    }
}

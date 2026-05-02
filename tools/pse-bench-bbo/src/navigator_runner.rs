//! Strand L: full TRITON navigator (spiral + simplex mesh + Fiedler-momentum
//! + Betti-guard) as a BBO optimizer.
//!
//! The pse-navigator [`Navigator`] does not fit the
//! [`crate::Optimizer`] trait directly because it drives its own
//! evaluator-call loop internally — the evaluator closure must
//! produce a [`SpectralSignature`] given a point in `[0, 1]^d`,
//! synchronously, while inside `Navigator::step()`. This module
//! adapts that shape to the BBO setting:
//!
//! - `run_navigator_optimization(seed, function, budget)` constructs
//!   a Navigator whose evaluator captures the test function and a
//!   shared time-step counter, then runs `step()` `budget` times,
//!   recording each probe + value into a [`Trajectory`] in the same
//!   format as `run_optimization`.
//!
//! The Trajectory's `optimizer` field is `"triton_full"`, so the
//! comparison binary reports it side-by-side with `triton_spiral`,
//! `random_search`, and `halton`.

use std::sync::{Arc, Mutex};

use pse_navigator::{Navigator, NavigatorConfig, SpectralSignature};

use crate::{TestFunction, Trajectory, TrajectoryStep};

/// Run a single optimization pass with the full TRITON navigator.
///
/// The navigator's evaluator computes `f(p, t)` for the test function
/// at the current time step `t`, transforms the result into a
/// SpectralSignature via `psi = exp(-value)` (so smaller objectives →
/// larger psi → more "resonant"), and returns it. The navigator then
/// runs its full machinery — golden-angle spiral generation,
/// k-NN simplex mesh growth, Fiedler-vector spectral gradient,
/// momentum update, Betti-guard topology safety — to choose the next
/// probe.
pub fn run_navigator_optimization(
    seed: u64,
    function: &dyn TestFunction,
    budget: usize,
) -> Trajectory {
    let t_state = Arc::new(Mutex::new(0_usize));
    let last_value = Arc::new(Mutex::new(0.0_f64));
    let f_min_at_t = Arc::new(Mutex::new(0.0_f64));

    // The evaluator captures the function reference by ordinary
    // borrow; the closure (and therefore the borrow) cannot outlive
    // this function, since the navigator is local to it.
    let last_value_for_eval = last_value.clone();
    let t_state_for_eval = t_state.clone();
    let evaluator = |point: &[f64]| -> SpectralSignature {
        let mut probe = [0.5_f64; 2];
        for i in 0..2.min(point.len()) {
            probe[i] = point[i].clamp(0.0, 1.0);
        }
        let t = *t_state_for_eval.lock().unwrap();
        let value = function.evaluate(&probe, t);
        *last_value_for_eval.lock().unwrap() = value;
        let psi = (-value).exp().clamp(0.0, 1.0);
        SpectralSignature::new(psi, 1.0, 1.0)
    };

    let nav_config = NavigatorConfig { dim: 2, seed, ..Default::default() };
    let mut navigator = Navigator::new(nav_config, evaluator);

    let mut steps = Vec::with_capacity(budget);
    let mut cumulative_regret = 0.0_f64;
    let mut best_value = f64::INFINITY;
    let mut best_point = vec![0.5_f64, 0.5];

    for t in 0..budget {
        *t_state.lock().unwrap() = t;
        *f_min_at_t.lock().unwrap() = function.f_min_at(t);
        let step = navigator.step();
        let value = *last_value.lock().unwrap();
        let f_min = *f_min_at_t.lock().unwrap();
        let step_regret = (value - f_min).max(0.0);
        cumulative_regret += step_regret;
        if value < best_value {
            best_value = value;
            best_point = step.point.clone();
        }
        steps.push(TrajectoryStep {
            t,
            point: step.point.clone(),
            value,
            simple_regret: (best_value - f_min).max(0.0),
            cumulative_regret,
        });
    }

    let final_t = budget.saturating_sub(1);
    let final_simple_regret = (best_value - function.f_min_at(final_t)).max(0.0);
    Trajectory {
        optimizer: "triton_full".to_string(),
        function: function.name().to_string(),
        budget,
        steps,
        final_simple_regret,
        final_cumulative_regret: cumulative_regret,
        best_point,
        best_value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::functions::{DriftRastrigin, Sphere};

    #[test]
    fn navigator_runs_to_completion_on_sphere() {
        let f = Sphere::default();
        let traj = run_navigator_optimization(42, &f, 30);
        assert_eq!(traj.optimizer, "triton_full");
        assert_eq!(traj.steps.len(), 30);
        assert!(traj.final_simple_regret >= 0.0);
        assert!(traj.final_cumulative_regret >= 0.0);
        for step in &traj.steps {
            for axis in &step.point {
                assert!((0.0..=1.0).contains(axis), "point out of [0,1]: {}", axis);
            }
        }
    }

    #[test]
    fn navigator_is_deterministic_under_seed() {
        let f = DriftRastrigin::default();
        let t1 = run_navigator_optimization(7, &f, 25);
        let t2 = run_navigator_optimization(7, &f, 25);
        assert_eq!(t1.steps.len(), t2.steps.len());
        for (a, b) in t1.steps.iter().zip(t2.steps.iter()) {
            assert!((a.value - b.value).abs() < 1e-12);
        }
        assert_eq!(t1.best_point, t2.best_point);
    }

    #[test]
    fn navigator_simple_regret_is_monotone_non_increasing_on_stationary() {
        let f = Sphere::default();
        let traj = run_navigator_optimization(42, &f, 50);
        for w in traj.steps.windows(2) {
            assert!(
                w[0].simple_regret + 1e-12 >= w[1].simple_regret,
                "simple regret jumped up on stationary function: {} → {}",
                w[0].simple_regret,
                w[1].simple_regret
            );
        }
    }
}

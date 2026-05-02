//! Black-box optimization benchmark for the TRITON navigator (Strand G).
//!
//! This crate compares TRITON — PSE's golden-angle spiral with
//! Fiedler-vector momentum and Betti-guard topology safety
//! (`pse-navigator`) — against two reference baselines on a small
//! battery of black-box test functions, including non-stationary
//! variants where the global optimum drifts during the run.
//!
//! All optimizers receive the *same* fixed budget (number of
//! evaluations) and *same* deterministic seed. Metrics:
//!
//!  - **simple regret**:    f(x*) − f_min, where x* is the best point
//!                          found by the optimizer at the end of the
//!                          budget.
//!  - **cumulative regret**: Σ (f(x_t) − f_min) over all t in 1..=T,
//!                           where x_t is the t-th evaluated point
//!                           and f_min is the *current* (possibly
//!                           drifting) global optimum at time t.
//!
//! Lower is better for both metrics. For non-stationary functions,
//! "best point found" is evaluated at the *final* time step.

use serde::{Deserialize, Serialize};

pub mod functions;
pub mod navigator_runner;
pub mod optimizers;

/// A scalar f64 evaluation of a (possibly non-stationary) test
/// function at a 2-D point.
pub type Evaluation = f64;

/// Trait for any 2-D black-box test function.
///
/// `t` is a discrete step index in `0..=T`; non-stationary functions
/// use it to drift their optimum, stationary ones simply ignore it.
pub trait TestFunction: Send + Sync {
    /// Human-readable name (e.g. "rastrigin", "drift_rastrigin").
    fn name(&self) -> &str;
    /// Evaluate at `point ∈ [0, 1]^2`, time step `t`.
    fn evaluate(&self, point: &[f64; 2], t: usize) -> Evaluation;
    /// Current global minimum at time `t` (used for regret computation).
    fn f_min_at(&self, t: usize) -> f64;
}

/// Trait for a 2-D black-box optimizer.
///
/// All optimizers must consume `budget` function evaluations and may
/// inspect each evaluation's value to inform the next probe. They
/// must be deterministic given a fixed seed.
pub trait Optimizer<'f> {
    /// Suggest the next point to evaluate, given the history of
    /// (point, value) pairs observed so far.
    fn next_point(&mut self, history: &[(Vec<f64>, f64)]) -> Vec<f64>;
    /// Free-form name for reporting.
    fn name(&self) -> &str;
}

/// Run an optimizer on a test function for `budget` evaluations.
/// Returns the trajectory of (point, value, simple_regret_at_step,
/// cumulative_regret_at_step) tuples.
pub fn run_optimization<'f, O: Optimizer<'f>>(
    optimizer: &mut O,
    function: &dyn TestFunction,
    budget: usize,
) -> Trajectory {
    let mut history: Vec<(Vec<f64>, f64)> = Vec::with_capacity(budget);
    let mut cumulative_regret = 0.0;
    let mut steps = Vec::with_capacity(budget);
    let mut best_value = f64::INFINITY;
    let mut best_point: Vec<f64> = vec![0.5, 0.5];

    for t in 0..budget {
        let point = optimizer.next_point(&history);
        let mut probe = [0.5_f64; 2];
        for i in 0..2.min(point.len()) {
            probe[i] = point[i].clamp(0.0, 1.0);
        }
        let value = function.evaluate(&probe, t);
        let f_min = function.f_min_at(t);
        let step_regret = (value - f_min).max(0.0);
        cumulative_regret += step_regret;
        if value < best_value {
            best_value = value;
            best_point = point.clone();
        }
        history.push((point, value));
        steps.push(TrajectoryStep {
            t,
            point: probe.to_vec(),
            value,
            simple_regret: (best_value - function.f_min_at(t)).max(0.0),
            cumulative_regret,
        });
    }

    let final_t = budget.saturating_sub(1);
    let final_simple_regret =
        (best_value - function.f_min_at(final_t)).max(0.0);
    Trajectory {
        optimizer: optimizer.name().to_string(),
        function: function.name().to_string(),
        budget,
        steps,
        final_simple_regret,
        final_cumulative_regret: cumulative_regret,
        best_point,
        best_value,
    }
}

/// One per-step record of an optimization run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrajectoryStep {
    pub t: usize,
    pub point: Vec<f64>,
    pub value: f64,
    pub simple_regret: f64,
    pub cumulative_regret: f64,
}

/// Full optimization run trajectory.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trajectory {
    pub optimizer: String,
    pub function: String,
    pub budget: usize,
    pub steps: Vec<TrajectoryStep>,
    pub final_simple_regret: f64,
    pub final_cumulative_regret: f64,
    pub best_point: Vec<f64>,
    pub best_value: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::functions::Sphere;
    use crate::optimizers::RandomSearch;

    #[test]
    fn run_optimization_records_full_trajectory() {
        let f = Sphere::default();
        let mut opt = RandomSearch::new(123, 2);
        let traj = run_optimization(&mut opt, &f, 50);
        assert_eq!(traj.budget, 50);
        assert_eq!(traj.steps.len(), 50);
        // Cumulative regret is monotone non-decreasing.
        for w in traj.steps.windows(2) {
            assert!(w[0].cumulative_regret <= w[1].cumulative_regret + 1e-12);
        }
        // Simple regret is monotone non-increasing on a stationary function.
        for w in traj.steps.windows(2) {
            assert!(w[0].simple_regret + 1e-12 >= w[1].simple_regret);
        }
        // Final simple regret is non-negative.
        assert!(traj.final_simple_regret >= 0.0);
    }

    #[test]
    fn run_optimization_clamps_points_to_unit_square() {
        let f = Sphere::default();
        let mut opt = RandomSearch::new(7, 2);
        let traj = run_optimization(&mut opt, &f, 10);
        for step in &traj.steps {
            for axis in &step.point {
                assert!((0.0..=1.0).contains(axis), "axis out of [0,1]: {}", axis);
            }
        }
    }
}

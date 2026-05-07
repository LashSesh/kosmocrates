//! Test functions for the BBO benchmark.
//!
//! All functions take `point ∈ [0, 1]^2` and return a scalar that the
//! optimizer is asked to minimize. Stationary functions ignore the
//! `t` parameter; non-stationary functions use it to drift the global
//! optimum.

use crate::TestFunction;

/// Sphere: `f(x, y) = (x − cx)² + (y − cy)²`. Convex; trivially solvable.
/// Default places the optimum at the centre of the unit square.
pub struct Sphere {
    pub cx: f64,
    pub cy: f64,
}

impl Default for Sphere {
    fn default() -> Self {
        Self { cx: 0.5, cy: 0.5 }
    }
}

impl TestFunction for Sphere {
    fn name(&self) -> &str {
        "sphere"
    }
    fn evaluate(&self, p: &[f64; 2], _t: usize) -> f64 {
        (p[0] - self.cx).powi(2) + (p[1] - self.cy).powi(2)
    }
    fn f_min_at(&self, _t: usize) -> f64 {
        0.0
    }
}

/// Rastrigin: highly multimodal, many local minima on a regular lattice.
/// Scaled to [0, 1]^2 so the global minimum sits at (0.5, 0.5).
pub struct Rastrigin;

impl TestFunction for Rastrigin {
    fn name(&self) -> &str {
        "rastrigin"
    }
    fn evaluate(&self, p: &[f64; 2], _t: usize) -> f64 {
        // Rescale [0,1] → [-5.12, 5.12] which is the canonical domain.
        let x = (p[0] - 0.5) * 10.24;
        let y = (p[1] - 0.5) * 10.24;
        let two_pi = std::f64::consts::TAU;
        20.0 + (x * x - 10.0 * (two_pi * x).cos()) + (y * y - 10.0 * (two_pi * y).cos())
    }
    fn f_min_at(&self, _t: usize) -> f64 {
        0.0
    }
}

/// Ackley: single global minimum surrounded by many local minima.
/// Scaled to [0, 1]^2 so the global minimum sits at (0.5, 0.5).
pub struct Ackley;

impl TestFunction for Ackley {
    fn name(&self) -> &str {
        "ackley"
    }
    fn evaluate(&self, p: &[f64; 2], _t: usize) -> f64 {
        let x = (p[0] - 0.5) * 10.0;
        let y = (p[1] - 0.5) * 10.0;
        let two_pi = std::f64::consts::TAU;
        let inner1 = -0.2 * (0.5 * (x * x + y * y)).sqrt();
        let inner2 = 0.5 * ((two_pi * x).cos() + (two_pi * y).cos());
        -20.0 * inner1.exp() - inner2.exp() + 20.0 + std::f64::consts::E
    }
    fn f_min_at(&self, _t: usize) -> f64 {
        0.0
    }
}

/// Drift Rastrigin: the global optimum precesses slowly along a circle
/// of radius `drift_amplitude` around the unit-square centre as `t`
/// increases. The function stays Rastrigin-like in shape; only the
/// optimum location changes. This is the canonical *non-stationary*
/// test for adaptive optimizers — TRITON's spectral momentum is built
/// to track such drifting structure.
pub struct DriftRastrigin {
    /// Radius of the drift circle in [0, 1] coordinates.
    pub drift_amplitude: f64,
    /// Angular speed: the optimum completes one full circuit every
    /// `period_steps` evaluations.
    pub period_steps: f64,
}

impl Default for DriftRastrigin {
    fn default() -> Self {
        Self {
            drift_amplitude: 0.15,
            period_steps: 60.0,
        }
    }
}

impl DriftRastrigin {
    fn optimum_at(&self, t: usize) -> (f64, f64) {
        let theta = (t as f64) / self.period_steps * std::f64::consts::TAU;
        let cx = 0.5 + self.drift_amplitude * theta.cos();
        let cy = 0.5 + self.drift_amplitude * theta.sin();
        (cx.clamp(0.0, 1.0), cy.clamp(0.0, 1.0))
    }
}

impl TestFunction for DriftRastrigin {
    fn name(&self) -> &str {
        "drift_rastrigin"
    }
    fn evaluate(&self, p: &[f64; 2], t: usize) -> f64 {
        let (cx, cy) = self.optimum_at(t);
        let x = (p[0] - cx) * 10.24;
        let y = (p[1] - cy) * 10.24;
        let two_pi = std::f64::consts::TAU;
        20.0 + (x * x - 10.0 * (two_pi * x).cos()) + (y * y - 10.0 * (two_pi * y).cos())
    }
    fn f_min_at(&self, _t: usize) -> f64 {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sphere_has_minimum_at_centre() {
        let f = Sphere::default();
        let v = f.evaluate(&[0.5, 0.5], 0);
        assert!(v.abs() < 1e-12);
    }

    #[test]
    fn rastrigin_global_minimum_at_origin() {
        let f = Rastrigin;
        let v = f.evaluate(&[0.5, 0.5], 0);
        assert!(v.abs() < 1e-9, "expected 0 at scaled-origin, got {}", v);
    }

    #[test]
    fn ackley_global_minimum_at_origin() {
        let f = Ackley;
        let v = f.evaluate(&[0.5, 0.5], 0);
        assert!(v.abs() < 1e-6, "expected ~0 at scaled-origin, got {}", v);
    }

    #[test]
    fn drift_rastrigin_optimum_moves_with_time() {
        let f = DriftRastrigin::default();
        let (x0, y0) = f.optimum_at(0);
        let (x_quarter, y_quarter) = f.optimum_at(15);
        // After a quarter period the optimum should have noticeably moved.
        assert!((x0 - x_quarter).abs() > 0.05 || (y0 - y_quarter).abs() > 0.05);
    }

    #[test]
    fn drift_rastrigin_minimum_value_remains_zero() {
        let f = DriftRastrigin::default();
        for t in [0, 10, 20, 50, 100] {
            let (cx, cy) = f.optimum_at(t);
            let v = f.evaluate(&[cx, cy], t);
            assert!(v.abs() < 1e-9, "drift opt at t={} gave value {}", t, v);
        }
    }
}

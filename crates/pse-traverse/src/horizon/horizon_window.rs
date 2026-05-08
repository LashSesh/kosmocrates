//! `EventHorizonWindowV3` and the `HorizonVisibility` operator.
//!
//! A window is a phase- and epoch-bounded sub-interval; the visibility
//! gate `G_visible` succeeds iff phase, epoch, amplitude, jitter and
//! visibility all lie inside the declared window.

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::phase_ray::PhaseRay;
use super::primitives::{fixed_ge, fixed_le, EpochRange, Fixed, HorizonError, StableId};
use super::run_descriptor::{HorizonRunDescriptorV3, HorizonWindowPolicyV3};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventHorizonWindowV3 {
    pub window_id: Hash256,
    pub carrier_id: StableId,
    pub epoch_range: EpochRange,
    pub phase_start: Fixed,
    pub phase_end: Fixed,
    pub min_amplitude: Fixed,
    pub max_jitter: Fixed,
    pub visibility_floor: Fixed,
}

impl EventHorizonWindowV3 {
    pub fn with_id(mut self) -> Result<Self, HorizonError> {
        let mut probe = self.clone();
        probe.window_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| HorizonError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.window_id = Hash256(id);
        Ok(self)
    }

    /// Permissive default window over `[0, 1)` phase and `[0, max_steps)`
    /// epoch range — used by tests and `HorizonWindowPolicyV3::AcceptAll`.
    pub fn permissive(carrier_id: impl Into<String>, max_epoch: u64) -> Result<Self, HorizonError> {
        let zero = Fixed::quantize(0.0, 9).map_err(|e| HorizonError::InvalidPrimitive {
            reason: e.to_string(),
        })?;
        let one = Fixed::quantize(1.0, 9).map_err(|e| HorizonError::InvalidPrimitive {
            reason: e.to_string(),
        })?;
        let small = Fixed::quantize(0.5, 9).map_err(|e| HorizonError::InvalidPrimitive {
            reason: e.to_string(),
        })?;
        let w = EventHorizonWindowV3 {
            window_id: Hash256::zero(),
            carrier_id: carrier_id.into(),
            epoch_range: EpochRange::new(0, max_epoch.max(1))?,
            phase_start: zero.clone(),
            phase_end: one,
            min_amplitude: zero.clone(),
            max_jitter: small,
            visibility_floor: zero,
        };
        w.with_id()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HorizonWindowReportV3 {
    pub report_id: Hash256,
    pub window_id: Hash256,
    pub ray_id: Hash256,
    pub phase_in_window: bool,
    pub epoch_in_range: bool,
    pub amplitude_ok: bool,
    pub jitter_ok: bool,
    pub visibility_ok: bool,
    pub passed: bool,
}

impl HorizonWindowReportV3 {
    pub fn with_id(mut self) -> Result<Self, HorizonError> {
        let mut probe = self.clone();
        probe.report_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| HorizonError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.report_id = Hash256(id);
        Ok(self)
    }
}

/// Evaluate one ray against one window. Implements
/// `G_visible(r, w)` from §7.3 of the spec.
pub fn evaluate_window(
    window: &EventHorizonWindowV3,
    ray: &PhaseRay,
) -> Result<HorizonWindowReportV3, HorizonError> {
    // Phase check: phase ∈ [phase_start, phase_end].
    let phase_in_window =
        fixed_ge(&ray.phase, &window.phase_start) && fixed_le(&ray.phase, &window.phase_end);
    let epoch_in_range = window.epoch_range.contains(ray.epoch);
    // Amplitude: rays use attenuation as the implicit amplitude proxy
    // (1/(epoch+1)). amplitude_ok ↔ attenuation ≥ min_amplitude.
    let amplitude_ok = fixed_ge(&ray.attenuation, &window.min_amplitude);
    // Jitter: boundary_tension stands in for phase jitter.
    let jitter_ok = fixed_le(&ray.boundary_tension, &window.max_jitter);
    // Visibility: ray.attenuation ≥ visibility_floor.
    let visibility_ok = fixed_ge(&ray.attenuation, &window.visibility_floor);
    let passed = phase_in_window && epoch_in_range && amplitude_ok && jitter_ok && visibility_ok;
    let report = HorizonWindowReportV3 {
        report_id: Hash256::zero(),
        window_id: window.window_id.clone(),
        ray_id: ray.ray_id.clone(),
        phase_in_window,
        epoch_in_range,
        amplitude_ok,
        jitter_ok,
        visibility_ok,
        passed,
    };
    report.with_id()
}

/// Evaluate the chart's rays against the descriptor's window policy.
/// Returns the per-ray reports and a single boolean verdict for
/// `G_visible`.
pub fn evaluate_horizon_windows(
    rays: &[PhaseRay],
    rd: &HorizonRunDescriptorV3,
) -> Result<(Vec<HorizonWindowReportV3>, bool), HorizonError> {
    match rd.policies.horizon_window_policy {
        HorizonWindowPolicyV3::AcceptAll => {
            let mut reports = Vec::with_capacity(rays.len());
            for ray in rays {
                let report = HorizonWindowReportV3 {
                    report_id: Hash256::zero(),
                    window_id: Hash256::zero(),
                    ray_id: ray.ray_id.clone(),
                    phase_in_window: true,
                    epoch_in_range: true,
                    amplitude_ok: true,
                    jitter_ok: true,
                    visibility_ok: true,
                    passed: true,
                };
                reports.push(report.with_id()?);
            }
            Ok((reports, !rays.is_empty()))
        }
        HorizonWindowPolicyV3::AllDeclared | HorizonWindowPolicyV3::AnyDeclared => {
            // Without explicit window declarations in the descriptor we
            // synthesise one window per carrier, covering every epoch
            // up to `max_steps`. The descriptor's
            // `max_phase_jitter` and `min_horizon_visibility`
            // thresholds are inherited so that tightening either of
            // them at descriptor level has the spec-defined effect of
            // closing windows.
            let mut reports = Vec::with_capacity(rays.len());
            let mut any_pass = false;
            let mut all_pass = !rays.is_empty();
            for ray in rays {
                let mut window = EventHorizonWindowV3::permissive(
                    ray.carrier_id.clone(),
                    rd.thresholds.max_steps,
                )?;
                window.max_jitter = rd.thresholds.max_phase_jitter.clone();
                window.visibility_floor = rd.thresholds.min_horizon_visibility.clone();
                let window = window.with_id()?;
                let report = evaluate_window(&window, ray)?;
                if report.passed {
                    any_pass = true;
                } else {
                    all_pass = false;
                }
                reports.push(report);
            }
            let passed = match rd.policies.horizon_window_policy {
                HorizonWindowPolicyV3::AllDeclared => all_pass,
                HorizonWindowPolicyV3::AnyDeclared => any_pass,
                HorizonWindowPolicyV3::AcceptAll => true,
            };
            Ok((reports, passed))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamic_state::Hash256;

    fn ray_with_phase(phase: f64) -> PhaseRay {
        let phase = Fixed::quantize(phase, 9).unwrap();
        PhaseRay {
            ray_id: Hash256::zero(),
            origin_null_id: Hash256::zero(),
            carrier_id: "c.test".into(),
            epoch: 1,
            phase,
            radial_depth: Fixed::zero(),
            attenuation: Fixed::quantize(0.5, 9).unwrap(),
            boundary_tension: Fixed::quantize(0.0, 9).unwrap(),
            gate_visibility: true,
            canonical_trace_hash: Hash256::zero(),
        }
        .with_id()
        .unwrap()
    }

    #[test]
    fn passes_inside_interval() {
        let w = EventHorizonWindowV3::permissive("c.test", 10).unwrap();
        let r = ray_with_phase(0.5);
        let rep = evaluate_window(&w, &r).unwrap();
        assert!(rep.passed);
    }

    #[test]
    fn fails_outside_interval() {
        let mut w = EventHorizonWindowV3::permissive("c.test", 10).unwrap();
        w.phase_start = Fixed::quantize(0.6, 9).unwrap();
        w.phase_end = Fixed::quantize(0.7, 9).unwrap();
        let w = w.with_id().unwrap();
        let r = ray_with_phase(0.5);
        let rep = evaluate_window(&w, &r).unwrap();
        assert!(!rep.passed);
        assert!(!rep.phase_in_window);
    }
}

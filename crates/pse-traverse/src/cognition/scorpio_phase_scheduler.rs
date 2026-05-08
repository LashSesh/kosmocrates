//! Layer C6 — Scorpio Phase Scheduler (§11).
//!
//! Neutralised: a deterministic phase + activation network. No
//! steganography, no hidden channels, no random masking. Every gate
//! input is a `Fixed`; every list is sorted before hashing.

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::primitives::{fixed_le, CognitionError, Fixed, StableId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TransportPolicy {
    Strict,
    Permissive,
    Disabled,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ActivationWindow {
    pub window_id: Hash256,
    pub carrier_id: StableId,
    pub phase_start: Fixed,
    pub phase_end: Fixed,
    pub active: bool,
    pub duty_cycle: Fixed,
}

impl ActivationWindow {
    pub fn new(
        carrier_id: impl Into<String>,
        phase_start: Fixed,
        phase_end: Fixed,
        active: bool,
        duty_cycle: Fixed,
    ) -> Result<Self, CognitionError> {
        let mut w = ActivationWindow {
            window_id: Hash256::zero(),
            carrier_id: carrier_id.into(),
            phase_start,
            phase_end,
            active,
            duty_cycle,
        };
        let id = content_address(&(
            &w.carrier_id,
            &w.phase_start,
            &w.phase_end,
            w.active,
            &w.duty_cycle,
        ))
        .map_err(|e| CognitionError::Canonicalization {
            reason: e.to_string(),
        })?;
        w.window_id = Hash256(id);
        Ok(w)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ResonanceOffset {
    pub offset_id: Hash256,
    pub carrier_id: StableId,
    pub offset: Fixed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScorpioPhaseScheduler {
    pub scheduler_id: Hash256,
    pub phase_metric_space: Hash256,
    pub activation_windows: Vec<ActivationWindow>,
    pub resonance_offsets: Vec<ResonanceOffset>,
    pub invariant_measure: Option<Hash256>,
    pub transport_policy: TransportPolicy,
}

impl ScorpioPhaseScheduler {
    pub fn with_id(mut self) -> Result<Self, CognitionError> {
        self.activation_windows.sort();
        self.resonance_offsets.sort();
        let mut probe = self.clone();
        probe.scheduler_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| CognitionError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.scheduler_id = Hash256(id);
        Ok(self)
    }

    /// Reference resonance test per §11.2:
    /// `R_ε(v, θ) = 1 ⇔ d_S1(θ, ψ(v)) < ε`.
    pub fn resonates(window: &ActivationWindow, phase: &Fixed, epsilon: &Fixed) -> bool {
        // d_S1 over a unit circle = min(|θ - ψ|, 1 - |θ - ψ|) when
        // phases live in [0, 1). For a window, we substitute the
        // window mid-point as ψ.
        let mid = mid_phase(&window.phase_start, &window.phase_end);
        let raw = super::primitives::fixed_abs_diff(phase, &mid);
        fixed_le(&raw, epsilon)
    }
}

fn mid_phase(a: &Fixed, b: &Fixed) -> Fixed {
    let sum = super::primitives::fixed_add(a, b);
    // divide by 2: multiply den by 2.
    match sum {
        Fixed::FixedI64 { raw, scale } => {
            // Promote to Rational so we can keep precision.
            Fixed::Rational {
                num: raw as i128,
                den: 2 * 10i128.pow(scale),
            }
        }
        Fixed::Rational { num, den } => Fixed::Rational {
            num,
            den: den.saturating_mul(2),
        },
    }
}

/// Build a deterministic scheduler stub with a single activation window
/// per carrier id in `carriers`.
pub fn build_scheduler_minimal(
    carriers: &[StableId],
    logical_step: u64,
) -> Result<ScorpioPhaseScheduler, CognitionError> {
    let mut windows = Vec::with_capacity(carriers.len());
    for c in carriers {
        windows.push(ActivationWindow::new(
            c.clone(),
            Fixed::quantize(0.0, 9).unwrap(),
            Fixed::quantize(1.0, 9).unwrap(),
            true,
            Fixed::quantize(0.5, 9).unwrap(),
        )?);
    }
    let phase_metric_space = Hash256(
        crate::canonical::content_address(&("scorpio.metric.space", logical_step)).map_err(
            |e| CognitionError::Canonicalization {
                reason: e.to_string(),
            },
        )?,
    );
    ScorpioPhaseScheduler {
        scheduler_id: Hash256::zero(),
        phase_metric_space,
        activation_windows: windows,
        resonance_offsets: Vec::new(),
        invariant_measure: None,
        transport_policy: TransportPolicy::Strict,
    }
    .with_id()
}

/// `VectorTunnelTransport` — the spec's transport packet (§11.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ReasonCode {
    ConstraintBridge,
    SpiralMemoryRecall,
    AttractorShortcut,
    HorizonAlignment,
    CarrierMigration,
    ProjectionPreparation,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct VectorTunnelTransport {
    pub transport_id: Hash256,
    pub source_node: Hash256,
    pub target_node: Hash256,
    pub path_hash: Hash256,
    pub ttl: u64,
    pub budget_cost: Fixed,
    pub reason_code: ReasonCode,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replays_byte_identical() {
        let a = build_scheduler_minimal(&["c.1".to_string(), "c.2".to_string()], 7).unwrap();
        let b = build_scheduler_minimal(&["c.2".to_string(), "c.1".to_string()], 7).unwrap();
        // Carrier order shouldn't matter — windows are sorted before
        // hashing.
        assert_eq!(a.scheduler_id, b.scheduler_id);
    }
}

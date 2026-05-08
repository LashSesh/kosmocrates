//! `PhaseRay` and the `PhaseRayLift` operator.
//!
//! Each ray is the deterministic image of `(NullCenter, Carrier, Epoch)`
//! into the hypertoroidal phase space. Rays carry phase, radial depth,
//! attenuation and boundary tension — all gate-relevant scalars, all
//! float-free.

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::horizon_chart::HorizonChart;
use super::phase_carrier_v3::{PhaseCarrierSpecV3, PhaseQuantizationV3};
use super::primitives::{Fixed, HorizonError, StableId};
use super::run_descriptor::HorizonRunDescriptorV3;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseRay {
    pub ray_id: Hash256,
    pub origin_null_id: Hash256,
    pub carrier_id: StableId,
    pub epoch: u64,
    pub phase: Fixed,
    pub radial_depth: Fixed,
    pub attenuation: Fixed,
    pub boundary_tension: Fixed,
    pub gate_visibility: bool,
    pub canonical_trace_hash: Hash256,
}

impl PhaseRay {
    pub fn with_id(mut self) -> Result<Self, HorizonError> {
        let mut probe = self.clone();
        probe.ray_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| HorizonError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.ray_id = Hash256(id);
        Ok(self)
    }
}

impl PartialOrd for PhaseRay {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PhaseRay {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Order by (carrier_id, epoch) so the canonical projection from
        // a chart is stable regardless of caller-side iteration order.
        self.carrier_id
            .cmp(&other.carrier_id)
            .then(self.epoch.cmp(&other.epoch))
    }
}

/// Phase-Ray-Lift: builds one ray per `(carrier, epoch)` pair. The
/// epoch sweep length is `min(thresholds.max_steps, 8)` for the default
/// canonical reference fibration so charts stay small in tests; callers
/// can extend by composing rays externally.
pub fn lift_phase_rays(
    chart: &HorizonChart,
    rd: &HorizonRunDescriptorV3,
) -> Result<Vec<PhaseRay>, HorizonError> {
    let sweep: u64 = std::cmp::min(rd.thresholds.max_steps, 8);
    let mut rays = Vec::with_capacity(chart.carriers.len() * sweep as usize);
    for carrier in &chart.carriers {
        for epoch in 0..sweep {
            rays.push(build_ray(&chart.null_center_id, carrier, epoch)?);
        }
    }
    rays.sort();
    Ok(rays)
}

fn build_ray(
    null_center_id: &Hash256,
    carrier: &PhaseCarrierSpecV3,
    epoch: u64,
) -> Result<PhaseRay, HorizonError> {
    // Phase = (frequency * epoch + phase_offset) mod 1, expressed
    // exactly as a CanonicalNumber::Rational. No floats touched.
    let f = &carrier.frequency;
    let off = &carrier.phase_offset;
    let raw_num = f
        .num
        .saturating_mul(epoch as i128)
        .saturating_mul(off.den)
        .saturating_add(off.num.saturating_mul(f.den));
    let raw_den = f.den.saturating_mul(off.den);
    let raw_den = if raw_den == 0 { 1 } else { raw_den };
    // mod 1: keep numerator within [0, raw_den)
    let modulated = raw_num.rem_euclid(raw_den);
    let phase = Fixed::Rational {
        num: modulated,
        den: raw_den,
    };

    let phase = match &carrier.quantization {
        PhaseQuantizationV3::Continuous => phase,
        PhaseQuantizationV3::KPolar { k } => quantise_kpolar(&phase, raw_den, *k),
        PhaseQuantizationV3::Tripolar { tau_low, tau_high } => {
            quantise_tripolar(&phase, tau_low, tau_high)
        }
        PhaseQuantizationV3::Quadrupolar { tau } => quantise_quadrupolar(&phase, tau),
    };

    // Radial depth: linear in epoch, scaled small.
    let radial_depth = Fixed::Rational {
        num: epoch as i128,
        den: 100,
    };
    // Attenuation: 1/(epoch+1) exactly.
    let attenuation = Fixed::Rational {
        num: 1,
        den: (epoch as i128) + 1,
    };
    // Boundary tension: 0 at epoch 0, growing slowly.
    let boundary_tension = Fixed::Rational {
        num: epoch as i128,
        den: 1000,
    };

    let canonical_trace_hash = Hash256(
        content_address(&(null_center_id, &carrier.carrier_id, epoch)).map_err(|e| {
            HorizonError::Canonicalization {
                reason: e.to_string(),
            }
        })?,
    );

    let ray = PhaseRay {
        ray_id: Hash256::zero(),
        origin_null_id: null_center_id.clone(),
        carrier_id: carrier.carrier_id.clone(),
        epoch,
        phase,
        radial_depth,
        attenuation,
        boundary_tension,
        gate_visibility: true,
        canonical_trace_hash,
    };
    ray.with_id()
}

fn quantise_kpolar(phase: &Fixed, raw_den: i128, k: u32) -> Fixed {
    if k == 0 {
        return phase.clone();
    }
    if let Fixed::Rational { num, den } = phase {
        // bin_index = floor(phase * k) where phase = num/den, both >= 0
        let bin = num.saturating_mul(k as i128) / *den;
        return Fixed::Rational {
            num: bin,
            den: k as i128,
        };
    }
    // For FixedI64 input we still need to land in [0, 1).
    let _ = raw_den;
    phase.clone()
}

fn quantise_tripolar(phase: &Fixed, tau_low: &Fixed, tau_high: &Fixed) -> Fixed {
    use std::cmp::Ordering::*;
    let bin = match (phase.cmp(tau_low), phase.cmp(tau_high)) {
        (Less, _) => 0,
        (_, Greater) => 2,
        _ => 1,
    };
    Fixed::Rational { num: bin, den: 3 }
}

fn quantise_quadrupolar(phase: &Fixed, tau: &Fixed) -> Fixed {
    let bin = if phase.cmp(tau) == std::cmp::Ordering::Less {
        0
    } else {
        1
    };
    Fixed::Rational { num: bin, den: 2 }
}

/// Fold the per-ray hashes into the chart's `phase_ray_hashes` field.
/// Returns the chart with rays attached and the chart_id refreshed.
pub fn attach_rays_to_chart(
    mut chart: HorizonChart,
    rays: &[PhaseRay],
) -> Result<HorizonChart, HorizonError> {
    let mut hashes: Vec<Hash256> = rays.iter().map(|r| r.ray_id.clone()).collect();
    hashes.sort();
    chart.phase_ray_hashes = hashes;
    chart.with_id()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::horizon::horizon_chart::null_center_unfold;
    use crate::horizon::run_descriptor::{HorizonPoliciesV3, HorizonThresholdsV3};
    use std::collections::BTreeMap;

    fn descriptor() -> HorizonRunDescriptorV3 {
        HorizonRunDescriptorV3 {
            run_id: "horizon.test".into(),
            projection_v2_report_hash: None,
            problem_spec_hash: Hash256::zero(),
            seed: 7,
            operator_versions: BTreeMap::new(),
            canonicalization_version: "horizon-v0.3".into(),
            thresholds: HorizonThresholdsV3::permissive(),
            policies: HorizonPoliciesV3::default_t4(),
        }
    }

    #[test]
    fn ray_hash_changes_with_epoch() {
        let rd = descriptor();
        let chart = null_center_unfold(&Hash256::zero(), &rd).unwrap();
        let rays = lift_phase_rays(&chart, &rd).unwrap();
        let r0 = rays.iter().find(|r| r.epoch == 0).unwrap();
        let r1 = rays.iter().find(|r| r.epoch == 1).unwrap();
        assert_ne!(r0.ray_id, r1.ray_id);
    }

    #[test]
    fn rays_are_sorted_by_carrier_then_epoch() {
        let rd = descriptor();
        let chart = null_center_unfold(&Hash256::zero(), &rd).unwrap();
        let rays = lift_phase_rays(&chart, &rd).unwrap();
        let mut prev: Option<&PhaseRay> = None;
        for r in &rays {
            if let Some(p) = prev {
                assert!(p <= r, "rays not sorted: {:?} > {:?}", p, r);
            }
            prev = Some(r);
        }
    }
}

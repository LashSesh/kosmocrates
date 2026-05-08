//! Layer C9 — Fixpoint Calibration, Attractor Migration, Carrier
//! Ladder (§14).
//!
//! `FixpointCalibrationShell` records a candidate config mutation
//! against the current one. The PoR-acceptance rule per §14.3:
//!
//! `Q' ≥ Q ∧ S' ≥ S − ε_S ∧ E' ≥ E_min ∧ PoR(C') = 1`.

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::primitives::{fixed_ge, fixed_le, fixed_sub, CognitionError, EvidenceRef, Fixed};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PerformanceTriplet {
    pub quality: Fixed,
    pub stability: Fixed,
    pub efficiency: Fixed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResonanceImpulse {
    pub impulse_id: Hash256,
    pub source_hash: Hash256,
    pub strength: Fixed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixpointCalibrationShell {
    pub calibration_id: Hash256,
    pub current_config_hash: Hash256,
    pub candidate_config_hash: Hash256,
    pub performance_triplet: PerformanceTriplet,
    pub contraction_region_hash: Hash256,
    pub por_acceptance: bool,
    pub resonance_impulse: Option<ResonanceImpulse>,
}

impl FixpointCalibrationShell {
    pub fn with_id(mut self) -> Result<Self, CognitionError> {
        let mut probe = self.clone();
        probe.calibration_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| CognitionError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.calibration_id = Hash256(id);
        Ok(self)
    }
}

/// PoR-acceptance per §14.3.
pub fn evaluate_por_acceptance(
    current: &PerformanceTriplet,
    candidate: &PerformanceTriplet,
    epsilon_s: &Fixed,
    e_min: &Fixed,
) -> bool {
    let q_ok = fixed_ge(&candidate.quality, &current.quality);
    let s_min = fixed_sub(&current.stability, epsilon_s);
    let s_ok = fixed_ge(&candidate.stability, &s_min);
    let e_ok = fixed_ge(&candidate.efficiency, e_min);
    q_ok && s_ok && e_ok
}

/// Build a calibration shell deterministically from current+candidate
/// triplets. Sets `por_acceptance` according to §14.3.
pub fn calibrate(
    current_config_hash: Hash256,
    candidate_config_hash: Hash256,
    current: PerformanceTriplet,
    candidate: PerformanceTriplet,
    epsilon_s: &Fixed,
    e_min: &Fixed,
) -> Result<FixpointCalibrationShell, CognitionError> {
    let accepted = evaluate_por_acceptance(&current, &candidate, epsilon_s, e_min);
    FixpointCalibrationShell {
        calibration_id: Hash256::zero(),
        current_config_hash,
        candidate_config_hash,
        performance_triplet: candidate,
        contraction_region_hash: Hash256::zero(),
        por_acceptance: accepted,
        resonance_impulse: None,
    }
    .with_id()
}

/// `CarrierMigrationPlan` per §14.2.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CarrierMigrationPlan {
    pub plan_id: Hash256,
    pub source_carrier: Hash256,
    pub target_carrier: Hash256,
    pub friction_score: Fixed,
    pub shock_score: Fixed,
    pub anti_drift_score: Fixed,
    pub migration_allowed: bool,
    pub evidence_refs: Vec<EvidenceRef>,
}

impl CarrierMigrationPlan {
    pub fn with_id(mut self) -> Result<Self, CognitionError> {
        let mut probe = self.clone();
        probe.plan_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| CognitionError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.plan_id = Hash256(id);
        Ok(self)
    }
}

/// Migration is admissible iff anti-drift is healthy AND friction +
/// shock are below their declared bounds.
pub fn evaluate_migration(
    plan: CarrierMigrationPlan,
    max_friction: &Fixed,
    max_shock: &Fixed,
    min_anti_drift: &Fixed,
) -> Result<CarrierMigrationPlan, CognitionError> {
    let mut p = plan;
    let admissible = fixed_le(&p.friction_score, max_friction)
        && fixed_le(&p.shock_score, max_shock)
        && fixed_ge(&p.anti_drift_score, min_anti_drift);
    p.migration_allowed = admissible;
    p.with_id()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(q: f64, s: f64, e: f64) -> PerformanceTriplet {
        PerformanceTriplet {
            quality: Fixed::quantize(q, 9).unwrap(),
            stability: Fixed::quantize(s, 9).unwrap(),
            efficiency: Fixed::quantize(e, 9).unwrap(),
        }
    }

    #[test]
    fn rejects_quality_regression() {
        let cur = t(0.8, 0.8, 0.8);
        let cand = t(0.7, 0.8, 0.8);
        let shell = calibrate(
            Hash256::zero(),
            Hash256::zero(),
            cur,
            cand,
            &Fixed::quantize(0.05, 9).unwrap(),
            &Fixed::quantize(0.5, 9).unwrap(),
        )
        .unwrap();
        assert!(!shell.por_acceptance);
    }

    #[test]
    fn carrier_migration_requires_antidrift() {
        let plan = CarrierMigrationPlan {
            plan_id: Hash256::zero(),
            source_carrier: Hash256::zero(),
            target_carrier: Hash256::zero(),
            friction_score: Fixed::quantize(0.1, 9).unwrap(),
            shock_score: Fixed::quantize(0.1, 9).unwrap(),
            anti_drift_score: Fixed::quantize(0.0, 9).unwrap(),
            migration_allowed: false,
            evidence_refs: vec![],
        };
        let p = evaluate_migration(
            plan,
            &Fixed::quantize(0.5, 9).unwrap(),
            &Fixed::quantize(0.5, 9).unwrap(),
            &Fixed::quantize(0.5, 9).unwrap(),
        )
        .unwrap();
        assert!(!p.migration_allowed);
    }
}

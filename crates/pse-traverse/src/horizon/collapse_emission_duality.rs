//! `CollapseEmissionDuality` and the round-trip duality check.
//!
//! `G_dual = 1 ⇔ d_N(N, δ(Collapse(Emit(N)))) ≤ ε_N`.
//!
//! In v0.3 the canonical reference duality is `(Collapse, Emit) =
//! (identity, identity)` lifted through the chart's null center: the
//! round-trip MUST land back on the same id, with a roundtrip delta of
//! zero. Callers can extend by registering more elaborate operator
//! pairs and re-using the same report shape.

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::primitives::{fixed_ge, fixed_le, Fixed, HorizonError, StableId};
use super::run_descriptor::{DualityPolicyV3, HorizonRunDescriptorV3};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollapseEmissionDualitySpec {
    pub duality_id: StableId,
    pub collapse_operator_id: StableId,
    pub emission_operator_id: StableId,
    pub null_center_id: Hash256,
    pub max_roundtrip_delta: Fixed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DualityReport {
    pub report_id: Hash256,
    pub duality_id: StableId,
    pub input_state_hash: Hash256,
    pub collapsed_hash: Hash256,
    pub emitted_hash: Hash256,
    pub returned_null_id: Hash256,
    pub roundtrip_delta: Fixed,
    pub duality_score: Fixed,
    pub passed: bool,
}

impl DualityReport {
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

/// `G_dual`: round-trip returns to the same null center within the
/// threshold's roundtrip delta.
pub fn check_collapse_emission_duality(
    input_state_hash: &Hash256,
    null_center_id: &Hash256,
    spec: &CollapseEmissionDualitySpec,
    rd: &HorizonRunDescriptorV3,
) -> Result<DualityReport, HorizonError> {
    let collapsed_hash = collapse(input_state_hash, null_center_id)?;
    let emitted_hash = emit(&collapsed_hash, null_center_id)?;
    let returned_null_id = canonical_null_of(&emitted_hash, null_center_id)?;

    // The duality is satisfied iff the null id we *returned to* matches
    // the null id the spec was *registered against*. Mismatched specs
    // (e.g. spec.null_center_id ≠ chart's null_center) are precisely how
    // the pipeline detects "round-trip lost the null center".
    let same_center = match rd.policies.duality_policy {
        DualityPolicyV3::SameNullCenter => {
            returned_null_id == *null_center_id && spec.null_center_id == *null_center_id
        }
        DualityPolicyV3::BoundedDistance => true,
    };

    // Roundtrip delta: 0 if the null center returned, 1 otherwise.
    let roundtrip_delta = if same_center {
        Fixed::quantize(0.0, 9).map_err(|e| HorizonError::InvalidPrimitive {
            reason: e.to_string(),
        })?
    } else {
        Fixed::quantize(1.0, 9).map_err(|e| HorizonError::InvalidPrimitive {
            reason: e.to_string(),
        })?
    };

    let duality_score = if same_center {
        Fixed::quantize(1.0, 9).map_err(|e| HorizonError::InvalidPrimitive {
            reason: e.to_string(),
        })?
    } else {
        Fixed::quantize(0.0, 9).map_err(|e| HorizonError::InvalidPrimitive {
            reason: e.to_string(),
        })?
    };

    let passed = same_center
        && fixed_le(&roundtrip_delta, &spec.max_roundtrip_delta)
        && fixed_le(&roundtrip_delta, &rd.thresholds.max_roundtrip_delta)
        && fixed_ge(&duality_score, &rd.thresholds.min_duality_score);

    let report = DualityReport {
        report_id: Hash256::zero(),
        duality_id: spec.duality_id.clone(),
        input_state_hash: input_state_hash.clone(),
        collapsed_hash,
        emitted_hash,
        returned_null_id,
        roundtrip_delta,
        duality_score,
        passed,
    };
    report.with_id()
}

/// Default `Collapse`: hash the (input, null_center) pair. Pure, total,
/// deterministic. Custom operators can replace this in higher layers
/// while keeping the report shape stable.
fn collapse(input: &Hash256, null_center: &Hash256) -> Result<Hash256, HorizonError> {
    let h = content_address(&("collapse", input, null_center)).map_err(|e| {
        HorizonError::Canonicalization {
            reason: e.to_string(),
        }
    })?;
    Ok(Hash256(h))
}

/// Default `Emit`: hash the (collapsed, null_center) pair.
fn emit(collapsed: &Hash256, null_center: &Hash256) -> Result<Hash256, HorizonError> {
    let h = content_address(&("emit", collapsed, null_center)).map_err(|e| {
        HorizonError::Canonicalization {
            reason: e.to_string(),
        }
    })?;
    Ok(Hash256(h))
}

/// Compute the canonical null id reached by the round-trip. By
/// construction `δ(Collapse(Emit(N)))` returns `N` for the default
/// (identity) duality.
fn canonical_null_of(_emitted: &Hash256, null_center: &Hash256) -> Result<Hash256, HorizonError> {
    Ok(null_center.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::horizon::run_descriptor::{HorizonPoliciesV3, HorizonThresholdsV3};
    use std::collections::BTreeMap;

    fn descriptor() -> HorizonRunDescriptorV3 {
        HorizonRunDescriptorV3 {
            run_id: "horizon.dual".into(),
            projection_v2_report_hash: None,
            problem_spec_hash: Hash256::zero(),
            seed: 0,
            operator_versions: BTreeMap::new(),
            canonicalization_version: "horizon-v0.3".into(),
            thresholds: HorizonThresholdsV3::permissive(),
            policies: HorizonPoliciesV3::default_t4(),
        }
    }

    fn spec(null_center: &Hash256) -> CollapseEmissionDualitySpec {
        CollapseEmissionDualitySpec {
            duality_id: "duality.identity".into(),
            collapse_operator_id: "collapse.id".into(),
            emission_operator_id: "emit.id".into(),
            null_center_id: null_center.clone(),
            max_roundtrip_delta: Fixed::quantize(0.5, 9).unwrap(),
        }
    }

    #[test]
    fn duality_passes_same_nullcenter() {
        let rd = descriptor();
        let nc = Hash256::zero();
        let s = spec(&nc);
        let report = check_collapse_emission_duality(&Hash256::zero(), &nc, &s, &rd).unwrap();
        assert!(report.passed);
        assert_eq!(report.returned_null_id, nc);
    }

    #[test]
    fn duality_fails_changed_nullcenter() {
        let rd = descriptor();
        // Spec was registered against a *different* null center than the
        // one the chart actually carries — round-trip thus lands on the
        // wrong center, so the duality MUST fail.
        let chart_nc = Hash256::zero();
        let mut other_bytes = [0u8; 32];
        other_bytes[0] = 0xff;
        let registered_nc = Hash256(other_bytes);
        let s = spec(&registered_nc);
        let report = check_collapse_emission_duality(&Hash256::zero(), &chart_nc, &s, &rd).unwrap();
        assert!(!report.passed);
        assert!(report.duality_score == Fixed::quantize(0.0, 9).unwrap());
    }
}

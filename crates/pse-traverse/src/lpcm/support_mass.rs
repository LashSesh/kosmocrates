//! Support mass vector computation (§10 of the spec).
//!
//! Raw support: w̃ᵢ = FA(i)·FT(i)·FS(i)·FR(i)·FP(i)·FG(i)·FH(i)  (§10.1)
//! Normalized:  wᵢ = w̃ᵢ / DU where DU = Σw̃ᵢ
//! Gate:        ZU = max(wᵢ) >= theta_activate and no tie.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::primitives::{fixed_add, fixed_div, fixed_mul, Fixed, Hash256, LpcmResult};

/// The 7 raw support factors for one candidate in a fragment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupportFactorVector {
    pub candidate_id: Hash256,
    /// FA: adjacency factor
    pub adjacency: Fixed,
    /// FT: topology factor
    pub topology: Fixed,
    /// FS: seam factor
    pub seam: Fixed,
    /// FR: resonance factor
    pub resonance: Fixed,
    /// FP: provenance factor
    pub provenance: Fixed,
    /// FG: gate factor
    pub gate: Fixed,
    /// FH: hysteresis factor
    pub hysteresis: Fixed,
}

impl SupportFactorVector {
    /// Compute raw support: product of all 7 factors (§10.1).
    ///
    /// w̃ᵢ = FA(i)·FT(i)·FS(i)·FR(i)·FP(i)·FG(i)·FH(i)
    ///
    /// A zero factor vetoes that support channel entirely.
    pub fn raw_support(&self) -> Fixed {
        let w = fixed_mul(&self.adjacency, &self.topology);
        let w = fixed_mul(&w, &self.seam);
        let w = fixed_mul(&w, &self.resonance);
        let w = fixed_mul(&w, &self.provenance);
        let w = fixed_mul(&w, &self.gate);
        fixed_mul(&w, &self.hysteresis)
    }
}

/// Normalized support mass vector for a fragment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupportMassVector {
    /// Content address of this vector.
    pub vector_id: Hash256,
    /// Fragment this vector belongs to.
    pub fragment_id: Hash256,
    /// candidate_id -> normalized mass (sums to 1 if valid and DU > 0).
    pub weights: BTreeMap<Hash256, Fixed>,
    /// Raw factor vectors (one per candidate).
    pub raw_factors: Vec<SupportFactorVector>,
    /// DU = sum of all raw supports.
    pub denominator: Fixed,
    /// Whether this vector is valid (DU > 0).
    pub valid: bool,
    /// Failure kind if not valid.
    pub failure: Option<super::LpcmFailureKind>,
}

/// Compute normalized support mass vector from raw factor vectors.
///
/// Returns ZeroAdmissibleSupport error if DU = 0.
pub fn compute_normalized_support(
    fragment_id: Hash256,
    raw_factors: Vec<SupportFactorVector>,
) -> LpcmResult<SupportMassVector> {
    use super::primitives::lpcm_content_address;

    if raw_factors.is_empty() {
        let vector_id = lpcm_content_address(&fragment_id)?;
        return Ok(SupportMassVector {
            vector_id,
            fragment_id,
            weights: BTreeMap::new(),
            raw_factors: vec![],
            denominator: Fixed::zero(),
            valid: false,
            failure: Some(super::LpcmFailureKind::ZeroAdmissibleSupport),
        });
    }

    // Sum raw supports.
    let mut denom = Fixed::zero();
    let raws: Vec<(Hash256, Fixed)> = raw_factors
        .iter()
        .map(|f| (f.candidate_id.clone(), f.raw_support()))
        .collect();

    for (_, r) in &raws {
        denom = fixed_add(&denom, r);
    }

    // DU must be > 0.
    if denom == Fixed::zero() {
        let vector_id = lpcm_content_address(&(&fragment_id, "zero-support"))?;
        return Ok(SupportMassVector {
            vector_id,
            fragment_id,
            weights: BTreeMap::new(),
            raw_factors,
            denominator: Fixed::zero(),
            valid: false,
            failure: Some(super::LpcmFailureKind::ZeroAdmissibleSupport),
        });
    }

    // Normalize.
    let mut weights = BTreeMap::new();
    for (cid, raw) in &raws {
        weights.insert(cid.clone(), fixed_div(raw, &denom));
    }

    let vector_id = lpcm_content_address(&(&fragment_id, &weights))?;
    Ok(SupportMassVector {
        vector_id,
        fragment_id,
        weights,
        raw_factors,
        denominator: denom,
        valid: true,
        failure: None,
    })
}

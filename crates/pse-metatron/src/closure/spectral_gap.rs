//! SpectralGapStitchReport — verifies that the spectral gap is preserved
//! or improved after stitching (G_gap gate).

use serde::{Deserialize, Serialize};

use super::primitives::{content_address, EvidenceRef, Fixed, Hash256, MetatronError};

/// Report on spectral gap change after stitching.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpectralGapStitchReport {
    pub report_id: Hash256,
    pub prior_gap: Fixed,
    pub post_gap: Fixed,
    /// post_gap − prior_gap (positive = improvement).
    pub delta_gap: Fixed,
    pub stitch_refs: Vec<Hash256>,
    pub tensor_delta_report_ref: Hash256,
    /// True when delta_gap ≥ 0 (gap preserved or improved).
    pub improved_or_preserved: bool,
    pub evidence_refs: Vec<EvidenceRef>,
}

impl SpectralGapStitchReport {
    pub fn build(
        prior_gap: Fixed,
        post_gap: Fixed,
        mut stitch_refs: Vec<Hash256>,
        tensor_delta_report_ref: Hash256,
        evidence_refs: Vec<EvidenceRef>,
    ) -> Result<Self, MetatronError> {
        stitch_refs.sort();

        use super::primitives::CanonicalNumber;
        let delta_gap = match (&post_gap, &prior_gap) {
            (
                CanonicalNumber::FixedI64 { raw: p, scale: ps },
                CanonicalNumber::FixedI64 { raw: q, scale: qs },
            ) => {
                let scale = ps.max(qs);
                let pn = *p * 10i64.pow(scale - ps);
                let qn = *q * 10i64.pow(scale - qs);
                CanonicalNumber::FixedI64 {
                    raw: pn - qn,
                    scale: *scale,
                }
            }
        };

        let improved_or_preserved = match &delta_gap {
            CanonicalNumber::FixedI64 { raw, .. } => *raw >= 0,
            _ => false,
        };

        let partial = SpectralGapStitchReport {
            report_id: Hash256::zero(),
            prior_gap,
            post_gap,
            delta_gap,
            stitch_refs,
            tensor_delta_report_ref,
            improved_or_preserved,
            evidence_refs,
        };
        let id = content_address(&partial)?;
        Ok(SpectralGapStitchReport {
            report_id: id,
            ..partial
        })
    }
}

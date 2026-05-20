//! LocalMonolithProjection — local closure of field-operator convergence.
//!
//! A projection is only system-relevant when it has been gated by
//! NCTCS/PhaseMatrix/Eval-Gates as traceable, evidence-linked and
//! replay-ready. It MUST NOT directly produce persistent state.

use serde::{Deserialize, Serialize};

use super::primitives::{content_address, EvidenceRef, Fixed, Hash256, MetatronError};

/// Local Monolith projection: L_i(t) = δ(D_i(t) · Ω_i(t) − Θ_i(t)).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalMonolithProjection {
    pub projection_id: Hash256,
    pub source_substrate_id: Hash256,
    pub local_operator_path_hash: Hash256,
    pub resonance_product: Fixed,
    pub convergence_envelope: Fixed,
    pub trigger_threshold: Fixed,
    /// True when the projection fired (resonance_product ≥ trigger_threshold).
    pub emitted: bool,
    /// Content-addressed reference to the materialized artifact (if emitted and gated).
    pub materialized_ref: Option<Hash256>,
    pub trace_ref: Hash256,
    pub evidence_refs: Vec<EvidenceRef>,
}

impl LocalMonolithProjection {
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        source_substrate_id: Hash256,
        local_operator_path_hash: Hash256,
        resonance_product: Fixed,
        convergence_envelope: Fixed,
        trigger_threshold: Fixed,
        materialized_ref: Option<Hash256>,
        trace_ref: Hash256,
        evidence_refs: Vec<EvidenceRef>,
    ) -> Result<Self, MetatronError> {
        use super::primitives::CanonicalNumber;
        let (CanonicalNumber::FixedI64 { raw: r, scale: rs }, CanonicalNumber::FixedI64 { raw: t, scale: ts }) =
            (&resonance_product, &trigger_threshold);
        let emitted = {
            let rn = *r * 10i64.pow(ts.saturating_sub(*rs));
            let tn = *t * 10i64.pow(rs.saturating_sub(*ts));
            rn >= tn
        };

        let partial = LocalMonolithProjection {
            projection_id: Hash256::zero(),
            source_substrate_id,
            local_operator_path_hash,
            resonance_product,
            convergence_envelope,
            trigger_threshold,
            emitted,
            materialized_ref,
            trace_ref,
            evidence_refs,
        };
        let id = content_address(&partial)?;
        Ok(LocalMonolithProjection {
            projection_id: id,
            ..partial
        })
    }
}

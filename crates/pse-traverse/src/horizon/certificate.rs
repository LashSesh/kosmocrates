//! Horizon Certificate.
//!
//! A `HorizonCertificate` is the *single* hash chain that ties a run's
//! descriptor, chart, crossing report, optional Projection-v0.2
//! certificate and optional finalized emission together. Tampering with
//! any of those changes the certificate id.

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::primitives::HorizonError;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HorizonCertificate {
    pub certificate_id: Hash256,
    pub rd_hash: Hash256,
    pub chart_id: Hash256,
    pub crossing_report_hash: Hash256,
    pub projection_v2_certificate_hash: Option<Hash256>,
    pub finalized_emission_hash: Option<Hash256>,
    pub replay_hash: Hash256,
}

impl HorizonCertificate {
    pub fn with_id(mut self) -> Result<Self, HorizonError> {
        let mut probe = self.clone();
        probe.certificate_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| HorizonError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.certificate_id = Hash256(id);
        Ok(self)
    }

    /// Build a certificate from hash inputs and recompute the id.
    pub fn build(
        rd_hash: Hash256,
        chart_id: Hash256,
        crossing_report_hash: Hash256,
        projection_v2_certificate_hash: Option<Hash256>,
        finalized_emission_hash: Option<Hash256>,
        replay_hash: Hash256,
    ) -> Result<Self, HorizonError> {
        HorizonCertificate {
            certificate_id: Hash256::zero(),
            rd_hash,
            chart_id,
            crossing_report_hash,
            projection_v2_certificate_hash,
            finalized_emission_hash,
            replay_hash,
        }
        .with_id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn certificate_id_is_function_of_inputs() {
        let cert_a = HorizonCertificate::build(
            Hash256::zero(),
            Hash256::zero(),
            Hash256::zero(),
            None,
            None,
            Hash256::zero(),
        )
        .unwrap();
        let cert_b = cert_a.clone();
        assert_eq!(cert_a.certificate_id, cert_b.certificate_id);

        let mut tampered = cert_a.clone();
        let mut alt = [0u8; 32];
        alt[0] = 1;
        tampered.crossing_report_hash = Hash256(alt);
        let tampered = tampered.with_id().unwrap();
        assert_ne!(cert_a.certificate_id, tampered.certificate_id);
    }
}

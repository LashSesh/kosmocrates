//! Primitive types for the TPT-MTL topology layer (PSE-TRAVERSE-TPT-MTL-04).
//!
//! Re-exports `Fixed` and `RationalFixed` from the dynamics layer so
//! the topology layer shares the same float-free numeric domain as
//! `horizon`. `Hash256` is the workspace canonical 256-bit hash.
//!
//! Determinism invariant: no platform floats, no wall-clock, no
//! non-seeded randomness in any type defined here.

use serde::{Deserialize, Serialize};

pub use crate::dynamic_state::{CanonicalNumber, Hash256};
pub use crate::horizon::primitives::{fixed_abs_diff, fixed_le, fixed_sub, RationalFixed};

/// `Fixed` alias: every gate-relevant scalar in the topology layer.
pub type Fixed = CanonicalNumber;

/// Content-addressed evidence reference (TPT-MTL §4.1).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TptEvidenceRef {
    pub evidence_id: Hash256,
    pub kind: String,
    pub source_digest: Hash256,
    pub trace_ref: Hash256,
}

/// Total error surface for the topology layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TopologyError {
    InvalidDescriptor { reason: String },
    InvalidWindow { reason: String },
    InvalidMesh { reason: String },
    InvalidMicroFiber { reason: String },
    TopologyGuardFailure { reason: String },
    DualizationFailure { reason: String },
    SeamFailure { reason: String },
    CarrierFailure { reason: String },
    GateFailure { reason: String },
    BoundaryFailure { reason: String },
    ReplayMismatch { reason: String },
    BudgetExhausted { reason: String },
    Canonicalization { reason: String },
    NullCenterViolation { reason: String },
}

impl std::fmt::Display for TopologyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TopologyError::InvalidDescriptor { reason } => {
                write!(f, "invalid descriptor: {reason}")
            }
            TopologyError::InvalidWindow { reason } => write!(f, "invalid window: {reason}"),
            TopologyError::InvalidMesh { reason } => write!(f, "invalid mesh: {reason}"),
            TopologyError::InvalidMicroFiber { reason } => {
                write!(f, "invalid micro fiber: {reason}")
            }
            TopologyError::TopologyGuardFailure { reason } => {
                write!(f, "topology guard failure: {reason}")
            }
            TopologyError::DualizationFailure { reason } => {
                write!(f, "dualization failure: {reason}")
            }
            TopologyError::SeamFailure { reason } => write!(f, "seam failure: {reason}"),
            TopologyError::CarrierFailure { reason } => write!(f, "carrier failure: {reason}"),
            TopologyError::GateFailure { reason } => write!(f, "gate failure: {reason}"),
            TopologyError::BoundaryFailure { reason } => write!(f, "boundary failure: {reason}"),
            TopologyError::ReplayMismatch { reason } => write!(f, "replay mismatch: {reason}"),
            TopologyError::BudgetExhausted { reason } => write!(f, "budget exhausted: {reason}"),
            TopologyError::Canonicalization { reason } => {
                write!(f, "canonicalization: {reason}")
            }
            TopologyError::NullCenterViolation { reason } => {
                write!(f, "null center violation: {reason}")
            }
        }
    }
}

impl std::error::Error for TopologyError {}

/// Helper: compute content address via JCS + SHA-256.
pub fn tpt_content_address<T: serde::Serialize>(value: &T) -> Result<Hash256, TopologyError> {
    crate::canonical::content_address(value)
        .map(Hash256)
        .map_err(|e| TopologyError::Canonicalization {
            reason: e.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tpt_content_address_is_deterministic() {
        let v = ("topology", "test", 42u64);
        let h1 = tpt_content_address(&v).unwrap();
        let h2 = tpt_content_address(&v).unwrap();
        assert_eq!(h1, h2);
    }
}

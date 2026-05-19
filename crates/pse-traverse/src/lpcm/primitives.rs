//! LPCM primitive types: Hash256, StableId, Fixed, EvidenceRef, ContentAddressed.

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

pub use crate::dynamic_state::{CanonicalNumber, Hash256};
pub use crate::cognition::primitives::{
    fixed_add, fixed_ge, fixed_le, fixed_sub, max_fixed, min_fixed, normalize_rational,
    EvidenceRef,
};

/// Fixed-point canonical scalar (alias for CanonicalNumber).
pub type Fixed = CanonicalNumber;

/// Declarative stable identifier string.
pub type StableId = String;

/// Content-addressed wrapper: pairs a SHA-256 id with a typed value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentAddressed<T> {
    pub id: Hash256,
    pub value: T,
}

/// LPCM layer error surface. All fail-paths map to this.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LpcmError {
    InvalidDescriptor { reason: String },
    InvalidWindow { reason: String },
    PartitionFailed { reason: String },
    TopologyGuardFailed { reason: String },
    NoCandidateDirections { reason: String },
    ZeroAdmissibleSupport { reason: String },
    NoMajority { reason: String },
    TieNoCondense { reason: String },
    SeamBreak { reason: String },
    FalsePercolation { reason: String },
    CoarseGrainConflict { reason: String },
    ReplayMismatch { reason: String },
    Canonicalization { reason: String },
    DeterminismViolation { reason: String },
}

impl std::fmt::Display for LpcmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LpcmError::InvalidDescriptor { reason } => write!(f, "invalid descriptor: {reason}"),
            LpcmError::InvalidWindow { reason } => write!(f, "invalid window: {reason}"),
            LpcmError::PartitionFailed { reason } => write!(f, "partition failed: {reason}"),
            LpcmError::TopologyGuardFailed { reason } => write!(f, "topology guard failed: {reason}"),
            LpcmError::NoCandidateDirections { reason } => write!(f, "no candidate directions: {reason}"),
            LpcmError::ZeroAdmissibleSupport { reason } => write!(f, "zero admissible support: {reason}"),
            LpcmError::NoMajority { reason } => write!(f, "no majority: {reason}"),
            LpcmError::TieNoCondense { reason } => write!(f, "tie no condense: {reason}"),
            LpcmError::SeamBreak { reason } => write!(f, "seam break: {reason}"),
            LpcmError::FalsePercolation { reason } => write!(f, "false percolation: {reason}"),
            LpcmError::CoarseGrainConflict { reason } => write!(f, "coarse grain conflict: {reason}"),
            LpcmError::ReplayMismatch { reason } => write!(f, "replay mismatch: {reason}"),
            LpcmError::Canonicalization { reason } => write!(f, "canonicalization: {reason}"),
            LpcmError::DeterminismViolation { reason } => write!(f, "determinism violation: {reason}"),
        }
    }
}

impl std::error::Error for LpcmError {}

pub type LpcmResult<T> = Result<T, LpcmError>;

/// Compute a SHA-256 content address for any serializable value (via JCS).
pub fn lpcm_content_address<T: Serialize>(value: &T) -> LpcmResult<Hash256> {
    let bytes = serde_jcs::to_vec(value)
        .map_err(|e| LpcmError::Canonicalization { reason: e.to_string() })?;
    let digest: [u8; 32] = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(&bytes);
        h.finalize().into()
    };
    Ok(Hash256(digest))
}

/// Multiply two Fixed values (exact i128 rational arithmetic, normalised).
pub fn fixed_mul(a: &Fixed, b: &Fixed) -> Fixed {
    let (an, ad) = as_rational(a);
    let (bn, bd) = as_rational(b);
    let num = an.saturating_mul(bn);
    let den = ad.saturating_mul(bd);
    normalize_rational(num, den)
}

/// Divide a by b; returns zero if b is zero.
pub fn fixed_div(a: &Fixed, b: &Fixed) -> Fixed {
    let (an, ad) = as_rational(a);
    let (bn, bd) = as_rational(b);
    if bn == 0 {
        return Fixed::zero();
    }
    let num = an.saturating_mul(bd);
    let den = ad.saturating_mul(bn);
    normalize_rational(num, den)
}

fn as_rational(f: &Fixed) -> (i128, i128) {
    match f {
        CanonicalNumber::FixedI64 { raw, scale } => (*raw as i128, 10i128.pow(*scale)),
        CanonicalNumber::Rational { num, den } => (*num, *den),
    }
}

/// Convert a float in [0,1] to Fixed (scale=9).
pub fn fixed_from_f64(v: f64) -> Fixed {
    Fixed::quantize_default(v).unwrap_or(Fixed::zero())
}

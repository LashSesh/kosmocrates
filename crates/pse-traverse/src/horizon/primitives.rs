//! Primitive types for the null-centered horizon geometry.
//!
//! Per `specs/pse_traverse_horizon_spec_v0_3.pdf`:
//!
//! * `Fixed` — every gate-relevant scalar; reuses [`CanonicalNumber`] from
//!   the dynamics layer so the entire workspace shares one float-free
//!   numeric domain.
//! * `RationalFixed` — exact rational used for carrier frequencies and
//!   phase offsets; carried as `(num, den)` with `den > 0`.
//! * `EpochRange` — half-open epoch interval used by horizon windows.
//! * `HorizonEvidenceRef` — content-addressed reference into the
//!   provenance ledger.
//! * `HorizonError` — total error surface for the horizon layer.
//!
//! Audit-pathway invariant: every value here is comparable, hashable and
//! canonically serialisable without touching IEEE-754 floats.

use serde::{Deserialize, Serialize};

use crate::dynamic_state::{CanonicalNumber, Hash256};

/// `Fixed` re-export: the spec's `Fixed = CanonicalNumber`.
pub type Fixed = CanonicalNumber;

/// `StableId` re-export: the spec's `StableId = String`. Carrier ids,
/// duality ids and other declarative names live here.
pub type StableId = String;

/// Exact rational scalar (`num/den`, `den > 0`). Used by carrier
/// frequencies and phase offsets — anywhere the spec requires "rational
/// frequency". `i128` keeps cross-platform determinism intact;
/// numerator and denominator are serialised as decimal strings so JCS
/// (RFC 8785) can canonicalise them without precision loss.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RationalFixed {
    #[serde(serialize_with = "ser_i128_str", deserialize_with = "de_i128_str")]
    pub num: i128,
    #[serde(serialize_with = "ser_i128_str", deserialize_with = "de_i128_str")]
    pub den: i128,
}

fn ser_i128_str<S: serde::Serializer>(v: &i128, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&v.to_string())
}

fn de_i128_str<'de, D: serde::Deserializer<'de>>(d: D) -> Result<i128, D::Error> {
    let s = String::deserialize(d)?;
    s.parse::<i128>().map_err(serde::de::Error::custom)
}

impl RationalFixed {
    pub fn new(num: i128, den: i128) -> Result<Self, HorizonError> {
        if den == 0 {
            return Err(HorizonError::InvalidPrimitive {
                reason: "RationalFixed denominator must be non-zero".into(),
            });
        }
        let (num, den) = if den < 0 { (-num, -den) } else { (num, den) };
        Ok(RationalFixed { num, den })
    }

    pub fn zero() -> Self {
        RationalFixed { num: 0, den: 1 }
    }

    pub fn one() -> Self {
        RationalFixed { num: 1, den: 1 }
    }

    pub fn to_canonical(&self) -> Fixed {
        Fixed::Rational {
            num: self.num,
            den: self.den,
        }
    }
}

/// Half-open epoch interval `[start, end)`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EpochRange {
    pub start: u64,
    pub end: u64,
}

impl EpochRange {
    pub fn new(start: u64, end: u64) -> Result<Self, HorizonError> {
        if end < start {
            return Err(HorizonError::InvalidPrimitive {
                reason: format!("EpochRange end ({end}) < start ({start})"),
            });
        }
        Ok(EpochRange { start, end })
    }

    pub fn contains(&self, epoch: u64) -> bool {
        epoch >= self.start && epoch < self.end
    }
}

/// Content-addressed evidence reference into the provenance ledger.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HorizonEvidenceRef {
    pub kind: String,
    pub address: Hash256,
    pub relation: String,
}

/// Error surface for the horizon layer. Every fail-path lands here so
/// the caller can map to a fail-closed outcome deterministically.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HorizonError {
    InvalidPrimitive { reason: String },
    InvalidDescriptor { reason: String },
    InvalidChart { reason: String },
    Canonicalization { reason: String },
    DeterminismViolation { reason: String },
    ReplayMismatch { reason: String },
    Schema { reason: String },
}

impl std::fmt::Display for HorizonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HorizonError::InvalidPrimitive { reason } => write!(f, "invalid primitive: {reason}"),
            HorizonError::InvalidDescriptor { reason } => write!(f, "invalid descriptor: {reason}"),
            HorizonError::InvalidChart { reason } => write!(f, "invalid chart: {reason}"),
            HorizonError::Canonicalization { reason } => write!(f, "canonicalization: {reason}"),
            HorizonError::DeterminismViolation { reason } => {
                write!(f, "determinism violation: {reason}")
            }
            HorizonError::ReplayMismatch { reason } => write!(f, "replay mismatch: {reason}"),
            HorizonError::Schema { reason } => write!(f, "schema: {reason}"),
        }
    }
}

impl std::error::Error for HorizonError {}

/// Subtract `b` from `a` in i128 rational space. The result is the exact
/// rational, returned as a [`Fixed::Rational`] so further gates can keep
/// comparing without floats.
pub fn fixed_sub(a: &Fixed, b: &Fixed) -> Fixed {
    let (an, ad) = as_rational(a);
    let (bn, bd) = as_rational(b);
    Fixed::Rational {
        num: an.saturating_mul(bd).saturating_sub(bn.saturating_mul(ad)),
        den: ad.saturating_mul(bd),
    }
}

/// Absolute difference `|a - b|` as a [`Fixed`].
pub fn fixed_abs_diff(a: &Fixed, b: &Fixed) -> Fixed {
    fixed_sub(a, b).abs()
}

/// Returns true iff `a <= b`. Implemented via the existing canonical
/// `Ord` so the comparison is float-free.
pub fn fixed_le(a: &Fixed, b: &Fixed) -> bool {
    a.cmp(b) != std::cmp::Ordering::Greater
}

/// Returns true iff `a >= b`. Implemented via the existing canonical `Ord`.
pub fn fixed_ge(a: &Fixed, b: &Fixed) -> bool {
    a.cmp(b) != std::cmp::Ordering::Less
}

fn as_rational(x: &Fixed) -> (i128, i128) {
    match x {
        Fixed::FixedI64 { raw, scale } => {
            let den = 10i128.pow(*scale);
            (*raw as i128, den)
        }
        Fixed::Rational { num, den } => (*num, *den),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rational_fixed_normalises_negative_denominator() {
        let r = RationalFixed::new(1, -2).unwrap();
        assert_eq!(r.num, -1);
        assert_eq!(r.den, 2);
    }

    #[test]
    fn rational_fixed_rejects_zero_denominator() {
        assert!(RationalFixed::new(1, 0).is_err());
    }

    #[test]
    fn epoch_range_contains_half_open() {
        let r = EpochRange::new(2, 5).unwrap();
        assert!(!r.contains(1));
        assert!(r.contains(2));
        assert!(r.contains(4));
        assert!(!r.contains(5));
    }

    #[test]
    fn fixed_sub_is_floatfree() {
        let a = Fixed::quantize(1.5, 9).unwrap();
        let b = Fixed::quantize(0.5, 9).unwrap();
        let diff = fixed_sub(&a, &b);
        assert_eq!(diff, Fixed::quantize(1.0, 9).unwrap());
    }

    #[test]
    fn fixed_abs_diff_is_symmetric() {
        let a = Fixed::quantize(0.25, 9).unwrap();
        let b = Fixed::quantize(0.75, 9).unwrap();
        assert_eq!(fixed_abs_diff(&a, &b), fixed_abs_diff(&b, &a));
    }
}

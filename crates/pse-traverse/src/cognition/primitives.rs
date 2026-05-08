//! Primitive types for the panoptic phase cognition kernel.
//!
//! The cognition layer reuses [`CanonicalNumber`] from the dynamics
//! layer as `Fixed`, plus a small set of layer-local helpers. Per the
//! spec (§5.1) `StableId = String` and `Fixed = i128`-compatible
//! `CanonicalNumber`.

use serde::{Deserialize, Serialize};

use crate::dynamic_state::{CanonicalNumber, Hash256};

/// `Fixed` re-export — every gate-relevant scalar travels as a
/// canonical, float-free number.
pub type Fixed = CanonicalNumber;

/// `StableId` — declarative names (carrier ids, infogene ids, etc.).
pub type StableId = String;

/// Content-addressed evidence reference. Same shape as the horizon
/// layer's evidence ref but kept layer-local so cognition compiles
/// without importing horizon.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub kind: String,
    pub address: Hash256,
    pub relation: String,
}

/// Total error surface for the cognition layer. Every fail-path lands
/// here so the caller can map to a fail-closed `CognitionHoldReport`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CognitionError {
    InvalidPrimitive { reason: String },
    InvalidDescriptor { reason: String },
    InvalidState { reason: String },
    Canonicalization { reason: String },
    OperatorTypeMismatch { reason: String },
    DeterminismViolation { reason: String },
    ReplayMismatch { reason: String },
    Schema { reason: String },
}

impl std::fmt::Display for CognitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CognitionError::InvalidPrimitive { reason } => write!(f, "invalid primitive: {reason}"),
            CognitionError::InvalidDescriptor { reason } => {
                write!(f, "invalid descriptor: {reason}")
            }
            CognitionError::InvalidState { reason } => write!(f, "invalid state: {reason}"),
            CognitionError::Canonicalization { reason } => write!(f, "canonicalization: {reason}"),
            CognitionError::OperatorTypeMismatch { reason } => {
                write!(f, "operator type mismatch: {reason}")
            }
            CognitionError::DeterminismViolation { reason } => {
                write!(f, "determinism violation: {reason}")
            }
            CognitionError::ReplayMismatch { reason } => write!(f, "replay mismatch: {reason}"),
            CognitionError::Schema { reason } => write!(f, "schema: {reason}"),
        }
    }
}

impl std::error::Error for CognitionError {}

/// Subtract `b` from `a` exactly in i128 rational space, normalised
/// by gcd so the result fits comfortably in i128 even after repeated
/// composition.
pub fn fixed_sub(a: &Fixed, b: &Fixed) -> Fixed {
    let (an, ad) = as_rational(a);
    let (bn, bd) = as_rational(b);
    let num = an.saturating_mul(bd).saturating_sub(bn.saturating_mul(ad));
    let den = ad.saturating_mul(bd);
    normalize_rational(num, den)
}

/// Add `a + b` exactly in i128 rational space, normalised by gcd.
pub fn fixed_add(a: &Fixed, b: &Fixed) -> Fixed {
    let (an, ad) = as_rational(a);
    let (bn, bd) = as_rational(b);
    let num = an.saturating_mul(bd).saturating_add(bn.saturating_mul(ad));
    let den = ad.saturating_mul(bd);
    normalize_rational(num, den)
}

/// Reduce a rational `num/den` by their gcd; ensures `den > 0`. Returns
/// a `Fixed::Rational`.
pub fn normalize_rational(num: i128, den: i128) -> Fixed {
    if den == 0 {
        return Fixed::Rational { num: 0, den: 1 };
    }
    let (mut n, mut d) = if den < 0 { (-num, -den) } else { (num, den) };
    let g = gcd_i128(n.unsigned_abs(), d.unsigned_abs());
    if g > 1 {
        n /= g as i128;
        d /= g as i128;
    }
    Fixed::Rational { num: n, den: d }
}

fn gcd_i128(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// Returns `|a - b|` as a [`Fixed`].
pub fn fixed_abs_diff(a: &Fixed, b: &Fixed) -> Fixed {
    fixed_sub(a, b).abs()
}

/// Float-free `<=` predicate via canonical `Ord`.
pub fn fixed_le(a: &Fixed, b: &Fixed) -> bool {
    a.cmp(b) != std::cmp::Ordering::Greater
}

/// Float-free `>=` predicate via canonical `Ord`.
pub fn fixed_ge(a: &Fixed, b: &Fixed) -> bool {
    a.cmp(b) != std::cmp::Ordering::Less
}

/// Float-free `min` over a slice — saturates to `Fixed::zero()` on empty.
pub fn min_fixed(values: &[Fixed]) -> Fixed {
    let mut iter = values.iter();
    let first = match iter.next() {
        Some(v) => v.clone(),
        None => return Fixed::zero(),
    };
    iter.fold(first, |acc, v| if v < &acc { v.clone() } else { acc })
}

/// Float-free `max` over a slice — saturates to `Fixed::zero()` on empty.
pub fn max_fixed(values: &[Fixed]) -> Fixed {
    let mut iter = values.iter();
    let first = match iter.next() {
        Some(v) => v.clone(),
        None => return Fixed::zero(),
    };
    iter.fold(first, |acc, v| if v > &acc { v.clone() } else { acc })
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
    fn fixed_sub_is_floatfree() {
        let a = Fixed::quantize(1.5, 9).unwrap();
        let b = Fixed::quantize(0.5, 9).unwrap();
        assert_eq!(fixed_sub(&a, &b), Fixed::quantize(1.0, 9).unwrap());
    }

    #[test]
    fn fixed_add_is_floatfree() {
        let a = Fixed::quantize(0.25, 9).unwrap();
        let b = Fixed::quantize(0.75, 9).unwrap();
        assert_eq!(fixed_add(&a, &b), Fixed::quantize(1.0, 9).unwrap());
    }

    #[test]
    fn min_max_over_empty_slice_is_zero() {
        assert_eq!(min_fixed(&[]), Fixed::zero());
        assert_eq!(max_fixed(&[]), Fixed::zero());
    }
}

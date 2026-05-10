//! Float-free primitives shared across every cell-substrate module
//! (§5).
//!
//! Re-exports `Hash256` and `CanonicalNumber` from `pse-traverse`'s
//! dynamics layer so the entire workspace shares one numeric and
//! addressing domain.

use serde::{Deserialize, Serialize};

pub use pse_traverse::dynamic_state::{CanonicalNumber, Hash256};

/// Every gate / metric / score scalar travels as a canonical, float-free
/// `CanonicalNumber`. Re-exported under the spec's `Fixed` name (§5.2).
pub type Fixed = CanonicalNumber;

/// Stable, declarative identifier (§5.2). Used for carrier ids, dual
/// ids, role names and other free-form names that must be canonical
/// without being content-hashes.
pub type StableId = String;

/// Content-addressed evidence reference into an external provenance
/// ledger (§5.2).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EvidenceRef {
    /// Free-form kind tag.
    pub kind: String,
    /// Content address of the referenced artefact.
    pub address: Hash256,
    /// Free-form relation tag.
    pub relation: String,
}

/// Total error surface for the cell substrate. Every fail-path lands
/// here so callers can translate to fail-closed outcomes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhaseError {
    /// A primitive value violated an invariant.
    InvalidPrimitive {
        /// Human-readable reason.
        reason: String,
    },
    /// Run descriptor failed validation.
    InvalidDescriptor {
        /// Human-readable reason.
        reason: String,
    },
    /// Canonical (JCS) serialisation failed.
    Canonicalization {
        /// Human-readable reason.
        reason: String,
    },
    /// A determinism violation was detected (e.g. wall-clock or float
    /// reached an audit pathway).
    DeterminismViolation {
        /// Human-readable reason.
        reason: String,
    },
    /// Replay byte-identity check failed.
    ReplayMismatch {
        /// Human-readable reason.
        reason: String,
    },
    /// Matrix boundary check rejected an attempted action.
    MatrixBoundaryViolation {
        /// Human-readable reason.
        reason: String,
    },
    /// FunnelGraph topology validation failed.
    FunnelTopology {
        /// Human-readable reason.
        reason: String,
    },
    /// Schema-level invariant violated.
    Schema {
        /// Human-readable reason.
        reason: String,
    },
}

impl std::fmt::Display for PhaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PhaseError::InvalidPrimitive { reason } => write!(f, "invalid primitive: {reason}"),
            PhaseError::InvalidDescriptor { reason } => write!(f, "invalid descriptor: {reason}"),
            PhaseError::Canonicalization { reason } => write!(f, "canonicalization: {reason}"),
            PhaseError::DeterminismViolation { reason } => {
                write!(f, "determinism violation: {reason}")
            }
            PhaseError::ReplayMismatch { reason } => write!(f, "replay mismatch: {reason}"),
            PhaseError::MatrixBoundaryViolation { reason } => {
                write!(f, "matrix boundary violation: {reason}")
            }
            PhaseError::FunnelTopology { reason } => write!(f, "funnel topology: {reason}"),
            PhaseError::Schema { reason } => write!(f, "schema: {reason}"),
        }
    }
}

impl std::error::Error for PhaseError {}

/// Compute the content address of any serialisable value via JCS +
/// SHA-256.
pub fn content_address<T: Serialize>(value: &T) -> Result<Hash256, PhaseError> {
    pse_traverse::canonical::content_address(value)
        .map(Hash256)
        .map_err(|e| PhaseError::Canonicalization {
            reason: e.to_string(),
        })
}

/// JCS-canonical bytes of any serialisable value.
pub fn canonical_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, PhaseError> {
    pse_traverse::canonical::canonical_bytes(value).map_err(|e| PhaseError::Canonicalization {
        reason: e.to_string(),
    })
}

/// `a − b` in i128 rational space, gcd-normalised.
pub fn fixed_sub(a: &Fixed, b: &Fixed) -> Fixed {
    let (an, ad) = as_rational(a);
    let (bn, bd) = as_rational(b);
    normalize_rational(
        an.saturating_mul(bd).saturating_sub(bn.saturating_mul(ad)),
        ad.saturating_mul(bd),
    )
}

/// `a + b` in i128 rational space, gcd-normalised.
pub fn fixed_add(a: &Fixed, b: &Fixed) -> Fixed {
    let (an, ad) = as_rational(a);
    let (bn, bd) = as_rational(b);
    normalize_rational(
        an.saturating_mul(bd).saturating_add(bn.saturating_mul(ad)),
        ad.saturating_mul(bd),
    )
}

/// `a * b` in i128 rational space, gcd-normalised.
pub fn fixed_mul(a: &Fixed, b: &Fixed) -> Fixed {
    let (an, ad) = as_rational(a);
    let (bn, bd) = as_rational(b);
    normalize_rational(an.saturating_mul(bn), ad.saturating_mul(bd))
}

/// `|a − b|`.
pub fn fixed_abs_diff(a: &Fixed, b: &Fixed) -> Fixed {
    fixed_sub(a, b).abs()
}

/// Float-free `≤` predicate via canonical `Ord`.
pub fn fixed_le(a: &Fixed, b: &Fixed) -> bool {
    a.cmp(b) != std::cmp::Ordering::Greater
}

/// Float-free `≥` predicate via canonical `Ord`.
pub fn fixed_ge(a: &Fixed, b: &Fixed) -> bool {
    a.cmp(b) != std::cmp::Ordering::Less
}

/// `min` over a slice; saturates to `Fixed::zero()` on empty.
pub fn min_fixed(values: &[Fixed]) -> Fixed {
    let mut iter = values.iter();
    let mut acc = match iter.next() {
        Some(v) => v.clone(),
        None => return Fixed::zero(),
    };
    for v in iter {
        if v < &acc {
            acc = v.clone();
        }
    }
    acc
}

/// `max` over a slice; saturates to `Fixed::zero()` on empty.
pub fn max_fixed(values: &[Fixed]) -> Fixed {
    let mut iter = values.iter();
    let mut acc = match iter.next() {
        Some(v) => v.clone(),
        None => return Fixed::zero(),
    };
    for v in iter {
        if v > &acc {
            acc = v.clone();
        }
    }
    acc
}

/// Reduce `num / den` by gcd; ensures `den > 0`.
pub fn normalize_rational(num: i128, den: i128) -> Fixed {
    if den == 0 {
        return Fixed::Rational { num: 0, den: 1 };
    }
    let (mut n, mut d) = if den < 0 { (-num, -den) } else { (num, den) };
    let g = gcd(n.unsigned_abs(), d.unsigned_abs());
    if g > 1 {
        n /= g as i128;
        d /= g as i128;
    }
    Fixed::Rational { num: n, den: d }
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

fn as_rational(x: &Fixed) -> (i128, i128) {
    match x {
        Fixed::FixedI64 { raw, scale } => (*raw as i128, 10i128.pow(*scale)),
        Fixed::Rational { num, den } => (*num, *den),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_arithmetic_is_floatfree() {
        let a = Fixed::quantize(1.5, 9).unwrap();
        let b = Fixed::quantize(0.5, 9).unwrap();
        assert_eq!(fixed_sub(&a, &b), Fixed::quantize(1.0, 9).unwrap());
        assert_eq!(fixed_add(&a, &b), Fixed::quantize(2.0, 9).unwrap());
        assert_eq!(fixed_mul(&a, &b), Fixed::quantize(0.75, 9).unwrap());
    }

    #[test]
    fn content_address_is_stable() {
        let a = ("phase-cell", 42u32);
        assert_eq!(content_address(&a).unwrap(), content_address(&a).unwrap());
    }
}

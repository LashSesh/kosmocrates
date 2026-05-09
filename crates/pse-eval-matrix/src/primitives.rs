//! Float-free primitives shared across every eval-matrix module.
//!
//! Re-exports `Hash256` and `CanonicalNumber` from `pse-traverse`'s
//! dynamics layer so the entire workspace shares one numeric and
//! addressing domain. Adds a small set of arithmetic helpers
//! (`fixed_sub`, `fixed_le`, `fixed_ge`, `fixed_abs_diff`) and the
//! crate-wide error type.

use serde::{Deserialize, Serialize};

pub use pse_traverse::dynamic_state::{CanonicalNumber, Hash256};

/// Every gate / metric / score scalar travels as a canonical, float-free
/// `CanonicalNumber`. Re-exported under the spec's `Fixed` name.
pub type Fixed = CanonicalNumber;

/// Content-addressed evidence reference into an external provenance
/// ledger. Same shape as the cognition / horizon evidence refs but
/// kept layer-local so this crate compiles without those modules.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EvidenceRef {
    /// Free-form kind tag (e.g. `"trial_report"`, `"gold_label"`).
    pub kind: String,
    /// SHA-256 content address of the referenced artefact.
    pub address: Hash256,
    /// Free-form relation tag (e.g. `"derived_from"`, `"validates"`).
    pub relation: String,
}

/// Total error surface for the eval-matrix layer. Every fail-path lands
/// here so callers can translate to fail-closed run invalidation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvalError {
    /// A primitive value violated an invariant (e.g. empty id).
    InvalidPrimitive {
        /// Human-readable reason.
        reason: String,
    },
    /// An evaluation spec failed validation.
    InvalidSpec {
        /// Human-readable reason.
        reason: String,
    },
    /// Canonicalisation (JCS / SHA-256) failed.
    Canonicalization {
        /// Human-readable reason.
        reason: String,
    },
    /// A replay byte-identity check failed.
    ReplayMismatch {
        /// Human-readable reason.
        reason: String,
    },
    /// A run could not be appended to the ledger because the chain hash
    /// would diverge.
    LedgerChainBroken {
        /// Human-readable reason.
        reason: String,
    },
    /// Ground-truth labels for the workload referenced by the run are
    /// missing or malformed.
    MissingLabels {
        /// Human-readable reason.
        reason: String,
    },
    /// A metric direction / aggregation rule was violated.
    MetricRuleViolation {
        /// Human-readable reason.
        reason: String,
    },
    /// Calibration leaked from the test split into another split.
    CalibrationLeakage {
        /// Human-readable reason.
        reason: String,
    },
    /// A schema-level invariant was violated.
    Schema {
        /// Human-readable reason.
        reason: String,
    },
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalError::InvalidPrimitive { reason } => write!(f, "invalid primitive: {reason}"),
            EvalError::InvalidSpec { reason } => write!(f, "invalid spec: {reason}"),
            EvalError::Canonicalization { reason } => write!(f, "canonicalization: {reason}"),
            EvalError::ReplayMismatch { reason } => write!(f, "replay mismatch: {reason}"),
            EvalError::LedgerChainBroken { reason } => write!(f, "ledger chain broken: {reason}"),
            EvalError::MissingLabels { reason } => write!(f, "missing labels: {reason}"),
            EvalError::MetricRuleViolation { reason } => {
                write!(f, "metric rule violation: {reason}")
            }
            EvalError::CalibrationLeakage { reason } => write!(f, "calibration leakage: {reason}"),
            EvalError::Schema { reason } => write!(f, "schema: {reason}"),
        }
    }
}

impl std::error::Error for EvalError {}

/// Compute the content address of any serialisable value via JCS +
/// SHA-256.
pub fn content_address<T: Serialize>(value: &T) -> Result<Hash256, EvalError> {
    pse_traverse::canonical::content_address(value)
        .map(Hash256)
        .map_err(|e| EvalError::Canonicalization {
            reason: e.to_string(),
        })
}

/// JCS-canonical bytes of any serialisable value.
pub fn canonical_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, EvalError> {
    pse_traverse::canonical::canonical_bytes(value).map_err(|e| EvalError::Canonicalization {
        reason: e.to_string(),
    })
}

/// `a - b` exactly in i128 rational space, gcd-normalised.
pub fn fixed_sub(a: &Fixed, b: &Fixed) -> Fixed {
    let (an, ad) = as_rational(a);
    let (bn, bd) = as_rational(b);
    let num = an.saturating_mul(bd).saturating_sub(bn.saturating_mul(ad));
    let den = ad.saturating_mul(bd);
    normalize_rational(num, den)
}

/// `a + b` exactly in i128 rational space, gcd-normalised.
pub fn fixed_add(a: &Fixed, b: &Fixed) -> Fixed {
    let (an, ad) = as_rational(a);
    let (bn, bd) = as_rational(b);
    let num = an.saturating_mul(bd).saturating_add(bn.saturating_mul(ad));
    let den = ad.saturating_mul(bd);
    normalize_rational(num, den)
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
    fn fixed_sub_is_floatfree() {
        let a = Fixed::quantize(1.5, 9).unwrap();
        let b = Fixed::quantize(0.5, 9).unwrap();
        assert_eq!(fixed_sub(&a, &b), Fixed::quantize(1.0, 9).unwrap());
    }

    #[test]
    fn content_address_is_stable() {
        let a = ("hello", 42u32);
        let h1 = content_address(&a).unwrap();
        let h2 = content_address(&a).unwrap();
        assert_eq!(h1, h2);
    }
}

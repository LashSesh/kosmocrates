use serde::{Deserialize, Serialize};
use std::fmt;

/// 16-bit fractional fixed-point number backed by `i64`.
///
/// Represents `inner / 65536`. Range: approximately ±140737.
/// Precision: ~0.0000153 (2⁻¹⁶).
///
/// Used for all gate-relevant numeric values (scores, densities, fitness,
/// resonance, D-density, support mass) to satisfy CROSS-007: no floats in
/// audit or gate comparison paths.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Q16(pub(crate) i64);

pub const Q16_SCALE: i64 = 1 << 16; // 65536

impl Q16 {
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(Q16_SCALE);
    pub const NEG_ONE: Self = Self(-Q16_SCALE);
    pub const HALF: Self = Self(Q16_SCALE / 2);
    pub const MAX: Self = Self(i64::MAX);
    pub const MIN: Self = Self(i64::MIN);

    /// Construct from an exact integer (no fractional part).
    pub fn from_i64(n: i64) -> Self {
        Self(n.saturating_mul(Q16_SCALE))
    }

    /// Construct by rounding a float. Use only at boundary input points,
    /// never inside gate or audit comparison logic.
    pub fn from_f64(v: f64) -> Self {
        Self((v * Q16_SCALE as f64).round() as i64)
    }

    /// Convert back to f64 for display or external reporting only.
    pub fn to_f64(self) -> f64 {
        self.0 as f64 / Q16_SCALE as f64
    }

    /// Raw integer representation (value × 65536).
    pub fn raw(self) -> i64 {
        self.0
    }

    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub fn is_positive(self) -> bool {
        self.0 > 0
    }

    pub fn is_negative(self) -> bool {
        self.0 < 0
    }

    pub fn abs(self) -> Self {
        Self(self.0.saturating_abs())
    }

    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }

    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }

    /// Fixed-point multiply: (a / 65536) × (b / 65536) = (a × b) >> 16 / 65536.
    /// Uses i128 intermediate to avoid overflow.
    pub fn checked_mul(self, rhs: Self) -> Option<Self> {
        let product = (self.0 as i128).checked_mul(rhs.0 as i128)?;
        let scaled = product >> 16;
        i64::try_from(scaled).ok().map(Self)
    }

    pub fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }

    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    /// Compare to threshold using integer arithmetic only.
    pub fn exceeds(self, threshold: Self) -> bool {
        self.0 > threshold.0
    }

    /// Returns true if self ≥ threshold.
    pub fn at_least(self, threshold: Self) -> bool {
        self.0 >= threshold.0
    }
}

impl std::ops::Neg for Q16 {
    type Output = Self;
    fn neg(self) -> Self {
        Self(self.0.saturating_neg())
    }
}

impl fmt::Display for Q16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.6}q16", self.to_f64())
    }
}

impl fmt::Debug for Q16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Q16({})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q16_zero_is_zero() {
        assert_eq!(Q16::ZERO.raw(), 0);
        assert!(Q16::ZERO.is_zero());
    }

    #[test]
    fn q16_one() {
        assert_eq!(Q16::ONE.raw(), 65536);
        assert!((Q16::ONE.to_f64() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn q16_half() {
        assert_eq!(Q16::HALF.raw(), 32768);
        assert!((Q16::HALF.to_f64() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn q16_from_f64_round_trip() {
        let v = 0.75_f64;
        let q = Q16::from_f64(v);
        assert!((q.to_f64() - v).abs() < 1e-4);
    }

    #[test]
    fn q16_integer_arithmetic_no_float() {
        let a = Q16::from_i64(3);
        let b = Q16::from_i64(2);
        let sum = a.checked_add(b).unwrap();
        assert_eq!(sum, Q16::from_i64(5));
        let diff = a.checked_sub(b).unwrap();
        assert_eq!(diff, Q16::from_i64(1));
    }

    #[test]
    fn q16_multiply() {
        let half = Q16::HALF;
        let result = half.checked_mul(half).unwrap();
        assert!((result.to_f64() - 0.25).abs() < 1e-4);
    }

    #[test]
    fn q16_comparison_is_integer_only() {
        let a = Q16::from_f64(0.6);
        let threshold = Q16::HALF;
        // All comparisons happen on i64 via Ord/PartialOrd — no float involved
        assert!(a > threshold);
        assert!(a.exceeds(threshold));
        assert!(a.at_least(threshold));
    }

    #[test]
    fn q16_negation() {
        let one = Q16::ONE;
        let neg = -one;
        assert_eq!(neg, Q16::NEG_ONE);
    }

    #[test]
    fn q16_serializes_as_integer() {
        let q = Q16::from_i64(2);
        let json = serde_json::to_string(&q).unwrap();
        assert_eq!(json, (2 * 65536i64).to_string());
    }
}

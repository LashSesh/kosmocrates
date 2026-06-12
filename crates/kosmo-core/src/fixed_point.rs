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

    /// Construct from an already-scaled raw integer (value × 65536).
    /// Use for accumulations: e.g. `Q16::from_raw(sum_raw / count)`.
    pub fn from_raw(raw: i64) -> Self {
        Self(raw)
    }

    /// Compute the ratio `numerator / denominator` as Q16 using integer
    /// arithmetic only. Returns `None` if `denominator == 0`.
    ///
    /// `Q16::ratio(1, 2)` → `Q16::HALF` (0.5)
    pub fn ratio(numerator: u64, denominator: u64) -> Option<Self> {
        if denominator == 0 {
            return None;
        }
        let raw = (numerator as i128 * Q16_SCALE as i128) / denominator as i128;
        i64::try_from(raw).ok().map(Self)
    }

    /// Integer division of two Q16 values. Returns `None` on overflow or
    /// divide-by-zero.
    pub fn checked_div(self, rhs: Self) -> Option<Self> {
        if rhs.0 == 0 {
            return None;
        }
        let num = (self.0 as i128) << 16;
        i64::try_from(num / rhs.0 as i128).ok().map(Self)
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

    /// Integer geometric mean of values in `[ZERO, ONE]` — the n-th root of
    /// the raw product, found by binary search on `i128`. Deterministic, no
    /// floats (CROSS-007), bit-replayable.
    ///
    /// Domain discipline (fail-closed): an empty slice yields `ZERO`; any
    /// member `≤ ZERO` yields `ZERO` (the geometric mean's soft-unanimity
    /// property — one dead value silences the ensemble); members above `ONE`
    /// are clamped to `ONE` (scores live on the unit interval).
    ///
    /// Derivation: for values vᵢ = rawᵢ/S (S = 2¹⁶), the geometric mean is
    /// (∏ vᵢ)^(1/n) = (∏ rawᵢ)^(1/n) / S — so the result's raw value is
    /// exactly the integer n-th root of the raw product. With rawᵢ ≤ 2¹⁶ the
    /// product fits `i128` for n ≤ 7 ensembles and far beyond.
    pub fn geomean(values: &[Q16]) -> Q16 {
        if values.is_empty() {
            return Q16::ZERO;
        }
        let mut product: i128 = 1;
        for v in values {
            if v.0 <= 0 {
                return Q16::ZERO;
            }
            let raw = v.0.min(Q16_SCALE) as i128;
            product = match product.checked_mul(raw) {
                Some(p) => p,
                None => return Q16::ONE, // unreachable for unit-interval inputs
            };
        }
        let n = values.len() as u32;
        // Largest r in [0, SCALE] with r^n ≤ product.
        let (mut lo, mut hi) = (0i128, Q16_SCALE as i128);
        while lo < hi {
            let mid = (lo + hi + 1) / 2;
            let mut pow: i128 = 1;
            let mut fits = true;
            for _ in 0..n {
                pow = match pow.checked_mul(mid) {
                    Some(p) if p <= product => p,
                    _ => {
                        fits = false;
                        break;
                    }
                };
            }
            if fits {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        Q16(lo as i64)
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

    #[test]
    fn q16_from_raw() {
        assert_eq!(Q16::from_raw(65536), Q16::ONE);
        assert_eq!(Q16::from_raw(0), Q16::ZERO);
        assert_eq!(Q16::from_raw(32768), Q16::HALF);
    }

    #[test]
    fn q16_ratio_half() {
        assert_eq!(Q16::ratio(1, 2).unwrap(), Q16::HALF);
    }

    #[test]
    fn q16_ratio_full() {
        assert_eq!(Q16::ratio(3, 3).unwrap(), Q16::ONE);
    }

    #[test]
    fn q16_ratio_zero_denominator() {
        assert!(Q16::ratio(1, 0).is_none());
    }

    #[test]
    fn q16_checked_div() {
        let one = Q16::ONE;
        let two = Q16::from_i64(2);
        assert_eq!(one.checked_div(two).unwrap(), Q16::HALF);
    }

    #[test]
    fn q16_accumulate_average() {
        // sum 3 half-values, divide by 3 → should still be HALF
        let half = Q16::HALF;
        let sum_raw: i64 = (0..3).map(|_| half.raw()).sum();
        let avg = Q16::from_raw(sum_raw / 3);
        assert_eq!(avg, Q16::HALF);
    }

    // ─── geomean ─────────────────────────────────────────────────────────────

    #[test]
    fn geomean_identities() {
        assert_eq!(Q16::geomean(&[]), Q16::ZERO, "empty is fail-closed");
        assert_eq!(Q16::geomean(&[Q16::HALF]), Q16::HALF, "singleton is itself");
        assert_eq!(Q16::geomean(&[Q16::ONE, Q16::ONE, Q16::ONE]), Q16::ONE);
    }

    #[test]
    fn geomean_soft_unanimity() {
        // One dead member silences the ensemble.
        assert_eq!(Q16::geomean(&[Q16::ONE, Q16::ONE, Q16::ZERO]), Q16::ZERO);
        assert_eq!(Q16::geomean(&[Q16::HALF, -Q16::HALF]), Q16::ZERO);
    }

    #[test]
    fn geomean_known_values() {
        // geomean(1, 1/4) = sqrt(1/4) = 1/2 — exact in raw arithmetic.
        let quarter = Q16::ratio(1, 4).unwrap();
        assert_eq!(Q16::geomean(&[Q16::ONE, quarter]), Q16::HALF);
        // geomean(x, x) = x for any unit-interval value.
        let v = Q16::ratio(7, 10).unwrap();
        let g = Q16::geomean(&[v, v]);
        assert!((g.raw() - v.raw()).abs() <= 1, "n-th root floor is tight");
    }

    #[test]
    fn geomean_monotone_and_bounded() {
        let lo = Q16::geomean(&[Q16::ratio(1, 4).unwrap(), Q16::HALF]);
        let hi = Q16::geomean(&[Q16::HALF, Q16::HALF]);
        assert!(lo < hi, "raising a member raises the mean");
        // Above-ONE inputs clamp to the unit interval.
        assert_eq!(Q16::geomean(&[Q16::from_i64(5), Q16::ONE]), Q16::ONE);
        // Bounded by min and max members.
        let g = Q16::geomean(&[Q16::ratio(1, 4).unwrap(), Q16::ONE]);
        assert!(g >= Q16::ratio(1, 4).unwrap() && g <= Q16::ONE);
    }

    #[test]
    fn geomean_is_deterministic_across_orderings_of_same_multiset() {
        let a = [
            Q16::ratio(3, 10).unwrap(),
            Q16::ratio(9, 10).unwrap(),
            Q16::HALF,
        ];
        let b = [
            Q16::HALF,
            Q16::ratio(3, 10).unwrap(),
            Q16::ratio(9, 10).unwrap(),
        ];
        assert_eq!(Q16::geomean(&a), Q16::geomean(&b));
    }
}

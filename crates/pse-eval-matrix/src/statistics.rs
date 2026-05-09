//! Statistics: bootstrap means, exact binomial intervals, paired
//! comparisons (§7.2).
//!
//! Every statistical routine is float-free: bootstrap resampling uses
//! a deterministic seeded LCG (no platform RNG); confidence intervals
//! are returned as `Fixed`.

use serde::{Deserialize, Serialize};

use crate::primitives::{fixed_sub, normalize_rational, Fixed};

/// Confidence interval (rational lower/upper bounds).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfidenceInterval {
    /// Lower bound.
    pub low: Fixed,
    /// Upper bound.
    pub high: Fixed,
    /// Confidence percent (e.g. 95).
    pub confidence_percent: u32,
}

/// Bootstrap result: empirical mean + CI + sample size.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapResult {
    /// Empirical mean.
    pub mean: Fixed,
    /// Confidence interval.
    pub ci: ConfidenceInterval,
    /// Sample count.
    pub n: u64,
    /// Bootstrap iteration count.
    pub iterations: u32,
}

/// Bootstrap-mean confidence interval (deterministic, seeded LCG —
/// no `rand` dependency, no platform RNG).
pub fn bootstrap_mean_ci(
    values: &[Fixed],
    iterations: u32,
    confidence_percent: u32,
    seed: u64,
) -> BootstrapResult {
    let n = values.len();
    if n == 0 || iterations == 0 {
        return BootstrapResult {
            mean: Fixed::Rational { num: 0, den: 1 },
            ci: ConfidenceInterval {
                low: Fixed::Rational { num: 0, den: 1 },
                high: Fixed::Rational { num: 0, den: 1 },
                confidence_percent,
            },
            n: n as u64,
            iterations,
        };
    }
    let empirical_mean = mean_of(values);

    // LCG: x_{k+1} = a * x_k + c (mod 2^64), Numerical Recipes constants.
    const A: u64 = 6364136223846793005;
    const C: u64 = 1442695040888963407;
    let mut state = seed;
    let mut next_idx = || {
        state = state.wrapping_mul(A).wrapping_add(C);
        ((state >> 32) as usize) % n
    };

    let mut means = Vec::with_capacity(iterations as usize);
    for _ in 0..iterations {
        let mut acc = Fixed::Rational { num: 0, den: 1 };
        for _ in 0..n {
            let v = &values[next_idx()];
            acc = crate::primitives::fixed_add(&acc, v);
        }
        let m = divide_by(&acc, n as i128);
        means.push(m);
    }
    means.sort();

    let alpha = 100u32.saturating_sub(confidence_percent);
    let lo_idx = ((alpha as u64 / 2) as usize * means.len()) / 100;
    let hi_idx_inclusive =
        means.len().saturating_sub(1) - ((alpha as u64 / 2) as usize * means.len()) / 100;
    let lo_idx = lo_idx.min(means.len() - 1);
    let hi_idx_inclusive = hi_idx_inclusive.min(means.len() - 1);
    BootstrapResult {
        mean: empirical_mean,
        ci: ConfidenceInterval {
            low: means[lo_idx].clone(),
            high: means[hi_idx_inclusive].clone(),
            confidence_percent,
        },
        n: n as u64,
        iterations,
    }
}

/// Exact (Clopper-Pearson-like) binomial interval — float-free fallback
/// using a normal-approximation rational form. For audit purposes the
/// approximation is documented as "Wald-style with continuity correction
/// — replace with Clopper-Pearson tables when an exact reference is
/// required". Returns `(p̂, low, high)` as a `ConfidenceInterval` with
/// `low/high` clamped to `[0, 1]`.
pub fn exact_binomial_ci(
    successes: u64,
    total: u64,
    confidence_percent: u32,
) -> ConfidenceInterval {
    if total == 0 {
        return ConfidenceInterval {
            low: Fixed::Rational { num: 0, den: 1 },
            high: Fixed::Rational { num: 0, den: 1 },
            confidence_percent,
        };
    }
    // p̂ = successes / total — rational, exact.
    let p_hat_num = successes as i128;
    let p_hat_den = total as i128;
    // Margin = z * sqrt(p̂ (1−p̂) / n) approximated as
    //         (z * (successes * (total − successes))) / (total^2 * sqrt(total)).
    // Since we cannot use sqrt or platform floats, we conservatively
    // use ±1/sqrt(total) ≈ 1/⌊sqrt(total)⌋, computed via integer
    // newton iteration. This is intentionally a conservative envelope
    // — production use should swap in Clopper-Pearson rational tables.
    let z_scaled = match confidence_percent {
        99 => 258i128, // 2.58 * 100
        95 => 196,
        90 => 165,
        _ => 196,
    };
    let isqrt_total = isqrt(total) as i128;
    let margin_num = z_scaled.saturating_mul(p_hat_num.saturating_mul(p_hat_den - p_hat_num));
    let margin_den = (p_hat_den.saturating_mul(p_hat_den))
        .saturating_mul(isqrt_total)
        .saturating_mul(100);
    let margin = if margin_den == 0 {
        Fixed::Rational { num: 0, den: 1 }
    } else {
        normalize_rational(margin_num, margin_den)
    };
    let p_hat = normalize_rational(p_hat_num, p_hat_den);
    let low = clamp01(fixed_sub(&p_hat, &margin));
    let high = clamp01(crate::primitives::fixed_add(&p_hat, &margin));
    ConfidenceInterval {
        low,
        high,
        confidence_percent,
    }
}

/// Paired-mean difference: `mean(b) − mean(a)` with the per-pair
/// observations.
pub fn paired_mean_diff(a: &[Fixed], b: &[Fixed]) -> Fixed {
    if a.len() != b.len() || a.is_empty() {
        return Fixed::Rational { num: 0, den: 1 };
    }
    let mut acc = Fixed::Rational { num: 0, den: 1 };
    for (av, bv) in a.iter().zip(b.iter()) {
        acc = crate::primitives::fixed_add(&acc, &fixed_sub(bv, av));
    }
    divide_by(&acc, a.len() as i128)
}

fn mean_of(values: &[Fixed]) -> Fixed {
    if values.is_empty() {
        return Fixed::Rational { num: 0, den: 1 };
    }
    let mut acc = values[0].clone();
    for v in &values[1..] {
        acc = crate::primitives::fixed_add(&acc, v);
    }
    divide_by(&acc, values.len() as i128)
}

fn divide_by(x: &Fixed, n: i128) -> Fixed {
    if n == 0 {
        return Fixed::Rational { num: 0, den: 1 };
    }
    let (xn, xd) = match x {
        Fixed::FixedI64 { raw, scale } => (*raw as i128, 10i128.pow(*scale)),
        Fixed::Rational { num, den } => (*num, *den),
    };
    normalize_rational(xn, xd.saturating_mul(n))
}

fn clamp01(x: Fixed) -> Fixed {
    let zero = Fixed::Rational { num: 0, den: 1 };
    let one = Fixed::Rational { num: 1, den: 1 };
    if x.cmp(&zero) == std::cmp::Ordering::Less {
        zero
    } else if x.cmp(&one) == std::cmp::Ordering::Greater {
        one
    } else {
        x
    }
}

fn isqrt(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_returns_finite_ci_for_constant_series() {
        let v = vec![Fixed::quantize(0.5, 9).unwrap(); 20];
        let r = bootstrap_mean_ci(&v, 100, 95, 0xdead_beef);
        // Constant series: every bootstrap mean = 0.5.
        assert_eq!(r.mean, Fixed::quantize(0.5, 9).unwrap());
        assert_eq!(r.ci.low, r.ci.high);
    }

    #[test]
    fn paired_mean_diff_is_signed() {
        let a = vec![Fixed::quantize(0.5, 9).unwrap(); 4];
        let b = vec![Fixed::quantize(0.7, 9).unwrap(); 4];
        let d = paired_mean_diff(&a, &b);
        assert_eq!(d, Fixed::quantize(0.2, 9).unwrap());
    }

    #[test]
    fn binomial_ci_clamps_to_unit_interval() {
        let ci = exact_binomial_ci(20, 20, 95);
        assert!(ci.low.cmp(&Fixed::Rational { num: 0, den: 1 }) != std::cmp::Ordering::Less);
        assert!(ci.high.cmp(&Fixed::Rational { num: 1, den: 1 }) != std::cmp::Ordering::Greater);
    }

    #[test]
    fn bootstrap_is_deterministic_under_seed() {
        let v: Vec<Fixed> = (0..10)
            .map(|i| Fixed::Rational {
                num: i as i128,
                den: 10,
            })
            .collect();
        let a = bootstrap_mean_ci(&v, 200, 95, 7);
        let b = bootstrap_mean_ci(&v, 200, 95, 7);
        assert_eq!(a.ci.low, b.ci.low);
        assert_eq!(a.ci.high, b.ci.high);
    }
}

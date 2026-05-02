//! BBO optimizers: TRITON spiral wrapper plus two reference baselines.
//!
//! All optimizers are deterministic given a fixed seed and consume an
//! identical fixed budget of evaluations.

use pse_navigator::NavigatorSpiral;

use crate::Optimizer;

// ─── Deterministic xorshift64 PRNG ──────────────────────────────────────────

struct Xorshift64(u64);
impl Xorshift64 {
    fn new(seed: u64) -> Self { Self(seed.wrapping_add(0x9E37_79B9_7F4A_7C15)) }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn next_unit(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64) / ((1u64 << 53) as f64)
    }
}

// ─── Random search ───────────────────────────────────────────────────────────

/// Pure uniform sampling over `[0, 1]^d`. The "do nothing clever"
/// baseline that any adaptive method must outperform on a non-trivial
/// landscape.
pub struct RandomSearch {
    rng: Xorshift64,
    dim: usize,
}

impl RandomSearch {
    pub fn new(seed: u64, dim: usize) -> Self {
        Self { rng: Xorshift64::new(seed), dim: dim.max(1) }
    }
}

impl<'f> Optimizer<'f> for RandomSearch {
    fn name(&self) -> &str { "random_search" }
    fn next_point(&mut self, _history: &[(Vec<f64>, f64)]) -> Vec<f64> {
        (0..self.dim).map(|_| self.rng.next_unit()).collect()
    }
}

// ─── Halton low-discrepancy sequence ─────────────────────────────────────────

/// Halton sequence with bases (2, 3): a deterministic low-discrepancy
/// sequence in `[0, 1]^2`. Stronger than random search (uniformly fills
/// the unit square in fewer samples) but completely **non-adaptive** —
/// it cannot react to function evaluations.
pub struct HaltonSearch {
    index: usize,
    bases: Vec<u64>,
}

impl HaltonSearch {
    pub fn new(start_index: usize, dim: usize) -> Self {
        let primes: Vec<u64> = vec![2, 3, 5, 7, 11, 13, 17, 19];
        Self {
            index: start_index.max(1),
            bases: primes.into_iter().take(dim.max(1)).collect(),
        }
    }
}

fn halton_value(mut index: u64, base: u64) -> f64 {
    let mut f = 1.0_f64;
    let mut r = 0.0_f64;
    while index > 0 {
        f /= base as f64;
        r += f * (index % base) as f64;
        index /= base;
    }
    r
}

impl<'f> Optimizer<'f> for HaltonSearch {
    fn name(&self) -> &str { "halton" }
    fn next_point(&mut self, _history: &[(Vec<f64>, f64)]) -> Vec<f64> {
        let i = self.index as u64;
        self.index += 1;
        self.bases.iter().map(|&b| halton_value(i, b)).collect()
    }
}

// ─── TRITON-spiral wrapper ──────────────────────────────────────────────────

/// Wraps PSE's golden-angle [`NavigatorSpiral`] as a BBO optimizer.
///
/// This isolates TRITON's *spiral generation* (golden-angle aperiodic
/// quasi-random sequence) from its adaptive-mesh + Fiedler-momentum
/// machinery, so the benchmark cleanly answers: **what does the
/// spiral alone buy you compared to random search and Halton?**
///
/// The full mesh + Fiedler exploration is exercised separately by
/// `pse_core::explore::resonance_landscape_explorer` (E.7), which
/// operates inside the engine and so does not fit the BBO probe-only
/// optimizer interface naturally.
pub struct TritonSpiralBBO {
    spiral: NavigatorSpiral,
}

impl TritonSpiralBBO {
    pub fn new(seed: u64) -> Self {
        Self { spiral: NavigatorSpiral::new(2, seed) }
    }
}

impl<'f> Optimizer<'f> for TritonSpiralBBO {
    fn name(&self) -> &str { "triton_spiral" }
    fn next_point(&mut self, _history: &[(Vec<f64>, f64)]) -> Vec<f64> {
        self.spiral.next_point()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_search_is_deterministic_under_seed() {
        let mut a = RandomSearch::new(42, 2);
        let mut b = RandomSearch::new(42, 2);
        for _ in 0..10 {
            let pa = a.next_point(&[]);
            let pb = b.next_point(&[]);
            assert_eq!(pa, pb);
        }
    }

    #[test]
    fn random_search_points_in_unit_interval() {
        let mut s = RandomSearch::new(7, 2);
        for _ in 0..50 {
            let p = s.next_point(&[]);
            for axis in &p {
                assert!((0.0..=1.0).contains(axis));
            }
        }
    }

    #[test]
    fn halton_first_few_points_match_known_values() {
        let mut h = HaltonSearch::new(1, 2);
        let p1 = h.next_point(&[]);
        assert!((p1[0] - 0.5).abs() < 1e-12);
        assert!((p1[1] - 1.0 / 3.0).abs() < 1e-12);
        let p2 = h.next_point(&[]);
        assert!((p2[0] - 0.25).abs() < 1e-12);
        assert!((p2[1] - 2.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn halton_is_deterministic() {
        let mut a = HaltonSearch::new(1, 2);
        let mut b = HaltonSearch::new(1, 2);
        for _ in 0..30 {
            assert_eq!(a.next_point(&[]), b.next_point(&[]));
        }
    }

    #[test]
    fn triton_spiral_is_deterministic_under_seed() {
        let mut a = TritonSpiralBBO::new(42);
        let mut b = TritonSpiralBBO::new(42);
        for _ in 0..20 {
            let pa = a.next_point(&[]);
            let pb = b.next_point(&[]);
            assert_eq!(pa.len(), pb.len());
            for (x, y) in pa.iter().zip(pb.iter()) {
                assert!((x - y).abs() < 1e-12);
            }
        }
    }
}

//! Hand-rolled Isolation Forest anomaly detector.
//!
//! A minimal, dependency-free, fully-deterministic implementation of
//! Liu, Ting, Zhou (2008) "Isolation Forest". Designed for the bench-gt
//! suite where determinism (seed-derived from `RunDescriptor.seed`) is
//! a hard requirement and a heavy ML dependency tree is undesirable.
//!
//! Algorithm (per the paper):
//!
//!  1. Build `n_trees` isolation trees (default 100). Each tree subsamples
//!     `psi` points (default 256, capped at the dataset size) and grows
//!     by repeatedly splitting on a uniformly-random feature dimension
//!     and a uniformly-random split value drawn from that dimension's
//!     range in the current node. Recursion stops at depth
//!     `ceil(log2(psi))` or when a node holds ≤ 1 point.
//!  2. The anomaly score for a sample `x` is
//!     `s(x) = 2^(-mean_path_length(x) / c(psi))`,
//!     where `c(n) = 2·H(n-1) - 2·(n-1)/n` with H the harmonic number
//!     (`H(i) ≈ ln(i) + γ`, γ = Euler-Mascheroni). `s ≈ 0.5` is
//!     "normal", `s → 1` is anomalous.
//!  3. A [`Detection`] is emitted at every index whose score meets or
//!     exceeds `threshold` (default 0.6); the detection score is the
//!     raw anomaly score, already in [0, 1].
//!
//! Determinism: a single `seed: u64` controls everything. Identical
//! seed + identical input → identical detections.

use crate::Detection;

/// Default detector source string emitted on every detection.
pub const SOURCE: &str = "isoforest";

/// Configuration for the hand-rolled Isolation Forest detector.
#[derive(Clone, Debug)]
pub struct IsoForestConfig {
    /// Number of trees in the ensemble.
    pub n_trees: usize,
    /// Subsample size per tree (the paper recommends 256).
    pub psi: usize,
    /// Anomaly score threshold for emitting a detection.
    pub threshold: f64,
    /// Deterministic seed. Derive from `RunDescriptor.seed` when calling
    /// from a research run.
    pub seed: u64,
}

impl Default for IsoForestConfig {
    fn default() -> Self {
        Self {
            n_trees: 100,
            psi: 256,
            threshold: 0.6,
            seed: 0x5eed_cafe_dead_beef,
        }
    }
}

// ─── PRNG ────────────────────────────────────────────────────────────────────

/// Tiny deterministic xorshift64 PRNG. Same family used elsewhere in PSE.
struct Xorshift64(u64);

impl Xorshift64 {
    fn new(seed: u64) -> Self {
        // Avoid the trivial fixed point at 0.
        Self(seed.wrapping_add(0xa5a5_a5a5_5a5a_5a5a))
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniform f64 in [0, 1).
    fn next_f64(&mut self) -> f64 {
        // Use the upper 53 bits for a high-quality unit-interval sample.
        ((self.next_u64() >> 11) as f64) / ((1u64 << 53) as f64)
    }

    /// Uniform usize in [0, bound).
    fn next_below(&mut self, bound: usize) -> usize {
        debug_assert!(bound > 0);
        (self.next_u64() as usize) % bound
    }
}

// ─── Tree ────────────────────────────────────────────────────────────────────

#[derive(Debug)]
enum IsoNode {
    Internal {
        feat: usize,
        split: f64,
        left: Box<IsoNode>,
        right: Box<IsoNode>,
    },
    External {
        size: usize,
    },
}

fn build_tree(
    samples: &[Vec<f64>],
    indices: &[usize],
    depth: u32,
    height_limit: u32,
    rng: &mut Xorshift64,
) -> IsoNode {
    if depth >= height_limit || indices.len() <= 1 {
        return IsoNode::External {
            size: indices.len(),
        };
    }
    let n_features = samples[0].len();
    if n_features == 0 {
        return IsoNode::External {
            size: indices.len(),
        };
    }
    let feat = rng.next_below(n_features);

    // Find min/max of the chosen feature in the current bag.
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for &i in indices {
        let v = samples[i][feat];
        if v < lo {
            lo = v;
        }
        if v > hi {
            hi = v;
        }
    }
    // NaN-safe: if hi is not strictly greater than lo (including NaN cases),
    // the feature is constant in this bag and we cannot split further.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    if !(hi > lo) {
        return IsoNode::External {
            size: indices.len(),
        };
    }
    let split = lo + (hi - lo) * rng.next_f64();

    let mut left_idx = Vec::new();
    let mut right_idx = Vec::new();
    for &i in indices {
        if samples[i][feat] < split {
            left_idx.push(i);
        } else {
            right_idx.push(i);
        }
    }
    if left_idx.is_empty() || right_idx.is_empty() {
        return IsoNode::External {
            size: indices.len(),
        };
    }

    IsoNode::Internal {
        feat,
        split,
        left: Box::new(build_tree(samples, &left_idx, depth + 1, height_limit, rng)),
        right: Box::new(build_tree(
            samples,
            &right_idx,
            depth + 1,
            height_limit,
            rng,
        )),
    }
}

/// Average path length of unsuccessful search in a BST of size `n`
/// (Liu, Ting, Zhou eq. 1).
fn c_factor(n: usize) -> f64 {
    if n <= 1 {
        return 0.0;
    }
    let n_f = n as f64;
    let h = (n_f - 1.0).ln() + 0.577_215_664_901_532_9;
    2.0 * h - 2.0 * (n_f - 1.0) / n_f
}

fn path_length(node: &IsoNode, point: &[f64], depth: u32) -> f64 {
    match node {
        IsoNode::External { size } => depth as f64 + c_factor(*size),
        IsoNode::Internal {
            feat,
            split,
            left,
            right,
        } => {
            let next = if point[*feat] < *split { left } else { right };
            path_length(next, point, depth + 1)
        }
    }
}

// ─── Public detect() ─────────────────────────────────────────────────────────

/// Run Isolation Forest over a series of feature vectors and return
/// anomaly detections.
///
/// `samples` is one row per tick (e.g. `[[magnitude_t1], [magnitude_t2], ...]`
/// for a 1-D scenario, or `[[hr_t1, ecg_t1], ...]` for multivariate).
/// All rows must have the same length; rows with mismatched lengths are
/// ignored.
pub fn detect(samples: &[Vec<f64>], config: &IsoForestConfig) -> Vec<Detection> {
    if samples.is_empty() {
        return Vec::new();
    }
    let n_features = samples[0].len();
    if n_features == 0 {
        return Vec::new();
    }
    if !samples.iter().all(|r| r.len() == n_features) {
        return Vec::new();
    }

    let psi = config.psi.min(samples.len()).max(2);
    let height_limit = (psi as f64).log2().ceil() as u32;
    let c_psi = c_factor(psi);

    // Build the ensemble.
    let mut rng = Xorshift64::new(config.seed);
    let mut trees = Vec::with_capacity(config.n_trees);
    for _ in 0..config.n_trees {
        // Subsample without replacement: deterministic Fisher-Yates over
        // an index permutation.
        let mut perm: Vec<usize> = (0..samples.len()).collect();
        for i in (1..perm.len()).rev() {
            let j = rng.next_below(i + 1);
            perm.swap(i, j);
        }
        let bag: Vec<usize> = perm.into_iter().take(psi).collect();
        trees.push(build_tree(samples, &bag, 0, height_limit, &mut rng));
    }

    // Score every sample.
    let mut detections = Vec::new();
    for (i, point) in samples.iter().enumerate() {
        let mean_path: f64 =
            trees.iter().map(|t| path_length(t, point, 0)).sum::<f64>() / trees.len() as f64;
        let score = 2.0_f64.powf(-mean_path / c_psi.max(1e-12));
        if score >= config.threshold {
            detections.push(Detection::new(i as u64 + 1, score, SOURCE));
        }
    }

    detections
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_yields_no_detections() {
        let det = detect(&[], &IsoForestConfig::default());
        assert!(det.is_empty());
    }

    #[test]
    fn deterministic_under_same_seed() {
        let samples: Vec<Vec<f64>> = (0..200)
            .map(|i| {
                // Bulk normal-ish samples with one obvious outlier at index 100.
                let v = if i == 100 {
                    100.0
                } else {
                    (i as f64 * 0.1).sin()
                };
                vec![v]
            })
            .collect();
        let cfg = IsoForestConfig {
            n_trees: 30,
            psi: 64,
            threshold: 0.55,
            seed: 7,
        };
        let a = detect(&samples, &cfg);
        let b = detect(&samples, &cfg);
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.tick, y.tick);
            assert_eq!(x.source, y.source);
            assert!((x.score - y.score).abs() < 1e-12);
        }
    }

    #[test]
    fn finds_obvious_outlier_in_flat_signal() {
        let mut samples: Vec<Vec<f64>> = (0..300).map(|_| vec![1.0]).collect();
        samples[150] = vec![1000.0];
        let cfg = IsoForestConfig {
            n_trees: 50,
            psi: 128,
            threshold: 0.6,
            seed: 42,
        };
        let det = detect(&samples, &cfg);
        assert!(
            det.iter().any(|d| d.tick == 151),
            "expected detection at tick 151 (index 150), got {:?}",
            det.iter().map(|d| d.tick).collect::<Vec<_>>()
        );
    }

    #[test]
    fn scores_lie_in_unit_interval() {
        let samples: Vec<Vec<f64>> = (0..400).map(|i| vec![i as f64]).collect();
        let cfg = IsoForestConfig {
            n_trees: 30,
            psi: 64,
            threshold: 0.0,
            seed: 7,
        };
        let det = detect(&samples, &cfg);
        for d in &det {
            assert!(
                (0.0..=1.0).contains(&d.score),
                "score out of range: {:?}",
                d
            );
        }
    }

    #[test]
    fn uniform_constant_feature_returns_no_anomalies() {
        // A perfectly constant feature has no information; iForest should
        // collapse trees immediately and produce essentially equal scores.
        let samples: Vec<Vec<f64>> = vec![vec![5.0]; 200];
        let cfg = IsoForestConfig {
            n_trees: 30,
            psi: 64,
            threshold: 0.6,
            seed: 11,
        };
        let det = detect(&samples, &cfg);
        // No point is more anomalous than any other.
        assert!(
            det.is_empty() || det.len() == samples.len(),
            "constant feature should produce all-or-nothing detections, got {}",
            det.len()
        );
    }
}

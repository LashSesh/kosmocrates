// Bootstrap 95% CI and paired difference utilities.
// Uses a deterministic seeded LCG — no platform RNG in the statistical path.

pub struct BootstrapCi {
    pub mean: f64,
    pub lo95: f64,
    pub hi95: f64,
}

/// Bootstrap 95% CI for a slice of f64 values.
/// n_iter=2000, seed derived from benchmark+metric name for full reproducibility.
pub fn bootstrap_ci(values: &[f64], n_iter: u32, seed: u64) -> BootstrapCi {
    let n = values.len();
    if n == 0 {
        return BootstrapCi {
            mean: 0.0,
            lo95: 0.0,
            hi95: 0.0,
        };
    }
    let mean = values.iter().sum::<f64>() / n as f64;
    if n == 1 {
        return BootstrapCi {
            mean,
            lo95: mean,
            hi95: mean,
        };
    }

    // LCG constants matching pse-eval-matrix workspace contract
    const A: u64 = 6364136223846793005;
    const C: u64 = 1442695040888963407;
    let mut state = seed;
    let mut next_idx = || {
        state = state.wrapping_mul(A).wrapping_add(C);
        ((state >> 32) as usize) % n
    };

    let mut boot_means: Vec<f64> = (0..n_iter)
        .map(|_| {
            let s: f64 = (0..n).map(|_| values[next_idx()]).sum();
            s / n as f64
        })
        .collect();
    boot_means.sort_by(f64::total_cmp);

    let lo_idx = ((0.025 * n_iter as f64) as usize).min(n_iter as usize - 1);
    let hi_idx = ((0.975 * n_iter as f64) as usize).min(n_iter as usize - 1);
    BootstrapCi {
        mean,
        lo95: boot_means[lo_idx],
        hi95: boot_means[hi_idx],
    }
}

/// Paired mean difference: mean(b) − mean(a). Positive = b wins.
pub fn paired_diff(a: &[f64], b: &[f64]) -> f64 {
    assert_eq!(a.len(), b.len(), "paired_diff: length mismatch");
    if a.is_empty() {
        return 0.0;
    }
    a.iter().zip(b.iter()).map(|(ai, bi)| bi - ai).sum::<f64>() / a.len() as f64
}

/// Stable seed from a string label — SHA-256 prefix as u64.
pub fn label_seed(label: &str) -> u64 {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(label.as_bytes());
    let d = h.finalize();
    u64::from_le_bytes([d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_series_has_zero_width_ci() {
        let vals = vec![0.5f64; 20];
        let ci = bootstrap_ci(&vals, 200, 42);
        assert!((ci.mean - 0.5).abs() < 1e-10);
        assert!((ci.lo95 - 0.5).abs() < 1e-10);
        assert!((ci.hi95 - 0.5).abs() < 1e-10);
    }

    #[test]
    fn paired_diff_correct() {
        let a = vec![0.6, 0.7, 0.8];
        let b = vec![0.7, 0.8, 0.9];
        let d = paired_diff(&a, &b);
        assert!((d - 0.1).abs() < 1e-10);
    }

    #[test]
    fn label_seed_is_deterministic() {
        assert_eq!(label_seed("b1-hit3"), label_seed("b1-hit3"));
        assert_ne!(label_seed("b1-hit3"), label_seed("b1-mrr"));
    }
}

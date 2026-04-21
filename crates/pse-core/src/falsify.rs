//! Surrogate-data falsification for Strand D.
//!
//! This module is the **pure-function core** of the falsification pass.
//! Step D-2 introduces only the surrogate generators; D-3 wires them to
//! a throwaway-engine driver that produces an empirical p-value; D-4
//! then opt-in integrates that into `macro_step` behind
//! [`pse_types::FalsificationConfig`].
//!
//! Why pure functions first: the generators are independently testable
//! against statistical properties (shuffle preserves the multiset,
//! block-bootstrap preserves first-moment structure within tolerance,
//! every method is deterministic under a fixed seed). Wiring them to
//! the engine afterwards is a single delta that does not change the
//! verified statistical core.
//!
//! Determinism note: every surrogate is generated from a single
//! `seed: u64` via xorshift64. This is the same family used elsewhere
//! in PSE for replayable runs and is consistent with Inv I4.

use pse_types::SurrogateMethod;

// ─── Tiny deterministic PRNG ─────────────────────────────────────────────────

/// Xorshift64 — same family used by iForest baseline and PSE's adapters.
pub(crate) struct Xorshift64(u64);

impl Xorshift64 {
    pub(crate) fn new(seed: u64) -> Self {
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

    pub(crate) fn next_below(&mut self, bound: usize) -> usize {
        debug_assert!(bound > 0);
        (self.next_u64() as usize) % bound
    }
}

// ─── Surrogate Generators ────────────────────────────────────────────────────

/// Errors from surrogate generation.
#[derive(Debug, thiserror::Error)]
pub enum SurrogateError {
    /// PhaseRandomize method requested but not yet implemented (lands in PR D-6).
    #[error("phase randomization is implemented in PR D-6")]
    PhaseRandomizeUnimplemented,
    /// Block size invalid (zero or larger than the observation count).
    #[error("invalid block size: {block_size} (observations: {n})")]
    InvalidBlockSize { block_size: usize, n: usize },
}

/// Generate one surrogate observation stream from the original stream.
///
/// The observation `Vec<u8>` payloads themselves are never modified; only
/// their *ordering* is permuted. The choice to permute ordering — rather
/// than perturb individual payloads — is intentional: it preserves the
/// marginal distribution of every observation (so any surrogate-vs-real
/// signal cannot be explained by a marginal-distribution shift) while
/// breaking temporal structure that would carry an actual anomaly.
pub fn generate_surrogate(
    observations: &[Vec<u8>],
    method: &SurrogateMethod,
    seed: u64,
) -> Result<Vec<Vec<u8>>, SurrogateError> {
    let mut rng = Xorshift64::new(seed);
    match method {
        SurrogateMethod::Shuffle => Ok(shuffle(observations, &mut rng)),
        SurrogateMethod::BlockBootstrap { block_size } => {
            block_bootstrap(observations, *block_size, &mut rng)
        }
        SurrogateMethod::PhaseRandomize => Err(SurrogateError::PhaseRandomizeUnimplemented),
    }
}

/// Uniformly shuffle the observation ordering (Fisher-Yates).
fn shuffle(observations: &[Vec<u8>], rng: &mut Xorshift64) -> Vec<Vec<u8>> {
    let mut out = observations.to_vec();
    if out.len() < 2 {
        return out;
    }
    for i in (1..out.len()).rev() {
        let j = rng.next_below(i + 1);
        out.swap(i, j);
    }
    out
}

/// Divide the stream into contiguous non-overlapping blocks of `block_size`,
/// then uniformly shuffle the *blocks*. The trailing partial block (if
/// the stream length is not a multiple of `block_size`) is treated as a
/// final shorter block — preserved as one unit so every observation
/// appears in the surrogate exactly once.
///
/// This preserves local temporal structure within each block while
/// breaking any signal that depends on the *positioning* of the blocks.
/// Useful for time-series anomalies where the local signature of an
/// event survives a within-block shuffle but the event's distinctiveness
/// lies in *when* it occurs.
fn block_bootstrap(
    observations: &[Vec<u8>],
    block_size: usize,
    rng: &mut Xorshift64,
) -> Result<Vec<Vec<u8>>, SurrogateError> {
    let n = observations.len();
    if block_size == 0 || block_size > n.max(1) {
        return Err(SurrogateError::InvalidBlockSize { block_size, n });
    }
    if n < 2 {
        return Ok(observations.to_vec());
    }
    // Cut into blocks.
    let n_full = n / block_size;
    let mut blocks: Vec<Vec<Vec<u8>>> = Vec::with_capacity(n_full + 1);
    for b in 0..n_full {
        let lo = b * block_size;
        let hi = lo + block_size;
        blocks.push(observations[lo..hi].to_vec());
    }
    if n % block_size != 0 {
        let lo = n_full * block_size;
        blocks.push(observations[lo..].to_vec());
    }
    // Fisher-Yates over the block sequence.
    if blocks.len() >= 2 {
        for i in (1..blocks.len()).rev() {
            let j = rng.next_below(i + 1);
            blocks.swap(i, j);
        }
    }
    // Flatten back to a single stream.
    Ok(blocks.into_iter().flatten().collect())
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn payloads(values: &[u32]) -> Vec<Vec<u8>> {
        values.iter().map(|v| v.to_le_bytes().to_vec()).collect()
    }

    fn sorted_multiset(observations: &[Vec<u8>]) -> Vec<Vec<u8>> {
        let mut s = observations.to_vec();
        s.sort();
        s
    }

    // ── Shuffle ───────────────────────────────────────────────────────────

    #[test]
    fn shuffle_preserves_multiset() {
        let original = payloads(&(0..100).collect::<Vec<_>>());
        let surrogate = generate_surrogate(&original, &SurrogateMethod::Shuffle, 42).unwrap();
        assert_eq!(surrogate.len(), original.len());
        assert_eq!(sorted_multiset(&original), sorted_multiset(&surrogate));
    }

    #[test]
    fn shuffle_actually_permutes_long_streams() {
        // For n=100 the probability of an identity permutation is 1/100!,
        // so this is statistically guaranteed for any reasonable seed.
        let original = payloads(&(0..100).collect::<Vec<_>>());
        let surrogate = generate_surrogate(&original, &SurrogateMethod::Shuffle, 42).unwrap();
        assert_ne!(original, surrogate);
    }

    #[test]
    fn shuffle_is_deterministic_under_same_seed() {
        let original = payloads(&(0..50).collect::<Vec<_>>());
        let a = generate_surrogate(&original, &SurrogateMethod::Shuffle, 7).unwrap();
        let b = generate_surrogate(&original, &SurrogateMethod::Shuffle, 7).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn shuffle_changes_with_seed() {
        let original = payloads(&(0..50).collect::<Vec<_>>());
        let a = generate_surrogate(&original, &SurrogateMethod::Shuffle, 7).unwrap();
        let b = generate_surrogate(&original, &SurrogateMethod::Shuffle, 8).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn shuffle_handles_trivial_input() {
        let one = payloads(&[42]);
        let s = generate_surrogate(&one, &SurrogateMethod::Shuffle, 1).unwrap();
        assert_eq!(s, one);
        let empty: Vec<Vec<u8>> = Vec::new();
        let e = generate_surrogate(&empty, &SurrogateMethod::Shuffle, 1).unwrap();
        assert!(e.is_empty());
    }

    // ── Block bootstrap ───────────────────────────────────────────────────

    #[test]
    fn block_bootstrap_preserves_multiset() {
        let original = payloads(&(0..100).collect::<Vec<_>>());
        let s = generate_surrogate(
            &original,
            &SurrogateMethod::BlockBootstrap { block_size: 10 },
            42,
        )
        .unwrap();
        assert_eq!(s.len(), original.len());
        assert_eq!(sorted_multiset(&original), sorted_multiset(&s));
    }

    #[test]
    fn block_bootstrap_keeps_within_block_order() {
        // For block_size = stream_len, there is exactly one block, so the
        // shuffle is a no-op and the within-block order is the original.
        let original = payloads(&(0..10).collect::<Vec<_>>());
        let s = generate_surrogate(
            &original,
            &SurrogateMethod::BlockBootstrap { block_size: 10 },
            123,
        )
        .unwrap();
        assert_eq!(s, original);
    }

    #[test]
    fn block_bootstrap_handles_uneven_division() {
        // 7 observations, block_size = 3 → blocks [0,1,2], [3,4,5], [6].
        // After shuffling, the trailing single-element block must still
        // appear contiguously somewhere in the output.
        let original = payloads(&(0..7).collect::<Vec<_>>());
        let s = generate_surrogate(
            &original,
            &SurrogateMethod::BlockBootstrap { block_size: 3 },
            5,
        )
        .unwrap();
        assert_eq!(s.len(), 7);
        assert_eq!(sorted_multiset(&original), sorted_multiset(&s));
    }

    #[test]
    fn block_bootstrap_rejects_zero_block_size() {
        let original = payloads(&[1, 2, 3]);
        let err = generate_surrogate(
            &original,
            &SurrogateMethod::BlockBootstrap { block_size: 0 },
            1,
        );
        assert!(matches!(err, Err(SurrogateError::InvalidBlockSize { .. })));
    }

    #[test]
    fn block_bootstrap_rejects_oversized_block() {
        let original = payloads(&[1, 2, 3]);
        let err = generate_surrogate(
            &original,
            &SurrogateMethod::BlockBootstrap { block_size: 10 },
            1,
        );
        assert!(matches!(err, Err(SurrogateError::InvalidBlockSize { .. })));
    }

    #[test]
    fn block_bootstrap_is_deterministic_under_same_seed() {
        let original = payloads(&(0..40).collect::<Vec<_>>());
        let a = generate_surrogate(
            &original,
            &SurrogateMethod::BlockBootstrap { block_size: 5 },
            99,
        )
        .unwrap();
        let b = generate_surrogate(
            &original,
            &SurrogateMethod::BlockBootstrap { block_size: 5 },
            99,
        )
        .unwrap();
        assert_eq!(a, b);
    }

    // ── Phase randomize stub ──────────────────────────────────────────────

    #[test]
    fn phase_randomize_returns_unimplemented_marker() {
        let original = payloads(&[1, 2, 3]);
        let err = generate_surrogate(&original, &SurrogateMethod::PhaseRandomize, 1);
        assert!(matches!(
            err,
            Err(SurrogateError::PhaseRandomizeUnimplemented)
        ));
    }
}

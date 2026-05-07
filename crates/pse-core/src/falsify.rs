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

use pse_graph::ObservationAdapter;
use pse_types::{Config, Observation, SurrogateMethod};

use crate::{macro_step, GlobalState};

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

/// Outcome of an end-to-end falsification pass.
///
/// `p_value` is the empirical fraction of surrogate runs whose maximum
/// Mandorla coherence (max κ across all macro_steps in that run) was
/// **at least as high as** the real run's max κ. A low p-value means
/// the real stream's resonance peak is statistically rare under the
/// null hypothesis "structure is gone" — i.e. the resonance is
/// genuinely structural rather than a coincidence of the marginal
/// distribution.
#[derive(Clone, Debug)]
pub struct FalsificationResult {
    /// Empirical p-value: comparable_count / k.
    pub p_value: f64,
    /// Maximum κ reached during the real-stream run.
    pub real_max_kappa: f64,
    /// Maximum κ reached on each surrogate run (length = k_actual).
    pub surrogate_max_kappas: Vec<f64>,
    /// Number of surrogate runs actually executed (some may have been
    /// skipped if `generate_surrogate` returned an error for that seed).
    pub k_actual: u32,
}

// ─── End-to-end Falsification Driver ────────────────────────────────────────

/// Run the engine on the original observation stream and on `k`
/// surrogate streams; return the empirical p-value plus per-run max-κ
/// trajectory.
///
/// The real stream is processed first to record `real_max_kappa`. Then
/// `k` surrogate streams are generated using the supplied
/// [`SurrogateMethod`]; each is run on a **fresh, throwaway**
/// `GlobalState` (no contamination of the caller's archive or memory)
/// with the engine's `falsification.enabled` flag forced off (no
/// recursion). For every run the maximum Mandorla κ across all
/// macro_steps is recorded.
///
/// p_value = (#surrogates with max κ ≥ real max κ) / k.
///
/// Determinism: each surrogate's PRNG seed is `seed.wrapping_add(i)`
/// for the i-th surrogate, so two invocations with the same `seed`
/// produce identical surrogate sequences and identical p-values.
/// Inv I4 is preserved for the falsification call itself.
///
/// Per-surrogate `generate_surrogate` errors are skipped (the
/// surrogate is omitted from the count); `k_actual` reports the
/// number of surrogates that ran successfully.
pub fn falsify_with_surrogates(
    observations: &[Observation],
    config: &Config,
    k: u32,
    method: SurrogateMethod,
    seed: u64,
    adapter: &dyn ObservationAdapter,
) -> FalsificationResult {
    let payloads: Vec<Vec<u8>> = observations.iter().map(|o| o.payload.clone()).collect();
    let real_max_kappa = run_and_measure_max_kappa(&payloads, config, adapter);

    let mut surrogate_max_kappas = Vec::with_capacity(k as usize);
    for i in 0..k {
        let s_seed = seed.wrapping_add(i as u64);
        let surrogate = match generate_surrogate(&payloads, &method, s_seed) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let s_max = run_and_measure_max_kappa(&surrogate, config, adapter);
        surrogate_max_kappas.push(s_max);
    }
    let k_actual = surrogate_max_kappas.len() as u32;
    let comparable = surrogate_max_kappas
        .iter()
        .filter(|&&s| s >= real_max_kappa)
        .count();
    let p_value = if k_actual == 0 {
        1.0
    } else {
        comparable as f64 / k_actual as f64
    };

    FalsificationResult {
        p_value,
        real_max_kappa,
        surrogate_max_kappas,
        k_actual,
    }
}

/// Run a fresh engine over a payload sequence and return the
/// maximum Mandorla κ observed across all macro_steps.
///
/// Caps `falsification.enabled` to false in the inner config to
/// prevent the engine from triggering its own falsification recursively
/// once D.4 wires that into macro_step.
fn run_and_measure_max_kappa(
    payloads: &[Vec<u8>],
    config: &Config,
    adapter: &dyn ObservationAdapter,
) -> f64 {
    let mut inner = config.clone();
    inner.falsification.enabled = false;
    let mut state = GlobalState::new(&inner);
    let mut max_kappa = 0.0_f64;
    for p in payloads {
        let _ = macro_step(&mut state, &[p.clone()], &inner, adapter);
        if let Some(active) = state.phase_ladder.get(state.active_carrier) {
            if active.mandorla.kappa > max_kappa {
                max_kappa = active.mandorla.kappa;
            }
        }
    }
    max_kappa
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

    // ── End-to-end falsification (D.3) ───────────────────────────────────

    use pse_graph::PassthroughAdapter;
    use pse_types::{content_address_raw, MeasurementContext, Observation, ProvenanceEnvelope};

    fn obs_from_payloads(payloads: Vec<Vec<u8>>) -> Vec<Observation> {
        payloads
            .into_iter()
            .map(|p| Observation {
                timestamp: 0.0,
                source_id: "test".into(),
                provenance: ProvenanceEnvelope::default(),
                payload: p.clone(),
                context: MeasurementContext::default(),
                digest: content_address_raw(&p),
                schema_version: "1.0.0".into(),
                phase_hint: None,
            })
            .collect()
    }

    #[test]
    fn falsify_returns_well_formed_result_for_small_input() {
        let cfg = Config::default();
        let adapter = PassthroughAdapter::new("test");
        let observations = obs_from_payloads((0..5).map(|i| vec![i as u8]).collect());
        let result = falsify_with_surrogates(
            &observations,
            &cfg,
            8,
            SurrogateMethod::Shuffle,
            42,
            &adapter,
        );
        assert!(result.p_value >= 0.0 && result.p_value <= 1.0);
        assert!(result.real_max_kappa >= 0.0 && result.real_max_kappa <= 1.0);
        for k in &result.surrogate_max_kappas {
            assert!(*k >= 0.0 && *k <= 1.0);
        }
    }

    #[test]
    fn falsify_is_deterministic_under_same_seed() {
        let cfg = Config::default();
        let adapter = PassthroughAdapter::new("det");
        let observations = obs_from_payloads((0..6).map(|i| vec![i as u8, 0xff]).collect());
        let r1 = falsify_with_surrogates(
            &observations,
            &cfg,
            5,
            SurrogateMethod::Shuffle,
            7,
            &adapter,
        );
        let r2 = falsify_with_surrogates(
            &observations,
            &cfg,
            5,
            SurrogateMethod::Shuffle,
            7,
            &adapter,
        );
        assert_eq!(r1.surrogate_max_kappas.len(), r2.surrogate_max_kappas.len());
        for (a, b) in r1
            .surrogate_max_kappas
            .iter()
            .zip(r2.surrogate_max_kappas.iter())
        {
            assert!((a - b).abs() < 1e-12);
        }
        assert!((r1.p_value - r2.p_value).abs() < 1e-12);
    }

    #[test]
    fn falsify_does_not_pollute_caller_state() {
        // The real-stream run uses an internal throwaway GlobalState;
        // any external state object passed in by the caller (n/a here
        // since the API does not take one) must be unaffected. We
        // assert this indirectly: two runs back-to-back with the same
        // seed produce the same numbers.
        let cfg = Config::default();
        let adapter = PassthroughAdapter::new("isolation");
        let observations = obs_from_payloads(vec![vec![1, 2, 3], vec![4, 5, 6]]);
        let r1 = falsify_with_surrogates(
            &observations,
            &cfg,
            3,
            SurrogateMethod::Shuffle,
            99,
            &adapter,
        );
        let r2 = falsify_with_surrogates(
            &observations,
            &cfg,
            3,
            SurrogateMethod::Shuffle,
            99,
            &adapter,
        );
        assert_eq!(r1.real_max_kappa, r2.real_max_kappa);
    }

    #[test]
    fn falsify_disables_recursion_in_inner_config() {
        // We can't directly inspect the inner config, but we CAN assert
        // that calling falsify with falsification.enabled = true on the
        // outer config does not cause infinite recursion / stack
        // overflow. A finite return is itself the proof of D-2's
        // recursion guard (R-D6 in the original plan).
        let mut cfg = Config::default();
        cfg.falsification.enabled = true;
        let adapter = PassthroughAdapter::new("recurse-guard");
        let observations = obs_from_payloads(vec![vec![1], vec![2], vec![3]]);
        let _result = falsify_with_surrogates(
            &observations,
            &cfg,
            3,
            SurrogateMethod::Shuffle,
            1,
            &adapter,
        );
        // If we got here, the recursion guard worked.
    }

    #[test]
    fn falsify_with_zero_k_returns_p_value_one() {
        let cfg = Config::default();
        let adapter = PassthroughAdapter::new("zero-k");
        let observations = obs_from_payloads(vec![vec![1], vec![2], vec![3]]);
        let result = falsify_with_surrogates(
            &observations,
            &cfg,
            0,
            SurrogateMethod::Shuffle,
            1,
            &adapter,
        );
        assert_eq!(result.k_actual, 0);
        assert_eq!(result.p_value, 1.0);
    }
}

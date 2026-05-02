//! Generic PSE driver that turns a stream of raw observation payloads into
//! a stream of [`Detection`]s.
//!
//! Two detection channels are emitted:
//!  - `"pse_crystal"` for every successful crystal commit (one Detection per
//!    `Ok(Some(crystal))` return); score is `crystal.stability_score`.
//!  - `"pse_memory_hit"` whenever the internal pattern memory short-circuits
//!    a tick (the `state.pattern_hits` counter ticks up); score is `0.5`
//!    since memory hits do not carry an intrinsic stability score.
//!
//! Detection `tick` is the value of `state.commit_index` **immediately after**
//! the `macro_step` call — PSE increments `commit_index` unconditionally at
//! the top of every macro-step, so this is a monotonically-increasing tick
//! index whether or not a crystal was committed.
//!
//! Two ingestion modes:
//!  - [`run_pse`] feeds one raw payload per macro_step (the original
//!    single-observation mode). Useful when the surrounding test wants
//!    to mirror the canonical "one tick = one event" semantics, but
//!    structurally limited: every payload from a single-source adapter
//!    collapses onto one graph vertex with no edges, so j-density
//!    saturates at 0 and the Kairos gate never opens.
//!  - [`run_pse_windowed`] feeds an expanding-then-sliding window of the
//!    last `window_size` observations per macro_step, paired with an
//!    [`EventScopedAdapter`] that derives a deterministic per-event
//!    source-id from the payload digest. This is the canonical PSE
//!    ingestion pattern: every distinct payload becomes its own vertex,
//!    pairwise-within-batch edge creation builds real topology, and the
//!    Engine work from Strand E (E.1–E.7) finally has substrate to act on.

use std::sync::atomic::{AtomicUsize, Ordering};

use pse_core::{macro_step, GlobalState};
use pse_graph::{ObservationAdapter, ObserveError};
use pse_types::{
    content_address_raw, Config, Hash256, MeasurementContext, Observation, ProvenanceEnvelope,
};

use crate::Detection;

/// Drive PSE over a stream of raw payloads, **one payload per macro_step**.
///
/// Backward-compatible mode preserved for prior scenarios. See module
/// docs for the structural caveat (single-source collapse).
pub fn run_pse(
    state: &mut GlobalState,
    observations: &[Vec<u8>],
    config: &Config,
    adapter: &dyn ObservationAdapter,
) -> Vec<Detection> {
    let mut detections = Vec::new();

    for payload in observations {
        let batch = vec![payload.clone()];
        let hits_before = state.pattern_hits;

        match macro_step(state, &batch, config, adapter) {
            Ok(Some(crystal)) => {
                detections.push(Detection::new(
                    state.commit_index,
                    crystal.stability_score.clamp(0.0, 1.0),
                    "pse_crystal",
                ));
            }
            Ok(None) => {
                if state.pattern_hits > hits_before {
                    detections.push(Detection::new(
                        state.commit_index,
                        0.5,
                        "pse_memory_hit",
                    ));
                }
            }
            Err(_) => {}
        }
    }

    detections
}

/// Adapter that issues a deterministic, *per-event* `source_id` derived
/// from the payload digest, while preserving idempotency under
/// content-address.
///
/// Mechanism: `canonicalize(raw)` computes
/// `digest = SHA-256(raw)` and emits an Observation with
/// `source_id = "{base}:event:{first 8 bytes of digest, hex-encoded}"`.
/// The same payload therefore always lands on the same vertex; two
/// different payloads always land on different vertices. This is what
/// makes pairwise-within-batch edge creation produce a non-trivial
/// graph topology when the runner feeds windowed batches.
pub struct EventScopedAdapter {
    base: String,
    schema: String,
    /// Diagnostic counter — increments on every canonicalize call.
    /// Read-only from outside; useful for asserting that the adapter
    /// was actually exercised.
    canonicalized: AtomicUsize,
}

impl EventScopedAdapter {
    /// Create an adapter for a base source label
    /// (e.g. `"seismo"`, `"vitals_b"`, `"binance"`).
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            schema: "1.0.0".into(),
            canonicalized: AtomicUsize::new(0),
        }
    }

    /// Number of canonicalize calls this adapter has serviced
    /// since construction.
    pub fn calls(&self) -> usize {
        self.canonicalized.load(Ordering::SeqCst)
    }

    fn event_source_id(&self, digest: &Hash256) -> String {
        let v = u64::from_le_bytes(digest[0..8].try_into().expect("SHA-256 has ≥ 8 bytes"));
        format!("{}:event:{:016x}", self.base, v)
    }
}

impl ObservationAdapter for EventScopedAdapter {
    fn source_id(&self) -> &str {
        &self.base
    }

    fn canonicalize(
        &self,
        raw: &[u8],
        context: &MeasurementContext,
    ) -> Result<Observation, ObserveError> {
        let payload = raw.to_vec();
        let digest: Hash256 = content_address_raw(&payload);
        let source_id = self.event_source_id(&digest);
        self.canonicalized.fetch_add(1, Ordering::SeqCst);
        Ok(Observation {
            timestamp: 0.0,
            source_id,
            provenance: ProvenanceEnvelope {
                origin: self.base.clone(),
                chain: Vec::new(),
                sig: None,
            },
            payload,
            context: context.clone(),
            digest,
            schema_version: self.schema.clone(),
        })
    }
}

/// Drive PSE over a stream of raw payloads using an
/// **expanding-then-sliding window** of the last `window_size`
/// observations per macro_step, with an [`EventScopedAdapter`] so each
/// payload becomes its own graph vertex.
///
/// At step `k`, the window is `observations[max(0, k+1-window_size) ..= k]`.
/// For `k < window_size − 1` the window grows; afterwards it slides.
/// At the largest stable size, every macro_step ingests `window_size`
/// observations and PSE creates pairwise-within-batch edges, giving
/// the graph real topological structure for E.3's 5D state and E.4's
/// resonance tests to operate on.
///
/// Returns one [`Detection`] per macro_step that produced a crystal
/// or a pattern-memory hit, keyed by `state.commit_index` after the
/// step (i.e. tick `k + 1` for the k-th input).
pub fn run_pse_windowed(
    state: &mut GlobalState,
    observations: &[Vec<u8>],
    config: &Config,
    base_source: &str,
    window_size: usize,
) -> Vec<Detection> {
    let adapter = EventScopedAdapter::new(base_source);
    let window = window_size.max(1);
    let mut detections = Vec::new();

    for k in 0..observations.len() {
        let lo = (k + 1).saturating_sub(window);
        let batch: Vec<Vec<u8>> = observations[lo..=k].to_vec();
        let hits_before = state.pattern_hits;

        match macro_step(state, &batch, config, &adapter) {
            Ok(Some(crystal)) => {
                detections.push(Detection::new(
                    state.commit_index,
                    crystal.stability_score.clamp(0.0, 1.0),
                    "pse_crystal",
                ));
            }
            Ok(None) => {
                if state.pattern_hits > hits_before {
                    detections.push(Detection::new(
                        state.commit_index,
                        0.5,
                        "pse_memory_hit",
                    ));
                }
            }
            Err(_) => {}
        }
    }

    detections
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_scoped_adapter_idempotent_under_same_payload() {
        let adapter = EventScopedAdapter::new("test");
        let ctx = MeasurementContext::default();
        let raw = b"hello world".to_vec();
        let o1 = adapter.canonicalize(&raw, &ctx).unwrap();
        let o2 = adapter.canonicalize(&raw, &ctx).unwrap();
        assert_eq!(o1.source_id, o2.source_id);
        assert_eq!(o1.digest, o2.digest);
    }

    #[test]
    fn event_scoped_adapter_distinct_payloads_distinct_source_ids() {
        let adapter = EventScopedAdapter::new("test");
        let ctx = MeasurementContext::default();
        let o1 = adapter.canonicalize(b"first", &ctx).unwrap();
        let o2 = adapter.canonicalize(b"second", &ctx).unwrap();
        assert_ne!(o1.source_id, o2.source_id);
        assert!(o1.source_id.starts_with("test:event:"));
        assert!(o2.source_id.starts_with("test:event:"));
    }

    #[test]
    fn event_scoped_adapter_canonical_calls_increment() {
        let adapter = EventScopedAdapter::new("test");
        assert_eq!(adapter.calls(), 0);
        let ctx = MeasurementContext::default();
        adapter.canonicalize(b"a", &ctx).unwrap();
        adapter.canonicalize(b"b", &ctx).unwrap();
        adapter.canonicalize(b"c", &ctx).unwrap();
        assert_eq!(adapter.calls(), 3);
    }

    #[test]
    fn windowed_runner_grows_then_slides() {
        let cfg = Config::default();
        let mut state = GlobalState::new(&cfg);
        let observations: Vec<Vec<u8>> = (0..10).map(|i| vec![i as u8]).collect();
        let _ = run_pse_windowed(&mut state, &observations, &cfg, "test", 3);
        // The graph must now have at least one vertex per *distinct*
        // payload, so the windowed-mode invariant holds: distinct
        // observations no longer collapse onto a single vertex.
        assert!(state.graph.id_map.len() >= observations.len(),
                "expected at least {} vertices in graph, got {}",
                observations.len(), state.graph.id_map.len());
    }

    #[test]
    fn windowed_runner_creates_edges() {
        // The whole point of windowed mode: pairwise-within-batch edge
        // creation produces a non-trivial graph topology.
        let cfg = Config::default();
        let mut state = GlobalState::new(&cfg);
        let observations: Vec<Vec<u8>> = (0..20).map(|i| vec![i as u8, (i * 7) as u8]).collect();
        let _ = run_pse_windowed(&mut state, &observations, &cfg, "test", 5);
        // With window=5 and 20 observations, after the window stabilises
        // every batch produces C(5, 2) = 10 pairwise edges. Even with
        // deduplication and weight decay, the graph cannot be edgeless.
        assert!(state.graph.graph.edge_count() > 0,
                "windowed runner must produce graph edges, got {}",
                state.graph.graph.edge_count());
    }
}


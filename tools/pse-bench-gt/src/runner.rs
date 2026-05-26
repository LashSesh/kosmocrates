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
    content_address_raw, Config, GateSnapshot, Hash256, MeasurementContext, Observation,
    ProvenanceEnvelope,
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
                let source = if config.consensus.startup_crystal_count > 0
                    && state.crystals_committed <= config.consensus.startup_crystal_count
                {
                    "pse_crystal_startup"
                } else {
                    "pse_crystal"
                };
                detections.push(Detection::new(
                    state.commit_index,
                    crystal.stability_score.clamp(0.0, 1.0),
                    source,
                ));
            }
            Ok(None) => {
                if state.pattern_hits > hits_before {
                    detections.push(Detection::new(state.commit_index, 0.5, "pse_memory_hit"));
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
    /// Optional Strand-I phase extractor: given the raw payload bytes,
    /// return a semantic phase in `[0, 2π)` or `None` to fall back to
    /// the avalanche-hash phase from the digest. Set via
    /// [`EventScopedAdapter::with_phase_fn`].
    phase_fn: Option<Box<dyn Fn(&[u8]) -> Option<f64> + Send + Sync>>,
}

impl EventScopedAdapter {
    /// Create an adapter for a base source label
    /// (e.g. `"seismo"`, `"vitals_b"`, `"binance"`).
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            schema: "1.0.0".into(),
            canonicalized: AtomicUsize::new(0),
            phase_fn: None,
        }
    }

    /// Attach a Strand-I semantic phase extractor. The closure is called
    /// on every raw payload during `canonicalize`; its return value is
    /// stored in `Observation::phase_hint`. Returning `None` falls back
    /// to the avalanche-hash phase from the digest (legacy behaviour).
    pub fn with_phase_fn<F>(mut self, f: F) -> Self
    where
        F: Fn(&[u8]) -> Option<f64> + Send + Sync + 'static,
    {
        self.phase_fn = Some(Box::new(f));
        self
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
        let phase_hint = self.phase_fn.as_ref().and_then(|f| f(raw));
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
            phase_hint,
        })
    }
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RunnerDiagnostics {
    pub observation_count: u64,
    pub window_count: u64,
    pub window_size: usize,
    pub adapter_event_count: u64,
    pub candidate_count: u64,
    pub gate_pass_count: u64,
    pub gate_hold_count: u64,
    pub gate_reject_count: u64,
    pub last_gate_reason: Option<String>,
    pub max_resonance_score: Option<f64>,
    pub max_kappa: Option<f64>,
    pub min_threshold: Option<f64>,
    pub warmup_remaining: Option<u64>,
    pub calibration_mode: String,
    pub engine_outcome_counts: std::collections::BTreeMap<String, u64>,
    pub gate_ticks: Vec<GateTickDiagnostic>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct GateTickDiagnostic {
    pub tick: u64,
    pub in_ground_truth_window: bool,
    pub gate_d: f64,
    pub gate_q: f64,
    pub gate_g: f64,
    pub gate_k: f64,
    pub gate_r: f64,
    pub gate_j: f64,
    pub gate_p: f64,
    pub gate_n: f64,
    pub kairos: bool,
}

fn gate_reasons(g: &GateSnapshot, c: &Config) -> String {
    let mut r = Vec::new();
    if g.d < c.thresholds.d {
        r.push("d");
    }
    if g.q < c.thresholds.q {
        r.push("q");
    }
    if g.r < c.thresholds.r {
        r.push("r");
    }
    if g.g < c.thresholds.g {
        r.push("g");
    }
    if g.j < c.thresholds.j {
        r.push("j");
    }
    if g.p < c.thresholds.p {
        r.push("p");
    }
    if g.n < c.thresholds.n {
        r.push("n");
    }
    if g.k < c.thresholds.k {
        r.push("k");
    }
    if r.is_empty() {
        "kairos=false".to_string()
    } else {
        format!("{} below threshold", r.join(","))
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
    ground_truth_windows: &[(u64, u64)],
) -> (Vec<Detection>, RunnerDiagnostics) {
    run_pse_windowed_inner(
        state,
        observations,
        config,
        EventScopedAdapter::new(base_source),
        window_size,
        ground_truth_windows,
    )
}

/// Variant of [`run_pse_windowed`] that attaches a Strand-I semantic
/// phase extractor to the adapter. The `phase_fn` closure is invoked
/// on every raw payload; its return value populates
/// `Observation::phase_hint`, so domain-aware bench scenarios can
/// drive PSE's Mandorla coherence with the data's semantic phase.
pub fn run_pse_windowed_with_phase<F>(
    state: &mut GlobalState,
    observations: &[Vec<u8>],
    config: &Config,
    base_source: &str,
    window_size: usize,
    phase_fn: F,
    ground_truth_windows: &[(u64, u64)],
) -> (Vec<Detection>, RunnerDiagnostics)
where
    F: Fn(&[u8]) -> Option<f64> + Send + Sync + 'static,
{
    let adapter = EventScopedAdapter::new(base_source).with_phase_fn(phase_fn);
    run_pse_windowed_inner(
        state,
        observations,
        config,
        adapter,
        window_size,
        ground_truth_windows,
    )
}

fn run_pse_windowed_inner(
    state: &mut GlobalState,
    observations: &[Vec<u8>],
    config: &Config,
    adapter: EventScopedAdapter,
    window_size: usize,
    ground_truth_windows: &[(u64, u64)],
) -> (Vec<Detection>, RunnerDiagnostics) {
    let window = window_size.max(1);
    let mut detections = Vec::new();
    let mut diag = RunnerDiagnostics {
        observation_count: observations.len() as u64,
        window_size: window,
        calibration_mode: if state.adaptive.is_some() {
            "adaptive".into()
        } else {
            "static".into()
        },
        min_threshold: Some(
            [
                config.thresholds.d,
                config.thresholds.q,
                config.thresholds.r,
                config.thresholds.g,
                config.thresholds.j,
                config.thresholds.p,
                config.thresholds.n,
                config.thresholds.k,
            ]
            .into_iter()
            .fold(f64::INFINITY, f64::min),
        ),
        ..Default::default()
    };
    for k in 0..observations.len() {
        let lo = (k + 1).saturating_sub(window);
        diag.window_count += 1;
        let batch: Vec<Vec<u8>> = observations[lo..=k].to_vec();
        let hits_before = state.pattern_hits;

        match macro_step(state, &batch, config, &adapter) {
            Ok(Some(crystal)) => {
                *diag
                    .engine_outcome_counts
                    .entry("crystal".into())
                    .or_insert(0) += 1;
                diag.candidate_count += 1;
                diag.gate_pass_count += 1;
                let source = if config.consensus.startup_crystal_count > 0
                    && state.crystals_committed <= config.consensus.startup_crystal_count
                {
                    "pse_crystal_startup"
                } else {
                    "pse_crystal"
                };
                detections.push(Detection::new(
                    state.commit_index,
                    crystal.stability_score.clamp(0.0, 1.0),
                    source,
                ));
            }
            Ok(None) => {
                *diag.engine_outcome_counts.entry("none".into()).or_insert(0) += 1;
                diag.gate_hold_count += 1;
                if let Some(g) = &state.last_gate {
                    diag.candidate_count += 1;
                    if g.kairos {
                        diag.gate_pass_count += 1;
                    } else {
                        diag.gate_reject_count += 1;
                        diag.last_gate_reason = Some(gate_reasons(g, config));
                    }
                    let kappa = g.k;
                    let res = g.r;
                    diag.max_kappa = Some(diag.max_kappa.map_or(kappa, |m| m.max(kappa)));
                    diag.max_resonance_score =
                        Some(diag.max_resonance_score.map_or(res, |m| m.max(res)));
                    diag.gate_ticks.push(GateTickDiagnostic {
                        tick: state.commit_index,
                        in_ground_truth_window: ground_truth_windows
                            .iter()
                            .any(|(s, e)| state.commit_index >= *s && state.commit_index < *e),
                        gate_d: g.d,
                        gate_q: g.q,
                        gate_g: g.g,
                        gate_k: g.k,
                        gate_r: g.r,
                        gate_j: g.j,
                        gate_p: g.p,
                        gate_n: g.n,
                        kairos: g.kairos,
                    });
                }
                if state.pattern_hits > hits_before {
                    detections.push(Detection::new(state.commit_index, 0.5, "pse_memory_hit"));
                }
            }
            Err(_) => {
                *diag
                    .engine_outcome_counts
                    .entry("error".into())
                    .or_insert(0) += 1;
            }
        }
    }

    diag.adapter_event_count = adapter.calls() as u64;
    diag.warmup_remaining = state
        .adaptive
        .as_ref()
        .map(|a| (a.warmup_ticks as u64).saturating_sub(a.seen));
    (detections, diag)
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
        let _ = run_pse_windowed(&mut state, &observations, &cfg, "test", 3, &[]);
        // The graph must now have at least one vertex per *distinct*
        // payload, so the windowed-mode invariant holds: distinct
        // observations no longer collapse onto a single vertex.
        assert!(
            state.graph.id_map.len() >= observations.len(),
            "expected at least {} vertices in graph, got {}",
            observations.len(),
            state.graph.id_map.len()
        );
    }

    #[test]
    fn windowed_runner_creates_edges() {
        // The whole point of windowed mode: pairwise-within-batch edge
        // creation produces a non-trivial graph topology.
        let cfg = Config::default();
        let mut state = GlobalState::new(&cfg);
        let observations: Vec<Vec<u8>> = (0..20).map(|i| vec![i as u8, (i * 7) as u8]).collect();
        let _ = run_pse_windowed(&mut state, &observations, &cfg, "test", 5, &[]);
        // With window=5 and 20 observations, after the window stabilises
        // every batch produces C(5, 2) = 10 pairwise edges. Even with
        // deduplication and weight decay, the graph cannot be edgeless.
        assert!(
            state.graph.graph.edge_count() > 0,
            "windowed runner must produce graph edges, got {}",
            state.graph.graph.edge_count()
        );
    }
}

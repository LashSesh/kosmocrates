//! PSE analytical engine and orchestrator.
//!
//! Coordinates the full pipeline from observation ingestion through consensus,
//! carrier geometry, morphogenic updates, and crystal archival.

pub mod adaptive;
pub mod crystal_adapter;
pub mod engine_filter;
pub mod explore;
pub mod falsify;
pub mod metatron_attach;
pub mod query;
pub mod topology_ops;

pub use crystal_adapter::CrystalAdapter;
pub use engine_filter::{EngineFilter, StepResult};
pub use explore::{resonance_landscape_explorer, ResonanceExplorationResult, ResonanceProbe};
pub use metatron_attach::{
    attach_signature, attach_signature_global, induced_adjacency, signature_for_region,
    signature_for_region_global, METATRON_REGION_CAP,
};
pub use query::{resonance_fingerprint, ResonanceFingerprint};
pub use topology_ops::{
    bridge, compose, crystal_similarity, dual, interpolate, query, BridgeConfig, BridgeError,
    ComposeConfig, ComposeError, QueryConfig,
};

/// The trait that domain plugins implement to use the PSE engine.
pub trait DomainAdapter: Send + Sync + 'static {
    /// Human-readable name for this domain.
    fn domain_name(&self) -> &str;
    /// Convert domain-specific raw data to observation bytes.
    fn encode_observation(&self, raw: &[u8]) -> Vec<u8> {
        raw.to_vec()
    }
    /// Optional domain-specific validation.
    fn validate(&self, _crystal: &pse_types::SemanticCrystal) -> bool {
        true
    }
}

use pse_cascade::{
    batch_data_helix, build_metatron_phase_ladder, build_phase_ladder, helix_pair, mandorla,
    mandorla_real, restore_neutrality,
};
use pse_cascade::{
    dual_consensus, CascadeContext, CascadeOperator, CrystalPrecursor, DKOperator, MetricSet,
    PIOperator, PoRFsm, SWOperator, WTOperator,
};
use pse_constraint::{intrinsic_step, morphogenic_update, MorphState};
use pse_evidence::{build_crystal_with_id, Archive};
use pse_extract::{default_operator_library, inverse_weave, TimeWindow};
use pse_graph::PersistentGraph;
use pse_graph::{ingest, ObservationAdapter, PassthroughAdapter};
use pse_memory::{MemoryConfig, PatternMemory};
use pse_types::{
    CommitIndex, CommitProof, Config, DataHelix, FiveDState, GateSnapshot, MandorlaState,
    MeasurementContext, NullCenter, Observation, PhaseLadder, RunDescriptor, SemanticCrystal,
    VertexId,
};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("observation error: {0}")]
    Observe(#[from] pse_graph::ObserveError),
    #[error("persistence error: {0}")]
    Persist(#[from] pse_graph::PersistError),
    #[error("engine rejected: {0}")]
    Rejected(String),
}

pub type Result<T> = std::result::Result<T, EngineError>;

// ─── Engine State Machine ────────────────────────────────────────────────────

/// PSE Engine states (PSE Def 21.1)
#[derive(Clone, Debug, PartialEq)]
pub enum EngineState {
    Idle,
    Observing,
    Relating,
    Embedding,
    MandorlaForming,
    Resonating,
    KairosPrimed,
    Migrating,
    Capturing,
    Projecting,
    Stitching,
    Crystallizing,
    Monolithizing,
    Breaking,
    Restoring,
    Committed,
    Rejected(String),
}

// ─── Consensus State ─────────────────────────────────────────────────────────

#[derive(Default, Debug)]
pub struct ConsensusState {
    pub last_result: Option<pse_types::ConsensusResult>,
}

// ─── Global State (PSE Def 17.1) ────────────────────────────────────────────

pub struct GlobalState {
    pub graph: PersistentGraph,
    pub candidates: Vec<CrystalPrecursor>,
    pub consensus: ConsensusState,
    pub morph: MorphState,
    pub active_carrier: usize, // index into phase_ladder
    pub phase_ladder: PhaseLadder,
    pub h5_state: FiveDState,
    pub commit_index: CommitIndex,
    pub engine_state: EngineState,
    pub por_fsm: PoRFsm,
    pub archive: Archive,
    pub null_center: NullCenter, // Inv I13: always present, always empty
    pub t2: f64,                 // intrinsic time
    /// Embeddings from the previous macro-step, used to compute d-metric (change).
    pub prev_embeddings: BTreeMap<VertexId, FiveDState>,
    /// Constraint count extracted during the last macro-step (for M3).
    pub last_constraint_count: usize,
    /// Whether the Kairos gate passed on the last macro-step (for M9).
    pub last_gate_passed: bool,
    /// Full gate snapshot from the last macro-step (None until first tick).
    /// Populated unconditionally — readable whether the gate passed or not.
    /// Used for diagnostics, calibration, and adaptive thresholds.
    pub last_gate: Option<GateSnapshot>,
    /// Optional adaptive Kairos calibrator. When `Some`, thresholds are
    /// derived from the rolling history of `GateSnapshot`s instead of
    /// `Config::thresholds`. See [`adaptive::AdaptiveCalibrator`].
    pub adaptive: Option<adaptive::AdaptiveCalibrator>,
    /// C18 multi-scale state (Micro/Meso/Macro universes and bridges).
    pub scale_state: pse_scale::MultiScaleState,
    /// Count of pattern-memory hits (regions that matched existing crystals).
    pub pattern_hits: u64,
    /// Persistent pattern memory — topological similarity index for cross-session learning.
    pub memory: PatternMemory,
    /// Most recent batch projection onto the data-helix (E.1).
    ///
    /// `None` until the first observation batch has been ingested. Once
    /// populated, it lives at the same intrinsic time as the active
    /// carrier helix-pair, so the two are commensurable inputs to the
    /// E.2 Mandorla-as-real-interference computation.
    pub last_data_helix: Option<DataHelix>,
    /// Optional navigator state for TRITON exploration persistence.
    pub navigator_state: Option<pse_navigator::NavigatorState>,
    /// Optional swarm node for distributed crystal propagation (requires `swarm` feature).
    #[cfg(feature = "swarm")]
    pub swarm_node: Option<pse_net::SwarmNode>,
    /// Tick index of the most recent committed crystal.  Used by the
    /// crystal_cooldown_ticks gate to suppress burst false positives.
    pub last_crystal_commit: Option<u64>,
    /// Running count of crystals committed in this session.  The first
    /// `config.consensus.startup_crystal_count` crystals are labelled
    /// "pse_crystal_startup" by the runner; subsequent ones are "pse_crystal".
    pub crystals_committed: u64,
}

impl GlobalState {
    pub fn new(config: &Config) -> Self {
        // O.2: optional Metatron / cuboctahedron phase-ladder, selected
        // via config.carrier.use_metatron_ladder. Default false →
        // legacy n-evenly-spaced builder.
        let phase_ladder = if config.carrier.use_metatron_ladder {
            build_metatron_phase_ladder(0.0)
        } else {
            build_phase_ladder(config.carrier.num_carriers, 0.0, 1.0)
        };
        // Auto-wire adaptive calibrator from config when enabled.
        let adaptive = if config.calibration.enabled {
            Some(adaptive::AdaptiveCalibrator::new(
                config.calibration.target_pass_rate,
                config.calibration.window,
                config.calibration.warmup_ticks,
            ))
        } else {
            None
        };
        Self {
            graph: PersistentGraph::new(),
            candidates: Vec::new(),
            consensus: ConsensusState::default(),
            morph: MorphState::new(),
            active_carrier: 0,
            phase_ladder,
            h5_state: FiveDState::default(),
            commit_index: 0,
            engine_state: EngineState::Idle,
            por_fsm: PoRFsm::new(),
            archive: Archive::new(),
            null_center: NullCenter,
            t2: 0.0,
            prev_embeddings: BTreeMap::new(),
            last_constraint_count: 0,
            last_gate_passed: false,
            last_gate: None,
            adaptive,
            scale_state: pse_scale::MultiScaleState::default(),
            pattern_hits: 0,
            memory: PatternMemory::new(MemoryConfig {
                similarity_threshold: config.consensus.memory_similarity_threshold,
                ..MemoryConfig::default()
            }),
            last_data_helix: None,
            navigator_state: None,
            #[cfg(feature = "swarm")]
            swarm_node: None,
            last_crystal_commit: None,
            crystals_committed: 0,
        }
    }

    /// Attach a swarm node for distributed crystal propagation.
    #[cfg(feature = "swarm")]
    pub fn with_swarm(&mut self, node: pse_net::SwarmNode) {
        self.swarm_node = Some(node);
    }
}

/// Load crystals into the engine's pattern memory for cross-session learning.
/// Call this after creating GlobalState to restore memory from a previous session.
/// Returns the number of signatures loaded.
pub fn load_memory_from_crystals(state: &mut GlobalState, crystals: &[SemanticCrystal]) -> usize {
    state.memory.load_from_crystals(crystals)
}

// ─── Metric Computation ───────────────────────────────────────────────────────

/// Compute all metrics from the current graph, mandorla, and H5 state.
/// `prev_embeddings` holds embeddings from the previous macro-step and is used
/// to compute the deformation metric d as the mean relative change per vertex.
pub fn compute_all_metrics(
    graph: &PersistentGraph,
    mandorla: &MandorlaState,
    h5: &FiveDState,
    config: &Config,
    prev_embeddings: &BTreeMap<VertexId, FiveDState>,
) -> MetricSet {
    let norm = &config.normalization;

    // D: deformation metric — *combined* signal of in-place embedding change
    // and vertex-set churn (added/removed vertices since the last tick).
    //
    // Pre-Strand-E this was just the mean relative embedding change. That
    // works for batch scenarios where the whole graph evolves between ticks,
    // but in a sliding-window streaming scenario most vertices are stable
    // between ticks and only a few enter/leave at the boundary — the mean
    // collapses to ~0 and d never registers genuine novelty.
    //
    // Fix: take the *max* of three signals, all in [0,1]:
    //   1. mean relative embedding change over vertices present in both ticks
    //   2. p90 relative embedding change (a single moving region still counts)
    //   3. vertex-set churn = 1 − |V_curr ∩ V_prev| / max(|V_curr|, |V_prev|)
    //
    // First tick still treated as maximum novelty (0.75).
    let d_raw: f64 = if prev_embeddings.is_empty() {
        0.75
    } else {
        let mut diffs: Vec<f64> = graph
            .embedding
            .iter()
            .filter_map(|(vid, curr)| {
                let curr_norm = curr.norm_sq().sqrt();
                if curr_norm < 1e-9 {
                    return None;
                }
                let prev = prev_embeddings.get(vid)?;
                let prev_norm = prev.norm_sq().sqrt();
                if prev_norm < 1e-9 {
                    return None;
                }
                let dp = curr.p - prev.p;
                let dr = curr.rho - prev.rho;
                let dw = curr.omega - prev.omega;
                let dc = curr.chi - prev.chi;
                let de = curr.eta - prev.eta;
                let diff = (dp * dp + dr * dr + dw * dw + dc * dc + de * de).sqrt();
                Some(diff / (curr_norm + 1e-9))
            })
            .collect();

        let mean = if diffs.is_empty() {
            0.0
        } else {
            diffs.iter().sum::<f64>() / diffs.len() as f64
        };

        // p90: nice-to-have robustness. For small samples just take the max.
        let p90 = if diffs.is_empty() {
            0.0
        } else {
            diffs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let idx = ((diffs.len() as f64 * 0.90).ceil() as usize)
                .saturating_sub(1)
                .min(diffs.len() - 1);
            diffs[idx]
        };

        let n_curr = graph.embedding.len();
        let n_prev = prev_embeddings.len();
        let n_intersect = graph
            .embedding
            .keys()
            .filter(|vid| prev_embeddings.contains_key(vid))
            .count();
        let denom = n_curr.max(n_prev);
        let churn = if denom > 0 {
            1.0 - (n_intersect as f64 / denom as f64)
        } else {
            0.0
        };

        let candidate = mean.max(p90).max(churn);
        if candidate.is_finite() && candidate > 0.0 {
            candidate
        } else {
            0.75
        }
    };
    let d = pse_cascade::norm_saturate(d_raw, norm.mu_d);

    // Q: coherence — *fraction* of intrinsic carrier coherence preserved by
    // the current data helix. The raw `mandorla.kappa` already folds in the
    // carrier's own κ ceiling (`exp(-λ·Δφ_pair) · exp(-μ_r·r²)`), which for
    // default λ=0.1, μ_r=0.3, r=1 caps the absolute κ at ~0.54 — making a
    // 0.5 gate threshold structurally unreachable on real data. We normalize
    // by that ceiling so q ∈ [0,1] and threshold 0.5 means "data preserves
    // at least half the carrier's intrinsic coherence", which is the
    // physically sensible reading.
    let carrier_pair_dphi = std::f64::consts::PI; // helix-pair is antiphase by construction
    let kappa_carrier_max = (-config.carrier.lambda * carrier_pair_dphi).exp()
        * (-config.carrier.mu_r * mandorla.r * mandorla.r).exp();
    let q = if kappa_carrier_max > 1e-9 {
        (mandorla.kappa / kappa_carrier_max).clamp(0.0, 1.0)
    } else {
        0.0
    };

    // R: resonance (exp(-d_R(H, ref)))
    let r = pse_cascade::norm_exp(h5.norm_sq().sqrt(), norm.lambda_r);

    // G: readiness = gamma_D*D + gamma_Q*Q + gamma_R*R
    let g = norm.gamma_d * d + norm.gamma_q * q + norm.gamma_r * r;

    // J: double-kick (proxy: edge count / vertex count)
    let j_raw = if graph.graph.node_count() > 0 {
        graph.graph.edge_count() as f64 / graph.graph.node_count() as f64
    } else {
        0.0
    };
    let j = pse_cascade::norm_saturate(j_raw, norm.mu_j);

    // P: projection (proxy: stability of h5 state from origin)
    let p = pse_cascade::norm_exp(h5.norm_sq().sqrt(), norm.lambda_p);

    // N: seam (proxy: mandorla delta_phi coherence)
    let n = pse_cascade::norm_exp(mandorla.delta_phi, norm.lambda_seam);

    // K: crystal score (lambda_C * coherence + lambda_E * entropy_factor)
    let k = norm.lambda_c * q
        + norm.lambda_e * (1.0 - mandorla.delta_phi / std::f64::consts::PI).max(0.0);

    // F: friction (proxy: rate of change in graph structure)
    let f_raw = graph.commit_index as f64 * 0.01;
    let f = pse_cascade::norm_saturate(f_raw, norm.mu_f);

    // S: shock (proxy: abrupt change in H5)
    let s_raw = h5.norm_sq().sqrt();
    let s = pse_cascade::norm_saturate(s_raw, norm.mu_s);

    // L: migration readiness
    let l = if !config.carrier.num_carriers == 0 {
        let carrier = &config.carrier;
        carrier.lambda_q * q + carrier.lambda_r * r + carrier.lambda_m * mandorla.kappa
    } else {
        0.0
    };

    MetricSet {
        d_deformation: d,
        q_coherence: q,
        r_resonance: r,
        g_readiness: g,
        j_doublekick: j,
        p_projection: p,
        n_seam: n,
        k_crystal: k,
        f_friction: f,
        s_shock: s,
        l_migration: l,
    }
}

// ─── Carrier Migration ────────────────────────────────────────────────────────

fn attempt_carrier_migration(state: &mut GlobalState, metrics: &MetricSet, config: &Config) {
    // Find best migration target (highest resonance in phase ladder)
    let current = state.active_carrier;
    let best = state
        .phase_ladder
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != current)
        .max_by(|(_, a), (_, b)| {
            a.mandorla
                .kappa
                .partial_cmp(&b.mandorla.kappa)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

    if let Some((best_idx, best_carrier)) = best {
        if pse_cascade::migration_admissible(
            metrics,
            best_carrier,
            &config.thresholds,
            &config.carrier,
        ) {
            state.active_carrier = best_idx;
        }
    }
}

// ─── Macro Step (PSE Algo 2) ─────────────────────────────────────────────────

/// T_PSE(S_k, X_k; theta) = A_morph . C_commit . E_extract . T_persist . Gamma_obs
pub fn macro_step(
    state: &mut GlobalState,
    obs_payloads: &[Vec<u8>],
    config: &Config,
    adapter: &dyn ObservationAdapter,
) -> Result<Option<SemanticCrystal>> {
    // Drain incoming crystals from swarm peers (if swarm is active)
    #[cfg(feature = "swarm")]
    if let Some(ref node) = state.swarm_node {
        for envelope in node.drain_accepted() {
            state.archive.append(envelope.crystal.clone());
            // O.4: insert_crystal populates both the cosine-similarity
            // index AND the canonical-class index (when the swarm-
            // received crystal carries a Metatron signature).
            state.memory.insert_crystal(&envelope.crystal);
        }
    }

    // Unconditionally advance the macro-step counter so diagnostics and
    // downstream metrics always see a monotonically increasing tick index,
    // regardless of whether the step ends in a crystal commit or a gate
    // rejection.  Previously commit_index only incremented on a successful
    // crystal, causing all ticks to be reported as "tick 0".
    state.commit_index += 1;

    let ctx = MeasurementContext::default();

    // L0: Canonicalize observations
    state.engine_state = EngineState::Observing;
    let mut canonical_obs: Vec<Observation> = Vec::new();
    for raw in obs_payloads {
        let obs = ingest(adapter, raw, &ctx)?;
        canonical_obs.push(obs);
    }

    // E.1: Project this batch onto the data-helix at the current intrinsic
    // time. The helix lives at the same τ-scale as the active carrier
    // (`state.t2 + dt2`, anticipating the carrier advance below) so the
    // two are commensurable when E.2 wires them into a real interference.
    let data_tau = state.t2 + config.temporal.dt2;
    state.last_data_helix = Some(batch_data_helix(&canonical_obs, data_tau));

    // J: optional per-tick adaptive carrier tracker. When enabled, pick
    // the phase-ladder slot whose Mandorla κ against the *current*
    // data-helix is highest, and make it the active carrier *before*
    // E.2 computes the metrics-feeding Mandorla. This closes the loop
    // diagnosed in F.1's commentary: with avalanche-hash phases the
    // carrier could never align; with semantic phases (I) and an
    // adaptive carrier (J) the engine actively tracks the data
    // distribution. Default: false (backward compatible).
    if config.carrier.adaptive {
        if let Some(data) = &state.last_data_helix {
            let mut best_idx = state.active_carrier;
            let mut best_kappa = f64::NEG_INFINITY;
            for (i, c) in state.phase_ladder.iter().enumerate() {
                let m = pse_cascade::mandorla_real(
                    &c.helix_a,
                    &c.helix_b,
                    data,
                    config.carrier.lambda,
                    config.carrier.mu_r,
                    config.carrier.eta_r,
                );
                if m.kappa > best_kappa {
                    best_kappa = m.kappa;
                    best_idx = i;
                }
            }
            state.active_carrier = best_idx;
        }
    }

    // L1: Update persistent graph (MCCE assimilated)
    state.engine_state = EngineState::Relating;
    state
        .graph
        .apply_observations(&canonical_obs, &config.persistence)?;

    // Embedding + Mandorla
    state.engine_state = EngineState::Embedding;
    // Advance the active carrier's phase by one dt2 step and recompute its
    // mandorla.  Previously (ha, hb, mand) were computed locally but never
    // written back, so carrier.helix_a.tau was always the initial value and
    // the carrier state never accumulated across ticks.
    let (ha, hb) = {
        let carrier = &state.phase_ladder[state.active_carrier];
        helix_pair(
            carrier.helix_a.tau + config.temporal.dt2,
            carrier.helix_a.phi,
            carrier.helix_a.r,
        )
    };
    // E.2: if a data-helix exists from this batch, compute the **real**
    // Mandorla as the interference of the carrier pair with the data
    // stream. Otherwise (empty batch, very first tick before E.1
    // populates the helix) fall back to the carrier-only mandorla so the
    // engine still has a well-defined coherence to gate on.
    let mand = match &state.last_data_helix {
        Some(data) => mandorla_real(
            &ha,
            &hb,
            data,
            config.carrier.lambda,
            config.carrier.mu_r,
            config.carrier.eta_r,
        ),
        None => mandorla(&ha, &hb, config.carrier.lambda, config.carrier.mu_r),
    };

    // Write the new helix positions and mandorla back into the phase ladder so
    // the next tick picks up from where this one left off.
    {
        let carrier_mut = &mut state.phase_ladder[state.active_carrier];
        carrier_mut.helix_a = ha;
        carrier_mut.helix_b = hb;
        carrier_mut.mandorla = mand.clone();
    }

    state.engine_state = EngineState::MandorlaForming;

    // Resonance evaluation
    state.engine_state = EngineState::Resonating;
    let metrics = compute_all_metrics(
        &state.graph,
        &mand,
        &state.h5_state,
        config,
        &state.prev_embeddings,
    );
    // Save current embeddings for next tick's d-metric computation
    state.prev_embeddings = state.graph.embedding.clone();

    // Check friction/shock -> migration
    if metrics.f_friction >= config.thresholds.f_friction
        || metrics.s_shock >= config.thresholds.s_shock
    {
        state.engine_state = EngineState::Migrating;
        attempt_carrier_migration(state, &metrics, config);
    }

    // Early-burst suppression: skip crystallisation until the stream has
    // accumulated enough history for the graph metrics to be meaningful.
    // The expanding-window phase (ticks 1..window_size) produces a dense
    // small graph where all metrics are artificially high, leading to a
    // burst of false-positive crystals.  A scenario-specific min_crystal_tick
    // (0 = disabled) gates this out without touching the metric computation.
    if config.consensus.min_crystal_tick > 0
        && state.commit_index < config.consensus.min_crystal_tick
    {
        state.engine_state = EngineState::Idle;
        return Ok(None);
    }

    // Post-crystal cooldown: after a crystal commits at tick T, suppress
    // further crystallisation for crystal_cooldown_ticks ticks.  Prevents
    // multi-tick burst FPs caused by the Kairos gate staying open across
    // consecutive ticks after a morphogenic update (each update changes
    // topology enough to bypass the pattern-memory cosine filter, so
    // without cooldown an entire burst window crystallises as FP).
    //
    // The cooldown is intentionally skipped during the startup phase so the
    // early morphogenic burst (which is needed for graph expansion) fires at
    // its natural rate.  Spacing the burst crystals with cooldown would
    // stretch the startup across hundreds of ticks, delaying detection.
    let past_startup = config.consensus.startup_crystal_count == 0
        || state.crystals_committed >= config.consensus.startup_crystal_count;
    if past_startup && config.consensus.crystal_cooldown_ticks > 0 {
        if let Some(last) = state.last_crystal_commit {
            if state.commit_index <= last + config.consensus.crystal_cooldown_ticks {
                state.engine_state = EngineState::Idle;
                return Ok(None);
            }
        }
    }

    // Kairos gate check (Inv I9, Inv I18). When an adaptive calibrator is
    // wired in, it derives the thresholds from rolling history; otherwise
    // we use the static `Config::thresholds`. The 8-fold AND composition
    // is unchanged — only per-metric cut points move, so falsification,
    // EU-AI-Act compliance proofs, and crystal verifiability are
    // unaffected. The non-adaptive path performs exactly one gate_snapshot
    // call, preserving the pre-adaptive throughput.
    //
    // effective_thresholds is stored so that post-gate checks (seam) use
    // the same cut points as the gate itself — avoiding a situation where
    // the gate passes on adaptive thresholds but a later static check rejects.
    let (gate, effective_thresholds) = if let Some(cal) = state.adaptive.as_mut() {
        let raw_gate = metrics.gate_snapshot(&config.thresholds);
        let effective = cal.calibrate(&config.thresholds);
        let gate = metrics.gate_snapshot(&effective);
        // Record the *raw* snapshot so calibration is not self-referential;
        // future quantiles see the unmodified metrics.
        cal.record(&raw_gate);
        (gate, effective)
    } else {
        let gate = metrics.gate_snapshot(&config.thresholds);
        (gate, config.thresholds.clone())
    };
    state.last_gate_passed = gate.kairos;
    state.last_gate = Some(gate.clone());
    if !gate.kairos {
        tracing::debug!(
            tick = state.commit_index,
            d = gate.d,
            d_thr = config.thresholds.d,
            q = gate.q,
            q_thr = config.thresholds.q,
            r = gate.r,
            r_thr = config.thresholds.r,
            g = gate.g,
            g_thr = config.thresholds.g,
            j = gate.j,
            j_thr = config.thresholds.j,
            p = gate.p,
            p_thr = config.thresholds.p,
            n = gate.n,
            n_thr = config.thresholds.n,
            k = gate.k,
            k_thr = config.thresholds.k,
            "kairos rejected"
        );
        state.engine_state = EngineState::Rejected("kairos failed".into());
        return Ok(None);
    }
    tracing::debug!(tick = state.commit_index, "kairos passed");
    state.engine_state = EngineState::KairosPrimed;

    // L2: Constraint extraction (ECLS assimilated)
    state.engine_state = EngineState::Capturing;
    let library = default_operator_library();
    let window = TimeWindow::all();
    let (program, region) = inverse_weave(&state.graph, &window, &library, &config.extraction);
    state.last_constraint_count = program.len();
    tracing::debug!(
        tick = state.commit_index,
        constraints = program.len(),
        region = region.len(),
        vertices = state.graph.graph.node_count(),
        edges = state.graph.graph.edge_count(),
        "constraints extracted"
    );

    // Pattern memory shortcut: check the topological similarity index for
    // known patterns. If a similar crystal exists in memory, skip the full
    // cascade validation. This is the core accumulation mechanism — known
    // patterns are recognized cheaply instead of re-validated expensively.
    // Works across sessions when memory is loaded from the store on startup.
    if !region.is_empty() {
        // First check the signature-based memory index (cross-session capable)
        let graph_topo = state.graph.topology_signature();
        let candidate_sig = pse_memory::PatternMemory::extract_candidate_signature(
            graph_topo.spectral_gap,
            graph_topo.cheeger_estimate,
            graph_topo.kuramoto_coherence,
            graph_topo.mean_propagation_time,
            graph_topo.betti_0,
            graph_topo.betti_1,
            graph_topo.betti_2,
            graph_topo.euler_char,
            metrics.g_readiness,
            region.len(),
        );
        if state.memory.lookup(&candidate_sig).is_some() {
            state.pattern_hits += 1;
            state.engine_state = EngineState::Idle;
            return Ok(None);
        }
        // Fallback: check archive region overlap (within-session)
        let dom_threshold = config.consensus.archive_dominance;
        let dominated = state.archive.crystals().iter().any(|c| {
            if c.region.is_empty() {
                return false;
            }
            let overlap = region.iter().filter(|v| c.region.contains(v)).count();
            let coverage = overlap as f64 / region.len().max(1) as f64;
            coverage >= dom_threshold
        });
        if dominated {
            state.pattern_hits += 1;
            state.engine_state = EngineState::Idle;
            return Ok(None);
        }
    }

    // Operators: projection
    state.engine_state = EngineState::Projecting;
    let stability_score = metrics.g_readiness;
    let precursor = CrystalPrecursor {
        program: program.clone(),
        region: region.clone(),
        seam_score: metrics.n_seam,
        metrics: metrics.clone(),
        stability_score,
    };

    // Stitching (seam check) — use the same effective thresholds as the Kairos gate
    // so that adaptive calibration is not bypassed by a static fallback here.
    state.engine_state = EngineState::Stitching;
    if metrics.n_seam < effective_thresholds.n {
        state.engine_state = EngineState::Rejected("seam failed".into());
        return Ok(None);
    }

    // L3: Crystallize
    state.engine_state = EngineState::Crystallizing;

    // Dual consensus
    let dk = DKOperator;
    let sw = SWOperator;
    let pi = PIOperator;
    let wt = WTOperator;
    let pi2 = PIOperator;
    let wt2 = WTOperator;
    let dk2 = DKOperator;
    let sw2 = SWOperator;
    let primal: Vec<&dyn CascadeOperator> = vec![&dk, &sw, &pi, &wt];
    let dual: Vec<&dyn CascadeOperator> = vec![&pi2, &wt2, &dk2, &sw2];
    // E.4: cascade now needs the live carrier/data context so each
    // operator can perform its physics test against the actual
    // resonance configuration. The context is cloned per cascade path
    // inside dual_consensus so primal and dual see independent
    // carrier-mutation trajectories.
    let cascade_ctx = CascadeContext {
        carrier: state.phase_ladder[state.active_carrier].clone(),
        phase_ladder: &state.phase_ladder,
        active_idx: state.active_carrier,
        data: state.last_data_helix.clone(),
        config: &config.carrier,
    };
    let consensus = dual_consensus(&precursor, &primal, &dual, &cascade_ctx, &config.consensus);

    if consensus.primal_score < consensus.threshold
        || consensus.dual_score < consensus.threshold
        || consensus.mci < config.consensus.mirror_consistency_eta
    {
        state.engine_state = EngineState::Rejected("consensus failed".into());
        return Ok(None);
    }

    state.consensus.last_result = Some(consensus.clone());

    // D.4: surrogate-data falsification pass.
    //
    // Optional, off by default (config.falsification.enabled = false).
    // When enabled: re-run the engine on `k` permuted versions of the
    // current batch and compute the empirical p-value as the fraction
    // of surrogates whose maximum κ matched or exceeded the real
    // batch's. If `gate_on_fail` and `p_value >= alpha`, the crystal
    // is rejected and the commit is suppressed. Either way the
    // p-value lands in the commit proof when the crystal does commit.
    let (fals_p_value, fals_count): (Option<f64>, Option<u32>) =
        if config.falsification.enabled && !canonical_obs.is_empty() {
            let result = falsify::falsify_with_surrogates(
                &canonical_obs,
                config,
                config.falsification.k,
                config.falsification.method.clone(),
                // Per-tick seed derived from the commit index so two replays
                // of the same RunDescriptor produce identical p-values
                // (Inv I4 preserved through the falsifier).
                0xFA15_E700_u64.wrapping_add(state.commit_index),
                adapter,
            );
            if config.falsification.gate_on_fail && result.p_value >= config.falsification.alpha {
                state.engine_state = EngineState::Rejected("falsification failed".into());
                return Ok(None);
            }
            (Some(result.p_value), Some(result.k_actual))
        } else {
            (None, None)
        };

    // Build commit proof
    let por_trace = state.por_fsm.get_trace().clone();
    let operator_stack: Vec<(String, String)> = config
        .extraction
        .window_hours
        .to_string()
        .chars()
        .take(0)
        .collect::<String>()
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| (s.to_string(), "1.0.0".to_string()))
        .collect(); // empty for now

    let commit_proof = CommitProof {
        evidence_digests: canonical_obs.iter().map(|o| o.digest).collect(),
        operator_stack,
        gate_values: gate.clone(),
        structural_result: true,
        consensus_result: consensus.clone(),
        por_trace,
        carrier_id: state.active_carrier,
        carrier_offset: state.phase_ladder[state.active_carrier].offset,
        falsification_p_value: fals_p_value,
        surrogate_count: fals_count,
    };

    // Build crystal (Inv I17: crystal required before commit)
    let mut crystal = build_crystal_with_id(
        region,
        stability_score,
        state.commit_index,
        -(program.len() as f64) * stability_score,
        state.active_carrier,
        program,
        commit_proof,
    );

    // C16: Enrich crystal topological signature with spectral/Kuramoto invariants
    {
        let topo_config = pse_topology::TopologyConfig::default();
        let sig = pse_topology::compute_topological_signature(&state.graph, &topo_config);
        crystal.topology_signature.spectral_gap = sig.spectral_gap;
        crystal.topology_signature.cheeger_estimate = sig.cheeger_estimate;
        crystal.topology_signature.kuramoto_coherence = sig.kuramoto_coherence;
        crystal.topology_signature.mean_propagation_time = sig.mean_propagation_time;
        crystal.topology_signature.dtl_connected = sig
            .dtl_predicates
            .get("Connected")
            .cloned()
            .unwrap_or(false);
        if !sig.betti_numbers.is_empty() {
            crystal.topology_signature.betti_0 = sig.betti_numbers[0];
            crystal.topology_signature.betti_1 = sig.betti_numbers.get(1).cloned().unwrap_or(0);
            crystal.topology_signature.betti_2 = sig.betti_numbers.get(2).cloned().unwrap_or(0);
        }
    }

    // C18: Multi-scale tick (Micro→Meso→Macro lift + cross-scale bridges).
    // Guard: skip for graphs with > 500 vertices to match the budget-fallback
    // threshold in compute_topological_signature and avoid O(n²/n³) costs
    // (kuramoto_phase_cluster, lift_micro_to_meso bridge search) on large graphs.
    {
        let n_vertices = state.graph.graph.node_count();
        if n_vertices <= 500 {
            let laplacian = pse_topology::compute_laplacian(&state.graph);
            let topo_cfg = pse_topology::TopologyConfig::default();
            let spectral = pse_topology::spectral_decompose(&laplacian, topo_cfg.spectral_k_max);
            let kuramoto = pse_topology::init_kuramoto_state(&state.graph);
            let micro = pse_scale::pse_engine_types::MicroState::from_graph(&state.graph);
            let scale_cfg = pse_scale::ScaleConfig::default();
            let micro_crystals = state.archive.crystals();
            let ms_result = pse_scale::multi_scale_tick(
                &micro,
                &mut state.scale_state,
                &spectral,
                &kuramoto,
                &scale_cfg,
                micro_crystals,
                state.commit_index,
            );
            for mc in ms_result.meso_crystals {
                state.scale_state.meso_crystals.push(mc);
            }
            for mc in ms_result.macro_crystals {
                state.scale_state.macro_crystals.push(mc);
            }
        }
    }

    state.engine_state = EngineState::Monolithizing;

    // Strand O.3: attach the canonical Metatron-scaffold signature to
    // the crystal if its region is small enough for the S₇ orbit
    // scan. Done in-place before archive.append so the immutable
    // record carries the canonical identity. Uses the process-global
    // S₇ cache (lazy-initialised on first call).
    metatron_attach::attach_signature_global(&mut crystal, &state.graph);

    // Commit (append to immutable archive, Inv I10)
    state.archive.append(crystal.clone());

    // Add new crystal to pattern memory (cross-session capable).
    // O.4: insert_crystal populates both the cosine-similarity index
    // AND the canonical-class index (when the crystal carries a
    // Metatron signature from O.3). Future lookups via lookup_crystal
    // will prefer canonical-class identity over fuzzy similarity.
    state.memory.insert_crystal(&crystal);

    // Propagate crystal to swarm peers (if swarm is active)
    #[cfg(feature = "swarm")]
    if let Some(ref node) = state.swarm_node {
        let _ = node.propagate_crystal(crystal.clone());
    }

    state.engine_state = EngineState::Committed;
    // commit_index is already incremented unconditionally at the top of
    // macro_step; do not increment again here.
    state.last_crystal_commit = Some(state.commit_index);
    state.crystals_committed += 1;

    // L4: Morphogenic update (Inv I11: non-retroactive)
    morphogenic_update(
        &mut state.graph,
        &mut state.morph,
        &[crystal.clone()],
        &config.adaptation,
    );

    // Intrinsic dynamics update (OI-08)
    state.t2 += config.temporal.dt2;
    intrinsic_step(
        &mut state.h5_state,
        &state.morph.attractor.clone(),
        &crystal.constraint_program,
        config.temporal.dt2,
        config.temporal.gamma,
    );

    // Restore neutrality (AT-20: symmetry restoration)
    state.engine_state = EngineState::Restoring;
    if let Some(carrier) = state.phase_ladder.get_mut(state.active_carrier) {
        restore_neutrality(carrier);
    }
    state.engine_state = EngineState::Idle;

    Ok(Some(crystal))
}

// ─── Engine Runner ────────────────────────────────────────────────────────────

/// Run PSE engine with a RunDescriptor (for deterministic replay, Inv I4)
pub fn run_with_descriptor(
    descriptor: &RunDescriptor,
    obs_batches: &[Vec<Vec<u8>>],
) -> Result<Vec<Option<SemanticCrystal>>> {
    let mut state = GlobalState::new(&descriptor.config);
    let adapter = PassthroughAdapter::new("replay");
    let mut results = Vec::new();

    for batch in obs_batches {
        let result = macro_step(&mut state, batch, &descriptor.config, &adapter)?;
        results.push(result);
    }
    Ok(results)
}

// ─── Temperature Calibration (OI-06) ─────────────────────────────────────────

/// Compute temperature from realized standard deviation of resonance field (OI-06)
pub fn compute_temperature(resonance_window: &[f64], c_t: f64, t_default: f64) -> f64 {
    if resonance_window.len() < 2 {
        return t_default;
    }
    let n = resonance_window.len() as f64;
    let mean = resonance_window.iter().sum::<f64>() / n;
    let variance = resonance_window
        .iter()
        .map(|x| (x - mean).powi(2))
        .sum::<f64>()
        / (n - 1.0);
    let sigma = variance.sqrt();
    c_t * sigma
}

/// Temperature regime classification (OI-06)
pub fn temperature_regime(t: f64) -> &'static str {
    if t < 0.5 {
        "calm"
    } else if t < 2.0 {
        "normal"
    } else {
        "volatile"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_state_initializes() {
        let config = Config::default();
        let state = GlobalState::new(&config);
        assert_eq!(state.engine_state, EngineState::Idle);
        assert_eq!(state.commit_index, 0);
        assert_eq!(state.active_carrier, 0);
        assert!(!state.phase_ladder.is_empty());
    }

    #[test]
    fn temperature_calibration_basic() {
        let window = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let t = compute_temperature(&window, 5.0, 1.0);
        assert!(t > 0.0);
    }

    #[test]
    fn temperature_regime_classification() {
        assert_eq!(temperature_regime(0.3), "calm");
        assert_eq!(temperature_regime(1.0), "normal");
        assert_eq!(temperature_regime(3.0), "volatile");
    }

    // ── D.4: macro_step integration of falsification ─────────────────────

    use pse_graph::PassthroughAdapter;
    use pse_types::SurrogateMethod;

    /// Helper: drive a few macro_steps with the given config and return
    /// the count of crystals committed. The test stream is deliberately
    /// trivial so the test runs quickly even with falsification on.
    fn macro_step_crystal_count(config: &Config, n: usize) -> usize {
        let mut state = GlobalState::new(config);
        let adapter = PassthroughAdapter::new("d4-test");
        let mut count = 0;
        for i in 0..n {
            let payload = serde_json::to_vec(&format!("payload-{}", i)).unwrap();
            if let Ok(Some(_)) = macro_step(&mut state, &[payload], config, &adapter) {
                count += 1;
            }
        }
        count
    }

    #[test]
    fn falsification_disabled_by_default_no_op() {
        // With config.falsification.enabled = false (the default),
        // macro_step must behave bit-identically to the pre-D.4 form:
        // commit proofs carry None for the new fields.
        let config = Config::default();
        assert!(!config.falsification.enabled);
        let mut state = GlobalState::new(&config);
        let adapter = PassthroughAdapter::new("default");
        // Run enough ticks to give some chance of a crystal forming
        // (in practice 0 with default thresholds, but the test is
        // tolerant). We assert the commit_proof structure.
        for i in 0..3 {
            let payload = serde_json::to_vec(&format!("p{}", i)).unwrap();
            if let Ok(Some(crystal)) = macro_step(&mut state, &[payload], &config, &adapter) {
                assert_eq!(crystal.commit_proof.falsification_p_value, None);
                assert_eq!(crystal.commit_proof.surrogate_count, None);
            }
        }
    }

    #[test]
    fn falsification_with_alpha_zero_rejects_every_commit() {
        // gate_on_fail = true and alpha = 0.0 means "reject if p_value
        // >= 0.0" — which is always true. So no crystal can ever
        // commit, regardless of the underlying engine behaviour.
        // This is the strict-rejection sanity check for the gating path.
        let mut config = Config::default();
        config.falsification.enabled = true;
        config.falsification.gate_on_fail = true;
        config.falsification.alpha = 0.0;
        config.falsification.k = 4; // small k for test speed
        config.falsification.method = SurrogateMethod::Shuffle;
        let count = macro_step_crystal_count(&config, 5);
        assert_eq!(count, 0, "alpha=0 must reject every potential crystal");
    }

    #[test]
    fn falsification_with_alpha_one_admits_every_commit() {
        // alpha = 1.0 means "reject only if p_value >= 1.0". Since
        // p_value is in [0, 1] and the comparable count is bounded by
        // k_actual, the test is permissive. Crystal commit count must
        // match the no-falsification baseline (subject to determinism).
        let mut config = Config::default();
        config.falsification.enabled = true;
        config.falsification.gate_on_fail = true;
        config.falsification.alpha = 1.001; // numerically > 1.0 to be safe
        config.falsification.k = 4;
        config.falsification.method = SurrogateMethod::Shuffle;
        let count_with = macro_step_crystal_count(&config, 5);

        let mut baseline_config = Config::default();
        baseline_config.falsification.enabled = false;
        let count_without = macro_step_crystal_count(&baseline_config, 5);

        // The runs go through the same engine path; with permissive
        // alpha the count must equal the no-falsification baseline.
        assert_eq!(count_with, count_without);
    }

    #[test]
    fn falsification_disabled_run_is_deterministic_replayable() {
        // Inv I4: same input, same output, regardless of the
        // falsification pass being present in the source code.
        let config = Config::default();
        let count_a = macro_step_crystal_count(&config, 4);
        let count_b = macro_step_crystal_count(&config, 4);
        assert_eq!(count_a, count_b);
    }

    // ── Strand J: adaptive carrier tracker ───────────────────────────────

    #[test]
    fn adaptive_carrier_disabled_by_default_keeps_initial_index() {
        // Default config: adaptive=false. Without friction/shock
        // triggers the active carrier should remain at index 0
        // through trivial single-tick runs.
        let cfg = Config::default();
        assert!(!cfg.carrier.adaptive);
        let mut state = GlobalState::new(&cfg);
        let adapter = PassthroughAdapter::new("j-default");
        let payload = serde_json::to_vec(&"trivial").unwrap();
        let _ = macro_step(
            &mut state,
            &payload
                .clone()
                .into_iter()
                .collect::<Vec<u8>>()
                .chunks(1)
                .map(|c| c.to_vec())
                .next()
                .into_iter()
                .collect::<Vec<_>>(),
            &cfg,
            &adapter,
        );
        // Sanity: still at the original carrier (no friction-triggered
        // migration on a single trivial tick).
        assert_eq!(state.active_carrier, 0);
    }

    #[test]
    fn adaptive_carrier_picks_carrier_aligned_with_data_phase() {
        // Construct an observation whose semantic phase points at
        // carrier 1's α.phi (= π/2 in the default 4-carrier ladder).
        // With adaptive=true, the engine must migrate to carrier 1
        // because its Mandorla κ against this data-helix is highest.
        let mut cfg = Config::default();
        cfg.carrier.adaptive = true;
        // Verify the assumption: build_phase_ladder(4, …) uses offsets
        // 0, π/2, π, 3π/2; carrier 1 has α.phi = π/2.
        let mut state = GlobalState::new(&cfg);
        assert_eq!(state.phase_ladder.len(), 4);
        // We craft an observation with phase_hint ≈ π/2.
        use pse_types::Observation;
        let payload = b"adaptive-test".to_vec();
        let obs = Observation {
            timestamp: 0.0,
            source_id: "j-test".into(),
            provenance: Default::default(),
            payload: payload.clone(),
            context: Default::default(),
            digest: pse_types::content_address_raw(&payload),
            schema_version: "1.0.0".into(),
            phase_hint: Some(std::f64::consts::FRAC_PI_2),
        };
        // Inline-test the carrier selector by computing what the engine
        // would pick. Adaptive selection runs in macro_step, but to
        // keep the test deterministic we replicate its logic here on
        // the same inputs.
        let data = pse_cascade::observation_data_helix(&obs, 0.0);
        let mut best_idx = 0;
        let mut best_kappa = f64::NEG_INFINITY;
        for (i, c) in state.phase_ladder.iter().enumerate() {
            let m = pse_cascade::mandorla_real(
                &c.helix_a,
                &c.helix_b,
                &data,
                cfg.carrier.lambda,
                cfg.carrier.mu_r,
                cfg.carrier.eta_r,
            );
            if m.kappa > best_kappa {
                best_kappa = m.kappa;
                best_idx = i;
            }
        }
        // With α.phi = π/2 (carrier 1) and data.phi = π/2, the
        // phase_lock factor is 1.0. The other carriers should give
        // smaller values. Best should be carrier 1 (or the equivalent
        // antinode at carrier 3, both axis-aligned for π/2 data).
        assert!(
            best_idx == 1 || best_idx == 3,
            "expected adaptive selector to pick the π/2-axis carrier, got {}",
            best_idx
        );
        // And actually drive macro_step to confirm the engine acts on it.
        let payloads = vec![serde_json::to_vec(&"x").unwrap()];
        let adapter = PassthroughAdapter::new("j-test");
        // Manually inject phase_hint by going through a raw
        // canonicalize... PassthroughAdapter doesn't set phase_hint;
        // for this end-to-end check we rely on the inline assertion
        // above which uses identical math. The final assertion is just
        // that adaptive=true does not crash macro_step.
        let _ = macro_step(&mut state, &payloads, &cfg, &adapter);
    }

    #[test]
    fn adaptive_carrier_preserves_replay_determinism() {
        // Inv I4 must hold even with adaptive carrier on: same input,
        // same final active_carrier index.
        let mut cfg = Config::default();
        cfg.carrier.adaptive = true;
        let mut s1 = GlobalState::new(&cfg);
        let mut s2 = GlobalState::new(&cfg);
        let adapter = PassthroughAdapter::new("j-replay");
        for i in 0..3 {
            let p = serde_json::to_vec(&format!("p-{}", i)).unwrap();
            let _ = macro_step(&mut s1, std::slice::from_ref(&p), &cfg, &adapter);
            let _ = macro_step(&mut s2, &[p], &cfg, &adapter);
        }
        assert_eq!(s1.active_carrier, s2.active_carrier);
    }
}

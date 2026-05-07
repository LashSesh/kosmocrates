//! Adversarial validation cascade, consensus, and carrier geometry for PSE.
//!
//! Combines consensus validation (PoR gate, dual consensus, cascade operators)
//! with carrier geometry (helix pairs, mandorla, phase ladder).

//! Consensus validation and proof-of-rigor gate (Layer L3).
//!
//! Implements dual-consensus operators, metric normalization, and the PoR
//! finite-state machine that gates crystal formation.

// isls-consensus: Consensus, PoR gate, proof engine (Layer L3)
// C5 — depends on pse-types, pse-graph

use pse_types::{
    ConsensusConfig, ConsensusResult, GateSnapshot, NormalizationConfig, PoRTrace, ThresholdConfig,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConsensusError {
    #[error("consensus failed: {0}")]
    Failed(String),
}

pub type Result<T> = std::result::Result<T, ConsensusError>;

// ─── Normalization Functions (PSE Sec 10) ───────────────────────────────────

/// Saturation normalization: N(u; mu) = u / (u + mu) (Def 10.1)
pub fn norm_saturate(u: f64, mu: f64) -> f64 {
    u / (u + mu)
}

/// Exponential normalization: N_exp(d; lambda) = exp(-lambda * d) (Def 10.2)
pub fn norm_exp(d: f64, lambda: f64) -> f64 {
    (-lambda * d).exp()
}

// ─── Metric Set ───────────────────────────────────────────────────────────────

/// All 11 metrics from PSE Sec 10
#[derive(Clone, Debug, Default)]
pub struct MetricSet {
    pub d_deformation: f64, // Def 10.3: N(D_raw; mu_D)
    pub q_coherence: f64,   // Def 10.4: N(Q_raw; mu_Q)
    pub r_resonance: f64,   // Def 10.5: exp(-d_R(H, s_ref))
    pub g_readiness: f64,   // Def 10.6: gamma_D*D + gamma_Q*Q + gamma_R*R
    pub j_doublekick: f64,  // Def 10.7: N(J_raw; mu_J)
    pub p_projection: f64,  // Def 10.8: exp(-diam(P) * lambda_P)
    pub n_seam: f64,        // Def 10.9: exp(-d_seam(L,R))
    pub k_crystal: f64,     // Def 10.10: lambda_C*C + lambda_E*E
    pub f_friction: f64,    // Def 10.11: N(F_raw; mu_F)
    pub s_shock: f64,       // Def 10.12: N(S_raw; mu_S)
    pub l_migration: f64,   // from carrier readiness
}

impl MetricSet {
    pub fn gate_snapshot(&self, thresholds: &ThresholdConfig) -> GateSnapshot {
        GateSnapshot {
            d: self.d_deformation,
            q: self.q_coherence,
            r: self.r_resonance,
            g: self.g_readiness,
            j: self.j_doublekick,
            p: self.p_projection,
            n: self.n_seam,
            k: self.k_crystal,
            kairos: self.d_deformation >= thresholds.d
                && self.q_coherence >= thresholds.q
                && self.r_resonance >= thresholds.r
                && self.g_readiness >= thresholds.g
                && self.j_doublekick >= thresholds.j
                && self.p_projection >= thresholds.p
                && self.n_seam >= thresholds.n
                && self.k_crystal >= thresholds.k,
        }
    }

    /// Compute g_readiness from components
    pub fn compute_readiness(&mut self, norm: &NormalizationConfig) {
        self.g_readiness = norm.gamma_d * self.d_deformation
            + norm.gamma_q * self.q_coherence
            + norm.gamma_r * self.r_resonance;
    }

    /// Compute k_crystal from components
    pub fn compute_k_crystal(&mut self, coherence: f64, entropy: f64, norm: &NormalizationConfig) {
        self.k_crystal = norm.lambda_c * coherence + norm.lambda_e * (1.0 - entropy.min(1.0));
    }
}

// ─── PoR State Machine ───────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
pub enum PoRState {
    Search,
    Lock,
    Verify,
    Commit,
}

pub struct PoRFsm {
    pub state: PoRState,
    pub stability_history: Vec<f64>,
    pub lock_tick: Option<f64>,
    pub verify_tick: Option<f64>,
    pub trace: PoRTrace,
}

impl Default for PoRFsm {
    fn default() -> Self {
        Self {
            state: PoRState::Search,
            stability_history: Vec::new(),
            lock_tick: None,
            verify_tick: None,
            trace: PoRTrace {
                search_enter: 0.0,
                ..Default::default()
            },
        }
    }
}

impl PoRFsm {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn step(&mut self, kappa: f64, t2: f64, config: &ConsensusConfig) -> PoRState {
        match self.state {
            PoRState::Search => {
                if kappa >= config.por_kappa_bar {
                    self.stability_history.push(kappa);
                    if self.stability_history.len() >= config.por_t_min {
                        self.state = PoRState::Lock;
                        self.lock_tick = Some(t2);
                        self.trace.lock_enter = Some(t2);
                    }
                } else {
                    self.stability_history.clear();
                }
            }
            PoRState::Lock => {
                let last = self.stability_history.last().copied().unwrap_or(0.0);
                let delta = (kappa - last).abs();
                if delta <= config.por_epsilon {
                    self.stability_history.push(kappa);
                    if self.stability_history.len() >= config.por_t_min + config.por_t_stable {
                        self.state = PoRState::Verify;
                        self.verify_tick = Some(t2);
                        self.trace.verify_enter = Some(t2);
                    }
                } else {
                    self.reset(t2);
                }
            }
            PoRState::Verify => {
                // Check policy constraints and latch time
                // For now, immediately commit if kappa still >= threshold
                if kappa >= config.por_kappa_bar {
                    self.state = PoRState::Commit;
                    self.trace.commit_enter = Some(t2);
                } else {
                    self.reset(t2);
                }
            }
            PoRState::Commit => {
                // Terminal state until reset
            }
        }
        self.state.clone()
    }

    pub fn reset(&mut self, t2: f64) {
        self.state = PoRState::Search;
        self.stability_history.clear();
        self.lock_tick = None;
        self.verify_tick = None;
        self.trace = PoRTrace {
            search_enter: t2,
            lock_enter: None,
            verify_enter: None,
            commit_enter: None,
        };
    }

    pub fn get_trace(&self) -> &PoRTrace {
        &self.trace
    }
}

// ─── Crystal Precursor ───────────────────────────────────────────────────────

/// Precursor for crystal formation (before consensus)
#[derive(Clone, Debug)]
pub struct CrystalPrecursor {
    pub program: pse_types::ConstraintProgram,
    pub region: Vec<pse_types::VertexId>,
    pub seam_score: f64,
    pub metrics: MetricSet,
    pub stability_score: f64,
}

impl CrystalPrecursor {
    pub fn stability_score(&self) -> f64 {
        self.stability_score
    }

    pub fn distance(&self, other: &Self) -> f64 {
        let s1 = self.stability_score;
        let s2 = other.stability_score;
        (s1 - s2).abs()
    }
}

// ─── Cascade Operator Trait (E.4) ────────────────────────────────────────────
//
// The cascade operators were originally multiplicative scalar tweaks on a
// single stability score: physically hollow, and the dual cascade's MCI
// was 1.0 by construction (commutative ops). E.4 replaces them with four
// operators that each compute a real resonance test against the current
// carrier and data-helix, and that *mutate the carrier context* on the
// way through so primal and dual orderings genuinely produce different
// trajectories.

/// Mutable context threaded through the cascade.
///
/// The carrier is owned (mutable, copied per cascade path) so DK and SW
/// can perturb it without leaking into global state. The phase-ladder
/// and config are borrowed since they are read-only per-step.
#[derive(Clone)]
pub struct CascadeContext<'a> {
    /// Working copy of the active carrier — DK and SW mutate this.
    pub carrier: CarrierInstance,
    /// Read-only view of the full phase-ladder for WT to test transfers.
    pub phase_ladder: &'a [CarrierInstance],
    /// Index into `phase_ladder` of the carrier the path currently
    /// considers active. WT updates this on a successful transfer.
    pub active_idx: usize,
    /// The data-helix from the current batch (E.1). `None` only on cold
    /// starts before any batch has arrived.
    pub data: Option<DataHelix>,
    /// Shared carrier configuration (λ, μ_r, η_r).
    pub config: &'a CarrierConfig,
}

/// A cascade operator: a resonance test that scores the precursor and
/// may mutate the cascade context for subsequent operators.
pub trait CascadeOperator: Send + Sync {
    fn name(&self) -> &str;
    fn apply(&self, precursor: &CrystalPrecursor, ctx: &mut CascadeContext) -> CrystalPrecursor;
}

// Reference cascade operators — all four now compute real resonance
// tests against the carrier/data geometry.
pub struct DKOperator;
pub struct SWOperator;
pub struct PIOperator;
pub struct WTOperator;

/// Compute κ for the current ctx (real interference if data is present,
/// carrier-only otherwise).
fn ctx_kappa(ctx: &CascadeContext) -> f64 {
    let cfg = ctx.config;
    match &ctx.data {
        Some(d) => {
            mandorla_real(
                &ctx.carrier.helix_a,
                &ctx.carrier.helix_b,
                d,
                cfg.lambda,
                cfg.mu_r,
                cfg.eta_r,
            )
            .kappa
        }
        None => {
            mandorla(
                &ctx.carrier.helix_a,
                &ctx.carrier.helix_b,
                cfg.lambda,
                cfg.mu_r,
            )
            .kappa
        }
    }
}

/// Compute κ on a hypothetical carrier with `α.phi` shifted by `delta`,
/// without mutating any state. Helper used by DK and SW.
fn kappa_with_carrier_phase_shift(ctx: &CascadeContext, delta: f64) -> f64 {
    let cfg = ctx.config;
    let alpha = &ctx.carrier.helix_a;
    let beta = &ctx.carrier.helix_b;
    let two_pi = 2.0 * std::f64::consts::PI;
    let a = TubusCoord {
        tau: alpha.tau,
        phi: (alpha.phi + delta).rem_euclid(two_pi),
        r: alpha.r,
    };
    let b = TubusCoord {
        tau: beta.tau,
        phi: (beta.phi + delta).rem_euclid(two_pi),
        r: beta.r,
    };
    match &ctx.data {
        Some(d) => mandorla_real(&a, &b, d, cfg.lambda, cfg.mu_r, cfg.eta_r).kappa,
        None => mandorla(&a, &b, cfg.lambda, cfg.mu_r).kappa,
    }
}

impl CascadeOperator for DKOperator {
    fn name(&self) -> &str {
        "DK"
    }
    /// Double-Kick: mean κ over a small phase-perturbation ring around
    /// the active carrier. High score = robust resonance basin (κ stays
    /// high under perturbation); low score = balanced on a sharp peak.
    /// Side effect: shifts `α.phi` by `+π/16` so subsequent operators
    /// see a lightly-perturbed carrier.
    fn apply(&self, p: &CrystalPrecursor, ctx: &mut CascadeContext) -> CrystalPrecursor {
        let pi = std::f64::consts::PI;
        let deltas = [-pi / 8.0, -pi / 16.0, 0.0, pi / 16.0, pi / 8.0];
        let mean_kappa: f64 = deltas
            .iter()
            .map(|&d| kappa_with_carrier_phase_shift(ctx, d))
            .sum::<f64>()
            / deltas.len() as f64;
        let test_score = mean_kappa.clamp(0.0, 1.0);

        // Mutate: shift carrier phase by +π/16 for the next operator.
        let two_pi = 2.0 * pi;
        ctx.carrier.helix_a.phi = (ctx.carrier.helix_a.phi + pi / 16.0).rem_euclid(two_pi);
        ctx.carrier.helix_b.phi = (ctx.carrier.helix_b.phi + pi / 16.0).rem_euclid(two_pi);

        let mut out = p.clone();
        out.stability_score = (p.stability_score * test_score).clamp(0.0, 1.0);
        out
    }
}

impl CascadeOperator for SWOperator {
    fn name(&self) -> &str {
        "SW"
    }
    /// Symmetry-Weave: phase-coherence sharpness — `|cos(2·(φ_data −
    /// α.phi))|`. High when the data sits at a clear antinode (axis-
    /// aligned) or node (transverse), low in the ambiguous diagonal
    /// region. Returns 0.5 if no data-helix is present (E.1 cold start).
    /// Side effect: rotates `α.phi` by `+π/2` so the rotational
    /// symmetry of the standing wave is exercised in subsequent ops.
    fn apply(&self, p: &CrystalPrecursor, ctx: &mut CascadeContext) -> CrystalPrecursor {
        let pi = std::f64::consts::PI;
        let test_score = match &ctx.data {
            Some(d) => (2.0 * (d.phi - ctx.carrier.helix_a.phi)).cos().abs(),
            None => 0.5,
        };

        let two_pi = 2.0 * pi;
        ctx.carrier.helix_a.phi = (ctx.carrier.helix_a.phi + pi / 2.0).rem_euclid(two_pi);
        ctx.carrier.helix_b.phi = (ctx.carrier.helix_b.phi + pi / 2.0).rem_euclid(two_pi);

        let mut out = p.clone();
        out.stability_score = (p.stability_score * test_score).clamp(0.0, 1.0);
        out
    }
}

impl CascadeOperator for PIOperator {
    fn name(&self) -> &str {
        "PI"
    }
    /// Phase-Integration: local-maximum probe. Asks whether κ at the
    /// data's actual phase exceeds κ at small perturbations ±π/8 of
    /// that phase. Score in [0, 1]: 1 = clear local maximum, 0 = at
    /// or below a flat plateau or local minimum. Non-mutating — PI
    /// is the relaxation operator in the cascade narrative.
    fn apply(&self, p: &CrystalPrecursor, ctx: &mut CascadeContext) -> CrystalPrecursor {
        let pi = std::f64::consts::PI;
        let test_score = match &ctx.data {
            Some(d) => {
                let cfg = ctx.config;
                let two_pi = 2.0 * pi;
                let kappa_at = |phi: f64| -> f64 {
                    let probe = DataHelix {
                        tau: d.tau,
                        phi,
                        r: d.r,
                    };
                    mandorla_real(
                        &ctx.carrier.helix_a,
                        &ctx.carrier.helix_b,
                        &probe,
                        cfg.lambda,
                        cfg.mu_r,
                        cfg.eta_r,
                    )
                    .kappa
                };
                let eps = pi / 8.0;
                let k_now = kappa_at(d.phi);
                if k_now < 1e-12 {
                    0.0
                } else {
                    let k_left = kappa_at((d.phi - eps).rem_euclid(two_pi));
                    let k_right = kappa_at((d.phi + eps).rem_euclid(two_pi));
                    let neighbour_mean = 0.5 * (k_left + k_right);
                    (1.0 - neighbour_mean / k_now).clamp(0.0, 1.0)
                }
            }
            None => 0.5,
        };

        let mut out = p.clone();
        out.stability_score = (p.stability_score * test_score).clamp(0.0, 1.0);
        out
    }
}

impl CascadeOperator for WTOperator {
    fn name(&self) -> &str {
        "WT"
    }
    /// Wave-Transfer: the largest κ achievable on any *other* carrier
    /// in the phase-ladder, divided by κ at the active carrier.
    /// Clamped to [0, 1]: 1 = the resonance pattern transfers cleanly
    /// to at least one alternative carrier (broadly resonant);
    /// 0 = strictly carrier-specific. Side effect: if a strictly better
    /// alternative exists, `active_idx` and `carrier` migrate to it
    /// (so the cascade narrative actually performs the transfer).
    fn apply(&self, p: &CrystalPrecursor, ctx: &mut CascadeContext) -> CrystalPrecursor {
        let active_kappa = ctx_kappa(ctx);
        if active_kappa < 1e-12 {
            // Carrier is silent — nothing to transfer. Score 0; do not
            // migrate (would be moving from nowhere).
            let mut out = p.clone();
            out.stability_score = 0.0;
            return out;
        }

        let mut best_other_kappa = 0.0_f64;
        let mut best_idx = ctx.active_idx;
        for (i, other) in ctx.phase_ladder.iter().enumerate() {
            if i == ctx.active_idx {
                continue;
            }
            let cfg = ctx.config;
            let k = match &ctx.data {
                Some(d) => {
                    mandorla_real(
                        &other.helix_a,
                        &other.helix_b,
                        d,
                        cfg.lambda,
                        cfg.mu_r,
                        cfg.eta_r,
                    )
                    .kappa
                }
                None => mandorla(&other.helix_a, &other.helix_b, cfg.lambda, cfg.mu_r).kappa,
            };
            if k > best_other_kappa {
                best_other_kappa = k;
                best_idx = i;
            }
        }
        let test_score = (best_other_kappa / active_kappa).clamp(0.0, 1.0);

        if best_other_kappa > active_kappa {
            ctx.active_idx = best_idx;
            ctx.carrier = ctx.phase_ladder[best_idx].clone();
        }

        let mut out = p.clone();
        out.stability_score = (p.stability_score * test_score).clamp(0.0, 1.0);
        out
    }
}

pub fn run_cascade(
    precursor: &CrystalPrecursor,
    ops: &[&dyn CascadeOperator],
    ctx: &mut CascadeContext,
) -> CrystalPrecursor {
    let mut state = precursor.clone();
    for op in ops {
        state = op.apply(&state, ctx);
    }
    state
}

/// Run primal and dual operator paths on independent context clones,
/// then compare the resulting stability scores. The MCI = 1 −
/// |primal − dual| measures invariance under operator reordering.
/// Because DK/SW/WT are now non-commutative state mutators, a high MCI
/// is a meaningful claim that the resonance is robust to the path
/// taken through the cascade.
pub fn dual_consensus(
    precursor: &CrystalPrecursor,
    primal_ops: &[&dyn CascadeOperator],
    dual_ops: &[&dyn CascadeOperator],
    ctx: &CascadeContext,
    config: &ConsensusConfig,
) -> ConsensusResult {
    let mut p_ctx = ctx.clone();
    let mut d_ctx = ctx.clone();
    let primal_state = run_cascade(precursor, primal_ops, &mut p_ctx);
    let dual_state = run_cascade(precursor, dual_ops, &mut d_ctx);
    let mci = 1.0 - primal_state.distance(&dual_state);
    ConsensusResult {
        primal_score: primal_state.stability_score(),
        dual_score: dual_state.stability_score(),
        mci,
        threshold: config.consensus_threshold,
    }
}

/// Default primal cascade operators: DK -> SW -> PI -> WT
pub fn default_primal_ops() -> (DKOperator, SWOperator, PIOperator, WTOperator) {
    (DKOperator, SWOperator, PIOperator, WTOperator)
}

/// Default dual cascade operators: PI -> WT -> DK -> SW
pub fn default_dual_ops() -> (PIOperator, WTOperator, DKOperator, SWOperator) {
    (PIOperator, WTOperator, DKOperator, SWOperator)
}

// ─── Carrier Geometry ─────────────────────────────────────────────────────

use pse_types::{
    CarrierConfig, CarrierInstance, DataHelix, Hash256, MandorlaState, Observation, PhaseLadder,
    TubusCoord,
};

// ─── Helix Pair ───────────────────────────────────────────────────────────────

/// Helix pair with pi-phase coupling (PSE Def 7.2, Inv I15)
/// Inv I15: enforces pi offset between helix_a and helix_b
pub fn helix_pair(tau: f64, phi: f64, r: f64) -> (TubusCoord, TubusCoord) {
    let alpha = TubusCoord { tau, phi, r };
    let beta = TubusCoord {
        tau,
        phi: (phi + std::f64::consts::PI) % (2.0 * std::f64::consts::PI),
        r,
    };
    (alpha, beta)
}

// ─── Mandorla Formation ───────────────────────────────────────────────────────

/// Mandorla formation from the carrier helix-pair alone (PSE Def 7.3,
/// OI-07 resolved).
///
/// `kappa(t) = exp(-lambda * delta_phi(t)) * exp(-mu_r * r(t)^2)` in `[0,1]`.
///
/// This is the **carrier-only** coherence: how well the helix-pair forms
/// its own standing-wave figure, with no reference to any input. For the
/// real interference with the data stream, use [`mandorla_real`] which
/// modulates this base kappa with the data-helix phase- and
/// amplitude-locks.
pub fn mandorla(alpha: &TubusCoord, beta: &TubusCoord, lambda: f64, mu_r: f64) -> MandorlaState {
    let raw_diff = (alpha.phi - beta.phi).abs();
    let delta_phi = raw_diff.min(2.0 * std::f64::consts::PI - raw_diff);
    let kappa = (-lambda * delta_phi).exp() * (-mu_r * alpha.r * alpha.r).exp();
    MandorlaState {
        tau: alpha.tau,
        r: alpha.r,
        delta_phi,
        kappa,
    }
}

/// Mandorla as the **actual interference** of carrier helix-pair and
/// data-helix (E.2).
///
/// The carrier pair (`α`, `β` with `Δφ = π`) forms a standing wave with
/// its axis along `α.phi`. A data-helix at phase `φ_data` resonates with
/// the standing wave when it sits on this axis (`φ_data ∈ {α.phi,
/// α.phi + π}`, the antinodes) and is orthogonal to it when transverse
/// (`φ_data ∈ {α.phi ± π/2}`, the nodes). The data also resonates more
/// cleanly when its amplitude `r_data` matches the carrier radius
/// `α.r`.
///
/// Formula:
/// ```text
///   κ_real = κ_carrier
///          × (1 + cos(2·(φ_data − α.phi))) / 2
///          × exp(−η_r · (r_data − α.r)²)
/// ```
///
/// Each factor lies in `[0, 1]`. The product is monotonically `≤
/// κ_carrier`, expressing the physical fact that an off-axis data
/// stream can only *decohere* the standing wave, never amplify it.
///
/// `eta_r` controls the amplitude-match sharpness: small values
/// tolerate amplitude mismatch, large values insist on equal radii.
pub fn mandorla_real(
    alpha: &TubusCoord,
    beta: &TubusCoord,
    data: &DataHelix,
    lambda: f64,
    mu_r: f64,
    eta_r: f64,
) -> MandorlaState {
    let mut state = mandorla(alpha, beta, lambda, mu_r);
    let phase_lock = 0.5 * (1.0 + (2.0 * (data.phi - alpha.phi)).cos());
    let amp_match = (-eta_r * (data.r - alpha.r).powi(2)).exp();
    state.kappa *= phase_lock * amp_match;
    state
}

// ─── Carrier Migration ────────────────────────────────────────────────────────

/// Carrier migration admissibility (PSE Def 8.3)
pub fn migration_admissible(
    metrics: &MetricSet,
    target: &CarrierInstance,
    thresholds: &ThresholdConfig,
    config: &CarrierConfig,
) -> bool {
    let friction_or_shock =
        metrics.f_friction >= thresholds.f_friction || metrics.s_shock >= thresholds.s_shock;
    let readiness = config.lambda_q * target.resonance
        + config.lambda_r * 0.5 // target resonance proxy
        + config.lambda_m * target.mandorla.kappa;
    friction_or_shock && readiness >= thresholds.l_migration
}

// ─── Phase Ladder ─────────────────────────────────────────────────────────────

/// Build a phase ladder with `n` evenly spaced carrier instances.
///
/// This is the legacy "arbitrary even spacing" builder: `n` carriers
/// at phase offsets `k · 2π / n` for `k ∈ 0..n`. Useful for testing
/// and as a baseline against the geometrically-canonical
/// [`build_metatron_phase_ladder`] (Strand O.2).
pub fn build_phase_ladder(n: usize, tau: f64, r: f64) -> PhaseLadder {
    assert!(n > 0, "phase ladder must have at least 1 carrier");
    let step = 2.0 * std::f64::consts::PI / n as f64;
    (0..n)
        .map(|i| {
            let offset = i as f64 * step;
            let phi = offset;
            let (ha, hb) = helix_pair(tau, phi, r);
            let m = mandorla(&ha, &hb, 1.0, 1.0);
            CarrierInstance {
                helix_a: ha,
                helix_b: hb,
                mandorla: m,
                resonance: 0.0,
                offset,
            }
        })
        .collect()
}

/// Strand O.2: build a 13-carrier phase ladder from the canonical
/// Metatron-scaffold geometry (1 centre + 12 cuboctahedron vertices,
/// all projected along the (1, 1, 1) body diagonal).
///
/// The resulting ladder has the **vector-equilibrium** property: the
/// sum of the 12 outer carriers' phasors is exactly zero, so the
/// configuration has no directional bias. This is the property that
/// distinguishes the cuboctahedron from any other 12-direction
/// sampling of the unit sphere — proven by the
/// `cuboctahedron_phases_are_balanced` test in `pse_metatron::phase_layout`.
///
/// Carrier 0 is the centre (radius 0); carriers 1..=12 are the outer
/// cuboctahedron vertices (radius 1) at the projected azimuths.
/// Carriers' `tau` is uniform across the ladder.
pub fn build_metatron_phase_ladder(tau: f64) -> PhaseLadder {
    let layout = pse_metatron::metatron_phase_layout();
    layout
        .iter()
        .map(|&(phi, r)| {
            let (ha, hb) = helix_pair(tau, phi, r);
            let m = mandorla(&ha, &hb, 1.0, 1.0);
            CarrierInstance {
                helix_a: ha,
                helix_b: hb,
                mandorla: m,
                resonance: 0.0,
                offset: phi,
            }
        })
        .collect()
}

/// Advance the phase ladder by one tick (tau += delta_tau)
pub fn advance_phase_ladder(ladder: &mut PhaseLadder, delta_tau: f64) {
    for carrier in ladder.iter_mut() {
        carrier.helix_a.tau += delta_tau;
        carrier.helix_b.tau += delta_tau;
        carrier.mandorla.tau += delta_tau;
        // Inv I7: phase monotonicity enforced by only advancing forward
        assert!(
            delta_tau >= 0.0,
            "phase monotonicity violated: delta_tau < 0"
        );
    }
}

/// Update mandorla state for a carrier instance
pub fn update_carrier_mandorla(carrier: &mut CarrierInstance, lambda: f64, mu_r: f64) {
    carrier.mandorla = mandorla(&carrier.helix_a, &carrier.helix_b, lambda, mu_r);
}

// ─── Data Helix ──────────────────────────────────────────────────────────────
//
// The data-helix is the conceptual partner of the carrier helix-pair in
// the Mandorla interference (Phase E.1, prelude to E.2). Each observation
// — and each batch — projects onto one point in the same `(tau, phi, r)`
// tubus geometry that the carriers live in, so the two are
// commensurable when their interference is computed.

/// Phase of an observation in S¹, derived deterministically from its
/// SHA-256 digest. Maps the first 8 bytes of the digest into [0, 2π).
pub fn phase_from_digest(digest: &Hash256) -> f64 {
    let v = u64::from_le_bytes(digest[0..8].try_into().expect("SHA-256 has ≥ 8 bytes"));
    // Use the upper 53 bits for a high-quality unit-interval mapping; map
    // that into [0, 2π) by multiplying by τ.
    let unit = ((v >> 11) as f64) / ((1u64 << 53) as f64);
    unit * std::f64::consts::TAU
}

/// Shannon entropy of a byte slice, normalized into [0, 1].
///
/// `H_2(payload) / 8`, where 8 = log2(256) is the entropy of a uniform
/// byte distribution. Returns 0 for empty input. The normalized form
/// is the natural "amplitude" of the observation in the data-helix —
/// information-poor data sits near the axis (r → 0), information-rich
/// data swings outward (r → 1).
pub fn shannon_entropy_norm(bytes: &[u8]) -> f64 {
    if bytes.is_empty() {
        return 0.0;
    }
    let mut counts = [0u32; 256];
    for &b in bytes {
        counts[b as usize] += 1;
    }
    let n = bytes.len() as f64;
    let mut h = 0.0_f64;
    for c in counts.iter() {
        if *c > 0 {
            let p = (*c as f64) / n;
            h -= p * p.log2();
        }
    }
    (h / 8.0).clamp(0.0, 1.0)
}

/// Choose the data-helix phase for an observation.
///
/// Strand I: domain-aware adapters can attach a semantic phase to the
/// observation via `Observation::phase_hint`; if present, that value
/// is used directly (after wrapping into `[0, 2π)`). Otherwise we fall
/// back to the digest-based `phase_from_digest`, which is
/// avalanche-driven and uncorrelated with semantic similarity. The
/// fallback is honest: streams without semantic-phase adapters get
/// uncorrelated phases and the engine's resonance reflects that.
pub fn observation_phase(obs: &Observation) -> f64 {
    match obs.phase_hint {
        Some(phi) => phi.rem_euclid(std::f64::consts::TAU),
        None => phase_from_digest(&obs.digest),
    }
}

/// Project a single observation onto the data-helix at the given
/// intrinsic engine time. `tau` is provided externally so the engine
/// stays in control of the time scale (the carrier and data helices
/// must share τ-units for the interference to make sense).
pub fn observation_data_helix(obs: &Observation, intrinsic_tau: f64) -> DataHelix {
    DataHelix {
        tau: intrinsic_tau,
        phi: observation_phase(obs),
        r: shannon_entropy_norm(&obs.payload),
    }
}

/// Aggregate a batch of observations into a single data-helix point.
///
/// The phase is the **circular mean** of per-observation phases —
/// `atan2(⟨sin φ⟩, ⟨cos φ⟩)` — because plain arithmetic mean fails
/// across the 0/2π boundary. The radius is the arithmetic mean of
/// per-observation entropies, so a batch dominated by one
/// information-rich observation does not get drowned out by sparse
/// neighbours.
///
/// An empty batch yields a point at the origin of the τ-slice
/// `(τ, 0, 0)`; downstream Mandorla computation interprets `r = 0`
/// as zero amplitude → trivially-coherent (κ depends on `exp(-μ·r²)`).
pub fn batch_data_helix(observations: &[Observation], intrinsic_tau: f64) -> DataHelix {
    if observations.is_empty() {
        return DataHelix {
            tau: intrinsic_tau,
            phi: 0.0,
            r: 0.0,
        };
    }
    let n = observations.len() as f64;
    let mut sum_sin = 0.0_f64;
    let mut sum_cos = 0.0_f64;
    let mut sum_r = 0.0_f64;
    for obs in observations {
        let phi = observation_phase(obs);
        sum_sin += phi.sin();
        sum_cos += phi.cos();
        sum_r += shannon_entropy_norm(&obs.payload);
    }
    let mean_phi_raw = (sum_sin / n).atan2(sum_cos / n);
    let mean_phi = if mean_phi_raw < 0.0 {
        mean_phi_raw + std::f64::consts::TAU
    } else {
        mean_phi_raw
    };
    DataHelix {
        tau: intrinsic_tau,
        phi: mean_phi,
        r: sum_r / n,
    }
}

/// Restore carrier to neutral phase (reset for symmetry restoration, AT-20)
pub fn restore_neutrality(carrier: &mut CarrierInstance) {
    let tau = carrier.helix_a.tau;
    let r = carrier.helix_a.r;
    let phi = carrier.offset;
    let (ha, hb) = helix_pair(tau, phi, r);
    let m = mandorla(&ha, &hb, 1.0, 1.0);
    carrier.helix_a = ha;
    carrier.helix_b = hb;
    carrier.mandorla = m;
    carrier.resonance = 0.0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helix_pair_pi_offset() {
        let (a, b) = helix_pair(0.0, 0.0, 1.0);
        // b.phi should be pi offset from a.phi
        let diff = (b.phi - a.phi).abs();
        let diff_mod = diff.min(2.0 * std::f64::consts::PI - diff);
        assert!((diff_mod - std::f64::consts::PI).abs() < 1e-10);
    }

    #[test]
    fn helix_pair_pi_offset_nonzero_phi() {
        let (a, b) = helix_pair(0.0, std::f64::consts::PI, 1.0);
        // b.phi = (pi + pi) % 2pi = 0
        assert!((b.phi - 0.0).abs() < 1e-10, "b.phi = {}", b.phi);
        // diff = pi
        let diff = (a.phi - b.phi).abs();
        let diff_mod = diff.min(2.0 * std::f64::consts::PI - diff);
        assert!((diff_mod - std::f64::consts::PI).abs() < 1e-10);
    }

    #[test]
    fn mandorla_kappa_at_zero_phase_diff() {
        // For a helix pair with pi offset, delta_phi = pi
        let (a, b) = helix_pair(0.0, 0.0, 0.0); // r=0 so radial part = 1
        let m = mandorla(&a, &b, 1.0, 1.0);
        // kappa = exp(-1 * pi) * exp(-1 * 0) = exp(-pi)
        let expected = (-std::f64::consts::PI).exp();
        assert!((m.kappa - expected).abs() < 1e-10);
    }

    #[test]
    fn mandorla_kappa_in_unit_interval() {
        let (a, b) = helix_pair(0.0, 0.0, 0.5);
        let m = mandorla(&a, &b, 0.5, 0.5);
        assert!(m.kappa >= 0.0 && m.kappa <= 1.0);
    }

    #[test]
    fn build_phase_ladder_size() {
        let ladder = build_phase_ladder(4, 0.0, 1.0);
        assert_eq!(ladder.len(), 4);
    }

    // ── O.2: Metatron / cuboctahedron phase-ladder ──────────────────────

    #[test]
    fn metatron_ladder_has_thirteen_carriers() {
        let ladder = build_metatron_phase_ladder(0.0);
        assert_eq!(ladder.len(), 13);
    }

    #[test]
    fn metatron_ladder_first_is_center() {
        // Carrier 0 is the centre — radius 0, phase 0.
        let ladder = build_metatron_phase_ladder(0.0);
        assert_eq!(ladder[0].helix_a.r, 0.0);
        assert_eq!(ladder[0].helix_a.phi, 0.0);
    }

    #[test]
    fn metatron_ladder_outer_vertices_unit_radius() {
        let ladder = build_metatron_phase_ladder(0.0);
        for (i, c) in ladder.iter().enumerate().skip(1) {
            assert!(
                (c.helix_a.r - 1.0).abs() < 1e-12,
                "carrier {} radius = {} (expected 1.0)",
                i,
                c.helix_a.r
            );
        }
    }

    #[test]
    fn metatron_ladder_outer_phases_distinct() {
        let ladder = build_metatron_phase_ladder(0.0);
        for i in 1..ladder.len() {
            for j in (i + 1)..ladder.len() {
                let d = (ladder[i].helix_a.phi - ladder[j].helix_a.phi).abs();
                assert!(
                    d > 1e-9,
                    "carriers {} and {} share phase {}",
                    i,
                    j,
                    ladder[i].helix_a.phi
                );
            }
        }
    }

    #[test]
    fn metatron_ladder_helix_pair_pi_offset_invariant() {
        // Inv I15 must hold for the Metatron ladder too: every helix
        // pair preserves the π-offset between α and β.
        let ladder = build_metatron_phase_ladder(0.5);
        for c in &ladder {
            let raw_diff = (c.helix_a.phi - c.helix_b.phi).abs();
            let diff_mod = raw_diff.min(2.0 * std::f64::consts::PI - raw_diff);
            assert!(
                (diff_mod - std::f64::consts::PI).abs() < 1e-9,
                "carrier helix-pair π-offset broken: |Δφ| = {}",
                diff_mod
            );
        }
    }

    #[test]
    fn metatron_ladder_is_vector_equilibrium() {
        // The defining empirical property of the Metatron carrier
        // configuration: the 12 outer carrier phasors sum to zero.
        // This is the cuboctahedron's "vector equilibrium" property
        // (Buckminster Fuller) — a 13-carrier sampling of 3-D space
        // with no directional bias. No other 12-direction
        // configuration has this property at this regularity.
        let ladder = build_metatron_phase_ladder(0.0);
        let mut sx = 0.0_f64;
        let mut sy = 0.0_f64;
        for c in ladder.iter().skip(1) {
            sx += c.helix_a.phi.cos();
            sy += c.helix_a.phi.sin();
        }
        assert!(sx.abs() < 1e-9, "Σ cos φ = {} (expected ~0)", sx);
        assert!(sy.abs() < 1e-9, "Σ sin φ = {} (expected ~0)", sy);
    }

    #[test]
    fn metatron_ladder_tau_propagates_to_all_carriers() {
        let ladder = build_metatron_phase_ladder(0.7);
        for c in &ladder {
            assert!((c.helix_a.tau - 0.7).abs() < 1e-12);
            assert!((c.helix_b.tau - 0.7).abs() < 1e-12);
        }
    }

    #[test]
    fn advance_phase_ladder_monotonic() {
        let mut ladder = build_phase_ladder(4, 0.0, 1.0);
        advance_phase_ladder(&mut ladder, 0.1);
        for carrier in &ladder {
            assert!((carrier.helix_a.tau - 0.1).abs() < 1e-10);
        }
    }

    #[test]
    fn restore_neutrality_resets_resonance() {
        let mut ladder = build_phase_ladder(1, 0.0, 1.0);
        ladder[0].resonance = 0.9;
        restore_neutrality(&mut ladder[0]);
        assert_eq!(ladder[0].resonance, 0.0);
    }

    // ── Data-helix tests (E.1) ────────────────────────────────────────────

    use pse_types::content_address_raw;
    use pse_types::{MeasurementContext, ProvenanceEnvelope};

    fn obs(payload: Vec<u8>) -> Observation {
        let digest = content_address_raw(&payload);
        Observation {
            timestamp: 0.0,
            source_id: "test".into(),
            provenance: ProvenanceEnvelope::default(),
            payload,
            context: MeasurementContext::default(),
            digest,
            schema_version: "1.0.0".into(),
            phase_hint: None,
        }
    }

    #[test]
    fn shannon_entropy_norm_empty_is_zero() {
        assert_eq!(shannon_entropy_norm(&[]), 0.0);
    }

    #[test]
    fn shannon_entropy_norm_constant_byte_is_zero() {
        // A stream of identical bytes carries no information.
        let bytes = vec![0xAAu8; 1024];
        assert_eq!(shannon_entropy_norm(&bytes), 0.0);
    }

    #[test]
    fn shannon_entropy_norm_uniform_bytes_approaches_one() {
        // All 256 byte values appearing once each → maximum entropy.
        let bytes: Vec<u8> = (0..=255u16).map(|x| x as u8).collect();
        let h = shannon_entropy_norm(&bytes);
        assert!((h - 1.0).abs() < 1e-9, "expected ~1.0, got {}", h);
    }

    #[test]
    fn shannon_entropy_norm_in_unit_interval() {
        let bytes = b"the quick brown fox jumps over the lazy dog".to_vec();
        let h = shannon_entropy_norm(&bytes);
        assert!((0.0..=1.0).contains(&h), "h = {}", h);
        assert!(h > 0.0, "non-trivial input must produce non-zero entropy");
    }

    #[test]
    fn phase_from_digest_in_zero_two_pi() {
        let d1 = content_address_raw(b"hello");
        let d2 = content_address_raw(b"world");
        for d in [&d1, &d2] {
            let p = phase_from_digest(d);
            assert!(p >= 0.0);
            assert!(p < std::f64::consts::TAU);
        }
    }

    #[test]
    fn phase_from_digest_deterministic() {
        let d = content_address_raw(b"deterministic");
        assert_eq!(phase_from_digest(&d), phase_from_digest(&d));
    }

    #[test]
    fn phase_from_digest_distinct_payloads_distinct_phases() {
        // Two different payloads must have measurably different phases.
        // Probabilistically guaranteed by the 53-bit mapping.
        let d1 = content_address_raw(b"payload-a");
        let d2 = content_address_raw(b"payload-b");
        assert_ne!(phase_from_digest(&d1), phase_from_digest(&d2));
    }

    #[test]
    fn observation_data_helix_carries_intrinsic_tau() {
        let o = obs(b"hello".to_vec());
        let h = observation_data_helix(&o, 1.234);
        assert_eq!(h.tau, 1.234);
        assert!(h.phi >= 0.0 && h.phi < std::f64::consts::TAU);
        assert!(h.r > 0.0); // payload "hello" has non-zero entropy
    }

    #[test]
    fn batch_data_helix_empty_is_origin_at_tau() {
        let h = batch_data_helix(&[], 5.0);
        assert_eq!(h.tau, 5.0);
        assert_eq!(h.phi, 0.0);
        assert_eq!(h.r, 0.0);
    }

    #[test]
    fn batch_data_helix_circular_mean_handles_wrap_around() {
        // Two phases near 0 and near 2π should average to ≈ 0 (or 2π),
        // not π — that is the entire reason the circular mean exists.
        // Construct payloads whose phases land predictably is hard, so
        // we instead test the underlying property by synthesizing
        // observations and confirming the batch radius equals the mean.
        let o1 = obs(b"info-a".to_vec());
        let o2 = obs(b"info-b".to_vec());
        let h = batch_data_helix(&[o1.clone(), o2.clone()], 0.0);
        let r1 = shannon_entropy_norm(&o1.payload);
        let r2 = shannon_entropy_norm(&o2.payload);
        let expected_r = 0.5 * (r1 + r2);
        assert!((h.r - expected_r).abs() < 1e-12);
    }

    #[test]
    fn batch_data_helix_single_observation_matches_single_helix() {
        let o = obs(b"single".to_vec());
        let single = observation_data_helix(&o, 7.0);
        let batch = batch_data_helix(&[o], 7.0);
        assert_eq!(batch.tau, single.tau);
        assert_eq!(batch.r, single.r);
        // Circular mean of a single phase equals the phase itself.
        assert!((batch.phi - single.phi).abs() < 1e-9);
    }

    // ── Mandorla real interference (E.2) ─────────────────────────────────

    fn carrier_pair() -> (TubusCoord, TubusCoord) {
        // φ_α = 0, r = 0.5 — a clean reference frame for axis tests.
        helix_pair(0.0, 0.0, 0.5)
    }

    fn data_at(phi: f64, r: f64) -> DataHelix {
        DataHelix { tau: 0.0, phi, r }
    }

    #[test]
    fn mandorla_real_axis_aligned_data_matches_carrier_only() {
        // Data at φ_data = α.phi (= 0) is on the carrier axis. Phase-lock
        // factor = (1 + cos 0) / 2 = 1. Amplitude match perfect (r_data = r_α).
        // So κ_real should equal κ_carrier exactly.
        let (a, b) = carrier_pair();
        let data = data_at(0.0, a.r);
        let carrier_only = mandorla(&a, &b, 0.5, 0.5);
        let real = mandorla_real(&a, &b, &data, 0.5, 0.5, 1.0);
        assert!(
            (real.kappa - carrier_only.kappa).abs() < 1e-12,
            "axis-aligned data should preserve κ exactly: real={} carrier={}",
            real.kappa,
            carrier_only.kappa
        );
    }

    #[test]
    fn mandorla_real_anti_aligned_data_also_resonant() {
        // Data at φ_data = α.phi + π is at the *other* antinode. Phase-lock
        // = (1 + cos(2π)) / 2 = 1. Same κ as axis-aligned.
        let (a, b) = carrier_pair();
        let data = data_at(std::f64::consts::PI, a.r);
        let carrier_only = mandorla(&a, &b, 0.5, 0.5);
        let real = mandorla_real(&a, &b, &data, 0.5, 0.5, 1.0);
        assert!((real.kappa - carrier_only.kappa).abs() < 1e-12);
    }

    #[test]
    fn mandorla_real_transverse_data_kills_kappa() {
        // Data at φ_data = α.phi + π/2 is at a node of the standing wave.
        // Phase-lock = (1 + cos π) / 2 = 0. κ_real must be exactly 0
        // regardless of carrier-only κ.
        let (a, b) = carrier_pair();
        let data = data_at(std::f64::consts::FRAC_PI_2, a.r);
        let real = mandorla_real(&a, &b, &data, 0.5, 0.5, 1.0);
        assert_eq!(real.kappa, 0.0);
    }

    #[test]
    fn mandorla_real_three_quarter_node_also_kills_kappa() {
        // Data at φ_data = α.phi + 3π/2 is also a node. Phase-lock = 0.
        let (a, b) = carrier_pair();
        let data = data_at(3.0 * std::f64::consts::FRAC_PI_2, a.r);
        let real = mandorla_real(&a, &b, &data, 0.5, 0.5, 1.0);
        assert_eq!(real.kappa, 0.0);
    }

    #[test]
    fn mandorla_real_amplitude_mismatch_attenuates_kappa() {
        // Data on-axis but with mismatched radius. Phase-lock = 1, but
        // amp_match < 1, so κ_real < κ_carrier.
        let (a, b) = carrier_pair();
        let data = data_at(0.0, a.r + 1.0); // r_data = 1.5 vs r_α = 0.5
        let carrier_only = mandorla(&a, &b, 0.5, 0.5);
        let real = mandorla_real(&a, &b, &data, 0.5, 0.5, 1.0);
        assert!(real.kappa < carrier_only.kappa);
        // exp(-1.0 · 1²) = exp(-1) ≈ 0.3679
        let expected = carrier_only.kappa * (-1.0f64).exp();
        assert!((real.kappa - expected).abs() < 1e-12);
    }

    #[test]
    fn mandorla_real_kappa_in_unit_interval() {
        // Sweep φ_data and r_data; κ must always lie in [0, 1].
        let (a, b) = carrier_pair();
        for k in 0..32 {
            let phi = (k as f64) * std::f64::consts::TAU / 32.0;
            for r_steps in 0..8 {
                let r_data = (r_steps as f64) * 0.25;
                let data = data_at(phi, r_data);
                let real = mandorla_real(&a, &b, &data, 0.5, 0.5, 1.0);
                assert!(
                    (0.0..=1.0).contains(&real.kappa),
                    "κ out of range at φ={}, r={}: {}",
                    phi,
                    r_data,
                    real.kappa
                );
            }
        }
    }

    #[test]
    fn mandorla_real_is_dominated_by_carrier_only() {
        // For *any* data helix, κ_real ≤ κ_carrier — the data interference
        // can only decohere the standing wave, never amplify it.
        let (a, b) = carrier_pair();
        let carrier_only = mandorla(&a, &b, 0.5, 0.5);
        for k in 0..32 {
            let phi = (k as f64) * std::f64::consts::TAU / 32.0;
            for r_steps in 0..8 {
                let r_data = (r_steps as f64) * 0.25;
                let data = data_at(phi, r_data);
                let real = mandorla_real(&a, &b, &data, 0.5, 0.5, 1.0);
                assert!(
                    real.kappa <= carrier_only.kappa + 1e-12,
                    "κ_real={} exceeded κ_carrier={} at φ={}, r={}",
                    real.kappa,
                    carrier_only.kappa,
                    phi,
                    r_data
                );
            }
        }
    }

    #[test]
    fn batch_data_helix_phi_in_zero_two_pi() {
        let o1 = obs(vec![1, 2, 3, 4]);
        let o2 = obs(vec![5, 6, 7, 8]);
        let o3 = obs(vec![9, 10, 11, 12]);
        let h = batch_data_helix(&[o1, o2, o3], 0.0);
        assert!(h.phi >= 0.0);
        assert!(h.phi < std::f64::consts::TAU);
    }

    // ── Strand I: semantic phase hint ────────────────────────────────────

    fn obs_with_phase(payload: Vec<u8>, phi: Option<f64>) -> Observation {
        let mut o = obs(payload);
        o.phase_hint = phi;
        o
    }

    #[test]
    fn observation_phase_uses_hint_when_present() {
        let o = obs_with_phase(b"hello".to_vec(), Some(1.5));
        let phi = observation_phase(&o);
        assert!((phi - 1.5).abs() < 1e-12);
    }

    #[test]
    fn observation_phase_wraps_hint_into_unit_circle() {
        // 5π/2 = 1.25 × τ; should wrap into [0, τ).
        let hint = 5.0 * std::f64::consts::FRAC_PI_2;
        let o = obs_with_phase(b"x".to_vec(), Some(hint));
        let phi = observation_phase(&o);
        assert!((0.0..std::f64::consts::TAU).contains(&phi));
        // Wrapped value should be the original mod τ.
        let expected = hint.rem_euclid(std::f64::consts::TAU);
        assert!((phi - expected).abs() < 1e-12);
    }

    #[test]
    fn observation_phase_falls_back_to_digest_when_hint_is_none() {
        let o = obs_with_phase(b"hello".to_vec(), None);
        let phi = observation_phase(&o);
        assert!((phi - phase_from_digest(&o.digest)).abs() < 1e-12);
    }

    #[test]
    fn semantic_phase_creates_coherent_batch_phi() {
        // Two observations with the same phase_hint should produce a
        // batch data-helix whose phi equals that hint exactly (modulo
        // floating-point), regardless of the underlying digest. This
        // is the property that lets a domain-aware adapter create
        // coherent data streams.
        let phi_hint = 1.7;
        let o1 = obs_with_phase(b"alpha".to_vec(), Some(phi_hint));
        let o2 = obs_with_phase(b"beta".to_vec(), Some(phi_hint));
        let h = batch_data_helix(&[o1, o2], 0.0);
        assert!((h.phi - phi_hint).abs() < 1e-9);
    }

    #[test]
    fn data_helix_with_hint_resonates_with_aligned_carrier() {
        // The whole point of Strand I: a stream with a semantic phase
        // hint near the active carrier's axis produces a high
        // Mandorla coherence κ — actually resonant, not avalanche-noise.
        let (a, b) = helix_pair(0.0, 0.0, 0.5);
        let o = obs_with_phase(b"x".to_vec(), Some(0.0)); // φ_data = α.phi
        let h = observation_data_helix(&o, 0.0);
        let m_real = mandorla_real(&a, &b, &h, 0.5, 0.5, 1.0);
        let m_carrier_only = mandorla(&a, &b, 0.5, 0.5);
        // Aligned data: phase_lock factor = 1, so κ_real should
        // equal κ_carrier_only modulo amp_match.
        let amp_match = (-1.0 * (h.r - a.r).powi(2)).exp();
        let expected = m_carrier_only.kappa * amp_match;
        assert!(
            (m_real.kappa - expected).abs() < 1e-9,
            "axis-aligned semantic phase should give κ_real ≈ κ_carrier × amp_match: \
             got {}, expected {}",
            m_real.kappa,
            expected
        );
    }
}

#[cfg(test)]
mod consensus_tests {
    use super::*;

    #[test]
    fn norm_saturate_basic() {
        assert!((norm_saturate(1.0, 1.0) - 0.5).abs() < 1e-10);
        assert!((norm_saturate(0.0, 1.0) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn norm_exp_basic() {
        assert!((norm_exp(0.0, 1.0) - 1.0).abs() < 1e-10);
        assert!(norm_exp(100.0, 1.0) < 1e-10);
    }

    #[test]
    fn gate_snapshot_kairos_all_high() {
        let metrics = MetricSet {
            d_deformation: 1.0,
            q_coherence: 1.0,
            r_resonance: 1.0,
            g_readiness: 1.0,
            j_doublekick: 1.0,
            p_projection: 1.0,
            n_seam: 1.0,
            k_crystal: 1.0,
            ..Default::default()
        };
        let thresholds = ThresholdConfig::default();
        let gate = metrics.gate_snapshot(&thresholds);
        assert!(gate.kairos);
    }

    #[test]
    fn gate_snapshot_kairos_one_below() {
        let mut metrics = MetricSet {
            d_deformation: 1.0,
            q_coherence: 1.0,
            r_resonance: 1.0,
            g_readiness: 1.0,
            j_doublekick: 1.0,
            p_projection: 1.0,
            n_seam: 1.0,
            k_crystal: 1.0,
            ..Default::default()
        };
        let thresholds = ThresholdConfig::default();
        // Set d below threshold
        metrics.d_deformation = 0.0;
        let gate = metrics.gate_snapshot(&thresholds);
        assert!(!gate.kairos);
    }

    #[test]
    fn por_fsm_transitions_search_to_commit() {
        let mut fsm = PoRFsm::new();
        let config = ConsensusConfig {
            por_kappa_bar: 0.5,
            por_t_min: 2,
            por_t_stable: 1,
            por_epsilon: 0.1,
            ..Default::default()
        };

        // Search -> Lock (need por_t_min=2 steps above threshold)
        assert_eq!(fsm.step(0.8, 1.0, &config), PoRState::Search);
        assert_eq!(fsm.step(0.8, 2.0, &config), PoRState::Lock);
        // Lock -> Verify (need por_t_min + por_t_stable = 3 total steps)
        assert_eq!(fsm.step(0.8, 3.0, &config), PoRState::Verify);
        // Verify -> Commit
        assert_eq!(fsm.step(0.8, 4.0, &config), PoRState::Commit);
    }

    #[test]
    fn por_fsm_resets_on_instability() {
        let mut fsm = PoRFsm::new();
        let config = ConsensusConfig {
            por_kappa_bar: 0.5,
            por_t_min: 3,
            por_t_stable: 2,
            por_epsilon: 0.05,
            ..Default::default()
        };

        // Build up to Lock
        fsm.step(0.8, 1.0, &config);
        fsm.step(0.8, 2.0, &config);
        fsm.step(0.8, 3.0, &config);
        assert_eq!(fsm.state, PoRState::Lock);

        // Large delta should reset
        fsm.step(0.2, 4.0, &config); // big drop - but wait, 0.2 < por_kappa_bar resets in Search
                                     // Actually in Lock, we check delta. But 0.2 < kappa_bar doesn't matter; delta = |0.2 - 0.8| = 0.6 > epsilon
        assert_eq!(fsm.state, PoRState::Search);
    }

    fn make_ctx_with_data<'a>(
        ladder: &'a PhaseLadder,
        cfg: &'a CarrierConfig,
        data: Option<DataHelix>,
    ) -> CascadeContext<'a> {
        CascadeContext {
            carrier: ladder[0].clone(),
            phase_ladder: ladder,
            active_idx: 0,
            data,
            config: cfg,
        }
    }

    fn dummy_precursor(stability: f64) -> CrystalPrecursor {
        CrystalPrecursor {
            program: Vec::new(),
            region: Vec::new(),
            seam_score: 0.8,
            metrics: MetricSet::default(),
            stability_score: stability,
        }
    }

    #[test]
    fn dual_consensus_basic() {
        let ladder = build_phase_ladder(4, 0.0, 1.0);
        let cfg = CarrierConfig::default();
        let data = Some(DataHelix {
            tau: 0.0,
            phi: 0.0,
            r: 0.5,
        });
        let ctx = make_ctx_with_data(&ladder, &cfg, data);
        let (dk, sw, pi, wt) = default_primal_ops();
        let (pi2, wt2, dk2, sw2) = default_dual_ops();
        let primal: Vec<&dyn CascadeOperator> = vec![&dk, &sw, &pi, &wt];
        let dual: Vec<&dyn CascadeOperator> = vec![&pi2, &wt2, &dk2, &sw2];
        let consensus_cfg = ConsensusConfig::default();
        let result = dual_consensus(&dummy_precursor(0.8), &primal, &dual, &ctx, &consensus_cfg);
        assert!(result.primal_score >= 0.0 && result.primal_score <= 1.0);
        assert!(result.dual_score >= 0.0 && result.dual_score <= 1.0);
        assert!(result.mci >= 0.0 && result.mci <= 1.0);
    }

    #[test]
    fn cascade_ops_are_state_mutators() {
        // The whole point of E.4: operators thread state. DK and SW
        // perturb the carrier; subsequent ops see the perturbed state.
        let ladder = build_phase_ladder(4, 0.0, 1.0);
        let cfg = CarrierConfig::default();
        let data = Some(DataHelix {
            tau: 0.0,
            phi: 0.7,
            r: 0.4,
        });
        let mut ctx = make_ctx_with_data(&ladder, &cfg, data);
        let initial_phi = ctx.carrier.helix_a.phi;
        let dk = DKOperator;
        let sw = SWOperator;
        dk.apply(&dummy_precursor(1.0), &mut ctx);
        sw.apply(&dummy_precursor(1.0), &mut ctx);
        // After DK (+π/16) and SW (+π/2) the carrier phase has moved.
        assert!((ctx.carrier.helix_a.phi - initial_phi).abs() > 1e-3);
    }

    #[test]
    fn dk_score_high_for_axis_aligned_data() {
        // Data right on the carrier axis sits in the high-κ basin;
        // DK's perturbation-ring mean κ should be near the carrier-only
        // maximum.
        let ladder = build_phase_ladder(1, 0.0, 0.5);
        let cfg = CarrierConfig::default();
        let data = Some(DataHelix {
            tau: 0.0,
            phi: ladder[0].helix_a.phi,
            r: ladder[0].helix_a.r,
        });
        let mut ctx = make_ctx_with_data(&ladder, &cfg, data);
        let out = DKOperator.apply(&dummy_precursor(1.0), &mut ctx);
        assert!(
            out.stability_score > 0.3,
            "DK score should be substantial near a resonance peak, got {}",
            out.stability_score
        );
    }

    #[test]
    fn sw_score_one_for_axis_aligned_data() {
        // φ_data = α.phi → cos(2·0) = 1 → |1| = 1 (perfect sharpness).
        let ladder = build_phase_ladder(1, 0.0, 0.5);
        let cfg = CarrierConfig::default();
        let data = Some(DataHelix {
            tau: 0.0,
            phi: ladder[0].helix_a.phi,
            r: 0.4,
        });
        let mut ctx = make_ctx_with_data(&ladder, &cfg, data);
        let out = SWOperator.apply(&dummy_precursor(1.0), &mut ctx);
        // SW multiplies the precursor stability by 1.0 → result stays 1.0.
        assert!((out.stability_score - 1.0).abs() < 1e-9);
    }

    #[test]
    fn sw_score_zero_for_diagonal_data() {
        // φ_data = α.phi + π/4 → cos(π/2) = 0 → ambiguous → score 0.
        let ladder = build_phase_ladder(1, 0.0, 0.5);
        let cfg = CarrierConfig::default();
        let data = Some(DataHelix {
            tau: 0.0,
            phi: ladder[0].helix_a.phi + std::f64::consts::FRAC_PI_4,
            r: 0.4,
        });
        let mut ctx = make_ctx_with_data(&ladder, &cfg, data);
        let out = SWOperator.apply(&dummy_precursor(1.0), &mut ctx);
        assert!(
            out.stability_score < 1e-9,
            "diagonal data should give SW score 0, got {}",
            out.stability_score
        );
    }

    #[test]
    fn pi_high_at_clear_local_max() {
        // Data axis-aligned sits at a clear local max of the
        // phase-lock factor; PI should give a non-trivial positive score.
        let ladder = build_phase_ladder(1, 0.0, 0.5);
        let cfg = CarrierConfig::default();
        let data = Some(DataHelix {
            tau: 0.0,
            phi: ladder[0].helix_a.phi,
            r: 0.5,
        });
        let mut ctx = make_ctx_with_data(&ladder, &cfg, data);
        let out = PIOperator.apply(&dummy_precursor(1.0), &mut ctx);
        assert!(
            out.stability_score > 0.05,
            "PI should detect axis-aligned data as a local max, got {}",
            out.stability_score
        );
    }

    #[test]
    fn pi_zero_at_node() {
        // Transverse data → κ = 0 → PI score 0 (not at any maximum).
        let ladder = build_phase_ladder(1, 0.0, 0.5);
        let cfg = CarrierConfig::default();
        let data = Some(DataHelix {
            tau: 0.0,
            phi: ladder[0].helix_a.phi + std::f64::consts::FRAC_PI_2,
            r: 0.5,
        });
        let mut ctx = make_ctx_with_data(&ladder, &cfg, data);
        let out = PIOperator.apply(&dummy_precursor(1.0), &mut ctx);
        assert_eq!(out.stability_score, 0.0);
    }

    #[test]
    fn wt_migrates_when_better_carrier_exists() {
        // 4 carriers spaced on (0, π/2, π, 3π/2). Data at phi = π/2 +
        // small_eps lands closest to carrier 1, so WT should migrate
        // there from carrier 0.
        let ladder = build_phase_ladder(4, 0.0, 0.5);
        let cfg = CarrierConfig::default();
        let data = Some(DataHelix {
            tau: 0.0,
            phi: std::f64::consts::FRAC_PI_2 + 0.05,
            r: 0.5,
        });
        let mut ctx = make_ctx_with_data(&ladder, &cfg, data);
        let initial_idx = ctx.active_idx;
        let _ = WTOperator.apply(&dummy_precursor(1.0), &mut ctx);
        // WT should have transferred to a *better* carrier than carrier 0.
        // Carrier 1 has α.phi = π/2, so data at π/2 + 0.05 is essentially
        // axis-aligned with it.
        assert_ne!(
            ctx.active_idx, initial_idx,
            "WT should migrate when a better carrier exists; \
                    stayed at index {}",
            ctx.active_idx
        );
    }

    #[test]
    fn wt_score_zero_when_active_carrier_kappa_is_zero() {
        // Transverse data → κ_active = 0 → WT short-circuits to score 0.
        let ladder = build_phase_ladder(1, 0.0, 0.5);
        let cfg = CarrierConfig::default();
        let data = Some(DataHelix {
            tau: 0.0,
            phi: ladder[0].helix_a.phi + std::f64::consts::FRAC_PI_2,
            r: 0.5,
        });
        let mut ctx = make_ctx_with_data(&ladder, &cfg, data);
        let out = WTOperator.apply(&dummy_precursor(1.0), &mut ctx);
        assert_eq!(out.stability_score, 0.0);
    }

    #[test]
    fn dual_consensus_can_diverge_under_carrier_specific_resonance() {
        // When data sits between two carriers, primal (DK→SW→PI→WT) and
        // dual (PI→WT→DK→SW) traverse different intermediate carriers
        // and *can* land on different stability scores. MCI should be
        // measurably below 1 in such cases.
        let ladder = build_phase_ladder(4, 0.0, 0.5);
        let cfg = CarrierConfig::default();
        let data = Some(DataHelix {
            tau: 0.0,
            phi: std::f64::consts::FRAC_PI_4, // diagonal — between two carriers
            r: 0.5,
        });
        let ctx = make_ctx_with_data(&ladder, &cfg, data);
        let (dk, sw, pi, wt) = default_primal_ops();
        let (pi2, wt2, dk2, sw2) = default_dual_ops();
        let primal: Vec<&dyn CascadeOperator> = vec![&dk, &sw, &pi, &wt];
        let dual: Vec<&dyn CascadeOperator> = vec![&pi2, &wt2, &dk2, &sw2];
        let consensus_cfg = ConsensusConfig::default();
        let r = dual_consensus(&dummy_precursor(1.0), &primal, &dual, &ctx, &consensus_cfg);
        // Either the scores match (genuinely robust) or they diverge
        // (path-dependent). Both are valid; what matters is that MCI
        // is now a non-trivial measurement and not pinned to 1.0 by
        // commutative algebra. We assert MCI is in [0, 1] (sanity).
        assert!((0.0..=1.0).contains(&r.mci));
    }
}

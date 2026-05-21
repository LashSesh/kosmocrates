//! Clean entry point for the PSE full-stack engine.
//!
//! [`EngineFilter`] abstracts over [`GlobalState`] + [`Config`] + [`macro_step`]
//! so callers don't need to construct internal types. It is the `pse-core`
//! analogue of `pse_traverse::cognition::filter::CognitionFilter`.
//!
//! # Schnellstart
//!
//! ```rust
//! use pse_core::engine_filter::EngineFilter;
//!
//! let mut filter = EngineFilter::new();
//! let obs: Vec<Vec<u8>> = vec![b"hello world".to_vec()];
//! let result = filter.step(&obs).unwrap();
//! println!("tick={} committed={}", result.tick, result.committed);
//! ```

use pse_graph::PassthroughAdapter;
use pse_types::{Config, GateSnapshot, SemanticCrystal};

use crate::{macro_step, GlobalState, Result};

/// Session-scoped engine filter.
///
/// Create once, call [`step`](EngineFilter::step) once per observation batch.
/// The internal [`GlobalState`] accumulates carrier geometry, consensus
/// history, and pattern memory across ticks — the engine needs multiple ticks
/// to converge before a crystal is emitted.
pub struct EngineFilter {
    state: GlobalState,
    config: Config,
    adapter: PassthroughAdapter,
    ticks: u64,
    commits: u64,
}

/// Result of one [`EngineFilter::step`] call.
pub struct StepResult {
    /// `true` when a `SemanticCrystal` was committed on this tick.
    pub committed: bool,
    /// The crystal, if committed.
    pub crystal: Option<SemanticCrystal>,
    /// Gate snapshot for this tick (populated unconditionally after tick 1).
    pub gate: Option<GateSnapshot>,
    /// Monotonic tick counter (1-based).
    pub tick: u64,
}

impl EngineFilter {
    /// Default filter: uses [`Config::default()`].
    pub fn new() -> Self {
        Self::with_config(Config::default())
    }

    /// Filter with a custom config.
    pub fn with_config(config: Config) -> Self {
        let state = GlobalState::new(&config);
        Self {
            state,
            config,
            adapter: PassthroughAdapter::new("engine-filter"),
            ticks: 0,
            commits: 0,
        }
    }

    /// Feed one observation batch and advance the engine by one tick.
    pub fn step(&mut self, obs: &[Vec<u8>]) -> Result<StepResult> {
        self.ticks += 1;
        let crystal = macro_step(&mut self.state, obs, &self.config, &self.adapter)?;
        if crystal.is_some() {
            self.commits += 1;
        }
        let gate = self.state.last_gate.clone();
        Ok(StepResult {
            committed: crystal.is_some(),
            crystal,
            gate,
            tick: self.ticks,
        })
    }

    /// Fraction of ticks that committed a crystal. `0.0` before any ticks.
    pub fn commit_rate(&self) -> f64 {
        if self.ticks == 0 {
            0.0
        } else {
            self.commits as f64 / self.ticks as f64
        }
    }

    /// Total ticks driven through this filter.
    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    /// Total crystals committed by this filter.
    pub fn commits(&self) -> u64 {
        self.commits
    }
}

impl Default for EngineFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(v: f64, tick: usize) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "entity": "sensor_0",
            "value": v,
            "tick": tick,
        }))
        .unwrap()
    }

    #[test]
    fn step_returns_monotonic_tick() {
        let mut f = EngineFilter::new();
        for i in 1..=5 {
            let r = f.step(&[obs(0.5, i)]).unwrap();
            assert_eq!(r.tick, i as u64);
        }
    }

    #[test]
    fn commit_rate_is_zero_before_any_ticks() {
        assert_eq!(EngineFilter::new().commit_rate(), 0.0);
    }

    #[test]
    fn commit_rate_bounded_0_to_1() {
        let mut f = EngineFilter::new();
        for i in 0..20 {
            f.step(&[obs((i as f64 * 0.1).sin(), i)]).unwrap();
        }
        let rate = f.commit_rate();
        assert!((0.0..=1.0).contains(&rate), "commit_rate={rate}");
    }

    #[test]
    fn engine_filter_is_deterministic() {
        fn run_n(n: usize) -> u64 {
            let mut f = EngineFilter::new();
            for i in 0..n {
                f.step(&[obs((i as f64 * 0.1).sin(), i)]).unwrap();
            }
            f.commits()
        }
        assert_eq!(
            run_n(30),
            run_n(30),
            "engine must be byte-identical across runs"
        );
    }
}

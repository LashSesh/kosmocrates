//! Search autopilot — lightweight phase-based state machine for orchestrating
//! multi-cycle blueprint search.
//!
//! The autopilot tracks how many evaluations have been run, gates passes, and
//! the current phase.  It advances through phases automatically as evaluation
//! thresholds are crossed.  No heavy computation lives here.
//!
//! Implements spec PSE-TRAVERSE-SIGNATURE-01 §13.

use serde::{Deserialize, Serialize};

// ──────────────────────────────────────────────────────────────────────────────
// AutopilotPhase
// ──────────────────────────────────────────────────────────────────────────────

/// Discrete phase of the autopilot state machine.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AutopilotPhase {
    /// Broad search — explore many distinct blueprints.
    Exploration,
    /// Narrow search — exploit promising regions of the blueprint space.
    Exploitation,
    /// Fine-tune the best blueprints found so far.
    Refinement,
    /// Confirm the best blueprint on held-out or replayed problems.
    Validation,
    /// All cycles / evaluations exhausted, or a stop condition was met.
    Complete,
}

// ──────────────────────────────────────────────────────────────────────────────
// AutopilotConfig
// ──────────────────────────────────────────────────────────────────────────────

/// Static configuration for the autopilot.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AutopilotConfig {
    /// Hard upper bound on the number of phase-advance cycles.
    pub max_cycles: u32,
    /// Hard upper bound on the total number of blueprint evaluations.
    pub max_evaluations: u32,
    /// Fraction of `max_evaluations` to spend in Exploration before advancing.
    /// Also used as the step size for subsequent phase thresholds.
    ///
    /// Thresholds (in evaluations):
    /// - Exploration → Exploitation: `exploration_ratio * max_evaluations`
    /// - Exploitation → Refinement:  `2 * exploration_ratio * max_evaluations`
    /// - Refinement → Validation:    `3 * exploration_ratio * max_evaluations`
    /// - Validation → Complete:      `max_evaluations`
    pub exploration_ratio: f64,
    /// Fraction of evaluations that must have passed the gate for the run to
    /// be considered satisfactory.  Informational; the autopilot does not
    /// currently terminate early based on this value.
    pub target_gate_pass_rate: f64,
    /// If true, each blueprint must produce an identical result on replay.
    pub require_replay_identity: bool,
    /// If true, every evaluation must pass the signature gate.
    pub require_signature_gate: bool,
    /// RNG seed passed down to [`BlueprintSearch`].
    pub seed: u64,
}

impl Default for AutopilotConfig {
    fn default() -> Self {
        Self {
            max_cycles: 10,
            max_evaluations: 100,
            exploration_ratio: 0.4,
            target_gate_pass_rate: 0.5,
            require_replay_identity: true,
            require_signature_gate: false,
            seed: 0,
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// AutopilotState
// ──────────────────────────────────────────────────────────────────────────────

/// Mutable runtime state of the autopilot.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AutopilotState {
    /// Current phase.
    pub phase: AutopilotPhase,
    /// Number of times the phase has advanced (starts at 0).
    pub cycle: u32,
    /// Total number of evaluations recorded so far.
    pub evaluations: u32,
    /// Number of those evaluations that passed the gate.
    pub gate_passes: u32,
    /// Set when a stop condition is met.
    pub stop_reason: Option<String>,
}

// ──────────────────────────────────────────────────────────────────────────────
// SearchAutopilot
// ──────────────────────────────────────────────────────────────────────────────

/// Phase-based orchestrator for multi-cycle blueprint search.
pub struct SearchAutopilot {
    pub config: AutopilotConfig,
    pub state: AutopilotState,
}

impl SearchAutopilot {
    /// Create a new autopilot starting in the [`AutopilotPhase::Exploration`]
    /// phase.
    pub fn new(config: AutopilotConfig) -> Self {
        Self {
            state: AutopilotState {
                phase: AutopilotPhase::Exploration,
                cycle: 0,
                evaluations: 0,
                gate_passes: 0,
                stop_reason: None,
            },
            config,
        }
    }

    /// Returns `true` if any stop condition is satisfied:
    /// 1. `state.cycle >= config.max_cycles`
    /// 2. `state.evaluations >= config.max_evaluations`
    /// 3. `phase == Complete`
    /// 4. `stop_reason.is_some()`
    pub fn should_stop(&self) -> bool {
        self.state.cycle >= self.config.max_cycles
            || self.state.evaluations >= self.config.max_evaluations
            || self.state.phase == AutopilotPhase::Complete
            || self.state.stop_reason.is_some()
    }

    /// Advance to the next phase in order:
    /// Exploration → Exploitation → Refinement → Validation → Complete.
    ///
    /// Sets `stop_reason` when reaching `Complete`.
    pub fn next_phase(&mut self) {
        self.state.phase = match self.state.phase {
            AutopilotPhase::Exploration => AutopilotPhase::Exploitation,
            AutopilotPhase::Exploitation => AutopilotPhase::Refinement,
            AutopilotPhase::Refinement => AutopilotPhase::Validation,
            AutopilotPhase::Validation => AutopilotPhase::Complete,
            AutopilotPhase::Complete => AutopilotPhase::Complete,
        };
        self.state.cycle += 1;

        if self.state.phase == AutopilotPhase::Complete
            && self.state.stop_reason.is_none()
        {
            self.state.stop_reason = Some("all phases complete".into());
        }
    }

    /// Record one evaluation result and advance the phase if a threshold is
    /// crossed.
    ///
    /// Phase thresholds (in total evaluations):
    /// - `t1 = floor(exploration_ratio × max_evaluations)` → move to Exploitation
    /// - `t2 = floor(2 × exploration_ratio × max_evaluations)` → move to Refinement
    /// - `t3 = floor(3 × exploration_ratio × max_evaluations)` → move to Validation
    /// - `t4 = max_evaluations` → move to Complete
    pub fn record_evaluation(&mut self, gate_passed: bool) {
        self.state.evaluations += 1;
        if gate_passed {
            self.state.gate_passes += 1;
        }

        // Compute phase thresholds.
        let max = self.config.max_evaluations as f64;
        let r = self.config.exploration_ratio;
        let t1 = (r * max).floor() as u32;
        let t2 = (2.0 * r * max).floor() as u32;
        let t3 = (3.0 * r * max).floor() as u32;
        let t4 = self.config.max_evaluations;

        let ev = self.state.evaluations;

        // Check if the current phase should advance.
        let should_advance = match self.state.phase {
            AutopilotPhase::Exploration => ev >= t1,
            AutopilotPhase::Exploitation => ev >= t2,
            AutopilotPhase::Refinement => ev >= t3,
            AutopilotPhase::Validation => ev >= t4,
            AutopilotPhase::Complete => false,
        };

        if should_advance {
            self.next_phase();
        }
    }

    /// Return a reference to the current phase.
    pub fn current_phase(&self) -> &AutopilotPhase {
        &self.state.phase
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autopilot_starts_in_exploration() {
        let ap = SearchAutopilot::new(AutopilotConfig::default());
        assert_eq!(*ap.current_phase(), AutopilotPhase::Exploration);
        assert_eq!(ap.state.cycle, 0);
        assert_eq!(ap.state.evaluations, 0);
        assert_eq!(ap.state.gate_passes, 0);
        assert!(ap.state.stop_reason.is_none());
    }

    #[test]
    fn autopilot_advances_through_all_phases() {
        let config = AutopilotConfig {
            max_cycles: 10,
            max_evaluations: 100,
            ..AutopilotConfig::default()
        };
        let mut ap = SearchAutopilot::new(config);
        ap.next_phase();
        assert_eq!(*ap.current_phase(), AutopilotPhase::Exploitation);
        ap.next_phase();
        assert_eq!(*ap.current_phase(), AutopilotPhase::Refinement);
        ap.next_phase();
        assert_eq!(*ap.current_phase(), AutopilotPhase::Validation);
        ap.next_phase();
        assert_eq!(*ap.current_phase(), AutopilotPhase::Complete);
        assert!(ap.state.stop_reason.is_some());
    }

    #[test]
    fn autopilot_should_stop_when_complete() {
        let mut ap = SearchAutopilot::new(AutopilotConfig::default());
        assert!(!ap.should_stop());
        ap.next_phase(); // Exploitation
        ap.next_phase(); // Refinement
        ap.next_phase(); // Validation
        ap.next_phase(); // Complete
        assert!(ap.should_stop());
    }

    #[test]
    fn autopilot_should_stop_when_max_evaluations_reached() {
        let config = AutopilotConfig {
            max_evaluations: 3,
            max_cycles: 100,
            exploration_ratio: 0.4,
            ..AutopilotConfig::default()
        };
        let mut ap = SearchAutopilot::new(config);
        assert!(!ap.should_stop());
        ap.state.evaluations = 3;
        assert!(ap.should_stop());
    }

    #[test]
    fn autopilot_record_evaluation_tracks_gate_passes() {
        let mut ap = SearchAutopilot::new(AutopilotConfig {
            max_evaluations: 1000,
            max_cycles: 100,
            exploration_ratio: 0.4,
            ..AutopilotConfig::default()
        });
        ap.record_evaluation(true);
        ap.record_evaluation(false);
        ap.record_evaluation(true);
        assert_eq!(ap.state.evaluations, 3);
        assert_eq!(ap.state.gate_passes, 2);
    }

    #[test]
    fn autopilot_phase_advances_at_threshold() {
        // max_evaluations=10, exploration_ratio=0.4 → t1=4
        let config = AutopilotConfig {
            max_evaluations: 10,
            max_cycles: 100,
            exploration_ratio: 0.4,
            ..AutopilotConfig::default()
        };
        let mut ap = SearchAutopilot::new(config);
        assert_eq!(*ap.current_phase(), AutopilotPhase::Exploration);

        // 3 evaluations: still Exploration
        for _ in 0..3 {
            ap.record_evaluation(false);
        }
        assert_eq!(*ap.current_phase(), AutopilotPhase::Exploration);

        // 4th evaluation crosses t1=4 → advance to Exploitation
        ap.record_evaluation(false);
        assert_eq!(*ap.current_phase(), AutopilotPhase::Exploitation);
    }
}

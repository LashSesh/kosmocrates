//! NCTCS metric identifiers for the Eval-Matrix integration (§9.1).
//!
//! These constants can be registered as MetricSpec entries in pse-eval-matrix.

/// Numeric mapping of NCTCS conformance class to score.
pub const NCTCS_CONFORMANCE_CLASS_SCORE: &str = "nctcs_conformance_class_score";

/// Fraction of candidates whose visibility condition was provably met.
pub const NCTCS_VISIBILITY_CANDIDATE_COMPLIANCE: &str = "nctcs_visibility_candidate_compliance";

/// Fraction of ephemeral events without direct tensor mutation.
pub const NCTCS_NO_DIRECT_PERSISTENCE_RATE: &str = "nctcs_no_direct_persistence_rate";

/// Fraction of tensor revisions with fully passed gate path.
pub const NCTCS_GATE_BOUND_REVISION_RATE: &str = "nctcs_gate_bound_revision_rate";

/// Fraction of persistent artifacts with Trace, Evidence, GateHistory and ReplayManifest.
pub const NCTCS_TRACE_REPLAY_CONTRACT_RATE: &str = "nctcs_trace_replay_contract_rate";

/// 1 if MacroControlState is derived from tensor history, not from resonance/fabric.
pub const NCTCS_MACRO_STATE_VALIDITY: &str = "nctcs_macro_state_validity";

/// Fraction of reports that do not classify coherence as truth.
pub const NCTCS_COHERENCE_TRUTH_SEPARATION_RATE: &str = "nctcs_coherence_truth_separation_rate";

/// 1 if Complete/Empirical status was only given with real domain validation.
pub const NCTCS_DOMAIN_VALIDATION_REQUIRED_COMPLIANCE: &str =
    "nctcs_domain_validation_required_compliance";

/// All NCTCS metric IDs in canonical order.
pub const ALL_NCTCS_METRIC_IDS: &[&str] = &[
    NCTCS_CONFORMANCE_CLASS_SCORE,
    NCTCS_VISIBILITY_CANDIDATE_COMPLIANCE,
    NCTCS_NO_DIRECT_PERSISTENCE_RATE,
    NCTCS_GATE_BOUND_REVISION_RATE,
    NCTCS_TRACE_REPLAY_CONTRACT_RATE,
    NCTCS_MACRO_STATE_VALIDITY,
    NCTCS_COHERENCE_TRUTH_SEPARATION_RATE,
    NCTCS_DOMAIN_VALIDATION_REQUIRED_COMPLIANCE,
];

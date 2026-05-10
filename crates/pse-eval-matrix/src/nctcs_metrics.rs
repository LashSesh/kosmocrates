//! NCTCS metric specs for the Eval-Matrix (PSE-NCTCS-CONFORMANCE-01 §9.1).

use crate::metrics::{AggregationPolicy, MetricDirection, MetricFamily, MetricSpec};

fn nctcs_spec(id: &str, _description: &str) -> MetricSpec {
    MetricSpec {
        metric_id: id.into(),
        family: MetricFamily::PostSymbolicStructural,
        direction: MetricDirection::HigherIsBetter,
        primary: false,
        aggregation: AggregationPolicy::PassThrough,
        invalidation_rules: vec![],
    }
}

/// Numeric mapping: C0=0.25, C1=0.40, C2=0.60, C3=0.80, C4=1.00.
pub const NCTCS_CONFORMANCE_CLASS_SCORE: &str = "nctcs_conformance_class_score";
/// Fraction of candidates with provably met phase-visibility condition.
pub const NCTCS_VISIBILITY_CANDIDATE_COMPLIANCE: &str = "nctcs_visibility_candidate_compliance";
/// Fraction of ephemeral events without direct tensor mutation.
pub const NCTCS_NO_DIRECT_PERSISTENCE_RATE: &str = "nctcs_no_direct_persistence_rate";
/// Fraction of tensor revisions with fully passed gate path.
pub const NCTCS_GATE_BOUND_REVISION_RATE: &str = "nctcs_gate_bound_revision_rate";
/// Fraction of persistent artifacts with Trace + Evidence + GateHistory + ReplayManifest.
pub const NCTCS_TRACE_REPLAY_CONTRACT_RATE: &str = "nctcs_trace_replay_contract_rate";
/// 1 if MacroControlState is derived from tensor history (not resonance/fabric).
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

/// Build all NCTCS metric specs in canonical order.
pub fn nctcs_metric_specs() -> Vec<MetricSpec> {
    ALL_NCTCS_METRIC_IDS
        .iter()
        .map(|id| nctcs_spec(id, id))
        .collect()
}

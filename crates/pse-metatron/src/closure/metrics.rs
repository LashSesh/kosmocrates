//! Metatron closure metric identifiers for Eval-Matrix integration (§8.3).

/// Aggregated coherence from MacroControlState, Monolith projections and FieldTensor.
pub const GLOBAL_COHERENCE_SCORE: &str = "metatron_global_coherence_score";

/// Compatibility of local operatorik with global operatorik.
pub const ISOMORPHIC_PROJECTION_SCORE: &str = "metatron_isomorphic_projection_score";

/// Change in spectral gap after stitches.
pub const SPECTRAL_GAP_DELTA: &str = "metatron_spectral_gap_delta";

/// Fraction of local projections accepted as valid.
pub const MONOLITH_PROJECTION_RATE: &str = "metatron_monolith_projection_rate";

/// Fraction of runs with a passing MetatronGate.
pub const HOLISTIC_CLOSURE_PASS_RATE: &str = "metatron_holistic_closure_pass_rate";

/// Unexplained drift not accounted for by Trace/Evidence.
pub const UNEXPLAINED_DRIFT_SCORE: &str = "metatron_unexplained_drift_score";

/// Proof that coherence was not used as sole truth indicator.
pub const COHERENCE_TRUTH_SEPARATION: &str = "metatron_coherence_truth_separation";

/// All Metatron metric IDs in canonical order.
pub const ALL_METATRON_METRIC_IDS: &[&str] = &[
    GLOBAL_COHERENCE_SCORE,
    ISOMORPHIC_PROJECTION_SCORE,
    SPECTRAL_GAP_DELTA,
    MONOLITH_PROJECTION_RATE,
    HOLISTIC_CLOSURE_PASS_RATE,
    UNEXPLAINED_DRIFT_SCORE,
    COHERENCE_TRUTH_SEPARATION,
];

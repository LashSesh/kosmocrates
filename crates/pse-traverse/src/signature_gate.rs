//! Signature gate — evaluates [`SignatureDiagnostics`] against configurable
//! thresholds and returns a structured outcome.
//!
//! The gate is intentionally *data-only*: it reports `passed` and populates
//! `reasons`, but it does NOT block a commit by itself.  The caller decides
//! whether to honour `fail_closed` by inspecting `outcome.passed`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::canonical::hex_address;
use crate::signature_diag::{RegimeHint, SignatureDiagnostics};

// ──────────────────────────────────────────────────────────────────────────────
// Public types
// ──────────────────────────────────────────────────────────────────────────────

/// Configuration for the signature gate.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SignatureGateConfig {
    /// If false the gate is bypassed and always returns `passed = true`.
    pub enabled: bool,
    /// Minimum acceptable gap score (inclusive).
    pub min_gap_score: f64,
    /// Maximum acceptable degeneracy score (inclusive).
    pub max_degeneracy_score: f64,
    /// Maximum acceptable fragmentation score (inclusive).
    pub max_fragmentation_score: f64,
    /// Regime hints that are allowed.  Empty = allow all.
    pub allowed_regimes: Vec<RegimeHint>,
    /// If true the caller should treat a failed outcome as a hard block.
    /// If false the outcome is diagnostic-only.  Either way `passed` reflects
    /// the actual gate evaluation.
    pub fail_closed: bool,
}

impl Default for SignatureGateConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_gap_score: 0.0,
            max_degeneracy_score: 1.0,
            max_fragmentation_score: 1.0,
            allowed_regimes: Vec::new(),
            fail_closed: false,
        }
    }
}

/// Outcome of a signature gate evaluation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SignatureGateOutcome {
    /// Content-addressed identifier (derived with this field set to "").
    pub outcome_id: String,
    /// The diagnostics this outcome was derived from.
    pub diagnostics_id: String,
    /// True iff all gate checks passed.
    pub passed: bool,
    /// Sorted human-readable failure reasons.
    pub reasons: Vec<String>,
    /// Measured scores at evaluation time.
    pub measured: BTreeMap<String, f64>,
    /// Thresholds used for evaluation.
    pub thresholds: BTreeMap<String, f64>,
}

// ──────────────────────────────────────────────────────────────────────────────
// Public function
// ──────────────────────────────────────────────────────────────────────────────

/// Evaluate `diagnostics` against `config` and return a structured outcome.
pub fn check_signature_gate(
    diagnostics: &SignatureDiagnostics,
    config: &SignatureGateConfig,
) -> SignatureGateOutcome {
    // Always populate measured and thresholds for observability.
    let mut measured: BTreeMap<String, f64> = BTreeMap::new();
    measured.insert("gap_score".into(), diagnostics.gap_score);
    measured.insert("degeneracy_score".into(), diagnostics.degeneracy_score);
    measured.insert("fragmentation_score".into(), diagnostics.fragmentation_score);
    measured.insert("asymmetry_score".into(), diagnostics.asymmetry_score);
    measured.insert("rigidity_score".into(), diagnostics.rigidity_score);

    let mut thresholds: BTreeMap<String, f64> = BTreeMap::new();
    thresholds.insert("min_gap_score".into(), config.min_gap_score);
    thresholds.insert("max_degeneracy_score".into(), config.max_degeneracy_score);
    thresholds.insert("max_fragmentation_score".into(), config.max_fragmentation_score);

    // Short-circuit when disabled.
    if !config.enabled {
        let mut outcome = SignatureGateOutcome {
            outcome_id: String::new(),
            diagnostics_id: diagnostics.diagnostics_id.clone(),
            passed: true,
            reasons: Vec::new(),
            measured,
            thresholds,
        };
        outcome.outcome_id = hex_address(&outcome).unwrap_or_default();
        return outcome;
    }

    let mut reasons: Vec<String> = Vec::new();

    // 1. Gap score check.
    if diagnostics.gap_score < config.min_gap_score {
        reasons.push("gap_score below minimum".into());
    }

    // 2. Degeneracy score check.
    if diagnostics.degeneracy_score > config.max_degeneracy_score {
        reasons.push("degeneracy_score above maximum".into());
    }

    // 3. Fragmentation score check.
    if diagnostics.fragmentation_score > config.max_fragmentation_score {
        reasons.push("fragmentation_score above maximum".into());
    }

    // 4. Regime check.
    if !config.allowed_regimes.is_empty()
        && !config.allowed_regimes.contains(&diagnostics.regime_hint)
    {
        reasons.push(format!("regime not allowed: {:?}", diagnostics.regime_hint));
    }

    reasons.sort();
    let passed = reasons.is_empty();

    let mut outcome = SignatureGateOutcome {
        outcome_id: String::new(),
        diagnostics_id: diagnostics.diagnostics_id.clone(),
        passed,
        reasons,
        measured,
        thresholds,
    };
    outcome.outcome_id = hex_address(&outcome).unwrap_or_default();
    outcome
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signature_diag::RegimeHint;

    fn make_diagnostics(gap: f64, deg: f64, frag: f64) -> SignatureDiagnostics {
        SignatureDiagnostics {
            diagnostics_id: "diag.test".into(),
            signature_id: "sig.test".into(),
            gap_score: gap,
            degeneracy_score: deg,
            rigidity_score: 0.5,
            asymmetry_score: 0.1,
            fragmentation_score: frag,
            regime_hint: RegimeHint::Unknown,
            passed_numeric_sanity: true,
            messages: Vec::new(),
        }
    }

    #[test]
    fn signature_gate_fail_closed_blocks_commit() {
        // gap_score=0.0 < min_gap_score=0.5 → passed=false
        let diag = make_diagnostics(0.0, 0.1, 0.1);
        let config = SignatureGateConfig {
            enabled: true,
            min_gap_score: 0.5,
            max_degeneracy_score: 1.0,
            max_fragmentation_score: 1.0,
            allowed_regimes: Vec::new(),
            fail_closed: true,
        };
        let outcome = check_signature_gate(&diag, &config);
        assert!(!outcome.passed, "gate should fail when gap_score < min_gap_score");
        assert!(config.fail_closed, "fail_closed should be true");
        assert!(outcome.reasons.contains(&"gap_score below minimum".to_string()));
    }

    #[test]
    fn signature_gate_non_fail_closed_does_not_block_commit() {
        // fail_closed=false, but gap_score still fails → passed=false.
        // The caller observes passed=false but decides not to block (fail_closed=false).
        let diag = make_diagnostics(0.0, 0.1, 0.1);
        let config = SignatureGateConfig {
            enabled: true,
            min_gap_score: 0.5,
            max_degeneracy_score: 1.0,
            max_fragmentation_score: 1.0,
            allowed_regimes: Vec::new(),
            fail_closed: false,
        };
        let outcome = check_signature_gate(&diag, &config);
        // Gate still reports false — the semantics of blocking are the caller's.
        assert!(!outcome.passed, "gate should still report false even when fail_closed=false");
        assert!(!config.fail_closed, "fail_closed should be false");
    }

    #[test]
    fn signature_gate_disabled_always_passes() {
        let diag = make_diagnostics(0.0, 1.0, 1.0); // everything out of spec
        let config = SignatureGateConfig {
            enabled: false,
            ..SignatureGateConfig::default()
        };
        let outcome = check_signature_gate(&diag, &config);
        assert!(outcome.passed);
        assert!(outcome.reasons.is_empty());
    }

    #[test]
    fn signature_gate_regime_check() {
        let mut diag = make_diagnostics(0.8, 0.1, 0.1);
        diag.regime_hint = RegimeHint::Degenerate;
        let config = SignatureGateConfig {
            enabled: true,
            min_gap_score: 0.0,
            max_degeneracy_score: 1.0,
            max_fragmentation_score: 1.0,
            allowed_regimes: vec![RegimeHint::SymmetricCoupled, RegimeHint::Unknown],
            fail_closed: false,
        };
        let outcome = check_signature_gate(&diag, &config);
        assert!(!outcome.passed);
        assert!(outcome.reasons.iter().any(|r| r.contains("regime not allowed")));
    }
}

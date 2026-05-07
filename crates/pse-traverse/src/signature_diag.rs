//! Signature diagnostics — score-based regime classification of a structural
//! operator's spectral signature.
//!
//! Implements spec §10.  All scores are in [0, 1].  The [`RegimeHint`] is a
//! rule-based classification; callers should treat it as an advisory label,
//! not a guarantee.

use serde::{Deserialize, Serialize};

use crate::canonical::hex_address;
use crate::operator::StructuralOperator;
use crate::signature::Signature;

// ──────────────────────────────────────────────────────────────────────────────
// Public types
// ──────────────────────────────────────────────────────────────────────────────

/// Advisory regime classification based on spectral diagnostics.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RegimeHint {
    Uncoupled,
    SymmetricCoupled,
    DirectedCoupled,
    Hierarchical,
    Degenerate,
    Unknown,
}

/// Thresholds used to classify diagnostics scores.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DiagnosticConfig {
    /// Fragmentation score above which the regime is Uncoupled.
    pub fragmentation_high: f64,
    /// Degeneracy score above which the regime is Degenerate.
    pub degeneracy_high: f64,
    /// Asymmetry score above which the regime is DirectedCoupled.
    pub asymmetry_high: f64,
    /// Asymmetry score below which (together with high rigidity) the regime
    /// is SymmetricCoupled.
    pub asymmetry_low: f64,
    /// Rigidity score above which (together with low asymmetry) the regime
    /// is SymmetricCoupled.
    pub rigidity_high: f64,
    /// Hierarchy indicator above which the regime is Hierarchical.
    pub hierarchy_high: f64,
    /// Reference gap (Δref) for the gap score formula.
    pub gap_ref: f64,
    /// Small epsilon to avoid division by zero.
    pub epsilon: f64,
}

impl Default for DiagnosticConfig {
    fn default() -> Self {
        Self {
            fragmentation_high: 0.5,
            degeneracy_high: 0.5,
            asymmetry_high: 0.3,
            asymmetry_low: 0.1,
            rigidity_high: 0.7,
            hierarchy_high: 0.6,
            gap_ref: 1.0,
            epsilon: 1e-9,
        }
    }
}

/// Full set of spectral diagnostic scores for a signature/operator pair.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SignatureDiagnostics {
    /// Content-addressed identifier (derived with this field set to "").
    pub diagnostics_id: String,
    /// The signature this was computed from.
    pub signature_id: String,
    /// Gap score ∈ [0, 1].  Higher = larger second positive gap.
    pub gap_score: f64,
    /// Degeneracy score ∈ [0, 1].  Higher = more repeated values.
    pub degeneracy_score: f64,
    /// Rigidity score ∈ [0, 1].  Higher = more uniform spacing.
    pub rigidity_score: f64,
    /// Asymmetry score ∈ [0, 1].  Higher = more directed edges dominate.
    pub asymmetry_score: f64,
    /// Fragmentation score ∈ [0, 1].  Higher = more disconnected components.
    pub fragmentation_score: f64,
    /// Advisory regime label.
    pub regime_hint: RegimeHint,
    /// True iff all five scores are in [0, 1] and signature.values is
    /// non-empty.
    pub passed_numeric_sanity: bool,
    /// Sorted diagnostic messages (e.g. clamping warnings).
    pub messages: Vec<String>,
}

// ──────────────────────────────────────────────────────────────────────────────
// Helper: clamp to [0, 1] with optional warning
// ──────────────────────────────────────────────────────────────────────────────

fn clamp01(v: f64, label: &str, messages: &mut Vec<String>) -> f64 {
    if v < 0.0 || v > 1.0 {
        messages.push(format!("{} score clamped to [0,1]", label));
        v.clamp(0.0, 1.0)
    } else {
        v
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Public function
// ──────────────────────────────────────────────────────────────────────────────

/// Compute the full set of spectral diagnostics for `signature` / `operator`.
pub fn compute_diagnostics(
    signature: &Signature,
    operator: &StructuralOperator,
    config: &DiagnosticConfig,
) -> SignatureDiagnostics {
    let mut messages: Vec<String> = Vec::new();
    let eps = config.epsilon;

    // ── Convenience: convert SignatureValue → f64 ─────────────────────────────
    let values_f64: Vec<f64> = signature.values.iter().map(|v| v.to_f64()).collect();
    let n = values_f64.len();

    // ── Fragmentation score ───────────────────────────────────────────────────
    // c (component proxy) = max(1, zero_multiplicity)
    let dim = signature.summary.dimension as f64;
    let c = (signature.summary.zero_multiplicity as f64).max(1.0);
    let n_adj = (dim - 1.0).max(1.0);
    let fragmentation_raw = (c - 1.0) / n_adj;
    let fragmentation_score = clamp01(fragmentation_raw.min(1.0), "fragmentation", &mut messages);

    // ── Asymmetry score ───────────────────────────────────────────────────────
    let sum_dir: f64 = operator
        .matrix_profile
        .directed_edges
        .iter()
        .map(|e| e.weight.to_f64().abs())
        .sum();
    let sum_sym: f64 = operator
        .matrix_profile
        .symmetric_edges
        .iter()
        .map(|e| e.weight.to_f64().abs())
        .sum();
    let asymmetry_raw = sum_dir / (sum_sym + sum_dir + eps);
    let asymmetry_score = clamp01(asymmetry_raw, "asymmetry", &mut messages);

    // ── Degeneracy score ──────────────────────────────────────────────────────
    // m_dup = total_count - unique_count
    let total_count = signature.values.len() as f64;
    let unique_count = {
        // Count unique values by converting to ordered comparable form.
        // We use a BTreeMap keyed by the bit-representation of the f64 to
        // avoid float-comparison issues with NaN (spec says no NaN/Inf).
        let mut seen: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
        for v in &values_f64 {
            seen.insert(v.to_bits());
        }
        seen.len() as f64
    };
    let m_dup = total_count - unique_count;
    let degeneracy_raw = m_dup / total_count.max(1.0);
    let degeneracy_score = clamp01(degeneracy_raw, "degeneracy", &mut messages);

    // ── Consecutive gaps ──────────────────────────────────────────────────────
    let gaps: Vec<f64> = if values_f64.len() >= 2 {
        values_f64.windows(2).map(|w| w[1] - w[0]).collect()
    } else {
        Vec::new()
    };

    // ── Gap score ─────────────────────────────────────────────────────────────
    // Δ2 = second positive gap; first if only one; 0 if none.
    let positive_gaps: Vec<f64> = gaps.iter().copied().filter(|&g| g > 0.0).collect();
    let delta2 = match positive_gaps.len() {
        0 => 0.0,
        1 => positive_gaps[0],
        _ => positive_gaps[1],
    };
    let gap_raw = (delta2 / (config.gap_ref + eps)).min(1.0);
    let gap_score = clamp01(gap_raw, "gap", &mut messages);

    // ── Rigidity score ────────────────────────────────────────────────────────
    let rigidity_score = if gaps.is_empty() {
        1.0
    } else {
        let mean_gap = gaps.iter().sum::<f64>() / gaps.len() as f64;
        let var_gap = gaps.iter().map(|g| (g - mean_gap).powi(2)).sum::<f64>() / gaps.len() as f64;
        let rigidity_raw = 1.0 - (var_gap / (mean_gap.powi(2) + eps)).min(1.0);
        clamp01(rigidity_raw, "rigidity", &mut messages)
    };

    // ── Hierarchy indicator ───────────────────────────────────────────────────
    let (min_val, max_val) = values_f64
        .iter()
        .copied()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(mn, mx), v| {
            (mn.min(v), mx.max(v))
        });
    let mean_spacing = if gaps.is_empty() {
        0.0
    } else {
        gaps.iter().sum::<f64>() / gaps.len() as f64
    };
    let hierarchy_raw = if n == 0 {
        0.0
    } else {
        (max_val - min_val) / (mean_spacing.abs() + eps)
    };
    let hierarchy_indicator = hierarchy_raw.clamp(0.0, 1.0);

    // ── Regime classification (spec §10.2 Listing 10) ─────────────────────────
    let regime_hint = if fragmentation_score >= config.fragmentation_high {
        RegimeHint::Uncoupled
    } else if degeneracy_score >= config.degeneracy_high {
        RegimeHint::Degenerate
    } else if asymmetry_score >= config.asymmetry_high {
        RegimeHint::DirectedCoupled
    } else if rigidity_score >= config.rigidity_high && asymmetry_score < config.asymmetry_low {
        RegimeHint::SymmetricCoupled
    } else if hierarchy_indicator >= config.hierarchy_high {
        RegimeHint::Hierarchical
    } else {
        RegimeHint::Unknown
    };

    // ── Numeric sanity ────────────────────────────────────────────────────────
    let passed_numeric_sanity = n > 0
        && [
            gap_score,
            degeneracy_score,
            rigidity_score,
            asymmetry_score,
            fragmentation_score,
        ]
        .iter()
        .all(|&s| (0.0..=1.0).contains(&s));

    // ── Sort messages ─────────────────────────────────────────────────────────
    messages.sort();

    // ── Content address ───────────────────────────────────────────────────────
    let mut diag = SignatureDiagnostics {
        diagnostics_id: String::new(),
        signature_id: signature.signature_id.clone(),
        gap_score,
        degeneracy_score,
        rigidity_score,
        asymmetry_score,
        fragmentation_score,
        regime_hint,
        passed_numeric_sanity,
        messages,
    };
    diag.diagnostics_id = hex_address(&diag).unwrap_or_default();
    diag
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use crate::operator::{
        MatrixProfile, OperatorKind, SignatureValue, StructuralOperator, WeightedDirectedEdge,
        WeightedEdge,
    };
    use crate::signature::{Signature, SignatureAlgorithm, SignatureSummary};

    /// Build a `StructuralOperator` with explicit symmetric and directed edge
    /// weights so tests can control asymmetry / fragmentation scores.
    fn make_operator(
        sym_weights: Vec<f64>,
        dir_weights: Vec<f64>,
        scale: i32,
    ) -> StructuralOperator {
        let n = (sym_weights.len() + dir_weights.len()).max(2);
        let diagonal: Vec<SignatureValue> = (0..n)
            .map(|_| SignatureValue::quantize(1.0, scale).unwrap())
            .collect();
        let symmetric_edges: Vec<WeightedEdge> = sym_weights
            .into_iter()
            .enumerate()
            .map(|(i, w)| WeightedEdge {
                u: 0,
                v: (i + 1).min(n - 1),
                weight: SignatureValue::quantize(w, scale)
                    .unwrap_or_else(|_| SignatureValue::zero(scale)),
            })
            .collect();
        let directed_edges: Vec<WeightedDirectedEdge> = dir_weights
            .into_iter()
            .enumerate()
            .map(|(i, w)| WeightedDirectedEdge {
                from: 0,
                to: (i + 1).min(n - 1),
                weight: SignatureValue::quantize(w, scale)
                    .unwrap_or_else(|_| SignatureValue::zero(scale)),
            })
            .collect();
        StructuralOperator {
            operator_id: "op.test".into(),
            substrate_id: "0".repeat(64),
            kind: OperatorKind::Laplacian,
            matrix_profile: MatrixProfile {
                n,
                diagonal,
                symmetric_edges,
                directed_edges,
                constraint_weights: vec![],
            },
            parameters: BTreeMap::new(),
            evidence_refs: vec![],
            operator_version: "v0.1".into(),
        }
    }

    fn make_signature(values_f64: Vec<f64>, zero_mult: usize, scale: i32) -> Signature {
        let vals: Vec<SignatureValue> = values_f64
            .iter()
            .map(|&v| {
                SignatureValue::quantize(v, scale).unwrap_or_else(|_| SignatureValue::zero(scale))
            })
            .collect();
        let dim = vals.len();
        let min_value = vals
            .first()
            .cloned()
            .unwrap_or_else(|| SignatureValue::zero(scale));
        let max_value = vals
            .last()
            .cloned()
            .unwrap_or_else(|| SignatureValue::zero(scale));
        Signature {
            signature_id: "sig.test".into(),
            operator_id: "op.test".into(),
            algorithm: SignatureAlgorithm::MatrixProfileApprox,
            values: vals,
            summary: SignatureSummary {
                dimension: dim,
                min_value,
                max_value,
                mean_value: SignatureValue::zero(scale),
                mean_spacing: SignatureValue::zero(scale),
                zero_multiplicity: zero_mult,
                gap_profile: vec![],
            },
            algorithm_version: "v0.1".into(),
        }
    }

    #[test]
    fn diagnostic_scores_in_unit_interval() {
        let scale = 6;
        let sig = make_signature(vec![0.0, 1.0, 2.0, 3.0], 1, scale);
        let op = make_operator(vec![1.0, 2.0], vec![0.5], scale);
        let cfg = DiagnosticConfig::default();
        let diag = compute_diagnostics(&sig, &op, &cfg);
        assert!(
            (0.0..=1.0).contains(&diag.gap_score),
            "gap_score={}",
            diag.gap_score
        );
        assert!(
            (0.0..=1.0).contains(&diag.degeneracy_score),
            "deg={}",
            diag.degeneracy_score
        );
        assert!(
            (0.0..=1.0).contains(&diag.rigidity_score),
            "rig={}",
            diag.rigidity_score
        );
        assert!(
            (0.0..=1.0).contains(&diag.asymmetry_score),
            "asym={}",
            diag.asymmetry_score
        );
        assert!(
            (0.0..=1.0).contains(&diag.fragmentation_score),
            "frag={}",
            diag.fragmentation_score
        );
    }

    #[test]
    fn regime_uncoupled_for_fragmented_graph() {
        // Many zeros → high zero_multiplicity → high fragmentation score.
        // dim=4, zero_mult=4 → c=4, n-1=3 → F=min(1,(4-1)/3)=1.0 ≥ 0.5.
        let scale = 6;
        let sig = make_signature(vec![0.0, 0.0, 0.0, 0.0], 4, scale);
        let op = make_operator(vec![0.1], vec![], scale);
        let cfg = DiagnosticConfig::default();
        let diag = compute_diagnostics(&sig, &op, &cfg);
        assert_eq!(diag.regime_hint, RegimeHint::Uncoupled);
    }

    #[test]
    fn regime_directed_for_asymmetric_graph() {
        // Heavily directed: sum_dir >> sum_sym → asymmetry_score ≥ 0.3.
        let scale = 6;
        let sig = make_signature(vec![1.0, 2.0, 3.0, 4.0], 0, scale);
        // sym=0.1 total, dir=10.0 total → asym = 10/(0.1+10) ≈ 0.99
        let op = make_operator(vec![0.1], vec![10.0], scale);
        let cfg = DiagnosticConfig::default();
        let diag = compute_diagnostics(&sig, &op, &cfg);
        assert_eq!(diag.regime_hint, RegimeHint::DirectedCoupled);
    }
}

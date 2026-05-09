//! Ablation ladder + summaries (§3.2, §6.5, §14.3).

use serde::{Deserialize, Serialize};

use crate::metrics::{MetricDirection, MetricSpec};
use crate::primitives::{fixed_sub, EvalError, Fixed};
use crate::reports::VariantSummary;
use crate::variants::{LayerMask, SystemVariantSpec, VariantLadder};

/// Mandatory ablation rungs per §3.2.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AblationRung {
    /// Without cognition.
    NoCognition,
    /// Without spiral memory.
    NoSpiralMemory,
    /// Without constraint lattice.
    NoConstraintLattice,
    /// Without phase panorama.
    NoPhasePanorama,
    /// Without governed wormholes.
    NoGovernedWormholes,
    /// Without self-model tensor.
    NoSelfModel,
    /// Without adaptive calibration.
    NoAdaptiveCalibration,
    /// Gate diagnostic-only vs. fail-closed.
    DiagnosticGatesOnly,
}

impl AblationRung {
    /// Bit to drop from the layer mask. (`DiagnosticGatesOnly` is
    /// modelled as no bit drop here — it is a feature-flag flip; a
    /// production ablation runner would wire it to the appropriate
    /// solver/gate flag.)
    pub fn bit_to_drop(self) -> u32 {
        match self {
            AblationRung::NoCognition => LayerMask::COGNITION,
            AblationRung::NoSpiralMemory => LayerMask::SPIRAL_MEMORY,
            AblationRung::NoConstraintLattice => LayerMask::CONSTRAINT_LATTICE,
            AblationRung::NoPhasePanorama => LayerMask::PHASE_PANORAMA,
            AblationRung::NoGovernedWormholes => LayerMask::GOVERNED_WORMHOLES,
            AblationRung::NoSelfModel => LayerMask::SELF_MODEL,
            AblationRung::NoAdaptiveCalibration => LayerMask::ADAPTIVE_CALIBRATION,
            AblationRung::DiagnosticGatesOnly => 0,
        }
    }

    /// Stable tag used in variant ids.
    pub fn tag(self) -> &'static str {
        match self {
            AblationRung::NoCognition => "noCognition",
            AblationRung::NoSpiralMemory => "noSpiralMemory",
            AblationRung::NoConstraintLattice => "noConstraintLattice",
            AblationRung::NoPhasePanorama => "noPhasePanorama",
            AblationRung::NoGovernedWormholes => "noGovernedWormholes",
            AblationRung::NoSelfModel => "noSelfModel",
            AblationRung::NoAdaptiveCalibration => "noAdaptiveCalibration",
            AblationRung::DiagnosticGatesOnly => "diagnosticGatesOnly",
        }
    }
}

/// `AblationLadder` — base + ablations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AblationLadder {
    /// Base variant the ablations are derived from.
    pub base: SystemVariantSpec,
    /// Ablation variants (one per rung).
    pub variants: Vec<SystemVariantSpec>,
}

/// Per §3.2 — build the canonical ablation ladder around `base`.
pub fn build_ablation_ladder(base: SystemVariantSpec) -> Result<AblationLadder, EvalError> {
    let rungs = [
        AblationRung::NoCognition,
        AblationRung::NoSpiralMemory,
        AblationRung::NoConstraintLattice,
        AblationRung::NoPhasePanorama,
        AblationRung::NoGovernedWormholes,
        AblationRung::NoSelfModel,
        AblationRung::NoAdaptiveCalibration,
        AblationRung::DiagnosticGatesOnly,
    ];
    let mut variants = Vec::with_capacity(rungs.len());
    for rung in rungs.iter() {
        let mut ablated = base.clone();
        ablated.variant_id = format!("{}.{}", base.variant_id, rung.tag());
        ablated.layer_mask = ablated.layer_mask.without(rung.bit_to_drop());
        if matches!(rung, AblationRung::DiagnosticGatesOnly) {
            ablated
                .feature_flags
                .insert("diagnostic_gates_only".into(), true);
        }
        variants.push(ablated.refresh_hashes()?);
    }
    Ok(AblationLadder { base, variants })
}

/// Convenience wrapper exposing the canonical full B0…B7 ladder as
/// the input to ablation builders. Spec presets call this.
pub fn full_layer_ladder() -> Vec<SystemVariantSpec> {
    VariantLadder::full()
}

/// Per-metric delta between two variants.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MetricDelta {
    /// Metric id.
    pub metric_id: String,
    /// Direction (carried to keep the summary self-describing).
    pub direction: MetricDirection,
    /// Base variant value.
    pub base_value: Fixed,
    /// Ablated variant value.
    pub ablated_value: Fixed,
    /// `ablated − base` (signed; the consumer applies direction).
    pub signed_delta: Fixed,
    /// `true` iff the change is positive in the spec's
    /// direction-aware sense (improvement).
    pub direction_aware_improvement: bool,
}

/// `AblationConclusion` (§14.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AblationConclusion {
    /// The ablated layer was empirically useful (LMU > 0, no safety
    /// regression).
    LayerIsUseful,
    /// The ablated layer was neutral or negative.
    LayerIsRedundant,
    /// The ablation surfaced a safety regression.
    SafetyRegression,
    /// The ablation could not be evaluated (insufficient data).
    Inconclusive,
}

/// `AblationSummary` (§14.3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AblationSummary {
    /// Base variant id.
    pub base_variant: String,
    /// Ablated variant id.
    pub ablated_variant: String,
    /// Per-metric deltas.
    pub metric_deltas: Vec<MetricDelta>,
    /// Layer Marginal Utility (signed; on the primary task metric).
    pub layer_marginal_utility: Fixed,
    /// Whether any safety metric got worse.
    pub safety_regression: bool,
    /// Conclusion.
    pub conclusion: AblationConclusion,
}

/// Compute an `AblationSummary` from two variant summaries + the metric
/// specs.
pub fn summarize_ablation(
    base: &VariantSummary,
    ablated: &VariantSummary,
    metric_specs: &[MetricSpec],
) -> Result<AblationSummary, EvalError> {
    let mut deltas = Vec::new();
    let mut safety_regression = false;
    let mut primary_lmu = Fixed::Rational { num: 0, den: 1 };
    for ms in metric_specs {
        let base_v = base.metric_values.get(&ms.metric_id);
        let abl_v = ablated.metric_values.get(&ms.metric_id);
        if let (Some(b), Some(a)) = (base_v, abl_v) {
            let signed = fixed_sub(a, b);
            let improvement = match ms.direction {
                MetricDirection::HigherIsBetter => a.cmp(b) == std::cmp::Ordering::Greater,
                MetricDirection::LowerIsBetter => a.cmp(b) == std::cmp::Ordering::Less,
                MetricDirection::Mixed => false,
            };
            if ms.metric_id == "task_success" {
                primary_lmu = signed.clone();
            }
            // A safety regression is when a *primary* `LowerIsBetter`
            // safety metric got worse OR a `HigherIsBetter` one got
            // worse.
            let is_safety_metric = matches!(
                ms.metric_id.as_str(),
                "false_commit_rate" | "hold_correctness"
            );
            if is_safety_metric {
                let worse = match ms.direction {
                    MetricDirection::HigherIsBetter => a.cmp(b) == std::cmp::Ordering::Less,
                    MetricDirection::LowerIsBetter => a.cmp(b) == std::cmp::Ordering::Greater,
                    MetricDirection::Mixed => false,
                };
                if worse {
                    safety_regression = true;
                }
            }
            deltas.push(MetricDelta {
                metric_id: ms.metric_id.clone(),
                direction: ms.direction,
                base_value: b.clone(),
                ablated_value: a.clone(),
                signed_delta: signed,
                direction_aware_improvement: improvement,
            });
        }
    }
    let conclusion = if safety_regression {
        AblationConclusion::SafetyRegression
    } else if primary_lmu.cmp(&Fixed::Rational { num: 0, den: 1 }) == std::cmp::Ordering::Greater {
        // Ablated > base on task_success ⇒ removing the layer *helped*
        // ⇒ layer is redundant.
        AblationConclusion::LayerIsRedundant
    } else if primary_lmu.cmp(&Fixed::Rational { num: 0, den: 1 }) == std::cmp::Ordering::Less {
        // Ablated < base on task_success ⇒ layer is useful.
        AblationConclusion::LayerIsUseful
    } else {
        AblationConclusion::Inconclusive
    };
    Ok(AblationSummary {
        base_variant: base.variant_id.clone(),
        ablated_variant: ablated.variant_id.clone(),
        metric_deltas: deltas,
        layer_marginal_utility: primary_lmu,
        safety_regression,
        conclusion,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ladder_drops_each_rung_bit() {
        let base = SystemVariantSpec::full_stack();
        let l = build_ablation_ladder(base.clone()).unwrap();
        for v in &l.variants {
            assert!(v.variant_id.starts_with(&base.variant_id));
        }
        assert_eq!(l.variants.len(), 8);
    }

    #[test]
    fn no_cognition_drops_cognition_bit() {
        let base = SystemVariantSpec::cognition();
        let l = build_ablation_ladder(base).unwrap();
        let target = l
            .variants
            .iter()
            .find(|v| v.variant_id.ends_with(".noCognition"))
            .unwrap();
        assert!(!target.layer_mask.has(LayerMask::COGNITION));
    }
}

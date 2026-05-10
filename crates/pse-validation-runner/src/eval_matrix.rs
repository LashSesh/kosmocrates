//! Eval-Matrix preset runner (L2, §8.3).
//!
//! Calls `pse-eval-matrix` CLI for each configured preset and collects
//! results. Reports `MissingPreset` for topology-tpt-mtl if configured.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::parsers::OpenObligation;
use crate::primitives::{Hash256, ValidationError, ValidationFailureKind};

/// Result of one Eval-Matrix preset run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresetResult {
    pub preset_name: String,
    pub completed: bool,
    pub replay_passed: bool,
    pub summary_ref: Option<Hash256>,
    pub failure: Option<String>,
}

/// Aggregated Eval-Matrix results.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalMatrixSummary {
    pub preset_results: Vec<PresetResult>,
    pub missing_presets: Vec<String>,
    pub open_obligations: Vec<OpenObligation>,
    pub all_required_passed: bool,
}

/// Known presets that MUST run (§8.3).
pub const MANDATORY_PRESETS: &[&str] = &[
    "agent-cognition",
    "streaming-event-detection",
    "post-symbolic-ablation",
    "phase-matrix-substrate",
    "dual-fabric-stitch",
];

/// Presets that MAY exist; absence is reported as open obligation.
pub const OPTIONAL_PRESETS: &[&str] = &["topology-tpt-mtl"];

/// Assess which presets are present/missing from the configured list.
pub fn assess_presets(
    configured: &[String],
    report_missing_topology: bool,
) -> (Vec<String>, Vec<OpenObligation>) {
    let configured_set: std::collections::BTreeSet<&str> =
        configured.iter().map(String::as_str).collect();

    let mut missing = Vec::new();
    let mut obligations = Vec::new();

    // Check mandatory presets.
    for &preset in MANDATORY_PRESETS {
        if !configured_set.contains(preset) {
            missing.push(preset.to_string());
        }
    }

    // Check topology preset — report as open obligation, not failure.
    if report_missing_topology && !configured_set.contains("topology-tpt-mtl") {
        obligations.push(OpenObligation {
            obligation_id: "OBL-TPT-MTL-PRESET".into(),
            description: "topology-tpt-mtl eval preset not yet registered in pse-eval-matrix. \
                TPT-MTL metrics exist (tpt_*) but no preset runs them end-to-end."
                .into(),
            failure_kind: ValidationFailureKind::MissingPreset
                .recovery_instruction()
                .into(),
            recovery: "Add topology-tpt-mtl preset to pse-eval-matrix presets.rs.".into(),
        });
    }

    (missing, obligations)
}

/// Build an EvalMatrixSummary from per-preset exit codes and replay flags.
pub fn build_eval_summary(
    preset_exits: &BTreeMap<String, i32>,
    preset_replay_ok: &BTreeMap<String, bool>,
    missing_presets: Vec<String>,
    open_obligations: Vec<OpenObligation>,
) -> EvalMatrixSummary {
    let mut preset_results = Vec::new();
    let mut all_required_passed = missing_presets.is_empty();

    for (name, &exit) in preset_exits {
        let completed = exit == 0;
        let replay_passed = preset_replay_ok.get(name).copied().unwrap_or(false);
        if !completed || !replay_passed {
            if MANDATORY_PRESETS.contains(&name.as_str()) {
                all_required_passed = false;
            }
        }
        preset_results.push(PresetResult {
            preset_name: name.clone(),
            completed,
            replay_passed,
            summary_ref: None,
            failure: if !completed {
                Some(format!("exit code {exit}"))
            } else {
                None
            },
        });
    }

    // Sort for determinism.
    preset_results.sort_by(|a, b| a.preset_name.cmp(&b.preset_name));

    EvalMatrixSummary {
        preset_results,
        missing_presets,
        open_obligations,
        all_required_passed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_preset_yields_missing_preset_failure() {
        let configured = vec!["agent-cognition".to_string()]; // missing 4 mandatory
        let (missing, _) = assess_presets(&configured, false);
        assert!(missing.contains(&"streaming-event-detection".to_string()));
        assert!(missing.contains(&"post-symbolic-ablation".to_string()));
        assert!(missing.contains(&"phase-matrix-substrate".to_string()));
        assert!(missing.contains(&"dual-fabric-stitch".to_string()));
    }

    #[test]
    fn topology_preset_missing_yields_open_obligation() {
        let configured: Vec<String> = MANDATORY_PRESETS.iter().map(|s| s.to_string()).collect();
        let (missing, obligations) = assess_presets(&configured, true);
        assert!(missing.is_empty());
        assert_eq!(obligations.len(), 1);
        assert_eq!(obligations[0].obligation_id, "OBL-TPT-MTL-PRESET");
    }

    #[test]
    fn all_mandatory_presets_present_yields_no_missing() {
        let configured: Vec<String> = MANDATORY_PRESETS
            .iter()
            .map(|s| s.to_string())
            .chain(std::iter::once("topology-tpt-mtl".to_string()))
            .collect();
        let (missing, obligations) = assess_presets(&configured, true);
        assert!(missing.is_empty());
        assert!(obligations.is_empty());
    }
}

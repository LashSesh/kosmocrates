//! PhaseVisibilityAudit — verifies that candidate formation implies
//! phase-gated visibility (C1 condition).

use serde::{Deserialize, Serialize};

use super::primitives::{content_address, Fixed, Hash256, NctcsError, NctcsThresholds};

/// Audit of which cells are visible in the current phase window.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseVisibilityAudit {
    pub audit_id: Hash256,
    pub theta_t_hash: Hash256,
    pub visible_cell_refs: Vec<Hash256>,
    pub invisible_cell_refs: Vec<Hash256>,
    pub visibility_coverage: Fixed,
    pub passed: bool,
}

impl PhaseVisibilityAudit {
    pub fn build(
        theta_t_hash: Hash256,
        mut visible_cell_refs: Vec<Hash256>,
        mut invisible_cell_refs: Vec<Hash256>,
        thresholds: &NctcsThresholds,
    ) -> Result<Self, NctcsError> {
        // Deterministic sort before hashing (per policy).
        visible_cell_refs.sort();
        invisible_cell_refs.sort();

        let total = (visible_cell_refs.len() + invisible_cell_refs.len()) as i64;
        let visible = visible_cell_refs.len() as i64;
        let coverage = if total == 0 {
            pse_traverse::dynamic_state::CanonicalNumber::FixedI64 { raw: 100, scale: 2 }
        } else {
            pse_traverse::dynamic_state::CanonicalNumber::FixedI64 {
                raw: visible * 10000 / total,
                scale: 2,
            }
        };

        use pse_traverse::dynamic_state::CanonicalNumber;
        let min = match &thresholds.min_visibility_coverage {
            CanonicalNumber::FixedI64 { raw, scale } => *raw * 10i64.pow(2 - scale.min(&2)),
            _ => 0,
        };
        let cov = match &coverage {
            CanonicalNumber::FixedI64 { raw, .. } => *raw,
            _ => 0,
        };
        let passed = cov >= min;

        let partial = PhaseVisibilityAudit {
            audit_id: Hash256::zero(),
            theta_t_hash,
            visible_cell_refs,
            invisible_cell_refs,
            visibility_coverage: coverage,
            passed,
        };
        let id = content_address(&partial)?;
        Ok(PhaseVisibilityAudit {
            audit_id: id,
            ..partial
        })
    }
}

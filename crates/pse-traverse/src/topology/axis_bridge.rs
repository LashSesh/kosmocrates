//! AxisBridge — axis-origin separation and translation proofs (TPT-MTL §4.3).
//!
//! Invariant I-03: semantic axes, operative coordinates and carrier metadata
//! MUST be declared separately. Any translation between layers requires a
//! proof digest.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::phase_window::PhaseSpaceWindow;
use super::primitives::{tpt_content_address, Hash256, TopologyError};
use super::run_descriptor::TptMtlRunDescriptor;

/// Axis origin declaration for one axis label.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AxisOriginDecl {
    pub label: String,
    pub layer: AxisLayer,
    pub source_ref: Hash256,
}

/// Which axis layer owns a given coordinate.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AxisLayer {
    Semantic,
    Runtime,
    Carrier,
}

/// Report produced by `build_axis_bridge_report`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AxisBridgeReport {
    pub report_id: Hash256,
    pub semantic_digest: Hash256,
    pub runtime_digest: Hash256,
    pub carrier_digest: Hash256,
    pub translation_proofs: Vec<Hash256>,
    pub violations: Vec<String>,
    pub passed: bool,
}

/// Build and validate the AxisBridge for a PhaseSpaceWindow.
pub fn build_axis_bridge_report(
    window: &PhaseSpaceWindow,
    rd: &TptMtlRunDescriptor,
) -> Result<AxisBridgeReport, TopologyError> {
    let mut violations = Vec::new();

    // Verify semantic axes are declared and non-empty.
    let semantic_labels: BTreeSet<&str> = rd
        .axis_policy
        .semantic_axes
        .iter()
        .map(|s| s.as_str())
        .collect();
    if semantic_labels.is_empty() {
        violations.push("semantic_axes is empty".into());
    }

    // Verify runtime axes are declared.
    let runtime_labels: BTreeSet<&str> = rd
        .axis_policy
        .runtime_axes
        .iter()
        .map(|s| s.as_str())
        .collect();
    if runtime_labels.is_empty() {
        violations.push("runtime_axes is empty".into());
    }

    // Verify no semantic axis label appears in runtime axes (I-03: no mixing).
    let overlap: Vec<&str> = semantic_labels
        .intersection(&runtime_labels)
        .copied()
        .collect();
    if !overlap.is_empty() {
        violations.push(format!(
            "axis label overlap between semantic and runtime layers: {:?}",
            overlap
        ));
    }

    // Verify carrier axes don't overlap with semantic axes.
    let carrier_labels: BTreeSet<&str> = rd
        .axis_policy
        .carrier_axes
        .iter()
        .map(|s| s.as_str())
        .collect();
    let carrier_semantic_overlap: Vec<&str> = semantic_labels
        .intersection(&carrier_labels)
        .copied()
        .collect();
    if !carrier_semantic_overlap.is_empty() {
        violations.push(format!(
            "axis label overlap between semantic and carrier layers: {:?}",
            carrier_semantic_overlap
        ));
    }

    // Verify all TptPoints in the window carry correct axis labels.
    for point in &window.points {
        let point_sem: BTreeSet<&str> = point
            .meta
            .semantic_axis_labels
            .iter()
            .map(|s| s.as_str())
            .collect();
        let point_rt: BTreeSet<&str> = point
            .meta
            .runtime_axis_labels
            .iter()
            .map(|s| s.as_str())
            .collect();

        // Check point semantic labels match policy.
        if point_sem != semantic_labels {
            violations.push(format!(
                "point {:?} semantic axis labels {:?} don't match policy {:?}",
                point.point_id.hex(),
                point_sem,
                semantic_labels
            ));
        }

        // Check point runtime labels match policy.
        if point_rt != runtime_labels {
            violations.push(format!(
                "point {:?} runtime axis labels {:?} don't match policy {:?}",
                point.point_id.hex(),
                point_rt,
                runtime_labels
            ));
        }
    }

    // Build content hashes for each axis layer declaration.
    let semantic_digest = tpt_content_address(&rd.axis_policy.semantic_axes)?;
    let runtime_digest = tpt_content_address(&rd.axis_policy.runtime_axes)?;
    let carrier_digest = tpt_content_address(&rd.axis_policy.carrier_axes)?;

    // Translation proof: content address of the translation policy declaration.
    let translation_proof = tpt_content_address(&rd.axis_policy.translation_policy)?;

    let passed = violations.is_empty();

    let partial = AxisBridgeReport {
        report_id: Hash256::zero(),
        semantic_digest,
        runtime_digest,
        carrier_digest,
        translation_proofs: vec![translation_proof],
        violations,
        passed,
    };
    let report_id = tpt_content_address(&partial)?;
    Ok(AxisBridgeReport {
        report_id,
        ..partial
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::phase_window::{PointMeta, PointStatus, TptPoint};
    use crate::topology::primitives::Fixed;
    use crate::topology::run_descriptor::TptMtlRunDescriptor;

    fn make_window_with_point(
        semantic_labels: [&str; 5],
        runtime_labels: [&str; 5],
    ) -> PhaseSpaceWindow {
        let rd = TptMtlRunDescriptor::default_permissive();
        let provenance = vec![crate::topology::primitives::TptEvidenceRef {
            evidence_id: Hash256::zero(),
            kind: "test".into(),
            source_digest: Hash256::zero(),
            trace_ref: Hash256::zero(),
        }];
        let point = TptPoint {
            point_id: Hash256::zero(),
            x: [
                Fixed::quantize(0.0, 9).unwrap(),
                Fixed::quantize(0.0, 9).unwrap(),
                Fixed::quantize(0.0, 9).unwrap(),
                Fixed::quantize(0.0, 9).unwrap(),
                Fixed::quantize(0.0, 9).unwrap(),
            ],
            meta: PointMeta {
                semantic_axis_labels: semantic_labels.map(|s| s.to_string()),
                runtime_axis_labels: runtime_labels.map(|s| s.to_string()),
                status: PointStatus::Active,
            },
            provenance,
            carrier_ref: Hash256::zero(),
            gate_refs: vec![],
        };
        PhaseSpaceWindow {
            window_id: Hash256::zero(),
            domain_profile: rd.domain_profile.clone(),
            input_refs: vec![],
            points: vec![point],
            carrier: Default::default(),
            horizon_refs: vec![],
            constraint_refs: vec![],
            boundary_ref: Hash256::zero(),
            sampling_policy_hash: Hash256::zero(),
            trace_ref: Hash256::zero(),
            rd_hash: Hash256::zero(),
        }
    }

    #[test]
    fn axis_bridge_passes_with_correct_labels() {
        let rd = TptMtlRunDescriptor::default_permissive();
        let window = make_window_with_point(
            ["R", "F", "T", "S", "E"],
            ["psi", "rho", "omega", "chi", "tau"],
        );
        let report = build_axis_bridge_report(&window, &rd).unwrap();
        assert!(report.passed, "violations: {:?}", report.violations);
    }

    #[test]
    fn axis_bridge_rejects_mixed_axis_origin() {
        let rd = TptMtlRunDescriptor::default_permissive();
        // Use semantic labels in runtime position to trigger violation.
        let window = make_window_with_point(
            ["R", "F", "T", "S", "E"],
            ["R", "F", "T", "S", "E"], // wrong — semantic labels in runtime slots
        );
        let report = build_axis_bridge_report(&window, &rd).unwrap();
        assert!(!report.passed);
        assert!(!report.violations.is_empty());
    }
}

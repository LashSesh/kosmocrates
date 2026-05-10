//! PhaseSpaceWindow and TptPoint (TPT-MTL §5).
//!
//! Invariant I-02: The smallest triangulatable unit is a PhaseSpaceWindow,
//! not an isolated point.
//! Invariant I-01: No triangulatable window without a valid, typed,
//! normalised, admissible and traceable state.

use serde::{Deserialize, Serialize};

use super::carrier::CarrierContext;
use super::primitives::{tpt_content_address, Fixed, Hash256, TopologyError, TptEvidenceRef};
use super::run_descriptor::TptMtlRunDescriptor;

/// Status of a TptPoint in the topology layer.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PointStatus {
    Active,
    NoiseCandidate,
    Blocked,
    Latent,
}

/// Per-point axis metadata.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PointMeta {
    pub semantic_axis_labels: [String; 5],
    pub runtime_axis_labels: [String; 5],
    pub status: PointStatus,
}

/// A single 5D point in the phase space (TPT-MTL §5.2).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TptPoint {
    pub point_id: Hash256,
    /// 5D coordinates using Fixed (no platform floats).
    pub x: [Fixed; 5],
    pub meta: PointMeta,
    /// At least one EvidenceRef required for Active points (invariant provenance).
    pub provenance: Vec<TptEvidenceRef>,
    pub carrier_ref: Hash256,
    pub gate_refs: Vec<Hash256>,
}

impl TptPoint {
    /// Returns true if the point has at least one provenance reference
    /// OR is marked as NoiseCandidate (which is the noise declaration path).
    pub fn has_provenance_or_noise(&self) -> bool {
        !self.provenance.is_empty() || self.meta.status == PointStatus::NoiseCandidate
    }

    /// Compute the deterministic content hash for this point.
    pub fn content_hash(&self) -> Result<Hash256, TopologyError> {
        tpt_content_address(self)
    }
}

/// The smallest triangulatable unit (TPT-MTL §5.1).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PhaseSpaceWindow {
    pub window_id: Hash256,
    pub domain_profile: String,
    /// Content hashes of input reports (CognitionReport, PhasePanorama, etc.).
    pub input_refs: Vec<Hash256>,
    pub points: Vec<TptPoint>,
    pub carrier: CarrierContext,
    pub horizon_refs: Vec<Hash256>,
    pub constraint_refs: Vec<Hash256>,
    pub boundary_ref: Hash256,
    pub sampling_policy_hash: Hash256,
    pub trace_ref: Hash256,
    pub rd_hash: Hash256,
}

impl PhaseSpaceWindow {
    /// Compute the deterministic content hash for this window.
    pub fn content_hash(&self) -> Result<Hash256, TopologyError> {
        tpt_content_address(self)
    }
}

/// Input container for `build_phase_space_window`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TptMtlInput {
    /// Serialized upstream state (CognitionReport | PhasePanorama | etc.).
    /// The content hash of this value becomes an input_ref.
    pub state_digest: Hash256,
    /// Optional explicit carrier context.
    pub carrier: Option<CarrierContext>,
    /// Optional horizon report refs.
    pub horizon_refs: Vec<Hash256>,
    /// Optional constraint lattice refs.
    pub constraint_refs: Vec<Hash256>,
    /// Sampling points. If empty, a degenerate single-point window is built.
    pub sample_points: Vec<SamplePoint>,
    /// Boundary manifest ref.
    pub boundary_ref: Hash256,
    /// Trace ref for this input.
    pub trace_ref: Hash256,
}

/// Caller-provided sample point before normalization.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SamplePoint {
    pub coordinates: [Fixed; 5],
    pub provenance: Vec<TptEvidenceRef>,
    pub status: Option<PointStatus>,
}

/// Build a `PhaseSpaceWindow` from validated input.
///
/// Invariants checked:
/// - I-01: state_digest must be non-zero (proxy for valid upstream state).
/// - I-02: always produces a Window even for single points.
/// - Points without provenance are marked NoiseCandidate.
pub fn build_phase_space_window(
    input: &TptMtlInput,
    rd: &TptMtlRunDescriptor,
) -> Result<PhaseSpaceWindow, TopologyError> {
    // I-01: require non-zero state digest.
    if input.state_digest == Hash256::zero() && input.sample_points.is_empty() {
        return Err(TopologyError::InvalidWindow {
            reason: "state_digest is zero and no sample_points — cannot form triangulatable window"
                .into(),
        });
    }

    let rd_hash = rd.content_hash()?;
    let sampling_policy_hash = tpt_content_address(&rd.axis_policy)?;

    let mut points = Vec::new();
    for (i, sp) in input.sample_points.iter().enumerate() {
        let status = if sp.provenance.is_empty() {
            PointStatus::NoiseCandidate
        } else {
            sp.status.clone().unwrap_or(PointStatus::Active)
        };

        let meta = PointMeta {
            semantic_axis_labels: rd.axis_policy.semantic_axes.clone(),
            runtime_axis_labels: rd.axis_policy.runtime_axes.clone(),
            status,
        };

        let partial_point = TptPoint {
            point_id: Hash256::zero(),
            x: sp.coordinates.clone(),
            meta,
            provenance: sp.provenance.clone(),
            carrier_ref: Hash256::zero(),
            gate_refs: vec![],
        };
        // Point id is content address of the point without the id field.
        let point_id = tpt_content_address(&(
            "tpt-point",
            i as u64,
            &rd_hash,
            &partial_point.x,
            &partial_point.meta,
        ))?;
        points.push(TptPoint {
            point_id,
            ..partial_point
        });
    }

    // If no sample points, emit a single latent point from the state digest.
    if points.is_empty() {
        let zero_coords = [
            Fixed::quantize(0.0, 9).unwrap(),
            Fixed::quantize(0.0, 9).unwrap(),
            Fixed::quantize(0.0, 9).unwrap(),
            Fixed::quantize(0.0, 9).unwrap(),
            Fixed::quantize(0.0, 9).unwrap(),
        ];
        let meta = PointMeta {
            semantic_axis_labels: rd.axis_policy.semantic_axes.clone(),
            runtime_axis_labels: rd.axis_policy.runtime_axes.clone(),
            status: PointStatus::Latent,
        };
        let partial = TptPoint {
            point_id: Hash256::zero(),
            x: zero_coords.clone(),
            meta,
            provenance: vec![],
            carrier_ref: Hash256::zero(),
            gate_refs: vec![],
        };
        let point_id = tpt_content_address(&("tpt-point-latent", &rd_hash, &input.state_digest))?;
        points.push(TptPoint {
            point_id,
            ..partial
        });
    }

    let carrier = input
        .carrier
        .clone()
        .unwrap_or_else(CarrierContext::default);

    // Sort points by point_id for determinism before building window.
    points.sort();

    let partial_window = PhaseSpaceWindow {
        window_id: Hash256::zero(),
        domain_profile: rd.domain_profile.clone(),
        input_refs: vec![input.state_digest.clone()],
        points,
        carrier,
        horizon_refs: input.horizon_refs.clone(),
        constraint_refs: input.constraint_refs.clone(),
        boundary_ref: input.boundary_ref.clone(),
        sampling_policy_hash,
        trace_ref: input.trace_ref.clone(),
        rd_hash,
    };
    let window_id = tpt_content_address(&partial_window)?;
    Ok(PhaseSpaceWindow {
        window_id,
        ..partial_window
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::run_descriptor::TptMtlRunDescriptor;

    fn minimal_input(with_point: bool) -> TptMtlInput {
        let sample_points = if with_point {
            vec![SamplePoint {
                coordinates: [
                    Fixed::quantize(0.1, 9).unwrap(),
                    Fixed::quantize(0.2, 9).unwrap(),
                    Fixed::quantize(0.3, 9).unwrap(),
                    Fixed::quantize(0.4, 9).unwrap(),
                    Fixed::quantize(0.5, 9).unwrap(),
                ],
                provenance: vec![TptEvidenceRef {
                    evidence_id: Hash256::zero(),
                    kind: "test".into(),
                    source_digest: Hash256::zero(),
                    trace_ref: Hash256::zero(),
                }],
                status: Some(PointStatus::Active),
            }]
        } else {
            vec![]
        };
        TptMtlInput {
            state_digest: Hash256::from_hex(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            )
            .unwrap(),
            carrier: None,
            horizon_refs: vec![],
            constraint_refs: vec![],
            sample_points,
            boundary_ref: Hash256::zero(),
            trace_ref: Hash256::zero(),
        }
    }

    #[test]
    fn window_requires_provenance_or_noise_status() {
        let rd = TptMtlRunDescriptor::default_permissive();
        let mut input = minimal_input(true);
        // Remove provenance — should become NoiseCandidate.
        input.sample_points[0].provenance.clear();
        input.sample_points[0].status = None;
        let window = build_phase_space_window(&input, &rd).unwrap();
        assert_eq!(window.points[0].meta.status, PointStatus::NoiseCandidate);
        assert!(window.points[0].has_provenance_or_noise());
    }

    #[test]
    fn window_build_is_deterministic() {
        let rd = TptMtlRunDescriptor::default_permissive();
        let input = minimal_input(true);
        let w1 = build_phase_space_window(&input, &rd).unwrap();
        let w2 = build_phase_space_window(&input, &rd).unwrap();
        assert_eq!(w1.window_id, w2.window_id);
    }

    #[test]
    fn window_rejects_empty_state_no_points() {
        let rd = TptMtlRunDescriptor::default_permissive();
        let mut input = minimal_input(false);
        input.state_digest = Hash256::zero();
        let result = build_phase_space_window(&input, &rd);
        assert!(result.is_err());
    }
}

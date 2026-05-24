//! PSE-bridge integration for nxalien.
//!
//! Implements `pse_graph::ObservationAdapter` so that a compiled
//! NxAlienBundle enters the PSE stack as a first-class Observation —
//! the same pathway used by every other PSE adapter (binance, weather,
//! seismo, …).
//!
//! Invariant I-BRIDGE-001: this crate never constructs SemanticCrystal
//! directly.  It produces Observations; crystal formation is PSE's job.

use pse_graph::{ingest, ObservationAdapter, ObserveError};
use pse_nxalien_types::{
    ArtifactDigest, BridgeAdmissibilityReport, GateOutcome, NxAlienBundle, NxAlienHandoffCandidate,
};
use pse_types::{
    canonical_bytes, content_address_raw, MeasurementContext, Observation, ProvenanceEnvelope,
};
use sha2::{Digest, Sha256};
use std::f64::consts::PI;

// ─── ObservationAdapter implementation ───────────────────────────────────────

/// Produces a PSE Observation from a JCS-serialized NxAlienBundle.
///
/// `phase_hint` is derived from the bundle's gate outcome so that the
/// Mandorla interference reflects semantic quality:
///   Accept        → π/4  (constructive, high-coherence quadrant)
///   Hold          → π/2  (orthogonal — pending)
///   EvidenceOnly  → 3π/4 (partial coherence)
///   Reject        → π    (destructive — out of phase)
pub struct NxAlienObservationAdapter;

impl ObservationAdapter for NxAlienObservationAdapter {
    fn source_id(&self) -> &str {
        "nxalien-v1"
    }

    fn canonicalize(
        &self,
        raw: &[u8],
        context: &MeasurementContext,
    ) -> Result<Observation, ObserveError> {
        let digest = content_address_raw(raw);

        // Decode the bundle to derive a semantic phase hint.
        let phase_hint = serde_json::from_slice::<NxAlienBundle>(raw)
            .ok()
            .map(|b| gate_phase(&b.gate_report.outcome));

        Ok(Observation {
            timestamp: 0.0,
            source_id: self.source_id().to_string(),
            provenance: ProvenanceEnvelope {
                origin: "nxalien-v1".to_string(),
                chain: vec![],
                sig: None,
            },
            payload: raw.to_vec(),
            context: context.clone(),
            digest,
            schema_version: "1.0".to_string(),
            phase_hint,
        })
    }
}

/// Map a gate outcome to a Mandorla phase hint in [0, 2π).
fn gate_phase(outcome: &GateOutcome) -> f64 {
    match outcome {
        GateOutcome::Accept => PI / 4.0,
        GateOutcome::Hold => PI / 2.0,
        GateOutcome::EvidenceOnly => 3.0 * PI / 4.0,
        GateOutcome::Reject => PI,
    }
}

// ─── Bundle → Observation ────────────────────────────────────────────────────

/// Serialise a bundle to canonical bytes and ingest it as a PSE Observation.
///
/// Returns the ingested Observation (payload = JCS(bundle)).
pub fn ingest_bundle(
    bundle: &NxAlienBundle,
    context: &MeasurementContext,
) -> Result<Observation, ObserveError> {
    let raw = canonical_bytes(bundle);
    let adapter = NxAlienObservationAdapter;
    ingest(&adapter, &raw, context)
}

// ─── Handoff candidate helpers ───────────────────────────────────────────────

/// Build an NxAlienHandoffCandidate from a compiled bundle.
///
/// Does NOT call SemanticCrystal constructors (I-BRIDGE-001).
pub fn build_handoff_candidate(bundle: &NxAlienBundle) -> NxAlienHandoffCandidate {
    // Observation digest = SHA-256 of sorted canonical slices.
    let slices = sorted_observation_slices(bundle);
    let obs_digest = {
        let mut h = Sha256::new();
        for s in &slices {
            h.update(s);
        }
        bytes_to_hex(&h.finalize())
    };

    let admissible = bundle.gate_report.outcome == GateOutcome::Accept;
    let reason = if admissible {
        "All gates passed".to_string()
    } else if bundle.gate_report.notes.is_empty() {
        format!("Gate outcome: {:?}", bundle.gate_report.outcome)
    } else {
        bundle.gate_report.notes.join("; ")
    };

    NxAlienHandoffCandidate {
        candidate_id: format!(
            "nxa-{}",
            &bundle.manifest_hash[..8.min(bundle.manifest_hash.len())]
        ),
        observation_bytes_digest: obs_digest,
        source_artifacts: bundle.artifacts.clone(),
        intended_bridge: "pse-bridge-v0.2".to_string(),
        admissibility: BridgeAdmissibilityReport { admissible, reason },
    }
}

/// Compute a SHA-256 artifact digest for a file at `path`.
pub fn artifact_digest(path: &str) -> Option<ArtifactDigest> {
    let content = std::fs::read(path).ok()?;
    let hash = bytes_to_hex(&Sha256::digest(&content));
    Some(ArtifactDigest {
        path: path.to_string(),
        sha256: hash,
        size_bytes: content.len() as u64,
    })
}

// ─── Private helpers ─────────────────────────────────────────────────────────

fn sorted_observation_slices(bundle: &NxAlienBundle) -> Vec<Vec<u8>> {
    let mut slices = vec![
        canonical_bytes(bundle),
        canonical_bytes(&bundle.gate_report),
        canonical_bytes(&bundle.descriptor),
    ];
    slices.sort();
    slices
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use pse_nxalien_types::{NxAlienGateReport, NxAlienRunDescriptor, ProjectMetadata};

    fn accept_bundle() -> NxAlienBundle {
        NxAlienBundle {
            descriptor: NxAlienRunDescriptor::default(),
            manifest_hash: "abcdef1234567890".to_string(),
            context_hash: "context".to_string(),
            replay_hash: "replay".to_string(),
            gate_report: NxAlienGateReport {
                evidence_ok: true,
                scope_ok: true,
                replay_ok: true,
                canon_ok: true,
                delta_ok: true,
                budget_ok: true,
                governance_ok: true,
                bridge_ok: true,
                outcome: GateOutcome::Accept,
                notes: vec![],
            },
            artifacts: vec![],
            handoff_candidates: vec![],
            rules: vec![],
            unknowns: vec![],
            metadata: ProjectMetadata::default(),
        }
    }

    #[test]
    fn ingest_bundle_produces_observation() {
        let bundle = accept_bundle();
        let ctx = MeasurementContext::default();
        let obs = ingest_bundle(&bundle, &ctx).expect("ingest must succeed");
        assert_eq!(obs.source_id, "nxalien-v1");
        // phase_hint should be π/4 for Accept
        let hint = obs
            .phase_hint
            .expect("phase_hint must be set for nxalien obs");
        let expected = std::f64::consts::PI / 4.0;
        assert!(
            (hint - expected).abs() < 1e-10,
            "phase hint mismatch: {hint}"
        );
    }

    #[test]
    fn observation_digest_matches_payload() {
        let bundle = accept_bundle();
        let ctx = MeasurementContext::default();
        let obs = ingest_bundle(&bundle, &ctx).expect("ingest must succeed");
        let recomputed = content_address_raw(&obs.payload);
        assert_eq!(obs.digest, recomputed, "digest must match payload SHA-256");
    }

    #[test]
    fn handoff_candidate_accept() {
        let bundle = accept_bundle();
        let candidate = build_handoff_candidate(&bundle);
        assert!(candidate.admissibility.admissible);
        assert!(candidate.candidate_id.starts_with("nxa-"));
    }

    #[test]
    fn handoff_candidate_reject() {
        let mut bundle = accept_bundle();
        bundle.gate_report.outcome = GateOutcome::Reject;
        bundle.gate_report.notes = vec!["G_evidence failed".to_string()];
        let candidate = build_handoff_candidate(&bundle);
        assert!(!candidate.admissibility.admissible);
        assert!(candidate.admissibility.reason.contains("G_evidence"));
    }

    #[test]
    fn reject_gate_phase_is_pi() {
        assert!((gate_phase(&GateOutcome::Reject) - std::f64::consts::PI).abs() < 1e-10);
    }

    /// Static source guard — I-BRIDGE-001.
    #[test]
    fn no_semantic_crystal_construction() {
        let forbidden = ["Semantic", "Crystal", " {"].join("");
        let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        for entry in std::fs::read_dir(&src_dir).expect("src/ must exist") {
            let path = entry.unwrap().path();
            if path.extension().map(|e| e == "rs").unwrap_or(false) {
                let content = std::fs::read_to_string(&path).unwrap();
                assert!(
                    !content.contains(&forbidden),
                    "I-BRIDGE-001 violated: SemanticCrystal construction in {:?}",
                    path
                );
            }
        }
    }
}

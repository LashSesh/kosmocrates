//! PSE-bridge observation adapter for nxalien.
//!
//! Produces sorted observation byte slices and handoff candidates from a
//! compiled NxAlienBundle.  This crate deliberately does NOT import or
//! construct SemanticCrystal — nxalien never touches the crystal path
//! directly (invariant I-BRIDGE-001).

use pse_nxalien_types::{
    ArtifactDigest, BridgeAdmissibilityReport, GateOutcome, NxAlienBundle, NxAlienHandoffCandidate,
};
use sha2::{Digest, Sha256};

/// Converts an nxalien bundle into PSE-bridge–ready byte slices and
/// handoff candidates.
pub struct NxAlienObservationAdapter;

impl NxAlienObservationAdapter {
    /// Produce sorted, deterministic observation byte slices for the PSE bridge.
    ///
    /// Does NOT call SemanticCrystal constructors (I-BRIDGE-001).
    pub fn to_observation_bytes(bundle: &NxAlienBundle) -> Result<Vec<Vec<u8>>, String> {
        let mut slices: Vec<Vec<u8>> = Vec::new();

        let manifest_bytes = serde_jcs::to_vec(bundle).map_err(|e| e.to_string())?;
        slices.push(manifest_bytes);

        let gate_bytes = serde_jcs::to_vec(&bundle.gate_report).map_err(|e| e.to_string())?;
        slices.push(gate_bytes);

        let desc_bytes = serde_jcs::to_vec(&bundle.descriptor).map_err(|e| e.to_string())?;
        slices.push(desc_bytes);

        slices.sort();
        Ok(slices)
    }

    /// Compute the SHA-256 digest of sorted observation bytes.
    pub fn observation_digest(slices: &[Vec<u8>]) -> String {
        let mut h = Sha256::new();
        for s in slices {
            h.update(s);
        }
        bytes_to_hex(&h.finalize())
    }

    /// Build a handoff candidate from a compiled bundle.
    pub fn build_handoff_candidate(bundle: &NxAlienBundle) -> NxAlienHandoffCandidate {
        let slices = Self::to_observation_bytes(bundle).unwrap_or_default();
        let obs_digest = Self::observation_digest(&slices);

        let admissible = bundle.gate_report.outcome == GateOutcome::Accept;
        let reason = if admissible {
            "All gates passed".to_string()
        } else {
            if bundle.gate_report.notes.is_empty() {
                format!("Gate outcome: {:?}", bundle.gate_report.outcome)
            } else {
                bundle.gate_report.notes.join("; ")
            }
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

    /// Compute a SHA-256 artifact digest for a file path (reads the file).
    pub fn artifact_digest(path: &str) -> Option<ArtifactDigest> {
        let content = std::fs::read(path).ok()?;
        let hash = {
            let mut h = Sha256::new();
            h.update(&content);
            bytes_to_hex(&h.finalize())
        };
        Some(ArtifactDigest {
            path: path.to_string(),
            sha256: hash,
            size_bytes: content.len() as u64,
        })
    }
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pse_nxalien_types::{NxAlienGateReport, NxAlienRunDescriptor, ProjectMetadata};

    fn empty_bundle() -> NxAlienBundle {
        NxAlienBundle {
            descriptor: NxAlienRunDescriptor::default(),
            manifest_hash: "abcdef1234567890".to_string(),
            context_hash: "context-hash".to_string(),
            replay_hash: "replay-hash".to_string(),
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
    fn observation_bytes_are_sorted() {
        let bundle = empty_bundle();
        let slices = NxAlienObservationAdapter::to_observation_bytes(&bundle).unwrap();
        let mut sorted = slices.clone();
        sorted.sort();
        assert_eq!(slices, sorted, "slices must be in sorted order");
    }

    #[test]
    fn handoff_candidate_accept() {
        let bundle = empty_bundle();
        let candidate = NxAlienObservationAdapter::build_handoff_candidate(&bundle);
        assert!(candidate.admissibility.admissible);
        assert!(candidate.candidate_id.starts_with("nxa-"));
    }

    #[test]
    fn handoff_candidate_reject_on_gate_fail() {
        let mut bundle = empty_bundle();
        bundle.gate_report.outcome = GateOutcome::Reject;
        bundle.gate_report.notes = vec!["G_evidence failed".to_string()];
        let candidate = NxAlienObservationAdapter::build_handoff_candidate(&bundle);
        assert!(!candidate.admissibility.admissible);
        assert!(candidate.admissibility.reason.contains("G_evidence"));
    }

    /// Static source guard — this crate must never construct SemanticCrystal.
    ///
    /// The forbidden pattern is assembled at runtime to avoid the guard
    /// triggering on its own source text.
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
                    "I-BRIDGE-001 violated: SemanticCrystal construction found in {:?}",
                    path
                );
            }
        }
    }
}

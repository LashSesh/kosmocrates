//! SemanticCrystal → IL payload conversion.

use pse_types::SemanticCrystal;
use sha2::{Digest, Sha256};

/// Everything IL needs for one ledger block + HNSW upsert.
#[derive(Debug, Clone)]
pub struct ILPayload {
    /// 64-char hex of crystal_id — used as HNSW record_id.
    pub crystal_id_hex: String,
    /// First 16 chars of crystal_id_hex — used as tic_id in the ledger block.
    pub tic_id: String,
    /// IL CompactTic JSON (matches IL's `CompactTic` struct field names).
    pub tic_json: serde_json::Value,
    /// IL snapshot JSON passed alongside the TIC to the ledger.
    pub snapshot_json: serde_json::Value,
    /// 8D normalised vector for HNSW indexing.
    pub vector8: Vec<f64>,
}

/// Converts a PSE `SemanticCrystal` + its source chunks into an `ILPayload`.
pub struct CrystalAdapter {
    seed: String,
}

impl CrystalAdapter {
    pub fn new(seed: &str) -> Self {
        Self {
            seed: seed.to_string(),
        }
    }

    /// Core conversion: crystal → IL payload.
    pub fn convert(
        &self,
        crystal: &SemanticCrystal,
        source_chunks: &[String],
    ) -> Result<ILPayload, String> {
        let crystal_id_hex: String = crystal
            .crystal_id
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();
        let tic_id = crystal_id_hex[..16].to_string();

        // 5D fixpoint from PSE topology invariants — structural analogue of
        // IL's Solve-Coagula fixpoint (both are topological fixed-points,
        // different numeric basis):
        //   [0] spectral_gap           ↔ IL invariants.gap
        //   [1] cheeger_estimate       ↔ IL invariants.delta_pi
        //   [2] kuramoto_coherence     ↔ IL sigma_bar.psi (via tanh)
        //   [3] mean_propagation_time  ↔ IL fixpoint dim 3
        //   [4] stability_score        ↔ IL fixpoint_norm
        let sig = &crystal.topology_signature;
        let fixpoint: Vec<f64> = vec![
            sig.spectral_gap,
            sig.cheeger_estimate,
            sig.kuramoto_coherence,
            sig.mean_propagation_time,
            crystal.stability_score,
        ];
        let fixpoint_norm: f64 = fixpoint.iter().map(|x| x * x).sum::<f64>().sqrt();

        // IL invariants — named after IL's TICCrystallizer::compute_invariants()
        let invariants = serde_json::json!({
            "variance":   1.0 - crystal.stability_score.clamp(0.0, 1.0),
            "retention":  sig.kuramoto_coherence.clamp(0.0, 1.0),
            "gap":        sig.spectral_gap,
            "delta_pi":   sig.cheeger_estimate,
        });

        // sigma_bar (ψ, ρ, ω) — spectral signature triple
        let sigma_psi = sig.kuramoto_coherence.tanh().clamp(0.0, 1.0);
        let sigma_rho = (crystal.stability_score * 0.5 + 0.5).clamp(0.0, 1.0);
        // cos of spectral_gap gives a bounded oscillatory dimension
        let sigma_omega = sig.spectral_gap.cos().clamp(0.0, 1.0);

        let sigma_bar = serde_json::json!({
            "psi":   sigma_psi,
            "rho":   sigma_rho,
            "omega": sigma_omega,
        });

        // Proof fields derived from the Kairos gate snapshot
        let proof = serde_json::json!({
            "por":    if crystal.stability_score > 0.5 { "valid" } else { "invalid" },
            "pi_gap": crystal.commit_proof.gate_values.n,
            "mci":    crystal.commit_proof.consensus_result.mci,
            "phi":    crystal.commit_proof.gate_values.q,
        });

        // window: time-ordered commit-index pair
        let window = vec![
            format!("{}", crystal.created_at),
            format!("{}", crystal.created_at.saturating_add(1)),
        ];

        let tic_json = serde_json::json!({
            "tic_id":         tic_id,
            "seed":           self.seed,
            "fixpoint":       fixpoint,
            "fixpoint_norm":  fixpoint_norm,
            "invariants":     invariants,
            "sigma_bar":      sigma_bar,
            "window":         window,
            "proof":          proof,
        });

        let snapshot_json = serde_json::json!({
            "crystal_id":    crystal_id_hex,
            "source_chunks": source_chunks,
            "coordinates":   fixpoint,
            "betti":         [sig.betti_0, sig.betti_1, sig.betti_2],
            "scale_tag":     crystal.scale_tag,
            "metrics": {
                "resonance": crystal.stability_score,
                "por": if crystal.stability_score > 0.5 { "valid" } else { "invalid" },
            },
        });

        // 8D HNSW vector: [x0..x4, ψ, ρ, ω] L2-normalised
        let vector8 = build_vector8(&fixpoint, sigma_psi, sigma_rho, sigma_omega)
            .ok_or("zero-norm 8D vector — crystal has no topology signal")?;

        Ok(ILPayload {
            crystal_id_hex,
            tic_id,
            tic_json,
            snapshot_json,
            vector8,
        })
    }

    /// Extended conversion that also embeds session + question provenance.
    pub fn convert_with_provenance(
        &self,
        crystal: &SemanticCrystal,
        source_chunks: &[String],
        session: usize,
        question: &str,
    ) -> Result<ILPayload, String> {
        let mut payload = self.convert(crystal, source_chunks)?;
        if let Some(snap) = payload.snapshot_json.as_object_mut() {
            snap.insert("session".into(), serde_json::json!(session));
            snap.insert("question".into(), serde_json::json!(question));
        }
        Ok(payload)
    }
}

/// Build a normalised 8D vector from a 5D fixpoint + 3 spectral scalars.
fn build_vector8(x5: &[f64], psi: f64, rho: f64, omega: f64) -> Option<Vec<f64>> {
    let mut z: Vec<f64> = x5.iter().copied().collect();
    z.push(psi);
    z.push(rho);
    z.push(omega);
    let norm: f64 = z.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm < 1e-10 {
        return None;
    }
    Some(z.iter().map(|x| x / norm).collect())
}

/// Derive an 8D embedding from a plain text string (for question-time retrieval).
/// Uses SHA-256 → 5D (same as IL's TritonCore) with neutral sigma = 0.5.
pub fn text_to_vector8(text: &str) -> Vec<f64> {
    let hash_bytes: Vec<u8> = Sha256::digest(text.as_bytes()).iter().copied().collect();
    let hash = hash_bytes;
    let mut x5 = Vec::with_capacity(5);
    for i in 0..5 {
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&hash[i * 4..(i + 1) * 4]);
        let v = u32::from_be_bytes(buf) as f64 / u32::MAX as f64;
        x5.push(v * 2.0 - 1.0);
    }
    build_vector8(&x5, 0.5, 0.5, 0.5).unwrap_or_else(|| vec![0.0; 8])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_crystal(stability: f64) -> SemanticCrystal {
        let mut c = SemanticCrystal {
            crystal_id: [0u8; 32],
            region: vec![],
            constraint_program: Default::default(),
            stability_score: stability,
            topology_signature: Default::default(),
            betti_numbers: vec![],
            evidence_chain: Default::default(),
            commit_proof: Default::default(),
            operator_versions: Default::default(),
            created_at: 0,
            free_energy: 0.0,
            carrier_instance_idx: 0,
            scale_tag: String::new(),
            universe_id: String::new(),
            sub_crystal_ids: vec![],
            parent_crystal_ids: vec![],
            genesis_metadata: None,
            metatron_signature: None,
        };
        // Give it a distinct crystal_id so tic_id prefix test works
        c.crystal_id[0] = 0xAB;
        c.crystal_id[1] = 0xCD;
        c
    }

    #[test]
    fn adapter_produces_valid_vector8() {
        let adapter = CrystalAdapter::new("TEST_SEED");
        let crystal = dummy_crystal(0.85);
        let payload = adapter.convert(&crystal, &["chunk one".into(), "chunk two".into()]);
        assert!(payload.is_ok(), "convert should not fail: {:?}", payload.err());
        let p = payload.unwrap();
        assert_eq!(p.vector8.len(), 8);
        let norm: f64 = p.vector8.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    #[test]
    fn tic_id_is_prefix_of_crystal_id() {
        let adapter = CrystalAdapter::new("TEST_SEED");
        let crystal = dummy_crystal(0.75);
        let p = adapter.convert(&crystal, &[]).unwrap();
        assert!(p.crystal_id_hex.starts_with(&p.tic_id));
        assert_eq!(p.tic_id.len(), 16);
        assert_eq!(p.crystal_id_hex.len(), 64);
    }

    #[test]
    fn text_embedding_has_unit_norm() {
        let v = text_to_vector8("What is ACT-R?");
        assert_eq!(v.len(), 8);
        let norm: f64 = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    #[test]
    fn low_stability_gives_invalid_por() {
        let adapter = CrystalAdapter::new("SEED");
        let crystal = dummy_crystal(0.3);
        let p = adapter.convert(&crystal, &[]).unwrap();
        assert_eq!(p.tic_json["proof"]["por"], "invalid");
    }
}

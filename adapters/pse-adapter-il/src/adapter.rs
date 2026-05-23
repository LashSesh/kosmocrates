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
    /// Phase angle derived from the fixpoint (used as HDAG node phase).
    pub phase: f64,
}

/// Converts a PSE `SemanticCrystal` + its source chunks into an `ILPayload`.
pub struct CrystalAdapter {
    seed: String,
}

impl CrystalAdapter {
    pub fn new(seed: &str) -> Self {
        Self { seed: seed.to_string() }
    }

    /// Core conversion: crystal → IL payload (pure PSE-side mapping).
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

        let sig = &crystal.topology_signature;

        // 5D fixpoint: structural analogue of IL's Solve-Coagula fixpoint.
        //   [0] spectral_gap          ↔ IL invariants.gap
        //   [1] cheeger_estimate      ↔ IL invariants.delta_pi
        //   [2] kuramoto_coherence    ↔ IL sigma_bar.psi (via tanh)
        //   [3] mean_propagation_time ↔ IL fixpoint dim 3
        //   [4] stability_score       ↔ IL fixpoint_norm
        let fixpoint: Vec<f64> = vec![
            sig.spectral_gap,
            sig.cheeger_estimate,
            sig.kuramoto_coherence,
            sig.mean_propagation_time,
            crystal.stability_score,
        ];
        let fixpoint_norm: f64 = fixpoint.iter().map(|x| x * x).sum::<f64>().sqrt();

        let sigma_psi = sig.kuramoto_coherence.tanh().clamp(0.0, 1.0);
        let sigma_rho = (crystal.stability_score * 0.5 + 0.5).clamp(0.0, 1.0);
        let sigma_omega = sig.spectral_gap.cos().clamp(0.0, 1.0);

        let invariants = serde_json::json!({
            "variance":  1.0 - crystal.stability_score.clamp(0.0, 1.0),
            "retention": sig.kuramoto_coherence.clamp(0.0, 1.0),
            "gap":       sig.spectral_gap,
            "delta_pi":  sig.cheeger_estimate,
        });

        let sigma_bar = serde_json::json!({
            "psi":   sigma_psi,
            "rho":   sigma_rho,
            "omega": sigma_omega,
        });

        let proof = serde_json::json!({
            "por":    if crystal.stability_score > 0.5 { "valid" } else { "invalid" },
            "pi_gap": crystal.commit_proof.gate_values.n,
            "mci":    crystal.commit_proof.consensus_result.mci,
            "phi":    crystal.commit_proof.gate_values.q,
        });

        let window = vec![
            format!("{}", crystal.created_at),
            format!("{}", crystal.created_at.saturating_add(1)),
        ];

        let tic_json = serde_json::json!({
            "tic_id":        tic_id,
            "seed":          self.seed,
            "fixpoint":      fixpoint,
            "fixpoint_norm": fixpoint_norm,
            "invariants":    invariants,
            "sigma_bar":     sigma_bar,
            "window":        window,
            "proof":         proof,
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

        let phase = fixpoint.first().copied().unwrap_or(0.0);

        let vector8 = build_vector8(&fixpoint, sigma_psi, sigma_rho, sigma_omega)
            .ok_or("zero-norm 8D vector — crystal has no topology signal")?;

        Ok(ILPayload { crystal_id_hex, tic_id, tic_json, snapshot_json, vector8, phase })
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
pub fn build_vector8(x5: &[f64], psi: f64, rho: f64, omega: f64) -> Option<Vec<f64>> {
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

/// Derive an 8D embedding from plain text using char-4-gram bucketed phases.
///
/// Each overlapping char-4-gram is hashed (FNV-1a) and routed to one of 5
/// buckets by `hash mod 5`.  Within each bucket the circular mean phase is
/// computed, giving 5 independent dimensions whose values are coherent across
/// semantically similar strings (unlike SHA-256).  The σ-triple is computed
/// from text-level statistics.
pub fn text_to_vector8(text: &str) -> Vec<f64> {
    let lower = text.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();

    let mut sum_sin = [0.0f64; 5];
    let mut sum_cos = [0.0f64; 5];
    let mut counts = [0usize; 5];

    for w in chars.windows(4) {
        let s: String = w.iter().collect();
        let h = fnv1a_u64(s.as_bytes());
        let bucket = (h % 5) as usize;
        let phi = (h as f64 / u64::MAX as f64) * std::f64::consts::TAU;
        sum_sin[bucket] += phi.sin();
        sum_cos[bucket] += phi.cos();
        counts[bucket] += 1;
    }

    let x5: Vec<f64> = (0..5)
        .map(|i| {
            if counts[i] == 0 {
                0.0
            } else {
                sum_sin[i].atan2(sum_cos[i])
            }
        })
        .collect();

    // Global circular mean across all 5 bucket phases
    let (gs, gc): (f64, f64) =
        x5.iter().fold((0.0, 0.0), |(s, c), &p| (s + p.sin(), c + p.cos()));
    let global_phase = gs.atan2(gc);

    // σ-triple from text statistics
    let sigma_psi = global_phase.tanh().clamp(0.0, 1.0);
    let unique = chars
        .windows(4)
        .map(|w| w.iter().collect::<String>())
        .collect::<std::collections::HashSet<_>>()
        .len();
    let total = chars.len().saturating_sub(3).max(1);
    let sigma_rho = (unique as f64 / total as f64).clamp(0.0, 1.0);
    let sigma_omega = global_phase.cos().clamp(0.0, 1.0);

    build_vector8(&x5, sigma_psi, sigma_rho, sigma_omega).unwrap_or_else(|| {
        // Fallback for very short text: SHA-256 → uniform distribution
        let hash_bytes: Vec<u8> = Sha256::digest(text.as_bytes()).iter().copied().collect();
        let mut fb = Vec::with_capacity(5);
        for i in 0..5 {
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&hash_bytes[i * 4..(i + 1) * 4]);
            fb.push(u32::from_be_bytes(buf) as f64 / u32::MAX as f64 * 2.0 - 1.0);
        }
        build_vector8(&fb, 0.5, 0.5, 0.5).unwrap_or_else(|| vec![0.0; 8])
    })
}

fn fnv1a_u64(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

// ── il-pipeline: MEFCore-backed conversion ───────────────────────────────────

#[cfg(feature = "il-pipeline")]
pub mod pipeline {
    use super::*;
    use mef_core::{MEFCore, MEFCoreConfig};

    /// Convert a crystal via the full MEFCore pipeline (Triton → SolveCoagula
    /// → TICCrystallizer).  The resulting TIC and 8D vector are authoritative
    /// IL outputs, not approximations.
    ///
    /// `mef_core` is mutably borrowed so it accumulates spiral memory across
    /// calls — pass the same instance for the lifetime of the session.
    pub fn convert_via_mef_core(
        crystal: &SemanticCrystal,
        source_chunks: &[String],
        mef_core: &mut MEFCore,
    ) -> Result<ILPayload, String> {
        let crystal_id_hex: String = crystal
            .crystal_id
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();

        let sig = &crystal.topology_signature;
        let crystal_json = serde_json::json!({
            "crystal_id":            crystal_id_hex,
            "stability_score":       crystal.stability_score,
            "spectral_gap":          sig.spectral_gap,
            "cheeger_estimate":      sig.cheeger_estimate,
            "kuramoto_coherence":    sig.kuramoto_coherence,
            "mean_propagation_time": sig.mean_propagation_time,
            "betti":                 [sig.betti_0, sig.betti_1, sig.betti_2],
            "scale_tag":             crystal.scale_tag,
            "source_chunks":         source_chunks,
        });

        let result = mef_core
            .process(crystal_json.clone(), "crystal", false)
            .map_err(|e| format!("MEFCore::process failed: {e}"))?;

        // The first 5 dims of vector8 carry the Solve-Coagula fixpoint projection.
        let fixpoint: Vec<f64> = result.vector8[..5].to_vec();
        let fixpoint_norm: f64 = fixpoint.iter().map(|x| x * x).sum::<f64>().sqrt();
        let sigma_psi = result.vector8.get(5).copied().unwrap_or(0.5);
        let sigma_rho = result.vector8.get(6).copied().unwrap_or(0.5);
        let sigma_omega = result.vector8.get(7).copied().unwrap_or(0.5);

        let invariants = serde_json::json!({
            "variance":  1.0 - crystal.stability_score.clamp(0.0, 1.0),
            "retention": sig.kuramoto_coherence.clamp(0.0, 1.0),
            "gap":       sig.spectral_gap,
            "delta_pi":  sig.cheeger_estimate,
        });
        let sigma_bar = serde_json::json!({ "psi": sigma_psi, "rho": sigma_rho, "omega": sigma_omega });
        let proof = serde_json::json!({
            "por":    if crystal.stability_score > 0.5 { "valid" } else { "invalid" },
            "pi_gap": crystal.commit_proof.gate_values.n,
            "mci":    crystal.commit_proof.consensus_result.mci,
        });

        let tic_json = serde_json::json!({
            "tic_id":        result.tic_id,
            "fixpoint":      fixpoint,
            "fixpoint_norm": fixpoint_norm,
            "converged":     result.converged,
            "iterations":    result.iterations,
            "sigma_bar":     sigma_bar,
            "invariants":    invariants,
            "proof":         proof,
            "window":        [format!("{}", crystal.created_at)],
        });

        let snapshot_json = serde_json::json!({
            "crystal_id":    crystal_id_hex,
            "source_chunks": source_chunks,
            "coordinates":   fixpoint,
            "snapshot_phase": result.snapshot_phase,
        });

        let phase = result.snapshot_phase;

        Ok(ILPayload {
            crystal_id_hex,
            tic_id: result.tic_id,
            tic_json,
            snapshot_json,
            vector8: result.vector8,
            phase,
        })
    }

    /// Build a `MEFCore` instance whose store dirs live under `base_path`.
    pub fn make_mef_core(base_path: &str, seed: &str) -> Result<MEFCore, String> {
        let config = MEFCoreConfig {
            seed: seed.to_string(),
            tic_store_path: format!("{base_path}/mef-tic-store"),
            ledger_path: format!("{base_path}/mef-ledger"),
            ..MEFCoreConfig::with_seed(seed)
        };
        MEFCore::new(seed, Some(config)).map_err(|e| format!("MEFCore::new failed: {e}"))
    }
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
        c.crystal_id[0] = 0xAB;
        c.crystal_id[1] = 0xCD;
        c
    }

    #[test]
    fn adapter_produces_valid_vector8() {
        let adapter = CrystalAdapter::new("TEST_SEED");
        let crystal = dummy_crystal(0.85);
        let p = adapter
            .convert(&crystal, &["chunk one".into(), "chunk two".into()])
            .unwrap();
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
    fn text_embedding_is_semantic() {
        // Similar strings must produce more similar vectors than dissimilar ones.
        let v_a = text_to_vector8("cognitive architecture working memory");
        let v_b = text_to_vector8("cognitive architecture long-term memory");
        let v_c = text_to_vector8("photosynthesis chlorophyll plant biology");

        let sim_ab: f64 = v_a.iter().zip(&v_b).map(|(x, y)| x * y).sum();
        let sim_ac: f64 = v_a.iter().zip(&v_c).map(|(x, y)| x * y).sum();

        assert!(
            sim_ab > sim_ac,
            "semantically similar texts should be closer: sim(a,b)={sim_ab:.3} sim(a,c)={sim_ac:.3}"
        );
    }

    #[test]
    fn short_text_does_not_panic() {
        let v = text_to_vector8("hi");
        assert_eq!(v.len(), 8);
    }

    #[test]
    fn low_stability_gives_invalid_por() {
        let adapter = CrystalAdapter::new("SEED");
        let crystal = dummy_crystal(0.3);
        let p = adapter.convert(&crystal, &[]).unwrap();
        assert_eq!(p.tic_json["proof"]["por"], "invalid");
    }
}

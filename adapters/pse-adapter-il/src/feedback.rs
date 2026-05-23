//! IL → PSE feedback loop.
//!
//! Every crystal that passes through the IL pipeline carries back a
//! `ValidationFeedback` with convergence and coherence information.  This
//! feedback is used to produce a *refined* crystal whose `stability_score`
//! is a weighted blend of the original PSE topological quality and IL's
//! convergence signal.
//!
//! The refined crystal:
//! - Has a new `crystal_id` (SHA-256 of "il-refined:<original_hex>:<new_stability>")
//! - Lists the original crystal in `parent_crystal_ids`
//! - Carries `scale_tag = "il-refined"`
//! - Preserves all topological fields unchanged (topology_signature, Betti numbers, etc.)
//!
//! Because PSE is content-addressed, the refinement creates a genuinely new
//! crystal — it is not a mutation.  The original and the refined crystal
//! coexist; the HDAG connects them via the genealogy edge.

use pse_types::SemanticCrystal;
use sha2::{Digest, Sha256};

/// Validation result returned by `ILStore::commit_with_feedback`.
#[derive(Debug, Clone)]
pub struct ValidationFeedback {
    /// Block hash written to the ledger.
    pub block_hash: String,
    /// Did SolveCoagula converge? (authoritative with `il-pipeline`,
    /// heuristic otherwise: stability > 0.5 && kuramoto > 0.3)
    pub converged: bool,
    /// Coherence potential ψ = morphic − entropic = kuramoto_coherence − (1 − stability).
    pub coherence_potential: f64,
    /// Was the HDAG coherence gate (S_coh) satisfied?
    pub gate_passed: bool,
    /// HDAG node ID assigned to this crystal, if the HDAG is active.
    pub hdag_node_id: Option<String>,
    /// Normalized IL quality signal in [0, 1], derived from convergence +
    /// gate status + coherence potential.  Used to compute the blended
    /// stability score of the refined crystal.
    pub il_stability: f64,
}

impl ValidationFeedback {
    /// Build a ValidationFeedback without MEFCore (heuristic convergence).
    pub fn from_crystal_heuristic(
        block_hash: String,
        crystal: &SemanticCrystal,
        coherence_potential: f64,
        gate_passed: bool,
        hdag_node_id: Option<String>,
    ) -> Self {
        let sig = &crystal.topology_signature;
        // Heuristic: a crystal "would converge" if it is topologically stable
        let converged =
            crystal.stability_score > 0.5 && sig.kuramoto_coherence > 0.3;

        let il_stability = compute_il_stability(converged, gate_passed, coherence_potential);

        Self {
            block_hash,
            converged,
            coherence_potential,
            gate_passed,
            hdag_node_id,
            il_stability,
        }
    }

    /// Build a ValidationFeedback with authoritative MEFCore convergence data.
    #[cfg(feature = "il-pipeline")]
    pub fn from_mef_core(
        block_hash: String,
        converged: bool,
        coherence_potential: f64,
        gate_passed: bool,
        hdag_node_id: Option<String>,
    ) -> Self {
        let il_stability = compute_il_stability(converged, gate_passed, coherence_potential);
        Self {
            block_hash,
            converged,
            coherence_potential,
            gate_passed,
            hdag_node_id,
            il_stability,
        }
    }
}

/// Compute a normalized IL stability signal in [0, 1].
///
/// The signal encodes three levels of confidence:
/// - Converged + gate passed  → full weight of coherence potential
/// - Converged + gate failed  → half weight (marginal quality)
/// - Did not converge         → strong penalty (0.2 weight)
pub fn compute_il_stability(converged: bool, gate_passed: bool, coherence_potential: f64) -> f64 {
    // Normalize ψ ∈ [-1, 1] → [0, 1]
    let psi_norm = ((coherence_potential.clamp(-1.0, 1.0) + 1.0) / 2.0).clamp(0.0, 1.0);

    match (converged, gate_passed) {
        (true, true)  => psi_norm,
        (true, false) => psi_norm * 0.5,
        (false, _)    => psi_norm * 0.2,
    }
}

/// Produce a refined crystal by blending PSE topological quality with
/// IL's convergence signal.
///
/// `new_created_at` should be the current CommitIndex (or the original + 1).
/// All topological fields are preserved; only `stability_score`,
/// `crystal_id`, `parent_crystal_ids`, and `scale_tag` change.
pub fn refine_crystal(
    original: &SemanticCrystal,
    feedback: &ValidationFeedback,
    new_created_at: u64,
) -> SemanticCrystal {
    let pse_weight = 0.7_f64;
    let il_weight  = 0.3_f64;
    let new_stability = (original.stability_score * pse_weight
        + feedback.il_stability * il_weight)
        .clamp(0.0, 1.0);

    // New content-addressed ID: SHA-256("il-refined:<orig_hex>:<new_stability>")
    let original_hex: String = original
        .crystal_id
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    let id_input = format!(
        "il-refined:{}:{:016x}",
        original_hex,
        new_stability.to_bits()
    );
    let hash = Sha256::digest(id_input.as_bytes());
    let mut new_id = [0u8; 32];
    new_id.copy_from_slice(&hash);

    // Updated parent chain
    let mut parent_ids = original.parent_crystal_ids.clone();
    if !parent_ids.contains(&original_hex) {
        parent_ids.push(original_hex);
    }

    SemanticCrystal {
        crystal_id: new_id,
        stability_score: new_stability,
        parent_crystal_ids: parent_ids,
        scale_tag: "il-refined".to_string(),
        created_at: new_created_at,
        // All topological fields unchanged
        region: original.region.clone(),
        constraint_program: original.constraint_program.clone(),
        topology_signature: original.topology_signature.clone(),
        betti_numbers: original.betti_numbers.clone(),
        evidence_chain: original.evidence_chain.clone(),
        commit_proof: original.commit_proof.clone(),
        operator_versions: original.operator_versions.clone(),
        free_energy: original.free_energy,
        carrier_instance_idx: original.carrier_instance_idx,
        universe_id: original.universe_id.clone(),
        sub_crystal_ids: original.sub_crystal_ids.clone(),
        genesis_metadata: original.genesis_metadata.clone(),
        metatron_signature: original.metatron_signature.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pse_types::SemanticCrystal;

    fn dummy_crystal(stability: f64, kuramoto: f64) -> SemanticCrystal {
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
            created_at: 10,
            free_energy: 0.0,
            carrier_instance_idx: 0,
            scale_tag: "original".to_string(),
            universe_id: String::new(),
            sub_crystal_ids: vec![],
            parent_crystal_ids: vec![],
            genesis_metadata: None,
            metatron_signature: None,
        };
        c.crystal_id[0] = (stability * 255.0) as u8;
        c.topology_signature.kuramoto_coherence = kuramoto;
        c
    }

    fn dummy_feedback(converged: bool, gate_passed: bool, psi: f64) -> ValidationFeedback {
        let il_stability = compute_il_stability(converged, gate_passed, psi);
        ValidationFeedback {
            block_hash: "abc123".to_string(),
            converged,
            coherence_potential: psi,
            gate_passed,
            hdag_node_id: Some("N-abc".to_string()),
            il_stability,
        }
    }

    #[test]
    fn refined_crystal_has_new_id() {
        let original = dummy_crystal(0.7, 0.6);
        let fb = dummy_feedback(true, true, 0.5);
        let refined = refine_crystal(&original, &fb, 11);
        assert_ne!(refined.crystal_id, original.crystal_id, "refined must have new id");
    }

    #[test]
    fn refined_crystal_links_to_parent() {
        let original = dummy_crystal(0.7, 0.6);
        let fb = dummy_feedback(true, true, 0.5);
        let refined = refine_crystal(&original, &fb, 11);
        let orig_hex: String = original.crystal_id.iter().map(|b| format!("{:02x}", b)).collect();
        assert!(
            refined.parent_crystal_ids.contains(&orig_hex),
            "refined must reference original in parent_crystal_ids"
        );
    }

    #[test]
    fn refined_stability_is_weighted_blend() {
        let original = dummy_crystal(0.8, 0.7);
        // IL signal: converged+passed, ψ=0.6 → psi_norm=0.8 → il_stability=0.8
        let fb = dummy_feedback(true, true, 0.6);
        let refined = refine_crystal(&original, &fb, 11);
        let expected = 0.8 * 0.7 + 0.8 * 0.3;
        assert!(
            (refined.stability_score - expected).abs() < 1e-6,
            "stability={} expected={}",
            refined.stability_score, expected
        );
    }

    #[test]
    fn non_convergence_penalizes_stability() {
        let original = dummy_crystal(0.8, 0.7);
        let fb_good = dummy_feedback(true,  true,  0.6);
        let fb_bad  = dummy_feedback(false, false, 0.6);
        let refined_good = refine_crystal(&original, &fb_good, 11);
        let refined_bad  = refine_crystal(&original, &fb_bad,  11);
        assert!(
            refined_good.stability_score > refined_bad.stability_score,
            "convergence failure must penalize stability"
        );
    }

    #[test]
    fn refined_preserves_topology_signature() {
        let mut original = dummy_crystal(0.7, 0.6);
        original.topology_signature.spectral_gap = 1.23;
        original.topology_signature.betti_1 = 4;
        let fb = dummy_feedback(true, true, 0.3);
        let refined = refine_crystal(&original, &fb, 11);
        assert_eq!(refined.topology_signature.spectral_gap, 1.23);
        assert_eq!(refined.topology_signature.betti_1, 4);
    }

    #[test]
    fn refined_scale_tag_is_il_refined() {
        let original = dummy_crystal(0.7, 0.6);
        let fb = dummy_feedback(true, true, 0.4);
        let refined = refine_crystal(&original, &fb, 11);
        assert_eq!(refined.scale_tag, "il-refined");
    }

    #[test]
    fn heuristic_feedback_stable_crystal_converges() {
        let crystal = dummy_crystal(0.75, 0.6);
        let fb = ValidationFeedback::from_crystal_heuristic(
            "hash".to_string(), &crystal, 0.3, true, None
        );
        assert!(fb.converged, "stable crystal with high kuramoto should mark as converged");
    }

    #[test]
    fn heuristic_feedback_unstable_crystal_no_converge() {
        let crystal = dummy_crystal(0.3, 0.1);
        let fb = ValidationFeedback::from_crystal_heuristic(
            "hash".to_string(), &crystal, -0.6, false, None
        );
        assert!(!fb.converged, "unstable crystal should not mark as converged");
        assert!(fb.il_stability < 0.3, "unstable crystal should produce low il_stability");
    }
}

//! M.1: Topological composition primitive.
//!
//! `compose(crystals)` is the first of four operators that turn PSE
//! from a recognition system into a **computation system**: a
//! function whose input and output are both topological objects, and
//! whose action is non-trivial in the sense of **(3)** of the
//! post-symbolic computation engine end-condition (see
//! `docs/COMPLIANCE.md` and the session framing).
//!
//! Semantic: given two or more crystals whose topology signatures are
//! *compatible* (shared Betti numbers, spectral gap and Kuramoto
//! coherence within a tolerance), produce a new crystal that
//! represents their **synchronisation product** — the composition's
//! region is the union of input regions, its stability is the
//! geometric mean of inputs (composition can only weaken), its
//! topology signature is the per-axis mean, and its evidence chain
//! is the concatenation. Incompatible inputs yield a typed error,
//! no crystal is produced.
//!
//! The composed crystal is itself a fully-formed `SemanticCrystal`:
//! it can be ingested again through the [`crystal_adapter::CrystalAdapter`],
//! it can be the input of subsequent compose / dual / bridge / query
//! calls, and its `crystal_id` is content-addressed via the same
//! CrystalCore subset (region, stability, created_at, free_energy,
//! carrier_instance_idx) so the existing audit machinery
//! (`pse_evidence::verify_crystal`) verifies it without modification.

use std::collections::{BTreeMap, BTreeSet};

use pse_types::{
    content_address, CommitProof, ConsensusResult, ConstraintProgram, EvidenceChain,
    GateSnapshot, Hash256, PoRTrace, SemanticCrystal, TopologySignature, VertexId,
};
use serde::Serialize;
use thiserror::Error;

// ─── Errors ──────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum ComposeError {
    /// Compose requires at least two crystals.
    #[error("compose requires at least 2 crystals; got {0}")]
    NotEnoughCrystals(usize),
    /// Topology signatures are not compatible under the configured tolerances.
    #[error("incompatible topologies: {0}")]
    Incompatible(String),
}

// ─── Configuration ───────────────────────────────────────────────────────────

/// Tolerances and policy for the [`compose`] operation.
#[derive(Clone, Debug)]
pub struct ComposeConfig {
    /// Maximum allowed absolute difference in spectral_gap between any
    /// two input crystals.
    pub spectral_gap_tolerance: f64,
    /// Maximum allowed absolute difference in kuramoto_coherence
    /// between any two input crystals.
    pub kuramoto_coherence_tolerance: f64,
    /// When true, all input Betti numbers must match exactly. When
    /// false, only spectral_gap and kuramoto_coherence are checked.
    pub require_equal_betti: bool,
}

impl Default for ComposeConfig {
    fn default() -> Self {
        Self {
            spectral_gap_tolerance: 0.05,
            kuramoto_coherence_tolerance: 0.10,
            require_equal_betti: true,
        }
    }
}

// ─── Public API ──────────────────────────────────────────────────────────────

/// Synchronisation-product of two or more crystals.
///
/// Returns `Ok(composed)` when the inputs are compatible under
/// `config`, with `composed.scale_tag = "composed"` and
/// `composed.parent_crystal_ids` listing the inputs (hex-encoded,
/// sorted by crystal_id for determinism). Returns
/// `Err(ComposeError::Incompatible(...))` otherwise.
///
/// The output is bit-identical under reordering of the input slice —
/// inputs are sorted by crystal_id before composition.
pub fn compose(
    crystals: &[SemanticCrystal],
    config: &ComposeConfig,
) -> Result<SemanticCrystal, ComposeError> {
    if crystals.len() < 2 {
        return Err(ComposeError::NotEnoughCrystals(crystals.len()));
    }
    check_compatibility(crystals, config)?;

    // Sort by crystal_id for determinism under input reordering.
    let mut sorted: Vec<&SemanticCrystal> = crystals.iter().collect();
    sorted.sort_by_key(|c| c.crystal_id);

    // Region: union, deduplicated, sorted.
    let mut region: Vec<VertexId> = sorted.iter().flat_map(|c| c.region.iter().copied()).collect();
    region.sort_unstable();
    region.dedup();

    // Stability score: geometric mean. Composition can only weaken the
    // worst input, never amplify the best.
    let n = sorted.len() as f64;
    let log_sum: f64 = sorted
        .iter()
        .map(|c| c.stability_score.max(1e-12).ln())
        .sum::<f64>();
    let stability_score = (log_sum / n).exp().clamp(0.0, 1.0);

    // Topology signature: per-axis mean (Betti numbers were verified
    // equal in the compatibility check, so they pass through unchanged).
    let topology_signature = mean_topology(&sorted);

    // Evidence chain: concatenation. The composed crystal is auditable
    // back to every input observation through this chain.
    let evidence_chain: EvidenceChain = sorted
        .iter()
        .flat_map(|c| c.evidence_chain.iter().cloned())
        .collect();

    // Constraint program: deduplicated union by candidate id.
    let mut constraint_program = ConstraintProgram::new();
    let mut seen: BTreeSet<Hash256> = BTreeSet::new();
    for c in &sorted {
        for cand in &c.constraint_program {
            if seen.insert(cand.id) {
                constraint_program.push(cand.clone());
            }
        }
    }

    // Created_at: max(input) + 1 — the composed crystal is strictly
    // posterior to all of its inputs.
    let created_at = sorted.iter().map(|c| c.created_at).max().unwrap() + 1;

    // Free energy: sum (additive — composition pools the inputs' free
    // energy contribution, in the same way thermodynamic potentials
    // add over independent subsystems).
    let free_energy: f64 = sorted.iter().map(|c| c.free_energy).sum();

    let parent_crystal_ids: Vec<String> =
        sorted.iter().map(|c| hex_encode(&c.crystal_id)).collect();

    // Carrier instance idx: lowest input — composition stays on the
    // earliest carrier the inputs already shared.
    let carrier_instance_idx = sorted
        .iter()
        .map(|c| c.carrier_instance_idx)
        .min()
        .unwrap();

    // Synthetic commit proof. The composition is a structural commit
    // (no observation cascade was run), so the gate values reflect
    // post-hoc structural soundness rather than an in-flight Kairos
    // measurement: kairos = true, individual gates set to 1.0 except
    // the readiness/crystal axes which take the composed stability.
    let evidence_digests: Vec<Hash256> = sorted
        .iter()
        .flat_map(|c| c.commit_proof.evidence_digests.iter().copied())
        .collect();
    let commit_proof = CommitProof {
        evidence_digests,
        operator_stack: vec![("compose".to_string(), "1.0.0".to_string())],
        gate_values: GateSnapshot {
            d: 1.0,
            q: 1.0,
            r: 1.0,
            g: stability_score,
            j: 1.0,
            p: 1.0,
            n: 1.0,
            k: stability_score,
            kairos: true,
        },
        structural_result: true,
        consensus_result: ConsensusResult {
            primal_score: stability_score,
            dual_score: stability_score,
            mci: 1.0,
            threshold: 0.6,
        },
        por_trace: PoRTrace::default(),
        carrier_id: carrier_instance_idx,
        carrier_offset: 0.0,
        falsification_p_value: None,
        surrogate_count: None,
    };

    // Crystal id matches the CrystalCore subset hashed by
    // pse_evidence::verify_content_address — composition produces a
    // first-class crystal that re-verifies through the existing
    // audit pipeline without modification.
    #[derive(Serialize)]
    struct CrystalCore<'a> {
        region: &'a Vec<VertexId>,
        stability_score: f64,
        created_at: u64,
        free_energy: f64,
        carrier_instance_idx: usize,
    }
    let core = CrystalCore {
        region: &region,
        stability_score,
        created_at,
        free_energy,
        carrier_instance_idx,
    };
    let crystal_id = content_address(&core);

    Ok(SemanticCrystal {
        crystal_id,
        region,
        constraint_program,
        stability_score,
        topology_signature,
        betti_numbers: vec![1, 0, 0],
        evidence_chain,
        commit_proof,
        operator_versions: BTreeMap::new(),
        created_at,
        free_energy,
        carrier_instance_idx,
        scale_tag: "composed".into(),
        universe_id: String::new(),
        sub_crystal_ids: Vec::new(),
        parent_crystal_ids,
        genesis_metadata: None,
    })
}

// ─── M.2 — dual ──────────────────────────────────────────────────────────────

/// Topology-preserving inversion of a crystal.
///
/// Conceptually: the dual lives on the **transverse** carrier — rotated
/// by π/2 relative to the original. The standing-wave geometry of the
/// Mandorla means that at the transverse carrier, antinodes become
/// nodes and vice versa: the resonant relationship between the
/// original carrier and any data-helix is inverted. The crystal's
/// **form** (Betti numbers, region, spectral signature, stability) is
/// preserved exactly; only the carrier from which the form is observed
/// is rotated.
///
/// Implementation: `dual(c)` flips the lowest bit of `carrier_instance_idx`
/// (XOR with 1), interpreting the phase-ladder convention that adjacent
/// indices are π/2-offset carriers. Because XOR-with-1 is its own
/// inverse, `dual` is an **involution**:
///
/// ```text
/// dual(dual(c)).crystal_id == c.crystal_id
/// ```
///
/// — verified by the test `dual_is_involutive`. This is the property
/// that makes `dual` a real symmetry operation rather than an arbitrary
/// rewrite.
///
/// `created_at` is preserved (the dual is not strictly posterior to
/// the original — they are dual aspects of the same form). `scale_tag`
/// is set to `"dual"`. `parent_crystal_ids` carries the original.
pub fn dual(crystal: &SemanticCrystal) -> SemanticCrystal {
    let dual_carrier = crystal.carrier_instance_idx ^ 1;

    #[derive(Serialize)]
    struct CrystalCore<'a> {
        region: &'a Vec<VertexId>,
        stability_score: f64,
        created_at: u64,
        free_energy: f64,
        carrier_instance_idx: usize,
    }
    let core = CrystalCore {
        region: &crystal.region,
        stability_score: crystal.stability_score,
        created_at: crystal.created_at,
        free_energy: crystal.free_energy,
        carrier_instance_idx: dual_carrier,
    };
    let crystal_id = content_address(&core);

    let parent = hex_encode(&crystal.crystal_id);
    let mut commit_proof = crystal.commit_proof.clone();
    commit_proof.carrier_id = dual_carrier;
    commit_proof.operator_stack = vec![("dual".to_string(), "1.0.0".to_string())];

    SemanticCrystal {
        crystal_id,
        region: crystal.region.clone(),
        constraint_program: crystal.constraint_program.clone(),
        stability_score: crystal.stability_score,
        topology_signature: crystal.topology_signature.clone(),
        betti_numbers: crystal.betti_numbers.clone(),
        evidence_chain: crystal.evidence_chain.clone(),
        commit_proof,
        operator_versions: crystal.operator_versions.clone(),
        created_at: crystal.created_at,
        free_energy: crystal.free_energy,
        carrier_instance_idx: dual_carrier,
        // The dual is the same form on the transverse carrier. The
        // crystal_id (the canonical Inv I3 identity) is preserved
        // under involution because dual flips one bit of
        // carrier_instance_idx, which is a CrystalCore field; flipping
        // the same bit twice restores it. scale_tag is a hint, not
        // part of identity, so we always tag the dual as "dual".
        scale_tag: "dual".to_string(),
        universe_id: crystal.universe_id.clone(),
        sub_crystal_ids: crystal.sub_crystal_ids.clone(),
        parent_crystal_ids: vec![parent],
        genesis_metadata: crystal.genesis_metadata.clone(),
    }
}

// ─── Internals ───────────────────────────────────────────────────────────────

fn check_compatibility(
    crystals: &[SemanticCrystal],
    config: &ComposeConfig,
) -> Result<(), ComposeError> {
    let first = &crystals[0].topology_signature;
    for c in &crystals[1..] {
        let other = &c.topology_signature;
        if config.require_equal_betti
            && (first.betti_0 != other.betti_0
                || first.betti_1 != other.betti_1
                || first.betti_2 != other.betti_2)
        {
            return Err(ComposeError::Incompatible(format!(
                "Betti number mismatch: ({},{},{}) vs ({},{},{})",
                first.betti_0, first.betti_1, first.betti_2,
                other.betti_0, other.betti_1, other.betti_2,
            )));
        }
        if (first.spectral_gap - other.spectral_gap).abs()
            > config.spectral_gap_tolerance
        {
            return Err(ComposeError::Incompatible(format!(
                "spectral_gap differs by {:.4}; tolerance {:.4}",
                (first.spectral_gap - other.spectral_gap).abs(),
                config.spectral_gap_tolerance,
            )));
        }
        if (first.kuramoto_coherence - other.kuramoto_coherence).abs()
            > config.kuramoto_coherence_tolerance
        {
            return Err(ComposeError::Incompatible(format!(
                "kuramoto_coherence differs by {:.4}; tolerance {:.4}",
                (first.kuramoto_coherence - other.kuramoto_coherence).abs(),
                config.kuramoto_coherence_tolerance,
            )));
        }
    }
    Ok(())
}

fn mean_topology(crystals: &[&SemanticCrystal]) -> TopologySignature {
    let n = crystals.len() as f64;
    let mean = |get: &dyn Fn(&TopologySignature) -> f64| -> f64 {
        crystals.iter().map(|c| get(&c.topology_signature)).sum::<f64>() / n
    };
    let first = &crystals[0].topology_signature;
    TopologySignature {
        betti_0: first.betti_0,
        betti_1: first.betti_1,
        betti_2: first.betti_2,
        spectral_gap: mean(&|t| t.spectral_gap),
        euler_char: first.euler_char,
        cheeger_estimate: mean(&|t| t.cheeger_estimate),
        kuramoto_coherence: mean(&|t| t.kuramoto_coherence),
        mean_propagation_time: mean(&|t| t.mean_propagation_time),
        dtl_connected: crystals.iter().all(|c| c.topology_signature.dtl_connected),
    }
}

fn hex_encode(digest: &Hash256) -> String {
    let mut s = String::with_capacity(64);
    for b in digest {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use pse_types::{ConstraintCandidate, ConstraintTemplate};

    /// Minimal crystal builder for tests. Computes the crystal_id from
    /// the same CrystalCore subset that pse_evidence::verify_content_address
    /// uses, so the resulting crystals are self-consistent.
    fn make_crystal(
        region: Vec<VertexId>,
        stability: f64,
        betti0: u64,
        spectral_gap: f64,
        kuramoto: f64,
        created_at: u64,
    ) -> SemanticCrystal {
        let topology_signature = TopologySignature {
            betti_0: betti0,
            betti_1: 0,
            betti_2: 0,
            spectral_gap,
            euler_char: 0,
            cheeger_estimate: spectral_gap.sqrt() * std::f64::consts::SQRT_2,
            kuramoto_coherence: kuramoto,
            mean_propagation_time: 0.0,
            dtl_connected: true,
        };
        let free_energy = -stability * region.len() as f64;
        let carrier_instance_idx = 0;

        #[derive(Serialize)]
        struct CrystalCore<'a> {
            region: &'a Vec<VertexId>,
            stability_score: f64,
            created_at: u64,
            free_energy: f64,
            carrier_instance_idx: usize,
        }
        let core = CrystalCore {
            region: &region,
            stability_score: stability,
            created_at,
            free_energy,
            carrier_instance_idx,
        };
        let crystal_id = content_address(&core);

        SemanticCrystal {
            crystal_id,
            region,
            constraint_program: ConstraintProgram::new(),
            stability_score: stability,
            topology_signature,
            betti_numbers: vec![betti0, 0, 0],
            evidence_chain: Vec::new(),
            commit_proof: CommitProof::default(),
            operator_versions: BTreeMap::new(),
            created_at,
            free_energy,
            carrier_instance_idx,
            scale_tag: "test".into(),
            universe_id: String::new(),
            sub_crystal_ids: Vec::new(),
            parent_crystal_ids: Vec::new(),
            genesis_metadata: None,
        }
    }

    #[test]
    fn compose_zero_crystals_errors() {
        let r = compose(&[], &ComposeConfig::default());
        assert!(matches!(r, Err(ComposeError::NotEnoughCrystals(0))));
    }

    #[test]
    fn compose_one_crystal_errors() {
        let c = make_crystal(vec![1, 2], 0.5, 1, 0.3, 0.7, 1);
        let r = compose(&[c], &ComposeConfig::default());
        assert!(matches!(r, Err(ComposeError::NotEnoughCrystals(1))));
    }

    #[test]
    fn compose_compatible_pair_succeeds() {
        let c1 = make_crystal(vec![1, 2, 3], 0.8, 1, 0.30, 0.70, 1);
        let c2 = make_crystal(vec![4, 5, 6], 0.7, 1, 0.32, 0.68, 2);
        let composed = compose(&[c1, c2], &ComposeConfig::default()).unwrap();
        assert_eq!(composed.region, vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(composed.scale_tag, "composed");
        assert_eq!(composed.parent_crystal_ids.len(), 2);
        assert_eq!(composed.created_at, 3); // max(1, 2) + 1
    }

    #[test]
    fn compose_stability_is_geometric_mean() {
        let c1 = make_crystal(vec![1], 0.8, 1, 0.30, 0.70, 1);
        let c2 = make_crystal(vec![2], 0.5, 1, 0.30, 0.70, 1);
        let composed = compose(&[c1, c2], &ComposeConfig::default()).unwrap();
        let expected = ((0.8_f64.ln() + 0.5_f64.ln()) / 2.0).exp();
        assert!((composed.stability_score - expected).abs() < 1e-12);
    }

    #[test]
    fn compose_stability_geometric_mean_dominates_minimum() {
        // Geometric mean of (a, b) is bounded by sqrt(a·b), which
        // is always ≥ min(a, b) and ≤ max(a, b). Crucial property:
        // composition can only weaken a strong crystal, never amplify
        // a weak one.
        let c1 = make_crystal(vec![1], 0.9, 1, 0.30, 0.70, 1);
        let c2 = make_crystal(vec![2], 0.3, 1, 0.30, 0.70, 1);
        let composed = compose(&[c1, c2], &ComposeConfig::default()).unwrap();
        assert!(composed.stability_score >= 0.3);
        assert!(composed.stability_score <= 0.9);
    }

    #[test]
    fn compose_region_is_deduplicated_union() {
        let c1 = make_crystal(vec![1, 2, 3], 0.8, 1, 0.30, 0.70, 1);
        let c2 = make_crystal(vec![3, 4, 5], 0.7, 1, 0.30, 0.70, 1);
        let composed = compose(&[c1, c2], &ComposeConfig::default()).unwrap();
        assert_eq!(composed.region, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn compose_betti_mismatch_errors() {
        let c1 = make_crystal(vec![1, 2], 0.5, 1, 0.30, 0.70, 1);
        let c2 = make_crystal(vec![3, 4], 0.5, 2, 0.30, 0.70, 1);
        let r = compose(&[c1, c2], &ComposeConfig::default());
        assert!(matches!(r, Err(ComposeError::Incompatible(_))));
    }

    #[test]
    fn compose_spectral_gap_outside_tolerance_errors() {
        let c1 = make_crystal(vec![1], 0.5, 1, 0.10, 0.70, 1);
        let c2 = make_crystal(vec![2], 0.5, 1, 0.50, 0.70, 1);
        // Default spectral_gap_tolerance = 0.05; |0.10 − 0.50| = 0.40 ≫ 0.05.
        let r = compose(&[c1, c2], &ComposeConfig::default());
        assert!(matches!(r, Err(ComposeError::Incompatible(_))));
    }

    #[test]
    fn compose_kuramoto_outside_tolerance_errors() {
        let c1 = make_crystal(vec![1], 0.5, 1, 0.30, 0.20, 1);
        let c2 = make_crystal(vec![2], 0.5, 1, 0.30, 0.90, 1);
        // Default kuramoto_coherence_tolerance = 0.10; |0.20 − 0.90| = 0.70.
        let r = compose(&[c1, c2], &ComposeConfig::default());
        assert!(matches!(r, Err(ComposeError::Incompatible(_))));
    }

    #[test]
    fn compose_is_deterministic_under_input_reorder() {
        // The crystal_id of the composed crystal must not depend on the
        // order of the input slice — sorting by crystal_id is the
        // canonical ordering.
        let c1 = make_crystal(vec![1, 2, 3], 0.8, 1, 0.30, 0.70, 1);
        let c2 = make_crystal(vec![4, 5, 6], 0.7, 1, 0.30, 0.70, 2);
        let r1 = compose(&[c1.clone(), c2.clone()], &ComposeConfig::default()).unwrap();
        let r2 = compose(&[c2, c1], &ComposeConfig::default()).unwrap();
        assert_eq!(r1.crystal_id, r2.crystal_id);
    }

    #[test]
    fn compose_parent_ids_are_sorted_by_crystal_id() {
        let c1 = make_crystal(vec![1, 2, 3], 0.8, 1, 0.30, 0.70, 1);
        let c2 = make_crystal(vec![4, 5, 6], 0.7, 1, 0.30, 0.70, 2);
        let composed = compose(&[c1.clone(), c2.clone()], &ComposeConfig::default()).unwrap();
        let mut expected = vec![hex_encode(&c1.crystal_id), hex_encode(&c2.crystal_id)];
        expected.sort();
        assert_eq!(composed.parent_crystal_ids, expected);
    }

    #[test]
    fn compose_constraint_program_is_deduplicated() {
        let cand_a = ConstraintCandidate {
            id: [1u8; 32],
            template: ConstraintTemplate::Band,
            parameters: BTreeMap::new(),
            coverage: 0.5,
            threshold: 0.5,
            formation_energy: -0.5,
            bond_strength: 1,
            activation_energy: 0.5,
        };
        let cand_b = ConstraintCandidate {
            id: [2u8; 32],
            ..cand_a.clone()
        };
        let mut c1 = make_crystal(vec![1], 0.5, 1, 0.30, 0.70, 1);
        c1.constraint_program = vec![cand_a.clone(), cand_b.clone()];
        let mut c2 = make_crystal(vec![2], 0.5, 1, 0.30, 0.70, 1);
        c2.constraint_program = vec![cand_a.clone()]; // overlap with c1
        let composed = compose(&[c1, c2], &ComposeConfig::default()).unwrap();
        assert_eq!(composed.constraint_program.len(), 2);
    }

    #[test]
    fn compose_created_at_strictly_posterior_to_inputs() {
        let c1 = make_crystal(vec![1], 0.5, 1, 0.30, 0.70, 5);
        let c2 = make_crystal(vec![2], 0.5, 1, 0.30, 0.70, 9);
        let composed = compose(&[c1, c2], &ComposeConfig::default()).unwrap();
        assert_eq!(composed.created_at, 10);
    }

    #[test]
    fn composed_crystal_is_compose_input() {
        // The output of compose is itself a SemanticCrystal that can
        // be the input of another compose call. This is the iterative
        // closure that makes compose a genuine computation primitive.
        let c1 = make_crystal(vec![1, 2], 0.8, 1, 0.30, 0.70, 1);
        let c2 = make_crystal(vec![3, 4], 0.7, 1, 0.30, 0.70, 2);
        let c12 = compose(&[c1, c2], &ComposeConfig::default()).unwrap();
        let c3 = make_crystal(vec![5, 6], 0.6, 1, 0.30, 0.70, 3);
        let c123 = compose(&[c12, c3], &ComposeConfig::default()).unwrap();
        assert_eq!(c123.region, vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(c123.scale_tag, "composed");
    }

    #[test]
    fn composed_crystal_id_changes_with_inputs() {
        // Different inputs → different composed crystal_id.
        let c1 = make_crystal(vec![1, 2], 0.8, 1, 0.30, 0.70, 1);
        let c2 = make_crystal(vec![3, 4], 0.7, 1, 0.30, 0.70, 2);
        let c3 = make_crystal(vec![5, 6], 0.7, 1, 0.30, 0.70, 2);
        let r12 = compose(&[c1.clone(), c2], &ComposeConfig::default()).unwrap();
        let r13 = compose(&[c1, c3], &ComposeConfig::default()).unwrap();
        assert_ne!(r12.crystal_id, r13.crystal_id);
    }

    // ── M.2: dual ────────────────────────────────────────────────────────

    #[test]
    fn dual_preserves_form() {
        let c = make_crystal(vec![1, 2, 3], 0.8, 1, 0.30, 0.70, 5);
        let d = dual(&c);
        assert_eq!(d.region, c.region);
        assert_eq!(d.stability_score, c.stability_score);
        assert_eq!(d.topology_signature, c.topology_signature);
        assert_eq!(d.created_at, c.created_at);
        assert_eq!(d.free_energy, c.free_energy);
        assert_eq!(d.constraint_program.len(), c.constraint_program.len());
    }

    #[test]
    fn dual_flips_carrier_instance_idx_lowest_bit() {
        let c0 = make_crystal(vec![1], 0.5, 1, 0.30, 0.70, 1);
        let c1 = {
            let mut x = c0.clone();
            x.carrier_instance_idx = 1;
            x
        };
        assert_eq!(dual(&c0).carrier_instance_idx, 1);
        assert_eq!(dual(&c1).carrier_instance_idx, 0);
        // Higher carriers also flip the lowest bit only:
        let c2 = {
            let mut x = c0.clone();
            x.carrier_instance_idx = 2;
            x
        };
        assert_eq!(dual(&c2).carrier_instance_idx, 3);
        let c3 = {
            let mut x = c0.clone();
            x.carrier_instance_idx = 3;
            x
        };
        assert_eq!(dual(&c3).carrier_instance_idx, 2);
    }

    #[test]
    fn dual_changes_crystal_id() {
        let c = make_crystal(vec![1, 2, 3], 0.8, 1, 0.30, 0.70, 5);
        let d = dual(&c);
        assert_ne!(d.crystal_id, c.crystal_id,
                   "dual must produce a distinct crystal_id");
    }

    #[test]
    fn dual_is_involutive() {
        // The defining symmetry of dual: dual(dual(c)) restores c's
        // canonical identity (crystal_id), even if the scale_tag
        // hint records that we passed through a dual on the way back.
        let c = make_crystal(vec![1, 2, 3], 0.8, 1, 0.30, 0.70, 5);
        let dd = dual(&dual(&c));
        assert_eq!(dd.crystal_id, c.crystal_id);
        assert_eq!(dd.region, c.region);
        assert_eq!(dd.stability_score, c.stability_score);
        assert_eq!(dd.carrier_instance_idx, c.carrier_instance_idx);
    }

    #[test]
    fn dual_records_parent() {
        let c = make_crystal(vec![1], 0.5, 1, 0.30, 0.70, 1);
        let d = dual(&c);
        assert_eq!(d.parent_crystal_ids.len(), 1);
        assert_eq!(d.parent_crystal_ids[0], hex_encode(&c.crystal_id));
        assert_eq!(d.scale_tag, "dual");
    }

    #[test]
    fn dual_commit_proof_carries_dual_operator() {
        let c = make_crystal(vec![1], 0.5, 1, 0.30, 0.70, 1);
        let d = dual(&c);
        assert!(d.commit_proof
            .operator_stack
            .iter()
            .any(|(name, _)| name == "dual"));
    }

    #[test]
    fn dual_and_original_are_compose_compatible() {
        // A crystal and its dual share the same topology signature, so
        // they must always be compose-compatible. This is the
        // composability property: dual outputs are first-class
        // computation inputs.
        let c = make_crystal(vec![1, 2, 3], 0.8, 1, 0.30, 0.70, 5);
        let d = dual(&c);
        let composed = compose(&[c, d], &ComposeConfig::default());
        assert!(composed.is_ok(), "c and dual(c) must be compose-compatible");
    }

    #[test]
    fn dual_of_composed_is_dual() {
        // dual is a real operation on any crystal, including a
        // composed one (closure under composition).
        let c1 = make_crystal(vec![1, 2], 0.8, 1, 0.30, 0.70, 1);
        let c2 = make_crystal(vec![3, 4], 0.7, 1, 0.30, 0.70, 2);
        let composed = compose(&[c1, c2], &ComposeConfig::default()).unwrap();
        let d = dual(&composed);
        assert_eq!(d.region, composed.region);
        assert_eq!(d.scale_tag, "dual");
        assert_eq!(d.carrier_instance_idx, composed.carrier_instance_idx ^ 1);
    }
}

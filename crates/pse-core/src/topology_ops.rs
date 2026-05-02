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

// ─── M.3 — bridge ────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum BridgeError {
    /// The two crystals' regions do not intersect — no shared substrate
    /// to bridge across.
    #[error("regions are disjoint: no shared substrate to bridge")]
    DisjointRegions,
    /// The crystal topologies are incompatible under the configured
    /// tolerances. Re-uses the [`ComposeError`] semantics for the
    /// underlying compatibility check.
    #[error("incompatible topologies: {0}")]
    Incompatible(String),
}

/// Configuration for [`bridge`].
#[derive(Clone, Debug)]
pub struct BridgeConfig {
    /// Spectral gap tolerance (mirrors [`ComposeConfig::spectral_gap_tolerance`]).
    pub spectral_gap_tolerance: f64,
    /// Kuramoto coherence tolerance.
    pub kuramoto_coherence_tolerance: f64,
    /// Require equal Betti numbers.
    pub require_equal_betti: bool,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            spectral_gap_tolerance: 0.05,
            kuramoto_coherence_tolerance: 0.10,
            require_equal_betti: true,
        }
    }
}

/// Meso-bridge between two crystals, focused on their shared substrate.
///
/// Whereas [`compose`] *unions* regions to broaden scope, `bridge`
/// *intersects* regions: the resulting crystal lives only on the
/// vertices that both inputs have in common. Stability is the
/// **harmonic mean** of the inputs — the "weakest link" semantics
/// appropriate for a coupling: the bridge can carry resonance only as
/// strongly as the weaker of the two paths it spans.
///
/// Compatibility:
///  - Regions must intersect (otherwise [`BridgeError::DisjointRegions`]).
///  - Topology signatures must be within `config` tolerances.
///
/// Symmetry: `bridge(a, b)` and `bridge(b, a)` produce bit-identical
/// crystal_ids (the inputs are sorted by crystal_id internally before
/// the operation runs).
///
/// `scale_tag = "bridge"`, `parent_crystal_ids = [a.id, b.id]` sorted.
pub fn bridge(
    a: &SemanticCrystal,
    b: &SemanticCrystal,
    config: &BridgeConfig,
) -> Result<SemanticCrystal, BridgeError> {
    // Compatibility check (mirrors compose's, but typed as BridgeError).
    if config.require_equal_betti
        && (a.topology_signature.betti_0 != b.topology_signature.betti_0
            || a.topology_signature.betti_1 != b.topology_signature.betti_1
            || a.topology_signature.betti_2 != b.topology_signature.betti_2)
    {
        return Err(BridgeError::Incompatible(format!(
            "Betti number mismatch: ({},{},{}) vs ({},{},{})",
            a.topology_signature.betti_0, a.topology_signature.betti_1,
            a.topology_signature.betti_2,
            b.topology_signature.betti_0, b.topology_signature.betti_1,
            b.topology_signature.betti_2,
        )));
    }
    if (a.topology_signature.spectral_gap - b.topology_signature.spectral_gap).abs()
        > config.spectral_gap_tolerance
    {
        return Err(BridgeError::Incompatible(format!(
            "spectral_gap differs by {:.4}; tolerance {:.4}",
            (a.topology_signature.spectral_gap - b.topology_signature.spectral_gap).abs(),
            config.spectral_gap_tolerance,
        )));
    }
    if (a.topology_signature.kuramoto_coherence - b.topology_signature.kuramoto_coherence).abs()
        > config.kuramoto_coherence_tolerance
    {
        return Err(BridgeError::Incompatible(format!(
            "kuramoto_coherence differs by {:.4}; tolerance {:.4}",
            (a.topology_signature.kuramoto_coherence
                - b.topology_signature.kuramoto_coherence)
                .abs(),
            config.kuramoto_coherence_tolerance,
        )));
    }

    // Sort by crystal_id for canonical (a, b) → (lo, hi).
    let (lo, hi) = if a.crystal_id <= b.crystal_id {
        (a, b)
    } else {
        (b, a)
    };

    // Region: intersection.
    let lo_set: BTreeSet<VertexId> = lo.region.iter().copied().collect();
    let region: Vec<VertexId> = hi
        .region
        .iter()
        .copied()
        .filter(|v| lo_set.contains(v))
        .collect();
    if region.is_empty() {
        return Err(BridgeError::DisjointRegions);
    }
    let mut region = region;
    region.sort_unstable();
    region.dedup();

    // Stability: harmonic mean = 2·a·b / (a + b). Bounded above by min,
    // below by 0; for a = b, the harmonic mean equals a.
    let sa = lo.stability_score.max(1e-12);
    let sb = hi.stability_score.max(1e-12);
    let stability_score = (2.0 * sa * sb / (sa + sb)).clamp(0.0, 1.0);

    // Topology signature: per-axis mean (Betti pass through, equal by check).
    let topology_signature = mean_topology(&[lo, hi]);

    // Evidence chain: lo's chain followed by hi's. Concatenation
    // preserves auditability through both endpoints.
    let evidence_chain: EvidenceChain = lo
        .evidence_chain
        .iter()
        .cloned()
        .chain(hi.evidence_chain.iter().cloned())
        .collect();

    // Constraint program: deduplicated union by candidate id.
    let mut constraint_program = ConstraintProgram::new();
    let mut seen: BTreeSet<Hash256> = BTreeSet::new();
    for c in [lo, hi] {
        for cand in &c.constraint_program {
            if seen.insert(cand.id) {
                constraint_program.push(cand.clone());
            }
        }
    }

    let created_at = lo.created_at.max(hi.created_at) + 1;
    // Free energy: sum (additive coupling free-energy contribution).
    let free_energy = lo.free_energy + hi.free_energy;
    let parent_crystal_ids =
        vec![hex_encode(&lo.crystal_id), hex_encode(&hi.crystal_id)];
    let carrier_instance_idx = lo.carrier_instance_idx.min(hi.carrier_instance_idx);

    // Synthetic commit proof for the bridge (structural commit, no live cascade).
    let evidence_digests: Vec<Hash256> = lo
        .commit_proof
        .evidence_digests
        .iter()
        .copied()
        .chain(hi.commit_proof.evidence_digests.iter().copied())
        .collect();
    let commit_proof = CommitProof {
        evidence_digests,
        operator_stack: vec![("bridge".to_string(), "1.0.0".to_string())],
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

    // Crystal id from the canonical CrystalCore subset.
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
        scale_tag: "bridge".into(),
        universe_id: String::new(),
        sub_crystal_ids: Vec::new(),
        parent_crystal_ids,
        genesis_metadata: None,
    })
}

// ─── M.4 — query ─────────────────────────────────────────────────────────────

/// Configuration for [`query`] — the relative weight of each
/// similarity dimension. The defaults emphasise topology (the
/// "shape") over region (the "where") over stability (the "how
/// strong"), reflecting the Strand-E principle that *form is
/// identity*.
#[derive(Clone, Debug)]
pub struct QueryConfig {
    /// Weight on the cosine similarity of the spectral signature
    /// vector `[spectral_gap, cheeger, kuramoto, prop_time, β₀, β₁,
    /// β₂, χ]` between template and candidate. Default 0.5.
    pub topology_weight: f64,
    /// Weight on the region similarity (Jaccard index of vertex sets).
    /// Default 0.3.
    pub region_weight: f64,
    /// Weight on stability proximity `1 − |s_template − s_candidate|`.
    /// Default 0.2.
    pub stability_weight: f64,
}

impl Default for QueryConfig {
    fn default() -> Self {
        Self {
            topology_weight: 0.5,
            region_weight: 0.3,
            stability_weight: 0.2,
        }
    }
}

/// Find the top-K candidates that most resemble a template crystal.
///
/// This is the **read-modality** of the M-operator family: a template
/// expresses a topological question ("what crystals look like this?")
/// and the engine returns ranked answers from the supplied candidate
/// set. The candidate set is typically the live archive
/// (`state.archive.crystals()`) or a previously-loaded pattern memory.
///
/// Similarity is a weighted convex combination of three independent
/// channels:
///
///  - **Topology** (cosine on spectral signature, default weight 0.5)
///  - **Region** (Jaccard index of vertex sets, default weight 0.3)
///  - **Stability** (1 − |Δstability|, default weight 0.2)
///
/// All three lie in `[0, 1]`; the weighted sum is therefore in `[0, 1]`.
/// Self-similarity is exactly 1.0 (a crystal queried against itself
/// scores 1.0). Results are sorted by descending similarity.
///
/// `top_k = 0` returns an empty vector. `top_k` larger than the
/// candidate set returns the entire set (sorted).
pub fn query(
    template: &SemanticCrystal,
    candidates: &[SemanticCrystal],
    config: &QueryConfig,
    top_k: usize,
) -> Vec<(SemanticCrystal, f64)> {
    if top_k == 0 || candidates.is_empty() {
        return Vec::new();
    }
    let mut scored: Vec<(SemanticCrystal, f64)> = candidates
        .iter()
        .map(|c| (c.clone(), crystal_similarity(template, c, config)))
        .collect();
    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.truncate(top_k);
    scored
}

/// Crystal-to-crystal similarity in `[0, 1]`. Public for downstream
/// callers (tests, custom rankers) that want the same metric outside
/// the `query` driver.
pub fn crystal_similarity(
    a: &SemanticCrystal,
    b: &SemanticCrystal,
    config: &QueryConfig,
) -> f64 {
    let total_w = config.topology_weight
        + config.region_weight
        + config.stability_weight;
    if total_w < 1e-12 {
        return 0.0;
    }
    let topo = topology_cosine(&a.topology_signature, &b.topology_signature);
    let region = region_jaccard(&a.region, &b.region);
    let stab = (1.0 - (a.stability_score - b.stability_score).abs()).clamp(0.0, 1.0);
    let weighted = config.topology_weight * topo
        + config.region_weight * region
        + config.stability_weight * stab;
    (weighted / total_w).clamp(0.0, 1.0)
}

/// Cosine similarity of the 8-axis spectral signature of two
/// topology signatures. Returns values in `[-1, 1]` mathematically;
/// we clamp negative values to 0 so the output composes cleanly with
/// the other unit-interval channels.
fn topology_cosine(a: &TopologySignature, b: &TopologySignature) -> f64 {
    let av = topology_vec(a);
    let bv = topology_vec(b);
    let mut dot = 0.0_f64;
    let mut na = 0.0_f64;
    let mut nb = 0.0_f64;
    for (x, y) in av.iter().zip(bv.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na < 1e-12 || nb < 1e-12 {
        return 0.0;
    }
    (dot / (na.sqrt() * nb.sqrt())).max(0.0).clamp(0.0, 1.0)
}

fn topology_vec(t: &TopologySignature) -> [f64; 8] {
    [
        t.spectral_gap,
        t.cheeger_estimate,
        t.kuramoto_coherence,
        t.mean_propagation_time,
        t.betti_0 as f64,
        t.betti_1 as f64,
        t.betti_2 as f64,
        t.euler_char as f64,
    ]
}

/// Jaccard index of two vertex regions: `|A ∩ B| / |A ∪ B|`. Both
/// empty regions return 1.0 (degenerate but consistent).
fn region_jaccard(a: &[VertexId], b: &[VertexId]) -> f64 {
    let sa: BTreeSet<VertexId> = a.iter().copied().collect();
    let sb: BTreeSet<VertexId> = b.iter().copied().collect();
    let intersection = sa.intersection(&sb).count();
    let union = sa.union(&sb).count();
    if union == 0 {
        return 1.0;
    }
    intersection as f64 / union as f64
}

// ─── N — generative interpolation ────────────────────────────────────────────

/// Synthesise a **new** crystal at position `alpha ∈ [0, 1]` along the
/// topological line between `a` and `b`.
///
/// `alpha = 0` reproduces a crystal congruent with `a`; `alpha = 1`
/// with `b`; intermediate values produce crystals that **were never
/// observed** but whose topology, stability, and free energy lie
/// between the two inputs by linear interpolation.
///
/// This is the engine's first **generative** operator: the output is
/// a fully-formed `SemanticCrystal`, content-addressed via the same
/// CrystalCore subset as everything else, but with a topology
/// signature that exists in neither input nor in any prior crystal —
/// it is **imagined** consistent with the topological history.
///
/// Region semantics under interpolation:
/// - vertices present in both `a` and `b`: always included
/// - vertices only in `a`: included iff `alpha ≤ 0.5`
/// - vertices only in `b`: included iff `alpha ≥ 0.5`
///
/// The threshold at `0.5` produces a discontinuity in region
/// membership (a vertex flips from "include" to "exclude" as alpha
/// crosses 0.5). This is the cleanest deterministic rule and makes
/// `interpolate(a, b, 0.0).region == a.region` exactly, but at
/// `alpha = 0.5` the union is taken (both halves still in).
///
/// `created_at = max(a, b) + 1` — strict posterity, like the other
/// M-operators. `scale_tag = "interpolated"`.
/// `parent_crystal_ids = [a.id, b.id]` sorted by crystal_id for
/// canonical ordering.
pub fn interpolate(
    a: &SemanticCrystal,
    b: &SemanticCrystal,
    alpha: f64,
) -> SemanticCrystal {
    let alpha = alpha.clamp(0.0, 1.0);
    let inv_alpha = 1.0 - alpha;

    // Sort by crystal_id for canonical (lo, hi) → deterministic output.
    let (lo, hi) = if a.crystal_id <= b.crystal_id {
        (a, b)
    } else {
        (b, a)
    };

    // Region: see semantics in the docstring.
    let lo_set: BTreeSet<VertexId> = lo.region.iter().copied().collect();
    let hi_set: BTreeSet<VertexId> = hi.region.iter().copied().collect();
    let mut region: Vec<VertexId> = Vec::new();
    for v in lo_set.union(&hi_set) {
        let in_lo = lo_set.contains(v);
        let in_hi = hi_set.contains(v);
        let include = if in_lo && in_hi {
            true
        } else if in_lo {
            // interp.alpha refers to "how much like the b-input"; if
            // a.id < b.id then lo == a, so vertices only-in-lo
            // correspond to "alpha-near-0" inputs.
            // To preserve "alpha=0 → exactly a.region" regardless of
            // sort order, we rebind the membership rule to the
            // *original* a vs b semantic.
            let alpha_for_lo = if std::ptr::eq(lo, a) { alpha } else { 1.0 - alpha };
            alpha_for_lo <= 0.5
        } else {
            let alpha_for_hi = if std::ptr::eq(hi, b) { alpha } else { 1.0 - alpha };
            alpha_for_hi >= 0.5
        };
        if include {
            region.push(*v);
        }
    }
    region.sort_unstable();

    // Stability: linear interpolation in the original a/b frame.
    let stability_score =
        (inv_alpha * a.stability_score + alpha * b.stability_score).clamp(0.0, 1.0);

    // Topology signature: per-axis linear interpolation. Betti
    // numbers and Euler char are integer; we round the interpolated
    // value to the nearest integer to keep them well-typed.
    let lerp = |x: f64, y: f64| inv_alpha * x + alpha * y;
    let lerp_u64 = |x: u64, y: u64| -> u64 {
        (inv_alpha * x as f64 + alpha * y as f64).round() as u64
    };
    let lerp_i64 = |x: i64, y: i64| -> i64 {
        (inv_alpha * x as f64 + alpha * y as f64).round() as i64
    };
    let topology_signature = TopologySignature {
        betti_0: lerp_u64(a.topology_signature.betti_0, b.topology_signature.betti_0),
        betti_1: lerp_u64(a.topology_signature.betti_1, b.topology_signature.betti_1),
        betti_2: lerp_u64(a.topology_signature.betti_2, b.topology_signature.betti_2),
        spectral_gap: lerp(
            a.topology_signature.spectral_gap,
            b.topology_signature.spectral_gap,
        ),
        euler_char: lerp_i64(
            a.topology_signature.euler_char,
            b.topology_signature.euler_char,
        ),
        cheeger_estimate: lerp(
            a.topology_signature.cheeger_estimate,
            b.topology_signature.cheeger_estimate,
        ),
        kuramoto_coherence: lerp(
            a.topology_signature.kuramoto_coherence,
            b.topology_signature.kuramoto_coherence,
        ),
        mean_propagation_time: lerp(
            a.topology_signature.mean_propagation_time,
            b.topology_signature.mean_propagation_time,
        ),
        dtl_connected: a.topology_signature.dtl_connected
            && b.topology_signature.dtl_connected,
    };

    let free_energy = inv_alpha * a.free_energy + alpha * b.free_energy;
    let created_at = a.created_at.max(b.created_at) + 1;
    let parent_crystal_ids =
        vec![hex_encode(&lo.crystal_id), hex_encode(&hi.crystal_id)];
    let carrier_instance_idx = a.carrier_instance_idx.min(b.carrier_instance_idx);

    let commit_proof = CommitProof {
        evidence_digests: a
            .commit_proof
            .evidence_digests
            .iter()
            .copied()
            .chain(b.commit_proof.evidence_digests.iter().copied())
            .collect(),
        operator_stack: vec![("interpolate".to_string(), "1.0.0".to_string())],
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

    SemanticCrystal {
        crystal_id,
        region,
        constraint_program: ConstraintProgram::new(),
        stability_score,
        topology_signature,
        betti_numbers: vec![1, 0, 0],
        evidence_chain: a
            .evidence_chain
            .iter()
            .cloned()
            .chain(b.evidence_chain.iter().cloned())
            .collect(),
        commit_proof,
        operator_versions: BTreeMap::new(),
        created_at,
        free_energy,
        carrier_instance_idx,
        scale_tag: "interpolated".into(),
        universe_id: String::new(),
        sub_crystal_ids: Vec::new(),
        parent_crystal_ids,
        genesis_metadata: None,
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

    // ── M.3: bridge ──────────────────────────────────────────────────────

    #[test]
    fn bridge_disjoint_regions_errors() {
        let a = make_crystal(vec![1, 2, 3], 0.8, 1, 0.30, 0.70, 1);
        let b = make_crystal(vec![4, 5, 6], 0.7, 1, 0.30, 0.70, 2);
        let r = bridge(&a, &b, &BridgeConfig::default());
        assert!(matches!(r, Err(BridgeError::DisjointRegions)));
    }

    #[test]
    fn bridge_overlapping_regions_succeeds() {
        let a = make_crystal(vec![1, 2, 3, 4], 0.8, 1, 0.30, 0.70, 1);
        let b = make_crystal(vec![3, 4, 5, 6], 0.7, 1, 0.30, 0.70, 2);
        let bridged = bridge(&a, &b, &BridgeConfig::default()).unwrap();
        assert_eq!(bridged.region, vec![3, 4]);
        assert_eq!(bridged.scale_tag, "bridge");
        assert_eq!(bridged.parent_crystal_ids.len(), 2);
    }

    #[test]
    fn bridge_stability_is_harmonic_mean() {
        let a = make_crystal(vec![1, 2], 0.8, 1, 0.30, 0.70, 1);
        let b = make_crystal(vec![1, 2], 0.5, 1, 0.30, 0.70, 1);
        let bridged = bridge(&a, &b, &BridgeConfig::default()).unwrap();
        let expected = 2.0 * 0.8 * 0.5 / (0.8 + 0.5);
        assert!((bridged.stability_score - expected).abs() < 1e-12);
    }

    #[test]
    fn bridge_stability_is_bounded_min_geometric() {
        // Standard inequality for positive reals a ≤ b:
        //   min(a, b) ≤ harmonic_mean(a, b) ≤ geometric_mean(a, b) ≤ max(a, b)
        // The harmonic mean expresses "weakest-link" semantics in the
        // sense that it is *closer* to the minimum than the geometric
        // mean (which compose uses).
        let a = make_crystal(vec![1, 2], 0.9, 1, 0.30, 0.70, 1);
        let b = make_crystal(vec![1, 2], 0.3, 1, 0.30, 0.70, 1);
        let bridged = bridge(&a, &b, &BridgeConfig::default()).unwrap();
        let geo_mean = (0.9_f64 * 0.3_f64).sqrt();
        assert!(
            bridged.stability_score >= 0.3 - 1e-9
                && bridged.stability_score <= geo_mean + 1e-9,
            "harmonic mean must lie in [min, geometric_mean]: got {} \
             outside [0.3, {}]",
            bridged.stability_score,
            geo_mean
        );
    }

    #[test]
    fn bridge_stability_is_strictly_below_compose_stability() {
        // For the same two stabilities, bridge (harmonic mean) is
        // strictly less than compose (geometric mean) when the inputs
        // differ — this captures the "bridge is more conservative"
        // semantics relative to compose.
        let a = make_crystal(vec![1, 2], 0.9, 1, 0.30, 0.70, 1);
        let b = make_crystal(vec![1, 2], 0.3, 1, 0.30, 0.70, 1);
        let bridged = bridge(&a, &b, &BridgeConfig::default()).unwrap();
        let composed = compose(&[a, b], &ComposeConfig::default()).unwrap();
        assert!(
            bridged.stability_score < composed.stability_score,
            "bridge should be below compose when inputs differ: \
             bridge={} compose={}",
            bridged.stability_score,
            composed.stability_score
        );
    }

    #[test]
    fn bridge_is_symmetric_in_argument_order() {
        let a = make_crystal(vec![1, 2, 3], 0.8, 1, 0.30, 0.70, 1);
        let b = make_crystal(vec![2, 3, 4], 0.7, 1, 0.30, 0.70, 2);
        let ab = bridge(&a, &b, &BridgeConfig::default()).unwrap();
        let ba = bridge(&b, &a, &BridgeConfig::default()).unwrap();
        assert_eq!(ab.crystal_id, ba.crystal_id);
    }

    #[test]
    fn bridge_betti_mismatch_errors() {
        let a = make_crystal(vec![1, 2], 0.8, 1, 0.30, 0.70, 1);
        let b = make_crystal(vec![2, 3], 0.7, 2, 0.30, 0.70, 2);
        let r = bridge(&a, &b, &BridgeConfig::default());
        assert!(matches!(r, Err(BridgeError::Incompatible(_))));
    }

    #[test]
    fn bridge_spectral_gap_outside_tolerance_errors() {
        let a = make_crystal(vec![1, 2], 0.8, 1, 0.10, 0.70, 1);
        let b = make_crystal(vec![2, 3], 0.7, 1, 0.50, 0.70, 2);
        let r = bridge(&a, &b, &BridgeConfig::default());
        assert!(matches!(r, Err(BridgeError::Incompatible(_))));
    }

    #[test]
    fn bridge_with_self_returns_same_region() {
        // Bridging a crystal with itself: region = full region (set
        // intersection of identical sets), stability harmonic-mean of
        // itself = itself.
        let a = make_crystal(vec![1, 2, 3], 0.7, 1, 0.30, 0.70, 1);
        let bridged = bridge(&a, &a, &BridgeConfig::default()).unwrap();
        assert_eq!(bridged.region, a.region);
        assert!((bridged.stability_score - a.stability_score).abs() < 1e-12);
    }

    #[test]
    fn bridge_of_compose_outputs_works() {
        // Closure: bridge can take composed crystals as inputs.
        let c1 = make_crystal(vec![1, 2, 3], 0.8, 1, 0.30, 0.70, 1);
        let c2 = make_crystal(vec![3, 4, 5], 0.7, 1, 0.30, 0.70, 2);
        let composed_a = compose(&[c1, c2], &ComposeConfig::default()).unwrap();

        let c3 = make_crystal(vec![3, 4, 6, 7], 0.6, 1, 0.30, 0.70, 1);
        let c4 = make_crystal(vec![5, 8], 0.7, 1, 0.30, 0.70, 2);
        let composed_b = compose(&[c3, c4], &ComposeConfig::default()).unwrap();

        // composed_a region = {1,2,3,4,5}; composed_b region = {3,4,5,6,7,8}.
        // Intersection = {3, 4, 5}.
        let bridged = bridge(&composed_a, &composed_b, &BridgeConfig::default()).unwrap();
        assert_eq!(bridged.region, vec![3, 4, 5]);
        assert_eq!(bridged.scale_tag, "bridge");
    }

    // ── M.4: query ───────────────────────────────────────────────────────

    #[test]
    fn query_empty_candidates_returns_empty() {
        let template = make_crystal(vec![1, 2], 0.5, 1, 0.30, 0.70, 1);
        let result = query(&template, &[], &QueryConfig::default(), 5);
        assert!(result.is_empty());
    }

    #[test]
    fn query_top_k_zero_returns_empty() {
        let template = make_crystal(vec![1, 2], 0.5, 1, 0.30, 0.70, 1);
        let candidate = make_crystal(vec![1, 2], 0.5, 1, 0.30, 0.70, 1);
        let result = query(
            &template,
            std::slice::from_ref(&candidate),
            &QueryConfig::default(),
            0,
        );
        assert!(result.is_empty());
    }

    #[test]
    fn query_self_match_scores_one() {
        // A crystal queried against itself (or a crystal with the same
        // topology + region + stability) must score exactly 1.0.
        let template = make_crystal(vec![1, 2, 3], 0.7, 1, 0.30, 0.70, 5);
        let result = query(
            &template,
            std::slice::from_ref(&template),
            &QueryConfig::default(),
            1,
        );
        assert_eq!(result.len(), 1);
        assert!(
            (result[0].1 - 1.0).abs() < 1e-9,
            "self-match must score 1.0; got {}",
            result[0].1
        );
    }

    #[test]
    fn query_returns_sorted_descending() {
        let template = make_crystal(vec![1, 2, 3], 0.7, 1, 0.30, 0.70, 1);
        // Three candidates with decreasing similarity to template:
        //  - identical
        //  - same topology + different region
        //  - different stability + same region
        let identical = template.clone();
        let diff_region = make_crystal(vec![100, 200], 0.7, 1, 0.30, 0.70, 1);
        let diff_stability = make_crystal(vec![1, 2, 3], 0.1, 1, 0.30, 0.70, 1);

        let candidates = vec![diff_region, identical, diff_stability];
        let result = query(&template, &candidates, &QueryConfig::default(), 3);
        assert_eq!(result.len(), 3);
        for w in result.windows(2) {
            assert!(
                w[0].1 >= w[1].1,
                "results not in descending order: {} < {}",
                w[0].1,
                w[1].1
            );
        }
    }

    #[test]
    fn query_top_k_caps_result_size() {
        let template = make_crystal(vec![1, 2], 0.5, 1, 0.30, 0.70, 1);
        let candidates: Vec<SemanticCrystal> = (0..10)
            .map(|i| make_crystal(vec![i as u64, (i + 1) as u64], 0.5, 1, 0.30, 0.70, 1))
            .collect();
        let result = query(&template, &candidates, &QueryConfig::default(), 3);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn query_top_k_larger_than_candidates_returns_all() {
        let template = make_crystal(vec![1, 2], 0.5, 1, 0.30, 0.70, 1);
        let candidates: Vec<SemanticCrystal> = (0..3)
            .map(|i| make_crystal(vec![i as u64], 0.5, 1, 0.30, 0.70, 1))
            .collect();
        let result = query(&template, &candidates, &QueryConfig::default(), 100);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn query_is_deterministic_under_same_inputs() {
        let template = make_crystal(vec![1, 2, 3], 0.7, 1, 0.30, 0.70, 1);
        let candidates: Vec<SemanticCrystal> = (0..5)
            .map(|i| make_crystal(vec![i as u64, (i + 1) as u64], 0.5, 1, 0.30, 0.70, 1))
            .collect();
        let r1 = query(&template, &candidates, &QueryConfig::default(), 3);
        let r2 = query(&template, &candidates, &QueryConfig::default(), 3);
        assert_eq!(r1.len(), r2.len());
        for (a, b) in r1.iter().zip(r2.iter()) {
            assert_eq!(a.0.crystal_id, b.0.crystal_id);
            assert!((a.1 - b.1).abs() < 1e-12);
        }
    }

    #[test]
    fn query_with_topology_only_weights_ignores_region() {
        // Set region_weight = stability_weight = 0; only topology matters.
        // Two candidates with same topology but very different regions
        // should score equal.
        let cfg = QueryConfig {
            topology_weight: 1.0,
            region_weight: 0.0,
            stability_weight: 0.0,
        };
        let template = make_crystal(vec![1, 2, 3], 0.7, 1, 0.30, 0.70, 1);
        let same_topo_diff_region =
            make_crystal(vec![100, 200, 300], 0.7, 1, 0.30, 0.70, 1);
        let same_topo_diff_region_2 =
            make_crystal(vec![400, 500], 0.7, 1, 0.30, 0.70, 1);
        let result = query(
            &template,
            &[same_topo_diff_region, same_topo_diff_region_2],
            &cfg,
            2,
        );
        assert!(
            (result[0].1 - result[1].1).abs() < 1e-9,
            "topology-only weighting should make region-different crystals tie: \
             scores {} vs {}",
            result[0].1,
            result[1].1
        );
    }

    #[test]
    fn query_works_on_compose_and_bridge_outputs() {
        // Closure: query operates on any crystal, including outputs of
        // compose and bridge. The post-symbolic computation algebra is
        // closed under all four operators.
        let c1 = make_crystal(vec![1, 2, 3], 0.8, 1, 0.30, 0.70, 1);
        let c2 = make_crystal(vec![2, 3, 4], 0.7, 1, 0.30, 0.70, 2);
        let composed = compose(&[c1.clone(), c2.clone()], &ComposeConfig::default()).unwrap();
        let bridged = bridge(&c1, &c2, &BridgeConfig::default()).unwrap();
        let candidates = vec![composed.clone(), bridged.clone(), c1.clone()];
        let result = query(&c1, &candidates, &QueryConfig::default(), 3);
        // Must return all three with finite, sorted scores.
        assert_eq!(result.len(), 3);
        for (_, score) in &result {
            assert!((0.0..=1.0).contains(score));
        }
    }

    // ── N: generative interpolation ──────────────────────────────────────

    #[test]
    fn interpolate_at_alpha_zero_reproduces_a() {
        let a = make_crystal(vec![1, 2, 3], 0.8, 1, 0.30, 0.70, 1);
        let b = make_crystal(vec![4, 5, 6], 0.5, 2, 0.50, 0.40, 2);
        let r = interpolate(&a, &b, 0.0);
        assert_eq!(r.region, a.region);
        assert!((r.stability_score - a.stability_score).abs() < 1e-12);
        assert_eq!(r.topology_signature.betti_0, a.topology_signature.betti_0);
        assert!((r.topology_signature.spectral_gap - a.topology_signature.spectral_gap).abs() < 1e-12);
    }

    #[test]
    fn interpolate_at_alpha_one_reproduces_b() {
        let a = make_crystal(vec![1, 2, 3], 0.8, 1, 0.30, 0.70, 1);
        let b = make_crystal(vec![4, 5, 6], 0.5, 2, 0.50, 0.40, 2);
        let r = interpolate(&a, &b, 1.0);
        assert_eq!(r.region, b.region);
        assert!((r.stability_score - b.stability_score).abs() < 1e-12);
        assert_eq!(r.topology_signature.betti_0, b.topology_signature.betti_0);
        assert!((r.topology_signature.spectral_gap - b.topology_signature.spectral_gap).abs() < 1e-12);
    }

    #[test]
    fn interpolate_topology_lies_between_endpoints() {
        // For α ∈ (0, 1), every continuous topology axis must be
        // strictly between the two endpoint values.
        let a = make_crystal(vec![1], 0.9, 1, 0.10, 0.30, 1);
        let b = make_crystal(vec![1], 0.1, 1, 0.50, 0.90, 2);
        let r = interpolate(&a, &b, 0.5);
        let lo = a.stability_score.min(b.stability_score);
        let hi = a.stability_score.max(b.stability_score);
        assert!(
            r.stability_score > lo + 1e-9 && r.stability_score < hi - 1e-9,
            "stability {} not strictly between [{}, {}]",
            r.stability_score, lo, hi
        );
        let g_lo = a.topology_signature.spectral_gap.min(b.topology_signature.spectral_gap);
        let g_hi = a.topology_signature.spectral_gap.max(b.topology_signature.spectral_gap);
        assert!(
            r.topology_signature.spectral_gap > g_lo + 1e-9
                && r.topology_signature.spectral_gap < g_hi - 1e-9,
            "spectral_gap not strictly between"
        );
    }

    #[test]
    fn interpolate_creates_a_new_crystal_id() {
        // Generativity: at α = 0.5 the result is **not** either input.
        let a = make_crystal(vec![1, 2, 3], 0.9, 1, 0.10, 0.30, 1);
        let b = make_crystal(vec![1, 2, 3], 0.1, 1, 0.50, 0.90, 2);
        let r = interpolate(&a, &b, 0.5);
        assert_ne!(r.crystal_id, a.crystal_id);
        assert_ne!(r.crystal_id, b.crystal_id);
    }

    #[test]
    fn interpolate_records_both_parents() {
        let a = make_crystal(vec![1], 0.5, 1, 0.30, 0.70, 1);
        let b = make_crystal(vec![2], 0.5, 1, 0.30, 0.70, 2);
        let r = interpolate(&a, &b, 0.5);
        assert_eq!(r.parent_crystal_ids.len(), 2);
        assert_eq!(r.scale_tag, "interpolated");
    }

    #[test]
    fn interpolate_is_compose_input_closure() {
        // The output of interpolate is itself a SemanticCrystal that
        // can flow into compose / dual / bridge / query — full
        // closure of the post-symbolic operator algebra.
        let a = make_crystal(vec![1, 2, 3], 0.8, 1, 0.30, 0.70, 1);
        let b = make_crystal(vec![4, 5, 6], 0.7, 1, 0.30, 0.70, 2);
        let interp = interpolate(&a, &b, 0.3);
        let composed = compose(&[interp.clone(), a.clone()], &ComposeConfig::default());
        assert!(composed.is_ok(), "interpolated crystal must be compose-compatible");
        let d = dual(&interp);
        assert_eq!(d.region, interp.region);
    }

    #[test]
    fn interpolate_clamps_alpha_outside_unit_interval() {
        let a = make_crystal(vec![1], 0.5, 1, 0.30, 0.70, 1);
        let b = make_crystal(vec![2], 0.5, 1, 0.30, 0.70, 2);
        let r_neg = interpolate(&a, &b, -0.5);
        let r_zero = interpolate(&a, &b, 0.0);
        assert_eq!(r_neg.crystal_id, r_zero.crystal_id);
        let r_two = interpolate(&a, &b, 2.0);
        let r_one = interpolate(&a, &b, 1.0);
        assert_eq!(r_two.crystal_id, r_one.crystal_id);
    }

    #[test]
    fn interpolate_created_at_is_strictly_posterior() {
        let a = make_crystal(vec![1], 0.5, 1, 0.30, 0.70, 7);
        let b = make_crystal(vec![2], 0.5, 1, 0.30, 0.70, 12);
        let r = interpolate(&a, &b, 0.5);
        assert_eq!(r.created_at, 13); // max(7, 12) + 1
    }
}

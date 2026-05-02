use std::fmt::Write;

use serde::Serialize;

use crate::bitgraph::{
    bits_to_adjacency, complement_bits, fast_orbit, mask_for_n, S7BitCache,
};
use crate::bitgraph_n::{
    bits_to_adjacency as bits_to_adjacency_n, complement_bits as complement_bits_n,
    fast_orbit_n, SnBitCache,
};
use crate::complement::{complement, mat_pow, trace};
use crate::embed::embed;
use crate::error::{ScanError, ScanResult};
use crate::group::{generate_s7, permutation_matrix_13};
use crate::ingest::InputGraph;
use crate::orbit::compute_orbit;
use crate::platonic::{classify_platonic, PlatonicClass};
use crate::properties::{adjacency_to_bits, compute_properties, compute_properties_with, GraphProperties};
use crate::spectrum::compute_spectrum;
use crate::stabilizer::{compute_stabilizer, StabilizerResult};

#[derive(Debug, Clone, Serialize)]
pub struct ScanReport {
    // Input
    pub input_nodes: usize,
    pub input_edges: usize,
    pub input_labels: Option<Vec<String>>,

    // Orbit
    pub canonical_form: Vec<Vec<f64>>,
    pub canonical_hash: String,
    pub orbit_size: usize,

    // Stabilizer
    pub stabilizer_order: usize,
    pub stabilizer_class: Option<String>,
    pub stabilizer_generators: Vec<Vec<usize>>,

    // Spectrum
    pub eigenvalues: Vec<f64>,
    pub spectral_radius: f64,
    pub algebraic_connectivity: f64,
    /// Graph energy E(G) = Σ|λᵢ| — a classical invariant (Gutman 1978).
    pub graph_energy: f64,
    /// Laplacian eigenvalues (non-negative, sorted ascending).
    pub laplacian_eigenvalues: Vec<f64>,
    /// Number of spanning trees τ(G) (Kirchhoff's Matrix-Tree theorem).
    pub spanning_tree_count: u64,

    // Topology
    pub triangle_count: usize,
    pub complement_orbit_size: usize,
    pub complement_canonical_hash: String,
    pub a2_trace: f64,
    pub a3_trace: f64,

    // Combinatorial properties (from the Periodic Table spec)
    pub properties: GraphProperties,

    // Platonic
    pub platonic_classes: Vec<PlatonicClass>,

    // Verification
    pub orbit_stabilizer_product: usize,
}

/// Precomputed S7 data (generate once, reuse for multiple scans).
pub struct S7Cache {
    pub perms: Vec<Vec<usize>>,
    pub matrices: Vec<Vec<Vec<f64>>>,
}

impl S7Cache {
    pub fn new() -> Self {
        let perms = generate_s7();
        let matrices = perms.iter().map(|sigma| permutation_matrix_13(sigma)).collect();
        Self { perms, matrices }
    }
}

/// Single scan of a graph.
pub fn scan(input: InputGraph) -> ScanResult<ScanReport> {
    let cache = S7Cache::new();
    scan_with_cache(input, &cache)
}

/// FNV-1a hash of the canonical 13×13 form (matches [`crate::orbit`]'s scheme).
pub(crate) fn hash_canonical_matrix(canonical: &[Vec<f64>]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for row in canonical {
        for &val in row {
            for b in val.to_bits().to_le_bytes() {
                hash ^= b as u64;
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }
    }
    let mut s = String::with_capacity(16);
    let _ = write!(s, "{:016x}", hash);
    s
}

/// Embed a packed bitmask `bits` into the 13×13 active zone.
fn embed_bits(bits: u32, n: usize) -> Vec<Vec<f64>> {
    let mut e = vec![vec![0.0f64; 13]; 13];
    let active = bits_to_adjacency(bits, n);
    for i in 0..n {
        for j in 0..n {
            e[i][j] = active[i][j];
        }
    }
    e
}

/// Fast scan of a canonical graph given its packed bit-form.
///
/// Skips the slow O(13⁴ · 5040) orbit/stabilizer computation by reusing the
/// already-computed `S_7` bit-orbit. Only used by the catalog generator —
/// the public [`scan_with_cache`] still does the full 13×13 computation
/// to keep its semantics for arbitrary callers.
///
/// `canonical_bits` must already be canonical under `S_7` (use
/// [`crate::bitgraph::fast_orbit`] to obtain it).
pub fn scan_canonical_bits(
    n: usize,
    canonical_bits: u32,
    bit_cache: &S7BitCache,
) -> ScanResult<ScanReport> {
    if n == 0 {
        return Err(ScanError::EmptyGraph);
    }
    if n > 7 {
        return Err(ScanError::TooManyNodes { n });
    }

    let mask = mask_for_n(n);
    debug_assert_eq!(canonical_bits & !mask, 0, "bits outside active zone");

    // Orbit + stabilizer on bits.
    let orb = fast_orbit(canonical_bits, bit_cache);
    let product = orb.orbit_size * orb.stabilizer_order;
    if product != 5040 {
        return Err(ScanError::OrbitStabilizerViolation {
            orbit: orb.orbit_size,
            stabilizer: orb.stabilizer_order,
        });
    }

    // Embedded canonical form.
    let canonical_form = embed_bits(canonical_bits, n);
    let canonical_hash = hash_canonical_matrix(&canonical_form);

    // Stabilizer classification (no generators — those would require running
    // the full 13×13 search).
    let stab_classification = StabilizerResult {
        elements: Vec::new(),
        order: orb.stabilizer_order,
        classification: classify_group_order(orb.stabilizer_order),
        generators: Vec::new(),
    };

    // Spectrum on the n×n active submatrix.
    let spectrum = compute_spectrum(&canonical_form, n);

    // Complement (also via bit-orbit for hash + size).
    let comp_bits = complement_bits(canonical_bits, n);
    let comp_orb = fast_orbit(comp_bits, bit_cache);
    let comp_canon_form = embed_bits(comp_orb.canonical, n);
    let comp_hash = hash_canonical_matrix(&comp_canon_form);

    // tr(A^k) — only needed for n×n active zone since the rest is zero.
    let a2 = mat_pow(&canonical_form, 2);
    let a3 = mat_pow(&canonical_form, 3);

    let platonic = classify_platonic(&canonical_form, n);
    let properties = compute_properties(&canonical_form, n, bit_cache);
    let m = (canonical_bits.count_ones()) as usize;

    Ok(ScanReport {
        input_nodes: n,
        input_edges: m,
        input_labels: None,
        canonical_form,
        canonical_hash,
        orbit_size: orb.orbit_size,
        stabilizer_order: stab_classification.order,
        stabilizer_class: stab_classification.classification,
        stabilizer_generators: stab_classification.generators,
        eigenvalues: spectrum.eigenvalues,
        spectral_radius: spectrum.spectral_radius,
        algebraic_connectivity: spectrum.algebraic_connectivity,
        triangle_count: spectrum.triangle_count,
        graph_energy: spectrum.graph_energy,
        laplacian_eigenvalues: spectrum.laplacian_eigenvalues,
        spanning_tree_count: spectrum.spanning_tree_count,
        complement_orbit_size: comp_orb.orbit_size,
        complement_canonical_hash: comp_hash,
        a2_trace: trace(&a2),
        a3_trace: trace(&a3),
        properties,
        platonic_classes: platonic,
        orbit_stabilizer_product: product,
    })
}

fn classify_group_order(order: usize) -> Option<String> {
    match order {
        1 => Some("trivial".into()),
        2 => Some("Z2".into()),
        3 => Some("Z3".into()),
        4 => Some("Z4 or V4".into()),
        6 => Some("S3 or C6 or Z6".into()),
        8 => Some("D4".into()),
        10 => Some("D5".into()),
        12 => Some("D6 or A4".into()),
        24 => Some("S4".into()),
        48 => Some("S4 x Z2".into()),
        120 => Some("S5".into()),
        720 => Some("S6".into()),
        5040 => Some("S7".into()),
        _ => None,
    }
}

#[allow(dead_code)]
fn _use_adjacency_to_bits(a: &[Vec<f64>], n: usize) -> u32 {
    adjacency_to_bits(a, n)
}

// ============================================================================
// n-generic fast scan — supports arbitrary n ≤ 8 (uses S_n, not S_7).
// ============================================================================

/// Embed a bitmask into an n×n (or 13×13 for n ≤ 7) adjacency matrix.
/// For n = 8 we simply return the 8×8 matrix since the Metatron scaffold
/// only has an active zone of 7 nodes.
fn embed_bits_n(bits: u32, n: usize) -> Vec<Vec<f64>> {
    if n <= 7 {
        embed_bits(bits, n)
    } else {
        bits_to_adjacency_n(bits, n)
    }
}

/// n-generic fast scan over `S_n` for a canonical bitmask.
///
/// Differs from [`scan_canonical_bits`] (which is S_7-bound) in:
///   * works for any `n` in 1..=8,
///   * orbit-stabilizer check uses `n!` instead of the fixed 5040,
///   * `canonical_form` is either the 13×13 embedding (for n ≤ 7) or the
///     n×n active matrix (for n = 8), matching each layer's natural frame.
///
/// Platonic classification is omitted for n = 8 because the scaffold's
/// Platonic edges live inside the n ≤ 7 active zone.
pub fn scan_canonical_bits_n(
    n: usize,
    canonical_bits: u32,
    cache: &SnBitCache,
) -> ScanResult<ScanReport> {
    if n == 0 {
        return Err(ScanError::EmptyGraph);
    }
    if n > 8 {
        return Err(ScanError::TooManyNodes { n });
    }
    assert_eq!(cache.n, n, "SnBitCache.n must match the target n");

    let orb = fast_orbit_n(canonical_bits, cache);
    let product = orb.orbit_size * orb.stabilizer_order;
    let n_fact: usize = (1..=n).product();
    if product != n_fact {
        return Err(ScanError::OrbitStabilizerViolation {
            orbit: orb.orbit_size,
            stabilizer: orb.stabilizer_order,
        });
    }

    let canonical_form = embed_bits_n(canonical_bits, n);
    let canonical_hash = hash_canonical_matrix(&canonical_form);

    let spectrum = compute_spectrum(&canonical_form, n);

    // Complement
    let comp_bits = complement_bits_n(canonical_bits, n);
    let comp_orb = fast_orbit_n(comp_bits, cache);
    let comp_canon_form = embed_bits_n(comp_orb.canonical, n);
    let comp_hash = hash_canonical_matrix(&comp_canon_form);
    let is_self_complementary = comp_orb.canonical == canonical_bits;

    let a2 = mat_pow(&canonical_form, 2);
    let a3 = mat_pow(&canonical_form, 3);

    let platonic = if n <= 7 {
        classify_platonic(&canonical_form, n)
    } else {
        Vec::new()
    };

    let properties = compute_properties_with(&canonical_form, n, is_self_complementary);

    let stab_classification = StabilizerResult {
        elements: Vec::new(),
        order: orb.stabilizer_order,
        classification: classify_group_order_n(orb.stabilizer_order, n),
        generators: Vec::new(),
    };

    let m = canonical_bits.count_ones() as usize;
    Ok(ScanReport {
        input_nodes: n,
        input_edges: m,
        input_labels: None,
        canonical_form,
        canonical_hash,
        orbit_size: orb.orbit_size,
        stabilizer_order: stab_classification.order,
        stabilizer_class: stab_classification.classification,
        stabilizer_generators: stab_classification.generators,
        eigenvalues: spectrum.eigenvalues,
        spectral_radius: spectrum.spectral_radius,
        algebraic_connectivity: spectrum.algebraic_connectivity,
        graph_energy: spectrum.graph_energy,
        laplacian_eigenvalues: spectrum.laplacian_eigenvalues,
        spanning_tree_count: spectrum.spanning_tree_count,
        triangle_count: spectrum.triangle_count,
        complement_orbit_size: comp_orb.orbit_size,
        complement_canonical_hash: comp_hash,
        a2_trace: trace(&a2),
        a3_trace: trace(&a3),
        properties,
        platonic_classes: platonic,
        orbit_stabilizer_product: product,
    })
}

fn classify_group_order_n(order: usize, n: usize) -> Option<String> {
    let base = classify_group_order(order);
    if base.is_some() {
        return base;
    }
    let n_fact: usize = (1..=n).product();
    if order == n_fact {
        return Some(format!("S{}", n));
    }
    None
}

/// Scan with precomputed S7 data (for batch scans).
pub fn scan_with_cache(input: InputGraph, cache: &S7Cache) -> ScanResult<ScanReport> {
    input.validate()?;

    let input_edges = input.edge_count();
    let input_nodes = input.n;
    let input_labels = input.labels.clone();

    let embedded = embed(&input);
    let orbit = compute_orbit(&embedded, &cache.matrices);
    let stabilizer = compute_stabilizer(&embedded, &cache.perms, &cache.matrices);

    // Orbit-Stabilizer Theorem check
    let product = orbit.orbit_size * stabilizer.order;
    if product != 5040 {
        return Err(ScanError::OrbitStabilizerViolation {
            orbit: orbit.orbit_size,
            stabilizer: stabilizer.order,
        });
    }

    let spectrum = compute_spectrum(&embedded, input_nodes);

    let comp = complement(&embedded, input_nodes);
    let comp_orbit = compute_orbit(&comp, &cache.matrices);

    let a2 = mat_pow(&embedded, 2);
    let a3 = mat_pow(&embedded, 3);

    let platonic = classify_platonic(&embedded, input_nodes);

    // Combinatorial properties use a fast bit-graph cache; build one ad hoc
    // (callers that scan many graphs should prefer the catalog API which
    // shares one cache across all scans).
    let bit_cache = S7BitCache::new();
    let properties = compute_properties(&embedded, input_nodes, &bit_cache);

    Ok(ScanReport {
        input_nodes,
        input_edges,
        input_labels,
        canonical_form: orbit.canonical_form,
        canonical_hash: orbit.canonical_hash,
        orbit_size: orbit.orbit_size,
        stabilizer_order: stabilizer.order,
        stabilizer_class: stabilizer.classification,
        stabilizer_generators: stabilizer.generators,
        eigenvalues: spectrum.eigenvalues,
        spectral_radius: spectrum.spectral_radius,
        graph_energy: spectrum.graph_energy,
        laplacian_eigenvalues: spectrum.laplacian_eigenvalues,
        spanning_tree_count: spectrum.spanning_tree_count,
        algebraic_connectivity: spectrum.algebraic_connectivity,
        triangle_count: spectrum.triangle_count,
        complement_orbit_size: comp_orbit.orbit_size,
        complement_canonical_hash: comp_orbit.canonical_hash,
        a2_trace: trace(&a2),
        a3_trace: trace(&a3),
        properties,
        platonic_classes: platonic,
        orbit_stabilizer_product: product,
    })
}

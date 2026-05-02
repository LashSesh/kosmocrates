//! Compact bit-vector representation of graphs with up to 7 nodes.
//!
//! Edges of the upper triangle of a 7x7 adjacency matrix are packed into
//! the low 21 bits of a `u32`. Bit `b = j*(j-1)/2 + i` (for `0 <= i < j < 7`)
//! is set iff the edge `{i+1, j+1}` is present.
//!
//! This representation allows the full S_7 orbit (5040 permutations) to be
//! computed in microseconds via precomputed bit-permutation tables.
//!
//! Together with [`generate_s7`](crate::group::generate_s7) this provides the
//! engine for the Periodic Table catalog generator.

use crate::group::generate_s7;

/// Maximum number of edges in the upper triangle of a 7x7 graph.
pub const MAX_EDGES_BITS: u32 = 21;

/// Total number of bit patterns (= number of distinct labelled graphs on 7 nodes).
pub const N7_LABELLED: u32 = 1 << MAX_EDGES_BITS; // 2_097_152

/// Returns the bit index of an edge `{i, j}` (0-indexed, `i < j < 7`).
#[inline]
pub fn edge_to_bit(i: usize, j: usize) -> u32 {
    debug_assert!(i < j && j < 7);
    (j * (j - 1) / 2 + i) as u32
}

/// Returns the (i, j) edge endpoints (0-indexed, `i < j`) for a bit index.
pub fn bit_to_edge(b: u32) -> (usize, usize) {
    // j is the largest integer with j*(j-1)/2 <= b
    let mut j = 1usize;
    while j * (j + 1) / 2 <= b as usize {
        j += 1;
    }
    let i = b as usize - j * (j - 1) / 2;
    (i, j)
}

/// Number of edges in a packed bitmask.
#[inline]
pub fn edge_count(bits: u32) -> usize {
    bits.count_ones() as usize
}

/// Convert a packed bitmask to a 7x7 (or n×n) adjacency matrix as `Vec<Vec<f64>>`.
pub fn bits_to_adjacency(bits: u32, n: usize) -> Vec<Vec<f64>> {
    let mut adj = vec![vec![0.0f64; n]; n];
    let mut b = bits;
    while b != 0 {
        let bit = b.trailing_zeros();
        let (i, j) = bit_to_edge(bit);
        if i < n && j < n {
            adj[i][j] = 1.0;
            adj[j][i] = 1.0;
        }
        b &= b - 1;
    }
    adj
}

/// Convert a packed bitmask to a list of 1-indexed edges.
pub fn bits_to_edges(bits: u32) -> Vec<(usize, usize)> {
    let mut edges = Vec::with_capacity(bits.count_ones() as usize);
    let mut b = bits;
    while b != 0 {
        let bit = b.trailing_zeros();
        let (i, j) = bit_to_edge(bit);
        edges.push((i + 1, j + 1));
        b &= b - 1;
    }
    edges
}

/// Restrict bitmask to edges fully inside the first `n` nodes.
#[inline]
pub fn mask_for_n(n: usize) -> u32 {
    if n < 2 {
        return 0;
    }
    let max_bit = (n * (n - 1) / 2) as u32;
    if max_bit >= 32 {
        u32::MAX
    } else {
        (1u32 << max_bit) - 1
    }
}

/// Complement (bitwise) of a graph restricted to `n` nodes.
#[inline]
pub fn complement_bits(bits: u32, n: usize) -> u32 {
    (!bits) & mask_for_n(n)
}

/// A precomputed bit-permutation table for one element of `S_7`.
/// `table[src_bit] = dst_bit` describes how each edge bit moves.
pub type BitPerm = [u32; 21];

/// Apply a bit-permutation to a packed bitmask.
#[inline]
pub fn permute_bits(g: u32, table: &BitPerm) -> u32 {
    let mut result = 0u32;
    let mut bits = g;
    while bits != 0 {
        let src = bits.trailing_zeros() as usize;
        result |= 1u32 << table[src];
        bits &= bits - 1;
    }
    result
}

/// Build the bit-permutation table corresponding to `sigma` ∈ S_7 (1-indexed,
/// length 7). For each upper-triangle edge bit, finds where the permuted edge
/// lands.
pub fn bit_perm_from_sigma(sigma: &[usize]) -> BitPerm {
    debug_assert_eq!(sigma.len(), 7);
    let mut table = [0u32; 21];
    for src in 0..MAX_EDGES_BITS {
        let (i, j) = bit_to_edge(src);
        let a = sigma[i] - 1;
        let b = sigma[j] - 1;
        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        table[src as usize] = edge_to_bit(lo, hi);
    }
    table
}

/// Cached bit-permutation tables for all 5040 elements of `S_7`.
///
/// One-time construction cost (≈ a millisecond), then every orbit/stabilizer
/// query touches only fixed-size arrays — no allocations.
pub struct S7BitCache {
    pub perms: Vec<Vec<usize>>,
    pub tables: Vec<BitPerm>,
}

impl S7BitCache {
    pub fn new() -> Self {
        let perms = generate_s7();
        let tables: Vec<BitPerm> = perms.iter().map(|s| bit_perm_from_sigma(s)).collect();
        Self { perms, tables }
    }

    /// Number of permutations (= 5040).
    pub fn len(&self) -> usize {
        self.tables.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tables.is_empty()
    }
}

impl Default for S7BitCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a fast orbit computation.
#[derive(Debug, Clone)]
pub struct FastOrbit {
    pub canonical: u32,
    pub orbit_size: usize,
    pub stabilizer_order: usize,
}

/// Fast orbit and stabilizer of a packed graph under the cached `S_7` action.
///
/// Verifies the orbit-stabilizer theorem (`|Orb| · |Stab| = 5040`) by
/// construction: the same loop counts both quantities.
pub fn fast_orbit(bits: u32, cache: &S7BitCache) -> FastOrbit {
    let mut canonical = u32::MAX;
    let mut stab = 0usize;
    let mut seen = std::collections::HashSet::with_capacity(64);
    for table in &cache.tables {
        let img = permute_bits(bits, table);
        if img == bits {
            stab += 1;
        }
        if img < canonical {
            canonical = img;
        }
        seen.insert(img);
    }
    FastOrbit {
        canonical,
        orbit_size: seen.len(),
        stabilizer_order: stab,
    }
}

/// Same as [`fast_orbit`] but only returns the canonical form (no allocation).
#[inline]
pub fn canonical_bits(bits: u32, cache: &S7BitCache) -> u32 {
    let mut canonical = u32::MAX;
    for table in &cache.tables {
        let img = permute_bits(bits, table);
        if img < canonical {
            canonical = img;
        }
    }
    canonical
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_edge_roundtrip() {
        for j in 1..7 {
            for i in 0..j {
                let b = edge_to_bit(i, j);
                assert_eq!(bit_to_edge(b), (i, j));
            }
        }
    }

    #[test]
    fn max_bit_is_20() {
        assert_eq!(edge_to_bit(5, 6), 20);
    }

    #[test]
    fn mask_for_n_values() {
        assert_eq!(mask_for_n(0), 0);
        assert_eq!(mask_for_n(1), 0);
        assert_eq!(mask_for_n(2), 0b1);
        assert_eq!(mask_for_n(3), 0b111);
        assert_eq!(mask_for_n(4), 0b111111);
        assert_eq!(mask_for_n(7), (1 << 21) - 1);
    }

    #[test]
    fn identity_perm_is_identity_table() {
        let id = vec![1, 2, 3, 4, 5, 6, 7];
        let table = bit_perm_from_sigma(&id);
        for b in 0..MAX_EDGES_BITS {
            assert_eq!(table[b as usize], b);
        }
    }

    #[test]
    fn cache_size_5040() {
        let cache = S7BitCache::new();
        assert_eq!(cache.len(), 5040);
    }

    #[test]
    fn empty_graph_orbit_1_stab_5040() {
        let cache = S7BitCache::new();
        let r = fast_orbit(0, &cache);
        assert_eq!(r.canonical, 0);
        assert_eq!(r.orbit_size, 1);
        assert_eq!(r.stabilizer_order, 5040);
        assert_eq!(r.orbit_size * r.stabilizer_order, 5040);
    }

    #[test]
    fn k7_orbit_1_stab_5040() {
        let cache = S7BitCache::new();
        let k7 = mask_for_n(7);
        let r = fast_orbit(k7, &cache);
        assert_eq!(r.orbit_size, 1);
        assert_eq!(r.stabilizer_order, 5040);
    }

    #[test]
    fn single_edge_orbit_stab() {
        let cache = S7BitCache::new();
        let r = fast_orbit(1u32, &cache);
        // orbit: 21 placements (C(7,2)); stab: 2 * 5! = 240
        assert_eq!(r.orbit_size, 21);
        assert_eq!(r.stabilizer_order, 240);
        assert_eq!(r.orbit_size * r.stabilizer_order, 5040);
    }

    #[test]
    fn complement_involution() {
        for &n in &[2usize, 3, 4, 5, 6, 7] {
            let mask = mask_for_n(n);
            for g in [0u32, mask, 1u32, mask ^ 1u32] {
                assert_eq!(complement_bits(complement_bits(g, n), n), g & mask);
            }
        }
    }

    #[test]
    fn bits_to_adjacency_matches_edges() {
        // Triangle on nodes 1-2-3
        let g = (1u32 << edge_to_bit(0, 1))
            | (1u32 << edge_to_bit(1, 2))
            | (1u32 << edge_to_bit(0, 2));
        let adj = bits_to_adjacency(g, 3);
        assert_eq!(adj[0][1], 1.0);
        assert_eq!(adj[1][2], 1.0);
        assert_eq!(adj[0][2], 1.0);
        assert_eq!(adj[1][0], 1.0);
    }
}

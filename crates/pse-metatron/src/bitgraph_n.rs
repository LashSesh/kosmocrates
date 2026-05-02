//! Generalized bit-graph representation for arbitrary `n ≤ 8`.
//!
//! Where [`crate::bitgraph`] hard-codes 21 upper-triangle bits for n = 7
//! (the active zone of the Metatron scaffold), this module generalizes the
//! same machinery to any `n` up to 8 — opening the door to the n = 8
//! frontier (12,346 graphs per OEIS A000088).
//!
//! `u32` still fits comfortably: C(8,2) = 28 ≤ 32 bits.
//!
//! The core speedup for the catalog is the *canonicity predicate*: to decide
//! whether a bitmask is the lex-smallest element of its orbit we iterate
//! over `S_n` and reject at the first permutation that produces a smaller
//! image. For asymmetric graphs (|Stab| = 1) this averages ~n/2 tries
//! instead of the full n! enumeration, i.e. hundreds of thousands of times
//! faster at n = 8.

use std::collections::HashSet;

use rayon::prelude::*;

/// Full permutation list of `S_n`.
pub fn generate_sn(n: usize) -> Vec<Vec<usize>> {
    fn rec(elements: &[usize], out: &mut Vec<Vec<usize>>) {
        if elements.len() <= 1 {
            out.push(elements.to_vec());
            return;
        }
        for (idx, &first) in elements.iter().enumerate() {
            let mut rest: Vec<usize> = elements.to_vec();
            rest.remove(idx);
            let mut sub: Vec<Vec<usize>> = Vec::new();
            rec(&rest, &mut sub);
            for mut s in sub {
                s.insert(0, first);
                out.push(s);
            }
        }
    }
    let base: Vec<usize> = (1..=n).collect();
    let mut all: Vec<Vec<usize>> = Vec::new();
    rec(&base, &mut all);
    all
}

/// Number of upper-triangle edge bits for a graph on n nodes.
#[inline]
pub const fn edge_bits(n: usize) -> usize {
    if n < 2 {
        0
    } else {
        n * (n - 1) / 2
    }
}

/// Bit-position for edge `{i, j}` (0-indexed, `i < j`). Independent of n.
#[inline]
pub fn edge_to_bit(i: usize, j: usize) -> u32 {
    debug_assert!(i < j);
    (j * (j - 1) / 2 + i) as u32
}

/// Inverse of `edge_to_bit`.
pub fn bit_to_edge(b: u32) -> (usize, usize) {
    let mut j = 1usize;
    while j * (j + 1) / 2 <= b as usize {
        j += 1;
    }
    let i = b as usize - j * (j - 1) / 2;
    (i, j)
}

/// Upper-triangle mask for n nodes.
#[inline]
pub fn mask_for_n(n: usize) -> u32 {
    let k = edge_bits(n);
    if k == 0 {
        0
    } else if k >= 32 {
        u32::MAX
    } else {
        (1u32 << k) - 1
    }
}

/// Edge bitmask complement restricted to n nodes.
#[inline]
pub fn complement_bits(bits: u32, n: usize) -> u32 {
    (!bits) & mask_for_n(n)
}

/// Convert a bitmask to a list of 1-indexed edges.
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

/// Convert a bitmask to an n×n adjacency matrix (f64 entries).
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

/// Precomputed bit-permutation tables for all elements of `S_n`.
/// `tables[k][src_bit] = dst_bit` for the k-th permutation.
pub struct SnBitCache {
    pub n: usize,
    pub perms: Vec<Vec<usize>>,
    pub tables: Vec<Vec<u32>>,
    identity_idx: usize,
}

impl SnBitCache {
    pub fn new(n: usize) -> Self {
        assert!(n >= 1 && n <= 8, "SnBitCache only supports 1 <= n <= 8");
        let perms = generate_sn(n);
        let k = edge_bits(n);
        let mut tables: Vec<Vec<u32>> = Vec::with_capacity(perms.len());
        let mut identity_idx = 0usize;
        let identity: Vec<usize> = (1..=n).collect();
        for (i, sigma) in perms.iter().enumerate() {
            if sigma == &identity {
                identity_idx = i;
            }
            let mut tbl = vec![0u32; k];
            for src in 0..k as u32 {
                let (a, b) = bit_to_edge(src);
                let pa = sigma[a] - 1;
                let pb = sigma[b] - 1;
                let (lo, hi) = if pa < pb { (pa, pb) } else { (pb, pa) };
                tbl[src as usize] = edge_to_bit(lo, hi);
            }
            tables.push(tbl);
        }
        Self {
            n,
            perms,
            tables,
            identity_idx,
        }
    }

    pub fn len(&self) -> usize {
        self.tables.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tables.is_empty()
    }

    /// Identity permutation index (avoid in canonicity checks).
    pub fn identity_idx(&self) -> usize {
        self.identity_idx
    }
}

/// Apply a precomputed table to a bitmask.
#[inline]
pub fn permute_bits(g: u32, table: &[u32]) -> u32 {
    let mut result = 0u32;
    let mut bits = g;
    while bits != 0 {
        let src = bits.trailing_zeros() as usize;
        result |= 1u32 << table[src];
        bits &= bits - 1;
    }
    result
}

/// Orbit + stabilizer of a bitmask under `S_n`.
#[derive(Debug, Clone)]
pub struct OrbitNResult {
    pub canonical: u32,
    pub orbit_size: usize,
    pub stabilizer_order: usize,
}

pub fn fast_orbit_n(bits: u32, cache: &SnBitCache) -> OrbitNResult {
    let mut canonical = u32::MAX;
    let mut stab = 0usize;
    let mut seen = HashSet::with_capacity(64);
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
    OrbitNResult {
        canonical,
        orbit_size: seen.len(),
        stabilizer_order: stab,
    }
}

/// Fast *canonicity predicate* with early reject.
///
/// Returns true iff `bits` is the lexicographically smallest element of its
/// orbit under `S_n`. This is the engine for n ≥ 8 catalog generation:
/// instead of computing the full orbit of every 2^C(n,2) candidate, we just
/// test whether each one is its own canonical form.
///
/// Asymptotically this replaces a per-candidate cost of O(|S_n|) with
/// O(|Stab|·|S_n|/|Orb|) = O(|S_n|²/|Orb|) in the worst case, but with
/// early-break the expected cost for random bitmasks is O(n²) or less.
#[inline]
pub fn is_canonical(bits: u32, cache: &SnBitCache) -> bool {
    for (i, table) in cache.tables.iter().enumerate() {
        if i == cache.identity_idx {
            continue;
        }
        let img = permute_bits(bits, table);
        if img < bits {
            return false;
        }
    }
    true
}

/// Count automorphisms of `bits`: size of its stabilizer.
pub fn stabilizer_order_n(bits: u32, cache: &SnBitCache) -> usize {
    cache
        .tables
        .iter()
        .filter(|t| permute_bits(bits, t) == bits)
        .count()
}

/// Parallel enumeration of canonical representatives for n nodes.
///
/// Splits the 2^C(n,2) candidate range across all rayon worker threads and
/// collects every `bits` that passes the canonicity predicate. Result is
/// sorted in ascending order so downstream processing is deterministic.
pub fn enumerate_canonical_parallel(cache: &SnBitCache) -> Vec<u32> {
    let mask = mask_for_n(cache.n);
    let total: u64 = if cache.n < 2 {
        1
    } else {
        mask as u64 + 1
    };
    (0u64..total)
        .into_par_iter()
        .filter_map(|bits_u64| {
            let bits = bits_u64 as u32;
            if is_canonical(bits, cache) {
                Some(bits)
            } else {
                None
            }
        })
        .collect::<Vec<u32>>()
        .into_iter()
        .collect::<Vec<u32>>()
}

/// Sequential variant (used as the fallback and reference).
pub fn enumerate_canonical_sequential(cache: &SnBitCache) -> Vec<u32> {
    let mask = mask_for_n(cache.n);
    let total: u64 = if cache.n < 2 {
        1
    } else {
        mask as u64 + 1
    };
    (0u64..total)
        .filter_map(|b| {
            let bits = b as u32;
            if is_canonical(bits, cache) {
                Some(bits)
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sn_counts() {
        assert_eq!(generate_sn(1).len(), 1);
        assert_eq!(generate_sn(2).len(), 2);
        assert_eq!(generate_sn(3).len(), 6);
        assert_eq!(generate_sn(5).len(), 120);
        assert_eq!(generate_sn(7).len(), 5040);
        assert_eq!(generate_sn(8).len(), 40320);
    }

    #[test]
    fn cache_n8_matches() {
        let c = SnBitCache::new(8);
        assert_eq!(c.tables.len(), 40320);
        assert_eq!(c.tables[0].len(), 28);
    }

    #[test]
    fn is_canonical_empty_and_complete() {
        let c = SnBitCache::new(7);
        assert!(is_canonical(0u32, &c));
        let k7 = mask_for_n(7);
        assert!(is_canonical(k7, &c));
    }

    #[test]
    fn is_canonical_matches_full_orbit_n5() {
        // For n=5 (1024 candidates), every canonical representative from
        // the full-orbit approach must also pass the canonicity predicate.
        let c = SnBitCache::new(5);
        let mask = mask_for_n(5);
        for bits in 0..=mask {
            let r = fast_orbit_n(bits, &c);
            let is_own_canon = r.canonical == bits;
            assert_eq!(
                is_own_canon,
                is_canonical(bits, &c),
                "mismatch at bits={:05b}",
                bits
            );
        }
    }

    #[test]
    fn orderly_count_matches_oeis_for_n6() {
        // OEIS A000088 for n=6 is 156.
        let c = SnBitCache::new(6);
        let mask = mask_for_n(6);
        let mut count = 0usize;
        for bits in 0..=mask {
            if is_canonical(bits, &c) {
                count += 1;
            }
        }
        assert_eq!(count, 156);
    }

    #[test]
    fn parallel_enumeration_matches_sequential_n6() {
        let c = SnBitCache::new(6);
        let par = enumerate_canonical_parallel(&c);
        let seq = enumerate_canonical_sequential(&c);
        let mut par_sorted = par;
        par_sorted.sort();
        assert_eq!(par_sorted, seq);
        assert_eq!(seq.len(), 156);
    }
}

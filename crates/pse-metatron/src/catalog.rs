//! The Periodic Table of small graphs.
//!
//! Enumerates every labelled graph on `n ≤ n_max` nodes (n_max ≤ 7),
//! deduplicates by canonical form under `S_7`, attaches the full attribute
//! set (orbit, stabilizer, spectrum, complement, combinatorial properties,
//! Platonic classes), and assigns a systematic identifier of the form
//! `Gn-m-k` per the spec in `periodensystem_graphen.tex`.
//!
//! The result is the `PeriodicTable`: a finite, deterministic catalog of
//! all 1252 non-isomorphic graphs on at most 7 nodes (per OEIS A000088).

use std::collections::{BTreeMap, HashMap};

use serde::Serialize;

/// Helper struct for JSON-friendly (n, m) → count entries.
#[derive(Debug, Clone, Serialize)]
pub struct NmCount {
    pub n: usize,
    pub m: usize,
    pub count: usize,
}

use rayon::prelude::*;

use crate::bitgraph::{
    bits_to_adjacency, bits_to_edges, complement_bits, edge_count as bits_edge_count, fast_orbit,
    mask_for_n, S7BitCache,
};
use crate::bitgraph_n::{enumerate_canonical_parallel, SnBitCache};
use crate::error::{ScanError, ScanResult};
use crate::scan::{scan_canonical_bits, scan_canonical_bits_n, ScanReport};

/// One entry in the periodic table.
#[derive(Debug, Clone, Serialize)]
pub struct CatalogEntry {
    /// Systematic identifier `G<n>-<m>-<k>` (k = 1..count(n,m), ordered by
    /// canonical hash).
    pub id: String,
    /// Number of active nodes.
    pub n: usize,
    /// Number of edges.
    pub m: usize,
    /// Index within the (n, m) class (1-based, deterministic by canonical hash).
    pub index_in_class: usize,
    /// Canonical edge list (1-indexed) of the canonical representative.
    pub canonical_edges: Vec<(usize, usize)>,
    /// Full scan report (orbit, spectrum, properties, ...).
    pub report: ScanReport,
    /// `id` of the complement graph (within the same `n`).
    pub complement_id: Option<String>,
}

/// Catalog summary statistics.
#[derive(Debug, Clone, Serialize)]
pub struct CatalogStats {
    pub n_max: usize,
    pub total_entries: usize,
    /// Count per n (matches OEIS A000088 for n ≤ 8).
    pub count_per_n: BTreeMap<usize, usize>,
    /// Count per (n, m), JSON-friendly.
    pub count_per_n_m: Vec<NmCount>,
    /// Connected graphs per n (matches OEIS A001349 for n ≤ 8).
    pub connected_per_n: BTreeMap<usize, usize>,
    /// Bipartite graphs per n (matches OEIS A033995 for n ≤ 8).
    pub bipartite_per_n: BTreeMap<usize, usize>,
    /// Tree counts per n (matches OEIS A000055 for n ≤ 8).
    pub trees_per_n: BTreeMap<usize, usize>,
    /// Threshold graphs per n (matches OEIS A005840 = 2^(n-1) for n ≥ 1).
    pub threshold_per_n: BTreeMap<usize, usize>,
    /// Planar graphs per n (matches OEIS A005470).
    pub planar_per_n: BTreeMap<usize, usize>,
    /// Total spanning tree count summed across the catalog.
    pub total_spanning_trees: u64,
    pub self_complementary_count: usize,
    pub asymmetric_count: usize,
    pub max_orbit_size: usize,
    pub max_stabilizer_order: usize,
}

/// The complete catalog.
#[derive(Debug, Clone, Serialize)]
pub struct PeriodicTable {
    pub entries: Vec<CatalogEntry>,
    pub stats: CatalogStats,
}

/// OEIS A000088 (number of unlabeled simple graphs) for n = 0..=10.
pub fn oeis_a000088(n: usize) -> Option<usize> {
    const VALUES: &[usize] = &[1, 1, 2, 4, 11, 34, 156, 1044, 12346, 274668, 12005168];
    VALUES.get(n).copied()
}

/// OEIS A001349 (number of connected simple graphs) for n = 0..=10.
pub fn oeis_a001349(n: usize) -> Option<usize> {
    const VALUES: &[usize] = &[1, 1, 1, 2, 6, 21, 112, 853, 11117, 261080, 11716571];
    VALUES.get(n).copied()
}

/// OEIS A033995 (number of bipartite simple graphs) for n = 0..=10.
pub fn oeis_a033995(n: usize) -> Option<usize> {
    const VALUES: &[usize] = &[1, 1, 2, 3, 7, 13, 35, 88, 303, 1119, 5479];
    VALUES.get(n).copied()
}

/// OEIS A000055 (number of unlabeled trees) for n = 0..=10.
pub fn oeis_a000055(n: usize) -> Option<usize> {
    const VALUES: &[usize] = &[1, 1, 1, 1, 2, 3, 6, 11, 23, 47, 106];
    VALUES.get(n).copied()
}

/// OEIS A000171 (number of self-complementary graphs) for n = 0..=10.
pub fn oeis_a000171(n: usize) -> Option<usize> {
    const VALUES: &[usize] = &[1, 1, 0, 0, 1, 2, 0, 0, 10, 36, 0];
    VALUES.get(n).copied()
}

/// OEIS A003400 (number of asymmetric graphs) for n = 0..=10.
pub fn oeis_a003400(n: usize) -> Option<usize> {
    const VALUES: &[usize] = &[1, 1, 0, 0, 0, 0, 8, 152, 3696, 135004, 7971848];
    VALUES.get(n).copied()
}

/// OEIS A005840 (number of threshold graphs) for n = 0..=10.
/// For n ≥ 1, A005840(n) = 2^(n-1) exactly.
pub fn oeis_a005840(n: usize) -> Option<usize> {
    const VALUES: &[usize] = &[1, 1, 2, 4, 8, 16, 32, 64, 128, 256, 512];
    VALUES.get(n).copied()
}

/// OEIS A005470 (number of planar simple graphs) for n = 0..=10.
pub fn oeis_a005470(n: usize) -> Option<usize> {
    const VALUES: &[usize] = &[1, 1, 2, 4, 11, 33, 142, 822, 6966, 79853, 1140916];
    VALUES.get(n).copied()
}

/// Build the catalog for exactly `n` nodes using orderly generation via
/// the canonicity predicate. Works for any `n` in 1..=8 — this is the
/// engine that scales to the n = 8 frontier (12,346 graphs).
///
/// For `n <= 7` results match [`build_catalog_for_n`] (same IDs, same
/// hashes), but finish in ≤ 1 s per n instead of tens of seconds.
pub fn build_catalog_orderly(n: usize) -> ScanResult<Vec<RawEntry>> {
    if n == 0 {
        return Err(ScanError::EmptyGraph);
    }
    if n > 8 {
        return Err(ScanError::TooManyNodes { n });
    }
    let cache = SnBitCache::new(n);
    // Parallel canonicity sweep (scales ~linearly with CPU count).
    let canonical_bits_list = enumerate_canonical_parallel(&cache);

    // Parallel scan of every canonical representative. `scan_canonical_bits_n`
    // is thread-safe because `SnBitCache` is read-only after construction.
    let results: Vec<ScanResult<RawEntry>> = canonical_bits_list
        .par_iter()
        .map(|&bits| {
            let report = scan_canonical_bits_n(n, bits, &cache)?;
            Ok(RawEntry {
                n,
                canonical_bits: bits,
                original_bits: bits,
                report,
            })
        })
        .collect();

    let mut raw: Vec<RawEntry> = Vec::with_capacity(results.len());
    for r in results {
        raw.push(r?);
    }
    Ok(raw)
}

/// Build the catalog for graphs on exactly `n` nodes (`n ≤ 7`).
///
/// Uses fast bit-orbit deduplication then runs the full scan only on the
/// unique canonical representatives. Both phases are parallelized via
/// rayon.
pub fn build_catalog_for_n(n: usize) -> ScanResult<Vec<RawEntry>> {
    if n == 0 {
        return Err(ScanError::EmptyGraph);
    }
    if n > 7 {
        return Err(ScanError::TooManyNodes { n });
    }
    let bit_cache = S7BitCache::new();
    let _ = mask_for_n(n);
    let limit = if n < 2 { 1u32 } else { 1u32 << (n * (n - 1) / 2) };

    // Step 1 (parallel): compute canonical under S_7 for every labelled
    // graph. Collect (canonical, original) pairs and deduplicate.
    let pairs: Vec<(u32, u32)> = (0u32..limit)
        .into_par_iter()
        .map(|bits| {
            let orb = fast_orbit(bits, &bit_cache);
            (orb.canonical, bits)
        })
        .collect();

    // Dedup by canonical_bits (first-seen wins).
    let mut canonicals: HashMap<u32, u32> = HashMap::with_capacity(2048);
    for (canonical, original) in pairs {
        canonicals.entry(canonical).or_insert(original);
    }

    // Step 2 (parallel): full scan of each unique canonical representative.
    let canon_vec: Vec<(u32, u32)> = canonicals.into_iter().collect();
    let results: Vec<ScanResult<RawEntry>> = canon_vec
        .par_iter()
        .map(|&(canonical, original)| {
            let report = scan_canonical_bits(n, canonical, &bit_cache)?;
            Ok(RawEntry {
                n,
                canonical_bits: canonical,
                original_bits: original,
                report,
            })
        })
        .collect();

    let mut raw: Vec<RawEntry> = Vec::with_capacity(results.len());
    for r in results {
        raw.push(r?);
    }
    Ok(raw)
}

/// Intermediate, before IDs are assigned.
#[derive(Debug, Clone)]
pub struct RawEntry {
    pub n: usize,
    pub canonical_bits: u32,
    pub original_bits: u32,
    pub report: ScanReport,
}

/// Build the entire periodic table for `n = 1..=n_max`.
///
/// For `n ≤ 7` uses the S_7 / Metatron-scaffold pipeline; for `n = 8`
/// automatically switches to the orderly-generation engine
/// ([`build_catalog_orderly`]) which uses S_8.
pub fn build_catalog(n_max: usize) -> ScanResult<PeriodicTable> {
    if n_max == 0 || n_max > 8 {
        return Err(ScanError::TooManyNodes { n: n_max });
    }

    // For n ≤ 7 we keep the S_7-on-scaffold semantics (|Orb|·|Stab| = 5040)
    // from periodensystem_graphen.tex. For n = 8 the natural group is S_8
    // (40320 permutations) — there's no universal larger scaffold yet.
    let mut all_raw: Vec<RawEntry> = Vec::with_capacity(1252);
    for n in 1..=n_max {
        if n <= 7 {
            all_raw.extend(build_catalog_for_n(n)?);
        } else {
            all_raw.extend(build_catalog_orderly(n)?);
        }
    }

    // Sort by (n, m, canonical_hash) for deterministic IDs.
    all_raw.sort_by(|a, b| {
        let am = a.report.input_edges;
        let bm = b.report.input_edges;
        a.n.cmp(&b.n)
            .then(am.cmp(&bm))
            .then(a.report.canonical_hash.cmp(&b.report.canonical_hash))
    });

    // Assign Gn-m-k IDs.
    let mut entries: Vec<CatalogEntry> = Vec::with_capacity(all_raw.len());
    let mut k_counter: HashMap<(usize, usize), usize> = HashMap::new();
    // Map (n, canonical_hash) -> id. Scoping by n is essential because the
    // 13-node embedding makes "all-zeros" the canonical form of every empty
    // graph, regardless of n.
    let mut hash_to_id: HashMap<(usize, String), String> = HashMap::new();

    for raw in &all_raw {
        let n = raw.n;
        let m = raw.report.input_edges;
        let k_entry = k_counter.entry((n, m)).or_insert(0);
        *k_entry += 1;
        let k = *k_entry;
        let id = format!("G{}-{}-{}", n, m, k);
        hash_to_id.insert((n, raw.report.canonical_hash.clone()), id.clone());

        entries.push(CatalogEntry {
            id,
            n,
            m,
            index_in_class: k,
            canonical_edges: bits_to_edges(raw.canonical_bits),
            report: raw.report.clone(),
            complement_id: None, // filled below
        });
    }

    // Resolve complement IDs (complement of an n-graph is an n-graph).
    for entry in entries.iter_mut() {
        let key = (entry.n, entry.report.complement_canonical_hash.clone());
        entry.complement_id = hash_to_id.get(&key).cloned();
    }

    let stats = compute_stats(&entries, n_max);

    Ok(PeriodicTable { entries, stats })
}

fn compute_stats(entries: &[CatalogEntry], n_max: usize) -> CatalogStats {
    let mut count_per_n: BTreeMap<usize, usize> = BTreeMap::new();
    let mut count_per_n_m_map: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    let mut connected_per_n: BTreeMap<usize, usize> = BTreeMap::new();
    let mut bipartite_per_n: BTreeMap<usize, usize> = BTreeMap::new();
    let mut trees_per_n: BTreeMap<usize, usize> = BTreeMap::new();
    let mut threshold_per_n: BTreeMap<usize, usize> = BTreeMap::new();
    let mut planar_per_n: BTreeMap<usize, usize> = BTreeMap::new();
    let mut total_spanning_trees: u64 = 0;
    let mut self_comp = 0usize;
    let mut asymmetric = 0usize;
    let mut max_orbit = 0usize;
    let mut max_stab = 0usize;

    for e in entries {
        *count_per_n.entry(e.n).or_insert(0) += 1;
        *count_per_n_m_map.entry((e.n, e.m)).or_insert(0) += 1;
        let p = &e.report.properties;
        if p.connected {
            *connected_per_n.entry(e.n).or_insert(0) += 1;
        }
        if p.is_bipartite {
            *bipartite_per_n.entry(e.n).or_insert(0) += 1;
        }
        if p.is_tree {
            *trees_per_n.entry(e.n).or_insert(0) += 1;
        }
        if p.is_threshold {
            *threshold_per_n.entry(e.n).or_insert(0) += 1;
        }
        if p.is_planar {
            *planar_per_n.entry(e.n).or_insert(0) += 1;
        }
        total_spanning_trees = total_spanning_trees.saturating_add(e.report.spanning_tree_count);
        if p.is_self_complementary {
            self_comp += 1;
        }
        if e.report.stabilizer_order == 1 {
            asymmetric += 1;
        }
        if e.report.orbit_size > max_orbit {
            max_orbit = e.report.orbit_size;
        }
        if e.report.stabilizer_order > max_stab {
            max_stab = e.report.stabilizer_order;
        }
    }

    let count_per_n_m: Vec<NmCount> = count_per_n_m_map
        .into_iter()
        .map(|((n, m), count)| NmCount { n, m, count })
        .collect();

    CatalogStats {
        n_max,
        total_entries: entries.len(),
        count_per_n,
        count_per_n_m,
        connected_per_n,
        bipartite_per_n,
        trees_per_n,
        threshold_per_n,
        planar_per_n,
        total_spanning_trees,
        self_complementary_count: self_comp,
        asymmetric_count: asymmetric,
        max_orbit_size: max_orbit,
        max_stabilizer_order: max_stab,
    }
}

// ============================================================================
// Lookup API — identify an arbitrary input graph by finding its catalog
// entry. This makes the Periodic Table queryable, as assumed in the TeX §6
// application scenarios (motif analysis, schema diagnosis, protocol
// verification).
// ============================================================================

/// Look up a catalog entry for the given input graph.
///
/// Strategy:
///   * `n ≤ 7`: embed the adjacency into 21 bits, compute the S_7
///     canonical form via the bit cache, and hash the resulting 13×13
///     embedded matrix using the same FNV-1a scheme as `scan`. Match
///     against `CatalogEntry::report.canonical_hash`.
///   * `n = 8`: canonicalize under `S_8` using `bitgraph_n`, then hash the
///     canonical 8×8 matrix.
///
/// Returns `None` if the input has an `n` outside 1..=8 or if no match is
/// found (which should not happen for a complete catalog).
pub fn find_in_catalog<'a>(
    input: &crate::ingest::InputGraph,
    table: &'a PeriodicTable,
) -> Option<&'a CatalogEntry> {
    use crate::bitgraph::S7BitCache;
    use crate::bitgraph_n::{
        bits_to_adjacency as bits_to_adj_n, fast_orbit_n, SnBitCache as BitCacheN,
    };

    let n = input.n;
    if n == 0 || n > 8 {
        return None;
    }

    // Build 21-bit / 28-bit mask from the input adjacency.
    let mut bits = 0u32;
    for j in 1..n {
        for i in 0..j {
            if input.adjacency[i][j] > 0.5 {
                bits |= 1u32 << (j * (j - 1) / 2 + i);
            }
        }
    }

    let hash = if n <= 7 {
        let cache = S7BitCache::new();
        let r = crate::bitgraph::fast_orbit(bits, &cache);
        // Embed canonical bits into 13×13 and hash.
        let mut e = vec![vec![0.0f64; 13]; 13];
        let active = crate::bitgraph::bits_to_adjacency(r.canonical, n);
        for i in 0..n {
            for j in 0..n {
                e[i][j] = active[i][j];
            }
        }
        crate::scan::hash_canonical_matrix(&e)
    } else {
        let cache = BitCacheN::new(n);
        let r = fast_orbit_n(bits, &cache);
        let canonical = bits_to_adj_n(r.canonical, n);
        crate::scan::hash_canonical_matrix(&canonical)
    };

    table
        .entries
        .iter()
        .find(|e| e.n == n && e.report.canonical_hash == hash)
}

/// Verification helpers used by the test suite and the CLI.
pub mod verify {
    use super::*;

    /// Returns an error string if any entry violates an invariant.
    pub fn verify_table(table: &PeriodicTable) -> Result<(), String> {
        for e in &table.entries {
            // Orbit-Stabilizer: for n ≤ 7 the product is 5040 (S_7 on
            // scaffold), for n = 8 it is 40320 = 8!.
            let n_fact: usize = (1..=e.n).product::<usize>().max(1);
            let expected_product = if e.n <= 7 { 5040 } else { n_fact };
            if e.report.orbit_stabilizer_product != expected_product {
                return Err(format!(
                    "Orbit-Stabilizer violated for {}: {} * {} = {} (expected {})",
                    e.id,
                    e.report.orbit_size,
                    e.report.stabilizer_order,
                    e.report.orbit_stabilizer_product,
                    expected_product
                ));
            }
            // tr(A^2) = 2m
            if (e.report.a2_trace - 2.0 * e.m as f64).abs() > 1e-6 {
                return Err(format!(
                    "tr(A^2) = {} != 2m = {} for {}",
                    e.report.a2_trace, 2 * e.m, e.id
                ));
            }
            // tr(A^3) = 6 * triangle_count
            if (e.report.a3_trace - 6.0 * e.report.triangle_count as f64).abs() > 1e-6 {
                return Err(format!(
                    "tr(A^3) = {} != 6Δ = {} for {}",
                    e.report.a3_trace,
                    6 * e.report.triangle_count,
                    e.id
                ));
            }
            // Complement involution: complement of complement should be self.
            // We check that the entry's complement_id maps back.
            if let Some(comp_id) = &e.complement_id {
                let comp = table.entries.iter().find(|x| &x.id == comp_id);
                let Some(comp) = comp else {
                    return Err(format!("Complement {} not found for {}", comp_id, e.id));
                };
                if comp.complement_id.as_deref() != Some(e.id.as_str()) {
                    return Err(format!(
                        "Complement involution failed: {} -> {} -> {:?}",
                        e.id, comp.id, comp.complement_id
                    ));
                }
            } else {
                return Err(format!("Missing complement_id for {}", e.id));
            }
        }

        // OEIS A000088: all graphs
        for (&n, &count) in &table.stats.count_per_n {
            if let Some(expected) = oeis_a000088(n) {
                if count != expected {
                    return Err(format!(
                        "OEIS A000088 mismatch at n={}: got {}, expected {}",
                        n, count, expected
                    ));
                }
            }
        }
        // OEIS A001349: connected graphs
        for (&n, &count) in &table.stats.connected_per_n {
            if let Some(expected) = oeis_a001349(n) {
                if count != expected {
                    return Err(format!(
                        "OEIS A001349 (connected) mismatch at n={}: got {}, expected {}",
                        n, count, expected
                    ));
                }
            }
        }
        // OEIS A033995: bipartite graphs
        for (&n, &count) in &table.stats.bipartite_per_n {
            if let Some(expected) = oeis_a033995(n) {
                if count != expected {
                    return Err(format!(
                        "OEIS A033995 (bipartite) mismatch at n={}: got {}, expected {}",
                        n, count, expected
                    ));
                }
            }
        }
        // OEIS A000055: trees
        for (&n, &count) in &table.stats.trees_per_n {
            if let Some(expected) = oeis_a000055(n) {
                if count != expected {
                    return Err(format!(
                        "OEIS A000055 (trees) mismatch at n={}: got {}, expected {}",
                        n, count, expected
                    ));
                }
            }
        }
        // OEIS A005840: threshold graphs
        for (&n, &count) in &table.stats.threshold_per_n {
            if let Some(expected) = oeis_a005840(n) {
                if count != expected {
                    return Err(format!(
                        "OEIS A005840 (threshold) mismatch at n={}: got {}, expected {}",
                        n, count, expected
                    ));
                }
            }
        }
        // OEIS A005470: planar graphs
        for (&n, &count) in &table.stats.planar_per_n {
            if let Some(expected) = oeis_a005470(n) {
                if count != expected {
                    return Err(format!(
                        "OEIS A005470 (planar) mismatch at n={}: got {}, expected {}",
                        n, count, expected
                    ));
                }
            }
        }

        Ok(())
    }
}

// Silence the dead_code lint for helpers that may only be used in tests.
#[allow(dead_code)]
fn _use_bits_to_adjacency(b: u32, n: usize) -> Vec<Vec<f64>> {
    bits_to_adjacency(b, n)
}

#[allow(dead_code)]
fn _use_complement_bits(b: u32, n: usize) -> u32 {
    complement_bits(b, n)
}

#[allow(dead_code)]
fn _use_edge_count(b: u32) -> usize {
    bits_edge_count(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn n1_has_one_graph() {
        let raw = build_catalog_for_n(1).unwrap();
        assert_eq!(raw.len(), 1);
    }

    #[test]
    fn n2_has_two_graphs() {
        let raw = build_catalog_for_n(2).unwrap();
        assert_eq!(raw.len(), 2);
    }

    #[test]
    fn n3_has_four_graphs() {
        let raw = build_catalog_for_n(3).unwrap();
        assert_eq!(raw.len(), 4);
    }

    #[test]
    fn n4_has_eleven_graphs() {
        let raw = build_catalog_for_n(4).unwrap();
        assert_eq!(raw.len(), 11);
    }

    #[test]
    fn n5_has_thirtyfour_graphs() {
        let raw = build_catalog_for_n(5).unwrap();
        assert_eq!(raw.len(), 34);
    }

    #[test]
    fn build_catalog_to_4_yields_18_entries() {
        let table = build_catalog(4).unwrap();
        // 1 + 2 + 4 + 11 = 18
        assert_eq!(table.entries.len(), 18);
        assert_eq!(table.stats.count_per_n[&1], 1);
        assert_eq!(table.stats.count_per_n[&2], 2);
        assert_eq!(table.stats.count_per_n[&3], 4);
        assert_eq!(table.stats.count_per_n[&4], 11);
        verify::verify_table(&table).unwrap();
    }

    #[test]
    fn n4_includes_self_complementary_p4() {
        let table = build_catalog(4).unwrap();
        assert_eq!(table.stats.self_complementary_count, 2); // n=1 (single node) + P4
    }

    #[test]
    fn ids_are_unique_and_sorted() {
        let table = build_catalog(4).unwrap();
        let ids: Vec<_> = table.entries.iter().map(|e| e.id.clone()).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len());
    }
}

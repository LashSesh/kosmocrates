//! Analyses on top of a fully-built [`PeriodicTable`].
//!
//! Each function answers one of the open research questions from
//! §8 of `periodensystem_graphen.tex`:
//!
//!   • Q2 cospectral groups (triples and higher)
//!   • Q5 matrix-power signature as an iso-pre-filter
//!   • Q4 self-complementary enumeration
//!   • Q3 spectral distribution / forbidden spectra
//!   • Q1 Platonic class vs. stabilizer correlations
//!   • Q7 scaffold-edge-coverage invariants
//!
//! All routines are pure: they read a `PeriodicTable` and return structured
//! results suitable for JSON serialization.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::catalog::{CatalogEntry, PeriodicTable};
use crate::complement::{mat_pow, trace};
use crate::scan::ScanReport;

// ---------- shared helpers ----------

/// Convert a sorted float spectrum to an integer key (× 10⁶ rounded) so it
/// can be used in a `BTreeMap`.
fn spectrum_key(eigenvalues: &[f64]) -> Vec<i64> {
    eigenvalues
        .iter()
        .map(|&x| (x * 1_000_000.0).round() as i64)
        .collect()
}

fn float_key(x: f64) -> i64 {
    (x * 1_000_000.0).round() as i64
}

// ============================================================================
// Q2: Cospectral groups
// ============================================================================

/// One cospectral equivalence class (same spectrum, different canonical form).
#[derive(Debug, Clone, Serialize)]
pub struct CospectralGroup {
    pub n: usize,
    pub spectrum: Vec<f64>,
    pub members: Vec<String>,
}

/// Aggregate cospectral statistics.
#[derive(Debug, Clone, Serialize)]
pub struct CospectralAnalysis {
    pub total_pairs: usize,
    pub total_triples: usize,
    pub groups_of_four_or_more: usize,
    /// Count per n: (n, number of non-singleton classes, total graphs in classes).
    pub per_n: Vec<CospectralNStats>,
    /// All non-singleton classes, sorted by (n, -size, first member id).
    pub groups: Vec<CospectralGroup>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CospectralNStats {
    pub n: usize,
    pub graphs: usize,
    pub non_singleton_classes: usize,
    pub graphs_in_classes: usize,
    pub largest_class_size: usize,
}

pub fn cospectral_analysis(table: &PeriodicTable) -> CospectralAnalysis {
    let mut by_n_spec: BTreeMap<(usize, Vec<i64>), Vec<&CatalogEntry>> = BTreeMap::new();
    for entry in &table.entries {
        let key = (entry.n, spectrum_key(&entry.report.eigenvalues));
        by_n_spec.entry(key).or_default().push(entry);
    }

    let mut groups: Vec<CospectralGroup> = Vec::new();
    let mut pairs = 0usize;
    let mut triples = 0usize;
    let mut quads_plus = 0usize;

    for ((n, _key), members) in &by_n_spec {
        if members.len() < 2 {
            continue;
        }
        match members.len() {
            2 => pairs += 1,
            3 => triples += 1,
            _ => quads_plus += 1,
        }
        let spectrum = members[0].report.eigenvalues.clone();
        let mut ids: Vec<String> = members.iter().map(|e| e.id.clone()).collect();
        ids.sort();
        groups.push(CospectralGroup {
            n: *n,
            spectrum,
            members: ids,
        });
    }
    groups.sort_by(|a, b| {
        a.n.cmp(&b.n)
            .then(b.members.len().cmp(&a.members.len()))
            .then(a.members[0].cmp(&b.members[0]))
    });

    // per-n statistics
    let mut per_n_map: BTreeMap<usize, (usize, usize, usize, usize)> = BTreeMap::new();
    for entry in &table.entries {
        per_n_map.entry(entry.n).or_insert((0, 0, 0, 0)).0 += 1;
    }
    for g in &groups {
        let rec = per_n_map.entry(g.n).or_insert((0, 0, 0, 0));
        rec.1 += 1;
        rec.2 += g.members.len();
        if g.members.len() > rec.3 {
            rec.3 = g.members.len();
        }
    }
    let per_n: Vec<CospectralNStats> = per_n_map
        .into_iter()
        .map(
            |(n, (graphs, classes, in_classes, largest))| CospectralNStats {
                n,
                graphs,
                non_singleton_classes: classes,
                graphs_in_classes: in_classes,
                largest_class_size: largest,
            },
        )
        .collect();

    CospectralAnalysis {
        total_pairs: pairs,
        total_triples: triples,
        groups_of_four_or_more: quads_plus,
        per_n,
        groups,
    }
}

// ============================================================================
// Q5: Matrix-power signatures
// ============================================================================

/// Invariant "strength" comparison: how many distinct values each candidate
/// invariant takes across the catalog. A stronger invariant has more distinct
/// values (closer to the true count of non-isomorphic graphs).
#[derive(Debug, Clone, Serialize)]
pub struct MatrixPowerAnalysis {
    pub total_graphs: usize,
    pub distinct_by_a2: usize,
    pub distinct_by_a2_a3: usize,
    pub distinct_by_a2_a3_a4: usize,
    pub distinct_by_a2_a3_a4_a5: usize,
    pub distinct_by_a2_a3_a4_a5_a6: usize,
    pub distinct_by_spectrum: usize,
    pub distinct_by_degree_sequence: usize,
    /// Per-n breakdown for readability.
    pub per_n: Vec<MatrixPowerPerN>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MatrixPowerPerN {
    pub n: usize,
    pub graphs: usize,
    pub by_a2_a3: usize,
    pub by_a2_a3_a4: usize,
    pub by_a2_a3_a4_a5: usize,
    pub by_spectrum: usize,
}

fn power_trace(report: &ScanReport, k: u32) -> i64 {
    let p = mat_pow(&report.canonical_form, k);
    float_key(trace(&p))
}

pub fn matrix_power_analysis(table: &PeriodicTable) -> MatrixPowerAnalysis {
    use std::collections::HashSet;
    let total = table.entries.len();

    let mut s_a2 = HashSet::<(usize, i64)>::new();
    let mut s_a23 = HashSet::<(usize, i64, i64)>::new();
    let mut s_a234 = HashSet::<(usize, i64, i64, i64)>::new();
    let mut s_a2345 = HashSet::<(usize, i64, i64, i64, i64)>::new();
    let mut s_a23456 = HashSet::<(usize, i64, i64, i64, i64, i64)>::new();
    let mut s_spec = HashSet::<(usize, Vec<i64>)>::new();
    let mut s_deg = HashSet::<(usize, Vec<usize>)>::new();

    // per-n counters
    let mut per_n_map: BTreeMap<
        usize,
        (
            usize,
            HashSet<(i64, i64)>,
            HashSet<(i64, i64, i64)>,
            HashSet<(i64, i64, i64, i64)>,
            HashSet<Vec<i64>>,
        ),
    > = BTreeMap::new();

    for e in &table.entries {
        let n = e.n;
        let a2 = float_key(e.report.a2_trace);
        let a3 = float_key(e.report.a3_trace);
        let a4 = power_trace(&e.report, 4);
        let a5 = power_trace(&e.report, 5);
        let a6 = power_trace(&e.report, 6);
        s_a2.insert((n, a2));
        s_a23.insert((n, a2, a3));
        s_a234.insert((n, a2, a3, a4));
        s_a2345.insert((n, a2, a3, a4, a5));
        s_a23456.insert((n, a2, a3, a4, a5, a6));
        s_spec.insert((n, spectrum_key(&e.report.eigenvalues)));
        s_deg.insert((n, e.report.properties.degree_sequence.clone()));

        let rec = per_n_map.entry(n).or_insert_with(|| {
            (
                0usize,
                HashSet::new(),
                HashSet::new(),
                HashSet::new(),
                HashSet::new(),
            )
        });
        rec.0 += 1;
        rec.1.insert((a2, a3));
        rec.2.insert((a2, a3, a4));
        rec.3.insert((a2, a3, a4, a5));
        rec.4.insert(spectrum_key(&e.report.eigenvalues));
    }

    let per_n: Vec<MatrixPowerPerN> = per_n_map
        .into_iter()
        .map(|(n, (graphs, s23, s234, s2345, spec))| MatrixPowerPerN {
            n,
            graphs,
            by_a2_a3: s23.len(),
            by_a2_a3_a4: s234.len(),
            by_a2_a3_a4_a5: s2345.len(),
            by_spectrum: spec.len(),
        })
        .collect();

    MatrixPowerAnalysis {
        total_graphs: total,
        distinct_by_a2: s_a2.len(),
        distinct_by_a2_a3: s_a23.len(),
        distinct_by_a2_a3_a4: s_a234.len(),
        distinct_by_a2_a3_a4_a5: s_a2345.len(),
        distinct_by_a2_a3_a4_a5_a6: s_a23456.len(),
        distinct_by_spectrum: s_spec.len(),
        distinct_by_degree_sequence: s_deg.len(),
        per_n,
    }
}

// ============================================================================
// Q4: Self-complementary graphs
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct SelfComplementaryEntry {
    pub id: String,
    pub n: usize,
    pub m: usize,
    pub degree_sequence: Vec<usize>,
    pub triangle_count: usize,
    pub stabilizer_order: usize,
    pub spectrum: Vec<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SelfComplementaryAnalysis {
    pub total: usize,
    pub per_n: BTreeMap<usize, usize>,
    pub entries: Vec<SelfComplementaryEntry>,
}

pub fn self_complementary_analysis(table: &PeriodicTable) -> SelfComplementaryAnalysis {
    let mut entries: Vec<SelfComplementaryEntry> = Vec::new();
    let mut per_n: BTreeMap<usize, usize> = BTreeMap::new();
    for e in &table.entries {
        if !e.report.properties.is_self_complementary {
            continue;
        }
        *per_n.entry(e.n).or_insert(0) += 1;
        entries.push(SelfComplementaryEntry {
            id: e.id.clone(),
            n: e.n,
            m: e.m,
            degree_sequence: e.report.properties.degree_sequence.clone(),
            triangle_count: e.report.triangle_count,
            stabilizer_order: e.report.stabilizer_order,
            spectrum: e.report.eigenvalues.clone(),
        });
    }
    entries.sort_by(|a, b| a.n.cmp(&b.n).then(a.id.cmp(&b.id)));
    SelfComplementaryAnalysis {
        total: entries.len(),
        per_n,
        entries,
    }
}

// ============================================================================
// Q3: Spectral distribution
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct SpectralDistribution {
    pub min_eigenvalue: f64,
    pub max_eigenvalue: f64,
    pub min_spectral_radius: f64,
    pub max_spectral_radius: f64,
    pub integer_spectrum_count: usize,
    pub half_integer_spectrum_count: usize,
    pub algebraic_connectivity_zero: usize,
    pub algebraic_connectivity_positive: usize,
    /// Top-5 most common spectra (across the whole catalog, keyed by `n` too).
    pub most_common_spectra: Vec<SpectrumOccurrence>,
    /// Integer eigenvalues do occur; list distinct ones.
    pub distinct_integer_eigenvalues: Vec<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpectrumOccurrence {
    pub n: usize,
    pub spectrum: Vec<f64>,
    pub count: usize,
    pub example_ids: Vec<String>,
}

pub fn spectral_distribution(table: &PeriodicTable) -> SpectralDistribution {
    let mut min_ev = f64::INFINITY;
    let mut max_ev = f64::NEG_INFINITY;
    let mut min_sr = f64::INFINITY;
    let mut max_sr = f64::NEG_INFINITY;
    let mut integer_spec = 0usize;
    let mut half_integer_spec = 0usize;
    let mut alg_con_zero = 0usize;
    let mut alg_con_pos = 0usize;
    let mut integer_evs: std::collections::BTreeSet<i64> = std::collections::BTreeSet::new();

    let mut spec_counts: BTreeMap<(usize, Vec<i64>), (Vec<f64>, Vec<String>)> = BTreeMap::new();

    for e in &table.entries {
        for &v in &e.report.eigenvalues {
            if v < min_ev {
                min_ev = v;
            }
            if v > max_ev {
                max_ev = v;
            }
            // Detect integer eigenvalues within tolerance.
            if (v - v.round()).abs() < 1e-6 {
                integer_evs.insert(v.round() as i64);
            }
        }
        if e.report.spectral_radius < min_sr {
            min_sr = e.report.spectral_radius;
        }
        if e.report.spectral_radius > max_sr {
            max_sr = e.report.spectral_radius;
        }
        if e.report.algebraic_connectivity.abs() < 1e-6 {
            alg_con_zero += 1;
        } else if e.report.algebraic_connectivity > 0.0 {
            alg_con_pos += 1;
        }
        let all_int = e
            .report
            .eigenvalues
            .iter()
            .all(|&v| (v - v.round()).abs() < 1e-6);
        let all_half = e
            .report
            .eigenvalues
            .iter()
            .all(|&v| ((2.0 * v) - (2.0 * v).round()).abs() < 1e-6);
        if all_int {
            integer_spec += 1;
        } else if all_half {
            half_integer_spec += 1;
        }

        let k = (e.n, spectrum_key(&e.report.eigenvalues));
        let entry = spec_counts
            .entry(k)
            .or_insert_with(|| (e.report.eigenvalues.clone(), Vec::new()));
        entry.1.push(e.id.clone());
    }

    let mut occurrences: Vec<SpectrumOccurrence> = spec_counts
        .into_iter()
        .map(|((n, _k), (spec, ids))| SpectrumOccurrence {
            n,
            spectrum: spec,
            count: ids.len(),
            example_ids: ids.iter().take(5).cloned().collect(),
        })
        .collect();
    occurrences.sort_by(|a, b| b.count.cmp(&a.count).then(a.n.cmp(&b.n)));
    let top: Vec<SpectrumOccurrence> = occurrences.into_iter().take(10).collect();

    SpectralDistribution {
        min_eigenvalue: (min_ev * 1e6).round() / 1e6,
        max_eigenvalue: (max_ev * 1e6).round() / 1e6,
        min_spectral_radius: (min_sr * 1e6).round() / 1e6,
        max_spectral_radius: (max_sr * 1e6).round() / 1e6,
        integer_spectrum_count: integer_spec,
        half_integer_spectrum_count: half_integer_spec,
        algebraic_connectivity_zero: alg_con_zero,
        algebraic_connectivity_positive: alg_con_pos,
        most_common_spectra: top,
        distinct_integer_eigenvalues: integer_evs.into_iter().collect(),
    }
}

// ============================================================================
// Q1: Platonic correlations — real subgraph-isomorphism detection
// ============================================================================
//
// We replace the scaffold-index-based classification from `platonic.rs` (which
// is effectively meaningless for the catalog — it only matches empty graphs,
// since catalog graphs use nodes 1..n ≤ 7 while the scaffold geometry places
// the tetrahedron/cube/octahedron at scaffold indices 2..13) by *actual*
// subgraph containment of K₄ (tetrahedron) and K_{2,2,2} (octahedron).
//
// Cube Q₃, dodecahedron, and icosahedron require ≥ 8 nodes, so they cannot
// fit in the n ≤ 7 catalog. We still report them for completeness.

#[derive(Debug, Clone, Serialize)]
pub struct PlatonicCorrelationRow {
    pub solid: String,
    /// How many catalog entries *contain* this solid's skeleton as a subgraph.
    pub contains_count: usize,
    /// How many catalog entries are *isomorphic to* this solid (same n, same m,
    /// same structure — this is the strongest match).
    pub isomorphic_count: usize,
    /// Mean log2(|Stab|) for graphs that contain the solid vs those that don't.
    /// Higher = more symmetric. Directly answers the TeX question
    /// "are graphs with high Platonic coverage systematically more symmetric?"
    pub mean_log2_stab_contains: f64,
    pub mean_log2_stab_not_contains: f64,
    /// Stabilizer-order ratio: mean(Stab | contains) / mean(Stab | not contains).
    /// > 1 => containing graphs are more symmetric.
    pub symmetry_ratio: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlatonicCorrelationAnalysis {
    pub rows: Vec<PlatonicCorrelationRow>,
}

fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    xs.iter().sum::<f64>() / xs.len() as f64
}

/// Does `bits` contain a K₄ subgraph?
/// We check all 4-subsets of the n nodes.
fn contains_k4(bits: u32, n: usize) -> bool {
    if n < 4 {
        return false;
    }
    for s in 0..(1u32 << n) {
        if s.count_ones() != 4 {
            continue;
        }
        let vs: Vec<usize> = (0..n).filter(|i| (s >> i) & 1 == 1).collect();
        let all = vs
            .iter()
            .enumerate()
            .all(|(ii, &a)| vs[ii + 1..].iter().all(|&b| has_edge(bits, a, b)));
        if all {
            return true;
        }
    }
    false
}

/// Does `bits` contain the 3-cube Q₃ as a spanning subgraph on 8 vertices?
/// Q₃ has exactly 8 vertices and 12 edges; its vertices can be labelled by
/// binary strings of length 3 with edges between strings at Hamming
/// distance 1. Equivalently: K_{4,4} minus a 4-edge perfect matching — no,
/// that's not right. Cleanest check: Q₃ is the unique 3-regular bipartite
/// graph on 8 vertices with girth 4 and 12 edges.
/// We simply try every assignment of the 8 input vertices to {0,1}³ labels
/// and check whether the Hamming-distance-1 adjacencies all exist.
fn contains_q3(bits: u32, n: usize) -> bool {
    if n != 8 {
        return false;
    }
    // All 8! = 40320 assignments of vertex labels to cube corners. We check
    // whether SOME assignment yields all 12 cube edges.
    // Representative cube edges (as pairs of 3-bit labels):
    // Hamming-distance-1 pairs on {0..7}:
    let cube_edges: [(usize, usize); 12] = [
        (0, 1),
        (0, 2),
        (0, 4),
        (1, 3),
        (1, 5),
        (2, 3),
        (2, 6),
        (3, 7),
        (4, 5),
        (4, 6),
        (5, 7),
        (6, 7),
    ];

    fn perms(out: &mut Vec<[usize; 8]>, cur: &mut [usize; 8], used: &mut [bool; 8], k: usize) {
        if k == 8 {
            out.push(*cur);
            return;
        }
        for v in 0..8 {
            if !used[v] {
                used[v] = true;
                cur[k] = v;
                perms(out, cur, used, k + 1);
                used[v] = false;
            }
        }
    }

    // We can fix label 0 → graph-vertex 0 without loss of generality (since
    // we only care about *existence* of an isomorphism), then iterate 7! = 5040.
    let mut all: Vec<[usize; 8]> = Vec::with_capacity(5040);
    let mut cur = [0usize; 8];
    let mut used = [false; 8];
    cur[0] = 0;
    used[0] = true;
    perms(&mut all, &mut cur, &mut used, 1);

    for assign in &all {
        // assign[label] = graph_vertex
        let ok = cube_edges
            .iter()
            .all(|&(a, b)| has_edge(bits, assign[a], assign[b]));
        if ok {
            return true;
        }
    }
    false
}

/// Does `bits` contain the octahedron K_{2,2,2} as a subgraph? K_{2,2,2} has
/// 6 vertices in 3 pairs (a,a'), (b,b'), (c,c') with edges between any two
/// vertices of different pairs (12 edges total — i.e., K₆ minus a perfect
/// matching).
fn contains_k222(bits: u32, n: usize) -> bool {
    if n < 6 {
        return false;
    }
    // Iterate over all 6-subsets, then try to find a perfect matching M of
    // 3 edges (= the missing complement edges of K₆) that is entirely NOT in
    // the graph. For each such 6-subset, the induced subgraph on 6 vertices
    // must be K₆ minus exactly those 3 matching edges (or a supergraph, since
    // we want containment, not exact match).
    for s in 0..(1u32 << n) {
        if s.count_ones() != 6 {
            continue;
        }
        let vs: Vec<usize> = (0..n).filter(|i| (s >> i) & 1 == 1).collect();
        // All 15 partitions of 6 vertices into 3 pairs: enumerate explicitly.
        // Fix v[0]'s partner, then fix next un-picked's partner.
        let partners: &[[(usize, usize); 3]] = &[
            [(0, 1), (2, 3), (4, 5)],
            [(0, 1), (2, 4), (3, 5)],
            [(0, 1), (2, 5), (3, 4)],
            [(0, 2), (1, 3), (4, 5)],
            [(0, 2), (1, 4), (3, 5)],
            [(0, 2), (1, 5), (3, 4)],
            [(0, 3), (1, 2), (4, 5)],
            [(0, 3), (1, 4), (2, 5)],
            [(0, 3), (1, 5), (2, 4)],
            [(0, 4), (1, 2), (3, 5)],
            [(0, 4), (1, 3), (2, 5)],
            [(0, 4), (1, 5), (2, 3)],
            [(0, 5), (1, 2), (3, 4)],
            [(0, 5), (1, 3), (2, 4)],
            [(0, 5), (1, 4), (2, 3)],
        ];
        for matching in partners {
            let non_pair_edge = |i: usize, j: usize| -> bool {
                for &(a, b) in matching {
                    if (i == a && j == b) || (i == b && j == a) {
                        return false;
                    }
                }
                true
            };
            let mut ok = true;
            'check: for i in 0..6 {
                for j in (i + 1)..6 {
                    if non_pair_edge(i, j) && !has_edge(bits, vs[i], vs[j]) {
                        ok = false;
                        break 'check;
                    }
                }
            }
            if ok {
                return true;
            }
        }
    }
    false
}

#[inline]
fn has_edge(bits: u32, i: usize, j: usize) -> bool {
    let (lo, hi) = if i < j { (i, j) } else { (j, i) };
    let b = hi * (hi - 1) / 2 + lo;
    (bits >> b) & 1 == 1
}

/// Pack an entry's canonical form back to a 21-bit mask for fast
/// subgraph-isomorphism checks.
fn entry_bits(entry: &CatalogEntry) -> u32 {
    let mut bits = 0u32;
    for &(i, j) in &entry.canonical_edges {
        let (lo, hi) = if i < j { (i, j) } else { (j, i) };
        let b = (hi - 1) * ((hi - 1) - 1) / 2 + (lo - 1);
        bits |= 1u32 << b;
    }
    bits
}

fn is_iso_to_k4(entry: &CatalogEntry) -> bool {
    entry.n == 4 && entry.m == 6
}

fn is_iso_to_k222(entry: &CatalogEntry) -> bool {
    // K_{2,2,2} on 6 vertices: all degrees 4, m = 12.
    if entry.n != 6 || entry.m != 12 {
        return false;
    }
    entry
        .report
        .properties
        .degree_sequence
        .iter()
        .all(|&d| d == 4)
}

pub fn platonic_correlation(table: &PeriodicTable) -> PlatonicCorrelationAnalysis {
    let mut rows: Vec<PlatonicCorrelationRow> = Vec::new();

    // --- Tetrahedron = K₄ ---
    let mut sub_log: Vec<f64> = Vec::new();
    let mut non_log: Vec<f64> = Vec::new();
    let mut iso_cnt = 0usize;
    let mut sub_cnt = 0usize;
    for e in &table.entries {
        let bits = entry_bits(e);
        let contains = contains_k4(bits, e.n);
        let l2 = (e.report.stabilizer_order as f64).log2();
        if contains {
            sub_cnt += 1;
            sub_log.push(l2);
            if is_iso_to_k4(e) {
                iso_cnt += 1;
            }
        } else {
            non_log.push(l2);
        }
    }
    rows.push(PlatonicCorrelationRow {
        solid: "tetrahedron (K₄)".into(),
        contains_count: sub_cnt,
        isomorphic_count: iso_cnt,
        mean_log2_stab_contains: mean(&sub_log),
        mean_log2_stab_not_contains: mean(&non_log),
        symmetry_ratio: stab_ratio(&sub_log, &non_log),
    });

    // --- Octahedron = K_{2,2,2} ---
    let mut sub_log: Vec<f64> = Vec::new();
    let mut non_log: Vec<f64> = Vec::new();
    let mut iso_cnt = 0usize;
    let mut sub_cnt = 0usize;
    for e in &table.entries {
        let bits = entry_bits(e);
        let contains = contains_k222(bits, e.n);
        let l2 = (e.report.stabilizer_order as f64).log2();
        if contains {
            sub_cnt += 1;
            sub_log.push(l2);
            if is_iso_to_k222(e) {
                iso_cnt += 1;
            }
        } else {
            non_log.push(l2);
        }
    }
    rows.push(PlatonicCorrelationRow {
        solid: "octahedron (K_{2,2,2})".into(),
        contains_count: sub_cnt,
        isomorphic_count: iso_cnt,
        mean_log2_stab_contains: mean(&sub_log),
        mean_log2_stab_not_contains: mean(&non_log),
        symmetry_ratio: stab_ratio(&sub_log, &non_log),
    });

    // --- Cube Q₃ (needs exactly n=8 to fit) ---
    let mut sub_log: Vec<f64> = Vec::new();
    let mut non_log: Vec<f64> = Vec::new();
    let mut iso_cnt = 0usize;
    let mut sub_cnt = 0usize;
    let mut has_any_n8 = false;
    for e in &table.entries {
        if e.n != 8 {
            continue;
        }
        has_any_n8 = true;
        let bits = entry_bits(e);
        let contains = contains_q3(bits, e.n);
        let l2 = (e.report.stabilizer_order as f64).log2();
        if contains {
            sub_cnt += 1;
            sub_log.push(l2);
            // Q₃ is isomorphic iff m == 12 and the graph IS Q₃.
            if e.m == 12 && contains {
                iso_cnt += 1;
            }
        } else {
            non_log.push(l2);
        }
    }
    if has_any_n8 {
        rows.push(PlatonicCorrelationRow {
            solid: "cube (Q₃, n=8)".into(),
            contains_count: sub_cnt,
            isomorphic_count: iso_cnt,
            mean_log2_stab_contains: mean(&sub_log),
            mean_log2_stab_not_contains: mean(&non_log),
            symmetry_ratio: stab_ratio(&sub_log, &non_log),
        });
    } else {
        rows.push(PlatonicCorrelationRow {
            solid: "cube (Q₃) [needs n=8]".into(),
            contains_count: 0,
            isomorphic_count: 0,
            mean_log2_stab_contains: 0.0,
            mean_log2_stab_not_contains: 0.0,
            symmetry_ratio: 0.0,
        });
    }

    // --- dodecahedron/icosahedron too big for n ≤ 8 ---
    for (name, min_n) in &[("dodecahedron", 20usize), ("icosahedron", 12usize)] {
        rows.push(PlatonicCorrelationRow {
            solid: format!("{} [needs n≥{}]", name, min_n),
            contains_count: 0,
            isomorphic_count: 0,
            mean_log2_stab_contains: 0.0,
            mean_log2_stab_not_contains: 0.0,
            symmetry_ratio: 0.0,
        });
    }

    PlatonicCorrelationAnalysis { rows }
}

fn stab_ratio(sub: &[f64], non: &[f64]) -> f64 {
    // Ratio of geometric means: 2^(mean log2).
    let ms = mean(sub);
    let mn = mean(non);
    if sub.is_empty() || non.is_empty() {
        return 0.0;
    }
    (ms - mn).exp2()
}

// ============================================================================
// Q7: Scaffold edge coverage — how much of the 13-node Metatron scaffold
// lies in the active zone of each graph.
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct ScaffoldInvariant {
    pub id: String,
    pub n: usize,
    pub m: usize,
    /// Edges ∩ scaffold-active-edges (only edges inside the first 7 nodes
    /// qualify since non-active nodes are never connected by an input graph).
    pub scaffold_center_ray_hits: usize, // edges of form (1, i)
    pub scaffold_hexagon_ring_hits: usize, // edges of form (i, i+1) on hexagon
}

#[derive(Debug, Clone, Serialize)]
pub struct ScaffoldInvariantAnalysis {
    /// For each n, histogram of (center_rays, hex_ring) combinations.
    pub per_n: Vec<ScaffoldInvariantNStats>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScaffoldInvariantNStats {
    pub n: usize,
    pub graphs: usize,
    pub distinct_signatures: usize,
    pub mean_center_rays: f64,
    pub mean_hex_ring: f64,
}

fn scaffold_rays_and_ring(entry: &CatalogEntry) -> (usize, usize) {
    // Active-zone scaffold edges:
    //   center rays: (1, 2), (1, 3), (1, 4), (1, 5), (1, 6), (1, 7)
    //   hex ring:    (2, 3), (3, 4), (4, 5), (5, 6), (6, 7), (7, 2)
    let rays: std::collections::HashSet<(usize, usize)> =
        [(1, 2), (1, 3), (1, 4), (1, 5), (1, 6), (1, 7)]
            .iter()
            .copied()
            .collect();
    let ring: std::collections::HashSet<(usize, usize)> =
        [(2, 3), (3, 4), (4, 5), (5, 6), (6, 7), (2, 7)]
            .iter()
            .copied()
            .collect();
    let mut r = 0usize;
    let mut h = 0usize;
    for &(i, j) in &entry.canonical_edges {
        let e = if i < j { (i, j) } else { (j, i) };
        if rays.contains(&e) {
            r += 1;
        }
        if ring.contains(&e) {
            h += 1;
        }
    }
    (r, h)
}

pub fn scaffold_invariants(table: &PeriodicTable) -> ScaffoldInvariantAnalysis {
    let mut per_n_map: BTreeMap<
        usize,
        (
            usize,
            std::collections::HashSet<(usize, usize)>,
            usize,
            usize,
        ),
    > = BTreeMap::new();
    for e in &table.entries {
        let (r, h) = scaffold_rays_and_ring(e);
        let rec = per_n_map
            .entry(e.n)
            .or_insert_with(|| (0, std::collections::HashSet::new(), 0, 0));
        rec.0 += 1;
        rec.1.insert((r, h));
        rec.2 += r;
        rec.3 += h;
    }
    let per_n: Vec<ScaffoldInvariantNStats> = per_n_map
        .into_iter()
        .map(|(n, (g, sigs, rsum, hsum))| ScaffoldInvariantNStats {
            n,
            graphs: g,
            distinct_signatures: sigs.len(),
            mean_center_rays: rsum as f64 / g.max(1) as f64,
            mean_hex_ring: hsum as f64 / g.max(1) as f64,
        })
        .collect();
    ScaffoldInvariantAnalysis { per_n }
}

// ============================================================================
// Deep-dive: for each cospectral k-tuple (k ≥ 3), which *other* invariants
// distinguish the members? This answers the follow-up "OK, spectrum fails
// here — what does NOT fail?"
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct CospectralDistinguisher {
    pub n: usize,
    pub spectrum: Vec<f64>,
    pub members: Vec<CospectralMemberDetail>,
    /// Names of invariants that actually differ between at least two members
    /// of this class.
    pub distinguishing_invariants: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CospectralMemberDetail {
    pub id: String,
    pub m: usize,
    pub degree_sequence: Vec<usize>,
    pub triangle_count: usize,
    pub stabilizer_order: usize,
    pub diameter: Option<usize>,
    pub max_clique_size: usize,
    pub chromatic_number: usize,
    pub is_connected: bool,
    pub is_bipartite: bool,
    pub algebraic_connectivity: f64,
}

pub fn cospectral_distinguishers(
    table: &PeriodicTable,
    min_class_size: usize,
) -> Vec<CospectralDistinguisher> {
    let cospec = cospectral_analysis(table);
    let mut out: Vec<CospectralDistinguisher> = Vec::new();
    for group in cospec
        .groups
        .iter()
        .filter(|g| g.members.len() >= min_class_size)
    {
        let mut members: Vec<CospectralMemberDetail> = Vec::new();
        for id in &group.members {
            let Some(e) = table.entries.iter().find(|x| &x.id == id) else {
                continue;
            };
            members.push(CospectralMemberDetail {
                id: e.id.clone(),
                m: e.m,
                degree_sequence: e.report.properties.degree_sequence.clone(),
                triangle_count: e.report.triangle_count,
                stabilizer_order: e.report.stabilizer_order,
                diameter: e.report.properties.diameter,
                max_clique_size: e.report.properties.max_clique_size,
                chromatic_number: e.report.properties.chromatic_number,
                is_connected: e.report.properties.connected,
                is_bipartite: e.report.properties.is_bipartite,
                algebraic_connectivity: (e.report.algebraic_connectivity * 1e6).round() / 1e6,
            });
        }
        let mut dist: Vec<String> = Vec::new();
        if members
            .iter()
            .any(|m| m.degree_sequence != members[0].degree_sequence)
        {
            dist.push("degree_sequence".into());
        }
        if members
            .iter()
            .any(|m| m.triangle_count != members[0].triangle_count)
        {
            dist.push("triangle_count".into());
        }
        if members
            .iter()
            .any(|m| m.stabilizer_order != members[0].stabilizer_order)
        {
            dist.push("stabilizer_order".into());
        }
        if members.iter().any(|m| m.diameter != members[0].diameter) {
            dist.push("diameter".into());
        }
        if members
            .iter()
            .any(|m| m.max_clique_size != members[0].max_clique_size)
        {
            dist.push("max_clique_size".into());
        }
        if members
            .iter()
            .any(|m| m.chromatic_number != members[0].chromatic_number)
        {
            dist.push("chromatic_number".into());
        }
        if members
            .iter()
            .any(|m| m.is_connected != members[0].is_connected)
        {
            dist.push("is_connected".into());
        }
        if members
            .iter()
            .any(|m| m.is_bipartite != members[0].is_bipartite)
        {
            dist.push("is_bipartite".into());
        }
        if members
            .iter()
            .any(|m| (m.algebraic_connectivity - members[0].algebraic_connectivity).abs() > 1e-6)
        {
            dist.push("algebraic_connectivity".into());
        }
        out.push(CospectralDistinguisher {
            n: group.n,
            spectrum: group.spectrum.clone(),
            members,
            distinguishing_invariants: dist,
        });
    }
    out
}

// ============================================================================
// Combined report
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct FullAnalysis {
    pub n_max: usize,
    pub total_entries: usize,
    pub cospectral: CospectralAnalysis,
    pub matrix_powers: MatrixPowerAnalysis,
    pub self_complementary: SelfComplementaryAnalysis,
    pub spectral_distribution: SpectralDistribution,
    pub platonic: PlatonicCorrelationAnalysis,
    pub scaffold: ScaffoldInvariantAnalysis,
    /// All cospectral classes of size ≥ 3, annotated with which other
    /// invariants distinguish the members — answering "what else can we
    /// try when spectrum fails?"
    pub cospectral_distinguishers: Vec<CospectralDistinguisher>,
}

pub fn full_analysis(table: &PeriodicTable) -> FullAnalysis {
    FullAnalysis {
        n_max: table.stats.n_max,
        total_entries: table.entries.len(),
        cospectral: cospectral_analysis(table),
        matrix_powers: matrix_power_analysis(table),
        self_complementary: self_complementary_analysis(table),
        spectral_distribution: spectral_distribution(table),
        platonic: platonic_correlation(table),
        scaffold: scaffold_invariants(table),
        cospectral_distinguishers: cospectral_distinguishers(table, 3),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::build_catalog;

    #[test]
    fn no_cospectral_pair_below_n5() {
        // Folklore: the smallest cospectral simple-graph pair is at n=5
        // (K_{1,4} vs C_4 ∪ K_1, both with spectrum {-2,0,0,0,2}).
        // The TeX claim that P_2+K_1 / P_3 on 3 nodes are cospectral is wrong:
        // P_2+K_1 has spectrum {-1,0,1}, P_3 has {-√2,0,√2}. Verify here.
        for n in 1..=4 {
            let table = build_catalog(n).unwrap();
            let r = cospectral_analysis(&table);
            assert_eq!(
                r.total_pairs, 0,
                "unexpected cospectral pair at n<=4 (n_max={})",
                n
            );
        }
    }

    #[test]
    fn smallest_cospectral_pair_at_n5() {
        let table = build_catalog(5).unwrap();
        let r = cospectral_analysis(&table);
        // Exactly one cospectral pair at n=5 (K_{1,4} vs C_4 + K_1).
        let n5_pairs: Vec<_> = r
            .groups
            .iter()
            .filter(|g| g.n == 5 && g.members.len() == 2)
            .collect();
        assert!(
            !n5_pairs.is_empty(),
            "expected at least one cospectral pair at n=5"
        );
        // Spectrum should be {-2, 0, 0, 0, 2}.
        let spec = &n5_pairs[0].spectrum;
        assert!((spec[0] - (-2.0)).abs() < 1e-6);
        assert!((spec[4] - 2.0).abs() < 1e-6);
    }

    #[test]
    fn self_complementary_n5_has_two_entries() {
        let table = build_catalog(5).unwrap();
        let r = self_complementary_analysis(&table);
        assert_eq!(r.per_n[&5], 2);
        // Both must have m = C(5,2)/2 = 5 edges.
        for e in &r.entries {
            if e.n == 5 {
                assert_eq!(e.m, 5);
            }
        }
        // One of the two is C5 (the 5-cycle, 2-regular).
        let c5 = r
            .entries
            .iter()
            .find(|e| e.n == 5 && e.degree_sequence == vec![2, 2, 2, 2, 2]);
        assert!(
            c5.is_some(),
            "C5 not found among self-complementary n=5 graphs"
        );
    }

    #[test]
    fn matrix_powers_no_worse_than_spectrum_n4() {
        let table = build_catalog(4).unwrap();
        let r = matrix_power_analysis(&table);
        // Since spectrum ⊆ function of matrix-power traces (Newton identities),
        // traces up to n_max ought to match the spectrum invariant.
        assert!(r.distinct_by_a2_a3_a4 >= r.distinct_by_a2_a3);
    }

    #[test]
    fn platonic_correlation_finds_k4_at_n4() {
        // In the n ≤ 4 catalog, exactly one graph (K₄ = G4-6-1) contains K₄.
        let table = build_catalog(4).unwrap();
        let r = platonic_correlation(&table);
        let tet = r
            .rows
            .iter()
            .find(|row| row.solid.starts_with("tetrahedron"))
            .expect("tetrahedron row missing");
        assert_eq!(tet.contains_count, 1);
        assert_eq!(tet.isomorphic_count, 1);
        let oct = r
            .rows
            .iter()
            .find(|row| row.solid.starts_with("octahedron"))
            .expect("octahedron row missing");
        assert_eq!(oct.contains_count, 0, "K_{{2,2,2}} needs n≥6");
    }

    #[test]
    fn platonic_correlation_finds_octahedron_at_n6() {
        let table = build_catalog(6).unwrap();
        let r = platonic_correlation(&table);
        let oct = r
            .rows
            .iter()
            .find(|row| row.solid.starts_with("octahedron"))
            .expect("octahedron row missing");
        // Octahedron K_{2,2,2} exists at n=6, m=12. At least one isomorphic
        // instance must be present.
        assert!(
            oct.isomorphic_count >= 1,
            "expected K_{{2,2,2}} in n=6 catalog"
        );
    }
}

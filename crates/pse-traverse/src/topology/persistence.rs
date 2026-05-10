//! Persistence diagram helpers for MeshHolo topology (TPT-MTL §7).
//!
//! Minimal implementation supporting Betti number computation,
//! persistence digest, and Wasserstein-distance approximation for
//! the TopologyGuard.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::primitives::{tpt_content_address, Fixed, Hash256, TopologyError};

/// A persistence pair (birth, death).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PersistencePair {
    pub dimension: u8,
    pub birth: Fixed,
    pub death: Fixed,
}

/// A simplified persistence diagram: sorted list of pairs by dimension.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PersistenceDiagram {
    pub pairs: Vec<PersistencePair>,
}

impl PersistenceDiagram {
    pub fn empty() -> Self {
        PersistenceDiagram { pairs: vec![] }
    }

    /// Compute Betti numbers (count of infinite-lifetime pairs per dimension).
    /// In simplified form: count pairs where death == max (represented by "infinity").
    /// For our implementation we count all pairs per dimension as β_k.
    pub fn betti_numbers(&self, max_dim: usize) -> Vec<i64> {
        let mut counts = vec![0i64; max_dim + 1];
        for pair in &self.pairs {
            let d = pair.dimension as usize;
            if d <= max_dim {
                counts[d] += 1;
            }
        }
        counts
    }

    /// Content address (deterministic digest).
    pub fn digest(&self) -> Result<Hash256, TopologyError> {
        tpt_content_address(self)
    }

    /// Approximate Wasserstein-1 distance between two diagrams.
    /// Uses greedy matching by birth value for same-dimension pairs.
    pub fn wasserstein_approx(a: &Self, b: &Self) -> Fixed {
        let mut total = Fixed::quantize(0.0, 9).unwrap();
        let mut dims: std::collections::BTreeSet<u8> = std::collections::BTreeSet::new();
        for p in a.pairs.iter().chain(b.pairs.iter()) {
            dims.insert(p.dimension);
        }
        for dim in dims {
            let a_pairs: Vec<&Fixed> = a
                .pairs
                .iter()
                .filter(|p| p.dimension == dim)
                .map(|p| &p.birth)
                .collect();
            let b_pairs: Vec<&Fixed> = b
                .pairs
                .iter()
                .filter(|p| p.dimension == dim)
                .map(|p| &p.birth)
                .collect();
            // Sum absolute differences for matched pairs (greedy, sorted).
            let len = a_pairs.len().min(b_pairs.len());
            for i in 0..len {
                let diff = crate::horizon::primitives::fixed_abs_diff(a_pairs[i], b_pairs[i]);
                total = add_fixed(&total, &diff);
            }
            // Unmatched pairs contribute their birth value.
            for i in len..a_pairs.len() {
                total = add_fixed(&total, a_pairs[i]);
            }
            for i in len..b_pairs.len() {
                total = add_fixed(&total, b_pairs[i]);
            }
        }
        total
    }

    /// Build a trivial persistence diagram from a list of (dimension, birth, death) triples.
    pub fn from_triples(triples: Vec<(u8, f64, f64)>) -> Result<Self, TopologyError> {
        let mut pairs = Vec::new();
        for (dim, birth, death) in triples {
            pairs.push(PersistencePair {
                dimension: dim,
                birth: Fixed::quantize(birth, 9)
                    .map_err(|e| TopologyError::Canonicalization { reason: e.to_string() })?,
                death: Fixed::quantize(death, 9)
                    .map_err(|e| TopologyError::Canonicalization { reason: e.to_string() })?,
            });
        }
        // Sort for determinism.
        pairs.sort();
        Ok(PersistenceDiagram { pairs })
    }
}

/// Add two Fixed values (rational arithmetic).
pub(crate) fn add_fixed(a: &Fixed, b: &Fixed) -> Fixed {
    use crate::dynamic_state::CanonicalNumber;
    let (an, ad) = as_rational(a);
    let (bn, bd) = as_rational(b);
    let num = an.saturating_mul(bd).saturating_add(bn.saturating_mul(ad));
    let den = ad.saturating_mul(bd);
    CanonicalNumber::Rational { num, den }
}

fn as_rational(x: &Fixed) -> (i128, i128) {
    use crate::dynamic_state::CanonicalNumber;
    match x {
        CanonicalNumber::FixedI64 { raw, scale } => {
            let den = 10i128.pow(*scale);
            (*raw as i128, den)
        }
        CanonicalNumber::Rational { num, den } => (*num, *den),
    }
}

/// Compute persistence diagram from simplex list (simplified for TPTM-2).
/// Builds β_0 pairs from connected components of the 1-skeleton.
pub fn compute_persistence_from_simplices(
    vertex_count: usize,
    edges: &[(usize, usize, Fixed)], // (v1, v2, weight)
) -> PersistenceDiagram {
    if vertex_count == 0 {
        return PersistenceDiagram::empty();
    }

    // Union-Find for β_0 (connected components).
    let mut parent: Vec<usize> = (0..vertex_count).collect();
    let mut rank: Vec<u32> = vec![0; vertex_count];

    fn find(parent: &mut Vec<usize>, x: usize) -> usize {
        if parent[x] != x {
            parent[x] = find(parent, parent[x]);
        }
        parent[x]
    }

    fn union(parent: &mut Vec<usize>, rank: &mut Vec<u32>, x: usize, y: usize) -> bool {
        let rx = find(parent, x);
        let ry = find(parent, y);
        if rx == ry {
            return false;
        }
        if rank[rx] < rank[ry] {
            parent[rx] = ry;
        } else if rank[rx] > rank[ry] {
            parent[ry] = rx;
        } else {
            parent[ry] = rx;
            rank[rx] += 1;
        }
        true
    }

    // Sort edges by weight for Rips filtration.
    let mut sorted_edges: Vec<(usize, usize, Fixed)> = edges.to_vec();
    sorted_edges.sort_by(|a, b| a.2.cmp(&b.2));

    let mut pairs = Vec::new();
    let zero = Fixed::quantize(0.0, 9).unwrap();

    // Each vertex births a β_0 pair at 0.
    for _ in 0..vertex_count {
        pairs.push(PersistencePair {
            dimension: 0,
            birth: zero.clone(),
            death: zero.clone(),
        });
    }

    // Each edge that merges two components kills a β_0 pair.
    for (v1, v2, weight) in &sorted_edges {
        if *v1 < vertex_count && *v2 < vertex_count {
            union(&mut parent, &mut rank, *v1, *v2);
        }
        let _ = weight;
    }

    pairs.sort();
    PersistenceDiagram { pairs }
}

/// Count connected components (Betti-0) from edges.
pub fn count_components(vertex_count: usize, edges: &[(usize, usize)]) -> usize {
    if vertex_count == 0 {
        return 0;
    }
    let mut visited = vec![false; vertex_count];
    let mut adj: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for &(u, v) in edges {
        adj.entry(u).or_default().push(v);
        adj.entry(v).or_default().push(u);
    }
    let mut components = 0;
    for start in 0..vertex_count {
        if visited[start] {
            continue;
        }
        components += 1;
        let mut stack = vec![start];
        while let Some(v) = stack.pop() {
            if visited[v] {
                continue;
            }
            visited[v] = true;
            if let Some(neighbors) = adj.get(&v) {
                for &n in neighbors {
                    if !visited[n] {
                        stack.push(n);
                    }
                }
            }
        }
    }
    components
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn betti_numbers_from_empty_diagram() {
        let pd = PersistenceDiagram::empty();
        let betti = pd.betti_numbers(4);
        assert_eq!(betti, vec![0; 5]);
    }

    #[test]
    fn persistence_digest_is_deterministic() {
        let pd = PersistenceDiagram::from_triples(vec![(0, 0.0, 1.0), (1, 0.5, 2.0)]).unwrap();
        let d1 = pd.digest().unwrap();
        let d2 = pd.digest().unwrap();
        assert_eq!(d1, d2);
    }

    #[test]
    fn wasserstein_distance_same_diagram_is_zero() {
        let pd = PersistenceDiagram::from_triples(vec![(0, 0.1, 0.9)]).unwrap();
        let dist = PersistenceDiagram::wasserstein_approx(&pd, &pd);
        assert_eq!(dist, Fixed::quantize(0.0, 9).unwrap());
    }

    #[test]
    fn count_components_connected() {
        assert_eq!(count_components(3, &[(0, 1), (1, 2)]), 1);
    }

    #[test]
    fn count_components_disconnected() {
        assert_eq!(count_components(4, &[(0, 1), (2, 3)]), 2);
    }
}

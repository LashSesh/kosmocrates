//! Topological / combinatorial graph properties used by the Periodic Table
//! catalog: degree sequence, connectivity, diameter, tree/bipartite tests,
//! chromatic number, self-complementarity.
//!
//! All routines operate on the active n×n submatrix of an embedded graph
//! (i.e. the same convention used by [`crate::spectrum`]).

use serde::Serialize;

use crate::bitgraph::{
    bit_perm_from_sigma, bit_to_edge, complement_bits, mask_for_n, permute_bits, S7BitCache,
};
use crate::group::generate_s7;

/// Aggregated combinatorial/topological properties of a graph on `n` nodes.
#[derive(Debug, Clone, Serialize)]
pub struct GraphProperties {
    pub connected: bool,
    pub degree_sequence: Vec<usize>,
    /// `None` iff the graph is disconnected (diameter is infinite).
    pub diameter: Option<usize>,
    pub is_tree: bool,
    pub is_bipartite: bool,
    pub chromatic_number: usize,
    pub is_self_complementary: bool,
    pub is_regular: bool,
    pub max_clique_size: usize,
    /// Sum of shortest-path distances over all unordered vertex pairs.
    /// `None` iff the graph is disconnected.
    pub wiener_index: Option<usize>,
    /// Radius = min over v of max-distance from v. `None` iff disconnected.
    pub radius: Option<usize>,
    /// Girth = length of the shortest cycle. `None` iff the graph is a forest
    /// (no cycle).
    pub girth: Option<usize>,
    /// Number of connected components.
    pub components: usize,
    /// Threshold graph? Recognized by iteratively removing an isolated or
    /// dominating vertex until the graph is empty. OEIS A005840.
    pub is_threshold: bool,
    /// Planar? Determined by Euler-formula necessary conditions plus
    /// explicit K₅/K_{3,3}-minor enumeration. OEIS A005470.
    pub is_planar: bool,
}

/// Compute the degree of every node in the n×n active zone of `embedded`.
fn degrees(embedded: &[Vec<f64>], n: usize) -> Vec<usize> {
    (0..n)
        .map(|i| (0..n).filter(|&j| embedded[i][j] > 0.5).count())
        .collect()
}

/// BFS from a start node; returns distances to every reachable node
/// (`usize::MAX` for unreachable).
fn bfs(embedded: &[Vec<f64>], n: usize, start: usize) -> Vec<usize> {
    let mut dist = vec![usize::MAX; n];
    dist[start] = 0;
    let mut queue = std::collections::VecDeque::with_capacity(n);
    queue.push_back(start);
    while let Some(u) = queue.pop_front() {
        for v in 0..n {
            if embedded[u][v] > 0.5 && dist[v] == usize::MAX {
                dist[v] = dist[u] + 1;
                queue.push_back(v);
            }
        }
    }
    dist
}

/// True iff every node is reachable from node 0.
fn is_connected(embedded: &[Vec<f64>], n: usize) -> bool {
    if n <= 1 {
        return true;
    }
    bfs(embedded, n, 0).iter().all(|&d| d != usize::MAX)
}

/// Diameter (longest shortest path), or `None` if disconnected.
fn diameter(embedded: &[Vec<f64>], n: usize) -> Option<usize> {
    if n <= 1 {
        return Some(0);
    }
    let mut diam = 0usize;
    for s in 0..n {
        let dist = bfs(embedded, n, s);
        for &d in &dist {
            if d == usize::MAX {
                return None;
            }
            if d > diam {
                diam = d;
            }
        }
    }
    Some(diam)
}

/// True iff the graph is 2-colorable (no odd cycles).
fn is_bipartite(embedded: &[Vec<f64>], n: usize) -> bool {
    if n <= 1 {
        return true;
    }
    let mut color = vec![-1i8; n];
    for s in 0..n {
        if color[s] != -1 {
            continue;
        }
        color[s] = 0;
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(s);
        while let Some(u) = queue.pop_front() {
            for v in 0..n {
                if embedded[u][v] > 0.5 {
                    if color[v] == -1 {
                        color[v] = 1 - color[u];
                        queue.push_back(v);
                    } else if color[v] == color[u] {
                        return false;
                    }
                }
            }
        }
    }
    true
}

/// Edge count of the n×n active zone (upper triangle).
fn edge_count(embedded: &[Vec<f64>], n: usize) -> usize {
    let mut m = 0usize;
    for i in 0..n {
        for j in (i + 1)..n {
            if embedded[i][j] > 0.5 {
                m += 1;
            }
        }
    }
    m
}

/// Greedy backtracking k-colorability check. For n ≤ 7 this is trivially fast.
fn is_k_colorable(embedded: &[Vec<f64>], n: usize, k: usize) -> bool {
    if k == 0 {
        return n == 0;
    }
    let mut colors = vec![0usize; n];
    fn rec(i: usize, n: usize, k: usize, colors: &mut [usize], adj: &[Vec<f64>]) -> bool {
        if i == n {
            return true;
        }
        'outer: for c in 1..=k {
            for j in 0..i {
                if adj[i][j] > 0.5 && colors[j] == c {
                    continue 'outer;
                }
            }
            colors[i] = c;
            if rec(i + 1, n, k, colors, adj) {
                return true;
            }
        }
        colors[i] = 0;
        false
    }
    rec(0, n, k, &mut colors, embedded)
}

/// Smallest k for which the graph is k-colorable. Returns 0 for n = 0.
fn chromatic_number(embedded: &[Vec<f64>], n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    if edge_count(embedded, n) == 0 {
        return if n == 0 { 0 } else { 1 };
    }
    if is_bipartite(embedded, n) {
        return 2;
    }
    for k in 3..=n {
        if is_k_colorable(embedded, n, k) {
            return k;
        }
    }
    n
}

/// Wiener index + radius: both derived from an all-pairs BFS.
/// Returns `(wiener, radius)` as `(None, None)` if disconnected.
fn wiener_and_radius(embedded: &[Vec<f64>], n: usize) -> (Option<usize>, Option<usize>) {
    if n == 0 {
        return (Some(0), Some(0));
    }
    let mut wiener = 0usize;
    let mut radius = usize::MAX;
    for s in 0..n {
        let dist = bfs(embedded, n, s);
        let mut ecc = 0usize;
        for (i, &d) in dist.iter().enumerate() {
            if d == usize::MAX {
                return (None, None);
            }
            if i > s {
                wiener += d;
            }
            if d > ecc {
                ecc = d;
            }
        }
        if ecc < radius {
            radius = ecc;
        }
    }
    (Some(wiener), Some(radius))
}

/// Girth = shortest cycle length. `None` if the graph is a forest.
fn girth(embedded: &[Vec<f64>], n: usize) -> Option<usize> {
    let mut best = usize::MAX;
    for start in 0..n {
        // BFS from `start`, track parent to detect back-edges that form
        // cycles passing through `start`.
        let mut dist = vec![usize::MAX; n];
        let mut parent = vec![usize::MAX; n];
        dist[start] = 0;
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(start);
        while let Some(u) = queue.pop_front() {
            for v in 0..n {
                if embedded[u][v] < 0.5 {
                    continue;
                }
                if dist[v] == usize::MAX {
                    dist[v] = dist[u] + 1;
                    parent[v] = u;
                    queue.push_back(v);
                } else if v != parent[u] {
                    let cyc = dist[u] + dist[v] + 1;
                    if cyc < best {
                        best = cyc;
                    }
                }
            }
        }
    }
    if best == usize::MAX {
        None
    } else {
        Some(best)
    }
}

/// Count connected components via BFS.
fn component_count(embedded: &[Vec<f64>], n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    let mut visited = vec![false; n];
    let mut count = 0usize;
    for s in 0..n {
        if visited[s] {
            continue;
        }
        count += 1;
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(s);
        visited[s] = true;
        while let Some(u) = queue.pop_front() {
            for v in 0..n {
                if embedded[u][v] > 0.5 && !visited[v] {
                    visited[v] = true;
                    queue.push_back(v);
                }
            }
        }
    }
    count
}

/// Threshold graph recognition.
/// A graph G is *threshold* iff it can be built from the empty graph by
/// iteratively adding a single vertex that is either
///   (a) isolated (connected to no existing vertex), or
///   (b) dominating (connected to every existing vertex).
/// Equivalently: the process runs in reverse — at every step there must
/// exist an isolated or universal vertex.
fn is_threshold(embedded: &[Vec<f64>], n: usize) -> bool {
    if n <= 1 {
        return true;
    }
    // Track "alive" vertices as a bitmask.
    let mut alive: u32 = (1u32 << n) - 1;
    loop {
        let remaining: u32 = alive.count_ones();
        if remaining <= 1 {
            return true;
        }
        let mut removed = false;
        for v in 0..n {
            if (alive >> v) & 1 == 0 {
                continue;
            }
            // Count edges to other alive vertices.
            let mut deg = 0usize;
            for u in 0..n {
                if u == v {
                    continue;
                }
                if (alive >> u) & 1 == 1 && embedded[v][u] > 0.5 {
                    deg += 1;
                }
            }
            // isolated (deg=0) or dominating (deg = remaining - 1)
            if deg == 0 || deg as u32 == remaining - 1 {
                alive &= !(1u32 << v);
                removed = true;
                break;
            }
        }
        if !removed {
            return false;
        }
    }
}

// Planarity detection by direct K₅/K_{3,3}-minor enumeration.
//
// G is planar ⟺ G has neither K₅ nor K_{3,3} as a minor (Wagner, 1937).
// A minor of G on k vertices corresponds to a partition of some subset
// S ⊆ V(G) into k non-empty *connected* "branch sets", with a cross-edge
// between every pair of branch sets for which the minor has an edge.
//
// Strategy:
//   1. Early outs from Euler's formula: m > 3n−6 ⇒ non-planar;
//      bipartite and m > 2n−4 ⇒ non-planar; n ≤ 4 ⇒ planar.
//   2. Exhaustive enumeration of branch-set partitions into 5 (for K₅) or
//      into 3+3 (for K_{3,3}). For n ≤ 8 this fits easily in memory.

fn is_planar_core(embedded: &[Vec<f64>], n: usize) -> bool {
    if n <= 4 {
        return true;
    }
    // Euler-formula necessary conditions.
    let m = edge_count(embedded, n);
    if m > 3 * n - 6 {
        return false;
    }
    if is_bipartite(embedded, n) && m > 2 * n - 4 {
        return false;
    }

    // No K₅ minor?
    if has_minor_complete_5(embedded, n) {
        return false;
    }
    // No K_{3,3} minor?
    if has_minor_k33(embedded, n) {
        return false;
    }
    true
}

/// Is block (bitmask over [0..n)) connected in the induced subgraph?
fn is_block_connected(embedded: &[Vec<f64>], n: usize, block: u32) -> bool {
    if block == 0 {
        return false;
    }
    let start = block.trailing_zeros() as usize;
    let mut visited = 1u32 << start;
    let mut stack = vec![start];
    while let Some(u) = stack.pop() {
        for v in 0..n {
            if (block >> v) & 1 == 1 && (visited >> v) & 1 == 0 && embedded[u][v] > 0.5 {
                visited |= 1u32 << v;
                stack.push(v);
            }
        }
    }
    visited == block
}

/// Is there at least one edge between blocks `a` and `b`?
fn has_cross_edge(embedded: &[Vec<f64>], n: usize, a: u32, b: u32) -> bool {
    for i in 0..n {
        if (a >> i) & 1 == 0 {
            continue;
        }
        for j in 0..n {
            if (b >> j) & 1 == 0 {
                continue;
            }
            if embedded[i][j] > 0.5 {
                return true;
            }
        }
    }
    false
}

/// Enumerate all ways to partition the vertices of `embedded[0..n]` into `k`
/// connected branch sets (possibly leaving some vertices unassigned).
/// Calls `cb` with the partition; stops early if `cb` returns `true`.
/// Returns `true` iff `cb` ever returned `true`.
fn for_each_branch_partition<F: FnMut(&[u32]) -> bool>(
    embedded: &[Vec<f64>],
    n: usize,
    k: usize,
    cb: &mut F,
) -> bool {
    let mut blocks = vec![0u32; k + 1]; // index 0 = "unassigned"
    fn rec<F: FnMut(&[u32]) -> bool>(
        v: usize,
        n: usize,
        k: usize,
        max_used: usize,
        blocks: &mut [u32],
        embedded: &[Vec<f64>],
        cb: &mut F,
    ) -> bool {
        if v == n {
            if max_used < k {
                return false;
            }
            for i in 1..=k {
                if !is_block_connected(embedded, n, blocks[i]) {
                    return false;
                }
            }
            return cb(&blocks[1..=k]);
        }
        // Symmetry reduction: the next vertex may join any existing block
        // up to `max_used`, or start a new one (at index max_used + 1,
        // if available).
        let upper = (max_used + 1).min(k);
        for b in 1..=upper {
            blocks[b] |= 1u32 << v;
            let new_max = max_used.max(b);
            if rec(v + 1, n, k, new_max, blocks, embedded, cb) {
                return true;
            }
            blocks[b] &= !(1u32 << v);
        }
        // Option: leave vertex v unassigned.
        blocks[0] |= 1u32 << v;
        if rec(v + 1, n, k, max_used, blocks, embedded, cb) {
            return true;
        }
        blocks[0] &= !(1u32 << v);
        false
    }
    rec(0, n, k, 0, &mut blocks, embedded, cb)
}

fn has_minor_complete_5(embedded: &[Vec<f64>], n: usize) -> bool {
    if n < 5 {
        return false;
    }
    let mut found = false;
    for_each_branch_partition(embedded, n, 5, &mut |blocks| {
        // K₅ needs every pair of blocks to have a cross-edge.
        for i in 0..5 {
            for j in (i + 1)..5 {
                if !has_cross_edge(embedded, n, blocks[i], blocks[j]) {
                    return false;
                }
            }
        }
        found = true;
        true
    });
    found
}

fn has_minor_k33(embedded: &[Vec<f64>], n: usize) -> bool {
    if n < 6 {
        return false;
    }
    let mut found = false;
    for_each_branch_partition(embedded, n, 6, &mut |blocks| {
        // K_{3,3}: split 6 blocks into a 3+3 bipartition. To kill symmetry,
        // fix block index 0 in side A and iterate over subsets of
        // {1..5} of size 2 for the rest of A.
        for mask in 0u32..(1 << 5) {
            if mask.count_ones() != 2 {
                continue;
            }
            let mut side_a = vec![0usize; 3];
            let mut side_b = vec![0usize; 3];
            side_a[0] = 0;
            let mut ai = 1;
            let mut bi = 0;
            for i in 1..6 {
                if (mask >> (i - 1)) & 1 == 1 {
                    side_a[ai] = i;
                    ai += 1;
                } else {
                    side_b[bi] = i;
                    bi += 1;
                }
            }
            let mut ok = true;
            'outer: for &a in &side_a {
                for &b in &side_b {
                    if !has_cross_edge(embedded, n, blocks[a], blocks[b]) {
                        ok = false;
                        break 'outer;
                    }
                }
            }
            if ok {
                found = true;
                return true;
            }
        }
        false
    });
    found
}

/// Size of the largest clique, by exhaustive subset enumeration over the
/// active zone (n ≤ 7 → at most 128 subsets).
fn max_clique_size(embedded: &[Vec<f64>], n: usize) -> usize {
    let mut best = 0usize;
    for mask in 0u32..(1u32 << n) {
        let size = mask.count_ones() as usize;
        if size <= best {
            continue;
        }
        let mut clique = true;
        'check: for i in 0..n {
            if (mask >> i) & 1 == 0 {
                continue;
            }
            for j in (i + 1)..n {
                if (mask >> j) & 1 == 0 {
                    continue;
                }
                if embedded[i][j] < 0.5 {
                    clique = false;
                    break 'check;
                }
            }
        }
        if clique {
            best = size;
        }
    }
    best
}

/// Compute all combinatorial properties for the n×n active zone of an
/// embedded graph, assuming self-complementarity has already been decided
/// by the caller (which typically knows the canonical form under its
/// preferred S_n action).
///
/// Use [`compute_properties`] for a self-contained variant that runs S_7
/// internally.
pub fn compute_properties_with(
    embedded: &[Vec<f64>],
    n: usize,
    is_self_complementary: bool,
) -> GraphProperties {
    let degree_sequence = {
        let mut d = degrees(embedded, n);
        d.sort_unstable();
        d
    };
    let connected = is_connected(embedded, n);
    let diameter_opt = diameter(embedded, n);
    let m = edge_count(embedded, n);
    let is_tree = connected && m + 1 == n && n >= 1;
    let bipartite = is_bipartite(embedded, n);
    let chromatic = chromatic_number(embedded, n);

    let is_regular = degree_sequence
        .first()
        .map(|&d0| degree_sequence.iter().all(|&d| d == d0))
        .unwrap_or(true);

    let max_clique = max_clique_size(embedded, n);
    let (wiener, radius) = wiener_and_radius(embedded, n);
    let g = girth(embedded, n);
    let components = component_count(embedded, n);
    let threshold = is_threshold(embedded, n);
    let planar = is_planar_core(embedded, n);

    GraphProperties {
        connected,
        degree_sequence,
        diameter: diameter_opt,
        is_tree,
        is_bipartite: bipartite,
        chromatic_number: chromatic,
        is_self_complementary,
        is_regular,
        max_clique_size: max_clique,
        wiener_index: wiener,
        radius,
        girth: g,
        components,
        is_threshold: threshold,
        is_planar: planar,
    }
}

/// Compute all combinatorial properties for the n×n active zone of an
/// embedded graph.
///
/// Self-complementarity is decided internally via S_7 canonical comparison.
/// For n = 8 use [`compute_properties_with`] directly and pass the
/// self-complementarity flag from [`crate::bitgraph_n::fast_orbit_n`].
pub fn compute_properties(
    embedded: &[Vec<f64>],
    n: usize,
    cache: &S7BitCache,
) -> GraphProperties {
    let bits = adjacency_to_bits(embedded, n);
    let comp_bits = complement_bits(bits, n);
    let canon_g = canonical_using_cache(bits, cache);
    let canon_comp = canonical_using_cache(comp_bits, cache);
    let is_self_complementary = n >= 1 && canon_g == canon_comp;
    compute_properties_with(embedded, n, is_self_complementary)
}

/// Pack the n×n active zone of an adjacency matrix into the 21-bit
/// upper-triangular `u32` representation.
pub fn adjacency_to_bits(embedded: &[Vec<f64>], n: usize) -> u32 {
    let mut bits = 0u32;
    for j in 1..n.min(7) {
        for i in 0..j {
            if embedded[i][j] > 0.5 {
                let b = j * (j - 1) / 2 + i;
                bits |= 1u32 << b;
            }
        }
    }
    bits
}

fn canonical_using_cache(bits: u32, cache: &S7BitCache) -> u32 {
    let mut canonical = u32::MAX;
    for table in &cache.tables {
        let img = permute_bits(bits, table);
        if img < canonical {
            canonical = img;
        }
    }
    canonical
}

/// Sanity helper that mirrors [`canonical_using_cache`] without a cache —
/// used internally and for ad-hoc tests. Kept available because callers may
/// want to verify behavior without instantiating an `S7BitCache`.
pub fn canonical_form_uncached(bits: u32) -> u32 {
    let mut canonical = u32::MAX;
    for sigma in generate_s7() {
        let table = bit_perm_from_sigma(&sigma);
        let img = permute_bits(bits, &table);
        if img < canonical {
            canonical = img;
        }
    }
    canonical
}

// Silence dead_code in case `bit_to_edge` import is only used from tests.
#[allow(dead_code)]
fn _use_bit_to_edge(b: u32) -> (usize, usize) {
    bit_to_edge(b)
}

#[allow(dead_code)]
fn _use_mask_for_n(n: usize) -> u32 {
    mask_for_n(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::embed;
    use crate::ingest::InputGraph;

    #[test]
    fn empty_graph_props() {
        let g = InputGraph::from_edges(3, &[]).unwrap();
        let e = embed(&g);
        let cache = S7BitCache::new();
        let p = compute_properties(&e, 3, &cache);
        assert_eq!(p.degree_sequence, vec![0, 0, 0]);
        assert!(!p.connected);
        assert_eq!(p.diameter, None);
        assert!(!p.is_tree);
        assert!(p.is_bipartite);
        assert_eq!(p.chromatic_number, 1);
        assert!(p.is_regular);
        assert_eq!(p.max_clique_size, 1);
    }

    #[test]
    fn k3_props() {
        let g = InputGraph::from_edges(3, &[(1, 2), (2, 3), (1, 3)]).unwrap();
        let e = embed(&g);
        let cache = S7BitCache::new();
        let p = compute_properties(&e, 3, &cache);
        assert_eq!(p.degree_sequence, vec![2, 2, 2]);
        assert!(p.connected);
        assert_eq!(p.diameter, Some(1));
        assert!(!p.is_tree);
        assert!(!p.is_bipartite);
        assert_eq!(p.chromatic_number, 3);
        assert!(p.is_regular);
        assert_eq!(p.max_clique_size, 3);
    }

    #[test]
    fn p4_is_self_complementary() {
        let g = InputGraph::from_edges(4, &[(1, 2), (2, 3), (3, 4)]).unwrap();
        let e = embed(&g);
        let cache = S7BitCache::new();
        let p = compute_properties(&e, 4, &cache);
        assert!(p.is_self_complementary);
        assert!(p.is_tree);
        assert!(p.is_bipartite);
        assert_eq!(p.chromatic_number, 2);
        assert_eq!(p.diameter, Some(3));
    }

    #[test]
    fn c5_is_self_complementary() {
        let g = InputGraph::from_edges(5, &[(1, 2), (2, 3), (3, 4), (4, 5), (5, 1)]).unwrap();
        let e = embed(&g);
        let cache = S7BitCache::new();
        let p = compute_properties(&e, 5, &cache);
        assert!(p.is_self_complementary);
        assert!(!p.is_tree);
        assert_eq!(p.chromatic_number, 3);
        assert_eq!(p.diameter, Some(2));
    }

    #[test]
    fn star_is_tree_and_bipartite() {
        let g = InputGraph::from_edges(7, &[(1, 2), (1, 3), (1, 4), (1, 5), (1, 6), (1, 7)])
            .unwrap();
        let e = embed(&g);
        let cache = S7BitCache::new();
        let p = compute_properties(&e, 7, &cache);
        assert!(p.is_tree);
        assert!(p.is_bipartite);
        assert_eq!(p.chromatic_number, 2);
        assert_eq!(p.diameter, Some(2));
        // Star is a threshold graph (build by adding one dominating vertex
        // then isolated vertices).
        assert!(p.is_threshold);
        // Star is planar.
        assert!(p.is_planar);
    }

    #[test]
    fn k5_is_not_planar() {
        let edges: Vec<(usize, usize)> = (1..=5)
            .flat_map(|i| ((i + 1)..=5).map(move |j| (i, j)))
            .collect();
        let g = InputGraph::from_edges(5, &edges).unwrap();
        let e = embed(&g);
        let cache = S7BitCache::new();
        let p = compute_properties(&e, 5, &cache);
        assert!(!p.is_planar);
    }

    #[test]
    fn k33_is_not_planar() {
        let edges: Vec<(usize, usize)> = (1..=3)
            .flat_map(|i| (4..=6).map(move |j| (i, j)))
            .collect();
        let g = InputGraph::from_edges(6, &edges).unwrap();
        let e = embed(&g);
        let cache = S7BitCache::new();
        let p = compute_properties(&e, 6, &cache);
        assert!(!p.is_planar);
        assert!(p.is_bipartite);
    }

    #[test]
    fn cycle_c6_is_planar_not_threshold() {
        let g = InputGraph::from_edges(6, &[(1, 2), (2, 3), (3, 4), (4, 5), (5, 6), (6, 1)])
            .unwrap();
        let e = embed(&g);
        let cache = S7BitCache::new();
        let p = compute_properties(&e, 6, &cache);
        assert!(p.is_planar);
        assert!(!p.is_threshold); // C_n for n ≥ 4 is not threshold
    }

    #[test]
    fn k4_is_planar_and_threshold() {
        let g = InputGraph::from_edges(
            4,
            &[(1, 2), (1, 3), (1, 4), (2, 3), (2, 4), (3, 4)],
        )
        .unwrap();
        let e = embed(&g);
        let cache = S7BitCache::new();
        let p = compute_properties(&e, 4, &cache);
        assert!(p.is_planar);
        assert!(p.is_threshold);
    }
}

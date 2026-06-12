//! Quarantined spectral mathematics over `Q16`-weighted coupling graphs.
//!
//! Assimilated from Wonderlamp's `isls-hypercube` (`laplacian` /
//! `fiedler_bisect` / `kuramoto_groups` / `singularities`) — the pure,
//! well-tested spectral kit — with its AppSpec parser, domain registries and
//! rigid dimension categories left behind. The graph here is *abstract*:
//! nodes are whatever the caller couples (Kosmocrates couples wish
//! proposals), and nothing in this crate knows about dimensions, stacks or
//! domains.
//!
//! ## The float quarantine
//!
//! This is the one crate in the kosmo layer where floating-point arithmetic
//! is genuinely load-bearing (power iteration, oscillator integration). The
//! discipline:
//!
//! - **The public API is `Q16` / discrete only** — weights in, index
//!   partitions and `Q16` masses out. Pinned by a source-scan test.
//! - Floats never cross the crate boundary and never enter anything
//!   content-addressed; every spectral output is advisory structure
//!   (CROSS-010 — it ranks and groups, it gates nothing).
//! - Determinism: fixed iteration counts, fixed seeded initial vectors, and
//!   no `libm` transcendentals — the one needed `sin` is an in-crate
//!   range-reduced Taylor polynomial, so results are reproducible across
//!   IEEE-754 platforms (`+ - * /` and `sqrt` are correctly rounded by the
//!   standard).
//!
//! ## What it computes
//!
//! - [`fiedler_bisect`]: split a graph in two along its sparsest seam
//!   (sign structure of the Fiedler vector, found by power iteration with
//!   deflation of the constant eigenvector).
//! - [`spectral_clusters`]: recursive bisection down to natural clusters —
//!   a split is accepted only while its conductance stays ≤ ½, so tight
//!   cliques refuse to shatter and the cluster count *emerges* instead of
//!   being imposed.
//! - [`kuramoto_groups`]: identical-oscillator Kuramoto sync at a fixed
//!   horizon — strongly coupled groups have synchronized by then, weak seams
//!   have not. A second, dynamics-flavoured lens on the same structure.
//! - [`singularities`]: articulation nodes (integer Tarjan) ranked by `Q16`
//!   coupling mass — the nodes whose removal disconnects the graph: the most
//!   consequential decisions first.

use kosmo_core::Q16;
use std::collections::BTreeMap;

// ─── The graph (public, Q16/discrete) ───────────────────────────────────────

/// An undirected graph with `Q16` coupling weights on the edges.
///
/// Simple by construction: no self-loops, no parallel edges, weights must be
/// positive. Nodes are `0..node_count()`; attaching meaning to an index is
/// the caller's job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CouplingGraph {
    nodes: usize,
    /// Keyed `(a, b)` with `a < b`.
    edges: BTreeMap<(usize, usize), Q16>,
}

impl CouplingGraph {
    pub fn new(nodes: usize) -> Self {
        Self {
            nodes,
            edges: BTreeMap::new(),
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Add an undirected edge. Returns `false` (and adds nothing) for
    /// self-loops, out-of-range endpoints, non-positive weights, or a pair
    /// that already has an edge — fail-closed explicitness over silent
    /// merging.
    pub fn add_edge(&mut self, a: usize, b: usize, weight: Q16) -> bool {
        if a == b || a >= self.nodes || b >= self.nodes || !weight.is_positive() {
            return false;
        }
        let key = (a.min(b), a.max(b));
        if self.edges.contains_key(&key) {
            return false;
        }
        self.edges.insert(key, weight);
        true
    }

    /// The weight between `a` and `b`, if an edge exists.
    pub fn weight(&self, a: usize, b: usize) -> Option<Q16> {
        self.edges.get(&(a.min(b), a.max(b))).copied()
    }

    /// All edges as `(a, b, weight)` with `a < b`, in deterministic order.
    pub fn edges(&self) -> impl Iterator<Item = (usize, usize, Q16)> + '_ {
        self.edges.iter().map(|(&(a, b), &w)| (a, b, w))
    }

    /// Total `Q16` weight incident to `node` (saturating).
    pub fn coupling_mass(&self, node: usize) -> Q16 {
        let mut mass = Q16::ZERO;
        for (&(a, b), &w) in &self.edges {
            if a == node || b == node {
                mass = mass.saturating_add(w);
            }
        }
        mass
    }

    /// Adjacency lists (sorted neighbours).
    fn adjacency(&self) -> Vec<Vec<usize>> {
        let mut adj = vec![Vec::new(); self.nodes];
        for &(a, b) in self.edges.keys() {
            adj[a].push(b);
            adj[b].push(a);
        }
        adj
    }

    /// Dense symmetric weight matrix in `f64` — the quarantine entry point.
    fn weights_f64(&self) -> Vec<Vec<f64>> {
        let mut w = vec![vec![0.0; self.nodes]; self.nodes];
        for (&(a, b), &q) in &self.edges {
            let v = q.raw() as f64 / 65536.0;
            w[a][b] = v;
            w[b][a] = v;
        }
        w
    }
}

// ─── Internal float kernel (private — the quarantine) ───────────────────────

const POWER_ITERATIONS: usize = 256;
const KURAMOTO_STEPS: usize = 200;
const KURAMOTO_DT: f64 = 0.1;
/// Phases closer than this (radians) at the horizon belong to one group.
const PHASE_TOLERANCE: f64 = 0.25;
/// A bisection is accepted only while `cut / min(vol)` stays at or below
/// this — tight clusters refuse to shatter.
const CONDUCTANCE_MAX: f64 = 0.5;
const TWO_PI: f64 = 2.0 * std::f64::consts::PI;

/// Deterministic seed value in `[-0.5, 0.5)` for index `i` (Knuth
/// multiplicative hash — no RNG state, no platform dependence).
fn seed(i: usize) -> f64 {
    ((i as u64).wrapping_add(1).wrapping_mul(2_654_435_761) % 8191) as f64 / 8191.0 - 0.5
}

/// Range-reduced Taylor sine: deterministic across platforms (no libm).
/// Max error ~1e-4 at |x| = π — far below every tolerance used here.
fn psin(x: f64) -> f64 {
    let y = x - TWO_PI * (x / TWO_PI + 0.5).floor();
    let y2 = y * y;
    y * (1.0
        + y2 * (-1.0 / 6.0
            + y2 * (1.0 / 120.0
                + y2 * (-1.0 / 5040.0 + y2 * (1.0 / 362_880.0 - y2 / 39_916_800.0)))))
}

fn subtract_mean(v: &mut [f64]) {
    let mean = v.iter().sum::<f64>() / v.len() as f64;
    for x in v.iter_mut() {
        *x -= mean;
    }
}

fn normalize(v: &mut [f64]) -> bool {
    let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm < 1e-12 {
        return false;
    }
    for x in v.iter_mut() {
        *x /= norm;
    }
    true
}

/// The Fiedler vector (second-smallest Laplacian eigenvector) by power
/// iteration on the spectrally shifted matrix `M = cI − L`, with the constant
/// eigenvector deflated by re-centering each step. Fixed iterations, fixed
/// seeded start — bit-deterministic. `None` when `n < 2` or the iteration
/// degenerates (the caller falls back to an index split).
fn fiedler_vector(g: &CouplingGraph) -> Option<Vec<f64>> {
    let n = g.node_count();
    if n < 2 {
        return None;
    }
    let w = g.weights_f64();
    let deg: Vec<f64> = (0..n).map(|i| w[i].iter().sum()).collect();
    let c = 2.0 * deg.iter().cloned().fold(0.0, f64::max) + 1.0;

    let mut v: Vec<f64> = (0..n).map(seed).collect();
    subtract_mean(&mut v);
    if !normalize(&mut v) {
        return None;
    }
    let mut u = vec![0.0; n];
    for _ in 0..POWER_ITERATIONS {
        for i in 0..n {
            // (Mv)[i] = (c − deg[i])·v[i] + Σ_j w[i][j]·v[j]
            let mut acc = (c - deg[i]) * v[i];
            for j in 0..n {
                acc += w[i][j] * v[j];
            }
            u[i] = acc;
        }
        subtract_mean(&mut u);
        if !normalize(&mut u) {
            return None;
        }
        v.copy_from_slice(&u);
    }
    Some(v)
}

/// Conductance of a cut inside the (sub)graph spanned by `a ∪ b`:
/// `cut / min(vol(a), vol(b))` with volumes counting all incident weight.
/// A zero cut is a perfect split (0.0).
fn conductance(w: &[Vec<f64>], a: &[usize], b: &[usize]) -> f64 {
    let mut cut = 0.0;
    for &i in a {
        for &j in b {
            cut += w[i][j];
        }
    }
    if cut == 0.0 {
        return 0.0;
    }
    let vol = |side: &[usize]| -> f64 {
        let mut v = 0.0;
        for &i in side {
            for &j in a.iter().chain(b.iter()) {
                v += w[i][j];
            }
        }
        v
    };
    cut / vol(a).min(vol(b))
}

// ─── Public spectral operations ──────────────────────────────────────────────

/// Split the graph along its sparsest seam: nodes with a non-negative
/// Fiedler component on the left, the rest on the right. Degenerate sign
/// splits (or a degenerate iteration) fall back to a deterministic median /
/// index split, so for `n ≥ 2` both sides are always non-empty.
pub fn fiedler_bisect(g: &CouplingGraph) -> (Vec<usize>, Vec<usize>) {
    let n = g.node_count();
    if n == 0 {
        return (vec![], vec![]);
    }
    if n == 1 {
        return (vec![0], vec![]);
    }
    let split_by_order = |order: Vec<usize>| -> (Vec<usize>, Vec<usize>) {
        let mid = order.len().div_ceil(2);
        let mut left = order[..mid].to_vec();
        let mut right = order[mid..].to_vec();
        left.sort_unstable();
        right.sort_unstable();
        (left, right)
    };
    let Some(v) = fiedler_vector(g) else {
        return split_by_order((0..n).collect());
    };
    let mut left: Vec<usize> = (0..n).filter(|&i| v[i] >= 0.0).collect();
    let mut right: Vec<usize> = (0..n).filter(|&i| v[i] < 0.0).collect();
    if left.is_empty() || right.is_empty() {
        // All one sign: order by component value (ties by index) and halve.
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| v[a].partial_cmp(&v[b]).unwrap_or(std::cmp::Ordering::Equal));
        return split_by_order(order);
    }
    left.sort_unstable();
    right.sort_unstable();
    (left, right)
}

/// Connected components, each sorted, ordered by smallest member.
pub fn connected_components(g: &CouplingGraph) -> Vec<Vec<usize>> {
    let n = g.node_count();
    let adj = g.adjacency();
    let mut seen = vec![false; n];
    let mut components = Vec::new();
    for start in 0..n {
        if seen[start] {
            continue;
        }
        let mut component = vec![];
        let mut queue = std::collections::VecDeque::from([start]);
        seen[start] = true;
        while let Some(i) = queue.pop_front() {
            component.push(i);
            for &j in &adj[i] {
                if !seen[j] {
                    seen[j] = true;
                    queue.push_back(j);
                }
            }
        }
        component.sort_unstable();
        components.push(component);
    }
    components
}

/// Natural clusters by recursive Fiedler bisection.
///
/// Starts from connected components, then repeatedly bisects the largest
/// splittable cluster — but a split is kept only while its conductance stays
/// ≤ ½, so tight cliques stay whole and the final count *emerges* (bounded
/// above by `max_clusters`). Deterministic; clusters sorted by first member.
pub fn spectral_clusters(g: &CouplingGraph, max_clusters: usize) -> Vec<Vec<usize>> {
    let n = g.node_count();
    if n == 0 {
        return vec![];
    }
    let max_clusters = max_clusters.max(1);
    let w = g.weights_f64();
    let mut clusters = connected_components(g);
    let mut frozen: Vec<bool> = clusters.iter().map(|c| c.len() < 2).collect();

    while clusters.len() < max_clusters {
        // Largest unfrozen cluster; ties resolved by smallest first member.
        let Some(target) = (0..clusters.len())
            .filter(|&i| !frozen[i])
            .max_by(|&a, &b| {
                clusters[a]
                    .len()
                    .cmp(&clusters[b].len())
                    .then(clusters[b][0].cmp(&clusters[a][0]))
            })
        else {
            break;
        };
        let members = clusters[target].clone();

        // Induced subgraph (local indices follow `members` order).
        let mut sub = CouplingGraph::new(members.len());
        for (li, &gi) in members.iter().enumerate() {
            for (lj, &gj) in members.iter().enumerate().skip(li + 1) {
                if let Some(weight) = g.weight(gi, gj) {
                    sub.add_edge(li, lj, weight);
                }
            }
        }
        if sub.edge_count() == 0 {
            frozen[target] = true;
            continue;
        }
        let (left_local, right_local) = fiedler_bisect(&sub);
        if left_local.is_empty() || right_local.is_empty() {
            frozen[target] = true;
            continue;
        }
        let left: Vec<usize> = left_local.iter().map(|&i| members[i]).collect();
        let right: Vec<usize> = right_local.iter().map(|&i| members[i]).collect();
        if conductance(&w, &left, &right) > CONDUCTANCE_MAX {
            frozen[target] = true; // a tight cluster refuses to shatter
            continue;
        }
        clusters[target] = left;
        frozen[target] = clusters[target].len() < 2;
        frozen.push(right.len() < 2);
        clusters.push(right);
    }

    clusters.sort_by_key(|c| c[0]);
    clusters
}

/// Kuramoto synchronization groups at a fixed horizon.
///
/// Every node is an identical oscillator (natural frequency 0) with a
/// deterministic, symmetry-broken initial phase; coupling follows the edge
/// weights (normalized by the maximum degree). After a fixed number of
/// integration steps, nodes whose phases lie within the tolerance — chained
/// circularly — form one group. Strongly coupled regions have synchronized
/// by the horizon; weak seams have not. Edgeless nodes stay solo.
pub fn kuramoto_groups(g: &CouplingGraph) -> Vec<Vec<usize>> {
    let n = g.node_count();
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        return vec![vec![0]];
    }
    let w = g.weights_f64();
    let deg: Vec<f64> = (0..n).map(|i| w[i].iter().sum()).collect();
    let max_deg = deg.iter().cloned().fold(0.0, f64::max);
    if max_deg == 0.0 {
        return (0..n).map(|i| vec![i]).collect();
    }
    let k = 1.0 / max_deg;

    // Splayed start with a deterministic jitter (a perfectly symmetric splay
    // is an unstable equilibrium — the jitter lets the dynamics decide).
    let mut theta: Vec<f64> = (0..n)
        .map(|i| TWO_PI * i as f64 / n as f64 + 0.02 * seed(i))
        .collect();
    let mut delta = vec![0.0; n];
    for _ in 0..KURAMOTO_STEPS {
        for i in 0..n {
            let mut acc = 0.0;
            for j in 0..n {
                if w[i][j] > 0.0 {
                    acc += w[i][j] * psin(theta[j] - theta[i]);
                }
            }
            delta[i] = k * acc;
        }
        for i in 0..n {
            theta[i] += KURAMOTO_DT * delta[i];
        }
    }

    // Circular single-link grouping of the final phases.
    let mut order: Vec<usize> = (0..n).collect();
    let phase = |i: usize| theta[i].rem_euclid(TWO_PI);
    order.sort_by(|&a, &b| {
        phase(a)
            .partial_cmp(&phase(b))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cmp(&b))
    });
    let mut groups: Vec<Vec<usize>> = vec![vec![order[0]]];
    for pair in order.windows(2) {
        if phase(pair[1]) - phase(pair[0]) <= PHASE_TOLERANCE {
            groups.last_mut().expect("non-empty").push(pair[1]);
        } else {
            groups.push(vec![pair[1]]);
        }
    }
    // Wrap-around: the first and last groups may meet across 2π.
    if groups.len() > 1 {
        let first = order[0];
        let last = *order.last().expect("non-empty");
        if phase(first) + TWO_PI - phase(last) <= PHASE_TOLERANCE {
            let head = groups.remove(0);
            groups.last_mut().expect("non-empty").extend(head);
        }
    }
    for group in &mut groups {
        group.sort_unstable();
    }
    groups.sort_by_key(|grp| grp[0]);
    groups
}

/// A node whose removal disconnects its component, ranked by how much
/// coupling runs through it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Singularity {
    pub node: usize,
    /// Total `Q16` weight incident to the node (saturating sum) — the mass
    /// of coupling that hinges on this single decision.
    pub coupling_mass: Q16,
}

/// Articulation nodes (iterative Tarjan, pure integer arithmetic), sorted by
/// coupling mass descending, then node index. These are the graph's
/// singularities: remove one and the structure falls apart — the most
/// consequential nodes first.
pub fn singularities(g: &CouplingGraph) -> Vec<Singularity> {
    let n = g.node_count();
    let adj = g.adjacency();
    let mut disc = vec![0usize; n];
    let mut low = vec![0usize; n];
    let mut visited = vec![false; n];
    let mut is_articulation = vec![false; n];
    let mut timer = 1usize;

    for start in 0..n {
        if visited[start] {
            continue;
        }
        visited[start] = true;
        disc[start] = timer;
        low[start] = timer;
        timer += 1;
        let mut root_children = 0usize;
        // (node, parent, next neighbour index)
        let mut stack: Vec<(usize, usize, usize)> = vec![(start, usize::MAX, 0)];
        while let Some(top) = stack.len().checked_sub(1) {
            let (v, parent, idx) = stack[top];
            if idx < adj[v].len() {
                let u = adj[v][idx];
                stack[top].2 += 1;
                if u == parent {
                    continue;
                }
                if visited[u] {
                    low[v] = low[v].min(disc[u]);
                } else {
                    visited[u] = true;
                    disc[u] = timer;
                    low[u] = timer;
                    timer += 1;
                    if v == start {
                        root_children += 1;
                    }
                    stack.push((u, v, 0));
                }
            } else {
                stack.pop();
                if parent != usize::MAX {
                    low[parent] = low[parent].min(low[v]);
                    if parent != start && low[v] >= disc[parent] {
                        is_articulation[parent] = true;
                    }
                }
            }
        }
        if root_children >= 2 {
            is_articulation[start] = true;
        }
    }

    let mut out: Vec<Singularity> = (0..n)
        .filter(|&i| is_articulation[i])
        .map(|node| Singularity {
            node,
            coupling_mass: g.coupling_mass(node),
        })
        .collect();
    out.sort_by(|a, b| {
        b.coupling_mass
            .cmp(&a.coupling_mass)
            .then(a.node.cmp(&b.node))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(n: u64, d: u64) -> Q16 {
        Q16::ratio(n, d).unwrap()
    }

    /// Two K4 cliques (0–3 and 4–7) joined by a single weak bridge 3–4.
    fn barbell() -> CouplingGraph {
        let mut g = CouplingGraph::new(8);
        for clique in [[0, 1, 2, 3], [4, 5, 6, 7]] {
            for (i, &a) in clique.iter().enumerate() {
                for &b in clique.iter().skip(i + 1) {
                    assert!(g.add_edge(a, b, Q16::ONE));
                }
            }
        }
        assert!(g.add_edge(3, 4, q(1, 10)));
        g
    }

    #[test]
    fn graph_construction_is_fail_closed() {
        let mut g = CouplingGraph::new(3);
        assert!(!g.add_edge(0, 0, Q16::ONE), "self-loop");
        assert!(!g.add_edge(0, 3, Q16::ONE), "out of range");
        assert!(!g.add_edge(0, 1, Q16::ZERO), "non-positive weight");
        assert!(g.add_edge(0, 1, Q16::HALF));
        assert!(!g.add_edge(1, 0, Q16::ONE), "duplicate pair (either order)");
        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.weight(1, 0), Some(Q16::HALF));
        assert_eq!(g.coupling_mass(0), Q16::HALF);
        assert_eq!(g.coupling_mass(2), Q16::ZERO);
    }

    #[test]
    fn fiedler_bisect_splits_the_barbell_at_the_bridge() {
        let (left, right) = fiedler_bisect(&barbell());
        let (a, b) = if left.contains(&0) {
            (left, right)
        } else {
            (right, left)
        };
        assert_eq!(a, vec![0, 1, 2, 3]);
        assert_eq!(b, vec![4, 5, 6, 7]);
    }

    #[test]
    fn bisection_is_deterministic_and_total() {
        let g = barbell();
        assert_eq!(fiedler_bisect(&g), fiedler_bisect(&g));
        // Tiny and degenerate graphs stay total.
        assert_eq!(fiedler_bisect(&CouplingGraph::new(0)), (vec![], vec![]));
        assert_eq!(fiedler_bisect(&CouplingGraph::new(1)), (vec![0], vec![]));
        let (l, r) = fiedler_bisect(&CouplingGraph::new(4)); // edgeless
        assert_eq!(l.len() + r.len(), 4);
        assert!(!l.is_empty() && !r.is_empty());
    }

    #[test]
    fn disconnected_components_separate_before_any_spectral_work() {
        let mut g = CouplingGraph::new(5);
        g.add_edge(0, 1, Q16::ONE);
        g.add_edge(3, 4, Q16::ONE);
        let components = connected_components(&g);
        assert_eq!(components, vec![vec![0, 1], vec![2], vec![3, 4]]);
    }

    #[test]
    fn clusters_emerge_instead_of_being_imposed() {
        // The barbell yields exactly 2 clusters even when 4 are allowed:
        // a K4 refuses to shatter (any internal cut has conductance > 1/2).
        let clusters = spectral_clusters(&barbell(), 4);
        assert_eq!(clusters, vec![vec![0, 1, 2, 3], vec![4, 5, 6, 7]]);

        // max_clusters = 1 keeps the graph whole.
        assert_eq!(spectral_clusters(&barbell(), 1).len(), 1);

        // Three disconnected triangles pin three clusters.
        let mut g = CouplingGraph::new(9);
        for t in [[0, 1, 2], [3, 4, 5], [6, 7, 8]] {
            g.add_edge(t[0], t[1], Q16::ONE);
            g.add_edge(t[1], t[2], Q16::ONE);
            g.add_edge(t[0], t[2], Q16::ONE);
        }
        let clusters = spectral_clusters(&g, 5);
        assert_eq!(clusters, vec![vec![0, 1, 2], vec![3, 4, 5], vec![6, 7, 8]]);
    }

    #[test]
    fn clustering_is_deterministic() {
        let g = barbell();
        assert_eq!(spectral_clusters(&g, 4), spectral_clusters(&g, 4));
    }

    #[test]
    fn kuramoto_groups_synchronize_within_coupled_regions() {
        // One strongly coupled triangle synchronizes into ONE group.
        let mut tri = CouplingGraph::new(3);
        tri.add_edge(0, 1, Q16::ONE);
        tri.add_edge(1, 2, Q16::ONE);
        tri.add_edge(0, 2, Q16::ONE);
        assert_eq!(kuramoto_groups(&tri), vec![vec![0, 1, 2]]);

        // Two disconnected triangles never merge.
        let mut two = CouplingGraph::new(6);
        for t in [[0, 1, 2], [3, 4, 5]] {
            two.add_edge(t[0], t[1], Q16::ONE);
            two.add_edge(t[1], t[2], Q16::ONE);
            two.add_edge(t[0], t[2], Q16::ONE);
        }
        assert_eq!(kuramoto_groups(&two), vec![vec![0, 1, 2], vec![3, 4, 5]]);

        // Edgeless nodes stay solo; determinism holds.
        let lonely = CouplingGraph::new(3);
        assert_eq!(kuramoto_groups(&lonely), vec![vec![0], vec![1], vec![2]]);
        assert_eq!(kuramoto_groups(&two), kuramoto_groups(&two));
    }

    #[test]
    fn singularities_are_articulation_nodes_ranked_by_mass() {
        // Path 0–1–2: node 1 is the only articulation point.
        let mut path = CouplingGraph::new(3);
        path.add_edge(0, 1, Q16::HALF);
        path.add_edge(1, 2, Q16::ONE);
        let s = singularities(&path);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].node, 1);
        assert_eq!(s[0].coupling_mass, Q16::HALF.saturating_add(Q16::ONE));

        // A clique has none (no single node disconnects it).
        let mut clique = CouplingGraph::new(4);
        for i in 0..4 {
            for j in (i + 1)..4 {
                clique.add_edge(i, j, Q16::ONE);
            }
        }
        assert!(singularities(&clique).is_empty());

        // Barbell: both bridge endpoints are singular, heaviest first.
        let s = singularities(&barbell());
        let nodes: Vec<usize> = s.iter().map(|x| x.node).collect();
        assert_eq!(nodes, vec![3, 4], "bridge endpoints, mass-ranked");
        assert!(s[0].coupling_mass >= s[1].coupling_mass);
    }

    /// The quarantine pin: no float type may appear anywhere in the public
    /// surface (signatures, struct/enum bodies, consts) of this crate.
    #[test]
    fn public_api_is_float_free() {
        let body = include_str!("lib.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap()
            .to_string();
        let mut surface = String::new();
        let mut lines = body.lines().peekable();
        while let Some(line) = lines.next() {
            let t = line.trim_start();
            if t.starts_with("pub fn") || t.starts_with("pub const") {
                surface.push_str(line);
                surface.push('\n');
                let mut current = line.to_string();
                while !current.contains('{') && !current.contains(';') {
                    let Some(next) = lines.next() else { break };
                    surface.push_str(next);
                    surface.push('\n');
                    current = next.to_string();
                }
            } else if t.starts_with("pub struct") || t.starts_with("pub enum") {
                surface.push_str(line);
                surface.push('\n');
                if !line.contains(';') && !line.contains('}') {
                    for next in lines.by_ref() {
                        surface.push_str(next);
                        surface.push('\n');
                        if next.trim_start().starts_with('}') {
                            break;
                        }
                    }
                }
            }
        }
        assert!(!surface.is_empty(), "the scan must see the API");
        for forbidden in ["f64", "f32"] {
            assert!(
                !surface.contains(forbidden),
                "float type '{forbidden}' leaked into the public surface:\n{surface}"
            );
        }
    }

    /// The dependency pin: spectral work needs no linear-algebra crate and
    /// stays kosmo-core-only.
    #[test]
    fn dependencies_stay_minimal() {
        let manifest = include_str!("../Cargo.toml");
        for forbidden in ["nalgebra", "ndarray", "petgraph", "rand"] {
            assert!(
                !manifest.contains(forbidden),
                "kosmo-spectral must not depend on {forbidden}"
            );
        }
        assert!(manifest.contains("kosmo-core"));
    }
}

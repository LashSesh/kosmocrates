//! Semantic knowledge clustering and topology analysis.
//!
//! Groups crystals into *knowledge islands* — dense regions of the IL vector
//! space where crystals share high semantic proximity.  Identifies:
//!
//! - **Clusters**: connected components of the cosine-similarity graph
//!   (edges where similarity ≥ `sim_threshold`).
//! - **Singletons**: crystals with no neighbours above the threshold.
//! - **Bridge crystals**: crystals that have direct causal links into at
//!   least two distinct clusters — high-value connectors whose loss would
//!   fragment the knowledge graph.
//!
//! ## Algorithm
//!
//! 1. Build an undirected *similarity graph* over all committed crystals:
//!    edge (i, j) iff `cosine(v_i, v_j) ≥ sim_threshold`.
//! 2. Find connected components via Union-Find → each component is a cluster.
//! 3. Compute per-cluster metrics:
//!    - `centroid`: mean 8D IL vector of all members.
//!    - `mean_stability`, `mean_uncertainty`: arithmetic means.
//!    - `causal_density`: fraction of member-pairs connected by at least one
//!      direct causal link.  A value of 1.0 means the cluster is fully
//!      causally wired; 0.0 means semantically similar but causally isolated.
//! 4. **Bridge crystals**: for every causal edge that crosses cluster
//!    boundaries, both endpoints are marked as bridges.
//!
//! ## Interpretation
//!
//! | Cluster property        | Interpretation                                |
//! |-------------------------|-----------------------------------------------|
//! | Large, high-density     | Mature, well-reinforced knowledge island      |
//! | Large, low causal density | Semantic accumulation without provenance    |
//! | Singleton               | Isolated — consolidation or deprecation target |
//! | Bridge crystal          | Critical connector — protect from decay       |
//!
//! ## Sim threshold guide
//!
//! | Threshold | Typical effect                               |
//! |-----------|----------------------------------------------|
//! | 0.95      | Only near-duplicate crystals cluster together |
//! | 0.80      | Topically related crystals merge             |
//! | 0.60      | Broad thematic clustering                    |

use serde::{Deserialize, Serialize};

// ─── Configuration ────────────────────────────────────────────────────────────

/// Configuration for [`ILStore::cluster_knowledge`].
#[derive(Debug, Clone)]
pub struct ClusterConfig {
    /// Cosine similarity threshold for adding an edge to the similarity graph.
    /// Crystals with similarity ≥ this value are placed in the same cluster.
    /// Default: 0.80.
    pub sim_threshold: f64,
    /// Minimum number of crystals for a component to be reported as a cluster
    /// (rather than a singleton).  Default: 2.
    pub min_cluster_size: usize,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self { sim_threshold: 0.80, min_cluster_size: 2 }
    }
}

// ─── Knowledge cluster ───────────────────────────────────────────────────────

/// A semantically coherent group of crystals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeCluster {
    /// Sequential cluster identifier (0-based, sorted by descending size).
    pub cluster_id: usize,
    /// Crystal ID prefixes (16 hex chars) of all member crystals.
    pub members: Vec<String>,
    /// Mean 8D IL vector of all members (the semantic centroid).
    pub centroid: Vec<f64>,
    /// Mean PSE stability score of cluster members.
    pub mean_stability: f64,
    /// Mean epistemic uncertainty of cluster members.
    pub mean_uncertainty: f64,
    /// Fraction of member-pairs that share a direct causal link.
    ///
    /// `causal_density = direct_causal_edges / C(|members|, 2)`
    /// where C(n,2) = n*(n-1)/2.  Always 0 when |members| < 2.
    pub causal_density: f64,
    /// Mean QTIC class across members with a certificate (None if none exist).
    pub mean_qtic_class: Option<f64>,
}

impl KnowledgeCluster {
    /// Number of member crystals.
    pub fn size(&self) -> usize {
        self.members.len()
    }

    /// True when every member-pair has a direct causal link (fully connected).
    pub fn is_causally_dense(&self) -> bool {
        self.causal_density >= 1.0
    }

    /// One-line summary.
    pub fn summary(&self) -> String {
        let q = self
            .mean_qtic_class
            .map(|q| format!("Q{:.1}", q))
            .unwrap_or_else(|| "Q?".to_string());
        format!(
            "Cluster[{id}] {n} members | stab={s:.2} u={u:.2} causal_d={d:.2} {q}",
            id = self.cluster_id,
            n = self.members.len(),
            s = self.mean_stability,
            u = self.mean_uncertainty,
            d = self.causal_density,
            q = q,
        )
    }
}

// ─── Bridge crystal ───────────────────────────────────────────────────────────

/// A crystal that causally connects two or more distinct clusters.
///
/// Bridge crystals are high-value: removing or deprecating them would fragment
/// the knowledge graph, severing causal paths between otherwise separate islands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeCrystal {
    /// Crystal ID prefix of the bridge.
    pub crystal_id: String,
    /// The distinct cluster IDs this crystal bridges.
    pub bridges: Vec<usize>,
    /// Number of cross-cluster causal edges incident to this crystal.
    pub cross_cluster_degree: usize,
}

// ─── Clustering report ────────────────────────────────────────────────────────

/// Full knowledge topology report for an IL store.
///
/// Built by [`ILStore::cluster_knowledge`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusteringReport {
    /// Configuration used to produce this report.
    pub sim_threshold: f64,
    /// All clusters with ≥ `min_cluster_size` members, sorted by descending size.
    pub clusters: Vec<KnowledgeCluster>,
    /// Crystals with no neighbours above `sim_threshold` (isolated knowledge).
    pub singletons: Vec<String>,
    /// Crystals that bridge two or more clusters via causal edges.
    pub bridge_crystals: Vec<BridgeCrystal>,
    /// Total crystals evaluated.
    pub total_crystals: usize,
    /// Fraction of crystals that belong to a cluster (not singletons).
    pub clustered_fraction: f64,
}

impl ClusteringReport {
    /// True when all crystals belong to a single coherent cluster (no singletons,
    /// no multi-cluster fragmentation) — the knowledge topology fixpoint.
    pub fn is_unified(&self) -> bool {
        self.singletons.is_empty() && self.clusters.len() <= 1
    }

    /// Compact one-line summary suitable for LLM context injection.
    pub fn summary(&self) -> String {
        if self.total_crystals == 0 {
            return "ClusteringReport: empty store".to_string();
        }
        format!(
            "ClusteringReport: {n} crystals | {c} cluster(s) | \
             {s} singleton(s) | {b} bridge(s) | clustered={pct:.0}%",
            n = self.total_crystals,
            c = self.clusters.len(),
            s = self.singletons.len(),
            b = self.bridge_crystals.len(),
            pct = self.clustered_fraction * 100.0,
        )
    }
}

// ─── Union-Find ───────────────────────────────────────────────────────────────

/// Minimal path-compressed Union-Find for connected-component labelling.
pub(crate) struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl UnionFind {
    pub fn new(n: usize) -> Self {
        Self { parent: (0..n).collect(), rank: vec![0; n] }
    }

    pub fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    pub fn union(&mut self, x: usize, y: usize) {
        let rx = self.find(x);
        let ry = self.find(y);
        if rx == ry {
            return;
        }
        match self.rank[rx].cmp(&self.rank[ry]) {
            std::cmp::Ordering::Less => self.parent[rx] = ry,
            std::cmp::Ordering::Greater => self.parent[ry] = rx,
            std::cmp::Ordering::Equal => {
                self.parent[ry] = rx;
                self.rank[rx] += 1;
            }
        }
    }
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── UnionFind ─────────────────────────────────────────────────────────

    #[test]
    fn union_find_all_distinct_initially() {
        let mut uf = UnionFind::new(5);
        for i in 0..5 {
            assert_eq!(uf.find(i), i);
        }
    }

    #[test]
    fn union_find_merges_components() {
        let mut uf = UnionFind::new(4);
        uf.union(0, 1);
        uf.union(2, 3);
        assert_eq!(uf.find(0), uf.find(1));
        assert_eq!(uf.find(2), uf.find(3));
        assert_ne!(uf.find(0), uf.find(2));
    }

    #[test]
    fn union_find_path_compression() {
        let mut uf = UnionFind::new(5);
        uf.union(0, 1);
        uf.union(1, 2);
        uf.union(2, 3);
        uf.union(3, 4);
        let root = uf.find(0);
        // All elements must resolve to the same root
        for i in 1..5 {
            assert_eq!(uf.find(i), root);
        }
    }

    // ── ClusterConfig ────────────────────────────────────────────────────

    #[test]
    fn default_config_sensible() {
        let cfg = ClusterConfig::default();
        assert!((cfg.sim_threshold - 0.80).abs() < 1e-9);
        assert_eq!(cfg.min_cluster_size, 2);
    }

    // ── KnowledgeCluster ─────────────────────────────────────────────────

    #[test]
    fn cluster_summary_format() {
        let c = KnowledgeCluster {
            cluster_id: 0,
            members: vec!["aaa".to_string(), "bbb".to_string()],
            centroid: vec![0.5; 8],
            mean_stability: 0.80,
            mean_uncertainty: 0.25,
            causal_density: 1.0,
            mean_qtic_class: Some(4.0),
        };
        let s = c.summary();
        assert!(s.contains("Cluster[0]"));
        assert!(s.contains("2 members"));
        assert!(c.is_causally_dense());
    }

    // ── ClusteringReport ─────────────────────────────────────────────────

    #[test]
    fn empty_report_summary() {
        let r = ClusteringReport {
            sim_threshold: 0.80,
            clusters: vec![],
            singletons: vec![],
            bridge_crystals: vec![],
            total_crystals: 0,
            clustered_fraction: 0.0,
        };
        assert!(r.summary().contains("empty"));
        assert!(r.is_unified()); // vacuously unified
    }

    #[test]
    fn unified_when_single_cluster_no_singletons() {
        let r = ClusteringReport {
            sim_threshold: 0.80,
            clusters: vec![KnowledgeCluster {
                cluster_id: 0,
                members: vec!["aaa".to_string(), "bbb".to_string()],
                centroid: vec![0.5; 8],
                mean_stability: 0.80,
                mean_uncertainty: 0.20,
                causal_density: 1.0,
                mean_qtic_class: Some(4.0),
            }],
            singletons: vec![],
            bridge_crystals: vec![],
            total_crystals: 2,
            clustered_fraction: 1.0,
        };
        assert!(r.is_unified());
    }

    #[test]
    fn not_unified_when_singletons_exist() {
        let r = ClusteringReport {
            sim_threshold: 0.80,
            clusters: vec![],
            singletons: vec!["orphan".to_string()],
            bridge_crystals: vec![],
            total_crystals: 1,
            clustered_fraction: 0.0,
        };
        assert!(!r.is_unified());
        assert!(r.summary().contains("1 singleton"));
    }
}

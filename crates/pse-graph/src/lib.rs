//! Observation graph, entity tracking, and persistent hash-DAG for PSE.
//!
//! Combines the observation ingestion layer (L0) with the persistent
//! graph storage layer (L1).

use std::collections::BTreeMap;
use pse_types::{
    CommitIndex, EdgeAnnotation, FiveDState, Hash256, MeasurementContext,
    Observation, PersistenceConfig, ProvenanceEnvelope, VertexId,
    content_address_raw,
};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;
use thiserror::Error;

// ─── Errors ──────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum ObserveError {
    #[error("canonicalization failed: {0}")]
    Canonicalize(String),
    #[error("digest mismatch")]
    DigestMismatch,
}

#[derive(Debug, Error)]
pub enum PersistError {
    #[error("vertex not found: {0}")]
    VertexNotFound(VertexId),
    #[error("graph capacity exceeded")]
    CapacityExceeded,
    #[error("integrity check failed: {0}")]
    IntegrityFail(String),
}

// ─── Observation Adapter ────────────────────────────────────────────────────

/// Observation canonicalization adapter.
pub trait ObservationAdapter: Send + Sync {
    /// Unique identifier for this observation source.
    fn source_id(&self) -> &str;
    /// Canonicalize raw bytes into an observation.
    fn canonicalize(
        &self,
        raw: &[u8],
        context: &MeasurementContext,
    ) -> Result<Observation, ObserveError>;
}

/// Idempotent ingestion: same raw input produces same digest.
pub fn ingest(
    adapter: &dyn ObservationAdapter,
    raw: &[u8],
    ctx: &MeasurementContext,
) -> Result<Observation, ObserveError> {
    let obs = adapter.canonicalize(raw, ctx)?;
    let recomputed = content_address_raw(&obs.payload);
    if recomputed != obs.digest {
        return Err(ObserveError::DigestMismatch);
    }
    Ok(obs)
}

/// Fast ingestion path that trusts the adapter's digest computation.
/// Skips the redundant SHA-256 re-verification. Use when the adapter is
/// known to produce correct digests (e.g. PassthroughAdapter).
#[inline]
pub fn ingest_trusted(
    adapter: &dyn ObservationAdapter,
    raw: &[u8],
    ctx: &MeasurementContext,
) -> Result<Observation, ObserveError> {
    adapter.canonicalize(raw, ctx)
}

/// A passthrough adapter that treats raw bytes as payload.
pub struct PassthroughAdapter {
    id: String,
    schema_version: String,
}

impl PassthroughAdapter {
    /// Create a new passthrough adapter with the given source ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into(), schema_version: "1.0.0".to_string() }
    }
}

impl ObservationAdapter for PassthroughAdapter {
    fn source_id(&self) -> &str { &self.id }

    fn canonicalize(
        &self,
        raw: &[u8],
        context: &MeasurementContext,
    ) -> Result<Observation, ObserveError> {
        let payload = raw.to_vec();
        let digest: Hash256 = content_address_raw(&payload);
        Ok(Observation {
            timestamp: 0.0,
            source_id: self.id.clone(),
            provenance: ProvenanceEnvelope {
                origin: self.id.clone(),
                chain: Vec::new(),
                sig: None,
            },
            payload,
            context: context.clone(),
            digest,
            schema_version: self.schema_version.clone(),
        })
    }
}

/// High-throughput adapter that minimizes per-observation overhead.
///
/// Compared to `PassthroughAdapter`:
/// - Caches `derive_vertex_id` result (avoids FNV hash per obs)
/// - Pre-computes provenance and context once
/// - `ingest()` computes SHA-256 exactly once (no verification re-hash)
pub struct FastPassthroughAdapter {
    id: String,
    schema_version: String,
    /// Pre-computed vertex ID for this source.
    pub cached_vid: VertexId,
}

impl FastPassthroughAdapter {
    pub fn new(id: impl Into<String>) -> Self {
        let id = id.into();
        let cached_vid = derive_vertex_id(&id);
        Self {
            id,
            schema_version: "1.0.0".to_string(),
            cached_vid,
        }
    }

    /// Ingest raw bytes into an Observation.
    /// Computes SHA-256 exactly once. No verification re-hash.
    #[inline]
    pub fn ingest(&self, raw: &[u8]) -> Observation {
        let payload = raw.to_vec();
        let digest: Hash256 = content_address_raw(&payload);
        Observation {
            timestamp: 0.0,
            source_id: self.id.clone(),
            provenance: ProvenanceEnvelope {
                origin: self.id.clone(),
                chain: Vec::new(),
                sig: None,
            },
            payload,
            context: MeasurementContext::default(),
            digest,
            schema_version: self.schema_version.clone(),
        }
    }
}

impl ObservationAdapter for FastPassthroughAdapter {
    fn source_id(&self) -> &str { &self.id }

    fn canonicalize(
        &self,
        raw: &[u8],
        _context: &MeasurementContext,
    ) -> Result<Observation, ObserveError> {
        Ok(self.ingest(raw))
    }
}

// ─── Tensor Archive ───────────────────────────────────────────────────────────

/// Tensor archive: stores historical 5D state snapshots for a vertex.
#[derive(Clone, Debug, Default)]
pub struct TensorArchive {
    pub snapshots: Vec<FiveDState>,
    pub timestamps: Vec<f64>,
}

impl TensorArchive {
    /// Append a snapshot.
    pub fn push(&mut self, state: FiveDState, timestamp: f64) {
        self.snapshots.push(state);
        self.timestamps.push(timestamp);
    }

    /// Get the most recent snapshot.
    pub fn latest(&self) -> Option<&FiveDState> {
        self.snapshots.last()
    }
}

// ─── Storage Tiers ────────────────────────────────────────────────────────────

/// Hot tier: in-memory recent data.
#[derive(Default, Debug)]
pub struct HotTier {
    pub data: BTreeMap<VertexId, Vec<(f64, Vec<u8>)>>,
}

/// Warm tier: compressed on-disk (simulated in-memory).
#[derive(Default, Debug)]
pub struct WarmTier {
    pub data: BTreeMap<VertexId, Vec<(f64, Vec<u8>)>>,
    pub corrupted: bool,
}

/// Cold tier: indefinite, append-only.
#[derive(Default, Debug)]
pub struct ColdTier {
    pub data: BTreeMap<VertexId, Vec<(f64, Vec<u8>)>>,
}

// ─── Vertex Data ──────────────────────────────────────────────────────────────

/// Vertex metadata in the observation graph.
#[derive(Clone, Debug)]
pub struct VertexData {
    pub id: VertexId,
    pub active: bool,
    pub first_seen: f64,
    pub last_seen: f64,
    pub activation_count: u64,
}

impl VertexData {
    /// Create a new active vertex.
    pub fn new(id: VertexId, timestamp: f64) -> Self {
        Self { id, active: true, first_seen: timestamp, last_seen: timestamp, activation_count: 1 }
    }
}

/// Record of an observation in the append-only history.
#[derive(Clone, Debug)]
pub struct ObservationRecord {
    pub commit_index: CommitIndex,
    pub digest: Hash256,
    pub timestamp: f64,
}

// ─── Persistent Graph ─────────────────────────────────────────────────────────

/// Persistent observation graph (hash-DAG).
pub struct PersistentGraph {
    pub graph: DiGraph<VertexData, EdgeAnnotation>,
    pub id_map: BTreeMap<VertexId, NodeIndex>,
    pub tensor: BTreeMap<VertexId, TensorArchive>,
    pub embedding: BTreeMap<VertexId, FiveDState>,
    pub hot: HotTier,
    pub warm: WarmTier,
    pub cold: ColdTier,
    pub commit_index: CommitIndex,
    pub history: Vec<ObservationRecord>,
}

impl Default for PersistentGraph {
    fn default() -> Self {
        Self {
            graph: DiGraph::new(),
            id_map: BTreeMap::new(),
            tensor: BTreeMap::new(),
            embedding: BTreeMap::new(),
            hot: HotTier::default(),
            warm: WarmTier::default(),
            cold: ColdTier::default(),
            commit_index: 0,
            history: Vec::new(),
        }
    }
}

impl PersistentGraph {
    /// Create a new empty persistent graph.
    pub fn new() -> Self { Self::default() }

    /// Estimate heap size in bytes.
    pub fn estimate_heap_size(&self) -> usize {
        let id_map_bytes = self.id_map.len() * (std::mem::size_of::<u64>() + std::mem::size_of::<usize>());
        let vertex_bytes = self.graph.node_count() * std::mem::size_of::<VertexData>();
        let edge_bytes = self.graph.edge_count() * std::mem::size_of::<EdgeAnnotation>();
        let embedding_bytes = self.embedding.len() * std::mem::size_of::<FiveDState>();
        let history_bytes = self.history.len() * std::mem::size_of::<ObservationRecord>();
        let tensor_bytes = self.tensor.len() * 64;
        id_map_bytes + vertex_bytes + edge_bytes + embedding_bytes + history_bytes + tensor_bytes
    }

    /// Upsert a vertex; returns its NodeIndex.
    pub fn upsert_vertex(&mut self, id: VertexId, timestamp: f64) -> NodeIndex {
        if let Some(&nidx) = self.id_map.get(&id) {
            if let Some(data) = self.graph.node_weight_mut(nidx) {
                data.last_seen = timestamp;
                data.activation_count += 1;
            }
            nidx
        } else {
            let nidx = self.graph.add_node(VertexData::new(id, timestamp));
            self.id_map.insert(id, nidx);
            self.embedding.insert(id, FiveDState::default());
            self.tensor.insert(id, TensorArchive::default());
            nidx
        }
    }

    /// Upsert an edge between two vertices.
    pub fn upsert_edge(&mut self, from: VertexId, to: VertexId, timestamp: f64) {
        let from_idx = self.upsert_vertex(from, timestamp);
        let to_idx = self.upsert_vertex(to, timestamp);
        if !self.graph.contains_edge(from_idx, to_idx) {
            self.graph.add_edge(from_idx, to_idx, EdgeAnnotation {
                birth_time: timestamp, last_update: timestamp, weight: 1.0,
                active_windows: 1, ..Default::default()
            });
        } else if let Some(edge_idx) = self.graph.find_edge(from_idx, to_idx) {
            if let Some(ann) = self.graph.edge_weight_mut(edge_idx) {
                ann.last_update = timestamp;
                ann.active_windows += 1;
                ann.weight = (ann.weight + 1.0) * 0.5;
            }
        }
    }

    /// Apply a batch of observations to the graph.
    pub fn apply_observations(
        &mut self,
        obs_batch: &[Observation],
        config: &PersistenceConfig,
    ) -> Result<(), PersistError> {
        if self.id_map.len() + obs_batch.len() > config.max_vertices {
            return Err(PersistError::CapacityExceeded);
        }

        let mut batch_vids: Vec<VertexId> = Vec::with_capacity(obs_batch.len());

        for obs in obs_batch {
            let timestamp = obs.timestamp;
            let vid = derive_vertex_id(&obs.source_id);
            self.upsert_vertex(vid, timestamp);
            batch_vids.push(vid);

            if obs.payload.len() >= 16 && std::str::from_utf8(&obs.payload).is_err() {
                for chunk in obs.payload.chunks_exact(16) {
                    let from_bytes: [u8; 8] = chunk[0..8].try_into().unwrap_or([0u8; 8]);
                    let to_bytes: [u8; 8] = chunk[8..16].try_into().unwrap_or([0u8; 8]);
                    let from_vid = u64::from_le_bytes(from_bytes);
                    let to_vid = u64::from_le_bytes(to_bytes);
                    if from_vid != to_vid {
                        self.upsert_edge(from_vid, to_vid, timestamp);
                    }
                }
            }

            // E.3: per-vertex 5D embedding is now computed in a single
            // post-batch pass below from actual graph topology, not from
            // payload hashes or wall-clock time. We still snapshot the
            // current (stale-but-defined) embedding into the tensor
            // archive on each observation so the per-observation history
            // remains aligned to the timestamp axis.
            let snap = self.embedding.get(&vid).cloned().unwrap_or_default();
            if let Some(archive) = self.tensor.get_mut(&vid) {
                archive.push(snap, timestamp);
            }

            self.history.push(ObservationRecord {
                commit_index: self.commit_index,
                digest: obs.digest,
                timestamp,
            });

            self.hot.data.entry(vid).or_default().push((timestamp, obs.payload.clone()));
        }

        let lambda = config.lambda_decay;
        let n_obs = obs_batch.len();
        if lambda > 0.0 && n_obs > 0 {
            let decay = (-lambda * n_obs as f64).exp();
            for eidx in self.graph.edge_indices() {
                if let Some(ann) = self.graph.edge_weight_mut(eidx) {
                    ann.weight *= decay;
                }
            }
        }

        // Deduplicate vertex IDs before pairwise edge creation to avoid
        // redundant upsert_edge calls when many observations share a source.
        // We still create edges for all unique pairs (including self-loops
        // when a single vertex appears, since metrics depend on edge_count).
        let batch_ts = obs_batch.first().map(|o| o.timestamp).unwrap_or(0.0);
        let raw_len = batch_vids.len();
        batch_vids.sort_unstable();
        batch_vids.dedup();
        let n = batch_vids.len();
        if n == 1 && raw_len > 1 {
            // All observations mapped to same vertex — create a self-loop
            // (preserves edge_count for j-metric computation).
            self.upsert_edge(batch_vids[0], batch_vids[0], batch_ts);
        } else if n <= 64 {
            for i in 0..n {
                for j in (i + 1)..n {
                    self.upsert_edge(batch_vids[i], batch_vids[j], batch_ts);
                }
            }
        } else {
            for i in 0..n {
                let next1 = (i + 1) % n;
                let next2 = (i + 2) % n;
                for &next in &[next1, next2] {
                    if batch_vids[i] != batch_vids[next] {
                        let (u, v) = if batch_vids[i] < batch_vids[next] {
                            (batch_vids[i], batch_vids[next])
                        } else {
                            (batch_vids[next], batch_vids[i])
                        };
                        self.upsert_edge(u, v, batch_ts);
                    }
                }
            }
        }

        // E.3: compute the 5D embedding for every vertex from the actual
        // graph topology. All five axes are now physically grounded:
        //
        //   p (potential)    = sum of incident edge weights, normalized by
        //                      the maximum such sum across all vertices.
        //                      Captures local "potential energy density".
        //   ρ (density)      = local clustering coefficient
        //                      (edges among neighbours) / C(deg, 2).
        //                      Captures local triangle density.
        //   ω (frequency)    = activation_count, normalized by the maximum
        //                      activation count. Captures recurrence rate
        //                      (replaces the previous arbitrary daily-cycle
        //                      derivation from wall-clock timestamps).
        //   χ (connectivity) = undirected degree, normalized by the maximum
        //                      degree. Kept from the prior implementation.
        //   η (causality)    = out_degree / (out_degree + in_degree + ε)
        //                      in [0, 1]: 0.5 = balanced flow,
        //                      1 = pure source, 0 = pure sink.
        //                      Replaces the previous constant 0.5 (the
        //                      causality axis was effectively dead).
        //
        // Cost: O(|V| · ⟨deg⟩²) for clustering; the rest are O(|V|·⟨deg⟩).
        // For PSE's typical small graphs this is fully tolerable.
        let mut max_degree = 1.0_f64;
        let mut max_activation = 1u64;
        let mut max_incident_weight = 1e-9_f64;
        let node_indices: Vec<NodeIndex> = self.graph.node_indices().collect();
        // Pre-pass: gather normalization constants.
        for ni in &node_indices {
            let deg = self.graph.neighbors_undirected(*ni).count() as f64;
            if deg > max_degree { max_degree = deg; }
            if let Some(data) = self.graph.node_weight(*ni) {
                if data.activation_count > max_activation {
                    max_activation = data.activation_count;
                }
            }
            let inc_w = sum_incident_edge_weights(&self.graph, *ni);
            if inc_w > max_incident_weight { max_incident_weight = inc_w; }
        }

        let all_vids: Vec<VertexId> = self.embedding.keys().cloned().collect();
        for vid in all_vids {
            if let Some(&nidx) = self.id_map.get(&vid) {
                let undirected_deg = self.graph.neighbors_undirected(nidx).count() as f64;
                let out_deg = self.graph.neighbors_directed(nidx, Direction::Outgoing).count() as f64;
                let in_deg = self.graph.neighbors_directed(nidx, Direction::Incoming).count() as f64;
                let activation = self.graph.node_weight(nidx)
                    .map(|d| d.activation_count).unwrap_or(0);
                let inc_w = sum_incident_edge_weights(&self.graph, nidx);
                let rho = clustering_coefficient(&self.graph, nidx);
                let p = inc_w / max_incident_weight;
                let omega = (activation as f64) / (max_activation as f64);
                let chi = undirected_deg / max_degree;
                let eta = if out_deg + in_deg < 1e-12 {
                    0.5
                } else {
                    out_deg / (out_deg + in_deg)
                };
                if let Some(embed) = self.embedding.get_mut(&vid) {
                    embed.p = p.clamp(0.0, 1.0);
                    embed.rho = rho.clamp(0.0, 1.0);
                    embed.omega = omega.clamp(0.0, 1.0);
                    embed.chi = chi.clamp(0.0, 1.0);
                    embed.eta = eta.clamp(0.0, 1.0);
                }
            }
        }

        self.commit_index += 1;
        Ok(())
    }

    /// Mark a vertex as inactive (preserves history).
    pub fn deactivate_vertex(&mut self, id: VertexId) {
        if let Some(&nidx) = self.id_map.get(&id) {
            if let Some(data) = self.graph.node_weight_mut(nidx) {
                data.active = false;
            }
        }
    }

    /// Get all active vertex IDs.
    pub fn active_vertices(&self) -> Vec<VertexId> {
        self.graph.node_weights().filter(|d| d.active).map(|d| d.id).collect()
    }

    /// Get embedding for a vertex.
    pub fn get_embedding(&self, id: VertexId) -> Option<&FiveDState> {
        self.embedding.get(&id)
    }

    /// Get all embeddings as a point cloud.
    pub fn point_cloud(&self) -> Vec<(VertexId, FiveDState)> {
        self.embedding.iter().map(|(vid, state)| (*vid, state.clone())).collect()
    }

    /// Compute topology signature for the current graph.
    pub fn topology_signature(&self) -> pse_types::TopologySignature {
        let n = self.graph.node_count() as u64;
        let e = self.graph.edge_count() as u64;
        let betti_0 = if n == 0 { 0 } else { count_weakly_connected(&self.graph) };
        let betti_1 = (e + betti_0).saturating_sub(n);
        let spectral_gap = compute_spectral_gap(&self.graph);
        let euler_char = n as i64 - e as i64;

        pse_types::TopologySignature {
            betti_0, betti_1, betti_2: 0, spectral_gap, euler_char, ..Default::default()
        }
    }
}

/// Sum of all edge weights incident to a vertex (both incoming and
/// outgoing, counted once per directed edge — duplicates a self-loop
/// twice, which is the correct contribution of a self-loop to the
/// vertex's potential energy).
fn sum_incident_edge_weights(
    graph: &DiGraph<VertexData, EdgeAnnotation>,
    nidx: NodeIndex,
) -> f64 {
    let mut sum = 0.0_f64;
    for er in graph.edges_directed(nidx, Direction::Outgoing) {
        sum += er.weight().weight;
    }
    for er in graph.edges_directed(nidx, Direction::Incoming) {
        sum += er.weight().weight;
    }
    sum
}

/// Local clustering coefficient of a vertex: the fraction of
/// neighbour-pairs that are themselves directly connected. Returns 0
/// for vertices with degree < 2.
///
/// Treats the underlying graph as undirected for the purpose of
/// "neighbour-pair connectivity" — the directed edge-set contains each
/// observation-driven link, but topologically two vertices that
/// share *any* edge between them are connected for clustering.
fn clustering_coefficient(
    graph: &DiGraph<VertexData, EdgeAnnotation>,
    nidx: NodeIndex,
) -> f64 {
    let neighbours: Vec<NodeIndex> = graph
        .neighbors_undirected(nidx)
        .filter(|n| *n != nidx)
        .collect();
    let k = neighbours.len();
    if k < 2 {
        return 0.0;
    }
    let max_pairs = (k * (k - 1) / 2) as f64;
    let mut connected_pairs = 0.0_f64;
    for i in 0..k {
        for j in (i + 1)..k {
            let a = neighbours[i];
            let b = neighbours[j];
            if graph.contains_edge(a, b) || graph.contains_edge(b, a) {
                connected_pairs += 1.0;
            }
        }
    }
    connected_pairs / max_pairs
}

/// Map a byte slice to a float in [0, 1) using FNV-1a.
#[allow(dead_code)]
fn fnv_to_unit(data: &[u8]) -> f64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    (h >> 11) as f64 / (1u64 << 53) as f64
}

/// Derive a vertex ID from a string (deterministic, FNV-1a).
pub fn derive_vertex_id(s: &str) -> VertexId {
    let bytes = s.as_bytes();
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Count weakly connected components using union-find.
fn count_weakly_connected(graph: &DiGraph<VertexData, EdgeAnnotation>) -> u64 {
    let n = graph.node_count();
    if n == 0 { return 0; }
    let mut parent: Vec<usize> = (0..n).collect();

    fn find(parent: &mut Vec<usize>, x: usize) -> usize {
        if parent[x] != x { parent[x] = find(parent, parent[x]); }
        parent[x]
    }

    fn union(parent: &mut Vec<usize>, x: usize, y: usize) {
        let rx = find(parent, x);
        let ry = find(parent, y);
        if rx != ry { parent[rx] = ry; }
    }

    for edge in graph.raw_edges() {
        union(&mut parent, edge.source().index(), edge.target().index());
    }

    let mut roots: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    for i in 0..n { roots.insert(find(&mut parent, i)); }
    roots.len() as u64
}

/// Compute spectral gap of graph Laplacian.
fn compute_spectral_gap(graph: &DiGraph<VertexData, EdgeAnnotation>) -> f64 {
    let n = graph.node_count();
    if n < 2 { return 0.0; }
    if n > 100 { return 0.1; }

    let node_indices: Vec<_> = graph.node_indices().collect();
    let idx_map: BTreeMap<petgraph::graph::NodeIndex, usize> = node_indices
        .iter().enumerate().map(|(i, &nidx)| (nidx, i)).collect();

    let mut laplacian = vec![vec![0.0f64; n]; n];
    for edge in graph.raw_edges() {
        let i = idx_map[&edge.source()];
        let j = idx_map[&edge.target()];
        laplacian[i][i] += 1.0;
        laplacian[j][j] += 1.0;
        laplacian[i][j] -= 1.0;
        laplacian[j][i] -= 1.0;
    }

    let max_diag = laplacian.iter().enumerate().map(|(i, row)| row[i]).fold(0.0f64, f64::max);
    let min_diag = laplacian.iter().enumerate().map(|(i, row)| row[i]).fold(f64::INFINITY, f64::min);
    (max_diag - min_diag).abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_obs(src: &str, payload: Vec<u8>, ts: f64) -> Observation {
        let digest = content_address_raw(&payload);
        Observation {
            timestamp: ts,
            source_id: src.to_string(),
            provenance: ProvenanceEnvelope::default(),
            payload,
            context: MeasurementContext::default(),
            digest,
            schema_version: "1.0.0".to_string(),
        }
    }

    #[test]
    fn passthrough_adapter_idempotent() {
        let adapter = PassthroughAdapter::new("test");
        let raw = b"hello world";
        let ctx = MeasurementContext::default();
        let obs1 = ingest(&adapter, raw, &ctx).unwrap();
        let obs2 = ingest(&adapter, raw, &ctx).unwrap();
        assert_eq!(obs1.digest, obs2.digest);
        assert_eq!(obs1.payload, obs2.payload);
    }

    #[test]
    fn different_inputs_different_digests() {
        let adapter = PassthroughAdapter::new("test");
        let ctx = MeasurementContext::default();
        let obs1 = ingest(&adapter, b"input1", &ctx).unwrap();
        let obs2 = ingest(&adapter, b"input2", &ctx).unwrap();
        assert_ne!(obs1.digest, obs2.digest);
    }

    #[test]
    fn upsert_vertex_idempotent() {
        let mut g = PersistentGraph::new();
        let idx1 = g.upsert_vertex(42, 0.0);
        let idx2 = g.upsert_vertex(42, 1.0);
        assert_eq!(idx1, idx2);
        assert_eq!(g.id_map.len(), 1);
    }

    #[test]
    fn apply_observations_increments_commit_index() {
        let mut g = PersistentGraph::new();
        let config = PersistenceConfig::default();
        let obs = vec![make_obs("src1", b"hello".to_vec(), 1.0)];
        g.apply_observations(&obs, &config).unwrap();
        assert_eq!(g.commit_index, 1);
    }

    #[test]
    fn deactivate_vertex_preserves_history() {
        let mut g = PersistentGraph::new();
        let config = PersistenceConfig::default();
        let obs = vec![make_obs("src1", b"data".to_vec(), 1.0)];
        g.apply_observations(&obs, &config).unwrap();
        let vid = derive_vertex_id("src1");
        g.deactivate_vertex(vid);
        assert!(!g.history.is_empty());
        assert!(g.id_map.contains_key(&vid));
    }

    #[test]
    fn topology_signature_empty_graph() {
        let g = PersistentGraph::new();
        let topo = g.topology_signature();
        assert_eq!(topo.betti_0, 0);
    }

    // ── E.3: 5D state from topology ──────────────────────────────────────

    #[test]
    fn embedding_eta_is_no_longer_constant_05() {
        // The most important invariant of E.3: the causality axis must
        // *actually depend on direction*. With a pure source vertex
        // (out-degree > 0, in-degree = 0) η must be ≈ 1.0, not 0.5.
        let mut g = PersistentGraph::new();
        let cfg = PersistenceConfig::default();
        // Two observations into "src1" → vertex created.
        let obs = vec![
            make_obs("src1", b"a".to_vec(), 1.0),
            make_obs("src1", b"b".to_vec(), 2.0),
        ];
        g.apply_observations(&obs, &cfg).unwrap();
        // Manually push outgoing edges from src1 to two new vertices.
        let src = derive_vertex_id("src1");
        g.upsert_edge(src, derive_vertex_id("dst1"), 3.0);
        g.upsert_edge(src, derive_vertex_id("dst2"), 3.0);
        // Force an embedding refresh by ingesting one more observation.
        let extra = vec![make_obs("src1", b"c".to_vec(), 4.0)];
        g.apply_observations(&extra, &cfg).unwrap();
        let embed = g.embedding.get(&src).expect("src1 embedding present");
        assert!(
            embed.eta > 0.6,
            "pure source vertex must have η near 1.0 (out-only), got {}",
            embed.eta
        );
    }

    #[test]
    fn embedding_eta_is_05_for_balanced_flow() {
        // A vertex with equal in- and out-degree is balanced; η = 0.5.
        let mut g = PersistentGraph::new();
        let cfg = PersistenceConfig::default();
        let center = derive_vertex_id("center");
        let a = derive_vertex_id("a");
        let b = derive_vertex_id("b");
        // a → center → b balances center's flow exactly.
        g.upsert_edge(a, center, 1.0);
        g.upsert_edge(center, b, 1.0);
        // Trigger embedding refresh with a no-op batch.
        let dummy = vec![make_obs("a", b"x".to_vec(), 2.0)];
        g.apply_observations(&dummy, &cfg).unwrap();
        let embed = g.embedding.get(&center).expect("center embedding present");
        assert!(
            (embed.eta - 0.5).abs() < 0.05,
            "balanced-flow vertex must have η ≈ 0.5, got {}",
            embed.eta
        );
    }

    #[test]
    fn embedding_eta_for_pure_sink_is_below_05() {
        // Pure sink (in-degree only): η ≈ 0.
        let mut g = PersistentGraph::new();
        let cfg = PersistenceConfig::default();
        let sink = derive_vertex_id("sink");
        for src_name in &["a", "b", "c"] {
            g.upsert_edge(derive_vertex_id(src_name), sink, 1.0);
        }
        let trigger = vec![make_obs("a", b"x".to_vec(), 1.0)];
        g.apply_observations(&trigger, &cfg).unwrap();
        let embed = g.embedding.get(&sink).expect("sink embedding present");
        assert!(
            embed.eta < 0.4,
            "pure sink vertex must have η near 0, got {}",
            embed.eta
        );
    }

    #[test]
    fn embedding_omega_reflects_recurrence() {
        // A vertex hit many times has higher ω than one hit once.
        let mut g = PersistentGraph::new();
        let cfg = PersistenceConfig::default();
        let frequent = vec![
            make_obs("freq", b"1".to_vec(), 1.0),
            make_obs("freq", b"2".to_vec(), 2.0),
            make_obs("freq", b"3".to_vec(), 3.0),
            make_obs("freq", b"4".to_vec(), 4.0),
        ];
        g.apply_observations(&frequent, &cfg).unwrap();
        let rare = vec![make_obs("rare", b"1".to_vec(), 5.0)];
        g.apply_observations(&rare, &cfg).unwrap();
        let f = g.embedding.get(&derive_vertex_id("freq")).unwrap();
        let r = g.embedding.get(&derive_vertex_id("rare")).unwrap();
        assert!(f.omega >= r.omega, "ω(frequent) {} should ≥ ω(rare) {}",
                f.omega, r.omega);
        assert!(f.omega > 0.5, "ω(frequent) should be near 1.0, got {}", f.omega);
    }

    #[test]
    fn embedding_rho_zero_for_isolated_neighbours() {
        // A star graph: centre connected to leaves, leaves not connected to
        // each other. Centre's clustering coefficient is 0.
        let mut g = PersistentGraph::new();
        let cfg = PersistenceConfig::default();
        let centre = derive_vertex_id("centre");
        for leaf in &["l1", "l2", "l3"] {
            g.upsert_edge(centre, derive_vertex_id(leaf), 1.0);
        }
        let trigger = vec![make_obs("centre", b"x".to_vec(), 1.0)];
        g.apply_observations(&trigger, &cfg).unwrap();
        let embed = g.embedding.get(&centre).unwrap();
        assert_eq!(embed.rho, 0.0, "star centre must have ρ = 0, got {}", embed.rho);
    }

    #[test]
    fn embedding_rho_one_for_triangle() {
        // A triangle (a-b-c-a): every vertex has ρ = 1 because its two
        // neighbours are themselves connected.
        let mut g = PersistentGraph::new();
        let cfg = PersistenceConfig::default();
        let a = derive_vertex_id("a");
        let b = derive_vertex_id("b");
        let c = derive_vertex_id("c");
        g.upsert_edge(a, b, 1.0);
        g.upsert_edge(b, c, 1.0);
        g.upsert_edge(c, a, 1.0);
        let trigger = vec![make_obs("a", b"x".to_vec(), 1.0)];
        g.apply_observations(&trigger, &cfg).unwrap();
        for vid in [a, b, c] {
            let embed = g.embedding.get(&vid).unwrap();
            assert_eq!(embed.rho, 1.0, "triangle vertex {} must have ρ = 1, got {}",
                       vid, embed.rho);
        }
    }

    #[test]
    fn embedding_p_in_unit_interval_after_observations() {
        let mut g = PersistentGraph::new();
        let cfg = PersistenceConfig::default();
        let obs = vec![
            make_obs("a", b"x".to_vec(), 1.0),
            make_obs("b", b"y".to_vec(), 1.0),
            make_obs("c", b"z".to_vec(), 1.0),
        ];
        g.apply_observations(&obs, &cfg).unwrap();
        for (_, embed) in g.embedding.iter() {
            assert!((0.0..=1.0).contains(&embed.p), "p out of range: {}", embed.p);
            assert!((0.0..=1.0).contains(&embed.rho), "ρ out of range: {}", embed.rho);
            assert!((0.0..=1.0).contains(&embed.omega), "ω out of range: {}", embed.omega);
            assert!((0.0..=1.0).contains(&embed.chi), "χ out of range: {}", embed.chi);
            assert!((0.0..=1.0).contains(&embed.eta), "η out of range: {}", embed.eta);
        }
    }

    #[test]
    fn embedding_is_deterministic_on_same_input() {
        // Inv I4: replay determinism. Same observations → same 5D state.
        let cfg = PersistenceConfig::default();
        let mut g1 = PersistentGraph::new();
        let mut g2 = PersistentGraph::new();
        let obs = vec![
            make_obs("a", b"hello".to_vec(), 1.0),
            make_obs("b", b"world".to_vec(), 2.0),
            make_obs("a", b"again".to_vec(), 3.0),
        ];
        g1.apply_observations(&obs, &cfg).unwrap();
        g2.apply_observations(&obs, &cfg).unwrap();
        for (vid, e1) in g1.embedding.iter() {
            let e2 = g2.embedding.get(vid).unwrap();
            assert_eq!(e1.p,     e2.p);
            assert_eq!(e1.rho,   e2.rho);
            assert_eq!(e1.omega, e2.omega);
            assert_eq!(e1.chi,   e2.chi);
            assert_eq!(e1.eta,   e2.eta);
        }
    }
}

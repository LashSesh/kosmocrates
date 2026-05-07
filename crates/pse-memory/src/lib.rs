//! # pse-memory — Persistent Pattern Memory
//!
//! Loads crystals from prior sessions, builds a topological similarity index,
//! and provides fast lookup for incoming patterns against known crystals.
//! This is what makes PSE learn across sessions.
//!
//! The memory index uses spectral/topological features extracted from each
//! crystal's `TopologySignature` and computes cosine similarity weighted
//! with resonance and confidence proximity.

use std::collections::BTreeMap;

use pse_types::{Hash256, SemanticCrystal};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A crystal's topological fingerprint for similarity comparison.
/// Extracted from the crystal's topology signature, stability score,
/// and consensus result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CrystalSignature {
    /// Crystal ID this signature belongs to.
    pub crystal_id: Hash256,
    /// Spectral signature: topological features of the crystal.
    /// Components: [spectral_gap, cheeger_estimate, kuramoto_coherence,
    ///              mean_propagation_time, betti_0, betti_1, betti_2, euler_char]
    pub spectral: Vec<f64>,
    /// Resonance at time of crystallization (stability_score).
    pub resonance: f64,
    /// Confidence at time of crystallization (consensus MCI).
    pub confidence: f64,
    /// Content hash for exact-match dedup.
    pub content_hash: [u8; 32],
    /// Tick when this crystal was created.
    pub tick_range: (u64, u64),
    /// Number of vertices in the crystal's region.
    pub observation_count: usize,
}

/// Configuration for pattern memory.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Similarity threshold for "known pattern" (0.0–1.0).
    /// Below this: pattern is considered novel → run full cascade.
    /// Above this: pattern is known → skip cascade, increment hit counter.
    pub similarity_threshold: f64,
    /// Maximum signatures to keep in memory (LRU eviction if exceeded).
    pub max_signatures: usize,
    /// Number of spectral components to use for comparison.
    pub spectral_k: usize,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.85,
            max_signatures: 100_000,
            spectral_k: 8,
        }
    }
}

/// Statistics for pattern memory performance.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MemoryStats {
    /// Total lookups performed.
    pub total_lookups: u64,
    /// Lookups that found a similar known crystal (hit).
    pub hits: u64,
    /// Lookups that found no similar crystal (miss → full cascade).
    pub misses: u64,
    /// Hit rate = hits / total_lookups.
    pub hit_rate: f64,
    /// Estimated time saved (hits × average_cascade_time_ms).
    pub estimated_time_saved_ms: f64,
    /// Number of signatures currently in index.
    pub index_size: usize,
    /// Strand O.4: hits resolved by canonical-class identity (Metatron
    /// canonical_hash match) — proof of graph isomorphism, not just
    /// similarity. Counted alongside `hits`; this field reports the
    /// subset of hits that came from the canonical index rather than
    /// from cosine similarity.
    #[serde(default)]
    pub canonical_hits: u64,
    /// Number of distinct canonical classes currently in the canonical
    /// index.
    #[serde(default)]
    pub canonical_index_size: usize,
}

/// The pattern memory index.
///
/// Built from persisted crystals on startup.
/// Updated incrementally as new crystals are created during a session.
pub struct PatternMemory {
    /// Topological signatures of known crystals, indexed for fast lookup
    /// (cosine-similarity index — the legacy soft-matching channel).
    index: Vec<CrystalSignature>,
    /// Strand O.4: Metatron canonical-hash → list of crystal_ids that
    /// share that canonical class. A hit here is **proof of graph
    /// isomorphism**, not just similarity, so it bypasses the
    /// expensive cosine sweep entirely.
    canonical_index: BTreeMap<String, Vec<Hash256>>,
    /// Strand O.4: optional periodic-table reference. When loaded
    /// (via [`PatternMemory::load_periodic_table`]), every canonical
    /// hit can be enriched with the corresponding catalog entry —
    /// what mendelian *element* this graph belongs to. Optional
    /// because building the table for n ≤ 7 is fast (~1 s) and for
    /// n = 8 is heavy (12 K graphs).
    catalog: Option<pse_metatron::PeriodicTable>,
    /// Configuration.
    config: MemoryConfig,
    /// Statistics.
    stats: MemoryStats,
    /// Estimated average cascade time in ms (for time-saved estimation).
    avg_cascade_ms: f64,
}

impl PatternMemory {
    /// Create empty memory with configuration.
    pub fn new(config: MemoryConfig) -> Self {
        Self {
            index: Vec::new(),
            canonical_index: BTreeMap::new(),
            catalog: None,
            config,
            stats: MemoryStats::default(),
            avg_cascade_ms: 1.0,
        }
    }

    /// Load crystals and build index.
    /// Called once on engine startup to restore cross-session memory.
    /// Returns number of signatures loaded.
    pub fn load_from_crystals(&mut self, crystals: &[SemanticCrystal]) -> usize {
        let mut loaded = 0;
        for crystal in crystals {
            let sig = Self::extract_signature(crystal);
            self.index.push(sig);
            loaded += 1;
            // Respect max_signatures during load
            if self.index.len() > self.config.max_signatures {
                self.index.remove(0); // LRU: remove oldest
            }
            // Strand O.4: also populate the canonical index from any
            // crystal that carries a Metatron signature.
            if let Some(metatron) = &crystal.metatron_signature {
                if !metatron.canonical_hash.is_empty() {
                    self.canonical_index
                        .entry(metatron.canonical_hash.clone())
                        .or_insert_with(Vec::new)
                        .push(crystal.crystal_id);
                }
            }
        }
        self.stats.index_size = self.index.len();
        self.stats.canonical_index_size = self.canonical_index.len();
        loaded
    }

    /// Check if a candidate pattern is similar to any known crystal.
    /// Returns `Some(crystal_id)` if similar crystal found (hit), `None` if novel (miss).
    /// This is the hot path — optimized for fast scanning.
    pub fn lookup(&mut self, candidate: &CrystalSignature) -> Option<Hash256> {
        self.stats.total_lookups += 1;

        let spectral_k = self.config.spectral_k;
        let threshold = self.config.similarity_threshold;

        let mut best_id: Option<Hash256> = None;
        let mut best_sim: f64 = 0.0;

        for sig in &self.index {
            let sim = similarity(candidate, sig, spectral_k);
            if sim >= 1.0 {
                // Exact match — return immediately
                let id = sig.crystal_id;
                self.stats.hits += 1;
                self.update_hit_rate();
                return Some(id);
            }
            if sim > best_sim {
                best_sim = sim;
                best_id = Some(sig.crystal_id);
            }
        }

        if best_sim >= threshold {
            self.stats.hits += 1;
            self.stats.estimated_time_saved_ms += self.avg_cascade_ms;
            self.update_hit_rate();
            best_id
        } else {
            self.stats.misses += 1;
            self.update_hit_rate();
            None
        }
    }

    /// Return the top-K crystals by similarity to a candidate, without
    /// updating hit/miss statistics.
    ///
    /// Used by the resonance-fingerprint query interface (E.6) where
    /// the operation is *introspection*, not a "decide whether to skip
    /// the cascade" call. The hot-path `lookup()` semantics — exact
    /// match short-circuit, hit/miss accounting — are deliberately
    /// absent here so that asking the engine "what does this look like?"
    /// does not perturb the cross-session learning statistics.
    ///
    /// Results are sorted by descending similarity. K=0 yields an empty
    /// vector. K larger than the index simply returns all entries.
    pub fn top_k(&self, candidate: &CrystalSignature, k: usize) -> Vec<(Hash256, f64)> {
        if k == 0 || self.index.is_empty() {
            return Vec::new();
        }
        let spectral_k = self.config.spectral_k;
        let mut scored: Vec<(Hash256, f64)> = self
            .index
            .iter()
            .map(|sig| (sig.crystal_id, similarity(candidate, sig, spectral_k)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(k);
        scored
    }

    /// Borrow a stored signature by its crystal id (for deviation
    /// computation against a top-K hit). O(n) over the index because
    /// the index is small relative to the cost of the surrounding
    /// query path.
    pub fn signature_for(&self, crystal_id: &Hash256) -> Option<&CrystalSignature> {
        self.index.iter().find(|s| &s.crystal_id == crystal_id)
    }

    /// Add a newly created crystal to the index.
    /// Called after successful cascade validation + crystallization.
    pub fn insert(&mut self, signature: CrystalSignature) {
        self.index.push(signature);
        // LRU eviction if over capacity
        if self.index.len() > self.config.max_signatures {
            self.index.remove(0);
        }
        self.stats.index_size = self.index.len();
    }

    /// Get current statistics.
    pub fn stats(&self) -> &MemoryStats {
        &self.stats
    }

    // ── O.4: canonical-class lookup ──────────────────────────────────────

    /// Strand O.4: insert a crystal into both the cosine-similarity
    /// index AND the canonical-class index (when the crystal carries a
    /// Metatron signature). The right call from macro_step in place of
    /// the legacy `insert(extract_signature(crystal))` chain.
    pub fn insert_crystal(&mut self, crystal: &SemanticCrystal) {
        let sig = Self::extract_signature(crystal);
        self.insert(sig);
        if let Some(metatron) = &crystal.metatron_signature {
            if !metatron.canonical_hash.is_empty() {
                self.canonical_index
                    .entry(metatron.canonical_hash.clone())
                    .or_insert_with(Vec::new)
                    .push(crystal.crystal_id);
                self.stats.canonical_index_size = self.canonical_index.len();
            }
        }
    }

    /// Strand O.4: enhanced lookup that prefers canonical-class
    /// identity over cosine similarity.
    ///
    /// Logic:
    ///  1. If the crystal carries a Metatron signature AND the
    ///     canonical-hash index has at least one stored crystal with
    ///     the same hash → **canonical hit**, return one of those
    ///     crystal ids. This is graph-isomorphism by proof.
    ///  2. Otherwise fall back to the cosine-similarity index — the
    ///     legacy soft-matching path. The fallback is essential because
    ///     crystals whose region exceeds METATRON_REGION_CAP carry no
    ///     canonical signature.
    ///
    /// Updates `stats.canonical_hits` on canonical matches; the broader
    /// `stats.hits/misses/total_lookups` remain consistent with the
    /// legacy `lookup` method.
    pub fn lookup_crystal(&mut self, crystal: &SemanticCrystal) -> Option<Hash256> {
        // Resolve the canonical hit first, releasing the borrow before
        // touching mutable stats.
        let canonical_hit: Option<Hash256> = crystal
            .metatron_signature
            .as_ref()
            .filter(|m| !m.canonical_hash.is_empty())
            .and_then(|m| self.canonical_index.get(&m.canonical_hash))
            .and_then(|stored| stored.first().copied());

        if let Some(id) = canonical_hit {
            self.stats.total_lookups += 1;
            self.stats.hits += 1;
            self.stats.canonical_hits += 1;
            self.stats.estimated_time_saved_ms += self.avg_cascade_ms;
            self.update_hit_rate();
            return Some(id);
        }

        let sig = Self::extract_signature(crystal);
        self.lookup(&sig)
    }

    /// Strand O.4: load the Periodic-Table-of-Graphs catalog up to the
    /// given vertex count. The catalog is *informational* — it lets
    /// canonical-hash hits be enriched with their mendelian class
    /// label (e.g. "G7-11-107"). Lookup itself does not require the
    /// catalog; the canonical_index alone is sufficient for proof-of-
    /// isomorphism matching.
    ///
    /// Cost: building for n ≤ 7 takes ~1 s and produces 1044 entries;
    /// n = 8 takes ~30 s and produces 12346 entries. PSE crystals
    /// typically have small regions (≤ 7 vertices), so n_max = 7 is
    /// the recommended call.
    pub fn load_periodic_table(&mut self, n_max: u8) -> Result<(), String> {
        let table = pse_metatron::build_catalog(n_max as usize)
            .map_err(|e| format!("build_catalog failed: {}", e))?;
        self.catalog = Some(table);
        Ok(())
    }

    /// Strand O.4: number of distinct canonical classes currently
    /// indexed. Diagnostic counterpart to `stats.canonical_index_size`.
    pub fn canonical_classes_known(&self) -> usize {
        self.canonical_index.len()
    }

    /// Strand O.4: borrow the loaded periodic table, if any.
    pub fn periodic_table(&self) -> Option<&pse_metatron::PeriodicTable> {
        self.catalog.as_ref()
    }

    /// Number of signatures in the index.
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Extract a `CrystalSignature` from a `SemanticCrystal`.
    /// Uses the crystal's topological features to create the fingerprint.
    pub fn extract_signature(crystal: &SemanticCrystal) -> CrystalSignature {
        let ts = &crystal.topology_signature;

        // Build spectral vector from topology signature fields
        let spectral = vec![
            ts.spectral_gap,
            ts.cheeger_estimate,
            ts.kuramoto_coherence,
            ts.mean_propagation_time,
            ts.betti_0 as f64,
            ts.betti_1 as f64,
            ts.betti_2 as f64,
            ts.euler_char as f64,
        ];

        // Confidence from consensus MCI
        let confidence = crystal.commit_proof.consensus_result.mci;

        // Content hash: SHA-256 of canonical crystal bytes
        let content_hash = compute_content_hash(crystal);

        CrystalSignature {
            crystal_id: crystal.crystal_id,
            spectral,
            resonance: crystal.stability_score,
            confidence,
            content_hash,
            tick_range: (crystal.created_at, crystal.created_at),
            observation_count: crystal.region.len(),
        }
    }

    /// Extract a signature from pre-crystal data (metrics + topology).
    /// Used for lookup before the cascade runs.
    pub fn extract_candidate_signature(
        spectral_gap: f64,
        cheeger_estimate: f64,
        kuramoto_coherence: f64,
        mean_propagation_time: f64,
        betti_0: u64,
        betti_1: u64,
        betti_2: u64,
        euler_char: i64,
        stability_score: f64,
        region_size: usize,
    ) -> CrystalSignature {
        let spectral = vec![
            spectral_gap,
            cheeger_estimate,
            kuramoto_coherence,
            mean_propagation_time,
            betti_0 as f64,
            betti_1 as f64,
            betti_2 as f64,
            euler_char as f64,
        ];

        CrystalSignature {
            crystal_id: [0u8; 32], // Unknown until crystallized
            spectral,
            resonance: stability_score,
            confidence: 0.0,         // Unknown before consensus
            content_hash: [0u8; 32], // Unknown before crystallization
            tick_range: (0, 0),
            observation_count: region_size,
        }
    }

    /// Update the hit rate statistic.
    fn update_hit_rate(&mut self) {
        if self.stats.total_lookups > 0 {
            self.stats.hit_rate = self.stats.hits as f64 / self.stats.total_lookups as f64;
        }
    }
}

/// Compute similarity between two crystal signatures.
/// Uses cosine similarity on spectral components + resonance proximity.
///
/// Returns value in [0.0, 1.0] where 1.0 = identical.
pub fn similarity(a: &CrystalSignature, b: &CrystalSignature, spectral_k: usize) -> f64 {
    // 1. Exact content match (hash comparison) → return 1.0 immediately
    if a.content_hash != [0u8; 32]
        && b.content_hash != [0u8; 32]
        && a.content_hash == b.content_hash
    {
        return 1.0;
    }

    // 2. Spectral cosine similarity (70% weight)
    let spectral_sim = cosine_similarity(&a.spectral, &b.spectral, spectral_k);

    // 3. Resonance proximity (15% weight)
    let resonance_sim = 1.0 - (a.resonance - b.resonance).abs().min(1.0);

    // 4. Confidence proximity (15% weight)
    let confidence_sim = 1.0 - (a.confidence - b.confidence).abs().min(1.0);

    0.70 * spectral_sim + 0.15 * resonance_sim + 0.15 * confidence_sim
}

/// Cosine similarity between two vectors, using at most `k` components.
/// Returns 0.0 if either vector is zero.
fn cosine_similarity(a: &[f64], b: &[f64], k: usize) -> f64 {
    let len = a.len().min(b.len()).min(k);
    if len == 0 {
        return 0.0;
    }

    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for i in 0..len {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom < 1e-12 {
        return 0.0;
    }

    // Clamp to [0, 1] (cosine can be negative for opposing vectors)
    (dot / denom).max(0.0).min(1.0)
}

/// Compute SHA-256 content hash of a crystal.
fn compute_content_hash(crystal: &SemanticCrystal) -> [u8; 32] {
    let mut hasher = Sha256::new();
    // Hash the crystal_id + stability_score + region size for a stable fingerprint
    hasher.update(crystal.crystal_id);
    hasher.update(crystal.stability_score.to_le_bytes());
    hasher.update((crystal.region.len() as u64).to_le_bytes());
    hasher.update(crystal.free_energy.to_le_bytes());
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sig(spectral: Vec<f64>, resonance: f64, confidence: f64) -> CrystalSignature {
        CrystalSignature {
            crystal_id: [1u8; 32],
            spectral,
            resonance,
            confidence,
            content_hash: [0u8; 32],
            tick_range: (0, 0),
            observation_count: 10,
        }
    }

    fn make_sig_with_hash(spectral: Vec<f64>, hash: [u8; 32]) -> CrystalSignature {
        CrystalSignature {
            crystal_id: [1u8; 32],
            spectral,
            resonance: 0.5,
            confidence: 0.5,
            content_hash: hash,
            tick_range: (0, 0),
            observation_count: 10,
        }
    }

    #[test]
    fn empty_memory_returns_none() {
        let mut mem = PatternMemory::new(MemoryConfig::default());
        let sig = make_sig(vec![1.0, 2.0, 3.0], 0.5, 0.5);
        assert!(mem.lookup(&sig).is_none());
        assert_eq!(mem.stats().misses, 1);
    }

    #[test]
    fn insert_and_lookup_returns_crystal_id() {
        let mut mem = PatternMemory::new(MemoryConfig::default());
        let mut sig = make_sig(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], 0.5, 0.5);
        sig.crystal_id = [42u8; 32];
        mem.insert(sig.clone());

        let result = mem.lookup(&sig);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), [42u8; 32]);
        assert_eq!(mem.stats().hits, 1);
    }

    #[test]
    fn similar_patterns_match_above_threshold() {
        let mut mem = PatternMemory::new(MemoryConfig {
            similarity_threshold: 0.85,
            ..Default::default()
        });
        let mut stored = make_sig(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], 0.5, 0.5);
        stored.crystal_id = [42u8; 32];
        mem.insert(stored);

        // Very similar but not identical
        let candidate = make_sig(
            vec![1.01, 2.01, 3.01, 4.01, 5.01, 6.01, 7.01, 8.01],
            0.51,
            0.51,
        );
        assert!(mem.lookup(&candidate).is_some());
    }

    #[test]
    fn dissimilar_patterns_dont_match() {
        let mut mem = PatternMemory::new(MemoryConfig {
            similarity_threshold: 0.85,
            ..Default::default()
        });
        let stored = make_sig(vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], 0.9, 0.9);
        mem.insert(stored);

        // Very different spectral signature
        let candidate = make_sig(vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0], 0.1, 0.1);
        assert!(mem.lookup(&candidate).is_none());
    }

    #[test]
    fn exact_hash_match_always_returns_hit() {
        let mut mem = PatternMemory::new(MemoryConfig {
            similarity_threshold: 0.99, // Very high threshold
            ..Default::default()
        });
        let hash = [99u8; 32];
        let mut stored = make_sig_with_hash(vec![1.0, 2.0, 3.0], hash);
        stored.crystal_id = [42u8; 32];
        mem.insert(stored);

        // Different spectral but same hash → still matches
        let candidate = make_sig_with_hash(vec![100.0, 200.0, 300.0], hash);
        assert!(mem.lookup(&candidate).is_some());
    }

    #[test]
    fn stats_tracking() {
        let mut mem = PatternMemory::new(MemoryConfig::default());
        let sig = make_sig(vec![1.0, 2.0, 3.0], 0.5, 0.5);

        // Three misses
        mem.lookup(&sig);
        mem.lookup(&sig);
        mem.lookup(&sig);

        assert_eq!(mem.stats().total_lookups, 3);
        assert_eq!(mem.stats().misses, 3);
        assert_eq!(mem.stats().hits, 0);
        assert!((mem.stats().hit_rate - 0.0).abs() < 1e-9);

        // Insert and hit
        let mut stored = sig.clone();
        stored.crystal_id = [1u8; 32];
        mem.insert(stored);
        mem.lookup(&sig);

        assert_eq!(mem.stats().total_lookups, 4);
        assert_eq!(mem.stats().hits, 1);
        assert!((mem.stats().hit_rate - 0.25).abs() < 1e-9);
    }

    #[test]
    fn lru_eviction() {
        let mut mem = PatternMemory::new(MemoryConfig {
            max_signatures: 3,
            ..Default::default()
        });

        for i in 0..5u8 {
            let mut sig = make_sig(vec![i as f64], 0.5, 0.5);
            sig.crystal_id = [i; 32];
            mem.insert(sig);
        }

        // Should have evicted oldest entries, keeping only last 3
        assert_eq!(mem.len(), 3);
        assert_eq!(mem.stats().index_size, 3);
    }

    #[test]
    fn cosine_similarity_identical_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &b, 8);
        assert!((sim - 1.0).abs() < 1e-9);
    }

    #[test]
    fn cosine_similarity_orthogonal_vectors() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b, 8);
        assert!(sim.abs() < 1e-9);
    }

    #[test]
    fn cosine_similarity_zero_vector() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &b, 8);
        assert!(sim.abs() < 1e-9);
    }

    // ── E.6: introspective queries ──────────────────────────────────────

    fn sig_with_id(spectral: Vec<f64>, id_byte: u8) -> CrystalSignature {
        CrystalSignature {
            crystal_id: [id_byte; 32],
            spectral,
            resonance: 0.5,
            confidence: 0.5,
            content_hash: [0u8; 32],
            tick_range: (0, 0),
            observation_count: 10,
        }
    }

    #[test]
    fn top_k_returns_highest_similarity_first() {
        let mut mem = PatternMemory::new(MemoryConfig::default());
        // a near-identical, b moderate-overlap, c orthogonal
        mem.insert(sig_with_id(
            vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            0xAA,
        ));
        mem.insert(sig_with_id(
            vec![0.5, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            0xBB,
        ));
        mem.insert(sig_with_id(
            vec![0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            0xCC,
        ));
        let candidate = sig_with_id(vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], 0x00);
        let top = mem.top_k(&candidate, 3);
        assert_eq!(top.len(), 3);
        // First entry must be the (near-)identical one (id 0xAA).
        assert_eq!(top[0].0, [0xAA; 32]);
        // Scores must be non-increasing.
        for w in top.windows(2) {
            assert!(w[0].1 >= w[1].1);
        }
    }

    #[test]
    fn top_k_does_not_update_stats() {
        let mut mem = PatternMemory::new(MemoryConfig::default());
        mem.insert(sig_with_id(
            vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            0xAA,
        ));
        let stats_before = mem.stats().clone();
        let candidate = sig_with_id(vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], 0x00);
        let _ = mem.top_k(&candidate, 1);
        let stats_after = mem.stats().clone();
        assert_eq!(stats_before.total_lookups, stats_after.total_lookups);
        assert_eq!(stats_before.hits, stats_after.hits);
        assert_eq!(stats_before.misses, stats_after.misses);
    }

    #[test]
    fn top_k_handles_k_larger_than_index() {
        let mut mem = PatternMemory::new(MemoryConfig::default());
        mem.insert(sig_with_id(
            vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            0xAA,
        ));
        mem.insert(sig_with_id(
            vec![0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            0xBB,
        ));
        let candidate = sig_with_id(vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], 0x00);
        let top = mem.top_k(&candidate, 100);
        assert_eq!(top.len(), 2);
    }

    #[test]
    fn top_k_zero_returns_empty() {
        let mut mem = PatternMemory::new(MemoryConfig::default());
        mem.insert(sig_with_id(
            vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            0xAA,
        ));
        let candidate = sig_with_id(vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], 0x00);
        assert!(mem.top_k(&candidate, 0).is_empty());
    }

    #[test]
    fn signature_for_returns_stored_signature() {
        let mut mem = PatternMemory::new(MemoryConfig::default());
        let sig = sig_with_id(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], 0x42);
        mem.insert(sig.clone());
        let fetched = mem.signature_for(&sig.crystal_id).unwrap();
        assert_eq!(fetched.spectral, sig.spectral);
    }

    #[test]
    fn signature_for_unknown_returns_none() {
        let mem = PatternMemory::new(MemoryConfig::default());
        assert!(mem.signature_for(&[0x99; 32]).is_none());
    }

    // ── O.4: canonical-class lookup ──────────────────────────────────────

    fn make_crystal_with_metatron(
        crystal_id_byte: u8,
        canonical_hash: &str,
        spectral: Vec<f64>,
    ) -> SemanticCrystal {
        SemanticCrystal {
            crystal_id: [crystal_id_byte; 32],
            region: vec![1, 2, 3],
            constraint_program: Vec::new(),
            stability_score: 0.5,
            topology_signature: pse_types::TopologySignature {
                spectral_gap: spectral.first().copied().unwrap_or(0.0),
                ..Default::default()
            },
            betti_numbers: vec![1, 0, 0],
            evidence_chain: Vec::new(),
            commit_proof: pse_types::CommitProof::default(),
            operator_versions: BTreeMap::new(),
            created_at: 1,
            free_energy: -0.5,
            carrier_instance_idx: 0,
            scale_tag: "test".into(),
            universe_id: String::new(),
            sub_crystal_ids: Vec::new(),
            parent_crystal_ids: Vec::new(),
            genesis_metadata: None,
            metatron_signature: Some(pse_types::MetatronTopologySignature {
                canonical_hash: canonical_hash.to_string(),
                n: 3,
                m: 3,
                orbit_size: 0,
                stabilizer_order: 0,
                spectrum: spectral,
                spectral_radius: 0.0,
                algebraic_connectivity: 0.0,
                graph_energy: 0.0,
                spanning_tree_count: 0,
                triangle_count: 1,
                contains_tetrahedron: false,
                contains_octahedron: false,
                contains_cube: false,
            }),
        }
    }

    #[test]
    fn insert_crystal_populates_canonical_index() {
        let mut mem = PatternMemory::new(MemoryConfig::default());
        let c = make_crystal_with_metatron(0xA1, "deadbeef0000cafe", vec![1.0, 0.0]);
        mem.insert_crystal(&c);
        assert_eq!(mem.canonical_classes_known(), 1);
        assert_eq!(mem.stats().canonical_index_size, 1);
    }

    #[test]
    fn lookup_crystal_canonical_hit_returns_stored_id() {
        let mut mem = PatternMemory::new(MemoryConfig::default());
        let stored = make_crystal_with_metatron(0xA1, "matching_hash", vec![1.0, 0.0]);
        mem.insert_crystal(&stored);

        // A new crystal with the SAME canonical_hash but a different
        // crystal_id and a different cosine-spectral vector should
        // hit on canonical, not on cosine.
        let probe = make_crystal_with_metatron(0xB2, "matching_hash", vec![0.0, 1.0]);
        let hit = mem.lookup_crystal(&probe);
        assert_eq!(hit, Some([0xA1; 32]));
        assert_eq!(mem.stats().canonical_hits, 1);
        assert_eq!(mem.stats().hits, 1);
    }

    #[test]
    fn lookup_crystal_falls_back_to_cosine_when_no_canonical_hash() {
        let mut mem = PatternMemory::new(MemoryConfig::default());
        // Stored with metatron, probe without — fallback path triggers.
        let mut stored =
            make_crystal_with_metatron(0xA1, "h1", vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        // Make stored's spectral distinctive enough that cosine matches.
        stored.topology_signature.spectral_gap = 1.0;
        mem.insert_crystal(&stored);

        let mut probe = stored.clone();
        probe.crystal_id = [0xB2; 32];
        probe.metatron_signature = None; // strip canonical signature
        let hit = mem.lookup_crystal(&probe);
        // Fallback to cosine — the spectra match, so a hit is expected.
        assert!(hit.is_some());
        // No canonical hit should be recorded.
        assert_eq!(mem.stats().canonical_hits, 0);
    }

    #[test]
    fn lookup_crystal_no_match_returns_none() {
        let mut mem = PatternMemory::new(MemoryConfig::default());
        let probe = make_crystal_with_metatron(0xCC, "nonexistent_hash", vec![1.0]);
        let hit = mem.lookup_crystal(&probe);
        assert_eq!(hit, None);
    }

    #[test]
    fn canonical_index_supports_multiple_crystals_per_class() {
        let mut mem = PatternMemory::new(MemoryConfig::default());
        let c1 = make_crystal_with_metatron(0xA1, "shared", vec![1.0]);
        let c2 = make_crystal_with_metatron(0xA2, "shared", vec![1.0]);
        let c3 = make_crystal_with_metatron(0xB1, "different", vec![0.0]);
        mem.insert_crystal(&c1);
        mem.insert_crystal(&c2);
        mem.insert_crystal(&c3);
        assert_eq!(mem.canonical_classes_known(), 2);
    }

    #[test]
    fn load_periodic_table_populates_catalog() {
        let mut mem = PatternMemory::new(MemoryConfig::default());
        assert!(mem.periodic_table().is_none());
        // n_max = 4 is fast (~12 graphs) and exercises the load path.
        let res = mem.load_periodic_table(4);
        assert!(res.is_ok());
        assert!(mem.periodic_table().is_some());
    }

    #[test]
    fn load_from_crystals_populates_canonical_index() {
        // Mirror behaviour for the cross-session loading path.
        let crystals = vec![
            make_crystal_with_metatron(0x01, "first", vec![1.0]),
            make_crystal_with_metatron(0x02, "second", vec![2.0]),
            make_crystal_with_metatron(0x03, "first", vec![1.0]), // shares class
        ];
        let mut mem = PatternMemory::new(MemoryConfig::default());
        let loaded = mem.load_from_crystals(&crystals);
        assert_eq!(loaded, 3);
        assert_eq!(mem.canonical_classes_known(), 2);
    }
}

//! File-based IL ledger + cosine-similarity nearest-neighbour search.
//!
//! `ILStore` persists IL-compatible ledger blocks on disk and maintains an
//! in-memory + on-disk index of 8D vectors for retrieval.  No running IL
//! server is required; the on-disk block format matches IL's `MefBlock`
//! JSON layout so blocks can be replayed by IL verbatim.
//!
//! When the `hdag` feature is enabled, every committed crystal is also
//! registered as a node in an `HDAG` (Hyperdimensional Directed Acyclic
//! Graph) that couples linear commit-time with spiral phase, and edges are
//! drawn from the previous node with weight = cosine-similarity of the two
//! 8D vectors.
//!
//! When the `il-pipeline` feature is enabled, `ILStore::open_with_pipeline`
//! initialises an internal `MEFCore` so that TIC generation and the 8D
//! vector are produced by the authoritative IL engine rather than by the
//! PSE-side approximation.

use crate::adapter::{CrystalAdapter, ILPayload};
use crate::feedback::ValidationFeedback;
use crate::hdag::{crystal_to_tensor, HDAG};
use pse_types::SemanticCrystal;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[cfg(feature = "il-pipeline")]
use mef_core::MEFCore;

/// Internal result from `commit_payload` carrying all data needed for feedback.
struct CommitResult {
    block_hash: String,
    hdag_node_id: Option<String>,
    coherence_potential: f64,
    gate_passed: bool,
}

/// A search hit returned by `ILStore::search`.
#[derive(Debug, Clone)]
pub struct ILMatch {
    pub crystal_id_hex: String,
    pub score: f64,
}

/// On-disk index entry (one per committed crystal).
#[derive(Serialize, Deserialize, Clone)]
struct IndexEntry {
    block_index: i64,
    crystal_id_hex: String,
    tic_id: String,
    block_hash: String,
    block_file: String,
    vector8: Vec<f64>,
    /// Spiral phase derived from the fixpoint; used as HDAG node phase.
    phase: f64,
    /// HDAG node ID for this entry (always populated).
    #[serde(default)]
    hdag_node_id: Option<String>,
    /// PSE stability score at commit time; ρ in the Pfauenthron++ retrieval formula.
    #[serde(default = "default_stability")]
    stability_score: f64,
}

fn default_stability() -> f64 {
    0.5
}

/// On-disk index file.
#[derive(Serialize, Deserialize, Default)]
struct ILIndex {
    entries: Vec<IndexEntry>,
}

/// File-based IL store: ledger blocks + cosine-similarity nearest-neighbour.
pub struct ILStore {
    ledger_path: PathBuf,
    index_path: PathBuf,
    index: ILIndex,
    adapter: CrystalAdapter,
    genesis_hash: String,
    base_path: PathBuf,
    hdag: Option<HDAG>,

    #[cfg(feature = "il-pipeline")]
    mef_core: Option<MEFCore>,
}

impl ILStore {
    /// Open or create an `ILStore` at `base_path`.
    /// `seed` is forwarded to `CrystalAdapter` (and MEFCore when active).
    pub fn open(base_path: impl AsRef<Path>, seed: &str) -> Result<Self, String> {
        Self::open_inner(base_path, seed, false)
    }

    /// Open or create an `ILStore` that drives `MEFCore::process()` internally.
    ///
    /// MEFCore's tic-store and ledger directories are created as subdirectories
    /// of `base_path` so everything is co-located.  Only available when the
    /// `il-pipeline` feature is compiled in.
    #[cfg(feature = "il-pipeline")]
    pub fn open_with_pipeline(base_path: impl AsRef<Path>, seed: &str) -> Result<Self, String> {
        Self::open_inner(base_path, seed, true)
    }

    fn open_inner(
        base_path: impl AsRef<Path>,
        seed: &str,
        #[allow(unused_variables)] with_pipeline: bool,
    ) -> Result<Self, String> {
        let base = base_path.as_ref().to_path_buf();
        std::fs::create_dir_all(&base)
            .map_err(|e| format!("cannot create IL store at {:?}: {e}", base))?;

        let ledger_path = base.join("ledger");
        std::fs::create_dir_all(&ledger_path)
            .map_err(|e| format!("cannot create ledger dir: {e}"))?;

        let index_path = base.join("il_index.json");
        let index = if index_path.exists() {
            let s = std::fs::read_to_string(&index_path)
                .map_err(|e| format!("cannot read IL index: {e}"))?;
            serde_json::from_str(&s).unwrap_or_default()
        } else {
            ILIndex::default()
        };

        let hdag = {
            let hdag_path = base.join("hdag");
            match HDAG::new(&hdag_path) {
                Ok(h) => Some(h),
                Err(e) => {
                    eprintln!("[IL] HDAG init warning: {e}");
                    None
                }
            }
        };

        #[cfg(feature = "il-pipeline")]
        let mef_core = if with_pipeline {
            let base_str = base
                .to_str()
                .ok_or("non-UTF8 base path")?
                .to_string();
            match crate::adapter::pipeline::make_mef_core(&base_str, seed) {
                Ok(m) => Some(m),
                Err(e) => {
                    eprintln!("[IL] MEFCore init warning: {e} — falling back to PSE mapping");
                    None
                }
            }
        } else {
            None
        };

        Ok(Self {
            ledger_path,
            index_path,
            index,
            adapter: CrystalAdapter::new(seed),
            genesis_hash: "0".repeat(64),
            base_path: base,
            hdag,
            #[cfg(feature = "il-pipeline")]
            mef_core,
        })
    }

    /// Convert a crystal and commit it to the ledger + index.
    /// Returns the block hash, or the existing hash if already committed.
    pub fn commit(
        &mut self,
        crystal: &SemanticCrystal,
        source_chunks: &[String],
        session: usize,
        question: &str,
    ) -> Result<String, String> {
        self.commit_with_feedback(crystal, source_chunks, session, question)
            .map(|fb| fb.block_hash)
    }

    /// Like `commit`, but also returns IL validation data for the feedback loop.
    ///
    /// The `ValidationFeedback` contains convergence status, coherence potential,
    /// and a normalized IL stability signal ready to be blended into a refined crystal
    /// via `pse_adapter_il::feedback::refine_crystal`.
    pub fn commit_with_feedback(
        &mut self,
        crystal: &SemanticCrystal,
        source_chunks: &[String],
        session: usize,
        question: &str,
    ) -> Result<ValidationFeedback, String> {
        let payload = self.build_payload(crystal, source_chunks, session, question)?;
        let result = self.commit_payload(crystal, payload)?;

        let feedback = ValidationFeedback::from_crystal_heuristic(
            result.block_hash,
            crystal,
            result.coherence_potential,
            result.gate_passed,
            result.hdag_node_id,
        );
        Ok(feedback)
    }

    /// Build an ILPayload, using MEFCore when `il-pipeline` is active.
    fn build_payload(
        &mut self,
        crystal: &SemanticCrystal,
        source_chunks: &[String],
        session: usize,
        question: &str,
    ) -> Result<ILPayload, String> {
        // il-pipeline path: delegate to MEFCore for authoritative TIC + vector8
        #[cfg(feature = "il-pipeline")]
        if let Some(ref mut mc) = self.mef_core {
            let mut payload =
                crate::adapter::pipeline::convert_via_mef_core(crystal, source_chunks, mc)?;
            if let Some(snap) = payload.snapshot_json.as_object_mut() {
                snap.insert("session".into(), serde_json::json!(session));
                snap.insert("question".into(), serde_json::json!(question));
            }
            return Ok(payload);
        }

        // Default PSE-side mapping
        self.adapter
            .convert_with_provenance(crystal, source_chunks, session, question)
    }

    fn commit_payload(
        &mut self,
        crystal: &SemanticCrystal,
        payload: ILPayload,
    ) -> Result<CommitResult, String> {
        // Idempotent: return existing data if already committed
        if let Some(existing) = self
            .index
            .entries
            .iter()
            .find(|e| e.crystal_id_hex == payload.crystal_id_hex)
        {
            let cp = existing.phase; // phase ≈ spectral_gap; exact ψ needs HDAG
            let node_id = existing.hdag_node_id.clone();
            let gate = self.hdag.as_ref()
                .and_then(|h| h.is_in_s_coh_for(node_id.as_deref().unwrap_or("")))
                .unwrap_or(crystal.stability_score > 0.5);
            return Ok(CommitResult {
                block_hash: existing.block_hash.clone(),
                hdag_node_id: node_id,
                coherence_potential: cp,
                gate_passed: gate,
            });
        }

        let block_index = self.index.entries.len() as i64;
        let previous_hash = self
            .index
            .entries
            .last()
            .map(|e| e.block_hash.as_str())
            .unwrap_or(&self.genesis_hash)
            .to_string();

        let timestamp = simple_timestamp();

        let mut block = serde_json::json!({
            "index":         block_index,
            "previous_hash": previous_hash,
            "timestamp":     timestamp,
            "tic_id":        payload.tic_id,
            "snapshot_hash": hex_hash(&payload.snapshot_json.to_string()),
            "data":          payload.tic_json,
            "proof":         payload.tic_json.get("proof").cloned()
                                 .unwrap_or(serde_json::Value::Null),
            "hash": "",
        });

        let block_hash = compute_block_hash(&block);
        block["hash"] = serde_json::Value::String(block_hash.clone());

        let block_file_name = format!("block_{:06}.mef", block_index);
        let block_file = self.ledger_path.join(&block_file_name);
        std::fs::write(
            &block_file,
            serde_json::to_string_pretty(&block)
                .map_err(|e| format!("serialise block: {e}"))?,
        )
        .map_err(|e| format!("write block file: {e}"))?;

        // ── HDAG: register 5D resonance tensor node ──────────────────────────
        let hdag_node_id = self.register_hdag_node(crystal, &payload);

        // ── HDAG: add semantic edges to all resonance-proximate predecessors ──
        if let Some(ref nid) = hdag_node_id {
            let n = self.add_semantic_edges(nid);
            if n > 0 {
                // Trace-level: only log when semantic edges were actually added
                // (avoids noise in single-crystal stores)
                let _ = n;
            }
        }

        // Compute coherence potential from the actual tensor for feedback
        let coherence_potential = {
            let sig = &crystal.topology_signature;
            sig.kuramoto_coherence - (1.0 - crystal.stability_score.clamp(0.0, 1.0))
        };
        let gate_passed = self.hdag.as_ref()
            .and_then(|h| h.is_in_s_coh_for(
                hdag_node_id.as_deref().unwrap_or(""),
            ))
            .unwrap_or(crystal.stability_score > 0.5);

        self.index.entries.push(IndexEntry {
            block_index,
            crystal_id_hex: payload.crystal_id_hex,
            tic_id: payload.tic_id,
            block_hash: block_hash.clone(),
            block_file: block_file_name,
            vector8: payload.vector8,
            phase: payload.phase,
            hdag_node_id: hdag_node_id.clone(),
            stability_score: crystal.stability_score,
        });
        self.save_index()?;

        Ok(CommitResult {
            block_hash,
            hdag_node_id,
            coherence_potential,
            gate_passed,
        })
    }

    /// Register the crystal as an HDAG node; attempt to draw a phase-gradient
    /// edge from the previous node.  Acyclicity and coherence are enforced by
    /// the HDAG itself — no explicit checks here.
    fn register_hdag_node(
        &mut self,
        crystal: &SemanticCrystal,
        payload: &ILPayload,
    ) -> Option<String> {
        let hdag = self.hdag.as_mut()?;

        let node_id = format!("N-{}", &payload.crystal_id_hex[..16]);
        let tensor = crystal_to_tensor(crystal);
        let kairos = crystal.stability_score > 0.5
            && crystal.topology_signature.kuramoto_coherence > 0.2;
        let ts = simple_timestamp();

        if let Err(e) = hdag.add_node(
            &node_id,
            &payload.crystal_id_hex,
            tensor,
            kairos,
            &ts,
        ) {
            eprintln!("[IL] HDAG add_node error: {e}");
            return None;
        }

        // Attempt phase-gradient edge from previous node.
        // HDAG enforces the coherence condition; None means gate was closed.
        if let Some(prev) = self.index.entries.last() {
            if let Some(ref prev_nid) = prev.hdag_node_id {
                match hdag.add_edge(prev_nid, &node_id, "sequential_commit") {
                    Ok(Some(edge)) => {
                        let _ = edge; // gradient available for future analytics
                    }
                    Ok(None) => {} // coherence gate closed — acyclicity enforced
                    Err(e) => eprintln!("[IL] HDAG add_edge error: {e}"),
                }
            }
        }

        // Refinement edges: wire parent crystals into the genealogy graph.
        // parent_crystal_ids holds the 64-char hex of each ancestor crystal,
        // matching the crystal_id_hex stored in IndexEntry.  These edges record
        // the IL feedback lineage explicitly in the HDAG topology.
        for parent_hex in &crystal.parent_crystal_ids {
            let parent_nid: Option<String> = self
                .index
                .entries
                .iter()
                .find(|e| &e.crystal_id_hex == parent_hex)
                .and_then(|e| e.hdag_node_id.clone());
            if let Some(ref pnid) = parent_nid {
                match hdag.add_edge(pnid, &node_id, "refinement") {
                    Ok(Some(_)) => {}
                    Ok(None) => {}
                    Err(e) => eprintln!("[IL] HDAG refinement edge error: {e}"),
                }
            }
        }

        Some(node_id)
    }

    /// Scan all existing HDAG nodes and add phase-gradient edges to `node_id`
    /// from any valid semantic predecessor (resonance_proximity edges).
    ///
    /// A node is a valid predecessor when it:
    /// - Is in S_coh (coherence gate)
    /// - Has ψ ≤ ψ(target) + ε  (coherence potential monotonicity)
    /// - Is within `resonance_radius` in 5D tensor space
    ///
    /// Capped at `max_candidates` nearest neighbors to bound runtime.
    /// Returns the number of semantic edges created.
    fn add_semantic_edges(&mut self, node_id: &str) -> usize {
        // Step 1: collect predecessors (immutable borrow)
        let predecessors: Vec<String> = if let Some(ref hdag) = self.hdag {
            hdag.find_semantic_predecessors(node_id, 20, 1.5)
        } else {
            return 0;
        };

        // Step 2: add edges (mutable borrow, one at a time)
        let mut count = 0;
        for pred_id in predecessors {
            if let Some(ref mut hdag) = self.hdag {
                match hdag.add_edge(&pred_id, node_id, "resonance_proximity") {
                    Ok(Some(_)) => count += 1,
                    Ok(None) => {}
                    Err(e) => eprintln!("[IL] HDAG semantic edge error: {e}"),
                }
            }
        }
        count
    }

    /// Cosine-similarity nearest-neighbour search over committed 8D vectors.
    /// Returns up to `top_k` matches sorted by descending score.
    pub fn search(&self, query: &[f64], top_k: usize) -> Vec<ILMatch> {
        let mut scored: Vec<(f64, &IndexEntry)> = self
            .index
            .entries
            .iter()
            .map(|e| (cosine(&e.vector8, query), e))
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored
            .into_iter()
            .take(top_k)
            .map(|(score, e)| ILMatch {
                crystal_id_hex: e.crystal_id_hex.clone(),
                score,
            })
            .collect()
    }

    /// Pfauenthron++ unified score over all committed crystals.
    ///
    /// Implements the Timeless Monolith tripolar formula D = ψ · ρ · ω:
    ///   ψ — IL semantic alignment: cosine(query_vec, crystal.vector8)
    ///   ρ — PSE structural coherence: crystal stability_score ∈ [0,1]
    ///   ω — HDAG temporal readiness: normalised coherence potential ∈ [0,1]
    ///
    /// Multiplicative form acts as a Gabriel4D Funnel: a crystal must score
    /// non-trivially on all three axes to reach the retrieval core.
    /// Entries with D = 0 are excluded.  Results sorted by descending D.
    pub fn score_tripolar(&self, query_vec: &[f64]) -> Vec<ILMatch> {
        let mut scored: Vec<ILMatch> = self
            .index
            .entries
            .iter()
            .filter_map(|entry| {
                // ψ: semantic alignment (IL 8D cosine similarity)
                let psi_sem = cosine(&entry.vector8, query_vec);
                if psi_sem <= 0.0 {
                    return None;
                }

                // ρ: structural coherence (PSE stability score)
                let rho_pse = entry.stability_score.clamp(0.0, 1.0);

                // ω: temporal readiness (HDAG coherence potential, normalised to [0,1])
                let omega_hdag: f64 = match &self.hdag {
                    Some(hdag) => match &entry.hdag_node_id {
                        Some(nid) => match hdag.tensor_of(nid) {
                            Some(t) => {
                                let psi_hdag = t[1] - t[4]; // morphic − entropic
                                ((psi_hdag + 1.0) / 2.0).clamp(0.0, 1.0)
                            }
                            None => 0.5,
                        },
                        None => 0.5,
                    },
                    None => 0.5, // HDAG disabled: neutral weight
                };

                // D = ψ · ρ · ω  (Pfauenthron++ score)
                let score = psi_sem * rho_pse * omega_hdag;
                if score > 0.0 {
                    Some(ILMatch { crystal_id_hex: entry.crystal_id_hex.clone(), score })
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }

    /// Topological order of committed crystals from the HDAG.
    /// Returns insertion order as fallback when HDAG is unavailable.
    pub fn topological_order(&self) -> Vec<String> {
        if let Some(ref hdag) = self.hdag {
            return hdag.topological_order();
        }
        self.index
            .entries
            .iter()
            .filter_map(|e| e.hdag_node_id.clone())
            .collect()
    }

    /// Verify path invariance between two HDAG nodes.
    pub fn verify_path_invariance(
        &self,
        from_crystal_id_hex: &str,
        to_crystal_id_hex: &str,
    ) -> Option<crate::hdag::PathInvarianceResult> {
        let hdag = self.hdag.as_ref()?;
        let from_nid = format!("N-{}", &from_crystal_id_hex[..16.min(from_crystal_id_hex.len())]);
        let to_nid   = format!("N-{}", &to_crystal_id_hex[..16.min(to_crystal_id_hex.len())]);
        Some(hdag.verify_path_invariance(&from_nid, &to_nid))
    }

    /// Mean coherence potential ψ = morphic − entropic across all HDAG nodes.
    pub fn mean_coherence_potential(&self) -> f64 {
        self.hdag
            .as_ref()
            .map(|h| h.mean_coherence_potential())
            .unwrap_or(0.0)
    }

    /// Number of committed blocks.
    pub fn len(&self) -> usize {
        self.index.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.index.entries.is_empty()
    }

    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    fn save_index(&self) -> Result<(), String> {
        let s = serde_json::to_string_pretty(&self.index)
            .map_err(|e| format!("serialise IL index: {e}"))?;
        std::fs::write(&self.index_path, s).map_err(|e| format!("write IL index: {e}"))
    }
}

fn cosine(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() {
        return 0.0;
    }
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let nb: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if na < 1e-10 || nb < 1e-10 {
        return 0.0;
    }
    dot / (na * nb)
}

fn hex_hash(s: &str) -> String {
    Sha256::digest(s.as_bytes())
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

fn compute_block_hash(block: &serde_json::Value) -> String {
    let mut obj = block.as_object().cloned().unwrap_or_default();
    obj.remove("hash");
    let canonical = serde_json::to_string(&serde_json::Value::Object(obj)).unwrap_or_default();
    hex_hash(&canonical)
}

fn simple_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (year, month, day) = days_to_ymd(secs / 86400);
    let tod = secs % 86400;
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year,
        month,
        day,
        tod / 3600,
        (tod % 3600) / 60,
        tod % 60
    )
}

fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    let mut d = days;
    let mut year = 1970u64;
    loop {
        let diy = if is_leap(year) { 366 } else { 365 };
        if d < diy {
            break;
        }
        d -= diy;
        year += 1;
    }
    let month_days = [
        31u64,
        if is_leap(year) { 29 } else { 28 },
        31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
    ];
    let mut month = 1u64;
    for &md in &month_days {
        if d < md {
            break;
        }
        d -= md;
        month += 1;
    }
    (year, month, d + 1)
}

fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use pse_types::SemanticCrystal;

    fn dummy_crystal(stability: f64) -> SemanticCrystal {
        SemanticCrystal {
            crystal_id: {
                let mut id = [0u8; 32];
                id[0] = (stability * 255.0) as u8;
                id[1] = 0xCD;
                id
            },
            region: vec![],
            constraint_program: Default::default(),
            stability_score: stability,
            topology_signature: Default::default(),
            betti_numbers: vec![],
            evidence_chain: Default::default(),
            commit_proof: Default::default(),
            operator_versions: Default::default(),
            created_at: 1,
            free_energy: 0.0,
            carrier_instance_idx: 0,
            scale_tag: String::new(),
            universe_id: String::new(),
            sub_crystal_ids: vec![],
            parent_crystal_ids: vec![],
            genesis_metadata: None,
            metatron_signature: None,
        }
    }

    #[test]
    fn commit_and_search_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ILStore::open(dir.path(), "TEST").unwrap();

        let crystal = dummy_crystal(0.8);
        let hash = store
            .commit(&crystal, &["cognitive arch".into()], 1, "What is ACT-R?")
            .unwrap();
        assert!(!hash.is_empty());
        assert_eq!(store.len(), 1);

        let adapter = CrystalAdapter::new("TEST");
        let payload = adapter.convert(&crystal, &[]).unwrap();
        let hits = store.search(&payload.vector8, 5);
        assert!(!hits.is_empty());
        assert!(hits[0].score > 0.99);
    }

    #[test]
    fn duplicate_commit_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ILStore::open(dir.path(), "TEST").unwrap();
        let crystal = dummy_crystal(0.75);
        store.commit(&crystal, &[], 1, "q1").unwrap();
        store.commit(&crystal, &[], 1, "q1").unwrap();
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn block_files_written_to_disk() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ILStore::open(dir.path(), "TEST").unwrap();
        store.commit(&dummy_crystal(0.9), &[], 1, "q").unwrap();
        assert!(dir.path().join("ledger").join("block_000000.mef").exists());
    }

    #[test]
    fn index_survives_reload() {
        let dir = tempfile::tempdir().unwrap();
        {
            let mut store = ILStore::open(dir.path(), "TEST").unwrap();
            store.commit(&dummy_crystal(0.8), &[], 1, "q").unwrap();
        }
        let store2 = ILStore::open(dir.path(), "TEST").unwrap();
        assert_eq!(store2.len(), 1);
    }

    #[test]
    fn search_ranks_similar_crystal_higher() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ILStore::open(dir.path(), "TEST").unwrap();

        // high stability → different vector from low
        let c_high = dummy_crystal(0.95);
        let c_low = dummy_crystal(0.05);
        store.commit(&c_high, &[], 1, "q").unwrap();
        store.commit(&c_low, &[], 1, "q").unwrap();

        let adapter = CrystalAdapter::new("TEST");
        let query_vec = adapter.convert(&c_high, &[]).unwrap().vector8;
        let hits = store.search(&query_vec, 2);
        assert_eq!(hits.len(), 2);
        // The high-stability crystal should score higher
        let adapter2 = CrystalAdapter::new("TEST");
        let high_hex: String = c_high.crystal_id.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(hits[0].crystal_id_hex, high_hex);
        let _ = adapter2; // suppress warning
    }

    #[test]
    fn hdag_nodes_created_on_commit() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ILStore::open(dir.path(), "TEST").unwrap();
        store.commit(&dummy_crystal(0.8), &[], 1, "q1").unwrap();
        store.commit(&dummy_crystal(0.6), &[], 2, "q2").unwrap();

        // Both index entries should have HDAG node IDs
        assert!(store.index.entries.iter().all(|e| e.hdag_node_id.is_some()));
        let order = store.topological_order();
        assert!(order.len() >= 2);
    }

    #[test]
    fn semantic_edges_connect_resonant_crystals() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ILStore::open(dir.path(), "TEST").unwrap();

        // Crystal A: moderate stability, ψ(A)=0.30 — in S_coh, tensor near C
        // tensor = [0.0, 0.60, 0.0, 0.10, 0.30]
        let mut c_a = dummy_crystal(0.70);
        c_a.topology_signature.kuramoto_coherence = 0.60;
        c_a.topology_signature.spectral_gap = 0.10;
        c_a.crystal_id[0] = 0xA1;

        // Crystal B: higher ψ(B)=0.50 — in S_coh but tensor-DISTANT (spectral_gap=3.0)
        // Sequential edge A→B passes (ψ increases). Semantic edge B→C blocked by distance.
        // tensor = [0.0, 0.70, 0.0, 3.00, 0.20]
        let mut c_b = dummy_crystal(0.80);
        c_b.topology_signature.kuramoto_coherence = 0.70;
        c_b.topology_signature.spectral_gap = 3.00;
        c_b.crystal_id[0] = 0xB1;

        // Crystal C: highest ψ(C)=0.60 — in S_coh, tensor-NEAR A (spectral_gap=0.12)
        // Sequential edge B→C passes (ψ increases). Semantic edge A→C expected.
        // tensor = [0.0, 0.75, 0.0, 0.12, 0.15]
        let mut c_c = dummy_crystal(0.85);
        c_c.topology_signature.kuramoto_coherence = 0.75;
        c_c.topology_signature.spectral_gap = 0.12;
        c_c.crystal_id[0] = 0xC1;

        store.commit(&c_a, &[], 1, "q1").unwrap();
        store.commit(&c_b, &[], 1, "q2").unwrap();
        store.commit(&c_c, &[], 1, "q3").unwrap();

        // C should have a sequential edge from B and a semantic edge from A
        let hdag = store.hdag.as_ref().expect("HDAG must be active");
        let seq_edges  = hdag.edge_count_by_cause("sequential_commit");
        let sem_edges  = hdag.edge_count_by_cause("resonance_proximity");
        assert!(seq_edges >= 2, "expect at least 2 sequential edges, got {seq_edges}");
        assert!(sem_edges >= 1, "expect at least 1 semantic edge A→C, got {sem_edges}");
    }

    #[test]
    fn mean_coherence_potential_is_finite() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ILStore::open(dir.path(), "TEST").unwrap();
        store.commit(&dummy_crystal(0.8), &[], 1, "q").unwrap();
        let psi = store.mean_coherence_potential();
        assert!(psi.is_finite());
    }

    /// Commit order: original (A) → intermediate (B) → refined(from A).
    ///
    /// Sequential edges: A→B, B→refined.
    /// Refinement edge:  A→refined  (distinct from the sequential edge B→refined).
    ///
    /// Tensor values are chosen so ψ is monotonically non-decreasing:
    ///   ψ(A)=0.30, ψ(B)=0.40, ψ(refined)≈0.36  (all above -0.1, all in S_coh).
    /// B→refined passes because ψ(refined)=0.36 ≥ ψ(B)-ε=0.35.
    /// A→refined passes because ψ(refined)=0.36 ≥ ψ(A)-ε=0.25.
    #[test]
    fn refinement_edge_links_parent_to_refined_crystal() {
        use crate::feedback::{compute_il_stability, refine_crystal, ValidationFeedback};

        let dir = tempfile::tempdir().unwrap();
        let mut store = ILStore::open(dir.path(), "TEST").unwrap();

        // Original — ψ(A) = 0.60 − 0.30 = 0.30
        let mut c_a = dummy_crystal(0.70);
        c_a.topology_signature.kuramoto_coherence = 0.60;
        c_a.crystal_id[0] = 0xA1;
        store.commit(&c_a, &[], 1, "q1").unwrap();

        // Intermediate — ψ(B) = 0.65 − 0.25 = 0.40 > ψ(A); sequential A→B created
        let mut c_b = dummy_crystal(0.75);
        c_b.topology_signature.kuramoto_coherence = 0.65;
        c_b.crystal_id[0] = 0xB1;
        store.commit(&c_b, &[], 1, "q2").unwrap();

        // IL-refined from c_a — genuinely new crystal_id (SHA-256 content-addressed).
        // il_stability=0.9 → new_stability=0.76 → ψ(refined)=0.36; passes B→refined gate.
        let il_stability = compute_il_stability(true, true, 0.8);
        let feedback = ValidationFeedback {
            block_hash: "testhash".to_string(),
            converged: true,
            coherence_potential: 0.8,
            gate_passed: true,
            hdag_node_id: None,
            il_stability,
        };
        let c_refined = refine_crystal(&c_a, &feedback, 2);
        let orig_hex: String = c_a.crystal_id.iter().map(|b| format!("{:02x}", b)).collect();
        assert!(
            c_refined.parent_crystal_ids.contains(&orig_hex),
            "sanity: parent_crystal_ids must reference the original"
        );
        store.commit(&c_refined, &[], 2, "q3").unwrap();

        let hdag = store.hdag.as_ref().expect("HDAG must be active");
        let ref_edges = hdag.edge_count_by_cause("refinement");
        assert!(ref_edges >= 1, "expect at least 1 refinement edge A→refined, got {ref_edges}");
    }
}

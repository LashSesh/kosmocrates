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
use pse_types::SemanticCrystal;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[cfg(feature = "hdag")]
use mef_hdag::HDAG;

#[cfg(feature = "il-pipeline")]
use mef_core::MEFCore;

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
    /// HDAG node ID for this entry (populated when `hdag` feature is active).
    #[serde(default)]
    hdag_node_id: Option<String>,
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

    #[cfg(feature = "hdag")]
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

        #[cfg(feature = "hdag")]
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
            #[cfg(feature = "hdag")]
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
        let payload = self.build_payload(crystal, source_chunks, session, question)?;
        self.commit_payload(payload)
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

    fn commit_payload(&mut self, payload: ILPayload) -> Result<String, String> {
        // Idempotent: skip if already committed
        if let Some(existing) = self
            .index
            .entries
            .iter()
            .find(|e| e.crystal_id_hex == payload.crystal_id_hex)
        {
            return Ok(existing.block_hash.clone());
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

        // ── HDAG: register node + edge to previous ───────────────────────────
        let hdag_node_id = self.register_hdag_node(&payload, block_index);

        self.index.entries.push(IndexEntry {
            block_index,
            crystal_id_hex: payload.crystal_id_hex,
            tic_id: payload.tic_id,
            block_hash: block_hash.clone(),
            block_file: block_file_name,
            vector8: payload.vector8,
            phase: payload.phase,
            hdag_node_id,
        });
        self.save_index()?;

        Ok(block_hash)
    }

    /// Register the crystal as an HDAG node; draw an edge from the previous node.
    /// Returns the node-ID on success, None on any error or when hdag is disabled.
    #[allow(unused_variables)]
    fn register_hdag_node(
        &mut self,
        payload: &ILPayload,
        _block_index: i64,
    ) -> Option<String> {
        #[cfg(feature = "hdag")]
        {
            if let Some(ref mut hdag) = self.hdag {
                let node_id = format!("N-{}", &payload.crystal_id_hex[..16]);
                let ts = simple_timestamp();

                match hdag.create_node(
                    &payload.tic_id,
                    payload.phase,
                    Some(ts),
                    Some(node_id.clone()),
                ) {
                    Ok(_) => {
                        // Edge from previous node (sequential commit causal link)
                        if let Some(prev) = self.index.entries.last() {
                            if let Some(ref prev_nid) = prev.hdag_node_id {
                                let phi = cosine(&payload.vector8, &prev.vector8);
                                let _ = hdag.create_edge(prev_nid, &node_id, phi, "sequential_commit");
                            }
                        }
                        return Some(node_id);
                    }
                    Err(e) => eprintln!("[IL] HDAG node error: {e}"),
                }
            }
        }
        None
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

    /// Topological order of committed crystals from the HDAG (when enabled).
    /// Returns commit order as fallback when HDAG is not active.
    pub fn topological_order(&mut self) -> Vec<String> {
        #[cfg(feature = "hdag")]
        if let Some(ref mut hdag) = self.hdag {
            return hdag.get_topological_order();
        }
        // Fallback: insertion order
        self.index
            .entries
            .iter()
            .filter_map(|e| e.hdag_node_id.clone())
            .collect()
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

    #[cfg(feature = "hdag")]
    #[test]
    fn hdag_nodes_created_on_commit() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ILStore::open(dir.path(), "TEST").unwrap();
        store.commit(&dummy_crystal(0.8), &[], 1, "q1").unwrap();
        store.commit(&dummy_crystal(0.6), &[], 2, "q2").unwrap();

        // Both entries should have HDAG node IDs
        // (We verify indirectly via topological_order)
        let order = store.topological_order();
        assert!(order.len() >= 2);
    }
}

//! Optional Infinity Ledger integration for pse-llm-demo.
//!
//! Activated by setting `PSE_IL_STORE` to a directory path.  When active:
//!   - Every new crystal is committed to the IL file-based ledger.
//!   - On Session 3+, the LLM question is embedded as an 8D vector and used
//!     to retrieve fuzzy-similar crystals from the IL HNSW index.
//!   - The fuzzy-retrieved records supplement the deterministic PSE recall.
//!
//! When `PSE_IL_STORE` is not set the bridge is a no-op and zero overhead.

use pse_adapter_il::{
    adapter::text_to_vector8,
    store::{ILMatch, ILStore},
};
use pse_types::SemanticCrystal;

use crate::memory::CrystalRecord;

pub struct ILBridge {
    store: Option<ILStore>,
}

impl ILBridge {
    /// Construct from environment.  Sets `PSE_IL_STORE` to a path to activate.
    pub fn from_env() -> Self {
        let store = std::env::var("PSE_IL_STORE").ok().and_then(|path| {
            match ILStore::open(&path, "pse-llm-demo") {
                Ok(s) => {
                    println!("  IL store  : {path} ({} block(s))", s.len());
                    Some(s)
                }
                Err(e) => {
                    eprintln!("  [IL] Warning: cannot open store at {path}: {e}");
                    None
                }
            }
        });
        Self { store }
    }

    /// Commit a crystal to the IL ledger.  Idempotent — safe to call twice.
    pub fn commit(
        &mut self,
        crystal: &SemanticCrystal,
        source_chunks: &[String],
        session: usize,
        question: &str,
    ) {
        if let Some(store) = &mut self.store {
            match store.commit(crystal, source_chunks, session, question) {
                Ok(hash) => {
                    let short: String = hash.chars().take(12).collect();
                    println!("  IL commit : block hash {}…", short);
                }
                Err(e) => eprintln!("  [IL] commit error: {e}"),
            }
        }
    }

    /// Query the IL HNSW index with `question` and return the `top_k` crystal
    /// records from `all_records` that are most similar by cosine distance.
    ///
    /// Returns an empty vec when IL is disabled or when no match exceeds 0.5.
    pub fn query_similar<'a>(
        &self,
        question: &str,
        top_k: usize,
        all_records: &'a [CrystalRecord],
    ) -> Vec<(&'a CrystalRecord, f64)> {
        let store = match &self.store {
            Some(s) => s,
            None => return vec![],
        };
        if store.is_empty() {
            return vec![];
        }

        let q_vec = text_to_vector8(question);
        let hits: Vec<ILMatch> = store.search(&q_vec, top_k);

        hits.into_iter()
            .filter(|h| h.score >= 0.5)
            .filter_map(|h| {
                all_records.iter().find(|r| {
                    let id_hex: String = r
                        .crystal
                        .crystal_id
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect();
                    id_hex == h.crystal_id_hex
                }).map(|r| (r, h.score))
            })
            .collect()
    }
}

//! Crystal persistence — saves and loads SemanticCrystals + prior LLM text.

use pse_types::SemanticCrystal;
use serde::{Deserialize, Serialize};

/// A crystal together with the source sentences that produced it.
/// Stored alongside the raw crystal so the context renderer can reconstruct
/// human-readable context for LLM injection.
#[derive(Serialize, Deserialize, Clone)]
pub struct CrystalRecord {
    pub crystal: SemanticCrystal,
    /// The sentence-level chunks from the sliding window that fired the gate.
    pub source_chunks: Vec<String>,
    /// 1-indexed session number in which this crystal was formed.
    pub session: usize,
    /// The question that was asked in this session.
    pub question: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct MemoryFile {
    /// Crystals from all prior sessions (used for PSE warm-start).
    pub crystals: Vec<SemanticCrystal>,
    /// Raw LLM responses from prior sessions — replayed in session 2+ to
    /// guarantee topology-identical observations → deterministic memory hits.
    pub prior_responses: Vec<String>,
    /// Crystals with source provenance — used by the context renderer.
    /// Absent in files written before this field was added (serde default = empty).
    #[serde(default)]
    pub crystal_records: Vec<CrystalRecord>,
}

pub struct CrystalStore {
    pub path: String,
}

impl CrystalStore {
    pub fn from_env() -> Self {
        let path =
            std::env::var("PSE_LLM_MEMORY").unwrap_or_else(|_| "pse-llm-memory.json".to_string());
        Self { path }
    }

    pub fn load(&self) -> MemoryFile {
        match std::fs::read_to_string(&self.path) {
            Ok(s) => match serde_json::from_str(&s) {
                Ok(mem) => mem,
                Err(e) => {
                    eprintln!(
                        "[PSE] Warning: memory file {:?} could not be parsed ({e}) — \
                         starting cold.  Back up and delete the file to suppress this warning.",
                        self.path
                    );
                    MemoryFile::default()
                }
            },
            Err(_) => MemoryFile::default(),
        }
    }

    pub fn save(&self, mem: &MemoryFile) -> Result<(), String> {
        let json =
            serde_json::to_string_pretty(mem).map_err(|e| format!("serialisation error: {e}"))?;
        std::fs::write(&self.path, json).map_err(|e| format!("write error: {e}"))
    }
}

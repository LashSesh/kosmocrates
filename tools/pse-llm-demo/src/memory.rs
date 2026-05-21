//! Crystal persistence — saves and loads SemanticCrystals + prior LLM text.

use pse_types::SemanticCrystal;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct MemoryFile {
    /// Crystals from all prior sessions.
    pub crystals: Vec<SemanticCrystal>,
    /// Raw LLM responses from prior sessions, kept for deterministic replay.
    /// Replay in session 2 guarantees topology-identical observations →
    /// guaranteed memory hits, proving cross-session recognition.
    pub prior_responses: Vec<String>,
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
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => MemoryFile::default(),
        }
    }

    pub fn save(&self, mem: &MemoryFile) -> Result<(), String> {
        let json =
            serde_json::to_string_pretty(mem).map_err(|e| format!("serialisation error: {e}"))?;
        std::fs::write(&self.path, json).map_err(|e| format!("write error: {e}"))
    }
}

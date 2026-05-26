use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Serialize)]
pub struct RunManifest {
    pub version: String,
    pub timestamp: String,
    pub artifacts: Vec<ArtifactEntry>,
    pub self_hash: String,
}

#[derive(Serialize)]
pub struct ArtifactEntry {
    pub name: String,
    pub sha256: String,
    pub size_bytes: u64,
}

pub fn build(out_dir: &Path, _results: &crate::benchmarks::AllResults) -> RunManifest {
    let files = [
        "b1_retrieval.json",
        "b2_noise.json",
        "b3_constitutional.json",
        "b4_etv.json",
        "b5_anomaly.json",
        "b6_agent.json",
        "b7_scalability.json",
        "report.md",
        "tables.tex",
    ];
    let artifacts: Vec<ArtifactEntry> = files
        .iter()
        .filter_map(|name| {
            let path = out_dir.join(name);
            let data = std::fs::read(&path).ok()?;
            let size_bytes = data.len() as u64;
            let sha256 = sha256_hex(&data);
            Some(ArtifactEntry {
                name: name.to_string(),
                sha256,
                size_bytes,
            })
        })
        .collect();

    // Compute self_hash: hash of artifact list in JCS order (simplified: alphabetical json)
    let entries_json = serde_json::to_string(&artifacts).unwrap_or_default();
    let self_hash = sha256_hex(entries_json.as_bytes());

    RunManifest {
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono_now(),
        artifacts,
        self_hash,
    }
}

fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn chrono_now() -> String {
    // Use UNIX_EPOCH as fallback — no external chrono dependency needed
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{secs}")
}

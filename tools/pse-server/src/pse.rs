//! PSE operations used by the HTTP handlers.
//! Mirrors the logic in tools/pse-llm-demo and bindings/python.

use std::f64::consts::TAU;

use pse_core::{load_memory_from_crystals, macro_step, GlobalState};
use pse_graph::{ObservationAdapter, ObserveError};
use pse_types::{
    content_address_raw, Config, MeasurementContext, Observation, ProvenanceEnvelope,
    SemanticCrystal,
};
use serde::{Deserialize, Serialize};

struct TextPhaseAdapter {
    source_id: String,
}

impl TextPhaseAdapter {
    fn new(source: impl Into<String>) -> Self {
        Self { source_id: source.into() }
    }
}

impl ObservationAdapter for TextPhaseAdapter {
    fn source_id(&self) -> &str { &self.source_id }

    fn canonicalize(&self, raw: &[u8], context: &MeasurementContext) -> Result<Observation, ObserveError> {
        let phase = if raw.is_empty() { 0.0 } else {
            let avg: f64 = raw.iter().map(|&b| b as f64).sum::<f64>() / raw.len() as f64;
            (avg / 255.0) * TAU
        };
        let payload = raw.to_vec();
        let digest = content_address_raw(&payload);
        Ok(Observation {
            timestamp: 0.0,
            source_id: self.source_id.clone(),
            provenance: ProvenanceEnvelope { origin: self.source_id.clone(), chain: Vec::new(), sig: None },
            payload,
            context: context.clone(),
            digest,
            schema_version: "1.0.0".to_string(),
            phase_hint: Some(phase),
        })
    }
}

// ── Config ────────────────────────────────────────────────────────────────────

pub fn default_config() -> Config {
    let mut cfg = Config::default();
    cfg.calibration.enabled = true;
    cfg.calibration.target_pass_rate = 0.30;
    cfg.calibration.window = 20;
    cfg.calibration.warmup_ticks = 2;
    cfg.carrier.adaptive = true;
    cfg.thresholds.d = 0.05;
    cfg.thresholds.q = 0.05;
    cfg.thresholds.r = 0.05;
    cfg.thresholds.g = 0.05;
    cfg.thresholds.j = 0.05;
    cfg.thresholds.p = 0.05;
    cfg.thresholds.n = 0.05;
    cfg.thresholds.k = 0.05;
    cfg.consensus.consensus_threshold = 0.0;
    cfg.consensus.mirror_consistency_eta = 0.0;
    cfg.consensus.por_kappa_bar = 0.0;
    cfg
}

// ── Chunking (mirrors tools/pse-llm-demo/src/observe.rs) ─────────────────────

pub fn chunk_text(text: &str) -> Vec<Vec<u8>> {
    let mut chunks: Vec<Vec<u8>> = Vec::new();
    for paragraph in text.split('\n') {
        let para = paragraph.trim();
        if para.is_empty() {
            continue;
        }
        let mut start = 0;
        let chars: Vec<char> = para.chars().collect();
        for (i, &ch) in chars.iter().enumerate() {
            if matches!(ch, '.' | '!' | '?') {
                let s: String = chars[start..=i].iter().collect();
                let s = s.trim().to_string();
                if s.len() >= 8 {
                    chunks.push(s.into_bytes());
                }
                start = i + 1;
            }
        }
        let tail: String = chars[start..].iter().collect();
        let tail = tail.trim().to_string();
        if tail.len() >= 8 {
            chunks.push(tail.into_bytes());
        }
    }
    if chunks.len() < 4 {
        chunks = text
            .split_whitespace()
            .collect::<Vec<_>>()
            .chunks(8)
            .map(|w| w.join(" ").into_bytes())
            .collect();
    }
    chunks
}

// ── CrystalRecord ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct CrystalRecord {
    pub crystal:       SemanticCrystal,
    pub source_chunks: Vec<String>,
    pub session:       usize,
    pub question:      String,
}

// ── CrystalInfo (wire format, no raw SemanticCrystal) ─────────────────────────

#[derive(Serialize)]
pub struct CrystalInfo {
    pub id:            String,
    pub stability:     f64,
    pub region_size:   usize,
    pub source_chunks: Vec<String>,
}

impl CrystalInfo {
    pub fn from_record(r: &CrystalRecord) -> Self {
        CrystalInfo {
            id: r.crystal.crystal_id.iter().map(|b| format!("{b:02x}")).collect(),
            stability: r.crystal.stability_score,
            region_size: r.crystal.region.len(),
            source_chunks: r.source_chunks.clone(),
        }
    }
}

// ── Ingest ────────────────────────────────────────────────────────────────────

pub struct IngestResult {
    pub new_records:   Vec<CrystalRecord>,
    pub all_crystals:  Vec<SemanticCrystal>,
    pub all_records:   Vec<CrystalRecord>,
    pub pattern_hits:  u64,
    pub commit_index:  u64,
}

/// Process `text` through PSE. Returns crystals formed + updated state JSON.
///
/// `memory_json`  — from a previous response's `memory_json` field (warm start).
/// `records_json` — from a previous response's `records_json` field.
pub fn ingest(
    text:         &str,
    memory_json:  Option<&str>,
    records_json: Option<&str>,
    session:      usize,
    question:     &str,
    source_name:  &str,
) -> Result<IngestResult, String> {
    let config = default_config();
    let mut state = GlobalState::new(&config);

    // Warm-start: load prior crystals into PatternMemory.
    let mut all_crystals: Vec<SemanticCrystal> = match memory_json {
        Some(j) => serde_json::from_str(j).map_err(|e| format!("memory_json: {e}"))?,
        None    => Vec::new(),
    };
    let mut all_records: Vec<CrystalRecord> = match records_json {
        Some(j) => serde_json::from_str(j).map_err(|e| format!("records_json: {e}"))?,
        None    => Vec::new(),
    };
    load_memory_from_crystals(&mut state, &all_crystals);

    let adapter = TextPhaseAdapter::new(source_name);
    let chunks  = chunk_text(text);

    const WINDOW: usize = 4;
    let n = chunks.len();
    let mut new_records = Vec::new();

    for i in 0..n.saturating_sub(WINDOW - 1) {
        let batch = chunks[i..(i + WINDOW).min(n)].to_vec();
        if let Ok(Some(crystal)) = macro_step(&mut state, &batch, &config, &adapter) {
            let source_chunks: Vec<String> = batch
                .iter()
                .map(|b| String::from_utf8_lossy(b).into_owned())
                .collect();
            let rec = CrystalRecord {
                crystal: crystal.clone(),
                source_chunks,
                session,
                question: question.to_string(),
            };
            all_crystals.push(crystal);
            all_records.push(rec.clone());
            new_records.push(rec);
        }
    }

    Ok(IngestResult {
        new_records,
        all_crystals,
        all_records,
        pattern_hits: state.pattern_hits,
        commit_index: state.commit_index,
    })
}

// ── Context renderer ─────────────────────────────────────────────────────────

pub fn render_context(records: &[CrystalRecord], top_k: usize) -> String {
    if records.is_empty() {
        return String::new();
    }
    let mut sorted = records.to_vec();
    sorted.sort_by(|a, b| {
        b.crystal
            .stability_score
            .partial_cmp(&a.crystal.stability_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    sorted.truncate(top_k);

    let mut out = format!(
        "[PSE Cognitive Substrate — {} stable pattern(s) from prior session(s)]\n\n",
        sorted.len()
    );
    for r in &sorted {
        let id: String = r.crystal.crystal_id.iter().take(4).map(|b| format!("{b:02x}")).collect();
        out.push_str(&format!(
            "▸ Pattern #{id} (stability {:.3}, session {})\n",
            r.crystal.stability_score, r.session
        ));
        for chunk in r.source_chunks.iter().take(3) {
            let t = chunk.trim();
            if !t.is_empty() {
                out.push_str(&format!("  \"{t}\"\n"));
            }
        }
        out.push('\n');
    }
    out.push_str(
        "These patterns are topologically stable structures identified by PSE. \
         Use them as conceptual anchors — not instructions to repeat specific phrases.\n",
    );
    out
}

// ── Coverage scorer ──────────────────────────────────────────────────────────

pub fn score_coverage(text: &str, keywords: &[String]) -> (usize, usize) {
    let lower = text.to_lowercase();
    let hits = keywords.iter().filter(|k| lower.contains(k.as_str())).count();
    (hits, keywords.len())
}

//! Chunks an LLM text response into PSE observations.
//!
//! Each chunk becomes one `Vec<u8>` observation — a sentence or short
//! paragraph unit. A sliding window over these chunks forms each tick's
//! batch, so the PSE graph evolves with the semantic flow of the response.

use std::f64::consts::TAU;

use pse_graph::{ObservationAdapter, ObserveError};
use pse_types::{content_address_raw, MeasurementContext, Observation, ProvenanceEnvelope};

/// Observation adapter that injects a semantic phase derived from the average
/// character value of each chunk.  Unlike `PassthroughAdapter` (which leaves
/// `phase_hint = None`, causing the engine to fall back to an avalanche-hash
/// phase that the carrier can never align with), this adapter gives the engine
/// a smooth, content-correlated phase signal so the Mandorla κ can converge.
pub struct TextPhaseAdapter {
    source_id: String,
}

impl TextPhaseAdapter {
    pub fn new(source: impl Into<String>) -> Self {
        Self { source_id: source.into() }
    }
}

impl ObservationAdapter for TextPhaseAdapter {
    fn source_id(&self) -> &str {
        &self.source_id
    }

    fn canonicalize(
        &self,
        raw: &[u8],
        context: &MeasurementContext,
    ) -> Result<Observation, ObserveError> {
        // Semantic phase: average byte value mapped to [0, 2π].
        // Sentences on the same topic share similar character distributions
        // (similar vocabulary → similar byte ranges), giving the carrier
        // a stable phase signal to lock onto across a sliding window.
        let phase = if raw.is_empty() {
            0.0
        } else {
            let avg: f64 = raw.iter().map(|&b| b as f64).sum::<f64>() / raw.len() as f64;
            (avg / 255.0) * TAU
        };
        let payload = raw.to_vec();
        let digest = content_address_raw(&payload);
        Ok(Observation {
            timestamp: 0.0,
            source_id: self.source_id.clone(),
            provenance: ProvenanceEnvelope {
                origin: self.source_id.clone(),
                chain: Vec::new(),
                sig: None,
            },
            payload,
            context: context.clone(),
            digest,
            schema_version: "1.0.0".to_string(),
            phase_hint: Some(phase),
        })
    }
}

/// Split a text response into sentence/paragraph chunks for PSE ingestion.
/// Returns each chunk as raw UTF-8 bytes.
pub fn chunk_response(text: &str) -> Vec<Vec<u8>> {
    let mut chunks: Vec<Vec<u8>> = Vec::new();

    // First split by newlines (paragraph boundaries)
    for paragraph in text.split('\n') {
        let para = paragraph.trim();
        if para.is_empty() {
            continue;
        }

        // Then split by sentence-ending punctuation
        let mut start = 0;
        let chars: Vec<char> = para.chars().collect();
        for (i, &ch) in chars.iter().enumerate() {
            if matches!(ch, '.' | '!' | '?') {
                // Grab everything up to and including the punctuation
                let end = i + 1;
                let sentence: String = chars[start..end].iter().collect();
                let sentence = sentence.trim().to_string();
                if sentence.len() >= 8 {
                    chunks.push(sentence.into_bytes());
                }
                start = end;
            }
        }

        // Remaining text after last sentence terminator
        let tail: String = chars[start..].iter().collect();
        let tail = tail.trim().to_string();
        if tail.len() >= 8 {
            chunks.push(tail.into_bytes());
        }
    }

    // Fall back: if we got very few chunks, treat each word-group as a chunk
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

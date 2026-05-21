//! Chunks an LLM text response into PSE observations.
//!
//! Each chunk becomes one `Vec<u8>` observation — a sentence or short
//! paragraph unit. A sliding window over these chunks forms each tick's
//! batch, so the PSE graph evolves with the semantic flow of the response.

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

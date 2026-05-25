use serde::Deserialize;

// ── Retrieval dataset (B1/B2) ──────────────────────────────────────────────

#[derive(Deserialize, Clone)]
pub struct RetrievalDoc {
    pub id: String,
    pub title: String,
    pub body: String,
    pub domain: String,
    pub stability: f64,   // used to set crystal stability_score
    pub kuramoto: f64,    // used to set crystal kuramoto_coherence
}

#[derive(Deserialize, Clone)]
pub struct RelevanceLabel {
    pub doc_id: String,
    pub grade: u8, // 0=irrelevant, 1=topical, 2=directly relevant
}

#[derive(Deserialize, Clone)]
pub struct RetrievalQuery {
    pub id: String,
    pub text: String,
    pub relevant: Vec<RelevanceLabel>,
}

#[derive(Deserialize)]
pub struct RetrievalDataset {
    pub documents: Vec<RetrievalDoc>,
    pub queries: Vec<RetrievalQuery>,
}

pub fn retrieval_dataset() -> RetrievalDataset {
    let raw = include_str!("retrieval.json");
    serde_json::from_str(raw).expect("parse retrieval.json")
}

// ── Constitutional dataset (B3) ────────────────────────────────────────────

#[derive(Deserialize, Clone)]
pub struct ConstitutionalRule {
    pub id: String,
    pub severity: String, // "Blocking" | "Required" | "Advisory"
    pub triggers: Vec<String>,
}

#[derive(Deserialize, Clone)]
pub struct ConstitutionalAction {
    pub id: String,
    pub verb: String,
    pub target: String,
    pub description: String,
    pub expected_decision: String, // "Allow" | "Warn" | "Block"
    pub category: String,
}

#[derive(Deserialize)]
pub struct ConstitutionalDataset {
    pub rules: Vec<ConstitutionalRule>,
    pub actions: Vec<ConstitutionalAction>,
}

pub fn constitutional_dataset() -> ConstitutionalDataset {
    let raw = include_str!("constitutional.json");
    serde_json::from_str(raw).expect("parse constitutional.json")
}

// ── Reasoning dataset (B4) ─────────────────────────────────────────────────

#[derive(Deserialize, Clone)]
pub struct ReasoningDoc {
    pub id: String,
    pub text: String,
    pub stability: f64,
    pub kuramoto: f64,
}

#[derive(Deserialize, Clone)]
pub struct ReasoningQuery {
    pub id: String,
    pub text: String,
    pub min_chain_length: usize,
    pub min_mean_d: f64,
}

#[derive(Deserialize)]
pub struct ReasoningDataset {
    pub corpus: Vec<ReasoningDoc>,
    pub queries: Vec<ReasoningQuery>,
}

pub fn reasoning_dataset() -> ReasoningDataset {
    let raw = include_str!("reasoning.json");
    serde_json::from_str(raw).expect("parse reasoning.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retrieval_dataset_size() {
        let ds = retrieval_dataset();
        assert_eq!(ds.documents.len(), 100, "expected 100 documents");
        assert_eq!(ds.queries.len(), 30, "expected 30 queries");
    }
    #[test]
    fn constitutional_dataset_size() {
        let ds = constitutional_dataset();
        assert!(!ds.rules.is_empty());
        assert_eq!(ds.actions.len(), 60, "expected 60 actions");
    }
    #[test]
    fn reasoning_dataset_size() {
        let ds = reasoning_dataset();
        assert!(!ds.corpus.is_empty());
        assert!(!ds.queries.is_empty());
    }
}

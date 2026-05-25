// Keyword-based constitutional baseline: block if any danger word is present, else allow.
// This is the strawman to beat with ConstitutionalEvaluator.

const DANGER_WORDS: &[&str] = &["rm -rf", "drop table", "truncate", "password", "api_key",
                                  "secret", "credential", "delete", "overwrite", "wipe"];

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum KeywordDecision { Allow, Block }

pub fn evaluate(verb: &str, target: &str, description: &str) -> KeywordDecision {
    let combined = format!("{verb} {target} {description}").to_lowercase();
    for word in DANGER_WORDS {
        if combined.contains(word) {
            return KeywordDecision::Block;
        }
    }
    KeywordDecision::Allow
}

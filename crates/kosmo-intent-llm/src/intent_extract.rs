//! LLM-backed utterance routing — with the keyword rules as its safety net.
//!
//! The model classifies an utterance into one of the existing
//! [`ChatIntent`]s via a pinned JSON contract. Routing is **not a durable
//! artifact** (no content addressing, no evidence binding — the routed-to
//! organs own all of that), which is precisely why a malformed or
//! unreachable model degrades to [`route_keywords`] instead of erroring:
//! the router stays total, and the worst a bad model can do is route like
//! the deterministic one.
//!
//! The JSON contract deliberately mirrors the enum: there is no field in
//! which a path, file list or entity scaffold could ride along — a model
//! that invents one changes nothing, because only the known fields are
//! read.

use kosmo_intent::{route_keywords, ChatIntent, IntentExtractor};
use kosmo_llm::{extract_json_object, LlmConfig};
use serde::Deserialize;

/// The fixed system prompt pinning the model to the intent-JSON contract.
pub fn intent_system_prompt() -> &'static str {
    "You route a developer's chat utterance to ONE intent of the Kosmocrates \
     substrate. You never plan, never generate code, never name files — you \
     only classify.\n\n\
     Rules:\n\
     - Output EXACTLY one JSON object and nothing else. No prose, no markdown \
     fences.\n\
     - `intent` is one of: \"make_wish\" (measure the utterance as a wish), \
     \"descend_wish\" (the user asks to build/realize it), \"show_landscape\", \
     \"adopt_landscape\", \"adopt_cluster\", \"show_status\", \"inject_norm\".\n\
     - `prose` carries the wish wording for make_wish/descend_wish (default: \
     the utterance itself).\n\n\
     JSON schema:\n\
     {\n\
       \"intent\": string,\n\
       \"prose\": string,    // optional\n\
       \"top\": number,      // optional, adopt_landscape\n\
       \"index\": number,    // optional, adopt_cluster (1-based)\n\
       \"geometry\": bool    // optional, show_landscape\n\
     }"
}

/// Build the per-request user prompt from the utterance.
pub fn build_intent_prompt(utterance: &str) -> String {
    format!(
        "# Utterance\n\n{}\n\nReturn the JSON object naming the one intent.",
        utterance.trim()
    )
}

#[derive(Deserialize)]
struct IntentJson {
    intent: String,
    #[serde(default)]
    prose: Option<String>,
    #[serde(default)]
    top: Option<u32>,
    #[serde(default)]
    index: Option<u32>,
    #[serde(default)]
    geometry: Option<bool>,
}

/// Parse a model response into a [`ChatIntent`]. `None` for anything
/// malformed or unknown — the caller falls back to the keyword rules.
pub fn parse_intent_response(raw: &str, utterance: &str) -> Option<ChatIntent> {
    let json = extract_json_object(raw)?;
    let parsed: IntentJson = serde_json::from_str(&json).ok()?;
    let prose = || {
        parsed
            .prose
            .clone()
            .filter(|p| !p.trim().is_empty())
            .unwrap_or_else(|| utterance.trim().to_string())
    };
    match parsed.intent.trim().to_ascii_lowercase().as_str() {
        "make_wish" => Some(ChatIntent::MakeWish { prose: prose() }),
        "descend_wish" => Some(ChatIntent::DescendWish { prose: prose() }),
        "show_landscape" => Some(ChatIntent::ShowLandscape {
            geometry: parsed.geometry.unwrap_or(false),
        }),
        "adopt_landscape" => Some(ChatIntent::AdoptLandscape {
            top: parsed.top.unwrap_or(3).max(1),
        }),
        "adopt_cluster" => Some(ChatIntent::AdoptCluster {
            index: parsed.index.unwrap_or(1).max(1),
        }),
        "show_status" => Some(ChatIntent::ShowStatus),
        "inject_norm" => Some(ChatIntent::InjectNorm),
        _ => None,
    }
}

/// An [`IntentExtractor`] that asks a model first and falls back to the
/// deterministic keyword rules on ANY failure (transport, malformed JSON,
/// unknown intent) — total by construction.
pub struct LlmIntentExtractor {
    config: LlmConfig,
    label: String,
}

impl LlmIntentExtractor {
    pub fn new(config: LlmConfig) -> Self {
        let label = format!("llm-intent:{}", config.label());
        Self { config, label }
    }

    pub fn config(&self) -> &LlmConfig {
        &self.config
    }
}

impl IntentExtractor for LlmIntentExtractor {
    fn extract(&self, utterance: &str) -> ChatIntent {
        self.config
            .complete(intent_system_prompt(), &build_intent_prompt(utterance))
            .ok()
            .and_then(|(raw, _tokens)| parse_intent_response(&raw, utterance))
            .unwrap_or_else(|| route_keywords(utterance))
    }
    fn name(&self) -> &str {
        &self.label
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_prompt_pins_the_contract_and_embeds_the_utterance() {
        let p = intent_system_prompt();
        for name in [
            "make_wish",
            "descend_wish",
            "show_landscape",
            "adopt_landscape",
            "adopt_cluster",
            "show_status",
            "inject_norm",
        ] {
            assert!(p.contains(name), "{name} missing from the contract");
        }
        assert!(p.contains("never name files"), "the router must not plan");
        assert!(build_intent_prompt("  adopt cluster 2  ").contains("adopt cluster 2"));
    }

    #[test]
    fn parse_maps_every_intent_shape() {
        let parse = |raw: &str| parse_intent_response(raw, "the utterance");
        assert_eq!(
            parse(r#"{"intent":"make_wish","prose":"a crate foo"}"#),
            Some(ChatIntent::MakeWish {
                prose: "a crate foo".into()
            })
        );
        assert_eq!(
            parse(r#"{"intent":"make_wish"}"#),
            Some(ChatIntent::MakeWish {
                prose: "the utterance".into()
            }),
            "missing prose defaults to the utterance"
        );
        assert_eq!(
            parse(r#"{"intent":"descend_wish","prose":""}"#),
            Some(ChatIntent::DescendWish {
                prose: "the utterance".into()
            }),
            "empty prose defaults to the utterance"
        );
        assert_eq!(
            parse(r#"{"intent":"show_landscape","geometry":true}"#),
            Some(ChatIntent::ShowLandscape { geometry: true })
        );
        assert_eq!(
            parse(r#"{"intent":"adopt_landscape","top":0}"#),
            Some(ChatIntent::AdoptLandscape { top: 1 }),
            "top clamps to >= 1"
        );
        assert_eq!(
            parse(r#"{"intent":"adopt_cluster","index":4}"#),
            Some(ChatIntent::AdoptCluster { index: 4 })
        );
        assert_eq!(
            parse(r#"{"intent":"show_status"}"#),
            Some(ChatIntent::ShowStatus)
        );
        assert_eq!(
            parse(r#"{"intent":"inject_norm"}"#),
            Some(ChatIntent::InjectNorm)
        );
    }

    #[test]
    fn malformed_or_unknown_yields_none() {
        assert_eq!(parse_intent_response("the model rambled", "u"), None);
        assert_eq!(
            parse_intent_response(r#"{"intent":"rm_rf_slash"}"#, "u"),
            None
        );
        assert_eq!(parse_intent_response(r#"{"no_intent_field":1}"#, "u"), None);
    }

    #[test]
    fn fences_are_tolerated() {
        let raw = "Sure:\n```json\n{\"intent\":\"show_status\"}\n```";
        assert_eq!(
            parse_intent_response(raw, "u"),
            Some(ChatIntent::ShowStatus)
        );
    }

    #[test]
    fn unreachable_model_falls_back_to_keywords_without_network() {
        // An empty API key fails fast at the transport; the extractor must
        // degrade to the deterministic router — total, never an error.
        let x = LlmIntentExtractor::new(LlmConfig::claude(""));
        assert!(x.name().starts_with("llm-intent:claude:"));
        assert_eq!(
            x.extract("show me the landscape"),
            ChatIntent::ShowLandscape { geometry: false }
        );
        assert_eq!(
            x.extract("total gibberish"),
            ChatIntent::MakeWish {
                prose: "total gibberish".into()
            }
        );
    }
}

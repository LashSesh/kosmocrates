//! The model-backed wish refiner — proposals for the atelier, never acts.
//!
//! Given the draft's dialogue and current facet sets, the model may do two
//! things and two things only: **suggest measurable facets** (which land in
//! the draft's `pending` list, awaiting the operator's verdict) and **ask
//! questions** (displayed, never answered by itself). The same JSON-contract
//! discipline as everywhere on the LLM seam: suggestions with unknown kinds
//! or empty keys are dropped, fences tolerated, and a malformed or
//! unreachable model degrades to an *empty* refinement with an honest note —
//! refinement is advisory, so a dead model costs suggestions, never
//! correctness.

use kosmo_core::WishFacet;
use kosmo_intent::{FacetSuggestion, SuggestionSource, WishDraft};
use kosmo_llm::{extract_json_object, truncate, LlmConfig};
use serde::Deserialize;

use crate::facet_kind_from_str;

/// The fixed system prompt pinning the refiner to the suggestion contract.
pub fn refine_system_prompt() -> &'static str {
    "You help an operator shape a WISH for the Kosmocrates substrate: a set \
     of measurable structural facets a workspace must exhibit. You may ONLY \
     suggest facets and ask clarifying questions — you never plan file \
     layouts, never name technologies, never generate code.\n\n\
     Rules:\n\
     - Output EXACTLY one JSON object and nothing else. No prose, no \
     markdown fences.\n\
     - Suggest only facets that serve the operator's stated intent. Do not \
     re-suggest facets already accepted or already pending.\n\
     - `kind` is one of: \"crate\", \"module\", \"symbol\", \"signature\", \
     \"contract\", \"test\", \"doc\", \"behavior\", \"capability\", \
     \"composition\". `key` is a bare name or spec — never a path.\n\
     - Ask a question when the intent is ambiguous instead of guessing.\n\n\
     JSON schema:\n\
     {\n\
       \"suggestions\": [ { \"kind\": string, \"key\": string, \"rationale\": string } ],\n\
       \"questions\": [ string ]\n\
     }"
}

/// Build the per-round user prompt from the draft's state.
pub fn build_refine_prompt(draft: &WishDraft) -> String {
    let mut out = String::from("# Dialogue so far\n\n");
    for (i, prose) in draft.prose_history.iter().enumerate() {
        out.push_str(&format!("{}. {}\n", i + 1, prose));
    }
    out.push_str("\n# Facets already in the wish (do not re-suggest)\n\n");
    if draft.accepted.is_empty() {
        out.push_str("(none yet)\n");
    }
    for f in &draft.accepted {
        out.push_str(&format!("- {:?} {}\n", f.kind, f.key));
    }
    out.push_str("\n# Suggestions already pending (do not re-suggest)\n\n");
    if draft.pending.is_empty() {
        out.push_str("(none)\n");
    }
    for s in &draft.pending {
        out.push_str(&format!("- {:?} {}\n", s.facet.kind, s.facet.key));
    }
    out.push_str(
        "\nReturn the JSON object with your suggestions and questions for \
         this wish.",
    );
    out
}

#[derive(Deserialize)]
struct SuggestionJson {
    kind: String,
    key: String,
    #[serde(default)]
    rationale: String,
}

#[derive(Deserialize)]
struct RefineJson {
    #[serde(default)]
    suggestions: Vec<SuggestionJson>,
    #[serde(default)]
    questions: Vec<String>,
}

/// One refinement round's outcome. `note` carries an honest explanation
/// when the model contributed nothing (unreachable, malformed).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RefineOutcome {
    pub suggestions: Vec<FacetSuggestion>,
    pub questions: Vec<String>,
    pub note: Option<String>,
}

/// Parse a model response into suggestions + questions. `None` when no JSON
/// object can be extracted or parsed — the caller reports and moves on.
/// Suggestions with unknown kinds or empty keys are silently dropped (the
/// refiner never invents a facet kind).
pub fn parse_refine_response(raw: &str, source_label: &str) -> Option<RefineOutcome> {
    let json = extract_json_object(raw)?;
    let parsed: RefineJson = serde_json::from_str(&json).ok()?;
    let suggestions = parsed
        .suggestions
        .into_iter()
        .filter_map(|s| {
            let kind = facet_kind_from_str(&s.kind)?;
            let key = s.key.trim();
            if key.is_empty() {
                return None;
            }
            Some(FacetSuggestion {
                facet: WishFacet::new(kind, key),
                rationale: if s.rationale.trim().is_empty() {
                    "model suggestion".to_string()
                } else {
                    s.rationale.trim().to_string()
                },
                source: SuggestionSource::Model {
                    label: source_label.to_string(),
                },
            })
        })
        .collect();
    Some(RefineOutcome {
        suggestions,
        questions: parsed
            .questions
            .into_iter()
            .map(|q| q.trim().to_string())
            .filter(|q| !q.is_empty())
            .collect(),
        note: None,
    })
}

/// The model-backed refiner. Advisory by contract: every failure mode
/// returns an empty outcome with a note instead of an error.
pub struct LlmWishRefiner {
    config: LlmConfig,
    label: String,
}

impl LlmWishRefiner {
    pub fn new(config: LlmConfig) -> Self {
        let label = config.label();
        Self { config, label }
    }

    pub fn name(&self) -> &str {
        &self.label
    }

    /// One refinement round over the draft. Never fails: an unreachable or
    /// malformed model yields an empty outcome with an honest `note`.
    pub fn refine(&self, draft: &WishDraft) -> RefineOutcome {
        match self
            .config
            .complete(refine_system_prompt(), &build_refine_prompt(draft))
        {
            Ok((raw, _tokens)) => {
                parse_refine_response(&raw, &self.label).unwrap_or_else(|| RefineOutcome {
                    note: Some(format!(
                        "refiner returned no parseable JSON: {}",
                        truncate(&raw, 120)
                    )),
                    ..RefineOutcome::default()
                })
            }
            Err(e) => RefineOutcome {
                note: Some(format!("refiner unavailable: {e}")),
                ..RefineOutcome::default()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::Digest;
    use kosmo_intent::NormCatalog;

    fn draft() -> WishDraft {
        WishDraft::new(Digest::of_bytes(b"pol")).speak(
            "a module parser and a function parse_line",
            &NormCatalog::empty(),
        )
    }

    #[test]
    fn refine_prompt_pins_the_contract_and_carries_the_state() {
        let p = refine_system_prompt();
        assert!(p.contains("never plan file"), "the refiner must not plan");
        assert!(p.contains("never name technologies"));
        assert!(p.contains("suggestions"));
        let user = build_refine_prompt(&draft());
        assert!(user.contains("a module parser and a function parse_line"));
        assert!(user.contains("Module parser"), "{user}");
        assert!(user.contains("do not re-suggest"));
    }

    #[test]
    fn parse_maps_suggestions_and_questions() {
        let raw = r#"{"suggestions":[
            {"kind":"test","key":"parser_roundtrip","rationale":"pin the behaviour"},
            {"kind":"behavior","key":"parse_line(\"a,b\")=>2"}
        ],"questions":["which separator should parse_line accept?"]}"#;
        let outcome = parse_refine_response(raw, "claude:test").unwrap();
        assert_eq!(outcome.suggestions.len(), 2);
        assert_eq!(
            outcome.suggestions[0].facet,
            WishFacet::test("parser_roundtrip")
        );
        assert_eq!(outcome.suggestions[0].rationale, "pin the behaviour");
        assert!(matches!(
            &outcome.suggestions[0].source,
            SuggestionSource::Model { label } if label == "claude:test"
        ));
        assert_eq!(outcome.suggestions[1].rationale, "model suggestion");
        assert_eq!(outcome.questions.len(), 1);
        assert_eq!(outcome.note, None);
    }

    #[test]
    fn parse_drops_unknown_kinds_and_empty_keys() {
        let raw = r#"{"suggestions":[
            {"kind":"microservice","key":"users"},
            {"kind":"test","key":"  "},
            {"kind":"doc","key":"parser"}
        ]}"#;
        let outcome = parse_refine_response(raw, "x").unwrap();
        assert_eq!(outcome.suggestions.len(), 1, "only the doc survives");
        assert_eq!(outcome.suggestions[0].facet, WishFacet::doc("parser"));
    }

    #[test]
    fn malformed_yields_none_and_fences_are_tolerated() {
        assert_eq!(parse_refine_response("the model rambled", "x"), None);
        let fenced = "Here:\n```json\n{\"suggestions\":[],\"questions\":[\"q?\"]}\n```";
        let outcome = parse_refine_response(fenced, "x").unwrap();
        assert_eq!(outcome.questions, vec!["q?".to_string()]);
    }

    #[test]
    fn unreachable_model_degrades_to_an_empty_outcome_with_a_note() {
        // Advisory by contract: a dead transport costs suggestions, never
        // correctness — and it says so.
        let refiner = LlmWishRefiner::new(LlmConfig::claude(""));
        let outcome = refiner.refine(&draft());
        assert!(outcome.suggestions.is_empty());
        assert!(outcome.questions.is_empty());
        assert!(outcome.note.unwrap().contains("refiner unavailable"));
    }
}

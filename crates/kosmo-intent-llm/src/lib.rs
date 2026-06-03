//! Kosmocrates LLM wish compiler — prose intent → content-addressed [`Wish`].
//!
//! The LLM-backed counterpart to `kosmo-intent`'s deterministic
//! [`compile_wish`](kosmo_intent::compile_wish). It implements the same
//! [`WishCompiler`] trait, so it drops into the agent loop wherever the rule
//! compiler does — the only difference is that free-form prose ("build me a
//! small HTTP server with a router and a request handler") is turned into a
//! structured facet set by a model instead of by keyword rules.
//!
//! The model is the **only** non-deterministic step: it emits a list of target
//! facets, which are folded into a `Wish` whose `id` is content-addressed over
//! the (sorted, de-duplicated) facet set and the prose label. Everything
//! downstream — assessment, the attractor contract, the loop — stays
//! deterministic and replayable.
//!
//! Transport (Anthropic / OpenAI-compatible, retry, JSON extraction) is shared
//! with the rest of the substrate via [`kosmo_llm`].

use kosmo_core::{Digest, Wish, WishFacet, WishFacetKind, WishPredicate};
use kosmo_intent::{WishCompileError, WishCompiler};
use kosmo_llm::{config_from_env, extract_json_object, truncate, LlmConfig, LlmError};
use serde::Deserialize;

/// A [`WishCompiler`] that delegates prose→`Wish` translation to an LLM.
pub struct LlmWishCompiler {
    config: LlmConfig,
    label: String,
}

impl LlmWishCompiler {
    pub fn new(config: LlmConfig) -> Self {
        let label = config.label();
        Self { config, label }
    }

    /// Claude via the Anthropic Messages API.
    pub fn claude(api_key: impl Into<String>) -> Self {
        Self::new(LlmConfig::claude(api_key))
    }

    /// Cerebras (OpenAI-compatible).
    pub fn cerebras(api_key: impl Into<String>) -> Self {
        Self::new(LlmConfig::cerebras(api_key))
    }

    /// Build from environment credentials (see [`kosmo_llm::config_from_env`]).
    pub fn from_env() -> Result<Self, LlmError> {
        Ok(Self::new(config_from_env()?))
    }

    pub fn config(&self) -> &LlmConfig {
        &self.config
    }
}

impl WishCompiler for LlmWishCompiler {
    fn compile(
        &self,
        prose: &str,
        policy_id: Digest,
        evidence_bundle_id: Digest,
    ) -> Result<Wish, WishCompileError> {
        let user = build_prompt(prose);
        let (raw, _tokens) = self
            .config
            .complete(system_prompt(), &user)
            .map_err(|e| WishCompileError { message: e.to_string() })?;
        parse_wish_response(&raw, prose, policy_id, evidence_bundle_id)
    }

    fn name(&self) -> &str {
        &self.label
    }
}

// ─── Prompt construction (pure) ─────────────────────────────────────────────

/// The fixed system prompt pinning the model to the facet-JSON contract.
pub fn system_prompt() -> &'static str {
    "You translate a developer's intent, written in prose, into a structured \
     target for the Kosmocrates substrate — the set of structural facets a Rust \
     workspace must exhibit to satisfy the wish.\n\n\
     Rules:\n\
     - Output EXACTLY one JSON object and nothing else. No prose, no markdown \
     fences.\n\
     - List only facets the workspace must POSITIVELY contain. Express \"the bad \
     thing must be gone\" as a `resolution` facet, never as a negation.\n\
     - `key` is the bare name: a crate/package name, a module name, or a symbol \
     (fn / type / trait) name.\n\n\
     JSON schema:\n\
     {\n\
       \"rationale\": string,        // one line on how these facets capture the intent\n\
       \"facets\": [                  // may be empty if the prose states no structure\n\
         { \"kind\": \"crate\"|\"module\"|\"symbol\"|\"resolution\", \"key\": string }\n\
       ]\n\
     }"
}

/// Build the per-request user prompt from a prose wish.
pub fn build_prompt(prose: &str) -> String {
    format!(
        "# Wish (developer intent)\n\n{}\n\nReturn the JSON object describing the \
         target facets.",
        prose.trim()
    )
}

// ─── Response parsing (pure) ────────────────────────────────────────────────

#[derive(Deserialize)]
struct FacetJson {
    kind: String,
    key: String,
}

#[derive(Deserialize)]
struct WishResponseJson {
    #[serde(default)]
    facets: Vec<FacetJson>,
}

/// Map a model-supplied kind string to a [`WishFacetKind`]. Unknown kinds are
/// dropped (the compiler never invents a facet kind).
fn facet_kind_from_str(s: &str) -> Option<WishFacetKind> {
    match s.trim().to_ascii_lowercase().as_str() {
        "crate" | "crates" | "package" => Some(WishFacetKind::Crate),
        "module" | "mod" => Some(WishFacetKind::Module),
        "symbol" | "function" | "fn" | "method" | "type" | "struct" | "enum" | "trait"
        | "const" | "static" => Some(WishFacetKind::Symbol),
        "resolution" | "void" => Some(WishFacetKind::Resolution),
        "dependency" | "depends" | "dep" => Some(WishFacetKind::Dependency),
        _ => None,
    }
}

/// Parse an LLM response into a content-addressed [`Wish`].
///
/// Tolerant of fences/prose around the JSON. Facets with empty keys or unknown
/// kinds are dropped; the prose becomes the wish label.
pub fn parse_wish_response(
    raw: &str,
    prose: &str,
    policy_id: Digest,
    evidence_bundle_id: Digest,
) -> Result<Wish, WishCompileError> {
    let json = extract_json_object(raw).ok_or_else(|| WishCompileError {
        message: format!("no JSON object in LLM response: {}", truncate(raw, 200)),
    })?;
    let parsed: WishResponseJson = serde_json::from_str(&json).map_err(|e| WishCompileError {
        message: format!("invalid wish JSON: {e}"),
    })?;
    let predicates: Vec<WishPredicate> = parsed
        .facets
        .into_iter()
        .filter_map(|f| {
            let kind = facet_kind_from_str(&f.kind)?;
            let key = f.key.trim();
            if key.is_empty() {
                return None;
            }
            Some(WishPredicate::require(WishFacet::new(kind, key)))
        })
        .collect();
    Ok(Wish::new(
        prose.trim().to_string(),
        predicates,
        policy_id,
        evidence_bundle_id,
    ))
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    #[test]
    fn system_prompt_pins_facet_schema() {
        let p = system_prompt();
        assert!(p.contains("facets"));
        assert!(p.contains("crate"));
    }

    #[test]
    fn build_prompt_embeds_prose() {
        let p = build_prompt("  an HTTP server  ");
        assert!(p.contains("an HTTP server"));
    }

    #[test]
    fn parse_wish_response_maps_facets() {
        let raw = r#"{"rationale":"x","facets":[
            {"kind":"crate","key":"kosmo-server"},
            {"kind":"module","key":"routes"},
            {"kind":"function","key":"handle_request"}
        ]}"#;
        let w = parse_wish_response(raw, "build a server", d(b"p"), d(b"e")).unwrap();
        assert_eq!(w.label, "build a server");
        assert!(w.predicates.iter().any(|p| p.facet == WishFacet::crate_("kosmo-server")));
        assert!(w.predicates.iter().any(|p| p.facet == WishFacet::module("routes")));
        assert!(w.predicates.iter().any(|p| p.facet == WishFacet::symbol("handle_request")));
    }

    #[test]
    fn parse_wish_response_tolerates_fences() {
        let raw = "Sure:\n```json\n{\"facets\":[{\"kind\":\"crate\",\"key\":\"alpha\"}]}\n```";
        let w = parse_wish_response(raw, "alpha", d(b"p"), d(b"e")).unwrap();
        assert!(w.predicates.iter().any(|p| p.facet == WishFacet::crate_("alpha")));
    }

    #[test]
    fn parse_wish_response_drops_unknown_kinds_and_empty_keys() {
        let raw = r#"{"facets":[
            {"kind":"wormhole","key":"x"},
            {"kind":"crate","key":""},
            {"kind":"crate","key":"real"}
        ]}"#;
        let w = parse_wish_response(raw, "p", d(b"p"), d(b"e")).unwrap();
        assert_eq!(w.predicate_count(), 1);
        assert!(w.predicates.iter().any(|p| p.facet == WishFacet::crate_("real")));
    }

    #[test]
    fn parse_wish_response_empty_facets_is_vacuous() {
        let w = parse_wish_response(r#"{"facets":[]}"#, "nothing", d(b"p"), d(b"e")).unwrap();
        assert_eq!(w.predicate_count(), 0);
    }

    #[test]
    fn parse_wish_response_no_json_errors() {
        let err = parse_wish_response("the model rambled", "p", d(b"p"), d(b"e")).unwrap_err();
        assert!(err.message.contains("no JSON"));
    }

    #[test]
    fn facet_kind_mapping() {
        assert_eq!(facet_kind_from_str("crate"), Some(WishFacetKind::Crate));
        assert_eq!(facet_kind_from_str("Module"), Some(WishFacetKind::Module));
        assert_eq!(facet_kind_from_str("fn"), Some(WishFacetKind::Symbol));
        assert_eq!(facet_kind_from_str("trait"), Some(WishFacetKind::Symbol));
        assert_eq!(facet_kind_from_str("resolution"), Some(WishFacetKind::Resolution));
        assert_eq!(facet_kind_from_str("dependency"), Some(WishFacetKind::Dependency));
        assert_eq!(facet_kind_from_str("nonsense"), None);
    }

    #[test]
    fn compiler_with_empty_key_errors_without_network() {
        // The WishCompiler contract end-to-end, minus the live call: an empty
        // API key fails fast at the transport, surfaced as a WishCompileError.
        let c = LlmWishCompiler::claude("");
        let err = c.compile("a crate foo", d(b"p"), d(b"e")).unwrap_err();
        assert!(err.message.contains("API key"));
        assert!(c.name().starts_with("claude:"));
    }
}

//! PSE-grounded LLM prompt construction.
//!
//! Closes the loop between context compression, causal discovery, and actual
//! LLM calls by building a ready-to-send prompt that injects verified crystal
//! knowledge as a structured system message.
//!
//! No HTTP client or async runtime required — `GroundedPrompt` is a plain
//! data structure you pass to whichever LLM SDK you already use.
//!
//! ## Usage
//! ```no_run
//! use pse_adapter_il::{ILStore, context::ContextBudget, prompt::PromptConfig};
//! use pse_adapter_il::adapter::text_to_vector8;
//!
//! let store = ILStore::open("./il_data", "seed").unwrap();
//! let config = PromptConfig::default();
//! let prompt = store.build_grounded_prompt("What is working memory?", &config);
//!
//! // Pass to any OpenAI-compatible SDK:
//! // client.chat(model, prompt.system_message(), prompt.user_message())
//! ```
//!
//! ## System message structure
//! ```text
//! [PSE-GROUNDING]
//! <optional system_prefix>
//!
//! [PSE-CONTEXT]
//! [PSE:abc12345ef012345] Q4 stab=0.87 D=0.742 t=3
//! tag: il-refined | "What is working memory?"
//! ...
//! [/PSE-CONTEXT]
//!
//! [CAUSAL-CONTEXT]          ← only when include_causal=true and crystals found
//! Crystal abc12345ef012345 has 1 direct cause(s):
//!   ← def67890ab123456 via refinement (strength=0.841)
//! [/CAUSAL-CONTEXT]
//! [/PSE-GROUNDING]
//! ```

use crate::context::{ContextBudget, CrystalSummary};

/// Configuration for [`ILStore::build_grounded_prompt`].
#[derive(Debug, Clone)]
pub struct PromptConfig {
    /// Crystal selection budget (QTIC filter + token cap + top_k).
    pub budget: ContextBudget,
    /// Text prepended inside `[PSE-GROUNDING]` before the crystal block.
    /// Use for model-specific instructions or persona.
    pub system_prefix: Option<String>,
    /// Whether to append a causal explanation of the top-ranked crystal.
    pub include_causal: bool,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            budget: ContextBudget::default(),
            system_prefix: None,
            include_causal: false,
        }
    }
}

/// A ready-to-send LLM prompt with PSE crystal context injected.
///
/// Compatible with any OpenAI-style chat API.
/// Use [`system_message`] and [`user_message`] to extract the two parts.
///
/// [`system_message`]: GroundedPrompt::system_message
/// [`user_message`]: GroundedPrompt::user_message
#[derive(Debug, Clone)]
pub struct GroundedPrompt {
    /// System message: PSE grounding block + optional prefix.
    /// Empty when no crystals passed the QTIC/budget filter.
    system: String,
    /// User message: the original query, unchanged.
    user: String,
    /// Crystals included in this prompt's context block.
    pub crystals_used: Vec<CrystalSummary>,
    /// Estimated token cost of the PSE context block.
    pub context_tokens: usize,
}

impl GroundedPrompt {
    /// The system message to pass to the LLM.
    /// Returns an empty string when no crystals satisfied the filter.
    pub fn system_message(&self) -> &str {
        &self.system
    }

    /// The user message — the original query, unchanged.
    pub fn user_message(&self) -> &str {
        &self.user
    }

    /// OpenAI-style messages array: `[(role, content), ...]`.
    ///
    /// If the system message is empty (no matching crystals), only the
    /// `"user"` message is returned.
    pub fn to_messages(&self) -> Vec<(&'static str, &str)> {
        if self.system.is_empty() {
            vec![("user", &self.user)]
        } else {
            vec![("system", &self.system), ("user", &self.user)]
        }
    }

    /// Whether any crystals were included in this prompt.
    pub fn is_grounded(&self) -> bool {
        !self.crystals_used.is_empty()
    }
}

// ── builder (called from ILStore) ────────────────────────────────────────────

/// Build a grounded prompt from pre-selected crystals and an optional causal explanation.
pub(crate) fn build_prompt(
    query: &str,
    config: &PromptConfig,
    crystals: Vec<CrystalSummary>,
    causal_context: Option<String>,
) -> GroundedPrompt {
    if crystals.is_empty() {
        return GroundedPrompt {
            system: String::new(),
            user: query.to_string(),
            crystals_used: vec![],
            context_tokens: 0,
        };
    }

    let context_tokens: usize = crystals.iter().map(|c| c.token_estimate()).sum();

    // Assemble [PSE-CONTEXT] block
    let mut context_block = String::from("[PSE-CONTEXT]\n");
    for c in &crystals {
        context_block.push_str(&c.to_compact_text());
        context_block.push('\n');
    }
    context_block.push_str("[/PSE-CONTEXT]");

    // Assemble full system message
    let mut system = String::from("[PSE-GROUNDING]\n");
    if let Some(ref prefix) = config.system_prefix {
        if !prefix.is_empty() {
            system.push_str(prefix);
            system.push_str("\n\n");
        }
    }
    system.push_str(&context_block);

    if let Some(causal) = causal_context {
        system.push_str("\n\n[CAUSAL-CONTEXT]\n");
        system.push_str(&causal);
        system.push_str("\n[/CAUSAL-CONTEXT]");
    }

    system.push_str("\n[/PSE-GROUNDING]");

    GroundedPrompt {
        system,
        user: query.to_string(),
        crystals_used: crystals,
        context_tokens,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::CrystalSummary;

    fn make_summary(stability: f64, qtic: u8, score: f64, question: &str) -> CrystalSummary {
        CrystalSummary {
            crystal_id: "abcdef1234567890".to_string(),
            stability,
            qtic_class: Some(qtic),
            tripolar_score: score,
            commit_index: 1,
            scale_tag: "original".to_string(),
            question: question.to_string(),
        }
    }

    #[test]
    fn prompt_with_crystals_has_pse_grounding_block() {
        let config = PromptConfig::default();
        let crystals = vec![make_summary(0.87, 4, 0.742, "What is working memory?")];
        let prompt = build_prompt("test query", &config, crystals, None);
        assert!(prompt.system_message().contains("[PSE-GROUNDING]"));
        assert!(prompt.system_message().contains("[PSE-CONTEXT]"));
        assert!(prompt.system_message().contains("[/PSE-CONTEXT]"));
        assert!(prompt.system_message().contains("[/PSE-GROUNDING]"));
    }

    #[test]
    fn user_message_is_original_query() {
        let config = PromptConfig::default();
        let crystals = vec![make_summary(0.8, 3, 0.6, "q")];
        let prompt = build_prompt("my specific query", &config, crystals, None);
        assert_eq!(prompt.user_message(), "my specific query");
    }

    #[test]
    fn no_crystals_gives_empty_system() {
        let config = PromptConfig::default();
        let prompt = build_prompt("query", &config, vec![], None);
        assert!(prompt.system_message().is_empty());
        assert!(!prompt.is_grounded());
    }

    #[test]
    fn crystals_used_matches_input() {
        let config = PromptConfig::default();
        let crystals = vec![
            make_summary(0.9, 5, 0.9, "q1"),
            make_summary(0.8, 4, 0.8, "q2"),
        ];
        let prompt = build_prompt("query", &config, crystals, None);
        assert_eq!(prompt.crystals_used.len(), 2);
    }

    #[test]
    fn causal_context_included_when_provided() {
        let config = PromptConfig::default();
        let crystals = vec![make_summary(0.8, 4, 0.7, "q")];
        let causal = "Crystal abc has 1 direct cause: xyz via refinement".to_string();
        let prompt = build_prompt("query", &config, crystals, Some(causal));
        assert!(prompt.system_message().contains("[CAUSAL-CONTEXT]"));
        assert!(prompt.system_message().contains("refinement"));
    }

    #[test]
    fn system_prefix_appears_before_context() {
        let config = PromptConfig {
            system_prefix: Some("You are a helpful assistant with PSE memory.".to_string()),
            ..Default::default()
        };
        let crystals = vec![make_summary(0.8, 4, 0.7, "q")];
        let prompt = build_prompt("query", &config, crystals, None);
        let sys = prompt.system_message();
        let prefix_pos = sys.find("helpful assistant").unwrap();
        let context_pos = sys.find("[PSE-CONTEXT]").unwrap();
        assert!(prefix_pos < context_pos, "prefix must appear before crystal context");
    }

    #[test]
    fn to_messages_includes_system_when_grounded() {
        let config = PromptConfig::default();
        let crystals = vec![make_summary(0.8, 4, 0.7, "q")];
        let prompt = build_prompt("user query", &config, crystals, None);
        let msgs = prompt.to_messages();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].0, "system");
        assert_eq!(msgs[1].0, "user");
        assert_eq!(msgs[1].1, "user query");
    }

    #[test]
    fn to_messages_only_user_when_no_crystals() {
        let config = PromptConfig::default();
        let prompt = build_prompt("query", &config, vec![], None);
        let msgs = prompt.to_messages();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].0, "user");
    }

    #[test]
    fn context_tokens_nonzero_when_crystals_present() {
        let config = PromptConfig::default();
        let crystals = vec![make_summary(0.8, 4, 0.7, "working memory question")];
        let prompt = build_prompt("query", &config, crystals, None);
        assert!(prompt.context_tokens > 0);
    }

    #[test]
    fn empty_system_prefix_not_added() {
        let config = PromptConfig {
            system_prefix: Some(String::new()),
            ..Default::default()
        };
        let crystals = vec![make_summary(0.8, 4, 0.7, "q")];
        let prompt = build_prompt("query", &config, crystals, None);
        // Should not have a double newline at the start
        assert!(!prompt.system_message().contains("[PSE-GROUNDING]\n\n[PSE-CONTEXT]"),
            "empty prefix must not insert extra blank line");
    }
}

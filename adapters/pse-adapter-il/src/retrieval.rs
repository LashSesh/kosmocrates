//! Causal retrieval: semantic search expanded through the HDAG causal graph.
//!
//! Standard Pfauenthron++ retrieval answers "which crystals are semantically
//! similar to this query?"  Causal retrieval answers the richer question:
//!
//! > *Given a query, find the most relevant crystals — then also surface*
//! > *what caused them (causal ancestry) and what they led to (causal*
//! > *descendants), so the LLM receives not just a fact but its lineage.*
//!
//! ## Algorithm
//!
//! 1. **Seed selection**: run Pfauenthron++ (D = ψ·ρ·ω) to find the top
//!    `seed_k` crystals semantically closest to the query.
//! 2. **Causal expansion**: for each seed, traverse the causal graph up to
//!    `max_depth` hops in both directions.  Each hop is weighted by the
//!    link strength along the traversal path.
//! 3. **Score blending**: every retrieved crystal receives a combined score:
//!    ```text
//!    final = α · D_semantic + (1 − α) · D_causal
//!    D_causal  = causal_strength / (1 + hop_count)
//!    ```
//!    where `α ∈ [0, 1]` is [`CausalRetrievalConfig::causal_blend`].
//! 4. **Budget filtering**: apply the [`ContextBudget`] (QTIC floor + token
//!    cap + top-k) to the merged, deduplicated result.
//!
//! ## Role annotation
//!
//! Each entry in the result carries a [`CausalRole`]:
//! - `Seed` — directly matched by semantic search.
//! - `Ancestor(depth)` — upstream cause of a seed.
//! - `Descendant(depth)` — downstream effect of a seed.
//!
//! The role annotation lets the LLM system message clearly distinguish
//! "retrieved as relevant" from "retrieved because it caused the relevant crystal".
//!
//! ## Example system message fragment
//!
//! ```text
//! [PSE:abc12345ef012345] Q4 stab=0.87 D=0.742 t=3  [SEED]
//! [PSE:def67890ab123456] Q3 stab=0.72 D=0.614 t=1  [ANCESTOR depth=1 via refinement]
//! [PSE:789abc0123456789] Q4 stab=0.81 D=0.591 t=7  [DESCENDANT depth=1 via sequential]
//! ```

use crate::context::{ContextBudget, CrystalSummary};
use serde::{Deserialize, Serialize};

// ─── Configuration ────────────────────────────────────────────────────────────

/// Configuration for [`ILStore::causal_retrieval`].
#[derive(Debug, Clone)]
pub struct CausalRetrievalConfig {
    /// Number of seed crystals to select via Pfauenthron++ before causal expansion.
    /// Default: 3.
    pub seed_k: usize,
    /// Maximum causal hops to traverse from each seed (in each direction).
    /// Default: 2.
    pub max_depth: usize,
    /// Score blending coefficient α:
    /// `final = α · D_semantic + (1 − α) · D_causal`.
    /// 0.0 = pure causal; 1.0 = pure semantic.  Default: 0.5.
    pub causal_blend: f64,
    /// Budget applied to the merged result (QTIC floor + token cap + top-k).
    pub budget: ContextBudget,
    /// Whether to include ancestor crystals in the result.  Default: true.
    pub include_ancestors: bool,
    /// Whether to include descendant crystals in the result.  Default: true.
    pub include_descendants: bool,
}

impl Default for CausalRetrievalConfig {
    fn default() -> Self {
        Self {
            seed_k: 3,
            max_depth: 2,
            causal_blend: 0.5,
            budget: ContextBudget::default(),
            include_ancestors: true,
            include_descendants: true,
        }
    }
}

// ─── Causal role ──────────────────────────────────────────────────────────────

/// The causal role a crystal plays in the retrieval result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CausalRole {
    /// Directly selected by the semantic seed search.
    Seed,
    /// Upstream cause of a seed crystal; depth = number of causal hops.
    Ancestor { depth: usize },
    /// Downstream effect of a seed crystal; depth = number of causal hops.
    Descendant { depth: usize },
}

impl CausalRole {
    pub fn label(&self) -> String {
        match self {
            Self::Seed => "SEED".to_string(),
            Self::Ancestor { depth } => format!("ANCESTOR depth={depth}"),
            Self::Descendant { depth } => format!("DESCENDANT depth={depth}"),
        }
    }

    pub fn is_seed(&self) -> bool {
        matches!(self, Self::Seed)
    }
}

// ─── Retrieved entry ──────────────────────────────────────────────────────────

/// A single crystal in the causal retrieval result, annotated with its role
/// and blended retrieval score.
#[derive(Debug, Clone)]
pub struct CausallyGroundedEntry {
    /// Crystal summary (for context building and token estimation).
    pub summary: CrystalSummary,
    /// Causal role relative to the seed crystals.
    pub role: CausalRole,
    /// Semantic Pfauenthron++ score D ∈ [0, 1].
    /// Zero for purely causal entries (no direct semantic match).
    pub semantic_score: f64,
    /// Causal score: `link_strength / (1 + hop_count)` ∈ [0, 1].
    /// Zero for seeds (they have no causal path cost).
    pub causal_score: f64,
    /// Blended final score: `α · semantic + (1 − α) · causal`.
    pub score: f64,
}

impl CausallyGroundedEntry {
    /// One-line text for LLM system message injection.
    ///
    /// Format:
    /// ```text
    /// [PSE:abc12345ef012345] Q4 stab=0.87 D=0.742 t=3  [SEED]
    /// ```
    pub fn to_annotated_text(&self) -> String {
        format!(
            "{}  [{}]",
            self.summary.to_compact_text().lines().next().unwrap_or(""),
            self.role.label(),
        )
    }
}

// ─── Retrieval result ────────────────────────────────────────────────────────

/// Full result of a causal retrieval query.
#[derive(Debug, Clone)]
pub struct CausalRetrievalResult {
    /// Entries in the result, sorted by descending `score`.
    /// Includes seeds, ancestors, and descendants (as configured).
    pub entries: Vec<CausallyGroundedEntry>,
    /// Number of seed crystals found.
    pub seed_count: usize,
    /// Number of ancestor crystals included.
    pub ancestor_count: usize,
    /// Number of descendant crystals included.
    pub descendant_count: usize,
    /// Estimated token cost of all entries' context text.
    pub context_tokens: usize,
}

impl CausalRetrievalResult {
    /// True when the result contains at least one entry.
    pub fn is_grounded(&self) -> bool {
        !self.entries.is_empty()
    }

    /// Build a `[PSE-CONTEXT]` block with causal role annotations.
    ///
    /// Drop-in replacement for the output of [`ILStore::context_for_query`],
    /// but with `[SEED]` / `[ANCESTOR depth=N]` / `[DESCENDANT depth=N]`
    /// suffixes so the LLM can reason about the provenance of each entry.
    pub fn to_annotated_context_block(&self) -> String {
        if self.entries.is_empty() {
            return String::new();
        }
        let mut out = String::from("[PSE-CONTEXT causal=true]\n");
        for e in &self.entries {
            out.push_str(&e.to_annotated_text());
            out.push('\n');
            // Second line of compact_text (question / tag line)
            let compact = e.summary.to_compact_text();
            let mut line_iter = compact.lines();
            line_iter.next(); // skip first line already included above
            if let Some(second) = line_iter.next() {
                out.push_str(second);
                out.push('\n');
            }
        }
        out.push_str("[/PSE-CONTEXT]");
        out
    }

    /// All seed entries.
    pub fn seeds(&self) -> impl Iterator<Item = &CausallyGroundedEntry> {
        self.entries.iter().filter(|e| e.role.is_seed())
    }

    /// All ancestor entries.
    pub fn ancestors(&self) -> impl Iterator<Item = &CausallyGroundedEntry> {
        self.entries
            .iter()
            .filter(|e| matches!(e.role, CausalRole::Ancestor { .. }))
    }

    /// All descendant entries.
    pub fn descendants(&self) -> impl Iterator<Item = &CausallyGroundedEntry> {
        self.entries
            .iter()
            .filter(|e| matches!(e.role, CausalRole::Descendant { .. }))
    }
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_summary(id: &str, stability: f64, qtic: u8, score: f64) -> CrystalSummary {
        CrystalSummary {
            crystal_id: id.to_string(),
            stability,
            qtic_class: Some(qtic),
            tripolar_score: score,
            commit_index: 1,
            scale_tag: "test".to_string(),
            question: "test question".to_string(),
        }
    }

    #[test]
    fn causal_role_labels_correct() {
        assert_eq!(CausalRole::Seed.label(), "SEED");
        assert_eq!(
            CausalRole::Ancestor { depth: 1 }.label(),
            "ANCESTOR depth=1"
        );
        assert_eq!(
            CausalRole::Descendant { depth: 2 }.label(),
            "DESCENDANT depth=2"
        );
    }

    #[test]
    fn causal_role_is_seed_predicate() {
        assert!(CausalRole::Seed.is_seed());
        assert!(!CausalRole::Ancestor { depth: 1 }.is_seed());
        assert!(!CausalRole::Descendant { depth: 1 }.is_seed());
    }

    #[test]
    fn annotated_text_includes_role_label() {
        let entry = CausallyGroundedEntry {
            summary: make_summary("abcdef1234567890", 0.8, 4, 0.7),
            role: CausalRole::Ancestor { depth: 1 },
            semantic_score: 0.0,
            causal_score: 0.6,
            score: 0.3,
        };
        let text = entry.to_annotated_text();
        assert!(
            text.contains("[ANCESTOR depth=1]"),
            "must include role label: {text}"
        );
        assert!(text.contains("PSE:"), "must include PSE prefix: {text}");
    }

    #[test]
    fn to_annotated_context_block_wraps_in_pse_context() {
        let entries = vec![CausallyGroundedEntry {
            summary: make_summary("abcdef1234567890", 0.85, 4, 0.75),
            role: CausalRole::Seed,
            semantic_score: 0.75,
            causal_score: 0.0,
            score: 0.75,
        }];
        let result = CausalRetrievalResult {
            entries,
            seed_count: 1,
            ancestor_count: 0,
            descendant_count: 0,
            context_tokens: 20,
        };
        let block = result.to_annotated_context_block();
        assert!(block.starts_with("[PSE-CONTEXT causal=true]"));
        assert!(block.ends_with("[/PSE-CONTEXT]"));
        assert!(block.contains("[SEED]"));
    }

    #[test]
    fn empty_result_gives_empty_context_block() {
        let result = CausalRetrievalResult {
            entries: vec![],
            seed_count: 0,
            ancestor_count: 0,
            descendant_count: 0,
            context_tokens: 0,
        };
        assert!(result.to_annotated_context_block().is_empty());
        assert!(!result.is_grounded());
    }

    #[test]
    fn seeds_ancestors_descendants_iterators() {
        let entries = vec![
            CausallyGroundedEntry {
                summary: make_summary("aaaa000000000000", 0.8, 4, 0.7),
                role: CausalRole::Seed,
                semantic_score: 0.7,
                causal_score: 0.0,
                score: 0.7,
            },
            CausallyGroundedEntry {
                summary: make_summary("bbbb000000000000", 0.7, 3, 0.5),
                role: CausalRole::Ancestor { depth: 1 },
                semantic_score: 0.0,
                causal_score: 0.5,
                score: 0.25,
            },
            CausallyGroundedEntry {
                summary: make_summary("cccc000000000000", 0.75, 3, 0.55),
                role: CausalRole::Descendant { depth: 1 },
                semantic_score: 0.0,
                causal_score: 0.4,
                score: 0.2,
            },
        ];
        let result = CausalRetrievalResult {
            seed_count: 1,
            ancestor_count: 1,
            descendant_count: 1,
            context_tokens: 60,
            entries,
        };
        assert_eq!(result.seeds().count(), 1);
        assert_eq!(result.ancestors().count(), 1);
        assert_eq!(result.descendants().count(), 1);
    }

    #[test]
    fn default_config_has_sensible_values() {
        let cfg = CausalRetrievalConfig::default();
        assert_eq!(cfg.seed_k, 3);
        assert_eq!(cfg.max_depth, 2);
        assert!((cfg.causal_blend - 0.5).abs() < 1e-9);
        assert!(cfg.include_ancestors);
        assert!(cfg.include_descendants);
    }
}

//! Crystal → LLM context compression.
//!
//! Selects the most relevant committed crystals for injection into an LLM
//! context window using Pfauenthron++ tripolar scoring (D = ψ·ρ·ω) and
//! QTIC conformance filtering.
//!
//! ## Workflow
//! 1. Embed the current query with [`crate::adapter::text_to_vector8`].
//! 2. Call [`ILStore::build_context_entries`] to score all committed crystals.
//! 3. Pass the result to [`ContextBudget::select`] (or call the convenience
//!    wrapper [`ILStore::context_for_query`]).
//! 4. Prepend the returned `[PSE-CONTEXT]` block to your LLM prompt.
//!
//! ## Default policy
//! - **QTIC filter:** Q3+ only (gate-passed, coherence-stable crystals).
//! - **Token budget:** ≤ 512 estimated tokens.
//! - **Cardinality cap:** ≤ 8 crystals.

/// Compact, LLM-readable summary of one committed crystal.
///
/// Built by [`ILStore::build_context_entries`]; use [`to_compact_text`] to
/// obtain the two-line injection string.
///
/// [`to_compact_text`]: CrystalSummary::to_compact_text
#[derive(Debug, Clone)]
pub struct CrystalSummary {
    /// First 16 hex chars of the crystal ID.
    pub crystal_id: String,
    /// PSE stability score ∈ [0, 1].
    pub stability: f64,
    /// QTIC conformance class (0–5), if classified at commit time.
    pub qtic_class: Option<u8>,
    /// Pfauenthron++ tripolar score D = ψ·ρ·ω.
    /// Used for ranking; higher = more relevant to the current query.
    pub tripolar_score: f64,
    /// Commit index (extrinsic time *t*).
    pub commit_index: i64,
    /// Scale tag of the crystal (e.g., `"il-refined"`, `"original"`).
    pub scale_tag: String,
    /// Question that prompted this crystal's formation (empty if unknown).
    pub question: String,
}

impl CrystalSummary {
    /// Rough token estimate (1 token ≈ 4 chars).
    pub fn token_estimate(&self) -> usize {
        let len = self.to_compact_text().len();
        len.div_ceil(4)
    }

    /// Two-line compact representation for direct LLM injection.
    ///
    /// ```text
    /// [PSE:abc12345ef012345] Q4 stab=0.87 D=0.742 t=3
    /// tag: il-refined | "What is working memory?"
    /// ```
    pub fn to_compact_text(&self) -> String {
        let id = &self.crystal_id[..self.crystal_id.len().min(16)];
        let qtic = self
            .qtic_class
            .map(|q| format!("Q{}", q))
            .unwrap_or_else(|| "Q?".to_string());

        let tag_line = if self.question.is_empty() {
            format!("tag: {}", self.scale_tag)
        } else {
            let q: String = if self.question.chars().count() > 60 {
                let t: String = self.question.chars().take(57).collect();
                format!("{}...", t)
            } else {
                self.question.clone()
            };
            format!("tag: {} | \"{}\"", self.scale_tag, q)
        };

        format!(
            "[PSE:{}] {} stab={:.2} D={:.3} t={}\n{}",
            id, qtic, self.stability, self.tripolar_score, self.commit_index, tag_line
        )
    }
}

/// Parameters controlling which crystals enter the LLM context window.
///
/// Apply via [`ContextBudget::select`] or the convenience wrapper
/// [`ILStore::context_for_query`].
#[derive(Debug, Clone)]
pub struct ContextBudget {
    /// Maximum estimated token count for the entire context block (header +
    /// all entries).
    pub max_tokens: usize,
    /// Minimum QTIC conformance class (inclusive).  Default 3 = Q3+ ensures
    /// every selected crystal has passed the Kairos coherence gate.
    pub min_qtic_class: u8,
    /// Hard cardinality cap (applied before token budget).
    pub top_k: usize,
}

impl Default for ContextBudget {
    fn default() -> Self {
        Self {
            max_tokens: 512,
            min_qtic_class: 3,
            top_k: 8,
        }
    }
}

impl ContextBudget {
    /// Filter and select crystals within this budget.
    ///
    /// `summaries` must be pre-sorted by **descending** `tripolar_score`
    /// (as produced by [`ILStore::build_context_entries`]).
    ///
    /// Selection stops at the first crystal that would push usage past
    /// `max_tokens` or when `top_k` is reached.  Crystals below
    /// `min_qtic_class` are skipped (not counted against the cap).
    pub fn select<'a>(&self, summaries: &'a [CrystalSummary]) -> Vec<&'a CrystalSummary> {
        let mut selected = Vec::new();
        let mut tokens_used = 0usize;
        for s in summaries {
            if s.qtic_class.unwrap_or(0) < self.min_qtic_class {
                continue;
            }
            if selected.len() >= self.top_k {
                break;
            }
            let cost = s.token_estimate();
            if tokens_used + cost > self.max_tokens {
                break;
            }
            tokens_used += cost;
            selected.push(s);
        }
        selected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_summary(stability: f64, qtic: Option<u8>, score: f64, question: &str) -> CrystalSummary {
        CrystalSummary {
            crystal_id: "abcdef1234567890".to_string(),
            stability,
            qtic_class: qtic,
            tripolar_score: score,
            commit_index: 1,
            scale_tag: "original".to_string(),
            question: question.to_string(),
        }
    }

    #[test]
    fn compact_text_contains_key_fields() {
        let s = make_summary(0.87, Some(4), 0.742, "What is working memory?");
        let text = s.to_compact_text();
        assert!(text.contains("Q4"), "must contain QTIC class");
        assert!(text.contains("0.87"), "must contain stability");
        assert!(text.contains("0.742"), "must contain tripolar score");
        assert!(text.contains("original"), "must contain scale_tag");
        assert!(text.contains("working memory"), "must contain question excerpt");
    }

    #[test]
    fn compact_text_unknown_qtic_shows_question_mark() {
        let s = make_summary(0.7, None, 0.5, "");
        assert!(s.to_compact_text().contains("Q?"));
    }

    #[test]
    fn compact_text_truncates_long_question() {
        let long_q = "a".repeat(80);
        let s = make_summary(0.7, Some(3), 0.5, &long_q);
        let text = s.to_compact_text();
        // Truncated to 57 chars + "..."
        assert!(text.contains("..."), "long question must be truncated");
    }

    #[test]
    fn compact_text_empty_question_shows_tag_only() {
        let s = make_summary(0.7, Some(3), 0.5, "");
        let text = s.to_compact_text();
        assert!(!text.contains("|"), "no separator when question is empty");
        assert!(text.contains("tag: original"));
    }

    #[test]
    fn token_estimate_nonzero() {
        let s = make_summary(0.8, Some(4), 0.7, "some query");
        assert!(s.token_estimate() > 0);
    }

    #[test]
    fn budget_filters_by_qtic_class() {
        let summaries = vec![
            make_summary(0.9, Some(5), 0.9, "q"),
            make_summary(0.8, Some(3), 0.8, "q"),
            make_summary(0.7, Some(2), 0.7, "q"),  // below min_qtic_class=3
            make_summary(0.6, Some(1), 0.6, "q"),  // below min_qtic_class=3
        ];
        let budget = ContextBudget { min_qtic_class: 3, top_k: 10, max_tokens: 4096 };
        let selected = budget.select(&summaries);
        assert_eq!(selected.len(), 2, "only Q3+ should be selected");
        assert!(selected.iter().all(|s| s.qtic_class.unwrap_or(0) >= 3));
    }

    #[test]
    fn budget_respects_top_k() {
        let summaries: Vec<_> = (0..10)
            .map(|i| make_summary(0.8, Some(4), 0.9 - i as f64 * 0.01, "q"))
            .collect();
        let budget = ContextBudget { min_qtic_class: 0, top_k: 3, max_tokens: 4096 };
        assert_eq!(budget.select(&summaries).len(), 3);
    }

    #[test]
    fn budget_respects_token_limit() {
        // Each summary is ~20-25 tokens; budget only allows 2
        let summaries: Vec<_> = (0..10)
            .map(|i| make_summary(0.8, Some(4), 0.9 - i as f64 * 0.01, "test question text"))
            .collect();
        let budget = ContextBudget { min_qtic_class: 0, top_k: 100, max_tokens: 40 };
        let selected = budget.select(&summaries);
        assert!(!selected.is_empty());
        let total_tokens: usize = selected.iter().map(|s| s.token_estimate()).sum();
        assert!(total_tokens <= 40, "total tokens {total_tokens} must not exceed budget");
    }

    #[test]
    fn budget_no_crystals_pass_filter() {
        let summaries = vec![
            make_summary(0.5, Some(1), 0.5, "q"),
            make_summary(0.4, Some(0), 0.4, "q"),
        ];
        let budget = ContextBudget { min_qtic_class: 5, top_k: 10, max_tokens: 4096 };
        assert!(budget.select(&summaries).is_empty());
    }

    #[test]
    fn budget_select_preserves_descending_order() {
        let summaries = vec![
            make_summary(0.9, Some(4), 0.95, "first"),
            make_summary(0.8, Some(4), 0.80, "second"),
            make_summary(0.7, Some(4), 0.60, "third"),
        ];
        let budget = ContextBudget::default();
        let selected = budget.select(&summaries);
        // Scores must be non-increasing
        for w in selected.windows(2) {
            assert!(
                w[0].tripolar_score >= w[1].tripolar_score,
                "selection must preserve descending tripolar order"
            );
        }
    }
}
